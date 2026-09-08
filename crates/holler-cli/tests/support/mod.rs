#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #138
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

use std::io::{BufRead, Write};
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
/// compiled binary, and this is read at *compile* time (`env!`), so the path
/// is baked into every test binary that pulls in this module. (We cannot use
/// `assert_cmd::cargo_bin` for the long-lived processes here — `Hub`/`Body`
/// need piped stdio to parse the live listener port — so we build a plain
/// `std::process::Command` off the baked path.)
///
/// The `env!` (not a runtime `std::env::var` lookup + `exit(2)`) is
/// deliberate: a test helper that terminates the process on a missing binary
/// path tears down the *whole* test process the
/// moment one binary path is missing, silently hiding every other test in the
/// crate behind a single `exit(2)`. `env!` instead fails the *compile* when the
/// variable is absent, so a missing binary is a loud, per-target error rather
/// than a silent blackout.
pub fn holler_bin() -> &'static str {
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
    // `--json` is a *global* flag (ADR 0003), so it is accepted at the root of
    // the command — *before* the subcommand path — not after `mint`. The
    // ADR 0003 table lists `hub token mint --label LABEL [--ttl 24h] [--json]`
    // but `--json` is global, so it may appear before `mint` (where we place it);
    // putting it after `mint` would only be accepted if `mint` had its *own*
    // `--json`, which it does not.
    let out = holler_cmd(state)
        .args(["--json", "hub", "token", "mint", "--label", label])
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
    // The `--json` document's id field is `token_id` (ADR 0003: the join token
    // is `ID:SECRET` and `ID` is the token id). `id` is the wrong field name —
    // the document has always carried `token_id`.
    let token_id = v["token_id"]
        .as_str()
        .expect("mint --json result carries `token_id`")
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

/// The absolute path to the built `stub-acp` agent binary.
///
/// Compile-time (`env!`) for the same reason as [`holler_bin`]: a missing
/// binary must fail the *compile* of the test target, never `exit(2)` the
/// whole process and hide every other test.
pub fn stub_acp_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stub-acp")
}

/// Write `<state>/body/sessions.toml` with one spawn-mode row per entry.
///
/// Each row points the body at the built `stub-acp` agent with the given extra
/// argv, so a session the body spawns runs the deterministic stub (story #130)
/// instead of a real model. Returns the path written.
pub fn write_sessions_toml(state: &StateDir, sessions: &[(&str, &[&str])]) -> PathBuf {
    // Compile-time read (defect #147): a missing var fails the build, not a
    // silently-exited test.
    let stub = env!("CARGO_BIN_EXE_stub-acp");

    let mut toml = String::new();
    for (name, extra) in sessions {
        // The full argv: the agent binary first, then its extra args. JSON's
        // string escaping is a valid subset of TOML basic-string escaping, so
        // each element is JSON-escaped and the elements are joined into one
        // TOML string array (an inline array — the loader sees `command` as a
        // list, matching how the body execs the agent).
        let argv = std::iter::once(stub)
            .chain(extra.iter().copied())
            .map(|a| serde_json::to_string(a).expect("json-encode argv element"))
            .collect::<Vec<_>>()
            .join(", ");
        toml.push_str(&format!(
            r#"[[session]]
name = {name_q}
harness = "opencode"
command = [{argv}]
"#,
            name_q = serde_json::to_string(name).expect("json-encode name"),
        ));
    }

    let path = state.body().join("sessions.toml");
    std::fs::create_dir_all(state.body()).expect("create body dir");
    std::fs::write(&path, toml).expect("write sessions.toml");
    path
}

/// The built `stub-acp` agent, driven over a real stdin/stdout pipe — the one
/// ACP client-side driver the test suite shares.
///
/// Both the stub's own selftests (`stub_acp_test`) and the ACP-driver story
/// talk to `stub-acp` the same way: spawn it, read its newline-delimited JSON
/// responses off stdout, write requests (JSON-RPC objects) to stdin, and
/// terminate it on drop. The stub tests used to re-implement this driver by
/// hand; centralising it here means the driver the story's tests rely on is the
/// *exact* one the stub is verified against (one implementation, not two).
///
/// The path is the compile-time `CARGO_BIN_EXE_stub-acp` (see [`stub_acp_bin`])
/// — never a runtime env read, so a missing binary fails the compile, not the
/// whole test process.
///
/// **Teardown closes stdin (EOF) first, then reaps.** `ChildStdin` has no
/// `close()`, so the only way to close the pipe is to drop the handle — which
/// is what [`Stub::drop`] does *before* it reaps. Closing stdin first (rather
/// than killing the child first) matters: a test may drop its `Stub` while a
/// turn is still in flight (the `ask-permission` turn parks on its permission
/// gate; a plain prompt may still be streaming its chunks), and the test may
/// still be reading that turn's remaining output. If we killed the child first,
/// its stdout pipe would truncate and the in-flight `read_response` would block
/// forever on a dead pipe. An EOF, by contrast, lets the stub finish the turn
/// and write the remaining output before it exits. (The stub is written so a
/// *bare* EOF never misreports an already-resolved turn as `cancelled` — only a
/// real `session/cancel` does — so closing stdin on drop is safe either way.)
/// After the EOF, `wait` reaps; a stub that is somehow still alive is `kill`-ed
/// as a fallback so a panicking test never leaks a child process.
/// The fixed JSON-RPC request lines the stub contract tests drive with. Each is
/// a pre-serialized JSON object (no trailing newline — [`Stub::send`] appends
/// it). The ids are the stub's own: initialize=1, session/new=2, prompt=3,
/// cancel=4, the unknown method=5, and the permission request the stub itself
/// raises is id 10 (see [`PERMISSION_REQUEST_ID`]). Sharing these here —
/// rather than re-declaring them in each test file — means the driver and the
/// tests that use it agree on the wire by construction.
/// The fixed id the stub assigns to the `session/request_permission` request it
/// raises under `--ask-permission`; the client answers it with that same id.
pub const PERMISSION_REQUEST_ID: i64 = 10;
pub const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":2,"info":{"name":"tester","version":"0"}}}"#;
pub const SESSION_NEW: &str =
    r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/"}}"#;
pub const PROMPT: &str = r#"{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"stub","prompt":[{"type":"text","text":"hi"}]}}"#;
pub const CANCEL: &str =
    r#"{"jsonrpc":"2.0","id":4,"method":"session/cancel","params":{"sessionId":"stub"}}"#;
pub const UNKNOWN: &str = r#"{"jsonrpc":"2.0","id":5,"method":"bogus/method","params":{}}"#;

pub struct Stub {
    child: Child,
    /// The stub's stdin pipe handle. `Option` so `Drop` can `take()` it (moving
    /// it out of the field, leaving `None`) and drop it to close the pipe — a
    /// `ChildStdin` cannot be moved out of `&mut self` any other way, and has no
    /// `close()`. The `Child`'s own stdin slot is emptied in `start` (see there),
    /// so this is the sole owner of the write end and the pipe is closed exactly
    /// once.
    stdin: Option<std::process::ChildStdin>,
    reader: std::io::BufReader<std::process::ChildStdout>,
}

impl Stub {
    /// Spawn `stub-acp` with `extra` args (e.g. `--slow`, `--ask-permission`)
    /// and piped stdio.
    pub fn start(extra: &[&str]) -> Stub {
        let mut child = Command::new(stub_acp_bin())
            .args(extra)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // stderr is nulled (not piped): the stub writes only the JSON-RPC
            // stream to stdout, and a piped stderr nobody reads could fill its
            // OS buffer and block the stub if it ever logged heavily.
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn stub-acp");
        // `Stub` becomes the sole owner of the stdin pipe: take the handle out
        // of the `Child` so the child's own drop cannot close the pipe (the
        // write end is closed exactly once — by `Stub::drop`).
        let stdin = child.stdin.take().expect("stub-acp stdin piped");
        let reader = std::io::BufReader::new(child.stdout.take().expect("stub-acp stdout piped"));
        Stub {
            child,
            stdin: Some(stdin),
            reader,
        }
    }

    /// Write one raw JSON request line to the stub's stdin and flush it.
    ///
    /// The line is written as a pre-serialized JSON object (the test's module
    /// constants such as [`PROMPT`] are already serialized). A trailing newline
    /// terminates the JSON-RPC message; the stub reads stdin line-delimited.
    /// This is the driver's only write primitive — every higher-level helper
    /// ([`Stub::handshake`], the tests' `send(CANCEL)`, …) is built on it.
    pub fn send(&mut self, line: &str) {
        self.stdin
            .as_mut()
            .expect("stub-acp stdin live")
            .write_all(line.as_bytes())
            .expect("write request line");
        self.stdin
            .as_mut()
            .expect("stub-acp stdin live")
            .write_all(b"\n")
            .expect("write newline");
        self.stdin
            .as_mut()
            .expect("stub-acp stdin live")
            .flush()
            .expect("flush stdin");
    }

    /// Read the next newline-delimited JSON line off the stub's stdout and
    /// parse it into a `Value`. Notifications and responses are indistinguishable
    /// here — the caller decides which it expects; use [`Stub::read_response`] to
    /// skip notifications and wait for a specific id.
    pub fn read_line(&mut self) -> Value {
        let mut buf = String::new();
        self.reader
            .read_line(&mut buf)
            .expect("read stdout line (EOF while a message was expected)");
        serde_json::from_str(&buf).expect("valid JSON line from stub")
    }

    /// Read messages until one with the given JSON-RPC id (a response) arrives.
    /// Notifications carry no id and are skipped. Returns the parsed response.
    /// `id` is the numeric JSON-RPC id as a string (e.g. "3"); the stub sends
    /// ids as JSON numbers, so we compare via `as_i64`.
    pub fn read_response(&mut self, id: &str) -> Value {
        let want: i64 = id.parse().expect("test ids are integers");
        loop {
            let v = self.read_line();
            match v.get("id").and_then(|i| i.as_i64()) {
                Some(i) if i == want => return v,
                Some(_) => panic!("unexpected response id: {v}"),
                None => continue, // a streamed session/update notification
            }
        }
    }

    /// Handshake: initialize + session/new. Asserts the negotiated protocol
    /// version is 2 and the session id is "stub".
    pub fn handshake(&mut self) {
        self.send(INITIALIZE);
        let init = self.read_response("1");
        assert_eq!(
            init["result"]["protocolVersion"].as_u64(),
            Some(2),
            "initialize must negotiate protocolVersion 2: {init}"
        );
        assert!(
            init["result"].get("agentCapabilities").is_some(),
            "initialize result must carry agentCapabilities: {init}"
        );
        self.send(SESSION_NEW);
        assert_eq!(self.read_response("2")["result"]["sessionId"], "stub");
    }

    /// Close stdin (EOF) and wait for the process to exit, returning the exit
    /// code. Taking the `ChildStdin` handle out and dropping it closes the write
    /// end of the pipe, so the stub's stdin reader sees EOF. Takes `self` by
    /// value because the `!Clone` stdin handle must be moved out and dropped;
    /// after `close` the `Stub` must not be used again.
    pub fn close(mut self) -> Option<i32> {
        // Drop the stdin handle (closes the write end → EOF to the stub), then
        // reap. `Option::take` empties the field so the (suppressed) `Drop` impl
        // does not close the pipe a second time.
        drop(self.stdin.take());
        self.child.wait().ok().map(|s| s.code().unwrap_or(-1))
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        // Order matters: close the stub's stdin FIRST, then reap.
        //
        // Closing the stdin pipe (by dropping the `ChildStdin` handle — it has
        // no `close()`) sends the stub an EOF. The stub is written so a *bare*
        // EOF never misreports an already-resolved turn as `cancelled`; for an
        // in-flight turn it lets the turn finish (emitting its remaining
        // chunks and terminal response) before the process exits. Closing stdin
        // before killing is essential: if we killed the child first, its stdout
        // pipe would be truncated and a test that is still mid-`read_response`
        // for the turn's remaining output would block forever on a dead pipe.
        //
        // `take()` moves the handle out of the field (leaving `None`, so the
        // `Child`'s own drop cannot close the pipe a second time) and dropping
        // it closes the pipe.
        if let Some(stdin) = self.stdin.take() {
            drop(stdin); // → EOF to the stub; the stub exits once it drains.
        }
        // Reap: the EOF above ends the stub (it drains and exits on EOF), so
        // `wait` returns its status. Only if the stub is somehow still alive
        // (e.g. parked on a permission gate and not reached by the EOF for some
        // reason) does `wait` fail, in which case `kill()` is the fallback so a
        // panicking test never leaks a child.
        if self.child.wait().is_err() {
            let _ = self.child.kill();
        }
    }
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
