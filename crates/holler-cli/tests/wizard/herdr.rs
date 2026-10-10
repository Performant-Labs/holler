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

    /// `herdr.sh <args>` for the instance `<instance>` / Herdr session `<session>` (empty = none).
    fn run(&self, instance: &str, session: &str, args: &[&str]) -> Output {
        let old_path = std::env::var("PATH").unwrap_or_default();
        Command::new("bash")
            .arg(script())
            .args(args)
            .env(
                "PATH",
                format!("{}:{old_path}", self.path().join("bin").display()),
            )
            .env("FAKE_HERDR_LOG", self.log_file())
            .env("FAKE_HERDR_SESSIONS", self.path().join("sessions.txt"))
            .env("WIZARD_INSTANCE_NAME", instance)
            .env("WIZARD_HERDR_SESSION", session)
            .env("WIZARD_LEDGER", self.path().join("wizard-ledger.toml"))
            .env("WIZARD_LOG_DIR", self.path())
            .env_remove("HERDR_PANE_ID")
            .env_remove("HERDR_SESSION")
            .current_dir(self.path())
            .output()
            .unwrap()
    }

    fn run_in_pane(&self, session: &str, pane_session: &str, args: &[&str]) -> Output {
        let old_path = std::env::var("PATH").unwrap_or_default();
        Command::new("bash")
            .arg(script())
            .args(args)
            .env(
                "PATH",
                format!("{}:{old_path}", self.path().join("bin").display()),
            )
            .env("FAKE_HERDR_LOG", self.log_file())
            .env("FAKE_HERDR_SESSIONS", self.path().join("sessions.txt"))
            .env("WIZARD_INSTANCE_NAME", "second")
            .env("WIZARD_HERDR_SESSION", session)
            .env("WIZARD_LEDGER", self.path().join("wizard-ledger.toml"))
            .env("WIZARD_LOG_DIR", self.path())
            .env("HERDR_PANE_ID", "p1")
            .env("HERDR_SESSION", pane_session)
            .current_dir(self.path())
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
    env.write_ledger(&ledger_with_herdr("second"));
    let out = env.run("second", "second", &["check-session"]);
    assert!(out.status.success(), "{}", text(&out));
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
        &["run", "pane", "split", "--pane", "p1", "--direction", "right"],
        &["run", "pane", "split", "--pane", "p2", "--direction", "down"],
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
    for (instance, session) in [("", ""), ("default", "")] {
        let out = env.run(instance, session, &["run", "server", "stop"]);
        assert!(!out.status.success(), "{}", text(&out));
        let out = env.run(instance, session, &["run", "session", "attach"]);
        assert!(!out.status.success(), "{}", text(&out));
    }
    assert!(env.recorded().is_empty(), "{:?}", env.recorded());
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
    let out = env.run("", "", &["run", "pane", "list"]);
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
        &["run", "pane", "split", "--pane", "p1", "--direction", "right"],
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
    let def = env.run("", "", &["log-path"]);
    assert!(String::from_utf8_lossy(&def.stdout).contains("herdr-server.log"));
}

#[test]
fn the_skill_runs_stages_8_and_9_through_the_wrapper_and_forbids_a_bare_server_stop() {
    let skill =
        fs::read_to_string(repo_root().join("agent-skills/setup-wizard/SKILL.md")).unwrap();
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
