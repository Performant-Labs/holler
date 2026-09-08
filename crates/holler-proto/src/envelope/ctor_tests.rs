//! Unit tests for the [`Envelope`] constructors and accessors.
//!
//! The clippy lints are denied workspace-wide (`unwrap_used` / `expect_used` /
//! `panic` / `unreachable`); test code is exempt from that strictness with a
//! story link (the build guard `scripts/lint.sh` only allows a `#[allow]`
//! that carries a `// #NNN` link on the same line).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #143

use serde_json::json;

// The parent `envelope` module re-exports `decode` / `encode` /
// `typed_params` and brings `Envelope`, `EnvelopeError`, `Code`, `WireError`,
// and `CorrelationId` into scope.
use super::super::*;

/// A well-formed `h-` id (the codec accepts any non-empty `h-`/`b-` prefix).
fn id() -> CorrelationId {
    CorrelationId::parse("h-1TEST").expect("valid id")
}

#[test]
fn request_ctor_roundtrips_and_shape() {
    let env = Envelope::request(&id(), "circuit/ping", Some(json!({"n": 1})));
    assert!(matches!(env, Envelope::Request { .. }));
    assert!(env.is_call(), "a request is a call frame");
    // The `id` accessor and the encoded `id` field both carry the id.
    assert_eq!(env.id().unwrap(), "h-1TEST");
    let s = encode(&env).expect("encode");
    assert!(s.contains("\"method\":\"circuit/ping\""));
    // Re-decoding a request yields a request.
    let back = decode(&s).expect("decode");
    assert!(matches!(back, Envelope::Request { .. }));
}

#[test]
fn notification_ctor_has_no_id() {
    let env = Envelope::notification("circuit/superseded", None);
    assert!(matches!(env, Envelope::Notification { .. }));
    assert!(env.id().is_none(), "a notification carries no id");
    assert!(env.is_call(), "a notification is a call frame");
}

#[test]
fn response_ctor_carries_id_and_result() {
    let env = Envelope::response(&id(), Some(json!({"ok": true})));
    assert!(matches!(env, Envelope::Response { .. }));
    assert_eq!(env.id().unwrap(), "h-1TEST");
    assert!(env.result().is_some());
    assert!(env.error().is_none());
}

#[test]
fn error_ctor_carries_id_and_error() {
    let err = WireError::new(
        Code::InvalidRequest,
        "bad request",
        Some("the id must be h-/b- prefixed"),
    );
    let env = Envelope::error_frame(&id(), &err);
    assert!(matches!(env, Envelope::Error { .. }));
    assert_eq!(env.id().unwrap(), "h-1TEST");
    // #145: the wire `error.code` is the JSON-RPC number, not a `Code`.
    assert_eq!(env.error().unwrap().code, -32600);
    assert_eq!(
        Code::from_jsonrpc(env.error().unwrap().code),
        Some(Code::InvalidRequest),
    );
    assert!(env.result().is_none());
}
