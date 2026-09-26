//! Building and resolving a "pending block" (issue #188's answerable
//! blocking): the fields a held-open `session/request_permission` or
//! `elicitation/create` request offers, and sending the eventual reply.
//!
//! Split out of `acp_driver.rs` (rather than folded into the connection
//! task's own file) to keep both under the workspace's 900-line file-size
//! guard, and because this half — "what does this request's shape mean, and
//! how do we reply to it" — has nothing to do with the connection/actor
//! plumbing in `connection.rs`.

use std::collections::BTreeMap;

use agent_client_protocol::schema::{v1, v2};
use agent_client_protocol::Responder;
use holler_proto::docs::{PendingItem, PendingKind};

use super::answerable::{OptionSet, PermissionKind};
use super::DriverError;

/// One resolvable pending field: its option set, the wire name to send it
/// back under (elicitation only — empty for a single-field permission
/// request), and whether it is a multi-select property (its resolved value is
/// wrapped as a one-element `StringArray` rather than a plain `String`).
pub(super) struct PendingField {
    pub(super) name: String,
    pub(super) options: OptionSet,
    pub(super) multi: bool,
}

/// Which typed inbound request this pending block holds open. Distinct
/// `Responder<T>` types (the SDK's own design — a `Responder` is bound to its
/// request's exact response type) mean this can't just be one generic slot.
///
/// `PermissionV1` is the ACP v1 fallback's own permission-request shape
/// (issue #362): v1 has no `elicitation/create` at all — that is a v2-only
/// extension — so a v1 session can only ever hold a permission request open.
pub(super) enum PendingResponder {
    Permission(Responder<v2::RequestPermissionResponse>),
    Elicitation(Responder<v2::CreateElicitationResponse>),
    PermissionV1(Responder<v1::RequestPermissionResponse>),
}

/// A held-open `session/request_permission` or `elicitation/create` request:
/// not auto-denied, not auto-answered — resolved only by a caller's
/// `AcpDriver::answer`, or replied `Cancelled`/`Cancel` by
/// `AcpDriver::cancel`.
pub(super) struct PendingBlock {
    pub(super) fields: Vec<PendingField>,
    pub(super) responder: PendingResponder,
    /// `Some(reason)` when this request is a shape this driver deliberately
    /// does not resolve (see the parent module doc's "Decisions I made").
    /// Still held open and still surfaced as `InputRequired` — never silently
    /// dropped — but `answer()` against it always fails closed.
    pub(super) unsupported: Option<String>,
    /// Which ACP request this is (issue #151's `session/presence.pending`
    /// needs this so the roster's `PENDING` column can say *what* is being
    /// asked).
    pub(super) kind: PendingKind,
    /// The human-readable question: the permission's own `title`, or the
    /// elicitation's own `message` (see [`permission_fields`]/
    /// [`elicitation_fields`]'s callers in `connection.rs`, which have the
    /// original request and thread it in here).
    pub(super) prompt: String,
}

/// Reply `Cancelled`/`Cancel` to a pending request (the ACP v2 cancellation
/// contract: every pending `session/request_permission` must be resolved this
/// way when the client cancels). Best-effort: a reply failure here has no
/// recovery the caller (already mid-cancel) can act on differently.
pub(super) fn reply_cancelled(responder: PendingResponder) {
    match responder {
        PendingResponder::Permission(r) => {
            let _ = r.respond(v2::RequestPermissionResponse::new(
                v2::RequestPermissionOutcome::Cancelled,
            ));
        }
        PendingResponder::Elicitation(r) => {
            let _ = r.respond(v2::CreateElicitationResponse::new(
                v2::ElicitationAction::Cancel,
            ));
        }
        PendingResponder::PermissionV1(r) => {
            let _ = r.respond(v1::RequestPermissionResponse::new(
                v1::RequestPermissionOutcome::Cancelled,
            ));
        }
    }
}

/// Send the real reply for a resolved choice: one `resolved` value per
/// `field_names` entry, in the same order `answerable::resolve_choice`
/// returned them.
pub(super) fn send_resolved_reply(
    responder: PendingResponder,
    field_names: Vec<(String, bool)>,
    resolved: Vec<String>,
) -> Result<(), DriverError> {
    match responder {
        PendingResponder::Permission(r) => {
            let option_id = resolved
                .into_iter()
                .next()
                .ok_or_else(|| DriverError::Answer("no option resolved".to_string()))?;
            r.respond(v2::RequestPermissionResponse::new(
                v2::RequestPermissionOutcome::Selected(v2::SelectedPermissionOutcome::new(
                    v2::PermissionOptionId::new(option_id),
                )),
            ))
            .map_err(|e| DriverError::Rpc(e.to_string()))
        }
        PendingResponder::Elicitation(r) => {
            let mut content: BTreeMap<String, v2::ElicitationContentValue> = BTreeMap::new();
            for ((name, multi), value) in field_names.into_iter().zip(resolved) {
                let content_value = if multi {
                    v2::ElicitationContentValue::StringArray(vec![value])
                } else {
                    v2::ElicitationContentValue::String(value)
                };
                content.insert(name, content_value);
            }
            r.respond(v2::CreateElicitationResponse::new(
                v2::ElicitationAction::Accept(
                    v2::ElicitationAcceptAction::new().content(Some(content)),
                ),
            ))
            .map_err(|e| DriverError::Rpc(e.to_string()))
        }
        PendingResponder::PermissionV1(r) => {
            let option_id = resolved
                .into_iter()
                .next()
                .ok_or_else(|| DriverError::Answer("no option resolved".to_string()))?;
            r.respond(v1::RequestPermissionResponse::new(
                v1::RequestPermissionOutcome::Selected(v1::SelectedPermissionOutcome::new(
                    v1::PermissionOptionId::new(option_id),
                )),
            ))
            .map_err(|e| DriverError::Rpc(e.to_string()))
        }
    }
}

/// Extract plain text from a content block; every other block kind (image,
/// audio, resource, …) contributes nothing to the driver's `Chunk` stream —
/// this story's scope is text streaming, not multi-modal content.
pub(super) fn chunk_text(content: &v2::ContentBlock) -> Option<String> {
    match content {
        v2::ContentBlock::Text(text) => Some(text.text.clone()),
        _ => None,
    }
}

/// The ACP v1 fallback's own [`chunk_text`] — v1's `ContentBlock` is a
/// distinct type from v2's, but shares the same `Text` variant shape.
pub(super) fn chunk_text_v1(content: &v1::ContentBlock) -> Option<String> {
    match content {
        v1::ContentBlock::Text(text) => Some(text.text.clone()),
        _ => None,
    }
}

/// The v2 SDK's permission-option kind as the resolver's [`PermissionKind`]
/// (issue #476). `Other` (a vendor or future ACP kind) and any variant a later
/// SDK adds (the enum is `#[non_exhaustive]`) map to no kind, so no shorthand
/// can ever select such an option.
fn permission_kind(kind: &v2::PermissionOptionKind) -> Option<PermissionKind> {
    match kind {
        v2::PermissionOptionKind::AllowOnce => Some(PermissionKind::AllowOnce),
        v2::PermissionOptionKind::AllowAlways => Some(PermissionKind::AllowAlways),
        v2::PermissionOptionKind::RejectOnce => Some(PermissionKind::RejectOnce),
        v2::PermissionOptionKind::RejectAlways => Some(PermissionKind::RejectAlways),
        _ => None,
    }
}

/// The ACP v1 fallback's own [`permission_kind`]: v1's `PermissionOptionKind`
/// is a distinct type (no `Other` variant, but also `#[non_exhaustive]`).
fn permission_kind_v1(kind: &v1::PermissionOptionKind) -> Option<PermissionKind> {
    match kind {
        v1::PermissionOptionKind::AllowOnce => Some(PermissionKind::AllowOnce),
        v1::PermissionOptionKind::AllowAlways => Some(PermissionKind::AllowAlways),
        v1::PermissionOptionKind::RejectOnce => Some(PermissionKind::RejectOnce),
        v1::PermissionOptionKind::RejectAlways => Some(PermissionKind::RejectAlways),
        _ => None,
    }
}

/// Build the pending fields + unsupported-reason for one
/// `session/request_permission` request: a single field whose options are the
/// request's own `options`, in order, each with its kind (issue #476).
pub(super) fn permission_fields(
    request: &v2::RequestPermissionRequest,
) -> (Vec<PendingField>, Option<String>) {
    let options: Vec<(String, String, Option<PermissionKind>)> = request
        .options
        .iter()
        .map(|o| (o.option_id.0.to_string(), o.name.clone(), permission_kind(&o.kind)))
        .collect();
    if options.is_empty() {
        return (
            Vec::new(),
            Some("session/request_permission carried zero options".to_string()),
        );
    }
    (
        vec![PendingField {
            name: String::new(),
            options: OptionSet::permission(options),
            multi: false,
        }],
        None,
    )
}

/// The ACP v1 fallback's own [`permission_fields`] — v1's
/// `RequestPermissionRequest` has no direct `title` field (only a
/// `tool_call` update, whose own optional `title` this driver's caller reads
/// separately for the pending block's `prompt`); the option-shape logic
/// itself is otherwise identical to v2's.
pub(super) fn permission_fields_v1(
    request: &v1::RequestPermissionRequest,
) -> (Vec<PendingField>, Option<String>) {
    let options: Vec<(String, String, Option<PermissionKind>)> = request
        .options
        .iter()
        .map(|o| (o.option_id.0.to_string(), o.name.clone(), permission_kind_v1(&o.kind)))
        .collect();
    if options.is_empty() {
        return (
            Vec::new(),
            Some("session/request_permission carried zero options".to_string()),
        );
    }
    (
        vec![PendingField {
            name: String::new(),
            options: OptionSet::permission(options),
            multi: false,
        }],
        None,
    )
}

/// Build the pending fields + unsupported-reason for one `elicitation/create`
/// request. Only a `Form` mode whose fields are all single-choice `enum`
/// strings or multi-select arrays resolves; anything else (a `Url` mode, or
/// any other field shape) is reported unsupported — the whole block, not just
/// the offending field, since a partial reply for a form the agent expects to
/// receive as one object is not a resolution ACP defines.
pub(super) fn elicitation_fields(
    request: &v2::CreateElicitationRequest,
) -> (Vec<PendingField>, Option<String>) {
    let form = match &request.mode {
        v2::ElicitationMode::Form(form) => form,
        v2::ElicitationMode::Url(_) => {
            return (
                Vec::new(),
                Some(
                    "elicitation/create in `url` mode is not resolvable by this driver".to_string(),
                ),
            );
        }
        v2::ElicitationMode::Other(other) => {
            return (
                Vec::new(),
                Some(format!(
                    "elicitation/create mode {:?} is not recognised",
                    other.mode
                )),
            );
        }
        // `ElicitationMode` is `#[non_exhaustive]`: a future ACP variant this
        // driver has never heard of is exactly as unsupported as `Url`.
        _ => {
            return (
                Vec::new(),
                Some("elicitation/create used an unrecognised mode".to_string()),
            );
        }
    };

    let mut fields = Vec::with_capacity(form.requested_schema.properties.len());
    // `BTreeMap` iteration is already sorted by key — the driver's own
    // documented field order for a multi-field `answer()`.
    for (name, schema) in &form.requested_schema.properties {
        match one_property_field(name, schema) {
            Ok(field) => fields.push(field),
            Err(reason) => return (Vec::new(), Some(reason)),
        }
    }
    if fields.is_empty() {
        return (
            Vec::new(),
            Some("elicitation/create form declared no properties".to_string()),
        );
    }
    (fields, None)
}

/// Build this pending block's [`PendingItem`]s (issue #151): one per
/// resolvable field (each field is independently answerable — a multi-field
/// elicitation's own `answer()` resolves one comma segment per field, in
/// this same order), or a single placeholder item when the block itself is
/// `unsupported` (held open, but with no resolvable field at all — still
/// surfaced, never silently dropped). `id` is the field's own name for a
/// named elicitation field, or its position for a permission's single
/// unnamed field / the unsupported placeholder — stable within one held
/// block, which is all `session/presence` needs it for (the wire `answer`
/// itself takes a `choice`, not this `id`).
pub(super) fn pending_items(block: &PendingBlock) -> Vec<PendingItem> {
    if block.fields.is_empty() {
        return vec![PendingItem {
            id: "0".to_string(),
            kind: block.kind,
            prompt: block.prompt.clone(),
            options: Vec::new(),
        }];
    }
    block
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| PendingItem {
            id: if field.name.is_empty() { index.to_string() } else { field.name.clone() },
            kind: block.kind,
            prompt: block.prompt.clone(),
            options: field.options.labels(),
        })
        .collect()
}

/// Resolve one elicitation form property to a [`PendingField`], or the
/// human-readable reason it is unsupported. Split out of [`elicitation_fields`]
/// purely to keep that function under the workspace's 100-line-per-function
/// guard (`clippy::too_many_lines`) — this is a single per-property `match`
/// with no state shared across properties.
fn one_property_field(
    name: &str,
    schema: &v2::ElicitationPropertySchema,
) -> Result<PendingField, String> {
    match schema {
        v2::ElicitationPropertySchema::String(s) => {
            if let Some(values) = &s.enum_values {
                let options = values.iter().map(|v| (v.clone(), v.clone())).collect();
                Ok(PendingField { name: name.to_string(), options: OptionSet::new(options), multi: false })
            } else if let Some(one_of) = &s.one_of {
                let options = one_of.iter().map(|o| (o.value.clone(), o.title.clone())).collect();
                Ok(PendingField { name: name.to_string(), options: OptionSet::new(options), multi: false })
            } else {
                Err(format!(
                    "field {name:?} is a freeform string (no enum/oneOf) — not resolvable"
                ))
            }
        }
        v2::ElicitationPropertySchema::Array(multi_select) => match &multi_select.items {
            v2::MultiSelectItems::String(items) => {
                let options = items.values.iter().map(|v| (v.clone(), v.clone())).collect();
                Ok(PendingField { name: name.to_string(), options: OptionSet::new(options), multi: true })
            }
            v2::MultiSelectItems::Titled(items) => {
                let options = items.options.iter().map(|o| (o.value.clone(), o.title.clone())).collect();
                Ok(PendingField { name: name.to_string(), options: OptionSet::new(options), multi: true })
            }
            // `MultiSelectItems::Other` and any future `#[non_exhaustive]`
            // variant are equally unsupported.
            _ => Err(format!("field {name:?} has an unrecognised multi-select item type")),
        },
        other => Err(format!(
            "field {name:?} is a {other:?} property — not resolvable (freeform, not enum/multi-select)"
        )),
    }
}

/// Issue #476: the permission shorthands (`once`/`always`/`reject`) resolve by
/// the option's ACP `kind`, for both the v2 and the v1 request shapes. These
/// build real SDK requests from JSON and go through the same
/// `permission_fields*` + `resolve_choice` path `AcpDriver::answer` uses, so
/// they pin the SDK-kind mapping and the resolver together.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #476
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::acp_driver::answerable::resolve_choice;

    /// One option of every kind; no label is a shorthand word.
    fn all_kinds() -> Value {
        json!([
            { "optionId": "opt-once",   "name": "Proceed this time",    "kind": "allow_once" },
            { "optionId": "opt-always", "name": "Proceed and remember", "kind": "allow_always" },
            { "optionId": "opt-reject", "name": "Stop this time",       "kind": "reject_once" },
            { "optionId": "opt-never",  "name": "Stop and remember",    "kind": "reject_always" }
        ])
    }

    fn sets(fields: &[PendingField]) -> Vec<OptionSet> {
        fields.iter().map(|f| f.options.clone()).collect()
    }

    fn resolve_v2(options: Value, choice: &str) -> Result<Vec<String>, String> {
        let request: v2::RequestPermissionRequest =
            serde_json::from_value(json!({ "sessionId": "s", "title": "t", "options": options }))
                .expect("a valid v2 request_permission");
        let (fields, unsupported) = permission_fields(&request);
        assert_eq!(unsupported, None);
        resolve_choice(&sets(&fields), choice)
    }

    fn resolve_v1(options: Value, choice: &str) -> Result<Vec<String>, String> {
        let request: v1::RequestPermissionRequest = serde_json::from_value(json!({
            "sessionId": "s",
            "toolCall": { "toolCallId": "tc-1" },
            "options": options
        }))
        .expect("a valid v1 request_permission");
        let (fields, unsupported) = permission_fields_v1(&request);
        assert_eq!(unsupported, None);
        resolve_choice(&sets(&fields), choice)
    }

    fn one(key: &str) -> Result<Vec<String>, String> {
        Ok(vec![key.to_string()])
    }

    #[test]
    fn v2_shorthands_select_the_option_of_the_matching_kind() {
        // AC1: once -> allow_once, always -> allow_always, reject -> reject_once.
        assert_eq!(resolve_v2(all_kinds(), "once"), one("opt-once"));
        assert_eq!(resolve_v2(all_kinds(), "always"), one("opt-always"));
        assert_eq!(resolve_v2(all_kinds(), "reject"), one("opt-reject"));
    }

    #[test]
    fn v1_shorthands_select_the_option_of_the_matching_kind() {
        // AC1 for the v1 fallback's separate `PermissionOptionKind` type.
        assert_eq!(resolve_v1(all_kinds(), "once"), one("opt-once"));
        assert_eq!(resolve_v1(all_kinds(), "always"), one("opt-always"));
        assert_eq!(resolve_v1(all_kinds(), "reject"), one("opt-reject"));
    }

    #[test]
    fn shorthand_words_match_case_insensitively_and_trimmed() {
        // Decision 1: matching is case-insensitive on the shorthand word.
        assert_eq!(resolve_v2(all_kinds(), "ONCE"), one("opt-once"));
        assert_eq!(resolve_v2(all_kinds(), " Always "), one("opt-always"));
        assert_eq!(resolve_v1(all_kinds(), "Reject"), one("opt-reject"));
    }

    #[test]
    fn reject_falls_back_to_reject_always_when_there_is_no_reject_once() {
        // AC2: with only a reject-always option, `reject` resolves to it.
        let options = json!([
            { "optionId": "opt-once",  "name": "Proceed this time", "kind": "allow_once" },
            { "optionId": "opt-never", "name": "Stop and remember", "kind": "reject_always" }
        ]);
        assert_eq!(resolve_v2(options.clone(), "reject"), one("opt-never"));
        assert_eq!(resolve_v1(options, "reject"), one("opt-never"));
    }

    #[test]
    fn always_without_an_allow_always_option_fails_closed_naming_it_and_the_labels() {
        // AC2 / decision 3: no option of the requested kind -> an error that
        // names the shorthand and lists the option labels; nothing resolves.
        let options = json!([
            { "optionId": "opt-once",   "name": "Proceed this time", "kind": "allow_once" },
            { "optionId": "opt-reject", "name": "Stop this time",    "kind": "reject_once" }
        ]);
        for err in [
            resolve_v2(options.clone(), "always").unwrap_err(),
            resolve_v1(options.clone(), "always").unwrap_err(),
        ] {
            assert!(err.contains("always"), "error must name the shorthand: {err:?}");
            assert!(err.contains("Proceed this time"), "error must list the labels: {err:?}");
            assert!(err.contains("Stop this time"), "error must list the labels: {err:?}");
        }
    }

    #[test]
    fn reject_never_resolves_to_an_allow_option() {
        // Risk: with only allow options offered, `reject` fails closed rather
        // than falling through to anything; `once`/`always` still resolve.
        let options = json!([
            { "optionId": "opt-once",   "name": "Proceed this time",    "kind": "allow_once" },
            { "optionId": "opt-always", "name": "Proceed and remember", "kind": "allow_always" }
        ]);
        assert!(resolve_v2(options.clone(), "reject").is_err());
        assert!(resolve_v1(options.clone(), "reject").is_err());
        assert_eq!(resolve_v2(options.clone(), "once"), one("opt-once"));
        assert_eq!(resolve_v1(options, "always"), one("opt-always"));
    }

    #[test]
    fn allow_shorthands_never_resolve_to_a_reject_option() {
        // The mirror of the rule above: only reject options offered.
        let options = json!([
            { "optionId": "opt-reject", "name": "Stop this time",    "kind": "reject_once" },
            { "optionId": "opt-never",  "name": "Stop and remember", "kind": "reject_always" }
        ]);
        assert!(resolve_v2(options.clone(), "once").is_err());
        assert!(resolve_v2(options.clone(), "always").is_err());
        assert_eq!(resolve_v2(options, "reject"), one("opt-reject"));
    }

    #[test]
    fn v2_unknown_kind_is_never_a_shorthand_target() {
        // Decision 7: v2's `Other(_)` (a custom or future kind) maps to no
        // kind, so no shorthand can select it; known kinds still resolve.
        let options = json!([
            { "optionId": "opt-custom", "name": "Custom thing",   "kind": "_vendor_allow_once" },
            { "optionId": "opt-reject", "name": "Stop this time", "kind": "reject_once" }
        ]);
        assert!(resolve_v2(options.clone(), "once").is_err());
        assert!(resolve_v2(options.clone(), "always").is_err());
        assert_eq!(resolve_v2(options, "reject"), one("opt-reject"));
    }

    #[test]
    fn an_option_literally_named_once_wins_over_the_allow_once_kind() {
        // AC3 / decision 2: exact key or label beats the shorthand.
        let by_label = json!([
            { "optionId": "opt-allow", "name": "Proceed", "kind": "allow_once" },
            { "optionId": "opt-lit",   "name": "Once",    "kind": "reject_always" }
        ]);
        assert_eq!(resolve_v2(by_label, "once"), one("opt-lit"));
        let by_key = json!([
            { "optionId": "opt-allow", "name": "Proceed",  "kind": "allow_once" },
            { "optionId": "once",      "name": "Whatever", "kind": "reject_once" }
        ]);
        assert_eq!(resolve_v1(by_key, "once"), one("once"));
    }

    #[test]
    fn an_index_still_wins_over_labels_and_shorthands() {
        // AC3: a 0-based index beats everything, and index answers still work.
        let options = json!([
            { "optionId": "opt-a", "name": "1",    "kind": "allow_once" },
            { "optionId": "opt-b", "name": "Stop", "kind": "reject_once" }
        ]);
        assert_eq!(resolve_v2(options.clone(), "1"), one("opt-b"));
        assert_eq!(resolve_v2(options, "0"), one("opt-a"));
    }

    #[test]
    fn codex_shaped_prompt_resolves_shorthand_and_comma_label() {
        // AC5 + AC7: the live Codex prompt (#473). `once` and the whole comma
        // label both resolve, so neither live error message can occur.
        let options = json!([
            { "optionId": "approved",             "name": "Yes, proceed",                               "kind": "allow_once" },
            { "optionId": "approved_for_session", "name": "Yes, and don't ask again for this command", "kind": "allow_always" },
            { "optionId": "abort",                "name": "No, and tell Codex what to do differently", "kind": "reject_once" }
        ]);
        for (choice, want) in [
            ("once", "approved"),
            ("always", "approved_for_session"),
            ("reject", "abort"),
            ("No, and tell Codex what to do differently", "abort"),
            ("no, and tell codex what to do differently", "abort"),
            ("Yes, proceed", "approved"),
        ] {
            match resolve_v2(options.clone(), choice) {
                Ok(keys) => assert_eq!(keys, vec![want.to_string()], "choice {choice:?}"),
                Err(e) => {
                    assert!(!e.contains("does not resolve to any of this field's options"), "{choice:?}: {e}");
                    assert!(!e.contains("comma-separated choice segment(s)"), "{choice:?}: {e}");
                    panic!("{choice:?} must resolve to {want:?}, got: {e}");
                }
            }
        }
    }

    #[test]
    fn elicitation_fields_carry_no_kinds_so_shorthands_fail_closed() {
        // AC4 at the builder: a real single-field elicitation never gets a
        // kind, so `once`/`always`/`reject` do not resolve on it.
        let request: v2::CreateElicitationRequest = serde_json::from_value(json!({
            "mode": "form",
            "sessionId": "s",
            "message": "pick",
            "requestedSchema": {
                "type": "object",
                "properties": { "choice": { "type": "string", "enum": ["yes", "no"] } },
                "required": ["choice"]
            }
        }))
        .expect("a valid form elicitation");
        let (fields, unsupported) = elicitation_fields(&request);
        assert_eq!(unsupported, None);
        for word in ["once", "always", "reject"] {
            assert!(resolve_choice(&sets(&fields), word).is_err(), "{word:?} must not resolve");
        }
        assert_eq!(resolve_choice(&sets(&fields), "no"), one("no"));
    }
}
