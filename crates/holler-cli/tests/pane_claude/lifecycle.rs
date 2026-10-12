//! What a Claude Code pane does under the verbs that act on a live pane (#831, ADR-0021 section 15
//! items 6 and 9; plan review W1, W4 and the operator's abort decision): `relaunch`, `switch`,
//! `reset` and `interrupt` are relaunches of the one process (stop, run, locate, record), never a
//! keypress; `switch` and `reset` refuse a pane that is not idle; `park` and `close` act as for
//! any pane.

use holler_cli::answer_cmd::answer_pane_target;
use holler_cli::interrupt_cmd::{interrupt_pane, PaneInterrupt};
use holler_cli::output::Format;
use holler_pane::error::class_of;
use holler_pane::pane::{HarnessKind, Hold};
use holler_pane::{HarnessPort, PaneError};
use holler_pane_testkit::herdr::HerdrOp;
use holler_pane_testkit::host::HostOp;

use crate::rig::{cell, data_of, error_of, has_pair, Calls, Rig, CLAUDE, OPENCODE, PORT};

/// A rig with one live, idle Claude pane at `r1c2` whose session has a conversation.
fn one_claude_pane() -> Rig {
    let rig = Rig::new();
    rig.seed_claude(CLAUDE, cell(1, 2));
    rig.converse(CLAUDE);
    rig
}

/// The run stopped the pane's process and ran a new one, in that order, and typed nothing.
fn assert_relaunched(calls: &Calls) {
    let stop = calls.host.iter().position(|op| *op == HostOp::StopOwned);
    let run = calls.host.iter().rposition(|op| *op == HostOp::Run);
    assert!(
        matches!((stop, run), (Some(s), Some(r)) if s < r),
        "stop_owned, then run: {calls:?}"
    );
    assert!(
        !calls
            .herdr
            .iter()
            .any(|op| matches!(op, HerdrOp::SendText | HerdrOp::SendKeys)),
        "no keystroke (I4): {calls:?}"
    );
}

/// The pane's process, record and host are exactly as before: nothing was stopped or run.
fn assert_untouched(rig: &Rig, before: &holler_pane::Pane, pid: u32, calls: &Calls) {
    assert_eq!(&rig.record(CLAUDE), before, "the record is unchanged");
    assert_eq!(
        rig.process_of(CLAUDE).map(|p| p.pid),
        Some(pid),
        "the same process runs"
    );
    assert!(
        !calls
            .host
            .iter()
            .any(|op| matches!(op, HostOp::StopOwned | HostOp::Run)),
        "nothing stopped or started: {calls:?}"
    );
}

// --- relaunch ---------------------------------------------------------------------------

#[test]
fn relaunch_keeps_the_session_of_record_and_records_the_new_pid() {
    let rig = one_claude_pane();
    let before = rig.record(CLAUDE);
    let (session, old_pid) = (
        before.session_of_record.clone().unwrap(),
        before.harness.pid.unwrap(),
    );

    let (run, calls) = rig.run_logged(&["pane", "relaunch", CLAUDE], Format::Json);

    data_of(&run);
    assert_relaunched(&calls);
    let after = rig.record(CLAUDE);
    let process = rig.process_of(CLAUDE).expect("a process runs");
    assert_ne!(process.pid, old_pid, "a new process");
    assert!(
        rig.claude().process(old_pid).is_none(),
        "the old one was stopped"
    );
    assert_eq!(process.session_id, session, "the session is kept");
    assert_eq!(after.session_of_record.as_deref(), Some(session.as_str()));
    assert_eq!(after.harness.pid, Some(process.pid));
    assert_eq!(after.harness.kind, HarnessKind::Claude);
    let last = rig.runs_of(CLAUDE).pop().expect("the host ran the process");
    assert!(
        has_pair(&last, "--resume", &session),
        "a conversation is resumed: {last:?}"
    );
}

#[test]
fn relaunch_naming_another_harness_is_harness_change_refused() {
    let rig = one_claude_pane();
    let before = rig.record(CLAUDE);
    let pid = before.harness.pid.unwrap();
    let (run, calls) = rig.run_logged(
        &[
            "pane",
            "relaunch",
            CLAUDE,
            "--harness",
            "opencode",
            "--port-policy",
            "fixed:48101",
        ],
        Format::Json,
    );
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "harness-change-refused");
    assert_untouched(&rig, &before, pid, &calls);
}

// --- switch (W1) ------------------------------------------------------------------------

#[test]
fn switch_relaunches_on_a_session_that_has_a_conversation() {
    let rig = one_claude_pane();
    let target = rig.claude().new_session_id().unwrap();
    rig.claude().seed_conversation(&target);
    let old_pid = rig.record(CLAUDE).harness.pid.unwrap();

    let (run, calls) = rig.run_logged(&["pane", "switch", CLAUDE, &target], Format::Json);

    assert_eq!(run.code, 0, "{run:?}");
    assert_relaunched(&calls);
    let process = rig.process_of(CLAUDE).expect("a process runs");
    assert_ne!(process.pid, old_pid);
    assert_eq!(process.session_id, target);
    let after = rig.record(CLAUDE);
    assert_eq!(after.session_of_record.as_deref(), Some(target.as_str()));
    assert_eq!(after.last_observed.shown.as_deref(), Some(target.as_str()));
    assert_eq!(after.harness.pid, Some(process.pid));
    let last = rig.runs_of(CLAUDE).pop().unwrap();
    assert!(has_pair(&last, "--resume", &target), "{last:?}");
}

/// W1: a switch to a conversation that exists nowhere is `session-not-found`, as for an OpenCode
/// pane, and nothing is stopped: a typo never starts and records an empty conversation.
#[test]
fn switch_to_an_unknown_session_is_session_not_found_and_touches_nothing() {
    let rig = one_claude_pane();
    let before = rig.record(CLAUDE);
    let pid = before.harness.pid.unwrap();
    let unknown = "4f1c2b8e-0000-4000-8000-0000000000ff";

    let (run, calls) = rig.run_logged(&["pane", "switch", CLAUDE, unknown], Format::Json);

    let (code, _) = error_of(&run);
    assert_eq!(code, "session-not-found", "{run:?}");
    assert_eq!(run.code, class_of(&code).exit_code());
    assert_untouched(&rig, &before, pid, &calls);
    assert!(
        !rig.claude().has_conversation(unknown),
        "no conversation was started"
    );
}

#[test]
fn switch_refuses_a_busy_pane_and_one_waiting_on_a_permission() {
    for hold in ["busy", "permission"] {
        let rig = one_claude_pane();
        let target = rig.claude().new_session_id().unwrap();
        rig.claude().seed_conversation(&target);
        let before = rig.record(CLAUDE);
        let pid = before.harness.pid.unwrap();
        match hold {
            "busy" => rig.claude().stall(pid).unwrap(),
            _ => rig.claude().hold_permission(pid).unwrap(),
        }

        let (run, calls) = rig.run_logged(&["pane", "switch", CLAUDE, &target], Format::Json);

        assert_eq!(run.code, 3, "{hold}: {run:?}");
        let want = match hold {
            "busy" => "session-busy",
            _ => "session-holds-permission",
        };
        assert_eq!(error_of(&run).0, want, "{hold}: {run:?}");
        assert_untouched(&rig, &before, pid, &calls);
    }
}

// --- reset (W4) -------------------------------------------------------------------------

#[test]
fn reset_relaunches_on_a_fresh_session() {
    let rig = one_claude_pane();
    let before = rig.record(CLAUDE);
    let old = before.session_of_record.clone().unwrap();

    let (run, calls) = rig.run_logged(&["pane", "reset", CLAUDE], Format::Json);

    assert_eq!(run.code, 0, "{run:?}");
    assert_relaunched(&calls);
    let after = rig.record(CLAUDE);
    let fresh = after.session_of_record.clone().unwrap();
    assert_ne!(fresh, old, "a fresh session");
    let process = rig.process_of(CLAUDE).unwrap();
    assert_eq!(process.session_id, fresh);
    assert_eq!(after.harness.pid, Some(process.pid));
    let last = rig.runs_of(CLAUDE).pop().unwrap();
    assert!(has_pair(&last, "--session-id", &fresh), "{last:?}");
}

/// W4: the first message goes to the process the reset started, never to the stopped one (the
/// endpoint of the record the relaunch stored, not of the record the plan read).
#[test]
fn reset_first_reaches_the_new_process() {
    let rig = one_claude_pane();
    let old_pid = rig.record(CLAUDE).harness.pid.unwrap();

    let run = rig.run(
        &["pane", "reset", CLAUDE, "--first", "start on the review"],
        Format::Json,
    );

    assert_eq!(run.code, 0, "{run:?}");
    let process = rig.process_of(CLAUDE).unwrap();
    assert_ne!(process.pid, old_pid);
    assert_eq!(process.inbox, vec!["start on the review"]);
}

#[test]
fn reset_refuses_a_busy_pane() {
    let rig = one_claude_pane();
    let before = rig.record(CLAUDE);
    let pid = before.harness.pid.unwrap();
    rig.claude().stall(pid).unwrap();

    let (run, calls) = rig.run_logged(&["pane", "reset", CLAUDE], Format::Json);

    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "session-busy");
    assert_untouched(&rig, &before, pid, &calls);
}

// --- interrupt and answer (item 9, the operator's abort decision) --------------------------

/// The interrupt of a Claude pane is a relaunch on its session of record: the turn in flight is
/// lost, the session is kept, and nothing is typed.
#[test]
fn interrupt_relaunches_a_busy_pane_on_its_session_of_record() {
    let rig = one_claude_pane();
    let before = rig.record(CLAUDE);
    let (session, old_pid) = (
        before.session_of_record.clone().unwrap(),
        before.harness.pid.unwrap(),
    );
    rig.claude().stall(old_pid).unwrap();
    let mark = rig.mark();

    let answer = rig.with_ports(|ports| interrupt_pane(ports, CLAUDE, None, None));

    assert_eq!(
        answer.unwrap(),
        PaneInterrupt::Relaunched {
            session: session.clone()
        }
    );
    let calls = rig.since(&mark);
    assert_relaunched(&calls);
    let process = rig.process_of(CLAUDE).unwrap();
    assert_ne!(process.pid, old_pid, "the busy process was stopped");
    assert_eq!(process.session_id, session, "the session is unchanged");
    let after = rig.record(CLAUDE);
    assert_eq!(after.session_of_record.as_deref(), Some(session.as_str()));
    assert_eq!(after.harness.pid, Some(process.pid));
}

#[test]
fn interrupt_with_text_on_a_claude_pane_is_refused_and_touches_nothing() {
    let rig = one_claude_pane();
    let before = rig.record(CLAUDE);
    let pid = before.harness.pid.unwrap();
    let mark = rig.mark();

    let answer =
        rig.with_ports(|ports| interrupt_pane(ports, CLAUDE, None, Some("do this instead")));

    assert_eq!(answer.unwrap_err().code(), "interrupt-text-unsupported");
    assert_untouched(&rig, &before, pid, &rig.since(&mark));
}

#[test]
fn interrupt_of_a_parked_claude_pane_is_refused_by_the_pane_gate() {
    let rig = one_claude_pane();
    let parked = rig.run(
        &[
            "pane",
            "park",
            CLAUDE,
            "--reason",
            "maintenance",
            "--release-when",
            "tomorrow",
        ],
        Format::Json,
    );
    assert_eq!(parked.code, 0, "{parked:?}");
    let before = rig.record(CLAUDE);
    let pid = before.harness.pid.unwrap();
    let mark = rig.mark();

    let answer = rig.with_ports(|ports| interrupt_pane(ports, CLAUDE, None, None));

    assert_eq!(answer.unwrap_err().code(), "pane-parked");
    assert_untouched(&rig, &before, pid, &rig.since(&mark));
}

#[test]
fn interrupt_of_an_opencode_pane_still_goes_to_the_hub() {
    let rig = Rig::new();
    let record = rig.seed_opencode(OPENCODE, cell(1, 1), PORT);
    let mark = rig.mark();

    let answer = rig.with_ports(|ports| interrupt_pane(ports, OPENCODE, None, None));

    assert_eq!(
        answer.unwrap(),
        PaneInterrupt::Hub {
            session: record.session_of_record.unwrap()
        }
    );
    let calls = rig.since(&mark);
    assert!(
        calls.host.is_empty() && calls.opencode.is_empty(),
        "nothing live: {calls:?}"
    );
}

#[test]
fn answer_on_a_claude_pane_is_answer_unsupported_and_on_opencode_routes() {
    let rig = Rig::new();
    rig.seed_claude(CLAUDE, cell(1, 2));
    let opencode = rig.seed_opencode(OPENCODE, cell(1, 1), PORT);
    let refused: Result<String, PaneError> =
        rig.with_ports(|ports| answer_pane_target(ports, CLAUDE, None));
    assert_eq!(refused.unwrap_err().code(), "answer-unsupported");
    let routed = rig.with_ports(|ports| answer_pane_target(ports, OPENCODE, None));
    assert_eq!(routed.unwrap(), opencode.session_of_record.unwrap());
}

/// No port call aborts a Claude turn in place: `abort` is refused, as the operator decided.
#[test]
fn abort_of_a_claude_process_is_abort_unsupported() {
    let rig = one_claude_pane();
    let record = rig.record(CLAUDE);
    let refused = rig
        .harnesses
        .abort(
            &rig.endpoint_of(CLAUDE),
            record.session_of_record.as_deref().unwrap(),
        )
        .unwrap_err();
    assert_eq!(refused.code(), "abort-unsupported");
    assert!(rig.process_of(CLAUDE).is_some(), "the process runs on");
}

// --- park and close ---------------------------------------------------------------------

#[test]
fn park_holds_a_claude_pane_and_leaves_its_process_alone() {
    let rig = one_claude_pane();
    let pid = rig.record(CLAUDE).harness.pid.unwrap();
    let (run, calls) = rig.run_logged(
        &[
            "pane",
            "park",
            CLAUDE,
            "--reason",
            "review",
            "--release-when",
            "after lunch",
        ],
        Format::Json,
    );
    assert_eq!(run.code, 0, "{run:?}");
    assert!(matches!(rig.record(CLAUDE).hold, Hold::Parked { .. }));
    assert_eq!(rig.process_of(CLAUDE).map(|p| p.pid), Some(pid));
    assert!(
        calls.host.is_empty() && calls.claude.is_empty(),
        "{calls:?}"
    );
}

#[test]
fn close_stops_the_process_and_deletes_the_record() {
    let rig = one_claude_pane();
    let run = rig.run(&["pane", "close", CLAUDE], Format::Json);
    assert_eq!(run.code, 0, "{run:?}");
    assert!(rig.process_of(CLAUDE).is_none(), "the process was stopped");
    assert!(rig.maybe_record(CLAUDE).is_none(), "the record is gone");
}
