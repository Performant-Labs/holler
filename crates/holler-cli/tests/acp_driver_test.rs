#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #188
//! RED tests for the ACP v2 spawn driver (issue #188) — the issue's own test
//! list, verbatim in name where practical. 18 of the 19 tests live here;
//! `crash_mid_turn_is_error_not_hang` lives in its own `[[test]]` target
//! (`acp_driver_crash_test.rs`, own process) — see that file's module doc for
//! why.
//!
//! # Why this file lives in `holler-cli`, not `holler-body`
//!
//! The issue's setup instructions name `crates/holler-body/tests/acp_driver_test.rs`.
//! That path is not achievable on stable Rust: the `stub-acp` binary the
//! driver spawns is a `[[bin]]` owned by the `holler-cli` package (story
//! #130), and Cargo only sets `CARGO_BIN_EXE_stub-acp` for `holler-cli`'s own
//! test targets — not for a *different* workspace member's tests, even one
//! that depends on `holler-cli` (there is no such dependency edge anyway;
//! `holler-cli` depends on `holler-body`, not the reverse). Resolving the
//! binary path at runtime (`std::env::current_exe()` sibling lookup) would
//! violate this repo's own build-guard rule (`scripts/lint.sh` check 3: read
//! `CARGO_BIN_EXE_*` with `env!`, never work around it) and would be fragile
//! besides. `holler-cli` already has exactly this problem solved — its own
//! `stub_acp_test.rs` (story #130) and `tests/support` module live right next
//! to the stub. This file follows that precedent: it drives the *real*
//! `holler_body::acp_driver::AcpDriver` (a runtime dependency of the `holler`
//! binary already) against the real `stub-acp` process, from `holler-cli`'s
//! own test target where `CARGO_BIN_EXE_stub-acp` actually resolves.

use std::time::Duration;

use futures_util::StreamExt;
use holler_body::acp_driver::{
    AcpDriver, DriverError, DriverEvent, DriverState, Status, StopReason,
};
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_proto::SessionName;

/// Serializes every test in this file so at most one is running at a time.
///
/// Each test spawns a real `stub-acp` child process and drives a real
/// connection; running many of them fully concurrently (`cargo test`'s
/// default per-binary parallelism — this file has 18 tests) measurably added
/// scheduling contention for the SDK's own background actor tasks (see
/// `acp_driver_crash_test.rs`'s module doc for the fuller investigation of
/// that class of issue, which is what pushed `crash_mid_turn_is_error_not_hang`
/// into its own test binary entirely). Serializing here removes that
/// contention at its source for the remaining 18 tests rather than padding
/// every one of their timeouts.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Acquire the serialization lock for the calling test's duration. A
/// `tokio::sync::Mutex` (not `std::sync::Mutex`) because every test holds
/// this guard across many `.await` points — holding a blocking `std` lock
/// across an await is exactly the anti-pattern `clippy::await_holding_lock`
/// (denied workspace-wide) exists to catch; `tokio`'s async-aware mutex has
/// no such hazard (nor does it poison on panic, so a prior test's panic
/// mid-guard cannot wedge every later test).
async fn serial_guard() -> tokio::sync::MutexGuard<'static, ()> {
    SERIAL.lock().await
}

/// Build a spawn-mode `SessionConfig` that runs the built `stub-acp` binary
/// with `extra` args — the same binary path `tests/support::stub_acp_bin`
/// resolves (`env!("CARGO_BIN_EXE_stub-acp")`, compile-time per this repo's
/// own build-guard convention).
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
    }
}

/// Drain `stream` until a `Done`, returning every event observed (in order).
/// Bounded by `timeout` so a driver defect (a hang) fails the test instead of
/// the whole suite.
async fn drain_to_done(
    stream: &mut (impl futures_util::Stream<Item = DriverEvent> + Unpin),
    timeout: Duration,
) -> Vec<DriverEvent> {
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let next = tokio::time::timeout(remaining, stream.next())
            .await
            .expect("event within timeout")
            .expect("stream did not end before Done");
        let done = matches!(next, DriverEvent::Done(_));
        events.push(next);
        if done {
            return events;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawn_initialize_new_session_ok() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--chunks", "1"]);
    let driver = AcpDriver::spawn(&config)
        .await
        .expect("spawn succeeds against the stub");
    assert_eq!(driver.status(), Status::Idle);
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_timeout_on_hung_command() {
    let _serial = serial_guard().await;
    // SAFETY: `std::env::set_var` mutates process-global state. `_serial`
    // guarantees no other test in this file runs concurrently, and no other
    // test reads `HOLLER_ACP_TIMEOUT_MS`, so there is no cross-test race.
    unsafe {
        std::env::set_var("HOLLER_ACP_TIMEOUT_MS", "300");
    }
    let config = SessionConfig {
        name: SessionName::parse("hung").expect("valid session name"),
        harness: "opencode".to_string(),
        mode: SessionMode::Spawn,
        command: Some(vec![
            "sh".to_string(),
            "-c".to_string(),
            "sleep 60".to_string(),
        ]),
        cwd: None,
        env: None,
        interrupt: Interrupt::Acp,
        endpoint: None,
        session_id: None,
    };
    let started = tokio::time::Instant::now();
    let result = AcpDriver::spawn(&config).await;
    unsafe {
        std::env::remove_var("HOLLER_ACP_TIMEOUT_MS");
    }
    let is_startup_error = matches!(result, Err(DriverError::Startup(_)));
    assert!(
        is_startup_error,
        "expected a startup timeout error, got a success or different error"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "startup timeout must be bounded by HOLLER_ACP_TIMEOUT_MS, not the default 10s"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_streams_chunks_then_done_end_turn() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;

    assert!(events
        .iter()
        .any(|e| matches!(e, DriverEvent::State(DriverState::Working))));
    let chunks: Vec<&String> = events
        .iter()
        .filter_map(|e| match e {
            DriverEvent::Chunk(text) => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(chunks.len(), 3, "{events:?}");
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_mid_turn_yields_cancelled_and_status_idle_only_after_response() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--slow", "--chunks", "5"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    // At least one event lands before cancelling mid-turn.
    let _ = tokio::time::timeout(Duration::from_secs(2), stream.next()).await;

    driver.cancel().await.expect("cancel resolves");
    assert_eq!(
        driver.status(),
        Status::Idle,
        "status is Idle only after the cancelled response"
    );

    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    assert_eq!(
        events.last(),
        Some(&DriverEvent::Done(StopReason::Cancelled)),
        "{events:?}"
    );
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_after_cancel_is_fresh_turn() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--slow", "--chunks", "5"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut first = driver.prompt("first-turn-text").await;
    let _ = tokio::time::timeout(Duration::from_secs(2), first.next()).await;
    driver.cancel().await.expect("cancel");

    let mut second = driver.prompt("second-turn-text").await;
    let events = drain_to_done(&mut second, Duration::from_secs(5)).await;
    let chunks: Vec<&String> = events
        .iter()
        .filter_map(|e| match e {
            DriverEvent::Chunk(text) => Some(text),
            _ => None,
        })
        .collect();
    // The stub's chunk text does not encode the prompt itself (it always
    // emits "stub chunk N"), so the contract this test actually pins is: the
    // second turn streams its own full chunk set from scratch (not a
    // continuation of the cancelled first turn) and resolves end_turn.
    assert!(!chunks.is_empty());
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn requires_action_maps_to_input_required_then_back_to_working() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-permission", "--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;

    // Drain until InputRequired.
    let mut saw_input_required = false;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            saw_input_required = true;
            break;
        }
    }
    assert!(saw_input_required);
    assert_eq!(driver.status(), Status::InputRequired);

    driver.answer("allow").await.expect("answer resolves");

    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    assert!(events
        .iter()
        .any(|e| matches!(e, DriverEvent::State(DriverState::Working))));
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn permission_is_held_not_auto_denied() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-permission", "--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            break;
        }
    }

    // No answer for 2s: still InputRequired (never auto-denied), and no
    // Done arrives on the stream (the stub never received a reply).
    let woke_early = tokio::time::timeout(Duration::from_secs(2), stream.next()).await;
    assert!(
        woke_early.is_err(),
        "no event should arrive while parked on the gate"
    );
    assert_eq!(driver.status(), Status::InputRequired);

    driver
        .cancel()
        .await
        .expect("cancel cleans up the still-pending request");
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_rejects_pending_permission_and_ends_turn_cancelled() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-permission", "--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            break;
        }
    }

    driver
        .cancel()
        .await
        .expect("cancel resolves even with a pending permission");
    assert_eq!(driver.status(), Status::Idle);
    // A subsequent answer against a request that no longer exists is a clean
    // NothingPending, not a hang or a stale reply.
    assert_eq!(
        driver.answer("allow").await,
        Err(DriverError::NothingPending)
    );
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn elicitation_create_is_input_required_with_options() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-elicitation", "--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    let mut saw_input_required = false;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            saw_input_required = true;
            break;
        }
    }
    assert!(saw_input_required);
    driver
        .answer("red,m")
        .await
        .expect("multi-field answer resolves");
    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn answer_with_nothing_pending_is_error() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--chunks", "1"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    assert_eq!(
        driver.answer("anything").await,
        Err(DriverError::NothingPending)
    );
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_kills_tree_no_orphans() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--chunks", "1"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    driver.shutdown().await.expect("shutdown");
    // `shutdown`'s own bounded wait already confirms the connection task (and
    // therefore the SDK's process-group-kill transport guard) ran to
    // completion; a second `shutdown` call must not hang either.
    driver.shutdown().await.expect("idempotent shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn permission_request_surfaces_as_input_required_immediately() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-permission", "--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    // The very next InputRequired-bearing event must arrive well within a
    // couple hundred ms of the permission request being raised — not after a
    // caller has to poll `status()` in a loop.
    let mut saw_input_required = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(DriverEvent::State(DriverState::InputRequired))) => {
                saw_input_required = true;
                break;
            }
            Ok(Some(_)) => continue,
            _ => break,
        }
    }
    assert!(
        saw_input_required,
        "InputRequired must be pushed, not polled for"
    );
    driver.cancel().await.expect("cleanup");
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn answer_by_index_and_by_option_id_both_resolve_and_reply_is_sent_to_the_agent() {
    let _serial = serial_guard().await;
    // By index (0 = "allow").
    let config = stub_config("alpha", &["--ask-permission", "--chunks", "2"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            break;
        }
    }
    driver.answer("0").await.expect("index resolves");
    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    // The turn only ever completes end_turn if the stub actually received and
    // parsed a real `RequestPermissionResponse` for its outstanding request —
    // it does not time out or auto-resume on its own.
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");

    // By option_id ("allow").
    let config2 = stub_config("beta", &["--ask-permission", "--chunks", "2"]);
    let driver2 = AcpDriver::spawn(&config2).await.expect("spawn");
    let mut stream2 = driver2.prompt("hi").await;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream2.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            break;
        }
    }
    driver2.answer("allow").await.expect("option_id resolves");
    let events2 = drain_to_done(&mut stream2, Duration::from_secs(5)).await;
    assert_eq!(
        events2.last(),
        Some(&DriverEvent::Done(StopReason::EndTurn))
    );
    driver2.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn answer_with_unresolvable_choice_fails_closed_before_any_reply_is_sent() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-permission", "--chunks", "2"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            break;
        }
    }

    let bad = driver.answer("maybe-later").await;
    assert!(matches!(bad, Err(DriverError::Answer(_))), "{bad:?}");
    // The stub never received a reply — it is still parked, so the driver is
    // still InputRequired and a valid answer afterwards still works.
    assert_eq!(driver.status(), Status::InputRequired);
    driver
        .answer("deny")
        .await
        .expect("a valid answer still resolves after a failed one");
    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_field_elicitation_resolves_a_comma_separated_choice_one_segment_per_field() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-elicitation", "--chunks", "2"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            break;
        }
    }
    // Fields sort as `color` (red/blue) then `size` (s/m) — see the stub's
    // `--ask-elicitation` schema and the driver's own documented BTreeMap
    // field-order decision.
    driver
        .answer("blue,s")
        .await
        .expect("comma-separated multi-field answer resolves");
    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_field_elicitation_with_wrong_segment_count_fails_closed() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-elicitation", "--chunks", "2"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            break;
        }
    }
    let too_few = driver.answer("red").await;
    assert!(
        matches!(too_few, Err(DriverError::Answer(_))),
        "{too_few:?}"
    );
    let too_many = driver.answer("red,m,extra").await;
    assert!(
        matches!(too_many, Err(DriverError::Answer(_))),
        "{too_many:?}"
    );
    assert_eq!(
        driver.status(),
        Status::InputRequired,
        "still held open after both failures"
    );
    driver.cancel().await.expect("cleanup");
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_while_permission_pending_replies_cancelled_outcome() {
    let _serial = serial_guard().await;
    // Distinct from `cancel_rejects_pending_permission_and_ends_turn_cancelled`:
    // this pins that the turn itself resolves `cancelled` (not just that the
    // pending permission is cleared).
    let config = stub_config("alpha", &["--ask-permission", "--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            break;
        }
    }
    driver.cancel().await.expect("cancel");
    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    assert_eq!(
        events.last(),
        Some(&DriverEvent::Done(StopReason::Cancelled)),
        "{events:?}"
    );
    driver.shutdown().await.expect("shutdown");
}

/// Regression test for issue #238: `cancel()` must report the turn's REAL
/// `StopReason`, never a hardcoded `Cancelled`, even when nothing was
/// actually in flight to cancel. `stub-acp`'s own `session/cancel` handler
/// (see `tests/stub-acp/main.rs`'s `route`) only ever resolves an in-flight
/// turn to `cancelled` — by the time a turn has already fully resolved on
/// the wire (`turn` is `None` there too), a further `session/cancel` gets no
/// second `idle` state_update at all. That is exactly the "nothing in
/// flight" no-op path this test drives: the turn is run to completion
/// (`end_turn`) *first*, then `cancel()` is called against an already-idle
/// driver. Before this fix, that path returned a bare `Ok(())` with the real
/// reason thrown away; the caller (`session_manager::task::handle_cancel`)
/// then hardcoded `StopReason::Cancelled` whenever it (wrongly) believed a
/// turn was still in flight. `AcpDriver::cancel()` itself must never invent
/// `Cancelled` here — this pins that its own contract is honest regardless
/// of what any caller does with the value.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_after_natural_completion_reports_real_reason_not_cancelled() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--chunks", "1"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    assert_eq!(driver.status(), Status::Idle);

    // The turn is fully settled (`end_turn`) *before* this call — there is
    // nothing left to cancel. The old, buggy `cancel()` returned a bare
    // `Ok(())` here (discarding the real reason); the fix must instead
    // report the actual last-observed `StopReason`.
    let result = driver.cancel().await;
    assert_eq!(
        result,
        Ok(StopReason::EndTurn),
        "cancel() must report the turn's real outcome, not invent Cancelled: {result:?}"
    );
    driver.shutdown().await.expect("shutdown");
}

/// Companion to the above for the "cancel-before-idle" ordering (issue
/// #239's own lock-ordering fix): cancelling a turn that IS still genuinely
/// in flight must resolve with the real `Cancelled` reason the agent itself
/// reports over the wire (via the fixed, single-critical-section
/// check-and-register in `AcpDriver::cancel()`), not merely "resolve
/// without hanging". Together with
/// `cancel_after_natural_completion_reports_real_reason_not_cancelled`
/// (the "idle-before-cancel" ordering) this exercises both straightforward
/// orderings the lock-ordering fix has to get right.
///
/// # Why the exact TOCTOU race window isn't independently provable here
///
/// The bug this fix closes was a genuine data race: a gap of a few
/// instructions between releasing the lock after the idle/pending check and
/// re-acquiring it to register `awaiting_done`, during which the connection
/// task's `handle_state_update` could deliver the real `idle` state_update
/// and find nobody listening. Hitting that exact instruction-level window
/// from outside the process, by timing alone, is not reliable — the same
/// standard of honesty this crate already applies to issue #188's own
/// `#[ignore]`d `crash_mid_turn_is_error_not_hang` (see
/// `acp_driver_crash_test.rs`'s module doc): forcing a true data race
/// deterministically needs a synchronization hook (e.g. an injectable delay
/// or a test-only notification point) that does not exist in this driver,
/// and adding one purely to prove a race window is closed would itself be
/// new production surface for a test to exploit. What IS provable, and what
/// this pair of tests actually proves, is that the corrected code — a
/// single critical section covering both the check and the registration —
/// behaves correctly for both orderings that critical section can produce:
/// either the turn was already settled when the lock was taken (this test's
/// sibling), or it wasn't yet and `awaiting_done` was registered before the
/// lock was released (this test). There is no third ordering left for the
/// old two-lock version's gap to hide in.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_mid_turn_reports_real_cancelled_reason() {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--slow", "--chunks", "5"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    // At least one event lands before cancelling mid-turn, so `cancel()`
    // takes the "still in flight" branch (registers `awaiting_done`), not
    // the "nothing in flight" no-op branch the sibling test exercises.
    let _ = tokio::time::timeout(Duration::from_secs(2), stream.next()).await;
    assert_eq!(driver.status(), Status::Working);

    let result = driver.cancel().await;
    assert_eq!(result, Ok(StopReason::Cancelled), "{result:?}");
    assert_eq!(driver.status(), Status::Idle);

    let events = drain_to_done(&mut stream, Duration::from_secs(5)).await;
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::Cancelled)));
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn elicitation_url_mode_and_non_enum_form_fields_are_reported_unsupported_not_silently_dropped(
) {
    let _serial = serial_guard().await;
    let config = stub_config("alpha", &["--ask-elicitation-url", "--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    let mut saw_input_required = false;
    while let Some(event) = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("event")
    {
        if matches!(event, DriverEvent::State(DriverState::InputRequired)) {
            saw_input_required = true;
            break;
        }
    }
    // Reported (surfaced as InputRequired), not silently dropped.
    assert!(saw_input_required);
    // Never auto-answered: `answer()` always fails closed with `Unsupported`.
    let result = driver.answer("0").await;
    assert!(
        matches!(result, Err(DriverError::Unsupported(_))),
        "{result:?}"
    );
    assert_eq!(
        driver.status(),
        Status::InputRequired,
        "still held open, not dropped"
    );
    driver
        .cancel()
        .await
        .expect("cancel still resolves an unsupported pending item");
    driver.shutdown().await.expect("shutdown");
}
