//! A real run with a `refused` step and no `failed` one exits 3 and prints only the refusal
//! (#853; ADR-0021 section 8, "Apply's step order and its exit code", and decision 33), the
//! twin of a failed step's `assert_stopped` (the decision on O6). A dry run that plans a
//! refusal still exits 0 with its plan (`dry_run.rs`), and a `reported` step alone still exits
//! 0 (`rows.rs`, row d).
//!
//! [`assert_refused`] is the one check of the refused form; the cases of `rows.rs`,
//! `failures.rs` and `order.rs` that run the verb with a refused step use it.

use holler_cli::output::Format;
use holler_cli::pane::profile_scope::reconcile_step;
use holler_pane::findings::quoted;
use holler_pane::tx_apply::{Action, Outcome as StepOutcome};
use holler_pane::PaneError;
use holler_pane_testkit::harness::{HarnessOp, AGENT_MISMATCH};
use serde_json::Value;

use super::rig::{
    apply_argv, assert_stopped, cell, envelope_of, new_spec, options, outcomes, profile_name,
    request, spec_of, step, Rig, Run, Seed, Stop, HS, P, Q,
};

/// What a real run with refused steps and no failed one prints (#853).
pub(crate) struct Refusal<'a> {
    /// Every refused step's pane and code, in step order.
    pub refused: &'a [(&'a str, &'a str)],
    /// How many other steps ran.
    pub ran: usize,
}

/// Decision 33, at the verb: each format on its own fresh rig (`seed`), run by `run`, exits 3
/// and prints only the refusal. JSON: one failure envelope, `data` null, `error.code` the first
/// refused step's code, `error.message` starting with the pinned form `apply refused <N>
/// step(s): <pane> (<code>), ...; <M> other step(s) ran`. Text: nothing on stdout and one
/// `error: <same message>` line on stderr. Returns the rest of the message, after that form:
/// empty when no refused step had begun its act.
pub(crate) fn assert_refused(
    seed: impl Fn() -> Rig,
    run: impl Fn(&Rig, Format) -> Run,
    refusal: &Refusal<'_>,
) -> String {
    let json = run(&seed(), Format::Json);
    assert_eq!(json.code, 3, "a refused step exits 3: {json:?}");
    let envelope = envelope_of(&json);
    assert_eq!(envelope.data, Value::Null, "no steps: {json:?}");
    let error = envelope.error.expect("a failure");
    let (_, first) = refusal.refused.first().expect("a case names a refused step");
    assert_eq!(error.code, *first, "the first refused step's code: {json:?}");
    let named: Vec<String> = refusal
        .refused
        .iter()
        .map(|(pane, code)| format!("{} ({code})", quoted(pane)))
        .collect();
    let head = format!(
        "apply refused {} step(s): {}; {} other step(s) ran",
        refusal.refused.len(),
        named.join(", "),
        refusal.ran
    );
    let message = error.message;
    let rest = message
        .strip_prefix(&head)
        .unwrap_or_else(|| panic!("starts with {head:?}: {message}"))
        .to_owned();

    let text = run(&seed(), Format::Text);
    assert_eq!(text.code, 3, "{text:?}");
    assert_eq!(text.out, "", "no steps on stdout: {text:?}");
    assert_eq!(text.err.lines().count(), 1, "one line: {text:?}");
    assert_eq!(text.err.trim_end(), format!("error: {message}"), "{text:?}");
    rest
}

/// `rest` (what [`assert_refused`] returned) is exactly one refused step's carried detail,
/// `; <pane>: <detail>`: the engine's `detail` of that step, then P's reconcile step at the end,
/// once (ADR-0021 section 8, step 6).
pub(crate) fn assert_carries(rest: &str, pane: &str, detail: &str) {
    let reconcile = reconcile_step(Some(&profile_name(P)));
    let carried = format!("; {}: {detail}", quoted(pane));
    assert!(rest.starts_with(&carried), "carries {carried:?}: {rest}");
    assert!(rest.ends_with(&reconcile), "ends with {reconcile:?}: {rest}");
    assert_eq!(rest.matches(&reconcile).count(), 1, "once: {rest}");
}

/// The engine's `detail` of `pane`'s step on a fresh `seed`, run with `take_over`.
fn engine_detail(seed: impl Fn() -> Rig, take_over: bool, pane: &str) -> String {
    let (answer, _) = seed().engine(&request(false, take_over, true), &options());
    let applied = answer.unwrap();
    let step = step(&applied, pane);
    assert_eq!(step.outcome, StepOutcome::Refused, "{step:?}");
    step.detail.clone().expect("a refused step has a detail")
}

/// The next `verify_serving` answers that the server on 48101 runs another agent (C25): a
/// refusal from inside the launch act.
fn refuse_the_next_verify_serving(rig: &Rig) {
    let refusal = PaneError::Refused {
        code: AGENT_MISMATCH,
        message: "the server on port 48101 does not run the agent the spec asks for".to_owned(),
    };
    rig.harness
        .faults()
        .fail_next(HarnessOp::VerifyServing, refusal);
}

/// `pane-a` of Q (r1c1: row a, refused before anything runs), then `pane-z` of P (r2c1,
/// healthy, matching), then `pane-x` (r1c2), whose launch act `verify_serving` answers with a
/// refusal (C25). `pane-x` has no record (a `create`) or, with `relaunch`, is an agent of P
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
    refuse_the_next_verify_serving(&rig);
    rig
}

/// C25 (#802; §4, §5) and #853: a refusal that arrives after the launch act began
/// (`verify_serving`, once the pane, its session and the server command have run) is still a
/// `refused` step with its code, and the run goes on. The run exits 3, coded as the first
/// refused step (row a's), and must not read as if nothing ran: in both formats the one line
/// carries `pane-x`'s detail, which ends with the reconcile step. Row a's refusal, which ran
/// nothing, adds nothing beyond its pane and code. For a `create` and for a row f relaunch.
#[test]
fn a_refusal_after_the_act_began_carries_its_detail_and_the_reconcile_step() {
    for (relaunch, action) in [(false, Action::Create), (true, Action::Relaunch)] {
        let rig = refused_mid_act(relaunch);
        let (answer, _) = rig.engine(&request(false, false, true), &options());
        let applied = answer.unwrap();
        assert_eq!(
            outcomes(&applied),
            vec![
                ("pane-a", StepOutcome::Refused),
                ("pane-z", StepOutcome::Adopted),
                ("pane-x", StepOutcome::Refused),
            ],
            "{action:?}"
        );
        let x = step(&applied, "pane-x");
        assert_eq!(
            (x.action, x.code.as_deref()),
            (action, Some(AGENT_MISMATCH.as_str())),
            "{x:?}"
        );

        let seed = || refused_mid_act(relaunch);
        let mismatch = AGENT_MISMATCH;
        let refusal = Refusal {
            refused: &[
                ("pane-a", "pane-in-other-profile"),
                ("pane-x", mismatch.as_str()),
            ],
            ran: 1,
        };
        let rest = assert_refused(seed, |rig, f| rig.run(&apply_argv(&HS), f), &refusal);
        assert_carries(&rest, "pane-x", &engine_detail(seed, false, "pane-x"));
        assert!(
            !rest.contains(&format!("{}:", quoted("pane-a"))),
            "row a ran nothing: {rest}"
        );
    }
}

/// `pane-x` of Q (r1c1), an agent whose server was killed, then `pane-z` of P (r2c1, healthy,
/// matching). Under `--take-over` `pane-x` leaves Q and joins P, and then its relaunch act is
/// refused by `verify_serving`.
fn taken_over_then_refused() -> Rig {
    let rig = Rig::new();
    let x = rig.seed_live(&Seed::at("pane-x", cell(1, 1), 48101).in_profile(Some(Q)));
    rig.kill(&x);
    let z = rig.seed_live(&Seed::at("pane-z", cell(2, 1), 48102));
    rig.set_specs(P, vec![spec_of(&x), spec_of(&z)]);
    refuse_the_next_verify_serving(&rig);
    rig
}

/// #853 (ADR-0021 section 8): an acted refusal after a take-over carries its take-over note
/// ("left profile Q and joined P before the relaunch, and stays in P") in the one line, with the
/// reconcile step, so the move is not lost when the run prints only the refusal.
#[test]
fn an_acted_refusal_after_a_take_over_carries_its_note() {
    let detail = engine_detail(taken_over_then_refused, true, "pane-x");
    let note = format!("joined {} before the relaunch", quoted(P));
    assert!(detail.contains(&note), "the engine notes the move: {detail}");

    let mismatch = AGENT_MISMATCH;
    let refusal = Refusal {
        refused: &[("pane-x", mismatch.as_str())],
        ran: 1,
    };
    let rest = assert_refused(
        taken_over_then_refused,
        |rig, f| rig.run(&apply_argv(&["--take-over"]), f),
        &refusal,
    );
    assert_carries(&rest, "pane-x", &detail);
}

/// `pane-a` of Q (r1c1: row a, refused), then `pane-y` (r2c1, no record), whose launch fails
/// once its pane, session and server are up (`create_session` answers `unavailable`).
fn refused_then_failed() -> Rig {
    let rig = Rig::new();
    let a = rig.seed_live(&Seed::at("pane-a", cell(1, 1), 48100).in_profile(Some(Q)));
    rig.set_specs(P, vec![spec_of(&a), new_spec("pane-y", cell(2, 1), 48101)]);
    rig.harness.faults().fail_next(
        HarnessOp::CreateSession,
        PaneError::Unavailable {
            what: "harness".to_owned(),
        },
    );
    rig
}

/// Decision 33's order: a `failed` step still stops the run and exits 1 with the failure
/// alone, even beside a refused step; the refusal does not turn it into exit 3.
#[test]
fn a_failed_step_beside_a_refused_one_still_exits_1() {
    let (answer, _) = refused_then_failed().engine(&request(false, false, true), &options());
    assert_eq!(
        outcomes(&answer.unwrap()),
        vec![
            ("pane-a", StepOutcome::Refused),
            ("pane-y", StepOutcome::Failed),
        ]
    );
    let stop = Stop {
        pane: "pane-y",
        code: "unavailable",
        action: Some(Action::Create),
        skipped: 0,
    };
    assert_stopped(
        refused_then_failed,
        |rig, f| rig.run(&apply_argv(&HS), f),
        &stop,
    );
}
