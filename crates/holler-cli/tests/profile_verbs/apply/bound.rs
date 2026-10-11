//! The run's bound and the re-read before each step (brief §9, C4, C13, C16 and O5; C14 entries
//! 19, 20, 26 and 27).

use std::time::{Duration, Instant};

use holler_pane::tx_apply::Outcome as StepOutcome;
use holler_pane::tx_launch::TxOptions;
use holler_pane::PaneError;
use holler_pane_testkit::harness::HarnessOp;

use super::rig::{cell, new_spec, options, outcomes, request, spec_of, step, Rig, Seed, P};

/// The module doc of `crates/holler-pane/src/tx_apply.rs`: its `//!` lines (an inner doc comment
/// can only open a module), each with the prefix dropped and trimmed, joined by single spaces
/// (C16).
fn tx_apply_doc() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../holler-pane/src/tx_apply.rs"
    );
    let source = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    source
        .lines()
        .filter(|line| line.starts_with("//!"))
        .map(|line| line.trim_start_matches("//!").trim())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Entry 20 (§9, C4, C16): the engine's module doc states the run's bound in `tx_launch`'s
/// form, so it cannot be dropped silently: the marker, each step's own I5 bound, no overall cap
/// (O5), the up-front observation's 10 s, and a step's `DEFAULT_BUDGET`.
#[test]
fn the_engine_doc_states_the_bound() {
    let doc = tx_apply_doc();
    for needle in [
        "**Bound (I5).**",
        "its own I5 bound",
        "no overall cap",
        "10 s",
        "DEFAULT_BUDGET",
    ] {
        assert!(
            doc.contains(needle),
            "the module doc holds {needle:?}: {doc:?}"
        );
    }
}

/// Two spec panes with no record (every step a `create`), so apply's own observations make no
/// harness call (C1) and every delayed harness call is a launch's.
fn two_creates() -> Rig {
    let rig = Rig::new();
    rig.set_specs(
        P,
        vec![
            new_spec("pane-y", cell(1, 1), 48100),
            new_spec("pane-z", cell(2, 1), 48101),
        ],
    );
    rig
}

/// Entry 26 (§9, C13, C7 R3 B-4): a step runs under its own budget, the `TxOptions` the engine
/// is given: a `create` that outlasts it is `failed` with `timeout` (op `pane.launch`), the later
/// step is `skipped`, and the engine returns well inside a generous bound (#644's numbers,
/// re-checked for #802 by C25: the plan's `serve_command` and port check take 1000 ms at 500 ms
/// a call, past the 800 ms budget, so the launch times out before its act, in about 1 s).
#[test]
fn a_step_that_outlasts_its_budget_fails_and_stops_the_run() {
    let rig = two_creates();
    let budget = TxOptions {
        budget: Duration::from_millis(800),
        ..options()
    };
    rig.harness
        .faults()
        .set_delay(Some(Duration::from_millis(500)));
    let started = Instant::now();
    let (answer, _) = rig.engine(&request(false, false, true), &budget);
    let took = started.elapsed();
    rig.harness.faults().set_delay(None);
    let applied = answer.unwrap();
    assert_eq!(
        outcomes(&applied),
        vec![
            ("pane-y", StepOutcome::Failed),
            ("pane-z", StepOutcome::Skipped)
        ]
    );
    let timeout = PaneError::Timeout {
        op: "pane.launch".to_owned(),
    };
    assert_eq!(step(&applied, "pane-y").error, Some(timeout));
    assert!(took < Duration::from_secs(3), "bounded: {took:?}");
    assert_eq!(rig.record("pane-y"), None);
    assert_eq!(rig.record("pane-z"), None);
}

/// Entry 27 (O5, C13, C7 R3 B-4; numbers re-derived for #802 by C25): there is no budget over
/// the whole run. Each `create` makes eight harness calls, so at 100 ms each one takes about
/// 800 ms: it fits a 1200 ms budget with about 400 ms to spare, and two in a row (about
/// 1600 ms) do not. Both are `created`, and the run is seen to outlast one budget.
#[test]
fn there_is_no_budget_over_the_whole_run() {
    let rig = two_creates();
    let budget = TxOptions {
        budget: Duration::from_millis(1200),
        ..options()
    };
    rig.harness
        .faults()
        .set_delay(Some(Duration::from_millis(100)));
    let started = Instant::now();
    let (answer, _) = rig.engine(&request(false, false, true), &budget);
    let took = started.elapsed();
    rig.harness.faults().set_delay(None);
    assert_eq!(
        outcomes(&answer.unwrap()),
        vec![
            ("pane-y", StepOutcome::Created),
            ("pane-z", StepOutcome::Created)
        ]
    );
    assert!(
        took > budget.budget,
        "the run outlasted one budget: {took:?}"
    );
}

/// Entry 19 (§9; plan-review warn 5(a) scopes it): a stale spec is never written back. Another
/// writer edits P's spec for `pane-y` during apply's up-front observation (right after
/// `pane-x`'s `health`); the step re-reads P, so the pane is created from the edited spec and P
/// keeps the other writer's version.
#[test]
fn a_spec_edited_during_the_observation_is_not_written_back() {
    let rig = Rig::new();
    let x = rig.seed_live(&Seed::at("pane-x", cell(1, 1), 48100));
    let y = new_spec("pane-y", cell(2, 1), 48101);
    rig.set_specs(P, vec![spec_of(&x), y.clone()]);
    let mut theirs = y;
    theirs.model.effort = "high".to_owned();
    let edited = vec![spec_of(&x), theirs.clone()];
    rig.after(HarnessOp::Health, move |rig| {
        rig.set_specs(P, edited);
    });
    let (answer, _) = rig.engine(&request(false, false, true), &options());
    assert_eq!(
        outcomes(&answer.unwrap()),
        vec![
            ("pane-x", StepOutcome::Adopted),
            ("pane-y", StepOutcome::Created)
        ]
    );
    let record = rig.record("pane-y").unwrap();
    assert_eq!(record.model.effort, "high", "created from the edited spec");
    let spec = rig
        .stored_profile(P)
        .panes
        .into_iter()
        .find(|s| s.pane == "pane-y")
        .unwrap();
    assert_eq!(spec, theirs, "the other writer's spec stays");
}
