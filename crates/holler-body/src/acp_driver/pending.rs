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

use agent_client_protocol::schema::v2;
use agent_client_protocol::Responder;
use holler_proto::docs::{PendingItem, PendingKind};

use super::answerable::OptionSet;
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
pub(super) enum PendingResponder {
    Permission(Responder<v2::RequestPermissionResponse>),
    Elicitation(Responder<v2::CreateElicitationResponse>),
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

/// Build the pending fields + unsupported-reason for one
/// `session/request_permission` request: a single field whose options are the
/// request's own `options`, in order.
pub(super) fn permission_fields(
    request: &v2::RequestPermissionRequest,
) -> (Vec<PendingField>, Option<String>) {
    let options: Vec<(String, String)> = request
        .options
        .iter()
        .map(|o| (o.option_id.0.to_string(), o.name.clone()))
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
            options: OptionSet::new(options),
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
