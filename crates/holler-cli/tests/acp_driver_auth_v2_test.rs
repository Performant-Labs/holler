#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #459
//! Issue #459: ACP **v2** parity for `auth_method`. A spawn-mode session
//! whose adapter negotiates v2 and answers `session/new` with auth-required
//! (JSON-RPC `-32000`) sends one `auth/login` for the configured, advertised
//! method and retries `session/new` exactly once, or fails startup saying
//! why, with the same guarantees as the v1 flow (`acp_driver_auth_test.rs`).
//!
//! Every in-process test drives the real `AcpDriver` against the real
//! `stub-acp` process with its #459 auth modes (see `stub-acp/auth.rs`). The
//! stub's `--auth-log` is the wire-sequence data source: it records what the
//! adapter actually received, appending across launches, so "no `auth/login`
//! was sent" and "no v1 fallback relaunch happened" are observed, not
//! inferred from an error string.
//!
//! Every test takes the file-wide serial guard, because some tests mutate
//! process environment the driver reads (`HOLLER_ACP_TIMEOUT_MS`).

mod support;

use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use holler_body::acp_driver::{AcpDriver, DriverEvent, Status, StopReason};
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_proto::SessionName;
use support::{holler_cmd, join, kill_tree, make_own_process_group, mint_token, wait_for, Hub, StateDir, STARTUP_WAIT};

const MARKER: &str = "HOSTILE-DESC-MARKER";
const STUB: &str = env!("CARGO_BIN_EXE_stub-acp");
const STUB_V1: &str = env!("CARGO_BIN_EXE_stub-acp-v1");

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Sets a process env var for the guard's lifetime and removes it on drop.
/// Only used while `SERIAL` is held.
struct EnvGuard(&'static str);

impl EnvGuard {
    fn set(name: &'static str, value: &str) -> Self {
        // SAFETY: every test in this file holds `SERIAL`, and no other test in
        // this binary reads these names, so no concurrent env access exists.
        unsafe { std::env::set_var(name, value) };
        EnvGuard(name)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: as in `EnvGuard::set`.
        unsafe { std::env::remove_var(self.0) };
    }
}

/// A random per-run credential stand-in, only ever placed in an environment.
fn sentinel() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("sntl459x{}x{nanos}", std::process::id())
}

/// One test's stub launch: a fresh state dir holding its auth-log.
struct Fixture {
    dir: StateDir,
}

impl Fixture {
    fn new() -> Self {
        Fixture { dir: StateDir::new() }
    }

    fn log_path(&self) -> PathBuf {
        self.dir.path().join("auth.log")
    }

    /// A spawn-mode config running `stub-acp --auth-log <log> <args>`.
    fn config(&self, args: &[&str], auth_method: Option<&str>) -> SessionConfig {
        let mut command = vec![STUB.to_string(), "--auth-log".to_string(), self.log_path().display().to_string()];
        command.extend(args.iter().map(|a| a.to_string()));
        SessionConfig {
            name: SessionName::parse("alpha").expect("valid session name"),
            harness: "opencode".to_string(),
            mode: SessionMode::Spawn,
            command: Some(command),
            cwd: None,
            env: None,
            interrupt: Interrupt::Acp,
            endpoint: None,
            session_id: None,
            auth_method: auth_method.map(str::to_string),
        }
    }

    /// Every line the adapter recorded, across all launches.
    fn log(&self) -> Vec<String> {
        std::fs::read_to_string(self.log_path())
            .map(|s| s.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }
}

/// Spawn and expect a startup failure; return `DriverError::message()`.
async fn spawn_err(config: &SessionConfig) -> String {
    match AcpDriver::spawn(config).await {
        Err(e) => e.message(),
        Ok(driver) => {
            let _ = driver.shutdown().await;
            panic!("expected a startup failure, got a ready driver");
        }
    }
}

const INIT_NEW: [&str; 2] = ["initialize", "session/new"];
const INIT_NEW_LOGIN: [&str; 3] = ["initialize", "session/new", "auth/login stub-key"];
const FULL_AUTH: [&str; 4] = ["initialize", "session/new", "auth/login stub-key", "session/new"];
/// The shared stage suffix for "startup ended before auth could apply"
/// (the #439 v1 wording, reused by v2 per the brief's decisions 3 and 8).
const NOT_APPLIED: &str = "; auth_method \"stub-key\" was configured but not applied (startup ended before \
     session/new answered auth-required -32000)";
/// Today's exact reason for a first `session/new` failing `-32603` (no data)
/// with no `auth_method` configured (criterion 13: byte-for-byte unchanged).
const TODAY_32603: &str = "acp driver: startup failed: stub session/new failure";

// ---- criterion 1: authenticates, then a prompt completes --------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_logs_in_with_configured_method_then_a_prompt_completes() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let config = fx.config(&["--require-auth"], Some("stub-key"));
    let driver = AcpDriver::spawn(&config).await.map_err(|e| e.message()).expect("spawn logs in and succeeds");
    assert_eq!(driver.status(), Status::Idle);
    assert_eq!(fx.log(), FULL_AUTH, "wire sequence seen by the adapter");

    let mut stream = driver.prompt("hi").await;
    let mut events = Vec::new();
    loop {
        match tokio::time::timeout(Duration::from_secs(5), futures_util::StreamExt::next(&mut stream)).await {
            Ok(Some(event)) => {
                let done = matches!(event, DriverEvent::Done(_));
                events.push(event);
                if done {
                    break;
                }
            }
            Ok(None) => panic!("stream ended before Done: {events:?}"),
            Err(_) => panic!("timed out waiting for Done: {events:?}"),
        }
    }
    assert!(events.iter().any(|e| matches!(e, DriverEvent::Chunk(t) if t.contains("stub chunk"))), "{events:?}");
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}

// ---- criteria 2, 3, 14: refusals, no auth/login sent -------------------------

/// Shared checks for every refusal with `auth_method` set: names the config
/// key, and the adapter saw `initialize`, one `session/new`, no `auth/login`
/// and no v1 relaunch.
fn assert_refused(fx: &Fixture, msg: &str) {
    assert!(msg.contains("auth_method"), "reason must name the config key: {msg}");
    assert_eq!(fx.log(), INIT_NEW, "no auth/login, one session/new, no relaunch ({msg})");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_missing_auth_method_fails_auth_required_and_sends_nothing() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth"], None)).await;
    assert!(msg.contains("Authentication required"), "{msg}");
    assert_eq!(fx.log(), INIT_NEW, "auth/login is never sent without auth_method: {msg}");
    assert!(!msg.contains("not implemented yet"), "{msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_unadvertised_auth_method_refuses_and_is_never_sent() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth"], Some("nope"))).await;
    assert_refused(&fx, &msg);
    assert!(msg.contains("\"nope\"") && msg.contains("\"stub-key\""), "names both sides: {msg}");
    assert!(msg.contains("not advertised"), "{msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_no_methods_advertised_refuses_naming_auth_method() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--no-methods"], Some("stub-key"))).await;
    assert_refused(&fx, &msg);
    assert!(msg.contains("none advertised"), "{msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_terminal_auth_method_refuses_as_manual_login() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--terminal-method"], Some("stub-tty"))).await;
    assert_refused(&fx, &msg);
    assert!(msg.contains("\"stub-tty\"") && msg.contains("manual login"), "{msg}");
}

/// Criterion 14 / decision 9: an unknown-type method (`AuthMethod::Other`) is
/// never selectable; a configured id that only matches it is unadvertised.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_other_type_method_is_never_selected_or_sent() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--other-method"], Some("stub-custom"))).await;
    assert_refused(&fx, &msg);
    assert!(msg.contains("\"stub-custom\"") && msg.contains("not advertised"), "{msg}");
}

// ---- criterion 4: failure is fatal and bounded ------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_auth_login_error_is_fatal_without_a_second_session_new() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--reject-auth"], Some("stub-key"))).await;
    assert_eq!(fx.log(), INIT_NEW_LOGIN, "no retry, no v1 fallback relaunch: {msg}");
    assert!(msg.contains("\"stub-key\"") && msg.contains("stub rejected auth/login"), "{msg}");
    assert!(msg.contains("failed"), "the existing failure wording: {msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_still_auth_required_after_login_says_it_did_not_clear() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--always-auth-required"], Some("stub-key"))).await;
    assert_eq!(fx.log(), FULL_AUTH, "one auth/login, two session/new, no third: {msg}");
    assert!(msg.contains("auth_method \"stub-key\"") && msg.contains("did not clear"), "{msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_non_auth_error_on_the_retry_is_fatal_with_no_fifth_line() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--error-after-auth", "-32603"], Some("stub-key"))).await;
    assert_eq!(fx.log(), FULL_AUTH, "{msg}");
    assert!(msg.contains("-32603") && msg.contains("auth_method \"stub-key\""), "{msg}");
    assert!(!msg.contains("did not clear"), "a non-auth retry failure is not the did-not-clear case: {msg}");
}

// ---- criterion 5: nothing leaks, hostile text bounded -----------------------

/// Every v2 auth-flow failure with `auth_method` set, a sentinel credential in
/// `env`, and the stub's marker in every method description and auth-error
/// `data`. (The no-`auth_method` texts keep today's display, criterion 13.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_credential_or_hostile_text_in_any_v2_auth_failure_reason() {
    let _serial = SERIAL.lock().await;
    let secret = sentinel();
    let cases: &[(&[&str], Option<&str>)] = &[
        (&["--require-auth"], None),
        (&["--require-auth"], Some("nope")),
        (&["--require-auth", "--terminal-method"], Some("stub-tty")),
        (&["--require-auth", "--other-method"], Some("stub-custom")),
        (&["--require-auth", "--no-methods"], Some("stub-key")),
        (&["--require-auth", "--reject-auth"], Some("stub-key")),
        (&["--require-auth", "--always-auth-required"], Some("stub-key")),
        (&["--require-auth", "--error-after-auth", "-32603"], Some("stub-key")),
        (&["--require-auth", "--require-env", "STUB_ACP_ABSENT_459"], Some("stub-key")),
        (&["--require-auth", "--flood"], Some("stub-key")),
        (&["--fail-session-new", "-32603", "--flood"], Some("stub-key")),
    ];
    for (args, auth_method) in cases {
        let fx = Fixture::new();
        let mut config = fx.config(args, *auth_method);
        config.env = Some(BTreeMap::from([("STUB_ACP_CREDENTIAL".to_string(), secret.clone())]));
        let msg = spawn_err(&config).await;
        assert!(!msg.contains(&secret), "{args:?}: credential leaked into {msg}");
        if auth_method.is_some() {
            assert!(!msg.contains(MARKER), "{args:?}: adapter description/data leaked into {msg}");
            assert!(!msg.chars().any(char::is_control), "{args:?}: control characters replaced: {msg:?}");
        }
    }
}

fn longest_run(s: &str, c: char) -> usize {
    s.split(|x| x != c).map(|run| run.chars().count()).max().unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_hostile_advertised_list_and_rejection_text_are_capped() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--flood"], Some("nope"))).await;
    assert_refused(&fx, &msg);
    assert!(msg.matches("filler-").count() <= 8, "at most 8 ids echoed: {msg}");
    assert!(msg.contains("(+93 more)"), "{msg}");
    assert!(longest_run(&msg, 'f') <= 64, "each id capped to 64 chars: {msg}");
    assert!(!msg.chars().any(char::is_control), "control characters replaced: {msg:?}");
    assert!(msg.chars().count() < 1200, "{} chars", msg.chars().count());

    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--flood"], Some("stub-key"))).await;
    assert_eq!(fx.log(), INIT_NEW_LOGIN, "the 101st id is selectable: {msg}");
    assert!(msg.matches('Q').count() <= 200, "adapter message capped to 200 chars: {msg}");
    assert!(msg.chars().count() < 1200, "{} chars", msg.chars().count());
}

/// A non-auth first `session/new` failure with `auth_method` set is rebuilt
/// from code and capped message (never `data`), then says "not applied".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_hostile_non_auth_first_failure_with_auth_method_is_bounded() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--fail-session-new", "-32603", "--flood"], Some("stub-key"))).await;
    assert_eq!(fx.log(), INIT_NEW, "{msg}");
    assert!(!msg.contains(MARKER), "adapter data leaked: {msg}");
    assert!(!msg.chars().any(char::is_control), "control characters replaced: {msg:?}");
    assert!(msg.matches('Q').count() <= 200, "adapter message capped to 200 chars: {msg}");
    assert!(msg.contains("(JSON-RPC code -32603)") && msg.ends_with(NOT_APPLIED), "{msg}");
    assert!(msg.chars().count() < 600, "{} chars", msg.chars().count());
}

// ---- criterion 6: no-auth adapters unchanged --------------------------------

/// Guard, not a RED contract: pins that the new flow stays reactive. Both a
/// stub that advertises a method it does not require and one that
/// advertises nothing see exactly `initialize`, `session/new`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_no_auth_needed_never_sends_auth_login() {
    let _serial = SERIAL.lock().await;
    for args in [&["--advertise-methods"][..], &[][..]] {
        let fx = Fixture::new();
        let driver = AcpDriver::spawn(&fx.config(args, Some("stub-key")))
            .await
            .map_err(|e| e.message())
            .expect("an adapter that needs no auth starts as before");
        assert_eq!(fx.log(), INIT_NEW, "{args:?}");
        driver.shutdown().await.expect("shutdown");
    }
}

// ---- criteria 7, 8, 13: stage suffixes, old text gone, text unchanged -------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_non_auth_first_failure_with_auth_method_reports_not_applied() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--fail-session-new", "-32603"], Some("stub-key"))).await;
    assert_eq!(fx.log(), INIT_NEW, "a non-auth error never triggers auth/login: {msg}");
    assert_eq!(msg, format!("{TODAY_32603} (JSON-RPC code -32603){NOT_APPLIED}"));
}

/// Criterion 13 guard: with no `auth_method`, the v2 text is today's display.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_non_auth_first_failure_without_auth_method_is_unchanged() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--fail-session-new", "-32603"], None)).await;
    assert_eq!(msg, TODAY_32603);
}

/// A command that cannot be launched fails the v2 attempt fatally before any
/// handshake: the one annotation point says `auth_method` was not applied,
/// once, and the #459 "not implemented yet" text is gone (criterion 8).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_fatal_startup_failure_with_auth_method_reports_not_applied() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let mut config = fx.config(&[], None);
    config.command = Some(vec!["/nonexistent/holler-459-no-such-adapter".to_string()]);
    let bare = spawn_err(&config).await;
    assert!(!bare.contains("v1 fallback") && !bare.contains("auth_method"), "{bare}");

    config.auth_method = Some("stub-key".to_string());
    let with = spawn_err(&config).await;
    assert_eq!(with, format!("{bare}{NOT_APPLIED}"));
    assert!(!with.contains("not implemented yet"), "{with}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_slow_auth_login_times_out_saying_it_was_pending() {
    let _serial = SERIAL.lock().await;
    let _timeout = EnvGuard::set("HOLLER_ACP_TIMEOUT_MS", "1500");
    let fx = Fixture::new();
    let started = std::time::Instant::now();
    let msg = spawn_err(&fx.config(&["--require-auth", "--auth-delay-ms", "30000"], Some("stub-key"))).await;
    assert!(started.elapsed() < Duration::from_secs(10), "bounded by the timeout, no hang");
    assert_eq!(fx.log(), INIT_NEW_LOGIN, "timed out inside auth/login: {msg}");
    assert!(msg.contains("HOLLER_ACP_TIMEOUT_MS"), "the existing timeout reason: {msg}");
    assert!(msg.contains("auth_method \"stub-key\" was pending"), "says auth was pending: {msg}");
    assert!(!msg.contains("not applied"), "{msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_slow_retried_session_new_times_out_saying_the_retry_did_not_complete() {
    let _serial = SERIAL.lock().await;
    let _timeout = EnvGuard::set("HOLLER_ACP_TIMEOUT_MS", "1500");
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--retry-delay-ms", "30000"], Some("stub-key"))).await;
    assert_eq!(fx.log(), FULL_AUTH, "timed out inside the retried session/new: {msg}");
    assert!(msg.contains("HOLLER_ACP_TIMEOUT_MS"), "{msg}");
    assert!(msg.contains("auth_method \"stub-key\"") && msg.contains("retried session/new"), "{msg}");
}

/// A hang in the first `session/new` never reached `auth/login`: not applied.
/// Without `auth_method` the timeout text carries no auth clause.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_timeout_before_auth_reports_not_applied() {
    let _serial = SERIAL.lock().await;
    let _timeout = EnvGuard::set("HOLLER_ACP_TIMEOUT_MS", "1500");
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--new-delay-ms", "30000"], Some("stub-key"))).await;
    assert!(msg.contains("HOLLER_ACP_TIMEOUT_MS") && msg.ends_with(NOT_APPLIED), "{msg}");
    assert_eq!(msg.matches("auth_method").count(), 1, "the clause appears once: {msg}");

    let fx = Fixture::new();
    let bare = spawn_err(&fx.config(&["--new-delay-ms", "30000"], None)).await;
    assert!(bare.contains("HOLLER_ACP_TIMEOUT_MS") && !bare.contains("auth_method"), "{bare}");
}

/// Criterion 8, docs half: the README no longer says v2 auth is unsupported
/// and names `auth/login` for v2.
#[test]
fn readme_no_longer_says_v2_auth_is_unsupported() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")).expect("README");
    assert!(!readme.contains("not implemented yet (#459)"), "stale #459 sentence");
    assert!(!readme.contains("ACP v1 adapters only"), "stale v1-only bullet");
    assert!(readme.contains("auth/login"), "the README names v2's auth/login");
}

// ---- body-level: debug log (criteria 1, 5) and v1 fallback (criterion 12) ---

/// Lines a subprocess wrote to stderr, drained on a background thread so the
/// pipe never backs up.
fn drain(stderr: std::process::ChildStderr) -> Arc<Mutex<Vec<String>>> {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let sink = lines.clone();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stderr).lines().map_while(Result::ok) {
            sink.lock().unwrap().push(line);
        }
    });
    lines
}

/// Run a real hub and `holler body run --debug noisy` for one spawn session
/// (`argv`, `auth_method = "stub-key"`, the credential in `env`), wait until a
/// `say` is delivered, and return the body's stderr lines as parsed JSON.
fn body_log_after_say(argv: &[&str], secret: &str) -> (Vec<serde_json::Value>, String) {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret_tok) = mint_token(&state, "alpha");
    join(&state, &state, &hub.ws_url(), &token_id, &secret_tok);
    let argv = argv.iter().map(|a| serde_json::to_string(a).unwrap()).collect::<Vec<_>>().join(", ");
    let toml = format!(
        "[[session]]\nname = \"alpha\"\nharness = \"opencode\"\ncommand = [{argv}]\n\
         auth_method = \"stub-key\"\nenv = {{ STUB_ACP_CREDENTIAL = {secret_q} }}\n",
        secret_q = serde_json::to_string(secret).unwrap(),
    );
    std::fs::create_dir_all(state.body()).unwrap();
    let config = state.body().join("sessions.toml");
    std::fs::write(&config, toml).unwrap();

    let mut cmd = holler_cmd(&state);
    cmd.args(["--debug", "noisy", "body", "run", "--config"])
        .arg(&config)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    make_own_process_group(&mut cmd);
    let mut body = cmd.spawn().expect("spawn `holler body run`");
    let lines = drain(body.stderr.take().expect("stderr piped"));
    let text = || lines.lock().unwrap().join("\n");
    let said = wait_for(STARTUP_WAIT, || {
        if let Ok(Some(status)) = body.try_wait() {
            panic!("body exited ({status}) before the session was reachable:\n{}", text());
        }
        let out = support::say(&state, "alpha", "hello");
        let stderr = String::from_utf8_lossy(&out.stderr);
        (out.status.success() || !(stderr.contains("unknown session") || stderr.contains("not_connected"))).then_some(out)
    });
    let out = said.unwrap_or_else(|| panic!("`say alpha` never became reachable:\n{}", text()));
    assert!(out.status.success(), "say failed: {}\n{}", String::from_utf8_lossy(&out.stderr), text());
    // The spawn event follows readiness; wait for it rather than sleeping.
    let _ = wait_for(Duration::from_secs(5), || {
        lines.lock().unwrap().iter().any(|l| l.contains("\"spawned\"")).then_some(())
    });
    kill_tree(&mut body);
    drop(hub);
    let raw = text();
    let events = raw.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()).collect();
    (events, raw)
}

/// Whether an ACP driver event of `kind` matching `extra` was logged (the hub
/// handshake's `circuit/authenticate` is not an `acp` event).
fn has_acp(events: &[serde_json::Value], kind: &str, extra: impl Fn(&serde_json::Value) -> bool) -> bool {
    events.iter().any(|v| v["component"] == "acp" && v["type"] == kind && extra(v))
}

/// A v2 adapter needing auth: the body logs an ACP `auth/login` event and then
/// `spawned protocol=v2`, and its log never contains the credential or the
/// adapter's description/data marker.
#[test]
fn body_debug_noisy_log_names_auth_login_but_never_the_credential() {
    let _serial = SERIAL.blocking_lock();
    let secret = sentinel();
    let (events, raw) = body_log_after_say(&[STUB, "--require-auth", "--require-env", "STUB_ACP_CREDENTIAL"], &secret);
    assert!(has_acp(&events, "auth/login", |_| true), "an acp `auth/login` event is logged:\n{raw}");
    assert!(has_acp(&events, "spawn", |v| v["event"] == "spawned" && v["protocol"] == "v2"), "{raw}");
    assert!(!raw.contains(&secret), "credential leaked into the body log");
    assert!(!raw.contains(MARKER), "adapter description/data leaked into the body log");
}

/// Criterion 12 guard: a v1-only adapter with `auth_method` set still takes
/// the v1 fallback (logged as `acp_v2_negotiation_failed ... fallback=v1`),
/// authenticates there and succeeds; the v2 path never makes it fatal.
#[test]
fn v1_only_adapter_with_auth_method_still_falls_back_and_authenticates() {
    let _serial = SERIAL.blocking_lock();
    let secret = sentinel();
    let (events, raw) =
        body_log_after_say(&[STUB_V1, "--require-auth", "--require-env", "STUB_ACP_CREDENTIAL"], &secret);
    assert!(
        has_acp(&events, "spawn", |v| v["event"] == "acp_v2_negotiation_failed" && v["fallback"] == "v1"),
        "the v2 attempt falls back to v1:\n{raw}"
    );
    assert!(has_acp(&events, "authenticate", |_| true), "authenticates on v1:\n{raw}");
    assert!(has_acp(&events, "spawn", |v| v["event"] == "spawned" && v["protocol"] == "v1"), "{raw}");
    assert!(!raw.contains(&secret), "credential leaked into the body log");
}
