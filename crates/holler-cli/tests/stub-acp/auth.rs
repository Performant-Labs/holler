//! `stub-acp`'s ACP v2 auth modes (issue #459), split out of `main.rs` to
//! keep that file under the workspace's 900-line guard (`scripts/lint.sh`),
//! following the `gates.rs` precedent. With no auth flag set every function
//! here is inert and the stub's wire behaviour is exactly what it was before.
//!
//! The flags mirror `stub-acp-v1`'s require-auth flags (same names where the
//! semantics match), with ACP v2's `auth/login` in place of v1's
//! `authenticate`:
//!
//! - `--auth-log <path>`: one flushed line per received `initialize`,
//!   `session/new` and `auth/login <methodId>`, opened in append mode, so a
//!   second launch against the same path (a v1 fallback relaunch) shows up
//!   as extra lines. Never a credential or a frame body.
//! - `--require-auth`: advertise `stub-key` and answer `session/new` with
//!   JSON-RPC `-32000` until an `auth/login` for `stub-key` succeeded.
//! - `--advertise-methods`: advertise `stub-key` without requiring it.
//! - `--terminal-method`: also advertise a terminal-type `stub-tty`.
//! - `--other-method`: also advertise `stub-custom` with an unknown `type`
//!   (`_stub_custom`), which the SDK deserializes as `AuthMethod::Other`.
//! - `--no-methods`: advertise nothing (overrides the three above).
//! - `--reject-auth`: answer every `auth/login` with an error.
//! - `--always-auth-required`: `auth/login` succeeds, `session/new` keeps
//!   answering `-32000`.
//! - `--require-env <NAME>`: accept `auth/login` only if `NAME` is set and
//!   non-empty in this process's own environment (never read beyond that
//!   check, never echoed).
//! - `--fail-session-new <code>`: every `session/new` fails with `<code>`
//!   (with `--flood`: a 5000-character message with a newline and an ANSI
//!   escape, plus hostile `data`).
//! - `--error-after-auth <code>`: after a successful `auth/login`, the next
//!   `session/new` fails with `<code>`.
//! - `--flood`: advertise 100 filler ids of 500 characters (each carrying an
//!   ANSI escape) plus `stub-key` as the 101st, and reject `auth/login` with
//!   a 5000-character message.
//! - `--auth-delay-ms <n>`: sleep `n` ms before answering `auth/login`.
//! - `--new-delay-ms <n>`: sleep `n` ms before answering every `session/new`.
//! - `--retry-delay-ms <n>`: sleep `n` ms before answering a `session/new`
//!   that arrives after a successful `auth/login` (the retry only).
//!
//! Every advertised method's `description`, and the `data` of every
//! auth-related error this module returns, carries [`HOSTILE_MARKER`], so a
//! test can assert that adapter-controlled description/data text never
//! reaches a driver error or a log.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

/// Planted in every method description and auth-error `data` (see module doc).
const HOSTILE_MARKER: &str = "HOSTILE-DESC-MARKER";
/// ACP's auth-required JSON-RPC code.
const AUTH_REQUIRED: i64 = -32000;

#[allow(clippy::struct_excessive_bools)] // #459 one independent test flag per field
struct AuthConfig {
    auth_log: Option<String>,
    require_auth: bool,
    advertise_methods: bool,
    terminal_method: bool,
    other_method: bool,
    no_methods: bool,
    reject_auth: bool,
    always_auth_required: bool,
    require_env: Option<String>,
    fail_session_new: Option<i64>,
    error_after_auth: Option<i64>,
    flood: bool,
    auth_delay_ms: u64,
    new_delay_ms: u64,
    retry_delay_ms: u64,
}

static CONFIG: Mutex<AuthConfig> = Mutex::new(AuthConfig {
    auth_log: None,
    require_auth: false,
    advertise_methods: false,
    terminal_method: false,
    other_method: false,
    no_methods: false,
    reject_auth: false,
    always_auth_required: false,
    require_env: None,
    fail_session_new: None,
    error_after_auth: None,
    flood: false,
    auth_delay_ms: 0,
    new_delay_ms: 0,
    retry_delay_ms: 0,
});
/// Set once an `auth/login` for `stub-key` succeeded (one process, one session).
static AUTHENTICATED: AtomicBool = AtomicBool::new(false);
static LOG: Mutex<Option<File>> = Mutex::new(None);

fn config() -> std::sync::MutexGuard<'static, AuthConfig> {
    CONFIG.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Consume one auth flag from `main`'s argv scan. Returns how many extra
/// arguments (the flag's value) were consumed; an unknown flag consumes none
/// and is ignored, as before.
pub(crate) fn parse_flag(flag: &str, next: Option<&String>) -> usize {
    let mut cfg = config();
    let num = || next.and_then(|v| v.parse::<i64>().ok());
    let ms = || next.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    match flag {
        "--auth-log" => cfg.auth_log = next.cloned(),
        "--require-env" => cfg.require_env = next.cloned(),
        "--fail-session-new" => cfg.fail_session_new = num(),
        "--error-after-auth" => cfg.error_after_auth = num(),
        "--auth-delay-ms" => cfg.auth_delay_ms = ms(),
        "--new-delay-ms" => cfg.new_delay_ms = ms(),
        "--retry-delay-ms" => cfg.retry_delay_ms = ms(),
        _ => {
            match flag {
                "--require-auth" => cfg.require_auth = true,
                "--advertise-methods" => cfg.advertise_methods = true,
                "--terminal-method" => cfg.terminal_method = true,
                "--other-method" => cfg.other_method = true,
                "--no-methods" => cfg.no_methods = true,
                "--reject-auth" => cfg.reject_auth = true,
                "--always-auth-required" => cfg.always_auth_required = true,
                "--flood" => cfg.flood = true,
                _ => {}
            }
            return 0;
        }
    }
    1
}

fn log_line(text: &str) {
    let path = config().auth_log.clone();
    let Some(path) = path else { return };
    let mut log = LOG.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if log.is_none() {
        *log = OpenOptions::new().create(true).append(true).open(path).ok();
    }
    if let Some(file) = log.as_mut() {
        let _ = writeln!(file, "{text}");
        let _ = file.flush();
    }
}

fn write_error(lock: &mut impl Write, id: &Value, code: i64, message: &str, data: Option<Value>) {
    let mut error = json!({ "code": code, "message": message });
    if let Some(data) = data {
        error["data"] = data;
    }
    let msg = json!({ "jsonrpc": "2.0", "id": id, "error": error });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}

fn hostile_data() -> Value {
    json!({ "hint": HOSTILE_MARKER })
}

fn method(kind: &str, id: &str) -> Value {
    json!({
        "type": kind, "methodId": id, "name": "Stub method",
        "description": format!("set the key {HOSTILE_MARKER}"),
    })
}

/// The `authMethods` this launch advertises in its `initialize` result.
fn auth_methods(cfg: &AuthConfig) -> Vec<Value> {
    if cfg.no_methods {
        return Vec::new();
    }
    let mut methods = Vec::new();
    if cfg.flood {
        for n in 0..100 {
            methods.push(method("agent", &format!("filler-{n:03}-\u{1b}[31m{}", "f".repeat(500 - 16))));
        }
    }
    if cfg.require_auth || cfg.advertise_methods || cfg.flood {
        methods.push(method("agent", "stub-key"));
    }
    if cfg.terminal_method {
        methods.push(method("terminal", "stub-tty"));
    }
    if cfg.other_method {
        methods.push(method("_stub_custom", "stub-custom"));
    }
    methods
}

/// Log the `initialize` and add any advertised `authMethods` to `result`
/// (left untouched when this launch advertises none).
pub(crate) fn initialize_result(mut result: Value) -> Value {
    log_line("initialize");
    let methods = auth_methods(&config());
    if !methods.is_empty() {
        result["authMethods"] = Value::Array(methods);
    }
    result
}

/// `session/new`: log it, then answer it with an error per the flags.
/// Returns `false` when the caller should answer it as before (success).
pub(crate) fn session_new(lock: &mut impl Write, id: &Value) -> bool {
    log_line("session/new");
    let cfg = config();
    let authenticated = AUTHENTICATED.load(Ordering::SeqCst);
    let delay = if authenticated && cfg.retry_delay_ms > 0 { cfg.retry_delay_ms } else { cfg.new_delay_ms };
    if delay > 0 {
        std::thread::sleep(Duration::from_millis(delay));
    }
    if let Some(code) = cfg.fail_session_new {
        if cfg.flood {
            let message = format!("line one\n\u{1b}[31m{}", "Q".repeat(5000));
            write_error(lock, id, code, &message, Some(hostile_data()));
        } else {
            write_error(lock, id, code, "stub session/new failure", None);
        }
    } else if cfg.require_auth && (!authenticated || cfg.always_auth_required) {
        write_error(lock, id, AUTH_REQUIRED, "Authentication required", Some(hostile_data()));
    } else if let (true, Some(code)) = (authenticated, cfg.error_after_auth) {
        write_error(lock, id, code, "stub failure after auth/login", None);
    } else {
        return false;
    }
    true
}

/// `auth/login`: log the method id, then accept or reject per the flags.
pub(crate) fn login(lock: &mut impl Write, msg: &Value) {
    let method_id = msg.pointer("/params/methodId").and_then(Value::as_str).unwrap_or_default();
    log_line(&format!("auth/login {method_id}"));
    let cfg = config();
    if cfg.auth_delay_ms > 0 {
        std::thread::sleep(Duration::from_millis(cfg.auth_delay_ms));
    }
    let Some(id) = msg.get("id") else { return };
    let env_ok = cfg
        .require_env
        .as_ref()
        .is_none_or(|name| std::env::var(name).is_ok_and(|v| !v.is_empty()));
    if cfg.flood {
        write_error(lock, id, -32603, &"Q".repeat(5000), Some(hostile_data()));
    } else if cfg.reject_auth {
        write_error(lock, id, -32603, "stub rejected auth/login", Some(hostile_data()));
    } else if !env_ok {
        write_error(lock, id, -32602, "stub credential missing from environment", Some(hostile_data()));
    } else if method_id != "stub-key" {
        write_error(lock, id, -32602, "stub unknown auth method", Some(hostile_data()));
    } else {
        AUTHENTICATED.store(true, Ordering::SeqCst);
        let ok = json!({ "jsonrpc": "2.0", "id": id, "result": {} });
        let _ = writeln!(lock, "{ok}");
        let _ = lock.flush();
    }
}
