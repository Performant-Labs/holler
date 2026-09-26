#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #163
//! The hub's token store (story #163), tested against the real file-backed
//! store (a real `HOLLER_STATE_DIR` scratch dir, a real `fs4` flock, a real
//! HMAC-SHA-256 under a real pepper). No mocks: the store is the thing under
//! test, so the tests exercise it through its public API.
//!
//! The 10 cases are the spec's RED list, verbatim. The two "concurrent" cases
//! verify the invariant the `flock` is for — *no lost writes under concurrent
//! mutation* — by driving the operations through the public API (the real
//! lock) and asserting the file's end state is exactly the expected merge.
//! (True multi-process concurrency — two CLIs racing over one `flock` — is
//! exercised by the CLI tests, which run separate processes.)

#![cfg(unix)]

use holler_proto::RedeemError;
use holler_proto::TokenError;
use holler_hub::state::{ensure_dirs, HubState};
use holler_hub::token;

/// A per-test scratch state dir, created under the OS temp dir and removed on
/// drop so a panicking test does not leak directories.
struct Tdir {
    root: std::path::PathBuf,
}

impl Tdir {
    fn new() -> Self {
        let unique = format!(
            "holler-tok-{}-{:x}-{:x}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        let root = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&root).expect("create scratch state dir");
        Self { root }
    }

    fn tokens_path(&self) -> std::path::PathBuf {
        self.root.join("hub").join("tokens.json")
    }
}

/// A well-formed (real, curve-valid), deterministic Ed25519 public key hex —
/// what a body would send as `circuit/join`'s `body_pubkey` (issue #323).
fn pubkey(seed: u8) -> String {
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
    hex::encode(signing_key.verifying_key().to_bytes())
}

/// A well-formed, deterministic X25519 public key hex — what a body would
/// send as `circuit/join`'s `body_x25519_pubkey` (issue #337).
fn x25519_pubkey(seed: u8) -> String {
    let secret = x25519_dalek::StaticSecret::from([seed; 32]);
    let public = x25519_dalek::PublicKey::from(&secret);
    hex::encode(public.as_bytes())
}

impl Drop for Tdir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Ensure the hub dir exists (the store's paths live under `<root>/hub`).
fn prep(dir: &Tdir) -> HubState {
    let state = HubState::from_root(dir.root.clone());
    let _ = ensure_dirs(&state);
    state
}

#[test]
fn mint_then_list_shows_unused_without_secret() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint io");
    // The secret is returned to the operator, but it is never the record's.
    let records = token::list(&state).expect("list");
    assert_eq!(records.len(), 1, "exactly one token after one mint");
    let row = &records[0];
    assert_eq!(row.token_id, minted.record.token_id);
    assert_eq!(row.label, "io");
    assert_eq!(row.state, holler_hub::token::TokenState::Unused);
    // The record holds the HMAC, not the secret.
    assert!(!row.secret_hmac.contains("hlr_join_"), "secret_hmac is a digest, not the secret");
    assert!(row.secret_hmac != minted.secret, "the stored hmac is not the plaintext secret");
    assert!(minted.secret.starts_with("hlr_join_"), "the returned secret is a join secret");
}

#[test]
fn duplicate_label_is_refused_exit_3() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let _ = token::mint("io", 3600, &state).expect("first mint of io");
    let err = token::mint("io", 3600, &state).expect_err("a second mint of the same label is refused");
    matches!(err, TokenError { .. });
    // A distinct label still succeeds.
    assert!(token::mint("web", 3600, &state).is_ok());
}

#[test]
fn redeem_binds_and_consumes_secret() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    let body_pubkey = pubkey(1);
    let body_x25519_pubkey = x25519_pubkey(1);
    let client_id = token::redeem(&minted.secret, "myhost", &body_pubkey, &body_x25519_pubkey, &state).expect("first redeem");
    assert!(client_id.starts_with("cli_"));
    // The secret is now consumed: a second redeem is AlreadyBound.
    let again = token::redeem(&minted.secret, "myhost", &pubkey(2), &x25519_pubkey(2), &state);
    assert_eq!(again, Err(RedeemError::AlreadyBound), "the second redeem is refused");
    // The record is now bound to the client, with both public keys registered.
    let row = token::list(&state).expect("list")[0].clone();
    assert_eq!(row.state, holler_hub::token::TokenState::Bound);
    assert_eq!(row.client_id.as_deref(), Some(client_id.as_str()));
    assert_eq!(row.hostname.as_deref(), Some("myhost"));
    assert_eq!(row.body_pubkey.as_deref(), Some(body_pubkey.as_str()));
    assert_eq!(row.body_x25519_pubkey.as_deref(), Some(body_x25519_pubkey.as_str()));
}

/// Model a "concurrent second client" redeeming the same secret. Two CLIs on
/// one hub are two *connections* over the same state dir (the operator's
/// `HOLLER_STATE_DIR`), so both resolve the same dir — the `flock` is what
/// serializes them. We exercise the consumption invariant (a secret is
/// single-use) through the public API: the first redeem wins and binds the
/// token; the second is refused `AlreadyBound`, never handed a second
/// registered key. (True multi-process contention over one `flock` is
/// covered by the CLI tests, which run real separate processes.)
#[test]
fn concurrent_redeem_has_exactly_one_winner() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    // First client redeems and wins.
    let client_id =
        token::redeem(&minted.secret, "host-a", &pubkey(3), &x25519_pubkey(3), &state).expect("first client redeems and wins");
    assert!(client_id.starts_with("cli_"));
    // The second client presents the same (already-consumed) secret: refused
    // AlreadyBound, not handed a second registered key.
    let again = token::redeem(&minted.secret, "host-b", &pubkey(4), &x25519_pubkey(4), &state);
    assert_eq!(again, Err(RedeemError::AlreadyBound), "a second client cannot redeem a consumed secret");
}

/// Drive 8 mints and 4 deletes through the public API (each takes the real
/// `flock`) and assert the file's end state is exactly the expected merge —
/// proof that no concurrent write is lost to the store's read-modify-write.
fn concurrent_mint_delete_drive(dir: &Tdir) {
    let state = prep(dir);
    // A victim the deletes race against (pre-existing, so each delete has a
    // real target).
    let pre = token::mint("victim", 3600, &state).expect("seed victim");
    let victim_id = pre.record.token_id.clone();
    // 8 mints (distinct labels) racing.
    for i in 0..8u32 {
        token::mint(&format!("w-{i}"), 3600, &state)
            .unwrap_or_else(|e| panic!("concurrent mint w-{i} failed: {e}"));
    }
    // 4 deletes of the same victim racing the mints.
    for _ in 0..4 {
        token::delete(&victim_id, &state)
            .unwrap_or_else(|e| panic!("concurrent delete failed: {e}"));
    }
}

#[test]
fn concurrent_mint_delete_do_not_lose_writes() {
    let dir = Tdir::new();
    concurrent_mint_delete_drive(&dir);
    // End state: all 8 mints present + the victim (revoked by the deletes, its
    // row kept — delete does not destroy the row, it flips state). No write is
    // lost: if any mint had been clobbered by a delete's stale snapshot, the
    // count would be short.
    let rows = token::list(&prep(&dir)).expect("final list");
    let mints: Vec<_> = rows.iter().filter(|r| r.label.starts_with("w-")).collect();
    assert_eq!(mints.len(), 8, "all 8 concurrent mints survived (no lost writes)");
    let victim = rows.iter().find(|r| r.label == "victim").expect("the victim row is kept");
    assert_eq!(victim.state, holler_hub::token::TokenState::Revoked, "the victim was revoked by the racing deletes");
    assert_eq!(rows.len(), 9, "9 rows: 8 mints + 1 revoked victim (no row lost, none duplicated)");
}

/// Issue #323's RED test: a bound token's row holds no secret material — the
/// join secret's HMAC only (the pepper-keyed digest of an already-spent,
/// one-time value ADR 0008 explicitly keeps), never the secret itself and
/// never a credential (there is none to store any more — only the body's
/// public key, which is not a secret).
#[test]
fn hub_store_contains_no_secret_material_for_a_bound_token() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    let body_pubkey = pubkey(5);
    let body_x25519_pubkey = x25519_pubkey(5);
    let _client_id = token::redeem(&minted.secret, "myhost", &body_pubkey, &body_x25519_pubkey, &state).expect("redeem");
    let raw = std::fs::read_to_string(dir.tokens_path()).expect("read tokens.json");
    assert!(!raw.contains(&minted.secret), "the join secret is not in tokens.json");
    // The at-rest forms (HMAC digest for the spent secret) are present, not
    // the secret itself; the public keys are public keys, so they are present
    // in the clear (issue #323/#337 — a stolen store yields no secret material).
    assert!(raw.contains(&minted.record.secret_hmac), "the at-rest secret_hmac is present");
    let row = token::list(&state).expect("list")[0].clone();
    assert_eq!(row.body_pubkey.as_deref(), Some(body_pubkey.as_str()), "the registered Ed25519 public key is present once bound");
    assert_eq!(row.body_x25519_pubkey.as_deref(), Some(body_x25519_pubkey.as_str()), "the registered X25519 public key is present once bound");
}

#[test]
fn pepper_autogenerated_0600() {
    let dir = Tdir::new();
    let state = prep(&dir);
    // No HOLLER_PEPPER in the env (the test process does not set it); the
    // store must autogenerate the pepper file at 0600 on first use.
    let _ = token::mint("io", 3600, &state).expect("mint (triggers pepper autogen)");
    let pepper = dir.root.join("hub").join(".pepper");
    assert!(pepper.exists(), "the pepper file was autogenerated");
    let bytes = std::fs::read(&pepper).expect("read pepper");
    assert_eq!(bytes.len(), 32, "the autogenerated pepper is 32 bytes");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&pepper).expect("stat pepper").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the pepper file is mode 0600 (owner-only)");
    }
}

#[test]
fn expired_secret_fails_closed() {
    let dir = Tdir::new();
    let state = prep(&dir);
    // Mint normally, then rewrite the record's `expires` into the past (the
    // store's own write path) so redeem must see it as expired. This tests the
    // redeem guard (expired → fail closed), not the TTL arithmetic.
    let minted = token::mint("io", 3600, &state).expect("mint");
    let path = dir.tokens_path();
    let raw = std::fs::read_to_string(&path).expect("read");
    let mut doc: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let arr = doc.as_array_mut().expect("array");
    let row = arr.get_mut(0).expect("one row");
    let obj = row.as_object_mut().expect("row object");
    obj.insert("expires".to_string(), serde_json::json!(1)); // in the past
    std::fs::write(&path, doc.to_string()).expect("write");
    let err = token::redeem(&minted.secret, "myhost", &pubkey(6), &x25519_pubkey(6), &state);
    assert_eq!(err, Err(RedeemError::Expired), "an expired secret fails closed with Expired");
}

#[test]
fn delete_unused_then_redeem_fails() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    token::delete(&minted.record.token_id, &state).expect("invalidate the unused token");
    // A revoked token's secret is no longer redeemable.
    let err = token::redeem(&minted.secret, "myhost", &pubkey(7), &x25519_pubkey(7), &state);
    assert_eq!(err, Err(RedeemError::Revoked), "a revoked (deleted) token's secret is refused");
}

#[test]
fn list_json_shape() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    let _ = token::redeem(&minted.secret, "myhost", &pubkey(8), &x25519_pubkey(8), &state).expect("redeem");
    let raw = std::fs::read_to_string(dir.tokens_path()).expect("read tokens.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let arr = doc.as_array().expect("tokens.json is a JSON array");
    let row = &arr[0];
    let obj = row.as_object().expect("row is an object");
    // The exact keys the spec pins on the record (the bound ones are present
    // after the redeem; the optional ones that are absent are omitted).
    for k in ["token_id", "label", "created", "expires", "state", "secret_hmac", "body_pubkey", "body_x25519_pubkey", "client_id", "hostname", "bound_at"] {
        assert!(obj.get(k).is_some(), "record has key {k:?}");
    }
    assert_eq!(obj["state"].as_str(), Some("bound"));
    // No secret ever appears as a top-level key.
    assert!(obj.get("secret").is_none(), "there is no raw `secret` key");
    assert!(obj.get("credential").is_none(), "there is no raw `credential` key (issue #323: none is ever minted)");
}

/// **Regression (issue #370's load baseline).** The hub's own live paths must
/// not fail each other on the token store's `flock`.
///
/// `bound_record` (twice per `circuit/authenticate`) and `touch_last_seen`
/// (once per presence heartbeat) both run *inside the hub*, on per-connection
/// tasks, so N connected bodies contend for this one lock continuously. Both
/// used the non-retrying `acquire_lock`, whose contention outcome is an
/// `Err` — which `circuit/authenticate` maps to `-32002 unauthenticated`, and
/// which the lockout then counts as a *failed auth*. The issue #370 harness
/// (`crates/holler-load-test`) measured the result directly: 3 of 50 and 7 of
/// 200 concurrent connections completed the handshake, the rest refused with
/// the lock's own "another holler process holds the token lock; retry" text,
/// and the spurious failures cascaded into an IP lockout. Issue #301 had
/// already fixed exactly this defect class at `redeem`'s call site; these are
/// its two siblings.
///
/// The assertion is specifically that *no* call fails for lock contention —
/// not that they are fast. A retry that waits is correct; an error is not.
#[test]
fn concurrent_live_path_reads_never_lose_the_lock_race() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("conc", 3600, &state).expect("mint");
    let _ = token::redeem(&minted.secret, "myhost", &pubkey(9), &x25519_pubkey(9), &state).expect("redeem");
    let token_id = minted.record.token_id;

    // Deliberately more threads than cores: the failure this pins is a lost
    // `try_lock` race, which needs real simultaneity to reproduce.
    const THREADS: usize = 24;
    const ROUNDS: usize = 8;
    let errors = std::sync::Mutex::new(Vec::<String>::new());
    std::thread::scope(|scope| {
        for _ in 0..THREADS {
            scope.spawn(|| {
                for _ in 0..ROUNDS {
                    if let Err(e) = token::bound_record(&token_id, &state) {
                        errors.lock().expect("errors lock").push(format!("bound_record: {}", e.message));
                    }
                    if let Err(e) = token::touch_last_seen(&token_id, &state) {
                        errors.lock().expect("errors lock").push(format!("touch_last_seen: {}", e.message));
                    }
                }
            });
        }
    });

    let errors = errors.lock().expect("errors lock");
    assert!(
        errors.is_empty(),
        "{} of {} live-path token-store calls failed under concurrency — \
         a bound, live token must never read as unavailable because a sibling \
         connection held the lock: {:?}",
        errors.len(),
        THREADS * ROUNDS * 2,
        errors.iter().take(5).collect::<Vec<_>>(),
    );
}

/// **Regression (issue #401).** The operator-facing store operations — `mint`,
/// `list` and `delete`, i.e. what `holler hub token mint|list|delete` run —
/// must not fail with the lock's "retry" error just because a live hub is
/// touching the same store. #373's churn run measured ~15% of mints failing
/// that way; `list` and `delete` used the same non-retrying lock.
///
/// Live-path threads hammer `touch_last_seen` (the per-heartbeat write) while
/// operator threads mint, list and delete distinct tokens. As with the
/// live-path test above, the assertion is that *no* call fails for lock
/// contention, not that any is fast: waiting is correct, an error is not.
#[test]
fn concurrent_operator_paths_never_lose_the_lock_race() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let bound = token::mint("bound-body", 3600, &state).expect("mint");
    let _ = token::redeem(&bound.secret, "myhost", &pubkey(9), &x25519_pubkey(9), &state).expect("redeem");
    let bound_id = bound.record.token_id.clone();

    const LIVE_THREADS: usize = 8;
    const OPERATOR_THREADS: usize = 16;
    const ROUNDS: usize = 6;
    let errors = std::sync::Mutex::new(Vec::<String>::new());
    let note = |what: &str, e: TokenError| errors.lock().expect("errors lock").push(format!("{what}: {}", e.message));
    std::thread::scope(|scope| {
        for _ in 0..LIVE_THREADS {
            scope.spawn(|| {
                for _ in 0..ROUNDS * 4 {
                    if let Err(e) = token::touch_last_seen(&bound_id, &state) {
                        note("touch_last_seen", e);
                    }
                }
            });
        }
        for t in 0..OPERATOR_THREADS {
            let (state, note) = (&state, &note);
            scope.spawn(move || {
                for r in 0..ROUNDS {
                    let label = format!("op-{t}-{r}");
                    match token::mint(&label, 3600, state) {
                        Ok(m) => {
                            if let Err(e) = token::delete(&m.record.token_id, state) {
                                note("delete", e);
                            }
                        }
                        Err(e) => note("mint", e),
                    }
                    if let Err(e) = token::list(state) {
                        note("list", e);
                    }
                }
            });
        }
    });

    let errors = errors.lock().expect("errors lock");
    assert!(
        errors.is_empty(),
        "{} operator-path token-store calls failed under concurrency — an operator \
         command must wait out a busy store, not ask its caller to retry: {:?}",
        errors.len(),
        errors.iter().take(5).collect::<Vec<_>>(),
    );
}

/// Rewrite the only record's `expires` in tokens.json (the store re-reads the
/// file on every call, so the next `bound_record` sees it).
fn set_only_expires(dir: &Tdir, expires: u64) {
    let path = dir.tokens_path();
    let mut doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
    let rows = doc.as_array_mut().expect("tokens.json is an array");
    assert_eq!(rows.len(), 1, "exactly one record: {rows:?}");
    rows[0]["expires"] = serde_json::json!(expires);
    std::fs::write(&path, doc.to_string()).expect("write");
}

/// Issue #453: `expires` bounds only the unredeemed join secret. A bound
/// token whose `expires` has passed still yields its record to the
/// authenticate path.
#[test]
fn bound_record_ignores_expires() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    token::redeem(&minted.secret, "myhost", &pubkey(20), &x25519_pubkey(20), &state).expect("redeem");
    set_only_expires(&dir, 1); // long past
    let record = token::bound_record(&minted.record.token_id, &state)
        .unwrap_or_else(|e| panic!("a bound token past its expires must still authenticate: {}", e.message));
    assert_eq!(record.token_id, minted.record.token_id);
    assert_eq!(record.body_pubkey.as_deref(), Some(pubkey(20).as_str()), "the bound key is returned");
}

/// Issue #453: removing the bound-token expiry check must not let a token
/// that was never redeemed authenticate, whatever its `expires`.
#[test]
fn bound_record_rejects_unused_token() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    for expires in [u64::MAX / 2, 1] {
        set_only_expires(&dir, expires);
        let err = token::bound_record(&minted.record.token_id, &state).expect_err("an unused token must not authenticate");
        assert!(err.message.contains("is not bound"), "expires={expires}: {}", err.message);
    }
}

// --- #483: a kill during a save never corrupts the store --------------------

const SAVE_LOOP_ROOT_ENV: &str = "HOLLER_483_SAVE_LOOP_ROOT";
const SAVE_LOOP_TOKEN_ENV: &str = "HOLLER_483_SAVE_LOOP_TOKEN";

/// Kills its child on drop, so a failing assertion never leaves a stray
/// save-loop process behind.
struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The child half of `a_kill_during_save_never_leaves_the_store_empty_or_corrupt`:
/// the parent re-executes this test binary to run only this test, with the
/// state dir in the environment. It rewrites the whole store in a tight loop
/// (each `token::delete` is a load-modify-save under the lock) until it is
/// SIGKILLed. Without the environment it does nothing.
#[test]
#[ignore = "child process of a_kill_during_save_never_leaves_the_store_empty_or_corrupt (#483)"]
fn kill_during_save_child_loop() {
    let (Ok(root), Ok(token_id)) = (std::env::var(SAVE_LOOP_ROOT_ENV), std::env::var(SAVE_LOOP_TOKEN_ENV)) else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let state = HubState::from_root(root.clone());
    token::delete(&token_id, &state).expect("child: first save");
    std::fs::write(root.join("child-ready"), b"").expect("child: ready marker");
    loop {
        token::delete(&token_id, &state).expect("child: save");
    }
}

/// Issue #483: the hub persists `tokens.json` on mint, revoke, bind and the
/// presence heartbeat. A hub killed in the middle of a save must leave the
/// old store or the new one, never an empty or partial file (an empty file
/// made every later authentication fail with "tokens store is corrupted").
///
/// A child process saves a padded store in a tight loop and is SIGKILLed at
/// varied moments, and at once whenever the file is seen empty. The file must
/// never be seen empty, and after every kill it must be non-empty, parse, and
/// still hold every record. (A truncate-then-write save fails the first
/// assertion within a few kills.)
#[test]
fn a_kill_during_save_never_leaves_the_store_empty_or_corrupt() {
    const PADDING: usize = 500;
    const KILLS: u64 = 40;
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("victim", 3600, &state).expect("mint");

    // Pad the store so every save rewrites a large file (a wider window).
    let path = dir.tokens_path();
    let mut rows: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
    let template = rows[0].clone();
    for i in 0..PADDING {
        let mut row = template.clone();
        row["token_id"] = serde_json::json!(format!("tok_pad{i:06}"));
        row["label"] = serde_json::json!(format!("pad-{i}"));
        rows.push(row);
    }
    std::fs::write(&path, serde_json::to_string_pretty(&rows).expect("serialize")).expect("write padded store");
    let expected = token::list(&state).expect("padded store loads").len();
    assert_eq!(expected, PADDING + 1);

    let exe = std::env::current_exe().expect("current test binary");
    let ready = dir.root.join("child-ready");
    for i in 0..KILLS {
        let _ = std::fs::remove_file(&ready);
        let child = std::process::Command::new(&exe)
            .args(["kill_during_save_child_loop", "--exact", "--ignored", "--nocapture", "--test-threads=1"])
            .env(SAVE_LOOP_ROOT_ENV, &dir.root)
            .env(SAVE_LOOP_TOKEN_ENV, &minted.record.token_id)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn save-loop child");
        let mut child = KillOnDrop(child);

        // Wait (bounded) for the child's first completed save.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !ready.exists() {
            if let Ok(Some(status)) = child.0.try_wait() {
                panic!("kill {i}: the save-loop child exited early ({status}); store size {:?}",
                    std::fs::metadata(&path).map(|m| m.len()));
            }
            assert!(std::time::Instant::now() < deadline, "kill {i}: the save-loop child never became ready");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        // Watch the file for a varied stretch of the save loop, and kill the
        // child the instant the file is seen empty (the worst moment to die),
        // or at the end of the stretch. The stretch varies the kill moment.
        let watch_until = std::time::Instant::now() + std::time::Duration::from_micros(2_000 + (i * 1_777) % 9_000);
        let mut seen_empty = false;
        while std::time::Instant::now() < watch_until {
            if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) == 0 {
                seen_empty = true;
                break;
            }
        }
        drop(child); // SIGKILL + reap

        let len = std::fs::metadata(&path).expect("tokens.json must still exist").len();
        assert!(!seen_empty, "kill {i}: tokens.json was observed empty (0 bytes) during a save (killed there; {len} bytes after the kill)");
        assert!(len > 0, "kill {i}: a kill during a save left tokens.json empty (0 bytes)");
        let records = token::list(&state)
            .unwrap_or_else(|e| panic!("kill {i}: the store no longer loads after a kill ({len} bytes): {}", e.message));
        assert_eq!(records.len(), expected, "kill {i}: a kill during a save lost records");
    }
}
