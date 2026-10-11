#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #362
//! `stub-acp-v1` — a minimal, deterministic ACP **v1** agent stub (issue
//! #362's v1-compatibility fallback).
//!
//! Every real ACP implementation checked while investigating #362 (the
//! newest `opencode acp`, the newest published `@agentclientprotocol/sdk`)
//! negotiates protocol v1, never v2 — `stub-acp` (this crate's other stub)
//! deliberately claims v2 unconditionally and so can never exercise
//! `AcpDriver`'s v1 fallback path at all. This stub exists purely to give
//! that fallback a real (if minimal) v1 peer to talk to in a test: it only
//! implements exactly what one `initialize` → `session/new` → `session/prompt`
//! round trip needs — one chunk, then `end_turn` — not the full turn/
//! permission/cancel repertoire `stub-acp` covers for v2. `AcpDriver`'s v1
//! permission-request handling is exercised at the unit level instead (its
//! wire shape and reply path are a direct parallel of the already-tested v2
//! code — see `pending.rs`'s `PermissionV1` variant).
//!
//! Same process constraints as `stub-acp`: no tokio, newline-delimited JSON
//! on stdout, diagnostics on stderr, blocking `std::io` line loop.
//!
//! # Auth modes (issue #439)
//!
//! With no flags the stub behaves exactly as before (no `authMethods`, every
//! `session/new` succeeds). The flags below turn it into an adapter that
//! requires the ACP v1 `authenticate` request, for `acp_driver_auth_test.rs`:
//!
//! - `--auth-log <path>`: one flushed line per received `initialize`,
//!   `session/new` and `authenticate <methodId>`, **only in the launch that
//!   speaks v1**: an `initialize` whose `protocolVersion` is not 1 (the
//!   driver's discarded v2 attempt) is not logged; the first v1 `initialize`
//!   creates/truncates the file. Never a credential or a frame body.
//! - `--require-auth`: advertise `stub-key` and answer `session/new` with
//!   JSON-RPC `-32000` until an `authenticate` for `stub-key` succeeded.
//! - `--advertise-methods`: advertise `stub-key` without requiring it.
//! - `--terminal-method`: also advertise a terminal-type `stub-tty`.
//! - `--no-methods`: advertise nothing (overrides the two above).
//! - `--reject-auth`: answer every `authenticate` with an error.
//! - `--always-auth-required`: `authenticate` succeeds, `session/new` keeps
//!   answering `-32000`.
//! - `--require-env <NAME>`: accept `authenticate` only if `NAME` is set and
//!   non-empty in this process's own environment (the value is never read
//!   beyond that check, never echoed).
//! - `--fail-session-new <code>`: every `session/new` fails with `<code>`
//!   (with `--flood`: a 5000-character message with a newline and an ANSI
//!   escape, plus hostile `data`).
//! - `--error-after-auth <code>`: after a successful `authenticate`, the next
//!   `session/new` fails with `<code>`.
//! - `--flood`: advertise 100 filler ids of 500 characters (each carrying an
//!   ANSI escape) plus `stub-key` as the 101st, and reject `authenticate`
//!   with a 5000-character message.
//! - `--auth-delay-ms <n>`: sleep `n` ms before answering `authenticate`.
//! - `--new-delay-ms <n>`: sleep `n` ms before answering `session/new`.
//!
//! Every advertised method's `description`, and the `data` of every
//! auth-related error this stub returns, carries [`HOSTILE_MARKER`], so a
//! test can assert that adapter-controlled description/data text never
//! reaches a driver error or a log.

use std::io::{BufRead, BufReader, Write};

use serde_json::{json, Value};

const SESSION_ID: &str = "stub-v1";
/// Planted in every method description and auth-error `data` (see module doc).
const HOSTILE_MARKER: &str = "HOSTILE-DESC-MARKER";

#[derive(Default)]
#[allow(clippy::struct_excessive_bools)] // #439 one independent test flag per field
struct Config {
    auth_log: Option<String>,
    require_auth: bool,
    advertise_methods: bool,
    terminal_method: bool,
    no_methods: bool,
    reject_auth: bool,
    always_auth_required: bool,
    require_env: Option<String>,
    fail_session_new: Option<i64>,
    error_after_auth: Option<i64>,
    flood: bool,
    auth_delay_ms: u64,
    new_delay_ms: u64,
}

fn parse_args() -> Config {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg = Config::default();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        let next = args.get(i + 1).cloned();
        let mut value = || {
            i += 1;
            next.clone()
        };
        match flag {
            "--auth-log" => cfg.auth_log = value(),
            "--require-auth" => cfg.require_auth = true,
            "--advertise-methods" => cfg.advertise_methods = true,
            "--terminal-method" => cfg.terminal_method = true,
            "--no-methods" => cfg.no_methods = true,
            "--reject-auth" => cfg.reject_auth = true,
            "--always-auth-required" => cfg.always_auth_required = true,
            "--require-env" => cfg.require_env = value(),
            "--fail-session-new" => cfg.fail_session_new = value().and_then(|v| v.parse().ok()),
            "--error-after-auth" => cfg.error_after_auth = value().and_then(|v| v.parse().ok()),
            "--flood" => cfg.flood = true,
            "--auth-delay-ms" => cfg.auth_delay_ms = value().and_then(|v| v.parse().ok()).unwrap_or(0),
            "--new-delay-ms" => cfg.new_delay_ms = value().and_then(|v| v.parse().ok()).unwrap_or(0),
            // Unknown / positional args are ignored.
            _ => {}
        }
        i += 1;
    }
    cfg
}

fn write_line(value: &Value) {
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{value}");
    let _ = stdout.flush();
}

fn respond(id: &Value, result: Value) {
    write_line(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
}

fn respond_error(id: &Value, code: i64, message: &str, data: Option<Value>) {
    let mut error = json!({ "code": code, "message": message });
    if let Some(data) = data {
        error["data"] = data;
    }
    write_line(&json!({ "jsonrpc": "2.0", "id": id, "error": error }));
}

fn notify(method: &str, params: Value) {
    write_line(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
}

fn hostile_data() -> Value {
    json!({ "hint": HOSTILE_MARKER })
}

fn agent_method(id: &str) -> Value {
    json!({ "id": id, "name": "Stub method", "description": format!("set the key {HOSTILE_MARKER}") })
}

/// The `authMethods` this launch advertises in its `initialize` result.
fn auth_methods(cfg: &Config) -> Vec<Value> {
    if cfg.no_methods {
        return Vec::new();
    }
    let mut methods = Vec::new();
    if cfg.flood {
        for n in 0..100 {
            let id = format!("filler-{n:03}-\u{1b}[31m{}", "f".repeat(500 - 16));
            methods.push(agent_method(&id));
        }
    }
    if cfg.require_auth || cfg.advertise_methods || cfg.flood {
        methods.push(agent_method("stub-key"));
    }
    if cfg.terminal_method {
        methods.push(json!({
            "type": "terminal", "id": "stub-tty", "name": "Stub tty",
            "description": format!("run a login {HOSTILE_MARKER}"),
        }));
    }
    methods
}

/// The v1-launch-only auth log (see the module doc). `None` until this
/// process has received a v1 `initialize`.
struct AuthLog {
    path: Option<String>,
    file: Option<std::fs::File>,
}

impl AuthLog {
    fn on_initialize(&mut self, protocol_version: Option<i64>) {
        if protocol_version != Some(1) {
            return;
        }
        if self.file.is_none() {
            if let Some(path) = &self.path {
                self.file = std::fs::File::create(path).ok();
            }
        }
        self.line("initialize");
    }

    fn line(&mut self, text: &str) {
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "{text}");
            let _ = file.flush();
        }
    }
}

/// `authenticate` (#439): log the method id, then accept or reject per flags.
fn authenticate(cfg: &Config, log: &mut AuthLog, msg: &Value, id: Option<&Value>, authenticated: &mut bool) {
    let method_id = msg.pointer("/params/methodId").and_then(Value::as_str).unwrap_or_default();
    log.line(&format!("authenticate {method_id}"));
    if cfg.auth_delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(cfg.auth_delay_ms));
    }
    let Some(id) = id else { return };
    let env_ok = cfg
        .require_env
        .as_ref()
        .is_none_or(|name| std::env::var(name).is_ok_and(|v| !v.is_empty()));
    if cfg.flood {
        respond_error(id, -32603, &"Q".repeat(5000), Some(hostile_data()));
    } else if cfg.reject_auth {
        respond_error(id, -32603, "stub rejected authenticate", Some(hostile_data()));
    } else if !env_ok {
        respond_error(id, -32602, "stub credential missing from environment", Some(hostile_data()));
    } else if method_id != "stub-key" {
        respond_error(id, -32602, "stub unknown auth method", Some(hostile_data()));
    } else {
        *authenticated = true;
        respond(id, json!({}));
    }
}

/// `session/new`: fail per flags (#439), else open the one stub session.
fn session_new(cfg: &Config, log: &mut AuthLog, id: Option<&Value>, authenticated: bool) {
    log.line("session/new");
    if cfg.new_delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(cfg.new_delay_ms));
    }
    let Some(id) = id else { return };
    if let Some(code) = cfg.fail_session_new {
        if cfg.flood {
            let message = format!("line one\n\u{1b}[31m{}", "Q".repeat(5000));
            respond_error(id, code, &message, Some(hostile_data()));
        } else {
            respond_error(id, code, "stub session/new failure", None);
        }
    } else if cfg.require_auth && (!authenticated || cfg.always_auth_required) {
        respond_error(id, -32000, "Authentication required", Some(hostile_data()));
    } else if let (true, Some(code)) = (authenticated, cfg.error_after_auth) {
        respond_error(id, code, "stub failure after authenticate", None);
    } else {
        respond(id, json!({ "sessionId": SESSION_ID }));
    }
}

fn main() {
    let cfg = parse_args();
    let mut log = AuthLog { path: cfg.auth_log.clone(), file: None };
    let mut authenticated = false;
    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let method = msg.get("method").and_then(Value::as_str).unwrap_or_default();
        let id = msg.get("id").cloned();

        match method {
            "initialize" => {
                let version = msg.pointer("/params/protocolVersion").and_then(Value::as_i64);
                log.on_initialize(version);
                if let Some(id) = &id {
                    // Every real ACP peer's own `initialize` result (see
                    // #362's writeup) — protocol v1, plus any advertised
                    // auth methods (#439).
                    let methods = auth_methods(&cfg);
                    if methods.is_empty() {
                        respond(id, json!({ "protocolVersion": 1 }));
                    } else {
                        respond(id, json!({ "protocolVersion": 1, "authMethods": methods }));
                    }
                }
            }
            "authenticate" => authenticate(&cfg, &mut log, &msg, id.as_ref(), &mut authenticated),
            "session/new" => session_new(&cfg, &mut log, id.as_ref(), authenticated),
            "session/prompt" => {
                // One chunk, then a terminal `end_turn` response — the
                // minimal shape `AcpDriver::prompt`'s v1 branch needs to
                // observe both a `Chunk` and a `Done(EndTurn)`.
                notify(
                    "session/update",
                    json!({
                        "sessionId": SESSION_ID,
                        "update": {
                            "sessionUpdate": "agent_message_chunk",
                            "content": { "type": "text", "text": "hello from v1" },
                        },
                    }),
                );
                if let Some(id) = &id {
                    respond(id, json!({ "stopReason": "end_turn" }));
                }
            }
            "session/cancel" => {
                // Notification, no response expected; this stub has nothing
                // in flight to cancel (session/prompt above always resolves
                // synchronously before the next line is even read).
            }
            "session/close" => {
                // `AcpDriver::shutdown` always sends this and awaits the
                // response with no timeout of its own (the real `stub-acp`
                // always answers it too) — omitting this handler hangs every
                // test in this file forever, not just the one calling
                // `shutdown()` directly.
                if let Some(id) = &id {
                    respond(id, json!({}));
                }
            }
            _ => {}
        }
    }
}
