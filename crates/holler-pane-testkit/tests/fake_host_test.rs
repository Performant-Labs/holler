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
    host.run(session, "/srv/demo", &argv(&["sleep", "30"]))
        .unwrap();
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
        host.run(&a, "/srv/demo", &argv(&["sleep", "30"])),
        Err(timeout("host.run"))
    );
    assert_eq!(host.stop_owned(&a), Err(timeout("host.stop_owned")));
    assert_eq!(host.ps(&a), Err(timeout("host.ps")));
    assert_eq!(
        host.foreground_commands(&a),
        Err(timeout("host.foreground_commands"))
    );
    assert_eq!(
        host.foreground_pids(&a),
        Err(timeout("host.foreground_pids"))
    );
    assert_eq!(
        host.attached_clients(&a),
        Err(timeout("host.attached_clients"))
    );

    host.faults().set(None);
    assert_eq!(
        host.sessions(),
        vec![],
        "the wedged ensure_session created nothing"
    );
    host.ensure_session(&a, "/srv/demo").unwrap();
    assert_eq!(host.ps(&a), Ok(vec![]), "the wedged run started nothing");
    host.run(&a, "/srv/demo", &argv(&["sleep", "30"])).unwrap();
    assert_eq!(host.ps(&a).unwrap().len(), 1);
}

#[test]
fn host_op_names_are_port_dot_method() {
    assert_eq!(HostOp::EnsureSession.as_str(), "host.ensure_session");
    assert_eq!(HostOp::Run.as_str(), "host.run");
    assert_eq!(HostOp::StopOwned.as_str(), "host.stop_owned");
    assert_eq!(HostOp::Ps.as_str(), "host.ps");
    assert_eq!(
        HostOp::ForegroundCommands.as_str(),
        "host.foreground_commands"
    );
    assert_eq!(HostOp::ForegroundPids.as_str(), "host.foreground_pids");
    assert_eq!(HostOp::AttachedClients.as_str(), "host.attached_clients");
}

// --- the call log ---

#[test]
fn calls_are_recorded_in_order() {
    let host = FakeHost::new();
    let a = name("demo-c1r1");
    host.ensure_session(&a, "/srv/demo").unwrap();
    assert!(host.run(&a, "/srv/demo", &argv(&[])).is_err());
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
    host.run(&a, "/srv/demo", &serve).unwrap();
    host.run(&a, "/srv/other", &shellish).unwrap();

    // #802: each entry carries the directory `run` was given, which need not be the one
    // the session was made with.
    let want = vec![
        (a.clone(), "/srv/demo".to_owned(), serve),
        (a.clone(), "/srv/other".to_owned(), shellish),
    ];
    assert_eq!(host.runs(), want);
    assert_eq!(host.runs()[1].2.as_slice().len(), 2, "never re-split");
    assert_eq!(host.runs()[1].2.as_slice()[1], "a b; rm -rf x");
    assert_eq!(
        host.cwd(&a).as_deref(),
        Some("/srv/demo"),
        "run never changes the session's own directory"
    );

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
        host.run(&name("demo-c2r1"), "/srv/demo", &argv(&["sleep", "30"])),
        Err(not_found(&name("demo-c2r1")))
    );
    assert_eq!(
        host.run(&a, "/srv/demo", &argv(&[])),
        Err(PaneError::Usage {
            message: "an empty argv has no program to run".to_owned()
        })
    );
    let injected = PaneError::Unavailable {
        what: "tmux".to_owned(),
    };
    host.faults().fail_next(HostOp::Run, injected.clone());
    assert_eq!(
        host.run(&a, "/srv/demo", &argv(&["sleep", "30"])),
        Err(injected)
    );

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
    assert_eq!(
        host.run(&a, "/srv/demo", &argv(&["sleep", "30"])),
        Err(not_found(&a))
    );
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

// --- foreground_commands (#835) ---

/// The fake names each process `run` started by the basename of its argv's first element,
/// as tmux reports a pane's command, in the order they started; a fresh session (the fake
/// has no shell) names nothing, and another session's processes are not named.
#[test]
fn foreground_commands_names_each_process_by_its_programs_basename() {
    let (host, a) = host_with("demo-c1r1");
    assert_eq!(host.foreground_commands(&a), Ok(vec![]));
    host.run(&a, "/srv/demo", &argv(&["/opt/demo/bin/claude", "--resume"]))
        .unwrap();
    host.run(&a, "/srv/demo", &argv(&["bash", "-c", "oc-holler"]))
        .unwrap();
    let b = name("demo-c2r1");
    host.ensure_session(&b, "/srv/demo").unwrap();
    host.run(&b, "/srv/demo", &argv(&["opencode", "serve"]))
        .unwrap();
    assert_eq!(
        host.foreground_commands(&a),
        Ok(vec!["claude".to_owned(), "bash".to_owned()])
    );
    assert_eq!(host.foreground_commands(&b), Ok(vec!["opencode".to_owned()]));
}

/// A process that exits, `stop_owned` and an ended session each take the names with the
/// processes.
#[test]
fn foreground_commands_follow_the_processes_out() {
    let (host, a) = host_with("demo-c1r1");
    host.run(&a, "/srv/demo", &argv(&["claude"])).unwrap();
    let first = host.ps(&a).unwrap()[0];
    host.run(&a, "/srv/demo", &argv(&["sleep", "30"])).unwrap();
    host.exit_process(&a, first).unwrap();
    assert_eq!(host.foreground_commands(&a), Ok(vec!["sleep".to_owned()]));
    host.stop_owned(&a).unwrap();
    assert_eq!(host.foreground_commands(&a), Ok(vec![]));
    host.end_session(&a).unwrap();
    assert_eq!(host.foreground_commands(&a), Err(not_found(&a)));
}

/// A fault queued for `foreground_commands` fails that call only, and the call is logged.
#[test]
fn a_fault_on_foreground_commands_fails_that_call_only() {
    let (host, a) = host_with("demo-c1r1");
    host.run(&a, "/srv/demo", &argv(&["claude"])).unwrap();
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    host.faults()
        .fail_next(HostOp::ForegroundCommands, unavailable.clone());
    assert_eq!(host.ps(&a).map(|pids| pids.len()), Ok(1));
    assert_eq!(host.foreground_commands(&a), Err(unavailable));
    assert_eq!(host.foreground_commands(&a), Ok(vec!["claude".to_owned()]));
    let calls = host.faults().calls();
    assert_eq!(
        calls[calls.len() - 3..],
        [
            HostOp::Ps,
            HostOp::ForegroundCommands,
            HostOp::ForegroundCommands
        ]
    );
}

// --- foreground_pids (#831) ---

/// The pids of a session's processes, ascending, one session's only; a fresh session (the fake
/// has no shell) has none, and a missing one is `pane-not-found`, as `ps` answers.
#[test]
fn foreground_pids_lists_the_sessions_processes_ascending() {
    let (host, a) = host_with("demo-c1r1");
    assert_eq!(host.foreground_pids(&a), Ok(vec![]));
    host.run(&a, "/srv/demo", &argv(&["bash"])).unwrap();
    host.run(&a, "/srv/demo", &argv(&["claude"])).unwrap();
    let b = name("demo-c2r1");
    host.ensure_session(&b, "/srv/demo").unwrap();
    host.run(&b, "/srv/demo", &argv(&["sleep", "30"])).unwrap();
    let in_a = host.ps(&a).unwrap();
    assert_eq!(in_a.len(), 2);
    let mut want = in_a;
    want.sort_unstable();
    assert_eq!(host.foreground_pids(&a), Ok(want));
    assert_eq!(host.foreground_pids(&b), host.ps(&b));
    assert_eq!(
        host.foreground_pids(&name("demo-c9r9")),
        Err(not_found(&name("demo-c9r9")))
    );
}

/// A process that exits, `stop_owned` and an ended session each take the pids with the
/// processes.
#[test]
fn foreground_pids_follow_the_processes_out() {
    let (host, a) = host_with("demo-c1r1");
    host.run(&a, "/srv/demo", &argv(&["claude"])).unwrap();
    host.run(&a, "/srv/demo", &argv(&["sleep", "30"])).unwrap();
    let pids = host.foreground_pids(&a).unwrap();
    host.exit_process(&a, pids[0]).unwrap();
    assert_eq!(host.foreground_pids(&a), Ok(vec![pids[1]]));
    host.stop_owned(&a).unwrap();
    assert_eq!(host.foreground_pids(&a), Ok(vec![]));
    host.end_session(&a).unwrap();
    assert_eq!(host.foreground_pids(&a), Err(not_found(&a)));
}

/// A fault queued for `foreground_pids` fails that call only, and the call is logged.
#[test]
fn a_fault_on_foreground_pids_fails_that_call_only() {
    let (host, a) = host_with("demo-c1r1");
    host.run(&a, "/srv/demo", &argv(&["claude"])).unwrap();
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    host.faults()
        .fail_next(HostOp::ForegroundPids, unavailable.clone());
    assert_eq!(host.foreground_pids(&a), Err(unavailable));
    assert_eq!(host.foreground_pids(&a).map(|pids| pids.len()), Ok(1));
    let calls = host.faults().calls();
    assert_eq!(
        calls[calls.len() - 2..],
        [HostOp::ForegroundPids, HostOp::ForegroundPids]
    );
}

/// `add_process` puts a process another fake started into the session under the pid that fake
/// minted, so `ps`, `foreground_commands` and `foreground_pids` all show that pid; it is not a
/// call through the port; a pid the session holds already is `usage`; a missing session is
/// `pane-not-found`.
#[test]
fn add_process_shares_one_pid_with_another_fake() {
    let (host, a) = host_with("demo-c1r1");
    host.run(&a, "/srv/demo", &argv(&["bash"])).unwrap();
    let shell = host.ps(&a).unwrap()[0];
    let before = host.faults().calls();
    host.add_process(&a, "/opt/demo/bin/claude", 30_000)
        .unwrap();
    assert_eq!(host.faults().calls(), before, "not a call through the port");
    assert_eq!(host.ps(&a), Ok(vec![shell, 30_000]));
    assert_eq!(
        host.foreground_commands(&a),
        Ok(vec!["bash".to_owned(), "claude".to_owned()])
    );
    assert_eq!(host.foreground_pids(&a), Ok(vec![shell, 30_000]));
    assert!(matches!(
        host.add_process(&a, "claude", 30_000),
        Err(PaneError::Usage { .. })
    ));
    let missing = name("demo-c9r9");
    assert_eq!(
        host.add_process(&missing, "claude", 30_001),
        Err(not_found(&missing))
    );
    host.exit_process(&a, 30_000).unwrap();
    assert_eq!(host.foreground_pids(&a), Ok(vec![shell]));
}

// --- attached_clients (#647 part 2) ---

/// A fresh session has no client, as a fresh detached tmux session has none. Each
/// `attach_client` adds one, `detach_clients` takes them all, and the count is per session.
#[test]
fn attached_clients_counts_what_attach_client_added() {
    let (host, a) = host_with("demo-c1r1");
    let b = name("demo-c2r1");
    host.ensure_session(&b, "/srv/demo").unwrap();
    assert_eq!(host.attached_clients(&a), Ok(0), "a fresh session has none");

    host.attach_client(&a).unwrap();
    host.attach_client(&a).unwrap();
    assert_eq!(host.attached_clients(&a), Ok(2));
    assert_eq!(host.attached_clients(&b), Ok(0), "another session's count");

    host.detach_clients(&a).unwrap();
    assert_eq!(host.attached_clients(&a), Ok(0));
}

/// A missing session is `pane-not-found`, from the port as from the two scenario methods,
/// and a session ended outside Holler takes its clients with it: ensured again, it has none.
#[test]
fn attached_clients_of_a_missing_session_is_pane_not_found() {
    let (host, a) = host_with("demo-c1r1");
    let missing = name("demo-c9r9");
    assert_eq!(host.attached_clients(&missing), Err(not_found(&missing)));
    assert_eq!(host.attach_client(&missing), Err(not_found(&missing)));
    assert_eq!(host.detach_clients(&missing), Err(not_found(&missing)));
    assert_eq!(host.sessions(), vec![a.clone()], "neither created a session");

    host.attach_client(&a).unwrap();
    host.end_session(&a).unwrap();
    assert_eq!(host.attached_clients(&a), Err(not_found(&a)));
    host.ensure_session(&a, "/srv/demo").unwrap();
    assert_eq!(host.attached_clients(&a), Ok(0), "it comes back with no client");
}

/// A client is not a process: `stop_owned` and `ensure_session` leave the count alone.
#[test]
fn clients_survive_stop_owned_and_a_second_ensure_session() {
    let (host, a) = host_with("demo-c1r1");
    run_sleep(&host, &a);
    host.attach_client(&a).unwrap();
    host.stop_owned(&a).unwrap();
    host.ensure_session(&a, "/srv/demo").unwrap();
    assert_eq!(host.attached_clients(&a), Ok(1));
}

/// A fault queued for `attached_clients` fails that call only and changes nothing, the call
/// is logged, and the two scenario methods are not calls through the port.
#[test]
fn a_fault_on_attached_clients_fails_that_call_only() {
    let (host, a) = host_with("demo-c1r1");
    let before = host.faults().calls();
    host.attach_client(&a).unwrap();
    assert_eq!(
        host.faults().calls(),
        before,
        "attach_client is a scenario method, not a call through the port"
    );
    let unavailable = PaneError::Unavailable {
        what: "made-up".to_owned(),
    };
    host.faults()
        .fail_next(HostOp::AttachedClients, unavailable.clone());
    assert_eq!(host.attached_clients(&a), Err(unavailable));
    assert_eq!(host.attached_clients(&a), Ok(1), "the failed call changed nothing");
    host.detach_clients(&a).unwrap();
    let calls = host.faults().calls();
    assert_eq!(
        calls[before.len()..],
        [HostOp::AttachedClients, HostOp::AttachedClients]
    );
}
