//! Test B of the #667 brief (the issue's step 6), on a scratch set of its own: a failing
//! health check refuses apply's create and its relaunch before anything live changes, the
//! other panes proceed, and the same check aimed at a live server lets both through. The bad
//! and the good check are one `curl` command that differs only in its port (decision 9): the
//! bad port is one `refused` proves nothing listens on, the good one `W`'s live server.

use holler_pane::tx_apply::{Action, Outcome};
use holler_pane::GridPos;
use serde_json::json;

use crate::plan::{assert_on_binary_path, assert_refused_run, Applied, Still};
use crate::scratch::rig::{free_port, no_model_can_be_reached, refused};
use crate::scratch::Scratch;

const Z: &str = "hlr649-pz";
const W: &str = "hlr649-pw";
const Y: &str = "hlr649-py";
const PROFILE: &str = "hlr649-probe";

/// The scratch set, the three panes' ports, and the port nothing listens on.
struct Run {
    scratch: Scratch,
    ports: [(&'static str, u16); 3],
    bad: u16,
}

impl Run {
    fn port(&self, pane: &str) -> u16 {
        self.ports
            .iter()
            .find(|(p, _)| *p == pane)
            .map(|(_, port)| *port)
            .unwrap()
    }

    /// `profile apply hlr649-probe --herdr-session <scratch>` and `extra`. `Y` has no record
    /// until (iii), so every run needs the session.
    fn argv<'a>(&'a self, extra: &[&'a str]) -> Vec<&'a str> {
        let mut argv = vec![
            "profile",
            "apply",
            PROFILE,
            "--herdr-session",
            &self.scratch.settings.session,
        ];
        argv.extend_from_slice(extra);
        argv
    }

    /// The real run, exit 0, read strictly.
    fn apply(&self) -> Applied {
        Applied::of(&self.scratch.json(&self.argv(&[]), 0))
    }

    /// The dry run's plan (exit 0, though it plans a refusal: it runs no probe).
    fn plan(&self) -> Applied {
        Applied::of(&self.scratch.json(&self.argv(&["--dry-run"]), 0))
    }

    /// The real run that refuses `refused` and runs `ran` other steps: exit 3 and only the
    /// refusal (#853). A failed probe acts nothing, so nothing follows the refused list.
    fn apply_refused(&self, refused: &[(&str, &str)], ran: usize) {
        let out = self.scratch.json(&self.argv(&[]), 3);
        let rest = assert_refused_run(&out, refused, ran);
        assert_eq!(rest, "", "a refusal before the act carries no detail: {out:?}");
    }

    fn still(&self) -> Still {
        Still::of(&self.scratch, &self.ports, PROFILE)
    }

    /// The health check: `curl` of OpenCode's health route on `port`, expecting `healthy`.
    fn check(port: u16) -> String {
        let url = format!("http://127.0.0.1:{port}/global/health");
        json!(["curl", "-s", "--max-time", "3", url]).to_string()
    }

    /// Spec-only: `Y`'s spec (a new pane at `hlr649h` r2c1) with the check on `port`.
    fn spec_y(&self, port: u16) {
        let check = Self::check(port);
        let extra = [
            "--profile",
            PROFILE,
            "--spec-only",
            "--check-json",
            &check,
            "--expect",
            "healthy",
        ];
        let argv = self
            .scratch
            .launch_argv(Y, "hlr649h", "r2c1", self.port(Y), &extra);
        self.scratch
            .json(&argv.iter().map(String::as_str).collect::<Vec<_>>(), 0);
    }

    /// Spec-only: `Z`'s spec with the check on `port`.
    fn spec_z(&self, port: u16) {
        let check = Self::check(port);
        let argv = [
            "pane",
            "relaunch",
            Z,
            "--profile",
            PROFILE,
            "--spec-only",
            "--check-json",
            &check,
            "--expect",
            "healthy",
        ];
        self.scratch.json(&argv, 0);
    }
}

/// Step 6, three phases.
#[test]
#[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#667)"]
fn a_failing_probe_refuses_the_create_and_the_relaunch_and_changes_nothing() {
    let Some(scratch) = Scratch::start() else {
        return;
    };
    assert_on_binary_path(&scratch.settings, "curl");
    let ports = [Z, W, Y].map(|pane| (pane, free_port()));
    let bad = free_port();
    assert!(refused(bad), "nothing listens on the bad check's port");
    let run = Run {
        scratch,
        ports,
        bad,
    };
    run.scratch.launch(Z, "hlr649h", "r1c1", run.port(Z), &[]);
    run.scratch.launch(W, "hlr649g", "r1c1", run.port(W), &[]);
    run.scratch
        .json(&["profile", "create", PROFILE, "--from-current"], 0);
    run.spec_y(run.bad);

    a_failing_check_refuses_the_create(&run);
    a_failing_check_refuses_the_relaunch_before_it_starts(&run);
    the_same_check_on_a_live_server_lets_both_through(&run);

    run.scratch
        .assert_end_state(&[Z, W, Y], &["hlr649g", "hlr649h"]);
    run.scratch.assert_tmux_env_is_scratch();
    run.scratch.assert_no_credential_leaked();
}

/// (i): `Y`'s create is refused `probe-failed`, and the run exits 3 (#853); `Z` and `W`
/// proceed (they were members already, so their adoption writes nothing); nothing exists for
/// `Y`.
fn a_failing_check_refuses_the_create(run: &Run) {
    let before = run.still();
    let plan = run.plan();
    assert_eq!(
        plan.panes(),
        [W, Z, Y],
        "apply's order: by workspace, then column 1 by row"
    );
    for pane in [Z, W] {
        assert_eq!(
            plan.step(pane).row(),
            (Action::Adopt, Outcome::Planned),
            "{pane}"
        );
    }
    assert_eq!(plan.step(Y).row(), (Action::Create, Outcome::Planned));
    run.apply_refused(&[(Y, "probe-failed")], 2);

    let get = run.scratch.json(&["pane", "get", Y], 3);
    assert_eq!(get.error_code().as_deref(), Some("pane-not-found"));
    assert_eq!(
        run.still(),
        before,
        "no record, no split, no process, no write"
    );
    assert!(
        !run.scratch.tmux.sessions().iter().any(|s| s == Y),
        "no tmux session for Y"
    );
    assert!(refused(run.port(Y)), "no server on Y's port");
}

/// (ii): `Z`'s server killed and its spec given the failing check: its relaunch is refused
/// `probe-failed` before anything started, and `Y` is refused again; `W` runs (exit 3, #853).
fn a_failing_check_refuses_the_relaunch_before_it_starts(run: &Run) {
    run.scratch.kill_server(Z, run.port(Z));
    run.spec_z(run.bad);
    let before = run.still();
    assert_eq!(
        run.plan().step(Z).row(),
        (Action::Relaunch, Outcome::Planned)
    );
    run.apply_refused(&[(Z, "probe-failed"), (Y, "probe-failed")], 1);

    let after = run.still();
    assert!(
        after.pane(Z).refusing,
        "Z's port still refuses: the relaunch started nothing"
    );
    assert!(after.pane(Z).owned.is_empty(), "Z owns no process");
    assert_eq!(
        after, before,
        "Z's record and every other reading unchanged"
    );
}

/// (iii): both checks aimed at `W`'s live server: `Z` is relaunched and `Y` created at its
/// cell, each with its probe recorded `ok`.
fn the_same_check_on_a_live_server_lets_both_through(run: &Run) {
    let good = run.port(W);
    run.spec_z(good);
    run.spec_y(good);
    let before = run.still();
    let applied = run.apply();
    no_model_can_be_reached(run.port(Z));
    no_model_can_be_reached(run.port(Y));
    assert_eq!(
        applied.step(Z).row(),
        (Action::Relaunch, Outcome::Relaunched)
    );
    assert_eq!(applied.step(Y).row(), (Action::Create, Outcome::Created));
    assert_eq!(applied.step(W).row(), (Action::Adopt, Outcome::Adopted));

    let after = run.still();
    assert!(
        !after.pane(Z).refusing && !after.pane(Z).owned.is_empty(),
        "Z runs again"
    );
    for pane in [Z, Y] {
        let record = run.scratch.record(pane);
        assert_eq!(record["probe"]["last"], "ok", "{pane}: {record}");
    }
    let y = run.scratch.record(Y);
    let y_id = y["herdr"]["pane_id"].as_str().unwrap();
    after.assert_one_herdr_pane_added(&before, y_id, "hlr649h", GridPos { row: 2, col: 1 });
}
