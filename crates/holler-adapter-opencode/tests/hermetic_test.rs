#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #642
//! The OpenCode adapter's server side with no OpenCode and no tmux (#642a, AC 1-8, 11, 11d,
//! 11e). Every call goes to the stub in `support/stub.rs` on an OS-assigned loopback port, or
//! to a held port that never had a listener (refused). `serve` runs only `false` and `sh` on a script
//! in a scratch directory, so the file runs in CI on Linux and macOS.

#[path = "support/stub.rs"]
mod stub;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use holler_adapter_opencode::http::{self, HttpError};
use holler_adapter_opencode::{
    OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig, TmuxSocket,
};
use holler_pane::{HarnessPort, PaneError, PaneId, PaneName};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::harness::HarnessOp;
use serde_json::Value;
use stub::{closed_port, Framing, Stub};

/// A session id of OpenCode's shape: `ses_` and 26 of `[0-9A-Za-z]`.
const ID: &str = "ses_0123456789abcdefABCDEFghij";
/// What a bound may overrun by on a loaded CI runner.
const SLACK: Duration = Duration::from_millis(300);
const HEALTHY: &str = r#"{"healthy":true,"version":"1.18.35"}"#;

/// Short bounds so the frozen cases finish quickly.
fn quick() -> Timeouts {
    Timeouts {
        call: Duration::from_secs(2),
        request: Duration::from_millis(800),
        health: Duration::from_millis(300),
        boot_try: Duration::from_millis(200),
        settle: Duration::from_millis(400),
    }
}

/// A scratch directory of its own per test, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    /// A scratch dir holding a `serve` script, so that `sh` as the OpenCode binary runs
    /// `sh serve --port ...`: it leaves a `spawned` marker in its working directory and
    /// exits 42.
    fn with_serve_script() -> Self {
        Self::with_script("touch spawned\nexit 42\n")
    }

    /// A scratch dir whose `serve` script is `body`.
    fn with_script(body: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("hlr642-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("serve"), body).unwrap();
        Scratch(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn spawned(&self) -> bool {
        self.0.join("spawned").exists()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A config whose tmux half is never reached in 642a: a binary and socket that do not exist,
/// and a `tui_session` that knows no pane.
fn config(opencode_bin: impl Into<PathBuf>, workdir: &Path, timeouts: Timeouts) -> OpenCodeConfig {
    let workdir = workdir.to_path_buf();
    OpenCodeConfig {
        opencode_bin: opencode_bin.into(),
        serve_args: Vec::new(),
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig {
            tmux_bin: PathBuf::from("/nonexistent/hlr642-tmux"),
            socket: TmuxSocket::Path(PathBuf::from("/nonexistent/hlr642-tmux.sock")),
        },
        workdir: Arc::new(move |_: &PaneName| Ok(workdir.clone())),
        tui_session: Arc::new(|pane: &PaneId| {
            Err(PaneError::PaneNotFound {
                what: pane.as_str().to_string(),
            })
        }),
        timeouts,
    }
}

/// An adapter for the calls that never spawn.
fn harness(timeouts: Timeouts) -> OpenCodeHarness {
    OpenCodeHarness::new(config(
        "/nonexistent/hlr642-opencode",
        &std::env::temp_dir(),
        timeouts,
    ))
}

fn pane_name() -> PaneName {
    PaneName::parse("demo-c1r1").unwrap()
}

fn unavailable<T: std::fmt::Debug>(result: Result<T, PaneError>, what: &str) -> String {
    match result {
        Err(PaneError::Unavailable { what: message }) => message,
        other => panic!("{what}: expected unavailable, got {other:?}"),
    }
}

fn timeout_op<T: std::fmt::Debug>(result: Result<T, PaneError>, what: &str) -> String {
    match result {
        Err(PaneError::Timeout { op }) => op,
        other => panic!("{what}: expected timeout, got {other:?}"),
    }
}

/// ADR-0021 section 9: one line, naming the route and the status, short.
fn assert_one_line_naming(message: &str, route: &str, what: &str) {
    assert!(
        !message.contains(['\n', '\r']),
        "{what}: the message is one line: {message:?}"
    );
    assert!(
        message.len() <= 200,
        "{what}: at most 200 bytes, got {}",
        message.len()
    );
    assert!(
        message.contains(route),
        "{what}: names the route {route}: {message:?}"
    );
    assert!(
        message.contains("200"),
        "{what}: names the status 200: {message:?}"
    );
}

fn assert_within(start: Instant, bound: Duration, what: &str) {
    let took = start.elapsed();
    assert!(took < bound, "{what}: took {took:?}, bound {bound:?}");
}

fn no_abort_sent(stub: &Stub) -> bool {
    !stub.lines().iter().any(|line| line.contains("/abort"))
}

/// `GET /session/<ID>` answers that session.
fn known_session(stub: &Stub) {
    stub.json(
        "GET",
        &format!("/session/{ID}"),
        200,
        &format!(r#"{{"id":"{ID}","title":"{ID}"}}"#),
    );
}

// ---- AC 1: http::request ----

#[test]
fn ac1_content_length_and_chunked_replies_read_to_the_same_bytes() {
    let stub = Stub::start();
    stub.set(
        "GET",
        "/sized",
        200,
        "application/json",
        HEALTHY,
        Framing::ContentLength,
    );
    stub.set(
        "GET",
        "/chunked",
        200,
        "application/json",
        HEALTHY,
        Framing::Chunked,
    );
    let t = Duration::from_secs(2);
    let sized = http::request(stub.port, "GET", "/sized", None, t).unwrap();
    let chunked = http::request(stub.port, "GET", "/chunked", None, t).unwrap();
    assert_eq!(sized.status, 200);
    assert_eq!(sized.body, HEALTHY.as_bytes(), "the Content-Length body");
    assert_eq!(
        chunked, sized,
        "a chunked reply reads to the same status and bytes"
    );
}

#[test]
fn ac1_a_closed_port_is_refused_within_a_second() {
    let port = closed_port();
    let start = Instant::now();
    let result = http::request(port, "GET", "/global/health", None, Duration::from_secs(5));
    assert_eq!(result, Err(HttpError::Refused));
    assert_within(start, Duration::from_secs(1), "a refused connect");
}

#[test]
fn ac1_a_frozen_server_times_out_within_the_timeout() {
    let stub = Stub::start();
    stub.freeze();
    let timeout = Duration::from_millis(400);
    let start = Instant::now();
    let result = http::request(stub.port, "GET", "/global/health", None, timeout);
    assert_eq!(result, Err(HttpError::TimedOut));
    assert_within(start, timeout + SLACK, "a frozen server");
}

#[test]
fn ac1_a_reply_that_is_not_http_is_garbled() {
    let stub = Stub::start();
    stub.raw(b"SSH-2.0-OpenSSH_9.6\r\nthis is not http\r\n\r\n");
    let result = http::request(
        stub.port,
        "GET",
        "/global/health",
        None,
        Duration::from_secs(2),
    );
    assert!(
        matches!(result, Err(HttpError::Garbled(_))),
        "expected Garbled, got {result:?}"
    );
}

/// A `200` with no `Content-Length` and no `Transfer-Encoding`, whose body runs to the close.
fn unframed(body_len: usize) -> Vec<u8> {
    let mut bytes = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
    bytes.resize(bytes.len() + body_len, b'x');
    bytes
}

/// The 64 MiB reply bound holds on the read-to-the-close path (outside diff gate r1, B-1): a
/// body one byte past it is `Garbled`, and one just under it is read whole, so the bound is
/// neither missing nor set lower.
#[test]
fn ac1_an_unframed_reply_past_64_mib_is_garbled_and_one_under_it_is_read() {
    const MIB64: usize = 64 << 20;
    let t = Duration::from_secs(10);
    let over = Stub::start();
    over.raw(&unframed(MIB64 + 1));
    let result = http::request(over.port, "GET", "/session", None, t);
    assert!(
        matches!(result, Err(HttpError::Garbled(_))),
        "a body of 64 MiB + 1 to the close: expected Garbled, got {:?}",
        result.map(|reply| (reply.status, reply.body.len()))
    );
    let under = Stub::start();
    under.raw(&unframed(MIB64 - 1024));
    let reply = http::request(under.port, "GET", "/session", None, t).unwrap();
    assert_eq!((reply.status, reply.body.len()), (200, MIB64 - 1024));
}

/// A chunk extension is ignored and the trailer after the `0` chunk is discarded (RFC 9112
/// section 7.1; outside diff gate r1, NV-2).
#[test]
fn ac1_a_chunked_reply_with_an_extension_and_a_trailer_reads_its_body() {
    let stub = Stub::start();
    stub.raw(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n\
          12;name=value\r\n{\"healthy\":true,\"v\r\n12\r\nersion\":\"1.18.35\"}\r\n\
          0\r\nX-Trailer: discarded\r\n\r\n",
    );
    let reply = http::request(stub.port, "GET", "/x", None, Duration::from_secs(2)).unwrap();
    assert_eq!(reply.status, 200);
    assert_eq!(
        String::from_utf8_lossy(&reply.body),
        HEALTHY,
        "the chunk data only"
    );
}

// ---- AC 2: health ----

#[test]
fn ac2_health_is_true_for_the_healthy_json() {
    let stub = Stub::start();
    stub.json("GET", "/global/health", 200, HEALTHY);
    assert_eq!(harness(quick()).health(stub.port), Ok(true));
}

#[test]
fn ac2_health_is_false_for_unbound_frozen_html_unhealthy_and_500() {
    let h = harness(quick());
    assert_eq!(h.health(closed_port()), Ok(false), "an unbound port");

    let frozen = Stub::start();
    frozen.freeze();
    let start = Instant::now();
    assert_eq!(h.health(frozen.port), Ok(false), "a frozen server");
    assert_within(start, quick().health + SLACK, "health of a frozen server");

    let html = Stub::start();
    html.html("GET", "/global/health");
    assert_eq!(
        h.health(html.port),
        Ok(false),
        "a 200 of the web app's HTML"
    );

    let unhealthy = Stub::start();
    unhealthy.json("GET", "/global/health", 200, r#"{"healthy":false}"#);
    assert_eq!(
        h.health(unhealthy.port),
        Ok(false),
        "a 200 saying not healthy"
    );

    let failing = Stub::start();
    failing.json("GET", "/global/health", 500, HEALTHY);
    assert_eq!(h.health(failing.port), Ok(false), "a 500");
}

// ---- AC 3: create_session ----

fn creatable(stub: &Stub, patch_status: u16, patch_title: &str) {
    let created = format!(
        r#"{{"id":"{ID}","directory":"/p","title":"New session - 2026-10-09T00:00:00.000Z"}}"#
    );
    stub.json("POST", "/session", 200, &created);
    let patched = format!(r#"{{"id":"{ID}","directory":"/p","title":"{patch_title}"}}"#);
    stub.json("PATCH", &format!("/session/{ID}"), patch_status, &patched);
}

#[test]
fn ac3_create_session_titles_the_session_with_its_id() {
    let stub = Stub::start();
    creatable(&stub, 200, ID);
    assert_eq!(
        harness(quick()).create_session(stub.port),
        Ok(ID.to_string())
    );
    let seen = stub.seen();
    let lines: Vec<&str> = seen.iter().map(|s| s.line.as_str()).collect();
    assert_eq!(
        lines,
        [
            "POST /session HTTP/1.1".to_string(),
            format!("PATCH /session/{ID} HTTP/1.1")
        ]
    );
    let patch: Value = serde_json::from_slice(&seen[1].body).expect("the PATCH body is JSON");
    assert_eq!(patch["title"], ID, "the session is titled with its own id");
}

#[test]
fn ac3_a_failed_title_patch_deletes_the_session_and_is_unavailable() {
    let stub = Stub::start();
    creatable(&stub, 500, ID);
    unavailable(
        harness(quick()).create_session(stub.port),
        "a PATCH that answers 500",
    );
    let delete = format!("DELETE /session/{ID} HTTP/1.1");
    assert!(
        stub.lines().contains(&delete),
        "the untitled session is deleted: {:?}",
        stub.lines()
    );
}

#[test]
fn ac3_a_patch_reply_with_another_title_is_unavailable() {
    let stub = Stub::start();
    creatable(&stub, 200, "New session - 2026-10-09T00:00:00.000Z");
    unavailable(
        harness(quick()).create_session(stub.port),
        "a PATCH reply whose title is not the id",
    );
}

// ---- AC 4 and 11e: list_sessions ----

#[test]
fn ac4_list_sessions_returns_the_listed_ids_in_order() {
    let stub = Stub::start();
    stub.json(
        "GET",
        "/session",
        200,
        r#"[{"id":"ses_b","title":"b"},{"id":"ses_a","title":"a"}]"#,
    );
    assert_eq!(
        harness(quick()).list_sessions(stub.port),
        Ok(vec!["ses_b".to_string(), "ses_a".to_string()])
    );
}

#[test]
fn ac11e_child_sessions_are_left_out_of_the_list() {
    let stub = Stub::start();
    let body =
        r#"[{"id":"ses_a"},{"id":"ses_b","parentID":"ses_a"},{"id":"ses_c","parentID":null}]"#;
    stub.json("GET", "/session", 200, body);
    assert_eq!(
        harness(quick()).list_sessions(stub.port),
        Ok(vec!["ses_a".to_string(), "ses_c".to_string()])
    );
}

// ---- AC 5: abort ----

#[test]
fn ac5_abort_of_an_unknown_id_is_session_not_found_and_sends_no_abort() {
    // 404 is what OpenCode answers for an unknown id; a 400 (a malformed id) reads the same.
    for status in [404, 400] {
        let stub = Stub::start();
        stub.json("GET", "/session/ses_missing", status, stub::NOT_FOUND);
        stub.json("POST", "/session/ses_missing/abort", 200, "true");
        let result = harness(quick()).abort(stub.port, "ses_missing");
        assert_eq!(
            result,
            Err(PaneError::SessionNotFound {
                what: "ses_missing".to_string()
            }),
            "the existence check answered {status}"
        );
        assert!(
            no_abort_sent(&stub),
            "no abort after a {status}: {:?}",
            stub.lines()
        );
    }
}

#[test]
fn ac5_abort_of_an_idle_known_session_is_ok() {
    let stub = Stub::start();
    known_session(&stub);
    stub.json("POST", &format!("/session/{ID}/abort"), 200, "true");
    stub.json("GET", "/session/status", 200, "{}");
    assert_eq!(harness(quick()).abort(stub.port, ID), Ok(()));
    let abort = format!("POST /session/{ID}/abort HTTP/1.1");
    assert!(
        stub.lines().contains(&abort),
        "the abort was sent: {:?}",
        stub.lines()
    );
}

#[test]
fn ac5_abort_of_a_session_that_stays_busy_times_out() {
    let stub = Stub::start();
    known_session(&stub);
    stub.json("POST", &format!("/session/{ID}/abort"), 200, "true");
    stub.json(
        "GET",
        "/session/status",
        200,
        &format!(r#"{{"{ID}":{{"type":"busy"}}}}"#),
    );
    let start = Instant::now();
    let op = timeout_op(
        harness(quick()).abort(stub.port, ID),
        "a session that stays busy",
    );
    assert_eq!(op, HarnessOp::Abort.as_str());
    assert_within(start, quick().call + SLACK, "abort of a busy session");
}

// ---- AC 6 and 7: unreachable and frozen servers ----

#[test]
fn ac6_server_calls_to_an_unbound_port_are_unavailable() {
    let h = harness(quick());
    let port = closed_port();
    let messages = [
        unavailable(h.create_session(port), "create_session"),
        unavailable(h.list_sessions(port), "list_sessions"),
        unavailable(h.abort(port, ID), "abort"),
    ];
    for message in messages {
        assert!(
            message.contains(&port.to_string()),
            "names the port {port}: {message:?}"
        );
    }
}

#[test]
fn ac7_the_call_bound_caps_the_request_timeout() {
    let stub = Stub::start();
    stub.freeze();
    let timeouts = Timeouts {
        call: Duration::from_secs(1),
        ..Timeouts::default()
    };
    let start = Instant::now();
    let op = timeout_op(
        harness(timeouts).create_session(stub.port),
        "create_session on a frozen server",
    );
    assert_eq!(op, HarnessOp::CreateSession.as_str());
    assert_within(start, Duration::from_millis(1500), "the 1 s call bound");
}

// ---- AC 8: serve ----

/// `config`, with a `workdir` that counts its calls: `serve` resolves the project directory
/// only once the port is refused (Behaviour, `serve`; outside diff gate r1, NV-4).
fn counting_workdir(mut config: OpenCodeConfig) -> (OpenCodeConfig, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let (inner, counted) = (config.workdir, Arc::clone(&calls));
    config.workdir = Arc::new(move |name: &PaneName| {
        counted.fetch_add(1, Ordering::SeqCst);
        inner(name)
    });
    (config, calls)
}

#[test]
fn ac8_serve_refuses_a_port_that_already_answers_healthy() {
    let stub = Stub::start();
    stub.json("GET", "/global/health", 200, HEALTHY);
    let scratch = Scratch::with_serve_script();
    let (config, resolved) = counting_workdir(config("sh", scratch.path(), quick()));
    let h = OpenCodeHarness::new(config);
    let message = unavailable(h.serve(&pane_name(), stub.port), "serve on a healthy port");
    assert!(
        message.contains(&stub.port.to_string()),
        "names the port: {message:?}"
    );
    assert!(!scratch.spawned(), "nothing is spawned for a port in use");
    assert_eq!(
        resolved.load(Ordering::SeqCst),
        0,
        "workdir is not resolved"
    );
    let health = "GET /global/health HTTP/1.1";
    assert!(
        stub.lines().iter().all(|l| l == health),
        "only health was asked: {:?}",
        stub.lines()
    );
}

#[test]
fn ac8_serve_of_a_missing_binary_names_it() {
    let scratch = Scratch::with_serve_script();
    let missing = scratch.path().join("no-such-opencode");
    let h = OpenCodeHarness::new(config(&missing, scratch.path(), quick()));
    let message = unavailable(
        h.serve(&pane_name(), closed_port()),
        "serve of a missing binary",
    );
    let shown = missing.display().to_string();
    assert!(message.contains(&shown), "names {shown}: {message:?}");
}

#[test]
fn ac8_serve_on_a_frozen_port_times_out_and_spawns_nothing() {
    let stub = Stub::start();
    stub.freeze();
    let scratch = Scratch::with_serve_script();
    let (config, resolved) = counting_workdir(config("sh", scratch.path(), quick()));
    let h = OpenCodeHarness::new(config);
    let start = Instant::now();
    let op = timeout_op(h.serve(&pane_name(), stub.port), "serve on a frozen port");
    assert_eq!(op, HarnessOp::Serve.as_str());
    assert_within(start, quick().call + SLACK, "serve on a frozen port");
    assert!(
        !scratch.spawned(),
        "nothing is spawned while a frozen server holds the port"
    );
    assert_eq!(
        resolved.load(Ordering::SeqCst),
        0,
        "workdir is not resolved"
    );
}

#[test]
fn ac8_serve_of_a_program_that_exits_at_once_is_unavailable_with_its_status() {
    let timeouts = Timeouts {
        call: Duration::from_secs(10),
        ..quick()
    };
    let scratch = Scratch::with_serve_script();

    // `false` ignores the `serve` arguments and exits 1.
    let start = Instant::now();
    let h = OpenCodeHarness::new(config("false", scratch.path(), timeouts));
    unavailable(h.serve(&pane_name(), closed_port()), "serve of `false`");
    assert_within(
        start,
        Duration::from_secs(3),
        "well before the 10 s call bound",
    );

    // `sh serve --port P ...` runs the scratch script in the project directory: exit 42.
    let port = closed_port();
    let start = Instant::now();
    let h = OpenCodeHarness::new(config("sh", scratch.path(), timeouts));
    let message = unavailable(
        h.serve(&pane_name(), port),
        "serve of a script that exits 42",
    );
    assert_within(
        start,
        Duration::from_secs(3),
        "well before the 10 s call bound",
    );
    assert!(
        scratch.spawned(),
        "the server ran in the pane's project directory"
    );
    // 42 as a number of its own, so a port, a pid or the `hlr642-...` scratch path that holds
    // the digits cannot satisfy it.
    assert!(
        message
            .split(|c: char| !c.is_ascii_digit())
            .any(|n| n == "42"),
        "holds the exit status 42: {message:?}"
    );
}

/// Whether `pid` is still a process (`kill -0`, which signals nothing).
fn alive(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Behaviour, `serve`: a server that never turns healthy is killed with its whole process
/// group when the deadline passes, so a failed `serve` leaves no process behind.
#[test]
fn serve_kills_its_process_group_when_the_deadline_passes() {
    let timeouts = Timeouts {
        call: Duration::from_secs(1),
        ..quick()
    };
    // Never binds the port; records its own pid and a background child's (same group).
    let scratch = Scratch::with_script("echo $$ > pid\nsleep 30 &\necho $! > child\nwait\n");
    let h = OpenCodeHarness::new(config("sh", scratch.path(), timeouts));
    let start = Instant::now();
    let op = timeout_op(
        h.serve(&pane_name(), closed_port()),
        "serve of a server that never answers",
    );
    assert_eq!(op, HarnessOp::Serve.as_str());
    assert_within(
        start,
        timeouts.call + SLACK,
        "serve of a server that never answers",
    );
    for file in ["pid", "child"] {
        let pid = std::fs::read_to_string(scratch.path().join(file))
            .unwrap_or_else(|e| panic!("the script recorded its {file}: {e}"));
        let pid = pid.trim();
        // A killed orphan is reaped by init shortly after the kill, not at once.
        let gone_by = Instant::now() + Duration::from_secs(2);
        while alive(pid) && Instant::now() < gone_by {
            std::thread::sleep(Duration::from_millis(20));
        }
        if alive(pid) {
            let _ = std::process::Command::new("kill")
                .args(["-KILL", pid])
                .status();
            panic!("the {file} process {pid} outlived serve's timeout");
        }
    }
}

// ---- AC 11: the adapter's shape ----

#[test]
fn ac11_the_adapter_is_send_sync_and_static() {
    // A compile-time pin: this file does not build if it is not.
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<OpenCodeHarness>();
}

#[test]
fn ac11_default_timeouts_are_10s_5s_2s_500ms_5s() {
    let expected = Timeouts {
        call: Duration::from_secs(10),
        request: Duration::from_secs(5),
        health: Duration::from_secs(2),
        boot_try: Duration::from_millis(500),
        settle: Duration::from_secs(5),
    };
    assert_eq!(Timeouts::default(), expected);
}

// ---- AC 11d: a 200 is not enough ----

#[test]
fn ac11d_an_html_200_for_the_session_is_unavailable_and_sends_no_abort() {
    let stub = Stub::start();
    stub.html("GET", &format!("/session/{ID}"));
    stub.json("POST", &format!("/session/{ID}/abort"), 200, "true");
    let message = unavailable(
        harness(quick()).abort(stub.port, ID),
        "an HTML session reply",
    );
    assert_one_line_naming(&message, "/session/", "an HTML session reply");
    assert!(
        no_abort_sent(&stub),
        "no abort after an HTML reply: {:?}",
        stub.lines()
    );
}

#[test]
fn ac11d_a_session_reply_for_another_id_is_unavailable_and_sends_no_abort() {
    let stub = Stub::start();
    stub.json(
        "GET",
        &format!("/session/{ID}"),
        200,
        r#"{"id":"ses_someoneelse","title":"x"}"#,
    );
    stub.json("POST", &format!("/session/{ID}/abort"), 200, "true");
    let message = unavailable(
        harness(quick()).abort(stub.port, ID),
        "a reply for another session",
    );
    assert_one_line_naming(&message, "/session/", "a reply for another session");
    assert!(
        no_abort_sent(&stub),
        "no abort after a mismatched reply: {:?}",
        stub.lines()
    );
}

#[test]
fn ac11d_an_html_200_for_the_abort_is_unavailable() {
    let stub = Stub::start();
    known_session(&stub);
    stub.html("POST", &format!("/session/{ID}/abort"));
    stub.json("GET", "/session/status", 200, "{}");
    let message = unavailable(harness(quick()).abort(stub.port, ID), "an HTML abort reply");
    assert_one_line_naming(&message, "/abort", "an HTML abort reply");
}

#[test]
fn ac11d_an_html_200_for_the_list_is_unavailable() {
    let stub = Stub::start();
    stub.html("GET", "/session");
    let message = unavailable(
        harness(quick()).list_sessions(stub.port),
        "an HTML list reply",
    );
    assert_one_line_naming(&message, "/session", "an HTML list reply");
}

#[test]
fn ac11d_a_session_id_is_percent_encoded_in_the_request_line() {
    let stub = Stub::start();
    let result = harness(quick()).abort(stub.port, "ses x/?");
    assert_eq!(
        result,
        Err(PaneError::SessionNotFound {
            what: "ses x/?".to_string()
        })
    );
    let lines = stub.lines();
    assert_eq!(
        lines.first().map(String::as_str),
        Some("GET /session/ses%20x%2F%3F HTTP/1.1"),
        "{lines:?}"
    );
}
