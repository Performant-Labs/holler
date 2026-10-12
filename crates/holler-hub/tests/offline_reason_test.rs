#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839
//! The offline `-32004` carries `data.reason: "offline"` (#839, P7; Acceptance 18's wire half).
//!
//! `-32004 not_connected` means five things. Only the resolve-time answer of `say`, `interrupt`
//! and `answer` (the target's body is offline) is marked, through `Error::offline`: that is how a
//! `--pane` verb tells a pane no body drives (`pane-not-driven`, exit 3) from a prompt the body
//! did not answer in time. The other four stay bare, so the CLI keeps their text and exit 1:
//! `say`'s turn timeout and its cancelled turn, `interrupt`'s unacknowledged cancel and its
//! failed redirect.

mod driven_support;

use driven_support::{
    attach_ad, code_and_reason, hub, id, CancelAnswer, SayAnswer, Script, TestHub,
};
use serde_json::{json, Value};

/// A hub whose one body `kiwi` (session `oc`) was connected and is now gone.
async fn offline() -> TestHub {
    let hub = hub();
    let body = hub
        .body("kiwi", vec![attach_ad("oc", &id(1))], Script::default())
        .await;
    hub.disconnect(&body).await;
    hub
}

fn assert_offline(reply: &Value, what: &str) {
    assert_eq!(
        code_and_reason(reply),
        (-32004, Some("offline".to_owned())),
        "{what}: {reply}"
    );
}

fn assert_bare(reply: &Value, what: &str) {
    assert_eq!(
        code_and_reason(reply),
        (-32004, None),
        "{what}: no data.reason: {reply}"
    );
}

#[tokio::test]
async fn say_interrupt_and_answer_to_an_offline_body_are_marked_offline() {
    let hub = offline().await;
    let by_id = id(1);
    for session in ["kiwi/oc", by_id.as_str()] {
        let say = hub
            .call(
                "control/say",
                json!({"session": session, "text": "hi", "timeout_ms": 5000}),
            )
            .await;
        assert_offline(&say, "say");
        let interrupt = hub
            .call("control/interrupt", json!({"session": session}))
            .await;
        assert_offline(&interrupt, "interrupt");
        let answer = hub
            .call("control/answer", json!({"session": session, "choice": "0"}))
            .await;
        assert_offline(&answer, "answer");
    }
}

/// A turn that outlives `timeout_ms`, and a turn cancelled before it completed: bare `-32004`.
#[tokio::test]
async fn say_s_timeout_and_cancelled_turn_stay_bare() {
    let hub = hub();
    let never = Script {
        say: SayAnswer::Never,
        ..Script::default()
    };
    hub.body("kiwi", vec![attach_ad("oc", &id(2))], never).await;
    let timeout = hub
        .call(
            "control/say",
            json!({"session": "kiwi/oc", "text": "hi", "timeout_ms": 300}),
        )
        .await;
    assert_bare(&timeout, "say's turn timeout");
    let message = timeout
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap();
    assert!(
        message.starts_with("no reply from kiwi/oc within"),
        "{message}"
    );

    let cancelled = Script {
        say: SayAnswer::Stop("cancelled"),
        ..Script::default()
    };
    hub.body("lime", vec![attach_ad("oc", &id(3))], cancelled)
        .await;
    let reply = hub
        .call(
            "control/say",
            json!({"session": "lime/oc", "text": "hi", "timeout_ms": 5000}),
        )
        .await;
    assert_bare(&reply, "say's cancelled turn");
}

/// A cancel the body does not acknowledge (the 2 s floor of `ack_timeout`) and a redirect that
/// fails after the cancel applied: bare `-32004`.
#[tokio::test]
async fn interrupt_s_unconfirmed_cancel_and_failed_redirect_stay_bare() {
    let hub = hub();
    let deaf = Script {
        cancel: CancelAnswer::Never,
        ..Script::default()
    };
    hub.body("kiwi", vec![attach_ad("oc", &id(4))], deaf).await;
    let reply = hub
        .call("control/interrupt", json!({"session": "kiwi/oc"}))
        .await;
    assert_bare(&reply, "interrupt's unconfirmed cancel");
    let message = reply
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap();
    assert!(
        message.starts_with("interrupt sent but not confirmed within"),
        "{message}"
    );

    let cancelled = Script {
        say: SayAnswer::Stop("cancelled"),
        ..Script::default()
    };
    hub.body("lime", vec![attach_ad("oc", &id(5))], cancelled)
        .await;
    let reply = hub
        .call(
            "control/interrupt",
            json!({"session": "lime/oc", "text": "instead"}),
        )
        .await;
    assert_bare(&reply, "interrupt's failed redirect");
}
