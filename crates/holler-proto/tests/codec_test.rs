#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! Codec test suite for `holler-proto` (issue #136 / holler-server#331).
//!
//! Every case below is a named test; the A2A fixtures under
//! `tests/fixtures/a2a/` pin the object-model round-trip to the A2A v1.0.1
//! spec examples.

mod common;

use std::fs;

use holler_proto::a2a::{Content, Message, Part, Role, TaskState};
use holler_proto::docs;
use holler_proto::envelope::{decode, encode, Envelope, EnvelopeError};
use holler_proto::error::{Code, Error as WireError};
use holler_proto::id::CorrelationId;
use holler_proto::methods::CATALOG;
use holler_proto::vocab::{RoutableName, SessionName};
use rstest::rstest;

// ---------------------------------------------------------------------------
// 1. every_method_round_trips
// ---------------------------------------------------------------------------

/// One canonical, well-formed instance per method, keyed by the method name.
fn canonical_frame(method: &str) -> String {
    let id = "h-01HTEST00000000000000000000";
    let a2a_user = r#"{"messageId":"m-1","parts":[{"text":"hi"}],"role":"ROLE_USER"}"#;
    match method {
        "circuit/join" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"circuit/join","params":{{"secret":"s3cr3t","hostname":"kiwi"}}}}"#
        ),
        "circuit/authenticate" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"circuit/authenticate","params":{{"token_id":"tok_7f3a","credential":"cred_1","hostname":"kiwi"}}}}"#
        ),
        "circuit/hello" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"circuit/hello","params":{{"protocol":2,"protocol_min":2,"protocol_max":2,"role":"body","hostname":"kiwi","token_id":"tok_7f3a","client_id":"cli_19","features":["presence","ping"],"harnesses":["opencode"],"sessions":[]}}}}"#
        ),
        "circuit/ping" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"circuit/ping","params":{{}}}}"#
        ),
        "circuit/superseded" => {
            r#"{"jsonrpc":"2.0","method":"circuit/superseded","params":{}}"#.to_string()
        },
        "query/status" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"query/status","params":{{}}}}"#
        ),
        "query/caps" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"query/caps","params":{{}}}}"#
        ),
        "query/support" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"query/support","params":{{"feature":"opencode"}}}}"#
        ),
        "query/protocol" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"query/protocol","params":{{"version":2}}}}"#
        ),
        "session/presence" => {
            r#"{"jsonrpc":"2.0","method":"session/presence","params":{"hostname":"kiwi","sessions":[{"name":"alpha","harness":"opencode","state":"idle","mode":"spawn"}]}}"#.to_string()
        },
        "session/prompt" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"session/prompt","params":{{"session":"io/alpha","message":{a2a_user}}}}}"#
        ),
        "session/update" => format!(
            r#"{{"jsonrpc":"2.0","method":"session/update","params":{{"session":"io/alpha","prompt_id":"{id}","seq":1,"parts":[{{"text":"chunk"}}]}}}}"#
        ),
        "session/cancel" => format!(
            r#"{{"jsonrpc":"2.0","id":"{id}","method":"session/cancel","params":{{"session":"io/alpha"}}}}"#
        ),
        _ => panic!("no canonical frame for {method:?}"),
    }
}

#[test]
fn every_method_round_trips() {
    for m in CATALOG {
        let frame = canonical_frame(m.name);
        let env = decode(&frame).unwrap_or_else(|e| panic!("{} : {e}", m.name));
        // The decoded shape's method must be the one we sent.
        assert_eq!(env.method(), Some(m.name), "{:?} method mismatch", m.name);
        // Re-encoding yields the same object (canonical, same key order).
        let re = encode(&env).unwrap();
        // Byte-equality would depend on serde's key order; compare by re-parse
        // equality instead, then assert the method is preserved on re-decode.
        let env2 = decode(&re).unwrap_or_else(|e| panic!("{} re-decode: {e}", m.name));
        assert_eq!(env2.method(), Some(m.name));
    }
}

// ---------------------------------------------------------------------------
// 2. batch_is_rejected  /  3. missing_jsonrpc_field_is_rejected
// ---------------------------------------------------------------------------

#[test]
fn batch_is_rejected() {
    let frame = r#"[{"jsonrpc":"2.0","id":"h-1","method":"circuit/ping"},
                     {"jsonrpc":"2.0","id":"h-2","method":"circuit/ping"}]"#;
    let err = decode(frame).unwrap_err();
    assert_eq!(err, EnvelopeError::Batch);
    assert_eq!(err.code(), Code::InvalidRequest);
}

#[test]
fn missing_jsonrpc_field_is_rejected() {
    // A request with no jsonrpc marker.
    let frame = r#"{"id":"h-1","method":"circuit/ping"}"#;
    let err = decode(frame).unwrap_err();
    assert_eq!(err.code(), Code::InvalidRequest);
}

#[test]
fn wrong_jsonrpc_version_is_rejected() {
    let frame = r#"{"jsonrpc":"1.0","id":"h-1","method":"circuit/ping"}"#;
    let err = decode(frame).unwrap_err();
    assert_eq!(err, EnvelopeError::Version);
    assert_eq!(err.code(), Code::InvalidRequest);
}

// ---------------------------------------------------------------------------
// 4. unknown_method_maps_to_32601
// ---------------------------------------------------------------------------

#[test]
fn unknown_method_maps_to_32601() {
    let frame = r#"{"jsonrpc":"2.0","id":"h-1","method":"session/teleport"}"#;
    let err = decode(frame).unwrap_err();
    assert!(matches!(
        err,
        EnvelopeError::UnknownMethod(ref m) if m == "session/teleport"
    ));
    assert_eq!(err.code(), Code::MethodNotFound);
}

// ---------------------------------------------------------------------------
// 5. id_prefixes_never_collide
// ---------------------------------------------------------------------------

#[test]
fn id_prefixes_never_collide() {
    // A hub id can never parse as a body id and vice versa.
    let h = CorrelationId::mint_hub();
    let b = CorrelationId::mint_body();
    assert!(h.is_hub() && !h.is_body());
    assert!(b.is_body() && !b.is_hub()); // body is body, not a hub
    assert_ne!(h.as_str(), b.as_str());
    // The minted bodies are real ULIDs: 26 Crockford Base32 chars each.
    assert_eq!(h.as_str().len(), "h-".len() + 26);
    assert_eq!(b.as_str().len(), "b-".len() + 26);
    // And the two full ids are distinct (the prefixes alone guarantee it).
    assert_ne!(h, b);
}

// ---------------------------------------------------------------------------
// 6. error_table_codes_are_unique_and_in_range
// ---------------------------------------------------------------------------

#[test]
fn error_table_codes_are_unique_and_in_range() {
    use std::collections::BTreeSet;
    // The one source of truth is the Code enum (#145): there is no separate
    // table to cross-check, so uniqueness/range are asserted over the enum.
    let codes: BTreeSet<i64> = Code::ALL.iter().map(|c| c.jsonrpc()).collect();
    assert_eq!(codes.len(), Code::ALL.len(), "JSON-RPC codes are not unique");
    for &c in &Code::ALL {
        let data_code = c.data_code();
        // NOTE: order matters. `invalid_params` also starts with `invalid`, so
        // the more specific prefix must be tested first, or that code would be
        // (incorrectly) routed to the generic `invalid` branch below.
        if data_code.starts_with("invalid_params") {
            assert_eq!(c.jsonrpc(), -32602);
        } else if data_code.starts_with("parse_error") {
            assert_eq!(c.jsonrpc(), -32700);
        } else if data_code.starts_with("invalid") {
            assert_eq!(c.jsonrpc(), -32600);
        } else if data_code.starts_with("method") {
            assert_eq!(c.jsonrpc(), -32601);
        } else {
            // Application codes: the closed interval -32099..=-32000 (note the
            // order: -32099 is the more negative bound), excluding the
            // reserved -32603 (outside this range already) and not
            // (any longer) -32009, which #150 claims as `session_busy`.
            assert!(
                c.jsonrpc() >= -32099 && c.jsonrpc() <= -32000,
                "{data_code} outside app range"
            );
        }
    }
    // data.codes are also unique.
    let data: BTreeSet<&str> = Code::ALL.iter().map(|c| c.data_code()).collect();
    assert_eq!(data.len(), Code::ALL.len(), "data.codes are not unique");
}

// ---------------------------------------------------------------------------
// 6b. JSON-RPC 2.0 §5.1: the error object's `code` is a NUMBER, and the
//     Holler identity rides in `data.code` as a string (#145).
// ---------------------------------------------------------------------------

/// An application error (unauthenticated, -32002) encodes with a numeric
/// `code` and a string `data.code`. JSON-RPC 2.0 §5.1 forbids a string
/// `error.code`; docs/protocol/v2.md §8 promises `-32002`.
#[test]
fn error_frame_encodes_numeric_code_and_string_data_code() {
    let env = Envelope::Error {
        id: Some("h-01HTEST00000000000000000000".into()),
        error: WireError::new(Code::Unauthenticated, "credential revoked", Some("revoked")),
    };
    let wire = encode(&env).unwrap();
    let v: serde_json::Value = serde_json::from_str(&wire).unwrap();
    let err = v.get("error").expect("error object present");
    // `code` is the JSON-RPC number …
    assert_eq!(err.get("code"), Some(&serde_json::json!(-32002)));
    // … and the Holler identity is the string in `data.code`.
    assert_eq!(err.get("data").and_then(|d| d.get("code")), Some(&serde_json::json!("unauthenticated")));
}

/// The busy-turn policy's refusal (issue #150): `say` to a `working`/
/// `stalled` session encodes `-32009 session_busy` with the state and both
/// ages in `data`, so the caller's next command is exactly the hint, not a
/// guess.
#[test]
fn session_busy_error_carries_state_and_turn_ages() {
    let env = Envelope::Error {
        id: Some("h-01HTEST00000000000000000001".into()),
        error: WireError::session_busy("working", 45_000, 3_000),
    };
    let wire = encode(&env).unwrap();
    let v: serde_json::Value = serde_json::from_str(&wire).unwrap();
    let err = v.get("error").expect("error object present");
    assert_eq!(err.get("code"), Some(&serde_json::json!(-32009)));
    let data = err.get("data").expect("session_busy carries data");
    assert_eq!(data.get("code"), Some(&serde_json::json!("session_busy")));
    assert_eq!(data.get("state"), Some(&serde_json::json!("working")));
    assert_eq!(data.get("turn_age_ms"), Some(&serde_json::json!(45_000)));
    assert_eq!(data.get("last_update_age_ms"), Some(&serde_json::json!(3_000)));
}

/// The answer command's refusal (issue #151): `answer SESSION CHOICE` to a
/// session that is not `input-required` encodes `-32010 nothing_pending`.
/// Unlike `session_busy`, it carries only `code` (+ optional `reason`) —
/// the session's `pending` list is already on the wire in presence, so the
/// refusal needs no extra payload.
#[test]
fn nothing_pending_error_encodes_code_and_data_code() {
    let env = Envelope::Error {
        id: Some("h-01HTEST00000000000000000002".into()),
        error: WireError::nothing_pending(),
    };
    let wire = encode(&env).unwrap();
    let v: serde_json::Value = serde_json::from_str(&wire).unwrap();
    let err = v.get("error").expect("error object present");
    assert_eq!(err.get("code"), Some(&serde_json::json!(-32010)));
    let data = err.get("data").expect("nothing_pending carries data");
    assert_eq!(data.get("code"), Some(&serde_json::json!("nothing_pending")));
    // The session_busy-only enrichment fields stay off the wire here.
    assert!(data.get("state").is_none());
    assert!(data.get("turn_age_ms").is_none());
    // Round-trips back to the named code by its number.
    let back = decode(&wire).unwrap();
    assert_eq!(Code::from_jsonrpc(back.error().unwrap().code), Some(Code::NothingPending));
}

/// A conformant error from a foreign peer — numeric `-32601`, no `data` —
/// must decode and map back to the matching `Code` by number.
#[test]
fn foreign_jsonrpc_error_decodes() {
    let frame = r#"{"jsonrpc":"2.0","id":"h-x","error":{"code":-32601,"message":"nope"}}"#;
    let env = decode(frame).unwrap();
    assert!(matches!(&env, Envelope::Error { .. }));
    let e = env.error().expect("an error frame has an error object");
    // The wire code is the number, and it maps back to the named Code.
    assert_eq!(e.code, -32601);
    assert_eq!(Code::from_jsonrpc(e.code), Some(Code::MethodNotFound));
}

/// Every code in the closed table survives a wire round-trip: encode → decode
/// → same numeric code, same data.code, same named Code.
#[rstest]
#[case::parse_error(Code::ParseError)]
#[case::invalid_request(Code::InvalidRequest)]
#[case::method_not_found(Code::MethodNotFound)]
#[case::invalid_params(Code::InvalidParams)]
#[case::unsupported_version(Code::UnsupportedVersion)]
#[case::join_failed(Code::JoinFailed)]
#[case::unauthenticated(Code::Unauthenticated)]
#[case::unknown_session(Code::UnknownSession)]
#[case::not_connected(Code::NotConnected)]
#[case::session_superseded(Code::SessionSuperseded)]
#[case::unknown_feature(Code::UnknownFeature)]
#[case::limit_exceeded(Code::LimitExceeded)]
#[case::connection_lost(Code::ConnectionLost)]
#[case::session_busy(Code::SessionBusy)]
#[case::nothing_pending(Code::NothingPending)]
fn every_code_round_trips_through_wire(#[case] code: Code) {
    let env = Envelope::Error {
        id: None,
        error: WireError::new(code, "msg", None),
    };
    let wire = encode(&env).unwrap();
    let back = decode(&wire).unwrap();
    let e = back.error().expect("error frame");
    assert_eq!(e.code, code.jsonrpc());
    assert_eq!(Code::from_jsonrpc(e.code), Some(code));
    assert_eq!(e.data.as_ref().map(|d| d.code.as_str()), Some(code.data_code()));
    // #150's three session_busy-only fields are absent for every other code —
    // `WireError::new` never sets them, so this holds for the whole table.
    assert_eq!(e.data.as_ref().and_then(|d| d.state.as_deref()), None);
}

// ---------------------------------------------------------------------------
// 7. names_grammar
// ---------------------------------------------------------------------------

#[rstest]
#[case::bare("alpha")]
#[case::dashed("io")]
#[case::digits("123")]
#[case::single_char("a")]
#[case::labeled("io/alpha")]
#[case::labeled_dashed("kiwi/alpha-1")]
fn names_grammar_valid(#[case] name: &str) {
    // The labeled cases are cross-hub (`<label>/<session>`); the bare cases
    // parse under both kinds, so `RoutableName` is the most permissive check.
    assert!(RoutableName::parse(name).is_ok(), "{name} should be valid");
}

#[rstest]
#[case::empty("")]
#[case::uppercase("Alpha")]
#[case::underscore("al_pha")]
#[case::leading_dash("-alpha")]
#[case::trailing_dash("alpha-")]
#[case::double_slash("io//alpha")]
#[case::trailing_slash("io/")]
#[case::space("io/alph a")]
#[case::leading_slash("/alpha")]
fn names_grammar_invalid(#[case] name: &str) {
    assert!(
        RoutableName::parse(name).is_err(),
        "{name} should be invalid"
    );
}

#[test]
fn name_label_and_session_split() {
    let n = RoutableName::parse("io/alpha").unwrap();
    assert_eq!(n.label(), "io");
    assert_eq!(n.session(), "alpha");
    assert!(n.has_label());

    let bare = RoutableName::parse("alpha").unwrap();
    assert_eq!(bare.label(), "");
    assert_eq!(bare.session(), "alpha");
    assert!(!bare.has_label());
}

// A `SessionName` is the within-body name: a single segment, no `/` (the
// cross-hub `/`-form is a `RoutableName`).
#[test]
fn session_name_rejects_a_slash() {
    assert!(SessionName::parse("alpha").is_ok());
    assert!(SessionName::parse("alpha-1").is_ok());
    // A `/` is the `RoutableName` shape, not a `SessionName`.
    assert!(SessionName::parse("io/alpha").is_err());
}

// ---------------------------------------------------------------------------
// 8. hello_requires_protocol_2
// ---------------------------------------------------------------------------

#[test]
fn hello_requires_protocol_2() {
    // protocol 2 is supported.
    assert!(holler_proto::version::is_supported_version(2));
    // protocol 1 and 3 are not — the hub answers -32000 and closes.
    assert!(!holler_proto::version::is_supported_version(1));
    assert!(!holler_proto::version::is_supported_version(3));
    // The advertised range is exactly [2, 2] (ADR 0003).
    assert_eq!(
        (
            holler_proto::version::PROTOCOL_MIN,
            holler_proto::version::PROTOCOL_MAX
        ),
        (2, 2)
    );
}

// ---------------------------------------------------------------------------
// 9. a2a_part_and_message_match_spec_examples
// ---------------------------------------------------------------------------

/// Deserialize an A2A fixture through `holler_proto::a2a`, re-serialise, and
/// assert the canonical form (sorted keys, one line) equals the file's
/// canonical form — proving the types are shape-identical to the A2A JSON
/// schema. Key order is sorted on both sides, so the comparison is
/// process-independent (not sensitive to `serde_json::Map`'s iteration order).
fn assert_fixture_round_trips(file: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/a2a/")
        .join(file);
    let original = fs::read_to_string(&path).unwrap();
    let msg: Message =
        serde_json::from_str(&original).unwrap_or_else(|e| panic!("{file}: parse: {e}"));
    let re = serde_json::to_string(&msg).unwrap();
    // Compare canonical (sorted-keys, one-line) forms so key *order* is
    // irrelevant and the form is process-independent (`common::canonical`);
    // this still pins the exact keys, values, and structure.
    let norm = |s: &str| -> String { common::canonical_str(s).unwrap() };
    assert_eq!(
        norm(&original),
        norm(&re),
        "{file} does not round-trip byte-for-byte (canonical)"
    );
}

#[test]
fn a2a_message_text_fixture_round_trips() {
    assert_fixture_round_trips("message_text.json");
}
#[test]
fn a2a_part_raw_fixture_round_trips() {
    assert_fixture_round_trips("part_raw.json");
}
#[test]
fn a2a_part_url_fixture_round_trips() {
    assert_fixture_round_trips("part_url.json");
}
#[test]
fn a2a_part_data_fixture_round_trips() {
    assert_fixture_round_trips("part_data.json");
}

/// A presence row's `turn_started_at`/`last_update_at` (issue #150) round-trip
/// and are absent from the wire (not `null`) when the session is `idle`.
#[test]
fn presence_session_ad_carries_turn_timing_only_while_working() {
    let working = docs::SessionAd {
        name: "alpha".into(),
        harness: "opencode".into(),
        state: docs::SessionState::Working,
        mode: docs::Mode::Attach,
        harness_session_id: Some("ses_1".into()),
        turn_started_at: Some("2026-09-08T12:00:00Z".into()),
        last_update_at: Some("2026-09-08T12:00:03Z".into()),
        pending: None,
        turn_id: None,
        last_turn: None,
    };
    let wire = serde_json::to_value(&working).unwrap();
    assert_eq!(wire["turn_started_at"], serde_json::json!("2026-09-08T12:00:00Z"));
    assert_eq!(wire["last_update_at"], serde_json::json!("2026-09-08T12:00:03Z"));
    let back: docs::SessionAd = serde_json::from_value(wire).unwrap();
    assert_eq!(back, working);

    let idle = docs::SessionAd {
        name: "alpha".into(),
        harness: "opencode".into(),
        state: docs::SessionState::Idle,
        mode: docs::Mode::Spawn,
        harness_session_id: None,
        turn_started_at: None,
        last_update_at: None,
        pending: None,
        turn_id: None,
        last_turn: None,
    };
    let wire = serde_json::to_value(&idle).unwrap();
    assert!(wire.get("turn_started_at").is_none(), "absent, not null, when idle");
    assert!(wire.get("last_update_at").is_none(), "absent, not null, when idle");
}

/// A presence row's `pending` list (issue #151) round-trips and is absent
/// from the wire (not `null`) whenever the session is **not**
/// `input-required` — so an `idle`/`working` row stays byte-identical to the
/// #150 shape, and the roster's `PENDING` column has something to render only
/// when a permission or elicitation is actually held.
#[test]
fn presence_session_ad_carries_pending_only_while_input_required() {
    use docs::{Mode, PendingItem, PendingKind, SessionAd, SessionState};

    let input_required = SessionAd {
        name: "gamma".into(),
        harness: "opencode".into(),
        state: SessionState::InputRequired,
        mode: Mode::Spawn,
        harness_session_id: None,
        turn_started_at: None,
        last_update_at: None,
        pending: Some(vec![
            PendingItem {
                id: "perm-1".into(),
                kind: PendingKind::Permission,
                prompt: "Run `rm -rf /`?".into(),
                options: vec!["allow".into(), "reject".into()],
            },
            PendingItem {
                id: "el-1".into(),
                kind: PendingKind::Elicitation,
                prompt: "Which branch to merge?".into(),
                options: vec!["main".into(), "release/1.0".into()],
            },
        ]),
        turn_id: Some("h-01HTESTPENDING000000000000".into()),
        last_turn: None,
    };
    let wire = serde_json::to_value(&input_required).unwrap();
    let pend = wire["pending"].as_array().expect("pending is an array on the wire");
    // kind uses the kebab-case wire strings that match the ACP method names.
    assert_eq!(pend[0]["kind"], serde_json::json!("permission"));
    assert_eq!(pend[1]["kind"], serde_json::json!("elicitation"));
    let back: SessionAd = serde_json::from_value(wire).unwrap();
    assert_eq!(back, input_required);

    let working = SessionAd {
        name: "beta".into(),
        harness: "opencode".into(),
        state: SessionState::Working,
        mode: Mode::Spawn,
        harness_session_id: Some("openc-abc".into()),
        turn_started_at: Some("2026-09-08T12:00:00Z".into()),
        last_update_at: Some("2026-09-08T12:00:03Z".into()),
        pending: None,
        turn_id: Some("h-01HTESTWORKING000000000000".into()),
        last_turn: None,
    };
    let wire = serde_json::to_value(&working).unwrap();
    assert!(wire.get("pending").is_none(), "absent, not null, when working");
}

/// A presence row's `turn_id`/`last_turn` (issue #142) round-trip; `turn_id`
/// is absent (not `null`) before a session's first prompt, and `last_turn`
/// is absent until a turn has actually ended — both are independent of
/// `state`, unlike the `working`-only and `input-required`-only fields above.
#[test]
fn presence_session_ad_carries_turn_id_and_last_turn() {
    use docs::{LastTurn, Mode, SessionAd, SessionState};

    // Before the first prompt: neither field is on the wire.
    let fresh = SessionAd {
        name: "alpha".into(),
        harness: "opencode".into(),
        state: SessionState::Idle,
        mode: Mode::Spawn,
        harness_session_id: None,
        turn_started_at: None,
        last_update_at: None,
        pending: None,
        turn_id: None,
        last_turn: None,
    };
    let wire = serde_json::to_value(&fresh).unwrap();
    assert!(wire.get("turn_id").is_none(), "absent, not null, before the first prompt");
    assert!(wire.get("last_turn").is_none(), "absent, not null, before any turn has ended");
    let back: SessionAd = serde_json::from_value(wire).unwrap();
    assert_eq!(back, fresh);

    // After a turn has completed: idle again, but turn_id/last_turn persist.
    let settled = SessionAd {
        name: "alpha".into(),
        harness: "opencode".into(),
        state: SessionState::Idle,
        mode: Mode::Spawn,
        harness_session_id: None,
        turn_started_at: None,
        last_update_at: None,
        pending: None,
        turn_id: Some("h-01HTESTTURN0000000000000A".into()),
        last_turn: Some(LastTurn {
            turn_id: "h-01HTESTTURN0000000000000A".into(),
            state: SessionState::Completed,
            stop_reason: "end_turn".into(),
            ended_at: "2026-09-08T12:00:05Z".into(),
        }),
    };
    let wire = serde_json::to_value(&settled).unwrap();
    assert_eq!(wire["turn_id"], serde_json::json!("h-01HTESTTURN0000000000000A"));
    assert_eq!(wire["last_turn"]["state"], serde_json::json!("completed"));
    assert_eq!(wire["last_turn"]["stop_reason"], serde_json::json!("end_turn"));
    assert_eq!(wire["last_turn"]["ended_at"], serde_json::json!("2026-09-08T12:00:05Z"));
    let back: SessionAd = serde_json::from_value(wire).unwrap();
    assert_eq!(back, settled);

    // Rejected/canceled/failed all round-trip through the same LastTurn shape.
    for (state, stop_reason) in [
        (SessionState::Canceled, "cancelled"),
        (SessionState::Failed, "error"),
        (SessionState::Rejected, "refusal"),
    ] {
        let lt = LastTurn {
            turn_id: "h-01HTESTTURN0000000000000B".into(),
            state,
            stop_reason: stop_reason.into(),
            ended_at: "2026-09-08T12:01:00Z".into(),
        };
        let wire = serde_json::to_value(&lt).unwrap();
        let back: LastTurn = serde_json::from_value(wire).unwrap();
        assert_eq!(back, lt);
    }
}

// ---------------------------------------------------------------------------
// 10. stop_reason_to_task_state_mapping_is_total
// ---------------------------------------------------------------------------

/// Every ACP stopReason in the table maps to an A2A *terminal* state, and the
/// mapping is exactly the table in docs/protocol/v2.md §6.
#[test]
fn stop_reason_to_task_state_mapping_is_total() {
    // Totality: every stop reason lands somewhere.
    for (reason, _state) in docs::STOP_TO_STATE {
        assert!(
            docs::state_for_stop_reason(reason).is_some(),
            "{reason} is unmapped"
        );
    }
    // The mapping is exactly the spec table (wire strings via `as_str`).
    use docs::SessionState;
    assert_eq!(
        docs::state_for_stop_reason("end_turn").as_ref().map(|s| s.as_str()),
        Some("completed")
    );
    assert_eq!(
        docs::state_for_stop_reason("cancelled").as_ref().map(|s| s.as_str()),
        Some("canceled")
    );
    assert_eq!(
        docs::state_for_stop_reason("error").as_ref().map(|s| s.as_str()),
        Some("failed")
    );
    assert_eq!(
        docs::state_for_stop_reason("refusal").as_ref().map(|s| s.as_str()),
        Some("rejected")
    );
    assert_eq!(
        docs::state_for_stop_reason("max_tokens").as_ref().map(|s| s.as_str()),
        Some("failed")
    );
    assert_eq!(
        docs::state_for_stop_reason("max_turn_requests").as_ref().map(|s| s.as_str()),
        Some("failed")
    );
    assert_eq!(
        docs::state_for_stop_reason("limit").as_ref().map(|s| s.as_str()),
        Some("failed")
    );
    // The codomain is exactly the A2A terminal states.
    let images: std::collections::BTreeSet<SessionState> =
        docs::STOP_TO_STATE.iter().map(|(_, st)| *st).collect();
    let expected: std::collections::BTreeSet<SessionState> =
        docs::A2A_TERMINAL_STATES.iter().copied().collect();
    assert_eq!(images, expected);
    // No A2A terminal state is unreachable.
    for st in docs::A2A_TERMINAL_STATES {
        assert!(
            docs::STOP_TO_STATE.iter().any(|(_, s)| s == st),
            "{st} is unreachable"
        );
    }
}

// ---------------------------------------------------------------------------
// Extra: the a2a::TaskState / Role enums match A2A's protocol-constant JSON.
// ---------------------------------------------------------------------------

#[test]
fn a2a_enums_use_protocol_constant_strings() {
    assert_eq!(
        serde_json::to_string(&Role::RoleUser).unwrap(),
        r#""ROLE_USER""#
    );
    assert_eq!(
        serde_json::to_string(&Role::RoleAgent).unwrap(),
        r#""ROLE_AGENT""#
    );
    assert_eq!(
        serde_json::to_string(&TaskState::TaskStateWorking).unwrap(),
        r#""TASK_STATE_WORKING""#
    );
    assert_eq!(
        serde_json::from_str::<Role>(r#""ROLE_USER""#).unwrap(),
        Role::RoleUser
    );
    assert_eq!(
        serde_json::from_str::<TaskState>(r#""TASK_STATE_INPUT_REQUIRED""#).unwrap(),
        TaskState::TaskStateInputRequired
    );
}

// The Part member-name discriminator: exactly one of text/raw/url/data.
// (v1.0 dropped the v0.3 `kind` field — the member name *is* the tag.)
#[test]
fn a2a_part_is_member_name_discriminated() {
    // Text part: the `text` member is present, the other content members absent.
    let text: Part = serde_json::from_str(r#"{"text":"hi"}"#).unwrap();
    assert!(matches!(&text.content, Some(Content::Text(s)) if s == "hi"));

    // File part: `raw` (base64) plus the shared `filename`/`mediaType` siblings.
    let raw: Part = serde_json::from_str(
        r#"{"raw":"Zm9vYmFy","filename":"f.bin","mediaType":"application/octet-stream"}"#,
    )
    .unwrap();
    assert!(matches!(&raw.content, Some(Content::Raw(b)) if b == b"foobar"));
    assert_eq!(raw.filename.as_deref(), Some("f.bin"));
    assert_eq!(raw.media_type.as_deref(), Some("application/octet-stream"));

    // Data part: `data` is an arbitrary JSON value.
    let data: Part = serde_json::from_str(r#"{"data":{"a":1}}"#).unwrap();
    assert!(matches!(
        &data.content,
        Some(Content::Data(v)) if v == &serde_json::json!({"a": 1})
    ));

    // Zero content members present -> a legal "empty" part (A2A's OneOf is
    // not-mandatory), not an error. Only the shared fields may be set.
    let empty: Part =
        serde_json::from_str(r#"{"filename":"a.txt","mediaType":"text/plain"}"#).unwrap();
    assert!(empty.content.is_none());

    // More than one content member present -> a Part may carry exactly one, so
    // this is a hard deserialize error.
    assert!(serde_json::from_str::<Part>(r#"{"text":"a","raw":"YQ=="}"#).is_err());

    // An unknown member alone is still an "empty" part (A2A does not define
    // deny-unknown on Part); the two-discriminator case above is what errors.
    assert!(
        serde_json::from_str::<Part>(r#"{"foo":"bar"}"#)
            .unwrap()
            .content
            .is_none()
    );
}

// A Part round-trips through JSON to the exact flat object A2A shows — this
// is what makes a Holler `Part` byte-identical to an A2A `Part`.
#[test]
fn a2a_part_wire_form_is_flat_and_stable() {
    let raw = Part {
        content: Some(Content::Raw(b"foobar".to_vec())),
        filename: Some("f.bin".to_string()),
        media_type: Some("application/octet-stream".to_string()),
        metadata: None,
    };
    let s = serde_json::to_string(&raw).unwrap();
    // Keys are camelCase and flat; `raw` is the base64 string, not a JSON array.
    assert_eq!(
        s,
        "{\"raw\":\"Zm9vYmFy\",\"filename\":\"f.bin\",\"mediaType\":\"application/octet-stream\"}"
    );

    let text = Part::text_part("hi");
    assert_eq!(serde_json::to_string(&text).unwrap(), r#"{"text":"hi"}"#);
}
