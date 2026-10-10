#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #642
//! The TUI side of the OpenCode adapter with no tmux and no OpenCode (#642b, AC 6 and 11d's
//! 642b clauses, AC 28-30 and 33). Every tmux call goes to the committed fake tmux,
//! `tests/fixtures/fake-tmux`, which records each call in `<sock>.calls` and answers from
//! the data files a test writes next to it; every HTTP call goes to the stub of
//! `support/stub.rs`. The only programs started are the fixture (through `/bin/sh`), so the
//! file runs in CI on Linux and macOS.

#[path = "support/fake_tmux.rs"]
mod fake_tmux;
#[path = "support/stub.rs"]
#[allow(dead_code)] // #642: shared with hermetic_test.rs
mod stub;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fake_tmux::{fixture, Fake};
use holler_adapter_opencode::tui::{respawn_args, tui_argv};
use holler_adapter_opencode::{
    OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig, TmuxSocket,
};
use holler_pane::{HarnessPort, PaneError, PaneId, PaneName};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::harness::HarnessOp;
use serde_json::{json, Value};
use stub::{on_refused_port, Stub};

/// The pane the resolver knows, and the tmux session it names.
const PANE: &str = "w9:p1";
const SESSION: &str = "demo-c1r1";
/// A pane the resolver does not know.
const OTHER: &str = "w9:p2";
/// The OpenCode binary the TUI argv names; it is never run here.
const BIN: &str = "/bin/oc";
/// The five fields of Decision 23, in order, joined by TAB.
const FORMAT: &str =
    "#{session_name}\t#{pane_dead}\t#{pane_dead_status}\t#{pane_start_command}\t#{pane_title}";
/// What `display-message -p` prints for a missing exact target: every field empty (E7).
const NO_PANE: &str = "\t\t\t\t\n";
/// What a bound may overrun by on a loaded CI runner (AC 32: at least 300 ms).
const SLACK: Duration = Duration::from_millis(500);

/// Short bounds, so the timeouts finish quickly.
fn quick() -> Timeouts {
    Timeouts {
        call: Duration::from_secs(3),
        request: Duration::from_millis(800),
        health: Duration::from_millis(300),
        boot_try: Duration::from_millis(200),
        settle: Duration::from_millis(600),
    }
}

/// One test's world: the fake tmux, the stub server, and an adapter whose `tui_session`
/// maps [`PANE`] to [`SESSION`], answers `pane-not-found` for anything else, and counts its
/// calls.
struct Rig {
    fake: Fake,
    stub: Stub,
    harness: OpenCodeHarness,
    resolved: Arc<AtomicUsize>,
}

fn rig() -> Rig {
    rig_with(quick(), fixture())
}

fn rig_with(timeouts: Timeouts, tmux_bin: PathBuf) -> Rig {
    let fake = Fake::new();
    let resolved = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&resolved);
    let config = OpenCodeConfig {
        opencode_bin: PathBuf::from(BIN),
        serve_args: Vec::new(),
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig {
            tmux_bin,
            socket: TmuxSocket::Path(fake.sock()),
        },
        workdir: Arc::new(|_: &PaneName| {
            Err(PaneError::Unavailable {
                what: "the TUI methods never ask for a workdir".to_owned(),
            })
        }),
        tui_session: Arc::new(move |pane: &PaneId| {
            counted.fetch_add(1, Ordering::SeqCst);
            if pane.as_str() == PANE {
                Ok(PaneName::parse(SESSION).unwrap())
            } else {
                Err(PaneError::PaneNotFound {
                    what: pane.as_str().to_owned(),
                })
            }
        }),
        timeouts,
    };
    Rig {
        fake,
        stub: Stub::start(),
        harness: OpenCodeHarness::new(config),
        resolved,
    }
}

impl Rig {
    fn resolved(&self) -> usize {
        self.resolved.load(Ordering::SeqCst)
    }

    /// `GET /session/<id>` answers that session, in `directory`.
    fn knows(&self, id: &str, directory: &str) {
        let body = json!({ "id": id, "directory": directory, "title": id }).to_string();
        self.stub.json("GET", &format!("/session/{id}"), 200, &body);
    }

    /// The pane is live, running a TUI attached to this stub, and titled `title`.
    fn showing(&self, title: &str) {
        self.fake.reply(
            "display-message",
            &live(&attach_line(self.stub.port), title),
        );
    }
}

/// How one scenario sets up the fake and the stub.
type Arrange = fn(&Rig);

fn pane(text: &str) -> PaneId {
    PaneId::new(text)
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|&p| p.to_owned()).collect()
}

/// The one query's token list (AC 28(a)).
fn query() -> Vec<String> {
    strings(&["display-message", "-p", "-t", "=demo-c1r1:", FORMAT])
}

/// The start command tmux prints for a TUI attached to `port` (AC 29).
fn attach_line(port: u16) -> String {
    format!(
        "env -u OPENCODE_DISABLE_TERMINAL_TITLE {BIN} attach http://127.0.0.1:{port} --dir /p \
         --session ses_A"
    )
}

fn live(start_command: &str, title: &str) -> String {
    format!("{SESSION}\t0\t\t{start_command}\t{title}\n")
}

fn dead(status: i32) -> String {
    format!("{SESSION}\t1\t{status}\tsh -c \"exit {status}\"\tOpenCode\n")
}

fn unavailable<T: std::fmt::Debug>(result: Result<T, PaneError>, what: &str) -> String {
    match result {
        Err(PaneError::Unavailable { what: message }) => message,
        other => panic!("{what}: expected unavailable, got {other:?}"),
    }
}

/// `n` as a number of its own in `message`, so that a port or a pane id that holds the
/// digit cannot satisfy it.
fn holds_number(message: &str, n: &str) -> bool {
    message.split(|c: char| !c.is_ascii_digit()).any(|m| m == n)
}

/// Risk 3: a message never echoes an argv element, a directory or a start command.
fn echoes_nothing_it_did_not_author(message: &str, what: &str) {
    for secret in [
        BIN,
        "/p#S",
        "sleep 3600",
        "OPENCODE_DISABLE_TERMINAL_TITLE",
        FORMAT,
    ] {
        assert!(
            !message.contains(secret),
            "{what}: the message echoes {secret:?}: {message:?}"
        );
    }
}

// ---- AC 33: the fixture ----

#[test]
fn ac33_the_fixture_is_an_executable_posix_sh_script() {
    let path = fixture();
    let text = std::fs::read_to_string(path).unwrap();
    assert_eq!(text.lines().next(), Some("#!/bin/sh"));
}

// ---- AC 28: shown_session ----

#[test]
fn ac28a_shown_session_reads_a_live_attach_titled_with_an_id_in_one_query() {
    let rig = rig();
    rig.showing("OC | ses_0123456789abcdefABCDEFghij");
    assert_eq!(
        rig.harness.shown_session(&pane(PANE)),
        Ok(Some("ses_0123456789abcdefABCDEFghij".to_owned()))
    );
    assert_eq!(rig.fake.calls(), Some(vec![query()]));
    assert!(rig.stub.lines().is_empty(), "it never contacts the server");
}

#[test]
fn ac28b_shown_session_is_none_whenever_it_cannot_tell() {
    let cases: [(&str, Arrange); 9] = [
        ("the home screen", |r| r.showing("OpenCode")),
        ("tmux's host name", |r| r.showing("somehost")),
        ("a cut title", |r| r.showing("OC | ses_ab…")),
        ("a dead pane", |r| r.fake.reply("display-message", &dead(1))),
        ("no attach", |r| {
            r.fake
                .reply("display-message", &live("sleep 3600", "OC | ses_A"));
        }),
        ("no tmux pane", |r| r.fake.reply("display-message", NO_PANE)),
        ("no server", |r| {
            r.fake
                .fail("display-message", 1, "no server running on /x/sock\n");
        }),
        ("no session", |r| {
            r.fake
                .fail("display-message", 1, "can't find session: demo-c1r1\n");
        }),
        ("no socket", |r| {
            let stderr = "error connecting to /x/sock (No such file or directory)\n";
            r.fake.fail("display-message", 1, stderr);
        }),
    ];
    for (why, arrange) in cases {
        let rig = rig();
        arrange(&rig);
        assert_eq!(rig.harness.shown_session(&pane(PANE)), Ok(None), "{why}");
    }
}

#[test]
fn ac28c_another_tmux_failure_is_unavailable_with_its_first_line_only() {
    let rig = rig();
    rig.fake.fail(
        "display-message",
        1,
        "protocol version mismatch (client 8, server 7)\nsecond line\n",
    );
    let message = unavailable(rig.harness.shown_session(&pane(PANE)), "a tmux failure");
    assert!(
        message.contains("protocol version mismatch (client 8, server 7)"),
        "{message:?}"
    );
    assert!(!message.contains("second line"), "{message:?}");
    assert!(!message.contains("#{session_name}"), "{message:?}");
    assert!(!message.contains("=demo-c1r1:"), "{message:?}");
}

#[test]
fn ac28d_a_tmux_that_cannot_run_is_unavailable_naming_it() {
    let rig = rig_with(quick(), PathBuf::from("/nonexistent/hlr642-tmux"));
    let message = unavailable(rig.harness.shown_session(&pane(PANE)), "a missing tmux");
    assert!(message.contains("/nonexistent/hlr642-tmux"), "{message:?}");
}

#[test]
fn ac28e_an_unknown_pane_is_the_resolvers_error_and_runs_no_tmux() {
    let rig = rig();
    assert_eq!(
        rig.harness.shown_session(&pane(OTHER)),
        Err(PaneError::PaneNotFound {
            what: OTHER.to_owned()
        })
    );
    assert_eq!(rig.fake.calls(), None);
}

// ---- AC 29: select_session ----

#[test]
fn ac29a_select_checks_the_session_switches_and_confirms_by_the_title() {
    let rig = rig();
    rig.showing("OC | ses_B");
    rig.knows("ses_B", "/p");
    rig.stub.json("POST", "/tui/select-session", 200, "true");
    assert_eq!(rig.harness.select_session(&pane(PANE), "ses_B"), Ok(()));
    let seen = rig.stub.seen();
    let lines: Vec<&str> = seen.iter().map(|s| s.line.as_str()).collect();
    assert_eq!(
        lines,
        [
            "GET /session/ses_B HTTP/1.1",
            "POST /tui/select-session HTTP/1.1"
        ]
    );
    let body: Value = serde_json::from_slice(&seen[1].body).expect("a JSON body");
    assert_eq!(body, json!({ "sessionID": "ses_B" }));
}

#[test]
fn ac29b_select_of_an_unknown_session_is_session_not_found() {
    let not_found = Err(PaneError::SessionNotFound {
        what: "ses_B".to_owned(),
    });
    // The existence check answers 404: nothing is switched.
    let first = rig();
    first.showing("OC | ses_A");
    first.stub.json("POST", "/tui/select-session", 200, "true");
    assert_eq!(
        first.harness.select_session(&pane(PANE), "ses_B"),
        not_found
    );
    assert!(
        !first.stub.lines().iter().any(|l| l.starts_with("POST")),
        "no switch after a 404: {:?}",
        first.stub.lines()
    );
    // The switch itself answers 404.
    let second = rig();
    second.showing("OC | ses_A");
    second.knows("ses_B", "/p");
    second
        .stub
        .json("POST", "/tui/select-session", 404, stub::NOT_FOUND);
    assert_eq!(
        second.harness.select_session(&pane(PANE), "ses_B"),
        not_found
    );
}

#[test]
fn ac29c_select_without_a_tui_is_unavailable_after_one_query_and_no_request() {
    for (why, stdout) in [
        ("no attach", live("sleep 3600", "OC | ses_A")),
        ("no tmux pane", NO_PANE.to_owned()),
        ("a dead pane", dead(1)),
    ] {
        let rig = rig();
        rig.fake.reply("display-message", &stdout);
        rig.knows("ses_B", "/p");
        rig.stub.json("POST", "/tui/select-session", 200, "true");
        let message = unavailable(rig.harness.select_session(&pane(PANE), "ses_B"), why);
        assert!(message.contains(PANE), "{why}: names the pane: {message:?}");
        echoes_nothing_it_did_not_author(&message, why);
        assert_eq!(rig.stub.lines(), Vec::<String>::new(), "{why}: no request");
        assert_eq!(rig.fake.calls(), Some(vec![query()]), "{why}: one query");
    }
}

#[test]
fn ac29d_a_switch_the_title_never_shows_times_out_within_settle() {
    let rig = rig();
    rig.showing("OC | ses_A");
    rig.knows("ses_B", "/p");
    rig.stub.json("POST", "/tui/select-session", 200, "true");
    let start = Instant::now();
    assert_eq!(
        rig.harness.select_session(&pane(PANE), "ses_B"),
        Err(PaneError::Timeout {
            op: HarnessOp::SelectSession.as_str().to_owned()
        })
    );
    let took = start.elapsed();
    assert!(took < quick().settle + SLACK, "took {took:?}");
}

#[test]
fn ac29e_a_switch_answered_by_the_web_app_is_unavailable() {
    let rig = rig();
    rig.showing("OC | ses_B");
    rig.knows("ses_B", "/p");
    rig.stub.html("POST", "/tui/select-session");
    unavailable(
        rig.harness.select_session(&pane(PANE), "ses_B"),
        "a 200 HTML switch reply",
    );
}

// ---- AC 30: attach_tui ----

/// The stub knows `ses_A` in a directory that needs both escapes.
fn attachable() -> Rig {
    let rig = rig();
    rig.knows("ses_A", "/p#S;");
    rig
}

#[test]
fn ac30a_attach_keeps_the_pane_respawns_the_tui_escaped_and_confirms_by_the_title() {
    let rig = attachable();
    rig.showing("OC | ses_A");
    assert_eq!(
        rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
        Ok(())
    );
    let calls = rig.fake.calls().expect("tmux was called");
    assert_eq!(
        calls.first(),
        Some(&strings(&[
            "set-option",
            "-p",
            "-t",
            "=demo-c1r1:",
            "remain-on-exit",
            "on"
        ]))
    );
    let respawn = calls.get(1).expect("a second call");
    let session = PaneName::parse(SESSION).unwrap();
    let argv = tui_argv(BIN, &ProcessEnv::Inherit, rig.stub.port, "/p#S;", "ses_A");
    assert_eq!(respawn, &respawn_args(&session, "/p#S;", &argv));
    assert_eq!(
        respawn.get(..7),
        Some(
            &strings(&[
                "respawn-pane",
                "-k",
                "-t",
                "=demo-c1r1:",
                "-c",
                r"/p##S\;",
                "--"
            ])[..]
        )
    );
    let dir = respawn.iter().position(|a| a == "--dir").expect("--dir");
    assert_eq!(respawn[dir + 1], r"/p#S\;");
    let polls = &calls[2..];
    assert!(!polls.is_empty(), "the title is observed: {calls:?}");
    assert!(polls.iter().all(|c| *c == query()), "{calls:?}");
}

#[test]
fn ac30b_a_missing_tmux_session_is_unavailable_and_nothing_is_respawned() {
    let rig = attachable();
    rig.fake
        .fail("set-option", 1, "no such pane: =demo-c1r1:\n");
    rig.showing("OC | ses_A");
    let message = unavailable(
        rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
        "a missing tmux session",
    );
    assert!(message.contains(PANE), "names the pane: {message:?}");
    let calls = rig.fake.calls().unwrap_or_default();
    assert!(
        !calls
            .iter()
            .any(|c| c.first().map(String::as_str) == Some("respawn-pane")),
        "{calls:?}"
    );
}

#[test]
fn ac30c_a_tui_that_exits_is_unavailable_with_its_status() {
    let rig = attachable();
    rig.fake.reply("display-message", &dead(3));
    let message = unavailable(
        rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
        "a TUI that exits 3",
    );
    assert!(
        holds_number(&message, "3"),
        "holds the status 3: {message:?}"
    );
    echoes_nothing_it_did_not_author(&message, "a TUI that exits 3");
}

/// Behaviour, `attach_tui`: when the TUI dies, the session is asked for again, and a session
/// that went away since the first check is `session-not-found`, not the exit status.
#[test]
fn ac30c_a_tui_that_exits_because_its_session_went_away_is_session_not_found() {
    let rig = rig();
    let known = json!({ "id": "ses_A", "directory": "/p" }).to_string();
    rig.stub.json_once("GET", "/session/ses_A", 200, &known);
    rig.fake.reply("display-message", &dead(1));
    assert_eq!(
        rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
        Err(PaneError::SessionNotFound {
            what: "ses_A".to_owned()
        })
    );
}

#[test]
fn ac30d_a_session_reply_without_a_directory_is_unavailable_and_touches_nothing() {
    let rig = rig();
    rig.stub
        .json("GET", "/session/ses_A", 200, r#"{"id":"ses_A"}"#);
    rig.showing("OC | ses_A");
    unavailable(
        rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
        "a session with no directory",
    );
    assert_eq!(rig.resolved(), 0, "the resolver is not called");
    assert_eq!(rig.fake.calls(), None, "no tmux call");
}

#[test]
fn ac30_attach_of_an_unknown_session_is_session_not_found_and_touches_nothing() {
    // 404 is OpenCode's answer for an unknown id; a 400 (a malformed id) reads the same.
    for status in [404, 400] {
        let rig = rig();
        rig.stub
            .json("GET", "/session/ses_A", status, stub::NOT_FOUND);
        rig.showing("OC | ses_A");
        assert_eq!(
            rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
            Err(PaneError::SessionNotFound {
                what: "ses_A".to_owned()
            }),
            "the existence check answered {status}"
        );
        assert_eq!(rig.resolved(), 0, "{status}: the resolver is not called");
        assert_eq!(rig.fake.calls(), None, "{status}: no tmux call");
    }
}

#[test]
fn ac30_an_attach_the_title_never_confirms_times_out_within_settle() {
    let rig = attachable();
    rig.showing("OpenCode");
    let start = Instant::now();
    assert_eq!(
        rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
        Err(PaneError::Timeout {
            op: HarnessOp::AttachTui.as_str().to_owned()
        })
    );
    let took = start.elapsed();
    assert!(took < quick().settle + SLACK, "took {took:?}");
}

/// I3 (F's design decision 1): the watch confirms an attach only for a TUI attached to the
/// requested port. Two servers share one data directory, so a TUI of another server can show
/// the same id; that is not this attach, and the answer is `timeout`, not `Ok`.
#[test]
fn ac30_a_tui_of_another_server_showing_the_session_does_not_confirm_the_attach() {
    let rig = attachable();
    let other = rig.stub.port.checked_add(1).unwrap_or(1);
    rig.fake
        .reply("display-message", &live(&attach_line(other), "OC | ses_A"));
    let start = Instant::now();
    assert_eq!(
        rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
        Err(PaneError::Timeout {
            op: HarnessOp::AttachTui.as_str().to_owned()
        })
    );
    let took = start.elapsed();
    assert!(took < quick().settle + SLACK, "took {took:?}");
}

#[test]
fn ac30e_each_method_past_its_deadline_times_out_with_its_own_op() {
    let spent = Timeouts {
        call: Duration::ZERO,
        ..quick()
    };
    let rig = rig_with(spent, fixture());
    rig.knows("ses_A", "/p");
    rig.showing("OC | ses_A");
    let p = pane(PANE);
    let timeout = |op: HarnessOp| -> Result<(), PaneError> {
        Err(PaneError::Timeout {
            op: op.as_str().to_owned(),
        })
    };
    assert_eq!(
        rig.harness.attach_tui(&p, rig.stub.port, "ses_A"),
        timeout(HarnessOp::AttachTui)
    );
    assert_eq!(
        rig.harness.select_session(&p, "ses_A"),
        timeout(HarnessOp::SelectSession)
    );
    assert_eq!(
        rig.harness.shown_session(&p).map(drop),
        timeout(HarnessOp::ShownSession)
    );
}

#[test]
fn ac30f_no_method_answers_not_implemented_for_any_input() {
    let scenarios: [(&str, Arrange); 5] = [
        ("a live TUI on ses_A", |r| r.showing("OC | ses_A")),
        ("the home screen", |r| r.showing("OpenCode")),
        ("no tmux pane", |r| r.fake.reply("display-message", NO_PANE)),
        ("a dead pane", |r| r.fake.reply("display-message", &dead(3))),
        ("a tmux failure", |r| {
            r.fake.fail("display-message", 1, "boom\n")
        }),
    ];
    for (why, arrange) in scenarios {
        for target in [PANE, OTHER] {
            let rig = rig();
            rig.knows("ses_A", "/p");
            rig.stub.json("POST", "/tui/select-session", 200, "true");
            arrange(&rig);
            let p = pane(target);
            let answers = [
                (
                    "attach_tui",
                    rig.harness.attach_tui(&p, rig.stub.port, "ses_A"),
                ),
                ("select_session", rig.harness.select_session(&p, "ses_A")),
                ("shown_session", rig.harness.shown_session(&p).map(drop)),
            ];
            for (method, answer) in answers {
                assert!(
                    !matches!(answer, Err(PaneError::NotImplemented)),
                    "{method} on {target} with {why}: {answer:?}"
                );
            }
        }
    }
}

// ---- AC 6 and 11d: the 642b clauses ----

#[test]
fn ac6_attach_to_an_unbound_port_is_unavailable_before_the_resolver() {
    let (_, (answer, resolved)) = on_refused_port(
        |port| {
            let rig = rig();
            (
                rig.harness.attach_tui(&pane(PANE), port, "ses_A"),
                rig.resolved(),
            )
        },
        |(answer, _)| matches!(answer, Err(PaneError::Unavailable { .. })),
    );
    unavailable(answer, "attach_tui to an unbound port");
    assert_eq!(resolved, 0, "the resolver is not called");
}

#[test]
fn ac11d_attach_to_a_session_reply_that_is_not_that_session_is_unavailable() {
    for (why, arrange) in [
        (
            "the web app's 200 HTML",
            (|r: &Rig| r.stub.html("GET", "/session/ses_A")) as fn(&Rig),
        ),
        ("another session's id", |r: &Rig| {
            r.stub.json(
                "GET",
                "/session/ses_A",
                200,
                r#"{"id":"ses_other","directory":"/p"}"#,
            );
        }),
    ] {
        let rig = rig();
        arrange(&rig);
        rig.showing("OC | ses_A");
        let message = unavailable(
            rig.harness.attach_tui(&pane(PANE), rig.stub.port, "ses_A"),
            why,
        );
        assert!(
            !message.contains(['\n', '\r']),
            "{why}: one line: {message:?}"
        );
        assert!(message.len() <= 200, "{why}: {} bytes", message.len());
        assert!(
            message.contains("/session/"),
            "{why}: names the route: {message:?}"
        );
        assert!(
            message.contains("200"),
            "{why}: names the status: {message:?}"
        );
        assert_eq!(rig.resolved(), 0, "{why}: the resolver is not called");
        assert_eq!(rig.fake.calls(), None, "{why}: no tmux call");
    }
}
