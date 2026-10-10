//! `holler pane reset` (#645, brief ACs 2 and 14-19): a switch to a session the run
//! creates. The runner and its per-run checks (both formats, one envelope, no Herdr or host
//! call) are `switch.rs`'s.

use holler_pane::findings::FindingKind as K;
use holler_pane::reconcile::Report;
use holler_pane::{HarnessPort, PaneError};
use holler_pane_testkit::harness::{HarnessOp as H, Quirk};
use holler_pane_testkit::pane_store::PaneStoreOp;

use crate::doctor::rig::{keys, kinds, of_kind, one, whole, Rig, Seed};
use crate::switch::{argv, both, data, failed, failed_before_the_act, Case, P, Q, RECONCILE_P};
use crate::verb_harness::parse::try_parse;

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
