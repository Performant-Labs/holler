#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #641
//! `TmuxHost` against a real tmux (#641, AC 1-5 and AC 11-14). Every test is
//! `#[ignore]` (the issue's opt-in): run them with
//! `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored`. Each returns
//! early, saying why, when `tmux -V` cannot run.
//!
//! Every host is built by [`private`] on its own tmux server, on a socket in a fresh
//! `/tmp/hlr-tmux-*` directory, with a config that forces `/bin/sh` (AC 9). The guard
//! kills that server and removes its directory. No test touches the default socket or
//! names a session outside `demo-*`. The real `kill` is used, and only ever on a process
//! the test started on its private server (Decision 14).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use holler_adapter_host::{TmuxHost, TmuxSocket};
use holler_pane::{Argv, HostPort, PaneError, PaneName};
use holler_pane_testkit::conformance::host::run_host_conformance;
use tempfile::TempDir;

/// The `sun_path` limit the socket path must stay under.
const SUN_PATH: usize = 100;

/// The longest a bounded wait polls.
const WAIT: Duration = Duration::from_secs(1);

/// A private tmux server's directory; dropping it kills the server, then removes the
/// directory.
struct Server {
    dir: TempDir,
}

impl Server {
    fn sock(&self) -> PathBuf {
        self.dir.path().join("s")
    }

    fn config(&self) -> PathBuf {
        self.dir.path().join("tmux.conf")
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Run tmux on this server only, with no inherited `TMUX`.
    fn tmux(&self, args: &[&str]) -> Output {
        Command::new("tmux")
            .arg("-S")
            .arg(self.sock())
            .arg("-f")
            .arg(self.config())
            .args(args)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    /// The trimmed stdout lines of a tmux call that must succeed.
    fn lines(&self, args: &[&str]) -> Vec<String> {
        let out = self.tmux(args);
        assert!(out.status.success(), "tmux {args:?}: {out:?}");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn sessions(&self) -> Vec<String> {
        self.lines(&["list-sessions", "-F", "#{session_name}"])
    }

    fn session_path(&self, session: &str) -> String {
        let target = format!("={session}:");
        self.lines(&["list-panes", "-t", &target, "-F", "#{session_path}"])
            .into_iter()
            .next()
            .unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.tmux(&["kill-server"]);
    }
}

fn tmux_available() -> bool {
    let found = Command::new("tmux")
        .arg("-V")
        .output()
        .is_ok_and(|o| o.status.success());
    if !found {
        eprintln!("skipped: tmux not found");
    }
    found
}

/// The one helper every real-tmux test builds its host with (AC 9).
fn private() -> (TmuxHost, Server) {
    let dir = tempfile::Builder::new()
        .prefix("hlr-tmux-")
        .tempdir_in("/tmp")
        .unwrap();
    let server = Server { dir };
    let sock = server.sock();
    assert!(
        sock.as_os_str().len() < SUN_PATH,
        "{} is too long",
        sock.display()
    );
    fs::write(server.config(), "set -g default-shell /bin/sh\n").unwrap();
    let host = TmuxHost::new(TmuxSocket::Path(sock)).with_config(server.config());
    (host, server)
}

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|&p| p.to_owned()).collect())
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Whether `pid` exists, asked with signal 0 through the `kill` binary.
fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-s", "0", "--", &pid.to_string()])
        .env("LC_ALL", "C")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Poll every 20 ms until `done` holds or [`WAIT`] passes; whether it held.
fn wait_until(mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + WAIT;
    loop {
        if done() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// The file's text once it ends in a newline, within [`WAIT`].
fn written(path: &Path) -> String {
    let mut seen = String::new();
    wait_until(|| {
        seen = fs::read_to_string(path).unwrap_or_default();
        seen.ends_with('\n')
    });
    seen
}

/// Ensure `session` working in `cwd`, which must succeed.
fn ensured(host: &TmuxHost, session: &str, cwd: &Path) -> PaneName {
    let n = name(session);
    host.ensure_session(&n, &text(cwd)).expect("ensure_session");
    n
}

/// Run `parts` in `n`, which must succeed; the pid it added to `ps`.
fn started(host: &TmuxHost, n: &PaneName, parts: &[&str]) -> u32 {
    let before = host.ps(n).expect("ps before run");
    host.run(n, &argv(parts)).expect("run");
    let after = host.ps(n).expect("ps after run");
    let added: Vec<u32> = after.into_iter().filter(|p| !before.contains(p)).collect();
    assert_eq!(added.len(), 1, "run added {added:?}");
    added[0]
}

/// `stop_owned`, which must succeed within the host's 10 s bound.
fn stopped(host: &TmuxHost, n: &PaneName) {
    let begun = Instant::now();
    assert_eq!(host.stop_owned(n), Ok(()));
    assert!(
        begun.elapsed() < Duration::from_secs(10),
        "{:?}",
        begun.elapsed()
    );
}

/// A dir under the server's directory, made first.
fn made(server: &Server, rel: &str) -> PathBuf {
    let dir = server.path(rel);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// --- AC 1 ---

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn the_tmux_host_passes_the_host_conformance_suite() {
    if !tmux_available() {
        return;
    }
    assert_eq!(run_host_conformance(private), Ok(()));
}

// --- AC 2, AC 3, AC 4, AC 5 ---

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn ensure_session_twice_makes_one_session() {
    if !tmux_available() {
        return;
    }
    let (host, server) = private();
    let (first, second) = (made(&server, "first"), made(&server, "second"));
    let n = ensured(&host, "demo-c1r1", &first);
    assert_eq!(host.ensure_session(&n, &text(&second)), Ok(()));
    assert_eq!(server.sessions(), ["demo-c1r1"]);
    assert_eq!(server.session_path("demo-c1r1"), text(&first));
    assert_eq!(host.ensure_session(&n, &text(&server.path("nope"))), Ok(()));
    assert_eq!(server.sessions(), ["demo-c1r1"]);
    assert_eq!(server.session_path("demo-c1r1"), text(&first));
}

/// Kills the outside `sleep` when the test ends, however it ends.
struct Outside(Child);

impl Drop for Outside {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn stop_owned_kills_only_the_owned_process() {
    if !tmux_available() {
        return;
    }
    let (host, server) = private();
    let a = ensured(&host, "demo-c1r1", server.dir.path());
    let b = ensured(&host, "demo-c2r1", server.dir.path());
    let shell = host.ps(&a).expect("ps of a fresh session");
    let in_a = started(&host, &a, &["sleep", "30"]);
    let in_b = started(&host, &b, &["sleep", "30"]);
    let outside = Outside(Command::new("sleep").arg("30").spawn().unwrap());
    stopped(&host, &a);
    assert!(wait_until(|| !alive(in_a)), "the owned {in_a} survived");
    let left = host
        .ps(&a)
        .expect("ps after stop_owned: the session survives");
    assert!(!left.contains(&in_a), "{left:?}");
    assert!(
        shell.iter().all(|p| left.contains(p)),
        "the shell pane {shell:?}: {left:?}"
    );
    assert!(server.sessions().contains(&"demo-c1r1".to_owned()));
    assert!(
        alive(in_b) && host.ps(&b).expect("ps(b)").contains(&in_b),
        "{in_b}"
    );
    assert!(alive(outside.0.id()), "the outside sleep was signalled");
}

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn stop_owned_escalates_to_kill() {
    if !tmux_available() {
        return;
    }
    let (host, server) = private();
    let host = host.with_stop_grace(Duration::from_millis(200));
    let n = ensured(&host, "demo-c1r1", server.dir.path());
    let stubborn = started(&host, &n, &["sh", "-c", "trap '' TERM HUP; sleep 30"]);
    stopped(&host, &n);
    assert!(
        wait_until(|| !alive(stubborn)),
        "{stubborn} ignored TERM and survived"
    );

    let pidfile = server.path("member.pid");
    let leader = "sh -c \"trap '' TERM HUP; exec sleep 30\" & echo $! > \"$0\"; wait";
    host.run(&n, &argv(&["sh", "-c", leader, &text(&pidfile)]))
        .expect("run");
    let member: u32 = written(&pidfile).trim().parse().unwrap();
    assert!(alive(member), "the member {member} never started");
    stopped(&host, &n);
    assert!(
        wait_until(|| !alive(member)),
        "the member {member} outlived its leader (W-14)"
    );
}

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn missing_session_is_typed() {
    if !tmux_available() {
        return;
    }
    let (host, server) = private();
    assert!(!server.sock().exists(), "no server was ever started");
    let n = name("demo-c1r1");
    let ps = host.ps(&n);
    assert!(
        matches!(ps, Err(PaneError::PaneNotFound { .. })),
        "ps: {ps:?}"
    );
    let run = host.run(&n, &argv(&["sleep", "30"]));
    assert!(
        matches!(run, Err(PaneError::PaneNotFound { .. })),
        "run: {run:?}"
    );
    assert_eq!(host.stop_owned(&n), Ok(()));
}

// --- AC 11, AC 12, AC 13, AC 14 ---

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn argv_and_cwd_pass_exactly() {
    if !tmux_available() {
        return;
    }
    let (host, server) = private();
    let ran = server.path("ran");
    let hash = made(&server, "p#S");
    let job = made(&server, &format!("#(touch {})", text(&ran)));
    let c1 = ensured(&host, "demo-c1r1", &hash);
    let c2 = ensured(&host, "demo-c2r1", &job);

    let args_out = server.path("args.out");
    let elements = [
        "x;",
        "y\\;",
        ";",
        "rename-session",
        "pwned",
        "#{session_name}",
    ];
    let mut parts = vec!["sh", "-c", "printf '%s\\n' \"$@\" > \"$0\""];
    let out = text(&args_out);
    parts.push(&out);
    parts.extend(elements);
    host.run(&c1, &argv(&parts)).expect("run");
    let got = written(&args_out);
    assert_eq!(got.lines().collect::<Vec<_>>(), elements, "{got:?}");
    assert!(
        server.sessions().contains(&"demo-c1r1".to_owned()),
        "{:?}",
        server.sessions()
    );

    for (session, n, cwd) in [("demo-c1r1", &c1, &hash), ("demo-c2r1", &c2, &job)] {
        assert_eq!(server.session_path(session), text(cwd));
        let pwd = server.path(&format!("{session}.pwd"));
        let pwd_text = text(&pwd);
        host.run(n, &argv(&["sh", "-c", "pwd -P > \"$0\"", &pwd_text]))
            .expect("run");
        let want = format!("{}\n", text(&fs::canonicalize(cwd).unwrap()));
        assert_eq!(written(&pwd), want, "{session}");
    }
    assert!(
        !wait_until(|| ran.exists()),
        "#(...) in a cwd ran a shell command"
    );
}

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn targets_are_exact() {
    if !tmux_available() {
        return;
    }
    let (host, server) = private();
    let long = ensured(&host, "demo-c1r10", server.dir.path());
    let p = started(&host, &long, &["sleep", "30"]);
    let short = name("demo-c1r1");
    let check = |when: &str| {
        let ps = host.ps(&short);
        assert!(
            matches!(ps, Err(PaneError::PaneNotFound { .. })),
            "{when}: {ps:?}"
        );
        assert_eq!(host.stop_owned(&short), Ok(()), "{when}");
        assert!(alive(p), "{when}: {p} was signalled");
        assert!(
            host.ps(&long).expect("ps(demo-c1r10)").contains(&p),
            "{when}"
        );
    };
    check("a prefix");
    ensured(&host, "demo-c2r1", server.dir.path());
    let made_window = server.tmux(&["new-window", "-d", "-n", "demo-c1r1", "-t", "=demo-c2r1:"]);
    assert!(made_window.status.success(), "{made_window:?}");
    check("a window name in the newest session");
}

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn run_works_in_the_session_cwd() {
    if !tmux_available() {
        return;
    }
    let (host, server) = private();
    let a = made(&server, "A");
    assert_ne!(
        fs::canonicalize(&a).unwrap(),
        env::current_dir().unwrap().canonicalize().unwrap()
    );
    let n = ensured(&host, "demo-c1r1", &a);
    let out = server.path("cwd.out");
    host.run(&n, &argv(&["sh", "-c", "pwd -P > \"$0\"", &text(&out)]))
        .expect("run");
    let want = format!("{}\n", text(&fs::canonicalize(&a).unwrap()));
    assert_eq!(written(&out), want);
}

#[test]
#[ignore = "needs tmux; run with --ignored"]
fn run_refuses_a_missing_or_relative_directory() {
    if !tmux_available() {
        return;
    }
    let (host, server) = private();
    let w = made(&server, "W");
    let n = ensured(&host, "demo-c1r1", &w);
    fs::remove_dir(&w).unwrap();
    let run = host.run(&n, &argv(&["sleep", "30"]));
    assert!(matches!(run, Err(PaneError::Unavailable { .. })), "{run:?}");
    let panes = server.lines(&["list-panes", "-s", "-t", "=demo-c1r1:", "-F", "#{pane_pid}"]);
    assert_eq!(panes.len(), 1, "only the shell pane: {panes:?}");

    let relative = host.ensure_session(&name("demo-c2r1"), ".");
    assert!(
        matches!(relative, Err(PaneError::Usage { .. })),
        "{relative:?}"
    );
    assert!(
        !server.sessions().contains(&"demo-c2r1".to_owned()),
        "{:?}",
        server.sessions()
    );
}
