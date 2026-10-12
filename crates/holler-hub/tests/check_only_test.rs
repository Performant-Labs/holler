#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839
//! `control/say` with `check_only` (#839, D5; Acceptance 10): the question `reset --first` asks
//! before the act. It runs `talk::admit`, the very prelude `say` runs (resolve, the hold fast
//! path, the busy check, and now the pane gate on the advertised id), and stops: no frame reaches
//! the body, no TalkLog line is written, no turn id moves, no grant is consumed. `say` and the
//! check refuse the same cases with the same errors, and `say` refuses a pane-gated prompt before
//! any TalkLog line is written or the turn id moves.

mod driven_support;

use std::time::Duration;

use driven_support::{attach_ad, error_of, hub, id, pane, state_dir, Script, TestHub};
use holler_hub::state::{talklog_path, HubState};
use holler_pane::pane::{Health, Hold};
use holler_proto::SessionState;
use serde_json::{json, Value};

fn say(session: &str, queue: bool, check_only: bool) -> Value {
    json!({"session": session, "text": "hi", "queue": queue, "timeout_ms": 5000, "check_only": check_only})
}

fn talklog_exists(label: &str, session: &str) -> bool {
    talklog_path(
        &HubState::from_root(state_dir().to_path_buf()),
        label,
        session,
    )
    .exists()
}

/// The error a reply carries, without its message (holds and refusals word theirs alike).
fn error_shape(reply: &Value) -> Value {
    let error = error_of(reply);
    json!({"code": error["code"], "data": error["data"]})
}

/// An admitted check answers `{admitted: true}` and changes nothing anywhere.
#[tokio::test]
async fn an_admitted_check_sends_logs_and_moves_nothing() {
    let hub = hub();
    let a = id(1);
    hub.put_pane(&pane("demo-c1r1", &a, Some(&a)));
    let body = hub
        .body("kiwi", vec![attach_ad("oc", &a)], Script::default())
        .await;
    let reply = hub.call("control/say", say("kiwi/oc", true, true)).await;
    assert_eq!(reply["result"], json!({"admitted": true}), "{reply}");
    assert!(
        body.seen().is_empty(),
        "no frame reached the body: {:?}",
        body.seen()
    );
    assert!(!talklog_exists("kiwi", "oc"), "no TalkLog line");
    assert_eq!(
        hub.roster.find_by_name("kiwi/oc").unwrap().turn_id,
        None,
        "no turn id moved"
    );
}

/// A check consumes no grant: a default hold and a one-time grant admit the check, and the grant
/// is still there for the prompt it was minted for.
#[tokio::test]
async fn a_check_consumes_no_grant() {
    let hub = hub();
    let a = id(2);
    hub.body("kiwi", vec![attach_ad("oc", &a)], Script::default())
        .await;
    hub.holds.set_join_held(vec!["kiwi/*".to_owned()]);
    hub.holds.note_joined(["kiwi/oc".to_owned()]);
    let grant = hub
        .holds
        .mint_grant("kiwi/oc", Duration::from_secs(60))
        .unwrap()
        .grant;
    let mut params = say("kiwi/oc", true, true);
    params["grant"] = json!(grant);
    let reply = hub.call("control/say", params).await;
    assert_eq!(reply["result"], json!({"admitted": true}), "{reply}");
    assert_eq!(
        hub.holds.live_grants("kiwi/oc"),
        1,
        "the grant is not spent"
    );
}

/// One case of the table: what makes the session refuse, and whether the prompt is queued.
struct Case {
    what: &'static str,
    queue: bool,
    set: fn(&TestHub, &str),
    state: SessionState,
}

fn parked(hub: &TestHub, a: &str) {
    let mut p = pane("demo-c1r1", a, Some(a));
    p.hold = Hold::Parked {
        reason: "disk full".to_owned(),
        release_when: "later".to_owned(),
        since: 5,
    };
    hub.put_pane(&p);
}

fn unhealthy(hub: &TestHub, a: &str) {
    let mut p = pane("demo-c1r1", a, Some(a));
    p.harness.health = Health::Unhealthy("server-down".to_owned());
    hub.put_pane(&p);
}

fn mismatched(hub: &TestHub, a: &str) {
    hub.put_pane(&pane("demo-c1r1", a, Some(&id(99))));
}

fn operator_hold(hub: &TestHub, _: &str) {
    hub.holds.hold("kiwi/oc", Some("freeze"));
}

fn default_hold(hub: &TestHub, _: &str) {
    hub.holds.set_join_held(vec!["kiwi/*".to_owned()]);
    hub.holds.note_joined(["kiwi/oc".to_owned()]);
}

fn nothing(_: &TestHub, _: &str) {}

const CASES: [Case; 7] = [
    Case {
        what: "parked",
        queue: true,
        set: parked,
        state: SessionState::Idle,
    },
    Case {
        what: "unhealthy",
        queue: true,
        set: unhealthy,
        state: SessionState::Idle,
    },
    Case {
        what: "SHOWN differs from DRIVEN",
        queue: true,
        set: mismatched,
        state: SessionState::Idle,
    },
    Case {
        what: "an operator hold",
        queue: true,
        set: operator_hold,
        state: SessionState::Idle,
    },
    Case {
        what: "a default hold",
        queue: true,
        set: default_hold,
        state: SessionState::Idle,
    },
    Case {
        what: "a held question",
        queue: true,
        set: nothing,
        state: SessionState::InputRequired,
    },
    Case {
        what: "busy, not queued",
        queue: false,
        set: nothing,
        state: SessionState::Working,
    },
];

/// The one prelude: for each case, `say` and the check refuse alike (code and data), and neither
/// reaches the body, writes a TalkLog line or moves the turn id. (No test of this binary admits a
/// `say`, so no TalkLog file of `kiwi/oc` exists in its state dir unless one of these wrote it.)
#[tokio::test]
async fn say_and_the_check_refuse_the_same_cases_alike_before_anything_moves() {
    for case in CASES {
        let mut answers = Vec::new();
        for check_only in [false, true] {
            let hub = hub();
            let a = id(3);
            (case.set)(&hub, &a);
            let mut ad = attach_ad("oc", &a);
            ad.state = case.state;
            let body = hub.body("kiwi", vec![ad], Script::default()).await;
            let reply = hub
                .call("control/say", say("kiwi/oc", case.queue, check_only))
                .await;
            assert!(
                reply.get("error").is_some(),
                "{} (check_only {check_only}) is refused: {reply}",
                case.what
            );
            answers.push(error_shape(&reply));
            assert!(
                body.seen().is_empty(),
                "{}: nothing reached the body",
                case.what
            );
            assert!(
                !talklog_exists("kiwi", "oc"),
                "{}: no TalkLog line",
                case.what
            );
            let row = hub.roster.find_by_name("kiwi/oc").unwrap();
            assert_eq!(row.turn_id, None, "{}: no turn id moved", case.what);
        }
        assert_eq!(
            answers[0], answers[1],
            "{}: say and the check refuse alike",
            case.what
        );
    }
}

/// A's warn 3 on the plan (D5's fast path): `interrupt` with text checks the hold before it
/// cancels anything, so a redirect the hold would refuse never costs the running turn. The pane
/// gate keyed on the advertised id is checked there too: a redirect to a body that drives a
/// parked pane is refused before the cancel, and the body sees nothing.
#[tokio::test]
async fn a_redirect_the_pane_gate_refuses_cancels_nothing() {
    let hub = hub();
    let a = id(4);
    parked(&hub, &a);
    let body = hub.body("kiwi", vec![attach_ad("oc", &a)], Script::default()).await;
    let params = json!({"session": "kiwi/oc", "text": "instead"});
    let reply = hub.call("control/interrupt", params).await;
    let error = error_of(&reply);
    assert_eq!(error["code"], -32011, "{reply}");
    assert_eq!(error.pointer("/data/hold_kind"), Some(&json!("pane")), "{reply}");
    assert!(body.seen().is_empty(), "nothing cancelled: {:?}", body.seen());
}
