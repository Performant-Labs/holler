//! Usage errors (exit 2) and `--format` (story #670, decisions 1(d) and 9).
//!
//! Under `pane` and `profile`, in JSON mode, a usage error is an envelope with code
//! `usage` (one line, exit 2). Everywhere else, and in text mode, clap's own message goes
//! to stderr as it does today, and nothing goes to stdout. "JSON mode" is read from the
//! raw argv (`--json`, `--format=json`, `--format json`) because a parse that failed has
//! no parsed flags; the namespace is the first non-flag token, so a flag's value (`--debug
//! pane`) is never mistaken for it.

use holler_cli::output::Format;

use crate::parse::{accepted, resolved_format};
use crate::{holler, Out};

/// The `usage` envelope of a run: exit 2, one line on stdout, a one-line message that
/// is clap's reason flattened (no `error:` prefix, no `Usage:` block).
fn usage_envelope(out: &Out) -> String {
    assert_eq!(out.code, 2, "{out:?}");
    let envelope = out.envelope();
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["data"], serde_json::Value::Null, "{envelope}");
    assert_eq!(envelope["error"]["code"], "usage", "{envelope}");
    let message = envelope["error"]["message"]
        .as_str()
        .expect("message is a string")
        .to_string();
    assert!(
        !message.contains('\n'),
        "the message is one line: {message:?}"
    );
    assert!(
        !message.starts_with("error:"),
        "no `error:` prefix inside the envelope: {message:?}"
    );
    assert!(
        !message.contains("Usage:"),
        "no usage block inside the envelope: {message:?}"
    );
    message
}

/// A text-mode usage error: clap's message on stderr, exit 2, nothing on stdout.
fn assert_plain_usage_error(out: &Out, must_name: &str) {
    assert_eq!(out.code, 2, "{out:?}");
    assert!(
        out.stdout.is_empty(),
        "a usage error writes nothing to stdout: {out:?}"
    );
    assert!(
        out.stderr.contains(must_name),
        "stderr names {must_name:?}: {out:?}"
    );
}

// --- --spec-only requires --profile --------------------------------------------

#[test]
fn spec_only_requires_profile() {
    for verb in ["launch", "relaunch", "close"] {
        let text = holler(&["pane", verb, "--spec-only"]);
        assert_plain_usage_error(&text, "--profile");

        let json = holler(&["pane", verb, "--spec-only", "--format=json"]);
        let message = usage_envelope(&json);
        assert!(
            message.contains("--profile"),
            "{verb}: the message names the missing flag: {message:?}"
        );
    }
}

#[test]
fn spec_only_with_a_profile_parses() {
    for verb in ["launch", "relaunch", "close"] {
        accepted(&["pane", verb, "--spec-only", "--profile", "demo"]).unwrap();
    }
}

// --- --format ------------------------------------------------------------------

#[test]
fn a_bad_format_value_is_a_usage_error() {
    let out = holler(&["pane", "list", "--format=bogus"]);
    assert_plain_usage_error(&out, "bogus");
    // The same on a legacy verb: the resolver applies to every verb.
    let out = holler(&["roster", "--format=bogus"]);
    assert_plain_usage_error(&out, "bogus");
}

#[test]
fn json_conflicts_text() {
    for argv in [
        &["pane", "list", "--json", "--format=text"][..],
        &["pane", "list", "--format=text", "--json"],
        &["profile", "list", "--json", "--format", "text"],
    ] {
        let message = usage_envelope(&holler(argv));
        assert!(
            message.contains("--json") || message.contains("--format"),
            "{argv:?}: the message names the conflict: {message:?}"
        );
    }
    // A legacy verb refuses the same pair, in plain text: its usage-error output is untouched.
    let out = holler(&["roster", "--json", "--format=text"]);
    assert_eq!(out.code, 2, "{out:?}");
    assert!(out.stdout.is_empty(), "{out:?}");
}

#[test]
fn format_text_alone_is_the_default_text_mode() {
    for argv in [
        &["pane", "list", "--format=text"][..],
        &["pane", "list"],
        &["profile", "list", "--format", "text"],
    ] {
        assert_eq!(resolved_format(argv).unwrap(), Format::Text, "{argv:?}");
    }
}

// --- mutually exclusive argv forms ---------------------------------------------

#[test]
fn command_arg_and_command_json_are_mutually_exclusive() {
    for (verb, a, b, json_like) in [
        ("launch", "--command-arg", "--command-json", "[]"),
        ("relaunch", "--command-arg", "--command-json", "[]"),
        ("launch", "--check-arg", "--check-json", "[]"),
        ("relaunch", "--check-arg", "--check-json", "[]"),
    ] {
        let text = holler(&["pane", verb, a, "x", b, json_like]);
        assert_plain_usage_error(&text, a);
        assert!(
            text.stderr.contains(b),
            "{verb} {a} {b}: stderr names both flags: {text:?}"
        );

        let json = holler(&["pane", verb, a, "x", b, json_like, "--format=json"]);
        let message = usage_envelope(&json);
        assert!(
            message.contains(a) && message.contains(b),
            "{verb}: the message names both flags: {message:?}"
        );
    }
}

#[test]
fn each_argv_form_alone_parses() {
    for verb in ["launch", "relaunch"] {
        for args in [
            &["--command-arg", "opencode", "--command-arg", "serve"][..],
            &["--command-json", "[]"],
            &[
                "--check-arg",
                "curl",
                "--check-arg",
                "http://127.0.0.1:1/health",
            ],
            &["--check-json", "[]"],
        ] {
            let mut argv = vec!["pane", verb];
            argv.extend_from_slice(args);
            accepted(&argv).unwrap();
        }
    }
}

// --- the usage envelope is for pane and profile only ---------------------------

#[test]
fn an_unknown_flag_under_pane_or_profile_is_an_envelope_in_json_mode() {
    for argv in [
        &["pane", "list", "--bogus", "--format=json"][..],
        &["pane", "list", "--bogus", "--json"],
        &["profile", "list", "--bogus", "--json"],
        &["profile", "apply", "--bogus", "--format", "json"],
        // Global flags before the namespace, with their values in both spellings.
        &["--debug", "quiet", "pane", "list", "--bogus", "--json"],
        &["--debug=quiet", "pane", "list", "--bogus", "--json"],
        &["--log-format", "json", "pane", "list", "--bogus", "--json"],
        &["--log-format=json", "profile", "list", "--bogus", "--json"],
        &["--format", "json", "pane", "list", "--bogus"],
        &["--format=json", "pane", "list", "--bogus"],
        // A bare namespace is a usage error too.
        &["pane", "--format=json"],
        &["profile", "--json"],
    ] {
        let out = holler(argv);
        let message = usage_envelope(&out);
        assert!(!message.is_empty(), "{argv:?}");
    }
}

#[test]
fn the_envelope_message_keeps_the_reason_clap_gave() {
    let message = usage_envelope(&holler(&["pane", "list", "--bogus", "--json"]));
    assert!(
        message.contains("--bogus"),
        "the message names the unexpected argument: {message:?}"
    );
}

#[test]
fn an_unknown_flag_in_text_mode_is_clap_s_own_message() {
    for namespace in ["pane", "profile"] {
        let out = holler(&[namespace, "list", "--bogus"]);
        assert_plain_usage_error(&out, "--bogus");
        assert!(
            out.stderr.contains("Usage"),
            "clap's own usage block is kept in text mode: {out:?}"
        );
    }
}

/// Every legacy verb keeps its `--json` usage-error output as it is today (ADR 0003:
/// stdout is for the result, the diagnostic is on stderr): no envelope appears.
#[test]
fn a_legacy_verbs_json_usage_error_is_untouched() {
    for argv in [
        &["roster", "--bogus", "--json"][..],
        &["hub", "status", "--bogus", "--format=json"],
        &["say", "--bogus", "--json"],
        &["body", "status", "--bogus", "--json"],
        // The value of a global flag is not the namespace: `--debug pane` is a value, the
        // first non-flag token is `roster`.
        &["--debug", "pane", "roster", "--bogus", "--json"],
        &["--log-format", "profile", "roster", "--bogus", "--json"],
        // A positional that happens to say "pane" is not the namespace either.
        &["roster", "--prefix", "pane", "--bogus", "--json"],
        &["say", "pane", "--bogus", "--json"],
    ] {
        let out = holler(argv);
        assert_eq!(out.code, 2, "{argv:?}: {out:?}");
        assert!(
            out.stdout.is_empty(),
            "{argv:?}: a legacy usage error is not an envelope: {out:?}"
        );
        assert!(out.stderr.contains("--bogus"), "{argv:?}: {out:?}");
    }
}
