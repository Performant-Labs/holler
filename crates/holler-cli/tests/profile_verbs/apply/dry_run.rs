//! What a dry run calls and prints (brief §6, C1, C2, C6 and C19; C14 entries 4, 7, 11 and 31).
//! A dry run past §4's gate whose plan has no `failed` step prints its plan and exits 0; the
//! failing dry run is `failures.rs`'s, and the gate's is `apply.rs`'s.
//!
//! A dry run's step that would act (rows b, c and f) is `planned` (C22), claiming no effect it
//! did not have; row d's is `reported`.

use holler_cli::output::Format;
use holler_pane::tx_apply::{Action, Outcome as StepOutcome};
use holler_pane::AgentKey;
use serde_json::{json, Value};

use super::rig::{
    apply_argv, assert_only_observed, cell, data_of, every_row, json_step, new_spec, options,
    request, spec_of, step, wire, Rig, Seed, EVERY_ROW, HS, P, Q,
};

/// Entries 4, 7 and 31 (§6, C1, C7 R3 NV-4): over a fixture with one step of each row, a dry
/// run calls nothing but its reads (Herdr's snapshot, `health`, `shown_session`): no probe, no
/// launch or relaunch, no write. Its plan has one step per spec pane in grid order, lists the
/// pane of another profile as refused, the differing pane with its diff, and names the action
/// of each other step without claiming its effect.
#[test]
fn a_dry_run_calls_only_its_reads_and_plans_every_row() {
    let rig = every_row();
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(true, false, true), &options());
    let plan = answer.unwrap();
    assert!(plan.dry_run);
    let panes: Vec<&str> = plan.steps.iter().map(|s| s.pane.as_str()).collect();
    assert_eq!(panes, EVERY_ROW);
    assert_only_observed(&calls);
    assert_eq!(rig.writes_since(before), vec![], "nothing written");
    assert_eq!(rig.herdr_panes().len(), 5, "no pane made");

    let a = step(&plan, "pane-a");
    assert_eq!(a.outcome, StepOutcome::Refused, "entry 7");
    assert_eq!(a.code.as_deref(), Some("pane-in-other-profile"));
    let d = step(&plan, "pane-d");
    assert_eq!(
        (d.action, d.outcome),
        (Action::Report, StepOutcome::Reported)
    );
    assert!(d.diff.is_some());
    let e = step(&plan, "pane-e");
    assert_eq!(e.outcome, StepOutcome::Refused);
    assert_eq!(e.code.as_deref(), Some("orchestrator-not-relaunched"));
    for (pane, action) in [
        ("pane-b", Action::Create),
        ("pane-c", Action::Adopt),
        ("pane-f", Action::Relaunch),
    ] {
        let s = step(&plan, pane);
        assert_eq!(
            (s.action, s.outcome),
            (action, StepOutcome::Planned),
            "{pane}: a dry run did nothing (C22): {s:?}"
        );
    }
}

/// Entry 4 (C6, E61, E62): the verb prints that plan as §7's object with `"dry_run": true`,
/// one step per spec pane in grid order, in an envelope the test kit's checker passes, and
/// exits 0 in both formats.
#[test]
fn a_dry_run_prints_its_plan_as_the_steps_object() {
    let run = every_row().run(&apply_argv(&["--dry-run", HS[0], HS[1]]), Format::Json);
    let data = data_of(&run);
    assert_eq!(data["profile"], json!(P));
    assert_eq!(data["dry_run"], json!(true));
    let listed: Vec<&Value> = data["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| &s["pane"])
        .collect();
    assert_eq!(
        listed,
        EVERY_ROW.map(|p| json!(p)).iter().collect::<Vec<_>>()
    );
    assert_eq!(json_step(&data, "pane-a")["outcome"], json!("refused"));
    assert_eq!(
        json_step(&data, "pane-a")["code"],
        json!("pane-in-other-profile")
    );
    assert_eq!(
        json_step(&data, "pane-b")["action"],
        json!(wire(Action::Create))
    );
    let text = every_row().run(&apply_argv(&["--dry-run", HS[0], HS[1]]), Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    assert!(text.out.contains("pane-a"), "{text:?}");
}

/// Entry 7 with `--take-over` (Acceptance 2): the dry-run plan lists the pane of another
/// profile as the adopt the live run would make, and writes nothing.
#[test]
fn a_dry_run_with_take_over_plans_the_adopt_and_writes_nothing() {
    let rig = Rig::new();
    let a = rig.seed_live(&Seed::at("pane-a", cell(1, 1), 48100).in_profile(Some(Q)));
    rig.set_specs(P, vec![spec_of(&a)]);
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(true, true, false), &options());
    let plan = answer.unwrap();
    assert_eq!(step(&plan, "pane-a").action, Action::Adopt);
    assert_only_observed(&calls);
    assert_eq!(rig.writes_since(before), vec![]);
    assert_eq!(rig.record("pane-a"), Some(a), "still in Q");
}

/// Entry 11 (Acceptance 4): `apply --dry-run` and `holler profile show` agree on one fixture.
/// The plan lists exactly the differences `show` reports, with the same `PaneDiff` (positions
/// in rowcol form): a matching pane is an adopt with no diff, a differing one carries `show`'s
/// row, a missing one is a create with no diff. `pane-g` runs another agent than its spec names:
/// both verbs call it matching while `SpecField::ALL` omits `opencode_agent`, and both report the
/// difference once #644 part 2 adds it, so the case holds either way and covers the agent case
/// by composition with `show`'s own tests (C18; plan-review round 4 warn 2).
#[test]
fn a_dry_run_and_profile_show_agree() {
    let seed = || {
        let rig = Rig::new();
        let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100));
        let d = rig.seed_live(&Seed::at("pane-d", cell(1, 2), 48101));
        let g = rig.seed_live(&Seed::at("pane-g", cell(3, 1), 48103));
        let mut moved = spec_of(&d);
        moved.herdr.grid = cell(3, 3);
        let mut other_agent = spec_of(&g);
        other_agent.opencode_agent = Some(AgentKey::parse("review").unwrap());
        assert_ne!(other_agent.opencode_agent, g.opencode_agent);
        rig.set_specs(
            P,
            vec![
                spec_of(&c),
                moved,
                new_spec("pane-b", cell(2, 1), 48102),
                other_agent,
            ],
        );
        rig
    };
    let shown = data_of(&seed().run(&["profile", "show", P], Format::Json));
    let planned = data_of(&seed().run(&apply_argv(&["--dry-run", HS[0], HS[1]]), Format::Json));
    let rows = shown["comparison"].as_array().expect("show's comparison");
    let mut checked = 0;
    for row in rows.iter().filter(|r| r["status"] != "extra") {
        let pane = row["pane"].as_str().unwrap();
        let planned_step = json_step(&planned, pane);
        match row["status"].as_str().unwrap() {
            "differs" => {
                let want = json!({
                    "pane": row["pane"],
                    "status": row["status"],
                    "differences": row["differences"],
                });
                assert_eq!(planned_step["diff"], want, "{pane}");
            }
            "matches" => {
                assert_eq!(planned_step["diff"], Value::Null, "{pane}");
                assert_eq!(planned_step["action"], json!(wire(Action::Adopt)), "{pane}");
            }
            "missing" => {
                assert_eq!(planned_step["diff"], Value::Null, "{pane}");
                assert_eq!(
                    planned_step["action"],
                    json!(wire(Action::Create)),
                    "{pane}"
                );
            }
            other => panic!("{pane}: {other}"),
        }
        checked += 1;
    }
    assert_eq!(checked, 4, "every spec pane: {shown}");
    assert_eq!(
        planned["steps"].as_array().unwrap().len(),
        4,
        "nothing beyond show's rows"
    );
    let d = json_step(&planned, "pane-d");
    assert_eq!(
        d["diff"]["differences"][0]["spec"],
        json!({"row": 3, "col": 3, "pos": "r3c3"})
    );
}
