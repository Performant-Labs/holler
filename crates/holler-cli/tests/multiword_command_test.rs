#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #294
//! Mechanism proof for a **multi-word** `command` array (issue #294 / the
//! Claude Code harness docs, README's "Harness recipes"): every existing
//! real-subprocess test in this crate spawns `command[0]` as an *absolute
//! path* baked in at compile time (`env!("CARGO_BIN_EXE_stub-acp")`), so
//! `query.rs`'s `resolvable()` always takes its "has a `/`, check the file
//! exists" branch, never the "bare name, search `PATH`" branch a real
//! `harness = "claude"` recipe (`command = ["npx", "-y",
//! "@agentclientprotocol/claude-agent-acp@<pin>"]`) actually exercises.
//!
//! This file proves the **mechanism** — config discovery → PATH resolution →
//! spawn — handles a bare-name, 3+-token `command` end to end, without `npx`,
//! network access, or real Claude Code credentials (issue #294 keeps that
//! part manual). It stands in a second, differently-invoked stub agent by
//! shelling `stub-acp` through `sh -c "exec <stub-acp> --chunks N"`:
//!
//! - `command[0]` is `"sh"` — a **bare name**, resolved by real `$PATH`
//!   lookup (both by `query.rs::resolvable()`'s own probe and by the OS at
//!   actual spawn time), exactly the branch a real `npx` recipe takes and
//!   the absolute-path stub commands elsewhere in this crate never do.
//! - The full argv is 3 tokens (`sh`, `-c`, the script) — matching the
//!   `["npx", "-y", "@scope/pkg"]` shape, not today's other tests' `[abs
//!   path, ...extra args]` shape.
//! - `sh` genuinely `exec`s the real `stub-acp` binary, so the spawned
//!   process ends up speaking real ACP v2 over stdio — a live `say`
//!   round-trip against it is exactly as real as any other harness's.
//!
//! What this does **not** prove: anything about the real
//! `@agentclientprotocol/claude-agent-acp` adapter or real Claude Code
//! itself — that remains the manual gate tracked in issue #294.

mod support;

use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;

use support::{holler_cmd, join, mint_token, wait_for, Body, Hub, StateDir};

const READY: Duration = Duration::from_secs(10);

fn run(state: &StateDir, args: &[&str]) -> (i32, String, String) {
    let out = holler_cmd(state)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holler")
        .wait_with_output()
        .expect("wait on holler");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Single-quote `s` for embedding in an `sh -c` script (escaping any
/// embedded `'` the POSIX way: close the quote, emit an escaped `'`, reopen
/// it) — defensive against a state-dir/binary path containing a space or
/// shell metacharacter, even though none of this repo's real paths do.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Write `<state>/body/sessions.toml` with one `[[session]]` row whose
/// `command` is a **3-token, bare-name-first** argv: `["sh", "-c", "exec
/// '<stub-acp>' <extra...>"]` — mirroring the shape (not the content) of a
/// real `harness = "claude"` recipe's `["npx", "-y", "@scope/pkg"]`, per
/// this file's own module doc. `harness` is `"claude"` (not `"opencode"`,
/// which every other test in this crate uses) so this also exercises the
/// harness id the docs recipe actually names.
fn write_multiword_claude_sessions_toml(state: &StateDir, name: &str, extra: &[&str]) -> std::path::PathBuf {
    let stub = env!("CARGO_BIN_EXE_stub-acp");
    let script = std::iter::once(sh_quote(stub))
        .chain(extra.iter().map(|a| sh_quote(a)))
        .collect::<Vec<_>>()
        .join(" ");
    let script = format!("exec {script}");

    std::fs::create_dir_all(state.body()).expect("create body dir");
    let path = state.body().join("sessions.toml");
    let toml = format!(
        "[[session]]\nname = {name:?}\nharness = \"claude\"\ncommand = [\"sh\", \"-c\", {script:?}]\n"
    );
    std::fs::write(&path, toml).expect("write sessions.toml");
    path
}

/// `body support claude` on a session whose `command` is the 3-token,
/// bare-name (`sh`) shell-wrapper shape is `ok:true` — proving
/// `query.rs::resolvable()`'s bare-name/`PATH`-search branch (never
/// exercised by this crate's other tests, which all pass an absolute path
/// as `command[0]`) correctly resolves a multi-word command, exactly as
/// issue #294's docs claim.
#[test]
fn body_support_claude_true_for_bare_name_multiword_command() {
    let state = StateDir::new();
    write_multiword_claude_sessions_toml(&state, "alpha", &["--chunks", "1"]);

    let (code, stdout, stderr) = run(&state, &["--json", "body", "support", "claude"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(
        doc["ok"].as_bool(),
        Some(true),
        "a bare-name, multi-word command[0] resolvable on PATH must be ok:true: {doc}"
    );
    assert_eq!(doc["how"].as_str(), Some("spawn: sh"), "how must name the resolved command[0]: {doc}");
}

/// The same shape with an unresolvable `command[0]` (a bare name that is not
/// on `PATH`) is `ok:false` — the probe is a real check, not a shape-only
/// "looks multi-word so it passes" shortcut.
#[test]
fn body_support_claude_false_when_bare_name_not_on_path() {
    let state = StateDir::new();
    std::fs::create_dir_all(state.body()).expect("create body dir");
    let path = state.body().join("sessions.toml");
    std::fs::write(
        &path,
        r#"[[session]]
name = "alpha"
harness = "claude"
command = ["definitely-not-a-real-shell-xyz", "-c", "exec stub-acp --chunks 1"]
"#,
    )
    .expect("write sessions.toml");

    let (code, stdout, stderr) = run(&state, &["--json", "body", "support", "claude"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let doc: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(
        doc["ok"].as_bool(),
        Some(false),
        "an unresolvable bare-name command[0] must be ok:false: {doc}"
    );
}

/// End-to-end: a real hub + a real `holler body run` spawn a session whose
/// `command` is the 3-token, bare-name-first shape (`["sh", "-c", "exec
/// '<stub-acp>' --chunks N"]`, `harness = "claude"`) — proving the full
/// config-discovery → PATH-resolution → spawn path handles a genuinely
/// multi-word command, not just today's `["opencode", "acp"]`-shaped
/// two-token case, and that a real `say` round trip against the process it
/// spawns succeeds exactly as it would for any other harness (this is the
/// mechanism issue #294 defers the *real* Claude Code adapter's own gate
/// on top of — see this file's own module doc for what is and is not
/// proved here).
#[test]
fn say_round_trip_over_multiword_bare_name_command() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);

    let (token_id, secret) = mint_token(&hub_state, "b");
    join(&body_state, &hub.ws_url(), &token_id, &secret);
    let config = write_multiword_claude_sessions_toml(&body_state, "alpha", &["--chunks", "2"]);
    let _body = Body::start(&body_state, &config);

    // Poll past the brief "hub hasn't cached this body's first presence yet"
    // window (the same observable-outcome pattern `talk_test.rs::say_ready`
    // uses) rather than a blind sleep.
    let out = wait_for(READY, || {
        let out = support::say(&hub_state, "alpha", "hi");
        if out.status.success() || !String::from_utf8_lossy(&out.stderr).contains("unknown session") {
            Some(out)
        } else {
            None
        }
    })
    .expect("`say alpha` never got past unknown_session within the ready timeout");

    assert!(
        out.status.success(),
        "say over a multi-word bare-name command must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("stub chunk"),
        "reply must carry the real stub-acp's streamed text, proving `sh` genuinely execed it: {text:?}"
    );
}
