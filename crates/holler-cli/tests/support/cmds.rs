//! The shared harness's one-shot CLI command runners (`roster`/`hub status`/
//! `body status`/`say`/`interrupt`/`answer`/`wait`), split out of `mod.rs`
//! (issue #142) to keep that file under the workspace's 900-line build guard
//! (`scripts/lint.sh` check 4) — the same kind of split `docs.rs` and
//! `roster.rs` made for their own growth. Plain functions, not a struct: each
//! one shells out to the real `holler` binary against `state` and hands back
//! either the raw `Output` (the caller asserts on exit code/stdout/stderr
//! itself) or the parsed `--json` document.

use std::process::{Output, Stdio};

use serde_json::Value;

use super::{holler_cmd, StateDir};

/// The hub's current roster as JSON (parsed from `holler roster --json`).
pub fn roster_json(state: &StateDir) -> Value {
    status_json_of(state, &["roster"])
}

/// The hub's status as JSON (parsed from `holler hub status --json`).
pub fn hub_status_json(state: &StateDir) -> Value {
    status_json_of(state, &["hub", "status"])
}

/// A body's status as JSON (parsed from `holler body status --json`).
pub fn body_status_json(state: &StateDir) -> Value {
    status_json_of(state, &["body", "status"])
}

fn status_json_of(state: &StateDir, sub: &[&str]) -> Value {
    let args: Vec<String> = sub
        .iter()
        .map(|s| s.to_string())
        .chain(["--json".to_string()])
        .collect();
    let out = holler_cmd(state)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run status command");
    assert!(
        out.status.success(),
        "`holler {}` failed: {}",
        sub.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("status --json is valid JSON")
}

/// Send a one-shot prompt to `session` on the hub and return the raw `Output`
/// (`say` prints the reply; tests assert on stdout).
pub fn say(state: &StateDir, session: &str, text: &str) -> Output {
    holler_cmd(state)
        .args(["say", session, text])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `say`")
}

/// Interrupt `session`'s in-flight turn and return the raw `Output`.
pub fn interrupt(state: &StateDir, session: &str) -> Output {
    holler_cmd(state)
        .args(["interrupt", session])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `interrupt`")
}

/// Resolve `session`'s held permission/elicitation with `choice` (issue
/// #151) and return the raw `Output`.
pub fn answer(state: &StateDir, session: &str, choice: &str) -> Output {
    holler_cmd(state)
        .args(["answer", session, choice])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `answer`")
}

/// `holler wait ARGS...` (issue #142), returning the raw `Output` — callers
/// assert on exit code / stdout / stderr / timing themselves (the RED list
/// exercises the timeout, no-hub, and match-shape cases, each with different
/// exit-code expectations).
pub fn wait_cmd(state: &StateDir, args: &[&str]) -> Output {
    let mut full = vec!["wait"];
    full.extend_from_slice(args);
    holler_cmd(state)
        .args(&full)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `wait`")
}
