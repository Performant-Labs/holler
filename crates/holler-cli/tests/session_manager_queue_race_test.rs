#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #189
//! One regression test, split out of `session_manager_test.rs` purely to
//! keep that file under the repo's 900-line file-size gate
//! (`scripts/lint.sh`) — this is not a new test area, it is
//! `queue_dispatch_never_publishes_transient_idle`, which needs its own
//! small slice of that file's own fixtures (`stub_config`/`sn`/`registry_of`/
//! `state_of`/`wait_for_state`/`assert_result_ok`), duplicated here rather
//! than factored into a shared module for one test — see that file's own
//! module doc for why these tests live in `holler-cli` at all (the
//! `stub-acp` `CARGO_BIN_EXE_stub-acp` resolution wall).
//!
//! # What this test guards against
//!
//! Investigated 2026-09-21 against the flaky
//! `queued_prompt_runs_after_drop_and_reply_is_discarded` test in
//! `reconnect_contract_test.rs`, which failed three times on a loaded
//! self-hosted CI runner with `session_busy: alpha is working (turn 0s, last
//! update 0s ago)` on a "fresh" `say` sent only *after* the roster had
//! already reported the session `idle`.
//!
//! Root cause: `holler-body`'s `session_manager::task::finish_turn` used to
//! unconditionally publish an `Idle` presence-changed notice the instant the
//! *current* turn ended — even when a `--queue`d prompt was already sitting
//! in the FIFO and about to start immediately after
//! (`dispatch_next_queued` → `start_turn`, the very next lines). The
//! connection loop (`holler-body`'s `connection.rs`) forwards every
//! presence-changed notice straight onto the wire as a `session/presence`
//! update, which the hub caches into both its roster (what `holler roster`/
//! `wait_for` reads) and its own fast-path busy check
//! (`holler-hub`'s `talk::say`, which refuses `-32009 session_busy` *before*
//! ever forwarding to the body when its cached state is `working`). Because
//! both caches update from that exact same presence notice, a caller polling
//! the roster and then immediately `say`-ing again can genuinely observe
//! "idle" and have the hub forward the prompt — only for the body's *own*
//! `SessionManager` to correctly refuse it moments later, because the queued
//! turn had, by then, actually started (`turn 0s ago`). The gap between the
//! transient `Idle` publish and the queued turn's own `Working` publish is
//! normally sub-millisecond (one `.await` inside `start_turn`), but that
//! await — spawning a fresh driver, or just the first round trip to an
//! already-spawned one — can stretch arbitrarily far under real CPU/
//! scheduling pressure (the exact "busy machine" scenario CI hit), which is
//! why this was reproducible on a loaded runner and not a local dev box: a
//! genuine, load-widened race in the product's own presence bookkeeping, not
//! test impatience.
//!
//! Fixed in `session_manager/task.rs`: `finish_turn_no_dispatch` now takes a
//! `publish_idle` flag, and `finish_turn` passes `false` whenever its own
//! FIFO queue is non-empty (mirroring the same fix in `handle_replace`,
//! which has an identical shape — a replacement turn always starts right
//! after the cancelled one is torn down). The session is never externally
//! observable as `Idle` unless it genuinely has nothing queued next.

use std::time::Duration;

use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_body::registry::SessionRegistry;
use holler_body::session_manager::{PromptOutcome, SessionManager, SessionManagerError};
use holler_proto::{Presence, SessionAd, SessionName, SessionState};

/// Build a spawn-mode `SessionConfig` running the built `stub-acp` binary —
/// mirrors `session_manager_test.rs::stub_config` exactly.
fn stub_config(name: &str, extra: &[&str]) -> SessionConfig {
    let mut command = vec![env!("CARGO_BIN_EXE_stub-acp").to_string()];
    command.extend(extra.iter().map(|s| s.to_string()));
    SessionConfig {
        name: SessionName::parse(name).expect("valid session name"),
        harness: "opencode".to_string(),
        mode: SessionMode::Spawn,
        command: Some(command),
        cwd: None,
        env: None,
        interrupt: Interrupt::Acp,
        endpoint: None,
        session_id: None,
        auth_method: None,
    }
}

fn sn(name: &str) -> SessionName {
    SessionName::parse(name).expect("valid session name")
}

fn registry_of(sessions: &[(&str, &[&str])]) -> SessionRegistry {
    let configs = sessions.iter().map(|(n, args)| stub_config(n, args)).collect();
    SessionRegistry::from_sessions(configs)
}

fn find(doc: &Presence, name: &str) -> SessionAd {
    doc.sessions
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no {name} in presence doc: {doc:?}"))
        .clone()
}

async fn state_of(manager: &SessionManager, name: &SessionName) -> SessionState {
    find(&manager.presence_doc("h".to_string()), name.as_str()).state
}

/// Poll `presence_doc()` until `name` reaches `want`, or panic after
/// `timeout` — mirrors `session_manager_test.rs::wait_for_state` exactly.
async fn wait_for_state(manager: &SessionManager, name: &SessionName, want: SessionState, timeout: Duration) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if state_of(manager, name).await == want {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timed out waiting for {name:?} to reach {want:?}");
        }
        tokio::time::sleep(Duration::from_millis(15)).await;
    }
}

/// Assert a `prompt`/`replace` outcome is a completed `Result` with the given
/// turn id, ACP `stop_reason`, and A2A terminal state — mirrors
/// `session_manager_test.rs::assert_result_ok` exactly.
fn assert_result_ok(
    res: &Result<PromptOutcome, SessionManagerError>,
    turn_id: &str,
    stop_reason: &str,
    state: SessionState,
) {
    match res {
        Ok(PromptOutcome::Result { turn_id: got_id, stop_reason: got_reason, state: got_state }) => {
            assert_eq!(got_id, turn_id, "{res:?}");
            assert_eq!(got_reason, stop_reason, "{res:?}");
            assert_eq!(*got_state, state, "{res:?}");
        }
        other => panic!("expected a completed Result outcome, got {other:?}"),
    }
}

/// Regression test for the `queued_prompt_runs_after_drop_and_reply_is_
/// discarded` reconnect-contract flake — see this file's own module doc for
/// the full root-cause writeup. Asserts the session is observably `Idle` on
/// the presence-changed stream **exactly once** across two chained turns
/// (a running turn plus one `--queue`d behind it) — only once the queue is
/// truly empty, never as a blip in between.
#[tokio::test]
async fn queue_dispatch_never_publishes_transient_idle() {
    let manager = SessionManager::start(&registry_of(&[("alpha", &["--slow", "--chunks", "3"])]));
    let alpha = sn("alpha");
    let mut changes = manager.subscribe_presence_changes();

    let first = manager.prompt(&alpha, "a1", "hi", false);
    tokio::pin!(first);
    tokio::select! {
        res = &mut first => panic!("finished early: {res:?}"),
        () = wait_for_state(&manager, &alpha, SessionState::Working, Duration::from_secs(2)) => {}
    }

    let queued = manager.prompt(&alpha, "a2", "hi again", true);
    tokio::pin!(queued);
    tokio::select! {
        res = &mut queued => panic!("queued prompt resolved too early: {res:?}"),
        () = tokio::time::sleep(Duration::from_millis(50)) => {}
    }

    let idle_observations = std::sync::atomic::AtomicUsize::new(0);
    let collector = async {
        while let Ok(_changed) = changes.recv().await {
            if state_of(&manager, &alpha).await == SessionState::Idle {
                idle_observations.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    };
    tokio::pin!(collector);

    let both_turns = async {
        let (first_res, queued_res) = tokio::join!(&mut first, &mut queued);
        assert_result_ok(&first_res, "a1", "end_turn", SessionState::Completed);
        assert_result_ok(&queued_res, "a2", "end_turn", SessionState::Completed);
    };
    tokio::pin!(both_turns);

    tokio::select! {
        _ = &mut collector => {}
        () = both_turns => {}
    }
    // Let the final, legitimate `Idle` presence-changed notice (published the
    // instant both turns are truly done, before either reply resolves) land
    // before checking the count.
    tokio::select! {
        _ = &mut collector => {}
        () = tokio::time::sleep(Duration::from_millis(200)) => {}
    }

    assert_eq!(
        idle_observations.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "session must be observably `Idle` exactly once — only once the queue is truly \
         empty — never as a transient blip while the queued turn is about to run"
    );
}
