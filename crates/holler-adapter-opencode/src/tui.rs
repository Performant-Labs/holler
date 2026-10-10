//! The tmux side of the adapter: which tmux server a pane's TUI runs on.
//!
//! In part 1 of #642 this module holds only [`TmuxSocket`] and [`TmuxConfig`], so that
//! [`crate::OpenCodeConfig`] is final. Part 2 adds the tmux calls of `attach_tui`,
//! `select_session` and `shown_session` here, with their builders and parsers. This adapter
//! never starts a tmux server (it only addresses sessions that exist), so a tmux config
//! file has no effect and there is no `-f`.

use std::path::PathBuf;

/// Which tmux server to address: the same variants as the host adapter's (#641)
/// `TmuxSocket`, so wiring (#649) configures one value and hands it to both adapters. An
/// adapter cannot depend on another adapter crate (ADR-0021 section 5), hence a mirror and
/// not a re-export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxSocket {
    /// tmux's own default socket: no flag. Never the server an inherited `$TMUX` names.
    Default,
    /// A named socket: `-L <name>`.
    Name(String),
    /// A socket path: `-S <path>`.
    Path(PathBuf),
}

/// How the adapter runs tmux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxConfig {
    /// The `tmux` binary; production: `"tmux"`, found on `PATH`.
    pub tmux_bin: PathBuf,
    /// Production: [`TmuxSocket::Default`]; tests: always [`TmuxSocket::Path`], a private
    /// server.
    pub socket: TmuxSocket,
}
