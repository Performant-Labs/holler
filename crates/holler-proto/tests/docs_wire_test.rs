#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #185/#251/#250
//! Wire-shape unit tests for `holler_proto::docs` that don't fit the golden-file
//! suite (`golden_test.rs` only exercises the `Serialize` direction). Split out
//! of `docs.rs` itself to keep that file under the 900-line lint gate.

use holler_proto::docs::{HelloRole, ProtocolParams, SessionState, Status, StatusSession};
use holler_proto::error::Code;

mod protocol_version_tests {
    use super::*;

    /// No `params` at all (`query/protocol` with no body): no version asked,
    /// never a rejection.
    #[test]
    fn no_params_is_no_version_asked() {
        assert_eq!(ProtocolParams::parse_version(None), Ok(None));
    }

    /// `params` present but with no `version` key: same as no `params`.
    #[test]
    fn params_without_version_key_is_no_version_asked() {
        let params = serde_json::json!({});
        assert_eq!(ProtocolParams::parse_version(Some(&params)), Ok(None));
    }

    /// `version: 0` is rejected — the docs require a *positive* integer.
    #[test]
    fn version_zero_is_unknown_feature() {
        let params = serde_json::json!({"version": 0});
        let err = ProtocolParams::parse_version(Some(&params)).expect_err("0 must be rejected");
        assert_eq!(err.code, Code::UnknownFeature.jsonrpc());
    }

    /// A negative `version` is rejected (not silently treated as "no
    /// version asked").
    #[test]
    fn negative_version_is_unknown_feature() {
        let params = serde_json::json!({"version": -1});
        let err = ProtocolParams::parse_version(Some(&params)).expect_err("negative must be rejected");
        assert_eq!(err.code, Code::UnknownFeature.jsonrpc());
    }

    /// A non-numeric `version` is rejected.
    #[test]
    fn non_numeric_version_is_unknown_feature() {
        let params = serde_json::json!({"version": "two"});
        let err = ProtocolParams::parse_version(Some(&params)).expect_err("a string must be rejected");
        assert_eq!(err.code, Code::UnknownFeature.jsonrpc());
    }

    /// A `version` too large to fit `u32` is rejected.
    #[test]
    fn oversized_version_is_unknown_feature() {
        let params = serde_json::json!({"version": 5_000_000_000_u64});
        let err = ProtocolParams::parse_version(Some(&params)).expect_err("must not silently truncate");
        assert_eq!(err.code, Code::UnknownFeature.jsonrpc());
    }

    /// A valid positive `version` parses through untouched.
    #[test]
    fn positive_version_parses() {
        let params = serde_json::json!({"version": 2});
        assert_eq!(ProtocolParams::parse_version(Some(&params)), Ok(Some(2)));
    }
}

mod status_sessions_wire_tests {
    //! Issue #250: `Status::session_list` serializes as `"sessions"`
    //! (matching docs §5.1) but must still round-trip through the
    //! hand-written `Deserialize` — the golden tests (`golden_test.rs`)
    //! only exercise the `Serialize` direction, so the shape-routing logic
    //! in `docs.rs`'s manual `impl Deserialize for Status` has no other
    //! coverage.
    use super::*;

    #[test]
    fn body_sessions_array_round_trips_into_session_list() {
        let status = Status {
            role: HelloRole::Body,
            protocol: 2,
            protocol_min: 2,
            protocol_max: 2,
            version: "0.1.0".into(),
            hostname: "kiwi".into(),
            connected: Some(true),
            token_id: Some("tok_1".into()),
            listening: None,
            features: vec!["ping".into()],
            harnesses: Some(vec!["opencode".into()]),
            harnesses_known: None,
            harnesses_confirmed: None,
            bodies: None,
            sessions: None,
            session_list: Some(vec![StatusSession {
                name: "alpha".into(),
                harness: "opencode".into(),
                state: SessionState::Idle,
                endpoint: None,
            }]),
        };
        let wire = serde_json::to_value(&status).unwrap();
        assert!(wire.get("sessions").unwrap().is_array(), "body role must serialize the session list under the wire key `sessions`");
        let back: Status = serde_json::from_value(wire).unwrap();
        assert_eq!(back, status);
    }

    #[test]
    fn hub_sessions_count_round_trips_into_sessions_field() {
        let status = Status {
            role: HelloRole::Hub,
            protocol: 2,
            protocol_min: 2,
            protocol_max: 2,
            version: "0.1.0".into(),
            hostname: "hub-host".into(),
            connected: None,
            token_id: None,
            listening: Some("ws://127.0.0.1:41807".into()),
            features: vec![],
            harnesses: None,
            harnesses_known: Some(vec!["opencode".into()]),
            harnesses_confirmed: None,
            bodies: Some(1),
            sessions: Some(2),
            session_list: None,
        };
        let wire = serde_json::to_value(&status).unwrap();
        assert!(wire.get("sessions").unwrap().is_number(), "hub role must serialize the session count under the wire key `sessions`");
        let back: Status = serde_json::from_value(wire).unwrap();
        assert_eq!(back, status);
    }

    #[test]
    fn missing_sessions_key_deserializes_to_neither() {
        let wire = serde_json::json!({
            "role": "body", "protocol": 2, "protocol_min": 2, "protocol_max": 2, "version": "0.1.0",
            "hostname": "kiwi", "features": [],
        });
        let status: Status = serde_json::from_value(wire).unwrap();
        assert_eq!(status.sessions, None);
        assert_eq!(status.session_list, None);
    }

    #[test]
    fn sessions_wrong_shape_is_rejected() {
        let wire = serde_json::json!({
            "role": "body", "protocol": 2, "protocol_min": 2, "protocol_max": 2, "version": "0.1.0",
            "hostname": "kiwi", "features": [], "sessions": "nope",
        });
        assert!(serde_json::from_value::<Status>(wire).is_err());
    }
}

mod admin_role_wire_test {
    //! #508/#509 (remote hub-admin client, epic #506) AC 2: `HelloRole::
    //! Admin` must serialize as `"admin"` and round-trip through `Hello`.
    //! Written against the wire form only (never `HelloRole::Admin` itself,
    //! which does not exist on current `main`) so this test compiles today
    //! and fails on the real assertion — not a compile error — exactly the
    //! Tier-2 rule this stack's `CLAUDE.md` states: "RED means the new test
    //! fails on an assertion about the missing behavior, not on a compile
    //! error."
    use holler_proto::docs::Hello;

    fn admin_hello_wire() -> serde_json::Value {
        serde_json::json!({
            "protocol": holler_proto::PROTOCOL_VERSION,
            "protocol_min": holler_proto::PROTOCOL_MIN,
            "protocol_max": holler_proto::PROTOCOL_MAX,
            "role": "admin",
            "hostname": "admin-client",
            "features": [],
        })
    }

    /// AC 2: a `circuit/hello` document with `role: "admin"` must
    /// deserialize into a `Hello` (today it fails: `"admin"` is not a
    /// `HelloRole` variant, so `serde_json::from_value` errors with
    /// "unknown variant `admin`, expected `body` or `hub`").
    #[test]
    fn hello_role_admin_deserializes() {
        let hello: Hello = serde_json::from_value(admin_hello_wire()).expect(
            "AC 2: `role: \"admin\"` must deserialize into `Hello` once `HelloRole::Admin` exists; \
             on current main this fails because `admin` is not a known `HelloRole` variant",
        );
        assert_eq!(
            serde_json::to_value(&hello).unwrap()["role"],
            serde_json::json!("admin"),
            "AC 2: the role must round-trip back out as the wire string \"admin\""
        );
    }
}
