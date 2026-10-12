//! The order of apply's steps (#853; ADR-0021 section 8, "Apply's step order and its exit
//! code", decision 32): by workspace, then the workspace's column-1 cells by row, then every
//! other cell by row and column, the order the adapter's split planner builds a grid in. The
//! whole 2-by-2 grid built on the split-only fake is `rows.rs`'s row b case.
//!
//! - The order itself, through a dry run (which calls no `ensure_pane`, so specs may name any
//!   workspace): Acceptance 1.
//! - The limit that stays: a row cannot be added below a row of two live panes, on the
//!   split-only fake; that step is refused and the run exits 3: Acceptance 4.

use holler_pane::tx_apply::Outcome as StepOutcome;
use holler_pane::ProfileSpec;
use holler_pane_testkit::herdr::{Placement, GRID_UNREACHABLE};

use super::refused::{assert_carries, assert_refused, Refusal};
use super::rig::{
    apply_argv, cell, new_spec, options, outcomes, request, spec_of, step, Rig, Seed, HS, P,
    WORKSPACE,
};

/// A spec for a pane with no record: `pane` at `r<row>c<col>` of `workspace`, on `port`.
fn spec_in(pane: &str, workspace: &str, (row, col): (u16, u16), port: u16) -> ProfileSpec {
    let mut spec = new_spec(pane, cell(row, col), port);
    spec.herdr.workspace = workspace.to_owned();
    spec
}

/// Acceptance 1 (decision 32): whatever P's order, the steps take each workspace in turn
/// (`main`, then `side`), its column-1 cells by row first, then its other cells by row and
/// column; two specs of one cell keep P's order (`dup-b` is listed before `dup-a`).
#[test]
fn steps_take_each_workspaces_column_1_by_row_then_the_rest_by_row_and_column() {
    let rig = Rig::new();
    let side = "side";
    assert!(WORKSPACE < side, "the workspaces' own order");
    let specs = [
        ("side-r1c2", side, (1, 2)),
        ("main-r2c2", WORKSPACE, (2, 2)),
        ("main-r1c3", WORKSPACE, (1, 3)),
        ("side-r2c1", side, (2, 1)),
        ("dup-b", WORKSPACE, (3, 1)),
        ("main-r2c1", WORKSPACE, (2, 1)),
        ("main-r1c2", WORKSPACE, (1, 2)),
        ("side-r1c1", side, (1, 1)),
        ("dup-a", WORKSPACE, (3, 1)),
        ("main-r1c1", WORKSPACE, (1, 1)),
    ];
    let specs = (48100..)
        .zip(specs)
        .map(|(port, (pane, workspace, at))| spec_in(pane, workspace, at, port))
        .collect();
    rig.set_specs(P, specs);
    let (answer, _) = rig.engine(&request(true, false, true), &options());
    let plan = answer.unwrap();
    let panes: Vec<&str> = plan.steps.iter().map(|s| s.pane.as_str()).collect();
    assert_eq!(
        panes,
        [
            "main-r1c1",
            "main-r2c1",
            "dup-b",
            "dup-a",
            "main-r1c2",
            "main-r1c3",
            "main-r2c2",
            "side-r1c1",
            "side-r2c1",
            "side-r1c2",
        ]
    );
    assert!(
        plan.steps.iter().all(|s| s.outcome == StepOutcome::Planned),
        "every step a planned create: {plan:?}"
    );
}

/// A live `pane-p` (r1c1) and `pane-q` (r1c2) of P on a split-only Herdr, and P's spec of
/// `pane-n` at r2c1 with no record, listed first.
fn below_a_row_of_two() -> Rig {
    let rig = Rig::new();
    let p = rig.seed_live(&Seed::at("pane-p", cell(1, 1), 48100));
    let q = rig.seed_live(&Seed::at("pane-q", cell(1, 2), 48101));
    rig.herdr.set_placement(Placement::SplitOnly);
    rig.set_specs(
        P,
        vec![
            new_spec("pane-n", cell(2, 1), 48102),
            spec_of(&q),
            spec_of(&p),
        ],
    );
    rig
}

/// Acceptance 4 (decision 32, the limit that stays): Herdr adds a row only below a row of one
/// pane, so `pane-n`'s create at r2c1 beside a live r1c1 and r1c2 is a `refused` step
/// (`grid-unreachable`), nothing is made for it and nothing live moves; the other steps still
/// run. The run exits 3 coded `grid-unreachable`, and the create's refusal came from inside
/// its act (`ensure_pane`), so the one line carries its detail with the reconcile step.
#[test]
fn a_row_below_a_row_of_two_live_panes_is_refused_and_the_run_exits_3() {
    let rig = below_a_row_of_two();
    let herdr_before = rig.herdr_panes();
    let (answer, _) = rig.engine(&request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-p", StepOutcome::Adopted),
            ("pane-n", StepOutcome::Refused),
            ("pane-q", StepOutcome::Adopted),
        ]
    );
    let n = step(&applied, "pane-n");
    assert_eq!(n.code.as_deref(), Some(GRID_UNREACHABLE.as_str()), "{n:?}");
    assert_eq!(rig.record("pane-n"), None, "nothing recorded for pane-n");
    assert_eq!(rig.herdr_panes(), herdr_before, "no pane made or moved");

    let unreachable = GRID_UNREACHABLE;
    let refusal = Refusal {
        refused: &[("pane-n", unreachable.as_str())],
        ran: 2,
    };
    let rest = assert_refused(
        below_a_row_of_two,
        |rig, f| rig.run(&apply_argv(&HS), f),
        &refusal,
    );
    let detail = n.detail.clone().expect("a refused step has a detail");
    assert_carries(&rest, "pane-n", &detail);
}
