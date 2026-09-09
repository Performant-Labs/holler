//! `body detach`: forget the body's identity.
//!
//! Story #176 landed the **no-run** form: with no live `body run`, detach
//! simply deletes the persisted identity (`<state>/body/credential.json`) —
//! there is nothing live to tear down.
//!
//! Issue #182 adds the **live-run** form the doc comment on `detach` (story
//! #176) already promised: when a `body run` holds the instance lock,
//! `detach` instead asks it to end the circuit — write `body/
//! detach_request`, wait up to 10s for `body/connection_state.json` to report
//! `disconnected` (the live process's own poll-and-close, `connection.rs`
//! step 6) — and only then tears down `credential.json` +
//! `connection_state.json` + the (now-consumed) `detach_request` file itself.
//! A `body run` that does not actually react within the 10s budget still gets
//! its identity cleared (the operator asked to detach; a wedged run should
//! not be able to refuse that) — but the *file* teardown always happens after
//! the wait, so a still-live run's very last read of `detach_request` — if it
//! is merely slow — is never raced out from under it before it can see it.
//!
//! Idempotent: detaching a body that has no identity (already detached, or
//! never joined) is a success, not an error — so `body detach` always exits 0
//! except on an I/O failure the OS reports (exit 1).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{connection_state, identity, instance_lock};

/// The CLI exit code `body detach` applies (ADR 0003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetachExit {
    /// The identity was forgotten (or there was none); the bin exits 0.
    Ok,
    /// The state dir could not be written (an OS I/O error); the bin exits 1.
    Io,
}

/// How long a live-run detach waits for `connection_state.json` to report
/// `disconnected` before giving up and tearing down anyway (issue #182: "wait
/// <=10 s").
const LIVE_DETACH_TIMEOUT: Duration = Duration::from_secs(10);
const LIVE_DETACH_POLL: Duration = Duration::from_millis(100);

fn detach_request_path(state_root: &Path) -> PathBuf {
    state_root.join("body").join("detach_request")
}

/// Forget the body's identity: the no-run form (story #176) deletes
/// `<state>/body/credential.json` outright; the live-run form (issue #182)
/// signals the live `body run` first (see the module doc). Returns
/// [`DetachExit`] the CLI bin turns into an exit code — always `Ok` except an
/// OS I/O error, since detach is idempotent by design.
pub fn detach(state_root: &Path) -> DetachExit {
    // A run is "live" iff the instance lock is currently held by someone
    // else: `acquire` tells us that without a separate PID-liveness check
    // (the flock IS the liveness check — a crashed run's lock is already
    // gone, per `instance_lock`'s own doc comment).
    match instance_lock::acquire(state_root) {
        Ok(guard) => {
            // We just took the lock ourselves: nothing was live. Release it
            // immediately (this call is not a `run`) and take the plain path.
            drop(guard);
            clear_all(state_root)
        }
        Err(instance_lock::LockError::Held(_)) => detach_live(state_root),
        Err(instance_lock::LockError::Io(e)) => {
            eprintln!("error: {e}");
            DetachExit::Io
        }
    }
}

/// The live-run half: signal the running `body run`, wait for it to report
/// `disconnected`, then tear the identity down regardless of whether it made
/// the deadline (see the module doc's note on a wedged run).
fn detach_live(state_root: &Path) -> DetachExit {
    let request_path = detach_request_path(state_root);
    if let Some(parent) = request_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("error: state dir: {e}");
            return DetachExit::Io;
        }
    }
    if let Err(e) = std::fs::write(&request_path, b"") {
        eprintln!("error: state dir: {e}");
        return DetachExit::Io;
    }
    let deadline = Instant::now() + LIVE_DETACH_TIMEOUT;
    while Instant::now() < deadline {
        if is_disconnected(state_root) {
            break;
        }
        std::thread::sleep(LIVE_DETACH_POLL);
    }
    clear_all(state_root)
}

fn is_disconnected(state_root: &Path) -> bool {
    matches!(
        connection_state::read(state_root).map(|d| d.state),
        Some(connection_state::ConnState::Disconnected)
    )
}

/// Tear down every file `body run`/`body join` leave behind: the credential,
/// the connection-state document, and (idempotently) a stray detach request.
fn clear_all(state_root: &Path) -> DetachExit {
    if let Err(e) = identity::clear(state_root) {
        eprintln!("error: state dir: {e}");
        return DetachExit::Io;
    }
    let _ = connection_state::clear(state_root);
    let _ = std::fs::remove_file(detach_request_path(state_root));
    DetachExit::Ok
}
