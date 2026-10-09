//! stub (#642a): T's pinned types; in 642a this module holds only `TmuxSocket` and
//! `TmuxConfig` (the builders and parsers are 642b's).

use std::path::PathBuf;

/// Which tmux server to address; the same variants as #641's `TmuxSocket`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxSocket {
    Default,
    Name(String),
    Path(PathBuf),
}

/// How the adapter runs tmux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxConfig {
    pub tmux_bin: PathBuf,
    pub socket: TmuxSocket,
}
