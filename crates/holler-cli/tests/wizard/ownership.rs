//! #731: the wizard stops, restarts or reports a process only when the instance's own ledger
//! recorded it and its recorded identity still matches (epic #726).
//!
//! Drives `agent-skills/setup-wizard/lib/stop-owned.sh` with a FAKE `ledger.sh` placed in a temp
//! directory and named by `WIZARD_LIB` (the real one belongs to #729). The fake answers
//! `owns <state_dir> <pid>` and `list <state_dir>` from a plain table kept in the state
//! directory. A fake `lsof` first on `PATH` reports which pid holds a planned port.
//!
//! Every process signalled here is a `sleep` the test started itself and signalled by the
//! pid the test recorded for it. Nothing is ever signalled by name or pattern.
//!
//! Unix-only: the script is bash and uses `ps`/`kill`.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)] // #731

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

const FAKE_LEDGER: &str = r#"#!/bin/bash
# Fake ledger.sh, same interface as the real one: the state directory comes from
# HOLLER_STATE_DIR; `owns <pid>` exits 0 live, 1 stale, 2 not recorded; `list` prints one
# TAB-separated line per entry: pid, live|stale, role, stage, session (may be empty), cmd.
# Table: $HOLLER_STATE_DIR/fake-ledger, already in the `list` shape, in start order.
tbl="$HOLLER_STATE_DIR/fake-ledger"
case "$1" in
  owns)
    [ -f "$tbl" ] || exit 2
    st="$(awk -F'\t' -v p="$2" '$1 == p { print $2; exit }' "$tbl")"
    case "$st" in live) exit 0 ;; stale) exit 1 ;; *) exit 2 ;; esac ;;
  list)
    [ -f "$tbl" ] && cat "$tbl"
    exit 0 ;;
  *) exit 64 ;;
esac
"#;

const FAKE_LSOF: &str = "#!/bin/bash\nfor p in $FAKE_LSOF_PIDS; do echo \"$p\"; done\n";

/// A harmless `sleep` the test owns. Reaped by a thread when it exits; killed on drop by pid.
struct Child {
    pid: u32,
    cmd: String,
}

impl Child {
    fn start(tag: &str) -> Child {
        // A unique, recognisable duration so the recorded command is distinctive.
        let secs = format!("3{}", 100 + tag.len());
        let mut c = Command::new("sleep")
            .arg(&secs)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = c.id();
        std::thread::spawn(move || {
            let _ = c.wait();
        });
        Child {
            pid,
            cmd: format!("sleep {secs}"),
        }
    }

    fn alive(&self) -> bool {
        // Give a signalled process a moment to die and be reaped by the thread.
        std::thread::sleep(Duration::from_millis(300));
        // SAFETY: signal 0 probes the pid of a child this test started.
        unsafe { libc::kill(self.pid as libc::pid_t, 0) == 0 }
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        // SAFETY: the pid is this test's own child.
        unsafe {
            libc::kill(self.pid as libc::pid_t, libc::SIGTERM);
        }
    }
}

struct Env {
    root: tempfile::TempDir,
}

impl Env {
    fn new() -> Env {
        let root = tempfile::tempdir().unwrap();
        let lib = root.path().join("lib");
        let bin = root.path().join("bin");
        fs::create_dir_all(&lib).unwrap();
        fs::create_dir_all(&bin).unwrap();
        write_exec(&lib.join("ledger.sh"), FAKE_LEDGER);
        write_exec(&bin.join("lsof"), FAKE_LSOF);
        Env { root }
    }

    fn lib(&self) -> PathBuf {
        self.root.path().join("lib")
    }

    fn state(&self, name: &str) -> PathBuf {
        let d = self.root.path().join(name);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Replace the fake ledger table of an instance. Entries: (child, "live" | "stale").
    fn ledger(&self, state: &Path, entries: &[(&Child, &str)]) {
        let full: Vec<(&Child, &str, u32, &str)> =
            entries.iter().map(|(c, st)| (*c, *st, 4, "s1")).collect();
        self.ledger_full(state, &full);
    }

    /// Like `ledger`, with each entry's stage and session (which may be empty) too.
    fn ledger_full(&self, state: &Path, entries: &[(&Child, &str, u32, &str)]) {
        let mut t = String::new();
        for (c, st, stage, session) in entries {
            t.push_str(&format!(
                "{}\t{}\tbackend\t{}\t{}\t{}\n",
                c.pid, st, stage, session, c.cmd
            ));
        }
        fs::write(state.join("fake-ledger"), t).unwrap();
        fs::write(state.join("wizard-ledger.toml"), "# placeholder\n").unwrap();
    }

    fn run(&self, args: &[&str], lsof_pids: &str) -> Output {
        self.run_with_lib(&self.lib(), args, lsof_pids)
    }

    fn run_with_lib(&self, lib: &Path, args: &[&str], lsof_pids: &str) -> Output {
        let path = format!(
            "{}:{}",
            self.root.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new("bash")
            .arg(script())
            .args(args)
            .env("WIZARD_LIB", lib)
            .env("PATH", path)
            .env("FAKE_LSOF_PIDS", lsof_pids)
            .env("STOP_OWNED_GRACE", "5")
            .output()
            .unwrap()
    }
}

fn write_exec(p: &Path, body: &str) {
    fs::write(p, body).unwrap();
    fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../agent-skills/setup-wizard/lib/stop-owned.sh")
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

#[test]
fn stop_signals_a_live_ledger_process() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("a");
    env.ledger(&st, &[(&c, "live")]);
    let o = env.run(&["stop", &s(&st), &c.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(!c.alive(), "a live ledger process must be stopped");
    assert!(text(&o).contains("STOPPED"), "{}", text(&o));
}

#[test]
fn stop_refuses_a_reused_pid_and_reports_it_stale() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("reused");
    // The ledger recorded this pid, but the process now there is not the recorded one.
    env.ledger(&st, &[(&c, "stale")]);
    let o = env.run(&["stop", &s(&st), &c.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    assert!(text(&o).contains("STALE"), "{}", text(&o));
    assert!(c.alive(), "a stale entry must never be signalled");
}

#[test]
fn stop_refuses_a_pid_in_no_ledger() {
    let env = Env::new();
    let st = env.state("inst-a");
    let other = Child::start("other");
    let c = Child::start("recorded");
    env.ledger(&st, &[(&c, "live")]);
    let o = env.run(&["stop", &s(&st), &other.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    assert!(text(&o).contains("FOREIGN"), "{}", text(&o));
    assert!(other.alive(), "a foreign pid must never be signalled");
    assert!(c.alive());
}

#[test]
fn rerun_reports_an_unrecorded_process_holding_a_planned_port_and_signals_nothing() {
    let env = Env::new();
    let st = env.state("inst-a");
    let mine = Child::start("mine");
    let squatter = Child::start("squatter");
    env.ledger(&st, &[(&mine, "live")]);
    let o = env.run(&["check-port", &s(&st), "47001"], &squatter.pid.to_string());
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    let t = text(&o);
    assert!(t.contains(&squatter.pid.to_string()), "{t}");
    assert!(t.contains(&squatter.cmd), "the command must be reported: {t}");
    assert!(t.contains("FOREIGN"), "{t}");
    assert!(t.contains("asks"), "the wizard must stop and ask: {t}");
    assert!(squatter.alive(), "the holder must not be signalled");
    assert!(mine.alive(), "no ledger process may be signalled by a port check");
}

#[test]
fn check_port_reports_a_reused_pid_holder_as_stale_and_a_ledger_holder_as_owned() {
    let env = Env::new();
    let st = env.state("inst-a");
    let reused = Child::start("reused");
    let mine = Child::start("mine");
    env.ledger(&st, &[(&reused, "stale"), (&mine, "live")]);
    let o = env.run(&["check-port", &s(&st), "47001"], &reused.pid.to_string());
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    assert!(text(&o).contains("STALE"), "{}", text(&o));
    assert!(reused.alive());
    let o = env.run(&["check-port", &s(&st), "47002"], &mine.pid.to_string());
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(text(&o).contains("OWNED"), "{}", text(&o));
    assert!(mine.alive(), "check-port never signals");
}

#[test]
fn check_port_free_exits_zero() {
    let env = Env::new();
    let st = env.state("inst-a");
    let o = env.run(&["check-port", &s(&st), "47001"], "");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(text(&o).contains("FREE"), "{}", text(&o));
}

#[test]
fn restart_stops_the_recorded_process_and_prints_its_recorded_command() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("restart");
    env.ledger(&st, &[(&c, "live")]);
    let o = env.run(&["restart", &s(&st), &c.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(
        text(&o).contains(&format!("RESTART-CMD {}", c.cmd)),
        "{}",
        text(&o)
    );
    assert!(!c.alive());
}

#[test]
fn restart_of_a_stale_entry_signals_nothing_and_prints_no_command() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("restale");
    env.ledger(&st, &[(&c, "stale")]);
    let o = env.run(&["restart", &s(&st), &c.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    assert!(!text(&o).contains("RESTART-CMD"), "{}", text(&o));
    assert!(c.alive());
}

#[test]
fn teardown_stops_ledger_processes_in_reverse_order_and_leaves_the_second_instance_alone() {
    let env = Env::new();
    let a = env.state("inst-a");
    let b = env.state("inst-b");
    let a1 = Child::start("a1");
    let a2 = Child::start("a2-longer");
    let a3 = Child::start("a3-longest!");
    let reused = Child::start("reused-in-a");
    let squatter = Child::start("squatter");
    let b1 = Child::start("b1");
    let b2 = Child::start("b2-longer");
    // File order is not start order: stage decides, then pid.
    env.ledger_full(
        &a,
        &[
            (&a1, "live", 6, "s1"),
            (&a2, "live", 4, ""),
            (&reused, "stale", 5, "s2"),
            (&a3, "live", 6, "s3"),
        ],
    );
    env.ledger(&b, &[(&b1, "live"), (&b2, "live")]);
    fs::write(a.join("keep.txt"), "x").unwrap();
    fs::write(b.join("data.txt"), "b state").unwrap();

    let o = env.run(&["teardown", &s(&a)], &squatter.pid.to_string());
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    let t = text(&o);

    // Reverse start order.
    let pos = |c: &Child| {
        t.find(&format!("STOPPED pid {} ", c.pid))
            .unwrap_or_else(|| panic!("no STOPPED line for {}: {t}", c.pid))
    };
    let (hi, lo) = if a1.pid > a3.pid { (&a1, &a3) } else { (&a3, &a1) };
    assert!(pos(hi) < pos(lo) && pos(lo) < pos(&a2), "{t}");

    assert!(!a1.alive() && !a2.alive() && !a3.alive());
    assert!(reused.alive(), "stale entries are never signalled");
    assert!(t.contains(&format!("STALE pid {}", reused.pid)), "{t}");
    assert!(squatter.alive(), "an unrecorded process is untouched");

    // The second instance: processes, ledger and state all untouched.
    assert!(b1.alive() && b2.alive());
    assert!(b.join("fake-ledger").exists());
    assert!(b.join("wizard-ledger.toml").exists());
    assert_eq!(fs::read_to_string(b.join("data.txt")).unwrap(), "b state");

    // This instance's ledger is gone; its other state stays, and the output says so.
    assert!(!a.join("wizard-ledger.toml").exists());
    assert!(a.join("keep.txt").exists());
    assert!(t.contains("LEFT"), "teardown must say what it left: {t}");
}

#[test]
fn teardown_never_signals_a_pid_that_is_live_only_in_another_instances_ledger() {
    let env = Env::new();
    let a = env.state("inst-a");
    let b = env.state("inst-b");
    let theirs = Child::start("theirs");
    // Instance A's ledger lists nothing for this pid; B's ledger owns it.
    let mine = Child::start("mine");
    env.ledger(&a, &[(&mine, "live")]);
    env.ledger(&b, &[(&theirs, "live")]);
    let o = env.run(&["teardown", &s(&a)], "");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(theirs.alive());
    let o = env.run(&["stop", &s(&a), &theirs.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    assert!(theirs.alive());
}

#[test]
fn teardown_with_purge_state_removes_only_the_named_state_directory() {
    let env = Env::new();
    let a = env.state("inst-a");
    let b = env.state("inst-b");
    let c = Child::start("purge");
    env.ledger(&a, &[(&c, "live")]);
    fs::write(b.join("data.txt"), "b state").unwrap();
    let o = env.run(&["teardown", &s(&a), "--purge-state"], "");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(!a.exists());
    assert!(b.join("data.txt").exists());
}

#[test]
fn nothing_is_signalled_when_the_ledger_script_is_missing() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("noledger");
    env.ledger(&st, &[(&c, "live")]);
    fs::remove_file(env.lib().join("ledger.sh")).unwrap();
    let o = env.run(&["stop", &s(&st), &c.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(4), "{}", text(&o));
    assert!(c.alive());
}

#[test]
fn unusable_pids_are_refused() {
    let env = Env::new();
    let st = env.state("inst-a");
    for bad in ["1", "0", "abc", ""] {
        let o = env.run(&["stop", &s(&st), bad], "");
        assert_eq!(o.status.code(), Some(4), "pid {bad:?}: {}", text(&o));
    }
}

#[test]
fn restart_prints_the_recorded_command_when_the_session_field_is_empty() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("nosession");
    env.ledger_full(&st, &[(&c, "live", 7, "")]);
    let o = env.run(&["restart", &s(&st), &c.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    let line = format!("RESTART-CMD {}", c.cmd);
    assert!(text(&o).lines().any(|l| l == line), "{}", text(&o));
    assert!(!c.alive());
}

/// The real ledger.sh from story #729, if its worktree is present next to this one.
fn real_ledger_lib(env: &Env) -> Option<PathBuf> {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../729-start-ledger/agent-skills/setup-wizard/lib/ledger.sh");
    if !src.exists() {
        eprintln!("note: real ledger.sh not found at {}; skipping", src.display());
        return None;
    }
    let lib = env.root.path().join("real-lib");
    fs::create_dir_all(&lib).unwrap();
    fs::copy(&src, lib.join("ledger.sh")).unwrap();
    Some(lib)
}

fn record(lib: &Path, state: &Path, c: &Child, stage: &str, session: &[&str]) {
    let o = Command::new("bash")
        .arg(lib.join("ledger.sh"))
        .args(["record", "--pid", &c.pid.to_string(), "--role", "backend"])
        .args(["--stage", stage])
        .args(session)
        .env("HOLLER_STATE_DIR", state)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", text(&o));
}

#[test]
fn agrees_with_the_real_ledger_script() {
    let env = Env::new();
    let Some(lib) = real_ledger_lib(&env) else {
        return;
    };
    let a = env.state("inst-a");
    let b = env.state("inst-b");
    let a1 = Child::start("r1");
    let a2 = Child::start("r2-longer");
    let b1 = Child::start("rb1");
    let foreign = Child::start("rforeign");
    record(&lib, &a, &a1, "4", &["--session", "alpha"]);
    record(&lib, &a, &a2, "5", &[]);
    record(&lib, &b, &b1, "4", &[]);

    let sa = s(&a);
    let o = env.run_with_lib(&lib, &["stop", &sa, &foreign.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    let o = env.run_with_lib(&lib, &["stop", &sa, &b1.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    assert!(foreign.alive() && b1.alive());

    let o = env.run_with_lib(&lib, &["restart", &sa, &a2.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(text(&o).contains(&format!("RESTART-CMD {}", a2.cmd)), "{}", text(&o));
    assert!(!a2.alive());
    // a2 is gone now: the real ledger calls it stale, and it is not signalled again.
    let o = env.run_with_lib(&lib, &["stop", &sa, &a2.pid.to_string()], "");
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));

    let o = env.run_with_lib(&lib, &["teardown", &sa], "");
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(!a1.alive());
    assert!(b1.alive() && foreign.alive());
    assert!(b.join("wizard-ledger.toml").exists());
}
