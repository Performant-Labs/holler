#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! `stub-acp` — a deterministic ACP **v2** agent stub (story #130).
//!
//! The test suite spawns this binary instead of a real agent. It is a real
//! process speaking JSON-RPC 2.0 over stdio (newline-delimited JSON, one
//! message per line). It is fully deterministic, cross-OS, and — unlike a
//! model — can simulate a permission request so the `blocked` state is
//! testable without a model.
//!
//! Constraints (see the issue): **no tokio** — a blocking `std::io` line
//! loop plus one worker thread; **no shell**; stdout carries *only* JSON
//! lines (diagnostics go to stderr).
//!
//! Concurrency model: the main thread reads stdin and forwards every inbound
//! message to a single worker over an `mpsc` channel. The worker is the
//! **sole writer** to stdout — it answers `initialize`/`session/new`
//! inline, streams `session/update` notifications for an in-flight prompt,
//! and resolves the prompt (or a cancel) — all behind one stdout lock, so
//! the JSON-line stream is serial and correctly ordered. `--slow` stretches
//! the inter-chunk gap so a `session/cancel` can land mid-turn.
//!
//! Method names below are ACP v2 as shipped in `agent-client-protocol` 2.1.0
//! (`session/request_permission`, `session/update`, …). If a name here stops
//! matching the schema, fix it here and note it in the story.

use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

/// ACP v2 protocol version (the `initialize` negotiation constant).
const PROTOCOL_VERSION: u8 = 2;
/// Every session the stub creates is addressed as this.
const SESSION_ID: &str = "stub";
/// Synthetic id for the `session/request_permission` request raised under
/// `--ask-permission`; the client's matching response resumes the turn.
const PERMISSION_REQUEST_ID: i64 = 10;
/// JSON-RPC "Method not found" code (for unknown inbound methods).
const ERROR_METHOD_NOT_FOUND: i64 = -32601;

#[derive(Clone)]
struct Config {
    /// Advertised session names (comma-separated). Parsed but not otherwise
    /// used by the wire behaviour: `session/new` always returns `SESSION_ID`.
    // Forward-contract stub path: exercised by a later story (the file-level
    // `dead_code` allow covers this — see the `// #149` blanket at the top).
    sessions: String,
    /// Number of `agent_message_chunk` notifications emitted per prompt.
    chunks: usize,
    /// Inter-chunk delay in ms: ~50 normally, 200 under `--slow` so a
    /// `session/cancel` can land mid-turn.
    chunk_delay_ms: u64,
    ask_permission: bool,
    crash_after_prompt: bool,
}

fn main() {
    let cfg = parse_args();
    let (tx, rx) = mpsc::channel::<Value>();
    let cancel: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));

    // The worker is the sole stdout writer; it is detached and drops when
    // the process exits (after the main thread returns on stdin EOF).
    thread::spawn(move || worker_loop(&cfg, rx, cancel));

    // The main thread only reads stdin and forwards; it never writes stdout,
    // so it cannot deadlock against the worker's stdout lock.
    let stdin = std::io::stdin();
    for line in BufReader::new(stdin).lines() {
        // NB: we do NOT break on the cancel flag here. The main thread keeps
        // reading stdin until EOF so the worker can receive the cancel message
        // and resolve the in-flight prompt. The flag is only consulted by the
        // worker's `emit_turn` (to abort mid-turn) and is cleared by the
        // worker once the turn resolves.
        let line = match line {
            Ok(line) => line,
            Err(_) => break, // EOF / closed stdin
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(m) => {
                // Raise shared flags *now* so the worker's in-flight
                // `emit_turn` (which polls them) reacts immediately, even
                // though the worker is still busy and hasn't `recv`'d this
                // message yet.
                match m.get("method").and_then(Value::as_str) {
                    Some("session/cancel") => cancel.store(true, Ordering::SeqCst),
                    None if m.get("id").and_then(Value::as_i64) == Some(PERMISSION_REQUEST_ID) => {
                        PERMISSION_ANSWERED.store(true, Ordering::SeqCst);
                    }
                    _ => {}
                }
                if tx.send(m).is_err() {
                    // The worker dropped (process exiting): stop feeding.
                    break;
                }
            }
            Err(e) => {
                eprintln!("stub-acp: ignoring malformed line: {e}");
            }
        }
    }
    // EOF: main returns (exit 0); the detached worker drops with the process.
}

/// Parse argv with a tiny `--flag` / `--flag value` scanner (no CLI lib, so
/// the stub stays a single file).
fn parse_args() -> Config {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut cfg = Config {
        sessions: "alpha,beta".into(),
        chunks: 3,
        chunk_delay_ms: 50,
        ask_permission: false,
        crash_after_prompt: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--sessions" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    cfg.sessions = v.clone();
                }
            }
            "--chunks" => {
                i += 1;
                cfg.chunks = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(3);
            }
            "--slow" => cfg.chunk_delay_ms = 200,
            "--ask-permission" => cfg.ask_permission = true,
            "--crash-after-prompt" => cfg.crash_after_prompt = true,
            // Unknown / positional args are ignored.
            _ => {}
        }
        i += 1;
    }
    cfg
}

/// The sole stdout writer. For each inbound message it drives the state
/// machine: answers `initialize`/`session/new` immediately, routes
/// `session/prompt` into an in-flight turn, and handles `session/cancel`.
/// All writes go through one `stdout.lock()`, so the stream is serial.
fn worker_loop(cfg: &Config, rx: Receiver<Value>, cancel: &'static AtomicBool) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();

    let mut prompt: Option<Value> = None;
    loop {
        let msg = match rx.recv() {
            Ok(m) => m,
            Err(_) => break, // main thread exited (EOF) → channel closed
        };

        let method = msg.get("method").and_then(Value::as_str);
        let id = msg.get("id").cloned();

        match method {
            Some("initialize") => {
                if let Some(id) = id {
                    send_response(
                        &mut lock,
                        id,
                        json!({ "protocolVersion": PROTOCOL_VERSION, "agentCapabilities": {} }),
                    );
                }
            }
            Some("session/new") => {
                if let Some(id) = id {
                    send_response(&mut lock, id, json!({ "sessionId": SESSION_ID }));
                }
            }
            Some("session/prompt") => {
                // One in-flight turn at a time (the tests use one prompt per
                // stub). A new prompt supersedes any still-pending one.
                //
                // Clear the cancel flag so a cancel that was delivered (or is
                // still being resolved) for the *previous* turn does not leak
                // into this new turn. The main thread only ever raises the
                // flag on a genuine `session/cancel`, so if a fresh cancel
                // arrives for *this* turn it will re-set the flag before
                // `emit_turn` polls it.
                cancel.store(false, Ordering::SeqCst);
                if let Some(id) = id {
                    prompt = Some(id);
                }
            }
            Some("session/cancel") => {
                // Stop the in-flight turn (its prompt resolves to `cancelled`,
                // handled by `emit_turn` below) and acknowledge the cancel.
                cancel.store(true, Ordering::SeqCst);
                if let Some(id) = id {
                    send_response(&mut lock, id, json!({}));
                }
            }
            Some(_) => {
                // A method the stub does not implement: -32601, keep running.
                if let Some(id) = id {
                    send_error(&mut lock, id, ERROR_METHOD_NOT_FOUND, "method not found");
                }
            }
            // No `method` → a response. Only the permission answer matters:
            // it unblocks a turn that was awaiting `--ask-permission`.
            None => {
                if msg.get("id").and_then(Value::as_i64) == Some(PERMISSION_REQUEST_ID) {
                    PERMISSION_ANSWERED.store(true, Ordering::SeqCst);
                }
            }
        }

        // Advance a pending turn. `emit_turn` returns `true` once the turn is
        // resolved (a response was sent); `false` when it is blocked on the
        // permission answer and must be resumed by a later inbound message.
        if let Some(id) = prompt.clone() {
            if emit_turn(cfg, &mut lock, &id, cancel) {
                prompt = None;
                cancel.store(false, Ordering::SeqCst);
            }
        }
    }
}

/// Drive one turn's `session/update` emissions on the given stdout lock.
/// Returns `true` when the turn is fully resolved (a response was sent);
/// `false` when the turn is blocked waiting for the `--ask-permission` answer
/// and must be resumed by a later inbound message.
fn emit_turn(cfg: &Config, lock: &mut impl Write, prompt_id: &Value, cancel: &AtomicBool) -> bool {
    // state_update: running
    send_notification(
        lock,
        json!({
            "sessionId": SESSION_ID,
            "update": { "sessionUpdate": "state_update", "state": "running" }
        }),
    );

    let mut emitted = 0usize;
    loop {
        if cancel.load(Ordering::SeqCst) {
            // A cancel landed mid-turn: stop emitting and report `cancelled`.
            send_response(
                lock,
                prompt_id.clone(),
                json!({ "stopReason": "cancelled" }),
            );
            return true;
        }

        if emitted >= cfg.chunks {
            send_response(lock, prompt_id.clone(), json!({ "stopReason": "end_turn" }));
            return true;
        }

        // agent_message_chunk <emitted> (0-based)
        send_notification(
            lock,
            json!({
                "sessionId": SESSION_ID,
                "update": {
                    "sessionUpdate": "agent_message_chunk",
                    "content": { "type": "text", "text": format!("stub chunk {emitted}") }
                }
            }),
        );
        emitted += 1;

        // Permission gate: after the first chunk, raise a request and report
        // `requires_action`; the turn stays pending until the client answers.
        if cfg.ask_permission && emitted == 1 {
            send_request(
                lock,
                PERMISSION_REQUEST_ID,
                "session/request_permission",
                json!({
                    "sessionId": SESSION_ID,
                    "toolCall": { "title": "stub tool" },
                    "options": [
                        { "optionId": "allow", "kind": "allow_once" },
                        { "optionId": "deny",  "kind": "reject_once" }
                    ]
                }),
            );
            send_notification(
                lock,
                json!({
                    "sessionId": SESSION_ID,
                    "update": { "sessionUpdate": "state_update", "state": "requires_action" }
                }),
            );
            let _ = lock.flush();
            // Block until the client answers the permission request. The main
            // thread raises PERMISSION_ANSWERED when it forwards the id-10
            // response; the gate wakes on that (or on a cancel).
            let cancelled = wait_for_permission(cancel);
            if cancelled {
                // Cancelled while blocked: report cancelled and finish.
                send_response(
                    lock,
                    prompt_id.clone(),
                    json!({ "stopReason": "cancelled" }),
                );
                return true;
            }
            // Resumed: back to running, then continue emitting remaining chunks.
            send_notification(
                lock,
                json!({
                    "sessionId": SESSION_ID,
                    "update": { "sessionUpdate": "state_update", "state": "running" }
                }),
            );
            continue;
        }

        // Crash simulation for driver-error tests.
        if cfg.crash_after_prompt {
            eprintln!("stub-acp: --crash-after-prompt: exiting mid-turn");
            std::process::exit(1);
        }

        // ~50ms (or 200ms under --slow) between chunks.
        thread::sleep(Duration::from_millis(cfg.chunk_delay_ms));
    }
}

/// Block (polling) while a `--ask-permission` gate is open, returning `true`
/// if the turn was cancelled in the meantime. The main thread raises
/// `PERMISSION_ANSWERED` when it forwards the id-10 response; the gate wakes
/// on that flag or on a cancel.
fn wait_for_permission(cancel: &AtomicBool) -> bool {
    loop {
        if cancel.load(Ordering::SeqCst) {
            return true;
        }
        // Clear-and-check the permission-answered flag (raised by the main
        // thread when the id-10 response is forwarded).
        if PERMISSION_ANSWERED.swap(false, Ordering::SeqCst) {
            return false;
        }
        // A very short sleep keeps the gate responsive and keeps CPU ~0.
        thread::sleep(Duration::from_millis(5));
    }
}

/// Shared flag the main thread raises when it forwards the id-10 (permission)
/// response, unblocking the worker's `wait_for_permission` gate.
static PERMISSION_ANSWERED: AtomicBool = AtomicBool::new(false);

/// Write a JSON-RPC response (id + result) to stdout as one line.
fn send_response(lock: &mut impl Write, id: Value, result: Value) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "result": result });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}

/// Write a JSON-RPC error (id + error) to stdout as one line.
fn send_error(lock: &mut impl Write, id: Value, code: i64, message: &str) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}

/// Write a `session/update` notification (no id) to stdout as one line.
fn send_notification(lock: &mut impl Write, params: Value) {
    let msg = json!({ "jsonrpc": "2.0", "method": "session/update", "params": params });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}

/// Write an outbound JSON-RPC request (the permission ask) as one line.
fn send_request(lock: &mut impl Write, id: i64, method: &str, params: Value) {
    let msg = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
    let _ = writeln!(lock, "{msg}");
    let _ = lock.flush();
}
