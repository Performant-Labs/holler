#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #878
//! `OpenCodeHarness::compact` (#878, ADR-0021 "Compaction", brief A2 and A3.5) against the
//! hermetic stub: it asks `POST /session/:id/summarize` for a summary on the model the server's
//! own config pins, never by typing `/compact` into a TUI (I4), and reports the context's
//! size before and after.
//!
//! **The replies are a stand-in.** They follow the OpenCode 1.19.0 OpenAPI document (read on
//! 2026-10-12, never run): `summarize` takes `{providerID, modelID}` and answers `200`; the
//! message list's assistant messages carry `info.tokens` (`input`, `output`, `reasoning`,
//! `cache.read`, `cache.write`, the shape of the real 1.19.0 fixture under
//! `fixtures/agents/messages.json`), and the context's size is the sum of those counts of the
//! last assistant message; `GET /config` carries the pin as the restart script writes it,
//! `agent.compaction.model` (`provider/model-id`) with an optional `variant`. No real
//! compaction has run: the spike (Part B.1) confirms or corrects every one of these, and its
//! recorded replies then replace the bodies below.
//!
//! What the tests pin, in the order of the brief: the request order and body; the model rule
//! (the server's pin, split at the first `/`, or the one given; no pin, no request); a busy
//! session or an unknown one sends nothing; no reply is ever quoted in an error (a config can
//! hold a provider key); a count that cannot be read is `unavailable`, never zero; and the
//! bounds (the 120 s `compact` reaches the summarize and the token reads after it, and only
//! those).

#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/stub.rs"]
#[allow(dead_code)] // #642: shared with hermetic_test.rs
mod stub;

use std::sync::Arc;
use std::time::{Duration, Instant};

use fixtures::*;
use holler_adapter_opencode::Timeouts;
use holler_pane::ports::{CompactModel, CompactOutcome, COMPACT_MODEL_UNPINNED};
use holler_pane::{HarnessEndpoint, HarnessPort, PaneError};
use holler_pane_testkit::fault::PortOp;
use holler_pane_testkit::harness::HarnessOp;
use serde_json::{json, Value};
use stub::{on_refused_port, Stub};

/// A text placed in a reply body: an error that quotes the body carries it. It starts every
/// body it is in, so the 60-byte excerpt `Call::unexpected` quotes would hold all of it.
const MARKER: &str = "sk-zq878-reply-marker";
/// The pin the server's config holds, and the pair a call that names a model names.
const PIN: &str = "pinned-provider/pinned-model";

fn at(port: u16) -> HarnessEndpoint {
    HarnessEndpoint::Port(port)
}

fn session_path() -> String {
    format!("/session/{ID}")
}
fn summarize_path() -> String {
    format!("/session/{ID}/summarize")
}
fn messages_path() -> String {
    format!("/session/{ID}/message")
}

/// An assistant message as a real 1.19.0 server lists it, with `tokens` as given.
fn assistant(input: u64, output: u64, cache_read: u64, cache_write: u64) -> Value {
    json!({
        "info": {
            "id": "msg_a", "sessionID": ID, "role": "assistant",
            "time": {"created": 1, "completed": 2},
            "modelID": "m", "providerID": "p", "mode": "build", "agent": "build",
            "path": {"cwd": "/srv/demo", "root": "/"}, "cost": 0,
            "tokens": {
                "input": input, "output": output, "reasoning": 0,
                "cache": {"read": cache_read, "write": cache_write}
            }
        },
        "parts": []
    })
}

fn user() -> Value {
    json!({"info": {"id": "msg_u", "sessionID": ID, "role": "user", "time": {"created": 0}},
           "parts": []})
}

/// The list before the compaction: a context of 90 000 tokens.
fn before_list() -> String {
    json!([user(), assistant(70_000, 5_000, 14_000, 1_000)]).to_string()
}

/// The list after it: the summary is the last assistant message, a context of 12 000 tokens.
fn after_list() -> String {
    json!([
        user(),
        assistant(70_000, 5_000, 14_000, 1_000),
        assistant(2_000, 9_000, 500, 500)
    ])
    .to_string()
}

const BEFORE: u64 = 90_000;
const AFTER: u64 = 12_000;

/// A server config as the restart script writes it: the pin (`pin` is `provider/model-id`) and a
/// provider key, which is why no part of a config may reach an error.
fn config(pin: Option<&str>) -> String {
    let mut agent = String::new();
    if let Some(pin) = pin {
        agent = format!(r#","agent":{{"compaction":{{"model":"{pin}","variant":"high"}}}}"#);
    }
    format!(r#"{{"apiKey":"{MARKER}","provider":{{"pinned-provider":{{}}}}{agent}}}"#)
}

/// A server that holds [`ID`], is idle, pins `pin`, lists [`before_list`], and, once the
/// summarize arrives, lists [`after_list`] and answers it `200 true`.
fn server(pin: Option<&str>) -> Arc<Stub> {
    server_listing(pin, after_list())
}

/// [`server`], whose message list reads `after` once the summarize has arrived.
fn server_listing(pin: Option<&str>, after: String) -> Arc<Stub> {
    let stub = Arc::new(Stub::start());
    known_session(&stub);
    stub.json("GET", "/session/status", 200, "{}");
    stub.json("GET", "/question", 200, "[]");
    stub.json("GET", "/permission", 200, "[]");
    stub.json("GET", "/config", 200, &config(pin));
    stub.json("GET", &messages_path(), 200, &before_list());
    stub.json("POST", &summarize_path(), 200, "true");
    let later = Arc::clone(&stub);
    stub.on_nth("POST", &summarize_path(), 1, move || {
        later.json("GET", &messages_path(), 200, &after);
    });
    stub
}

/// The pinned server's compaction through a harness with the ordinary short bounds and a
/// `compact` wide enough that no case here is cut by it.
fn compact(stub: &Stub) -> Result<CompactOutcome, PaneError> {
    harness(bounds(Duration::from_secs(4))).compact(&at(stub.port), ID, None)
}

/// The quick bounds of the fixtures with `compact` set.
fn bounds(compact: Duration) -> Timeouts {
    Timeouts { compact, ..quick() }
}

/// The requests the stub saw, as `METHOD path`.
fn requests(stub: &Stub) -> Vec<String> {
    stub.lines()
        .iter()
        .map(|line| line.trim_end_matches(" HTTP/1.1").to_owned())
        .collect()
}

fn posts(stub: &Stub) -> Vec<String> {
    requests(stub)
        .into_iter()
        .filter(|line| !line.starts_with("GET "))
        .collect()
}

/// The one POST of a compaction, and its JSON body.
fn summarize_body(stub: &Stub) -> Value {
    let sent: Vec<_> = stub
        .seen()
        .into_iter()
        .filter(|seen| seen.line.starts_with("POST "))
        .collect();
    assert_eq!(sent.len(), 1, "exactly one POST: {:?}", requests(stub));
    serde_json::from_slice(&sent[0].body).expect("the summarize body is JSON")
}

/// Neither the error's message nor its debug form carries the marker (or the pin's provider
/// key, which the marker stands for).
fn assert_no_leak(error: &PaneError, what: &str) {
    let text = format!("{error} {error:?}");
    assert!(!text.contains(MARKER), "{what}: a reply was quoted: {text}");
}

// ---- the act ----

/// A pinned server, an idle session: the context before and after are the two token reads, and
/// the request is `known`, the activity reads, `GET /config`, the token read, **one** POST naming
/// the pin and nothing else (no `auto`, no `variant` though the pin has one), the token read
/// again. Nothing else is called: no TUI route, no prompt.
#[test]
fn compact_posts_one_summarize_on_the_servers_pin_and_reads_both_counts() {
    let stub = server(Some(PIN));
    assert_eq!(
        compact(&stub),
        Ok(CompactOutcome {
            tokens_before: BEFORE,
            tokens_after: AFTER
        })
    );

    assert_eq!(
        summarize_body(&stub),
        json!({"providerID": "pinned-provider", "modelID": "pinned-model"}),
        "the pin's pair and nothing else"
    );
    let seen = requests(&stub);
    let at_post = seen.iter().position(|r| r.starts_with("POST ")).unwrap();
    assert_eq!(
        seen[0],
        format!("GET {}", session_path()),
        "known comes first: {seen:?}"
    );
    assert_eq!(seen[at_post], format!("POST {}", summarize_path()));
    let config_read = seen
        .iter()
        .position(|r| r == "GET /config")
        .expect("GET /config");
    assert!(
        config_read < at_post,
        "the pin is read before the act: {seen:?}"
    );
    let message_reads = |range: &[String]| {
        range
            .iter()
            .filter(|r| **r == format!("GET {}", messages_path()))
            .count()
    };
    assert_eq!(
        message_reads(&seen[..at_post]),
        1,
        "the count before: {seen:?}"
    );
    assert_eq!(
        seen[at_post + 1..],
        [format!("GET {}", messages_path())],
        "after the act only the count after is read: {seen:?}"
    );
    let allowed = [
        format!("GET {}", session_path()),
        "GET /question".to_owned(),
        "GET /permission".to_owned(),
        "GET /session/status".to_owned(),
        "GET /config".to_owned(),
        format!("GET {}", messages_path()),
        format!("POST {}", summarize_path()),
    ];
    for request in &seen {
        assert!(
            allowed.contains(request),
            "an unexpected request {request}: {seen:?}"
        );
    }
}

/// The pin is `provider/model-id` split at the **first** `/`: a routed provider's model id may
/// hold slashes of its own.
#[test]
fn the_pin_is_split_at_the_first_slash() {
    let stub = server(Some("routed/vendor/model-1"));
    assert!(compact(&stub).is_ok());
    assert_eq!(
        summarize_body(&stub),
        json!({"providerID": "routed", "modelID": "vendor/model-1"})
    );
}

/// A call that names a model gets exactly that model, with no pin on the server and without
/// reading the server's config at all (a config can hold a key, and nothing needs it).
#[test]
fn an_explicit_model_is_sent_as_given_and_the_config_is_not_read() {
    let stub = server(None);
    let given = CompactModel {
        provider: "given-provider".to_owned(),
        model_id: "given/model".to_owned(),
    };
    let outcome = harness(bounds(Duration::from_secs(4))).compact(&at(stub.port), ID, Some(&given));
    assert_eq!(
        outcome,
        Ok(CompactOutcome {
            tokens_before: BEFORE,
            tokens_after: AFTER
        })
    );
    assert_eq!(
        summarize_body(&stub),
        json!({"providerID": "given-provider", "modelID": "given/model"})
    );
    assert!(
        !requests(&stub).iter().any(|r| r.contains("/config")),
        "the config was not read: {:?}",
        requests(&stub)
    );
}

// ---- the model rule: no pin, no request ----

/// No pin the adapter can use is `compact-model-unpinned` and nothing is posted: the model is
/// never guessed. The pin is absent, empty, has no `/`, or lacks its provider or its model; the
/// message names no part of the config (it holds a provider key).
#[test]
fn a_missing_or_unusable_pin_is_compact_model_unpinned_and_nothing_is_posted() {
    let pins = [
        None,
        Some(""),
        Some("no-slash"),
        Some("/model-only"),
        Some("provider-only/"),
    ];
    for pin in pins {
        let stub = server(pin);
        let error = compact(&stub).expect_err("no usable pin");
        assert_eq!(
            error.code(),
            COMPACT_MODEL_UNPINNED.as_str(),
            "{pin:?}: {error}"
        );
        assert!(
            posts(&stub).is_empty(),
            "{pin:?}: nothing was posted: {:?}",
            requests(&stub)
        );
        assert_no_leak(&error, &format!("{pin:?}"));
    }
}

/// A pin object without a `model`, or a config whose `agent.compaction` is not an object, is
/// no pin either.
#[test]
fn a_compaction_entry_without_a_model_is_no_pin() {
    for agent in [
        json!({"agent": {"compaction": {"variant": "high"}}}),
        json!({"agent": {"compaction": "provider/model"}}),
        json!({"agent": {"compaction": {"model": 7}}}),
        json!({"agent": "compaction"}),
    ] {
        let stub = server(None);
        stub.json("GET", "/config", 200, &agent.to_string());
        let error = compact(&stub).expect_err("no usable pin");
        assert_eq!(
            error.code(),
            COMPACT_MODEL_UNPINNED.as_str(),
            "{agent}: {error}"
        );
        assert!(posts(&stub).is_empty(), "{agent}");
    }
}

// ---- the session rules: nothing is sent to a session that cannot take it ----

/// An id the server does not know is `session-not-found`, and nothing else is asked: not the
/// activity, not the config, not a POST.
#[test]
fn an_unknown_session_is_session_not_found_and_only_the_existence_check_is_made() {
    for status in [404, 400] {
        let stub = server(Some(PIN));
        stub.json("GET", &session_path(), status, stub::NOT_FOUND);
        assert_eq!(
            compact(&stub),
            Err(PaneError::SessionNotFound {
                what: ID.to_owned()
            }),
            "{status}"
        );
        assert_eq!(
            requests(&stub),
            [format!("GET {}", session_path())],
            "{status}"
        );
    }
}

/// A session that runs a turn, or holds a question or a permission request, is refused with
/// the code of what it is doing, and no summarize is posted. (The engine has checked already;
/// the adapter checks again, because a turn can start in between.)
#[test]
fn a_busy_or_waiting_session_is_refused_and_no_summarize_is_posted() {
    let busy = json!({ ID: {"type": "busy"} }).to_string();
    let retrying = json!({ ID: {"type": "retry"} }).to_string();
    let asked = json!([{"id": "que_1", "sessionID": ID}]).to_string();
    let cases: [(&str, &str, &str, &str); 4] = [
        ("/session/status", &busy, "session-busy", "a busy turn"),
        (
            "/session/status",
            &retrying,
            "session-busy",
            "a retrying turn",
        ),
        ("/question", &asked, "session-holds-question", "a question"),
        (
            "/permission",
            &asked,
            "session-holds-question",
            "a permission request",
        ),
    ];
    for (route, body, code, what) in cases {
        let stub = server(Some(PIN));
        stub.json("GET", route, 200, body);
        let error = compact(&stub).expect_err(what);
        assert_eq!(error.code(), code, "{what}: {error}");
        assert!(posts(&stub).is_empty(), "{what}: {:?}", requests(&stub));
    }
}

// ---- no reply is quoted ----

/// A reply that is not what a step needs is `unavailable`, naming the route, the port and the
/// status, and quoting **no** part of the body: `GET /config` is a whole server config that
/// can hold a provider key, and the message list is conversation text. Every route `compact`
/// reads or posts is in the loop, each with the marker leading its body.
#[test]
fn no_reply_ever_reaches_an_error_message() {
    let hold = format!(r#"{{"error":"{MARKER}"}}"#);
    let garbled = format!(r#"["{MARKER}"]"#);
    let cases: [(&str, String, u16, String, &str); 7] = [
        ("GET", "/config".to_owned(), 500, hold.clone(), "/config"),
        ("GET", "/config".to_owned(), 200, garbled, "/config"),
        (
            "GET",
            messages_path(),
            200,
            hold.clone(),
            "/session/:id/message",
        ),
        (
            "GET",
            messages_path(),
            500,
            hold.clone(),
            "/session/:id/message",
        ),
        (
            "POST",
            summarize_path(),
            500,
            hold.clone(),
            "/session/:id/summarize",
        ),
        (
            "POST",
            summarize_path(),
            400,
            hold.clone(),
            "/session/:id/summarize",
        ),
        (
            "POST",
            summarize_path(),
            204,
            hold,
            "/session/:id/summarize",
        ),
    ];
    for (method, path, status, body, label) in cases {
        let stub = server(Some(PIN));
        stub.json(method, &path, status, &body);
        let error = compact(&stub).expect_err("the reply is not the wanted one");
        assert_eq!(
            error.code(),
            "unavailable",
            "{method} {path} {status}: {error}"
        );
        assert_no_leak(&error, &format!("{method} {path} {status}"));
        let text = error.to_string();
        assert!(
            text.contains(label),
            "{method} {path} {status}: names the route: {text}"
        );
        assert!(
            text.contains(&status.to_string()),
            "names the status: {text}"
        );
        assert!(!text.contains('\n'), "one line: {text:?}");
    }
}

/// A config that holds a key but no pin, and an unpinned refusal in general, carry no config
/// text either (the refusal names the pair or nothing).
#[test]
fn the_unpinned_refusal_does_not_carry_the_config() {
    let stub = server(None);
    let error = compact(&stub).expect_err("no pin");
    assert_eq!(error.code(), COMPACT_MODEL_UNPINNED.as_str());
    assert_no_leak(&error, "an unpinned config with a key");
}

// ---- a count that cannot be read is unavailable, never zero ----

/// A session with no assistant message, or whose last assistant message has no readable
/// counts, has no context size to report: `unavailable`, before anything is posted.
#[test]
fn an_unreadable_count_before_is_unavailable_and_nothing_is_posted() {
    let no_assistant = json!([user()]).to_string();
    let no_tokens =
        json!([user(), {"info": {"id": "m", "role": "assistant"}, "parts": []}]).to_string();
    let words =
        json!([user(), {"info": {"role": "assistant", "tokens": "many"}, "parts": []}]).to_string();
    for (list, what) in [
        ("[]".to_owned(), "an empty list"),
        (no_assistant, "no assistant message"),
        (no_tokens, "no tokens"),
        (words, "tokens that are not numbers"),
    ] {
        let stub = server(Some(PIN));
        stub.json("GET", &messages_path(), 200, &list);
        let error = compact(&stub).expect_err(what);
        assert_eq!(error.code(), "unavailable", "{what}: {error}");
        assert!(posts(&stub).is_empty(), "{what}: {:?}", requests(&stub));
    }
}

/// A count after the act that cannot be read is `unavailable` as well: the adapter never infers
/// it (the act did happen, and it says it cannot tell), never answers zero.
#[test]
fn an_unreadable_count_after_is_unavailable_not_zero() {
    let stub = server_listing(Some(PIN), json!([user()]).to_string());
    let error = compact(&stub).expect_err("no count after");
    assert_eq!(error.code(), "unavailable", "{error}");
    assert_eq!(
        posts(&stub),
        [format!("POST {}", summarize_path())],
        "the act happened once"
    );
}

// ---- the bounds ----

/// A summarize is a model call: slower than one request's bound (`request`) and than the whole
/// call's (`call`) and still inside `compact`, it succeeds. The token reads after it are within
/// the same bound.
#[test]
fn a_summarize_slower_than_a_request_but_inside_compact_succeeds() {
    let timeouts = Timeouts {
        call: Duration::from_millis(1_000),
        request: Duration::from_millis(400),
        compact: Duration::from_secs(5),
        ..quick()
    };
    let stub = server(Some(PIN));
    stub.delay("POST", &summarize_path(), Duration::from_millis(1_500));
    let start = Instant::now();
    let outcome = harness(timeouts).compact(&at(stub.port), ID, None);
    assert_eq!(
        outcome,
        Ok(CompactOutcome {
            tokens_before: BEFORE,
            tokens_after: AFTER
        })
    );
    assert!(
        start.elapsed() >= Duration::from_millis(1_500),
        "the call waited for the reply"
    );
}

/// The token reads after the act get the same bound: a message list that is slower than `call`
/// and `request` (a long session) is read.
#[test]
fn the_token_reads_get_the_compact_bound_too() {
    let timeouts = Timeouts {
        call: Duration::from_millis(1_000),
        request: Duration::from_millis(400),
        compact: Duration::from_secs(8),
        ..quick()
    };
    let stub = server(Some(PIN));
    stub.delay("GET", &messages_path(), Duration::from_millis(1_300));
    let outcome = harness(timeouts).compact(&at(stub.port), ID, None);
    assert_eq!(
        outcome,
        Ok(CompactOutcome {
            tokens_before: BEFORE,
            tokens_after: AFTER
        })
    );
}

/// A summarize that outlasts `compact` is `timeout` of the method's own op, and the call ends
/// at that bound, not when the reply would have come.
#[test]
fn a_summarize_outlasting_compact_is_a_timeout_of_the_compact_op() {
    let timeouts = Timeouts {
        call: Duration::from_millis(500),
        request: Duration::from_millis(300),
        compact: Duration::from_millis(1_200),
        ..quick()
    };
    let stub = server(Some(PIN));
    stub.delay("POST", &summarize_path(), Duration::from_secs(6));
    let start = Instant::now();
    let op = timeout_op(
        harness(timeouts).compact(&at(stub.port), ID, None),
        "a summarize that outlasts compact",
    );
    assert_eq!(op, HarnessOp::Compact.as_str());
    let took = start.elapsed();
    assert!(
        took >= Duration::from_millis(1_000),
        "waited for the bound: {took:?}"
    );
    assert!(
        took < Duration::from_millis(1_200) + SLACK + SLACK,
        "ended at it: {took:?}"
    );
}

/// The 120 s is for the summarize and the token reads **only**: a server that hangs on the
/// existence check, the activity read or the config read holds the caller for `call`, with
/// nothing posted, however wide `compact` is. (`session_activity` builds its own `Call`, so it
/// is never given the compact bound either.)
#[test]
fn a_hang_before_the_act_ends_at_call_not_at_compact() {
    // `session_activity` builds a `Call` of its own, so a hang on one of its reads ends as its
    // own op's timeout; the existence check and the config read are the compact call's.
    let hangs = [
        (session_path(), HarnessOp::Compact.as_str()),
        ("/config".to_owned(), HarnessOp::Compact.as_str()),
        ("/session/status".to_owned(), "harness.session_activity"),
        ("/question".to_owned(), "harness.session_activity"),
    ];
    for (path, op) in hangs {
        let timeouts = Timeouts {
            call: Duration::from_millis(700),
            request: Duration::from_millis(300),
            compact: Duration::from_secs(30),
            ..quick()
        };
        let stub = server(Some(PIN));
        stub.hang("GET", &path);
        let start = Instant::now();
        let got = timeout_op(
            harness(timeouts).compact(&at(stub.port), ID, None),
            &format!("a hang on {path}"),
        );
        assert_eq!(got, op, "{path}");
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "{path}: ended at call, not at compact: {:?}",
            start.elapsed()
        );
        assert!(posts(&stub).is_empty(), "{path}: nothing was posted");
    }
}

/// A call whose deadline is spent before it starts is a `timeout` of the method's op, as every
/// method's is (`ac30e`), and sends no summarize.
#[test]
fn a_spent_call_times_out_with_the_compact_op() {
    let timeouts = Timeouts {
        call: Duration::ZERO,
        ..bounds(Duration::from_secs(4))
    };
    let stub = server(Some(PIN));
    let op = timeout_op(
        harness(timeouts).compact(&at(stub.port), ID, None),
        "a spent call",
    );
    assert_eq!(op, HarnessOp::Compact.as_str());
    assert!(posts(&stub).is_empty());
}

// ---- a server that is not there ----

/// A port nothing listens on is `unavailable`, checked as every call that reaches the server is.
#[test]
fn a_refused_port_is_unavailable() {
    let (port, result) = on_refused_port(
        |port| harness(quick()).compact(&at(port), ID, None),
        |result| matches!(result, Err(PaneError::Unavailable { .. })),
    );
    let message = unavailable(result, "a refused port");
    assert!(
        message.contains(&port.to_string()),
        "names the port: {message}"
    );
}
