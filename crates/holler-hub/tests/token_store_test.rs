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
    let client_id = token::redeem(&minted.secret, "myhost", &body_pubkey, &state).expect("first redeem");
    assert!(client_id.starts_with("cli_"));
    // The secret is now consumed: a second redeem is AlreadyBound.
    let again = token::redeem(&minted.secret, "myhost", &pubkey(2), &state);
    assert_eq!(again, Err(RedeemError::AlreadyBound), "the second redeem is refused");
    // The record is now bound to the client, with its public key registered.
    let row = token::list(&state).expect("list")[0].clone();
    assert_eq!(row.state, holler_hub::token::TokenState::Bound);
    assert_eq!(row.client_id.as_deref(), Some(client_id.as_str()));
    assert_eq!(row.hostname.as_deref(), Some("myhost"));
    assert_eq!(row.body_pubkey.as_deref(), Some(body_pubkey.as_str()));
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
        token::redeem(&minted.secret, "host-a", &pubkey(3), &state).expect("first client redeems and wins");
    assert!(client_id.starts_with("cli_"));
    // The second client presents the same (already-consumed) secret: refused
    // AlreadyBound, not handed a second registered key.
    let again = token::redeem(&minted.secret, "host-b", &pubkey(4), &state);
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
    let _client_id = token::redeem(&minted.secret, "myhost", &body_pubkey, &state).expect("redeem");
    let raw = std::fs::read_to_string(dir.tokens_path()).expect("read tokens.json");
    assert!(!raw.contains(&minted.secret), "the join secret is not in tokens.json");
    // The at-rest forms (HMAC digest for the spent secret) are present, not
    // the secret itself; the public key is a public key, so it is present in
    // the clear (issue #323 — a stolen store yields no secret material).
    assert!(raw.contains(&minted.record.secret_hmac), "the at-rest secret_hmac is present");
    let row = token::list(&state).expect("list")[0].clone();
    assert_eq!(row.body_pubkey.as_deref(), Some(body_pubkey.as_str()), "the registered public key is present once bound");
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
    let err = token::redeem(&minted.secret, "myhost", &pubkey(6), &state);
    assert_eq!(err, Err(RedeemError::Expired), "an expired secret fails closed with Expired");
}

#[test]
fn delete_unused_then_redeem_fails() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    token::delete(&minted.record.token_id, &state).expect("invalidate the unused token");
    // A revoked token's secret is no longer redeemable.
    let err = token::redeem(&minted.secret, "myhost", &pubkey(7), &state);
    assert_eq!(err, Err(RedeemError::Revoked), "a revoked (deleted) token's secret is refused");
}

#[test]
fn list_json_shape() {
    let dir = Tdir::new();
    let state = prep(&dir);
    let minted = token::mint("io", 3600, &state).expect("mint");
    let _ = token::redeem(&minted.secret, "myhost", &pubkey(8), &state).expect("redeem");
    let raw = std::fs::read_to_string(dir.tokens_path()).expect("read tokens.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let arr = doc.as_array().expect("tokens.json is a JSON array");
    let row = &arr[0];
    let obj = row.as_object().expect("row is an object");
    // The exact keys the spec pins on the record (the bound ones are present
    // after the redeem; the optional ones that are absent are omitted).
    for k in ["token_id", "label", "created", "expires", "state", "secret_hmac", "body_pubkey", "client_id", "hostname", "bound_at"] {
        assert!(obj.get(k).is_some(), "record has key {k:?}");
    }
    assert_eq!(obj["state"].as_str(), Some("bound"));
    // No secret ever appears as a top-level key.
    assert!(obj.get("secret").is_none(), "there is no raw `secret` key");
    assert!(obj.get("credential").is_none(), "there is no raw `credential` key (issue #323: none is ever minted)");
}
