//! The shared harness's token-onboarding helpers (`mint_token`, `hub_pubkey`,
//! `join`), split out of `mod.rs` (issue #450) to keep that file under the
//! workspace's 900-line build guard (`scripts/lint.sh` check 4), the same
//! split `cmds.rs` made for the one-shot command runners.

use super::*;

/// Mint a join token on the (local, state-dir-bound) hub.
///
/// Returns `(token_id, secret)` — the pair a body joins with (`body join --token <id>:<secret>`).
/// No live hub process is required to *mint*. Retries on "another holler process holds the token
/// lock; retry" (`acquire_lock`'s `WouldBlock` is a plain error, never a wait): a second mint can
/// land mid-authenticate on the same store.
pub fn mint_token(state: &StateDir, label: &str) -> (String, String) {
    let mut out = run_mint(state, label); // `--json` is global (ADR 0003): before the subcommand.
    for attempt in 1..=5 {
        if out.status.success() {
            break;
        }
        let stderr = String::from_utf8_lossy(&out.stderr);
        if !stderr.contains("holds the token lock") || attempt == 5 {
            assert!(out.status.success(), "hub token mint failed (after {attempt} attempts): {stderr}");
        }
        std::thread::sleep(Duration::from_millis(200 * attempt as u64));
        out = run_mint(state, label);
    }
    let v: Value = serde_json::from_slice(&out.stdout).expect("mint --json is a JSON object");
    // The `--json` document's id field is `token_id` (ADR 0003: the join token
    // is `ID:SECRET` and `ID` is the token id). `id` is the wrong field name —
    // the document has always carried `token_id`.
    let token_id = v["token_id"]
        .as_str()
        .expect("mint --json result carries `token_id`")
        .to_string();
    let secret = v["secret"]
        .as_str()
        .expect("mint result carries `secret`")
        .to_string();
    (token_id, secret)
}

/// One attempt at `hub token mint` (see [`mint_token`]'s retry loop).
fn run_mint(state: &StateDir, label: &str) -> Output {
    holler_cmd(state)
        .args(["--json", "hub", "token", "mint", "--label", label])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `hub token mint`")
}

/// The hub's X25519 public key (issue #322), read via `hub status --json`
/// against `state`'s own hub state dir — the test harness's stand-in for the
/// out-of-band channel a real operator copies the join line's `--hub-key`
/// over: it never asks the body's own connection for this, only the hub's
/// local control surface, which is exactly the "physically carried" trust
/// model `body join --hub-key` is built to require.
pub fn hub_pubkey(state: &StateDir) -> String {
    let out = holler_cmd(state)
        .args(["--json", "hub", "status"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `hub status`");
    assert!(out.status.success(), "hub status --json failed: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).expect("hub status --json is valid JSON");
    v["hub_pubkey"]
        .as_str()
        .expect("hub status --json carries hub_pubkey (issue #322)")
        .to_string()
}

/// Join `state`'s body to the hub at `ws_url` using the minted token, and
/// assert the join succeeded (exit 0). Pins the hub's real public key (read
/// via [`hub_pubkey`] against `hub_state` — the hub's *own* state dir, which
/// is where the live hub's control socket lives; `hub_state` and `state` are
/// the same directory in the tests that share one dir for hub+body, and
/// different directories in the tests that keep the body isolated) — the
/// correct-key path every caller of this helper wants; a test that
/// specifically wants to exercise a *wrong* key builds its own `body join
/// --hub-key` invocation instead of using this helper.
///
/// Retries the join a few times before failing. `body join` reads the hub's
/// `tokens.json` (in the hub's state dir) to redeem the one-time secret, and a
/// token is minted by a *separate* CLI process writing that same file — so on
/// a loaded CI runner the freshly-minted token can transiently read as absent
/// ("no matching token"), a test-harness race that is unrelated to the feature
/// under test. A real failure (a genuinely bad token, an already-redeemed
/// secret) is reproduced on every retry, so a bounded retry loop only absorbs
/// the race and still surfaces every genuine join failure. (Issue #186 CI
/// hardening — this is what kept `say_ambiguous*` flapping on the shared
/// ubuntu runner.)
pub fn join(state: &StateDir, hub_state: &StateDir, ws_url: &str, token_id: &str, secret: &str) {
    let token = format!("{token_id}:{secret}");
    let hub_key = hub_pubkey(hub_state);
    let mut out = run_join(state, ws_url, &token, &hub_key);
    for attempt in 1..=5 {
        if out.status.success() {
            return;
        }
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Only the transient "hub hasn't flushed the token yet" race is
        // retryable; anything else (bad token, already redeemed, …) will just
        // fail again and should be surfaced immediately.
        if !stderr.contains("no matching token") || attempt == 5 {
            assert!(
                out.status.success(),
                "body join failed (exit {:?}, after {attempt} attempts): {stderr}",
                out.status.code(),
            );
        }
        std::thread::sleep(Duration::from_millis(200 * attempt as u64));
        out = run_join(state, ws_url, &token, &hub_key);
    }
}

/// One attempt at `body join` (see [`join`]'s retry loop).
fn run_join(state: &StateDir, ws_url: &str, token: &str, hub_key: &str) -> Output {
    holler_cmd(state)
        .args(["body", "join", "--server", ws_url, "--token", token, "--hub-key", hub_key])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `body join`")
}
