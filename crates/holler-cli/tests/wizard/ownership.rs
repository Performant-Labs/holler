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

const FAKE_SS: &str = "#!/bin/bash\nfor p in $FAKE_SS_PIDS; do echo \"LISTEN 0 4096 127.0.0.1:1 0.0.0.0:* users:((\\\"x\\\",pid=$p,fd=3))\"; done\n";

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
        write_exec(&bin.join("ss"), FAKE_SS);
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
            .env("FAKE_SS_PIDS", lsof_pids)
            .env("FAKE_LSOF_PIDS", "")
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

fn assert_code(o: &Output, want: i32) {
    assert_eq!(o.status.code(), Some(want), "{}", text(o));
}

fn assert_has(o: &Output, needle: &str) {
    assert!(text(o).contains(needle), "{}", text(o));
}

fn assert_lacks(o: &Output, needle: &str) {
    assert!(!text(o).contains(needle), "{}", text(o));
}

fn assert_alive(c: &Child) {
    assert!(c.alive(), "pid {} must not have been signalled", c.pid);
}

fn assert_dead(c: &Child) {
    assert!(!c.alive(), "pid {} must have been stopped", c.pid);
}

fn assert_exists(p: &Path) {
    assert!(p.exists(), "{} must exist", p.display());
}

fn assert_missing(p: &Path) {
    assert!(!p.exists(), "{} must be gone", p.display());
}

fn assert_line(o: &Output, line: &str) {
    assert!(text(o).lines().any(|l| l == line), "{}", text(o));
}

fn assert_file_text(p: &Path, want: &str) {
    assert_eq!(fs::read_to_string(p).unwrap(), want);
}

fn assert_all_alive(cs: &[&Child]) {
    cs.iter().for_each(|c| assert_alive(c));
}

fn assert_all_dead(cs: &[&Child]) {
    cs.iter().for_each(|c| assert_dead(c));
}

#[test]
fn stop_signals_a_live_ledger_process() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("a");
    env.ledger(&st, &[(&c, "live")]);
    let o = env.run(&["stop", &s(&st), &c.pid.to_string()], "");
    assert_code(&o, 0);
    assert_dead(&c);
    assert_has(&o, "STOPPED");
}

#[test]
fn stop_refuses_a_reused_pid_and_reports_it_stale() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("reused");
    // The ledger recorded this pid, but the process now there is not the recorded one.
    env.ledger(&st, &[(&c, "stale")]);
    let o = env.run(&["stop", &s(&st), &c.pid.to_string()], "");
    assert_code(&o, 1);
    assert_has(&o, "STALE");
    assert_alive(&c);
}

#[test]
fn stop_refuses_a_pid_in_no_ledger() {
    let env = Env::new();
    let st = env.state("inst-a");
    let other = Child::start("other");
    let c = Child::start("recorded");
    env.ledger(&st, &[(&c, "live")]);
    let o = env.run(&["stop", &s(&st), &other.pid.to_string()], "");
    assert_code(&o, 2);
    assert_has(&o, "FOREIGN");
    assert_alive(&other);
    assert_alive(&c);
}

#[test]
fn rerun_reports_an_unrecorded_process_holding_a_planned_port_and_signals_nothing() {
    let env = Env::new();
    let st = env.state("inst-a");
    let mine = Child::start("mine");
    let squatter = Child::start("squatter");
    env.ledger(&st, &[(&mine, "live")]);
    let o = env.run(&["check-port", &s(&st), "47001"], &squatter.pid.to_string());
    assert_code(&o, 2);
    assert_has(&o, &squatter.pid.to_string());
    assert_has(&o, &squatter.cmd);
    assert_has(&o, "FOREIGN");
    assert_has(&o, "asks");
    assert_alive(&squatter);
    assert_alive(&mine);
}

#[test]
fn check_port_reports_a_reused_pid_holder_as_stale_and_a_ledger_holder_as_owned() {
    let env = Env::new();
    let st = env.state("inst-a");
    let reused = Child::start("reused");
    let mine = Child::start("mine");
    env.ledger(&st, &[(&reused, "stale"), (&mine, "live")]);
    let o = env.run(&["check-port", &s(&st), "47001"], &reused.pid.to_string());
    assert_code(&o, 1);
    assert_has(&o, "STALE");
    assert_alive(&reused);
    let o = env.run(&["check-port", &s(&st), "47002"], &mine.pid.to_string());
    assert_code(&o, 0);
    assert_has(&o, "OWNED");
    assert_alive(&mine);
}

#[test]
fn check_port_prefers_ss_when_lsof_lists_no_listener() {
    let env = Env::new();
    let st = env.state("inst-a");
    let squatter = Child::start("squatter");
    // The fake ss lists a listener; the fake lsof (a non-root view) lists none.
    let o = env.run(&["check-port", &s(&st), "47001"], &squatter.pid.to_string());
    assert_code(&o, 2);
    assert_has(&o, &squatter.pid.to_string());
    assert_lacks(&o, "FREE");
    assert_alive(&squatter);
}

#[test]
fn check_port_free_exits_zero() {
    let env = Env::new();
    let st = env.state("inst-a");
    let o = env.run(&["check-port", &s(&st), "47001"], "");
    assert_code(&o, 0);
    assert_has(&o, "FREE");
}

#[test]
fn restart_stops_the_recorded_process_and_prints_its_recorded_command() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("restart");
    env.ledger(&st, &[(&c, "live")]);
    let o = env.run(&["restart", &s(&st), &c.pid.to_string()], "");
    assert_code(&o, 0);
    assert_has(&o, &format!("RESTART-CMD {}", c.cmd));
    assert_dead(&c);
}

#[test]
fn restart_prints_a_note_that_the_recorded_command_alone_uses_the_default_state_directory() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("note");
    env.ledger(&st, &[(&c, "live")]);
    let o = env.run(&["restart", &s(&st), &c.pid.to_string()], "");
    assert_code(&o, 0);
    assert_line(
        &o,
        "RESTART-NOTE: re-run the stage's own start command (with HOLLER_STATE_DIR, nohup and the \
         log path), then record the new pid; this recorded command alone would use the default \
         state directory",
    );
}

#[test]
fn restart_of_a_stale_entry_signals_nothing_and_prints_no_command() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("restale");
    env.ledger(&st, &[(&c, "stale")]);
    let o = env.run(&["restart", &s(&st), &c.pid.to_string()], "");
    assert_code(&o, 1);
    assert_lacks(&o, "RESTART-CMD");
    assert_alive(&c);
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
    assert_code(&o, 0);
    assert_reverse_order(&text(&o), &a1, &a3, &a2);
    assert_all_dead(&[&a1, &a2, &a3]);
    assert_alive(&reused);
    assert_has(&o, &format!("STALE pid {}", reused.pid));
    assert_alive(&squatter);

    // The second instance: processes, ledger and state all untouched.
    assert_all_alive(&[&b1, &b2]);
    assert_exists(&b.join("fake-ledger"));
    assert_exists(&b.join("wizard-ledger.toml"));
    assert_file_text(&b.join("data.txt"), "b state");

    // This instance's ledger is gone; its other state stays, and the output says so.
    assert_missing(&a.join("wizard-ledger.toml"));
    assert_exists(&a.join("keep.txt"));
    assert_has(&o, "LEFT");
}

/// Same-stage entries stop highest pid first, then the lower stage (`low`) last.
fn assert_reverse_order(t: &str, x: &Child, y: &Child, low: &Child) {
    let (hi, lo) = if x.pid > y.pid { (x, y) } else { (y, x) };
    let pos = |c: &Child| t.find(&format!("STOPPED pid {} ", c.pid));
    assert!(pos(hi) < pos(lo), "{t}");
    assert!(pos(lo) < pos(low), "{t}");
    assert!(pos(low).is_some(), "{t}");
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
    assert_code(&o, 0);
    assert_alive(&theirs);
    let o = env.run(&["stop", &s(&a), &theirs.pid.to_string()], "");
    assert_code(&o, 2);
    assert_alive(&theirs);
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
    assert_code(&o, 0);
    assert_missing(&a);
    assert_exists(&b.join("data.txt"));
}

/// A fake home whose default state directory holds a ledger and a file, both of which must
/// survive a refused purge.
fn default_state_home(env: &Env) -> (PathBuf, PathBuf) {
    let home = env.state("home");
    let dflt = home.join(".holler");
    fs::create_dir_all(&dflt).unwrap();
    fs::write(dflt.join("data.txt"), "first instance").unwrap();
    (home, dflt)
}

fn purge_with_home(env: &Env, home: &Path, state: &str) -> Output {
    let path = format!(
        "{}:{}",
        env.root.path().join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new("bash")
        .arg(script())
        .args(["teardown", state, "--purge-state"])
        .env("WIZARD_LIB", env.lib())
        .env("PATH", path)
        .env("HOME", home)
        .env("FAKE_SS_PIDS", "")
        .env("FAKE_LSOF_PIDS", "")
        .output()
        .unwrap()
}

#[test]
fn purge_state_refuses_the_default_state_directory_and_removes_nothing() {
    let env = Env::new();
    let (home, dflt) = default_state_home(&env);
    let o = purge_with_home(&env, &home, &s(&dflt));
    assert_has(&o, "LEFT");
    assert_lacks(&o, "REMOVED");
    assert_exists(&dflt.join("data.txt"));
    assert_exists(&dflt);
}

#[test]
fn purge_state_refuses_the_default_state_directory_spelled_with_a_trailing_slash() {
    let env = Env::new();
    let (home, dflt) = default_state_home(&env);
    let o = purge_with_home(&env, &home, &format!("{}/", s(&dflt)));
    assert_lacks(&o, "REMOVED");
    assert_exists(&dflt.join("data.txt"));
}

#[test]
fn nothing_is_signalled_when_the_ledger_script_is_missing() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("noledger");
    env.ledger(&st, &[(&c, "live")]);
    fs::remove_file(env.lib().join("ledger.sh")).unwrap();
    let o = env.run(&["stop", &s(&st), &c.pid.to_string()], "");
    assert_code(&o, 4);
    assert_alive(&c);
}

#[test]
fn unusable_pids_are_refused() {
    let env = Env::new();
    let st = env.state("inst-a");
    for bad in ["1", "0", "abc", ""] {
        assert_code(&env.run(&["stop", &s(&st), bad], ""), 4);
    }
}

#[test]
fn restart_prints_the_recorded_command_when_the_session_field_is_empty() {
    let env = Env::new();
    let st = env.state("inst-a");
    let c = Child::start("nosession");
    env.ledger_full(&st, &[(&c, "live", 7, "")]);
    let o = env.run(&["restart", &s(&st), &c.pid.to_string()], "");
    assert_code(&o, 0);
    let line = format!("RESTART-CMD {}", c.cmd);
    assert_line(&o, &line);
    assert_dead(&c);
}

/// The repository's own ledger.sh (story #729), copied into a temp lib dir. `None` (with a
/// printed note) until that story's file is in this tree.
fn real_ledger_lib(env: &Env) -> Option<PathBuf> {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../agent-skills/setup-wizard/lib/ledger.sh");
    if !src.exists() {
        eprintln!(
            "note: ledger.sh not in this tree yet ({}); skipping",
            src.display()
        );
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

fn run_real(env: &Env, lib: &Path, args: &[&str]) -> Output {
    env.run_with_lib(lib, args, "")
}

fn assert_real_refuses(env: &Env, lib: &Path, state: &Path, c: &Child) {
    let o = run_real(env, lib, &["stop", &s(state), &c.pid.to_string()]);
    assert_code(&o, 2);
    assert_alive(c);
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

    assert_real_refuses(&env, &lib, &a, &foreign);
    assert_real_refuses(&env, &lib, &a, &b1);
    assert_real_restart_then_stale(&env, &lib, &a, &a2);

    assert_code(&run_real(&env, &lib, &["teardown", &s(&a)]), 0);
    assert_dead(&a1);
    assert_all_alive(&[&b1, &foreign]);
    assert_exists(&b.join("wizard-ledger.toml"));
}

fn assert_real_restart_then_stale(env: &Env, lib: &Path, state: &Path, c: &Child) {
    let o = run_real(env, lib, &["restart", &s(state), &c.pid.to_string()]);
    assert_code(&o, 0);
    assert_has(&o, &format!("RESTART-CMD {}", c.cmd));
    assert_dead(c);
    // Gone now: the real ledger calls it stale, and it is not signalled again.
    let o = run_real(env, lib, &["stop", &s(state), &c.pid.to_string()]);
    assert_code(&o, 1);
}
