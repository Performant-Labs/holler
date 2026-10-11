//! Pure wire shapes and resolution logic for [`super::HttpAttachDriver`]
//! (issue #194) — no networking here, just the serde structs OpenCode's real
//! HTTP surface returns and the pure functions that turn a `holler-server
//! answer` `choice` into the exact reply body OpenCode expects. Split out of
//! `http_attach_driver.rs`/`connection.rs` so both can use it and so these
//! pure resolvers are unit-testable without a fake server at all.
//!
//! Route names/shapes are pinned from the legacy `holler-client`
//! `src/http_attach_driver.rs` (issue #100/#133/#382), verified there live
//! against a real `opencode serve` v1.18.20 — this port was not
//! independently re-verified against a live `opencode serve` in this
//! environment (no `opencode` binary was available); see this crate's PR
//! description for that caveat.

use serde::Deserialize;

/// Which real OpenCode endpoint a [`PendingBlock`] answers against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockKind {
    Question,
    Permission,
}

/// One currently-pending question or permission for the attached session (or
/// one of its child sessions — issue #382's child-session caveat), as last
/// observed by the background poll loop. `options` is only populated for a
/// question — one inner `Vec` per question in the request, in order (almost
/// always a single entry) — the exact labels `answer` resolves each
/// comma-separated `choice` segment against. A permission's reply vocabulary
/// is the fixed `once`/`always`/`reject` enum, not an options list, so it
/// stays empty for that kind.
#[derive(Debug, Clone)]
pub(super) struct PendingBlock {
    pub(super) kind: BlockKind,
    pub(super) id: String,
    pub(super) session_id: String,
    /// Human-readable question text (the first question's `question` field
    /// for a multi-question request, or OpenCode's own permission
    /// description) — surfaced on [`holler_proto::docs::PendingItem::prompt`].
    pub(super) prompt: String,
    pub(super) options: Vec<Vec<String>>,
}

/// A single global SSE event's shape this driver cares about. OpenCode's
/// event payload has many more fields/variants than this; serde's default
/// behavior (ignore unknown fields, since `deny_unknown_fields` is not set)
/// keeps this forward-compatible with fields this driver doesn't need.
#[derive(Debug, Deserialize)]
pub(super) struct OcEvent {
    #[serde(rename = "type")]
    pub(super) kind: String,
    #[serde(default)]
    pub(super) properties: OcEventProperties,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct OcEventProperties {
    #[serde(rename = "sessionID")]
    pub(super) session_id: Option<String>,
    pub(super) info: Option<OcMessageInfo>,
    pub(super) part: Option<OcPart>,
    pub(super) status: Option<OcStatus>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OcMessageInfo {
    pub(super) id: String,
    pub(super) role: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct OcPart {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) text: Option<String>,
    #[serde(rename = "messageID")]
    pub(super) message_id: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct OcStatus {
    #[serde(rename = "type")]
    pub(super) kind: String,
}

/// One pending item from `GET {endpoint}/question` (issue #133/#382), per
/// OpenCode's real `QuestionRequest` schema — the JSON field is `id`, not
/// `requestID` (that name is only the URL path parameter on the reply
/// endpoint).
#[derive(Debug, Deserialize)]
pub(super) struct OcQuestionRequest {
    pub(super) id: String,
    #[serde(rename = "sessionID")]
    pub(super) session_id: String,
    pub(super) questions: Vec<OcQuestionInfo>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OcQuestionInfo {
    #[serde(default)]
    pub(super) question: String,
    pub(super) options: Vec<OcQuestionOption>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OcQuestionOption {
    pub(super) label: String,
}

/// One pending item from `GET {endpoint}/permission` (issue #133/#382).
/// Only `id`/`sessionID` are needed — the reply vocabulary
/// (`once`/`always`/`reject`) is fixed, not derived from anything else in
/// this shape.
#[derive(Debug, Deserialize)]
pub(super) struct OcPermissionRequest {
    pub(super) id: String,
    #[serde(rename = "sessionID")]
    pub(super) session_id: String,
    #[serde(default)]
    pub(super) permission: String,
}

/// Maps a `holler-server answer` `choice` to OpenCode's real permission
/// reply enum (`once`/`always`/`reject`), accepting a few obvious
/// operator-facing aliases since the wire's `choice` is free text.
pub(super) fn normalize_permission_reply(choice: &str) -> Option<&'static str> {
    match choice.trim().to_ascii_lowercase().as_str() {
        "once" | "allow" | "approve" | "yes" | "y" => Some("once"),
        "always" => Some("always"),
        "reject" | "deny" | "no" | "n" => Some("reject"),
        _ => None,
    }
}

/// Resolves one `choice` segment against a single question's real option
/// labels: a 0-based numeric index into `options`, or an exact
/// (case-insensitive) label match. Returns the real label OpenCode expects
/// on the wire (`options`' own casing), not the caller's input.
fn resolve_question_choice(choice: &str, options: &[String]) -> Option<String> {
    if let Ok(index) = choice.trim().parse::<usize>() {
        if let Some(label) = options.get(index) {
            return Some(label.clone());
        }
    }
    options
        .iter()
        .find(|label| label.eq_ignore_ascii_case(choice.trim()))
        .cloned()
}

/// Resolves a `holler-server answer` `choice` against every pending question
/// in order (issue #139): comma-separated for more than one question
/// (`"Yes,2,No"` for a 3-question request), a bare single value for the
/// overwhelmingly common one-question case (no comma required). Fails
/// closed — `None` — the moment the segment count doesn't match
/// `options.len()`, or any individual segment doesn't resolve against its
/// own question's options; a partial answer is never sent.
pub(super) fn resolve_question_choices(
    choice: &str,
    options: &[Vec<String>],
) -> Option<Vec<String>> {
    let segments: Vec<&str> = choice.split(',').collect();
    if segments.len() != options.len() {
        return None;
    }
    segments
        .iter()
        .zip(options.iter())
        .map(|(segment, question_options)| resolve_question_choice(segment, question_options))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #194
mod tests {
    use super::*;

    #[test]
    fn normalize_permission_reply_accepts_aliases() {
        assert_eq!(normalize_permission_reply("once"), Some("once"));
        assert_eq!(normalize_permission_reply("Allow"), Some("once"));
        assert_eq!(normalize_permission_reply("approve"), Some("once"));
        assert_eq!(normalize_permission_reply("yes"), Some("once"));
        assert_eq!(normalize_permission_reply("y"), Some("once"));
        assert_eq!(normalize_permission_reply("always"), Some("always"));
        assert_eq!(normalize_permission_reply("reject"), Some("reject"));
        assert_eq!(normalize_permission_reply("deny"), Some("reject"));
        assert_eq!(normalize_permission_reply("no"), Some("reject"));
        assert_eq!(normalize_permission_reply("n"), Some("reject"));
        assert_eq!(normalize_permission_reply("bogus"), None);
    }

    #[test]
    fn resolve_question_choices_single_question_by_index_or_label() {
        let options = vec![vec!["Yes".to_string(), "No".to_string()]];
        assert_eq!(
            resolve_question_choices("0", &options),
            Some(vec!["Yes".to_string()])
        );
        assert_eq!(
            resolve_question_choices("no", &options),
            Some(vec!["No".to_string()])
        );
        assert_eq!(resolve_question_choices("bogus", &options), None);
    }

    #[test]
    fn resolve_question_choices_splits_on_comma_for_multi_question() {
        let options = vec![
            vec!["Yes".to_string(), "No".to_string()],
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            vec!["No".to_string(), "Yes".to_string()],
        ];
        assert_eq!(
            resolve_question_choices("Yes,2,No", &options),
            Some(vec!["Yes".to_string(), "C".to_string(), "No".to_string()])
        );
        // Segment count mismatch fails closed, not a partial answer.
        assert_eq!(resolve_question_choices("Yes,2", &options), None);
    }
}
