#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #662
//! The spec-versus-live comparison (#662 AC 2): `profile_diff::{diff_spec,
//! diff_profile, is_member}`, the field table `SpecField` and the typed
//! `FieldValue`. Pure functions over records: no port, no fake.

mod common;

use common::{pane, pane_name, profile, profile_name, spec};
use holler_pane::pane::PaneRole;
use holler_pane::profile_diff::{
    diff_profile, diff_spec, is_member, FieldDiff, FieldValue, PaneStatus, SpecField,
};
use holler_pane::profile_snapshot::spec_from_pane;
use holler_pane::{Argv, EnvVarName, GridPos, Pane, ProfileSpec};
use serde_json::{json, Value};

fn argv(parts: &[&str]) -> Argv {
    Argv::new(parts.iter().map(|p| (*p).to_owned()).collect())
}

fn env(names: &[&str]) -> Vec<EnvVarName> {
    names
        .iter()
        .map(|n| EnvVarName::parse(n).unwrap())
        .collect()
}

fn text(s: &str) -> FieldValue {
    FieldValue::Text(s.to_owned())
}

fn list(items: &[&str]) -> FieldValue {
    FieldValue::List(items.iter().map(|s| (*s).to_owned()).collect())
}

/// `common::pane()` renamed to `name`.
fn named(name: &str) -> Pane {
    let mut p = pane();
    p.name = pane_name(name);
    p
}

/// A pane with no command, no check, no env and no expect: the `None`/empty side.
fn sparse_pane() -> Pane {
    let mut p = named("demo-c1r2");
    p.herdr.grid = GridPos { row: 2, col: 1 };
    p.role = PaneRole::Orchestrator;
    p.env = Vec::new();
    p.command = None;
    p.probe.check = None;
    p.probe.expect = Vec::new();
    p
}

fn fields(diffs: &[FieldDiff]) -> Vec<SpecField> {
    diffs.iter().map(|d| d.field).collect()
}

#[test]
fn diff_reports_each_differing_field_once() {
    let p = pane();
    let spec = spec_from_pane(&p);
    let mut live = p.clone();
    // Changed in the reverse of `ALL` order: the result is still in `ALL` order.
    live.model.effort = "low".to_owned();
    live.host.cwd = "/srv/demo".to_owned();
    live.herdr.grid = GridPos { row: 2, col: 1 };

    let diffs = diff_spec(&spec, &live);
    assert_eq!(
        diffs,
        vec![
            FieldDiff {
                field: SpecField::Grid,
                spec: FieldValue::Grid(GridPos { row: 1, col: 1 }),
                live: FieldValue::Grid(GridPos { row: 2, col: 1 }),
            },
            FieldDiff {
                field: SpecField::Cwd,
                spec: text("/work/holler"),
                live: text("/srv/demo"),
            },
            FieldDiff {
                field: SpecField::Effort,
                spec: text("high"),
                live: text("low"),
            },
        ]
    );
    // The raw string, not re-parsed: row before col, and the dotted field path.
    assert_eq!(
        serde_json::to_string(&diffs[0]).unwrap(),
        r#"{"field":"herdr.grid","spec":{"row":1,"col":1,"pos":"r1c1"},"live":{"row":2,"col":1,"pos":"r2c1"}}"#
    );
}

#[test]
fn diff_compares_env_and_expect_as_sets_and_argv_in_order() {
    let mut p = pane();
    p.env = env(&["ALPHA_TOKEN", "BETA_URL"]);
    p.probe.expect = vec!["qwen38".to_owned(), "ready".to_owned()];
    let spec = spec_from_pane(&p);

    // A reorder or a repeat of env names or expected strings is not a difference.
    let mut reordered = p.clone();
    reordered.env = env(&["BETA_URL", "ALPHA_TOKEN", "BETA_URL"]);
    reordered.probe.expect = vec!["ready".to_owned(), "qwen38".to_owned(), "ready".to_owned()];
    assert_eq!(diff_spec(&spec, &reordered), vec![]);

    // A different set is, with both sides in their stored order.
    let mut other_set = p.clone();
    other_set.env = env(&["GAMMA_KEY"]);
    other_set.probe.expect = vec!["qwen38".to_owned()];
    assert_eq!(
        diff_spec(&spec, &other_set),
        vec![
            FieldDiff {
                field: SpecField::Env,
                spec: list(&["ALPHA_TOKEN", "BETA_URL"]),
                live: list(&["GAMMA_KEY"]),
            },
            FieldDiff {
                field: SpecField::Expect,
                spec: list(&["qwen38", "ready"]),
                live: list(&["qwen38"]),
            },
        ]
    );

    // Argv is ordered: the same elements in another order differ, command and check alike.
    let mut swapped = p.clone();
    let command: Vec<String> = p.command.clone().unwrap().into_vec();
    let check: Vec<String> = p.probe.check.clone().unwrap().into_vec();
    swapped.command = Some(Argv::new(command.iter().rev().cloned().collect()));
    swapped.probe.check = Some(Argv::new(check.iter().rev().cloned().collect()));
    let diffs = diff_spec(&spec, &swapped);
    assert_eq!(fields(&diffs), vec![SpecField::Command, SpecField::Check]);
    assert_eq!(diffs[0].spec, FieldValue::Argv(p.command.clone()));
    assert_eq!(diffs[0].live, FieldValue::Argv(swapped.command.clone()));

    // An argv that is absent on one side differs from one that is present.
    let mut no_command = p.clone();
    no_command.command = None;
    let diffs = diff_spec(&spec, &no_command);
    assert_eq!(fields(&diffs), vec![SpecField::Command]);
    assert_eq!(diffs[0].live, FieldValue::Argv(None));
}

#[test]
fn snapshot_round_trips_to_no_difference() {
    for p in [pane(), sparse_pane()] {
        assert_eq!(diff_spec(&spec_from_pane(&p), &p), vec![], "{}", p.name);
    }
}

#[test]
fn diff_profile_classifies_matches_differs_missing_extra() {
    let a = named("demo-c1r1");
    let b = named("demo-c2r1");
    let mut prof = profile();
    prof.panes = vec![
        spec_from_pane(&a),
        spec_from_pane(&b),
        ProfileSpec {
            pane: "demo-c3r1".to_owned(),
            ..spec_from_pane(&a)
        },
    ];
    let mut b_live = b.clone();
    b_live.model.model_id = "demo-model-2".to_owned();
    let live = vec![named("demo-c9r1"), a.clone(), named("demo-c8r1"), b_live];

    let rows = diff_profile(&prof, &live);
    let got: Vec<(&str, PaneStatus)> = rows.iter().map(|r| (r.pane.as_str(), r.status)).collect();
    assert_eq!(
        got,
        vec![
            ("demo-c1r1", PaneStatus::Matches),
            ("demo-c2r1", PaneStatus::Differs),
            ("demo-c3r1", PaneStatus::Missing),
            ("demo-c9r1", PaneStatus::Extra),
            ("demo-c8r1", PaneStatus::Extra),
        ],
        "spec rows in the profile's order, then extras in the order of `live`"
    );
    for row in &rows {
        assert_eq!(
            !row.differences.is_empty(),
            row.status == PaneStatus::Differs,
            "{row:?}"
        );
    }
    assert_eq!(fields(&rows[1].differences), vec![SpecField::ModelId]);
    assert_eq!(
        serde_json::to_value(&rows[2]).unwrap(),
        json!({"pane": "demo-c3r1", "status": "missing", "differences": []})
    );
}

/// The dotted paths, in `ALL` order: the field table is public API (#664, #665).
const PATHS: [&str; 15] = [
    "herdr.workspace",
    "herdr.grid",
    "host.cwd",
    "harness.kind",
    "harness.port_policy",
    "model.provider",
    "model.model_id",
    "model.effort",
    "role",
    "env",
    "context.soft",
    "context.hard",
    "command",
    "check",
    "expect",
];

#[test]
fn spec_field_values_serialize_like_the_spec() {
    let got: Vec<&str> = SpecField::ALL.iter().map(|f| f.as_str()).collect();
    assert_eq!(got, PATHS);

    let mut bare = spec();
    bare.command = None;
    bare.check = None;
    bare.role = PaneRole::Orchestrator;
    for one in [spec(), bare] {
        let whole = serde_json::to_value(&one).unwrap();
        for field in SpecField::ALL {
            assert_eq!(
                serde_json::to_value(field).unwrap(),
                Value::String(field.as_str().to_owned()),
                "{field:?} serializes as its path"
            );
            let pointer = format!("/{}", field.as_str().replace('.', "/"));
            let want = whole.pointer(&pointer).cloned().unwrap_or(Value::Null);
            assert_eq!(
                serde_json::to_value(field.value(&one)).unwrap(),
                want,
                "{field:?} at {pointer}"
            );
        }
    }
}

#[test]
fn field_text_escapes_control_characters() {
    let shown = text("/srv/a\u{1b}[31mb\nc").to_string();
    assert_eq!(shown, r"/srv/a\u{1b}[31mb\nc");
    assert!(
        !shown.contains('\u{1b}') && !shown.contains('\n'),
        "{shown:?}"
    );

    assert_eq!(
        FieldValue::Argv(Some(argv(&["opencode", "serve"]))).to_string(),
        r#"["opencode","serve"]"#
    );
    // An element with a space stays one element: printed as JSON, never shell-joined.
    assert_eq!(
        FieldValue::Argv(Some(argv(&["sh", "a b"]))).to_string(),
        r#"["sh","a b"]"#
    );

    // DEL and a C1 control (CSI, U+009B) are escaped too, in text and in a JSON array
    // alike (`serde_json` alone writes both raw), so none reaches a terminal.
    let raw = "a\u{9b}2Jb\u{7f}";
    let shown = [
        text(raw).to_string(),
        list(&[raw]).to_string(),
        FieldValue::Argv(Some(argv(&["sh", raw]))).to_string(),
    ];
    assert_eq!(
        shown,
        [
            r"a\u{9b}2Jb\u{7f}".to_owned(),
            r#"["a\u009b2Jb\u007f"]"#.to_owned(),
            r#"["sh","a\u009b2Jb\u007f"]"#.to_owned(),
        ]
    );
    for s in &shown {
        assert!(!s.chars().any(char::is_control), "{s:?}");
    }
    // The array form is still the JSON of the same strings.
    let parsed: Vec<String> = serde_json::from_str(&shown[1]).unwrap();
    assert_eq!(parsed, vec![raw.to_owned()]);
}

#[test]
fn field_values_print_for_a_person() {
    assert_eq!(
        FieldValue::Grid(GridPos { row: 2, col: 1 }).to_string(),
        "r2c1"
    );
    assert_eq!(FieldValue::Number(100_000).to_string(), "100000");
    assert_eq!(
        list(&["ALPHA_TOKEN", "BETA_URL"]).to_string(),
        r#"["ALPHA_TOKEN","BETA_URL"]"#
    );
    assert_eq!(FieldValue::Argv(None).to_string(), "none");
    assert_eq!(text("scratch").to_string(), "scratch");
}

#[test]
fn diff_compares_port_policy_as_the_live_port() {
    let mut p = pane();
    p.harness.port = 48101;
    let mut spec = spec_from_pane(&p);
    spec.harness.port_policy = "fixed:48100".to_owned();
    assert_eq!(
        diff_spec(&spec, &p),
        vec![FieldDiff {
            field: SpecField::PortPolicy,
            spec: text("fixed:48100"),
            live: text("fixed:48101"),
        }]
    );

    spec.harness.port_policy = "fixed".to_owned();
    assert_eq!(fields(&diff_spec(&spec, &p)), vec![SpecField::PortPolicy]);
}

#[test]
fn is_member_compares_slugs() {
    let some = profile_name("Some Profile");
    let mut p = pane();
    p.profile = Some(profile_name("SOME-PROFILE"));
    assert!(is_member(&p, &some));
    p.profile = Some(profile_name("Some Profile"));
    assert!(is_member(&p, &some));
    p.profile = Some(profile_name("Other"));
    assert!(!is_member(&p, &some));
    p.profile = None;
    assert!(!is_member(&p, &some));
}
