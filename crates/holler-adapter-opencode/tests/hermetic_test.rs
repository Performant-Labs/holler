#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #642
//! The OpenCode adapter's server side with no OpenCode and no tmux (#642a, AC 1-8, 11, 11d,
//! 11e). Every call goes to the stub in `support/stub.rs` on an OS-assigned loopback port, or
//! to a refused port from `stub::on_refused_port`, which retries when another bind took the
//! port (#642b, AC 31). #802: `serve_command` only builds the server's argv (it starts
//! nothing; `HostPort::run` starts it in the pane's tmux session), so no test here runs a
//! process but the re-executed copy of this binary in the credential case; the file runs in
//! CI on Linux and macOS.

#[path = "support/stub.rs"]
mod stub;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use holler_adapter_opencode::http::{self, HttpError};
use holler_adapter_opencode::{
    OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig, TmuxSocket,
};
use holler_pane::{AgentKey, Argv, HarnessPort, PaneError, PaneId, PaneName, ServeSpec};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::harness::HarnessOp;
use serde_json::Value;
use stub::{on_refused_port, Framing, Stub};

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
        settle: Duration::from_millis(400),
    }
}

/// A scratch directory of its own per test, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    /// A scratch dir holding an executable `opencode` script that leaves a `spawned` marker
    /// next to itself: a stand-in binary that shows whether anything ran it.
    fn with_fake_opencode() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("hlr802-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("opencode");
        let body = format!(
            "#!/bin/sh\ntouch '{}'\nexit 42\n",
            dir.join("spawned").display()
        );
        std::fs::write(&script, body).unwrap();
        let mut mode = std::fs::metadata(&script).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
        std::fs::set_permissions(&script, mode).unwrap();
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
fn config(opencode_bin: impl Into<PathBuf>, timeouts: Timeouts) -> OpenCodeConfig {
    OpenCodeConfig {
        opencode_bin: opencode_bin.into(),
        serve_args: Vec::new(),
        env: ProcessEnv::Inherit,
        tmux: TmuxConfig {
            tmux_bin: PathBuf::from("/nonexistent/hlr642-tmux"),
            socket: TmuxSocket::Path(PathBuf::from("/nonexistent/hlr642-tmux.sock")),
        },
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
    OpenCodeHarness::new(config("/nonexistent/hlr642-opencode", timeouts))
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
    let (_, (result, took)) = on_refused_port(
        |port| {
            let start = Instant::now();
            let result = http::request(port, "GET", "/global/health", None, Duration::from_secs(5));
            (result, start.elapsed())
        },
        |(result, _)| *result == Err(HttpError::Refused),
    );
    assert_eq!(result, Err(HttpError::Refused));
    assert!(
        took < Duration::from_secs(1),
        "a refused connect took {took:?}"
    );
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

/// The 64 MiB reply bound holds on the read-to-the-close path too: a body one byte past it is
/// `Garbled`, and one just under it is read whole, so the bound is neither missing nor set
/// lower.
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
/// section 7.1).
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
    let (_, unbound) = on_refused_port(|port| h.health(port), |answer| *answer == Ok(false));
    assert_eq!(unbound, Ok(false), "an unbound port");

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
    let (port, [created, listed, aborted]) = on_refused_port(
        |port| {
            [
                h.create_session(port).map(drop),
                h.list_sessions(port).map(drop),
                h.abort(port, ID),
            ]
        },
        |answers| {
            answers
                .iter()
                .all(|a| matches!(a, Err(PaneError::Unavailable { .. })))
        },
    );
    let messages = [
        unavailable(created, "create_session"),
        unavailable(listed, "list_sessions"),
        unavailable(aborted, "abort"),
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

// ---- AC 8 (#802 AC 5): serve_command and verify_serving ----

/// The binary every `serve_command` case names.
const BIN: &str = "/opt/oc/bin/opencode";

/// The secret-looking variable of the credential case: set only in the adapter process's own
/// environment, never in a configured `ProcessEnv`.
const SECRET_VAR: &str = "HLR802_PROVIDER_API_KEY";
const SECRET: &str = "s3cr3t-802-do-not-leak";

/// The marker that makes [`credential_probe_child`] build the argvs and check them.
const CREDENTIAL_PROBE: &str = "HLR802_CREDENTIAL_PROBE";

fn strings(argv: &Argv) -> Vec<&str> {
    argv.as_slice().iter().map(String::as_str).collect()
}

/// An adapter with `env` and `serve_args` `["--pure"]`, naming [`BIN`].
fn serving(env: ProcessEnv) -> OpenCodeHarness {
    let mut config = config(BIN, quick());
    config.serve_args = vec!["--pure".to_owned()];
    config.env = env;
    OpenCodeHarness::new(config)
}

/// The isolated environment of the cases: a scratch `HOME` and one `XDG_*` dir.
fn isolated() -> ProcessEnv {
    ProcessEnv::Isolated(vec![
        ("HOME".to_owned(), "/tmp/hlr802/home".to_owned()),
        ("XDG_DATA_HOME".to_owned(), "/tmp/hlr802/data".to_owned()),
    ])
}

/// #802 AC 5: with no agent, `serve_command` is the argv `start()` built before (`serve
/// --port P --hostname 127.0.0.1`, then `serve_args`), behind the one `env` prefix the TUI's
/// argv also uses: `env -u OPENCODE_DISABLE_TERMINAL_TITLE` under `Inherit`, `env -i K=V ...`
/// under `Isolated`.
#[test]
fn ac8_serve_command_is_the_server_argv_behind_the_env_prefix() {
    let name = pane_name();
    let inherit = serving(ProcessEnv::Inherit)
        .serve_command(&name, 48123, &ServeSpec::default())
        .unwrap();
    assert_eq!(
        strings(&inherit),
        [
            "env",
            "-u",
            "OPENCODE_DISABLE_TERMINAL_TITLE",
            BIN,
            "serve",
            "--port",
            "48123",
            "--hostname",
            "127.0.0.1",
            "--pure"
        ]
    );
    let isolated = serving(isolated())
        .serve_command(&name, 48124, &ServeSpec::default())
        .unwrap();
    assert_eq!(
        strings(&isolated),
        [
            "env",
            "-i",
            "HOME=/tmp/hlr802/home",
            "XDG_DATA_HOME=/tmp/hlr802/data",
            BIN,
            "serve",
            "--port",
            "48124",
            "--hostname",
            "127.0.0.1",
            "--pure"
        ]
    );
}

/// #802 AC 5: the env prefix is the TUI's own: the server's argv starts with exactly the
/// elements `tui_argv` starts with, up to the binary (one helper, not two).
#[test]
fn ac8_the_server_and_the_tui_share_one_env_prefix() {
    for env in [ProcessEnv::Inherit, isolated()] {
        let server = serving(env.clone())
            .serve_command(&pane_name(), 48123, &ServeSpec::default())
            .unwrap();
        let tui = holler_adapter_opencode::tui::tui_argv(BIN, &env, 48123, "/p", "ses_1");
        let at = tui.iter().position(|e| e == BIN).unwrap();
        assert_eq!(strings(&server)[..=at], tui[..=at], "{env:?}");
    }
}

/// #802 AC 5: `serve_command` is pure: it starts no process (the stand-in binary leaves no
/// marker) and sends nothing to the port, not even the health check `serve` used to make.
#[test]
fn ac8_serve_command_starts_nothing() {
    let scratch = Scratch::with_fake_opencode();
    let stub = Stub::start();
    stub.json("GET", "/global/health", 200, HEALTHY);
    let h = OpenCodeHarness::new(config(scratch.path().join("opencode"), quick()));
    let argv = h
        .serve_command(&pane_name(), stub.port, &ServeSpec::default())
        .unwrap();
    assert!(!argv.as_slice().is_empty());
    std::thread::sleep(Duration::from_millis(200));
    assert!(!scratch.spawned(), "nothing was run");
    assert!(
        stub.lines().is_empty(),
        "nothing was sent: {:?}",
        stub.lines()
    );
}

fn agent(text: &str) -> AgentKey {
    AgentKey::parse(text).unwrap()
}

fn is_agent_unsupported<T: std::fmt::Debug>(answer: &Result<T, PaneError>) -> bool {
    matches!(answer, Err(e @ PaneError::Refused { .. }) if e.code() == "agent-unsupported")
}

/// #802 AC 5 and plan review W-3: until #642 part 3 the adapter applies no agent, so a spec
/// with one is refused `agent-unsupported` by `serve_command` (no argv at all) and by
/// `verify_serving` (fail closed); a spec without one verifies `Ok` and sends nothing.
#[test]
fn ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies() {
    let stub = Stub::start();
    let h = serving(ProcessEnv::Inherit);
    let with = ServeSpec::default().with_agent(agent("feature-implementor"));
    let built = h.serve_command(&pane_name(), stub.port, &with);
    assert!(is_agent_unsupported(&built), "{built:?}");
    let verified = h.verify_serving(stub.port, &with);
    assert!(is_agent_unsupported(&verified), "{verified:?}");
    assert_eq!(h.verify_serving(stub.port, &ServeSpec::default()), Ok(()));
    let message = match built {
        Err(PaneError::Refused { message, .. }) => message,
        other => panic!("{other:?}"),
    };
    assert!(!message.contains('\n'), "one line: {message:?}");
}

/// Run by [`ac8_no_credential_of_the_adapters_environment_reaches_an_argv`] as a re-executed
/// child whose own environment holds [`SECRET_VAR`]; returns at once otherwise. It fails when
/// any argv the adapter builds, under either `ProcessEnv`, holds the secret's name or value.
#[test]
fn credential_probe_child() {
    if std::env::var_os(CREDENTIAL_PROBE).is_none() {
        return;
    }
    assert_eq!(
        std::env::var(SECRET_VAR).as_deref(),
        Ok(SECRET),
        "the child's env"
    );
    for env in [ProcessEnv::Inherit, isolated()] {
        let argv = serving(env.clone())
            .serve_command(&pane_name(), 48123, &ServeSpec::default())
            .unwrap();
        for element in argv.as_slice() {
            assert!(
                !element.contains(SECRET) && !element.contains(SECRET_VAR),
                "{env:?}: an argv element leaks the adapter's environment"
            );
        }
    }
}

/// #802 AC 5 (plan review W-2): nothing from the adapter's own process environment reaches
/// the server's argv, which `ps` and tmux show. The variable is set only in a re-executed
/// child (never `set_var` here, which races other tests).
#[test]
fn ac8_no_credential_of_the_adapters_environment_reaches_an_argv() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["credential_probe_child", "--exact", "--test-threads=1"])
        .env(CREDENTIAL_PROBE, "1")
        .env(SECRET_VAR, SECRET)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "the child test failed: {text}");
    assert!(text.contains("1 passed"), "the child ran the probe: {text}");
}

// ---- AC 11: the adapter's shape ----

#[test]
fn ac11_the_adapter_is_send_sync_and_static() {
    // A compile-time pin: this file does not build if it is not.
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<OpenCodeHarness>();
}

/// #802 removed `boot_try` with the adapter's own boot poll: the engine waits now.
#[test]
fn ac11_default_timeouts_are_10s_5s_2s_5s() {
    let expected = Timeouts {
        call: Duration::from_secs(10),
        request: Duration::from_secs(5),
        health: Duration::from_secs(2),
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
