#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #850
//! A Claude Code row end to end (#850, Acceptance 4; ADR 0014 points 7 to 11): a real hub, a real
//! `holler body run` whose config has a `harness = "claude"`, `mode = "attach"` row, and a program
//! that stands for `claude` (`claude_rig`): `agents --json` prints a scripted process, and the
//! process's inbox is a listener the test reads. Nothing runs a real Claude Code, reads a login
//! or touches a pane (I7).
//!
//! The pane gate for a typed address is not exercised here; Acceptance 5
//! (`say_pane_scenario_test.rs`) is the pane gate and `say --pane`.
//!
//! Readiness is observed: the roster row, the inbox line, the program's run log. The fake process
//! moves through `busy` and `idle` only after the test saw the body list the processes twice.

#[path = "claude_rig/mod.rs"]
mod claude_rig;
mod support;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use claude_rig::{claude_row, frame_of, parsed, FakeProgram, Process, LOGIN_VARIABLES, PID, SENTINEL_KEY, SESSION};
use serde_json::Value;
use support::{holler_cmd, mint_token, stub_acp_bin, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A hub, the program, and a body whose config holds the Claude row `c1r1` (and `extra` rows).
struct Rig {
    state: StateDir,
    hub: Option<Hub>,
    body: Option<Body>,
    program: FakeProgram,
}

impl Rig {
    fn start(processes: &[Process<'_>], extra_rows: &str) -> Rig {
        let state = StateDir::new();
        let hub = Hub::start(&state);
        let (token, secret) = mint_token(&state, "kiwi");
        support::join(&state, &state, &hub.ws_url(), &token, &secret);
        let program = FakeProgram::new(&state);
        program.set_processes(processes);
        for process in processes {
            program.register(process.pid);
        }
        std::fs::create_dir_all(state.body()).unwrap();
        let config = state.body().join("sessions.toml");
        std::fs::write(&config, format!("{}{extra_rows}", claude_row("c1r1", SESSION))).unwrap();
        let env = program.env();
        let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let body = Body::start_with_env(&state, &config, &env);
        Rig { state, hub: Some(hub), body: Some(body), program }
    }

    /// The roster row `kiwi/<session>` once `want` holds, or a panic that shows the body's log
    /// (a body that refused the config says why there).
    fn row(&self, session: &str, want: impl Fn(&Value) -> bool) -> Value {
        let suffix = format!("/{session}");
        wait_for(STARTUP_WAIT, || {
            let doc = support::roster_json(&self.state);
            doc["rows"]
                .as_array()?
                .iter()
                .find(|r| r["name"].as_str().is_some_and(|n| n.ends_with(&suffix)) && want(r))
                .cloned()
        })
        .unwrap_or_else(|| {
            panic!(
                "no matching roster row for {session}: {}\nbody log:\n{}",
                support::roster_json(&self.state),
                self.body.as_ref().map(Body::log_text).unwrap_or_default()
            )
        })
    }

    fn rows_named(&self, session: &str) -> usize {
        let suffix = format!("/{session}");
        support::roster_json(&self.state)["rows"]
            .as_array()
            .map_or(0, |rows| rows.iter().filter(|r| r["name"].as_str().is_some_and(|n| n.ends_with(&suffix))).count())
    }

    fn holler(&self, args: &[&str]) -> Output {
        holler_cmd(&self.state).args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).output().unwrap()
    }

    /// `holler say kiwi/c1r1 TEXT` while a helper thread plays the process: it waits for the line
    /// in the inbox, shows the process busy until the body has listed it twice, then idle. Returns
    /// the output of `say` and the frame the helper read.
    fn say_a_turn(&self, text: &str) -> (Output, Value) {
        std::thread::scope(|scope| {
            let process = scope.spawn(|| {
                let frames = self.program.wait_frames(1, Duration::from_secs(20));
                self.program.set_processes(&[Process::busy(PID, SESSION)]);
                assert!(self.program.until_listed(2, Duration::from_secs(20)), "the body never polled the busy process");
                self.program.set_processes(&[Process::idle(PID, SESSION)]);
                parsed(&frames).remove(0)
            });
            let out = self.holler(&["say", "kiwi/c1r1", text]);
            (out, process.join().unwrap())
        })
    }

    fn stop(mut self) {
        if let Some(body) = self.body.take() {
            body.stop(&self.state, Duration::from_secs(5));
        }
        if let Some(hub) = self.hub.take() {
            hub.stop(Duration::from_secs(5));
        }
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn idle() -> [Process<'static>; 1] {
    [Process::idle(PID, SESSION)]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn say_reaches_a_claude_row_as_one_inbox_frame() {
    let _guard = SERIAL.lock().await;
    let rig = Rig::start(&idle(), "");
    rig.row("c1r1", |_| true);

    let (out, frame) = rig.say_a_turn("hello pane");
    assert!(out.status.success(), "say: {}", stderr(&out));
    assert_eq!(frame, frame_of("hello pane"), "one frame, the text as typed");
    assert_eq!(rig.program.frames().len(), 1, "one line in the inbox: {:?}", rig.program.frames());
    assert!(
        stdout(&out).trim().is_empty(),
        "the body reads nothing the process prints, so the reply is empty: {:?}",
        stdout(&out)
    );
    rig.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_operator_hold_on_the_row_refuses_say_and_writes_nothing() {
    let _guard = SERIAL.lock().await;
    let rig = Rig::start(&idle(), "");
    rig.row("c1r1", |_| true);
    let held = rig.holler(&["hold", "kiwi/c1r1"]);
    assert!(held.status.success(), "hold: {}", stderr(&held));

    let out = rig.holler(&["say", "kiwi/c1r1", "should not arrive"]);
    assert_eq!(out.status.code(), Some(4), "{}", stderr(&out));
    assert!(stderr(&out).contains("session_held"), "{}", stderr(&out));
    assert!(rig.program.frames().is_empty(), "nothing reached the inbox: {:?}", rig.program.frames());
    rig.stop();
}

/// `interrupt` of a turn in flight is refused: the body cannot cancel a turn in a process it does
/// not own, so the refusal names the verb that relaunches the pane, and the turn runs on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupt_in_flight_is_refused_and_the_turn_runs_on() {
    let _guard = SERIAL.lock().await;
    let rig = Rig::start(&idle(), "");
    rig.row("c1r1", |_| true);
    let state_path: PathBuf = rig.state.path().to_path_buf();
    let said = std::thread::spawn(move || {
        Command::new(support::holler_bin())
            .env("HOLLER_STATE_DIR", state_path)
            .env("HOLLER_DEBUG", "quiet")
            .args(["say", "kiwi/c1r1", "a long thing"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    });
    rig.program.wait_frames(1, Duration::from_secs(20));
    rig.program.set_processes(&[Process::busy(PID, SESSION)]);
    assert!(rig.program.until_listed(2, Duration::from_secs(20)));

    let interrupted = rig.holler(&["interrupt", "kiwi/c1r1"]);
    assert!(!interrupted.status.success(), "a cancel in flight is refused: {}", stdout(&interrupted));
    assert!(
        stderr(&interrupted).contains("holler interrupt --pane c1r1"),
        "the refusal names the verb that relaunches the pane: {}",
        stderr(&interrupted)
    );
    rig.row("c1r1", |row| row["state"] == "working");
    assert_eq!(rig.program.frames().len(), 1, "nothing else was written");
    assert!(!said.is_finished(), "the turn runs on");

    rig.program.set_processes(&[Process::idle(PID, SESSION)]);
    let out = said.join().unwrap();
    assert!(out.status.success(), "the turn ended on its own: {}", stderr(&out));
    rig.stop();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_row_shows_in_the_roster_as_claude_attach_with_its_uuid() {
    let _guard = SERIAL.lock().await;
    let rig = Rig::start(&idle(), "");
    let row = rig.row("c1r1", |_| true);
    assert_eq!(row["name"], "kiwi/c1r1", "{row}");
    assert_eq!(row["harness"], "claude", "{row}");
    assert_eq!(row["mode"], "attach", "{row}");
    assert_eq!(row["harness_session_id"], SESSION, "{row}");
    assert_eq!(row["state"], "idle", "{row}");
    rig.stop();
}

/// The attach fails closed: no process holds the row's session, so the row is off the roster. A
/// sibling spawn row stands for "the body is up and has attached what it can".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_row_whose_session_no_process_holds_stays_off_the_roster() {
    let _guard = SERIAL.lock().await;
    let sibling = format!(
        "[[session]]\nname = \"beta\"\nharness = \"opencode\"\ncommand = [{}]\n\n",
        serde_json::to_string(stub_acp_bin()).unwrap()
    );
    let rig = Rig::start(&[], &sibling);
    rig.row("beta", |_| true);
    let looked = wait_for(STARTUP_WAIT, || (rig.program.listings() >= 1).then_some(()));
    assert!(looked.is_some(), "the body never looked for the process\nbody log:\n{}", rig.body.as_ref().unwrap().log_text());
    assert_eq!(rig.rows_named("c1r1"), 0, "no process holds the session, so no row: {}", support::roster_json(&rig.state));
    assert!(rig.program.frames().is_empty());
    rig.stop();
}

/// `query/support` for a Claude attach row is a PATH lookup (the support rule): `ok:true` with
/// `how: "attach: claude"` when `claude` resolves on `PATH`, else `ok:false` with `reason:
/// "claude not found on PATH"`. It runs no `claude`. (`HOLLER_CLAUDE_BIN` is unset in both runs, so
/// the answer does not depend on whether the probe also honours it.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn body_support_claude_is_a_path_lookup() {
    let _guard = SERIAL.lock().await;
    let state = StateDir::new();
    std::fs::create_dir_all(state.body()).unwrap();
    std::fs::write(state.body().join("sessions.toml"), claude_row("c1r1", SESSION)).unwrap();
    let with_claude = state.path().join("bin-with");
    let without = state.path().join("bin-without");
    for dir in [&with_claude, &without] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(with_claude.join("claude"), "#!/bin/sh\nexit 3\n").unwrap();
    std::fs::set_permissions(with_claude.join("claude"), std::fs::Permissions::from_mode(0o755)).unwrap();

    let support = |path: &PathBuf| -> Value {
        let out = holler_cmd(&state)
            .env("PATH", path)
            .env_remove("HOLLER_CLAUDE_BIN")
            .args(["body", "support", "claude", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert!(out.status.success(), "body support claude: {}", stderr(&out));
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", stdout(&out)))
    };
    let found = support(&with_claude);
    assert_eq!(found["ok"], true, "{found}");
    assert_eq!(found["how"], "attach: claude", "{found}");
    let missing = support(&without);
    assert_eq!(missing["ok"], false, "{missing}");
    assert_eq!(missing["reason"], "claude not found on PATH", "{missing}");
}

/// I7: the body never reads a login. The program was run with `agents --json` and nothing else, no
/// variable that changes a login reached it (the adapter removes them), and neither the sentinel
/// credential in the body's environment nor a variable's name reached an output or a log.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_login_is_read_or_printed() {
    let _guard = SERIAL.lock().await;
    let rig = Rig::start(&idle(), "");
    rig.row("c1r1", |_| true);
    let (out, _) = rig.say_a_turn("hi");
    assert!(out.status.success(), "say: {}", stderr(&out));

    let runs = rig.program.runs();
    assert!(!runs.is_empty());
    assert!(runs.iter().all(|run| run == "agents --json"), "only the listing was ever run: {runs:?}");
    let seen = rig.program.env_names();
    assert!(!seen.is_empty(), "the program ran, so it recorded its environment");
    for variable in LOGIN_VARIABLES {
        assert!(!seen.iter().any(|name| name == variable), "{variable} reached the program");
    }
    let roster = serde_json::to_string(&support::roster_json(&rig.state)).unwrap();
    let outputs = [
        stdout(&out),
        stderr(&out),
        rig.body.as_ref().unwrap().log_text(),
        rig.hub.as_ref().unwrap().log_text(),
        roster,
    ];
    for text in &outputs {
        assert!(!text.contains(SENTINEL_KEY), "a credential value reached an output or a log");
        for variable in LOGIN_VARIABLES {
            assert!(!text.contains(variable), "{variable} reached an output or a log: {text}");
        }
    }
    rig.stop();
}
