#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #439
//! Issue #439: the body's ACP driver answers an adapter's `authenticate`
//! requirement on the ACP **v1** path. A spawn-mode session names an auth
//! method (`auth_method`); when `session/new` fails with JSON-RPC `-32000`
//! the driver sends one `authenticate` for that id and retries `session/new`
//! exactly once, or fails startup saying why.
//!
//! Every test drives the real `AcpDriver` against the real `stub-acp-v1`
//! process (see that stub's module doc for its auth modes and the
//! v1-launch-only `--auth-log`). The auth-log is the wire-sequence data
//! source: it records what the adapter actually received, so "no
//! `authenticate` was sent" is observed, not inferred from an error string.
//!
//! Every test takes the file-wide serial guard, because two tests mutate
//! process environment the driver or the stub reads
//! (`HOLLER_ACP_TIMEOUT_MS`, and a credential variable inherited by the
//! spawned adapter).
//!
//! Lives in `holler-cli`, not `holler-body`, for the `CARGO_BIN_EXE_*`
//! reason `acp_driver_test.rs`'s module doc explains.

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
const STUB: &str = env!("CARGO_BIN_EXE_stub-acp-v1");

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Sets a process env var for the guard's lifetime and removes it on drop
/// (also on panic). Only used while `SERIAL` is held.
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

/// A random per-run credential stand-in. Only ever placed in an environment;
/// asserted absent from every error and log.
fn sentinel() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("sntl439x{}x{nanos}", std::process::id())
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

    /// A spawn-mode config running `stub-acp-v1 --auth-log <log> <args>`.
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

    /// The lines the v1 launch recorded (empty if it never ran).
    fn log(&self) -> Vec<String> {
        std::fs::read_to_string(self.log_path())
            .map(|s| s.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }
}

fn env_of(name: &str, value: &str) -> Option<BTreeMap<String, String>> {
    Some(BTreeMap::from([(name.to_string(), value.to_string())]))
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
const FULL_AUTH: [&str; 4] = ["initialize", "session/new", "authenticate stub-key", "session/new"];

// ---- criterion 1: happy path ------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_authenticates_with_configured_method_then_a_prompt_completes() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let config = fx.config(&["--require-auth"], Some("stub-key"));
    let driver = AcpDriver::spawn(&config).await.map_err(|e| e.message()).expect("spawn authenticates and succeeds");
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
    assert!(events.iter().any(|e| matches!(e, DriverEvent::Chunk(t) if t == "hello from v1")), "{events:?}");
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}

// ---- criterion 2: refusals, no authenticate sent ----------------------------

/// Shared checks for every refusal: names the config key, and the adapter saw
/// exactly `initialize`, one `session/new`, and no `authenticate`.
fn assert_refused(fx: &Fixture, msg: &str) {
    assert!(msg.contains("auth_method"), "reason must name the config key: {msg}");
    assert_eq!(fx.log(), INIT_NEW, "no authenticate, one session/new ({msg})");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_missing_auth_method_refuses_listing_advertised_ids() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth"], None)).await;
    assert_refused(&fx, &msg);
    assert!(msg.contains("\"stub-key\""), "lists the advertised id, quoted: {msg}");
    for other in ["not advertised", "none advertised", "manual login"] {
        assert!(!msg.contains(other), "missing-config reason is distinct from {other:?}: {msg}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_unadvertised_auth_method_refuses_and_is_never_sent() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth"], Some("nope"))).await;
    assert_refused(&fx, &msg);
    assert!(msg.contains("\"nope\"") && msg.contains("\"stub-key\""), "{msg}");
    assert!(msg.contains("not advertised"), "{msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_terminal_auth_method_refuses_as_manual_login() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--terminal-method"], Some("stub-tty"))).await;
    assert_refused(&fx, &msg);
    assert!(msg.contains("\"stub-tty\"") && msg.contains("manual login"), "{msg}");
    assert!(!msg.contains("not advertised"), "terminal reason is distinct: {msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_no_methods_advertised_refuses_naming_auth_method() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--no-methods"], Some("stub-key"))).await;
    assert_refused(&fx, &msg);
    assert!(msg.contains("none advertised"), "{msg}");
}

// ---- criterion 3: authenticate error is fatal -------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_authenticate_error_is_fatal_without_a_second_session_new() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--reject-auth"], Some("stub-key"))).await;
    assert_eq!(fx.log(), ["initialize", "session/new", "authenticate stub-key"], "{msg}");
    assert!(msg.contains("\"stub-key\"") && msg.contains("stub rejected authenticate"), "{msg}");
}

// ---- criterion 4: bounded retry ---------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_still_auth_required_after_authenticate_says_it_did_not_clear() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--always-auth-required"], Some("stub-key"))).await;
    assert_eq!(fx.log(), FULL_AUTH, "one authenticate, two session/new, no third: {msg}");
    assert!(msg.contains("auth_method \"stub-key\"") && msg.contains("did not clear"), "{msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_non_auth_error_on_the_retry_is_fatal_with_no_fifth_line() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--error-after-auth", "-32603"], Some("stub-key"))).await;
    assert_eq!(fx.log(), FULL_AUTH, "{msg}");
    assert!(msg.contains("-32603"), "{msg}");
    assert!(!msg.contains("did not clear"), "a non-auth retry failure is not the did-not-clear case: {msg}");
}

// ---- criterion 5: not applied (v1) ------------------------------------------

/// Today's exact reason for a first `session/new` failing `-32603` with no
/// `auth_method` configured (must stay byte-identical, decision (f)).
const TODAY_32603: &str = "acp driver: startup failed: stub session/new failure";
const V1_SUFFIX: &str = "; auth_method \"stub-key\" was configured but not applied (startup ended before \
     session/new answered auth-required -32000)";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_non_auth_first_failure_with_auth_method_reports_not_applied() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--fail-session-new", "-32603"], Some("stub-key"))).await;
    assert_eq!(fx.log(), INIT_NEW, "a non-auth error never triggers authenticate: {msg}");
    assert_eq!(msg, format!("{TODAY_32603} (JSON-RPC code -32603){V1_SUFFIX}"));
    assert!(!msg.contains("#459"), "a v1 failure never carries the v2 suffix: {msg}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_non_auth_first_failure_without_auth_method_is_unchanged() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--fail-session-new", "-32603"], None)).await;
    assert_eq!(msg, TODAY_32603);
}

/// The class: every adapter-supplied string in a v1 error path is capped and
/// sanitized and never carries `data`, including the first `session/new`
/// failure with `auth_method` set (which used to echo the SDK error's full
/// display). Without `auth_method` the text stays today's, byte for byte.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_hostile_non_auth_first_failure_with_auth_method_is_bounded() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--fail-session-new", "-32603", "--flood"], Some("stub-key"))).await;
    assert_eq!(fx.log(), INIT_NEW, "{msg}");
    assert!(!msg.contains(MARKER), "adapter data leaked: {msg}");
    assert!(!msg.chars().any(char::is_control), "control characters replaced: {msg:?}");
    assert!(msg.matches('Q').count() <= 200, "adapter message capped to 200 chars: {msg}");
    assert!(msg.contains("(JSON-RPC code -32603)") && msg.contains(V1_SUFFIX), "{msg}");
    assert!(msg.chars().count() < 600, "{} chars", msg.chars().count());
}

// ---- criterion 6: not applied (v2 fatal arm) --------------------------------

const V2_SUFFIX: &str =
    "; auth_method \"stub-key\" was configured but not applied: ACP v2 auth support is not implemented yet (#459)";

/// A command that cannot be launched fails the **v2** attempt fatally (the
/// connection task ends before readiness; no v1 fallback is tried).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_fatal_startup_failure_with_auth_method_reports_not_applied() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let mut config = fx.config(&[], None);
    config.command = Some(vec!["/nonexistent/holler-439-no-such-adapter".to_string()]);
    let bare = spawn_err(&config).await;
    assert!(!bare.contains("v1 fallback"), "must take the v2 Fatal arm: {bare}");
    assert!(!bare.contains("not applied"), "{bare}");

    config.auth_method = Some("stub-key".to_string());
    let with = spawn_err(&config).await;
    assert_eq!(with, format!("{bare}{V2_SUFFIX}"));
}

// ---- criterion 7: no auth needed --------------------------------------------

/// Guard, not a RED contract: today's driver never authenticates, so this
/// passes before and after; it pins that the new flow stays reactive.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_no_auth_needed_never_sends_authenticate() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let driver = AcpDriver::spawn(&fx.config(&["--advertise-methods"], Some("stub-key")))
        .await
        .map_err(|e| e.message())
        .expect("an adapter that needs no auth starts as before");
    assert_eq!(fx.log(), INIT_NEW);
    driver.shutdown().await.expect("shutdown");
}

// ---- criterion 8: credential from the environment ---------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_credential_in_config_env_reaches_the_adapter() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let mut config = fx.config(&["--require-auth", "--require-env", "STUB_ACP_CREDENTIAL"], Some("stub-key"));
    let missing = spawn_err(&config).await;
    assert!(missing.contains("stub credential missing"), "without the env var the adapter rejects: {missing}");

    let fx = Fixture::new();
    config = fx.config(&["--require-auth", "--require-env", "STUB_ACP_CREDENTIAL"], Some("stub-key"));
    config.env = env_of("STUB_ACP_CREDENTIAL", &sentinel());
    let driver = AcpDriver::spawn(&config).await.map_err(|e| e.message()).expect("credential from config.env");
    assert_eq!(fx.log(), FULL_AUTH);
    driver.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_credential_in_body_process_env_reaches_the_adapter() {
    let _serial = SERIAL.lock().await;
    let _env = EnvGuard::set("STUB_ACP_ENV_ONLY_439", &sentinel());
    let fx = Fixture::new();
    let config = fx.config(&["--require-auth", "--require-env", "STUB_ACP_ENV_ONLY_439"], Some("stub-key"));
    assert!(config.env.is_none());
    let driver = AcpDriver::spawn(&config).await.map_err(|e| e.message()).expect("credential inherited from the body's env");
    assert_eq!(fx.log(), FULL_AUTH);
    driver.shutdown().await.expect("shutdown");
}

// ---- criterion 9: nothing leaks, hostile text bounded -----------------------

/// Every auth-flow failure reason, with a sentinel credential in `env` and the
/// stub's marker in every method description and auth-error `data`. (Without
/// `auth_method` a non-auth first failure keeps today's text, which includes
/// adapter `data`; that unchanged case is asserted in criterion 5.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_credential_or_hostile_text_in_any_auth_failure_reason() {
    let _serial = SERIAL.lock().await;
    let secret = sentinel();
    let cases: &[(&[&str], Option<&str>)] = &[
        (&["--require-auth"], None),
        (&["--require-auth"], Some("nope")),
        (&["--require-auth", "--terminal-method"], Some("stub-tty")),
        (&["--require-auth", "--no-methods"], Some("stub-key")),
        (&["--require-auth", "--reject-auth"], Some("stub-key")),
        (&["--require-auth", "--always-auth-required"], Some("stub-key")),
        (&["--require-auth", "--error-after-auth", "-32603"], Some("stub-key")),
        (&["--require-auth", "--require-env", "STUB_ACP_ABSENT_439"], Some("stub-key")),
        (&["--require-auth", "--flood"], None),
        (&["--require-auth", "--flood"], Some("stub-key")),
        (&["--fail-session-new", "-32603", "--flood"], Some("stub-key")),
    ];
    for (args, auth_method) in cases {
        let fx = Fixture::new();
        let mut config = fx.config(args, *auth_method);
        config.env = env_of("STUB_ACP_CREDENTIAL", &secret);
        let msg = spawn_err(&config).await;
        assert!(!msg.contains(&secret), "{args:?}: credential leaked into {msg}");
        assert!(!msg.contains(MARKER), "{args:?}: adapter description/data leaked into {msg}");
    }
}

fn longest_run(s: &str, c: char) -> usize {
    s.split(|x| x != c).map(|run| run.chars().count()).max().unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_hostile_advertised_list_and_rejection_text_are_capped() {
    let _serial = SERIAL.lock().await;
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--flood"], None)).await;
    assert_refused(&fx, &msg);
    assert!(msg.matches("filler-").count() <= 8, "at most 8 ids echoed: {msg}");
    assert!(msg.contains("(+93 more)"), "{msg}");
    assert!(longest_run(&msg, 'f') <= 64, "each id capped to 64 chars: {msg}");
    assert!(!msg.chars().any(char::is_control), "control characters replaced: {msg:?}");
    assert!(msg.chars().count() < 1200, "{} chars", msg.chars().count());

    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--require-auth", "--flood"], Some("stub-key"))).await;
    assert_eq!(fx.log(), ["initialize", "session/new", "authenticate stub-key"], "the 101st id is selectable");
    assert!(msg.matches('Q').count() <= 200, "adapter message capped to 200 chars: {msg}");
    assert!(msg.chars().count() < 1200, "{} chars", msg.chars().count());
}

/// Lines a subprocess wrote to stderr, drained on a background thread (the
/// `Hub` helper's pattern) so the pipe never backs up.
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

fn write_body_config(state: &StateDir, secret: &str) -> PathBuf {
    let argv = [STUB, "--require-auth", "--require-env", "STUB_ACP_CREDENTIAL"]
        .iter()
        .map(|a| serde_json::to_string(a).unwrap())
        .collect::<Vec<_>>()
        .join(", ");
    let toml = format!(
        "[[session]]\nname = \"alpha\"\nharness = \"opencode\"\ncommand = [{argv}]\n\
         auth_method = \"stub-key\"\nenv = {{ STUB_ACP_CREDENTIAL = {secret_q} }}\n",
        secret_q = serde_json::to_string(secret).unwrap(),
    );
    let path = state.body().join("sessions.toml");
    std::fs::create_dir_all(state.body()).unwrap();
    std::fs::write(&path, toml).unwrap();
    path
}

/// A real hub and a real `holler body run --debug noisy` whose session config
/// carries the credential in `env` and `auth_method = "stub-key"`: a `say`
/// is delivered, the driver logs an ACP `authenticate` event and then
/// `spawned protocol=v1`, and the body's stderr never contains the credential
/// or the adapter's description/data marker.
#[test]
fn body_debug_noisy_log_names_authenticate_but_never_the_credential() {
    let _serial = SERIAL.blocking_lock();
    let state = StateDir::new();
    let hub = Hub::start(&state);
    let (token_id, secret_tok) = mint_token(&state, "alpha");
    join(&state, &state, &hub.ws_url(), &token_id, &secret_tok);
    let secret = sentinel();
    let config = write_body_config(&state, &secret);

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
    // The hub handshake also logs a `circuit/authenticate` wire event, so a
    // substring match proves nothing: look for the ACP driver's own events.
    let acp_event = |kind: &str, extra: &dyn Fn(&serde_json::Value) -> bool| {
        lines.lock().unwrap().iter().any(|l| {
            serde_json::from_str::<serde_json::Value>(l).is_ok_and(|v| {
                v["component"] == "acp" && v["type"] == kind && extra(&v)
            })
        })
    };
    let spawned = wait_for(Duration::from_secs(5), || {
        acp_event("spawn", &|v| v["event"] == "spawned" && v["protocol"] == "v1").then_some(())
    });
    let authenticated = acp_event("authenticate", &|_| true);
    kill_tree(&mut body);
    let log = text();
    assert!(out.status.success(), "say failed: {}", String::from_utf8_lossy(&out.stderr));
    assert!(authenticated, "an acp `authenticate` event is logged at --debug noisy");
    assert!(spawned.is_some(), "the driver reached `spawned protocol=v1` after authenticating");
    assert!(!log.contains(&secret), "credential leaked into the body log");
    assert!(!log.contains(MARKER), "adapter description/data leaked into the body log");
    drop(hub);
}

// ---- criterion 11: timeout ---------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_slow_authenticate_ends_in_the_existing_startup_timeout() {
    let _serial = SERIAL.lock().await;
    let _timeout = EnvGuard::set("HOLLER_ACP_TIMEOUT_MS", "1500");
    let fx = Fixture::new();
    let started = std::time::Instant::now();
    let msg = spawn_err(&fx.config(&["--require-auth", "--auth-delay-ms", "30000"], Some("stub-key"))).await;
    assert!(started.elapsed() < Duration::from_secs(10), "bounded by the timeout, no hang");
    assert_eq!(fx.log(), ["initialize", "session/new", "authenticate stub-key"], "timed out inside authenticate: {msg}");
    assert!(msg.contains("HOLLER_ACP_TIMEOUT_MS"), "the existing timeout reason: {msg}");
    assert!(msg.contains("authenticate with auth_method \"stub-key\" was pending"), "says auth was pending: {msg}");
}

/// A hang in the first `session/new` never reached `authenticate`: the
/// timeout says the configured `auth_method` was not applied; without
/// `auth_method` the timeout text is unchanged.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v1_timeout_before_auth_reports_not_applied() {
    let _serial = SERIAL.lock().await;
    let _timeout = EnvGuard::set("HOLLER_ACP_TIMEOUT_MS", "1500");
    let fx = Fixture::new();
    let msg = spawn_err(&fx.config(&["--new-delay-ms", "30000"], Some("stub-key"))).await;
    assert!(msg.contains("HOLLER_ACP_TIMEOUT_MS") && msg.ends_with(V1_SUFFIX), "{msg}");

    let fx = Fixture::new();
    let bare = spawn_err(&fx.config(&["--new-delay-ms", "30000"], None)).await;
    assert!(bare.ends_with("v1 fallback)") && !bare.contains("auth_method"), "{bare}");
}
