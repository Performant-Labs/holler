#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #149
//! Integration tests for the ACP v2 stub agent (story #130).
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

mod support;

use support::Stub;
use support::{CANCEL, PERMISSION_REQUEST_ID, PROMPT, UNKNOWN};

/// Spawn `stub-acp` with `args` and piped stdio via the shared driver.
/// (`stderr` is nulled inside [`Stub::start`]: the stub writes only the
/// JSON-RPC stream to stdout, so there is nothing a test needs from stderr.)
fn spawn_stub(args: &[&str]) -> Stub {
    Stub::start(args)
}

#[test]
fn prompt_streams_chunks_then_end_turn() {
    let mut stub = spawn_stub(&["--chunks", "3"]);
    stub.handshake();

    stub.send(PROMPT);
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
    }

    // Finally the prompt response ends the turn.
    let resp = stub.read_response("3");
    assert_eq!(resp["result"]["stopReason"], "end_turn");

    stub.close();
}

#[test]
fn cancel_yields_cancelled_stop_reason_and_stub_survives() {
    let mut stub = spawn_stub(&["--slow"]);
    stub.handshake();

    stub.send(PROMPT);
    // At least one notification (running, or a chunk) lands before the cancel.
    stub.read_line();
    // Cancel mid-turn.
    stub.send(CANCEL);
    // The in-flight prompt resolves with the cancelled stop reason...
    let resp = stub.read_response("3");
    assert_eq!(resp["result"]["stopReason"], "cancelled", "{resp}");
    // ...and the stub answers the cancel notification itself with {}.
    let cancel_resp = stub.read_response("4");
    assert_eq!(cancel_resp["result"], serde_json::json!({}));

    // The stub survives: a subsequent prompt still completes normally.
    stub.send(PROMPT);
    let _ = stub.read_line();
    let resp2 = stub.read_response("3");
    assert_eq!(resp2["result"]["stopReason"], "end_turn", "{resp2}");

    stub.close();
}

#[test]
fn ask_permission_emits_requires_action_then_resumes() {
    let mut stub = spawn_stub(&["--ask-permission"]);
    stub.handshake();

    stub.send(PROMPT);
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
    assert_eq!(allow.unwrap()["kind"], "allow_once");
    assert_eq!(deny.unwrap()["kind"], "reject_once");

    let blocked = stub.read_line();
    assert_eq!(blocked["params"]["update"]["sessionUpdate"], "state_update");
    assert_eq!(blocked["params"]["update"]["state"], "requires_action");

    // Approve: respond to the agent's request (id 10) with a selected option.
    stub.send(
        r#"{"jsonrpc":"2.0","id":10,"result":{"outcome":{"outcome":"selected","optionId":"allow"}}}"#,
    );
    // The agent resumes: back to running, then finishes the turn.
    let running = stub.read_line();
    assert_eq!(running["params"]["update"]["state"], "running");
    let resp = stub.read_response("3");
    assert_eq!(resp["result"]["stopReason"], "end_turn");

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

    // Still alive: a prompt afterwards completes with end_turn.
    stub.send(PROMPT);
    let _ = stub.read_line();
    let resp = stub.read_response("3");
    assert_eq!(resp["result"]["stopReason"], "end_turn", "{resp}");

    stub.close();
}

#[test]
fn eof_exits_zero() {
    let mut stub = spawn_stub(&[]);
    stub.handshake();
    // Close stdin: EOF must make the agent exit cleanly with status 0.
    assert_eq!(stub.close(), Some(0), "stub must exit 0 on EOF");
}
