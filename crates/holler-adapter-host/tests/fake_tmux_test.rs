#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #641
//! `TmuxHost` against a fake `tmux` and a fake `kill` (#641, AC 6 and AC 7): the
//! default run, which needs no tmux. Every host is built by `common::host_on`, so no
//! test reaches a real tmux server or sends a real signal (Decision 14, W-20). The
//! fakes and their three answer modes are documented in `common/mod.rs`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{
    alive, answer, args, argv, flag_value, host, host_on, name, queue, run_ok, subcommand, Fake,
    GRACE, SAY_DIR,
};
use holler_adapter_host::TmuxSocket;
use holler_pane::{HostPort, PaneError};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::host::HostOp;

const C1: &str = "demo-c1r1";

const METHODS: [HostOp; 4] = [
    HostOp::EnsureSession,
    HostOp::Run,
    HostOp::StopOwned,
    HostOp::Ps,
];

/// The missing-session answers of Decision 8: `pane-not-found` for `run` and `ps`,
/// `Ok` for `stop_owned`.
const MISSING: [&str; 5] = [
    "can't find session: x",
    "can't find window: x",
    "no server running on /s",
    "error connecting to /s (No such file or directory)",
    "error connecting to /s (Connection refused)",
];

/// Failures that are not a missing session: `unavailable`, carrying the first line.
const OTHER: [&str; 3] = [
    "error connecting to /s (Permission denied)",
    "error connecting to /s (File name too long)",
    "server exploded: protocol mismatch\nsecond line",
];

const SENTINEL_ARGV: &str = "HLR_SENTINEL_ARGV_641";
const SENTINEL_CWD: &str = "HLR_SENTINEL_CWD_641";

/// The marker that makes [`env_probe_child`] drive the adapter (AC 6f).
const ENV_PROBE: &str = "HLR_641_ENV_PROBE_DIR";

/// The `case` pattern of `stop_owned`'s listing, the one format naming `@holler-pid`.
const STOP_LISTING: &str = "*'#{@holler-pid}'*";

/// The fake `kill` of the W-14 bullets: the group probe of `101` finds a member until
/// a `KILL` of the group is recorded, then `No such process`.
const MEMBER_UNTIL_KILL: &str = r#"case "$*" in
'-s 0 -- -101') killed '-s KILL -- -101' && gone "$4"; exit 0 ;;
'-s 0 -- '*) gone "$4" ;;
*) exit 0 ;;
esac"#;

/// Call `op` on `C1`; `run` runs `sleep 30`, `ensure_session` works in `cwd`.
fn call(host: &dyn HostPort, op: HostOp, cwd: &str) -> Result<(), PaneError> {
    let n = name(C1);
    match op {
        HostOp::EnsureSession => host.ensure_session(&n, cwd),
        HostOp::Run => host.run(&n, &argv(&["sleep", "30"])),
        HostOp::StopOwned => host.stop_owned(&n),
        HostOp::Ps => host.ps(&n).map(|_| ()),
    }
}

fn unavailable(result: Result<(), PaneError>, what: &str) -> String {
    match result {
        Err(PaneError::Unavailable { what }) => what,
        other => panic!("{what}: expected Unavailable, got {other:?}"),
    }
}

// --- AC 6a, 6b: bounded, and a missing binary ---

#[test]
fn every_method_is_bounded_and_reaps_its_child() {
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
        let pid = fs::read_to_string(fake.path("hung.pid")).unwrap();
        let pid: u32 = pid.trim().parse().unwrap();
        assert!(
            !alive(pid),
            "{}: the hung tmux {pid} was not reaped",
            op.as_str()
        );
    }
}

#[test]
fn a_missing_tmux_binary_is_unavailable_from_every_method() {
    let fake = Fake::new(&answer("", "", 0));
    let host = host(&fake).with_tmux_binary(PathBuf::from("/nonexistent/hlr-641/tmux"));
    for op in METHODS {
        unavailable(call(&host, op, &fake.dir_str()), op.as_str());
    }
}

// --- AC 6c: error mapping ---

#[test]
fn a_missing_session_is_pane_not_found_for_run_and_ps_and_ok_for_stop() {
    for err in MISSING {
        let fake = Fake::new(&answer("", &format!("{err}\n"), 1));
        let host = host(&fake);
        let ps = host.ps(&name(C1));
        assert!(
            matches!(ps, Err(PaneError::PaneNotFound { .. })),
            "ps, {err:?}: {ps:?}"
        );
        let run = host.run(&name(C1), &argv(&["sleep", "30"]));
        assert!(
            matches!(run, Err(PaneError::PaneNotFound { .. })),
            "run, {err:?}: {run:?}"
        );
        assert_eq!(host.stop_owned(&name(C1)), Ok(()), "stop_owned, {err:?}");
        assert!(fake.kills().is_empty(), "{err:?}: {:?}", fake.kills());
    }
}

#[test]
fn other_tmux_failures_are_unavailable_with_its_line_and_never_the_argv_or_cwd() {
    for err in OTHER {
        let fake = Fake::new(&answer("", &format!("{err}\n"), 1));
        let first = err.lines().next().unwrap();
        let cwd = fake.path(SENTINEL_CWD);
        fs::create_dir(&cwd).unwrap();
        let host = host(&fake);
        let results = [
            ("ps", host.ps(&name(C1)).map(|_| ())),
            ("run", host.run(&name(C1), &argv(&["prog", SENTINEL_ARGV]))),
            ("stop_owned", host.stop_owned(&name(C1))),
            (
                "ensure_session",
                host.ensure_session(&name(C1), &cwd.to_string_lossy()),
            ),
        ];
        for (method, result) in results {
            let what = unavailable(result, &format!("{method}, {err:?}"));
            assert!(what.contains(first), "{method}: {what:?} lacks {first:?}");
            assert!(!what.contains(SENTINEL_ARGV), "{method}: {what:?}");
            assert!(!what.contains(SENTINEL_CWD), "{method}: {what:?}");
        }
    }
}

// --- AC 6d: the argv passes exactly, in three spawns ---

#[test]
fn run_passes_the_argv_exactly_in_three_tmux_calls() {
    let fake = Fake::new(&run_ok());
    let sock = fake.sock_str();
    let given = ["prog", "a b", "$(id);x", "-t"];
    assert_eq!(host(&fake).run(&name(C1), &argv(&given)), Ok(()));
    let calls = fake.calls();
    assert_eq!(calls.len(), 3, "the read, new-window, the tag: {calls:?}");
    let read = [
        "-S",
        &sock,
        "list-panes",
        "-t",
        "=demo-c1r1:",
        "-F",
        "#{session_path}",
    ];
    assert_eq!(calls[0], args(&read));
    let mut window = args(&[
        "-S",
        &sock,
        "new-window",
        "-d",
        "-P",
        "-F",
        "#{pane_pid} #{window_id}",
        "-c",
        "#{session_path}",
        "-t",
        "=demo-c1r1:",
        "--",
        "env",
        "--",
    ]);
    window.extend(args(&given));
    assert_eq!(
        calls[1], window,
        "no set-option chained onto new-window (K2)"
    );
    let tag = args(&[
        "-S",
        &sock,
        "set-option",
        "-w",
        "-t",
        "@7",
        "@holler-pid",
        "4242",
        ";",
        "set-option",
        "-w",
        "-t",
        "@7",
        "remain-on-exit",
        "off",
    ]);
    assert_eq!(
        calls[2], tag,
        "the tag is its own spawn, with no new-window"
    );
    assert!(fake.kills().is_empty(), "{:?}", fake.kills());
}

#[test]
fn run_prefixes_a_one_element_argv_with_env() {
    let fake = Fake::new(&run_ok());
    assert_eq!(host(&fake).run(&name(C1), &argv(&["prog"])), Ok(()));
    let window = &fake.calls()[1];
    assert_eq!(
        window[window.len() - 4..],
        args(&["--", "env", "--", "prog"]),
        "{window:?}"
    );
}

#[test]
fn run_escapes_a_trailing_semicolon_in_every_element() {
    let fake = Fake::new(&run_ok());
    let given = ["prog", "x;", "y\\;", ";", "kill-server"];
    assert_eq!(host(&fake).run(&name(C1), &argv(&given)), Ok(()));
    let window = &fake.calls()[1];
    let program = window.iter().position(|a| a == "env").unwrap() + 2;
    let escaped = ["prog", "x\\;", "y\\\\;", "\\;", "kill-server"];
    assert_eq!(window[program..], args(&escaped), "{window:?}");
}

// --- AC 6e: exact targets ---

#[test]
fn every_target_names_the_session_exactly() {
    let body = format!(
        r#"case "$*" in
*new-window*) echo '4242 @7' ;;
{STOP_LISTING}) killed '-s TERM -- -101' || echo '101 0 101' ;;
*'#{{session_path}}'*) printf '%s\n' "$D" ;;
*list-panes*) echo '101 0' ;;
esac
exit 0"#
    );
    let fake = Fake::new(&body);
    let (host, n) = (host(&fake), name(C1));
    assert_eq!(
        host.ensure_session(&n, &fake.dir_str()),
        Ok(()),
        "the new-session path"
    );
    assert_eq!(host.ensure_session(&n, "."), Ok(()), "the has-session path");
    assert_eq!(host.run(&n, &argv(&["prog"])), Ok(()));
    assert_eq!(host.ps(&n), Ok(vec![101]));
    assert_eq!(host.stop_owned(&n), Ok(()));
    let calls = fake.calls();
    let subs: Vec<&str> = calls.iter().map(|c| subcommand(c)).collect();
    for want in ["new-session", "has-session", "new-window", "list-panes"] {
        assert!(subs.contains(&want), "no {want}: {calls:?}");
    }
    let listings = calls
        .iter()
        .filter(|c| c.iter().any(|a| a.contains("@holler-pid}")))
        .count();
    assert!(
        listings >= 2,
        "stop_owned's first listing and a poll: {calls:?}"
    );
    calls.iter().for_each(|c| assert_exact_target(c));
}

fn assert_exact_target(call: &[String]) {
    let target = flag_value(call, "-t");
    assert_ne!(target, Some(C1), "a bare name is prefix-matched: {call:?}");
    match subcommand(call) {
        "has-session" => assert_eq!(target, Some("=demo-c1r1"), "{call:?}"),
        "new-session" => {
            assert_eq!(
                flag_value(call, "-s"),
                Some(C1),
                "a name, not a target: {call:?}"
            );
            assert_eq!(target, None, "{call:?}");
        }
        "set-option" => assert_eq!(target, Some("@7"), "{call:?}"),
        _ => assert_eq!(target, Some("=demo-c1r1:"), "{call:?}"),
    }
}

// --- AC 6f: socket flags and the environment ---

#[test]
fn the_socket_and_config_flags_come_before_the_subcommand() {
    let fake = Fake::new(&answer("101 0\n", "", 0));
    let sockets = [
        (
            TmuxSocket::Path(fake.sock()),
            args(&["-S", &fake.sock_str()]),
        ),
        (
            TmuxSocket::Name("hlr-641-test".to_owned()),
            args(&["-L", "hlr-641-test"]),
        ),
        (TmuxSocket::Default, Vec::new()),
    ];
    for (i, (socket, want)) in sockets.into_iter().enumerate() {
        assert_eq!(host_on(&fake, socket).ps(&name(C1)), Ok(vec![101]));
        assert_eq!(globals(&fake.calls()[i]), want);
    }
    let config = fake.path("tmux.conf");
    let configured = host(&fake).with_config(config.clone());
    assert_eq!(configured.ps(&name(C1)), Ok(vec![101]));
    let flags = globals(&fake.calls()[3]);
    assert_eq!(flags.len(), 4, "{flags:?}");
    assert_eq!(flag_value(&flags, "-S"), Some(fake.sock_str().as_str()));
    assert_eq!(
        flag_value(&flags, "-f"),
        Some(config.to_string_lossy().as_ref())
    );
}

/// The arguments of `call` before its subcommand.
fn globals(call: &[String]) -> Vec<String> {
    let at = call.iter().position(|a| a == "list-panes").unwrap();
    call[..at].to_vec()
}

/// Run by [`tmux_gets_no_tmux_variables_and_nothing_added`] as a re-executed child;
/// returns at once otherwise.
#[test]
fn env_probe_child() {
    let Some(dir) = std::env::var_os(ENV_PROBE) else {
        return;
    };
    let fake = Fake::at(PathBuf::from(dir));
    let _ = host(&fake).ps(&name(C1));
}

#[test]
fn tmux_gets_no_tmux_variables_and_nothing_added() {
    let fake = Fake::new(
        r#"printf '%s %s %s\n' "${TMUX-unset}" "${TMUX_PANE-unset}" "${LC_ALL-unset}" > "$D/env.out"
echo '101 0'"#,
    );
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["env_probe_child", "--exact", "--test-threads=1"])
        .env(ENV_PROBE, &fake.dir)
        .env("TMUX", "/nonexistent,1,0")
        .env("TMUX_PANE", "%99")
        .env_remove("LC_ALL")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "the child test failed: {status}");
    let seen = fs::read_to_string(fake.path("env.out")).unwrap_or_default();
    assert_eq!(
        seen, "unset unset unset\n",
        "the tmux subprocess's environment"
    );
}

// --- AC 6g: bad input refused, values escaped ---

#[test]
fn ensure_session_with_a_cwd_it_cannot_create_in_only_asks_has_session() {
    for relative in [false, true] {
        for exists in [false, true] {
            let fake = if exists {
                Fake::new(&answer("", "", 0))
            } else {
                Fake::new(&answer("", "can't find session: demo-c1r1\n", 1))
            };
            let missing = fake.path("hlr-missing-641").to_string_lossy().into_owned();
            let cwd = if relative { "." } else { missing.as_str() };
            let result = host(&fake).ensure_session(&name(C1), cwd);
            if exists {
                assert_eq!(result, Ok(()), "{cwd:?} in an existing session");
            } else {
                assert!(
                    matches!(result, Err(PaneError::Usage { .. })),
                    "{cwd:?}: {result:?}"
                );
            }
            let has = args(&["-S", &fake.sock_str(), "has-session", "-t", "=demo-c1r1"]);
            assert_eq!(fake.calls(), vec![has], "{cwd:?}: never new-session");
        }
    }
}

#[test]
fn ensure_session_escapes_the_cwd_for_tmux() {
    for (dir, recorded) in [("p#S", "p##S"), ("q;", "q\\;")] {
        let fake = Fake::new(&answer("", "", 0));
        let cwd = fake.path(dir);
        fs::create_dir(&cwd).unwrap();
        assert_eq!(
            host(&fake).ensure_session(&name(C1), &cwd.to_string_lossy()),
            Ok(())
        );
        let calls = fake.calls();
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(subcommand(&calls[0]), "new-session", "{calls:?}");
        assert!(calls[0].iter().any(|a| a == "-d"), "{calls:?}");
        assert_eq!(flag_value(&calls[0], "-s"), Some(C1));
        let want = format!("{}/{recorded}", fake.dir_str());
        assert_eq!(flag_value(&calls[0], "-c"), Some(want.as_str()));
    }
}

#[test]
fn run_refuses_an_assignment_as_argv0_without_echoing_it() {
    let fake = Fake::new(SAY_DIR);
    let result = host(&fake).run(&name(C1), &argv(&["HLR_SENTINEL_641=s3cr3t", "x"]));
    let Err(error @ PaneError::Usage { .. }) = result else {
        panic!("expected Usage, got {result:?}");
    };
    let text = format!("{error} {error:?}");
    assert!(
        text.contains("argv[0]"),
        "the message names the rule: {text}"
    );
    assert!(
        !text.contains("HLR_SENTINEL_641") && !text.contains("s3cr3t"),
        "{text}"
    );
    let calls = fake.calls();
    assert_eq!(calls.len(), 1, "the read, and no new-window: {calls:?}");
    assert_eq!(subcommand(&calls[0]), "list-panes");

    let fake = Fake::new(&answer("", "can't find session: demo-c1r1\n", 1));
    let result = host(&fake).run(&name(C1), &argv(&["HLR_SENTINEL_641=s3cr3t"]));
    assert!(
        matches!(result, Err(PaneError::PaneNotFound { .. })),
        "{result:?}"
    );
}

#[test]
fn run_refuses_a_session_directory_that_is_relative_missing_or_empty() {
    let reads = [
        "printf 'hlr-rel-641\\n'",
        "printf '%s\\n' \"$D/hlr-gone-641\"",
        "exit 0",
    ];
    for read in reads {
        let fake = Fake::new(read);
        let what = unavailable(host(&fake).run(&name(C1), &argv(&["prog"])), read);
        assert!(
            !what.contains("hlr-rel-641") && !what.contains("hlr-gone-641"),
            "{what:?}"
        );
        let calls = fake.calls();
        assert_eq!(
            calls.len(),
            1,
            "{read}: the read, and no new-window: {calls:?}"
        );
        assert_eq!(subcommand(&calls[0]), "list-panes");
    }
}

#[test]
fn run_checks_the_session_before_the_argv() {
    let fake = Fake::new(&answer("", "can't find session: demo-c1r1\n", 1));
    let result = host(&fake).run(&name(C1), &argv(&[]));
    assert!(
        matches!(result, Err(PaneError::PaneNotFound { .. })),
        "{result:?}"
    );
    let fake = Fake::new(SAY_DIR);
    let result = host(&fake).run(&name(C1), &argv(&[]));
    assert!(matches!(result, Err(PaneError::Usage { .. })), "{result:?}");
    assert_eq!(fake.calls().len(), 1, "{:?}", fake.calls());
}

// --- AC 6h: stop by ownership, through the kill seam ---

/// A fake tmux whose stop listing is the snippet `listing`; every other call exits 0.
fn stop_fake(listing: &str, kill_body: &str) -> Fake {
    let body = format!("case \"$*\" in\n{STOP_LISTING})\n{listing}\n;;\nesac\nexit 0");
    Fake::with_kill(&body, kill_body)
}

fn count(kills: &[String], line: &str) -> usize {
    kills.iter().filter(|k| *k == line).count()
}

#[test]
fn stop_owned_terms_only_owned_live_groups_then_kills_a_live_survivor() {
    let listing = r#"killed '-s KILL -- -101' || echo '101 0 101'
printf '202 0 \n303 0 999\n404 1 404\n4294967295 0 4294967295\n'"#;
    let fake = stop_fake(listing, common::KILL_GONE);
    assert_eq!(host(&fake).stop_owned(&name(C1)), Ok(()));
    let kills = fake.kills();
    let terms: Vec<&String> = kills.iter().filter(|k| k.contains("TERM")).collect();
    assert_eq!(terms, ["-s TERM -- -101"], "{kills:?}");
    assert!(kills.contains(&"-s KILL -- -101".to_owned()), "{kills:?}");
    for stranger in ["202", "303", "404", "4294967295"] {
        let named = kills
            .iter()
            .any(|k| k.split(' ').any(|a| a.trim_start_matches('-') == stranger));
        assert!(!named, "{stranger} is not owned: {kills:?}");
    }
    let env = fake.kill_env();
    assert!(
        !env.is_empty() && env.iter().all(|e| e == "C"),
        "kill's LC_ALL: {env:?}"
    );
}

#[test]
fn a_tagged_pane_listed_dead_counts_as_gone() {
    let listing =
        r#"if killed '-s TERM -- -101'; then echo '101 1 101'; else echo '101 0 101'; fi"#;
    let fake = stop_fake(listing, common::KILL_GONE);
    assert_eq!(host(&fake).stop_owned(&name(C1)), Ok(()));
    let kills = fake.kills();
    assert_eq!(count(&kills, "-s TERM -- -101"), 1, "{kills:?}");
    assert!(!kills.iter().any(|k| k.contains("KILL")), "{kills:?}");
}

#[test]
fn a_group_that_outlives_its_pane_is_killed_after_the_grace() {
    let fake = stop_fake(
        "killed '-s TERM -- -101' || echo '101 0 101'",
        MEMBER_UNTIL_KILL,
    );
    assert_eq!(host(&fake).stop_owned(&name(C1)), Ok(()));
    assert_killed_once_then_probed(&fake.kills());
}

#[test]
fn a_session_that_ends_on_term_still_has_its_group_checked() {
    let listing = r#"if killed '-s TERM -- -101'; then
echo "can't find session: demo-c1r1" >&2; exit 1
fi
echo '101 0 101'"#;
    let fake = stop_fake(listing, MEMBER_UNTIL_KILL);
    assert_eq!(host(&fake).stop_owned(&name(C1)), Ok(()));
    assert_killed_once_then_probed(&fake.kills());
}

/// One `KILL` of group 101, and a group probe after it (the `Ok` waited for `No such
/// process`).
fn assert_killed_once_then_probed(kills: &[String]) {
    assert_eq!(count(kills, "-s TERM -- -101"), 1, "{kills:?}");
    assert_eq!(count(kills, "-s KILL -- -101"), 1, "{kills:?}");
    let killed = kills.iter().position(|k| k == "-s KILL -- -101").unwrap();
    let probed = kills.iter().rposition(|k| k == "-s 0 -- -101");
    assert!(
        probed.is_some_and(|p| p > killed),
        "no group probe after the KILL: {kills:?}"
    );
}

#[test]
fn a_kill_failure_other_than_no_such_process_is_unavailable() {
    let denied = r#"echo "/usr/bin/kill: ($4): Operation not permitted" >&2; exit 1"#;
    let fake = stop_fake("echo '101 0 101'", denied);
    unavailable(host(&fake).stop_owned(&name(C1)), "the TERM failed");
    let probe_denied = format!("case \"$*\" in\n'-s 0 -- '*) {denied} ;;\nesac\nexit 0");
    let fake = stop_fake(
        "killed '-s TERM -- -101' || echo '101 0 101'",
        &probe_denied,
    );
    unavailable(host(&fake).stop_owned(&name(C1)), "the group probe failed");
}

#[test]
fn one_grace_is_shared_by_every_owned_group() {
    let listing = r#"killed '-s KILL -- -101' || echo '101 0 101'
killed '-s KILL -- -505' || echo '505 0 505'"#;
    let fake = stop_fake(listing, common::KILL_GONE);
    let started = Instant::now();
    assert_eq!(host(&fake).stop_owned(&name(C1)), Ok(()));
    let took = started.elapsed();
    assert!(took < GRACE + Duration::from_secs(1), "took {took:?}");
    let kills = fake.kills();
    let first_kill = kills.iter().position(|k| k.contains("KILL")).unwrap();
    for group in ["-101", "-505"] {
        let term = kills
            .iter()
            .position(|k| *k == format!("-s TERM -- {group}"))
            .unwrap();
        assert!(term < first_kill, "every TERM before any KILL: {kills:?}");
        assert_eq!(
            count(&kills, &format!("-s KILL -- {group}")),
            1,
            "{kills:?}"
        );
    }
}

// --- AC 6i: no untagged orphan, strict parsing ---

fn tag_fails(err: &str) -> Fake {
    Fake::new(&queue(&[
        SAY_DIR.to_owned(),
        answer("4242 @7\n", "", 0),
        answer("", err, 1),
    ]))
}

#[test]
fn a_window_gone_before_its_tag_is_ok_and_signals_nothing() {
    let fake = tag_fails("can't find window: @7\n");
    assert_eq!(host(&fake).run(&name(C1), &argv(&["prog"])), Ok(()));
    assert!(fake.kills().is_empty(), "{:?}", fake.kills());
}

#[test]
fn a_failed_tag_kills_the_untagged_process() {
    let fake = tag_fails("lost server: protocol error\n");
    unavailable(
        host(&fake).run(&name(C1), &argv(&["prog"])),
        "the tag failed",
    );
    assert_eq!(fake.kills(), ["-s KILL -- -4242"]);
}

#[test]
fn a_malformed_or_out_of_range_new_window_answer_is_unavailable_and_goes_no_further() {
    let answers = [
        "abc @7\n",
        "4242 7\n",
        "4242 @7x\n",
        "",
        "0 @7\n",
        "1 @7\n",
        "4294967295 @7\n",
    ];
    for out in answers {
        let fake = Fake::new(&queue(&[SAY_DIR.to_owned(), answer(out, "", 0)]));
        unavailable(host(&fake).run(&name(C1), &argv(&["prog"])), out);
        assert_eq!(
            fake.calls().len(),
            2,
            "{out:?}: no set-option: {:?}",
            fake.calls()
        );
        assert!(fake.kills().is_empty(), "{out:?}: {:?}", fake.kills());
    }
}

#[test]
fn ps_lists_the_live_panes_and_refuses_an_out_of_range_pid() {
    let fake = Fake::new(&answer("101 0\n202 0\n303 1\n", "", 0));
    let mut pids = host(&fake).ps(&name(C1)).unwrap();
    pids.sort_unstable();
    assert_eq!(pids, [101, 202]);
    let fake = Fake::new(&answer("101 0\n4294967295 0\n", "", 0));
    let ps = host(&fake).ps(&name(C1));
    assert!(matches!(ps, Err(PaneError::Unavailable { .. })), "{ps:?}");
}

// --- AC 7, AC 9 (W-20): source guards ---

#[test]
fn no_broad_kill_in_source() {
    let mut files = Vec::new();
    collect_rs(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(!files.is_empty());
    let mut hits = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for (i, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();
            for word in ["pkill", "killall", "pgrep", "pidof"] {
                if code.contains(word) {
                    hits.push(format!("{}:{}: {line}", file.display(), i + 1));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "broad kill in production source: {hits:#?}"
    );
}

#[test]
fn hosts_are_built_by_one_helper_and_real_tmux_tests_use_private_sockets() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let read = |file: &str| fs::read_to_string(tests.join(file)).unwrap();
    let new = concat!("TmuxHost", "::new");
    let fake_side =
        read("fake_tmux_test.rs").matches(new).count() + read("common/mod.rs").matches(new).count();
    assert_eq!(
        fake_side, 1,
        "every fake-side host goes through common::host_on"
    );
    let real = read("real_tmux_test.rs");
    assert_eq!(
        real.matches(new).count(),
        1,
        "every real host goes through one helper"
    );
    for socket in [
        concat!("TmuxSocket", "::Default"),
        concat!("TmuxSocket", "::Name"),
    ] {
        assert!(!real.contains(socket), "a real-tmux test names {socket}");
    }
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}
