//! `say`, `interrupt`, `answer` and `roster` with `--pane` / `--profile` (story #670,
//! decision 5), on the real binary.
//!
//! The forms parse and are refused, in plain text under every format (these are legacy
//! verbs: no envelope, so #646 inherits one shape), before any hub is contacted. A
//! `--pane` that parsed and was ignored would deliver a prompt unchecked: the state dir
//! here holds no hub, so a verb that went on to contact one would fail with a different
//! message, and these assert the refusal line itself.

use crate::stub::PANE_FORM_REFUSAL as PANE_REFUSAL;
use crate::stub::ROSTER_PROFILE_REFUSAL as ROSTER_REFUSAL;
use crate::{assert_no_failures, holler};

/// Every form that names a pane or a profile on `say`, `interrupt` and `answer`.
const REFUSED_FORMS: &[&[&str]] = &[
    &["say", "--pane", "demo-c1r1", "hello"],
    &["say", "--pane", "demo-c1r1", "--parts-file", "msg.json"],
    &["say", "--pane", "demo-c1r1", "--profile", "demo", "hello"],
    &["say", "--profile", "demo", "io/alpha", "hello"],
    &["say", "--pane", "demo-c1r1", "hello", "--queue"],
    &["interrupt", "--pane", "demo-c1r1"],
    &["interrupt", "--pane", "demo-c1r1", "stop"],
    &["interrupt", "--profile", "demo", "io/alpha"],
    &["answer", "--pane", "demo-c1r1", "yes"],
    &["answer", "--profile", "demo", "io/alpha", "yes"],
];

#[test]
fn a_pane_or_profile_form_is_refused_with_exit_1_before_any_hub() {
    let mut failures = Vec::new();
    for argv in REFUSED_FORMS {
        let out = holler(argv);
        if out.code != 1 || !out.stdout.is_empty() || !out.stderr_has_line(PANE_REFUSAL) {
            failures.push(format!("holler {argv:?}: want exit 1, empty stdout, stderr line {PANE_REFUSAL:?}; got {out:?}"));
        }
    }
    assert_no_failures(failures);
}

/// The refusal is the same plain text under `--json` and `--format=json`: no envelope.
#[test]
fn the_refusal_is_plain_text_under_every_format() {
    let mut failures = Vec::new();
    for argv in REFUSED_FORMS {
        for format in [
            &["--json"][..],
            &["--format=json"],
            &["--format", "json"],
            &["--format=text"],
        ] {
            let mut full: Vec<&str> = argv.to_vec();
            full.extend_from_slice(format);
            let out = holler(&full);
            if out.code != 1 || !out.stdout.is_empty() || !out.stderr_has_line(PANE_REFUSAL) {
                failures.push(format!("holler {full:?}: want exit 1, empty stdout, stderr line {PANE_REFUSAL:?}; got {out:?}"));
            }
        }
    }
    assert_no_failures(failures);
}

#[test]
fn roster_profile_is_refused_with_the_roster_story() {
    for format in [&[][..], &["--json"], &["--format=json"]] {
        let mut argv = vec!["roster", "--profile", "demo"];
        argv.extend_from_slice(format);
        let out = holler(&argv);
        assert_eq!(out.code, 1, "{argv:?}: {out:?}");
        assert!(out.stdout.is_empty(), "{argv:?}: {out:?}");
        assert!(out.stderr_has_line(ROSTER_REFUSAL), "{argv:?}: {out:?}");
    }
}

/// A usage error wins over the refusal: a `--pane` form that is malformed is exit 2, and
/// its message is not the story-646 refusal.
#[test]
fn a_malformed_pane_form_is_a_usage_error_not_a_refusal() {
    let cases: [(&[&str], &str); 11] = [
        (&["say"], "session"),
        (&["say", "io/alpha"], "text"),
        (&["say", "--pane", "demo-c1r1"], "text"),
        (&["say", "--pane", "demo-c1r1", "a", "b"], ""),
        (&["say", "--pane", "demo-c1r1", "io/alpha", "hello"], ""),
        (&["interrupt"], "session"),
        (&["answer"], "session"),
        (&["answer", "io/alpha"], "choice"),
        (&["answer", "--pane", "demo-c1r1"], "choice"),
        // A flag between positionals lets clap accept a third one; the accessor refuses it,
        // before the #646 refusal and before any hub is contacted.
        (
            &["say", "io/alpha", "hello", "--queue", "extra"],
            "positionals",
        ),
        (
            &[
                "say",
                "--profile",
                "demo",
                "io/alpha",
                "hello",
                "--queue",
                "extra",
            ],
            "positionals",
        ),
    ];
    let mut failures = Vec::new();
    for (argv, names) in cases {
        let out = holler(argv);
        let lower = out.stderr.to_lowercase();
        if out.code != 2
            || !out.stdout.is_empty()
            || out.stderr_has_line(PANE_REFUSAL)
            || lower.contains("no live holler hub")
            || !lower.contains(names)
        {
            failures.push(format!(
                "holler {argv:?}: want exit 2 naming {names:?}, no refusal; got {out:?}"
            ));
        }
    }
    assert_no_failures(failures);
}

/// The existing forms still reach a hub: with none running they fail on that, not on a
/// refusal and not on a usage error (the old tests in `talk_test.rs` and
/// `interrupt_test.rs` pin their output against a real hub).
#[test]
fn the_session_forms_are_not_refused() {
    for argv in [
        &["say", "io/alpha", "hello"][..],
        &["say", "io/alpha", "--parts-file", "msg.json"],
        &["say", "io/alpha", "hello", "--queue"],
        &["interrupt", "io/alpha"],
        &["interrupt", "io/alpha", "stop"],
        &["answer", "io/alpha", "yes"],
    ] {
        let out = holler(argv);
        assert_ne!(out.code, 2, "{argv:?} is a valid form: {out:?}");
        assert!(
            !out.stderr_has_line(PANE_REFUSAL),
            "{argv:?} names a session, not a pane: {out:?}"
        );
    }
}
