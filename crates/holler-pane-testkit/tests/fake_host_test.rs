#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #684
//! `FakeHost`'s own mechanisms: fault injection, the call log, the argv log, pids, and
//! the scenario controls (a process that exits, a session that ends). The generic port
//! rules are the conformance suite's job; this file pins what only the fake has, and
//! what a real tmux adapter may differ on, such as a fresh session's empty `ps`
//! (#638, slice e).

use holler_pane::{Argv, HostPort, PaneError, PaneName};
use holler_pane_testkit::fault::{Fault, PortOp};
use holler_pane_testkit::host::{FakeHost, HostOp};

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}

fn not_found(session: &PaneName) -> PaneError {
    PaneError::PaneNotFound {
        what: session.to_string(),
    }
}

/// A host with the session `text` ensured in `/srv/demo`.
fn host_with(text: &str) -> (FakeHost, PaneName) {
    let host = FakeHost::new();
    let session = name(text);
    host.ensure_session(&session, "/srv/demo").unwrap();
    (host, session)
}

fn run_sleep(host: &FakeHost, session: &PaneName) -> u32 {
    let before = host.ps(session).unwrap();
    host.run(session, &argv(&["sleep", "30"])).unwrap();
    let after = host.ps(session).unwrap();
    *after.iter().find(|pid| !before.contains(pid)).unwrap()
}

// --- wedged ---

#[test]
fn a_wedged_host_times_out_every_method() {
    let host = FakeHost::new();
    let a = name("demo-c1r1");
    host.faults().set(Some(Fault::Wedged));

    assert_eq!(
        host.ensure_session(&a, "/srv/demo"),
        Err(timeout("host.ensure_session"))
    );
    assert_eq!(
        host.run(&a, &argv(&["sleep", "30"])),
        Err(timeout("host.run"))
    );
    assert_eq!(host.stop_owned(&a), Err(timeout("host.stop_owned")));
    assert_eq!(host.ps(&a), Err(timeout("host.ps")));

    host.faults().set(None);
    assert_eq!(
        host.sessions(),
        vec![],
        "the wedged ensure_session created nothing"
    );
    host.ensure_session(&a, "/srv/demo").unwrap();
    assert_eq!(host.ps(&a), Ok(vec![]), "the wedged run started nothing");
    host.run(&a, &argv(&["sleep", "30"])).unwrap();
    assert_eq!(host.ps(&a).unwrap().len(), 1);
}

#[test]
fn host_op_names_are_port_dot_method() {
    assert_eq!(HostOp::EnsureSession.as_str(), "host.ensure_session");
    assert_eq!(HostOp::Run.as_str(), "host.run");
    assert_eq!(HostOp::StopOwned.as_str(), "host.stop_owned");
    assert_eq!(HostOp::Ps.as_str(), "host.ps");
}

// --- the call log ---

#[test]
fn calls_are_recorded_in_order() {
    let host = FakeHost::new();
    let a = name("demo-c1r1");
    host.ensure_session(&a, "/srv/demo").unwrap();
    assert!(host.run(&a, &argv(&[])).is_err());
    host.ps(&a).unwrap();
    let expected = vec![HostOp::EnsureSession, HostOp::Run, HostOp::Ps];
    assert_eq!(host.faults().calls(), expected);

    // Scenario controls and inspection are not calls through the port.
    assert_eq!(host.sessions(), vec![a.clone()]);
    assert_eq!(host.cwd(&a).as_deref(), Some("/srv/demo"));
    assert_eq!(host.runs(), vec![]);
    host.exit_process(&a, 1).unwrap_err();
    host.end_session(&a).unwrap();
    assert_eq!(host.faults().calls(), expected);
}

#[test]
fn exiting_a_process_is_not_a_call_through_the_port() {
    let (host, a) = host_with("demo-c1r1");
    let pid = run_sleep(&host, &a);
    let before = host.faults().calls();
    host.exit_process(&a, pid).unwrap();
    assert_eq!(host.faults().calls(), before);
}

// --- the argv log ---

#[test]
fn run_records_each_argv_verbatim() {
    let (host, a) = host_with("demo-c1r1");
    let serve = argv(&["opencode", "serve", "--port", "48100"]);
    let shellish = argv(&["echo", "a b; rm -rf x"]);
    host.run(&a, &serve).unwrap();
    host.run(&a, &shellish).unwrap();

    let want = vec![(a.clone(), serve), (a.clone(), shellish)];
    assert_eq!(host.runs(), want);
    assert_eq!(host.runs()[1].1.as_slice().len(), 2, "never re-split");
    assert_eq!(host.runs()[1].1.as_slice()[1], "a b; rm -rf x");

    host.stop_owned(&a).unwrap();
    assert_eq!(host.runs(), want, "stop_owned does not clear the log");
    host.end_session(&a).unwrap();
    assert_eq!(host.runs(), want, "end_session does not clear the log");
}

#[test]
fn a_refused_run_records_nothing() {
    let (host, a) = host_with("demo-c1r1");
    let before = host.ps(&a).unwrap();

    assert_eq!(
        host.run(&name("demo-c2r1"), &argv(&["sleep", "30"])),
        Err(not_found(&name("demo-c2r1")))
    );
    assert_eq!(
        host.run(&a, &argv(&[])),
        Err(PaneError::Usage {
            message: "an empty argv has no program to run".to_owned()
        })
    );
    let injected = PaneError::Unavailable {
        what: "tmux".to_owned(),
    };
    host.faults().fail_next(HostOp::Run, injected.clone());
    assert_eq!(host.run(&a, &argv(&["sleep", "30"])), Err(injected));

    assert_eq!(host.runs(), vec![]);
    assert_eq!(host.ps(&a), Ok(before));
}

// --- sessions, cwd, pids ---

#[test]
fn ensure_session_keeps_the_first_cwd() {
    let host = FakeHost::new();
    let (a, b) = (name("demo-c1r1"), name("demo-c2r1"));
    host.ensure_session(&b, "/srv/second").unwrap();
    host.ensure_session(&a, "/srv/first").unwrap();
    host.ensure_session(&a, "/srv/other").unwrap();

    assert_eq!(host.cwd(&a).as_deref(), Some("/srv/first"));
    assert_eq!(host.cwd(&b).as_deref(), Some("/srv/second"));
    assert_eq!(host.cwd(&name("demo-c3r1")), None);
    assert_eq!(
        host.sessions(),
        vec![a, b],
        "sorted by name, not creation order"
    );
}

#[test]
fn a_fresh_session_has_no_process_and_survives_stop_owned() {
    let (host, a) = host_with("demo-c1r1");
    assert_eq!(host.ps(&a), Ok(vec![]), "the fake has no shell process");
    run_sleep(&host, &a);

    host.stop_owned(&a).unwrap();
    assert_eq!(host.ps(&a), Ok(vec![]));
    assert_eq!(host.sessions(), vec![a.clone()], "the session stays");

    let missing = name("demo-c9r9");
    host.stop_owned(&missing).unwrap();
    assert_eq!(
        host.sessions(),
        vec![a],
        "stopping a missing session creates nothing"
    );
}

#[test]
fn pids_are_distinct_and_never_reused() {
    let (host, a) = host_with("demo-c1r1");
    let b = name("demo-c2r1");
    host.ensure_session(&b, "/srv/demo").unwrap();

    let first = run_sleep(&host, &a);
    let in_b = run_sleep(&host, &b);
    host.stop_owned(&a).unwrap();
    let after_stop = run_sleep(&host, &a);
    host.end_session(&b).unwrap();
    host.ensure_session(&b, "/srv/demo").unwrap();
    let after_end = run_sleep(&host, &b);
    let second = run_sleep(&host, &a);
    host.exit_process(&a, second).unwrap();
    let after_exit = run_sleep(&host, &a);

    assert_eq!(first, 10_000);
    let all = [first, in_b, after_stop, after_end, second, after_exit];
    let mut sorted = all.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), all.len(), "no pid repeats: {all:?}");
    assert_eq!(
        host.ps(&a).unwrap(),
        vec![after_stop, after_exit],
        "in start order"
    );
}

// --- scenario controls ---

#[test]
fn a_process_that_exits_leaves_ps() {
    let (host, a) = host_with("demo-c1r1");
    let p1 = run_sleep(&host, &a);
    let p2 = run_sleep(&host, &a);

    host.exit_process(&a, p1).unwrap();
    assert_eq!(host.ps(&a), Ok(vec![p2]));

    assert_eq!(
        host.exit_process(&a, p1),
        Err(not_found(&a)),
        "it already exited"
    );
    assert_eq!(
        host.exit_process(&a, 1),
        Err(not_found(&a)),
        "never started"
    );
    let gone = name("demo-c9r9");
    assert_eq!(host.exit_process(&gone, p2), Err(not_found(&gone)));
    assert_eq!(
        host.ps(&a),
        Ok(vec![p2]),
        "the failed exits changed nothing"
    );
}

#[test]
fn an_ended_session_is_missing_until_ensured_again() {
    let (host, a) = host_with("demo-c1r1");
    run_sleep(&host, &a);

    host.end_session(&a).unwrap();
    assert_eq!(host.sessions(), vec![]);
    assert_eq!(host.ps(&a), Err(not_found(&a)));
    assert_eq!(host.run(&a, &argv(&["sleep", "30"])), Err(not_found(&a)));
    assert_eq!(host.stop_owned(&a), Ok(()));

    host.ensure_session(&a, "/srv/demo").unwrap();
    assert_eq!(host.ps(&a), Ok(vec![]), "it comes back with no processes");
    assert_eq!(
        host.end_session(&name("demo-c9r9")),
        Err(not_found(&name("demo-c9r9")))
    );
}

#[test]
fn the_fake_host_is_send_and_sync() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<FakeHost>();
    send_sync::<HostOp>();
}
