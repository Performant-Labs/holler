//! Parse-only checks of the `holler` clap tree, in this process (story #670, AC 5).
//!
//! Included by `pane_verbs`, `profile_verbs` (through `verb_harness`) and by the
//! `pane_cli_process` target (through `#[path]`), so "which flag parses on which verb" is
//! asked of clap itself and never of a verb's stub. Nothing here runs a verb.
//!
//! A flag is *accepted* by a verb when the parse succeeds or fails only because a required
//! positional is missing: the sibling stories add those positionals, and a flag check must
//! not break when they do. It is *refused* when clap reports `UnknownArgument`.

use clap::error::ErrorKind;
use clap::Parser;
use holler_cli::output::{resolve_format, Format};
use holler_cli::Cli;

fn full(argv: &[&str]) -> Vec<String> {
    std::iter::once("holler")
        .chain(argv.iter().copied())
        .map(str::to_owned)
        .collect()
}

/// `Cli::try_parse_from` of `holler <argv...>`.
pub fn try_parse(argv: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(full(argv))
}

/// `Ok` when `holler <argv...>` is accepted: it parses, or only a required positional is missing.
pub fn accepted(argv: &[&str]) -> Result<(), String> {
    match try_parse(argv) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::MissingRequiredArgument => Ok(()),
        Err(e) => Err(format!(
            "{argv:?} should be accepted: {} ({:?})",
            e.kind(),
            e.kind()
        )),
    }
}

/// `Ok` when clap refuses `holler <argv...>` as an unknown argument.
pub fn unknown_argument(argv: &[&str]) -> Result<(), String> {
    match try_parse(argv) {
        Err(e) if e.kind() == ErrorKind::UnknownArgument => Ok(()),
        Err(e) => Err(format!(
            "{argv:?} should be an unknown argument; got {:?}",
            e.kind()
        )),
        Ok(_) => Err(format!("{argv:?} should be an unknown argument; it parsed")),
    }
}

/// The format `holler <argv...>` resolves to, through `output::resolve_format`.
pub fn resolved_format(argv: &[&str]) -> Result<Format, String> {
    let cli = try_parse(argv).map_err(|e| format!("{argv:?} must parse: {e}"))?;
    resolve_format(cli.json, cli.format)
        .map(|choice| choice.format)
        .map_err(|e| format!("{argv:?} must resolve: {e:?}"))
}

/// The flag sets of `SpecFlags`, which `pane launch` and `pane relaunch` share. A bad value
/// is still a parse success: the values stay strings at clap time and
/// `SpecFlags::validate` types them.
pub const SPEC_FLAG_SETS: &[&[&str]] = &[
    &["--project", "/srv/demo"],
    &["--workspace", "main"],
    &["--grid", "r2c1"],
    &["--grid", "banana"],
    &["--model", "provider/model-id"],
    &["--effort", "high"],
    &["--role", "agent"],
    &["--role", "orchestrator"],
    &["--env", "ALPHA_TOKEN", "--env", "BETA_URL"],
    &["--env", "NAME=value"],
    &["--ctx-soft", "100000", "--ctx-hard", "150000"],
    &["--port-policy", "fixed"],
    &["--command-arg", "opencode", "--command-arg", "serve"],
    &["--command-json", r#"["opencode","serve"]"#],
    &["--command-json", "not json"],
    &[
        "--check-arg",
        "curl",
        "--check-arg",
        "http://127.0.0.1:1/health",
    ],
    &["--check-json", r#"["true"]"#],
    &["--expect", "ok", "--expect", "ready"],
    // All the value-taking groups at once, with the verbs' shared `--profile --spec-only`.
    &[
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
        "--ctx-soft",
        "1",
        "--ctx-hard",
        "2",
        "--port-policy",
        "fixed",
        "--command-arg",
        "opencode",
        "--check-arg",
        "curl",
        "--expect",
        "ok",
        "--profile",
        "demo",
        "--spec-only",
    ],
];

/// Every set of [`SPEC_FLAG_SETS`] is accepted by `holler pane <verb>`.
pub fn assert_spec_flags_accepted(verb: &str) {
    let failures: Vec<String> = SPEC_FLAG_SETS
        .iter()
        .filter_map(|set| {
            let mut argv = vec!["pane", verb];
            argv.extend_from_slice(set);
            accepted(&argv).err()
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
