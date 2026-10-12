//! DRIVEN follows each pane's session of record (#839): the verbs over the test kit's fakes and
//! `FakeHub`, the hub's live attach bodies.
//!
//! - [`order`]: switch and reset re-point the pane's body after the observation and record what
//!   it acknowledged (Acceptance 1, 2 and 8).
//! - [`first`]: `reset --first` is checked at the hub before the act and sent through it after the
//!   record (Acceptance 7 and 12).
//! - [`evidence`]: a body is a pane's only on evidence, and a switch onto a session another body
//!   drives is refused (Acceptance 20).
//! - [`doctor`]: `pane doctor` records DRIVEN and reports a body that is behind, and the gate
//!   then sees it (Acceptance 6 and 19).
//!
//! This file holds the shared helpers and the in-process checks of the routed verbs' contract:
//! the `ses_` fixture (Acceptance 15), the one predicate (14), and the hub-refusal mapping, the
//! routed target and the `Send` call (18). Every body here advertises a `ses_`-shaped id minted
//! by the fake harness (D8): a body named by a name-shaped string would prove nothing.

mod doctor;
mod evidence;
mod first;
mod order;

use holler_cli::pane::args::ProfileOpt;
use holler_cli::pane::hub_refusal::{classify, HubRefusal};
use holler_cli::pane::wiring::hub::{send_call, SEND_TURN_BUDGET};
use holler_cli::prompt_target::{PromptArgs, PromptTarget};
use holler_cli::say_cmd::{resolve_pane_target, route_target, QUEUE_ACCEPT_WAIT};
use holler_pane::gate::prompt_refusal;
use holler_pane::hub::Advertised;
use holler_pane::pane::{Health, Hold};
use holler_pane::ports::Activity;
use holler_pane_testkit::fixture::session_id;
use holler_proto::{Code, RoutableName, WireError};

use crate::doctor::rig::{Rig, Seed};
use crate::switch::P;

/// The roster name of the body that drives `pane` on these rigs: `kiwi/<pane>`.
pub(crate) fn body_name(pane: &str) -> String {
    format!("kiwi/{pane}")
}

/// A body that drives `pane`: it advertises the pane's live session, as the attach body the
/// cutover starts for each pane does.
pub(crate) fn drive(rig: &Rig, pane: &str) {
    body(
        rig,
        &body_name(pane),
        &rig.live(pane).session,
        Activity::Idle,
    );
}

/// A live attach body `name` advertising `id`, in `state`.
pub(crate) fn body(rig: &Rig, name: &str, id: &str, state: Activity) {
    rig.hub
        .with_body(&format!("tok_{}", name.replace('/', "_")), name, id, state);
}

/// The live body named `name`, as the hub advertises it now.
pub(crate) fn advertised(rig: &Rig, name: &str) -> Advertised {
    rig.hub
        .bodies()
        .into_iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("no body {name}: {:?}", rig.hub.bodies()))
}

// --- Acceptance 15: the fixture ---------------------------------------------------------

/// D8: a hub-routed fixture is `ses_` and 26 characters, the shape the fake harness mints and
/// OpenCode's ids have, and never a valid session name, so a fixture cannot pass by being one.
#[test]
fn the_session_id_fixture_is_never_a_name() {
    for n in [0, 1, 0xabc, u32::MAX] {
        let id = session_id(n);
        assert_eq!(id.len(), 30, "{id}");
        assert!(id.starts_with("ses_"), "{id}");
        assert!(
            RoutableName::parse(&id).is_err(),
            "{id} must not parse as a name"
        );
    }
    assert_ne!(session_id(1), session_id(2));
    let rig = Rig::new(&[Seed::new(P, 1, 1)]);
    let minted = &rig.live(P).session;
    assert_eq!(
        minted.len(),
        30,
        "the fake harness mints the same shape: {minted}"
    );
}

// --- Acceptance 14: one predicate --------------------------------------------------------

/// The routed verbs refuse a pane by `holler_pane::gate::prompt_refusal`, the predicate the
/// hub's gate calls (its side is `holler-hub`'s `panes/gate/tests.rs`): the same records give
/// the same code.
#[test]
fn the_routed_verbs_refuse_by_the_one_predicate() {
    type Change = fn(&mut holler_pane::Pane);
    let changes: [(&str, Change); 3] = [
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
            p.last_observed.driven = Some(session_id(7));
        }),
    ];
    for (code, change) in changes {
        let rig = Rig::new(&[Seed::new(P, 1, 1)]);
        rig.rewrite(P, |p| {
            p.last_observed.shown = p.session_of_record.clone();
            change(p);
        });
        let record = rig.record(P);
        let predicate = prompt_refusal(&record).expect("the predicate refuses");
        assert_eq!(predicate.code().as_str(), code);
        let routed = resolve_pane_target(rig.ports(), P, None).expect_err("routing refuses");
        assert_eq!(
            routed.code().to_string(),
            code,
            "the same code as the predicate"
        );
    }
}

// --- Acceptance 18: the routed verbs' contract, in process ------------------------------

fn wire(code: Code, reason: Option<&'static str>) -> WireError {
    WireError::new(code, "the hub's text", reason)
}

fn held(kind: &str, reason: &'static str) -> WireError {
    WireError::new(Code::SessionHeld, "held", Some(reason)).with_hold_kind(kind)
}

/// One classification of a hub refusal (D5): `-32003` plain or `ambiguous` and the offline
/// `-32004` are `NotDriven`; a bare `-32004` (a turn that outlived its wait, a cancel not
/// confirmed), another reason on it, and an invalid grant are `Other`.
#[test]
fn a_hub_refusal_is_not_driven_only_for_an_unknown_id_or_an_offline_body() {
    let not_driven = [
        wire(Code::UnknownSession, None),
        wire(Code::UnknownSession, Some("ambiguous")),
        WireError::offline("kiwi/demo-c1r1's body is not connected"),
    ];
    for error in not_driven {
        assert!(
            matches!(classify(&error), HubRefusal::NotDriven(_)),
            "{error:?}"
        );
    }
    let other = [
        wire(Code::NotConnected, None),
        wire(Code::NotConnected, Some("endpoint did not answer")),
        WireError::invalid_grant("used"),
    ];
    for error in other {
        assert!(matches!(classify(&error), HubRefusal::Other), "{error:?}");
    }
}

/// `-32011` is `PaneGate` for a pane hold, with its code, and `Held` for an operator or default
/// hold, with its kind; `-32009` is `Busy`.
#[test]
fn a_hub_refusal_names_the_gate_the_hold_or_the_busy_session() {
    let gate = held("pane", "pane-parked");
    let classified = classify(&gate);
    assert!(
        matches!(classified, HubRefusal::PaneGate { ref code, .. } if code == "pane-parked"),
        "{classified:?}"
    );
    for want in ["operator", "default"] {
        let classified = classify(&held(want, "freeze"));
        assert!(
            matches!(classified, HubRefusal::Held { ref kind, .. } if kind == want),
            "{want}: {classified:?}"
        );
    }
    let busy = WireError::session_busy("input-required", 1, 1);
    assert!(matches!(classify(&busy), HubRefusal::Busy(_)));
}

/// A typed SESSION is a name: a bare `ses_…` is refused before any hub is contacted, with the
/// hub's own text and exit 1 (P3), and a SESSION is never routed as a pane (`pane` is `None`).
#[test]
fn a_typed_bare_id_is_unknown_and_a_session_is_no_pane() {
    let id = session_id(42);
    let args = |target: &str| PromptArgs {
        target: PromptTarget::Session(target.to_owned()),
        arg: Some("hi".to_owned()),
    };
    let stop = route_target(Ok(args(&id)), &ProfileOpt { profile: None })
        .expect_err("an id is not a SESSION");
    assert_eq!(stop.message, format!("unknown session: {id}"));
    assert_eq!(stop.exit_code, 1);

    let routed = route_target(Ok(args("kiwi/oc")), &ProfileOpt { profile: None })
        .expect("a name routes as before");
    assert_eq!(routed.session, "kiwi/oc");
    assert_eq!(routed.pane, None, "a SESSION is not a --pane target");
}

/// `reset --first`'s `Send` is built as `say_cmd::run` builds the accept-and-return call (D5,
/// F11): the hub's `timeout_ms` is the turn's budget (600 s), and only the CLI's own read wait is
/// the accept wait, so the hub never times the turn out at 2 s.
#[test]
fn the_send_call_carries_the_turn_budget_and_waits_only_for_acceptance() {
    assert_eq!(SEND_TURN_BUDGET.as_secs(), 600);
    let id = session_id(3);
    let call = send_call(&id, "ship the fix");
    assert_eq!(call.method, "control/say");
    assert_eq!(
        call.timeout, QUEUE_ACCEPT_WAIT,
        "the CLI waits for acceptance only"
    );
    let params = call.params.expect("params");
    assert_eq!(params["session"], id.as_str());
    assert_eq!(params["text"], "ship the fix");
    assert_eq!(params["queue"], true);
    assert_eq!(
        params["timeout_ms"], 600_000,
        "the hub keeps the turn's budget"
    );
    assert!(
        params.get("check_only").is_none_or(|v| v == false),
        "a send is not a check: {params}"
    );
}
