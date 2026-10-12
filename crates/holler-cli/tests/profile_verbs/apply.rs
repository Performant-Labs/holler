//! `holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]` (story #664):
//! the declarative convergence of the live panes to a profile. Replaces #670's stub case.
//!
//! The engine is `holler_pane::tx_apply::apply`; the cases call it over the rig's fakes and read
//! its steps from the result it returns (the decision on O6), or run the verb in-process and read
//! its envelope. The brief's acceptance list is C14; each case names the entries it covers.
//!
//! - `apply.rs`: the verb's surface: errors before any plan (C8), the `--herdr-session` gate
//!   (§4, C1; entry 18), the envelope of a run that exits 0 (C6) and its text form.
//! - `apply/rows.rs`: one case per row of §3 and per per-pane answer of §4 (entries 1-3, 5, 8-10,
//!   12, 13, 16, 30, 32).
//! - `apply/order.rs`: the steps' order, column 1 first, and the grid limit that stays (#853).
//! - `apply/refused.rs`: a real run with a `refused` step exits 3 and prints only the refusal
//!   (#853), with C25's refusal after the act began.
//! - `apply/takeover.rs`: `--take-over` and the registry writes of rows c, d, e and f (entries 6,
//!   14, 15, 25, 29).
//! - `apply/failures.rs`: a `failed` step stops the run (entries 17, 21, 22, 23, 33).
//! - `apply/observe.rs`: what "healthy" observes (entries 24, 28, and plan-review warn 1).
//! - `apply/dry_run.rs`: what a dry run calls and prints (entries 4, 7, 11, 31).
//! - `apply/bound.rs`: the bound (entries 20, 26, 27) and the stale spec (entry 19).

pub(crate) mod rig;

mod bound;
mod dry_run;
mod failures;
mod observe;
mod order;
mod refused;
mod rows;
mod takeover;

use holler_cli::output::Format;
use holler_pane::tx_apply::{Action, Outcome as StepOutcome};
use holler_pane::PaneError;
use holler_pane_testkit::fault::Fault;
use serde_json::{json, Value};

use rig::{
    apply_argv, assert_no_store_write, cell, data_of, envelope_of, json_step, new_spec, spec_of,
    wire, Rig, Run, Seed, HS, P,
};

/// One recorded, healthy pane of P that matches its spec (`pane-c`, r1c1) and one spec pane
/// with no record (`pane-b`, r2c1): a plan that holds a `create`.
fn with_a_create() -> Rig {
    let rig = Rig::new();
    let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100));
    rig.set_specs(P, vec![spec_of(&c), new_spec("pane-b", cell(2, 1), 48101)]);
    rig
}

/// A run that failed before any step (exit `exit`, coded `code`): no Herdr, harness or host
/// call, no probe, no store write, and only the error printed: in JSON a failure envelope whose
/// `data` is null, in text an `error:` line on stderr and nothing on stdout.
fn assert_failed_before_any_step(run: &Run, format: Format, code: &str, exit: i32) {
    assert_eq!(run.code, exit, "{format:?}: {run:?}");
    assert_eq!(run.calls.herdr, vec![], "no Herdr call: {run:?}");
    assert_eq!(run.calls.harness, vec![], "no harness call: {run:?}");
    assert_eq!(run.calls.host, vec![], "no host call");
    assert_eq!(run.calls.probes, vec![], "no probe");
    assert_no_store_write(&run.calls);
    match format {
        Format::Text => {
            assert!(run.out.is_empty(), "no plan on stdout: {run:?}");
            assert!(run.err.starts_with("error: "), "{run:?}");
        }
        Format::Json => {
            let envelope = envelope_of(run);
            assert_eq!(envelope.data, Value::Null, "{run:?}");
            let error = envelope.error.expect("a failure");
            assert_eq!(error.code, code, "{run:?}");
        }
    }
}

/// Entry 18 (§4, C1, C7 R2 W-3): a plan that holds a `create` without `--herdr-session` exits 2
/// before any step runs, live and dry, in both formats: the usage error alone (no plan, not even
/// the recorded pane's step), no Herdr or harness call, no probe, no write.
#[test]
fn a_create_without_herdr_session_is_usage_before_any_call() {
    for extra in [&[][..], &["--dry-run"][..]] {
        for format in [Format::Text, Format::Json] {
            let rig = with_a_create();
            let before = rig.cursor();
            let run = rig.run(&apply_argv(extra), format);
            assert_failed_before_any_step(&run, format, "usage", 2);
            assert_eq!(
                rig.writes_since(before),
                vec![],
                "nothing written: {extra:?}"
            );
            assert!(
                !run.out.contains("pane-c") && !run.err.contains("pane-c"),
                "{run:?}"
            );
            if format == Format::Json {
                let message = envelope_of(&run).error.expect("a failure").message;
                assert!(message.contains("--herdr-session"), "{message}");
            }
        }
    }
}

/// Entry 1's shape and C6: a live run that exits 0 prints one envelope whose `data` is §7's
/// object itself, `{"profile", "dry_run", "steps"}`, one step per spec pane with its pane,
/// action and outcome; the text form names every pane on stdout.
#[test]
fn a_run_that_exits_0_prints_the_steps_object() {
    let rig = with_a_create();
    let run = rig.run(&apply_argv(&HS), Format::Json);
    let data = data_of(&run);
    assert_eq!(data["profile"], json!(P));
    assert_eq!(data["dry_run"], json!(false));
    let panes: Vec<&Value> = data["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| &s["pane"])
        .collect();
    assert_eq!(panes, [&json!("pane-c"), &json!("pane-b")], "{data}");
    let c = json_step(&data, "pane-c");
    assert_eq!(c["action"], json!(wire(Action::Adopt)));
    assert_eq!(c["outcome"], json!("adopted"));
    assert_eq!(wire(StepOutcome::Adopted), "adopted");
    let b = json_step(&data, "pane-b");
    assert_eq!(b["action"], json!(wire(Action::Create)));
    assert_eq!(b["outcome"], json!("created"));

    let rig = with_a_create();
    let text = rig.run(&apply_argv(&HS), Format::Text);
    assert_eq!(text.code, 0, "{text:?}");
    for pane in ["pane-c", "pane-b"] {
        assert!(text.out.contains(pane), "the text names {pane}: {text:?}");
    }
}

/// One case of an error before any plan exists.
struct BeforePlan {
    what: &'static str,
    break_it: fn(&Rig),
    profile: &'static str,
    code: &'static str,
    exit: i32,
}

/// C8: an error before any plan exists is the verb's own error, in both formats: a profile
/// that does not exist is `profile-not-found` (exit 3), a store that cannot be read is its own
/// failure (exit 1), a bad profile name is `usage` (exit 2). Nothing live is called.
#[test]
fn an_error_before_any_plan_is_the_verbs_own() {
    let cases = [
        BeforePlan {
            what: "no such profile",
            break_it: |_| {},
            profile: "Nope",
            code: "profile-not-found",
            exit: 3,
        },
        BeforePlan {
            what: "unreadable registry",
            break_it: |rig| {
                rig.panes
                    .faults()
                    .set(Some(Fault::Fail(PaneError::Unavailable {
                        what: "registry".to_owned(),
                    })))
            },
            profile: P,
            code: "unavailable",
            exit: 1,
        },
        BeforePlan {
            what: "bad name",
            break_it: |_| {},
            profile: "   ",
            code: "usage",
            exit: 2,
        },
    ];
    for case in cases {
        for format in [Format::Text, Format::Json] {
            let rig = with_a_create();
            (case.break_it)(&rig);
            let argv = ["profile", "apply", case.profile, HS[0], HS[1]];
            let run = rig.run(&argv, format);
            assert_eq!(run.code, case.exit, "{}: {run:?}", case.what);
            assert_failed_before_any_step(&run, format, case.code, case.exit);
        }
    }
}
