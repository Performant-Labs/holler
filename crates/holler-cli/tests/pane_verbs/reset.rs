//! `holler pane reset` (#645, brief ACs 2 and 14-19): a switch to a session the run
//! creates. The runner and its per-run checks (a text run and a JSON run on separate rigs, the
//! same exit code, a valid envelope, no Herdr or host call) are `switch.rs`'s.

use std::sync::Arc;

use holler_pane::findings::FindingKind as K;
use holler_pane::ports::Activity;
use holler_pane::reconcile::Report;
use holler_pane::{HarnessPort, PaneError};
use holler_pane_testkit::harness::{HarnessOp as H, Quirk};
use holler_pane_testkit::pane_store::PaneStoreOp;

use crate::doctor::rig::{keys, kinds, of_kind, one, whole, Rig, Seed};
use crate::switch::{
    argv, both, both_with, data, failed, failed_before_the_act, Case, SeamHarness, SeamLog, P, Q,
    RECONCILE_P,
};
use crate::verb_harness::parse::try_parse;
use crate::verb_harness::run_verb_with;

/// `pane reset <pane>`, with `extra` after it.
fn reset(pane: &'static str, extra: &'static [&'static str]) -> impl Fn(&Rig, &()) -> Vec<String> {
    move |_, ()| {
        let mut words = argv(&["pane", "reset", pane]);
        words.extend(argv(extra));
        words
    }
}

fn one_pane() -> (Rig, ()) {
    (Rig::new(&[Seed::new(P, 1, 1)]), ())
}

/// The sessions on the server at `port`.
fn sessions(rig: &Rig, port: u16) -> Vec<String> {
    rig.harness.list_sessions(port).expect("list the sessions")
}

/// The one session on `P`'s server that `before` does not have: the one the run created.
fn created(case: &Case<Vec<String>>) -> String {
    let now = sessions(&case.rig, case.rig.live(P).port);
    let new: Vec<String> = now
        .into_iter()
        .filter(|s| !case.setup.contains(s))
        .collect();
    assert_eq!(
        new.len(),
        1,
        "{:?}: the run created one session: {new:?}",
        case.format
    );
    new[0].clone()
}

/// The stray sessions a whole-fleet doctor pass reports.
fn strays(report: &Report) -> Vec<String> {
    of_kind(report, K::StraySession)
        .iter()
        .filter_map(|f| f.session.clone())
        .collect()
}

/// ACs 14 and 2: a fresh session, shown and recorded, through the harness API only.
#[test]
fn reset_creates_a_fresh_session_and_switches_to_it() {
    let cases = both(one_pane, reset(P, &[]));
    let data = data(&cases);
    for case in &cases {
        let s1 = &case.rig.live(P).session;
        let new = case
            .record(P)
            .session_of_record
            .expect("a session of record");
        assert_ne!(&new, s1, "{:?}: a new session", case.format);
        assert!(sessions(&case.rig, case.rig.live(P).port).contains(&new));
        case.assert_recorded(&new);
        let ops = [
            H::Health,
            H::CreateSession,
            H::SelectSession,
            H::ShownSession,
        ];
        assert_eq!(case.calls.harness, ops, "{:?}", case.format);
        let writes = [PaneStoreOp::Get, PaneStoreOp::CasPut];
        assert_eq!(case.calls.panes, writes, "{:?}", case.format);
    }
    let [text, json] = &cases;
    let (s1, new) = (&text.rig.live(P).session, text.record(P).session_of_record);
    let new = new.expect("a session of record");
    let want = format!("reset {P} to a new session \"{new}\" (was \"{s1}\")\n");
    assert_eq!(text.run.out, want);
    assert_eq!(data["verb"], "reset");
    assert_eq!(data["previous"], json.rig.live(P).session.as_str());
    assert_eq!(
        data["pane"],
        serde_json::to_value(json.record(P)).expect("json")
    );
}

/// No finding doctor's `reset` remedy is for is left about `P`.
fn assert_remedied(report: &Report) {
    for kind in [
        K::NoSessionOfRecord,
        K::SessionOfRecordMissing,
        K::ShownDrivenMismatch,
    ] {
        let about_p = of_kind(report, kind)
            .iter()
            .any(|f| f.pane.as_ref().is_some_and(|p| p.as_str() == P));
        assert!(!about_p, "no {} for {P}: {:?}", kind.code(), keys(report));
    }
}

/// AC 15 (a): doctor's remedy for a pane with no session of record runs and clears it.
#[test]
fn reset_is_doctors_remedy_for_no_session_of_record() {
    let build = || {
        let (rig, ()) = one_pane();
        rig.rewrite(P, |p| p.session_of_record = None);
        (rig, ())
    };
    let cases = both(build, reset(P, &[]));
    assert!(data(&cases)["previous"].is_null());
    let [text, _] = &cases;
    assert!(text.run.out.ends_with(" (was none)\n"), "{:?}", text.run);
    for case in &cases {
        assert_remedied(&case.rig.run(&whole(false)));
    }
}

/// AC 15 (b): doctor's remedy for a deleted session of record parses, runs and clears it.
#[test]
fn reset_is_doctors_remedy_for_a_deleted_session_of_record() {
    let build = || {
        let (rig, ()) = one_pane();
        let s1 = rig.live(P).session.clone();
        rig.harness.delete_session(&s1).expect("delete S1");
        let report = rig.run(&whole(false));
        let remedy = one(&report, K::SessionOfRecordMissing, P).remedy.clone();
        (rig, remedy.expect("doctor names a remedy"))
    };
    let words = |_: &Rig, remedy: &String| {
        let words: Vec<&str> = remedy.split(' ').collect();
        assert_eq!(words.first(), Some(&"holler"), "{remedy:?}");
        try_parse(&words[1..]).unwrap_or_else(|e| panic!("{remedy:?} parses: {e}"));
        argv(&words[1..])
    };
    let cases = both(build, words);
    let data = data(&cases);
    assert_eq!(data["previous"], cases[1].rig.live(P).session.as_str());
    for case in &cases {
        assert_eq!(case.setup, "holler pane reset demo-c1r1");
        assert_remedied(&case.rig.run(&whole(false)));
    }
}

/// AC 16 (Decision 18): the session `reset` moved away from stays, as a stray.
#[test]
fn reset_leaves_the_old_session_as_a_stray() {
    let cases = both(one_pane, reset(P, &[]));
    data(&cases);
    for case in &cases {
        let report = case.rig.run(&whole(false));
        assert_eq!(kinds(&report), ["stray-session"], "{:?}", keys(&report));
        assert_eq!(strays(&report), [case.rig.live(P).session.clone()]);
    }
}

/// AC 17: every refusal comes before the create, so it leaves no session behind. `Q`
/// shares `P`'s data directory, so its server lists every session even when `P`'s is dead.
#[test]
fn reset_refusals_create_nothing() {
    let refusals: [(Seed, &str, &[&str], &str); 4] = [
        (
            Seed::new(P, 1, 1).orchestrator(),
            P,
            &[],
            "orchestrator-pane",
        ),
        (Seed::new(P, 1, 1), P, &[], "server-unhealthy"),
        (
            Seed::new(P, 1, 1).in_profile("demo"),
            Q,
            &["--profile", "demo"],
            "pane-not-in-profile",
        ),
        (Seed::new(P, 1, 1), "demo-c9r9", &[], "pane-not-found"),
    ];
    for (seed, pane, extra, code) in refusals {
        let build = || {
            let rig = Rig::new(&[seed.clone(), Seed::new(Q, 1, 2)]);
            if code == "server-unhealthy" {
                rig.harness.kill(rig.live(P).port).expect("kill P's server");
            }
            let listed = sessions(&rig, rig.live(Q).port);
            (rig, listed)
        };
        let words = |_: &Rig, _: &Vec<String>| {
            let mut words = argv(&["pane", "reset", pane]);
            words.extend(argv(extra));
            words
        };
        let cases = both(build, words);
        failed_before_the_act(&cases, 3, code);
        for case in &cases {
            let calls = &case.calls.harness;
            assert!(!calls.contains(&H::CreateSession), "{code}: {calls:?}");
            assert_eq!(
                sessions(&case.rig, case.rig.live(Q).port),
                case.setup,
                "{code}"
            );
            case.assert_unchanged();
        }
    }
}

/// A one-pane rig with `fault` applied, and the sessions on `P`'s server before the run.
fn faulted(fault: impl Fn(&Rig)) -> impl Fn() -> (Rig, Vec<String>) {
    move || {
        let rig = Rig::new(&[Seed::new(P, 1, 1)]);
        fault(&rig);
        let listed = sessions(&rig, rig.live(P).port);
        (rig, listed)
    }
}

fn reset_p(_: &Rig, _: &Vec<String>) -> Vec<String> {
    argv(&["pane", "reset", P])
}

/// A failed create comes before `select_session`: the TUI and the record are untouched, so
/// the message has no reconcile step and names no created session.
#[test]
fn reset_create_failure_changes_nothing() {
    let build = faulted(|rig| {
        let error = PaneError::Unavailable {
            what: "harness".to_owned(),
        };
        rig.harness.faults().fail_next(H::CreateSession, error);
    });
    let cases = both(build, reset_p);
    for message in failed_before_the_act(&cases, 1, "unavailable") {
        assert!(!message.contains("was created"), "{message:?}");
    }
    for case in &cases {
        let calls = &case.calls.harness;
        assert!(!calls.contains(&H::SelectSession), "{calls:?}");
        case.assert_unchanged();
    }
}

/// AC 18: a select that fails after the create names the session it left unrecorded,
/// which doctor then reports as a stray.
#[test]
fn reset_failure_after_create_names_the_unrecorded_session() {
    let build = faulted(|rig| {
        let error = PaneError::Unavailable {
            what: "tui".to_owned(),
        };
        rig.harness.faults().fail_next(H::SelectSession, error);
    });
    let cases = both(build, reset_p);
    let messages = failed(&cases, 1, "unavailable");
    for (case, message) in cases.iter().zip(messages) {
        let new = created(case);
        let named = format!("session \"{new}\" was created and is not recorded");
        assert!(message.contains(&named), "{message:?}");
        assert!(message.ends_with(RECONCILE_P), "{message:?}");
        case.assert_unchanged();
        assert!(strays(&case.rig.run(&whole(false))).contains(&new));
    }
}

/// AC 19 (I3): the TUI does not show the new session after the act, so nothing is
/// recorded, and the message names the session left behind.
#[test]
fn reset_mismatch_records_nothing() {
    let build = faulted(|rig| {
        rig.harness
            .close_tui(&rig.pane_id(P))
            .expect("close the TUI");
        rig.harness.set_quirk(Quirk::SelectAckedWithoutTui, true);
    });
    let cases = both(build, reset_p);
    let messages = failed(&cases, 1, "unavailable");
    for (case, message) in cases.iter().zip(messages) {
        let new = created(case);
        assert!(message.contains(&format!("\"{new}\"")), "{message:?}");
        assert!(message.contains("its home screen"), "{message:?}");
        assert!(message.ends_with(RECONCILE_P), "{message:?}");
        case.assert_unchanged();
    }
}

/// Run `pane reset P <extra...>` in both formats over a [`SeamHarness`] that answers
/// `activity` to every activity query and `prompt` to every prompt, recording the
/// seam's calls into each case's own log (the runner builds one rig per format, so
/// the two runs' logs stay apart).
fn reset_over_seam(
    build: impl Fn() -> Rig,
    extra: &[&str],
    activity: Activity,
    prompt: Option<PaneError>,
) -> [Case<Arc<SeamLog>>; 2] {
    let build = move || (build(), Arc::new(SeamLog::default()));
    let words = move |_: &Rig, _: &Arc<SeamLog>| {
        let mut words = argv(&["pane", "reset", P]);
        words.extend(argv(extra));
        words
    };
    let exec = move |rig: &Rig, log: &Arc<SeamLog>, argv: &[&str], format| {
        let seam = SeamHarness::new(rig, P, log, activity, prompt.clone());
        run_verb_with(argv, format, rig.ports_with(&seam))
    };
    both_with(build, words, exec)
}

/// ACs 1 and 2 (part 2), the "order ran in the wrong session" incident as a test:
/// with `Q` and the old session present (one shared data directory, so the server
/// lists both), `--first` queues the text to the NEW session only — never the old
/// one, never another pane's session or port — and only after the record write. The
/// run is part 1's reset otherwise (same fake-log op sequence; no keystroke, by the
/// runner's I4 check).
#[test]
fn reset_with_first_queues_the_message_to_the_new_session_only() {
    let cases = reset_over_seam(
        || Rig::new(&[Seed::new(P, 1, 1), Seed::new(Q, 1, 2)]),
        &["--first", "ship the fix"],
        Activity::Idle,
        None,
    );
    data(&cases);
    for case in &cases {
        let s1 = case.rig.live(P).session.clone();
        let q = case.rig.live(Q).session.clone();
        let new = case
            .record(P)
            .session_of_record
            .expect("the new session of record");
        assert_ne!(
            &new, &s1,
            "{:?}: a new session, not the old one",
            case.format
        );
        assert_ne!(&new, &q, "{:?}: not the other pane's session", case.format);
        case.assert_recorded(&new);
        let ops = [
            H::Health,
            H::CreateSession,
            H::SelectSession,
            H::ShownSession,
        ];
        assert_eq!(case.calls.harness, ops, "{:?}", case.format);
        let prompts = case.setup.prompts();
        assert_eq!(
            prompts.len(),
            1,
            "{:?}: one prompt, to the new session only: {prompts:?} (old {s1:?}, {Q} {q:?})",
            case.format
        );
        let prompt = &prompts[0];
        assert_eq!(prompt.port, case.rig.live(P).port, "{:?}", case.format);
        assert_eq!(prompt.session, new, "{:?}: the new session", case.format);
        assert_eq!(prompt.text, "ship the fix", "{:?}", case.format);
        assert_eq!(
            prompt.then_of_record.as_deref(),
            Some(new.as_str()),
            "{:?}: the prompt follows the record write",
            case.format
        );
    }
}

/// AC 1 (part 2): `--first` absent is part 1 exactly — no prompt is queued (the
/// wrapper would have recorded one), and the op sequence and the record are part
/// 1's.
#[test]
fn reset_without_first_queues_no_prompt() {
    let cases = reset_over_seam(
        || Rig::new(&[Seed::new(P, 1, 1)]),
        &[],
        Activity::Idle,
        None,
    );
    data(&cases);
    for case in &cases {
        assert!(
            case.setup.prompts().is_empty(),
            "{:?}: no send_prompt call without --first",
            case.format
        );
        let new = case
            .record(P)
            .session_of_record
            .expect("the new session of record");
        case.assert_recorded(&new);
        let ops = [
            H::Health,
            H::CreateSession,
            H::SelectSession,
            H::ShownSession,
        ];
        assert_eq!(case.calls.harness, ops, "{:?}", case.format);
        let writes = [PaneStoreOp::Get, PaneStoreOp::CasPut];
        assert_eq!(case.calls.panes, writes, "{:?}", case.format);
    }
}

/// AC 3 (part 2): a `send_prompt` failure after a successful record is reported
/// honestly: the run fails with the port's own code, the message says the first
/// message did not land, and nothing is appended — no reconcile step (the record is
/// correct) and no "is not recorded" clause (the session IS recorded). The record
/// and the TUI stand exactly as a successful reset left them.
#[test]
fn reset_first_prompt_failure_after_the_record_is_honest() {
    let fault = PaneError::Unavailable {
        what: "the harness".to_owned(),
    };
    let cases = reset_over_seam(
        || Rig::new(&[Seed::new(P, 1, 1)]),
        &["--first", "ship the fix"],
        Activity::Idle,
        Some(fault),
    );
    for message in failed(&cases, 1, "unavailable") {
        assert!(
            message.contains("first message did not land"),
            "names what failed: {message:?}"
        );
        assert!(
            !message.contains("to reconcile"),
            "no reconcile step: {message:?}"
        );
        assert!(
            !message.contains("is not recorded"),
            "the session is recorded: {message:?}"
        );
    }
    for case in &cases {
        assert_eq!(
            case.setup.prompts().len(),
            1,
            "{:?}: the prompt was attempted",
            case.format
        );
        let new = case
            .record(P)
            .session_of_record
            .expect("the new session of record");
        case.assert_recorded(&new);
    }
}

/// AC 4 (part 2): a pane whose conversation is busy or holds a question refuses in
/// the plan, before any write: nothing changes, no session is created, and the seam
/// saw exactly one query — the pane's CURRENT session of record. The codes are the
/// stable pair ADR-0021's amendment of the PROPOSED 645b row names:
/// `session-busy`, `session-holds-question`.
#[test]
fn reset_refuses_a_busy_or_questioning_conversation() {
    let refusals: [(Activity, &str); 2] = [
        (Activity::Busy, "session-busy"),
        (Activity::HoldingQuestion, "session-holds-question"),
    ];
    for (activity, code) in refusals {
        let cases = reset_over_seam(|| Rig::new(&[Seed::new(P, 1, 1)]), &[], activity, None);
        for message in failed_before_the_act(&cases, 3, code) {
            assert!(message.contains(P), "names the pane: {message:?}");
        }
        for case in &cases {
            assert!(
                !case.calls.harness.contains(&H::CreateSession),
                "{code}: no session created"
            );
            assert!(
                !case.calls.panes.contains(&PaneStoreOp::CasPut),
                "{code}: no record written"
            );
            assert_eq!(
                case.setup.activity(),
                [(case.rig.live(P).port, case.rig.live(P).session.clone())],
                "{code}: one query, on the session of record"
            );
            case.assert_unchanged();
        }
    }
}

/// AC 4 (part 2), the no-session-of-record proof: a pane with NO session of record
/// is not refused (doctor's remedy case) — and the wrapper would answer busy for
/// any query, so the run succeeding with no query at all proves the gate skips on
/// `None`, not because a default said idle.
#[test]
fn reset_skips_the_activity_gate_without_a_session_of_record() {
    let cases = reset_over_seam(
        || {
            let (rig, ()) = one_pane();
            rig.rewrite(P, |p| p.session_of_record = None);
            rig
        },
        &[],
        Activity::Busy,
        None,
    );
    assert!(data(&cases)["previous"].is_null());
    for case in &cases {
        assert!(
            case.setup.activity().is_empty(),
            "{:?}: no session_activity call",
            case.format
        );
        assert!(
            case.setup.prompts().is_empty(),
            "{:?}: no prompt without --first",
            case.format
        );
        let new = case
            .record(P)
            .session_of_record
            .expect("the new session of record");
        case.assert_recorded(&new);
    }
}
