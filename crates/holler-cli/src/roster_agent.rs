//! `backgroundAgentLastOutput` (issue #914, for pfleet#506): when a Claude pane's background
//! agent last produced output.
//!
//! A Claude Code pane running a sub-agent or a workflow sends the hub no `session/update` while
//! it works, so the roster's `last_update_at` ages although the pane is healthy. Claude Code
//! writes each sub-agent's transcript to `<projects>/<project>/<sessionId>/subagents/agent-*.jsonl`,
//! so the newest write time under that directory is when any background agent of the session
//! last produced output. That is all this reports. Whether an agent is *running* is not
//! reported: a cut-off agent leaves the same last row as a running one (#914's probe), and only
//! an age limit, which the reader owns, tells them apart.
//!
//! The answer is `None`, never a zero, whenever it cannot be told: no projects directory, no
//! such session, no `subagents` directory or no `agent-*.jsonl` file in it, an unreadable
//! entry, a write time before the epoch or at it, or a session id that is not a plain name.
//! The layout is the Claude CLI's own and undocumented, so a change to it reads as `None`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// The Claude projects directory of the ambient environment: `$CLAUDE_CONFIG_DIR/projects`, else
/// `$HOME/.claude/projects`. `None` when neither variable is set.
#[must_use]
pub fn ambient_projects_dir() -> Option<PathBuf> {
    let base = std::env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|dir| !dir.is_empty())
                .map(|home| PathBuf::from(home).join(".claude"))
        })?;
    Some(base.join("projects"))
}

/// Whether `id` is one plain path component, so joining it cannot leave the project directory.
fn plain_name(id: &str) -> bool {
    !id.is_empty() && id != "." && id != ".." && !id.contains(['/', '\\', '\0'])
}

/// The epoch seconds of the newest `agent-*.jsonl` under
/// `<projects>/*/<session_id>/subagents/`, or `None` (see the module docs).
#[must_use]
pub fn last_output(projects: &Path, session_id: &str) -> Option<u64> {
    if !plain_name(session_id) {
        return None;
    }
    let mut newest: Option<u64> = None;
    for project in fs::read_dir(projects).ok()?.flatten() {
        let dir = project.path().join(session_id).join("subagents");
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !(name.starts_with("agent-") && name.ends_with(".jsonl")) {
                continue;
            }
            let secs = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|age| age.as_secs())
                .filter(|secs| *secs > 0);
            newest = newest.max(secs);
        }
    }
    newest
}
