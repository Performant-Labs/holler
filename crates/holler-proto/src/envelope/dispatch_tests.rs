//! Unit tests for the dispatch helper: projecting a call frame's raw
//! `params` into a method's typed params `T`.
//!
//! #146 asks these to run against the real `Presence` and `Prompt` params
//! types (docs §7 / §6), because those structs are `#[serde(deny_unknown_fields)]`
//! and the dispatch helper is the first code that *deserializes* them from an
//! already-decoded `Envelope` — the codec round-trip tests never exercise that
//! path (the tests build the structs directly). So this is where the
//! `deny_unknown_fields` guard on those structs is first exercised.

#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used, clippy::unreachable)] // #146

use serde_json::json;

use crate::{Code, Envelope, Mode, Presence, Prompt, Role, SessionState, typed_params};

/// A valid `session/presence` params object decodes into the typed `Presence`.
#[test]
fn typed_presence_valid() {
    let env = Envelope::Notification {
        method: "session/presence".into(),
        params: Some(json!({
            "hostname": "kiwi",
            "sessions": [{
                "name": "alpha",
                "harness": "opencode",
                "state": "working",
                "mode": "attach",
                "harness_session_id": "ses_1"
            }]
        })),
    };
    let p = typed_params::<Presence>(&env).expect("valid presence params");
    assert_eq!(p.hostname, "kiwi");
    assert_eq!(p.sessions.len(), 1);
    assert_eq!(p.sessions[0].state, SessionState::Working);
    assert_eq!(p.sessions[0].mode, Mode::Attach);
}

/// An unknown field in the `params` object is an `invalid_params` error — the
/// `deny_unknown_fields` guard the codec round-trip never hit.
#[test]
fn typed_presence_unknown_field_is_invalid_params() {
    let env = Envelope::Notification {
        method: "session/presence".into(),
        params: Some(json!({
            "hostname": "kiwi",
            "sessions": [],
            "bogusField": "not in the schema"
        })),
    };
    let err = typed_params::<Presence>(&env).expect_err("unknown field must fail");
    assert_eq!(err.code, Code::InvalidParams.jsonrpc());
}

/// A valid `session/prompt` params object (with its nested A2A `message`)
/// decodes into the typed `Prompt`.
#[test]
fn typed_prompt_valid() {
    let env = Envelope::Request {
        id: "h-01HTESTGOLDEN000000000000".into(),
        method: "session/prompt".into(),
        params: Some(json!({
            "session": "io/alpha",
            "message": {
                "messageId": "m-1",
                "contextId": "io/alpha",
                "role": "ROLE_USER",
                "parts": [{"text": "reply with the word ALPHA"}]
            }
        })),
    };
    let p = typed_params::<Prompt>(&env).expect("valid prompt params");
    assert_eq!(p.session, "io/alpha");
    assert_eq!(p.message.role, Role::RoleUser);
    assert_eq!(p.message.parts.len(), 1);
    assert!(matches!(p.message.parts[0].content, Some(crate::a2a::Content::Text(_))));
}

/// An unknown field at the `Prompt` level is an `invalid_params` error — this
/// is the first code path that actually trips `Prompt`'s
/// `#[serde(deny_unknown_fields)]` (the codec round-trip builds the struct
/// directly and never deserialises it from a peer's raw `params`).
#[test]
fn typed_prompt_unknown_field_is_invalid_params() {
    let env = Envelope::Request {
        id: "h-01HTESTGOLDEN000000000000".into(),
        method: "session/prompt".into(),
        params: Some(json!({
            "session": "io/alpha",
            "bogusField": "not in the schema",
            "message": {
                "messageId": "m-1",
                "contextId": "io/alpha",
                "role": "ROLE_USER",
                "parts": [{"text": "x"}]
            }
        })),
    };
    let err = typed_params::<Prompt>(&env).expect_err("unknown field must fail");
    assert_eq!(err.code, Code::InvalidParams.jsonrpc());
}

/// `None` params (a parameterless method) decodes from an empty object; for a
/// method that *requires* params this is an `invalid_params` error.
#[test]
fn typed_none_params_is_invalid_for_required_shape() {
    // `Presence` requires `hostname`; an empty object is not valid `Presence`.
    let env = Envelope::Notification {
        method: "session/presence".into(),
        params: None,
    };
    let err = typed_params::<Presence>(&env).expect_err("required field missing");
    assert_eq!(err.code, Code::InvalidParams.jsonrpc());
}
