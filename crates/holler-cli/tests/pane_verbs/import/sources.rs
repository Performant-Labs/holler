//! The readers of the old launcher's files (#650 acceptance 7): a hand-edited file is
//! `source-unreadable`, naming the file and a position only, never text read from the file
//! or a parser's error; a missing file, variable or entry is `source-missing`. Each case
//! first shows that the secret is in the text a leaking implementation would print, so that
//! its absence from the output means something.

use holler_body::config::{load, ConfigError};
use holler_cli::output::Format;
use serde_json::Value;

use super::rig::{
    assert_absent, data_of, error_of, findings, message, one_finding, pane_env, pane_row,
    ImportRig, SECRET,
};
use crate::doctor::rig::Seed;
use crate::verb_harness::Outcome;

const A: &str = "hj-c1r1";

fn one_pane() -> ImportRig {
    ImportRig::new(&[Seed::new(A, 1, 1)])
}

/// The dry run in both formats: the JSON data, after checking that neither run prints
/// `secret` (nor the rig's own `pane.env` secret) and that the pane is not recorded.
fn dry_without(rig: &ImportRig, secret: &str) -> Value {
    let json = rig.run(&["--dry-run"], Format::Json);
    let text = rig.run(&["--dry-run"], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    for run in [&json, &text] {
        assert_absent(run, secret);
        assert_absent(run, SECRET);
    }
    let data = data_of(&json);
    assert_eq!(pane_row(&data, A)["action"], "blocked", "{data}");
    data
}

/// A real run is refused, prints no secret, and writes nothing.
fn real_run_refused(rig: &ImportRig, secret: &str) {
    let run: Outcome = rig.run(&[], Format::Json);
    assert_eq!(run.code, 3, "{run:?}");
    assert_eq!(error_of(&run).0, "import-blocked");
    assert_absent(&run, secret);
    assert!(rig.records().is_empty(), "no record");
}

/// The 1-based line and column of byte `at` in `text`.
fn line_col(text: &str, at: usize) -> (usize, usize) {
    let before = &text[..at];
    let line = before.matches('\n').count() + 1;
    let col = before.chars().rev().take_while(|&c| c != '\n').count() + 1;
    (line, col)
}

// --- sessions.toml -----------------------------------------------------------------------

#[test]
fn sessions_toml_that_is_not_toml_names_the_file_only() {
    let secret = "zq-sessions-secret-41c9";
    let rig = one_pane();
    let rel = format!("{A}/sessions.toml");
    rig.write(
        &rel,
        &format!("[[session]]\nname = \"{A}\"\nharness = \"opencode\"\napi_key = {secret}\n"),
    );
    match load(&rig.path(&rel)) {
        Err(ConfigError::Toml(text)) => assert!(text.contains(secret), "the parser quotes it"),
        other => panic!("the fixture is a TOML error: {other:?}"),
    }
    let data = dry_without(&rig, secret);
    let finding = one_finding(&data, "source-unreadable", A, "problem");
    let text = message(finding);
    assert!(text.contains("sessions.toml"), "{finding}");
    assert!(text.contains("not valid TOML"), "{finding}");
    assert!(!text.contains("api_key"), "no text of the file: {finding}");
    real_run_refused(&rig, secret);
}

#[test]
fn sessions_toml_with_an_invalid_row_names_the_row_and_field_only() {
    let secret = "zq-mode-secret-90d2";
    let row_name = "row-name-zq";
    let rig = one_pane();
    let rel = format!("{A}/sessions.toml");
    let session = rig.world.live(A).session.clone();
    rig.write(
        &rel,
        &format!(
            "[[session]]\nname = \"{row_name}\"\nharness = \"opencode\"\nmode = \"{secret}\"\n\
             endpoint = \"http://127.0.0.1:1\"\nsession_id = \"{session}\"\n"
        ),
    );
    match load(&rig.path(&rel)) {
        Err(ConfigError::Invalid {
            field: "mode",
            reason,
            ..
        }) => assert!(reason.contains(secret), "the reason quotes it: {reason}"),
        other => panic!("the fixture is an invalid mode: {other:?}"),
    }
    let data = dry_without(&rig, secret);
    let finding = one_finding(&data, "source-unreadable", A, "problem");
    let text = message(finding);
    assert!(text.contains("sessions.toml"), "{finding}");
    assert!(text.contains("mode"), "names the field: {finding}");
    assert!(!text.contains(row_name), "not the row's name: {finding}");
    assert!(!text.contains("unknown mode"), "not the reason: {finding}");
    real_run_refused(&rig, secret);
}

// --- The launcher table ----------------------------------------------------------------

#[test]
fn a_launcher_table_that_is_not_toml_names_the_file_line_and_column() {
    let secret = "zq-launcher-secret-5aa0";
    let rig = one_pane();
    let port = rig.world.live(A).port;
    let text = format!(
        "[{A}]\nport = {port}\ncwd = \"/srv/demo/{A}\"\ncommand = [\"opencode\"]\n\
         token = {secret}\n"
    );
    rig.write("launcher.toml", &text);
    let error = toml::from_str::<toml::Value>(&text).expect_err("the fixture is not TOML");
    assert!(error.to_string().contains(secret), "the parser quotes it");
    let span = error.span().expect("the parser gives a span");
    let (line, col) = line_col(&text, span.start);
    assert_eq!(line, 5, "the secret's line");

    let data = dry_without(&rig, secret);
    let unreadable = findings(&data, "source-unreadable");
    assert!(!unreadable.is_empty(), "{data}");
    for finding in unreadable {
        let message = message(finding);
        assert!(message.contains("launcher.toml"), "{finding}");
        assert!(message.contains(&format!("line {line}")), "{finding}");
        assert!(message.contains(&format!("column {col}")), "{finding}");
        assert!(!message.contains("token"), "no text of the file: {finding}");
    }
    real_run_refused(&rig, secret);
}

#[test]
fn a_missing_or_partial_launcher_entry_is_source_missing() {
    let port = crate::doctor::rig::FIRST_PORT;
    let cases = [
        ("no entry", "# no table for the pane\n".to_owned()),
        (
            "no port",
            format!("[{A}]\ncwd = \"/srv/demo/{A}\"\ncommand = [\"opencode\"]\n"),
        ),
        (
            "no cwd",
            format!("[{A}]\nport = {port}\ncommand = [\"opencode\"]\n"),
        ),
    ];
    for (case, text) in cases {
        let rig = one_pane();
        rig.write("launcher.toml", &text);
        let data = dry_without(&rig, SECRET);
        let finding = one_finding(&data, "source-missing", A, "problem");
        assert!(message(finding).contains("launcher"), "{case}: {finding}");
    }
}

// --- pane.env ----------------------------------------------------------------------------

#[test]
fn a_pane_env_line_that_does_not_parse_names_the_file_and_line_only() {
    let secret = "zq-env-secret-c3c3";
    // Each offending line, placed as line 3 of the file (after the first two lines of the
    // rig's `pane.env`, which are a comment and a parsable line).
    // The ceiling's own line is dropped first, so the bad value is its only one.
    let lines = [
        (format!("export OC_MODEL = {secret}"), None),
        (format!("{secret} with no equals sign"), None),
        (format!("  OC_EFFORT={secret}"), None),
        (
            format!("OC_CONTEXT_SOFT={secret}"),
            Some("OC_CONTEXT_SOFT="),
        ),
    ];
    for (bad, replaces) in lines {
        let rig = one_pane();
        let mut text: Vec<String> = pane_env()
            .lines()
            .filter(|l| replaces.is_none_or(|name| !l.contains(name)))
            .map(str::to_owned)
            .collect();
        text.insert(2, bad.clone());
        let file = text.join("\n") + "\n";
        assert!(file.lines().nth(2).is_some_and(|l| l.contains(secret)));
        rig.write(&format!("{A}/pane.env"), &file);
        let data = dry_without(&rig, secret);
        let finding = one_finding(&data, "source-unreadable", A, "problem");
        let text = message(finding);
        assert!(text.contains("pane.env"), "{bad:?}: {finding}");
        assert!(text.contains("line 3"), "{bad:?}: {finding}");
    }
}

#[test]
fn a_missing_pane_env_or_required_variable_is_source_missing() {
    let rig = one_pane();
    rig.remove(&format!("{A}/pane.env"));
    let data = dry_without(&rig, SECRET);
    let finding = one_finding(&data, "source-missing", A, "problem");
    assert!(message(finding).contains("pane.env"), "{finding}");

    for variable in [
        "OC_MODEL",
        "OC_EFFORT",
        "OC_CONTEXT_SOFT",
        "OC_CONTEXT_HARD",
    ] {
        let rig = one_pane();
        let file: String = pane_env()
            .lines()
            .filter(|l| !l.contains(&format!("{variable}=")))
            .map(|l| format!("{l}\n"))
            .collect();
        rig.write(&format!("{A}/pane.env"), &file);
        let data = dry_without(&rig, SECRET);
        let finding = one_finding(&data, "source-missing", A, "problem");
        assert!(message(finding).contains(variable), "{variable}: {finding}");
        real_run_refused(&rig, SECRET);
    }
}

/// A pane named by `--orchestrator` still needs its own `pane.env` (join item 5).
#[test]
fn the_orchestrator_needs_its_own_pane_env() {
    let rig = one_pane();
    rig.remove(&format!("{A}/pane.env"));
    let data = data_of(&rig.run(&["--dry-run", "--orchestrator", A], Format::Json));
    one_finding(&data, "source-missing", A, "problem");
}

#[test]
fn a_missing_state_root_is_a_usage_error() {
    let rig = one_pane();
    let mut argv = rig.args(&["--dry-run"]);
    let at = argv.iter().position(|a| a == "--from").expect("--from") + 1;
    argv[at] = rig
        .path("no-such-directory")
        .to_str()
        .expect("UTF-8")
        .to_owned();
    let json = rig.run_argv(&argv, Format::Json);
    assert_eq!(json.code, 2, "{json:?}");
    assert_eq!(error_of(&json).0, "usage");
    let text = rig.run_argv(&argv, Format::Text);
    assert_eq!(text.code, 2, "{text:?}");
    assert!(text.out.is_empty(), "{text:?}");
    assert_eq!(text.err.lines().count(), 1, "one line: {text:?}");
}
