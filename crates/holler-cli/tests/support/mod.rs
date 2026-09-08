#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! Shared test harness (story #138).
//!
//! One hand-written Rust module every integration test composes from: real
//! subprocess orchestration of the real `holler` binary (and the `stub-acp`
//! agent it spawns), with **no Cucumber, no YAML step registry, and no mocks of
//! the circuit**. Tests call these helpers directly.
//!
//! Conventions (ADR 0002 / this module's rules):
//! - Bind `127.0.0.1`, never `localhost` (avoids the IPv6/`::1` ambiguity).
//! - Every spawn places the child in its own process group so [`kill_tree`] can
//!   reap the whole tree (the body spawns the agent as a child; killing only the
//!   direct child would orphan the agent).
//! - Nothing sleeps blindly: every "is it ready" check goes through
//!   [`wait_for`] on an *observable* (a log line, a roster row, a file) — never
//!   a fixed `thread::sleep` gate.
//!
//! This module is a *helper*, not a test: it lives at `tests/support/mod.rs`
//! (a directory, so Cargo does not treat it as its own test target) and each
//! integration test declares `mod support;` to pull it in.
//!
//! The file-level `#![allow]` at the top of this file is the one blanket
//! `scripts/lint.sh` permits (it carries the `// #149` issue link): the
//! forward-contract fns (Hub::start, join, …) are not yet exercised until the
//! hub/body stories land, and the harness's spawn/wait paths panic (not
//! a hard process exit) on harness failure so a test failure is a test failure,
//! not a masked 0.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// A per-test scratch directory for holler state.
///
/// Each test gets its own isolated state dir so tests never share hub token
/// files, session files, or locks. Created under the OS temp dir; removed on
/// drop so a panicking test does not leak directories.
pub struct StateDir {
    path: PathBuf,
}

impl StateDir {
    /// Create a fresh, empty state directory under the OS temp dir.
    pub fn new() -> Self {
        // A unique subdir so concurrent test threads (the integration tests
        // run on a thread pool) never collide. `nanos` + pid + thread counter
        // keeps it unique without a cross-platform uuid dependency.
        let unique = format!(
            "holler-test-{}-{:x}-{:x}",
            std::process::id(),
            SystemTimeNanos::now(),
            counter()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("create state dir");
        Self { path }
    }

    /// The root of this state dir (where holler roots `hub/`, `body/`, etc.).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Path to the hub's per-state dir (`<state>/hub`).
    pub fn hub(&self) -> PathBuf {
        self.path.join("hub")
    }

    /// Path to the body's per-state dir (`<state>/body`).
    pub fn body(&self) -> PathBuf {
        self.path.join("body")
    }
}

impl Drop for StateDir {
    fn drop(&mut self) {
        // Best-effort: a panicking test still gets its dir removed. Errors here
        // are non-fatal (the dir is in the temp area; the OS reaps it too).
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Build a `Command` that invokes the built `holler` binary with this state
/// dir active and deterministic logging (quiet + json) so tests can parse
/// stdout/stderr as JSON lines.
pub fn holler_cmd(state: &StateDir) -> Command {
    let mut cmd = Command::new(holler_bin());
    cmd.env("HOLLER_STATE_DIR", state.path())
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json");
    cmd
}

/// The absolute path to the built `holler` binary.
///
/// Cargo sets `CARGO_BIN_EXE_holler` for integration-test targets to the
/// compiled binary. (We cannot use `assert_cmd::cargo_bin` for the long-lived
/// processes here — `Hub`/`Body` need piped stdio to parse the live listener
/// port — so we build a plain `std::process::Command` off the raw path.)
///
/// Read with `env!` (compile time), not `env::var` (runtime): the var is set
/// by cargo per test target, so a missing one is a build/setup error. A missing
/// var must fail the *build*, not let a helper silently `exit(2)` (defect #147).
fn holler_bin() -> &'static str {
    env!("CARGO_BIN_EXE_holler")
}

/// Poll `check` every 50 ms until it returns `Some` or `timeout` elapses.
///
/// This is the module's *only* sanctioned wait: it always terminates (bounded
/// by `timeout`) and returns `None` on timeout, so callers can distinguish
/// "became ready" from "timed out". No `thread::sleep` gates in test code —
/// readiness is always observed, never guessed.
pub fn wait_for<T>(timeout: Duration, mut check: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    loop {
        // Poll first: a check that is already ready should not pay a sleep.
        if let Some(v) = check() {
            return Some(v);
        }
        // Then, only if budget remains, sleep for the *lesser* of the poll
        // interval and the time left. Sleeping a fixed POLL_INTERVAL would let
        // the loop overshoot the deadline by up to one interval (and, on a
        // loaded CI runner, by more, since `thread::sleep` is a *minimum* —
        // the OS may wake us late). Sleeping exactly the remaining budget keeps
        // the whole call bounded by `timeout` plus at most one `check()`'s cost.
        let now = Instant::now();
        if now >= deadline {
            return None;
        }
        let remaining = deadline
            .checked_duration_since(now)
            .unwrap_or(Duration::ZERO);
        std::thread::sleep(remaining.min(POLL_INTERVAL));
    }
}

const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// A running hub subprocess.
///
/// `Hub::start` launches `holler hub serve --listen 127.0.0.1:0` (port 0 → the
/// OS picks a free one), then parses the port out of the hub's first
/// `event="listening"` JSON log line on stderr within 10 s. `port`/`ws_url`
/// expose that bound port.
///
/// Forward contract: the `hub serve` implementation lands in the hub stories;
/// until then this struct's `start` is the thing those stories' tests drive
/// first.
pub struct Hub {
    pub port: u16,
    child: Child,
    /// Set once [`Hub::stop`] has torn the process down. The `Drop` impl
    /// re-runs the same teardown if a test lets the `Hub` fall out of scope
    /// *without* calling `stop` (e.g. the WS first-frame tests just `drop(hub)`,
    /// and any test that panics mid-body). Without this, dropping the `Child`
    /// handle would orphan the hub process (reparented to init) and leak it.
    stopped: bool,
}

/// Tear the hub's whole process tree down: SIGINT first (graceful), then a
/// hard SIGKILL of the tree if it has not exited within `timeout`, and reap so
/// no zombie is left. Idempotent: if the child has already exited (e.g. a
/// prior `stop` or `Drop` already reaped it, or it died on its own) it is a
/// no-op, so [`Hub::stop`] and `impl Drop for Hub` can both call it safely.
fn stop_hub(child: &mut Child, timeout: Duration) {
    // Already exited? Nothing to signal or reap.
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    // SIGINT first (graceful teardown); on non-Unix there is no process-group
    // signal, so go straight to the kill-tree fallback below.
    #[cfg(unix)]
    signal_tree(child);
    // Give it `timeout` to wind down on the signal… (`wait_for` polls until the
    // closure returns `Some`, i.e. until `try_wait` reports an exit.)
    wait_for(timeout, || child.try_wait().ok().flatten());
    // …then hard-kill the whole tree and reap (no-op if it already exited).
    kill_tree(child);
}

impl Drop for Hub {
    fn drop(&mut self) {
        // `stop()` flips `stopped` before tearing down; if it's already true the
        // child was reaped there and there is nothing left to do. Otherwise
        // (a test let the `Hub` drop without calling `stop`, or panicked) we
        // still reap the tree so no hub process is ever orphaned.
        if !self.stopped {
            stop_hub(&mut self.child, Duration::from_secs(5));
        }
    }
}

impl Hub {
    /// Spawn a hub bound to a free loopback port and wait (≤10 s) for it to
    /// report the bound port on stderr.
    pub fn start(state: &StateDir) -> Hub {
        // Bind the base `Command` to a name first (rather than chaining on the
        // `holler_cmd` temporary) so we can keep it alive long enough to call
        // `make_own_process_group` on it — see the note below.
        let mut cmd = holler_cmd(state);
        cmd.args(["hub", "serve", "--listen", "127.0.0.1:0"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        // Give the hub its own process group (ADR 0002) so `signal_tree` /
        // `kill_tree` can signal it by group: `Hub::stop` sends
        // `kill(-pid, SIGINT)` for a graceful stop and falls back to
        // `kill(-pid, SIGKILL)`. Without a group of its own, `-pid` names no
        // group and both calls are no-ops, so `stop` would hang on `wait()`.
        // (`Body::start` already does this; the hub spawn had simply omitted
        // it.)
        make_own_process_group(&mut cmd);
        let mut child = cmd.spawn().expect("spawn `holler hub serve`");

        // The hub emits one JSON object per event on stderr; the
        // listener-ready line is `{"event":"listening","addr":"127.0.0.1:<port>"}`.
        // We drain stderr line-by-line until that event arrives or 10 s pass.
        let port = {
            let stderr = child.stderr.take().expect("hub stderr is piped");
            let reader = std::io::BufReader::new(stderr);
            let mut reader = Some(reader);
            wait_for(Duration::from_secs(10), || {
                let r = reader.as_mut()?;
                read_json_event(r, "listening").map(|v| parse_port(&v))
            })
        }
        .unwrap_or_else(|| {
            let _ = child.kill();
            panic!(
                "hub did not report a listening port within 10s \
                 (`hub serve` is not implemented yet?)"
            );
        });

        Hub {
            port,
            child,
            stopped: false,
        }
    }

    /// The WebSocket URL bodies dial to reach this hub.
    pub fn ws_url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.port)
    }

    /// The hub child's pid (to signal it directly from a test, e.g. SIGINT).
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// A mutable handle to the hub child, for tests that need to
    /// `try_wait` / `kill` it without consuming the `Hub`.
    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Stop the hub gracefully: SIGINT (Ctrl-C) first, then a hard kill of the
    /// process tree if it has not exited within `timeout`.
    ///
    /// Takes `self` by value (consuming the `Hub`). We flip `stopped` *before*
    /// tearing down so that the `Drop` impl that runs when `self` is dropped at
    /// the end of this function is a no-op (the child is already reaped).
    pub fn stop(mut self, timeout: Duration) {
        self.stopped = true;
        stop_hub(&mut self.child, timeout);
        // `self` (and thus `self.child`) is dropped here; `Drop` sees
        // `stopped == true` and does nothing.
    }
}

/// Mint a join token on the (local, state-dir-bound) hub.
///
/// Returns `(token_id, secret)` — the pair a body joins with
/// (`body join --token <id>:<secret>`). The hub persists the token under
/// `<state>/hub`, so no live hub process is required to *mint*.
pub fn mint_token(state: &StateDir, label: &str) -> (String, String) {
    let out = holler_cmd(state)
        .args(["hub", "token", "mint", "--label", label, "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `hub token mint`");
    assert!(
        out.status.success(),
        "hub token mint failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("mint --json is a JSON object");
    let token_id = v["id"]
        .as_str()
        .expect("mint result carries `id`")
        .to_string();
    let secret = v["secret"]
        .as_str()
        .expect("mint result carries `secret`")
        .to_string();
    (token_id, secret)
}

/// Join `state`'s body to the hub at `ws_url` using the minted token, and
/// assert the join succeeded (exit 0).
pub fn join(state: &StateDir, ws_url: &str, token_id: &str, secret: &str) {
    let token = format!("{token_id}:{secret}");
    let out = holler_cmd(state)
        .args(["body", "join", "--server", ws_url, "--token", &token])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `body join`");
    assert!(
        out.status.success(),
        "body join failed (exit {:?}): {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Write `<state>/body/sessions.toml` with one spawn-mode row per entry.
///
/// Each row points the body at the built `stub-acp` agent with the given extra
/// argv, so a session the body spawns runs the deterministic stub (story #130)
/// instead of a real model. Returns the path written.
pub fn write_sessions_toml(
    state: &StateDir,
    sessions: &[(&str, &[&str])],
) -> PathBuf {
    // Compile-time read (defect #147): a missing var fails the build, not a
    // silently-exited test.
    let stub = env!("CARGO_BIN_EXE_stub-acp");

    let mut toml = String::new();
    for (name, extra) in sessions {
        // spawn-mode row: the body launches `stub-acp <extra…>` for this
        // session. `args` is a single TOML string array — one `[a, b, …]` row,
        // NOT one `args = [x]` per element (that would be a duplicate key and
        // fail to parse the moment the body story reads the file).
        //
        // Each arg is JSON-escaped so a path with a quote or backslash still
        // produces valid TOML (JSON string escaping is a valid TOML basic-string
        // escape for the characters TOML cares about: `\"` and `\\`).
        let args = extra
            .iter()
            .map(|a| serde_json::to_string(a).unwrap_or_else(|_| panic!("json-encode {a:?}")))
            .collect::<Vec<_>>()
            .join(", ");
        // Both `command` (a path) and each arg are JSON-escaped and then placed
        // in a TOML basic string: JSON's `\"`/`\\` escaping is a valid subset of
        // TOML basic-string escaping, so the result is always parseable TOML.
        let stub_q = serde_json::to_string(&stub).unwrap();
        toml.push_str(&format!(
            r#"[sessions."{name}"]
mode = "spawn"
command = {stub_q}
args = [{args}]
"#
        ));
    }

    let path = state.body().join("sessions.toml");
    std::fs::create_dir_all(state.body()).expect("create body dir");
    std::fs::write(&path, toml).expect("write sessions.toml");
    path
}

/// A running body subprocess.
pub struct Body {
    child: Child,
}

impl Body {
    /// Spawn a body running the config at `config` (its `sessions.toml`), in
    /// its own process group so [`kill_tree`] can reap the agents it spawns.
    pub fn start(state: &StateDir, config: &Path) -> Body {
        let mut cmd = holler_cmd(state);
        cmd.arg("body")
            .arg("run")
            .arg("--config")
            .arg(config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        make_own_process_group(&mut cmd);
        let child = cmd.spawn().expect("spawn `holler body run`");
        Body { child }
    }

    /// Detach the body from its hub and wait for it to exit; if it is still up
    /// after `timeout`, `kill_tree` it.
    pub fn stop(self, state: &StateDir, timeout: Duration) {
        let mut child = self.child;
        // Graceful: ask the body to detach, then wait, then hard-kill the tree.
        let _ = holler_cmd(state).args(["body", "detach"]).status();
        wait_for(timeout, || child.try_wait().ok().flatten());
        kill_tree(&mut child);
    }
}

/// The hub's current roster as JSON (parsed from `holler roster --json`).
pub fn roster_json(state: &StateDir) -> Value {
    status_json_of(state, &["roster"])
}

/// The hub's status as JSON (parsed from `holler hub status --json`).
pub fn hub_status_json(state: &StateDir) -> Value {
    status_json_of(state, &["hub", "status"])
}

/// A body's status as JSON (parsed from `holler body status --json`).
pub fn body_status_json(state: &StateDir) -> Value {
    status_json_of(state, &["body", "status"])
}

fn status_json_of(state: &StateDir, sub: &[&str]) -> Value {
    let args: Vec<String> = sub
        .iter()
        .map(|s| s.to_string())
        .chain(["--json".to_string()])
        .collect();
    let out = holler_cmd(state)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run status command");
    assert!(
        out.status.success(),
        "`holler {}` failed: {}",
        sub.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("status --json is valid JSON")
}

/// Send a one-shot prompt to `session` on the hub and return the raw `Output`
/// (`say` prints the reply; tests assert on stdout).
pub fn say(state: &StateDir, session: &str, text: &str) -> Output {
    holler_cmd(state)
        .args(["say", session, text])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `say`")
}

/// Interrupt `session`'s in-flight turn and return the raw `Output`.
pub fn interrupt(state: &StateDir, session: &str) -> Output {
    holler_cmd(state)
        .args(["interrupt", session])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `interrupt`")
}

/// Kill `child`'s entire process tree.
///
/// Unix: the child was spawned in its own process group (see
/// [`make_own_process_group`]), so `kill(-pgid, SIGKILL)` reaps the child and
/// every descendant (the spawned agents) in one call.
/// Windows: `taskkill /F /T <pid>` kills the process and its children.
///
/// Idempotent and non-fatal: if the process already exited, this is a no-op.
pub fn kill_tree(child: &mut Child) {
    let pid = child.id() as i32;
    #[cfg(unix)]
    {
        // Negative pid = the whole process group the child leads.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    {
        // On Windows a process group is not the unit of control; `taskkill /F
        // /T` kills the process and its whole descendant tree.
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    // Reap so we do not leave a zombie behind.
    let _ = child.wait();
}

/// Put the command's child in a new process group (pgid == child pid), so
/// [`kill_tree`] can signal the whole tree. Unix-only; a no-op elsewhere.
///
/// Public (not just internal) so the harness's own selftests — and any test
/// that spawns a helper process and wants it reaped as a tree — reuse the exact
/// same spawn-side setup the harness uses.
pub fn make_own_process_group(cmd: &mut Command) {
    // `process_group(0)` is a safe API: it only records that the child should
    // be placed in a new process group (pgid == its own pid) at spawn time.
    // (The actual group placement is done by the kernel at `fork`+`exec`.)
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    {
        let _ = cmd;
    }
}

/// Send SIGINT to a child's process tree (the graceful step before a
/// kill-tree fallback). On Unix this signals the whole group; on Windows a
/// plain `CtrlBreak`-style stop is not portable, so the fallback is the
/// `taskkill /F /T` in [`kill_tree`] (this is a no-op there).
#[cfg(unix)]
fn signal_tree(child: &mut Child) {
    let pid = child.id() as i32;
    unsafe {
        libc::kill(-pid, libc::SIGINT);
    }
}
#[cfg(not(unix))]
fn signal_tree(child: &mut Child) {
    let _ = child;
}

// --- internal helpers -------------------------------------------------------

/// Read a single line off `reader`; if it parses as a JSON object whose
/// `event` field equals `want`, return that object, else `None`.
///
/// Returns `None` (without distinguishing "wrong event" from "EOF") on EOF or
/// a non-matching line. [`Hub::start`] drives this inside [`wait_for`], which
/// re-invokes the closure every 50 ms — so each tick consumes at most one
/// stderr line and the loop keeps draining until the `listening` event (or the
/// 10 s budget) is reached. That tolerates any preamble lines the hub writes
/// before it is listening.
fn read_json_event(reader: &mut impl std::io::BufRead, want: &str) -> Option<Value> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => return None, // EOF
        Ok(_) => {}
        Err(_) => return None, // read error: treat as "not yet"
    }
    let v: Value = serde_json::from_str(&line).ok()?;
    (v.get("event").and_then(|e| e.as_str()) == Some(want)).then_some(v)
}

/// Parse the port out of a `listening` event's `addr` field
/// (`"127.0.0.1:8700"` → `8700`).
fn parse_port(event: &Value) -> u16 {
    let addr = event
        .get("addr")
        .and_then(|a| a.as_str())
        .or_else(|| event.get("address").and_then(|a| a.as_str()))
        .expect("listening event carries an addr");
    let port = addr
        .rsplit(':')
        .next()
        .expect("addr has a port")
        .parse::<u16>()
        .expect("port is numeric");
    port
}

// A monotonic-ish nanosecond clock source for unique temp dir names.
struct SystemTimeNanos;
impl SystemTimeNanos {
    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}

// A per-process counter to make temp dir names unique across threads that
// share a pid and tick within the same nanosecond.
fn counter() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(0);
    C.fetch_add(1, Ordering::Relaxed)
}
