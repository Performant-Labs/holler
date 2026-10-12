//! Where the six columns come from (acceptance 7 and 8, R2, R3 and A's W-2 and W-5): the
//! registry's records, read fresh on each call, as `pane list` prints them, with no adapter
//! call; and what the roster prints when the registry cannot answer or is not read.

use holler_cli::pane::list::{PaneRow, COLUMNS};
use holler_cli::roster_cmd::{Registry, PANE_COLUMNS};
use holler_cli::say_cmd::resolve_pane_target;
use holler_pane::pane::{Health, LastObserved};
use holler_pane::PaneError;
use holler_pane_testkit::fault::Fault;
use serde_json::{json, Value};

use crate::{
    base_panes, envelope_data, json_rows, last_six, pane, render, reply, table, Mode, Rig,
    BASE_ROWS, GOLDEN_NO_REGISTRY, NEW_KEYS, SUMMARY,
};

/// The six cells `pane list` prints for `pane`, taken by each roster column's name from
/// `pane list`'s own columns.
fn pane_list_cells(pane: &holler_pane::Pane) -> Vec<String> {
    let cells = PaneRow::from(pane).cells();
    PANE_COLUMNS
        .iter()
        .map(|name| {
            let at = COLUMNS.iter().position(|c| c == name);
            cells[at.unwrap_or_else(|| panic!("`{name}` is a column of `pane list`"))].clone()
        })
        .collect()
}

/// Acceptance 7 (and A's W-5): for every joined row, the six cells are the cells `pane list`
/// prints for the same record, column by column name, so a column `pane list` moves or adds
/// cannot shift the roster's.
#[test]
fn the_cells_are_pane_lists_cells_for_the_same_record() {
    let panes = base_panes();
    let rig = Rig::new(panes.clone());
    let run = rig.render(&reply(&BASE_ROWS), &[], Mode::Text);
    assert_eq!(run.code, 0, "{run:?}");
    let lines = table(&run.out);
    for (line, row) in lines[1..].iter().zip(BASE_ROWS) {
        let label = row.split('/').next().unwrap();
        let expected = match panes.iter().find(|p| p.name.as_str() == label) {
            Some(pane) => pane_list_cells(pane),
            None => vec!["-".to_owned(); 6],
        };
        assert_eq!(last_six(line), expected, "row {row}: {line}");
    }
}

/// Acceptance 7: the registry is read on each call. A record rewritten by another writer
/// between two calls shows its new values, and the summary line appears, then goes again.
#[test]
fn a_record_rewritten_between_calls_shows_its_new_values() {
    let rig = Rig::new(base_panes());
    let doc = reply(&BASE_ROWS);
    let first = rig.render(&doc, &[], Mode::Text);
    assert!(!first.out.contains(SUMMARY), "no mismatch yet: {first:?}");

    let mut demo = base_panes().remove(0);
    demo.last_observed = LastObserved {
        shown: Some("ses-new".to_owned()),
        driven: Some("ses-a".to_owned()),
        at: 2_000,
    };
    rig.panes.concurrent_put(&demo).unwrap();
    let second = rig.render(&doc, &[], Mode::Text);
    let first_line = second.out.lines().next().unwrap();
    assert_eq!(first_line, format!("1{SUMMARY}demo-c1r1"), "{second:?}");
    let demo_line = table(&second.out)[2];
    assert_eq!(
        last_six(demo_line)[4],
        "ses-new",
        "SHOWN is the new value: {demo_line}"
    );

    demo.last_observed.shown = Some("ses-a".to_owned());
    rig.panes.concurrent_put(&demo).unwrap();
    let third = rig.render(&doc, &[], Mode::Text);
    assert!(
        !third.out.contains(SUMMARY),
        "the line goes when the pair agrees again: {third:?}"
    );
}

/// Acceptance 7: the roster reads the registry and makes no Herdr, host, harness or probe
/// call, in any mode and under `--profile` (R1: not even `pane list`'s unmanaged reads).
#[test]
fn the_roster_reads_the_registry_and_calls_no_adapter() {
    let rig = Rig::new(base_panes());
    let doc = reply(&BASE_ROWS);
    for args in [&[][..], &["--profile", "fleet"]] {
        for mode in Mode::ALL {
            let run = rig.render(&doc, args, mode);
            assert_eq!(run.code, 0, "{args:?} {mode:?}: {run:?}");
            let joined = match mode {
                Mode::Text => table(&run.out)
                    .iter()
                    .any(|l| last_six(l)[0] == "demo-c1r1"),
                Mode::Legacy => json_rows(&run.out).iter().any(|r| r["pane"] == "demo-c1r1"),
                Mode::Envelope => envelope_data(&run)["rows"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["pane"] == "demo-c1r1"),
            };
            assert!(
                joined,
                "{args:?} {mode:?}: the store's record fills its row: {run:?}"
            );
        }
    }
    assert_eq!(rig.herdr.faults().calls(), vec![], "no Herdr call");
    assert_eq!(rig.host.faults().calls(), vec![], "no host call");
    assert_eq!(rig.harness.faults().calls(), vec![], "no harness call");
    assert_eq!(rig.prober.calls(), vec![], "no probe run");
}

/// Every row of a `--json` or `data.rows` array has the six keys, all `null`.
fn assert_six_nulls(rows: &[Value]) {
    assert_eq!(rows.len(), BASE_ROWS.len());
    for row in rows {
        for key in NEW_KEYS {
            assert_eq!(row.get(key), Some(&Value::Null), "`{key}` is null: {row}");
        }
    }
}

/// What every mode prints when no row is joined: the golden `-` table, six `null`s per JSON
/// row, `mismatches` `[]`, exit 0, and `err` exactly `note` (empty for none).
fn assert_unjoined<'a>(registry: impl Fn() -> Registry<'a>, note: &str) {
    let doc = reply(&BASE_ROWS);
    for mode in Mode::ALL {
        let run = render(&doc, registry(), &[], mode);
        assert_eq!((run.code, run.err.as_str()), (0, note), "{mode:?}: {run:?}");
        match mode {
            Mode::Text => assert_eq!(
                run.out, GOLDEN_NO_REGISTRY,
                "the hub's rows, `-` columns, no summary"
            ),
            Mode::Legacy => assert_six_nulls(&json_rows(&run.out)),
            Mode::Envelope => {
                let data = envelope_data(&run);
                assert_six_nulls(data["rows"].as_array().unwrap());
                assert_eq!(data["mismatches"], json!([]));
            }
        }
    }
}

/// A rig whose pane store's `list` (and every other method) fails with `error`.
fn failing_store(error: PaneError) -> Rig {
    let rig = Rig::new(base_panes());
    rig.panes.faults().set(Some(Fault::Fail(error)));
    rig
}

/// Acceptance 8 (i): a store that answers `not-implemented` is no failure and no note.
#[test]
fn a_not_implemented_store_leaves_the_columns_empty_and_says_nothing() {
    let rig = failing_store(PaneError::NotImplemented);
    assert_unjoined(|| Registry::Ports(rig.ports()), "");
}

/// Acceptance 8 (ii) and round 5: every other store error is one `note:` line naming its
/// stable code, and still exit 0, in every mode.
#[test]
fn any_other_store_error_is_one_note_line_with_its_code() {
    let errors = [
        PaneError::Unavailable {
            what: "the hub".to_owned(),
        },
        PaneError::Timeout {
            op: "pane_store.list".to_owned(),
        },
        PaneError::StoreCorrupt {
            what: "panes".to_owned(),
        },
    ];
    for error in errors {
        let note = format!("note: pane registry unavailable ({})\n", error.code());
        let rig = failing_store(error);
        assert_unjoined(|| Registry::Ports(rig.ports()), &note);
    }
}

/// R2: a `Wiring::connect` that failed after the hub answered is a registry error like the
/// store's, without `--profile`.
#[test]
fn a_failed_connect_is_a_registry_error() {
    let failed = || {
        Registry::Failed(PaneError::Unavailable {
            what: "the hub".to_owned(),
        })
    };
    assert_unjoined(failed, "note: pane registry unavailable (unavailable)\n");
}

/// R3: under `--server` the registry is not read: `-` columns, `null` keys, no summary line,
/// and one note naming why.
#[test]
fn a_registry_not_read_under_server_says_so() {
    assert_unjoined(
        || Registry::NotRead,
        "note: pane registry not read (--server names another hub)\n",
    );
}

/// A's W-2: the roster's mismatch rule is the one `say --pane` applies to a recorded pair.
/// For a record that is not parked, not unhealthy and has a session of record, the roster
/// names the pane exactly when `resolve_pane_target` refuses it `pane-shown-driven-mismatch`,
/// and that is only when both sides are known and differ.
#[test]
fn the_mismatch_rule_is_say_pane_targets_rule() {
    let pairs = [
        (Some("ses-a"), Some("ses-a"), false),
        (Some("ses-a"), Some("ses-b"), true),
        (Some("ses-a"), None, false),
        (None, Some("ses-b"), false),
        (None, None, false),
    ];
    for (shown, driven, expected) in pairs {
        let mut record = pane("demo-c1r1", (1, 1), None, Health::Healthy, (shown, driven));
        record.session_of_record = Some("ses-record".to_owned());
        let rig = Rig::new([record]);
        let run = rig.render(&reply(&["demo-c1r1/main"]), &[], Mode::Envelope);
        let named = envelope_data(&run)["mismatches"] == json!(["demo-c1r1"]);
        let refused = matches!(
            resolve_pane_target(rig.ports(), "demo-c1r1", None),
            Err(ref e) if e.code() == "pane-shown-driven-mismatch"
        );
        assert_eq!(
            named, refused,
            "{shown:?}/{driven:?}: the roster and `say --pane` agree"
        );
        assert_eq!(named, expected, "{shown:?}/{driven:?}");
    }
}
