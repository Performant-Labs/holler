#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #831
//! The bound and the missing binaries of `TmuxHost::foreground_pids` (#831): the port call has
//! one deadline, shared by its one tmux call and every `ps` it runs, so a hung `ps`, or `ps`
//! calls that together outlast the deadline, make the whole call `timeout` (the `op` being the
//! port's `host.foreground_pids`), never a partial list; each child that hung is reaped; and a
//! missing `tmux` or `ps` binary is `unavailable`, its text naming the program and none of its
//! arguments (W-2).
//!
//! `fake_tmux_test.rs` pins the same two rules for the other six methods (`METHODS` there is
//! six rows, and the file is near the 900 lines `scripts/lint.sh` allows), so this file carries
//! its own list of the seven operations for the shared rules and the `ps` cases besides.

mod common;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use common::{alive, answer, argv, host, name, queue, Fake};
use holler_pane::{HostPort, PaneError};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::host::HostOp;

const C1: &str = "demo-c1r1";

/// Every method of the port, `foreground_pids` included.
const METHODS: [HostOp; 7] = [
    HostOp::EnsureSession,
    HostOp::Run,
    HostOp::StopOwned,
    HostOp::Ps,
    HostOp::ForegroundCommands,
    HostOp::ForegroundPids,
    HostOp::AttachedClients,
];

fn call(host: &dyn HostPort, op: HostOp, cwd: &str) -> Result<(), PaneError> {
    let n = name(C1);
    match op {
        HostOp::EnsureSession => host.ensure_session(&n, cwd),
        HostOp::Run => host.run(&n, cwd, &argv(&["sleep", "30"])),
        HostOp::StopOwned => host.stop_owned(&n),
        HostOp::Ps => host.ps(&n).map(|_| ()),
        HostOp::ForegroundCommands => host.foreground_commands(&n).map(|_| ()),
        HostOp::ForegroundPids => host.foreground_pids(&n).map(|_| ()),
        HostOp::AttachedClients => host.attached_clients(&n).map(|_| ()),
    }
}

/// A fake `tmux` that lists `ttys` as live panes, whatever it is asked.
fn listing(ttys: &[&str]) -> String {
    let mut body = String::new();
    for tty in ttys {
        body.push_str(&format!("echo '0 {tty}'\n"));
    }
    body.push_str("exit 0");
    body
}

fn assert_timeout_then_reaped(result: Result<(), PaneError>, took: Duration, pid_file: PathBuf) {
    match result {
        Err(PaneError::Timeout { op }) => assert_eq!(op, HostOp::ForegroundPids.as_str()),
        other => panic!("expected Timeout, got {other:?}"),
    }
    assert!(took < Duration::from_millis(1_500), "took {took:?}");
    let pid: u32 = fs::read_to_string(pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(!alive(pid), "the hung child {pid} was not reaped");
}

// --- every method, as fake_tmux_test pins six of them -------------------------------------

#[test]
fn every_method_of_the_seven_is_bounded_and_reaps_its_child() {
    for op in METHODS {
        let fake = Fake::new("echo $$ > \"$D/hung.pid\"\nexec sleep 30");
        let host = host(&fake).with_timeout(Duration::from_millis(200));
        let started = Instant::now();
        let result = call(&host, op, &fake.dir_str());
        let took = started.elapsed();
        match result {
            Err(PaneError::Timeout { op: got }) => assert_eq!(got, op.as_str()),
            other => panic!("{}: expected Timeout, got {other:?}", op.as_str()),
        }
        assert!(
            took < Duration::from_millis(1_200),
            "{}: took {took:?}",
            op.as_str()
        );
        let pid: u32 = fs::read_to_string(fake.path("hung.pid"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(
            !alive(pid),
            "{}: the hung tmux {pid} was not reaped",
            op.as_str()
        );
    }
}

#[test]
fn a_missing_tmux_binary_is_unavailable_from_every_method_of_the_seven() {
    let fake = Fake::new(&answer("", "", 0));
    let host = host(&fake).with_tmux_binary(PathBuf::from("/nonexistent/hlr-831/tmux"));
    for op in METHODS {
        match call(&host, op, &fake.dir_str()) {
            Err(PaneError::Unavailable { .. }) => {}
            other => panic!("{}: expected Unavailable, got {other:?}", op.as_str()),
        }
    }
}

// --- ps: one deadline for the whole call ---------------------------------------------------

/// A `ps` that never answers: the call is `timeout` within the bound, with the port's `op`, and
/// the hung `ps` is killed and reaped.
#[test]
fn a_hung_ps_is_a_timeout_and_is_reaped() {
    let fake = Fake::with_ps(
        &listing(&["/dev/pts/3"]),
        "echo $$ > \"$D/hung.pid\"\nexec sleep 30",
    );
    let host = host(&fake)
        .with_ps_binary(fake.ps())
        .with_timeout(Duration::from_millis(300));
    let started = Instant::now();
    let result = host.foreground_pids(&name(C1)).map(|_| ());
    assert_timeout_then_reaped(result, started.elapsed(), fake.path("hung.pid"));
}

/// All or nothing: the first pane's `ps` answers, the second's outlives the deadline. The call
/// is `timeout`, not the first pane's pids.
#[test]
fn a_deadline_reached_on_the_second_pane_is_a_timeout_and_no_partial_list() {
    let fake = Fake::with_ps(
        &listing(&["/dev/pts/3", "/dev/pts/4"]),
        &queue(&[
            answer("900 900 900\n", "", 0),
            "echo $$ > \"$D/hung.pid\"\nexec sleep 30".to_owned(),
        ]),
    );
    let host = host(&fake)
        .with_ps_binary(fake.ps())
        .with_timeout(Duration::from_millis(400));
    let started = Instant::now();
    let result = host.foreground_pids(&name(C1)).map(|_| ());
    assert_timeout_then_reaped(result, started.elapsed(), fake.path("hung.pid"));
    assert_eq!(
        fake.ps_calls().len(),
        2,
        "the first pane was asked, then the second"
    );
}

/// The deadline is one for the whole call, not one per `ps`: three panes whose `ps` each takes
/// 0.2 s, under a 0.5 s bound that each of them meets alone, outlast it together; nothing is
/// spawned once the deadline has passed.
#[test]
fn the_deadline_is_shared_by_every_ps_of_the_call() {
    let fake = Fake::with_ps(
        &listing(&["/dev/pts/3", "/dev/pts/4", "/dev/pts/5", "/dev/pts/6"]),
        "sleep 0.2\necho '900 900 900'\nexit 0",
    );
    let host = host(&fake)
        .with_ps_binary(fake.ps())
        .with_timeout(Duration::from_millis(500));
    let started = Instant::now();
    let result = host.foreground_pids(&name(C1));
    let took = started.elapsed();
    match result {
        Err(PaneError::Timeout { op }) => assert_eq!(op, HostOp::ForegroundPids.as_str()),
        other => panic!("expected Timeout, got {other:?}"),
    }
    assert!(took < Duration::from_millis(1_500), "took {took:?}");
    assert!(
        fake.ps_calls().len() < 4,
        "no ps is spawned once the deadline has passed: {} calls",
        fake.ps_calls().len()
    );
}

// --- a missing ps binary ---------------------------------------------------------------------

/// A missing `ps` is `unavailable`, naming the program and no argument (not the tty).
#[test]
fn a_missing_ps_binary_is_unavailable_and_names_no_argument() {
    let fake = Fake::new(&listing(&["/dev/pts/3"]));
    let host = host(&fake).with_ps_binary(PathBuf::from("/nonexistent/hlr-831/ps"));
    match host.foreground_pids(&name(C1)) {
        Err(PaneError::Unavailable { what }) => {
            assert!(
                what.contains("/nonexistent/hlr-831/ps"),
                "names the program: {what:?}"
            );
            assert!(!what.contains("pts/3"), "never an argument: {what:?}");
        }
        other => panic!("expected unavailable, got {other:?}"),
    }
}
