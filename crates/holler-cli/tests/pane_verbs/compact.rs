//! `holler pane compact PANE [--profile NAME]` (#878, brief ACs 1-5): compact the context of the
//! session of record of a pane, through the harness API and never by typing `/compact` into a
//! TUI (ADR-0021 I4), on the model the harness's own config pins (the verb carries no model).
//!
//! Every case runs the verb in **both** formats, each on a fresh doctor rig, through
//! `switch::both` (the runner `switch` and `reset` share): the exit code is the same in both
//! formats, JSON is one valid envelope with nothing on `err`, text is one line, and no run
//! calls Herdr or the host, so no keystroke reaches a TUI (AC 5). The engine the verb runs,
//! `holler_pane::tx_compact::compact`, is called directly in `compact/engine.rs`.
//!
//! The fake's contract these cases seed is `FakeHarness::{seed_context, set_compaction_pin,
//! set_activity, compact_runs}` (see `holler-pane-testkit/tests/fake_harness_compact_test.rs`).

use holler_pane::pane::{Health, Hold};
use holler_pane::ports::{Activity, CompactModel};
use holler_pane_testkit::harness::{CompactRun, HarnessOp as H};
use serde_json::json;

use crate::doctor::rig::{Rig, Seed};
use crate::switch::{argv, both, failed, Case, P};

mod engine;

/// The model the fake harness's own config pins: the one every successful run must have run on.
pub(crate) fn pinned() -> CompactModel {
    CompactModel {
        provider: "pinned-provider".to_owned(),
        model_id: "pinned-model".to_owned(),
    }
}

/// A rig of one pane whose session has a context of `before` tokens that a compaction brings
/// to `after`, on a server whose config pins [`pinned`].
pub(crate) fn compactable(before: u64, after: u64) -> Rig {
    let rig = Rig::new(&[Seed::new(P, 1, 1)]);
    let (port, session) = (rig.live(P).port, rig.live(P).session.clone());
    rig.harness.seed_context(port, &session, before, after);
    rig.harness.set_compaction_pin(port, Some(pinned()));
    rig
}

fn build() -> (Rig, ()) {
    (compactable(90_000, 12_000), ())
}

/// `pane compact P`, with `extra` after it.
fn compact_p(extra: &'static [&'static str]) -> impl Fn(&Rig, &()) -> Vec<String> {
    move |_, ()| {
        let mut words = argv(&["pane", "compact", P]);
        words.extend(argv(extra));
        words
    }
}

/// No compaction was sent to a model in any case of `cases`.
fn ran_nothing(cases: &[Case<()>; 2]) {
    for case in cases {
        assert!(
            case.rig.harness.compact_runs().is_empty(),
            "{:?}: nothing was sent to a model",
            case.format
        );
    }
}

/// AC 1: the context shrank, so the verb exits 0 and prints both counts. The harness was asked
/// once, naming no model, and the model that ran is the harness's own pin. The record is read,
/// never written: a compaction records nothing in the registry.
#[test]
fn compact_prints_before_and_after_on_the_harness_pinned_model() {
    let cases = both(build, compact_p(&[]));
    let [text, json_run] = &cases;

    assert_eq!(
        text.run.out,
        format!("compacted {P}: 90000 -> 12000 tokens\n"),
        "text mode prints the pane and both counts on one line"
    );
    assert_eq!(
        crate::switch::data(&cases),
        json!({
            "pane": P,
            "session": json_run.rig.live(P).session,
            "tokens_before": 90_000,
            "tokens_after": 12_000,
        }),
        "JSON mode: the four fields of the brief and nothing else"
    );
    for case in &cases {
        let live = case.rig.live(P);
        assert_eq!(
            case.rig.harness.compact_runs(),
            vec![CompactRun {
                port: live.port,
                session: live.session.clone(),
                asked: None,
                ran_on: pinned(),
            }],
            "{:?}: exactly one compaction, asked for no model, run on the pin",
            case.format
        );
        assert_eq!(
            case.calls.harness,
            [H::Health, H::Compact],
            "{:?}: the health check, then the one act",
            case.format
        );
        assert!(
            !case.calls.panes.iter().any(|op| matches!(
                op,
                holler_pane_testkit::pane_store::PaneStoreOp::CasPut
                    | holler_pane_testkit::pane_store::PaneStoreOp::Delete
            )),
            "{:?}: no registry write: {:?}",
            case.format,
            case.calls.panes
        );
        case.assert_unchanged();
    }
}

/// AC 2: a session that is running a turn, holds a question or waits on a permission is
/// refused with the code of what it is doing (exit 3), and the harness was never asked.
#[test]
fn compact_refuses_a_busy_session_and_sends_nothing() {
    for (activity, code) in [
        (Activity::Busy, "session-busy"),
        (Activity::HoldingQuestion, "session-holds-question"),
        (Activity::WaitingPermission, "session-holds-permission"),
    ] {
        let busy = move || {
            let rig = compactable(90_000, 12_000);
            let live = rig.live(P);
            rig.harness.set_activity(live.port, &live.session, activity);
            (rig, ())
        };
        let cases = both(busy, compact_p(&[]));
        for message in failed(&cases, 3, code) {
            assert!(message.contains(P), "{code}: names the pane: {message:?}");
        }
        for case in &cases {
            assert_eq!(
                case.calls.harness,
                [H::Health],
                "{code}, {:?}: refused by the engine, before the act",
                case.format
            );
            case.assert_unchanged();
        }
        ran_nothing(&cases);
    }
}

/// AC 3 (OpenCode half): a harness whose server config pins no compaction model is refused
/// `compact-model-unpinned` (exit 3). The harness was asked, and sent nothing to a model: the
/// verb never picks a model nobody chose.
#[test]
fn compact_without_a_pinned_model_is_refused_and_sends_nothing() {
    let unpinned = || {
        let rig = compactable(90_000, 12_000);
        rig.harness.set_compaction_pin(rig.live(P).port, None);
        (rig, ())
    };
    let cases = both(unpinned, compact_p(&[]));
    failed(&cases, 3, "compact-model-unpinned");
    for case in &cases {
        assert_eq!(
            case.calls.harness,
            [H::Health, H::Compact],
            "{:?}",
            case.format
        );
        case.assert_unchanged();
    }
    ran_nothing(&cases);
}

/// AC 4: a compaction that did not shrink the context is a failure (exit 1, `unavailable`, the
/// closed code of a mismatch observed after the act), whose message says so and names both
/// counts. `--json` gives `ok: false`. No retry: the harness was asked once.
#[test]
fn compact_that_does_not_shrink_exits_1_naming_both_counts() {
    for (before, after) in [(5_000, 5_000), (5_000, 6_000)] {
        let build = move || (compactable(before, after), ());
        let cases = both(build, compact_p(&[]));
        for message in failed(&cases, 1, "unavailable") {
            assert!(
                message.contains("did not shrink"),
                "{before}->{after}: says the context did not shrink: {message:?}"
            );
            for count in [before, after] {
                assert!(
                    message.contains(&count.to_string()),
                    "{before}->{after}: names {count}: {message:?}"
                );
            }
        }
        let [text, json_run] = &cases;
        assert!(
            text.run.out.is_empty(),
            "text: nothing on out: {:?}",
            text.run
        );
        assert!(!json_run.envelope().ok, "--json: ok is false");
        for case in &cases {
            assert_eq!(
                case.rig.harness.compact_runs().len(),
                1,
                "{:?}: asked once, never retried",
                case.format
            );
        }
    }
}

/// A server that does not answer is refused before anything else, as for `switch`: the
/// engine's health check, then nothing is sent.
#[test]
fn compact_refuses_an_unhealthy_server_first() {
    let killed = || {
        let rig = compactable(90_000, 12_000);
        rig.harness.kill(rig.live(P).port).expect("kill the server");
        (rig, ())
    };
    let cases = both(killed, compact_p(&[]));
    for message in failed(&cases, 3, "server-unhealthy") {
        assert!(
            message.contains("run holler pane relaunch demo-c1r1"),
            "{message:?}"
        );
    }
    for case in &cases {
        assert_eq!(case.calls.harness, [H::Health], "{:?}", case.format);
    }
    ran_nothing(&cases);
}

/// The pane-state refusals are the CLI's (`resolve_pane_record`, the `interrupt` precedent): a
/// parked pane, a pane whose record says its harness is unhealthy, and a pane that shows one
/// session while its body drives another are refused before the harness is reached at all,
/// because a compaction is a model call on the session of record.
#[test]
fn compact_applies_the_pane_state_refusals_before_the_harness() {
    type Change = fn(&mut holler_pane::Pane);
    let changes: [(&str, Change); 3] = [
        ("pane-parked", |p| {
            p.hold = Hold::Parked {
                reason: "disk full".to_owned(),
                release_when: "after the cleanup".to_owned(),
                since: 1,
            }
        }),
        ("pane-unhealthy", |p| {
            p.harness.health = Health::Unhealthy("no answer".to_owned())
        }),
        ("pane-shown-driven-mismatch", |p| {
            p.last_observed.shown = Some("ses_shown".to_owned());
            p.last_observed.driven = Some("ses_other".to_owned());
        }),
    ];
    for (code, change) in changes {
        let build = move || {
            let rig = compactable(90_000, 12_000);
            rig.rewrite(P, change);
            (rig, ())
        };
        let cases = both(build, compact_p(&[]));
        failed(&cases, 3, code);
        for case in &cases {
            assert!(
                case.calls.harness.is_empty(),
                "{code}, {:?}: the harness is not reached: {:?}",
                case.format,
                case.calls
            );
        }
        ran_nothing(&cases);
    }
}

/// A pane with no session of record has nothing to compact: `session-not-found`, naming the
/// pane, from `resolve_pane_record`, before the harness.
#[test]
fn compact_of_a_pane_without_a_session_of_record_is_session_not_found() {
    let build = || {
        let rig = compactable(90_000, 12_000);
        rig.rewrite(P, |p| p.session_of_record = None);
        (rig, ())
    };
    let cases = both(build, compact_p(&[]));
    for message in failed(&cases, 3, "session-not-found") {
        assert!(message.contains(P), "{message:?}");
    }
    for case in &cases {
        assert!(case.calls.harness.is_empty(), "{:?}", case.calls);
    }
}

/// `--profile` scopes the pane like every pane verb: a pane inside it compacts, a pane outside
/// it or a missing profile is refused before the harness is reached.
#[test]
fn compact_in_a_profile() {
    let in_profile = || {
        let rig = Rig::new(&[
            Seed::new(P, 1, 1).in_profile("demo"),
            Seed::new("demo-c2r1", 1, 2),
        ]);
        let live = rig.live(P);
        rig.harness.seed_context(live.port, &live.session, 800, 80);
        rig.harness.set_compaction_pin(live.port, Some(pinned()));
        (rig, ())
    };
    let cases = both(in_profile, compact_p(&["--profile", "demo"]));
    assert_eq!(crate::switch::data(&cases)["tokens_after"], 80);

    let outside = |_: &Rig, (): &()| argv(&["pane", "compact", "demo-c2r1", "--profile", "demo"]);
    let cases = both(in_profile, outside);
    failed(&cases, 3, "pane-not-in-profile");
    for case in &cases {
        assert!(case.calls.harness.is_empty(), "{:?}", case.calls);
    }

    let cases = both(in_profile, compact_p(&["--profile", "nope"]));
    failed(&cases, 3, "profile-not-found");
    for case in &cases {
        assert!(case.calls.harness.is_empty(), "{:?}", case.calls);
    }
}

/// A pane that has no record is `pane-not-found` (exit 3), and a bad name is `usage` (exit 2),
/// each before any harness call.
#[test]
fn compact_of_an_unknown_pane_and_a_bad_name() {
    use crate::switch::both_with_missing_record;
    let exec = |rig: &Rig, _: &(), argv: &[&str], format| rig.verb(argv, format);
    for (pane, exit, code) in [("demo-c9r9", 3, "pane-not-found"), ("BAD_NAME", 2, "usage")] {
        let words = move |_: &Rig, (): &()| argv(&["pane", "compact", pane]);
        let cases = both_with_missing_record(build, words, exec);
        failed(&cases, exit, code);
        for case in &cases {
            assert!(case.calls.harness.is_empty(), "{pane}: {:?}", case.calls);
        }
    }
}

/// The verb takes the shared target flags and no more: a model flag is an unknown argument, so
/// a compaction can never be sent to a model the operator did not choose through the harness's
/// own config (N1), and the spec flags (`--spec-only`, `--take-over`) are not its flags either.
#[test]
fn compact_has_no_model_flag_and_takes_the_target_flags_only() {
    use crate::verb_harness::parse::{accepted, unknown_argument};
    let mut failures = Vec::new();
    failures.extend(accepted(&["pane", "compact", P]).err());
    failures.extend(accepted(&["pane", "compact", P, "--profile", "demo"]).err());
    for flag in [
        &["--model", "a/b"][..],
        &["--provider", "a"][..],
        &["--effort", "high"][..],
        &["--spec-only"][..],
        &["--take-over"][..],
    ] {
        let mut words = vec!["pane", "compact", P];
        words.extend_from_slice(flag);
        failures.extend(unknown_argument(&words).err());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
