#![allow(clippy::unwrap_used)] // #649
#![allow(clippy::expect_used)] // #649
#![allow(clippy::panic)] // #649
#![allow(clippy::unreachable)] // #649
#![allow(dead_code)] // #649
//! The pane wiring (#649) against a real hub, and the scripted scenario against scratch
//! Herdr, tmux and OpenCode instances (epic #633). One target, declared by #670.
//!
//! **Two kinds of test (D13).** The default run (CI) needs only a hub subprocess and a
//! state dir: the no-hub answer of every verb (`no_hub`), the verbs that need no adapter
//! (`store_only`), the store clients against a real hub (`clients`: the test kit's two
//! conformance suites) and against a hub that misbehaves on purpose (`fake_hub`), the
//! watch (`watch`), the environment the wiring reads (`env`, and `wired` for the ports
//! `connect` builds from it), the bindings, decorators and
//! lazy ports (`bind`), and the scenario's guards as pure functions (`guards`). The
//! scenario itself (`scenario`) is `#[ignore]` and runs only with
//! `HOLLER_TEST_PANE_INTEGRATION=1` exactly, on scratch instances it starts itself
//! (`scratch`). `out` and `scratch` are shared with the `profile_apply_scenario` target
//! (#667), which includes them by path.
//!
//! **The state-dir lock.** The store clients reach the hub through
//! `holler_hub::control::run`, which reads `HOLLER_STATE_DIR` from this process on every
//! call. Tests on parallel threads would race on it, so every in-process client call runs
//! inside [`with_state_dir`], which holds one lock while it sets the variable (A's W-10).
//! The process-level tests set the variable on the child only and need no lock.

#[path = "../support/mod.rs"]
mod support;

#[path = "../verb_harness/mod.rs"]
mod verb_harness;

mod bind;
mod clients;
mod env;
mod fake_hub;
mod forwarding;
mod guards;
mod no_hub;
mod observe;
mod out;
mod scenario;
mod scratch;
mod store_only;
mod watch;
mod wired;

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

pub use out::{output_of, Out};

/// Held while an in-process test points `HOLLER_STATE_DIR` at its own hub.
static STATE_DIR_LOCK: Mutex<()> = Mutex::new(());

/// Take the state-dir lock (a test that panicked while holding it poisons nothing that
/// matters: the variable is set again by the next holder).
pub fn state_dir_lock() -> MutexGuard<'static, ()> {
    STATE_DIR_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Run `f` with `HOLLER_STATE_DIR` set to `dir`, under the state-dir lock.
pub fn with_state_dir<R>(dir: &Path, f: impl FnOnce() -> R) -> R {
    let _held = state_dir_lock();
    // The workspace is edition 2021, where `set_var` is safe; the lock serializes it
    // against every other in-process client test of this target.
    std::env::set_var("HOLLER_STATE_DIR", dir);
    f()
}

/// [`with_state_dir`], with each variable of `vars` set (`Some`) or removed (`None`) for the
/// length of `f` and put back as it was afterwards. Only the in-process `Wiring::connect`
/// tests need it: `connect` reads the wiring's settings from this process's environment. A
/// value is any OS string, so a test can set one that is not UTF-8 (A's W-T).
pub fn with_env<R, V: AsRef<std::ffi::OsStr>>(
    dir: &Path,
    vars: &[(&str, Option<V>)],
    f: impl FnOnce() -> R,
) -> R {
    let _held = state_dir_lock();
    std::env::set_var("HOLLER_STATE_DIR", dir);
    let saved: Vec<(String, Option<std::ffi::OsString>)> = vars
        .iter()
        .map(|(name, _)| ((*name).to_owned(), std::env::var_os(name)))
        .collect();
    for (name, value) in vars {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    for (name, value) in saved {
        match value {
            Some(value) => std::env::set_var(&name, value),
            None => std::env::remove_var(&name),
        }
    }
    result.unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

/// The variables a verb run of the default run never inherits: anything that could point
/// the binary at the operator's Herdr, tmux or OpenCode, or at a wiring setting the test
/// did not choose.
pub const SCRUBBED: &[&str] = &[
    "TMUX",
    "TMUX_PANE",
    "HERDR_SESSION",
    "HERDR_SOCKET_PATH",
    "HERDR_PANE_ID",
    "HOLLER_HERDR_SESSION",
    "HOLLER_HERDR_SOCKET",
    "HOLLER_HERDR_WORKSPACES",
    "HOLLER_TMUX_SOCKET",
    "HOLLER_OPENCODE_BIN",
    "HOLLER_OPENCODE_SERVE_ARGS",
];

/// `holler <args>` on `state`, with every variable of [`SCRUBBED`] removed and `envs` set.
pub fn holler_on(state: &support::StateDir, args: &[&str], envs: &[(&str, &str)]) -> Out {
    let mut command = support::holler_cmd(state);
    for name in SCRUBBED {
        command.env_remove(name);
    }
    command.envs(envs.iter().copied()).args(args);
    output_of(command)
}
