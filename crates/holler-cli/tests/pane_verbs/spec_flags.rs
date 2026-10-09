//! `SpecFlags::validate()` (story #670): the shared spec-flag group of `pane launch` and
//! `pane relaunch` keeps every value a string at clap time and types it here, with the
//! `holler-pane` guards and their stable codes. A guard's refusal is exit 3 in the verb (one
//! coded `usage` exits 2); neither is a clap usage error, so the code is stable and the
//! message is one line.

use clap::Parser;
use holler_cli::pane::args::SpecFlags;

/// A parser that holds only the shared group, so these tests need no verb's `Args`.
#[derive(Parser, Debug)]
struct Holder {
    #[command(flatten)]
    spec: SpecFlags,
}

fn validate(args: &[&str]) -> Result<(), String> {
    let mut argv = vec!["holder"];
    argv.extend_from_slice(args);
    let holder =
        Holder::try_parse_from(&argv).unwrap_or_else(|e| panic!("{argv:?} must parse: {e}"));
    holder
        .spec
        .validate()
        .map(|_| ())
        .map_err(|e| e.code().to_string())
}

#[test]
fn every_spec_flag_parses_as_a_string_and_a_good_set_validates() {
    let result = validate(&[
        "--project",
        "/srv/demo",
        "--workspace",
        "main",
        "--grid",
        "r2c1",
        "--model",
        "provider/model-id",
        "--effort",
        "high",
        "--role",
        "agent",
        "--env",
        "ALPHA_TOKEN",
        "--env",
        "BETA_URL",
        "--ctx-soft",
        "100000",
        "--ctx-hard",
        "150000",
        "--port-policy",
        "fixed",
        "--command-json",
        r#"["opencode","serve"]"#,
        "--check-arg",
        "curl",
        "--check-arg",
        "http://127.0.0.1:1/health",
        "--expect",
        "ok",
    ]);
    assert_eq!(result, Ok(()));
}

#[test]
fn no_flags_validates() {
    assert_eq!(validate(&[]), Ok(()));
}

#[test]
fn grid_in_each_accepted_notation_validates() {
    for grid in ["r2c1", "c1r2", "2,1"] {
        assert_eq!(validate(&["--grid", grid]), Ok(()), "{grid}");
    }
}

#[test]
fn a_bad_grid_is_refused_with_the_grid_code() {
    // Not a cell at all.
    assert_eq!(
        validate(&["--grid", "banana"]),
        Err("grid-ambiguous".to_string())
    );
    // A cell, but zero is not a row.
    assert_eq!(
        validate(&["--grid", "r0c1"]),
        Err("grid-out-of-range".to_string())
    );
    assert_eq!(
        validate(&["--grid", "r99999c1"]),
        Err("grid-out-of-range".to_string())
    );
}

#[test]
fn an_env_entry_with_a_value_is_a_secret_refusal_and_a_blank_name_is_invalid() {
    assert_eq!(
        validate(&["--env", "TOKEN=hunter2"]),
        Err("profile-secret-refused".to_string())
    );
    assert_eq!(
        validate(&["--env", ""]),
        Err("env-name-invalid".to_string())
    );
    assert_eq!(
        validate(&["--env", "BAD NAME"]),
        Err("env-name-invalid".to_string())
    );
}

#[test]
fn a_secret_value_never_reaches_the_refusal_message() {
    let holder = Holder::try_parse_from(["holder", "--env", "TOKEN=hunter2"]).unwrap();
    let err = holder.spec.validate().map(|_| ()).unwrap_err();
    let shown = format!("{err} {err:?}");
    assert!(
        !shown.contains("hunter2"),
        "the refusal must not echo the value: {shown}"
    );
}

#[test]
fn command_json_and_check_json_must_be_an_array_of_strings() {
    for flag in ["--command-json", "--check-json"] {
        assert_eq!(validate(&[flag, r#"["a","b"]"#]), Ok(()), "{flag}");
        assert_eq!(
            validate(&[flag, r#""just-a-string""#]),
            Err("command-not-argv".to_string()),
            "{flag}"
        );
        assert_eq!(
            validate(&[flag, r#"["a",1]"#]),
            Err("command-not-argv".to_string()),
            "{flag}"
        );
        assert_eq!(
            validate(&[flag, r#"{"a":"b"}"#]),
            Err("command-not-argv".to_string()),
            "{flag}"
        );
    }
}

#[test]
fn command_json_that_is_not_json_is_a_usage_refusal() {
    // `Argv::from_json` calls text that is not JSON `usage` (exit 2 through `emit`).
    assert_eq!(
        validate(&["--command-json", "not json"]),
        Err("usage".to_string())
    );
}
