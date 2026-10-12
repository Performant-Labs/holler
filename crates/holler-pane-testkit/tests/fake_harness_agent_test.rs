#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
//! `FakeHarness::agent_view` (#647 part 2, ADR-0021 section 12) and the two scenario methods
//! that give it something to report, `seed_turns` and `seed_dispatchers`. The port's own rules
//! that a real server also keeps (a fresh session has no turn, an unknown id, an unserved port)
//! are the conformance suite's case 17; this file pins what only the fake has: what was seeded,
//! the newest `max_turns` of it, the check order shared with `abort`, the faults and the log.

use holler_pane::{AgentKey, AgentView, HarnessEndpoint, HarnessPort, PaneError, TurnAgent};
use holler_pane::{Argv, PaneName, ServeSpec};
use holler_pane_testkit::fault::{Fault, PortOp};
use holler_pane_testkit::harness::{FakeHarness, HarnessOp};

const P0: u16 = 48100;
const P1: u16 = 48101;
const UNKNOWN: &str = "ses_zzzzzzzzzzzzzzzzzzzzzzzzzz";

fn at(port: u16) -> HarnessEndpoint {
    HarnessEndpoint::Port(port)
}

fn agent(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap()
}

fn turn(key: Option<&str>, can_dispatch: Option<bool>) -> TurnAgent {
    TurnAgent::new(key.map(agent), can_dispatch)
}

/// Start the server of `owner` on `port` through the fake's own argv, as a host's `run` would.
fn serve(harness: &FakeHarness, owner: &str, port: u16) {
    let argv: Argv = harness
        .serve_command(
            &PaneName::parse(owner).unwrap(),
            port,
            &ServeSpec::default(),
        )
        .unwrap();
    harness.start_server(&argv).unwrap().unwrap();
}

/// A running server on P0 with one session on it: the harness and the session.
fn served_session() -> (FakeHarness, String) {
    let harness = FakeHarness::new();
    serve(&harness, "demo-c1r1", P0);
    let session = harness.create_session(&at(P0)).unwrap();
    (harness, session)
}

#[test]
fn the_op_is_named_harness_dot_agent_view() {
    assert_eq!(HarnessOp::AgentView.as_str(), "harness.agent_view");
}

/// The view holds the newest `max_turns` of the turns seeded for the session, oldest first,
/// and the dispatchers seeded for the port, sorted by key without duplicates.
#[test]
fn agent_view_answers_the_newest_seeded_turns_and_the_dispatchers() {
    let (harness, a) = served_session();
    harness.seed_turns(
        P0,
        &a,
        vec![
            turn(Some("plan"), Some(true)),
            turn(Some("build"), Some(false)),
            turn(None, None),
            turn(Some("orchestrator"), Some(true)),
        ],
    );
    harness.seed_dispatchers(
        P0,
        vec![agent("reviewer"), agent("orchestrator"), agent("reviewer")],
    );

    let view = harness.agent_view(&at(P0), &a, 3).unwrap();
    assert_eq!(
        view.turns,
        vec![
            turn(Some("build"), Some(false)),
            turn(None, None),
            turn(Some("orchestrator"), Some(true)),
        ],
        "the newest three, oldest first"
    );
    assert_eq!(
        view.dispatchers,
        vec![agent("orchestrator"), agent("reviewer")]
    );

    let all = harness.agent_view(&at(P0), &a, 10).unwrap();
    assert_eq!(
        all.turns.len(),
        4,
        "fewer turns than asked for: all of them"
    );
    let none = harness.agent_view(&at(P0), &a, 0).unwrap();
    assert_eq!(none.turns, vec![], "max_turns 0 asks for no turn");
    assert_eq!(none.dispatchers, view.dispatchers);
}

/// Turns belong to their session and dispatchers to their server: a session with nothing
/// seeded on a running server with nothing seeded answers the empty view.
#[test]
fn an_unseeded_session_on_a_running_server_answers_the_empty_view() {
    let (harness, a) = served_session();
    let b = harness.create_session(&at(P0)).unwrap();
    serve(&harness, "demo-c2r1", P1);
    harness.seed_turns(P0, &a, vec![turn(Some("build"), Some(false))]);
    harness.seed_dispatchers(P0, vec![agent("orchestrator")]);

    let of_b = harness.agent_view(&at(P0), &b, 3).unwrap();
    assert_eq!(of_b.turns, vec![], "another session's turns are not b's");
    // The servers share one data directory, so P1 knows `a`, but not P0's agents.
    let on_p1 = harness.agent_view(&at(P1), &a, 3).unwrap();
    assert_eq!(on_p1.dispatchers, vec![], "another server's dispatchers");

    let fresh = FakeHarness::new();
    serve(&fresh, "demo-c1r1", P0);
    let c = fresh.create_session(&at(P0)).unwrap();
    assert_eq!(fresh.agent_view(&at(P0), &c, 3), Ok(AgentView::default()));
}

/// In `abort`'s check order: the port's server first (frozen is `timeout` with the method's
/// op, killed or never started is `unavailable`), then the session (`session-not-found`), and
/// only then what was seeded.
#[test]
fn agent_view_reaches_the_server_before_it_looks_at_the_session() {
    let (harness, a) = served_session();
    harness.seed_turns(P0, &a, vec![turn(Some("build"), Some(false))]);
    assert_eq!(
        harness.agent_view(&at(P0), UNKNOWN, 3),
        Err(PaneError::SessionNotFound {
            what: UNKNOWN.to_owned()
        })
    );

    harness.freeze(P0).unwrap();
    let timeout = Err(PaneError::Timeout {
        op: "harness.agent_view".to_owned(),
    });
    assert_eq!(harness.agent_view(&at(P0), &a, 3), timeout);
    assert_eq!(harness.agent_view(&at(P0), UNKNOWN, 3), timeout);

    harness.kill(P0).unwrap();
    let unreachable = |port: u16| {
        Err(PaneError::Unavailable {
            what: format!("the harness server on port {port}"),
        })
    };
    assert_eq!(harness.agent_view(&at(P0), &a, 3), unreachable(P0));
    assert_eq!(
        harness.agent_view(&at(P1), &a, 3),
        unreachable(P1),
        "a port never served"
    );
}

/// A process endpoint is the refusal `process-unsupported`, the one conversion a server
/// harness makes.
#[test]
fn a_process_endpoint_is_process_unsupported() {
    let (harness, a) = served_session();
    let process = HarnessEndpoint::Process {
        pane: PaneName::parse("demo-c1r1").unwrap(),
        pid: 4242,
    };
    match harness.agent_view(&process, &a, 3) {
        Err(PaneError::Refused { code, .. }) => assert_eq!(code.as_str(), "process-unsupported"),
        other => panic!("expected process-unsupported, got {other:?}"),
    }
}

/// The faults come first: a wedged adapter times out, a queued failure fails one call, and
/// each call is logged. The scenario methods are not calls through the port.
#[test]
fn agent_view_passes_the_faults_and_the_seeds_bypass_the_log() {
    let (harness, a) = served_session();
    let before = harness.faults().calls();
    harness.seed_turns(P0, &a, vec![turn(Some("build"), Some(false))]);
    harness.seed_dispatchers(P0, vec![agent("orchestrator")]);
    assert_eq!(harness.faults().calls(), before, "seeding is not a call");

    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    harness
        .faults()
        .fail_next(HarnessOp::AgentView, unavailable.clone());
    assert_eq!(harness.agent_view(&at(P0), &a, 3), Err(unavailable));
    assert_eq!(harness.agent_view(&at(P0), &a, 3).unwrap().turns.len(), 1);

    harness.faults().set(Some(Fault::Wedged));
    assert_eq!(
        harness.agent_view(&at(P0), &a, 3),
        Err(PaneError::Timeout {
            op: "harness.agent_view".to_owned()
        })
    );
    harness.faults().set(None);
    assert_eq!(
        harness.faults().calls()[before.len()..],
        [
            HarnessOp::AgentView,
            HarnessOp::AgentView,
            HarnessOp::AgentView
        ]
    );
}
