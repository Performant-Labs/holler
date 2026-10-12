//! What `claude agents --json` says (ADR-0021 section 15 items 3 and 8): one entry per running
//! Claude Code process, `{cwd, kind, name, pid, sessionId, startedAt, status, waitingFor?}` (the
//! spike's shape; a background session adds keys this adapter does not read). An entry is
//! matched by its `pid` (a pane's identity) or its `sessionId` (unique per process), never by
//! its directory, which several panes can share. A dead process has no entry.
//!
//! The answer must be a JSON array, or it is `unavailable` (quoting none of it); an entry that
//! has no numeric `pid` or no string `sessionId` cannot be matched and is skipped.

use holler_pane::error::RefusalCode;
use holler_pane::ports::Activity;
use holler_pane::PaneError;
use serde_json::Value;

/// `harness-dialog-open`: the process waits on a dialog a person must answer in the pane (a
/// first-run or trust dialog, `waitingFor: "dialog open"`), so it cannot be brought up or
/// driven unattended; a launch that meets it rolls back. The adapter's own open code: a
/// refusal, exit 3.
pub const HARNESS_DIALOG_OPEN: RefusalCode = RefusalCode::from_static("harness-dialog-open");

/// The `waitingFor` of a question, which the hub cannot answer.
const INPUT_NEEDED: &str = "input needed";

/// The `waitingFor` of a dialog that holds the whole process.
const DIALOG_OPEN: &str = "dialog open";

/// One process `claude agents --json` lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) pid: u32,
    pub(crate) session: String,
    pub(crate) status: String,
    pub(crate) waiting_for: Option<String>,
}

/// The entries of `claude agents --json`'s answer `text`.
pub(crate) fn parse(text: &str) -> Result<Vec<Entry>, PaneError> {
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(text) else {
        return Err(PaneError::Unavailable {
            what: "claude agents --json did not answer a JSON array".to_owned(),
        });
    };
    Ok(items.iter().filter_map(entry).collect())
}

/// One entry, when it can be matched: a `pid` that fits a `u32`, a string `sessionId` and a
/// string `status`.
fn entry(item: &Value) -> Option<Entry> {
    let pid = u32::try_from(item.get("pid")?.as_u64()?).ok()?;
    let text = |key: &str| item.get(key).and_then(Value::as_str).map(str::to_owned);
    Some(Entry {
        pid,
        session: text("sessionId")?,
        status: text("status")?,
        waiting_for: text("waitingFor"),
    })
}

/// What the process of `entry` is doing (item 8): `idle`, `busy`, `waiting` on `input needed`
/// (a question) or on anything else a person answers (a permission prompt, a sandbox or a
/// worker request). A `waiting` on `dialog open` is [`HARNESS_DIALOG_OPEN`], and a status the
/// adapter does not know is `unavailable`: neither is ever taken for idle.
pub(crate) fn activity(entry: &Entry) -> Result<Activity, PaneError> {
    match (entry.status.as_str(), entry.waiting_for.as_deref()) {
        ("idle", _) => Ok(Activity::Idle),
        ("busy", _) => Ok(Activity::Busy),
        ("waiting", Some(INPUT_NEEDED)) => Ok(Activity::HoldingQuestion),
        ("waiting", Some(DIALOG_OPEN)) => Err(PaneError::Refused {
            code: HARNESS_DIALOG_OPEN,
            message: format!(
                "the Claude Code process {} waits on a dialog that a person must answer in its \
                 pane (a first-run or trust dialog)",
                entry.pid
            ),
        }),
        ("waiting", _) => Ok(Activity::WaitingPermission),
        _ => Err(PaneError::Unavailable {
            what: format!(
                "claude agents --json reported a status of the process {} that the adapter does \
                 not know",
                entry.pid
            ),
        }),
    }
}
