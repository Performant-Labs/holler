//! `reset --first` goes through the hub's gate (#839, D4, D5 and D6; Acceptance 7 and 12): the
//! plan asks the hub, without sending, whether it would admit the prompt (`HubPort::prompt` with
//! `Check`), so a parked, unhealthy, mismatched or held pane, and a pane no body drives, is
//! refused before anything moves: no session is created and the record is byte-for-byte as it
//! was. A healthy pane's first message goes through `HubPort::prompt` with `Send`, never
//! through `HarnessPort::send_prompt`.

use std::sync::Arc;

use holler_pane::hub::PromptMode;
use holler_pane::pane::{Health, Hold};
use holler_pane::ports::Activity;
use holler_pane::{Pane, PaneError};
use holler_pane_testkit::harness::HarnessOp as H;
use holler_pane_testkit::hub::{HoldKind, HubOp};

use super::{advertised, body, body_name, drive};
use crate::doctor::rig::{Rig, Seed};
use crate::switch::{
    argv, both_with, data, failed, failed_before_the_act, Case, SeamHarness, SeamLog, P,
};
use crate::verb_harness::run_verb_with;

const TEXT: &str = "ship the fix";

/// `pane reset P --first TEXT` over a seam harness that records any `send_prompt`, on the rig
/// `build` makes.
fn reset_first(build: impl Fn() -> Rig) -> [Case<Arc<SeamLog>>; 2] {
    let build = move || (build(), Arc::new(SeamLog::default()));
    let words = |_: &Rig, _: &Arc<SeamLog>| argv(&["pane", "reset", P, "--first", TEXT]);
    let exec = |rig: &Rig, log: &Arc<SeamLog>, argv: &[&str], format| {
        let seam = SeamHarness::new(rig, P, log, Activity::Idle, None);
        run_verb_with(argv, format, rig.ports_with(&seam))
    };
    both_with(build, words, exec)
}

/// A rig with `P` driven by its body and its record changed by `change` (SHOWN set to the
/// session of record first, so the pair is observed).
fn driven_with(change: fn(&mut Pane)) -> Rig {
    let rig = Rig::new(&[Seed::new(P, 1, 1)]);
    drive(&rig, P);
    rig.rewrite(P, |p| {
        p.last_observed.shown = p.session_of_record.clone();
        p.last_observed.driven = p.session_of_record.clone();
        change(p);
    });
    rig
}

/// Nothing moved: no session created or selected, the record and TUI as they were (generation
/// included), the body not re-pointed, and no prompt sent anywhere.
fn assert_refused_in_the_plan<T>(case: &Case<T>, log: &SeamLog) {
    let f = case.format;
    case.assert_unchanged();
    for op in [H::CreateSession, H::SelectSession] {
        assert!(
            !case.calls.harness.contains(&op),
            "{f:?}: no {op:?}: {:?}",
            case.calls.harness
        );
    }
    assert!(
        !case.calls.hub.contains(&HubOp::Rebind),
        "{f:?}: {:?}",
        case.calls.hub
    );
    assert!(
        case.rig.hub.prompts().is_empty(),
        "{f:?}: nothing sent through the hub"
    );
    assert!(
        log.prompts().is_empty(),
        "{f:?}: nothing sent through the harness"
    );
}

/// Acceptance 7: the pane gate's three refusals, asked at the hub before the act, each with the
/// pane's own code and no reconcile step.
#[test]
fn reset_first_on_a_gated_pane_is_refused_before_anything_moves() {
    type Change = fn(&mut Pane);
    let gated: [(&str, Change); 3] = [
        ("pane-parked", |p| {
            p.hold = Hold::Parked {
                reason: "disk full".to_owned(),
                release_when: "later".to_owned(),
                since: 5,
            }
        }),
        ("pane-unhealthy", |p| {
            p.harness.health = Health::Unhealthy("server-down".to_owned())
        }),
        ("pane-shown-driven-mismatch", |p| {
            p.last_observed.driven = Some(holler_pane_testkit::fixture::session_id(9));
        }),
    ];
    for (code, change) in gated {
        let cases = reset_first(move || driven_with(change));
        for message in failed_before_the_act(&cases, 3, code) {
            assert!(message.contains(P), "{code}: names the pane: {message:?}");
        }
        for case in &cases {
            assert_refused_in_the_plan(case, &case.setup);
            let checks: Vec<_> = case
                .rig
                .hub
                .log()
                .into_iter()
                .map(|c| (c.op, c.mode))
                .collect();
            assert_eq!(checks, [(HubOp::Prompt, Some(PromptMode::Check))], "{code}");
        }
    }
}

/// Acceptance 7 and D4: an operator or a default hold on the body's session refuses
/// `reset --first` with `session-held` (exit 3: no pane verb exits 4), naming the release.
#[test]
fn reset_first_on_a_held_pane_is_session_held() {
    for kind in [HoldKind::Operator, HoldKind::Default] {
        let cases = reset_first(move || {
            let rig = driven_with(|_| {});
            rig.hub.hold(&body_name(P), kind);
            rig
        });
        for message in failed_before_the_act(&cases, 3, "session-held") {
            let release = format!("holler release {}", body_name(P));
            assert!(message.contains(&release), "{kind:?}: {message:?}");
        }
        for case in &cases {
            assert_refused_in_the_plan(case, &case.setup);
        }
    }
}

/// Acceptance 7 and D6: no body drives the pane, or its body is behind its session of record (the
/// gate joins by the session of record, so it can judge no other): `pane-not-driven`, before
/// anything moves. `reset` without `--first` heals either.
#[test]
fn reset_first_on_a_pane_no_body_drives_is_not_driven() {
    let behind = || {
        let rig = Rig::new(&[Seed::new(P, 1, 1)]);
        let old = rig.harness.seed_session(rig.live(P).port);
        body(&rig, &body_name(P), &old, Activity::Idle);
        rig.rewrite(P, |p| p.last_observed.driven = Some(old.clone()));
        rig
    };
    type Build = fn() -> Rig;
    let builds: [(&str, Build); 2] = [
        ("no body", || Rig::new(&[Seed::new(P, 1, 1)])),
        ("behind", behind),
    ];
    for (what, build) in builds {
        let cases = reset_first(build);
        for message in failed_before_the_act(&cases, 3, "pane-not-driven") {
            assert!(message.contains(P), "{what}: {message:?}");
        }
        for case in &cases {
            assert_refused_in_the_plan(case, &case.setup);
        }
    }
}

/// Acceptance 7: a healthy pane's first message is delivered through the hub's prompt path to the
/// session the reset created, and the harness port sends nothing.
#[test]
fn reset_first_on_a_healthy_pane_is_sent_through_the_hub() {
    let cases = reset_first(|| driven_with(|_| {}));
    data(&cases);
    for case in &cases {
        let new = case.record(P).session_of_record.expect("a new session");
        let sent = case.rig.hub.prompts();
        assert_eq!(sent.len(), 1, "{:?}: {sent:?}", case.format);
        assert_eq!(
            (sent[0].id.as_str(), sent[0].text.as_str()),
            (new.as_str(), TEXT)
        );
        assert!(sent[0].of_record, "sent after the record write");
        assert!(
            case.setup.prompts().is_empty(),
            "no HarnessPort::send_prompt"
        );
        let hub = &case.calls.hub;
        let want = [
            HubOp::Advertised,
            HubOp::Prompt,
            HubOp::Rebind,
            HubOp::Prompt,
        ];
        assert_eq!(
            hub, &want,
            "{:?}: check, act, rebind, record, send",
            case.format
        );
    }
}

/// Acceptance 7, the race D5 accepts: the check admits, and the hub refuses the send (the pane
/// was parked in between). The reset has completed: exit 3 with the refusal's code, the record
/// stands, and the message says the first message did not land, with no reconcile step.
#[test]
fn a_send_refused_after_the_check_reports_the_reset_and_the_lost_message() {
    let cases = reset_first(|| {
        let rig = driven_with(|_| {});
        rig.hub.fail_send(PaneError::Refused {
            code: holler_pane::gate::PANE_PARKED,
            message: format!("pane {P} is parked, so it refuses prompts (pane-parked)"),
        });
        rig
    });
    for message in failed(&cases, 3, "pane-parked") {
        assert!(
            message.contains("first message did not land"),
            "{message:?}"
        );
        assert!(!message.contains("to reconcile"), "{message:?}");
    }
    for case in &cases {
        let after = case.record(P);
        let new = after.session_of_record.clone().expect("the reset stands");
        assert_ne!(new, case.rig.live(P).session);
        assert_eq!(after.last_observed.driven.as_deref(), Some(new.as_str()));
        assert!(case.rig.hub.prompts().is_empty());
    }
}

/// Acceptance 12 on the fakes (D4): the hold key is the body session's name, which a rebind does
/// not change. A hold stops prompts, not a rebind: `reset` re-points the held body, the next
/// `reset --first` is still `session-held`, and after the release the first message is delivered.
#[test]
fn a_hold_carries_across_a_reset_and_stops_only_the_prompt() {
    let rig = driven_with(|_| {});
    let name = body_name(P);
    rig.hub.hold(&name, HoldKind::Operator);
    let plain = rig.verb(&["pane", "reset", P], holler_cli::output::Format::Json);
    assert_eq!(plain.code, 0, "a hold does not stop the rebind: {plain:?}");
    let first = rig
        .record(P)
        .session_of_record
        .expect("the first reset's session");
    assert_eq!(
        advertised(&rig, &name).harness_session_id,
        first,
        "re-pointed, same name"
    );

    let held = rig.verb(
        &["pane", "reset", P, "--first", TEXT],
        holler_cli::output::Format::Json,
    );
    assert_eq!(held.code, 3, "{held:?}");
    assert!(held.out.contains("session-held"), "{held:?}");
    assert_eq!(
        rig.record(P).session_of_record.as_deref(),
        Some(first.as_str()),
        "nothing moved"
    );

    rig.hub.release(&name);
    let delivered = rig.verb(
        &["pane", "reset", P, "--first", TEXT],
        holler_cli::output::Format::Json,
    );
    assert_eq!(delivered.code, 0, "{delivered:?}");
    assert_eq!(rig.hub.prompts().len(), 1);
    assert_eq!(advertised(&rig, &name).name, name, "the key never changed");
}
