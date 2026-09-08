//! State-dir resolution (ADR 0001).
//!
//! `holler` roots everything under one **state dir**: `hub/` for the hub role
//! (tokens, roster, talk log, control socket, instance lock) and `body/` for
//! the body role (client id, sessions). The dir is `~/.holler` by default and
//! may be overridden with `HOLLER_STATE_DIR` (used by the test harness so each
//! test gets isolated state).

use std::path::{Path, PathBuf};

/// The resolved state dir and the role subdirs it roots.
#[derive(Debug, Clone)]
pub struct HubState {
    /// The state dir root (`HOLLER_STATE_DIR`, else `~/.holler`).
    pub root: PathBuf,
    /// The hub's per-state dir (`<root>/hub`).
    pub hub_dir: PathBuf,
}

impl HubState {
    /// Create a [`HubState`] rooted at `root` (the `hub/` subdir is derived).
    #[inline]
    pub fn from_root(root: PathBuf) -> Self {
        let hub_dir = root.join("hub");
        Self { root, hub_dir }
    }
}

/// Resolve the state dir from the environment.
///
/// `HOLLER_STATE_DIR` wins; the default is `~/.holler`. A missing `$HOME` with
/// no override is a fail-closed refusal — we refuse to guess a state location.
/// Returns `None` (after printing the refusal) in that case so the **caller**
/// (the CLI's `hub serve` leaf, a bin) owns the exit; a lib helper never exits
/// (an exit here would mask the code from the caller).
pub fn resolve_state_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("HOLLER_STATE_DIR") {
        return Some(PathBuf::from(dir));
    }
    let Some(home) = std::env::var_os("HOME") else {
        eprintln!("error: HOLLER_STATE_DIR is not set and $HOME is unavailable");
        return None;
    };
    Some(PathBuf::from(home).join(".holler"))
}

/// Ensure the state dir and the hub subdir exist (creating them).
pub fn ensure_dirs(state: &HubState) -> std::io::Result<()> {
    std::fs::create_dir_all(&state.root)?;
    std::fs::create_dir_all(&state.hub_dir)
}

/// The control socket path (`<root>/hub/control.sock`).
pub fn control_sock_path(state: &HubState) -> PathBuf {
    state.hub_dir.join("control.sock")
}

/// The instance lock path (`<root>/hub/serve.lock`).
pub fn serve_lock_path(state: &HubState) -> PathBuf {
    state.hub_dir.join("serve.lock")
}

/// The advertise-config path (`<root>/hub/advertise.json`).
pub fn advertise_path(state: &HubState) -> PathBuf {
    state.hub_dir.join("advertise.json")
}

/// True iff `p` is a descendant of (or equal to) `root`. Used to refuse to
/// create sockets in a state dir that does not actually own them.
#[allow(dead_code)] // #143 forward-declared for a later story that guards socket paths
pub fn is_within(root: &Path, p: &Path) -> bool {
    p.starts_with(root)
}
