//! Test A of the #667 brief: the issue's steps 1 to 5 and 7, then the end state, in order, on
//! one scratch set. Each step is a function (A's W-3); the brief's step table is the contract.
//! Every JSON verb goes through `Scratch::json`, which checks the envelope; the two text-mode
//! runs (`profile show` in step 1, the dry run in step 2) go through `Scratch::holler`. After
//! every verb that starts a server, the model guard runs on its port. No step types into a
//! TUI, and no step resets, switches or asserts DRIVEN (decision 11).

use std::path::{Path, PathBuf};

use holler_pane::tx_apply::{Action, Outcome};
use holler_pane::GridPos;
use serde_json::{json, Value};

use crate::plan::{assert_on_binary_path, assert_plan_agrees_with_show, Applied, Still};
use crate::scratch::rig::{free_port, no_model_can_be_reached, refused};
use crate::scratch::Scratch;

const A1: &str = "hlr649-ap1";
const A2: &str = "hlr649-ap2";
const B1: &str = "hlr649-ap3";
const X: &str = "hlr649-ap4";
const P: &str = "hlr649-apply-p";
const Q: &str = "hlr649-apply-q";

/// The scenario's scratch set, its ports (one per pane, fixed in each spec) and the file the
/// marker check of `X`'s spec touches.
struct Run {
    scratch: Scratch,
    ports: [(&'static str, u16); 4],
    marker: PathBuf,
}

impl Run {
    fn port(&self, pane: &str) -> u16 {
        self.ports
            .iter()
            .find(|(p, _)| *p == pane)
            .map(|(_, port)| *port)
            .unwrap()
    }

    fn session(&self) -> &str {
        &self.scratch.settings.session
    }

    /// `profile apply <profile> <flags>`, its exit code 0, read strictly.
    fn apply(&self, profile: &str, flags: &[&str]) -> Applied {
        let mut argv = vec!["profile", "apply", profile];
        argv.extend_from_slice(flags);
        Applied::of(&self.scratch.json(&argv, 0))
    }

    /// `profile apply <profile> --herdr-session <scratch> <flags>`.
    fn apply_in_session(&self, profile: &str, flags: &[&str]) -> Applied {
        let mut argv = vec!["--herdr-session", self.session()];
        argv.extend_from_slice(flags);
        self.apply(profile, &argv)
    }

    /// The four panes and `profile`, read (decision 8).
    fn still(&self, profile: &str) -> Still {
        Still::of(&self.scratch, &self.ports, profile)
    }

    /// `profile show <profile>`'s data.
    fn show(&self, profile: &str) -> Value {
        self.scratch.json(&["profile", "show", profile], 0).data()
    }

    /// A spec-only `pane launch` of `pane` into `profile`, with the flags `Scratch::launch`
    /// sends (`launch_args`).
    fn spec_only(&self, pane: &str, workspace: &str, grid: &str, profile: &str, extra: &[&str]) {
        let mut flags = vec!["--profile", profile, "--spec-only"];
        flags.extend_from_slice(extra);
        let argv = self
            .scratch
            .launch_argv(pane, workspace, grid, self.port(pane), &flags);
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        self.scratch.json(&argv, 0);
    }
}

/// `show`'s comparison row for `pane`.
fn row<'a>(shown: &'a Value, pane: &str) -> &'a Value {
    shown["comparison"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["pane"] == pane))
        .unwrap_or_else(|| panic!("show lists {pane}: {shown}"))
}

/// The statuses of every row of `show`'s comparison.
fn statuses(shown: &Value) -> Vec<String> {
    shown["comparison"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| row["status"].as_str().unwrap().to_owned())
        .collect()
}

/// The marker check of decision 9: `sh -c` touches `marker` and prints `ok`.
fn marker_check(marker: &Path) -> String {
    json!(["sh", "-c", format!("touch {}; echo ok", marker.display())]).to_string()
}

/// Steps 1 to 5 and 7, then the end state.
#[test]
#[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#667)"]
fn the_apply_scenario_runs_in_order_on_scratch_instances() {
    let Some(scratch) = Scratch::start() else {
        return;
    };
    assert_on_binary_path(&scratch.settings, "sh");
    scratch.assert_tmux_env_is_scratch();
    let marker = scratch.settings.base.join("hlr649-probe-marker");
    let ports = [A1, A2, B1, X].map(|pane| (pane, free_port()));
    let run = Run {
        scratch,
        ports,
        marker,
    };
    step_1(&run);
    step_2(&run);
    step_3(&run);
    step_3_again_and_converge(&run);
    step_4(&run);
    step_5(&run);
    step_7(&run);
    end_state(&run);
}

/// Step 1: two launches, P from current, Q a detached copy of P, then `B1` in no profile.
fn step_1(run: &Run) {
    let scratch = &run.scratch;
    scratch.launch(A1, "hlr649", "r1c1", run.port(A1), &[]);
    scratch.launch(A2, "hlr649", "r2c1", run.port(A2), &[]);
    let created = scratch
        .json(&["profile", "create", P, "--from-current"], 0)
        .data();
    let mut members: Vec<&str> = created["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    members.sort_unstable();
    assert_eq!(members, [A1, A2], "{created}");
    let copied = scratch
        .json(&["profile", "create", Q, "--from", P], 0)
        .data();
    assert_eq!(copied["members"], json!([]), "a detached copy: {copied}");
    scratch.launch(B1, "hlr649b", "r1c1", run.port(B1), &[]);
    assert_eq!(scratch.record(A1)["profile"], P);
    assert_eq!(scratch.record(A2)["profile"], P);
    assert_eq!(
        scratch.record(B1)["profile"],
        Value::Null,
        "B1 is in no profile"
    );

    let shown = run.show(P);
    assert_eq!(statuses(&shown), ["matches", "matches"], "{shown}");
    for row in shown["comparison"].as_array().unwrap() {
        assert_eq!(row["differences"], json!([]), "{row}");
    }
    let text = scratch.holler(&["profile", "show", P]);
    assert_eq!(text.code, 0, "{text:?}");
    assert!(!text.stdout.contains("differs"), "{}", text.stdout);

    // Control: Q's live panes are its members, and it has none.
    assert_eq!(statuses(&run.show(Q)), ["missing", "missing"]);
    let text = scratch.holler(&["profile", "show", Q]);
    assert_eq!(
        text.stdout.matches("missing (no live pane)").count(),
        2,
        "{}",
        text.stdout
    );
}

/// Step 2: three spec-only edits of P, then `show` and the dry runs, which change nothing.
fn step_2(run: &Run) {
    let scratch = &run.scratch;
    scratch.json(
        &[
            "pane",
            "relaunch",
            A2,
            "--profile",
            P,
            "--spec-only",
            "--agent",
            "plan",
        ],
        0,
    );
    run.spec_only(B1, "hlr649b", "r1c1", P, &[]);
    let check = marker_check(&run.marker);
    run.spec_only(
        X,
        "hlr649b",
        "r2c1",
        P,
        &["--check-json", &check, "--expect", "ok"],
    );

    let shown = run.show(P);
    let a2 = row(&shown, A2);
    assert_eq!(a2["status"], "differs", "{shown}");
    assert_eq!(
        a2["differences"],
        json!([{"field": "opencode_agent", "spec": "plan", "live": null}]),
        "{shown}"
    );
    assert_eq!(row(&shown, A1)["status"], "matches", "{shown}");
    assert_eq!(row(&shown, X)["status"], "missing", "{shown}");
    assert_eq!(
        row(&shown, B1)["status"],
        "missing",
        "B1 is not a member: {shown}"
    );
    step_2_dry_runs(run, &shown);
}

/// Step 2, continued: the dry runs, without and with the session, and in text.
fn step_2_dry_runs(run: &Run, shown: &Value) {
    let scratch = &run.scratch;
    let before = run.still(P);
    let usage = scratch.json(&["profile", "apply", P, "--dry-run"], 2);
    assert_eq!(
        usage.error_code().as_deref(),
        Some("usage"),
        "a create needs the session"
    );
    let plan = run.apply_in_session(P, &["--dry-run"]);
    assert!(plan.dry_run);
    assert_eq!(plan.profile, P);
    assert_eq!(plan.panes(), [A1, A2, B1, X], "(workspace, row, col) order");
    assert_plan_agrees_with_show(&plan, shown, &[B1]);
    // The stated exception: show says B1 is missing (not a member); apply adopts its record.
    assert_eq!(plan.step(B1).row(), (Action::Adopt, Outcome::Planned));
    assert_eq!(plan.step(B1).diff, Value::Null);

    assert_text_plan(run, &plan);
    assert_eq!(run.still(P), before, "a dry run changes nothing");
    assert!(!run.marker.exists(), "a dry run runs no probe");
}

/// The dry run of step 2 in text: the header, then one `PANE: ACTION, OUTCOME` line per step
/// of `plan` (E48).
fn assert_text_plan(run: &Run, plan: &Applied) {
    let scratch = &run.scratch;
    let session = run.session();
    let text = scratch.holler(&[
        "profile",
        "apply",
        P,
        "--dry-run",
        "--herdr-session",
        session,
    ]);
    assert_eq!(text.code, 0, "{text:?}");
    let lines: Vec<&str> = text.stdout.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some(format!("profile {P}: 4 steps, planned (dry run; nothing changed)").as_str())
    );
    for step in &plan.steps {
        let line = format!(
            "{}: {}, {}",
            step.pane,
            step.action.as_str(),
            step.outcome.as_str()
        );
        assert_eq!(
            lines.iter().filter(|l| l.starts_with(&line)).count(),
            1,
            "{line}: {}",
            text.stdout
        );
    }
}

/// Step 3: apply P. A member is adopted and never killed, a differing pane reported and left
/// alone, a pane in no profile joins with its processes untouched, and the missing pane is
/// created at its cell after its probe ran.
fn step_3(run: &Run) {
    let before = run.still(P);
    let applied = run.apply_in_session(P, &[]);
    no_model_can_be_reached(run.port(X));
    assert!(!applied.dry_run);
    assert_eq!(applied.panes(), [A1, A2, B1, X]);
    assert_eq!(applied.step(A1).row(), (Action::Adopt, Outcome::Adopted));
    assert_eq!(applied.step(A2).row(), (Action::Report, Outcome::Reported));
    assert_eq!(applied.step(A2).diff["status"], "differs");
    assert_eq!(applied.step(B1).row(), (Action::Adopt, Outcome::Adopted));
    assert_eq!(applied.step(X).row(), (Action::Create, Outcome::Created));
    let after = run.still(P);

    assert_eq!(
        after.pane(A1),
        before.pane(A1),
        "A1, a member already: nothing written or killed"
    );
    assert!(!after.pane(A1).owned.is_empty(), "A1 runs processes");
    assert_eq!(
        after.pane(A2),
        before.pane(A2),
        "A2 is reported, its live pane untouched"
    );
    assert_joined_untouched(&before, &after, B1);
    assert_created_after_its_probe(run, &before, &after);
}

/// Step 3's create: `X` has a record in P, new processes (the control for `A1`'s unchanged
/// ones), a probe that ran in the live create (the control for the dry run's), and its Herdr
/// pane at its cell beside the earlier ones (decision 10).
fn assert_created_after_its_probe(run: &Run, before: &Still, after: &Still) {
    assert!(before.pane(X).record.is_none() && before.pane(X).owned.is_empty());
    assert!(
        !after.pane(X).owned.is_empty(),
        "X's processes are new (control for A1's)"
    );
    let x = run.scratch.record(X);
    assert_eq!(x["profile"], P);
    assert_eq!(x["probe"]["last"], "ok", "{x}");
    assert!(run.marker.exists(), "the live create ran X's probe");

    // Herdr: the earlier panes at the same ids and cells, plus X at hlr649b r2c1.
    let x_id = x["herdr"]["pane_id"].as_str().unwrap();
    after.assert_one_herdr_pane_added(before, x_id, "hlr649b", GridPos { row: 2, col: 1 });
}

/// Step 3's join: `pane`, in no profile `before`, is in P `after`, by a write, with its
/// processes, its port and its session of record as they were (E34).
fn assert_joined_untouched(before: &Still, after: &Still, pane: &str) {
    let (was, now) = (before.pane(pane), after.pane(pane));
    assert_eq!(now.record.as_ref().unwrap().1, json!(P), "{pane} joins P");
    assert!(
        after.generation(pane) > before.generation(pane),
        "{pane}: the join is a write"
    );
    assert_eq!(now.owned, was.owned, "{pane}'s processes are the same");
    assert_eq!(
        now.refusing, was.refusing,
        "{pane}'s port answers as before"
    );
    assert_eq!(
        now.record.as_ref().unwrap().2,
        was.record.as_ref().unwrap().2,
        "{pane}'s session of record"
    );
}

/// Step 3, continued: apply again (idempotent), then the operator converges the reported pane.
fn step_3_again_and_converge(run: &Run) {
    let before = run.still(P);
    let again = run.apply_in_session(P, &[]);
    assert!(!again.any(Outcome::Created), "{again:#?}");
    assert_eq!(again.step(A2).row(), (Action::Report, Outcome::Reported));
    for pane in [A1, B1, X] {
        assert_eq!(
            again.step(pane).row(),
            (Action::Adopt, Outcome::Adopted),
            "{pane}"
        );
    }
    assert_eq!(run.still(P), before, "apply is idempotent on real state");

    run.scratch
        .json(&["pane", "relaunch", A2, "--profile", P], 0);
    no_model_can_be_reached(run.port(A2));
    let after = run.still(P);
    assert!(!after.pane(A2).owned.is_empty());
    assert_ne!(
        after.pane(A2).owned,
        before.pane(A2).owned,
        "the relaunch replaced A2's processes"
    );
    assert_eq!(
        after.pane(A2).record.as_ref().unwrap().2,
        before.pane(A2).record.as_ref().unwrap().2,
        "A2's session"
    );
    assert_eq!(run.scratch.record(A2)["opencode_agent"], "plan");
    assert_eq!(
        statuses(&run.show(P)),
        ["matches"; 4],
        "show P matches again"
    );
}

/// Step 4: B1's server killed; apply relaunches it in place, without `--herdr-session`.
fn step_4(run: &Run) {
    let live = run.still(P);
    run.scratch.kill_server(B1, run.port(B1));
    let killed = run.still(P);
    let plan = run.apply(P, &["--dry-run"]);
    let b1 = plan.step(B1);
    assert_eq!(b1.row(), (Action::Relaunch, Outcome::Planned));
    let why = format!("its harness server on port {} did not answer", run.port(B1));
    assert!(
        b1.detail.as_deref().is_some_and(|d| d.contains(&why)),
        "{b1:?}"
    );
    assert_eq!(run.still(P), killed, "a dry run changes nothing");

    let applied = run.apply(P, &[]);
    no_model_can_be_reached(run.port(B1));
    assert_eq!(
        applied.step(B1).row(),
        (Action::Relaunch, Outcome::Relaunched)
    );
    for pane in [A1, A2, X] {
        assert_eq!(
            applied.step(pane).row(),
            (Action::Adopt, Outcome::Adopted),
            "{pane}"
        );
    }
    let after = run.still(P);
    assert!(!after.pane(B1).refusing, "B1's port answers again");
    assert_eq!(
        after.pane(B1).record.as_ref().unwrap().2,
        live.pane(B1).record.as_ref().unwrap().2,
        "B1's session"
    );
    assert!(!after.pane(B1).owned.is_empty());
    assert_ne!(
        after.pane(B1).owned,
        live.pane(B1).owned,
        "B1's processes are new (control)"
    );
    for pane in [A1, A2, X] {
        assert_eq!(
            after.pane(pane).owned,
            live.pane(pane).owned,
            "{pane} was not touched"
        );
    }
    assert_eq!(
        after.generation(A1),
        live.generation(A1),
        "nothing wrote A1"
    );
}

/// Step 5: apply Q, whose panes are P's: refused without `--take-over`, moved with it.
fn step_5(run: &Run) {
    let before = run.still(Q);
    let refused_run = run.apply_in_session(Q, &[]);
    assert_eq!(refused_run.panes(), [A1, A2]);
    for pane in [A1, A2] {
        let step = refused_run.step(pane);
        assert_eq!(step.row(), (Action::Refuse, Outcome::Refused), "{pane}");
        assert_eq!(
            step.code.as_deref(),
            Some("pane-in-other-profile"),
            "{pane}"
        );
    }
    assert_eq!(run.still(Q), before, "a refusal changes no record");

    let taken = run.apply_in_session(Q, &["--take-over"]);
    assert_eq!(taken.step(A1).row(), (Action::Adopt, Outcome::Adopted));
    let a2 = taken.step(A2);
    assert_eq!(a2.row(), (Action::Report, Outcome::Reported));
    assert_eq!(
        a2.diff["differences"][0]["field"], "opencode_agent",
        "{a2:?}"
    );
    let after = run.still(Q);
    for pane in [A1, A2] {
        assert_eq!(
            after.pane(pane).record.as_ref().unwrap().1,
            json!(Q),
            "{pane} moved to Q"
        );
        assert_eq!(
            after.pane(pane).owned,
            before.pane(pane).owned,
            "{pane}: nothing live touched"
        );
    }
    assert_eq!(
        after.generation(A1),
        before.generation(A1) + 2,
        "the leave and the join"
    );
    for pane in [B1, X] {
        assert_eq!(
            after.pane(pane),
            before.pane(pane),
            "{pane} is not Q's and stays in P"
        );
    }
}

/// Step 7: the changed pane rolled back to Q's known-good spec, then Q applied.
fn step_7(run: &Run) {
    let before = run.still(Q);
    run.scratch
        .json(&["pane", "relaunch", A2, "--profile", Q], 0);
    no_model_can_be_reached(run.port(A2));
    assert_eq!(
        run.scratch.record(A2)["opencode_agent"],
        Value::Null,
        "Q's spec has no agent"
    );
    let after = run.still(Q);
    assert!(!after.pane(A2).owned.is_empty());
    assert_ne!(
        after.pane(A2).owned,
        before.pane(A2).owned,
        "A2's processes are new"
    );

    let applied = run.apply(Q, &[]);
    for pane in [A1, A2] {
        assert_eq!(
            applied.step(pane).row(),
            (Action::Adopt, Outcome::Adopted),
            "{pane}"
        );
    }
    assert_eq!(statuses(&run.show(Q)), ["matches", "matches"]);
    for (pane, port) in run.ports {
        assert!(!refused(port), "{pane}'s server answers");
    }
}

/// The end state: the registry equals what the adapters observe, the live counts, and a
/// profile that does not exist.
fn end_state(run: &Run) {
    let scratch = &run.scratch;
    scratch.assert_end_state(&[A1, A2, B1, X], &["hlr649", "hlr649b"]);
    let listed = scratch.json(&["profile", "list"], 0).data();
    let live = |name: &str| {
        listed
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["name"] == name))
            .map(|row| row["live"].clone())
            .unwrap_or_else(|| panic!("profile list has {name}: {listed}"))
    };
    assert_eq!(live(P), 2, "B1 and X");
    assert_eq!(live(Q), 2, "A1 and A2");
    let missing = scratch.json(&["profile", "apply", "hlr649-nonexistent"], 3);
    assert_eq!(missing.error_code().as_deref(), Some("profile-not-found"));
    scratch.assert_tmux_env_is_scratch();
    scratch.assert_no_credential_leaked();
}
