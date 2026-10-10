//! The setup wizard's Herdr stage in a named session (issue #730, epic #726).
//!
//! Drives `agent-skills/setup-wizard/lib/herdr.sh` with a fake `herdr` first on `PATH` that
//! records its argv, one line per invocation. The real `herdr` is never run and no real Herdr
//! session or socket is touched. Ledgers are fixture files in the contract's format.
//!
//! Unix-only: the script is bash.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #730

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn script() -> PathBuf {
    repo_root().join("agent-skills/setup-wizard/lib/herdr.sh")
}

/// A scratch directory holding the fake `herdr`, its argv log, its session list and a ledger.
struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new(sessions: &[&str]) -> Env {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let fake = bin.join("herdr");
        fs::write(
            &fake,
            "#!/bin/sh\n\
             echo \"$*\" >> \"$FAKE_HERDR_LOG\"\n\
             echo \"$$\" >> \"$FAKE_HERDR_LOG.pid\"\n\
             case \"$*\" in\n\
             *\"session list\"*) cat \"$FAKE_HERDR_SESSIONS\" ;;\n\
             esac\n\
             exit 0\n",
        )
        .unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        let mut list = String::new();
        for s in sessions {
            list.push_str(&format!("{s}  1 workspace\n"));
        }
        fs::write(dir.path().join("sessions.txt"), list).unwrap();
        Env { dir }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn log_file(&self) -> PathBuf {
        self.path().join("herdr-argv.log")
    }

    /// Every command the fake `herdr` was given, one per line.
    fn recorded(&self) -> Vec<String> {
        fs::read_to_string(self.log_file())
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn write_ledger(&self, body: &str) -> PathBuf {
        let p = self.path().join("wizard-ledger.toml");
        fs::write(&p, body).unwrap();
        p
    }

    /// A `herdr.sh <args>` command with the fake first on `PATH`; `instance` of `None` leaves
    /// `WIZARD_INSTANCE_NAME` unset. No log directory or state directory is set.
    fn base(&self, instance: Option<&str>, session: &str, args: &[&str]) -> Command {
        let old_path = std::env::var("PATH").unwrap_or_default();
        let mut c = Command::new("bash");
        c.arg(script())
            .args(args)
            .env(
                "PATH",
                format!("{}:{old_path}", self.path().join("bin").display()),
            )
            .env("FAKE_HERDR_LOG", self.log_file())
            .env("FAKE_HERDR_SESSIONS", self.path().join("sessions.txt"))
            .env("WIZARD_HERDR_SESSION", session)
            .env("WIZARD_LEDGER", self.path().join("wizard-ledger.toml"))
            .env("HOME", self.path())
            .env("TMPDIR", self.path().join("tmp"))
            .env_remove("WIZARD_LOG_DIR")
            .env_remove("WIZARD_STATE_DIR")
            .env_remove("WIZARD_INSTANCE_PREFIX")
            .env_remove("HERDR_BIN")
            .env_remove("WIZARD_LIB")
            .env_remove("HOLLER_STATE_DIR")
            .env_remove("HERDR_PANE_ID")
            .env_remove("HERDR_SESSION")
            .current_dir(self.path());
        match instance {
            Some(i) => c.env("WIZARD_INSTANCE_NAME", i),
            None => c.env_remove("WIZARD_INSTANCE_NAME"),
        };
        c
    }

    /// `herdr.sh <args>` for the instance `<instance>` / Herdr session `<session>` (empty = none).
    fn run(&self, instance: &str, session: &str, args: &[&str]) -> Output {
        self.base(Some(instance), session, args)
            .env("WIZARD_LOG_DIR", self.path())
            .output()
            .unwrap()
    }

    fn run_in_pane(&self, session: &str, pane_session: &str, args: &[&str]) -> Output {
        self.base(Some("second"), session, args)
            .env("WIZARD_LOG_DIR", self.path())
            .env("HERDR_PANE_ID", "p1")
            .env("HERDR_SESSION", pane_session)
            .output()
            .unwrap()
    }
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// A real, harmless process the fixture ledger can describe with `ps` exactly as the ledger does.
struct LiveProcess {
    child: std::process::Child,
}

impl LiveProcess {
    fn start() -> LiveProcess {
        let child = Command::new("sleep").arg("60").spawn().unwrap();
        LiveProcess { child }
    }

    fn ps(&self, field: &str) -> String {
        let out = Command::new("ps")
            .env("LC_ALL", "C")
            .args(["-o", field, "-p", &self.child.id().to_string()])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// The ledger table for this process as a `herdr` row of `session`.
    fn ledger_row(&self, session: &str) -> String {
        format!(
            "[[process]]\npid = {}\nstarted = \"{}\"\ncmd = \"{}\"\nrole = \"herdr\"\n\
             stage = 8\nsession = \"{session}\"\n",
            self.child.id(),
            self.ps("lstart="),
            self.ps("command=")
        )
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn ledger_with_herdr(session: &str) -> String {
    format!(
        "[[process]]\npid = 4242\nstarted = \"Sat Oct 10 12:00:00 2026\"\n\
         cmd = \"herdr --session {session} server\"\nrole = \"herdr\"\nstage = 8\n\
         session = \"{session}\"\n"
    )
}

#[test]
fn refuses_a_session_of_the_same_name_the_ledger_did_not_create() {
    let env = Env::new(&["other", "second"]);
    // The ledger exists but records a different Herdr session, not `second`.
    env.write_ledger(&ledger_with_herdr("other"));
    let out = env.run("second", "second", &["check-session"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("second"), "must name the session");
    let cmds = env.recorded();
    assert!(
        cmds.iter()
            .all(|c| !c.contains("split") && !c.contains("server")),
        "nothing may be built or started: {cmds:?}"
    );
}

#[test]
fn refuses_an_existing_session_when_there_is_no_ledger_at_all() {
    let env = Env::new(&["second"]);
    let out = env.run("second", "second", &["check-session"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("second"));
}

#[test]
fn a_session_the_ledger_created_is_accepted() {
    let env = Env::new(&["second"]);
    let mut live = LiveProcess::start();
    env.write_ledger(&live.ledger_row("second"));
    let out = env.run("second", "second", &["check-session"]);
    live.stop();
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn a_stale_herdr_row_does_not_count_as_created_by_the_wizard() {
    let env = Env::new(&["second"]);
    let mut live = LiveProcess::start();
    // Right pid, but the recorded start time differs: a reused pid, stale.
    let row = live
        .ledger_row("second")
        .replace("started = \"", "started = \"not ");
    env.write_ledger(&row);
    let out = env.run("second", "second", &["check-session"]);
    live.stop();
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("second"), "{}", text(&out));
}

#[test]
fn a_live_row_of_another_role_does_not_count() {
    let env = Env::new(&["second"]);
    let mut live = LiveProcess::start();
    let other_role = live
        .ledger_row("second")
        .replace("role = \"herdr\"", "role = \"hub\"");
    env.write_ledger(&other_role);
    let out = env.run("second", "second", &["check-session"]);
    live.stop();
    assert!(!out.status.success(), "{}", text(&out));
}

#[test]
fn a_fresh_session_name_is_accepted() {
    let env = Env::new(&["other"]);
    let out = env.run("second", "second", &["check-session"]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn every_stage_command_carries_the_session_and_no_bare_server_stop_is_issued() {
    let env = Env::new(&[]);
    // The stage's command list, in order, as the skill runs it.
    let stage: &[&[&str]] = &[
        &["run", "status"],
        &["run", "pane", "list"],
        &["run", "workspace", "list"],
        &[
            "run",
            "pane",
            "split",
            "--pane",
            "p1",
            "--direction",
            "right",
        ],
        &[
            "run",
            "pane",
            "split",
            "--pane",
            "p2",
            "--direction",
            "down",
        ],
        &["run", "pane", "run", "p2", "cd /x && cmd"],
        &["run", "pane", "send-keys", "p2", "enter"],
        &["run", "pane", "read", "p2"],
        &["run", "server", "stop"],
        &["run", "session", "attach"],
    ];
    for args in stage {
        let out = env.run("second", "second", args);
        assert!(out.status.success(), "{args:?}: {}", text(&out));
    }
    let start = env.run("second", "second", &["server-start"]);
    assert!(start.status.success(), "{}", text(&start));
    // server-start backgrounds the server; wait for the fake to record it.
    let deadline = Instant::now() + Duration::from_secs(5);
    while env.recorded().len() < stage.len() + 1 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let cmds = env.recorded();
    assert_eq!(cmds.len(), stage.len() + 1, "{cmds:?}");
    for c in &cmds {
        assert!(c.starts_with("--session second "), "no session on: {c}");
    }
    assert!(cmds.contains(&"--session second server stop".to_owned()));
    assert!(cmds.contains(&"--session second session attach".to_owned()));
    assert!(
        cmds.contains(&"--session second server".to_owned()),
        "{cmds:?}"
    );
    assert!(
        !cmds.iter().any(|c| c == "server stop"),
        "a bare server stop was issued: {cmds:?}"
    );
}

#[test]
fn server_stop_without_a_session_is_refused_even_for_the_default_instance() {
    let env = Env::new(&[]);
    for args in [
        &["run", "server", "stop"][..],
        &["run", "session", "attach"],
    ] {
        let out = env.run("default", "", args);
        assert!(!out.status.success(), "{}", text(&out));
    }
    assert!(env.recorded().is_empty(), "{:?}", env.recorded());
}

#[test]
fn an_unset_instance_name_refuses_every_verb_and_runs_no_herdr() {
    let env = Env::new(&[]);
    for args in [
        &["run", "status"][..],
        &[
            "run",
            "pane",
            "split",
            "--pane",
            "p1",
            "--direction",
            "right",
        ],
        &["run", "pane", "run", "p1", "x"],
        &["run", "pane", "send-keys", "p1", "enter"],
        &["server-start"],
        &["check-session"],
        &["check-pane"],
        &["log-path"],
    ] {
        for session in ["", "second"] {
            let out = env.base(None, session, args).output().unwrap();
            assert_eq!(out.status.code(), Some(2), "{args:?}: {}", text(&out));
            let err = String::from_utf8_lossy(&out.stderr).into_owned();
            assert_eq!(err.trim().lines().count(), 1, "{err}");
            assert!(err.contains("WIZARD_INSTANCE_NAME"), "{err}");
        }
    }
    assert!(env.recorded().is_empty(), "{:?}", env.recorded());
}

/// Runs `server-start` and returns (output, the pid the fake recorded for itself last).
fn start_server(env: &Env, mut c: Command) -> (Output, String) {
    let pid_file = format!("{}.pid", env.log_file().display());
    let lines = |f: &str| fs::read_to_string(f).unwrap_or_default().lines().count();
    let before = lines(&pid_file);
    let out = c.output().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while lines(&pid_file) <= before && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let all = fs::read_to_string(&pid_file).unwrap_or_default();
    let pid = all.lines().last().unwrap_or("").to_owned();
    (out, pid)
}

fn first_line(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .to_owned()
}

#[test]
fn the_default_instance_without_a_session_starts_an_unnamed_server_and_prints_its_pid() {
    let env = Env::new(&[]);
    let c = env.base(Some("default"), "", &["server-start"]);
    let (out, server_pid) = start_server(&env, c);
    assert!(out.status.success(), "{}", text(&out));
    let first = first_line(&out);
    assert!(
        !first.is_empty() && first.chars().all(|c| c.is_ascii_digit()),
        "{first:?}"
    );
    assert_eq!(first, server_pid, "the printed pid is not the server's");
    assert_eq!(env.recorded(), vec!["server".to_owned()]);
}

#[test]
fn a_named_session_prints_the_servers_pid_alone_on_its_first_line() {
    let env = Env::new(&[]);
    let c = env.base(Some("second"), "second", &["server-start"]);
    let (out, server_pid) = start_server(&env, c);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(first_line(&out), server_pid);
    assert_eq!(env.recorded(), vec!["--session second server".to_owned()]);
}

#[test]
fn the_server_log_defaults_under_the_state_directory_and_never_under_tmp() {
    let env = Env::new(&[]);
    let state = env.path().join("state");
    let mut c = env.base(Some("second"), "second", &["server-start"]);
    c.env("WIZARD_STATE_DIR", &state);
    let (out, _) = start_server(&env, c);
    assert!(out.status.success(), "{}", text(&out));
    let log = state.join("logs/second-herdr-server.log");
    assert!(log.exists(), "{} missing", log.display());
    assert!(!env.path().join("tmp").exists(), "TMPDIR was used");
    let mut p = env.base(Some("second"), "second", &["log-path"]);
    p.env("WIZARD_STATE_DIR", &state);
    let shown = String::from_utf8_lossy(&p.output().unwrap().stdout)
        .trim()
        .to_owned();
    assert_eq!(shown, log.display().to_string());
}

#[test]
fn without_a_state_directory_the_log_goes_under_home_dot_holler_logs() {
    let env = Env::new(&[]);
    let c = env.base(Some("second"), "second", &["server-start"]);
    let (out, _) = start_server(&env, c);
    assert!(out.status.success(), "{}", text(&out));
    assert!(env
        .path()
        .join(".holler/logs/second-herdr-server.log")
        .exists());
    assert!(!env.path().join("tmp").exists(), "TMPDIR was used");
}

#[test]
fn herdr_bin_names_the_binary_when_it_is_not_on_path() {
    let env = Env::new(&[]);
    let off = env.path().join("off-path");
    fs::create_dir(&off).unwrap();
    let named = off.join("herdr-custom");
    fs::copy(env.path().join("bin/herdr"), &named).unwrap();
    fs::remove_file(env.path().join("bin/herdr")).unwrap();
    let mut c = env.base(Some("second"), "second", &["run", "status"]);
    c.env("HERDR_BIN", &named);
    let out = c.output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(env.recorded(), vec!["--session second status".to_owned()]);
    let mut s = env.base(Some("second"), "second", &["server-start"]);
    s.env("HERDR_BIN", &named);
    let (started, _) = start_server(&env, s);
    assert!(started.status.success(), "{}", text(&started));
    assert!(env
        .recorded()
        .contains(&"--session second server".to_owned()));
}

#[test]
fn a_non_default_instance_without_a_session_name_is_refused() {
    let env = Env::new(&[]);
    for args in [
        &["run", "status"][..],
        &["check-session"],
        &["server-start"],
        &["check-pane"],
    ] {
        let out = env.run("second", "", args);
        assert!(!out.status.success(), "{args:?}: {}", text(&out));
        assert!(text(&out).contains("herdr_session"), "{}", text(&out));
    }
    assert!(env.recorded().is_empty(), "{:?}", env.recorded());
}

#[test]
fn the_default_instance_runs_commands_as_before() {
    let env = Env::new(&[]);
    let out = env.run("default", "", &["run", "pane", "list"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(env.recorded(), vec!["pane list".to_owned()]);
}

#[test]
fn a_caller_supplied_session_flag_is_refused() {
    let env = Env::new(&[]);
    let out = env.run("second", "second", &["run", "--session", "other", "status"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(env.recorded().is_empty());
}

#[test]
fn an_invalid_session_name_is_refused() {
    let env = Env::new(&[]);
    let out = env.run("second", "Bad Name", &["run", "status"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(env.recorded().is_empty());
}

#[test]
fn a_pane_of_a_different_session_stops_before_any_split() {
    let env = Env::new(&["other"]);
    let check = env.run_in_pane("second", "other", &["check-pane"]);
    assert!(!check.status.success(), "{}", text(&check));
    let t = text(&check);
    assert!(t.contains("other") && t.contains("second"), "{t}");
    // The stage's split goes through the same gate and is refused.
    let split = env.run_in_pane(
        "second",
        "other",
        &[
            "run",
            "pane",
            "split",
            "--pane",
            "p1",
            "--direction",
            "right",
        ],
    );
    assert!(!split.status.success(), "{}", text(&split));
    assert!(
        env.recorded().iter().all(|c| !c.contains("split")),
        "{:?}",
        env.recorded()
    );
}

#[test]
fn a_pane_whose_session_cannot_be_established_is_refused() {
    let env = Env::new(&[]);
    let check = env.run_in_pane("second", "", &["check-pane"]);
    assert!(!check.status.success(), "{}", text(&check));
}

#[test]
fn inside_a_herdr_pane_with_no_session_variable_check_pane_names_the_fix_and_runs_no_herdr() {
    let env = Env::new(&["second"]);
    let mut c = env.base(Some("second"), "second", &["check-pane"]);
    c.env("HERDR_PANE_ID", "p1");
    let out = c.output().unwrap();
    assert!(!out.status.success(), "{}", text(&out));
    let t = text(&out);
    assert!(t.contains("inside a Herdr pane"), "{t}");
    assert!(t.contains("outside any Herdr pane"), "{t}");
    assert!(env.recorded().is_empty(), "{:?}", env.recorded());
}

/// `session-delete` for `session`, with the fake listing `rows` (one `name status` per line).
fn session_delete(env: &Env, session: &str, rows: &[(&str, &str)], extra: &[&str]) -> Output {
    let mut list = String::new();
    for (n, st) in rows {
        list.push_str(&format!("{n}  {st}  /dir  /dir/sock\n"));
    }
    fs::write(env.path().join("sessions.txt"), list).unwrap();
    let mut args = vec!["session-delete"];
    args.extend_from_slice(extra);
    env.run("second", session, &args)
}

fn deleted(env: &Env) -> bool {
    env.recorded().iter().any(|c| c.contains("session delete"))
}

#[test]
fn session_delete_removes_a_stopped_own_session() {
    let env = Env::new(&[]);
    let out = session_delete(&env, "second", &[("second", "stopped")], &[]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(env
        .recorded()
        .contains(&"--session second session delete second".to_owned()));
}

#[test]
fn session_delete_refuses_a_running_session() {
    let env = Env::new(&[]);
    let out = session_delete(&env, "second", &[("second", "running")], &[]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("second"), "{}", text(&out));
    assert!(!deleted(&env), "{:?}", env.recorded());
}

#[test]
fn session_delete_refuses_a_session_that_the_ledger_shows_live() {
    let env = Env::new(&[]);
    let mut live = LiveProcess::start();
    env.write_ledger(&live.ledger_row("second"));
    let out = session_delete(&env, "second", &[("second", "stopped")], &[]);
    live.stop();
    assert!(!out.status.success(), "{}", text(&out));
    assert!(!deleted(&env), "{:?}", env.recorded());
}

#[test]
fn session_delete_refuses_another_instances_session_name() {
    let env = Env::new(&[]);
    let out = session_delete(
        &env,
        "second",
        &[("second", "stopped"), ("other", "stopped")],
        &["other"],
    );
    assert!(!out.status.success(), "{}", text(&out));
    assert!(!deleted(&env), "{:?}", env.recorded());
}

#[test]
fn session_delete_refuses_the_default_session() {
    let env = Env::new(&[]);
    // The default instance has no named session of its own.
    fs::write(
        env.path().join("sessions.txt"),
        "default  stopped  /d  /s\n",
    )
    .unwrap();
    let out = env.run("default", "", &["session-delete"]);
    assert!(!out.status.success(), "{}", text(&out));
    let named = env.run("default", "", &["session-delete", "default"]);
    assert!(!named.status.success(), "{}", text(&named));
    assert!(!deleted(&env), "{:?}", env.recorded());
}

#[test]
fn session_delete_refuses_a_session_that_is_not_listed() {
    let env = Env::new(&[]);
    let out = session_delete(&env, "second", &[("other", "stopped")], &[]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(!deleted(&env), "{:?}", env.recorded());
}

#[test]
fn the_header_says_the_session_semantics_are_unverified_until_734() {
    let src = fs::read_to_string(script()).unwrap();
    let header: String = src
        .to_lowercase()
        .lines()
        .take(20)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        header.contains("#734") && header.contains("unverified"),
        "{header}"
    );
}

#[test]
fn a_pane_of_the_instances_own_session_is_accepted_and_none_is_accepted() {
    let env = Env::new(&[]);
    let same = env.run_in_pane("second", "second", &["check-pane"]);
    assert!(same.status.success(), "{}", text(&same));
    let none = env.run("second", "second", &["check-pane"]);
    assert!(none.status.success(), "{}", text(&none));
}

#[test]
fn the_server_log_is_named_for_the_instance() {
    let env = Env::new(&[]);
    let out = env.run("second", "second", &["log-path"]);
    assert!(out.status.success(), "{}", text(&out));
    let p = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    assert!(p.contains("second"), "{p}");
    assert!(p.starts_with(env.path().to_str().unwrap()), "{p}");
    let def = env.run("default", "", &["log-path"]);
    assert!(String::from_utf8_lossy(&def.stdout).contains("herdr-server.log"));
}

#[test]
fn the_skill_runs_stages_8_and_9_through_the_wrapper_and_forbids_a_bare_server_stop() {
    let skill = fs::read_to_string(repo_root().join("agent-skills/setup-wizard/SKILL.md")).unwrap();
    let start = skill.find("## Stage 8 ").expect("stage 8");
    let end = skill.find("## Stage 10 ").expect("stage 10");
    let section = &skill[start..end];
    for (i, line) in section.lines().enumerate() {
        let l = line.trim_start();
        assert!(
            !(l.starts_with("herdr ") || l.starts_with("nohup herdr")),
            "bare herdr command in Stage 8/9 (line {i}): {line}"
        );
    }
    assert!(section.contains("herdr.sh"));
    assert!(section.contains("bare `herdr server stop`"));
    let docs = fs::read_to_string(repo_root().join("docs/setup-wizard.md")).unwrap();
    assert!(docs.contains("## Herdr session"));
}
