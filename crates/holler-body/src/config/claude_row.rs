//! The row rule of an attach row that attaches to a Claude Code process (#850, ADR 0014 point
//! 7). Such a process has no endpoint, so the row names none: it names the session the process
//! holds, as `session_id`, in the 36-character UUID form Holler mints for a pane's session
//! (lower-case hexadecimal digits in groups of 8, 4, 4, 4 and 12, joined by hyphens). A row with
//! an `endpoint`, with `interrupt = "http"`, or with no such `session_id` is refused when the file
//! is parsed, naming the key. Whether a row is one is the parent's
//! [`super::SessionConfig::attaches_to_claude_code`].

use super::{ConfigError, RawSession};

/// The lengths of the five hyphen-separated groups of a UUID.
const UUID_GROUPS: [usize; 5] = [8, 4, 4, 4, 12];

/// Refuse the Claude Code attach row `r` (row `row` of the file) that breaks the row rule,
/// naming the first key at fault: `endpoint`, then `interrupt`, then `session_id`.
pub(super) fn check(row: usize, r: &RawSession) -> Result<(), ConfigError> {
    let refused = |field: &'static str, reason: &str| ConfigError::Invalid {
        row,
        name: r.name.clone(),
        field,
        reason: reason.to_owned(),
    };
    if r.endpoint.is_some() {
        return Err(refused(
            "endpoint",
            "a Claude Code attach row names no `endpoint`: it attaches to the process that holds \
             `session_id` (ADR 0014 point 7)",
        ));
    }
    if r.interrupt.as_deref() == Some("http") {
        return Err(refused(
            "interrupt",
            "a Claude Code attach row has no HTTP interrupt: `holler interrupt --pane` relaunches \
             its pane",
        ));
    }
    match r.session_id.as_deref() {
        Some(id) if is_uuid(id) => Ok(()),
        Some(_) => Err(refused(
            "session_id",
            "must be the pane's session of record, a lower-case UUID (8-4-4-4-12 hexadecimal \
             digits)",
        )),
        None => Err(refused("session_id", "attach mode requires `session_id`")),
    }
}

/// Whether `text` is a UUID in its 36-character lower-case form.
fn is_uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == UUID_GROUPS.len()
        && groups.iter().zip(UUID_GROUPS).all(|(group, len)| {
            group.len() == len
                && group
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

