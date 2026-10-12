#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #831
//! `FakeHarnesses`, the router of the two harness fakes (#831, ADR-0021 section 15 item 4; plan
//! review W5): one `HarnessPort` over `FakeHarness` (OpenCode) and `FakeClaude`, built on the one
//! `holler_pane` router the wiring also builds, so a grid that mixes the two harnesses needs no
//! new field in `Ports`.
//!
//! - both conformance suites run over it, which pins every route at once;
//! - `start` and `stop` are the one linker a rig's host calls (`run` of an argv either fake
//!   built starts it; `stop_owned` of a pane ends both fakes' processes of that pane), so no rig
//!   writes its own Claude linker;
//! - a call reaches the fake its endpoint names, and the calls that carry no endpoint go to the
//!   fake of their shape.

use holler_pane::ports::{LoginStatus, SessionChoice};
use holler_pane::{Argv, HarnessEndpoint, HarnessPort, PaneError, PaneName, ServeSpec};
use holler_pane_testkit::claude::ClaudeOp;
use holler_pane_testkit::conformance::claude::{run_claude_conformance, ClaudeRig};
use holler_pane_testkit::conformance::harness::{run_harness_conformance, HarnessRig};
use holler_pane_testkit::harness::{HarnessOp, ServerState};
use holler_pane_testkit::harnesses::FakeHarnesses;

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn launch(h: &FakeHarnesses, _: &PaneName, argv: &Argv) -> Result<(), PaneError> {
    match h.start(argv) {
        Some(started) => started.map(drop),
        None => Err(PaneError::Usage {
            message: "not an argv either fake built".to_owned(),
        }),
    }
}

#[test]
fn the_router_of_the_two_fakes_passes_the_server_suite() {
    let result =
        run_harness_conformance(|| (FakeHarnesses::new(), HarnessRig::sample(), ()), launch);
    assert_eq!(result, Ok(()));
}

#[test]
fn the_router_of_the_two_fakes_passes_the_claude_suite() {
    let result = run_claude_conformance(
        || (FakeHarnesses::new(), ClaudeRig::sample(), ()),
        launch,
        |h: &FakeHarnesses, pane: &PaneName| {
            h.stop(pane);
            Ok(())
        },
    );
    assert_eq!(result, Ok(()));
}

#[test]
fn start_runs_each_fakes_own_argv_and_no_other() {
    let h = FakeHarnesses::new();
    let server = h
        .serve_command(&name("demo-c1r1"), 48100, &ServeSpec::default())
        .unwrap();
    h.start(&server).expect("the server's argv").unwrap();
    assert_eq!(
        h.opencode().server(48100).map(|s| s.state),
        Some(ServerState::Running)
    );

    let id = h.new_session_id().unwrap();
    let process = h
        .process_command(
            &name("demo-c2r1"),
            &ServeSpec::default().with_cwd("/srv/demo"),
            &SessionChoice::Fresh(id.clone()),
        )
        .unwrap();
    let pid = h.start(&process).expect("the process's argv").unwrap();
    assert_eq!(h.claude().process(pid).map(|p| p.session_id), Some(id));

    assert!(h.start(&Argv::new(vec!["true".to_owned()])).is_none());
}

#[test]
fn stop_ends_one_panes_processes_in_both_fakes() {
    let h = FakeHarnesses::new();
    let server = h
        .serve_command(&name("demo-c1r1"), 48100, &ServeSpec::default())
        .unwrap();
    h.start(&server).unwrap().unwrap();
    let id = h.new_session_id().unwrap();
    let process = h
        .process_command(
            &name("demo-c2r1"),
            &ServeSpec::default().with_cwd("/srv/demo"),
            &SessionChoice::Fresh(id),
        )
        .unwrap();
    let pid = h.start(&process).unwrap().unwrap();

    h.stop(&name("demo-c1r1"));
    assert_eq!(
        h.opencode().server(48100).map(|s| s.state),
        Some(ServerState::Killed)
    );
    assert!(
        h.claude().process(pid).is_some(),
        "another pane's process runs on"
    );

    h.stop(&name("demo-c2r1"));
    assert!(h.claude().process(pid).is_none());
}

#[test]
fn a_call_reaches_the_fake_its_endpoint_or_its_shape_names() {
    let h = FakeHarnesses::new();
    let _ = h.health(&HarnessEndpoint::Port(48100));
    let _ = h.health(&HarnessEndpoint::Process {
        pane: name("demo-c2r1"),
        pid: 1,
    });
    let _ = h.serve_command(&name("demo-c1r1"), 48100, &ServeSpec::default());
    let _ = h.new_session_id();
    let _ = h.locate_process(&name("demo-c2r1"), "s-1");
    assert_eq!(h.login_status().unwrap(), LoginStatus::Ok);

    assert_eq!(
        h.opencode().faults().calls(),
        vec![HarnessOp::Health, HarnessOp::ServeCommand],
        "the port endpoint and the server's argv go to OpenCode"
    );
    assert_eq!(
        h.claude().faults().calls(),
        vec![
            ClaudeOp::Health,
            ClaudeOp::NewSessionId,
            ClaudeOp::LocateProcess,
            ClaudeOp::LoginStatus,
        ],
        "the process endpoint and the process methods go to Claude Code"
    );
}
