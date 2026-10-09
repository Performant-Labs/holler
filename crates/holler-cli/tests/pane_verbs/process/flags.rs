//! Which flag parses on which verb (story #670, AC 5), asked of the clap tree in this
//! process (`Cli::try_parse_from`, as `cli_surface_test` does) and not of a verb's stub:
//! nothing here depends on what a verb does once it has parsed, so no sibling story's
//! work changes these results.
//!
//! A flag is *accepted* when the parse succeeds or only a required positional is missing
//! (the sibling stories add those), and *refused* when clap says `UnknownArgument`. The
//! values stay strings at clap time (`SpecFlags::validate`, tested in `pane_verbs`, types
//! them). The positive `launch`/`relaunch` spec-flag matrix is in
//! `pane_verbs/{launch,relaunch}.rs`, next to the verbs it is about.

use holler_cli::output::Format;

use crate::parse::{accepted, resolved_format, try_parse, unknown_argument};
use crate::{assert_no_failures, PANE_VERBS, PROFILE_VERBS};

#[test]
fn profile_parses_on_every_pane_verb_except_import() {
    let mut failures = Vec::new();
    for &verb in PANE_VERBS {
        // The verb exists, so that the refusal below means something.
        failures.extend(accepted(&["pane", verb]).err());
        if verb == "import" {
            failures.extend(unknown_argument(&["pane", verb, "--profile", "demo"]).err());
        } else {
            failures.extend(accepted(&["pane", verb, "--profile", "demo"]).err());
        }
    }
    assert_no_failures(failures);
}

#[test]
fn no_profile_verb_takes_profile() {
    let mut failures = Vec::new();
    for &verb in PROFILE_VERBS {
        failures.extend(accepted(&["profile", verb]).err());
        failures.extend(unknown_argument(&["profile", verb, "--profile", "demo"]).err());
    }
    assert_no_failures(failures);
}

#[test]
fn spec_only_parses_on_launch_relaunch_and_close_only() {
    let mut failures = Vec::new();
    for &verb in PANE_VERBS {
        let argv = ["pane", verb, "--spec-only", "--profile", "demo"];
        if matches!(verb, "launch" | "relaunch" | "close") {
            failures.extend(accepted(&argv).err());
        } else {
            failures.extend(accepted(&["pane", verb]).err());
            failures.extend(unknown_argument(&argv).err());
        }
    }
    for &verb in PROFILE_VERBS {
        failures.extend(accepted(&["profile", verb]).err());
        failures.extend(unknown_argument(&["profile", verb, "--spec-only"]).err());
    }
    assert_no_failures(failures);
}

#[test]
fn take_over_parses_on_profile_apply_only() {
    let mut failures = Vec::new();
    failures.extend(accepted(&["profile", "apply", "--take-over"]).err());
    for &verb in PROFILE_VERBS {
        if verb != "apply" {
            failures.extend(accepted(&["profile", verb]).err());
            failures.extend(unknown_argument(&["profile", verb, "--take-over"]).err());
        }
    }
    for &verb in PANE_VERBS {
        failures.extend(accepted(&["pane", verb]).err());
        failures.extend(unknown_argument(&["pane", verb, "--take-over"]).err());
    }
    assert_no_failures(failures);
}

/// `--format` is global: it parses after the verb and before the namespace, on a legacy
/// verb as well, and resolves to the format it names.
#[test]
fn format_is_a_global_flag() {
    let mut failures = Vec::new();
    for argv in [
        &["pane", "list", "--format=text"][..],
        &["profile", "list", "--format=text"],
        &["--format=text", "pane", "list"],
        &["--format", "text", "profile", "list"],
        &["roster", "--format=text"],
    ] {
        match try_parse(argv) {
            Ok(cli) if cli.format == Some(Format::Text) => {}
            other => failures.push(format!(
                "{argv:?}: want --format=text parsed as Some(Text); got {:?}",
                other.map(|cli| cli.format).map_err(|e| e.kind())
            )),
        }
        match resolved_format(argv) {
            Ok(Format::Text) => {}
            other => failures.push(format!("{argv:?}: want Text; got {other:?}")),
        }
    }
    for argv in [
        &["pane", "list", "--format=json"][..],
        &["profile", "list", "--format", "json"],
        &["--format=json", "pane", "list"],
        &["roster", "--format=json"],
    ] {
        match resolved_format(argv) {
            Ok(Format::Json) => {}
            other => failures.push(format!("{argv:?}: want Json; got {other:?}")),
        }
    }
    assert_no_failures(failures);
}
