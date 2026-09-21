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

use std::io::{BufRead, BufReader, Write};

use serde_json::{json, Value};

const SESSION_ID: &str = "stub-v1";

fn write_line(value: &Value) {
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{value}");
    let _ = stdout.flush();
}

fn respond(id: &Value, result: Value) {
    write_line(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
}

fn notify(method: &str, params: Value) {
    write_line(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
}

fn main() {
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
                if let Some(id) = &id {
                    // Every real ACP peer's own `initialize` result (see
                    // #362's writeup) — protocol v1, nothing else required.
                    respond(id, json!({ "protocolVersion": 1 }));
                }
            }
            "session/new" => {
                if let Some(id) = &id {
                    respond(id, json!({ "sessionId": SESSION_ID }));
                }
            }
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
