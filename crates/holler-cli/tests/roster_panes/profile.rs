//! `--profile P` (acceptance 4, 6, 10 and 14, R4, A's W-1 and W-4): only the rows joined to a
//! pane of P, in roster order; an unknown P is a refusal; a malformed P is a usage error; and
//! a registry that cannot resolve P is an error, never an empty list.

use holler_cli::roster_cmd::Registry;
use holler_pane::PaneError;
use holler_pane_testkit::envelope::check_envelope;
use holler_pane_testkit::fault::Fault;
use serde_json::{json, Value};

use crate::{
    base_panes, envelope_data, json_rows, reply, sessions, two_mismatch_panes, Mode, Rig, Run,
    BASE_ROWS, SUMMARY, TWO_ROWS,
};

/// The row names a run printed, in its mode's form.
fn printed_rows(run: &Run, mode: Mode) -> Vec<String> {
    let names = |rows: &[Value]| {
        rows.iter()
            .map(|r| r["name"].as_str().unwrap().to_owned())
            .collect()
    };
    match mode {
        Mode::Text => crate::sessions(&run.out)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        Mode::Legacy => names(&json_rows(&run.out)),
        Mode::Envelope => names(envelope_data(run)["rows"].as_array().unwrap()),
    }
}

/// Acceptance 6 and 10: `--profile fleet` lists the rows joined to a pane of `fleet`, in the
/// reply's order, and drops the rows with no pane and the rows of a pane of no profile.
#[test]
fn profile_lists_only_the_rows_of_its_panes_in_roster_order() {
    let rig = Rig::new(base_panes());
    let doc = reply(&BASE_ROWS);
    for mode in Mode::ALL {
        let run = rig.render(&doc, &["--profile", "fleet"], mode);
        assert_eq!((run.code, run.err.as_str()), (0, ""), "{mode:?}: {run:?}");
        assert_eq!(
            printed_rows(&run, mode),
            ["demo-c1r1/main", "zz-c2r2/work"],
            "{mode:?}"
        );
    }
    let all = rig.render(&doc, &[], Mode::Text);
    assert_eq!(
        sessions(&all.out),
        BASE_ROWS,
        "without --profile every row is listed"
    );
}

/// Scope item 4 (round 4): `--profile` changes which rows are printed, so the summary line
/// and `data.mismatches` name only the panes of P.
#[test]
fn profile_narrows_the_summary_line_to_its_panes() {
    let rig = Rig::new(two_mismatch_panes());
    let doc = reply(&TWO_ROWS);
    let text = rig.render(&doc, &["--profile", "fleet"], Mode::Text);
    assert_eq!(
        text.out.lines().next(),
        Some(format!("1{SUMMARY}demo-c1r1").as_str()),
        "{text:?}"
    );
    let json = rig.render(&doc, &["--profile", "fleet"], Mode::Envelope);
    assert_eq!(envelope_data(&json)["mismatches"], json!(["demo-c1r1"]));
}

/// An error a run printed: nothing on `out` and exactly one `error:` line on `err` in text
/// and under `--json`; one failure envelope with `code` and nothing on `err` under
/// `--format=json`; `exit` in every mode.
fn assert_error(run: &Run, mode: Mode, exit: i32, code: &str) {
    assert_eq!(run.code, exit, "{mode:?}: {run:?}");
    match mode {
        Mode::Text | Mode::Legacy => {
            assert_eq!(run.out, "", "{mode:?}: nothing on out: {run:?}");
            let lines: Vec<&str> = run.err.lines().collect();
            assert_eq!(lines.len(), 1, "{mode:?}: one line on err: {run:?}");
            assert!(lines[0].starts_with("error: "), "{mode:?}: {run:?}");
        }
        Mode::Envelope => {
            let envelope =
                check_envelope(&run.out, run.code).unwrap_or_else(|f| panic!("{f:?}: {run:?}"));
            assert_eq!(envelope.error.unwrap().code, code, "{run:?}");
            assert_eq!(run.err, "", "nothing on err: {run:?}");
        }
    }
}

/// Acceptance 4 and 6, and A's W-1: an unknown profile is refused `profile-not-found`, exit
/// 3, in every mode, even when the hub has no row.
#[test]
fn an_unknown_profile_is_refused_in_every_mode() {
    let rig = Rig::new(base_panes());
    for doc in [reply(&BASE_ROWS), reply(&[])] {
        for mode in Mode::ALL {
            let run = rig.render(&doc, &["--profile", "nosuch"], mode);
            assert_error(&run, mode, 3, "profile-not-found");
        }
    }
}

/// Acceptance 4 and A's W-4: a malformed profile name is a usage error, which under `roster`
/// is one `error:` line on err with nothing on out and exit 2, in every mode (no envelope).
#[test]
fn a_malformed_profile_name_is_a_usage_line_in_every_mode() {
    let rig = Rig::new(base_panes());
    for mode in Mode::ALL {
        let run = rig.render(&reply(&BASE_ROWS), &["--profile", "!!!"], mode);
        assert_eq!((run.code, run.out.as_str()), (2, ""), "{mode:?}: {run:?}");
        let lines: Vec<&str> = run.err.lines().collect();
        assert_eq!(lines.len(), 1, "{mode:?}: {run:?}");
        assert!(lines[0].starts_with("error: "), "{mode:?}: {run:?}");
    }
}

/// Acceptance 14 (R4): `--profile P` over a pane store that cannot answer is that error, exit
/// 1, with nothing else printed (no `note:` line beside it); without `--profile` the same
/// store is acceptance 8's note and exit 0.
#[test]
fn profile_over_a_registry_that_cannot_answer_is_its_error() {
    let rig = Rig::new(base_panes());
    rig.panes
        .faults()
        .set(Some(Fault::Fail(PaneError::Unavailable {
            what: "the hub".to_owned(),
        })));
    let doc = reply(&BASE_ROWS);
    for mode in Mode::ALL {
        let run = rig.render(&doc, &["--profile", "fleet"], mode);
        assert_error(&run, mode, 1, "unavailable");
        let unscoped = rig.render(&doc, &[], mode);
        assert_eq!(unscoped.code, 0, "{mode:?}: {unscoped:?}");
        assert_eq!(
            unscoped.err,
            "note: pane registry unavailable (unavailable)\n"
        );
    }
}

/// A's W-1: under `--profile`, a `Wiring::connect` that failed is R4's error (exit 1), not
/// R2's note over an empty listing, which would read as "P has no pane".
#[test]
fn profile_after_a_failed_connect_is_its_error() {
    for mode in Mode::ALL {
        let failed = Registry::Failed(PaneError::Unavailable {
            what: "the hub".to_owned(),
        });
        let run = crate::render(&reply(&BASE_ROWS), failed, &["--profile", "fleet"], mode);
        assert_error(&run, mode, 1, "unavailable");
    }
}
