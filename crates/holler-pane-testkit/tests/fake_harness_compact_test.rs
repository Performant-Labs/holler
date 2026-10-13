#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #878
//! `FakeHarness::compact` (#878, ADR-0021 "Compaction"), and the scenario methods that give it
//! something to do: `seed_context` (a session's context size, and what a compaction brings it
//! to), `set_compaction_pin` (the model the harness's own config pins, for a call that names
//! none), `set_activity` (what `session_activity` answers) and `compact_runs` (every compaction
//! that was sent to a model).
//!
//! The port's own rules that any harness keeps (an unserved port, an unknown id, a shrink or a
//! refusal) are the conformance suite's; this file pins what only the fake has: the check
//! order, the model that ran, a busy session, a refused call that changes nothing, the faults
//! and the log. The check order is the contract the OpenCode adapter follows (A2): endpoint,
//! the server is reached, the session is known, the session is idle, the model is resolved,
//! the context is read, then the act.

use holler_pane::ports::{Activity, CompactModel, CompactOutcome, COMPACT_MODEL_UNPINNED};
use holler_pane::{Argv, HarnessEndpoint, HarnessPort, PaneError, PaneName, ServeSpec};
use holler_pane_testkit::fault::{Fault, PortOp};
use holler_pane_testkit::harness::{CompactRun, FakeHarness, HarnessOp};

const P0: u16 = 48100;
const UNKNOWN: &str = "ses_zzzzzzzzzzzzzzzzzzzzzzzzzz";

fn at(port: u16) -> HarnessEndpoint {
    HarnessEndpoint::Port(port)
}

fn model(provider: &str, id: &str) -> CompactModel {
    CompactModel {
        provider: provider.to_owned(),
        model_id: id.to_owned(),
    }
}

/// The model the harness's own config pins.
fn pinned() -> CompactModel {
    model("pinned-provider", "pinned-model")
}

/// A running server on P0 with one session on it.
fn served_session() -> (FakeHarness, String) {
    let harness = FakeHarness::new();
    let argv: Argv = harness
        .serve_command(
            &PaneName::parse("demo-c1r1").unwrap(),
            P0,
            &ServeSpec::default(),
        )
        .unwrap();
    harness.start_server(&argv).unwrap().unwrap();
    let session = harness.create_session(&at(P0)).unwrap();
    (harness, session)
}

/// [`served_session`], with a context of 90 000 tokens that a compaction brings to 12 000, and
/// the pin.
fn ready() -> (FakeHarness, String) {
    let (harness, session) = served_session();
    harness.seed_context(P0, &session, 90_000, 12_000);
    harness.set_compaction_pin(P0, Some(pinned()));
    (harness, session)
}

fn refusal_code(result: Result<CompactOutcome, PaneError>) -> String {
    result.expect_err("the call is refused").code().to_owned()
}

#[test]
fn the_op_is_named_harness_dot_compact() {
    assert_eq!(HarnessOp::Compact.as_str(), "harness.compact");
}

/// The act: a compaction on the pin brings the seeded size to the seeded target, and the fake
/// records the one run with the model asked for (none) and the model it ran on (the pin).
#[test]
fn compact_without_a_model_runs_on_the_pin_and_shrinks_the_context() {
    let (harness, session) = ready();
    assert_eq!(
        harness.compact(&at(P0), &session, None),
        Ok(CompactOutcome {
            tokens_before: 90_000,
            tokens_after: 12_000
        })
    );
    assert_eq!(
        harness.compact_runs(),
        vec![CompactRun {
            port: P0,
            session,
            asked: None,
            ran_on: pinned(),
        }]
    );
}

/// A caller that names a model gets that model, whatever the pin says; with no pin at all the
/// named model is enough.
#[test]
fn an_explicit_model_runs_in_place_of_the_pin() {
    let (harness, session) = ready();
    let asked = model("asked-provider", "asked/model");
    assert!(harness.compact(&at(P0), &session, Some(&asked)).is_ok());
    let runs = harness.compact_runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].asked, Some(asked.clone()));
    assert_eq!(runs[0].ran_on, asked, "the explicit model, not the pin");

    let (harness, session) = served_session();
    harness.seed_context(P0, &session, 500, 100);
    assert!(harness.compact(&at(P0), &session, Some(&asked)).is_ok());
    assert_eq!(harness.compact_runs()[0].ran_on, asked, "no pin needed");
}

/// No model asked for and none pinned: `compact-model-unpinned`, and nothing was sent to a
/// model. The pin is per server: a pin on another port does not count.
#[test]
fn no_model_and_no_pin_is_compact_model_unpinned_and_runs_nothing() {
    let (harness, session) = served_session();
    harness.seed_context(P0, &session, 90_000, 12_000);
    harness.set_compaction_pin(P0 + 1, Some(pinned()));
    assert_eq!(
        refusal_code(harness.compact(&at(P0), &session, None)),
        COMPACT_MODEL_UNPINNED.as_str()
    );
    assert!(harness.compact_runs().is_empty(), "no summarize was sent");

    harness.set_compaction_pin(P0, Some(pinned()));
    harness.set_compaction_pin(P0, None);
    assert_eq!(
        refusal_code(harness.compact(&at(P0), &session, None)),
        COMPACT_MODEL_UNPINNED.as_str(),
        "a pin that was cleared is no pin"
    );
    assert!(harness.compact_runs().is_empty());
}

/// A session that is not idle is refused with the code of what it is doing, with the model and
/// the context in place, so only the activity can be what refuses; nothing was sent.
#[test]
fn a_session_that_is_not_idle_is_refused_and_runs_nothing() {
    for (activity, code) in [
        (Activity::Busy, "session-busy"),
        (Activity::HoldingQuestion, "session-holds-question"),
        (Activity::WaitingPermission, "session-holds-permission"),
    ] {
        let (harness, session) = ready();
        harness.set_activity(P0, &session, activity);
        assert_eq!(
            refusal_code(harness.compact(&at(P0), &session, None)),
            code,
            "{activity:?}"
        );
        assert!(harness.compact_runs().is_empty(), "{activity:?}");
    }
}

/// The activity is per session, and settable back: an idle session of the same server still
/// compacts, and the busy one compacts once it is idle again, from the size it had.
#[test]
fn activity_is_per_session_and_a_refused_call_changes_nothing() {
    let (harness, busy) = ready();
    let other = harness.create_session(&at(P0)).unwrap();
    harness.seed_context(P0, &other, 700, 70);
    harness.set_activity(P0, &busy, Activity::Busy);
    assert_eq!(
        harness.compact(&at(P0), &other, None),
        Ok(CompactOutcome {
            tokens_before: 700,
            tokens_after: 70
        }),
        "the other session is idle"
    );
    assert_eq!(
        refusal_code(harness.compact(&at(P0), &busy, None)),
        "session-busy"
    );
    harness.set_activity(P0, &busy, Activity::Idle);
    assert_eq!(
        harness.compact(&at(P0), &busy, None),
        Ok(CompactOutcome {
            tokens_before: 90_000,
            tokens_after: 12_000
        }),
        "the refused call did not touch the context"
    );
}

/// The `session_activity` default answers `Idle` for a session nobody set, and the fake's
/// answer does not enter the call log: the engines' call-sequence assertions (a `switch`'s op
/// list) hold with or without a seeded activity.
#[test]
fn session_activity_answers_the_seed_and_is_not_logged() {
    let (harness, session) = served_session();
    let before = harness.faults().calls();
    assert_eq!(
        harness.session_activity(&at(P0), &session),
        Ok(Activity::Idle)
    );
    harness.set_activity(P0, &session, Activity::Busy);
    assert_eq!(
        harness.session_activity(&at(P0), &session),
        Ok(Activity::Busy)
    );
    assert_eq!(harness.faults().calls(), before);
}

/// The check order, one pair at a time: each earlier refusal wins over the later one, so a
/// caller sees the first thing wrong.
#[test]
fn the_checks_run_in_order() {
    // 1. An unserved port is unavailable, before the session is looked at.
    let harness = FakeHarness::new();
    assert_eq!(
        refusal_code(harness.compact(&at(P0), UNKNOWN, None)),
        "unavailable"
    );

    // 2. An unknown session is session-not-found, before the pin.
    let (harness, _) = served_session();
    assert_eq!(
        refusal_code(harness.compact(&at(P0), UNKNOWN, None)),
        "session-not-found"
    );

    // 3. A busy session is session-busy, before the pin is looked at.
    let (harness, session) = served_session();
    harness.set_activity(P0, &session, Activity::Busy);
    assert_eq!(
        refusal_code(harness.compact(&at(P0), &session, None)),
        "session-busy"
    );

    // 4. No pin is compact-model-unpinned, before the context is read.
    let (harness, session) = served_session();
    assert_eq!(
        refusal_code(harness.compact(&at(P0), &session, None)),
        COMPACT_MODEL_UNPINNED.as_str()
    );

    // 5. A pin and no readable context is unavailable, never a zero.
    harness.set_compaction_pin(P0, Some(pinned()));
    assert_eq!(
        refusal_code(harness.compact(&at(P0), &session, None)),
        "unavailable"
    );
    assert!(harness.compact_runs().is_empty());
}

/// A frozen server times out with the method's op; a killed one is unavailable; a process
/// endpoint is not a server's.
#[test]
fn a_frozen_or_killed_server_and_a_process_endpoint_answer_as_every_method_does() {
    let (harness, session) = ready();
    harness.freeze(P0).unwrap();
    assert_eq!(
        harness.compact(&at(P0), &session, None),
        Err(PaneError::Timeout {
            op: "harness.compact".to_owned()
        })
    );
    harness.kill(P0).unwrap();
    assert_eq!(
        refusal_code(harness.compact(&at(P0), &session, None)),
        "unavailable"
    );
    assert!(harness.compact_runs().is_empty());

    let process = HarnessEndpoint::Process {
        pane: PaneName::parse("demo-c2r1").unwrap(),
        pid: 1,
    };
    assert_eq!(
        refusal_code(harness.compact(&process, &session, None)),
        "process-unsupported"
    );
}

/// The faults and the log, as for every method: a call is logged when it is made, a fault
/// fails it before anything runs, a wedged adapter times out, and the seeds are not calls.
#[test]
fn compact_passes_the_faults_and_the_seeds_bypass_the_log() {
    let (harness, session) = served_session();
    let before = harness.faults().calls();
    harness.seed_context(P0, &session, 90_000, 12_000);
    harness.set_compaction_pin(P0, Some(pinned()));
    harness.set_activity(P0, &session, Activity::Idle);
    assert_eq!(harness.faults().calls(), before, "seeding is not a call");
    assert!(harness.compact_runs().is_empty(), "nor is a run");

    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    harness
        .faults()
        .fail_next(HarnessOp::Compact, unavailable.clone());
    assert_eq!(harness.compact(&at(P0), &session, None), Err(unavailable));
    assert!(harness.compact_runs().is_empty(), "a fault runs nothing");
    assert!(harness.compact(&at(P0), &session, None).is_ok());

    harness.faults().set(Some(Fault::Wedged));
    assert_eq!(
        harness.compact(&at(P0), &session, None),
        Err(PaneError::Timeout {
            op: "harness.compact".to_owned()
        })
    );
    harness.faults().set(None);
    assert_eq!(
        harness.faults().calls()[before.len()..],
        [HarnessOp::Compact, HarnessOp::Compact, HarnessOp::Compact]
    );
    assert_eq!(harness.compact_runs().len(), 1, "only the middle call ran");
}
