//! `--take-over` and the registry writes of rows c, d, e and f (brief §3, C10, C11, C15, O1, O4;
//! C14 entries 6, 14, 15, 25 and 29).
//!
//! Every write is read from the fake pane store's feed, opened at a cursor taken before the run
//! (C7, R4 W-2): a leave is the record as re-read with `profile` none and its generation one
//! higher, and a join is the record it starts from with `profile` P and its generation one
//! higher, every other field unchanged (C7, R5 NV-6).

use holler_cli::output::Format;
use holler_pane::tx_apply::{Action, Outcome as StepOutcome};
use holler_pane::{Pane, PaneError, PaneEvent};
use holler_pane_testkit::harness::ServerState;
use holler_pane_testkit::herdr::HerdrOp;
use holler_pane_testkit::pane_store::PaneStoreOp;

use super::rig::{
    apply_argv, assert_only_observed, assert_stopped, cell, options, outcomes, profile_name,
    request, spec_of, step, Calls, Rig, Seed, Stop, P, Q,
};
use crate::list::rig::{NthCasPut, NthPut};

/// `record` with `profile` set to `profile` and its generation one higher.
fn written(record: &Pane, profile: Option<&str>) -> Pane {
    Pane {
        profile: profile.map(profile_name),
        generation: record.generation + 1,
        ..record.clone()
    }
}

/// The records of `events`, in order (every event here is a put).
fn records(events: &[PaneEvent]) -> Vec<Pane> {
    events
        .iter()
        .map(|e| *e.pane.clone().expect("a put"))
        .collect()
}

/// How many `cas_put`s reached the fake pane store.
fn cas_puts(calls: &Calls) -> usize {
    calls
        .panes
        .iter()
        .filter(|op| **op == PaneStoreOp::CasPut)
        .count()
}

/// Nothing live but apply's observations: the pane-store writes are the case's to check.
fn assert_nothing_live(calls: &Calls) {
    assert_only_observed(&Calls {
        panes: Vec::new(),
        ..calls.clone()
    });
}

/// Entry 25, first half (§3 row c, C10, C7 R2 NV-3 and R3 NV-2): an adopt writes nothing for a
/// pane already in P, joins a pane in no profile with one write, and under `--take-over` moves
/// a pane of Q with two, the leave and then the join. Nothing live is touched.
#[test]
fn row_c_writes_nothing_joins_once_or_leaves_then_joins() {
    let cases: [(Option<&'static str>, bool, usize); 3] =
        [(Some(P), false, 0), (None, false, 1), (Some(Q), true, 2)];
    for (profile, take_over, writes) in cases {
        let rig = Rig::new();
        let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100).in_profile(profile));
        rig.set_specs(P, vec![spec_of(&c)]);
        let before = rig.cursor();
        let (answer, calls) = rig.engine(&request(false, take_over, false), &options());
        let applied = answer.unwrap();
        assert_eq!(
            outcomes(&applied),
            vec![("pane-c", StepOutcome::Adopted)],
            "{profile:?}"
        );
        assert_eq!(cas_puts(&calls), writes, "{profile:?}");
        assert_nothing_live(&calls);
        let events = records(&rig.writes_since(before));
        let want = match writes {
            0 => vec![],
            1 => vec![written(&c, Some(P))],
            _ => {
                let left = written(&c, None);
                let joined = written(&left, Some(P));
                vec![left, joined]
            }
        };
        assert_eq!(events, want, "{profile:?}");
    }
}

/// Entry 6 (Acceptance 2): with `--take-over`, the pane of Q that row a would refuse is
/// `adopted` and ends in P. (The writes are the case above's.)
#[test]
fn take_over_adopts_a_pane_of_another_profile() {
    let rig = Rig::new();
    let a = rig.seed_live(&Seed::at("pane-a", cell(1, 1), 48100).in_profile(Some(Q)));
    rig.set_specs(P, vec![spec_of(&a)]);
    let run = rig.run(&apply_argv(&["--take-over"]), Format::Json);
    assert_eq!(run.code, 0, "{run:?}");
    let after = rig.record("pane-a").unwrap();
    assert_eq!(after.profile.as_ref().map(|p| p.as_str()), Some(P));
}

/// Entry 14 (§3 row d): under `--take-over` a healthy pane of Q that differs is `reported`,
/// its profile moved (the leave, then the join), and nothing live touched.
#[test]
fn row_d_take_over_moves_the_profile_and_touches_nothing_live() {
    let rig = Rig::new();
    let d = rig.seed_live(&Seed::at("pane-d", cell(1, 1), 48100).in_profile(Some(Q)));
    let mut spec = spec_of(&d);
    spec.model.effort = "high".to_owned();
    rig.set_specs(P, vec![spec]);
    let before = rig.cursor();
    let (answer, calls) = rig.engine(&request(false, true, false), &options());
    let applied = answer.unwrap();
    assert_eq!(outcomes(&applied), vec![("pane-d", StepOutcome::Reported)]);
    assert_eq!(step(&applied, "pane-d").action, Action::Report);
    assert!(step(&applied, "pane-d").diff.is_some());
    assert_nothing_live(&calls);
    let left = written(&d, None);
    let joined = written(&left, Some(P));
    assert_eq!(records(&rig.writes_since(before)), vec![left, joined]);
}

/// Entry 15 (§3 row e, C15): an orchestrator that is not healthy is `refused`
/// (`orchestrator-not-relaunched`) with no registry write and nothing live touched, in P
/// without `--take-over` and in Q with it (no leave and no join).
#[test]
fn row_e_refuses_an_unhealthy_orchestrator_and_writes_nothing() {
    for (profile, take_over) in [(P, false), (Q, true)] {
        let rig = Rig::new();
        let e = rig.seed_live(
            &Seed::at("pane-e", cell(1, 1), 48100)
                .in_profile(Some(profile))
                .orchestrator(),
        );
        rig.kill(&e);
        rig.set_specs(P, vec![spec_of(&e)]);
        let before = rig.cursor();
        let (answer, calls) = rig.engine(&request(false, take_over, false), &options());
        let applied = answer.unwrap();
        assert_eq!(
            outcomes(&applied),
            vec![("pane-e", StepOutcome::Refused)],
            "{profile}"
        );
        assert_eq!(
            step(&applied, "pane-e").code.as_deref(),
            Some("orchestrator-not-relaunched")
        );
        assert_only_observed(&calls);
        assert_eq!(rig.writes_since(before), vec![], "{profile}: no write");
        assert_eq!(rig.record("pane-e"), Some(e), "{profile}: unchanged");
        assert_eq!(
            rig.harness.server(48100).unwrap().state,
            ServerState::Killed
        );
    }
}

/// The agent `pane-f` of Q, its server killed, then `pane-z` of P, healthy and matching: the
/// fixture of row f under `--take-over`.
fn f_of_q_then_z() -> (Rig, Pane) {
    let rig = Rig::new();
    let f = rig.seed_live(&Seed::at("pane-f", cell(1, 1), 48100).in_profile(Some(Q)));
    rig.kill(&f);
    let z = rig.seed_live(&Seed::at("pane-z", cell(2, 1), 48101));
    rig.set_specs(P, vec![spec_of(&f), spec_of(&z)]);
    (rig, f)
}

/// Entry 29, first half (§3 row f, O1, C11, C17): under `--take-over` an agent of Q whose
/// server was killed leaves Q, joins P, and is then relaunched; its record names P. The feed
/// shows the three writes in that order.
#[test]
fn row_f_take_over_leaves_joins_then_relaunches() {
    let (rig, f) = f_of_q_then_z();
    let before = rig.cursor();
    let (answer, _) = rig.engine(&request(false, true, false), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-f", StepOutcome::Relaunched),
            ("pane-z", StepOutcome::Adopted)
        ]
    );
    let left = written(&f, None);
    let joined = written(&left, Some(P));
    let events = records(&rig.writes_since(before));
    assert_eq!(events.len(), 3, "{events:?}");
    assert_eq!(
        events[..2],
        [left, joined.clone()],
        "the leave, then the join"
    );
    let relaunched = &events[2];
    assert_eq!(relaunched.name, f.name);
    assert_eq!(
        relaunched.generation,
        joined.generation + 1,
        "after the join"
    );
    assert_eq!(relaunched.profile.as_ref().map(|p| p.as_str()), Some(P));
    assert_ne!(relaunched.harness.pid, f.harness.pid, "a new server");
    assert_eq!(rig.record("pane-f").as_ref(), Some(relaunched));
}

/// Entry 29, second half (O1): when that relaunch then fails (Herdr's `version`, which only
/// the relaunch calls, answers `unavailable`), the step is `failed`, the pane stays in P with no
/// write after the join, the later step is `skipped`, and the verb prints only the failure.
#[test]
fn row_f_take_over_whose_relaunch_fails_stays_in_p() {
    let unavailable = || PaneError::Unavailable {
        what: "herdr".to_owned(),
    };
    let (rig, f) = f_of_q_then_z();
    rig.herdr
        .faults()
        .fail_next(HerdrOp::Version, unavailable());
    let before = rig.cursor();
    let (answer, _) = rig.engine(&request(false, true, false), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-f", StepOutcome::Failed),
            ("pane-z", StepOutcome::Skipped)
        ]
    );
    assert_eq!(step(&applied, "pane-f").error, Some(unavailable()));
    let left = written(&f, None);
    let joined = written(&left, Some(P));
    assert_eq!(
        records(&rig.writes_since(before)),
        vec![left, joined.clone()],
        "no write after the join"
    );
    assert_eq!(rig.record("pane-f"), Some(joined), "the pane stays in P");

    let seed = || {
        let (rig, _) = f_of_q_then_z();
        rig.herdr
            .faults()
            .fail_next(HerdrOp::Version, unavailable());
        rig
    };
    let stop = Stop {
        pane: "pane-f",
        code: "unavailable",
        action: Some(Action::Relaunch),
        skipped: 1,
    };
    assert_stopped(
        seed,
        |rig, format| rig.run(&apply_argv(&["--take-over"]), format),
        &stop,
    );
}

/// Entry 25, second half (C10, C7 R2 W-1, O4): under `--take-over`, an adopt whose join is
/// answered `generation-conflict` after its leave landed is a `failed` step: the pane is left in
/// no profile (no compensating re-join), nothing live is touched, the later step is `skipped`,
/// and the verb's message says the pane left Q and is in no profile.
#[test]
fn a_take_over_join_that_conflicts_leaves_the_pane_in_no_profile() {
    let seed = || {
        let rig = Rig::new();
        let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100).in_profile(Some(Q)));
        let z = rig.seed_live(&Seed::at("pane-z", cell(2, 1), 48101));
        rig.set_specs(P, vec![spec_of(&c), spec_of(&z)]);
        (rig, c)
    };
    let plan = vec![(2, NthPut::Fail(PaneError::Conflict))];
    let (rig, c) = seed();
    let seam = NthCasPut::new(&rig.panes, plan.clone());
    let before = rig.cursor();
    let (answer, calls) = rig.engine_over(&seam, &request(false, true, false), &options());
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-c", StepOutcome::Failed),
            ("pane-z", StepOutcome::Skipped)
        ]
    );
    assert_eq!(
        step(&applied, "pane-c").code.as_deref(),
        Some("generation-conflict")
    );
    assert_nothing_live(&calls);
    let left = written(&c, None);
    assert_eq!(
        records(&rig.writes_since(before)),
        vec![left.clone()],
        "the leave alone"
    );
    assert_eq!(rig.record("pane-c"), Some(left), "in no profile");

    let stop = Stop {
        pane: "pane-c",
        code: "generation-conflict",
        action: Some(Action::Adopt),
        skipped: 1,
    };
    let message = assert_stopped(
        || seed().0,
        |rig, format| {
            let seam = NthCasPut::new(&rig.panes, plan.clone());
            rig.run_over(&seam, &apply_argv(&["--take-over"]), format)
        },
        &stop,
    );
    assert!(message.contains(Q), "names Q: {message}");
    assert!(message.contains("no profile"), "{message}");
}
