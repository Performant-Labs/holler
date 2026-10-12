#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #831
//! `TmuxHost::foreground_pids` against the fake `tmux` and the fake `ps` (#831, brief
//! "`foreground_pids`: mechanism"): the pids of the foreground process group of every live pane
//! of the session. tmux has the pane's tty but no foreground pid, so the adapter lists the
//! panes (`#{pane_dead} #{pane_tty}`, one `list-panes -s` call, parsed as strictly as
//! `foreground_commands`' listing) and asks `ps -t <tty> -o pid= -o pgid= -o tpgid=` once per
//! live pane. A process whose group id equals the tty's foreground group id is in the answer, so
//! a `claude` typed into a shell is found (the pane's first process, `ps`'s `pane_pid`, is the
//! shell) and a member of the group that is not its leader is found too. Numbers only, never an
//! argv (I7).
//!
//! The answer is all or nothing: any malformed line, failing `ps` or missed deadline fails the
//! whole call (`unavailable` or `timeout`), never a partial list. The bound and the missing
//! binary are `foreground_pids_bound_test.rs`.
//!
//! The fakes answer from canned text, so these tests pin what the adapter reads and runs, not
//! how it spells its tmux format. `ps`'s answer is by tty: the fake `ps` sees `-t` as `$1` and
//! the tty as `$2`.

mod common;

use common::{answer, args, flag_value, host, name, queue, subcommand, Fake};
use holler_adapter_host::TmuxHost;
use holler_pane::{HostPort, PaneError};

const C1: &str = "demo-c1r1";

/// A fake `tmux` that prints `lines` verbatim for any call (the listing).
fn listing(lines: &[&str]) -> String {
    let mut body = String::new();
    for line in lines {
        body.push_str(&format!("printf '%s\\n' {}\n", common::sq(line)));
    }
    body.push_str("exit 0");
    body
}

/// A fake `ps` that answers by its tty argument (`$2`) with the rows given, one `pid pgid tpgid`
/// string per line, and exits 1 with no output for any tty it does not know (procps's "no
/// process on this tty").
fn ps_by_tty(ttys: &[(&str, &[&str])]) -> String {
    let mut body = String::from("case \"$2\" in\n");
    for (tty, rows) in ttys {
        body.push_str(&format!("{})\n", common::sq(tty)));
        for row in *rows {
            body.push_str(&format!("printf '%s\\n' {}\n", common::sq(row)));
        }
        body.push_str("exit 0 ;;\n");
    }
    body.push_str("*) exit 1 ;;\nesac");
    body
}

/// The host on `fake`, with `ps` the fake `ps`.
fn host_with_ps(fake: &Fake) -> TmuxHost {
    host(fake).with_ps_binary(fake.ps())
}

/// The arguments the adapter runs `ps` with for the tty named `tty` (no `/dev/`).
fn ps_args(tty: &str) -> Vec<String> {
    args(&["-t", tty, "-o", "pid=", "-o", "pgid=", "-o", "tpgid="])
}

// --- the typed-in shell: the case the mechanism exists for ---------------------------------

/// The fleet's real shape (D2): the pane's first process is a shell (pid 500), and the `claude`
/// a person typed into it (pid 900, the leader of the tty's foreground group) is not the first
/// process. The shell is in the background while `claude` runs (its group, 500, is not the
/// foreground group 900), so it is not in the answer, and the answer is `claude`'s pid.
/// The calls are exactly one `list-panes -s` listing and one `ps`, with `ps` told the tty
/// without `/dev/`, numbers only, and `LC_ALL=C`.
#[test]
fn a_program_typed_into_the_shell_is_found_and_the_shell_is_not() {
    let fake = Fake::with_ps(
        &listing(&["0 /dev/pts/3"]),
        &ps_by_tty(&[("pts/3", &["  500   500   900", "  900   900   900"])]),
    );
    assert_eq!(
        host_with_ps(&fake).foreground_pids(&name(C1)),
        Ok(vec![900])
    );

    let tmux = fake.calls();
    assert_eq!(tmux.len(), 1, "one tmux call: {tmux:?}");
    assert_eq!(subcommand(&tmux[0]), "list-panes", "{:?}", tmux[0]);
    assert!(
        tmux[0].iter().any(|a| a == "-s"),
        "every window: {:?}",
        tmux[0]
    );
    assert_eq!(
        flag_value(&tmux[0], "-t"),
        Some("=demo-c1r1:"),
        "{:?}",
        tmux[0]
    );
    let format = flag_value(&tmux[0], "-F").expect("a -F format");
    assert!(
        format.contains("#{pane_dead}") && format.contains("#{pane_tty}"),
        "{format:?}"
    );
    assert!(
        !format.contains("start_command"),
        "no command line (I7): {format:?}"
    );

    assert_eq!(
        fake.ps_calls(),
        vec![ps_args("pts/3")],
        "one ps, numbers only"
    );
    assert_eq!(fake.ps_env(), ["C"], "LC_ALL=C");
}

/// A member of the foreground group that is not its leader is in the answer: `claude` under a
/// wrapper that exited, or its worker processes. The group's id is the tty's `tpgid` (900), the
/// leader (900) is gone, and 901 and 902 are members.
#[test]
fn a_member_of_the_foreground_group_that_is_not_its_leader_is_found() {
    let fake = Fake::with_ps(
        &listing(&["0 /dev/pts/3"]),
        &ps_by_tty(&[(
            "pts/3",
            &["500 500 900", "901 900 900", "902 900 900", "777 777 900"],
        )]),
    );
    assert_eq!(
        host_with_ps(&fake).foreground_pids(&name(C1)),
        Ok(vec![901, 902])
    );
}

// --- several panes, a dead pane, no controlling terminal --------------------------------------

/// Two live panes: the answer is every pane's group together, ascending whatever the order tmux
/// lists the panes or `ps` prints the rows in, each pid once; `ps` is asked once per pane, in
/// tmux's order.
#[test]
fn two_panes_answer_together_ascending_and_ps_runs_once_per_pane_in_tmux_order() {
    let fake = Fake::with_ps(
        &listing(&["0 /dev/pts/4", "0 /dev/pts/3"]),
        &ps_by_tty(&[
            ("pts/4", &["410 400 400", "400 400 400"]),
            ("pts/3", &["910 900 900", "900 900 900", "905 900 900"]),
        ]),
    );
    assert_eq!(
        host_with_ps(&fake).foreground_pids(&name(C1)),
        Ok(vec![400, 410, 900, 905, 910])
    );
    assert_eq!(
        fake.ps_calls(),
        vec![ps_args("pts/4"), ps_args("pts/3")],
        "tmux's order"
    );
}

/// A dead pane is dropped, as `ps` and `foreground_commands` drop it: `ps` is not asked about
/// its tty.
#[test]
fn a_dead_pane_is_dropped_and_its_tty_is_not_asked() {
    let fake = Fake::with_ps(
        &listing(&["1 /dev/pts/5", "0 /dev/pts/3"]),
        &ps_by_tty(&[("pts/5", &["111 111 111"]), ("pts/3", &["900 900 900"])]),
    );
    assert_eq!(
        host_with_ps(&fake).foreground_pids(&name(C1)),
        Ok(vec![900])
    );
    assert_eq!(fake.ps_calls(), vec![ps_args("pts/3")]);
}

/// `tpgid` -1 means no controlling terminal and matches nothing, not even a process whose group
/// id is -1 or whose own pid is 1.
#[test]
fn a_tty_with_no_foreground_group_matches_nothing() {
    let fake = Fake::with_ps(
        &listing(&["0 /dev/pts/3"]),
        &ps_by_tty(&[("pts/3", &["500 500 -1", "1 1 -1"])]),
    );
    assert_eq!(host_with_ps(&fake).foreground_pids(&name(C1)), Ok(vec![]));
}

/// `ps` of a tty with no process exits non-zero with nothing on either output (procps's "no
/// process on this tty"): an empty pane, not a failure; the other pane still answers.
#[test]
fn an_empty_ps_answer_is_an_empty_pane_not_a_failure() {
    let fake = Fake::with_ps(
        &listing(&["0 /dev/pts/4", "0 /dev/pts/3"]),
        &ps_by_tty(&[("pts/3", &["900 900 900"])]),
    );
    assert_eq!(
        host_with_ps(&fake).foreground_pids(&name(C1)),
        Ok(vec![900])
    );
    let fake = Fake::with_ps(&listing(&["0 /dev/pts/4"]), &ps_by_tty(&[]));
    assert_eq!(host_with_ps(&fake).foreground_pids(&name(C1)), Ok(vec![]));
}

/// A session with no live pane asks `ps` nothing and answers an empty list.
#[test]
fn a_session_with_no_live_pane_answers_nothing_and_runs_no_ps() {
    let fake = Fake::with_ps(&listing(&["1 /dev/pts/5"]), &ps_by_tty(&[]));
    assert_eq!(host_with_ps(&fake).foreground_pids(&name(C1)), Ok(vec![]));
    assert_eq!(fake.ps_calls(), Vec::<Vec<String>>::new());
}

// --- the tty reaches ps as a bare name; a bad tty never reaches it ------------------------

/// Linux's `/dev/pts/3` and macOS's `/dev/ttys004` each reach `ps` as the name `ps -t` takes:
/// `pts/3` and `ttys004`.
#[test]
fn a_linux_and_a_macos_tty_reach_ps_as_the_names_ps_takes() {
    for (listed, given) in [("/dev/pts/3", "pts/3"), ("/dev/ttys004", "ttys004")] {
        let fake = Fake::with_ps(
            &listing(&[&format!("0 {listed}")]),
            &ps_by_tty(&[(given, &["900 900 900"])]),
        );
        assert_eq!(
            host_with_ps(&fake).foreground_pids(&name(C1)),
            Ok(vec![900]),
            "{listed}"
        );
        assert_eq!(fake.ps_calls(), vec![ps_args(given)], "{listed}");
    }
}

/// The listing is parsed as strictly as `foreground_commands`' and checked before any `ps`: a
/// dead flag that is neither `0` nor `1`, a tty that is not `/dev/` and a non-empty relative path
/// of `[A-Za-z0-9._/-]`, one with a `..` segment or starting with `-` (an option to `ps`), a
/// line that is not two fields, and an empty tty are each `unavailable` for the whole answer,
/// and `ps` is never run, even for the good pane that comes first.
#[test]
fn a_malformed_listing_is_unavailable_and_runs_no_ps() {
    for bad in [
        "x /dev/pts/4",
        "2 /dev/pts/4",
        "0 pts/4",
        "0 /dev/",
        "0 /dev",
        "0 /dev/../etc/passwd",
        "0 /dev/pts/../4",
        "0 /dev/-x",
        "0 /dev/pts/4;x",
        "0 /dev/pts/4 extra",
        "0",
        "",
    ] {
        let fake = Fake::with_ps(
            &listing(&["0 /dev/pts/3", bad]),
            &ps_by_tty(&[("pts/3", &["900 900 900"])]),
        );
        let got = host_with_ps(&fake).foreground_pids(&name(C1));
        assert!(
            matches!(got, Err(PaneError::Unavailable { .. })),
            "{bad:?}: {got:?}"
        );
        assert_eq!(fake.ps_calls().len(), 0, "{bad:?}: no ps for a bad listing");
    }
}

// --- ps failing -----------------------------------------------------------------------------

/// A `ps` that exits non-zero with anything on stderr is `unavailable`, carrying its first
/// stderr line and no other, and fails the whole answer even though the first pane answered.
#[test]
fn another_ps_failure_is_unavailable_with_its_first_line_and_fails_the_whole_answer() {
    let fake = Fake::with_ps(
        &listing(&["0 /dev/pts/3", "0 /dev/pts/4"]),
        &queue(&[
            answer("900 900 900\n", "", 0),
            answer("", "ps: unknown option -- t\nsecond line\n", 1),
        ]),
    );
    match host_with_ps(&fake).foreground_pids(&name(C1)) {
        Err(PaneError::Unavailable { what }) => {
            assert!(what.contains("ps: unknown option -- t"), "{what:?}");
            assert!(!what.contains("second line"), "{what:?}");
        }
        other => panic!("expected unavailable, got {other:?}"),
    }
}

/// A `ps` line that is not three decimal fields fails the whole answer.
#[test]
fn a_ps_line_that_is_not_three_decimal_fields_is_unavailable() {
    for bad in [
        "900 900",
        "900 900 900 900",
        "a b c",
        "900 900 x",
        "9x0 900 900",
        "",
    ] {
        let fake = Fake::with_ps(
            &listing(&["0 /dev/pts/3"]),
            &ps_by_tty(&[("pts/3", &["500 500 900", bad])]),
        );
        let got = host_with_ps(&fake).foreground_pids(&name(C1));
        assert!(
            matches!(got, Err(PaneError::Unavailable { .. })),
            "{bad:?}: {got:?}"
        );
    }
}

// --- tmux failing: the same answers as foreground_commands ----------------------------------

/// A session that does not exist is `pane-not-found`, and `ps` is not run.
#[test]
fn a_missing_session_is_pane_not_found_and_runs_no_ps() {
    for err in [
        "can't find session: demo-c1r1",
        "can't find window: demo-c1r1",
        "no server running on /s",
    ] {
        let fake = Fake::with_ps(&answer("", &format!("{err}\n"), 1), &ps_by_tty(&[]));
        let got = host_with_ps(&fake).foreground_pids(&name(C1));
        assert!(
            matches!(got, Err(PaneError::PaneNotFound { .. })),
            "{err:?}: {got:?}"
        );
        assert_eq!(fake.ps_calls().len(), 0, "{err:?}");
    }
}

/// Another tmux failure is `unavailable` with its first line, as for the other methods.
#[test]
fn another_tmux_failure_is_unavailable_with_its_first_line() {
    let fake = Fake::with_ps(
        &answer("", "server exploded: protocol mismatch\nsecond line\n", 1),
        &ps_by_tty(&[]),
    );
    match host_with_ps(&fake).foreground_pids(&name(C1)) {
        Err(PaneError::Unavailable { what }) => {
            assert!(
                what.contains("server exploded: protocol mismatch"),
                "{what:?}"
            );
            assert!(!what.contains("second line"), "{what:?}");
        }
        other => panic!("expected unavailable, got {other:?}"),
    }
}
