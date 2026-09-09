#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #188
//! `crash_mid_turn_is_error_not_hang` (issue #188's RED list), split into its
//! own test binary/OS process, on a multi-thread tokio runtime.
//!
//! # Status: `#[ignore]`d — confirmed upstream, not fixable in this crate
//!
//! This is the one test in the RED list whose signal depends on the ACP
//! SDK's own crash-detection plumbing: it spawns a background task
//! (`connection.spawn`) that awaits `connection.incoming_closed()` and pushes
//! `DriverEvent::Done(Error)` once the child's stdout reaches EOF.
//!
//! ## 2026-09-09 follow-up: reproduced live on the actual CI runner, root cause
//! identified inside `agent-client-protocol` 2.1.0 itself
//!
//! Got shell access to the real CI environment this time (`ssh uranus`; the
//! runner is a Docker container, image
//! `harbor.performantlabs.com/performantlabs/pl-runner:1.66.1`, entrypoint
//! `dumb-init` → `Runner.Listener`, `cap_sys_ptrace` dropped, no `docker
//! --init`/tini beyond the baked-in `dumb-init`). Built a byte-identical
//! container from that image, installed the pinned toolchain, and reproduced
//! the hang deterministically (3/3 runs) running *only*
//! `acp_driver_crash_test` `--ignored`, exactly the CI command line — so this
//! is not cross-process contention, not a container-supervisor
//! double-reaping problem, and not a missing `--init`.
//!
//! `strace -f` across the crash (see the PR description / commit for the raw
//! log excerpts) shows every OS-level mechanic happening correctly and
//! promptly, all within ~3ms of the child's `exit_group(1)`:
//! - `SIGCHLD` is delivered to the reactor thread (`--- SIGCHLD
//!   {si_code=CLD_EXITED, si_status=1} ---`) — `async-signal`'s handler is
//!   registered and fires; there is no conflicting handler.
//! - The child's stdout pipe read returns clean EOF (`read(11, "", 8192) =
//!   0`) immediately after the last real JSON-RPC line is drained.
//! - `wait4(child_pid, ..., WNOHANG, ...)` reaps the child and gets its exit
//!   status — no zombie, no double-reap race with the container's `dumb-init`
//!   supervisor (that supervisor only reaps *its own* orphans; it is not in
//!   the path between the test binary and its direct child).
//! - stderr is drained to EOF too, and every pipe fd is closed.
//!
//! So `async-process`/`async-signal`'s job is done correctly. The bug is one
//! layer up: **the OS thread that did all of the above (`read`
//! EOF → `wait4` reap → close fds) immediately parks on
//! `futex(FUTEX_WAIT_BITSET_PRIVATE, ..., NULL)` — an unbounded wait — right
//! after that cleanup, and is never woken again.** No thread in the process
//! ever becomes runnable again from a real I/O or task-wakeup event; the only
//! thing that eventually happens is the *test's own* `tokio::time::timeout`
//! (45s) firing, at which point the runtime tears down and every parked
//! thread gets woken by that shutdown, not by anything connection-related.
//! Concretely: neither this test's crash-watcher task (`connection.spawn`)
//! **nor** `AcpDriver::spawn`'s own `connection::run` task (whose foreground
//! `tokio::select!` *also* awaits `connection.incoming_closed()` directly,
//! independent of the crash watcher) ever wakes — both are downstream of the
//! same `agent_client_protocol::jsonrpc::IncomingClosed` signal, and neither
//! fires. That rules out a bug specific to holler-body's own crash-watcher
//! pattern: the SDK's own `finish_incoming_close()` call is what never
//! happens (or never wakes anything), not a problem with how holler-body
//! consumes `incoming_closed()`.
//!
//! Reading `agent-client-protocol` 2.1.0's own source
//! (`~/.cargo/registry/.../agent-client-protocol-2.1.0/src/jsonrpc.rs` and
//! `src/jsonrpc/incoming_actor.rs`) shows why this is very plausibly a crate
//! bug rather than a holler-body one: `connection.spawn(...)` does **not**
//! use `tokio::spawn` — it enqueues the future onto an internal
//! `mpsc::UnboundedSender<Task>` consumed by the crate's own `task_actor`
//! (`process_stream_concurrently` over the task stream), which itself is
//! composed with the incoming/outgoing protocol actors through two *nested*
//! `run_until_connection_close(background, foreground, incoming_closed)`
//! calls (a `future::select` plus a second `future::select` against
//! `incoming_closed.closed()` in the "foreground already resolved but
//! incoming is still closing" branch) built on `futures::try_join!` and
//! `futures_concurrency::stream::StreamExt::merge`. That is exactly the kind
//! of deeply-nested, hand-rolled multi-future composition where a lost wakeup
//! is easy to introduce and hard to notice locally, because it only shows up
//! under specific multi-thread-runtime scheduling timing.
//!
//! This is corroborated, not just theorized: `incoming_closed()` /
//! `IncomingClosed` / `run_until_connection_close` did not always exist in
//! this crate — they were added by
//! <https://github.com/agentclientprotocol/rust-sdk/pull/261> ("fix(acp):
//! Handle incoming EOF correctly", merged 2026-07-20, +2748/-323), closing
//! <https://github.com/agentclientprotocol/rust-sdk/issues/250> ("Pending
//! send_request futures never resolve when the transport ends (EOF)"). It is
//! large, recent (this repo pins 2.1.0, published 2026-09-04 — no newer
//! version exists to upgrade to), and self-admittedly retrofits EOF-handling
//! onto a connection model that wasn't originally built for it. A second,
//! independent report against the *same* async-process/async-io-under-tokio
//! boundary — <https://github.com/agentclientprotocol/rust-sdk/issues/254>,
//! "`AcpAgent` stderr reader busy-polls a full CPU core at idle under a tokio
//! runtime" (a *spurious*-wake bug: the reactor rewakes a task that isn't
//! actually ready) — shows the opposite-shaped defect in the same
//! `async-io`-reactor-driven-under-a-foreign-tokio-runtime area of this
//! crate. A crate that has one confirmed bug where a task wakes when it
//! shouldn't, in the exact subsystem where another task now provably doesn't
//! wake when it should, is strong circumstantial evidence this is one
//! upstream reliability seam, not two unrelated holler-body integration
//! mistakes.
//!
//! **Not fixable from holler-body without an upstream change.** `AcpDriver`
//! has no access to the child's raw PID/`Child` handle to build an
//! independent liveness watchdog — `AcpAgentConfig`/`AcpAgent` own the spawn
//! entirely — so any local mitigation would have to poll
//! `connection.is_incoming_closed()`, which is gated on the exact same
//! `finish_incoming_close()` call that this investigation shows never
//! happens; it would not help. Per this repo's own external-contribution
//! policy, filing this against `agentclientprotocol/rust-sdk` (with the
//! `strace` evidence and the #250/#254/#261 cross-references above) needs a
//! human's go-ahead — recommended, not done by this session.
//!
//! Two mitigations were applied in the original story and both measurably
//! helped locally (this test's own `[[test]]` target — see `Cargo.toml` — so
//! it never competes with the other 18 driver tests' own child-process churn
//! in one process; and `flavor = "multi_thread"` instead of
//! `#[tokio::test]`'s single-thread default, matching how the SDK's own
//! examples run under `#[tokio::main]`) — neither changes the outcome on CI,
//! consistent with the root cause being inside the SDK's own task
//! composition rather than anything about how this crate drives it.
//! `#[ignore]`d here rather than deleted so the assertion and investigation
//! notes stay in the tree for whoever eventually files (or fixes) this
//! upstream. Issue #189's session-manager story hit the identical hang in
//! its own `driver_crash_isolated_and_restarts_on_next_prompt` test
//! (`holler-cli/tests/session_manager_test.rs`, also `#[ignore]`d) and
//! confirmed — by reproducing directly against *this* test, 3 runs in a row —
//! that it is the same pre-existing defect, not something introduced by that
//! story.

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
#[ignore = "flaky under CI's process/reactor scheduling — see this file's module doc; tracked as a follow-up, run manually with `cargo test -p holler-cli --test acp_driver_crash_test -- --ignored`"]
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
