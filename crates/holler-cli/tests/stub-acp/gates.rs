//! The gate requests `stub-acp` raises mid-turn (the permission or
//! elicitation ask), split out of `main.rs` to keep that file under the
//! workspace's 900-line guard (`scripts/lint.sh`, issue #476). This module
//! only builds the request JSON; the one request writer stays
//! `super::send_request`.

use std::io::Write;

use serde_json::{json, Value};

use super::{send_request, GateKind, ELICITATION_REQUEST_ID, PERMISSION_REQUEST_ID, SESSION_ID};

/// Send the one outbound request that raises `kind`'s gate (the permission or
/// elicitation ask). Split out of `advance` purely to keep that function under
/// the workspace's 100-line-per-function guard (`clippy::too_many_lines`) —
/// this is a single `match` with no state of its own.
pub(crate) fn send_gate_request(lock: &mut impl Write, kind: GateKind) {
    match kind {
        GateKind::Permission => send_request(
            lock,
            PERMISSION_REQUEST_ID,
            "session/request_permission",
            json!({
                "sessionId": SESSION_ID,
                "title": "stub tool wants to run",
                "toolCall": { "title": "stub tool" },
                "options": [
                    { "optionId": "allow", "name": "Allow", "kind": "allow_once" },
                    { "optionId": "deny",  "name": "Deny",  "kind": "reject_once" }
                ]
            }),
        ),
        // `--ask-permission-kinds` (issue #476): one option of every ACP
        // permission kind, none labelled with a shorthand word, and a
        // reject-always option whose label carries a comma (the real Codex
        // adapter's wording). Resuming from this gate echoes the selected
        // `optionId` as a chunk (`stub selected <id>`), so a test sees which
        // option actually reached the adapter.
        GateKind::PermissionKinds => send_request(
            lock,
            PERMISSION_REQUEST_ID,
            "session/request_permission",
            json!({
                "sessionId": SESSION_ID,
                "title": "stub tool wants to run",
                "toolCall": { "title": "stub tool" },
                "options": [
                    { "optionId": "opt-once",   "name": "Proceed this time",   "kind": "allow_once" },
                    { "optionId": "opt-always", "name": "Proceed and remember", "kind": "allow_always" },
                    { "optionId": "opt-reject", "name": "Stop this time",       "kind": "reject_once" },
                    {
                        "optionId": "opt-tell",
                        "name": "No, and tell Codex what to do differently",
                        "kind": "reject_always"
                    }
                ]
            }),
        ),
        GateKind::Elicitation => send_request(
            lock,
            ELICITATION_REQUEST_ID,
            "elicitation/create",
            json!({
                "mode": "form",
                "sessionId": SESSION_ID,
                "message": "pick your options",
                "requestedSchema": {
                    "type": "object",
                    "properties": {
                        "color": { "type": "string", "enum": ["red", "blue"] },
                        "size": {
                            "type": "string",
                            "oneOf": [
                                { "const": "s", "title": "Small" },
                                { "const": "m", "title": "Medium" }
                            ]
                        }
                    },
                    "required": ["color", "size"]
                }
            }),
        ),
        GateKind::ElicitationUrl => send_request(
            lock,
            ELICITATION_REQUEST_ID,
            "elicitation/create",
            json!({
                "mode": "url",
                "sessionId": SESSION_ID,
                "elicitationId": "elic-1",
                "url": "https://example.invalid/consent",
                "message": "open this url to continue"
            }),
        ),
    }
}

/// The `optionId` a client's permission answer selected
/// (`result.outcome.optionId`), if the answer carries one.
pub(crate) fn selected_option_id(answer: &Value) -> Option<String> {
    answer
        .pointer("/result/outcome/optionId")
        .and_then(Value::as_str)
        .map(str::to_string)
}
