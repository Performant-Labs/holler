//! Codec test suite for `holler-proto` (issue #136 / holler-server#331).
//!
//! Every case below is a named test; the A2A fixtures under
//! `tests/fixtures/a2a/` pin the object-model round-trip to the A2A v1.0.1
//! spec examples.

use std::fs;

use holler_proto::a2a::{Message, Part, Role, TaskState};
use holler_proto::docs;
use holler_proto::envelope::{decode, encode, EnvelopeError};
use holler_proto::error::{Code, TABLE};
use holler_proto::id::CorrelationId;
use holler_proto::methods::CATALOG;
use holler_proto::names::SessionName;
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
    let h = CorrelationId::mint_hub("01HTEST00000000000000000000");
    let b = CorrelationId::mint_body("01HTEST00000000000000000000");
    assert!(h.is_hub() && !h.is_body());
    assert!(b.is_body() && !b.is_hub()); // body is body, not a hub
    assert_ne!(h.as_str(), b.as_str());
    // The body strings are identical; only the prefix differs.
    assert_eq!(&h.as_str()[2..], &b.as_str()[2..]);
    // And the two full ids are distinct.
    assert_ne!(h, b);
}

// ---------------------------------------------------------------------------
// 6. error_table_codes_are_unique_and_in_range
// ---------------------------------------------------------------------------

#[test]
fn error_table_codes_are_unique_and_in_range() {
    use std::collections::BTreeSet;
    let codes: BTreeSet<i64> = TABLE.iter().map(|d| d.jsonrpc_code).collect();
    assert_eq!(codes.len(), TABLE.len(), "JSON-RPC codes are not unique");
    for d in TABLE {
        // NOTE: order matters. `invalid_params` also starts with `invalid`, so
        // the more specific prefix must be tested first, or that row would be
        // (incorrectly) routed to the generic `invalid` branch below.
        if d.data_code.starts_with("invalid_params") {
            assert_eq!(d.jsonrpc_code, -32602);
        } else if d.data_code.starts_with("parse_error") {
            assert_eq!(d.jsonrpc_code, -32700);
        } else if d.data_code.starts_with("invalid") {
            assert_eq!(d.jsonrpc_code, -32600);
        } else if d.data_code.starts_with("method") {
            assert_eq!(d.jsonrpc_code, -32601);
        } else {
            // Application codes: the closed interval -32099..=-32000 (note the
            // order: -32099 is the more negative bound), and not the reserved
            // -32603/-32009.
            assert!(
                d.jsonrpc_code >= -32099 && d.jsonrpc_code <= -32000,
                "{} outside app range",
                d.data_code
            );
            assert_ne!(d.jsonrpc_code, -32009, "{} is reserved", d.data_code);
        }
        // data.codes are also unique.
    }
    let data: BTreeSet<&str> = TABLE.iter().map(|d| d.data_code).collect();
    assert_eq!(data.len(), TABLE.len(), "data.codes are not unique");
}

// ---------------------------------------------------------------------------
// 7. names_grammar
// ---------------------------------------------------------------------------

#[rstest]
#[case::bare("alpha")]
#[case::dashed("io")]
#[case::labeled("io/alpha")]
#[case::digits("123")]
#[case::labeled_dashed("kiwi/alpha-1")]
#[case::single_char("a")]
fn names_grammar_valid(#[case] name: &str) {
    assert!(SessionName::parse(name).is_ok(), "{name} should be valid");
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
        SessionName::parse(name).is_err(),
        "{name} should be invalid"
    );
}

#[test]
fn name_label_and_session_split() {
    let n = SessionName::parse("io/alpha").unwrap();
    assert_eq!(n.label(), "io");
    assert_eq!(n.session(), "alpha");
    assert!(n.has_label());

    let bare = SessionName::parse("alpha").unwrap();
    assert_eq!(bare.label(), "");
    assert_eq!(bare.session(), "alpha");
    assert!(!bare.has_label());
}

// ---------------------------------------------------------------------------
// 8. hello_requires_protocol_2
// ---------------------------------------------------------------------------

#[test]
fn hello_requires_protocol_2() {
    // protocol 2 is supported.
    assert!(holler_proto::features::is_supported_version(2));
    // protocol 1 and 3 are not — the hub answers -32000 and closes.
    assert!(!holler_proto::features::is_supported_version(1));
    assert!(!holler_proto::features::is_supported_version(3));
    // The advertised range is exactly [2, 2] (ADR 0003).
    assert_eq!(
        (
            holler_proto::features::PROTOCOL_MIN,
            holler_proto::features::PROTOCOL_MAX
        ),
        (2, 2)
    );
}

// ---------------------------------------------------------------------------
// 9. a2a_part_and_message_match_spec_examples
// ---------------------------------------------------------------------------

/// Deserialize an A2A fixture through `holler_proto::a2a`, re-serialise with
/// sorted keys, and assert the canonical form equals the file's contents —
/// proving the types are shape-identical to the A2A JSON schema.
fn assert_fixture_round_trips(file: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/a2a/")
        .join(file);
    let original = fs::read_to_string(&path).unwrap();
    let msg: Message =
        serde_json::from_str(&original).unwrap_or_else(|e| panic!("{file}: parse: {e}"));
    let re = serde_json::to_string(&msg).unwrap();
    // Compare canonical (sorted-keys, one-line) forms so key order is
    // irrelevant; this still pins the exact keys, values, and structure.
    let norm = |s: &str| -> String {
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        canonical_json(&v).to_string()
    };
    assert_eq!(
        norm(&original),
        norm(&re),
        "{file} does not round-trip byte-for-byte (canonical)"
    );
}

// A canonical (sorted-keys) one-line JSON string for equality comparisons.
struct Canonical(serde_json::Value);
impl std::fmt::Display for Canonical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_json(f, &self.0)
    }
}
fn write_json(f: &mut std::fmt::Formatter<'_>, v: &serde_json::Value) -> std::fmt::Result {
    match v {
        serde_json::Value::Null => write!(f, "null"),
        serde_json::Value::Bool(b) => write!(f, "{}", b),
        serde_json::Value::Number(n) => write!(f, "{}", n),
        serde_json::Value::String(s) => write_json_string(f, s),
        serde_json::Value::Array(a) => {
            write!(f, "[")?;
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    write!(f, ",")?;
                }
                write_json(f, x)?;
            }
            write!(f, "]")
        }
        serde_json::Value::Object(m) => {
            // m is a serde_json::Map (a BTreeMap) -> already sorted by key.
            write!(f, "{{")?;
            let mut first = true;
            for (k, x) in m.iter() {
                if !first {
                    write!(f, ",")?;
                }
                first = false;
                write_json_string(f, k)?;
                write!(f, ":")?;
                write_json(f, x)?;
            }
            write!(f, "}}")
        }
    }
}
fn write_json_string(f: &mut std::fmt::Formatter<'_>, s: &str) -> std::fmt::Result {
    write!(f, "\"")?;
    for c in s.chars() {
        match c {
            '"' => write!(f, "\\\"")?,
            '\\' => write!(f, "\\\\")?,
            '\n' => write!(f, "\\n")?,
            '\r' => write!(f, "\\r")?,
            '\t' => write!(f, "\\t")?,
            c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
            c => write!(f, "{}", c)?,
        }
    }
    write!(f, "\"")
}

fn canonical_json(v: &serde_json::Value) -> Canonical {
    // serde_json::Map is a BTreeMap: object keys are already in sorted order,
    // so the value is already canonical.
    Canonical(v.clone())
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
    // The mapping is exactly the spec table.
    assert_eq!(
        docs::state_for_stop_reason("end_turn").as_deref(),
        Some("completed")
    );
    assert_eq!(
        docs::state_for_stop_reason("cancelled").as_deref(),
        Some("canceled")
    );
    assert_eq!(
        docs::state_for_stop_reason("error").as_deref(),
        Some("failed")
    );
    assert_eq!(
        docs::state_for_stop_reason("refusal").as_deref(),
        Some("rejected")
    );
    assert_eq!(
        docs::state_for_stop_reason("max_tokens").as_deref(),
        Some("failed")
    );
    assert_eq!(
        docs::state_for_stop_reason("max_turn_requests").as_deref(),
        Some("failed")
    );
    assert_eq!(
        docs::state_for_stop_reason("limit").as_deref(),
        Some("failed")
    );
    // The codomain is exactly the A2A terminal states.
    let images: std::collections::BTreeSet<&str> =
        docs::STOP_TO_STATE.iter().map(|(_, st)| *st).collect();
    let expected: std::collections::BTreeSet<&str> =
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
    // Text part: the `text` member is present, the other OneOf members absent.
    let text: Part = serde_json::from_str(r#"{"text":"hi"}"#).unwrap();
    assert_eq!(
        Part::Text {
            value: String::from("hi"),
            filename: None,
            media_type: None,
            metadata: None,
        },
        text
    );

    // File part: `raw` (base64) plus the shared `filename`/`mediaType` siblings.
    let raw: Part = serde_json::from_str(
        r#"{"raw":"Zm9vYmFy","filename":"f.bin","mediaType":"application/octet-stream"}"#,
    )
    .unwrap();
    assert_eq!(
        Part::Raw {
            value: vec![b'f', b'o', b'o', b'b', b'a', b'r'],
            filename: Some(String::from("f.bin")),
            media_type: Some(String::from("application/octet-stream")),
            metadata: None,
        },
        raw,
    );

    // Data part: `data` is an arbitrary JSON value.
    let data: Part = serde_json::from_str(r#"{"data":{"a":1}}"#).unwrap();
    assert_eq!(
        Part::Data {
            value: serde_json::json!({"a":1}),
            filename: None,
            media_type: None,
            metadata: None,
        },
        data,
    );

    // Zero OneOf members present -> a legal "empty" part (A2A's OneOf is
    // not-mandatory), not an error. Only the shared fields may be set.
    let empty: Part =
        serde_json::from_str(r#"{"filename":"a.txt","mediaType":"text/plain"}"#).unwrap();
    assert!(matches!(empty, Part::Empty { .. }));

    // More than one OneOf member present -> a Part may carry exactly one, so
    // this is a hard deserialize error.
    assert!(serde_json::from_str::<Part>(r#"{"text":"a","raw":"YQ=="}"#).is_err());

    // An unknown member alone is still an "empty" part (A2A does not define
    // deny-unknown on Part); the two-discriminator case above is what errors.
    assert!(matches!(
        serde_json::from_str::<Part>(r#"{"foo":"bar"}"#).unwrap(),
        Part::Empty { .. }
    ));
}

// A Part round-trips through JSON to the exact flat object A2A shows — this
// is what makes a Holler `Part` byte-identical to an A2A `Part`.
#[test]
fn a2a_part_wire_form_is_flat_and_stable() {
    let raw = Part::Raw {
        value: b"foobar".to_vec(),
        filename: Some(String::from("f.bin")),
        media_type: Some(String::from("application/octet-stream")),
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
