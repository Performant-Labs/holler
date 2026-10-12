//! `profile-drift` (#665, D10): a pane whose record differs from its profile's spec on any
//! field of `profile_diff`'s table. It is `profile show`'s comparison, so the two agree; a spec
//! with no pane and a member with no spec are not drift. One finding per drifted pane per
//! pass, naming every differing field and no value, with no remedy; a pass keeps no memory, so
//! the same finding comes back every pass while the difference stands.
//!
//! The kind is read by its code (`"profile-drift"`), so this file names no enum variant.

use holler_cli::output::Format;
use holler_pane::findings::{quoted, Finding};
use holler_pane::reconcile::{ReconcileRequest, Report};
use holler_pane::{
    Actor, Argv, EnvVarName, GridPos, PaneError, PaneName, Profile, ProfileName, ProfileStore,
};
use holler_pane_testkit::pane_store::PaneStoreOp;
use holler_pane_testkit::profile_store::ProfileStoreOp;
use serde_json::{json, Value};

use super::rig::{json_data, json_findings, keys, whole, Rig, Seed};

const DRIFT: &str = "profile-drift";

/// The profile of the comparison fixture.
const P: &str = "Drift Fleet";

fn pname(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

fn actor() -> Actor {
    Actor::parse("drift-test").unwrap()
}

fn drifts(report: &Report) -> Vec<&Finding> {
    report
        .findings
        .iter()
        .filter(|f| f.kind.code() == DRIFT)
        .collect()
}

/// The drifted panes of `report`, each with the field paths its message names, in order.
fn drifted(report: &Report) -> Vec<(String, Vec<String>)> {
    drifts(report)
        .iter()
        .map(|f| {
            let pane = f.pane.as_ref().map(ToString::to_string).expect("a pane");
            (pane, fields_of(&f.message))
        })
        .collect()
}

/// The field paths a drift message names: `... on <field>, <field> (see holler profile show)`.
fn fields_of(message: &str) -> Vec<String> {
    let (_, tail) = message
        .rsplit_once(" on ")
        .unwrap_or_else(|| panic!("a drift message names its fields after ` on `: {message}"));
    let list = tail
        .strip_suffix(" (see holler profile show)")
        .unwrap_or_else(|| panic!("a drift message ends with the show hint: {message}"));
    list.split(", ").map(str::to_owned).collect()
}

/// Rewrite the stored profile `name` (another writer, outside the call log).
fn edit_profile(rig: &Rig, name: &str, change: impl FnOnce(&mut Profile)) {
    let mut profile = rig
        .profiles
        .get(&pname(name))
        .unwrap()
        .expect("the profile");
    change(&mut profile);
    rig.profiles.concurrent_put(&profile, &actor()).unwrap();
}

fn spec_mut<'p>(profile: &'p mut Profile, pane: &str) -> &'p mut holler_pane::ProfileSpec {
    profile
        .panes
        .iter_mut()
        .find(|spec| spec.pane == pane)
        .unwrap_or_else(|| panic!("a spec for {pane}"))
}

/// The comparison fixture: in `Drift Fleet`, a pane that matches (its spec lists the env in
/// another order), one that differs on one field, one that differs on three, a spec with no
/// pane and a member with no spec. `demo-alone` is in no profile.
fn comparison_rig() -> Rig {
    let rig = Rig::new(&[
        Seed::new("demo-match", 1, 1).in_profile(P),
        Seed::new("demo-one", 1, 2).in_profile(P),
        Seed::new("demo-three", 1, 3).in_profile(P),
        Seed::new("demo-extra", 2, 1).in_profile(P),
        Seed::new("demo-alone", 2, 2),
    ]);
    let env = |names: &[&str]| -> Vec<EnvVarName> {
        names
            .iter()
            .map(|n| EnvVarName::parse(n).unwrap())
            .collect()
    };
    rig.rewrite("demo-match", |pane| pane.env = env(&["DEMO_A", "DEMO_B"]));
    edit_profile(&rig, P, |profile| {
        spec_mut(profile, "demo-match").env = env(&["DEMO_B", "DEMO_A"]);
        spec_mut(profile, "demo-one").model.effort = "high".to_owned();
        let three = spec_mut(profile, "demo-three");
        three.herdr.grid = GridPos { row: 3, col: 3 };
        three.model.model_id = "demo-other-model".to_owned();
        three.context.hard += 1;
        profile.panes.retain(|spec| spec.pane != "demo-extra");
        let mut missing = profile.panes[0].clone();
        missing.pane = "demo-missing".to_owned();
        profile.panes.push(missing);
    });
    rig
}

/// Acceptance 14: the pass and `profile show` agree on which panes differ and on which
/// fields, in the same order; `missing` and `extra` rows are not drift, and `env` is
/// compared as a set.
#[test]
fn drift_agrees_with_profile_show_on_one_fixture() {
    let rig = comparison_rig();

    let report = rig.run(&whole(false));
    let shown = json_data(&rig.verb(&["profile", "show", P], Format::Json));

    let from_show: Vec<(String, Vec<String>)> = shown["comparison"]
        .as_array()
        .expect("comparison rows")
        .iter()
        .filter(|row| row["status"] == "differs")
        .map(|row| {
            let fields = row["differences"].as_array().unwrap().iter();
            let fields = fields.map(|d| d["field"].as_str().unwrap().to_owned());
            (row["pane"].as_str().unwrap().to_owned(), fields.collect())
        })
        .collect();
    let want = vec![
        ("demo-one".to_owned(), vec!["model.effort".to_owned()]),
        (
            "demo-three".to_owned(),
            vec![
                "herdr.grid".to_owned(),
                "model.model_id".to_owned(),
                "context.hard".to_owned(),
            ],
        ),
    ];
    assert_eq!(
        from_show, want,
        "the fixture is what this test says: {shown}"
    );
    let mut found = drifted(&report);
    found.sort();
    assert_eq!(found, want, "doctor and show agree: {:?}", keys(&report));
}

/// Acceptance 15: one finding per drifted pane naming every differing field, the same one
/// every pass, none once repaired, and one again when it drifts again.
#[test]
fn drift_is_one_finding_per_pane_per_pass_for_as_long_as_it_stands() {
    let rig = Rig::new(&[Seed::new("demo-c1r1", 1, 1).in_profile(P)]);
    let original = rig.record("demo-c1r1");
    let drift = |pane: &mut holler_pane::Pane| {
        pane.model.model_id = "demo-drifted".to_owned();
        pane.model.effort = "low".to_owned();
        pane.context.hard += 7;
    };
    rig.rewrite("demo-c1r1", drift);

    let first = rig.run(&whole(false));
    let second = rig.run(&whole(false));
    let fields = vec![
        "model.model_id".to_owned(),
        "model.effort".to_owned(),
        "context.hard".to_owned(),
    ];
    let want = vec![("demo-c1r1".to_owned(), fields)];
    assert_eq!(drifted(&first), want, "one finding, every field");
    assert_eq!(drifted(&second), want, "the same finding again, no memory");

    rig.rewrite("demo-c1r1", |pane| {
        pane.model = original.model.clone();
        pane.context = original.context;
    });
    assert_eq!(drifted(&rig.run(&whole(false))), vec![], "repaired: none");

    rig.rewrite("demo-c1r1", drift);
    assert_eq!(drifts(&rig.run(&whole(false))).len(), 1, "a new episode");
}

/// Acceptance 16: a pass over a profiled pane reads the profiles with one `list` and writes
/// no profile; the pane-store calls are a pass's usual ones. (A `--profile` pass also reads
/// the profile through the scope, `get`: not pinned here.)
#[test]
fn a_pass_over_a_profiled_pane_reads_the_profiles_once_and_writes_none() {
    let rig = Rig::new(&[
        Seed::new("demo-c1r1", 1, 1).in_profile(P),
        Seed::new("demo-c2r1", 1, 2),
    ]);
    let name = PaneName::parse("demo-c1r1").unwrap();
    for request in [
        whole(false),
        ReconcileRequest {
            pane: Some(&name),
            ..whole(false)
        },
    ] {
        let mark = rig.mark();
        rig.run(&request);
        let calls = rig.calls_since(&mark);
        assert_eq!(calls.profiles, vec![ProfileStoreOp::List], "{request:?}");
        let reads = [PaneStoreOp::List, PaneStoreOp::Get, PaneStoreOp::CasPut];
        assert!(
            calls.panes.iter().all(|op| reads.contains(op)),
            "{:?}",
            calls.panes
        );
    }
}

/// Acceptance 16, the other half: a pass over panes in no profile makes no profile call.
#[test]
fn a_pass_over_unprofiled_panes_makes_no_profile_call() {
    let rig = Rig::new(&[Seed::new("demo-c1r1", 1, 1)]);
    let mark = rig.mark();
    rig.run(&whole(false));
    assert_eq!(rig.calls_since(&mark).profiles, vec![]);
}

/// Acceptance 17: the scope of the pass is the scope of the drift.
#[test]
fn drift_follows_the_scope_of_the_pass() {
    let rig = Rig::new(&[
        Seed::new("demo-p", 1, 1).in_profile("Pfleet"),
        Seed::new("demo-q", 1, 2).in_profile("Qfleet"),
    ]);
    for name in ["demo-p", "demo-q"] {
        rig.rewrite(name, |pane| pane.context.soft += 1);
    }
    let names = |report: &Report| -> Vec<String> {
        drifted(report).into_iter().map(|(pane, _)| pane).collect()
    };

    assert_eq!(names(&rig.run(&whole(false))), ["demo-p", "demo-q"]);
    let p = pname("Pfleet");
    let scoped = rig.run(&ReconcileRequest {
        profile: Some(&p),
        ..whole(false)
    });
    assert_eq!(names(&scoped), ["demo-p"]);
    let q = PaneName::parse("demo-q").unwrap();
    let one = rig.run(&ReconcileRequest {
        pane: Some(&q),
        ..whole(false)
    });
    assert_eq!(names(&one), ["demo-q"]);
}

/// Acceptance 18: a failed read of the profiles is one `observe-failed` naming
/// `profile_store.list` and its code, no drift is concluded, the rest of the pass is
/// unchanged, and the verb exits 0.
#[test]
fn a_failed_profile_read_is_observe_failed_and_concludes_no_drift() {
    let rig = Rig::new(&[
        Seed::new("demo-c1r1", 1, 1).in_profile(P),
        Seed::new("demo-c2r1", 1, 2),
    ]);
    rig.rewrite("demo-c1r1", |pane| pane.context.soft += 1);
    rig.move_tui_to_new_session("demo-c2r1");
    let others = |report: &Report| -> Vec<(&'static str, Option<String>, Option<String>)> {
        keys(report)
            .into_iter()
            .filter(|(kind, _, _)| *kind != DRIFT && *kind != "observe-failed")
            .collect()
    };
    let baseline = rig.run(&whole(false));
    let fail = || {
        let error = PaneError::Unavailable {
            what: "the profile registry".to_owned(),
        };
        rig.profiles.faults().fail_next(ProfileStoreOp::List, error);
    };

    fail();
    let report = rig.run(&whole(false));

    assert!(drifts(&report).is_empty(), "{:?}", keys(&report));
    let failed: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|f| f.kind.code() == "observe-failed")
        .collect();
    assert_eq!(failed.len(), 1, "{:?}", keys(&report));
    assert!(
        failed[0].message.contains("profile_store.list")
            && failed[0].message.contains("unavailable"),
        "{}",
        failed[0].message
    );
    assert_eq!(others(&report), others(&baseline), "the rest of the pass");
    for format in [Format::Text, Format::Json] {
        fail();
        let run = rig.verb(&["pane", "doctor"], format);
        assert_eq!(run.code, 0, "{format:?}: {run:?}");
    }
}

/// Acceptance 19: the finding's shape, its text line, a quoted profile name and no value.
#[test]
fn a_drift_finding_has_its_shape_and_never_quotes_a_value() {
    const NAME: &str = r#"Drift "quoted" \ fleet"#;
    let rig = Rig::new(&[Seed::new("demo-c1r2", 2, 1).in_profile(NAME)]);
    edit_profile(&rig, NAME, |profile| {
        let spec = &mut profile.panes[0];
        spec.host.cwd = "/srv/t665-cwd-canary".to_owned();
        spec.command = Some(Argv::new(vec!["t665-command-canary".to_owned()]));
    });

    let report = rig.run(&whole(false));
    let found = drifts(&report);
    assert_eq!(found.len(), 1, "{:?}", keys(&report));
    let message = &found[0].message;
    assert_eq!(
        message,
        &format!(
            "differs from profile {} on host.cwd, command (see holler profile show)",
            quoted(NAME)
        )
    );
    for value in ["t665-cwd-canary", "t665-command-canary", "/srv/demo"] {
        assert!(
            !message.contains(value),
            "a value in the message: {message}"
        );
    }

    let data = json_data(&rig.verb(&["pane", "doctor"], Format::Json));
    let json = json_findings(&data, DRIFT);
    assert_eq!(json.len(), 1, "{data}");
    let finding: &Value = json[0];
    assert_eq!(finding["pane"], json!("demo-c1r2"));
    assert_eq!(finding["grid"], json!({"row": 2, "col": 1, "pos": "r2c1"}));
    assert_eq!(finding["remedy"], Value::Null);
    assert_eq!(finding["fix"], json!("not-fixable"));
    assert_eq!(finding["fix_error"], Value::Null);
    assert_eq!(finding["message"], json!(message));

    let text = rig.verb(&["pane", "doctor"], Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    let line = format!("demo-c1r2 r2c1 profile-drift: {message}");
    assert!(
        text.out.lines().any(|l| l == line),
        "the text line, with no `(run: ...)`: {}",
        text.out
    );
}
