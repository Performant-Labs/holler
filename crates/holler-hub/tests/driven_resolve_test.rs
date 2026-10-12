#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839
//! The hub's side of DRIVEN (#839): the advertised `harness_session_id` is an address
//! (`Registry::resolve_prompt_target`, D1; Acceptance 5), `control/rebind` re-points a body and
//! updates the cache and the roster before it replies (D2; Acceptance 9's hub side), the
//! rebind rides its own forward and never the generic one (P1; Acceptance 13), and one forwarded
//! request is in flight per connection, inside each caller's own bound (Acceptance 21).
//!
//! The hub runs in process over its real control socket, with fake bodies on the registry's
//! command channels (`driven_support`). Ids are `ses_`-shaped (D8).

mod driven_support;

use std::time::{Duration, Instant};

use driven_support::{attach_ad, code_and_reason, hub, id, QueryAnswer, Script, Seen};
use holler_hub::control::ControlCall;
use holler_hub::control_rebind::REBIND_FORWARD;
use holler_hub::live::ResolveOutcome;
use holler_proto::{SessionState, WireError};
use serde_json::{json, Value};

/// `(label, session)` of a resolution that must have found one session.
fn found(outcome: ResolveOutcome) -> (String, String) {
    match outcome {
        ResolveOutcome::Found(handle, ad) => (handle.hostname, ad.name),
        ResolveOutcome::Unknown => panic!("unknown"),
        ResolveOutcome::NotConnected => panic!("not connected"),
        ResolveOutcome::Ambiguous(candidates) => panic!("ambiguous: {candidates:?}"),
    }
}

// --- Acceptance 5: resolution ------------------------------------------------------------

#[tokio::test]
async fn an_id_resolves_to_the_one_live_attach_session_and_a_name_as_before() {
    let hub = hub();
    let a = id(1);
    hub.body("kiwi", vec![attach_ad("oc", &a)], Script::default())
        .await;
    let pair = ("kiwi".to_owned(), "oc".to_owned());
    assert_eq!(
        found(hub.registry.resolve_prompt_target(&a).await),
        pair,
        "by id"
    );
    assert_eq!(
        found(hub.registry.resolve_prompt_target("kiwi/oc").await),
        pair,
        "by name"
    );
    assert_eq!(
        found(hub.registry.resolve_prompt_target("oc").await),
        pair,
        "by bare name"
    );
    assert!(matches!(
        hub.registry.resolve_prompt_target(&id(2)).await,
        ResolveOutcome::Unknown
    ));
}

#[tokio::test]
async fn an_id_on_an_offline_body_is_not_connected_and_two_bodies_are_ambiguous() {
    let hub = hub();
    let gone = hub
        .body("kiwi", vec![attach_ad("oc", &id(3))], Script::default())
        .await;
    hub.disconnect(&gone).await;
    assert!(matches!(
        hub.registry.resolve_prompt_target(&id(3)).await,
        ResolveOutcome::NotConnected
    ));

    hub.body("lime", vec![attach_ad("oc", &id(4))], Script::default())
        .await;
    hub.body("plum", vec![attach_ad("other", &id(4))], Script::default())
        .await;
    match hub.registry.resolve_prompt_target(&id(4)).await {
        ResolveOutcome::Ambiguous(mut candidates) => {
            candidates.sort();
            assert_eq!(candidates, ["lime/oc", "plum/other"]);
        }
        _ => panic!("two bodies on one id are ambiguous"),
    }
}

/// An id that is also a valid name resolves as a name, never by id; a spawn session's row (no
/// advertised id) is never matched by one.
#[tokio::test]
async fn a_name_shaped_id_never_resolves_by_id_and_a_spawn_row_is_never_matched() {
    let hub = hub();
    hub.body(
        "kiwi",
        vec![attach_ad("oc", "openc-abc")],
        Script::default(),
    )
    .await;
    assert!(matches!(
        hub.registry.resolve_prompt_target("openc-abc").await,
        ResolveOutcome::Unknown
    ));
    let mut spawn = attach_ad("sp", &id(5));
    spawn.mode = holler_proto::Mode::Spawn;
    hub.body("lime", vec![spawn], Script::default()).await;
    assert!(matches!(
        hub.registry.resolve_prompt_target(&id(5)).await,
        ResolveOutcome::Unknown
    ));
}

/// `control/say`, `control/interrupt` and `control/answer` take the id as their `session` (a
/// client of the control socket may send one; the `holler` CLI does only for `--pane`), and each
/// reaches the body session by its name.
#[tokio::test]
async fn say_interrupt_and_answer_accept_an_id() {
    let hub = hub();
    let a = id(6);
    let body = hub
        .body("kiwi", vec![attach_ad("oc", &a)], Script::default())
        .await;
    let say = hub
        .call(
            "control/say",
            json!({"session": a, "text": "hi", "timeout_ms": 5000}),
        )
        .await;
    assert_eq!(say["result"]["session"], "kiwi/oc", "{say}");
    let interrupt = hub.call("control/interrupt", json!({"session": a})).await;
    assert_eq!(interrupt["result"]["applied"], true, "{interrupt}");
    let answer = hub
        .call("control/answer", json!({"session": a, "choice": "0"}))
        .await;
    assert_eq!(answer["result"]["applied"], true, "{answer}");
    let seen = body.seen();
    assert_eq!(
        seen[0],
        Seen::Say {
            session: "oc".to_owned(),
            replace: false
        }
    );
    assert_eq!(
        seen[1],
        Seen::Cancel {
            session: "oc".to_owned()
        }
    );
    assert!(
        matches!(&seen[2], Seen::Query { method, .. } if method == "session/answer"),
        "{seen:?}"
    );
}

// --- Acceptance 9, the hub: control/rebind ----------------------------------------------

fn rebind(token_id: &str, session: &str, to: &str) -> Value {
    json!({"token_id": token_id, "session": session, "harness_session_id": to})
}

/// The row of `name` in `control/roster`.
async fn row(hub: &driven_support::TestHub, name: &str) -> Value {
    let roster = hub.call("control/roster", json!({"all": true})).await;
    let rows = roster["result"]["rows"]
        .as_array()
        .unwrap_or_else(|| panic!("rows: {roster}"));
    rows.iter()
        .find(|r| r["name"] == name)
        .cloned()
        .unwrap_or_else(|| panic!("no row {name}: {roster}"))
}

/// The body is asked through the rebind's own forward (`session/rebind`, the body's bare session
/// name); before the reply the hub's cache and roster name the new id, the body's other row is
/// untouched (connected, its id and state), the old id resolves to nothing, and no hold appears.
#[tokio::test]
async fn control_rebind_re_points_one_row_and_updates_the_cache_and_roster_first() {
    let hub = hub();
    hub.holds.set_join_held(vec!["kiwi/*".to_owned()]);
    let (a, b, c) = (id(7), id(8), id(9));
    let mut other = attach_ad("oc2", &b);
    other.state = SessionState::Working;
    let body = hub
        .body("kiwi", vec![attach_ad("oc", &a), other], Script::default())
        .await;
    let reply = hub
        .call("control/rebind", rebind(&body.token_id, "oc", &c))
        .await;
    assert_eq!(
        reply["result"],
        json!({"session": "oc", "harness_session_id": c}),
        "{reply}"
    );
    let asked = Seen::Query {
        method: "session/rebind".to_owned(),
        params: Some(json!({"session": "oc", "harness_session_id": c})),
    };
    assert_eq!(body.seen(), [asked]);

    assert_eq!(row(&hub, "kiwi/oc").await["harness_session_id"], c.as_str());
    let oc2 = row(&hub, "kiwi/oc2").await;
    assert_eq!(
        (
            oc2["harness_session_id"].as_str(),
            oc2["conn_state"].as_str()
        ),
        (Some(b.as_str()), Some("connected"))
    );
    assert_eq!(
        oc2["state"], "working",
        "the other row's state is untouched: {oc2}"
    );
    assert_eq!(found(hub.registry.resolve_prompt_target(&c).await).1, "oc");
    assert!(matches!(
        hub.registry.resolve_prompt_target(&a).await,
        ResolveOutcome::Unknown
    ));
    assert!(
        hub.holds.keys().is_empty(),
        "a rebind sets no hold: {:?}",
        hub.holds.keys()
    );
}

/// The body's refusal is passed through verbatim, and nothing about the row changes.
#[tokio::test]
async fn a_body_s_refusal_is_passed_through_and_changes_nothing() {
    let hub = hub();
    let (a, c) = (id(10), id(11));
    let busy = WireError::session_busy("working", 5, 6);
    let script = Script {
        query: QueryAnswer::Refuse(busy),
        ..Script::default()
    };
    let body = hub.body("kiwi", vec![attach_ad("oc", &a)], script).await;
    let reply = hub
        .call("control/rebind", rebind(&body.token_id, "oc", &c))
        .await;
    assert_eq!(code_and_reason(&reply).0, -32009, "{reply}");
    assert_eq!(
        reply.pointer("/error/data/state"),
        Some(&json!("working")),
        "{reply}"
    );
    assert_eq!(
        reply.pointer("/error/data/turn_age_ms"),
        Some(&json!(5)),
        "{reply}"
    );
    assert_eq!(row(&hub, "kiwi/oc").await["harness_session_id"], a.as_str());
    assert_eq!(found(hub.registry.resolve_prompt_target(&a).await).1, "oc");
}

/// `control/rebind` names its body by `token_id` (as `control/token_ping` does), and a session
/// that is not in that body's cached presence is refused: in neither case is a body asked.
#[tokio::test]
async fn control_rebind_takes_token_id_and_a_session_of_that_body() {
    let hub = hub();
    let body = hub
        .body("kiwi", vec![attach_ad("oc", &id(12))], Script::default())
        .await;
    let loose = hub
        .call(
            "control/rebind",
            json!({"token": body.token_id, "session": "oc", "harness_session_id": id(13)}),
        )
        .await;
    assert_eq!(
        code_and_reason(&loose).0,
        -32602,
        "`token` is not `token_id`: {loose}"
    );
    let stranger = hub
        .call("control/rebind", rebind(&body.token_id, "nope", &id(13)))
        .await;
    assert_eq!(
        code_and_reason(&stranger).0,
        -32003,
        "not a session of that body: {stranger}"
    );
    let nobody = hub
        .call("control/rebind", rebind("tok_nobody", "oc", &id(13)))
        .await;
    assert!(
        nobody.get("error").is_some(),
        "no live body for the token: {nobody}"
    );
    assert!(
        body.seen().is_empty(),
        "no body was asked: {:?}",
        body.seen()
    );
}

// --- Acceptance 13, the hub: the generic forward stays closed ---------------------------

/// `session/rebind` is not on the generic forward's allowlist: `control/query_remote` refuses it
/// `-32601`, as `LiveHandle::query` does, and the body is never asked.
#[tokio::test]
async fn the_generic_forward_refuses_session_rebind() {
    let hub = hub();
    let body = hub
        .body("kiwi", vec![attach_ad("oc", &id(14))], Script::default())
        .await;
    let params = json!({"session": "oc", "harness_session_id": id(15)});
    let remote = hub
        .call(
            "control/query_remote",
            json!({"target": "kiwi", "method": "session/rebind", "params": params}),
        )
        .await;
    assert_eq!(code_and_reason(&remote).0, -32601, "{remote}");
    let handle = hub.registry.find_by_token(&body.token_id).await.unwrap();
    let direct = handle
        .query("session/rebind", Some(params), Duration::from_secs(1))
        .await;
    assert!(
        matches!(direct, Some(Err(ref e)) if e.code == -32601),
        "the allowlist is unchanged"
    );
    assert!(body.seen().is_empty());
}

// --- Acceptance 21: one request at a time per connection --------------------------------

/// A rebind and several `hub query` requests issued at once on one body each get their own reply
/// (none is reported as not connected), the rebind applies, and the body never holds two of them
/// unanswered at once: the hub serialises them on the one reply slot.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_forwards_on_one_body_each_get_their_own_reply() {
    let hub = hub();
    let script = Script {
        query: QueryAnswer::Slow(Duration::from_millis(100)),
        ..Script::default()
    };
    let body = hub
        .body("kiwi", vec![attach_ad("oc", &id(16))], script)
        .await;
    let query = json!({"target": "kiwi", "method": "query/status"});
    let (re, q1, q2, q3) = tokio::join!(
        hub.call("control/rebind", rebind(&body.token_id, "oc", &id(17))),
        hub.call("control/query_remote", query.clone()),
        hub.call("control/query_remote", query.clone()),
        hub.call("control/query_remote", query),
    );
    assert_eq!(re["result"]["harness_session_id"], id(17).as_str(), "{re}");
    for q in [q1, q2, q3] {
        assert!(
            q.get("result").is_some(),
            "each query has its own reply: {q}"
        );
    }
    assert_eq!(
        body.most_outstanding(),
        1,
        "one forwarded request in flight at a time"
    );
    assert_eq!(
        found(hub.registry.resolve_prompt_target(&id(17)).await).1,
        "oc"
    );
}

/// Wait, bounded, until `body` has seen `n` commands: the forward holding the slot has reached it.
async fn until_seen(body: &driven_support::FakeBody, n: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while body.seen().len() < n {
        assert!(Instant::now() < deadline, "the body saw {:?}", body.seen());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// Each caller's bound includes the wait for the slot: a rebind behind a query that never
/// answers times out at its own bound, and a query behind a rebind that never answers returns
/// `None` at its own `timeout`, not one forward later.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_caller_waiting_for_the_slot_times_out_at_its_own_bound() {
    let hub = hub();
    let script = Script {
        query: QueryAnswer::Never,
        ..Script::default()
    };
    let body = hub
        .body("kiwi", vec![attach_ad("oc", &id(18))], script)
        .await;
    let handle = hub.registry.find_by_token(&body.token_id).await.unwrap();
    let params = json!({"session": "oc", "harness_session_id": id(19)});

    let holder = handle.clone();
    let held = tokio::spawn(async move {
        holder
            .query("query/status", None, Duration::from_secs(30))
            .await
    });
    until_seen(&body, 1).await;
    let started = Instant::now();
    let rebound = handle
        .rebind(params.clone(), Duration::from_millis(300))
        .await;
    assert!(rebound.is_none(), "no answer within the rebind's own bound");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    held.abort();

    let holder = handle.clone();
    let held = tokio::spawn(async move { holder.rebind(params, Duration::from_secs(30)).await });
    until_seen(&body, 2).await;
    let started = Instant::now();
    assert!(handle
        .query("query/status", None, Duration::from_millis(300))
        .await
        .is_none());
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    held.abort();
}

/// The bounds nest: the hub's rebind forward is 8 s, strictly below the CLI's 10 s call, so the
/// CLI always sees the hub's answer (the body's 6 s wait is pinned in `holler-cli`).
#[test]
fn the_hub_s_rebind_bound_is_below_the_cli_s() {
    assert_eq!(REBIND_FORWARD, Duration::from_secs(8));
    let call = ControlCall::rebind("tok_kiwi", "oc", &id(20));
    assert_eq!(call.method, "control/rebind");
    assert_eq!(call.timeout, Duration::from_secs(10));
    assert!(REBIND_FORWARD < call.timeout);
    assert_eq!(
        call.params,
        Some(json!({"token_id": "tok_kiwi", "session": "oc", "harness_session_id": id(20)}))
    );
}
