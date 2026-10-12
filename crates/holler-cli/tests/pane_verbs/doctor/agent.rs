//! `agent-cannot-dispatch` (#647 part 2, ADR-0021 section 12, brief D6 to D8): an OpenCode
//! pane whose record names no agent, whose server is healthy and lists the session of record,
//! whose recent turns (the last three user turns of the session of record) all ran as an agent
//! that cannot dispatch, while the project ships an orchestrator (an agent that can). The rule
//! reads `HarnessPort::agent_view` once rules (1) to (3) hold, and only then.
//!
//! The world is the doctor rig; the fake harness's `seed_turns` and `seed_dispatchers` say what
//! the server would report. A rig pane records no agent (`sample_pane`), so rules (1) to (3)
//! hold for every healthy rig pane, and an unseeded view is empty: no finding.

use clap::Parser;
use holler_cli::pane::PaneCmd;
use holler_cli::{Cli, Command};
use holler_pane::findings::{FindingKind as K, FixState};
use holler_pane::{AgentKey, PaneError, TurnAgent};
use holler_pane_testkit::harness::HarnessOp;

use super::rig::{keys, of_kind, one, whole, Rig, Seed};
use super::{one_pane, PANE};

fn key(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap()
}

fn turn(can_dispatch: Option<bool>) -> TurnAgent {
    TurnAgent::new(Some(key("build")), can_dispatch)
}

/// What the server of `pane` reports: `turns` for its session of record (oldest first), and
/// `dispatchers` as the agents that can dispatch.
fn seed(rig: &Rig, pane: &str, turns: Vec<TurnAgent>, dispatchers: &[&str]) {
    let live = rig.live(pane);
    rig.harness.seed_turns(live.port, &live.session, turns);
    rig.harness
        .seed_dispatchers(live.port, dispatchers.iter().map(|d| key(d)).collect());
}

/// One pane whose recent turns all ran as `build`, which cannot dispatch, in a project that
/// ships the one orchestrator `orchestrator`. An older turn could dispatch: only the newest
/// three count.
fn cannot_dispatch() -> Rig {
    let rig = one_pane();
    let turns = vec![
        turn(Some(true)),
        turn(Some(false)),
        turn(Some(false)),
        turn(Some(false)),
    ];
    seed(&rig, PANE, turns, &["orchestrator"]);
    rig
}

/// Whether the pass made since `mark` asked any server for an agent view.
fn asked_agent_view(rig: &Rig, mark: &super::rig::Calls) -> bool {
    rig.calls_since(mark)
        .harness
        .contains(&HarnessOp::AgentView)
}

// ---- the finding ----

#[test]
fn every_recent_turn_unable_to_dispatch_raises_one_finding_naming_the_relaunch() {
    let rig = cannot_dispatch();
    let live = rig.live(PANE).clone();

    let report = rig.run(&whole(false));

    let finding = one(&report, K::AgentCannotDispatch, PANE);
    assert_eq!(report.findings.len(), 1, "{:?}", keys(&report));
    assert_eq!(finding.session.as_deref(), Some(live.session.as_str()));
    assert_eq!(finding.ports, vec![live.port]);
    assert_eq!(finding.fix, FixState::NotFixable);
    assert_eq!(
        finding.remedy.as_deref(),
        Some("holler pane relaunch demo-c1r1 --agent=orchestrator")
    );
}

/// `--fix` is accepted, as for every kind, and repairs nothing of this one: never a relaunch,
/// never a switch, never a change of the pane's agent.
#[test]
fn fix_changes_nothing_for_it() {
    let rig = cannot_dispatch();
    rig.run(&whole(false));
    let before = rig.record(PANE);
    let mark = rig.mark();

    let report = rig.run(&whole(true));

    let finding = one(&report, K::AgentCannotDispatch, PANE);
    assert_eq!(finding.fix, FixState::NotFixable);
    let after = rig.record(PANE);
    assert_eq!(
        after, before,
        "the record, its agent included, is unchanged"
    );
    assert_eq!(after.opencode_agent, None);
    let calls = rig.calls_since(&mark);
    for writer in [HarnessOp::ServeCommand, HarnessOp::SelectSession] {
        assert!(
            !calls.harness.contains(&writer),
            "{writer:?}: {:?}",
            calls.harness
        );
    }
}

/// The remedy names the key joined to its flag, and a key that starts with `-` still parses,
/// through the real clap tree, as `relaunch`'s agent (brief D8, acceptance 4).
#[test]
fn a_dispatcher_whose_key_starts_with_a_dash_still_parses_as_the_agent() {
    let rig = one_pane();
    seed(&rig, PANE, vec![turn(Some(false)); 3], &["-x"]);

    let report = rig.run(&whole(false));

    let remedy = one(&report, K::AgentCannotDispatch, PANE)
        .remedy
        .clone()
        .expect("one dispatcher: a remedy");
    assert_eq!(remedy, "holler pane relaunch demo-c1r1 --agent=-x");
    let cli = Cli::try_parse_from(remedy.split(' '))
        .unwrap_or_else(|e| panic!("{remedy:?} must parse: {e}"));
    match cli.command {
        Command::Pane(PaneCmd::Relaunch(relaunch)) => {
            assert_eq!(relaunch.pane, PANE);
            assert_eq!(relaunch.spec.agent.as_deref(), Some("-x"));
        }
        other => panic!("{remedy:?} is not a relaunch: {other:?}"),
    }
}

/// With several dispatchers no single command is right: no remedy, and the message lists them
/// quoted, up to three, for the operator to pass one.
#[test]
fn several_dispatchers_give_no_remedy_and_a_message_that_lists_them() {
    let rig = one_pane();
    seed(
        &rig,
        PANE,
        vec![turn(Some(false)); 3],
        &["reviewer", "orchestrator"],
    );

    let report = rig.run(&whole(false));

    let finding = one(&report, K::AgentCannotDispatch, PANE);
    assert_eq!(finding.remedy, None, "{finding:?}");
    for name in ["\"orchestrator\"", "\"reviewer\""] {
        assert!(
            finding.message.contains(name),
            "{name}: {:?}",
            finding.message
        );
    }

    let many = one_pane();
    let four = ["alpha", "beta", "delta", "gamma"];
    seed(&many, PANE, vec![turn(Some(false)); 3], &four);
    let report = many.run(&whole(false));
    let message = &one(&report, K::AgentCannotDispatch, PANE).message;
    let listed = four
        .iter()
        .filter(|name| message.contains(&format!("\"{name}\"")))
        .count();
    assert_eq!(listed, 3, "up to three of four: {message:?}");
}

/// A pane in a profile: following the remedy without `--profile` would trade this finding for
/// `profile-drift`, so the message says the relaunch needs the pane's profile (plan review W-3).
/// The remedy itself is the table's.
#[test]
fn a_profile_members_message_names_its_profile() {
    let rig = Rig::new(&[Seed::new(PANE, 1, 1).in_profile("blue")]);
    seed(&rig, PANE, vec![turn(Some(false)); 3], &["orchestrator"]);

    let report = rig.run(&whole(false));

    let finding = one(&report, K::AgentCannotDispatch, PANE);
    assert!(
        finding.message.contains("--profile"),
        "{:?}",
        finding.message
    );
    assert!(finding.message.contains("blue"), "{:?}", finding.message);
    assert_eq!(
        finding.remedy.as_deref(),
        Some("holler pane relaunch demo-c1r1 --agent=orchestrator")
    );
}

// ---- what stops the rule ----

/// A record that names an agent chose it: nothing is raised, and the view is never asked for,
/// so such a pane costs no extra call.
fn assert_a_chosen_agent_is_left_alone(chosen: &str) {
    let rig = cannot_dispatch();
    rig.rewrite(PANE, |pane| pane.opencode_agent = Some(key(chosen)));
    let mark = rig.mark();

    let report = rig.run(&whole(false));

    assert!(report.findings.is_empty(), "{chosen}: {:?}", keys(&report));
    assert!(
        !asked_agent_view(&rig, &mark),
        "{chosen}: the view was asked for"
    );
}

/// pfleet#372's acceptance: a pane running `build` deliberately raises nothing.
#[test]
fn a_record_that_names_build_raises_nothing_and_asks_nothing() {
    assert_a_chosen_agent_is_left_alone("build");
}

/// A pane whose record names a dispatching agent opted in.
#[test]
fn a_record_that_names_the_orchestrator_raises_nothing_and_asks_nothing() {
    assert_a_chosen_agent_is_left_alone("orchestrator");
}

/// No orchestrator shipped: nothing to switch to, so nothing is raised.
#[test]
fn no_dispatcher_raises_nothing() {
    let rig = one_pane();
    seed(&rig, PANE, vec![turn(Some(false)); 3], &[]);
    let report = rig.run(&whole(false));
    assert!(report.findings.is_empty(), "{:?}", keys(&report));
}

/// Every one of the three recent turns must be unable to dispatch: the oldest of the three
/// answering `odd` stops the rule.
fn assert_one_odd_turn_stops_the_rule(odd: Option<bool>) {
    let rig = one_pane();
    let turns = vec![turn(odd), turn(Some(false)), turn(Some(false))];
    seed(&rig, PANE, turns, &["orchestrator"]);
    let report = rig.run(&whole(false));
    assert!(report.findings.is_empty(), "{odd:?}: {:?}", keys(&report));
}

#[test]
fn one_recent_turn_that_could_dispatch_raises_nothing() {
    assert_one_odd_turn_stops_the_rule(Some(true));
}

#[test]
fn one_recent_turn_the_server_says_nothing_about_raises_nothing() {
    assert_one_odd_turn_stops_the_rule(None);
}

/// An empty view (no turn: the server cannot tell) raises nothing, whatever the dispatchers.
#[test]
fn an_empty_view_raises_nothing() {
    let rig = one_pane();
    seed(&rig, PANE, Vec::new(), &["orchestrator"]);
    let report = rig.run(&whole(false));
    assert!(report.findings.is_empty(), "{:?}", keys(&report));
}

/// A failed view is `observe-failed` naming `harness.agent_view`, and nothing is concluded.
#[test]
fn a_failed_view_is_observe_failed() {
    let rig = cannot_dispatch();
    rig.harness.faults().fail_next(
        HarnessOp::AgentView,
        PaneError::Unavailable {
            what: "GET /agent answered 500".to_owned(),
        },
    );
    let report = rig.run(&whole(false));
    let failed = one(&report, K::ObserveFailed, PANE);
    assert!(
        failed.message.contains("harness.agent_view"),
        "{:?}",
        failed.message
    );
    assert!(
        of_kind(&report, K::AgentCannotDispatch).is_empty(),
        "{:?}",
        keys(&report)
    );
}

/// The trust rule (D6 rule 3): the view is asked for only when the server is healthy and its
/// trusted list holds the session of record. After `breaks`, the pass keeps its own finding
/// `own`, as on `main`, never asks for the view and raises no `agent-cannot-dispatch`.
fn assert_never_asked(what: &str, breaks: impl FnOnce(&Rig), own: &str) {
    let rig = cannot_dispatch();
    breaks(&rig);
    let mark = rig.mark();

    let report = rig.run(&whole(false));

    assert!(
        !asked_agent_view(&rig, &mark),
        "{what}: the view was asked for"
    );
    assert!(
        of_kind(&report, K::AgentCannotDispatch).is_empty(),
        "{what}: {:?}",
        keys(&report)
    );
    assert!(
        report.findings.iter().any(|f| f.kind.code() == own),
        "{what}: its own {own}: {:?}",
        keys(&report)
    );
}

/// A made-up read failure for a fault.
fn made_up() -> PaneError {
    PaneError::Unavailable {
        what: "made-up".to_owned(),
    }
}

#[test]
fn a_down_server_is_never_asked() {
    let down = |rig: &Rig| rig.harness.kill(rig.live(PANE).port).unwrap();
    assert_never_asked("a down server", down, "server-down");
}

/// `Server::Unknown`: the health check itself failed.
#[test]
fn a_server_whose_health_check_fails_is_never_asked() {
    let unknown = |rig: &Rig| rig.harness.faults().fail_next(HarnessOp::Health, made_up());
    assert_never_asked("a health check that fails", unknown, "observe-failed");
}

/// `Server::Healthy { sessions: None }`: healthy, and its list failed.
#[test]
fn a_healthy_server_whose_list_fails_is_never_asked() {
    let unlisted = |rig: &Rig| {
        rig.harness
            .faults()
            .fail_next(HarnessOp::ListSessions, made_up())
    };
    assert_never_asked(
        "a healthy server whose list fails",
        unlisted,
        "observe-failed",
    );
}

#[test]
fn a_server_without_the_session_of_record_is_never_asked() {
    let deleted = |rig: &Rig| rig.harness.delete_session(&rig.live(PANE).session).unwrap();
    assert_never_asked(
        "a server without the session of record",
        deleted,
        "session-of-record-missing",
    );
}

#[test]
fn a_record_without_a_session_is_never_asked() {
    let none = |rig: &Rig| rig.rewrite(PANE, |pane| pane.session_of_record = None);
    assert_never_asked("a record without a session", none, "no-session-of-record");
}

// ---- once per pass ----

/// A pass reports every current finding exactly once and keeps no memory between passes:
/// a second pass over the same world reports the same two findings of part 2 once each, and
/// neither rule writes the record.
#[test]
fn a_second_pass_reports_the_same_two_findings_once_each_and_writes_nothing() {
    let rig = cannot_dispatch();
    rig.host
        .detach_clients(&rig.live(PANE).name)
        .expect("the session exists");
    let first = rig.run(&whole(false));
    let stored = rig.record(PANE);

    let second = rig.run(&whole(false));

    for report in [&first, &second] {
        one(report, K::Unattached, PANE);
        one(report, K::AgentCannotDispatch, PANE);
        assert_eq!(report.findings.len(), 2, "{:?}", keys(report));
    }
    assert_eq!(keys(&first), keys(&second));
    assert_eq!(rig.record(PANE), stored, "the second pass wrote nothing");
}
