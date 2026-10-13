//! The drive rule of a single process (#850, Acceptance 6; ADR-0021 section 15 item 19): a Claude
//! Code pane has a body once a `[[session]]` row attaches to its session of record, and `switch`,
//! `reset` and `doctor` find that body as they find a server pane's (#839), over an empty server
//! list. A switch or a reset is the relaunch (#831), then `hub.rebind` (the body checks the new
//! session against `claude agents --json`), then a second compare-and-swap that records the id the
//! body acknowledged as `last_observed.driven`. A pane with no body is `Driver::None`: its verbs
//! work without the drive step and DRIVEN stays as stored. `HarnessPort::send_prompt` is never
//! called: a first message goes only through the hub. Its `reset --first` with no body is in
//! `lifecycle.rs`.
//!
//! The fake hub stands for the hub's live attach bodies: `with_body` is a body that advertises the
//! pane's session of record, `rebind` moves it, and its log says, for each `rebind` and `prompt`,
//! whether a pane of the store had the call's id as its session of record at that moment (how a
//! test orders a call on the hub against a write on the store).

use holler_cli::interrupt_cmd::{interrupt_pane, PaneInterrupt};
use holler_cli::output::Format;
use holler_pane::hub::{HubPort, PromptMode};
use holler_pane::ports::Activity;
use holler_pane::{HarnessPort, PaneError};
use holler_pane_testkit::claude::ClaudeOp;
use holler_pane_testkit::hub::{HubCall, HubOp};
use holler_pane_testkit::pane_store::PaneStoreOp;

use crate::rig::{cell, data_of, error_of, findings, kinds, Calls, Rig, CLAUDE};

/// A DRIVEN a pass of the old era might have stored, which nothing may infer.
const STORED: &str = "4f1c2b8e-0000-4000-8000-00000000d1d1";
const BODY: &str = "kiwi/claude";

/// The DRIVEN a pane's record holds before a test runs.
#[derive(Clone, Copy)]
enum Stored {
    /// A stale value some earlier pass wrote.
    Stale,
    /// The session of record: what a body that drives the pane would have had recorded.
    Session,
}

/// One live, idle Claude pane with a conversation and a stored DRIVEN, and (`with_body`) a body
/// that advertises its session of record. Returns the rig and the session of record.
fn claude_pane(stored_driven: Stored, with_body: bool) -> (Rig, String) {
    let rig = Rig::new();
    rig.seed_claude(CLAUDE, cell(1, 2));
    rig.converse(CLAUDE);
    let record = rig.record(CLAUDE);
    let sor = record.session_of_record.clone().unwrap();
    let mut stored = record;
    stored.last_observed.driven = Some(match stored_driven {
        Stored::Stale => STORED.to_owned(),
        Stored::Session => sor.clone(),
    });
    rig.panes.concurrent_put(&stored).unwrap();
    if with_body {
        rig.hub.with_body("tok_x", BODY, &sor, Activity::Idle);
    }
    (rig, sor)
}

fn driven(rig: &Rig) -> Option<String> {
    rig.record(CLAUDE).last_observed.driven
}

/// The session the body advertises now.
fn body_on(rig: &Rig) -> String {
    rig.hub.bodies().remove(0).harness_session_id
}

fn cas_puts(calls: &Calls) -> usize {
    calls.panes.iter().filter(|op| **op == PaneStoreOp::CasPut).count()
}

/// A session the pane can be switched to: it has a conversation, so the relaunch resumes it.
fn another_session(rig: &Rig) -> String {
    let target = rig.claude().new_session_id().unwrap();
    rig.claude().seed_conversation(&target);
    target
}

#[test]
fn a_claude_switch_relaunches_then_rebinds_then_records_driven_in_a_second_write() {
    let (rig, sor) = claude_pane(Stored::Stale, true);
    let target = another_session(&rig);
    let old_pid = rig.record(CLAUDE).harness.pid.unwrap();

    let (run, calls) = rig.run_logged(&["pane", "switch", CLAUDE, &target], Format::Json);

    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(calls.hub, vec![HubOp::Advertised, HubOp::Rebind], "the hub is asked who drives the pane, then to follow");
    assert_eq!(
        rig.hub.log(),
        vec![HubCall { op: HubOp::Rebind, id: target.clone(), mode: None, of_record: true }],
        "the rebind came after the relaunch stored the target as the session of record"
    );
    assert_eq!(cas_puts(&calls), 2, "the relaunch stores the record, and a second write records DRIVEN: {:?}", calls.panes);
    let after = rig.record(CLAUDE);
    assert_eq!(after.session_of_record.as_deref(), Some(target.as_str()));
    assert_eq!(after.last_observed.shown.as_deref(), Some(target.as_str()));
    assert_eq!(after.last_observed.driven.as_deref(), Some(target.as_str()), "the id the body acknowledged");
    assert_ne!(after.harness.pid, Some(old_pid), "the process was relaunched");
    assert_eq!(body_on(&rig), target, "the body follows the pane");
    assert_ne!(sor, target);
    assert!(!calls.claude.contains(&ClaudeOp::SendPrompt), "the engine never delivers itself: {:?}", calls.claude);
}

#[test]
fn a_claude_reset_first_is_delivered_through_the_hub_after_the_rebind() {
    let (rig, sor) = claude_pane(Stored::Session, true);
    let (run, calls) =
        rig.run_logged(&["pane", "reset", CLAUDE, "--first", "start on the review"], Format::Json);

    assert_eq!(run.code, 0, "{run:?}");
    let fresh = rig.record(CLAUDE).session_of_record.unwrap();
    assert_ne!(fresh, sor, "a fresh session");
    assert_eq!(
        calls.hub,
        vec![HubOp::Advertised, HubOp::Prompt, HubOp::Rebind, HubOp::Prompt],
        "who drives, the gate's check, the rebind, the send"
    );
    let log = rig.hub.log();
    assert_eq!(
        (log[0].op, log[0].mode, log[0].id.as_str(), log[0].of_record),
        (HubOp::Prompt, Some(PromptMode::Check), sor.as_str(), true),
        "the check names the body's own session, then of record"
    );
    assert_eq!((log[1].op, log[1].id.as_str(), log[1].of_record), (HubOp::Rebind, fresh.as_str(), true));
    assert_eq!(
        (log[2].op, log[2].mode, log[2].id.as_str(), log[2].of_record),
        (HubOp::Prompt, Some(PromptMode::Send), fresh.as_str(), true)
    );
    let sent = rig.hub.prompts();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!((sent[0].id.as_str(), sent[0].text.as_str()), (fresh.as_str(), "start on the review"));
    assert_eq!(driven(&rig).as_deref(), Some(fresh.as_str()));
    assert!(
        rig.process_of(CLAUDE).unwrap().inbox.is_empty(),
        "the process's inbox was not written by the engine"
    );
    assert!(!calls.claude.contains(&ClaudeOp::SendPrompt));
}

/// The first rewritten test of #839's no-body era. A pane no body drives is `Driver::None`: its
/// switch and reset are the relaunch they were, the hub is asked only who is advertised, nothing
/// is rebound, and the stored DRIVEN stays as stored.
#[test]
fn a_claude_pane_with_no_body_switches_and_resets_without_a_drive_step() {
    let (rig, _) = claude_pane(Stored::Stale, false);
    let target = another_session(&rig);

    let (switched, calls) = rig.run_logged(&["pane", "switch", CLAUDE, &target], Format::Json);
    assert_eq!(switched.code, 0, "{switched:?}");
    assert_eq!(calls.hub, vec![HubOp::Advertised], "only who is advertised: {:?}", calls.hub);
    assert_eq!(driven(&rig).as_deref(), Some(STORED), "switch keeps the stored DRIVEN");

    let (reset, calls) = rig.run_logged(&["pane", "reset", CLAUDE], Format::Json);
    assert_eq!(reset.code, 0, "{reset:?}");
    assert_eq!(calls.hub, vec![HubOp::Advertised], "only who is advertised: {:?}", calls.hub);
    assert_eq!(driven(&rig).as_deref(), Some(STORED), "reset keeps the stored DRIVEN");
    assert!(rig.hub.log().is_empty(), "nothing was rebound or prompted");
}

/// The rebind fails after the relaunch moved the process and stored the record: the run is
/// reported `acted`, with the reconcile step, and the body is left on the session it had.
/// `holler pane switch PANE <session of record>` is then the remedy: for a single process it
/// relaunches the process on the session it holds first, and re-points the body.
#[test]
fn a_failed_rebind_after_the_relaunch_is_reported_acted_with_the_reconcile_step() {
    let (rig, sor) = claude_pane(Stored::Session, true);
    let target = another_session(&rig);
    rig.hub.faults().fail_next(
        HubOp::Rebind,
        PaneError::Unavailable { what: "the hub did not answer".to_owned() },
    );

    let run = rig.run(&["pane", "switch", CLAUDE, &target], Format::Json);

    assert_eq!(run.code, 1, "{run:?}");
    let (code, message) = error_of(&run);
    assert_eq!(code, "unavailable");
    assert!(
        message.contains("to reconcile, run") && message.contains("pane doctor") && message.contains("--fix"),
        "a failure after the act carries the reconcile step: {message}"
    );
    let after = rig.record(CLAUDE);
    assert_eq!(after.session_of_record.as_deref(), Some(target.as_str()), "the relaunch stored its record");
    assert_eq!(rig.process_of(CLAUDE).unwrap().session_id, target);
    assert_eq!(after.last_observed.driven.as_deref(), Some(sor.as_str()), "DRIVEN was not written");
    assert_eq!(body_on(&rig), sor, "the body is where it was");

    // The remedy: switch the pane to the session it holds. The process is relaunched on it first.
    let pid = after.harness.pid.unwrap();
    let again = rig.run(&["pane", "switch", CLAUDE, &target], Format::Json);
    assert_eq!(again.code, 0, "{again:?}");
    assert_ne!(rig.record(CLAUDE).harness.pid, Some(pid), "a single process is relaunched on the session it holds");
    assert_eq!(body_on(&rig), target);
    assert_eq!(driven(&rig).as_deref(), Some(target.as_str()));
}

/// An interrupt and a doctor repair keep the session, so the drive rule does not run for them: no
/// body is asked, nothing is rebound, and DRIVEN is not written.
#[test]
fn an_interrupt_moves_no_body_and_writes_no_driven() {
    let (rig, sor) = claude_pane(Stored::Session, true);
    let mark = rig.mark();

    let answer = rig.with_ports(|ports| interrupt_pane(ports, CLAUDE, None, None));

    assert_eq!(answer.unwrap(), PaneInterrupt::Relaunched { session: sor.clone() });
    let calls = rig.since(&mark);
    assert!(calls.hub.is_empty(), "no hub call: {:?}", calls.hub);
    assert!(rig.hub.log().is_empty());
    assert_eq!(body_on(&rig), sor);
    assert_eq!(driven(&rig).as_deref(), Some(sor.as_str()), "DRIVEN is as it was");
    assert_eq!(cas_puts(&calls), 1, "only the relaunch's own record: {:?}", calls.panes);
}

/// The second rewritten test of #839's no-body era. `pane doctor` records the id the body of a
/// single process advertises as DRIVEN (it used to leave it as stored), asking the hub once, and
/// reports a body that is behind.
#[test]
fn doctor_records_driven_for_a_claude_pane_and_reports_a_body_that_is_behind() {
    // A body on the session of record: DRIVEN is recorded, a stale stored value goes, no finding.
    let (rig, sor) = claude_pane(Stored::Stale, true);
    let (run, calls) = rig.run_logged(&["pane", "doctor"], Format::Json);
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(kinds(&findings(&data_of(&run))), Vec::<String>::new());
    assert_eq!(calls.hub, vec![HubOp::Advertised], "once per pass: {:?}", calls.hub);
    assert_eq!(driven(&rig).as_deref(), Some(sor.as_str()));
    assert!(!calls.claude.contains(&ClaudeOp::SendPrompt));

    // A person's /clear moves SHOWN: the one existing mismatch, and the body's id still recorded.
    let pid = rig.record(CLAUDE).harness.pid.unwrap();
    rig.claude().clear(pid).unwrap();
    let found = findings(&data_of(&rig.run(&["pane", "doctor"], Format::Json)));
    assert_eq!(kinds(&found), vec!["shown-driven-mismatch"], "{found:?}");
    assert_eq!(driven(&rig).as_deref(), Some(sor.as_str()));
}

/// A body that is behind: it drives a session the record no longer names, found by the stored
/// DRIVEN. It is reported `not-fixable`, with the switch back to the session of record as the
/// remedy, and what it advertises is recorded.
#[test]
fn a_body_that_drives_an_earlier_session_is_reported_behind_with_the_switch_as_its_remedy() {
    let rig = Rig::new();
    rig.seed_claude(CLAUDE, cell(1, 2));
    rig.converse(CLAUDE);
    let record = rig.record(CLAUDE);
    let sor = record.session_of_record.clone().unwrap();
    let behind = "5fad1c04-0000-4000-8000-00000000be41";
    let mut stored = record;
    stored.last_observed.driven = Some(behind.to_owned());
    rig.panes.concurrent_put(&stored).unwrap();
    rig.hub.with_body("tok_x", BODY, behind, Activity::Idle);

    let found = findings(&data_of(&rig.run(&["pane", "doctor"], Format::Json)));

    assert_eq!(kinds(&found), vec!["shown-driven-mismatch"], "{found:?}");
    assert_eq!(found[0]["fix"], "not-fixable", "{found:?}");
    assert_eq!(found[0]["remedy"], format!("holler pane switch {CLAUDE} {sor}"), "{found:?}");
    assert!(found[0]["message"].as_str().unwrap_or_default().contains("its body"), "{found:?}");
    assert_eq!(driven(&rig).as_deref(), Some(behind), "what the body advertises is recorded");
}

/// A lost second write is healed by the next doctor pass: the relaunch stored the session of
/// record, so the body now on it is the pane's by `driver_of`'s first rule, and the pass records
/// DRIVEN with no finding.
#[test]
fn a_lost_second_write_is_healed_by_the_next_doctor_pass() {
    let (rig, sor) = claude_pane(Stored::Session, true);
    let target = another_session(&rig);
    // The switch's rebind reached the body, but the DRIVEN write was lost: the same state as a
    // failed rebind, plus the body having followed.
    rig.hub.faults().fail_next(HubOp::Rebind, PaneError::Unavailable { what: "lost".to_owned() });
    let run = rig.run(&["pane", "switch", CLAUDE, &target], Format::Json);
    assert_eq!(run.code, 1, "{run:?}");
    let body = rig.hub.bodies().remove(0);
    assert_eq!(rig.hub.rebind(&body, &target).unwrap(), target, "the body follows the pane");
    assert_eq!(driven(&rig).as_deref(), Some(sor.as_str()), "the record still holds the old DRIVEN");

    let found = findings(&data_of(&rig.run(&["pane", "doctor"], Format::Json)));

    assert_eq!(kinds(&found), Vec::<String>::new(), "{found:?}");
    assert_eq!(driven(&rig).as_deref(), Some(target.as_str()), "healed");
}

/// The window between the two writes is a refusal, never a delivery to the wrong session (plan
/// review W-5): until DRIVEN is recorded, the record shows the new session while it still names the
/// old one as DRIVEN, and the hub's gate refuses a prompt to either id.
#[test]
fn a_say_between_the_two_writes_is_refused_and_not_delivered() {
    let (rig, sor) = claude_pane(Stored::Session, true);
    let target = another_session(&rig);
    rig.hub.faults().fail_next(HubOp::Rebind, PaneError::Unavailable { what: "window".to_owned() });
    let run = rig.run(&["pane", "switch", CLAUDE, &target], Format::Json);
    assert_eq!(run.code, 1, "{run:?}");

    // The body is still on the old session: a prompt to the new one reaches no body.
    let to_new = rig.hub.prompt(&target, "wrong session", PromptMode::Send);
    assert_eq!(to_new.unwrap_err().code(), "pane-not-driven");

    // The body followed, the DRIVEN write is not recorded yet: the gate sees SHOWN and DRIVEN differ.
    let body = rig.hub.bodies().remove(0);
    rig.hub.rebind(&body, &target).unwrap();
    let to_followed = rig.hub.prompt(&target, "wrong session", PromptMode::Send);
    assert_eq!(to_followed.unwrap_err().code(), "pane-shown-driven-mismatch");
    let to_old = rig.hub.prompt(&sor, "wrong session", PromptMode::Send);
    assert!(to_old.is_err(), "and nothing reaches the old id either: it is no body's now");
    assert!(rig.hub.prompts().is_empty(), "nothing was delivered: {:?}", rig.hub.prompts());
}
