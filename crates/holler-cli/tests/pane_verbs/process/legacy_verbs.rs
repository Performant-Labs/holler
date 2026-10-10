//! `say`, `interrupt`, `answer` and `roster` with `--pane` / `--profile` (story #670,
//! decision 5; the pane forms routed by story #646), on the real binary.
//!
//! No hub is running and the wiring is still #670's unwired port set, so a routed
//! form fails on that — what these pin is the behaviour around the hub: the #646 stub
//! refusal is gone (the forms route now, and fail as pane-path failures, never as the
//! old `not implemented (story #646)` line), `--profile P` without `--pane` is a
//! usage error (the prompt verbs still need a pane, ADR-0021 section 3), and a
//! malformed form is still exit 2. A `--pane` that parsed and was ignored would
//! deliver a prompt unchecked; the routed engine (`say_cmd::resolve_pane_target`,
//! tested in `pane_verbs/target_flags.rs`) is what these forms now run.

use crate::stub::ROSTER_PROFILE_REFUSAL as ROSTER_REFUSAL;
use crate::{assert_no_failures, holler};

/// The #646 stub refusal line, spelled once: what no routed form may print again.
const OLD_STUB: &str = "error: not implemented (story #646)";

/// Every form that names a pane on `say`, `interrupt` and `answer`.
const PANE_FORMS: &[&[&str]] = &[
    &["say", "--pane", "demo-c1r1", "hello"],
    &["say", "--pane", "demo-c1r1", "--profile", "demo", "hello"],
    &["say", "--pane", "demo-c1r1", "hello", "--queue"],
    &["interrupt", "--pane", "demo-c1r1"],
    &["interrupt", "--pane", "demo-c1r1", "stop"],
    &["answer", "--pane", "demo-c1r1", "yes"],
];

/// The pane forms route (story #646): with no hub and no wiring they fail — exit 1,
/// the failure on stderr, nothing on stdout — but never with the #646 stub line, and
/// never as a usage error: they are valid forms.
#[test]
fn the_pane_forms_route_and_never_print_the_646_stub() {
    let mut failures = Vec::new();
    for argv in PANE_FORMS {
        let out = holler(argv);
        if out.code != 1
            || !out.stdout.is_empty()
            || out.stderr_has_line(OLD_STUB)
            || !out.stderr.contains("error:")
        {
            failures.push(format!(
                "holler {argv:?}: want exit 1, empty stdout, an error line and no {OLD_STUB:?}; got {out:?}"
            ));
        }
    }
    assert_no_failures(failures);
}

/// The routed forms keep the legacy verbs' plain-text shape under every format: no
/// envelope, nothing on stdout, the failure on stderr.
#[test]
fn the_pane_forms_stay_plain_text_under_every_format() {
    let mut failures = Vec::new();
    for argv in PANE_FORMS {
        for format in [
            &["--json"][..],
            &["--format=json"],
            &["--format", "json"],
            &["--format=text"],
        ] {
            let mut full: Vec<&str> = argv.to_vec();
            full.extend_from_slice(format);
            let out = holler(&full);
            if out.code != 1 || !out.stdout.is_empty() || out.stderr_has_line(OLD_STUB) {
                failures.push(format!(
                    "holler {full:?}: want exit 1, empty stdout, no {OLD_STUB:?}; got {out:?}"
                ));
            }
        }
    }
    assert_no_failures(failures);
}

/// `--profile P` without `--pane` names no target: the prompt verbs still need a pane
/// (ADR-0021 section 3, the scoping class), so it is a usage error whose message
/// names `--pane`.
#[test]
fn a_profile_without_a_pane_is_a_usage_error() {
    for argv in [
        &["say", "--profile", "demo", "io/alpha", "hello"][..],
        &["interrupt", "--profile", "demo", "io/alpha"],
        &["answer", "--profile", "demo", "io/alpha", "yes"],
    ] {
        let out = holler(argv);
        assert_eq!(out.code, 2, "{argv:?}: {out:?}");
        assert!(out.stdout.is_empty(), "{argv:?}: {out:?}");
        assert!(
            out.stderr.to_lowercase().contains("--pane"),
            "{argv:?}: the message names --pane: {out:?}"
        );
    }
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

/// A usage error wins over any routing: a `--pane` form that is malformed is exit 2,
/// and its message is not a failure line.
#[test]
fn a_malformed_pane_form_is_a_usage_error_not_a_failure() {
    let cases: [(&[&str], &str); 9] = [
        (&["say"], "session"),
        (&["say", "io/alpha"], "text"),
        (&["say", "--pane", "demo-c1r1"], "text"),
        (&["say", "--pane", "demo-c1r1", "a", "b"], "positionals"),
        (
            &["say", "--pane", "demo-c1r1", "io/alpha", "hello"],
            "positionals",
        ),
        (&["interrupt"], "session"),
        (&["answer"], "session"),
        (&["answer", "io/alpha"], "choice"),
        (&["answer", "--pane", "demo-c1r1"], "choice"),
    ];
    let mut failures = Vec::new();
    for (argv, names) in cases {
        let out = holler(argv);
        let lower = out.stderr.to_lowercase();
        if out.code != 2
            || !out.stdout.is_empty()
            || out.stderr_has_line(OLD_STUB)
            || lower.contains("no live holler hub")
            || !lower.contains(names)
        {
            failures.push(format!(
                "holler {argv:?}: want exit 2 naming {names:?}, no stub, no hub failure; got {out:?}"
            ));
        }
    }
    assert_no_failures(failures);
}

/// The existing SESSION forms are untouched by the routing: they still reach for a
/// hub (and fail on there being none), never on a usage error and never on the stub.
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
            !out.stderr_has_line(OLD_STUB),
            "{argv:?} names a session, not a pane: {out:?}"
        );
    }
}
