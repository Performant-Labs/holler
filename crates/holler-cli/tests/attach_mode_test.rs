#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #195
//! RED/GREEN tests for `mode = "attach"` end to end (issue #195) — the
//! issue's own test list, verbatim in name — against a **real** hub + body
//! pair (real subprocesses, real loopback sockets, the same discipline every
//! other `holler-cli` integration test in this crate follows) and a **fake**
//! OpenCode HTTP server standing in for the harness process an attach
//! session never spawns.
//!
//! # Why this file lives in `holler-cli`, not `holler-body`
//!
//! Same reason `session_manager_test.rs` (#189) and `acp_driver_test.rs`
//! (#188) do: real end-to-end coverage of `mode=attach` needs the real `body
//! run`/`hub serve` binaries and the CLI verbs (`say`/`interrupt`/`roster`/
//! `body detach`/`hub query … support`) — none of which are reachable from a
//! `holler-body` unit/integration test.
//!
//! # Fake server reuse (issue #194)
//!
//! This file reuses the exact fake OpenCode HTTP server `http_attach_driver_
//! test.rs` built for issue #194 (`crates/holler-body/tests/
//! http_attach_driver_test/fake_server.rs`) via a `#[path]` include, rather
//! than building a second one — the issue's own instruction. `push_event`/
//! `requests()`/`set_questions()` etc. all take `&self` and touch nothing but
//! a `std::sync::Mutex`-guarded `FakeState`, so they are safe to call from a
//! plain `std::thread::scope`d thread racing a blocking CLI subprocess call —
//! no tokio context needed on that side, only for `FakeServer::start()`
//! itself (hence every test here is `#[tokio::test(flavor =
//! "multi_thread")]`, not a plain `#[test]`).

#[path = "../../holler-body/tests/http_attach_driver_test/fake_server.rs"]
mod fake_server;
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use fake_server::{message_updated_assistant, part_updated_text, session_idle, FakeServer};
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_body::http_attach_driver::HttpAttachDriver;
use holler_proto::SessionName;
use serde_json::Value;

use support::{stub_acp_bin, wait_for, Body, Hub, StateDir, STARTUP_WAIT};

/// Run `holler ARGS... --json` against `state_path` directly (a bare
/// `PathBuf`, not `&StateDir`) — for callers that need to run a CLI command
/// from a background thread while the owning `StateDir` stays on the test's
/// main thread (mirrors `talk_test.rs`'s own `say_full`, duplicated here
/// since that helper lives in a different test binary target).
fn say_at(state_path: &Path, session: &str, text: &str) -> Output {
    Command::new(support::holler_bin())
        .env("HOLLER_STATE_DIR", state_path)
        .env("HOLLER_DEBUG", "quiet")
        .env("HOLLER_LOG_FORMAT", "json")
        .args(["say", session, text])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|c| c.wait_with_output())
        .expect("run `say`")
}

/// Poll `say SESSION TEXT` until it stops failing with `unknown session` (the
/// hub has not yet cached this body's first `session/presence`) or `timeout`
/// elapses — mirrors `talk_test.rs`'s own `say_ready`.
fn say_ready(state: &StateDir, session: &str, text: &str, timeout: Duration) -> Output {
    wait_for(timeout, || {
        let out = support::say(state, session, text);
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.success() || !stderr.contains("unknown session") {
            Some(out)
        } else {
            None
        }
    })
    .unwrap_or_else(|| panic!("`say {session}` never got past unknown_session within {timeout:?}"))
}

/// `holler body support FEATURE --json`, parsed.
fn body_support_json(state: &StateDir) -> impl Fn(&str) -> Value + '_ {
    move |feature: &str| {
        let out = Command::new(support::holler_bin())
            .env("HOLLER_STATE_DIR", state.path())
            .env("HOLLER_DEBUG", "quiet")
            .args(["body", "support", feature, "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|c| c.wait_with_output())
            .expect("run `body support`");
        assert!(
            out.status.success(),
            "`body support {feature}` must exit 0; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).expect("body support --json is valid JSON")
    }
}

/// A `[[session]]` TOML block for an attach-mode session.
fn attach_block(name: &str, endpoint: &str, session_id: &str) -> String {
    format!(
        "[[session]]\nname = {}\nharness = \"opencode\"\nmode = \"attach\"\nendpoint = {}\nsession_id = {}\n\n",
        serde_json::to_string(name).expect("json-encode name"),
        serde_json::to_string(endpoint).expect("json-encode endpoint"),
        serde_json::to_string(session_id).expect("json-encode session_id"),
    )
}

/// A `[[session]]` TOML block for a spawn-mode session running the built
/// `stub-acp` fixture (issue #130) — the same fixture every other
/// `holler-cli` integration test spawns for a "normal" session.
fn spawn_block(name: &str, extra: &[&str]) -> String {
    let argv = std::iter::once(stub_acp_bin())
        .chain(extra.iter().copied())
        .map(|a| serde_json::to_string(a).expect("json-encode argv element"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "[[session]]\nname = {}\nharness = \"opencode\"\ncommand = [{}]\n\n",
        serde_json::to_string(name).expect("json-encode name"),
        argv,
    )
}

/// Write `<state>/body/sessions.toml` from a mix of attach/spawn blocks
/// (`spawn_block`/`attach_block`) — the mixed-mode case #195's own RED list
/// asks for (`mixed_spawn_and_attach_in_one_config`) needs both kinds in one
/// file, which `support::write_sessions_toml` (spawn-only) cannot express.
fn write_mixed_sessions_toml(state: &StateDir, blocks: &[String]) -> PathBuf {
    let path = state.body().join("sessions.toml");
    std::fs::create_dir_all(state.body()).expect("create body dir");
    std::fs::write(&path, blocks.concat()).expect("write sessions.toml");
    path
}

/// Join `state`'s body to `hub` with a freshly minted token (mirrors
/// `body_run_test.rs`'s own `join_fresh`, duplicated here rather than shared
/// since that one lives in a different test binary target).
fn join_fresh(state: &StateDir, ws_url: &str, label: &str) {
    let (token_id, secret) = support::mint_token(state, label);
    support::join(state, ws_url, &token_id, &secret);
}

/// Poll `holler roster --json`'s rows until one whose name ends with
/// `/<session>` satisfies `want`, or panic after `timeout`.
fn wait_for_roster_row(
    state: &StateDir,
    session: &str,
    timeout: Duration,
    want: impl Fn(&Value) -> bool,
) -> Value {
    wait_for(timeout, || {
        let doc = support::roster_json(state);
        doc.get("rows")
            .and_then(|r| r.as_array())
            .and_then(|rows| {
                rows.iter()
                    .find(|r| r.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.ends_with(&format!("/{session}"))) && want(r))
                    .cloned()
            })
    })
    .unwrap_or_else(|| panic!("no matching roster row for {session:?} within {timeout:?}: {:?}", support::roster_json(state)))
}

/// Build the `SessionConfig` this test file's direct `HttpAttachDriver::
/// attach` liveness checks use (mirrors `http_attach_driver_test.rs`'s own
/// `config` helper) — used only to prove the fake server is still answering
/// after a detach/SIGINT, exactly like that story's own
/// `shutdown_does_not_touch_fake_server`.
fn attach_config(endpoint: &str, session_id: &str) -> SessionConfig {
    SessionConfig {
        name: SessionName::parse("probe").expect("valid session name"),
        harness: "opencode".to_string(),
        mode: SessionMode::Attach,
        command: None,
        cwd: None,
        env: None,
        interrupt: Interrupt::Http,
        endpoint: Some(endpoint.to_string()),
        session_id: Some(session_id.to_string()),
    }
}

/// Block waiting for a `POST {path}` to show up in `server`'s recorded
/// requests, or panic after `timeout` — the "the driver actually sent it"
/// half of every prompt/interrupt test below (mirrors
/// `http_attach_driver_test.rs`'s own inline polling loops).
fn wait_for_request(server: &FakeServer, method: &str, path: &str, timeout: Duration) {
    wait_for_new_request(server, 0, method, path, timeout);
}

/// [`wait_for_request`], but only counting requests recorded from index
/// `since` onward — `server.requests()` never clears its history, so a test
/// that hits the same `path` more than once (e.g. two prompts to the same
/// attach session) must not let an *earlier* call's already-recorded request
/// satisfy a *later* wait.
fn wait_for_new_request(server: &FakeServer, since: usize, method: &str, path: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if server.requests().iter().skip(since).any(|r| r.method == method && r.path == path) {
            return;
        }
        assert!(Instant::now() < deadline, "{method} {path} never arrived (since request #{since})");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_session_appears_on_roster_with_mode_attach_and_harness_session_id() {
    let server = FakeServer::start().await;
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = write_mixed_sessions_toml(&state, &[attach_block("alpha", &server.endpoint(), "ses_1")]);
    let body = Body::start(&state, &config);

    let row = wait_for_roster_row(&state, "alpha", STARTUP_WAIT, |_| true);
    assert_eq!(row["mode"].as_str(), Some("attach"), "row: {row}");
    assert_eq!(row["harness_session_id"].as_str(), Some("ses_1"), "row: {row}");
    assert_eq!(row["state"].as_str(), Some("idle"), "an attached session with nothing in flight is idle: {row}");

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn say_to_attached_session_round_trips_via_http() {
    let server = FakeServer::start().await;
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = write_mixed_sessions_toml(&state, &[attach_block("alpha", &server.endpoint(), "ses_2")]);
    let body = Body::start(&state, &config);
    wait_for_roster_row(&state, "alpha", STARTUP_WAIT, |_| true);

    let out = std::thread::scope(|scope| {
        scope.spawn(|| {
            wait_for_request(&server, "POST", "/session/ses_2/prompt_async", Duration::from_secs(10));
            server.push_event(message_updated_assistant("m1", "ses_2"));
            server.push_event(part_updated_text("m1", "ses_2", "hello from opencode"));
            server.push_event(session_idle("ses_2"));
        });
        support::say(&state, "alpha", "hi")
    });

    assert!(out.status.success(), "say must exit 0; stderr: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("hello from opencode"), "say's reply must carry the HTTP-driven turn's text: {text:?}");

    let requests = server.requests();
    assert!(
        requests.iter().any(|r| r.method == "POST" && r.path == "/session/ses_2/prompt_async"),
        "prompt_async must have reached the fake OpenCode server"
    );

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupt_attached_session_posts_interrupt_and_session_survives() {
    let server = FakeServer::start().await;
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = write_mixed_sessions_toml(&state, &[attach_block("alpha", &server.endpoint(), "ses_3")]);
    let body = Body::start(&state, &config);
    wait_for_roster_row(&state, "alpha", STARTUP_WAIT, |_| true);

    let say_handle = {
        let state_path = state.path().to_path_buf();
        std::thread::spawn(move || say_at(&state_path, "alpha", "do a long thing"))
    };
    wait_for_request(&server, "POST", "/session/ses_3/prompt_async", Duration::from_secs(10));
    // Let the session actually reach `working` on the roster before racing
    // the interrupt against it.
    wait_for_roster_row(&state, "alpha", Duration::from_secs(10), |r| r["state"].as_str() == Some("working"));

    let interrupt_out = std::thread::scope(|scope| {
        let joiner = scope.spawn(|| {
            wait_for_request(&server, "POST", "/api/session/ses_3/interrupt", Duration::from_secs(10));
            server.push_event(session_idle("ses_3"));
        });
        let out = support::interrupt(&state, "alpha");
        joiner.join().expect("event-pusher thread must not panic");
        out
    });
    assert!(
        interrupt_out.status.success(),
        "interrupt must exit 0; stderr: {}",
        String::from_utf8_lossy(&interrupt_out.stderr)
    );
    let _ = say_handle.join().expect("say thread must not panic");

    // The session survives: it accepts and completes a fresh prompt right
    // after, proving the attach driver (and the fake OpenCode "process") are
    // still alive and usable, not torn down by the interrupt. `since` is the
    // request count *before* this second prompt, so the wait below cannot be
    // spuriously satisfied by the first turn's already-recorded
    // `prompt_async` (`server.requests()` never clears its history).
    let since = server.requests().len();
    let out2 = std::thread::scope(|scope| {
        scope.spawn(|| {
            wait_for_new_request(&server, since, "POST", "/session/ses_3/prompt_async", Duration::from_secs(10));
            server.push_event(message_updated_assistant("m2", "ses_3"));
            server.push_event(part_updated_text("m2", "ses_3", "still alive"));
            server.push_event(session_idle("ses_3"));
        });
        say_ready(&state, "alpha", "hi again", Duration::from_secs(10))
    });
    assert!(out2.status.success(), "a fresh prompt after interrupt must succeed; stderr: {}", String::from_utf8_lossy(&out2.stderr));

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detach_leaves_fake_opencode_running() {
    let server = FakeServer::start().await;
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = write_mixed_sessions_toml(&state, &[attach_block("alpha", &server.endpoint(), "ses_4")]);
    let body = Body::start(&state, &config);
    wait_for_roster_row(&state, "alpha", STARTUP_WAIT, |_| true);

    body.stop(&state, Duration::from_secs(10));

    // `body detach` never touched the fake OpenCode "process": a fresh
    // attach existence check against the very same endpoint/session still
    // succeeds — the same idiom `http_attach_driver_test.rs`'s own
    // `shutdown_does_not_touch_fake_server` uses.
    let still_alive = HttpAttachDriver::attach(&attach_config(&server.endpoint(), "ses_4")).await;
    assert!(still_alive.is_ok(), "the fake OpenCode server must still answer after `body detach`");

    hub.stop(Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigint_leaves_fake_opencode_running() {
    let server = FakeServer::start().await;
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = write_mixed_sessions_toml(&state, &[attach_block("alpha", &server.endpoint(), "ses_5")]);
    let mut body = Body::start(&state, &config);
    wait_for_roster_row(&state, "alpha", STARTUP_WAIT, |_| true);

    // SIGINT the whole body process group directly (not `body detach`) — the
    // other real teardown path issue #195's spec names.
    let child = body.child_mut();
    let pid = child.id() as i32;
    #[cfg(unix)]
    unsafe {
        libc::kill(-pid, libc::SIGINT);
    }
    let exited = wait_for(Duration::from_secs(10), || child.try_wait().ok().flatten());
    assert!(exited.is_some(), "body run must exit on SIGINT within 10s");

    let still_alive = HttpAttachDriver::attach(&attach_config(&server.endpoint(), "ses_5")).await;
    assert!(still_alive.is_ok(), "the fake OpenCode server must still answer after SIGINT");

    hub.stop(Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_failure_does_not_stop_sibling_spawn_session() {
    let server = FakeServer::start().await;
    // A 404 existence check on both prefixes — this attach session's initial
    // attach must fail closed (issue #194's own `check_exists` contract).
    {
        let mut fake_state = server.state.lock().expect("fake server state lock");
        fake_state.exists_v1 = false;
        fake_state.exists_v2 = false;
    }
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = write_mixed_sessions_toml(
        &state,
        &[
            attach_block("alpha", &server.endpoint(), "ses_missing"),
            spawn_block("beta", &["--chunks", "2"]),
        ],
    );
    let body = Body::start(&state, &config);

    // The sibling spawn-mode session is entirely unaffected: it dispatches
    // and completes an ordinary turn on its own, proving the failed attach
    // never stopped the other session's task or the run as a whole.
    let out = say_ready(&state, "beta", "hi beta", STARTUP_WAIT);
    assert!(out.status.success(), "beta must complete normally; stderr: {}", String::from_utf8_lossy(&out.stderr));

    // alpha never appears on the roster at all while its attach keeps
    // failing (issue #195: "presence omits the session until it attaches").
    let doc = support::roster_json(&state);
    let alpha_present = doc
        .get("rows")
        .and_then(|r| r.as_array())
        .is_some_and(|rows| rows.iter().any(|r| r.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.ends_with("/alpha"))));
    assert!(!alpha_present, "alpha must be omitted from presence while its attach keeps failing: {doc}");

    // Now let the fake server start answering: alpha's own 30s retry loop is
    // too slow for a test budget, so this only proves the omission holds
    // throughout — the retry cadence itself is covered by `task.rs`'s own
    // `ATTACH_RETRY_INTERVAL` constant, not re-timed here.
    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn support_attach_true_support_opencode_reflects_endpoint() {
    let server = FakeServer::start().await;
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = write_mixed_sessions_toml(&state, &[attach_block("alpha", &server.endpoint(), "ses_6")]);
    let body = Body::start(&state, &config);
    wait_for_roster_row(&state, "alpha", STARTUP_WAIT, |_| true);

    // The `attach` capability is always ok:true, independent of any session
    // config or live endpoint.
    let support = body_support_json(&state);
    let attach_support = support("attach");
    assert_eq!(attach_support["ok"].as_bool(), Some(true), "{attach_support}");
    let opencode_http_support = support("opencode-http");
    assert_eq!(opencode_http_support["ok"].as_bool(), Some(true), "{opencode_http_support}");

    // `support opencode` for a live, answering endpoint is ok:true.
    let harness_support = support("opencode");
    assert_eq!(harness_support["ok"].as_bool(), Some(true), "{harness_support}");

    // Once the endpoint stops answering (both existence-check prefixes),
    // `support opencode` flips to ok:false — this is the real, live probe
    // issue #195 asks for, not a fabricated "attach configured" placeholder.
    {
        let mut fake_state = server.state.lock().expect("fake server state lock");
        fake_state.exists_v1 = false;
        fake_state.exists_v2 = false;
    }
    let harness_support_down = support("opencode");
    assert_eq!(harness_support_down["ok"].as_bool(), Some(false), "{harness_support_down}");

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mixed_spawn_and_attach_in_one_config() {
    let server = FakeServer::start().await;
    let state = StateDir::new();
    let hub = Hub::start(&state);
    join_fresh(&state, &hub.ws_url(), "kiwi");
    let config = write_mixed_sessions_toml(
        &state,
        &[
            attach_block("alpha", &server.endpoint(), "ses_7"),
            spawn_block("beta", &["--chunks", "2"]),
        ],
    );
    let body = Body::start(&state, &config);

    let alpha_row = wait_for_roster_row(&state, "alpha", STARTUP_WAIT, |_| true);
    assert_eq!(alpha_row["mode"].as_str(), Some("attach"), "{alpha_row}");
    let beta_row = wait_for_roster_row(&state, "beta", STARTUP_WAIT, |_| true);
    assert_eq!(beta_row["mode"].as_str(), Some("spawn"), "{beta_row}");

    // Both sessions actually work, independently, in the same run.
    let beta_out = say_ready(&state, "beta", "hi beta", STARTUP_WAIT);
    assert!(beta_out.status.success(), "beta must complete; stderr: {}", String::from_utf8_lossy(&beta_out.stderr));

    let alpha_out = std::thread::scope(|scope| {
        scope.spawn(|| {
            wait_for_request(&server, "POST", "/session/ses_7/prompt_async", Duration::from_secs(10));
            server.push_event(message_updated_assistant("m1", "ses_7"));
            server.push_event(part_updated_text("m1", "ses_7", "hi from alpha"));
            server.push_event(session_idle("ses_7"));
        });
        support::say(&state, "alpha", "hi alpha")
    });
    assert!(alpha_out.status.success(), "alpha must complete; stderr: {}", String::from_utf8_lossy(&alpha_out.stderr));
    assert!(String::from_utf8_lossy(&alpha_out.stdout).contains("hi from alpha"));

    body.stop(&state, Duration::from_secs(5));
    hub.stop(Duration::from_secs(5));
}
