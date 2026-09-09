//! `body/connection_state.json` (issue #182 step 7): the live connection's
//! own observable state, so `body status` can report `connected` without
//! reaching into the live process. Every transition is written **atomically**
//! (write to a temp file, then rename) — a reader (another process) never
//! observes a half-written file.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The four states a live `body run` cycles through (issue #182 step 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

impl ConnState {
    pub fn as_str(self) -> &'static str {
        match self {
            ConnState::Connecting => "connecting",
            ConnState::Connected => "connected",
            ConnState::Reconnecting => "reconnecting",
            ConnState::Disconnected => "disconnected",
        }
    }
}

/// The document written to `body/connection_state.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionState {
    pub state: ConnState,
    /// Unix seconds this state was entered.
    pub since: i64,
    /// Unix seconds the last frame of any kind arrived from the hub (`None`
    /// before the first frame of the current/most recent connection).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_frame_at: Option<i64>,
    /// The current reconnect attempt count (`0` while connected/connecting
    /// for the first time; bumped on each retry while `reconnecting`).
    pub attempt: u32,
}

/// The path: `<state>/body/connection_state.json`.
pub fn path(state_root: &Path) -> PathBuf {
    state_root.join("body").join("connection_state.json")
}

/// Write `doc` atomically: serialize to a sibling temp file, then rename over
/// the real path (a rename within the same directory is atomic on every OS
/// this project targets). `body status` never observes a partial write.
pub fn write(state_root: &Path, doc: &ConnectionState) -> std::io::Result<()> {
    let final_path = path(state_root);
    let Some(parent) = final_path.parent() else {
        return Err(std::io::Error::other("connection state path has no parent"));
    };
    std::fs::create_dir_all(parent)?;
    let tmp_path = parent.join(format!(".connection_state.json.{}.tmp", std::process::id()));
    let json = serde_json::to_vec_pretty(doc).map_err(|e| std::io::Error::other(e.to_string()))?;
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, &final_path)
}

/// Read the persisted connection state, if any (`None` if the file is absent
/// — a body that has never run, or one whose state was already torn down by a
/// clean `detach`).
pub fn read(state_root: &Path) -> Option<ConnectionState> {
    let bytes = std::fs::read(path(state_root)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Remove the state file (part of a clean detach's teardown). A missing file
/// is a no-op success.
pub fn clear(state_root: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path(state_root)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #182
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_write_and_read() {
        let dir = tempfile::tempdir().expect("tempdir");
        let doc = ConnectionState {
            state: ConnState::Connected,
            since: 100,
            last_frame_at: Some(105),
            attempt: 0,
        };
        write(dir.path(), &doc).expect("write");
        let back = read(dir.path()).expect("state file present");
        assert_eq!(back.state, ConnState::Connected);
        assert_eq!(back.attempt, 0);
    }

    #[test]
    fn absent_file_reads_as_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(read(dir.path()).is_none());
    }
}
