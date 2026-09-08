#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #149
//! Integration tests for the ACP v2 stub agent (story #130).
//!
//! Each test spawns the built `stub-acp` binary as a real process and drives
//! it over stdio with newline-delimited JSON-RPC 2.0: writes requests, reads
//! the streamed notifications + response. The protocol shapes asserted here
//! are ACP v2 (`agent-client-protocol` 2.1.0's schema): `session/update`
//! carries a `sessionUpdate` discriminator and a nested typed payload
//! (e.g. `state_update` → `{state:"running"}`, `agent_message_chunk` →
//! `{content:{type:"text", text}}`).

use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

// ACP v2 wire shapes ---------------------------------------------------------

const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":2,"info":{"name":"tester","version":"0"}}}"#;
const SESSION_NEW: &str = r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/"}}"#;
const PROMPT: &str = r#"{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"stub","prompt":[{"type":"text","text":"hi"}]}}"#;
const CANCEL: &str =
    r#"{"jsonrpc":"2.0","id":4,"method":"session/cancel","params":{"sessionId":"stub"}}"#;
const UNKNOWN: &str = r#"{"jsonrpc":"2.0","id":5,"method":"bogus/method","params":{}}"#;

// Harness ---------------------------------------------------------------------

struct Stub {
    child: Child,
    reader: Option<BufReader<std::process::ChildStdout>>,
    stdin: std::process::ChildStdin,
}

/// Cargo sets `CARGO_BIN_EXE_<name>` for integration tests to the built binary
/// path. (We can't use `assert_cmd::cargo_bin` here because its `Command`
/// wrapper cannot hand us a live, piped stdin for interactive I/O.)
fn stub_bin_path() -> &'static str {
    // Compile-time read (defect #147): a missing var fails the build, not a
    // silently-exited test.
    env!("CARGO_BIN_EXE_stub-acp")
}

fn spawn_stub(args: &[&str]) -> Stub {
    let mut child = Command::new(stub_bin_path())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn stub-acp");
    let stdin = child.stdin.take().expect("child stdin is piped");
    let stdout = child.stdout.take().expect("child stdout is piped");
    Stub {
        child,
        reader: Some(BufReader::new(stdout)),
        stdin,
    }
}

impl Stub {
    fn reader(&mut self) -> &mut BufReader<std::process::ChildStdout> {
        self.reader.as_mut().expect("stdout reader is present")
    }

    fn send(&mut self, line: &str) {
        self.stdin
            .write_all(line.as_bytes())
            .expect("write request line");
        self.stdin.write_all(b"\n").expect("write newline");
        self.stdin.flush().expect("flush stdin");
    }

    /// Read the next JSON line and parse it into a `Value`.
    fn read_line(&mut self) -> Value {
        let mut buf = String::new();
        self.reader()
            .read_line(&mut buf)
            .expect("read stdout line (EOF while a message was expected)");
        serde_json::from_str(&buf).expect("valid JSON line from stub")
    }

    /// Read messages until one with the given JSON-RPC id (a response) arrives.
    /// Notifications carry no id and are skipped. Returns the parsed response.
    /// `id` is the numeric JSON-RPC id as a string (e.g. "3"); the stub sends
    /// ids as JSON numbers, so we compare via `as_i64`.
    fn read_response(&mut self, id: &str) -> Value {
        let want: i64 = id.parse().expect("test ids are integers");
        loop {
            let v = self.read_line();
            match v.get("id").and_then(|i| i.as_i64()) {
                Some(i) if i == want => return v,
                Some(_) => panic!("unexpected response id: {v}"),
                None => continue, // a streamed session/update notification
            }
        }
    }

    /// Handshake: initialize + session/new. Returns the session id (expect "stub").
    fn handshake(&mut self) {
        self.send(INITIALIZE);
        let init = self.read_response("1");
        assert_eq!(
            init["result"]["protocolVersion"].as_u64(),
            Some(2),
            "initialize must negotiate protocolVersion 2: {init}"
        );
        assert!(
            init["result"].get("agentCapabilities").is_some(),
            "initialize result must carry agentCapabilities: {init}"
        );
        self.send(SESSION_NEW);
        assert_eq!(self.read_response("2")["result"]["sessionId"], "stub");
    }

    /// Close stdin (EOF) and wait for the process to exit, returning the exit
    /// code. Dropping the `ChildStdin` closes the write end of the pipe, so
    /// the stub's stdin reader sees EOF. Takes `self` by value because the
    /// `ChildStdin` is a `!Clone` field we must move out and drop.
    fn close(mut self) -> Option<i32> {
        drop(self.stdin);
        self.child.wait().ok().map(|s| s.code().unwrap_or(-1))
    }
}

// Tests ------------------------------------------------------------------------

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
    assert_eq!(perm["id"].as_i64(), Some(10), "permission request: {perm}");
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
