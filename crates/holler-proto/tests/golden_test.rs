#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! Golden wire files (issue #155 §4).
//!
//! Every wire shape has a checked-in JSON file under `tests/golden/`:
//! - `envelope/<shape>.json` — one per `Envelope` variant (request,
//!   notification, response, error), as `holler_proto::encode` emits it;
//! - `docs/<Type>.json` — one per params/result type in `docs.rs`, as serde
//!   emits it.
//!
//! A test asserts the encoder's output equals the file (canonical form).
//! **Changing the wire therefore changes a checked-in file**, visible in
//! every PR diff. Regenerate deliberately with `BLESS=1 cargo test -p
//! holler-proto --test golden_test` and review the diff.
//!
//! `foreign/` holds frames written **by hand from the JSON-RPC 2.0 and A2A
//! v1.0.1 spec text**, not produced by our encoder. Each must decode (or be
//! rejected for a documented, asserted reason). This is what would have
//! caught #145.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common::{canonical, canonical_str};
use holler_proto::a2a::{Message, Part, Role};
use holler_proto::docs::*;
use holler_proto::envelope::{decode, encode, Envelope, EnvelopeError};
use holler_proto::error::{Code, Error as WireError};
use serde_json::{json, Value};

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Compare `actual` (any JSON text) to the golden file; with `BLESS=1`,
/// (re)write the file instead.
fn assert_golden(rel: &str, actual_json: &str) {
    let path = golden_dir().join(rel);
    let actual = canonical_str(actual_json).unwrap_or_else(|e| panic!("{rel}: encoder output is not JSON: {e}"));
    if std::env::var("BLESS").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let pretty: Value = serde_json::from_str(actual_json).unwrap();
        std::fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&pretty).unwrap())).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing golden file {} (run with BLESS=1 to create it, then review the diff)", path.display()));
    let expected = canonical_str(&expected).unwrap_or_else(|e| panic!("{rel}: golden file is not JSON: {e}"));
    assert_eq!(actual, expected, "wire bytes changed for {rel} — if intended, BLESS=1 and review the golden diff");
}

// --- one canonical instance of every envelope shape --------------------------

fn id() -> String {
    "h-01HTESTGOLDEN000000000000".to_string()
}

fn envelopes() -> Vec<Envelope> {
    vec![
        Envelope::Request {
            id: id(),
            method: "circuit/ping".to_string(),
            params: Some(json!({})),
        },
        Envelope::Notification {
            method: "session/presence".to_string(),
            params: Some(json!({"hostname":"kiwi","sessions":[]})),
        },
        Envelope::Response {
            id: id(),
            result: Some(json!({"hostname":"uranus","ts":"2026-09-08T12:00:00Z"})),
        },
        Envelope::Error {
            id: Some(id()),
            error: WireError::new(Code::Unauthenticated, "credential revoked", Some("revoked")),
        },
    ]
}

#[test]
fn every_envelope_shape_has_a_golden_file() {
    let shapes: Vec<&str> = envelopes().iter().map(Envelope::shape_name).collect();
    assert_eq!(shapes, ["request", "notification", "response", "error"]);
    for shape in shapes {
        let path = golden_dir().join(format!("envelope/{shape}.json"));
        assert!(
            path.exists() || std::env::var("BLESS").is_ok(),
            "no golden file for envelope shape {shape:?} at {}",
            path.display()
        );
    }
}

#[test]
fn envelope_request_notification_response_match_golden() {
    for env in envelopes() {
        if env.shape_name() == "error" {
            continue; // covered by the (currently ignored) error test below
        }
        let wire = encode(&env).unwrap();
        assert_golden(&format!("envelope/{}.json", env.shape_name()), &wire);
        // and the golden decodes back to the same shape
        let back = decode(&wire).unwrap();
        assert_eq!(back.shape_name(), env.shape_name());
    }
}

/// The error golden file is written **by hand in the JSON-RPC-conformant
/// form** (`error.code` is a number, per JSON-RPC 2.0 §5.1). #145 made the
/// encoder emit that conformant form, so this test now runs: the golden file
/// is the spec of record, not a snapshot of the (former) bug.
#[test]
fn envelope_error_matches_golden() {
    let env = envelopes().into_iter().find(|e| e.shape_name() == "error").unwrap();
    let wire = encode(&env).unwrap();
    assert_golden("envelope/error.json", &wire);
}

// --- one instance of every docs.rs wire type ----------------------------------

fn meta() -> Option<BTreeMap<String, Value>> {
    let mut m = BTreeMap::new();
    m.insert("origin".to_string(), json!("cli"));
    Some(m)
}

fn user_message() -> Message {
    Message {
        message_id: "m-1".into(),
        context_id: Some("io/alpha".into()),
        task_id: None,
        role: Role::RoleUser,
        parts: vec![Part::text_part("reply with the word ALPHA")],
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    }
}

fn agent_message() -> Message {
    Message {
        message_id: "m-2".into(),
        context_id: Some("io/alpha".into()),
        task_id: Some(id()),
        role: Role::RoleAgent,
        parts: vec![Part::text_part("ALPHA")],
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    }
}

fn docs_instances() -> Vec<(&'static str, Value)> {
    vec![
        ("Hello.body", serde_json::to_value(Hello {
            protocol: 2, protocol_min: 2, protocol_max: 2, role: "body".into(), hostname: "kiwi".into(),
            token_id: Some("tok_7f3a".into()), client_id: Some("cli_19".into()),
            features: vec!["interrupt".into(), "presence".into(), "ping".into(), "query".into()],
            harnesses: Some(vec!["opencode".into()]), harnesses_known: None, harnesses_confirmed: None,
            sessions: Some(vec![HelloSession { name: "alpha".into(), harness: "opencode".into(), mode: "spawn".into(), harness_session_id: None }]),
        }).unwrap()),
        ("Hello.hub", serde_json::to_value(Hello {
            protocol: 2, protocol_min: 2, protocol_max: 2, role: "hub".into(), hostname: "uranus".into(),
            token_id: None, client_id: None,
            features: vec!["interrupt".into(), "presence".into(), "ping".into(), "query".into(), "roster".into(), "token".into()],
            harnesses: None, harnesses_known: Some(vec!["opencode".into()]), harnesses_confirmed: Some(vec!["opencode".into()]), sessions: None,
        }).unwrap()),
        ("Status.body", serde_json::to_value(Status {
            role: "body".into(), protocol: 2, protocol_min: 2, protocol_max: 2, hostname: "kiwi".into(),
            connected: Some(true), token_id: Some("tok_7f3a".into()), listening: None,
            features: vec!["ping".into()], harnesses: Some(vec!["opencode".into()]), harnesses_known: None, harnesses_confirmed: None,
            bodies: None, sessions: None,
            session_list: Some(vec![StatusSession { name: "alpha".into(), harness: "opencode".into(), state: "idle".into() }]),
        }).unwrap()),
        ("Status.hub", serde_json::to_value(Status {
            role: "hub".into(), protocol: 2, protocol_min: 2, protocol_max: 2, hostname: "uranus".into(),
            connected: None, token_id: None, listening: Some("ws://127.0.0.1:41807".into()),
            features: vec!["roster".into()], harnesses: None, harnesses_known: Some(vec!["opencode".into()]),
            harnesses_confirmed: Some(vec![ConfirmedHarness { id: "opencode".into(), bodies: vec!["kiwi".into()] }]),
            bodies: Some(1), sessions: Some(2), session_list: None,
        }).unwrap()),
        ("Caps", serde_json::to_value(Caps {
            status: Status {
                role: "body".into(), protocol: 2, protocol_min: 2, protocol_max: 2, hostname: "kiwi".into(),
                connected: Some(false), token_id: None, listening: None, features: vec![], harnesses: None,
                harnesses_known: None, harnesses_confirmed: None, bodies: None, sessions: None, session_list: None,
            },
            caps: BTreeMap::from([("opencode".to_string(), Support { feature: "opencode".into(), kind: "harness".into(), ok: true, how: Some("opencode acp".into()), reason: None })]),
        }).unwrap()),
        ("SupportParams", serde_json::to_value(SupportParams { feature: "opencode".into() }).unwrap()),
        ("Support.ok", serde_json::to_value(Support { feature: "opencode".into(), kind: "harness".into(), ok: true, how: Some("opencode acp".into()), reason: None }).unwrap()),
        ("Support.no", serde_json::to_value(Support { feature: "claude".into(), kind: "harness".into(), ok: false, how: None, reason: Some("no adapter".into()) }).unwrap()),
        ("ProtocolParams", serde_json::to_value(ProtocolParams { version: Some(2) }).unwrap()),
        ("ProtocolAnswer", serde_json::to_value(ProtocolAnswer { session: 2, min: 2, max: 2, asked: Some(2), ok: Some(true) }).unwrap()),
        ("Presence", serde_json::to_value(Presence {
            hostname: "kiwi".into(),
            sessions: vec![SessionAd { name: "alpha".into(), harness: "opencode".into(), state: "working".into(), mode: "attach".into(), harness_session_id: Some("ses_1".into()) }],
        }).unwrap()),
        ("Prompt", serde_json::to_value(Prompt { session: "io/alpha".into(), message: user_message(), meta: meta() }).unwrap()),
        ("PromptResult", serde_json::to_value(PromptResult { stop_reason: "end_turn".into(), state: "completed".into(), message: agent_message() }).unwrap()),
        ("Update", serde_json::to_value(Update { session: "io/alpha".into(), prompt_id: id(), seq: 1, parts: vec![Part::text_part("AL")] }).unwrap()),
        ("Cancel", serde_json::to_value(Cancel { session: "io/alpha".into() }).unwrap()),
        ("CancelResult", serde_json::to_value(CancelResult { applied: true }).unwrap()),
        ("Join", serde_json::to_value(Join { secret: "hlr_join_x".into(), hostname: "kiwi".into() }).unwrap()),
        ("JoinResult", serde_json::to_value(JoinResult { client_id: "cli_19".into(), credential: "hlr_live_x".into() }).unwrap()),
        ("Authenticate", serde_json::to_value(Authenticate { token_id: "tok_7f3a".into(), credential: "hlr_live_x".into(), hostname: "kiwi".into() }).unwrap()),
        ("AuthOk", serde_json::to_value(AuthOk { ok: true }).unwrap()),
    ]
}

#[test]
fn every_docs_wire_type_matches_golden() {
    for (name, value) in docs_instances() {
        assert_golden(&format!("docs/{name}.json"), &value.to_string());
    }
}

// --- foreign frames: hand-written from the specs, must decode -------------------

fn foreign(name: &str) -> String {
    std::fs::read_to_string(golden_dir().join("foreign").join(name)).unwrap()
}

#[test]
fn foreign_notification_without_params_decodes() {
    let env = decode(&foreign("notification_no_params.json")).unwrap();
    assert_eq!(env.shape_name(), "notification");
    assert_eq!(env.method(), Some("session/presence"));
}

#[test]
fn foreign_response_with_null_result_decodes() {
    let env = decode(&foreign("response_null_result.json")).unwrap();
    assert_eq!(env.shape_name(), "response");
}

/// JSON-RPC permits numeric ids; Holler v2 requires `h-`/`b-` prefixed
/// strings (ADR 0004). The rejection is deliberate and precise (`BadId`), not
/// a parse error — asserted so the restriction can never become accidental.
#[test]
fn foreign_numeric_id_is_rejected_as_bad_id_not_parse_error() {
    let err = decode(&foreign("request_numeric_id.json")).unwrap_err();
    assert_eq!(err, EnvelopeError::BadId);
}

/// A2A v1.0.1 `Part` with `metadata` (spec §4.1.6) decodes and re-encodes
/// byte-for-byte (canonical).
#[test]
fn foreign_a2a_part_with_metadata_round_trips() {
    let text = foreign("part_with_metadata.json");
    let part: Part = serde_json::from_str(&text).unwrap();
    assert!(part.metadata().is_some());
    let re = serde_json::to_string(&part).unwrap();
    assert_eq!(canonical_str(&re).unwrap(), canonical_str(&text).unwrap());
}

/// A conformant JSON-RPC error from a foreign peer — numeric `-32601`, no
/// `data`. Must decode (#145 made the codec accept a numeric `error.code`).
/// The wire code is the number; it maps back to the named `Code` by number.
#[test]
fn foreign_method_not_found_error_decodes() {
    let env = decode(&foreign("error_method_not_found.json")).unwrap();
    assert_eq!(env.shape_name(), "error");
    let e = env.error().expect("an error frame has an error object");
    assert_eq!(e.code, -32601);
    assert_eq!(Code::from_jsonrpc(e.code), Some(Code::MethodNotFound));
}

// Keep the canonicaliser honest: sorted keys, stable.
#[test]
fn canonical_is_key_order_independent() {
    let a: Value = serde_json::from_str(r#"{"b":1,"a":{"y":2,"x":1}}"#).unwrap();
    let b: Value = serde_json::from_str(r#"{"a":{"x":1,"y":2},"b":1}"#).unwrap();
    assert_eq!(canonical(&a), canonical(&b));
}
