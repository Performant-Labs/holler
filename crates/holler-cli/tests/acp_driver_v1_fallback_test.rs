#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #362
//! Regression tests for `AcpDriver`'s ACP v1 fallback (issue #362): every
//! real ACP implementation checked while investigating that issue (the
//! newest `opencode acp`, the newest published `@agentclientprotocol/sdk`)
//! negotiates protocol v1, never v2, so `AcpDriver::spawn` tries v2 first and
//! falls back to a plain (v1) connection on that specific negotiation
//! failure — see `crates/holler-body/src/acp_driver/spawn.rs` and
//! `connection_v1.rs`'s module docs for the full mechanism.
//!
//! `stub-acp` (this crate's other stub, used by `acp_driver_test.rs`)
//! deliberately claims protocol v2 unconditionally, so it can never exercise
//! this path. `stub-acp-v1` (`tests/stub-acp-v1/main.rs`) exists purely to
//! give the fallback a real v1 peer for exactly this test file — see that
//! stub's own module doc for what it does and does not implement.
//!
//! Lives in `holler-cli`, not `holler-body`, for the identical
//! `CARGO_BIN_EXE_*`-resolution reason `acp_driver_test.rs`'s own module doc
//! explains.

use std::time::Duration;

use holler_body::acp_driver::{AcpDriver, DriverEvent, Status, StopReason};
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_proto::SessionName;

fn stub_v1_config(name: &str) -> SessionConfig {
    SessionConfig {
        name: SessionName::parse(name).expect("valid session name"),
        harness: "opencode".to_string(),
        mode: SessionMode::Spawn,
        command: Some(vec![env!("CARGO_BIN_EXE_stub-acp-v1").to_string()]),
        cwd: None,
        env: None,
        interrupt: Interrupt::Acp,
        endpoint: None,
        session_id: None,
        auth_method: None,
    }
}

/// `AcpDriver::spawn` against a peer that only negotiates v1 falls all the
/// way through to a ready driver — not a `DriverError::Startup` — and its
/// status reads exactly as it would for a v2 peer.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawn_falls_back_to_v1_and_reports_idle() {
    let config = stub_v1_config("alpha");
    let driver = AcpDriver::spawn(&config)
        .await
        .expect("spawn succeeds via the v1 fallback");
    assert_eq!(driver.status(), Status::Idle);
    driver.shutdown().await.expect("shutdown");
}

/// A real turn against the v1 fallback streams the agent's chunk and settles
/// `Done(EndTurn)` — proving `AcpDriver::prompt`'s v1 branch (which awaits
/// the `session/prompt` response directly, unlike v2's fire-and-forget) is
/// wired correctly end to end, not just that the handshake completes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_streams_chunk_then_done_end_turn_via_v1() {
    let config = stub_v1_config("alpha");
    let driver = AcpDriver::spawn(&config).await.expect("spawn via v1 fallback");
    let mut stream = driver.prompt("hi").await;

    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(remaining > Duration::ZERO, "timed out waiting for Done: {events:?}");
        match tokio::time::timeout(remaining, futures_util::StreamExt::next(&mut stream)).await {
            Ok(Some(event)) => {
                let done = matches!(event, DriverEvent::Done(_));
                events.push(event);
                if done {
                    break;
                }
            }
            Ok(None) => panic!("stream ended before Done: {events:?}"),
            Err(_timed_out) => panic!("timed out waiting for Done: {events:?}"),
        }
    }

    assert!(
        events.iter().any(|e| matches!(e, DriverEvent::Chunk(text) if text == "hello from v1")),
        "{events:?}"
    );
    assert_eq!(events.last(), Some(&DriverEvent::Done(StopReason::EndTurn)));
    driver.shutdown().await.expect("shutdown");
}
