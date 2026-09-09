//! `body detach` (story #176): forget the body's identity.
//!
//! The no-`body run` form of detach: it simply deletes the persisted identity
//! (`<state>/body/credential.json`). There is no live connection to tear down —
//! a `body run` that is live detaches through the live-session path (a later
//! story), which this does not touch. Deleting the credential is what makes the
//! body "forget" its identity: a later `body status` then reports `joined:
//! false`, and a later `body run` has nothing to re-authenticate with.
//!
//! Idempotent: detaching a body that has no identity (already detached, or
//! never joined) is a success, not an error — so `body detach` always exits 0
//! except on an I/O failure the OS reports (exit 1).

use std::path::Path;

use crate::identity;

/// The CLI exit code `body detach` applies (ADR 0003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetachExit {
    /// The identity was forgotten (or there was none); the bin exits 0.
    Ok,
    /// The state dir could not be written (an OS I/O error); the bin exits 1.
    Io,
}

/// Forget the body's identity: delete `<state>/body/credential.json`.
///
/// Returns [`DetachExit`] the CLI bin turns into an exit code. A missing
/// identity is a no-op success (`DetachExit::Ok`) — detach is idempotent.
/// The one-line reason for an I/O failure is printed to stderr here (the
/// helper is synchronous and the bin has no runtime in scope).
pub fn detach(state_root: &Path) -> DetachExit {
    match identity::clear(state_root) {
        Ok(()) => DetachExit::Ok,
        Err(e) => {
            eprintln!("error: state dir: {e}");
            DetachExit::Io
        }
    }
}
