//! The engine `holler pane compact` runs, `holler_pane::tx_compact::compact(ports, &Pane)`
//! (#878, brief A2 and A3.4), called directly with a `Pane` and the doctor rig's fakes in
//! `Ports`. It lives beside the verb's tests because `holler-pane` has no test kit to build a
//! `Ports` from (the kit depends on `holler-pane`); every other engine's tests are here too.
//!
//! The engine owns, in this order: the endpoint of the record; a record with no session of
//! record is `session-not-found`; health (`server-unhealthy`); idle (`session-busy`,
//! `session-holds-question`, `session-holds-permission`); the act, called once with no model;
//! and the shrink rule (a context that did not shrink is `unavailable`, naming both counts). It
//! reads no record, applies no pane-state predicate (that is the CLI's `resolve_pane_record`,
//! `compact.rs`), writes no record and calls no Herdr or host port.

use holler_pane::pane::{Health, Hold};
use holler_pane::ports::{Activity, CompactOutcome, COMPACT_MODEL_UNPINNED};
use holler_pane::tx_compact::compact;
use holler_pane::{HarnessPort, Pane, PaneError};
use holler_pane_testkit::harness::{CompactRun, HarnessOp as H};
use holler_pane_testkit::pane_store::PaneStoreOp;

use super::{compactable, pinned};
use crate::doctor::rig::Rig;
use crate::switch::P;

/// The engine over the rig's ports, for the record the rig stores for `P`, with the calls it
/// made through every port.
fn run(rig: &Rig) -> (Result<CompactOutcome, PaneError>, crate::doctor::rig::Calls) {
    let record = rig.record(P);
    run_record(rig, &record)
}

fn run_record(
    rig: &Rig,
    record: &Pane,
) -> (Result<CompactOutcome, PaneError>, crate::doctor::rig::Calls) {
    let mark = rig.mark();
    let result = compact(rig.ports(), record);
    (result, rig.calls_since(&mark))
}

fn code(result: Result<CompactOutcome, PaneError>) -> String {
    result.expect_err("the engine refuses").code().to_owned()
}

/// The act is called once, with no model (the verb names none), the answer is the harness's
/// own outcome, and the run is the health check and the act, nothing else: no registry write,
/// no Herdr call, no host call.
#[test]
fn the_engine_compacts_once_naming_no_model() {
    let rig = compactable(90_000, 12_000);
    let live = rig.live(P).clone();
    let (result, calls) = run(&rig);

    assert_eq!(
        result,
        Ok(CompactOutcome {
            tokens_before: 90_000,
            tokens_after: 12_000
        })
    );
    assert_eq!(calls.harness, [H::Health, H::Compact]);
    assert_eq!(
        rig.harness.compact_runs(),
        vec![CompactRun {
            port: live.port,
            session: live.session,
            asked: None,
            ran_on: pinned(),
        }]
    );
    assert!(
        calls
            .panes
            .iter()
            .all(|op| !matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete)),
        "a compaction records nothing in the registry: {:?}",
        calls.panes
    );
    assert!(
        calls.herdr.is_empty() && calls.host.is_empty() && calls.hub.is_empty(),
        "no Herdr, host or hub call, so no keystroke (I4): {calls:?}"
    );
}

/// The health check comes first and stops everything: a killed or frozen server is
/// `server-unhealthy`, and the act is never called. A check that fails is passed on as it is.
#[test]
fn an_unhealthy_server_is_refused_before_anything_else() {
    for stop in [
        holler_pane_testkit::harness::FakeHarness::kill,
        holler_pane_testkit::harness::FakeHarness::freeze,
    ] {
        let rig = compactable(90_000, 12_000);
        stop(&rig.harness, rig.live(P).port).expect("stop the server");
        let (result, calls) = run(&rig);
        assert_eq!(code(result), "server-unhealthy");
        assert_eq!(calls.harness, [H::Health]);
    }

    let rig = compactable(90_000, 12_000);
    let timeout = PaneError::Timeout {
        op: "harness.health".to_owned(),
    };
    rig.harness.faults().fail_next(H::Health, timeout.clone());
    let (result, calls) = run(&rig);
    assert_eq!(
        result,
        Err(timeout),
        "a failing check is passed on as it is"
    );
    assert_eq!(calls.harness, [H::Health]);
}

/// A session that is running a turn, holds a question or waits on a permission is refused
/// with that state's code after the health check and before the act: no `compact` was logged,
/// and nothing was sent to a model.
#[test]
fn a_session_that_is_not_idle_is_refused_before_the_act() {
    for (activity, want) in [
        (Activity::Busy, "session-busy"),
        (Activity::HoldingQuestion, "session-holds-question"),
        (Activity::WaitingPermission, "session-holds-permission"),
    ] {
        let rig = compactable(90_000, 12_000);
        let live = rig.live(P).clone();
        rig.harness.set_activity(live.port, &live.session, activity);
        let (result, calls) = run(&rig);

        let error = result.expect_err("refused");
        assert_eq!(error.code(), want, "{activity:?}");
        assert!(
            error.to_string().contains(&live.session) && error.to_string().contains(P),
            "{activity:?}: names the session and the pane: {error}"
        );
        assert_eq!(
            calls.harness,
            [H::Health],
            "{activity:?}: no `compact` logged"
        );
        assert!(rig.harness.compact_runs().is_empty(), "{activity:?}");
    }
}

/// A context that did not shrink is `unavailable` (the I3 mismatch rule: a failure seen after
/// the act is the closed code, not an open one), the message says so and names both counts,
/// and the act did run once.
#[test]
fn a_context_that_did_not_shrink_is_unavailable_naming_both_counts() {
    for (before, after) in [(5_000_u64, 5_000_u64), (5_000, 6_001)] {
        let rig = compactable(before, after);
        let (result, calls) = run(&rig);

        let error = result.expect_err("no shrink is a failure");
        assert_eq!(error.code(), "unavailable", "{before}->{after}");
        let text = error.to_string();
        assert!(text.contains("did not shrink"), "{before}->{after}: {text}");
        assert!(
            text.contains(&before.to_string()) && text.contains(&after.to_string()),
            "{before}->{after}: names both counts: {text}"
        );
        assert_eq!(calls.harness, [H::Health, H::Compact]);
        assert_eq!(
            rig.harness.compact_runs().len(),
            1,
            "asked once, never retried"
        );
    }
}

/// What the harness refuses or fails with passes through the engine unchanged: the same
/// error, not one re-worded or re-coded.
#[test]
fn a_harness_refusal_or_failure_passes_through_unchanged() {
    // An open code the harness owns: no pin.
    let rig = compactable(90_000, 12_000);
    let live = rig.live(P).clone();
    rig.harness.set_compaction_pin(live.port, None);
    let (result, _) = run(&rig);
    let direct = rig.harness.compact(
        &holler_pane::HarnessEndpoint::Port(live.port),
        &live.session,
        None,
    );
    assert_eq!(
        result.clone().expect_err("refused").code(),
        COMPACT_MODEL_UNPINNED.as_str()
    );
    assert_eq!(result, direct, "the engine adds and removes nothing");

    // A failure of the act itself: a timeout, as a 120 s bound ends one.
    let rig = compactable(90_000, 12_000);
    let timeout = PaneError::Timeout {
        op: "harness.compact".to_owned(),
    };
    rig.harness.faults().fail_next(H::Compact, timeout.clone());
    let (result, calls) = run(&rig);
    assert_eq!(result, Err(timeout));
    assert_eq!(calls.harness, [H::Health, H::Compact]);
    assert!(rig.harness.compact_runs().is_empty());
}

/// A record with no session of record has nothing to compact: `session-not-found`, and no
/// port was called at all (the CLI refuses it first; this is the engine's defence).
#[test]
fn a_record_without_a_session_of_record_is_session_not_found_and_calls_nothing() {
    let rig = compactable(90_000, 12_000);
    let mut record = rig.record(P);
    record.session_of_record = None;
    let (result, calls) = run_record(&rig, &record);
    assert_eq!(code(result), "session-not-found");
    assert!(calls.harness.is_empty(), "{calls:?}");
}

/// The engine applies no pane-state predicate: a record that is parked or says its harness is
/// unhealthy is compacted all the same, because `pane-parked` and `pane-unhealthy` are the
/// CLI's (`resolve_pane_record`, the `interrupt` precedent) and the engine must not grow a
/// second copy of them. The verb's tests (`compact.rs`) pin that the verb refuses both.
#[test]
fn the_engine_applies_no_pane_state_predicate() {
    let rig = compactable(90_000, 12_000);
    let mut record = rig.record(P);
    record.hold = Hold::Parked {
        reason: "disk full".to_owned(),
        release_when: "after the cleanup".to_owned(),
        since: 1,
    };
    record.harness.health = Health::Unhealthy("no answer".to_owned());
    let (result, calls) = run_record(&rig, &record);
    assert_eq!(
        result,
        Ok(CompactOutcome {
            tokens_before: 90_000,
            tokens_after: 12_000
        })
    );
    assert_eq!(calls.harness, [H::Health, H::Compact]);
}
