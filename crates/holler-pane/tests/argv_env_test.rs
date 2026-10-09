#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637
//! `Argv` and `EnvVarName` (#637 AC 3): every stored command is an argv array, and
//! an env entry is a NAME only, so no record can carry a secret value (I7).

mod common;

use holler_pane::{Argv, EnvVarName, Pane, ProfileSpec};
use serde_json::{json, Value};

fn err_text<T: std::fmt::Debug>(r: Result<T, serde_json::Error>) -> String {
    r.expect_err("must be refused").to_string()
}

#[test]
fn argv_round_trips_as_a_json_array_of_strings() {
    let text = r#"["curl","-s","http://127.0.0.1:8095/v1/models"]"#;
    let argv: Argv = serde_json::from_str(text).unwrap();
    assert_eq!(serde_json::to_string(&argv).unwrap(), text);

    // Spaces and shell metacharacters are data in an argv, never re-split.
    let tricky = r#"["sh","-c","echo a b; rm -rf /"]"#;
    let argv: Argv = serde_json::from_str(tricky).unwrap();
    assert_eq!(serde_json::to_string(&argv).unwrap(), tricky);
}

#[test]
fn argv_bare_string_refused() {
    let msg = err_text(serde_json::from_str::<Argv>(r#""ls -l /tmp""#));
    assert!(msg.contains("command-not-argv"), "{msg}");

    // Other non-array shapes are not an argv either.
    for bad in [r#"{"cmd":"ls"}"#, r#"["ls", 1]"#, r#"["ls", null]"#, "7"] {
        assert!(serde_json::from_str::<Argv>(bad).is_err(), "{bad}");
    }
}

#[test]
fn argv_bare_string_is_refused_where_a_record_stores_a_command() {
    // Pane.command, Pane.probe.check, ProfileSpec.command and ProfileSpec.check.
    let mut pane = common::pane_json();
    pane["command"] = json!("opencode --port 8095");
    assert!(err_text(serde_json::from_value::<Pane>(pane)).contains("command-not-argv"));

    let mut pane = common::pane_json();
    pane["probe"]["check"] = json!("curl -s http://127.0.0.1:8095");
    assert!(err_text(serde_json::from_value::<Pane>(pane)).contains("command-not-argv"));

    for field in ["command", "check"] {
        let mut spec = common::spec_json();
        spec[field] = json!("curl -s http://127.0.0.1:8095");
        let msg = err_text(serde_json::from_value::<ProfileSpec>(spec));
        assert!(msg.contains("command-not-argv"), "{field}: {msg}");
    }
}

#[test]
fn env_var_name_accepts_names_and_refuses_values_and_blanks() {
    for ok in ["HOME", "ANTHROPIC_API_KEY", "MY_VAR_2", "lower_case", "X"] {
        let name = EnvVarName::parse(ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
        assert_eq!(name.as_str(), ok);
    }

    // A `=` means the entry carries a value (I7): a secret refusal, not a syntax one.
    for secret in ["TOKEN=abc123", "KEY=", "=value", "A=B=C"] {
        let code = EnvVarName::parse(secret).map_err(|e| e.code().to_string());
        assert_eq!(
            code,
            Err("profile-secret-refused".to_string()),
            "{secret:?}"
        );
    }

    // Empty or whitespace-bearing names are malformed.
    for blank in ["", " ", "\t", "MY VAR", "MY\tVAR", "VAR\n", " VAR"] {
        let code = EnvVarName::parse(blank).map_err(|e| e.code().to_string());
        assert_eq!(code, Err("env-name-invalid".to_string()), "{blank:?}");
    }
}

#[test]
fn env_var_name_serde_is_a_plain_string_with_the_same_refusals() {
    let name: EnvVarName = serde_json::from_str(r#""ANTHROPIC_API_KEY""#).unwrap();
    assert_eq!(
        serde_json::to_string(&name).unwrap(),
        r#""ANTHROPIC_API_KEY""#
    );

    let msg = err_text(serde_json::from_str::<EnvVarName>(r#""TOKEN=abc""#));
    assert!(msg.contains("profile-secret-refused"), "{msg}");
    let msg = err_text(serde_json::from_str::<EnvVarName>(r#""""#));
    assert!(msg.contains("env-name-invalid"), "{msg}");
    let msg = err_text(serde_json::from_str::<EnvVarName>(r#""A B""#));
    assert!(msg.contains("env-name-invalid"), "{msg}");
}

#[test]
fn no_profile_spec_or_pane_field_can_hold_an_environment_value() {
    // An env entry that carries a value is refused in both records.
    let mut spec = common::spec_json();
    spec["env"] = json!(["ANTHROPIC_API_KEY", "TOKEN=hunter2"]);
    let msg = err_text(serde_json::from_value::<ProfileSpec>(spec));
    assert!(msg.contains("profile-secret-refused"), "{msg}");
    assert!(
        !msg.contains("hunter2"),
        "the refusal must not echo the value: {msg}"
    );

    let mut pane = common::pane_json();
    pane["env"] = json!(["TOKEN=hunter2"]);
    let msg = err_text(serde_json::from_value::<Pane>(pane));
    assert!(msg.contains("profile-secret-refused"), "{msg}");
    assert!(!msg.contains("hunter2"), "{msg}");

    // A name/value map is not an env list at all.
    let mut spec = common::spec_json();
    spec["env"] = json!({"TOKEN": "hunter2"});
    assert!(serde_json::from_value::<ProfileSpec>(spec).is_err());

    // What does load serializes back with names only: no `=` in any env entry.
    let spec: ProfileSpec = serde_json::from_value(common::spec_json()).unwrap();
    let back = serde_json::to_value(&spec).unwrap();
    let env = back["env"].as_array().expect("env is an array");
    assert!(!env.is_empty());
    for entry in env {
        let Value::String(s) = entry else {
            panic!("env entry is not a string: {entry}")
        };
        assert!(!s.contains('='), "{s}");
    }
}
