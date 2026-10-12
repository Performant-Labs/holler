//! Delivery to a Claude Code process's inbox socket (ADR-0021 section 15 item 7; plan review
//! W8): where the socket is, and the one frame written to it.
//!
//! **Where.** The launch's `SessionStart` hook wrote the socket's path to `<socket dir>/<pid>`
//! (`argv::SOCKET_HOOK`). The reader fails closed: the directory must be a directory of the
//! user's alone (no group or other permission bits) and the file a regular file of the user's
//! alone, not a link, holding one absolute path. Anything else, a missing file included, is
//! `unavailable`, and no path is guessed: neither Claude Code's own `sessions/*.json` nor a
//! `/tmp/cc-socks/<pid>.sock` is read (the spike's decision 4). The caller trusts the file only
//! for a process that `claude agents --json` lists now, so a file a killed process left behind
//! is never written to; a pid reused by another Claude Code process rewrote it at its own
//! `SessionStart`.
//!
//! **What.** One line, `{"type":"user","message":{"role":"user","content":TEXT}}`: the frame the
//! client prints in its own startup log, read from it and not documented, so the opt-in real
//! test is its guard. JSON escapes every newline of `TEXT`, so the frame is one line; the seam
//! ends it.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use holler_pane::PaneError;
use serde_json::json;

/// The permission bits a file or directory of the user's alone never has.
const SHARED_BITS: u32 = 0o077;

/// The socket path the hook recorded for the process `pid`, under `dir`.
pub(crate) fn socket_of(dir: &Path, pid: u32) -> Result<PathBuf, PaneError> {
    let unusable = |why: &str| PaneError::Unavailable {
        what: format!(
            "no inbox socket is known for the Claude Code process {pid}: {why}; the pane was not \
             launched by holler, or its launch hook has not run"
        ),
    };
    let dir_meta = fs::symlink_metadata(dir).map_err(|_| unusable("no socket directory"))?;
    if !dir_meta.is_dir() || dir_meta.permissions().mode() & SHARED_BITS != 0 {
        return Err(unusable("the socket directory is not the user's alone"));
    }
    let file = dir.join(pid.to_string());
    let meta = fs::symlink_metadata(&file).map_err(|_| unusable("no file for it"))?;
    if !meta.is_file() || meta.permissions().mode() & SHARED_BITS != 0 {
        return Err(unusable(
            "its file is not a regular file of the user's alone",
        ));
    }
    let text = fs::read_to_string(&file).map_err(|_| unusable("its file cannot be read"))?;
    let path = PathBuf::from(text.trim_end_matches('\n'));
    if !path.is_absolute() || text.trim_end_matches('\n').contains('\n') {
        return Err(unusable("its file does not hold one absolute path"));
    }
    Ok(path)
}

/// The one line that delivers `text` as a message.
pub(crate) fn frame(text: &str) -> String {
    json!({"type": "user", "message": {"role": "user", "content": text}}).to_string()
}
