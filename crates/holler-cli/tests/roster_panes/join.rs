//! The join (acceptance 10): a hub row carries a pane's cells when the part of its name before
//! the first `/` is the pane's name. A row with no label is joined by its whole name, and a
//! labelled row is never joined by the session after its `/`.

use holler_pane::pane::Health;
use serde_json::{json, Value};

use crate::{json_rows, last_six, pane, reply, table, Mode, Rig, NEW_KEYS};

/// Rows `hj-c3r1` (no label, a pane of that name) and `other/demo-c1r1` (label `other`, whose
/// session is named like the pane `demo-c1r1`).
fn rig() -> Rig {
    Rig::new([
        pane(
            "hj-c3r1",
            (1, 3),
            Some("fleet"),
            Health::Healthy,
            (Some("s1"), Some("s1")),
        ),
        pane(
            "demo-c1r1",
            (1, 1),
            None,
            Health::Healthy,
            (Some("s2"), Some("s2")),
        ),
    ])
}

const ROWS: [&str; 2] = ["hj-c3r1", "other/demo-c1r1"];

#[test]
fn a_row_with_no_label_joins_the_pane_of_its_whole_name() {
    let rig = rig();
    let text = rig.render(&reply(&ROWS), &[], Mode::Text);
    let lines = table(&text.out);
    assert_eq!(
        last_six(lines[1]),
        ["hj-c3r1", "r1c3", "fleet", "healthy", "s1", "s1"],
        "{text:?}"
    );

    let rows = json_rows(&rig.render(&reply(&ROWS), &[], Mode::Legacy).out);
    let six: Vec<&Value> = NEW_KEYS.iter().map(|k| &rows[0][*k]).collect();
    let expected = [
        json!("hj-c3r1"),
        json!({ "row": 1, "col": 3, "pos": "r1c3" }),
        json!("fleet"),
        json!("healthy"),
        json!("s1"),
        json!("s1"),
    ];
    assert_eq!(six, expected.iter().collect::<Vec<_>>());
}

#[test]
fn a_labelled_row_is_never_joined_by_its_session() {
    let rig = rig();
    let text = rig.render(&reply(&ROWS), &[], Mode::Text);
    assert_eq!(last_six(table(&text.out)[2]), ["-"; 6], "{text:?}");
    let rows = json_rows(&rig.render(&reply(&ROWS), &[], Mode::Legacy).out);
    for key in NEW_KEYS {
        assert_eq!(rows[1][key], Value::Null, "`{key}`: {}", rows[1]);
    }
}
