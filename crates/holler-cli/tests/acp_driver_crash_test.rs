#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #188
//! `crash_mid_turn_is_error_not_hang` (issue #188's RED list), split into its
//! own test binary/OS process, on a multi-thread tokio runtime.
//!
//! This is the one test in the RED list whose signal depends on the ACP
//! SDK's own crash-detection plumbing: it spawns a background task
//! (`connection.spawn`) that awaits `connection.incoming_closed()` and pushes
//! `DriverEvent::Done(Error)` once the child's stdout reaches EOF. Under
//! concurrent load (other test binaries — or other CI jobs on a shared
//! runner — competing for the same machine) that background task has been
//! observed, both locally and on the self-hosted CI runner, to go
//! unscheduled for many seconds even though the OS-level pipe EOF already
//! happened (confirmed directly with `lsof` on a stuck process: the pipe fd
//! was already gone, so this is a scheduling/wakeup delay inside the SDK's
//! task, not an I/O wait). Two mitigations, both applied here:
//!
//! - **Its own `[[test]]` target** (a separate OS process — see
//!   `Cargo.toml`), so it never competes with this crate's *other* 18 driver
//!   tests' own child-process churn within one process.
//! - **`flavor = "multi_thread"`** instead of `#[tokio::test]`'s
//!   single-thread default: the SDK's own examples run under `#[tokio::main]`
//!   (multi-thread by default), and giving the connection's background actors
//!   (spawned via `connection.spawn`) a genuinely separate OS thread to run on
//!   — rather than cooperating for turns on one thread with the test's own
//!   future — measurably reduced (did not eliminate) how often this
//!   particular wakeup went missing in repeated local runs.
//!
//! Even with both, this test occasionally still takes much longer than the
//! sub-100ms it needs alone (confirmed: never longer than the generous bound
//! below in dozens of local runs, but not reliably fast under load) — a
//! residual characteristic of the SDK's task scheduling under contention,
//! not a defect in this repo's driver code (which owns none of the spawning,
//! reactor, or task-scheduling logic this depends on). If this test is ever
//! the sole reason a CI run goes red, re-running the job is the right call,
//! not increasing the bound further or reverting the crash-detection design.

use std::time::Duration;

use futures_util::StreamExt;
use holler_body::acp_driver::{AcpDriver, DriverEvent, StopReason};
use holler_body::config::{Interrupt, SessionConfig, SessionMode};
use holler_proto::SessionName;

fn stub_config(name: &str, extra: &[&str]) -> SessionConfig {
    let mut command = vec![env!("CARGO_BIN_EXE_stub-acp").to_string()];
    command.extend(extra.iter().map(|s| s.to_string()));
    SessionConfig {
        name: SessionName::parse(name).expect("valid session name"),
        harness: "opencode".to_string(),
        mode: SessionMode::Spawn,
        command: Some(command),
        cwd: None,
        env: None,
        interrupt: Interrupt::Acp,
        endpoint: None,
        session_id: None,
    }
}

async fn drain_to_done(
    stream: &mut (impl futures_util::Stream<Item = DriverEvent> + Unpin),
    timeout: Duration,
) -> Vec<DriverEvent> {
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let next = tokio::time::timeout(remaining, stream.next())
            .await
            .expect("event within timeout")
            .expect("stream did not end before Done");
        let done = matches!(next, DriverEvent::Done(_));
        events.push(next);
        if done {
            return events;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crash_mid_turn_is_error_not_hang() {
    let config = stub_config("alpha", &["--crash-after-prompt", "--chunks", "3"]);
    let driver = AcpDriver::spawn(&config).await.expect("spawn");
    let mut stream = driver.prompt("hi").await;
    let events = drain_to_done(&mut stream, Duration::from_secs(45)).await;
    assert_eq!(
        events.last(),
        Some(&DriverEvent::Done(StopReason::Error)),
        "{events:?}"
    );
}
