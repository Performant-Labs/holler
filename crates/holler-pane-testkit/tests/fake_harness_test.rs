#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #684
//! `FakeHarness`'s own mechanisms: fault injection, the call log, the scenarios the
//! port cannot cause (a frozen or killed server, a session deleted under a TUI, a
//! person moving the TUI, a TUI that closes), the two quirks of raw OpenCode, and
//! separate data directories. The port contract is the conformance suite's job
//! (#638, slice e).
//!
//! #802: `serve_command` is the port method (pure: it builds an argv and starts nothing),
//! and `start_server` is the scenario method that starts a server from such an argv, the
//! way a host's `run` would. Setup here starts a server with [`serve`], which does both.

use holler_pane::{AgentKey, Argv, HarnessPort, PaneError, PaneId, PaneName, ServeSpec};
use holler_pane_testkit::fault::{Fault, PortOp};
use holler_pane_testkit::harness::{
    FakeHarness, HarnessOp, Quirk, ServerState, ServerView, TuiView,
};

const P0: u16 = 48100;
const P1: u16 = 48101;
const UNKNOWN: &str = "ses_zzzzzzzzzzzzzzzzzzzzzzzzzz";

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn pane0() -> PaneId {
    PaneId::new("scratch:demo-c1r1")
}

fn pane1() -> PaneId {
    PaneId::new("scratch:demo-c2r1")
}

fn pane2() -> PaneId {
    PaneId::new("scratch:demo-c3r1")
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}

fn unreachable(port: u16) -> PaneError {
    PaneError::Unavailable {
        what: format!("the harness server on port {port}"),
    }
}

fn no_tui(pane: &PaneId) -> PaneError {
    PaneError::Unavailable {
        what: format!("no TUI in pane {}", pane.as_str()),
    }
}

fn not_found(session: &str) -> PaneError {
    PaneError::SessionNotFound {
        what: session.to_owned(),
    }
}

fn view(owner: &str, pid: u32, state: ServerState) -> Option<ServerView> {
    Some(ServerView {
        name: name(owner),
        pid,
        state,
        agent: None,
    })
}

fn agent(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap()
}

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

/// The argv `serve_command` builds for `owner` on `port` with `spec` (through the port, so
/// logged as `ServeCommand`).
fn command(harness: &FakeHarness, owner: &str, port: u16, spec: &ServeSpec) -> Argv {
    harness.serve_command(&name(owner), port, spec).unwrap()
}

/// Start the server of `owner` on `port` with no agent: `serve_command`, then the scenario
/// method on its argv. Its pid, or the fake's refusal.
fn serve(harness: &FakeHarness, owner: &str, port: u16) -> Result<u32, PaneError> {
    let argv = command(harness, owner, port, &ServeSpec::default());
    harness
        .start_server(&argv)
        .expect("start_server knows the argv serve_command built")
}

/// Servers `demo-c1r1` on P0 and `demo-c2r1` on P1; sessions `a` and `b` on P0; the TUI
/// of pane0 attached to `a` on P0. Returns the harness, the pid of the P0 server, `a`
/// and `b`.
fn scene() -> (FakeHarness, u32, String, String) {
    let harness = FakeHarness::new();
    let pid = serve(&harness, "demo-c1r1", P0).unwrap();
    serve(&harness, "demo-c2r1", P1).unwrap();
    let a = harness.create_session(P0).unwrap();
    let b = harness.create_session(P0).unwrap();
    harness.attach_tui(&pane0(), P0, &a).unwrap();
    (harness, pid, a, b)
}

fn shown(harness: &FakeHarness, pane: &PaneId) -> Option<String> {
    harness.shown_session(pane).unwrap()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

// --- wedged adapter ---

#[test]
fn a_wedged_harness_times_out_every_method() {
    let (harness, _, a, _) = scene();
    let c = name("demo-c1r1");
    harness.faults().set(Some(Fault::Wedged));

    let spec = ServeSpec::default();
    assert_eq!(
        harness.serve_command(&c, P0, &spec),
        Err(timeout("harness.serve_command"))
    );
    assert_eq!(
        harness.verify_serving(P0, &spec),
        Err(timeout("harness.verify_serving"))
    );
    assert_eq!(harness.health(P0), Err(timeout("harness.health")));
    assert_eq!(
        harness.create_session(P0),
        Err(timeout("harness.create_session"))
    );
    assert_eq!(
        harness.list_sessions(P0),
        Err(timeout("harness.list_sessions"))
    );
    assert_eq!(harness.abort(P0, &a), Err(timeout("harness.abort")));
    assert_eq!(
        harness.attach_tui(&pane1(), P0, &a),
        Err(timeout("harness.attach_tui"))
    );
    assert_eq!(
        harness.select_session(&pane0(), &a),
        Err(timeout("harness.select_session"))
    );
    assert_eq!(
        harness.shown_session(&pane0()),
        Err(timeout("harness.shown_session"))
    );

    harness.faults().set(None);
    assert_eq!(
        harness.tui(&pane1()),
        None,
        "the wedged attach_tui attached nothing"
    );
    assert_eq!(harness.aborts(), vec![], "the wedged abort aborted nothing");
    assert_eq!(harness.list_sessions(P0).unwrap().len(), 2);
    assert_eq!(shown(&harness, &pane0()), Some(a));
}

#[test]
fn harness_op_names_are_port_dot_method() {
    let names = [
        (HarnessOp::ServeCommand, "harness.serve_command"),
        (HarnessOp::VerifyServing, "harness.verify_serving"),
        (HarnessOp::Health, "harness.health"),
        (HarnessOp::CreateSession, "harness.create_session"),
        (HarnessOp::ListSessions, "harness.list_sessions"),
        (HarnessOp::Abort, "harness.abort"),
        (HarnessOp::AttachTui, "harness.attach_tui"),
        (HarnessOp::SelectSession, "harness.select_session"),
        (HarnessOp::ShownSession, "harness.shown_session"),
    ];
    for (op, text) in names {
        assert_eq!(op.as_str(), text);
    }
}

#[test]
fn calls_are_recorded_in_order() {
    let harness = FakeHarness::new();
    serve(&harness, "demo-c1r1", P0).unwrap();
    harness.health(P0).unwrap();
    harness.verify_serving(P0, &ServeSpec::default()).unwrap();
    let a = harness.create_session(P0).unwrap();
    let b = harness.create_session(P0).unwrap();
    harness.list_sessions(P0).unwrap();
    harness.attach_tui(&pane0(), P0, &a).unwrap();
    harness.select_session(&pane0(), &b).unwrap();
    harness.shown_session(&pane0()).unwrap();
    harness.abort(P0, &b).unwrap();
    harness.abort(P0, UNKNOWN).unwrap_err();
    let expected = vec![
        HarnessOp::ServeCommand,
        HarnessOp::Health,
        HarnessOp::VerifyServing,
        HarnessOp::CreateSession,
        HarnessOp::CreateSession,
        HarnessOp::ListSessions,
        HarnessOp::AttachTui,
        HarnessOp::SelectSession,
        HarnessOp::ShownSession,
        HarnessOp::Abort,
        HarnessOp::Abort,
    ];
    assert_eq!(harness.faults().calls(), expected);

    // Configuration, scenario controls and inspection are not calls through the port.
    harness.set_quirk(Quirk::AbortUnknownAcked, false);
    harness.set_data_dir(P1, "default");
    harness.seed_session(P0);
    harness.freeze(P0).unwrap();
    harness.thaw(P0).unwrap();
    harness.navigate(&pane0(), None).unwrap();
    harness.delete_session(&a).unwrap();
    harness.close_tui(&pane0()).unwrap();
    harness.kill(P0).unwrap();
    harness.start_server(&argv(&["sleep", "30"]));
    harness.server(P0);
    harness.tui(&pane0());
    harness.aborts();
    harness.serve_specs();
    assert_eq!(harness.faults().calls(), expected);
}

#[test]
fn session_ids_are_ses_prefixed_and_distinct() {
    let harness = FakeHarness::new();
    serve(&harness, "demo-c1r1", P0).unwrap();
    serve(&harness, "demo-c2r1", P1).unwrap();
    let ids = vec![
        harness.create_session(P0).unwrap(),
        harness.create_session(P0).unwrap(),
        harness.create_session(P1).unwrap(),
        harness.seed_session(P0),
    ];
    for id in &ids {
        assert!(id.starts_with("ses_"), "{id}");
        assert_eq!(id.len(), 30, "{id}");
    }
    let mut unique = sorted(ids.clone());
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "no two ids are equal: {ids:?}");
}

// --- a frozen server ---

#[test]
fn a_frozen_server_answers_health_false_and_times_out_its_calls() {
    let (harness, pid, a, b) = scene();
    harness.freeze(P0).unwrap();

    assert_eq!(
        harness.server(P0),
        view("demo-c1r1", pid, ServerState::Frozen)
    );
    assert_eq!(harness.health(P0), Ok(false));
    let again = command(&harness, "demo-c1r1", P0, &ServeSpec::default());
    assert!(
        matches!(
            harness.start_server(&again),
            Some(Err(PaneError::Timeout { .. }))
        ),
        "a frozen server holds its port"
    );
    assert_eq!(
        harness.verify_serving(P0, &ServeSpec::default()),
        Err(timeout("harness.verify_serving"))
    );
    assert_eq!(
        harness.create_session(P0),
        Err(timeout("harness.create_session"))
    );
    assert_eq!(
        harness.list_sessions(P0),
        Err(timeout("harness.list_sessions"))
    );
    assert_eq!(harness.abort(P0, &a), Err(timeout("harness.abort")));
    assert_eq!(
        harness.attach_tui(&pane1(), P0, &a),
        Err(timeout("harness.attach_tui"))
    );
    assert_eq!(
        harness.select_session(&pane0(), &b),
        Err(timeout("harness.select_session"))
    );

    // The TUI keeps its screen, and the adapter that reads it still answers.
    assert_eq!(shown(&harness, &pane0()), Some(a));
    // A server on another port is unaffected.
    assert_eq!(harness.health(P1), Ok(true));
    harness.create_session(P1).unwrap();
    assert_eq!(harness.aborts(), vec![], "a frozen server aborted nothing");
}

#[test]
fn thaw_brings_a_frozen_server_back() {
    let (harness, pid, a, b) = scene();
    harness.freeze(P0).unwrap();
    harness.thaw(P0).unwrap();

    assert_eq!(
        harness.server(P0),
        view("demo-c1r1", pid, ServerState::Running)
    );
    assert_eq!(
        shown(&harness, &pane0()),
        Some(a.clone()),
        "the TUI still shows a"
    );
    assert_eq!(harness.health(P0), Ok(true));
    assert_eq!(
        sorted(harness.list_sessions(P0).unwrap()),
        sorted(vec![a.clone(), b.clone()])
    );
    harness.create_session(P0).unwrap();
    harness.abort(P0, &a).unwrap();
    harness.select_session(&pane0(), &b).unwrap();
    assert_eq!(shown(&harness, &pane0()), Some(b));
}

// --- a killed server ---

#[test]
fn a_killed_server_answers_health_false_and_is_unavailable() {
    let (harness, pid, a, b) = scene();
    harness.kill(P0).unwrap();

    assert_eq!(
        harness.server(P0),
        view("demo-c1r1", pid, ServerState::Killed)
    );
    assert_eq!(harness.health(P0), Ok(false));
    assert_eq!(harness.create_session(P0), Err(unreachable(P0)));
    assert_eq!(harness.list_sessions(P0), Err(unreachable(P0)));
    assert_eq!(harness.abort(P0, &a), Err(unreachable(P0)));
    assert_eq!(harness.attach_tui(&pane1(), P0, &a), Err(unreachable(P0)));
    assert_eq!(harness.select_session(&pane0(), &b), Err(unreachable(P0)));
    assert_eq!(
        harness.verify_serving(P0, &ServeSpec::default()),
        Err(unreachable(P0))
    );
    assert_eq!(
        shown(&harness, &pane0()),
        Some(a),
        "the TUI keeps its screen"
    );

    assert_eq!(harness.freeze(P0), Err(unreachable(P0)));
    assert_eq!(harness.thaw(P0), Err(unreachable(P0)));
    assert_eq!(harness.kill(P0), Ok(()), "killing a dead server is fine");
}

#[test]
fn serving_a_killed_port_again_keeps_the_sessions() {
    let (harness, old_pid, a, b) = scene();
    harness.kill(P0).unwrap();

    let new_pid = serve(&harness, "demo-c1r1", P0).unwrap();
    assert_ne!(new_pid, old_pid, "a new process");
    assert!(new_pid >= 20_000);
    assert_eq!(
        harness.server(P0),
        view("demo-c1r1", new_pid, ServerState::Running)
    );
    assert_eq!(harness.health(P0), Ok(true));
    assert_eq!(
        sorted(harness.list_sessions(P0).unwrap()),
        sorted(vec![a.clone(), b.clone()])
    );
    assert_eq!(shown(&harness, &pane0()), Some(a));
    harness.select_session(&pane0(), &b).unwrap();
    assert_eq!(
        shown(&harness, &pane0()),
        Some(b),
        "the old TUI is driven by the new server"
    );
}

#[test]
fn starting_a_server_on_a_running_port() {
    let harness = FakeHarness::new();
    let first = serve(&harness, "demo-c1r1", P0).unwrap();
    assert!(first >= 20_000);
    assert_eq!(serve(&harness, "demo-c1r1", P0), Ok(first), "idempotent");
    assert_eq!(
        serve(&harness, "demo-c2r1", P0),
        Err(PaneError::Unavailable {
            what: "port 48100 is in use by the server of demo-c1r1".to_owned()
        })
    );
    assert_eq!(
        harness.server(P0),
        view("demo-c1r1", first, ServerState::Running)
    );
    assert_eq!(
        harness.list_sessions(P0),
        Ok(vec![]),
        "a started server creates no session"
    );
    let other = serve(&harness, "demo-c2r1", P1).unwrap();
    assert_ne!(other, first, "pids are never reused");
}

#[test]
fn controls_of_an_unserved_port_are_unavailable() {
    let harness = FakeHarness::new();
    assert_eq!(harness.freeze(P0), Err(unreachable(P0)));
    assert_eq!(harness.thaw(P0), Err(unreachable(P0)));
    assert_eq!(harness.kill(P0), Err(unreachable(P0)));
    assert_eq!(harness.server(P0), None);
}

// --- scenarios the port cannot cause ---

#[test]
fn a_session_deleted_under_a_tui_sends_it_home() {
    let (harness, _, a, b) = scene();
    harness.attach_tui(&pane1(), P1, &a).unwrap();
    harness.attach_tui(&pane2(), P0, &b).unwrap();

    harness.delete_session(&a).unwrap();

    assert_eq!(shown(&harness, &pane0()), None);
    assert_eq!(shown(&harness, &pane1()), None, "every TUI that showed it");
    assert_eq!(
        shown(&harness, &pane2()),
        Some(b.clone()),
        "a TUI on another session stays"
    );
    assert_eq!(
        harness.tui(&pane0()),
        Some(TuiView {
            port: P0,
            shown: None
        })
    );
    assert_eq!(harness.list_sessions(P0), Ok(vec![b.clone()]));

    harness.select_session(&pane0(), &b).unwrap();
    assert_eq!(
        shown(&harness, &pane0()),
        Some(b),
        "a live session brings it back"
    );
    assert_eq!(harness.delete_session(UNKNOWN), Err(not_found(UNKNOWN)));
}

#[test]
fn navigating_by_hand_changes_the_shown_session() {
    let (harness, _, _, b) = scene();
    harness.navigate(&pane0(), Some(&b)).unwrap();
    assert_eq!(shown(&harness, &pane0()), Some(b.clone()));

    assert_eq!(
        harness.navigate(&pane0(), Some(UNKNOWN)),
        Err(not_found(UNKNOWN))
    );
    assert_eq!(
        shown(&harness, &pane0()),
        Some(b.clone()),
        "a refused move changes nothing"
    );
    assert_eq!(harness.navigate(&pane1(), Some(&b)), Err(no_tui(&pane1())));
    assert_eq!(harness.navigate(&pane1(), None), Err(no_tui(&pane1())));

    harness.navigate(&pane0(), None).unwrap();
    assert_eq!(shown(&harness, &pane0()), None);
    assert_eq!(
        harness.tui(&pane0()),
        Some(TuiView {
            port: P0,
            shown: None
        })
    );
}

#[test]
fn closing_the_tui_leaves_no_shown_session() {
    let (harness, _, a, _) = scene();
    harness.close_tui(&pane0()).unwrap();

    assert_eq!(harness.tui(&pane0()), None);
    assert_eq!(harness.shown_session(&pane0()), Ok(None));
    assert_eq!(harness.select_session(&pane0(), &a), Err(no_tui(&pane0())));
    assert_eq!(harness.close_tui(&pane0()), Err(no_tui(&pane0())));
}

#[test]
fn attaching_again_replaces_the_tui_and_a_refused_attach_leaves_it() {
    let (harness, _, a, _) = scene();
    let on_p1 = harness.create_session(P1).unwrap();

    assert_eq!(
        harness.attach_tui(&pane0(), P0, UNKNOWN),
        Err(not_found(UNKNOWN))
    );
    assert_eq!(
        harness.tui(&pane0()),
        Some(TuiView {
            port: P0,
            shown: Some(a)
        })
    );

    harness.attach_tui(&pane0(), P1, &on_p1).unwrap();
    assert_eq!(
        harness.tui(&pane0()),
        Some(TuiView {
            port: P1,
            shown: Some(on_p1)
        })
    );
}

// --- the quirks of raw OpenCode ---

#[test]
fn select_without_a_tui_is_acked_with_the_quirk() {
    let (harness, _, a, _) = scene();
    assert_eq!(
        harness.select_session(&pane1(), &a),
        Err(no_tui(&pane1())),
        "off by default"
    );

    harness.set_quirk(Quirk::SelectAckedWithoutTui, true);
    assert_eq!(harness.select_session(&pane1(), &a), Ok(()));
    assert_eq!(harness.tui(&pane1()), None, "and changes nothing");
    assert_eq!(shown(&harness, &pane1()), None);

    harness.set_quirk(Quirk::SelectAckedWithoutTui, false);
    assert_eq!(harness.select_session(&pane1(), &a), Err(no_tui(&pane1())));
}

#[test]
fn abort_of_an_unknown_id_is_acked_with_the_quirk() {
    let (harness, _, a, _) = scene();
    assert_eq!(
        harness.abort(P0, UNKNOWN),
        Err(not_found(UNKNOWN)),
        "off by default"
    );
    assert_eq!(harness.aborts(), vec![], "nothing logged");

    harness.abort(P0, &a).unwrap();
    assert_eq!(
        harness.aborts(),
        vec![(P0, a.clone())],
        "a known abort is logged"
    );

    harness.set_quirk(Quirk::AbortUnknownAcked, true);
    assert_eq!(harness.abort(P0, UNKNOWN), Ok(()));
    assert_eq!(harness.aborts(), vec![(P0, a), (P0, UNKNOWN.to_owned())]);
}

// --- data directories and seeded sessions ---

#[test]
fn separate_data_dirs_do_not_share_sessions() {
    let harness = FakeHarness::new();
    let early = {
        serve(&harness, "demo-c1r1", P0).unwrap();
        serve(&harness, "demo-c2r1", P1).unwrap();
        harness.create_session(P1).unwrap()
    };
    harness.set_data_dir(P1, "other");
    assert_eq!(
        harness.list_sessions(P0),
        Ok(vec![early]),
        "a session made before stays"
    );
    assert_eq!(harness.list_sessions(P1), Ok(vec![]));

    let s = harness.create_session(P1).unwrap();
    assert_eq!(harness.list_sessions(P1), Ok(vec![s.clone()]));
    assert!(!harness.list_sessions(P0).unwrap().contains(&s));
    assert_eq!(harness.abort(P0, &s), Err(not_found(&s)));
    assert_eq!(harness.attach_tui(&pane0(), P0, &s), Err(not_found(&s)));
    harness.attach_tui(&pane1(), P1, &s).unwrap();

    let a = harness.create_session(P0).unwrap();
    harness.attach_tui(&pane0(), P0, &a).unwrap();
    assert_eq!(harness.select_session(&pane0(), &s), Err(not_found(&s)));
    assert_eq!(shown(&harness, &pane0()), Some(a));
}

#[test]
fn a_seeded_session_is_listed_and_bypasses_the_log() {
    let harness = FakeHarness::new();
    let seeded = harness.seed_session(P0);
    serve(&harness, "demo-c1r1", P0).unwrap();
    serve(&harness, "demo-c2r1", P1).unwrap();

    assert_eq!(harness.list_sessions(P0), Ok(vec![seeded.clone()]));
    assert_eq!(
        harness.list_sessions(P1),
        Ok(vec![seeded.clone()]),
        "one data dir"
    );
    let own = harness.create_session(P0).unwrap();
    assert_ne!(own, seeded);
    assert_eq!(
        harness.faults().calls(),
        vec![
            HarnessOp::ServeCommand,
            HarnessOp::ServeCommand,
            HarnessOp::ListSessions,
            HarnessOp::ListSessions,
            HarnessOp::CreateSession
        ]
    );
}

// --- #802: serve_command, start_server and verify_serving ---

/// #802: `serve_command` starts nothing (no server, no session, nothing healthy on the
/// port), records the spec it was asked for, and fails through the fault switch like every
/// port method, changing nothing.
#[test]
fn serve_command_starts_nothing_and_records_its_spec() {
    let harness = FakeHarness::new();
    let spec = ServeSpec::default();
    let built = command(&harness, "demo-c1r1", P0, &spec);
    assert!(!built.as_slice().is_empty(), "an argv with a program");
    assert_eq!(harness.server(P0), None, "no server was started");
    assert_eq!(harness.health(P0), Ok(false));
    assert_eq!(harness.list_sessions(P0), Err(unreachable(P0)));

    let refusal = PaneError::Unavailable {
        what: "injected".to_owned(),
    };
    harness
        .faults()
        .fail_next(HarnessOp::ServeCommand, refusal.clone());
    let with = ServeSpec::default().with_agent(agent("orchestrator"));
    assert_eq!(
        harness.serve_command(&name("demo-c1r1"), P0, &with),
        Err(refusal)
    );
    assert_eq!(
        harness.serve_specs(),
        vec![spec],
        "a failed call records no spec"
    );
    assert_eq!(
        harness.faults().calls(),
        vec![
            HarnessOp::ServeCommand,
            HarnessOp::Health,
            HarnessOp::ListSessions,
            HarnessOp::ServeCommand
        ]
    );
}

/// #802: the argv names the agent the spec asks for (some element holds the key, the
/// port's contract), the server started from it runs for that pane and keeps the agent,
/// and every spec is recorded in call order.
#[test]
fn the_argv_carries_the_agent_and_the_server_keeps_it() {
    let harness = FakeHarness::new();
    let key = agent("feature-implementor");
    let with = ServeSpec::default().with_agent(key.clone());
    let built = command(&harness, "demo-c1r1", P0, &with);
    assert!(
        built.as_slice().iter().any(|e| e.contains(key.as_str())),
        "some element holds the key: {built:?}"
    );
    let plain = command(&harness, "demo-c2r1", P1, &ServeSpec::default());
    assert!(
        !plain.as_slice().iter().any(|e| e.contains(key.as_str())),
        "no agent, no key: {plain:?}"
    );
    assert_ne!(built, plain, "the argv names its pane, port and agent");

    let pid = harness
        .start_server(&built)
        .expect("a fake server argv")
        .unwrap();
    let server = harness.server(P0).expect("a server on P0");
    assert_eq!(
        server,
        ServerView {
            name: name("demo-c1r1"),
            pid,
            state: ServerState::Running,
            agent: Some(key.clone()),
        }
    );
    harness
        .start_server(&plain)
        .expect("a fake server argv")
        .unwrap();
    assert_eq!(harness.server(P1).unwrap().agent, None);
    assert_eq!(harness.serve_specs(), vec![with, ServeSpec::default()]);
}

/// #802: the scenario method answers `None` for any argv `serve_command` did not build (a
/// spec's own command, say) and starts nothing.
#[test]
fn start_server_ignores_an_argv_it_did_not_build() {
    let harness = FakeHarness::new();
    for other in [
        argv(&["sleep", "30"]),
        argv(&["opencode", "serve", "--port", "48100"]),
        argv(&[]),
    ] {
        assert_eq!(harness.start_server(&other), None, "{other:?}");
    }
    assert_eq!(harness.server(P0), None);
    assert_eq!(harness.server(P1), None);
    assert_eq!(
        harness.faults().calls(),
        vec![],
        "not a call through the port"
    );
}

/// #802 (plan review W-3): `verify_serving` answers from the agent the running server was
/// started with: `Ok` when the spec asks for exactly that, a refusal otherwise, and the
/// server's own state for a port that does not answer.
#[test]
fn verify_serving_answers_from_the_running_servers_agent() {
    let harness = FakeHarness::new();
    let key = agent("feature-implementor");
    let with = ServeSpec::default().with_agent(key.clone());
    let other = ServeSpec::default().with_agent(agent("orchestrator"));
    let started = command(&harness, "demo-c1r1", P0, &with);
    harness.start_server(&started).unwrap().unwrap();
    serve(&harness, "demo-c2r1", P1).unwrap();

    assert_eq!(harness.verify_serving(P0, &with), Ok(()));
    for asked in [ServeSpec::default(), other.clone()] {
        let answer = harness.verify_serving(P0, &asked);
        assert!(
            matches!(answer, Err(PaneError::Refused { .. })),
            "{asked:?}: {answer:?}"
        );
    }
    assert_eq!(harness.verify_serving(P1, &ServeSpec::default()), Ok(()));
    let answer = harness.verify_serving(P1, &with);
    assert!(
        matches!(answer, Err(PaneError::Refused { .. })),
        "a server without the agent: {answer:?}"
    );
    assert_eq!(
        harness.verify_serving(48102, &ServeSpec::default()),
        Err(unreachable(48102)),
        "no server on the port"
    );
}

#[test]
fn the_fake_harness_is_send_and_sync() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<FakeHarness>();
    send_sync::<HarnessOp>();
}
