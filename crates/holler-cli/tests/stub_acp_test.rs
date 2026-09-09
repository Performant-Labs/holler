#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #149
//! Integration tests for the ACP v2 stub agent (story #130, updated in #188).
//!
//! These pin the stub's *observable* ACP v2 behaviour — the surface a real ACP
//! client (and later the body's ACP-driver story) talks to. The driver (spawn,
//! read, write, close) is the shared `tests/support` `Stub` — the *same* driver
//! the ACP-driver story's tests will use — so the stub is verified through
//! exactly the client-side path the body will later drive. (This file used to
//! re-implement that driver by hand, keeping a second copy of the same logic in
//! the tree — see issue #147.)
//!
//! Request lines and the fixed ids are the shared `support` constants
//! (`INITIALIZE`, `SESSION_NEW`, `PROMPT`, `CANCEL`, `UNKNOWN`), so the driver
//! and these tests agree on the wire by construction.
//!
//! # Wire contract change (issue #188)
//!
//! The real `agent-client-protocol` 2.1.0 crate's `v2::PromptResponse` carries
//! no `stopReason` field at all — a `session/prompt` response only means
//! "accepted"; turn completion is reported independently through an `idle`
//! `state_update` notification. So the stub now acks `session/prompt`
//! **immediately** (an empty `{}` result on id "3", before any notification
//! streams), and every test below reads that ack first, then the notification
//! stream, rather than treating the id-"3" response as the turn's terminal
//! step the way the original (v1-shaped) design did.

mod support;

use support::Stub;
use support::{CANCEL, PERMISSION_REQUEST_ID, PROMPT, UNKNOWN};

/// Spawn `stub-acp` with `args` and piped stdio via the shared driver.
/// (`stderr` is nulled inside [`Stub::start`]: the stub writes only the
/// JSON-RPC stream to stdout, so there is nothing a test needs from stderr.)
fn spawn_stub(args: &[&str]) -> Stub {
    Stub::start(args)
}

/// Send [`PROMPT`] and assert its immediate accept-ack (id "3", empty result).
fn send_prompt_and_ack(stub: &mut Stub) {
    stub.send(PROMPT);
    let ack = stub.read_response("3");
    assert_eq!(ack["result"], serde_json::json!({}), "{ack}");
}

#[test]
fn prompt_streams_chunks_then_end_turn() {
    let mut stub = spawn_stub(&["--chunks", "3"]);
    stub.handshake();

    send_prompt_and_ack(&mut stub);

    // First streamed update is the state transition to running. (The
    // JSON-RPC notification carries `params`; sessionId/update live there.)
    let u1 = stub.read_line();
    assert_eq!(u1["params"]["sessionId"], "stub");
    assert_eq!(u1["params"]["update"]["sessionUpdate"], "state_update");
    assert_eq!(u1["params"]["update"]["state"], "running");

    // Then three text chunks, streamed (no response interleaves them).
    for i in 0..3 {
        let u = stub.read_line();
        assert_eq!(
            u["params"]["update"]["sessionUpdate"],
            "agent_message_chunk"
        );
        assert_eq!(u["params"]["update"]["content"]["type"], "text");
        assert_eq!(
            u["params"]["update"]["content"]["text"],
            format!("stub chunk {i}")
        );
        assert_eq!(u["params"]["update"]["messageId"], "stub-message");
    }

    // Finally the `idle` state_update — the sole real-ACP-v2 signal of
    // completion — carries `stopReason`.
    let idle = stub.read_line();
    assert_eq!(idle["params"]["update"]["state"], "idle");
    assert_eq!(idle["params"]["update"]["stopReason"], "end_turn");

    stub.close();
}

#[test]
fn cancel_yields_cancelled_idle_state_and_stub_survives() {
    let mut stub = spawn_stub(&["--slow"]);
    stub.handshake();

    send_prompt_and_ack(&mut stub);
    // At least one notification (running, or a chunk) lands before the cancel.
    stub.read_line();
    // Cancel mid-turn.
    stub.send(CANCEL);
    // The in-flight turn resolves via an `idle` state_update carrying the
    // cancelled stop reason (there is no second response to the already-acked
    // prompt id)...
    let idle = stub.read_line();
    assert_eq!(idle["params"]["update"]["state"], "idle");
    assert_eq!(
        idle["params"]["update"]["stopReason"], "cancelled",
        "{idle}"
    );
    // ...and the stub answers the cancel notification itself with {}.
    let cancel_resp = stub.read_response("4");
    assert_eq!(cancel_resp["result"], serde_json::json!({}));

    // The stub survives: a subsequent prompt still completes normally
    // (default `--chunks 3`).
    send_prompt_and_ack(&mut stub);
    let _ = stub.read_line(); // running
    for _ in 0..3 {
        let _ = stub.read_line(); // chunk 0/1/2
    }
    let idle2 = stub.read_line();
    assert_eq!(
        idle2["params"]["update"]["stopReason"], "end_turn",
        "{idle2}"
    );

    stub.close();
}

/// Assert the shape of the `session/request_permission` request the stub
/// raises under `--ask-permission` (id, method, and its two options). Split
/// out of the test itself to keep `ask_permission_emits_requires_action_then_resumes`
/// under the workspace's cognitive-complexity guard (`clippy::cognitive_complexity`).
fn assert_permission_request(perm: &serde_json::Value) {
    assert_eq!(
        perm["id"].as_i64(),
        Some(PERMISSION_REQUEST_ID),
        "permission request: {perm}"
    );
    assert_eq!(perm["method"], "session/request_permission");
    assert_eq!(perm["params"]["sessionId"], "stub");
    assert_eq!(perm["params"]["toolCall"]["title"], "stub tool");
    let opts = perm["params"]["options"].as_array().expect("options array");
    let allow = opts.iter().find(|o| o["optionId"] == "allow");
    let deny = opts.iter().find(|o| o["optionId"] == "deny");
    assert!(
        allow.is_some() && deny.is_some(),
        "allow+deny options: {perm}"
    );
    assert_eq!(allow.expect("checked above")["kind"], "allow_once");
    assert_eq!(deny.expect("checked above")["kind"], "reject_once");
}

#[test]
fn ask_permission_emits_requires_action_then_resumes() {
    let mut stub = spawn_stub(&["--ask-permission"]);
    stub.handshake();

    send_prompt_and_ack(&mut stub);
    // First chunk is emitted before the permission gate. (The state_update
    // "running" notification precedes it.)
    let running = stub.read_line();
    assert_eq!(running["params"]["update"]["state"], "running");
    let first = stub.read_line();
    assert_eq!(
        first["params"]["update"]["sessionUpdate"],
        "agent_message_chunk"
    );

    // The agent then raises a permission request (id 10) and reports
    // requires_action.
    let perm = stub.read_line();
    assert_permission_request(&perm);

    let blocked = stub.read_line();
    assert_eq!(blocked["params"]["update"]["sessionUpdate"], "state_update");
    assert_eq!(blocked["params"]["update"]["state"], "requires_action");

    // Approve: respond to the agent's request (id 10) with a selected option.
    stub.send(
        r#"{"jsonrpc":"2.0","id":10,"result":{"outcome":{"outcome":"selected","optionId":"allow"}}}"#,
    );
    // The agent resumes: back to running, then finishes the turn (the
    // default `--chunks 3`: chunk 0 already streamed before the gate, so 2
    // more chunks — index 1, 2 — remain).
    let running = stub.read_line();
    assert_eq!(running["params"]["update"]["state"], "running");
    for _ in 0..2 {
        let chunk = stub.read_line();
        assert_eq!(
            chunk["params"]["update"]["sessionUpdate"],
            "agent_message_chunk"
        );
    }
    let idle = stub.read_line();
    assert_eq!(idle["params"]["update"]["stopReason"], "end_turn");

    stub.close();
}

#[test]
fn unknown_method_is_32601_not_a_crash() {
    let mut stub = spawn_stub(&[]);
    stub.handshake();

    // Unknown methods are answered with JSON-RPC -32601, and the stub keeps
    // serving (it does not crash).
    stub.send(UNKNOWN);
    let err = stub.read_response("5");
    assert_eq!(err["error"]["code"], serde_json::json!(-32601), "{err}");

    // Still alive: a prompt afterwards completes with end_turn (default
    // `--chunks 3`).
    send_prompt_and_ack(&mut stub);
    let _ = stub.read_line(); // running
    for _ in 0..3 {
        let _ = stub.read_line(); // chunk 0/1/2
    }
    let idle = stub.read_line();
    assert_eq!(idle["params"]["update"]["stopReason"], "end_turn", "{idle}");

    stub.close();
}

#[test]
fn eof_exits_zero() {
    let mut stub = spawn_stub(&[]);
    stub.handshake();
    // Close stdin: EOF must make the agent exit cleanly with status 0.
    assert_eq!(stub.close(), Some(0), "stub must exit 0 on EOF");
}
