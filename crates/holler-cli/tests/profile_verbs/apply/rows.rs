//! One case per row of the step table (§3) and per per-pane answer of §4, on the rig's fakes
//! (brief C14 entries 1-3, 5, 8-10, 12, 13, 16, 30 and 32, C22's vocabulary, and C25's refusal
//! after the act began). `--take-over` is in `takeover.rs`.

use std::sync::atomic::{AtomicBool, Ordering};

use holler_cli::output::Format;
use holler_cli::pane::profile_scope::reconcile_step;
use holler_pane::profile_diff::{diff_spec, PaneDiff, PaneStatus};
use holler_pane::tx_apply::{Action, Outcome as StepOutcome};
use holler_pane::{Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, ProbeResult, Watch};
use holler_pane_testkit::harness::{HarnessOp, ServerState, AGENT_MISMATCH};
use holler_pane_testkit::herdr::FakeHerdr;
use holler_pane_testkit::herdr::{HerdrOp, Placement};
use holler_pane_testkit::pane_store::FakePaneStore;
use serde_json::{json, Value};

use super::rig::{
    apply_argv, argv, assert_only_observed, cell, data_of, every_row, json_step, new_spec, options,
    outcomes, profile_name, request, spec_of, step, Rig, Seed, EVERY_ROW, HS, P, Q, SESSION,
};

/// §3 in one run: one step per spec pane, in `(workspace, row, col)` order whatever the
/// profile's own order (C6), each answered by its row; `refused` and `reported` do not change
/// the exit code (§5).
#[test]
fn one_run_takes_every_row_in_grid_order() {
    let rig = every_row();
    let (answer, _) = rig.engine(&request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-a", StepOutcome::Refused),
            ("pane-b", StepOutcome::Created),
            ("pane-c", StepOutcome::Adopted),
            ("pane-d", StepOutcome::Reported),
            ("pane-e", StepOutcome::Refused),
            ("pane-f", StepOutcome::Relaunched),
        ]
    );
    assert_eq!(
        step(&applied, "pane-a").code.as_deref(),
        Some("pane-in-other-profile")
    );
    assert_eq!(
        step(&applied, "pane-e").code.as_deref(),
        Some("orchestrator-not-relaunched")
    );
    assert!(!applied.dry_run);

    let rig = every_row();
    let run = rig.run(&apply_argv(&HS), Format::Json);
    let data = data_of(&run);
    let panes: Vec<&str> = data["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["pane"].as_str().unwrap())
        .collect();
    assert_eq!(panes, EVERY_ROW);
    assert_eq!(json_step(&data, "pane-e")["outcome"], json!("refused"));
    assert_eq!(
        json_step(&data, "pane-e")["code"],
        json!("orchestrator-not-relaunched")
    );
    // C22: the action strings, and the step object's keys, an absent one printed as null.
    let actions: Vec<&str> = data["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["action"].as_str().unwrap_or_default())
        .collect();
    let want = ["refuse", "create", "adopt", "report", "refuse", "relaunch"];
    assert_eq!(actions, want, "{data}");
    let c = json_step(&data, "pane-c");
    let mut keys: Vec<&str> = c.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    let want = ["action", "code", "detail", "diff", "outcome", "pane"];
    assert_eq!(keys, want, "{c}");
    for key in ["code", "diff", "detail"] {
        assert_eq!(c[key], Value::Null, "{key}: {c}");
    }
}

/// Entry 2 (row c): a healthy pane that matches its spec is `adopted`, not relaunched. Nothing
/// is called but apply's own observations, and nothing is written (it is already in P).
#[test]
fn row_c_a_healthy_matching_pane_is_adopted_with_no_live_call() {
    let rig = Rig::new();
    let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100));
    rig.set_specs(P, vec![spec_of(&c)]);
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(false, false, false), &options());
    let applied = answer.unwrap();
    assert_eq!(outcomes(&applied), vec![("pane-c", StepOutcome::Adopted)]);
    assert_eq!(step(&applied, "pane-c").action, Action::Adopt);
    assert_only_observed(&calls);
    assert_eq!(rig.writes_since(before), vec![], "no registry write");
    assert_eq!(rig.record("pane-c"), Some(c), "the record is unchanged");
}

/// Entries 3 and 10 (row d): a healthy pane in the wrong cell is `reported` with its
/// `PaneDiff`, embedded unchanged (C6), never moved, killed or written. The JSON step's `diff`
/// is `{"pane", "status", "differences"}` with the cell in rowcol form.
#[test]
fn row_d_a_healthy_pane_that_differs_is_reported_and_never_moved() {
    let seed = || {
        let rig = Rig::new();
        let d = rig.seed_live(&Seed::at("pane-d", cell(2, 1), 48100));
        let mut spec = spec_of(&d);
        spec.herdr.grid = cell(1, 1);
        rig.set_specs(P, vec![spec]);
        (rig, d)
    };
    let (rig, d) = seed();
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(false, false, false), &options());
    let applied = answer.unwrap();
    let reported = step(&applied, "pane-d");
    assert_eq!(reported.outcome, StepOutcome::Reported);
    assert_eq!(reported.action, Action::Report);
    let spec = rig.stored_profile(P).panes[0].clone();
    let want = PaneDiff {
        pane: "pane-d".to_owned(),
        status: PaneStatus::Differs,
        differences: diff_spec(&spec, &d),
    };
    assert_eq!(reported.diff.as_ref(), Some(&want));
    assert_only_observed(&calls);
    assert_eq!(rig.writes_since(before), vec![], "no registry write");
    let listed = rig
        .herdr_panes()
        .into_iter()
        .any(|p| p.pane_id == d.herdr.pane_id && p.grid == cell(2, 1));
    assert!(listed, "the pane is still at r2c1");
    let server = rig.harness.server(48100).unwrap();
    assert_eq!(server.state, ServerState::Running, "never killed");

    let (rig, _) = seed();
    let data = data_of(&rig.run(&apply_argv(&[]), Format::Json));
    assert_eq!(
        json_step(&data, "pane-d")["diff"],
        json!({
            "pane": "pane-d",
            "status": "differs",
            "differences": [{
                "field": "herdr.grid",
                "spec": {"row": 1, "col": 1, "pos": "r1c1"},
                "live": {"row": 2, "col": 1, "pos": "r2c1"},
            }],
        })
    );
}

/// Entry 5 (row a): a pane whose record names another profile is `refused`
/// (`pane-in-other-profile`) without `--take-over`; its record still names Q, and the run
/// exits 0.
#[test]
fn row_a_a_pane_of_another_profile_is_refused_without_take_over() {
    let rig = Rig::new();
    let a = rig.seed_live(&Seed::at("pane-a", cell(1, 1), 48100).in_profile(Some(Q)));
    rig.set_specs(P, vec![spec_of(&a)]);
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(false, false, false), &options());
    let applied = answer.unwrap();
    assert_eq!(outcomes(&applied), vec![("pane-a", StepOutcome::Refused)]);
    assert_eq!(
        step(&applied, "pane-a").code.as_deref(),
        Some("pane-in-other-profile")
    );
    assert_only_observed(&calls);
    assert_eq!(rig.writes_since(before), vec![]);
    assert_eq!(rig.record("pane-a"), Some(a));
    assert_eq!(rig.run(&apply_argv(&[]), Format::Json).code, 0);
}

/// Entry 9 (row b, Acceptance 3): against #638's split-only fake, the missing panes are
/// `created` in grid order, each by a split from a pane made before it; P lists them in the
/// reverse order, which no split reaches.
#[test]
fn row_b_missing_panes_are_created_by_splits_in_grid_order() {
    let rig = Rig::new();
    rig.herdr.set_placement(Placement::SplitOnly);
    rig.set_specs(
        P,
        vec![
            new_spec("pane-r2c1", cell(2, 1), 48102),
            new_spec("pane-r1c2", cell(1, 2), 48101),
            new_spec("pane-r1c1", cell(1, 1), 48100),
        ],
    );
    let (answer, _) = rig.engine(&request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-r1c1", StepOutcome::Created),
            ("pane-r1c2", StepOutcome::Created),
            ("pane-r2c1", StepOutcome::Created),
        ]
    );
    for (pane, grid) in [
        ("pane-r1c1", cell(1, 1)),
        ("pane-r1c2", cell(1, 2)),
        ("pane-r2c1", cell(2, 1)),
    ] {
        let record = rig.record(pane).expect("recorded");
        assert_eq!(record.herdr.grid, grid, "{pane}");
        assert_eq!(record.profile.as_ref().map(|p| p.as_str()), Some(P));
        let listed = rig
            .herdr_panes()
            .into_iter()
            .any(|p| p.pane_id == record.herdr.pane_id && p.grid == grid);
        assert!(listed, "{pane} is at {grid}");
    }
}

/// Entry 12 (row b, §4): a missing pane whose cell holds an unrecorded Herdr pane is refused
/// through launch (`grid-occupied`; the occupant is never adopted, §1), and the run goes on.
#[test]
fn row_b_an_occupied_cell_is_refused_through_launch() {
    let rig = Rig::new();
    let occupant = rig.place_herdr_pane(cell(1, 1));
    rig.set_specs(
        P,
        vec![
            new_spec("pane-b", cell(1, 1), 48100),
            new_spec("pane-z", cell(2, 1), 48101),
        ],
    );
    let (answer, _) = rig.engine(&request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-b", StepOutcome::Refused),
            ("pane-z", StepOutcome::Created)
        ]
    );
    assert_eq!(
        step(&applied, "pane-b").code.as_deref(),
        Some("grid-occupied")
    );
    assert_eq!(rig.record("pane-b"), None, "never adopted");
    assert!(rig
        .herdr_panes()
        .iter()
        .any(|p| p.pane_id == occupant.pane_id));
}

/// Entry 8 (Acceptance 3): a pane whose probe fails is `refused` (`probe-failed`) before
/// anything of it is made, and the others proceed; the run exits 0.
#[test]
fn a_failing_probe_refuses_that_pane_and_the_others_proceed() {
    let seed = || {
        let rig = Rig::new();
        let mut probed = new_spec("pane-b", cell(1, 1), 48100);
        probed.check = Some(argv(&["curl", "http://127.0.0.1:8095/v1/models"]));
        probed.expect = vec!["qwen38".to_owned()];
        rig.prober.script(
            probed.check.clone().unwrap(),
            ProbeResult::Failed {
                missing: vec!["qwen38".to_owned()],
            },
        );
        rig.set_specs(P, vec![probed, new_spec("pane-z", cell(2, 1), 48101)]);
        rig
    };
    let rig = seed();
    let (answer, _) = rig.engine(&request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-b", StepOutcome::Refused),
            ("pane-z", StepOutcome::Created)
        ]
    );
    assert_eq!(
        step(&applied, "pane-b").code.as_deref(),
        Some("probe-failed")
    );
    assert_eq!(rig.prober.calls().len(), 1, "pane-b's probe ran once");
    assert_eq!(rig.record("pane-b"), None);
    assert!(
        rig.herdr_panes().iter().all(|p| p.grid != cell(1, 1)),
        "nothing made"
    );
    assert_eq!(seed().run(&apply_argv(&HS), Format::Json).code, 0);
}

/// A `PaneStore` over the fake that lets `record` appear, as another writer's, right after the
/// first read (`get` or `list`) made once Herdr's snapshot was called: after apply's up-front
/// observation and its re-read before the step (§9), and before launch's own `pane-exists`
/// check (brief C7, B-3).
struct AppearsAfterReread<'a> {
    inner: &'a FakePaneStore,
    herdr: &'a FakeHerdr,
    record: Pane,
    done: AtomicBool,
}

impl AppearsAfterReread<'_> {
    fn after_read(&self) {
        let observed = self.herdr.faults().calls().contains(&HerdrOp::Snapshot);
        if observed && !self.done.swap(true, Ordering::SeqCst) {
            self.inner.concurrent_put(&self.record).unwrap();
        }
    }
}

impl PaneStore for AppearsAfterReread<'_> {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        let answer = self.inner.get(name);
        self.after_read();
        answer
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        let answer = self.inner.list();
        self.after_read();
        answer
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        self.inner.cas_put(pane, expected_generation)
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        self.inner.delete(name, expected_generation)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        self.inner.watch(since)
    }
}

/// Entry 13 (§11 b, C7 B-3): a `create` that launch answers `pane-exists` (the record appeared
/// after apply's re-read) is a `refused` step carrying `pane-exists`, and the run goes on.
#[test]
fn a_create_answered_pane_exists_is_refused_and_the_run_goes_on() {
    let rig = Rig::new();
    let x = rig.seed_live(&Seed::at("pane-x", cell(2, 1), 48101));
    rig.set_specs(
        P,
        vec![
            new_spec("pane-y", cell(1, 1), 48100),
            spec_of(&x),
            new_spec("pane-z", cell(3, 1), 48102),
        ],
    );
    let theirs = holler_pane_testkit::fixture::sample_pane("pane-y").unwrap();
    let seam = AppearsAfterReread {
        inner: &rig.panes,
        herdr: &rig.herdr,
        record: theirs,
        done: AtomicBool::new(false),
    };
    let (answer, _) = rig.engine_over(&seam, &request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-y", StepOutcome::Refused),
            ("pane-x", StepOutcome::Adopted),
            ("pane-z", StepOutcome::Created),
        ]
    );
    assert_eq!(
        step(&applied, "pane-y").code.as_deref(),
        Some("pane-exists")
    );
}

/// Entry 16 (row f): an agent whose server was killed is `relaunched` in place: a new server
/// runs on its port, its record (still in P) names it, and the relaunch's record write is the
/// run's only one.
#[test]
fn row_f_a_killed_agent_is_relaunched() {
    let rig = Rig::new();
    let f = rig.seed_live(&Seed::at("pane-f", cell(1, 1), 48100));
    rig.kill(&f);
    rig.set_specs(P, vec![spec_of(&f)]);
    let before = rig.cursor();
    let (answer, _) = rig.engine(&request(false, false, false), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![("pane-f", StepOutcome::Relaunched)]
    );
    assert_eq!(step(&applied, "pane-f").action, Action::Relaunch);
    let after = rig.record("pane-f").unwrap();
    let server = rig.harness.server(48100).unwrap();
    assert_eq!(server.state, ServerState::Running);
    // #802: the host owns the server, so a relaunch records no pid; the server is a new one.
    assert_eq!(after.harness.pid, None);
    assert_ne!(Some(server.pid), f.harness.pid, "a new server");
    assert_eq!(after.profile.as_ref().map(|p| p.as_str()), Some(P));
    let writes = rig.writes_since(before);
    assert_eq!(writes.len(), 1, "{writes:?}");
    assert_eq!(writes[0].pane.as_deref(), Some(&after));
}

/// Entry 30 (§4, C7 R3 NV-3): an unhealthy agent whose spec moves its directory is answered
/// `usage` by the relaunch rules; that is a `refused` step coded `spec-not-launchable` (C22),
/// not a run failure: the run goes on and exits 0, and nothing of the pane is touched.
#[test]
fn a_per_pane_usage_from_the_relaunch_rules_is_refused_and_the_run_goes_on() {
    let seed = || {
        let rig = Rig::new();
        let f = rig.seed_live(&Seed::at("pane-f", cell(1, 1), 48100));
        rig.kill(&f);
        let z = rig.seed_live(&Seed::at("pane-z", cell(2, 1), 48101));
        let mut moved = spec_of(&f);
        moved.host.cwd = "/srv/elsewhere".to_owned();
        rig.set_specs(P, vec![moved, spec_of(&z)]);
        (rig, f)
    };
    let (rig, f) = seed();
    let before = rig.cursor();
    let (answer, _) = rig.engine(&request(false, false, false), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-f", StepOutcome::Refused),
            ("pane-z", StepOutcome::Adopted)
        ]
    );
    let code = step(&applied, "pane-f").code.clone();
    assert_eq!(code.as_deref(), Some("spec-not-launchable"), "C22");
    assert_eq!(rig.writes_since(before), vec![]);
    assert_eq!(rig.record("pane-f"), Some(f));
    assert_eq!(
        rig.harness.server(48100).unwrap().state,
        ServerState::Killed
    );
    let (rig, _) = seed();
    assert_eq!(rig.run(&apply_argv(&[]), Format::Json).code, 0);
}

/// Entry 1 (Acceptance 1): apply twice. The first run creates, joins, relaunches and adopts;
/// the second adopts every pane and changes nothing: no write and no live call but its own
/// observations.
#[test]
fn apply_twice_changes_nothing_the_second_time() {
    let rig = Rig::new();
    let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100));
    let n = rig.seed_live(&Seed::at("pane-n", cell(1, 2), 48101).in_profile(None));
    let f = rig.seed_live(&Seed::at("pane-f", cell(1, 3), 48102));
    rig.kill(&f);
    let b = new_spec("pane-b", cell(2, 1), 48103);
    rig.set_specs(P, vec![spec_of(&c), spec_of(&n), spec_of(&f), b]);
    let (first, _) = rig.engine(&request(false, false, true), &options());
    assert_eq!(
        outcomes(&first.unwrap()),
        vec![
            ("pane-c", StepOutcome::Adopted),
            ("pane-n", StepOutcome::Adopted),
            ("pane-f", StepOutcome::Relaunched),
            ("pane-b", StepOutcome::Created),
        ]
    );
    let before = rig.cursor();
    let (second, calls) = rig.engine(&request(false, false, true), &options());
    let second = second.unwrap();
    assert!(
        second
            .steps
            .iter()
            .all(|s| s.outcome == StepOutcome::Adopted),
        "{second:?}"
    );
    assert_only_observed(&calls);
    assert_eq!(
        rig.writes_since(before),
        vec![],
        "the second run writes nothing"
    );
}

/// Entry 32 (C1, C7 R5 B-1): `--herdr-session` is needed for a `create` alone. With every spec
/// pane recorded and the flag absent, a dry run and a live run both pass the gate; the live
/// run relaunches the killed agent in its record's Herdr session and exits 0.
#[test]
fn herdr_session_is_needed_for_a_create_alone() {
    let seed = || {
        let rig = Rig::new();
        let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100));
        let f = rig.seed_live(&Seed::at("pane-f", cell(2, 1), 48101));
        rig.kill(&f);
        rig.set_specs(P, vec![spec_of(&c), spec_of(&f)]);
        rig
    };
    for format in [Format::Text, Format::Json] {
        let dry = seed().run(&apply_argv(&["--dry-run"]), format);
        assert_eq!(dry.code, 0, "{format:?}: {dry:?}");
    }
    let rig = seed();
    let run = rig.run(&apply_argv(&[]), Format::Json);
    let data = data_of(&run);
    assert_eq!(json_step(&data, "pane-f")["outcome"], json!("relaunched"));
    let after = rig.record("pane-f").unwrap();
    assert_eq!(
        after.herdr.session, SESSION,
        "the record's own Herdr session"
    );
    let text = seed().run(&apply_argv(&[]), Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
}

/// `pane-a` of Q (r1c1: row a, refused before anything runs), then `pane-x` (r1c2), whose launch
/// act the harness's `verify_serving` answers with a refusal (C25), then `pane-z` of P (r2c1,
/// healthy, matching). `pane-x` has no record (a `create`) or, with `relaunch`, is an agent of P
/// whose server was killed (row f).
fn refused_mid_act(relaunch: bool) -> Rig {
    let rig = Rig::new();
    let a = rig.seed_live(&Seed::at("pane-a", cell(1, 1), 48100).in_profile(Some(Q)));
    let x = if relaunch {
        let x = rig.seed_live(&Seed::at("pane-x", cell(1, 2), 48101));
        rig.kill(&x);
        spec_of(&x)
    } else {
        new_spec("pane-x", cell(1, 2), 48101)
    };
    let z = rig.seed_live(&Seed::at("pane-z", cell(2, 1), 48102));
    rig.set_specs(P, vec![spec_of(&a), x, spec_of(&z)]);
    let refusal = PaneError::Refused {
        code: AGENT_MISMATCH,
        message: "the server on port 48101 does not run the agent the spec asks for".to_owned(),
    };
    rig.harness
        .faults()
        .fail_next(HarnessOp::VerifyServing, refusal);
    rig
}

/// C25 (#802; §4, §5): a refusal that arrives after the launch act began (`verify_serving`, once
/// the pane, its session and the server command have run) is still a `refused` step with its
/// code, and the run goes on and exits 0. But it must not read as if nothing ran: in both
/// formats its `detail` names the reconcile step, as `pane launch` does for an acted failure.
/// Row a's refusal, which ran nothing, does not. For a `create` and for a row f relaunch.
#[test]
fn a_refusal_after_the_act_began_names_the_reconcile_step() {
    let reconcile = reconcile_step(Some(&profile_name(P)));
    for (relaunch, action) in [(false, Action::Create), (true, Action::Relaunch)] {
        let rig = refused_mid_act(relaunch);
        let (answer, _) = rig.engine(&request(false, false, true), &options());
        let applied = answer.unwrap();
        assert_eq!(
            outcomes(&applied),
            vec![
                ("pane-a", StepOutcome::Refused),
                ("pane-x", StepOutcome::Refused),
                ("pane-z", StepOutcome::Adopted),
            ],
            "{action:?}"
        );
        let x = step(&applied, "pane-x");
        assert_eq!(
            (x.action, x.code.as_deref()),
            (action, Some(AGENT_MISMATCH.as_str())),
            "{x:?}"
        );

        let run = refused_mid_act(relaunch).run(&apply_argv(&HS), Format::Json);
        let data = data_of(&run);
        let x = json_step(&data, "pane-x");
        assert_eq!(x["outcome"], json!("refused"), "{action:?}: {x}");
        let detail = x["detail"].as_str().unwrap_or_default();
        assert!(
            detail.contains(&reconcile),
            "{action:?}: the detail names {reconcile:?}: {x}"
        );
        let a = json_step(&data, "pane-a");
        assert!(
            !a["detail"]
                .as_str()
                .unwrap_or_default()
                .contains("to reconcile"),
            "row a ran nothing: {a}"
        );

        let text = refused_mid_act(relaunch).run(&apply_argv(&HS), Format::Text);
        assert_eq!(text.code, 0, "{text:?}");
        let line = text
            .out
            .lines()
            .find(|line| line.starts_with("pane-x:"))
            .unwrap_or_else(|| panic!("a line for pane-x: {text:?}"));
        assert!(line.contains(&reconcile), "{action:?}: {line}");
    }
}
