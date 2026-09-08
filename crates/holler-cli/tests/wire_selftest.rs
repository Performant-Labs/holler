#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #149
//! Test-of-tests canary (story #129).
//!
//! Proves the harness can *see* a failure before any real test exists. If this
//! file were green while the runner was broken, every later e2e would be
//! theatre. Each test is fast (well under the 2s budget the spec sets) and
//! together they pin the three failure classes the runner must detect:
//!
//! 1. a child process that exits non-zero (and a nested panic),
//! 2. a closed-port connect that must fail fast,
//! 3. the async runtime the whole binary depends on actually booting.

use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

const BUDGET: Duration = Duration::from_secs(2);

/// A child process that exits 1 must be observed as a non-zero exit, and a
/// nested `assert!(false)` wrapped in `catch_unwind` must be observed as a
/// panic. Two independent ways the runner can go blind, checked in one fast
/// case. `cfg!` picks the spawn command per platform (mirrors the old
/// server's selftest).
#[test]
fn designed_to_fail_case_is_detected() {
    let started = Instant::now();

    // (a) A real child that exits 1. On Unix `sh -c 'exit 1'`; on Windows
    // `cmd /c exit 1`. We must *see* the non-zero status — not have it
    // swallowed.
    #[cfg(unix)]
    let child = std::process::Command::new("sh").arg("-c").arg("exit 1").spawn();
    #[cfg(windows)]
    let child = std::process::Command::new("cmd").arg("/c").arg("exit 1").spawn();

    let mut child = child.expect("spawn the child process");
    let status = child
        .wait()
        .expect("wait for the child to exit");
    assert!(
        !status.success(),
        "harness must observe the child's exit-1 as a failure; it reported success"
    );

    // (b) A nested panic: `catch_unwind` around a *direct* `assert!(false)`
    // must come back as `Err`, i.e. the panic is *observed*, not silently
    // dropped. (Note: wrapping the assert in its own catch_unwind would
    // swallow the panic and make the outer one observe nothing — which is
    // precisely the "runner goes blind" failure this canary is meant to catch.)
    let caught = std::panic::catch_unwind(|| {
        // A constant-false assert trips Clippy's `assertions_on_constants`.
        // That lint is correct in ordinary code, but here the panic is the
        // *subject* under test: we deliberately panic so `catch_unwind` can be
        // seen to observe it. Scope the exemption to this closure only.
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(false, "nested assert must trip");
        }
    });
    assert!(caught.is_err(), "catch_unwind must report the nested panic");

    assert!(
        started.elapsed() < BUDGET,
        "fail-detection must be fast; took {:?} (budget {BUDGET:?})",
        started.elapsed()
    );
}

/// Bind a real listener, read the OS-assigned port, drop it, then try to
/// connect to that now-closed port. On a healthy machine this is refused
/// immediately (or times out far under budget). The old server test was flaky
/// on Windows because real connect-refused latency there blew the budget —
/// which is exactly why Windows is off the CI matrix (ADR 0002). The point of
/// this canary is to *fail fast and deterministically* on the OSes we do run.
///
/// `bind("127.0.0.1:0")` hands out a *random* port, and in a busy CI runner
/// another process can transiently occupy that exact port for a moment. A
/// single draw could therefore "succeed" by colliding with a live port — a
/// false-negative that would read as "closed ports accept connections." So we
/// draw a few distinct ports and require *at least one* to fail, which a
/// broken runner (one that swallows refusals) could not fake.
const TRIES: u32 = 5;

#[test]
fn dial_closed_port_fails_fast() {
    let started = Instant::now();

    let mut saw_refusal = false;
    for _ in 0..TRIES {
        let listener = TcpListener::bind("127.0.0.1:0")
            .expect("bind a throwaway listener on 127.0.0.1");
        let port = listener
            .local_addr()
            .expect("read the assigned port")
            .port();
        // Drop the listener *before* dialing so the port is closed.
        drop(listener);

        let target = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port,
        );

        // A genuinely closed port must *fail* the connect (refused). We do not
        // hard-assert a specific `ErrorKind` — refused can surface differently
        // across OSes — we only require that it is *not* a successful
        // connection. On a healthy box the first draw is refused in ~micros.
        if TcpStream::connect_timeout(&target, BUDGET).is_ok() {
            // This draw collided with a live port; try the next one.
            continue;
        }
        saw_refusal = true;
        break;
    }

    // If no draw was refused, every port we closed "accepted" — the runner is
    // blind to refusals, which is exactly the failure this canary exists to catch.
    assert!(
        saw_refusal,
        "every dial to a closed port succeeded; the runner cannot observe a refusal"
    );

    assert!(
        started.elapsed() < BUDGET,
        "refusal must be fast; took {:?} (budget {BUDGET:?})",
        started.elapsed()
    );
}

/// The whole binary is built on tokio's multi-thread runtime. This proves the
/// runtime the binary depends on actually *boots* under the test harness: if
/// `#[tokio::test]` could not construct a runtime, nothing async in the later
/// e2e stories would even start.
#[tokio::test]
async fn tokio_runtime_boots() {
    // `sleep` on the runtime forces a tick of the reactor; if the runtime had
    // not booted this would either panic or hang past the budget.
    let started = Instant::now();
    tokio::time::sleep(Duration::from_millis(1)).await;
    assert!(
        started.elapsed() < BUDGET,
        "a 1ms sleep must not exceed the budget; the runtime did not drive it"
    );
}
