#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #151
//! RED tests for `answer SESSION CHOICE` (issue #151), against real hub +
//! body + `stub-acp` (#130) subprocesses — no mocks of the circuit. Mirrors
//! `talk_test.rs`'s own setup pattern (issue #190); a session's `--ask-
//! permission` gate (the same stub-acp switch `acp_driver_test.rs` already
//! exercises in-process) is what actually drives `input-required` here, end
//! to end through the real wire.
//!
//! # Coverage
//!
//! - `answer` resolves a held permission and the turn resumes/completes.
//! - `answer` on a session with nothing pending is `-32010 nothing_pending`
//!   (exit 1).
//! - A plain `say` to an `input-required` session is `session_busy` with an
//!   `answer` hint, not silently queued — and `--queue` does not override
//!   that refusal (distinct from the `working` busy case).
//! - `roster --json` carries the `pending` array (kind/prompt/options) while
//!   `input-required`, and it is gone once answered.

mod support;

use std::process::{Command, Output, Stdio};
use std::time::Duration;

use support::{join, mint_token, wait_for, write_sessions_toml, Body, Hub, StateDir};

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}
fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Roster rows are keyed by the **qualified** name (`<label>/<session>`,
/// ADR 0005 §2) — bodies never send the label on the wire, but the hub
/// attributes it at `circuit/authenticate` time from the minted token
/// (`start_body`'s own "b" label here). `say`/`answer`/`interrupt` accept a
/// bare `<session>` when it resolves unambiguously, but a roster row's own
/// `name` is always qualified — this matches either the exact bare name or
/// a `.../<session>` suffix, so the tests below can still write the bare
/// name they pass to those verbs.
fn row_name_matches(row: &serde_json::Value, session: &str) -> bool {
    match row.get("name").and_then(|n| n.as_str()) {
        Some(name) => name == session || name.ends_with(&format!("/{session}")),
        None => false,
    }
}

/// Same setup `talk_test.rs` uses: mint a token against `hub_state`, join
/// `body_state`, and start `holler body run` over `sessions`.
fn start_body(hub_state: &StateDir, body_state: &StateDir, hub: &Hub, sessions: &[(&str, &[&str])]) -> Body {
    let (token_id, secret) = mint_token(hub_state, "b");
    join(body_state, hub_state, &hub.ws_url(), &token_id, &secret);
    let config = write_sessions_toml(body_state, sessions);
    Body::start(body_state, &config)
}

/// `say SESSION TEXT [ARGS...]` in a background thread — used here only to
/// get a permission-gated turn under way; the RED list's own assertions run
/// against `answer`/`roster`/a second `say`, not this call's own result.
fn say_full_in_background(state: &StateDir, args: Vec<String>) -> std::thread::JoinHandle<Output> {
    let state_path = state.path().to_path_buf();
    std::thread::spawn(move || {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        Command::new(support::holler_bin())
            .env("HOLLER_STATE_DIR", &state_path)
            .env("HOLLER_DEBUG", "quiet")
            .env("HOLLER_LOG_FORMAT", "json")
            .arg("say")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|c| c.wait_with_output())
            .expect("run `say`")
    })
}

/// Poll `say session TEXT` until it stops failing with `unknown session`
/// (the hub has not yet cached this body's first presence) or `timeout`
/// elapses — the same "not ready yet" retry `talk_test.rs::say_ready` uses.
fn say_ready(state: &StateDir, session: &str, text: &str, timeout: Duration) -> Output {
    wait_for(timeout, || {
        let out = support::say(state, session, text);
        if out.status.success() || !stderr_of(&out).contains("unknown session") {
            Some(out)
        } else {
            None
        }
    })
    .unwrap_or_else(|| panic!("`say {session}` never got past unknown_session within {timeout:?}"))
}

/// Poll the roster until `session`'s row reports `state == want`, or panic
/// after `timeout`.
fn wait_for_roster_state(state: &StateDir, session: &str, want: &str, timeout: Duration) -> serde_json::Value {
    wait_for(timeout, || {
        let doc = support::roster_json(state);
        let rows = doc.get("rows")?.as_array()?;
        let row = rows.iter().find(|r| row_name_matches(r, session))?;
        (row.get("state").and_then(|s| s.as_str()) == Some(want)).then(|| row.clone())
    })
    .unwrap_or_else(|| panic!("`{session}` never reached roster state {want:?} within {timeout:?}"))
}

/// Poll the roster until `session` appears at all (any state) — the same
/// "wait for the body's first presence to land" race `talk_test.rs::
/// say_ready` guards against via a real `say`, but done here by reading the
/// roster instead: a gated session's *every* turn (including the first)
/// raises the gate, so a warm-up `say` would itself park on it.
fn wait_until_session_present(state: &StateDir, session: &str, timeout: Duration) {
    wait_for(timeout, || {
        let doc = support::roster_json(state);
        let rows = doc.get("rows")?.as_array()?.clone();
        rows.iter().any(|r| row_name_matches(r, session)).then_some(())
    })
    .unwrap_or_else(|| panic!("`{session}` never appeared in the roster within {timeout:?}"));
}

/// Get a fresh, live `input-required` session under way: wait for the
/// body's first presence (so `say` never races `unknown_session`), fire a
/// gated `say` in the background, and wait until the roster reports
/// `input-required`. Returns the background `say`'s join handle (still
/// running, parked on the gate) and the roster row observed at that moment.
fn gate_a_session(
    hub_state: &StateDir,
    session: &str,
    text: &str,
) -> (std::thread::JoinHandle<Output>, serde_json::Value) {
    wait_until_session_present(hub_state, session, Duration::from_secs(10));
    let handle = say_full_in_background(hub_state, vec![session.to_string(), text.to_string()]);
    let row = wait_for_roster_state(hub_state, session, "input-required", Duration::from_secs(10));
    (handle, row)
}

#[test]
fn answer_resolves_held_permission_and_turn_completes() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--ask-permission", "--chunks", "2"])]);

    let (handle, _row) = gate_a_session(&hub_state, "alpha", "please run the tool");

    let ans = support::answer(&hub_state, "alpha", "allow");
    assert!(ans.status.success(), "answer must exit 0; stderr: {}", stderr_of(&ans));

    let say_result = handle.join().expect("say thread");
    assert!(
        say_result.status.success(),
        "the gated turn must resume and complete after answer; stderr: {}",
        stderr_of(&say_result)
    );
    assert!(stdout_of(&say_result).contains("stub chunk"), "the reply must carry the stub's streamed text");
}

#[test]
fn answer_by_multi_field_comma_separated_choice_resolves_each_field() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--ask-elicitation", "--chunks", "2"])]);

    let (handle, row) = gate_a_session(&hub_state, "alpha", "please pick");
    let pending = row.get("pending").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    assert_eq!(pending.len(), 2, "the stub's elicitation form has two fields (color, size): {pending:?}");

    // One comma segment per field, in the block's own declared order (the
    // stub names its fields so BTreeMap order is `color`, then `size` — see
    // `acp_driver.rs`'s own module doc).
    let ans = support::answer(&hub_state, "alpha", "red,m");
    assert!(ans.status.success(), "multi-field answer must exit 0; stderr: {}", stderr_of(&ans));

    let say_result = handle.join().expect("say thread");
    assert!(say_result.status.success(), "the gated turn must complete; stderr: {}", stderr_of(&say_result));
}

#[test]
fn answer_with_nothing_pending_is_exit_1_nothing_pending() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--chunks", "1"])]);

    // The session is idle (never gated) — nothing to resolve.
    let warm = say_ready(&hub_state, "alpha", "hi", Duration::from_secs(10));
    assert!(warm.status.success(), "stderr: {}", stderr_of(&warm));

    let ans = support::answer(&hub_state, "alpha", "allow");
    assert_eq!(ans.status.code(), Some(1), "nothing-pending answer must exit 1; stderr: {}", stderr_of(&ans));
    let err = stderr_of(&ans);
    assert!(
        err.contains("no held permission or elicitation to answer"),
        "must carry the nothing_pending refusal's own wording: {err:?}"
    );
}

#[test]
fn plain_say_to_input_required_is_session_busy_with_answer_hint() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--ask-permission", "--chunks", "2"])]);

    let (handle, _row) = gate_a_session(&hub_state, "alpha", "please run the tool");

    let racer = support::say(&hub_state, "alpha", "interrupting");
    assert_eq!(racer.status.code(), Some(1), "say to an input-required session must exit 1; stderr: {}", stderr_of(&racer));
    let err = stderr_of(&racer);
    assert!(err.contains("session_busy"), "must carry the session_busy refusal: {err:?}");
    assert!(err.contains("input-required"), "must name the input-required state: {err:?}");
    assert!(err.contains("answer"), "must hint 'answer', not interrupt/--queue: {err:?}");

    // Clean up: resolve the still-held gate so the background thread ends.
    let _ = support::answer(&hub_state, "alpha", "allow");
    let _ = handle.join();
}

#[test]
fn queue_is_refused_on_input_required_distinct_from_working() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--ask-permission", "--chunks", "2"])]);

    let (handle, _row) = gate_a_session(&hub_state, "alpha", "please run the tool");

    // `--queue` must NOT override an input-required refusal (it does
    // override a plain `working` busy session — issue #150/#190's own
    // `say_queue_appends_and_runs_after_turn`).
    let queued = Command::new(support::holler_bin())
        .env("HOLLER_STATE_DIR", hub_state.path())
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json")
        .args(["say", "--queue", "alpha", "queued while gated"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `say --queue`");

    assert_eq!(
        queued.status.code(),
        Some(1),
        "--queue must not resolve an input-required refusal; stderr: {}",
        stderr_of(&queued)
    );
    assert!(stderr_of(&queued).contains("session_busy"), "still a session_busy refusal: {}", stderr_of(&queued));

    let _ = support::answer(&hub_state, "alpha", "allow");
    let _ = handle.join();
}

#[test]
fn roster_shows_pending_column_with_content_then_clears_it() {
    let hub_state = StateDir::new();
    let body_state = StateDir::new();
    let hub = Hub::start(&hub_state);
    let _body = start_body(&hub_state, &body_state, &hub, &[("alpha", &["--ask-permission", "--chunks", "2"])]);

    let (handle, row) = gate_a_session(&hub_state, "alpha", "please run the tool");

    let pending = row.get("pending").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    assert_eq!(pending.len(), 1, "the stub's permission gate is one field: {pending:?}");
    let item = &pending[0];
    assert_eq!(item.get("kind").and_then(|v| v.as_str()), Some("permission"));
    assert_eq!(item.get("prompt").and_then(|v| v.as_str()), Some("stub tool wants to run"));
    let options: Vec<String> = item
        .get("options")
        .and_then(|o| o.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    assert_eq!(options, vec!["Allow".to_string(), "Deny".to_string()]);

    let ans = support::answer(&hub_state, "alpha", "allow");
    assert!(ans.status.success(), "stderr: {}", stderr_of(&ans));
    let _ = handle.join();

    // Once answered (and the turn settles), the roster no longer carries a
    // pending block for this row.
    let after = wait_for(Duration::from_secs(10), || {
        let doc = support::roster_json(&hub_state);
        let rows = doc.get("rows")?.as_array()?.clone();
        let row = rows.into_iter().find(|r| row_name_matches(r, "alpha"))?;
        (row.get("pending").is_none() || row.get("pending") == Some(&serde_json::Value::Null)).then_some(row)
    });
    assert!(after.is_some(), "pending must clear once the session leaves input-required");
}
