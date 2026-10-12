//! A `failed` step stops the run (brief §5, C1-C3, C5, C9, C19, C22, C25 and the decision on O6;
//! C14 entries 17, 21, 22, 23 and 33).
//!
//! Each case reads the `failed` step and the `skipped` steps after it from the result the engine
//! returns, and runs the verb in both formats to see that it exits 1 and prints only the
//! failure (`assert_stopped`). A case that fails one pane's `health` with `fail_next` holds one
//! recorded spec pane only, since apply observes the panes in parallel (C7).

use holler_cli::pane::profile_scope::reconcile_step;
use holler_pane::tx_apply::{Action, Outcome as StepOutcome};
use holler_pane::PaneError;
use holler_pane_testkit::harness::HarnessOp;
use holler_pane_testkit::herdr::HerdrOp;

use super::refused::{assert_refused, Refusal};
use super::rig::{
    apply_argv, assert_only_observed, assert_stopped, cell, new_spec, options, outcomes,
    profile_name, request, spec_of, step, Rig, Seed, Stop, HS, P, Q,
};
use crate::list::rig::{NthCasPut, NthPut};

fn unavailable(what: &str) -> PaneError {
    PaneError::Unavailable {
        what: what.to_owned(),
    }
}

/// `pane-x` of P (r1c1, recorded, healthy, matching: the only pane whose health is observed),
/// then `pane-y` (r2c1, no record), with the next `health` call answering `unavailable`.
fn health_fails() -> Rig {
    let rig = Rig::new();
    let x = rig.seed_live(&Seed::at("pane-x", cell(1, 1), 48100));
    rig.set_specs(P, vec![spec_of(&x), new_spec("pane-y", cell(2, 1), 48101)]);
    rig.harness
        .faults()
        .fail_next(HarnessOp::Health, unavailable("harness"));
    rig
}

/// Entry 17 (§2, §5, §11): a `health` call that fails is a `failed` step that stops the run:
/// the later `create` is `skipped` and never started, nothing is written, and the verb exits 1
/// printing only the failure.
#[test]
fn a_failed_health_call_stops_the_run() {
    let rig = health_fails();
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-x", StepOutcome::Failed),
            ("pane-y", StepOutcome::Skipped)
        ]
    );
    assert_eq!(step(&applied, "pane-x").error, Some(unavailable("harness")));
    assert_eq!(
        step(&applied, "pane-x").code.as_deref(),
        Some("unavailable")
    );
    assert_eq!(step(&applied, "pane-x").action, Action::Observe, "C22");
    assert_only_observed(&calls);
    assert_eq!(rig.writes_since(before), vec![]);
    assert_eq!(rig.record("pane-y"), None, "the create never ran");

    let stop = Stop {
        pane: "pane-x",
        code: "unavailable",
        action: Some(Action::Observe),
        skipped: 1,
    };
    assert_stopped(health_fails, |rig, f| rig.run(&apply_argv(&HS), f), &stop);
}

/// Entry 21 (C2, the decision on O6): a dry run treats a failed `health` call as a live run
/// does: its result lists the `failed` step and the `skipped` one; it runs no probe and writes
/// nothing; and the verb exits 1 and prints only the failure, no plan.
#[test]
fn a_dry_run_with_a_failed_health_call_prints_only_the_failure() {
    let rig = health_fails();
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(true, false, true), &options());
    let applied = answer.unwrap();
    assert!(applied.dry_run);
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-x", StepOutcome::Failed),
            ("pane-y", StepOutcome::Skipped)
        ]
    );
    assert_only_observed(&calls);
    assert_eq!(rig.writes_since(before), vec![]);

    let stop = Stop {
        pane: "pane-x",
        code: "unavailable",
        action: Some(Action::Observe),
        skipped: 1,
    };
    let argv = apply_argv(&["--dry-run", HS[0], HS[1]]);
    assert_stopped(health_fails, |rig, f| rig.run(&argv, f), &stop);
}

/// `pane-x`, recorded in no profile, healthy and matching (an adopt that joins P), then
/// `pane-y` of P, healthy and matching.
fn join_then_adopt() -> Rig {
    let rig = Rig::new();
    let x = rig.seed_live(&Seed::at("pane-x", cell(1, 1), 48100).in_profile(None));
    let y = rig.seed_live(&Seed::at("pane-y", cell(2, 1), 48101));
    rig.set_specs(P, vec![spec_of(&x), spec_of(&y)]);
    rig
}

/// Entry 22 (C5, C7 NV-9): a store conflict at a step's write (the join of an adopt, answered
/// `generation-conflict` by the rig's `NthCasPut`) is a `failed` step that stops the run; the
/// later step is `skipped`; nothing live is touched; the verb exits 1.
#[test]
fn a_store_conflict_at_a_steps_write_stops_the_run() {
    let plan = vec![(1, NthPut::Fail(PaneError::Conflict))];
    let rig = join_then_adopt();
    let seam = NthCasPut::new(&rig.panes, plan.clone());
    let before = rig.cursor();
    let (answer, calls) = rig.engine_over(&seam, &request(false, false, false), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-x", StepOutcome::Failed),
            ("pane-y", StepOutcome::Skipped)
        ]
    );
    assert_eq!(
        step(&applied, "pane-x").code.as_deref(),
        Some("generation-conflict")
    );
    assert_eq!(step(&applied, "pane-x").action, Action::Adopt);
    // The seam's failed write never reaches the fake, so its log holds no `cas_put`.
    assert_only_observed(&calls);
    assert_eq!(
        rig.writes_since(before),
        vec![],
        "the failed write never landed"
    );

    let stop = Stop {
        pane: "pane-x",
        code: "generation-conflict",
        action: Some(Action::Adopt),
        skipped: 1,
    };
    assert_stopped(
        join_then_adopt,
        |rig, format| {
            let seam = NthCasPut::new(&rig.panes, plan.clone());
            rig.run_over(&seam, &apply_argv(&[]), format)
        },
        &stop,
    );
}

/// `pane-z` (r1c1, no record), then `pane-x` and `pane-y` of P (r2c1 and r3c1, recorded,
/// healthy, matching), with the next Herdr snapshot answering `unavailable`.
fn snapshot_fails() -> Rig {
    let rig = Rig::new();
    let x = rig.seed_live(&Seed::at("pane-x", cell(2, 1), 48101));
    let y = rig.seed_live(&Seed::at("pane-y", cell(3, 1), 48102));
    rig.set_specs(
        P,
        vec![
            new_spec("pane-z", cell(1, 1), 48100),
            spec_of(&x),
            spec_of(&y),
        ],
    );
    rig.herdr
        .faults()
        .fail_next(HerdrOp::Snapshot, unavailable("herdr"));
    rig
}

/// Entry 23 (C3, C1): a Herdr snapshot that cannot be read fails the first step whose row needs
/// it (`pane-x`, recorded); the step before it, a `create`, which needs no snapshot, still runs;
/// the step after it is `skipped`; the verb exits 1.
#[test]
fn an_unreadable_snapshot_fails_the_first_step_that_needs_it() {
    let rig = snapshot_fails();
    let (answer, _) = rig.engine(&request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-z", StepOutcome::Created),
            ("pane-x", StepOutcome::Failed),
            ("pane-y", StepOutcome::Skipped),
        ]
    );
    assert_eq!(
        step(&applied, "pane-x").code.as_deref(),
        Some("unavailable")
    );
    assert_eq!(step(&applied, "pane-x").action, Action::Observe, "C22");

    let stop = Stop {
        pane: "pane-x",
        code: "unavailable",
        action: Some(Action::Observe),
        skipped: 1,
    };
    assert_stopped(snapshot_fails, |rig, f| rig.run(&apply_argv(&HS), f), &stop);
}

/// `pane-a` of Q (r1c1, the only recorded spec pane), then `pane-y` (r2c1, no record), with the
/// next `health` call (pane-a's) answering `unavailable`.
fn row_a_health_fails() -> Rig {
    let rig = Rig::new();
    let a = rig.seed_live(&Seed::at("pane-a", cell(1, 1), 48100).in_profile(Some(Q)));
    rig.set_specs(P, vec![spec_of(&a), new_spec("pane-y", cell(2, 1), 48101)]);
    rig.harness
        .faults()
        .fail_next(HarnessOp::Health, unavailable("harness"));
    rig
}

/// Entry 33 (C1, C7 R5 B-3): a failed `health` call for a pane that row a refuses changes
/// nothing: its step is `refused` (`pane-in-other-profile`), not `failed`; the next step runs and
/// is `created`; its record is never written; and the verb exits 3 for the refusal (#853), not
/// 1 for a failure.
#[test]
fn a_failed_health_for_a_pane_row_a_refuses_is_not_a_failure() {
    let rig = row_a_health_fails();
    let before = rig.cursor();
    let (answer, _) = rig.engine(&request(false, false, true), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-a", StepOutcome::Refused),
            ("pane-y", StepOutcome::Created)
        ]
    );
    assert_eq!(
        step(&applied, "pane-a").code.as_deref(),
        Some("pane-in-other-profile")
    );
    let touched: Vec<_> = rig
        .writes_since(before)
        .into_iter()
        .filter(|e| e.name.as_str() == "pane-a")
        .collect();
    assert_eq!(touched, vec![], "pane-a is never written");

    let refusal = Refusal {
        refused: &[("pane-a", "pane-in-other-profile")],
        ran: 1,
    };
    let rest = assert_refused(
        row_a_health_fails,
        |rig, f| rig.run(&apply_argv(&HS), f),
        &refusal,
    );
    assert_eq!(rest, "", "row a acted nothing");
}

/// `pane-y` and `pane-z` (r1c1 and r2c1, no record), with the next `create_session`, which
/// `pane-y`'s launch calls once its pane, session and server are up, answering `unavailable`.
fn create_fails_after_its_act() -> Rig {
    let rig = Rig::new();
    rig.set_specs(
        P,
        vec![
            new_spec("pane-y", cell(1, 1), 48100),
            new_spec("pane-z", cell(2, 1), 48101),
        ],
    );
    rig.harness
        .faults()
        .fail_next(HarnessOp::CreateSession, unavailable("harness"));
    rig
}

/// C25 (ADR-0021 section 8, step 6): a `failed` step whose launch had started its act prints the
/// reconcile step in the failure's message, in both formats, as `pane launch` does.
#[test]
fn a_failed_step_that_had_acted_names_the_reconcile_step() {
    let stop = Stop {
        pane: "pane-y",
        code: "unavailable",
        action: Some(Action::Create),
        skipped: 1,
    };
    let message = assert_stopped(
        create_fails_after_its_act,
        |rig, f| rig.run(&apply_argv(&HS), f),
        &stop,
    );
    let reconcile = reconcile_step(Some(&profile_name(P)));
    assert!(
        message.contains(&reconcile),
        "names {reconcile:?}: {message}"
    );
}
