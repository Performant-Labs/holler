#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839
//! `FakeHub`, the test kit's `HubPort` (#839, brief D10), and the `HubPort` conformance suite
//! run against it (Acceptance 19: the same suite runs against the real client over a real hub in
//! `holler-cli`'s `driven_e2e_test.rs`).
//!
//! What the fake must do so that the verbs' tests mean what the hub does:
//!
//! - `advertised` lists the scripted bodies (`with_body`), in the order they were added;
//! - `rebind` moves the matching body to the id and answers that id, or the scripted fault, and
//!   moves nothing on a fault;
//! - `prompt` resolves the id against the bodies (none or two: `pane-not-driven`), then a
//!   scripted hold (`session-held`), then the pane gate through `holler_pane::gate::first_refusal`
//!   over the store it was built over (the very selection the hub's `pane_gate` calls), then a
//!   held question (`session-busy`); `Check` sends nothing, `Send` is logged in `prompts()`;
//! - `fail_send` refuses the next `Send` only (a race between the check and the send);
//! - the call log is `faults().calls()` (`hub.advertised`, `hub.rebind`, `hub.prompt`), and each
//!   `Rebind` and `Prompt` entry of `log()` records whether a pane in the store has the call's id
//!   as its session of record at that moment (`of_record`), as `SeamHarness` records
//!   `then_of_record`;
//! - its reads of the store never enter the store's own call log (they stand for the hub's read
//!   of its own registry), so a verb test's store log shows the CLI's calls alone.

use std::sync::Arc;

use holler_pane::hub::{Advertised, HubPort, PromptMode};
use holler_pane::pane::{Health, Hold, LastObserved};
use holler_pane::ports::Activity;
use holler_pane::{Pane, PaneError, PaneStore};
use holler_pane_testkit::conformance::hub::{hub_cases, run_hub_conformance, HubFixture};
use holler_pane_testkit::fault::{Fault, PortOp};
use holler_pane_testkit::fixture::{sample_pane, session_id};
use holler_pane_testkit::hub::{FakeHub, HoldKind, HubOp};
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};

/// The case ids of the `HubPort` suite, in order.
const DOCUMENTED_CASES: [&str; 7] = [
    "advertised-names-each-live-body",
    "a-healthy-pane-is-admitted",
    "the-session-of-record-selects-a-pane",
    "the-recorded-driven-selects-a-pane",
    "two-panes-on-one-id-fail-closed",
    "the-first-bad-pane-in-name-order-is-named",
    "an-id-no-body-advertises-is-pane-not-driven",
];

fn hub() -> (FakeHub, Arc<FakePaneStore>) {
    let panes = Arc::new(FakePaneStore::new());
    (FakeHub::new(panes.clone()), panes)
}

/// A healthy pane `name` whose session of record and SHOWN are `sor` and whose recorded DRIVEN
/// is `driven`.
fn pane(name: &str, sor: &str, driven: Option<&str>) -> Pane {
    let mut pane = sample_pane(name).unwrap();
    pane.harness.health = Health::Healthy;
    pane.session_of_record = Some(sor.to_owned());
    pane.last_observed = LastObserved {
        shown: Some(sor.to_owned()),
        driven: driven.map(str::to_owned),
        at: 1,
    };
    pane
}

fn code(result: Result<(), PaneError>) -> Option<String> {
    result.err().map(|e| e.code().to_string())
}

#[test]
fn the_fake_passes_the_hub_conformance_suite() {
    let result = run_hub_conformance(|| {
        let (hub, panes) = hub();
        let ids = [session_id(1), session_id(2)];
        hub.with_body("tok_a", "kiwi/a", &ids[0], Activity::Idle);
        hub.with_body("tok_b", "kiwi/b", &ids[1], Activity::Idle);
        let store: Arc<dyn PaneStore> = panes;
        (hub, HubFixture { panes: store, ids }, ())
    });
    assert_eq!(result, Ok(()));
}

#[test]
fn the_hub_suite_runs_the_documented_cases() {
    assert_eq!(hub_cases(), DOCUMENTED_CASES.to_vec());
}

#[test]
fn the_ops_name_their_port_methods() {
    assert_eq!(HubOp::Advertised.as_str(), "hub.advertised");
    assert_eq!(HubOp::Rebind.as_str(), "hub.rebind");
    assert_eq!(HubOp::Prompt.as_str(), "hub.prompt");
}

#[test]
fn advertised_lists_the_bodies_in_the_order_they_were_added() {
    let (hub, _) = hub();
    assert_eq!(hub.advertised().unwrap(), Vec::<Advertised>::new());
    hub.with_body("tok_b", "kiwi/b", &session_id(2), Activity::Busy);
    hub.with_body("tok_a", "kiwi/a", &session_id(1), Activity::Idle);
    let names: Vec<(String, String, Activity)> = hub
        .advertised()
        .unwrap()
        .into_iter()
        .map(|b| (b.name, b.harness_session_id, b.state))
        .collect();
    assert_eq!(
        names,
        [
            ("kiwi/b".to_owned(), session_id(2), Activity::Busy),
            ("kiwi/a".to_owned(), session_id(1), Activity::Idle),
        ]
    );
    assert_eq!(hub.faults().calls(), [HubOp::Advertised, HubOp::Advertised]);
}

#[test]
fn rebind_moves_the_body_and_answers_the_id() {
    let (hub, _) = hub();
    hub.with_body("tok_a", "kiwi/a", &session_id(1), Activity::Idle);
    let driver = hub.advertised().unwrap().remove(0);
    assert_eq!(hub.rebind(&driver, &session_id(9)).unwrap(), session_id(9));
    assert_eq!(hub.bodies()[0].harness_session_id, session_id(9));
    assert_eq!(hub.bodies()[0].name, "kiwi/a", "the name does not change");
}

#[test]
fn a_faulted_rebind_moves_nothing() {
    let (hub, _) = hub();
    hub.with_body("tok_a", "kiwi/a", &session_id(1), Activity::Idle);
    let driver = hub.advertised().unwrap().remove(0);
    let busy = PaneError::Unavailable {
        what: "the body refused: session busy".to_owned(),
    };
    hub.faults().fail_next(HubOp::Rebind, busy.clone());
    assert_eq!(hub.rebind(&driver, &session_id(9)), Err(busy));
    assert_eq!(hub.bodies()[0].harness_session_id, session_id(1));
    hub.faults().set(Some(Fault::Wedged));
    assert_eq!(
        hub.rebind(&driver, &session_id(9)),
        Err(PaneError::Timeout {
            op: "hub.rebind".to_owned()
        })
    );
}

/// `Check` judges and sends nothing; `Send` is logged; neither touches the store's call log.
#[test]
fn check_sends_nothing_and_send_is_logged() {
    let (hub, panes) = hub();
    let id = session_id(1);
    hub.with_body("tok_a", "kiwi/a", &id, Activity::Idle);
    panes
        .cas_put(&pane("demo-c1r1", &id, Some(&id)), 0)
        .unwrap();
    let store_calls = panes.faults().calls().len();
    assert_eq!(hub.prompt(&id, "hi", PromptMode::Check), Ok(()));
    assert!(hub.prompts().is_empty(), "a check sends nothing");
    assert_eq!(hub.prompt(&id, "hi", PromptMode::Send), Ok(()));
    let sent = hub.prompts();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        (sent[0].id.as_str(), sent[0].text.as_str()),
        (id.as_str(), "hi")
    );
    assert!(
        sent[0].of_record,
        "the pane records the id as its session of record"
    );
    assert_eq!(
        panes.faults().calls().len(),
        store_calls,
        "the fake's own reads of the store are not the CLI's: {:?}",
        panes.faults().calls()
    );
    assert!(!panes.faults().calls().contains(&PaneStoreOp::List));
}

#[test]
fn a_prompt_to_an_id_no_body_or_two_bodies_advertise_is_not_driven() {
    let (hub, _) = hub();
    assert_eq!(
        code(hub.prompt(&session_id(1), "hi", PromptMode::Check)).as_deref(),
        Some("pane-not-driven")
    );
    hub.with_body("tok_a", "kiwi/a", &session_id(1), Activity::Idle);
    hub.with_body("tok_b", "lime/a", &session_id(1), Activity::Idle);
    assert_eq!(
        code(hub.prompt(&session_id(1), "hi", PromptMode::Send)).as_deref(),
        Some("pane-not-driven")
    );
    assert!(hub.prompts().is_empty());
}

/// An operator or default hold refuses first, `session-held`, naming `holler release`; a hold
/// stops prompts, not a rebind (D4).
#[test]
fn a_hold_refuses_prompts_and_not_a_rebind() {
    for kind in [HoldKind::Operator, HoldKind::Default] {
        let (hub, panes) = hub();
        let id = session_id(1);
        hub.with_body("tok_a", "kiwi/a", &id, Activity::Idle);
        let mut parked = pane("demo-c1r1", &id, Some(&id));
        parked.hold = Hold::Parked {
            reason: "disk full".to_owned(),
            release_when: "later".to_owned(),
            since: 5,
        };
        panes.cas_put(&parked, 0).unwrap();
        hub.hold("kiwi/a", kind);
        let refused = hub.prompt(&id, "hi", PromptMode::Check).unwrap_err();
        assert_eq!(refused.code().to_string(), "session-held", "{kind:?} first");
        assert!(
            refused.to_string().contains("holler release kiwi/a"),
            "{refused}"
        );
        let driver = hub.advertised().unwrap().remove(0);
        assert!(hub.rebind(&driver, &session_id(2)).is_ok(), "{kind:?}");
        hub.release("kiwi/a");
        assert_eq!(
            code(hub.prompt(&session_id(2), "hi", PromptMode::Check)),
            None,
            "released, and the body is on the new id"
        );
    }
}

/// The pane gate, by the selection `holler_pane::gate::first_refusal` makes: the session of
/// record and the recorded DRIVEN both select, and the pane's own code is the answer.
#[test]
fn the_pane_gate_is_the_one_selection() {
    let (hub, panes) = hub();
    let (old, new) = (session_id(1), session_id(2));
    hub.with_body("tok_a", "kiwi/a", &old, Activity::Idle);
    panes
        .cas_put(&pane("demo-c1r1", &new, Some(&old)), 0)
        .unwrap();
    let refused = hub.prompt(&old, "hi", PromptMode::Check).unwrap_err();
    assert_eq!(refused.code().to_string(), "pane-shown-driven-mismatch");
    assert!(refused.to_string().contains("demo-c1r1"), "{refused}");
}

/// A held question refuses even a queued prompt (`--queue` never overrides it); a busy body
/// admits it (the wiring sends `queue: true`).
#[test]
fn a_held_question_refuses_and_a_busy_body_admits() {
    let (hub, _) = hub();
    let id = session_id(1);
    hub.with_body("tok_a", "kiwi/a", &id, Activity::Busy);
    assert_eq!(code(hub.prompt(&id, "hi", PromptMode::Check)), None);
    hub.set_state("kiwi/a", Activity::HoldingQuestion);
    assert_eq!(
        code(hub.prompt(&id, "hi", PromptMode::Check)).as_deref(),
        Some("session-busy")
    );
}

/// `fail_send` refuses the next `Send` only: the check before it admits.
#[test]
fn fail_send_refuses_the_next_send_only() {
    let (hub, _) = hub();
    let id = session_id(1);
    hub.with_body("tok_a", "kiwi/a", &id, Activity::Idle);
    let parked = PaneError::Refused {
        code: holler_pane::gate::PANE_PARKED,
        message: "pane demo-c1r1 is parked".to_owned(),
    };
    hub.fail_send(parked.clone());
    assert_eq!(hub.prompt(&id, "hi", PromptMode::Check), Ok(()));
    assert_eq!(hub.prompt(&id, "hi", PromptMode::Send), Err(parked));
    assert!(hub.prompts().is_empty());
    assert_eq!(hub.prompt(&id, "hi", PromptMode::Send), Ok(()));
    assert_eq!(hub.prompts().len(), 1);
}

/// Each `Rebind` and `Prompt` entry records whether a pane has the call's id as its session of
/// record at that moment: how a verb test sees a call on this fake against a write on another.
#[test]
fn the_log_records_whether_the_id_was_of_record_at_the_call() {
    let (hub, panes) = hub();
    let (old, new) = (session_id(1), session_id(2));
    hub.with_body("tok_a", "kiwi/a", &old, Activity::Idle);
    let stored = panes
        .cas_put(&pane("demo-c1r1", &old, Some(&old)), 0)
        .unwrap();
    let driver = hub.advertised().unwrap().remove(0);
    hub.rebind(&driver, &new).unwrap();
    let mut moved = pane("demo-c1r1", &new, Some(&new));
    moved.generation = stored.generation;
    panes.cas_put(&moved, stored.generation).unwrap();
    hub.prompt(&new, "hi", PromptMode::Send).unwrap();
    let log: Vec<(HubOp, String, Option<PromptMode>, bool)> = hub
        .log()
        .into_iter()
        .map(|c| (c.op, c.id, c.mode, c.of_record))
        .collect();
    assert_eq!(
        log,
        [
            (HubOp::Rebind, new.clone(), None, false),
            (HubOp::Prompt, new, Some(PromptMode::Send), true),
        ]
    );
}
