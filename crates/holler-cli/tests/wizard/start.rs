//! Setup wizard story #729 (epic #726): the per-instance process ledger, `ledger.sh`.
//!
//! Runs `agent-skills/setup-wizard/lib/ledger.sh` with `std::process::Command` in temp
//! directories. `holler`, `opencode` and `tailscale` are fakes placed first on `PATH`; every
//! state directory is a fresh temp directory, every process is a throwaway `sleep` started by
//! a test, and is killed by the pid the ledger recorded (never by name).
//!
//! Unix-only: the scripts are bash.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #729

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ledger_sh() -> PathBuf {
    repo_root().join("agent-skills/setup-wizard/lib/ledger.sh")
}

/// A scratch world: fakes on `PATH`, plus every pid a test started (killed on drop).
struct World {
    root: tempfile::TempDir,
    pids: std::cell::RefCell<Vec<u32>>,
}

const FAKE_HOLLER: &str = r#"#!/bin/sh
# Fake holler: state lives in $HOLLER_STATE_DIR, the way the real one's does.
S="${HOLLER_STATE_DIR:?}"
mkdir -p "$S/tokens"
case "$1 $2" in
  "hub serve") echo "$*" >> "$S/hub-args"; exec sleep 600 ;;
  "hub token") label="$5"; id="tok-$label"; echo "secret-$$" > "$S/tokens/$id"
               echo "$id:$(cat "$S/tokens/$id")" ;;
  "body join") id="${6%%:*}"
               [ -f "$S/tokens/$id" ] || { echo 'no such token' >&2; exit 1; }
               echo "$id" > "$S/joined" ;;
  "body run") [ -f "$S/joined" ] || { echo 'not joined' >&2; exit 1; }
              echo "$(cat "$S/joined") connected" >> "$S/roster"; exec sleep 600 ;;
  "roster ") cat "$S/roster" 2>/dev/null ;;
  *) echo "fake holler: unhandled: $*" >&2; exit 2 ;;
esac
"#;

const FAKE_OPENCODE: &str = r#"#!/bin/sh
echo "$*" >> "${HOLLER_STATE_DIR:?}/opencode-args"
exec sleep 600
"#;

const FAKE_TAILSCALE: &str = r#"#!/bin/sh
echo "$*" >> "${HOLLER_STATE_DIR:?}/tailscale-args"
"#;

fn write_exec(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

impl World {
    fn new() -> World {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        write_exec(&bin.join("holler"), FAKE_HOLLER);
        write_exec(&bin.join("opencode"), FAKE_OPENCODE);
        write_exec(&bin.join("tailscale"), FAKE_TAILSCALE);
        World { root, pids: Default::default() }
    }

    fn state(&self, name: &str) -> PathBuf {
        let d = self.root.path().join(name);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn path(&self) -> String {
        format!(
            "{}:{}",
            self.root.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }

    /// A bash command with the fakes first on PATH and the given state directory.
    fn bash(&self, state: &Path, script: &str) -> Command {
        let mut c = Command::new("bash");
        c.arg("-c")
            .arg(script)
            .env("PATH", self.path())
            .env("HOLLER_STATE_DIR", state)
            .env("LEDGER", ledger_sh())
            .env("HOME", self.root.path().join("home"))
            .stdin(Stdio::null());
        c
    }

    fn ledger(&self, state: &Path, args: &[&str]) -> Output {
        let mut c = Command::new("bash");
        c.arg(ledger_sh())
            .args(args)
            .env("PATH", self.path())
            .env("HOLLER_STATE_DIR", state)
            .env("HOME", self.root.path().join("home"))
            .stdin(Stdio::null());
        c.output().unwrap()
    }

    /// Start a throwaway `sleep <secs>` this test owns; returns its pid.
    fn sleeper(&self, secs: &str) -> u32 {
        let child = Command::new("sleep")
            .arg(secs)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        self.pids.borrow_mut().push(pid);
        std::mem::forget(child); // reaped by init when the test process exits; killed on drop
        pid
    }

    fn track(&self, pid: u32) {
        self.pids.borrow_mut().push(pid);
    }
}

impl Drop for World {
    fn drop(&mut self) {
        for pid in self.pids.borrow().iter() {
            // Only pids this test started and recorded.
            let _ = Command::new("kill").arg(pid.to_string()).stderr(Stdio::null()).status();
        }
    }
}

fn text(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn code(o: &Output) -> i32 {
    o.status.code().unwrap()
}

/// The ledger's `list` output, one `Vec<String>` of tab-separated fields per entry.
fn list(w: &World, state: &Path) -> Vec<Vec<String>> {
    let o = w.ledger(state, &["list"]);
    assert_eq!(code(&o), 0, "list failed: {}", String::from_utf8_lossy(&o.stderr));
    text(&o)
        .lines()
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

fn record(
    w: &World,
    state: &Path,
    pid: u32,
    role: &str,
    stage: &str,
    session: Option<&str>,
) -> Output {
    let pid = pid.to_string();
    let mut args = vec!["record", "--pid", &pid, "--role", role, "--stage", stage];
    if let Some(s) = session {
        args.push("--session");
        args.push(s);
    }
    w.ledger(state, &args)
}

#[test]
fn ledger_script_exists_and_parses() {
    assert!(ledger_sh().is_file(), "ledger.sh is missing");
    let o = Command::new("bash").arg("-n").arg(ledger_sh()).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn record_writes_the_contract_fields_atomically_with_mode_0600() {
    let w = World::new();
    let st = w.state("a");
    let pid = w.sleeper("601");
    let o = record(&w, &st, pid, "backend", "4", Some("alpha"));
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));

    let file = st.join("wizard-ledger.toml");
    let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let body = fs::read_to_string(&file).unwrap();
    assert!(body.contains("[[process]]"));
    assert!(body.contains(&format!("pid = {pid}\n")));
    assert!(body.contains("role = \"backend\""));
    assert!(body.contains("stage = 4\n"));
    assert!(body.contains("session = \"alpha\""));
    assert!(body.contains("cmd = \"sleep 601\""));
    assert!(body.contains("started = \""));
    // The recorded start time is exactly `LC_ALL=C ps -o lstart= -p <pid>`, trimmed.
    let ps = Command::new("ps")
        .env("LC_ALL", "C")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    let started = text(&ps).trim().to_string();
    assert!(body.contains(&format!("started = \"{started}\"")), "{body}");
    // No temp file is left behind by the atomic write, and no lock.
    let names: Vec<String> = fs::read_dir(&st)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["wizard-ledger.toml".to_string()], "{names:?}");
}

#[test]
fn default_state_dir_is_dot_holler_under_home() {
    let w = World::new();
    let home = w.root.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let pid = w.sleeper("602");
    let o = Command::new("bash")
        .arg(ledger_sh())
        .args(["record", "--pid", &pid.to_string(), "--role", "hub", "--stage", "6"])
        .env_remove("HOLLER_STATE_DIR")
        .env("HOME", &home)
        .output()
        .unwrap();
    assert_eq!(code(&o), 0, "{}", String::from_utf8_lossy(&o.stderr));
    assert!(home.join(".holler/wizard-ledger.toml").is_file());
}

#[test]
fn ledger_lists_exactly_what_was_recorded_and_never_a_foreign_process() {
    let w = World::new();
    let st = w.state("a");
    let mine1 = w.sleeper("603");
    let mine2 = w.sleeper("604");
    let foreign = w.sleeper("605");
    assert_eq!(code(&record(&w, &st, mine1, "backend", "4", Some("alpha"))), 0);
    assert_eq!(code(&record(&w, &st, mine2, "hub", "6", None)), 0);

    let rows = list(&w, &st);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0][0], mine1.to_string());
    assert_eq!(rows[0][1], "live");
    assert_eq!(rows[0][2], "backend");
    assert_eq!(rows[0][3], "4");
    assert_eq!(rows[0][4], "alpha");
    assert_eq!(rows[0][5], "sleep 603");
    assert_eq!(rows[1][0], mine2.to_string());
    assert_eq!(rows[1][2], "hub");
    assert_eq!(rows[1][4], "", "an absent session is empty");
    assert!(rows.iter().all(|r| r[0] != foreign.to_string()));

    assert_eq!(code(&w.ledger(&st, &["owns", &mine1.to_string()])), 0);
    assert_eq!(code(&w.ledger(&st, &["owns", &foreign.to_string()])), 2);
}

#[test]
fn list_on_a_missing_ledger_is_empty_and_owns_says_not_recorded() {
    let w = World::new();
    let st = w.state("a");
    assert!(list(&w, &st).is_empty());
    assert_eq!(code(&w.ledger(&st, &["owns", "1"])), 2);
}

#[test]
fn recording_a_pid_that_does_not_exist_fails_and_writes_nothing() {
    let w = World::new();
    let st = w.state("a");
    // A pid that cannot exist: above any kernel's pid_max.
    let o = w.ledger(
        &st,
        &["record", "--pid", "999999999", "--role", "backend", "--stage", "4"],
    );
    assert_eq!(code(&o), 1);
    assert!(!st.join("wizard-ledger.toml").exists());
}

#[test]
fn record_rejects_bad_arguments() {
    let w = World::new();
    let st = w.state("a");
    let pid = w.sleeper("606").to_string();
    for args in [
        vec!["record", "--pid", "x", "--role", "hub", "--stage", "6"],
        vec!["record", "--pid", &pid, "--role", "nonsense", "--stage", "6"],
        vec!["record", "--pid", &pid, "--role", "hub", "--stage", "3"],
        vec!["record", "--pid", &pid, "--role", "hub"],
        vec!["frobnicate"],
    ] {
        let o = w.ledger(&st, &args);
        assert_ne!(code(&o), 0, "{args:?}");
        assert!(
            !matches!(code(&o), 1 | 2),
            "usage errors must not look like stale/foreign: {args:?}"
        );
    }
    assert!(!st.join("wizard-ledger.toml").exists());
}

#[test]
fn a_dead_recorded_process_is_stale_and_never_live() {
    let w = World::new();
    let st = w.state("a");
    let pid = w.sleeper("607");
    assert_eq!(code(&record(&w, &st, pid, "backend", "4", Some("alpha"))), 0);
    assert_eq!(code(&w.ledger(&st, &["owns", &pid.to_string()])), 0);

    // Kill the very pid we recorded, then wait until it is gone (a zombie still shows in ps
    // until reaped, so reap it via wait on a fresh handle is not possible; poll `kill -0`/ps).
    assert!(Command::new("kill").arg(pid.to_string()).status().unwrap().success());
    let mut gone = false;
    for _ in 0..100 {
        let ps = Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let stat = text(&ps).trim().to_string();
        if stat.is_empty() || stat.starts_with('Z') {
            gone = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(gone);
    let owns = code(&w.ledger(&st, &["owns", &pid.to_string()]));
    // A zombie keeps its start time and an empty command line on some systems; either way it
    // is not the process that was recorded.
    assert_eq!(owns, 1);
    let rows = list(&w, &st);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][1], "stale");
}

#[test]
fn a_reused_pid_with_another_start_time_or_command_is_stale() {
    let w = World::new();
    let st = w.state("a");
    let live = w.sleeper("608");
    // Hand-written ledger: the pid is alive, but what is recorded is a different process.
    let ps_started = |pid: u32| {
        let o = Command::new("ps")
            .env("LC_ALL", "C")
            .args(["-o", "lstart=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        text(&o).trim().to_string()
    };
    let started = ps_started(live);
    let entry = |started: &str, cmd: &str| {
        format!(
            "[[process]]\npid = {live}\nstarted = \"{started}\"\ncmd = \"{cmd}\"\n\
             role = \"backend\"\nstage = 4\nsession = \"alpha\"\n"
        )
    };
    let file = st.join("wizard-ledger.toml");

    fs::write(&file, entry(&started, "opencode --port 47001")).unwrap();
    assert_eq!(code(&w.ledger(&st, &["owns", &live.to_string()])), 1, "different command");
    assert_eq!(list(&w, &st)[0][1], "stale");

    fs::write(&file, entry("Thu Jan  1 00:00:00 1970", "sleep 608")).unwrap();
    assert_eq!(code(&w.ledger(&st, &["owns", &live.to_string()])), 1, "different start time");

    fs::write(&file, entry(&started, "sleep 608")).unwrap();
    assert_eq!(code(&w.ledger(&st, &["owns", &live.to_string()])), 0, "matching entry is live");
}

#[test]
fn recording_the_same_pid_twice_keeps_one_entry() {
    let w = World::new();
    let st = w.state("a");
    let pid = w.sleeper("609");
    assert_eq!(code(&record(&w, &st, pid, "backend", "4", Some("alpha"))), 0);
    assert_eq!(code(&record(&w, &st, pid, "backend", "4", Some("alpha"))), 0);
    assert_eq!(list(&w, &st).len(), 1);
}

#[test]
fn a_command_with_quotes_and_backslashes_round_trips() {
    let w = World::new();
    let st = w.state("a");
    let child = Command::new("bash")
        .args(["-c", "exec -a 'x \"q\" \\\\b' sleep 610"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    w.track(pid);
    std::mem::forget(child);
    assert_eq!(code(&record(&w, &st, pid, "body", "7", Some("alpha"))), 0);
    assert_eq!(code(&w.ledger(&st, &["owns", &pid.to_string()])), 0);
}

/// The skill's Stage 4 to 7 start-or-reuse step, as the skill text spells it out: look the
/// role and session up in the ledger, reuse a live one, otherwise start and record.
const START_OR_REUSE: &str = r#"
start_or_reuse() { # role stage session -- command...
  role="$1"; stage="$2"; session="$3"; shift 4
  pid=$(bash "$LEDGER" list | awk -F'\t' -v r="$role" -v s="$session" \
    '$2=="live" && $3==r && $5==s {print $1; exit}')
  if [ -n "$pid" ] && bash "$LEDGER" owns "$pid"; then echo "reuse $pid"; return 0; fi
  nohup "$@" >/dev/null 2>&1 &
  pid=$!
  sleep 0.3 # the skill records only once the process is up (its Verify step), not mid-exec
  bash "$LEDGER" record --pid "$pid" --role "$role" --stage "$stage" \
    --session "$session" || return 1
  echo "start $pid"
}
"#;

fn run_start(w: &World, st: &Path, script: &str) -> String {
    let o = w
        .bash(st, &format!("{START_OR_REUSE}\n{script}"))
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    text(&o).trim().to_string()
}

fn pid_of(line: &str) -> u32 {
    line.split_whitespace().last().unwrap().parse().unwrap()
}

#[test]
fn a_rerun_reuses_a_live_recorded_process_and_restarts_a_dead_one() {
    let w = World::new();
    let st = w.state("a");
    let start = r#"start_or_reuse backend 4 alpha -- opencode --port 47001 --hostname 0.0.0.0"#;

    let first = run_start(&w, &st, start);
    assert!(first.starts_with("start "), "{first}");
    let pid1 = pid_of(&first);
    w.track(pid1);

    // Rerun with the process alive and matching: reused, nothing new recorded.
    let second = run_start(&w, &st, start);
    assert_eq!(second, format!("reuse {pid1}"));
    assert_eq!(list(&w, &st).len(), 1);

    // The process dies: a rerun starts a new one and records it.
    assert!(Command::new("kill").arg(pid1.to_string()).status().unwrap().success());
    for _ in 0..100 {
        if code(&w.ledger(&st, &["owns", &pid1.to_string()])) != 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let third = run_start(&w, &st, start);
    assert!(third.starts_with("start "), "{third}");
    let pid3 = pid_of(&third);
    w.track(pid3);
    assert_ne!(pid3, pid1);
    let rows = list(&w, &st);
    assert!(rows.iter().any(|r| r[0] == pid3.to_string() && r[1] == "live"), "{rows:?}");
    assert!(rows.iter().any(|r| r[0] == pid1.to_string() && r[1] == "stale"), "{rows:?}");
}

#[test]
fn a_rerun_never_adopts_a_look_alike_process_it_did_not_record() {
    let w = World::new();
    let st = w.state("a");
    // Same command line as the backend would have, started by someone else, not recorded.
    let foreign = w.sleeper("611");
    let start = r#"start_or_reuse backend 4 alpha -- opencode --port 47001"#;
    let out = run_start(&w, &st, start);
    assert!(out.starts_with("start "), "{out}");
    w.track(pid_of(&out));
    let rows = list(&w, &st);
    assert_eq!(rows.len(), 1);
    assert!(rows.iter().all(|r| r[0] != foreign.to_string()));
}

/// The instance start sequence of Stages 4 to 7, in one instance's own state directory, with
/// the instance's own ports. Prints the join line it minted, for the isolation checks.
const START_INSTANCE: &str = r#"
start_or_reuse backend 4 alpha -- opencode --port "$BACKEND_PORT" --hostname 127.0.0.1
start_or_reuse hub 6 "" -- holler hub serve --listen "127.0.0.1:$HUB_PORT" \
  --advertise loopback.example.ts.net
tailscale serve --bg --https "$SERVE_PORT" "$HUB_PORT"
JOIN=$(holler hub token mint --label "$NAME-body")
echo "$JOIN" > "$HOLLER_STATE_DIR/join-line"
holler body join --server wss://loopback.example.ts.net --token "$JOIN" --hub-key k
start_or_reuse body 7 alpha -- holler body run --config "$HOLLER_STATE_DIR/sessions.toml"
"#;

fn start_instance(w: &World, st: &Path, name: &str, hub: &str, serve: &str, backend: &str) {
    let o = w
        .bash(st, &format!("{START_OR_REUSE}\n{START_INSTANCE}"))
        .env("NAME", name)
        .env("HUB_PORT", hub)
        .env("SERVE_PORT", serve)
        .env("BACKEND_PORT", backend)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    for row in list(w, st) {
        w.track(row[0].parse().unwrap());
    }
}

#[test]
fn two_instances_on_loopback_keep_separate_rosters_ledgers_and_tokens() {
    let w = World::new();
    let one = w.state("instance-one");
    let two = w.state("instance-two");
    start_instance(&w, &one, "one", "41807", "443", "47001");
    // The second instance is told apart by its own ports and state directory alone.
    start_instance(&w, &two, "two", "41808", "8443", "47011");

    // Each ledger lists exactly its own three processes, and only those.
    let (a, b) = (list(&w, &one), list(&w, &two));
    assert_eq!(a.len(), 3, "{a:?}");
    assert_eq!(b.len(), 3, "{b:?}");
    let roles = |rows: &[Vec<String>]| rows.iter().map(|r| r[2].clone()).collect::<Vec<_>>();
    assert_eq!(roles(&a), ["backend", "hub", "body"]);
    assert_eq!(roles(&b), ["backend", "hub", "body"]);
    for ra in &a {
        assert!(b.iter().all(|rb| rb[0] != ra[0]), "pid in both ledgers: {ra:?}");
        // One instance's ledger does not own the other's processes.
        assert_eq!(code(&w.ledger(&two, &["owns", &ra[0]])), 2);
    }
    assert!(a.iter().all(|r| r[1] == "live") && b.iter().all(|r| r[1] == "live"));

    // Each hub's roster lists only its own body.
    let roster = |st: &Path| {
        text(&w.bash(st, "holler roster").output().unwrap())
    };
    assert_eq!(roster(&one).trim(), "tok-one-body connected");
    assert_eq!(roster(&two).trim(), "tok-two-body connected");

    // Each instance used its own ports and state directory.
    let args = |st: &Path, f: &str| fs::read_to_string(st.join(f)).unwrap();
    assert!(args(&one, "hub-args").contains("127.0.0.1:41807"));
    assert!(args(&two, "hub-args").contains("127.0.0.1:41808"));
    assert!(args(&one, "opencode-args").contains("--port 47001"));
    assert!(args(&two, "opencode-args").contains("--port 47011"));
    assert!(args(&one, "tailscale-args").contains("--https 443"));
    assert!(args(&two, "tailscale-args").contains("--https 8443"));

    // A body of one instance cannot join the other: the token does not exist there.
    let join_one = fs::read_to_string(one.join("join-line")).unwrap();
    let o = w
        .bash(
            &two,
            &format!(
                "holler body join --server wss://loopback.example.ts.net --token {} --hub-key k",
                join_one.trim()
            ),
        )
        .output()
        .unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("no such token"));
    assert_eq!(roster(&two).trim(), "tok-two-body connected");
}

#[test]
fn the_instance_state_dir_holds_only_its_own_ledger() {
    let w = World::new();
    let one = w.state("instance-one");
    let two = w.state("instance-two");
    start_instance(&w, &one, "one", "41807", "443", "47001");
    assert!(one.join("wizard-ledger.toml").is_file());
    assert!(!two.join("wizard-ledger.toml").exists());
}
