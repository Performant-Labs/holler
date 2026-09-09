#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #187
//! RED tests for the body's session config TOML + registry (issue #187) —
//! the issue's own test list, verbatim in name.

use std::path::Path;

use holler_body::config::{self, ConfigError, Interrupt, SessionMode};
use holler_body::registry::SessionRegistry;
use holler_proto::{Mode, SessionName, SessionState};
use rstest::rstest;

// --- parsing -----------------------------------------------------------

#[test]
fn parses_two_spawn_sessions() {
    let toml = r#"
        [[session]]
        name = "alpha"
        harness = "opencode"
        command = ["opencode", "acp"]

        [[session]]
        name = "beta"
        harness = "claude"
        mode = "spawn"
        command = ["claude", "acp"]
        cwd = "/tmp/beta"
        env = { FOO = "bar" }
    "#;
    let parsed = config::parse(toml).expect("parses");
    assert_eq!(parsed.sessions.len(), 2);
    assert!(parsed.warnings.is_empty());

    let alpha = &parsed.sessions[0];
    assert_eq!(alpha.name.as_str(), "alpha");
    assert_eq!(alpha.harness, "opencode");
    assert_eq!(alpha.mode, SessionMode::Spawn);
    assert_eq!(alpha.command, Some(vec!["opencode".to_string(), "acp".to_string()]));
    assert_eq!(alpha.interrupt, Interrupt::Acp);

    let beta = &parsed.sessions[1];
    assert_eq!(beta.name.as_str(), "beta");
    assert_eq!(beta.cwd.as_deref(), Some("/tmp/beta"));
    assert_eq!(beta.env.as_ref().and_then(|e| e.get("FOO")).map(String::as_str), Some("bar"));
}

#[test]
fn parses_attach_session() {
    let toml = r#"
        [[session]]
        name = "gamma"
        harness = "opencode"
        mode = "attach"
        endpoint = "http://127.0.0.1:4096"
        session_id = "ses_123"
    "#;
    let parsed = config::parse(toml).expect("parses");
    assert_eq!(parsed.sessions.len(), 1);
    let s = &parsed.sessions[0];
    assert_eq!(s.mode, SessionMode::Attach);
    assert_eq!(s.endpoint.as_deref(), Some("http://127.0.0.1:4096"));
    assert_eq!(s.session_id.as_deref(), Some("ses_123"));
    assert!(s.command.is_none());
}

#[test]
fn mixed_modes_ok() {
    let toml = r#"
        [[session]]
        name = "spawned"
        harness = "opencode"
        command = ["opencode", "acp"]

        [[session]]
        name = "attached"
        harness = "opencode"
        mode = "attach"
        endpoint = "http://127.0.0.1:4096"
        session_id = "ses_1"
    "#;
    let parsed = config::parse(toml).expect("parses");
    assert_eq!(parsed.sessions.len(), 2);
    assert_eq!(parsed.sessions[0].mode, SessionMode::Spawn);
    assert_eq!(parsed.sessions[1].mode, SessionMode::Attach);
}

// --- validation (all fail-closed; CLI maps every ConfigError to exit 3) ---

#[test]
fn duplicate_name_exit_3() {
    let toml = r#"
        [[session]]
        name = "dup"
        harness = "opencode"
        command = ["opencode", "acp"]

        [[session]]
        name = "dup"
        harness = "claude"
        command = ["claude", "acp"]
    "#;
    let err = config::parse(toml).expect_err("duplicate name must be refused");
    match err {
        ConfigError::Invalid { row, field, .. } => {
            assert_eq!(row, 1);
            assert_eq!(field, "name");
        }
        other => panic!("wrong error variant: {other:?}"),
    }
}

#[test]
fn unknown_key_exit_3() {
    let toml = r#"
        [[session]]
        name = "alpha"
        harness = "opencode"
        command = ["opencode", "acp"]
        typo_field = "oops"
    "#;
    let err = config::parse(toml).expect_err("unknown key must be refused");
    assert!(matches!(err, ConfigError::Toml(_)));
}

#[test]
fn unknown_top_level_key_exit_3() {
    let toml = r#"
        not_a_real_key = true

        [[session]]
        name = "alpha"
        harness = "opencode"
        command = ["opencode", "acp"]
    "#;
    let err = config::parse(toml).expect_err("unknown top-level key must be refused");
    assert!(matches!(err, ConfigError::Toml(_)));
}

#[test]
fn spawn_missing_command_exit_3() {
    let toml = r#"
        [[session]]
        name = "alpha"
        harness = "opencode"
    "#;
    let err = config::parse(toml).expect_err("spawn without command must be refused");
    match err {
        ConfigError::Invalid { field, .. } => assert_eq!(field, "command"),
        other => panic!("wrong error variant: {other:?}"),
    }
}

#[test]
fn attach_missing_session_id_exit_3() {
    let toml = r#"
        [[session]]
        name = "alpha"
        harness = "opencode"
        mode = "attach"
        endpoint = "http://127.0.0.1:4096"
    "#;
    let err = config::parse(toml).expect_err("attach without session_id must be refused");
    match err {
        ConfigError::Invalid { field, .. } => assert_eq!(field, "session_id"),
        other => panic!("wrong error variant: {other:?}"),
    }
}

#[test]
fn attach_missing_endpoint_exit_3() {
    let toml = r#"
        [[session]]
        name = "alpha"
        harness = "opencode"
        mode = "attach"
        session_id = "ses_1"
    "#;
    let err = config::parse(toml).expect_err("attach without endpoint must be refused");
    match err {
        ConfigError::Invalid { field, .. } => assert_eq!(field, "endpoint"),
        other => panic!("wrong error variant: {other:?}"),
    }
}

#[test]
fn interrupt_http_missing_endpoint_exit_3() {
    let toml = r#"
        [[session]]
        name = "alpha"
        harness = "opencode"
        command = ["opencode", "acp"]
        interrupt = "http"
    "#;
    let err = config::parse(toml).expect_err("interrupt=http without endpoint must be refused");
    match err {
        ConfigError::Invalid { field, .. } => assert_eq!(field, "endpoint"),
        other => panic!("wrong error variant: {other:?}"),
    }
}

#[test]
fn attach_with_command_warns_and_ignores() {
    let toml = r#"
        [[session]]
        name = "alpha"
        harness = "opencode"
        mode = "attach"
        command = ["opencode", "acp"]
        endpoint = "http://127.0.0.1:4096"
        session_id = "ses_1"
    "#;
    let parsed = config::parse(toml).expect("attach+command is a warning, not a refusal");
    assert_eq!(parsed.sessions.len(), 1);
    assert!(parsed.sessions[0].command.is_none(), "command must be dropped, never spawned");
    assert_eq!(parsed.warnings.len(), 1);
    assert!(parsed.warnings[0].contains("command"));
}

// --- discovery -----------------------------------------------------------

#[rstest]
#[case::flag_wins_over_everything(Some("flag.toml"), Some("env.toml"), true, true, "flag.toml")]
#[case::env_wins_over_files(None, Some("env.toml"), true, true, "env.toml")]
#[case::sessions_toml_wins_over_session_toml(None, None, true, true, "sessions.toml")]
#[case::session_toml_is_the_last_resort(None, None, false, true, "session.toml")]
fn discovery_order_flag_env_file(
    #[case] flag: Option<&str>,
    #[case] env: Option<&str>,
    #[case] sessions_toml_present: bool,
    #[case] session_toml_present: bool,
    #[case] want_suffix: &str,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    if sessions_toml_present {
        std::fs::write(dir.path().join("sessions.toml"), "").expect("write sessions.toml");
    }
    if session_toml_present {
        std::fs::write(dir.path().join("session.toml"), "").expect("write session.toml");
    }
    let flag_path = flag.map(Path::new);
    let env_path = env.map(Path::new);
    let got = config::discover(flag_path, env_path, dir.path()).expect("a source must resolve");
    assert!(
        got.ends_with(want_suffix),
        "wanted a path ending {want_suffix:?}, got {}",
        got.display()
    );
}

#[test]
fn discovery_finds_nothing_when_no_source_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(config::discover(None, None, dir.path()).is_none());
}

#[test]
fn no_config_exit_3_with_hint() {
    let dir = tempfile::tempdir().expect("tempdir");
    // No `--config`, no `HOLLER_CONFIG` (env_config is passed explicitly as
    // `None` via `discover`, so this exercises the same "none found" path
    // `discover_and_load` reaches when the env var is unset).
    let found = config::discover(None, None, dir.path());
    assert!(found.is_none());
    let err = ConfigError::NoConfigFound;
    let msg = err.message();
    assert!(msg.contains("sessions.toml"));
    assert!(msg.contains("--config"));
}

// --- registry / presence --------------------------------------------------

#[test]
fn presence_doc_lists_every_session_idle() {
    let toml = r#"
        [[session]]
        name = "alpha"
        harness = "opencode"
        command = ["opencode", "acp"]

        [[session]]
        name = "beta"
        harness = "claude"
        mode = "attach"
        endpoint = "http://127.0.0.1:4096"
        session_id = "ses_1"
    "#;
    let parsed = config::parse(toml).expect("parses");
    let registry = SessionRegistry::from_sessions(parsed.sessions);
    assert_eq!(registry.len(), 2);

    let presence = registry.presence_doc("myhost".to_string());
    assert_eq!(presence.hostname, "myhost");
    assert_eq!(presence.sessions.len(), 2);
    for ad in &presence.sessions {
        assert_eq!(ad.state, SessionState::Idle);
    }

    let alpha = presence.sessions.iter().find(|a| a.name == "alpha").expect("alpha present");
    assert_eq!(alpha.mode, Mode::Spawn);
    assert_eq!(alpha.harness, "opencode");

    let beta = presence.sessions.iter().find(|a| a.name == "beta").expect("beta present");
    assert_eq!(beta.mode, Mode::Attach);
    assert_eq!(beta.harness_session_id.as_deref(), Some("ses_1"));

    // and the registry itself is queryable by name.
    let name = SessionName::parse("alpha").expect("valid name");
    assert!(registry.get(&name).is_some());
}
