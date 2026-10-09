//! Which flag parses on which verb (story #670, AC 5). Every verb is a stub, so "parses"
//! is observable as exit 1 with the `not implemented` line, and "does not parse" as exit
//! 2; the values stay strings at clap time (`SpecFlags::validate`, in `pane_verbs`, types
//! them).
//!
//! The verb-specific positionals and flags of the sibling stories are deliberately not
//! declared here, so they are not asserted either way.

use crate::{assert_no_failures, holler, PANE_VERBS, PROFILE_VERBS};

fn story_of(namespace: &str, verb: &str) -> u32 {
    let table = if namespace == "pane" {
        PANE_VERBS
    } else {
        PROFILE_VERBS
    };
    table
        .iter()
        .find(|(v, _)| *v == verb)
        .map(|(_, s)| *s)
        .expect("a known verb")
}

/// `holler <namespace> <verb> <flags...>` is parsed and reaches its stub.
fn parses(namespace: &str, verb: &str, flags: &[&str]) -> Result<(), String> {
    let mut argv = vec![namespace, verb];
    argv.extend_from_slice(flags);
    let out = holler(&argv);
    let line = format!(
        "error: not implemented (story #{})",
        story_of(namespace, verb)
    );
    if out.code == 1 && out.stdout.is_empty() && out.stderr_has_line(&line) {
        Ok(())
    } else {
        Err(format!(
            "{argv:?} should reach its stub (exit 1, {line:?}); got {out:?}"
        ))
    }
}

/// `holler <namespace> <verb> <flags...>` is a usage error.
fn is_usage_error(namespace: &str, verb: &str, flags: &[&str]) -> Result<(), String> {
    let mut argv = vec![namespace, verb];
    argv.extend_from_slice(flags);
    let out = holler(&argv);
    if out.code == 2 && out.stdout.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{argv:?} should be a usage error (exit 2); got {out:?}"
        ))
    }
}

#[test]
fn every_spec_flag_parses_on_launch_and_relaunch() {
    let flags: &[&[&str]] = &[
        &["--project", "/srv/demo"],
        &["--workspace", "main"],
        &["--grid", "r2c1"],
        // A bad value is a refusal for `SpecFlags::validate()`, not a clap error.
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
    ];
    let mut failures = Vec::new();
    for verb in ["launch", "relaunch"] {
        for set in flags {
            failures.extend(parses("pane", verb, set).err());
        }
        // All of them together.
        let all = [
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
        ];
        failures.extend(parses("pane", verb, &all).err());
    }
    assert_no_failures(failures);
}

#[test]
fn profile_parses_on_every_pane_verb_except_import() {
    let mut failures = Vec::new();
    for &(verb, _) in PANE_VERBS {
        failures.extend(parses("pane", verb, &[]).err());
        if verb == "import" {
            failures.extend(is_usage_error("pane", verb, &["--profile", "demo"]).err());
        } else {
            failures.extend(parses("pane", verb, &["--profile", "demo"]).err());
        }
    }
    assert_no_failures(failures);
}

#[test]
fn no_profile_verb_takes_profile() {
    let mut failures = Vec::new();
    for &(verb, _) in PROFILE_VERBS {
        // The verb must exist (a bare run reaches its stub) for the refusal to mean anything.
        failures.extend(parses("profile", verb, &[]).err());
        failures.extend(is_usage_error("profile", verb, &["--profile", "demo"]).err());
    }
    assert_no_failures(failures);
}

#[test]
fn spec_only_parses_on_launch_relaunch_and_close_only() {
    let mut failures = Vec::new();
    for &(verb, _) in PANE_VERBS {
        let flags = ["--spec-only", "--profile", "demo"];
        failures.extend(parses("pane", verb, &[]).err());
        if matches!(verb, "launch" | "relaunch" | "close") {
            failures.extend(parses("pane", verb, &flags).err());
        } else {
            failures.extend(is_usage_error("pane", verb, &flags).err());
        }
    }
    for &(verb, _) in PROFILE_VERBS {
        failures.extend(parses("profile", verb, &[]).err());
        failures.extend(is_usage_error("profile", verb, &["--spec-only"]).err());
    }
    assert_no_failures(failures);
}

#[test]
fn take_over_parses_on_profile_apply_only() {
    let mut failures = Vec::new();
    failures.extend(parses("profile", "apply", &["--take-over"]).err());
    for &(verb, _) in PROFILE_VERBS {
        if verb != "apply" {
            failures.extend(parses("profile", verb, &[]).err());
            failures.extend(is_usage_error("profile", verb, &["--take-over"]).err());
        }
    }
    for &(verb, _) in PANE_VERBS {
        failures.extend(parses("pane", verb, &[]).err());
        failures.extend(is_usage_error("pane", verb, &["--take-over"]).err());
    }
    assert_no_failures(failures);
}

/// `--format` is global: it parses after the verb and before the namespace, on a legacy
/// verb as well.
#[test]
fn format_is_a_global_flag() {
    let mut failures = Vec::new();
    for namespace in ["pane", "profile"] {
        failures.extend(parses(namespace, "list", &["--format=text"]).err());
        // JSON mode puts the (not-implemented) envelope on stdout, so only the exit code
        // and the envelope's presence are checked here.
        let out = holler(&[namespace, "list", "--format=json"]);
        if out.code != 1 || out.stdout.lines().count() != 1 {
            failures.push(format!(
                "{namespace} list --format=json: want exit 1 and one envelope; got {out:?}"
            ));
        }
    }
    let out = holler(&["--format=text", "pane", "list"]);
    if out.code != 1 || !out.stdout.is_empty() {
        failures.push(format!("--format before the namespace: {out:?}"));
    }
    assert_no_failures(failures);
}
