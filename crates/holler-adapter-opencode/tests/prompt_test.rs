#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #642
//! `send_prompt` and `session_activity` against the stub of `support/stub.rs` (#642 part 3,
//! acceptance 7 and 8). `send_prompt` queues a prompt through OpenCode's HTTP API (I4, D6) and
//! never puts the prompt's text in a message; `session_activity` reads `HoldingQuestion`,
//! `Busy` or `Idle` from the server's live state (D7).

#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/stub.rs"]
#[allow(dead_code)] // #642: shared with hermetic_test.rs
mod stub;

use std::time::Instant;

use fixtures::*;
use holler_pane::ports::Activity;
use holler_pane::{HarnessEndpoint, HarnessPort, PaneError};
use serde_json::{json, Value};
use stub::Stub;

/// A distinctive prompt: no message may carry it. Short enough that a 60-byte excerpt of an
/// echo of the posted body (`{"parts":[{"type":"text","text":"` is 33 bytes) would hold it.
const PROMPT: &str = "zq642-prompt-text";
/// Another session on the same server.
const OTHER: &str = "ses_other0000000000000000000";
/// A direct child (subagent) session of [`ID`], and a child of that one.
const CHILD: &str = "ses_child0000000000000000000";
const GRANDCHILD: &str = "ses_grand0000000000000000000";

fn prompt_path() -> String {
    format!("/session/{ID}/prompt_async")
}

/// The body D6 posts, as `holler-body` does (E16): the text part and nothing else.
fn prompt_body() -> String {
    json!({ "parts": [{ "type": "text", "text": PROMPT }] }).to_string()
}

fn posted(stub: &Stub) -> bool {
    stub.lines()
        .iter()
        .any(|line| line.contains("prompt_async"))
}

/// Neither the error's message nor its debug form carries the prompt.
fn assert_no_prompt(error: &PaneError, what: &str) {
    let text = format!("{error} {error:?}");
    assert!(!text.contains(PROMPT), "{what}: the prompt leaked: {text}");
}

// ---- Acceptance 7: send_prompt ----

/// D6: after the existence check, the bare route gets exactly the text part, and a `204` is
/// `Ok`.
#[test]
fn ac7_send_prompt_posts_the_text_part_to_prompt_async_and_a_204_is_ok() {
    let stub = Stub::start();
    known_session(&stub);
    stub.json("POST", &prompt_path(), 204, "");
    assert_eq!(
        harness(quick()).send_prompt(&HarnessEndpoint::Port(stub.port), ID, PROMPT),
        Ok(())
    );
    let seen = stub.seen();
    let lines: Vec<&str> = seen.iter().map(|s| s.line.as_str()).collect();
    assert_eq!(
        lines,
        [
            format!("GET /session/{ID} HTTP/1.1"),
            format!("POST /session/{ID}/prompt_async HTTP/1.1")
        ]
    );
    let body: Value = serde_json::from_slice(&seen[1].body).expect("the body is JSON");
    assert_eq!(
        body,
        json!({ "parts": [{ "type": "text", "text": PROMPT }] }),
        "the text part only: no agent"
    );
}

/// D6 (`known`, E29): a 404 or a 400 to `GET /session/:id` is `session-not-found`, and
/// nothing is posted.
#[test]
fn ac7_an_unknown_session_is_session_not_found_and_nothing_is_posted() {
    for status in [404, 400] {
        let stub = Stub::start();
        stub.json("GET", &format!("/session/{ID}"), status, stub::NOT_FOUND);
        stub.json("POST", &prompt_path(), 204, "");
        assert_eq!(
            harness(quick()).send_prompt(&HarnessEndpoint::Port(stub.port), ID, PROMPT),
            Err(PaneError::SessionNotFound {
                what: ID.to_string()
            }),
            "{status}"
        );
        assert!(!posted(&stub), "{status}: {:?}", stub.lines());
    }
}

/// D6 (E29): any other answer to the existence check, a `500` or a `200` that is not that
/// session, is `unavailable`, and nothing is posted.
#[test]
fn ac7_another_answer_to_the_existence_check_is_unavailable_and_nothing_is_posted() {
    let failing = Stub::start();
    failing.json("GET", &format!("/session/{ID}"), 500, stub::NOT_FOUND);
    let another = Stub::start();
    another.json(
        "GET",
        &format!("/session/{ID}"),
        200,
        &format!(r#"{{"id":"{OTHER}","title":"x"}}"#),
    );
    for (stub, what) in [(&failing, "a 500"), (&another, "another session")] {
        stub.json("POST", &prompt_path(), 204, "");
        unavailable(
            harness(quick()).send_prompt(&HarnessEndpoint::Port(stub.port), ID, PROMPT),
            what,
        );
        assert!(!posted(stub), "{what}: {:?}", stub.lines());
    }
}

/// D6: a post answered with anything but a `204` is `unavailable` in one line naming the
/// route and the port, and quotes no part of the reply: each reply here echoes the posted
/// body (the prompt within its first 60 bytes), so a message that quoted it would carry the
/// prompt.
#[test]
fn ac7_a_post_not_answered_204_is_unavailable_and_never_carries_the_prompt() {
    let html = Stub::start();
    html.set(
        "POST",
        &prompt_path(),
        200,
        "text/html",
        &prompt_body(),
        stub::Framing::ContentLength,
    );
    let failing = Stub::start();
    failing.json("POST", &prompt_path(), 500, &prompt_body());
    let ok_json = Stub::start();
    ok_json.json("POST", &prompt_path(), 200, &prompt_body());
    for (stub, status, what) in [
        (&html, "200", "an HTML 200"),
        (&failing, "500", "a 500"),
        (&ok_json, "200", "a JSON 200"),
    ] {
        known_session(stub);
        let result = harness(quick()).send_prompt(&HarnessEndpoint::Port(stub.port), ID, PROMPT);
        if let Err(error) = &result {
            assert_no_prompt(error, what);
        }
        let message = unavailable(result, what);
        assert!(!message.contains(['\n', '\r']), "{what}: one line");
        assert!(message.contains("/prompt_async"), "{what}: {message:?}");
        assert!(message.contains(status), "{what}: {message:?}");
        assert!(
            message.contains(&stub.port.to_string()),
            "{what}: {message:?}"
        );
    }
}

/// D8: the `op` of a `timeout` is `harness.send_prompt` and `harness.session_activity`,
/// pinned as literals: the test kit's `HarnessOp` has no counterpart for them yet (#684).
#[test]
fn ac7_ac8_a_frozen_server_is_timeout_with_each_methods_op() {
    let stub = Stub::start();
    stub.freeze();
    let h = harness(quick());
    let start = Instant::now();
    let sent = h.send_prompt(&HarnessEndpoint::Port(stub.port), ID, PROMPT);
    if let Err(error) = &sent {
        assert_no_prompt(error, "a frozen server");
    }
    assert_eq!(timeout_op(sent, "send_prompt"), "harness.send_prompt");
    assert_within(start, quick().call + SLACK, "send_prompt");
    let start = Instant::now();
    assert_eq!(
        timeout_op(
            h.session_activity(&HarnessEndpoint::Port(stub.port), ID),
            "session_activity"
        ),
        "harness.session_activity"
    );
    assert_within(start, quick().call + SLACK, "session_activity");
}

// ---- Acceptance 8: session_activity ----

/// How the stub answers one route.
#[derive(Clone)]
enum Answer {
    Json(u16, String),
    Html,
}

fn ok(body: impl Into<String>) -> Answer {
    Answer::Json(200, body.into())
}

/// The server's state as the routes D7 reads answer it. The default is a server where
/// nothing is held, `ID` is the one session, nothing runs, and the message list is unset.
#[derive(Clone)]
struct Scene {
    questions: Answer,
    permissions: Answer,
    sessions: Answer,
    status: Answer,
    messages: Answer,
}

impl Default for Scene {
    fn default() -> Self {
        Scene {
            questions: ok("[]"),
            permissions: ok("[]"),
            sessions: ok(format!(r#"[{{"id":"{ID}"}}]"#)),
            status: ok("{}"),
            messages: Answer::Json(404, stub::NOT_FOUND.to_owned()),
        }
    }
}

/// `session_activity(ID)` on a stub set up as `scene`.
fn activity(scene: &Scene) -> Result<Activity, PaneError> {
    let stub = Stub::start();
    for (path, answer) in [
        ("/question".to_owned(), &scene.questions),
        ("/permission".to_owned(), &scene.permissions),
        ("/session".to_owned(), &scene.sessions),
        ("/session/status".to_owned(), &scene.status),
        (format!("/session/{ID}/message"), &scene.messages),
    ] {
        match answer {
            Answer::Json(status, body) => stub.json("GET", &path, *status, body),
            Answer::Html => stub.html("GET", &path),
        }
    }
    harness(quick()).session_activity(&HarnessEndpoint::Port(stub.port), ID)
}

/// A pending question of `session`, in OpenCode's `QuestionRequest` shape (E17).
fn question(session: &str) -> Answer {
    ok(format!(
        r#"[{{"id":"que_1","sessionID":"{session}","questions":[{{"question":"q","options":[{{"label":"a"}}]}}]}}]"#
    ))
}

/// A pending permission request of `session` (E17).
fn permission(session: &str) -> Answer {
    ok(format!(
        r#"[{{"id":"per_1","sessionID":"{session}","permission":"bash"}}]"#
    ))
}

/// `ID`'s status entry of type `kind`.
fn status_of(kind: &str) -> Answer {
    ok(format!(r#"{{"{ID}":{{"type":"{kind}"}}}}"#))
}

/// `GET /session`: `ID`, its child, and the child's own child.
fn family() -> Answer {
    ok(format!(
        r#"[{{"id":"{ID}"}},{{"id":"{CHILD}","parentID":"{ID}"}},{{"id":"{GRANDCHILD}","parentID":"{CHILD}"}},{{"id":"{OTHER}","parentID":null}}]"#
    ))
}

/// D7(a): a question or a permission request of this session, or of a direct child of it,
/// is `HoldingQuestion`, checked before the status (which says `busy` here).
#[test]
fn ac8_a_held_question_or_permission_of_the_session_or_its_child_is_holding_question() {
    for (held, what) in [
        (ID, "the session's own"),
        (CHILD, "a direct child's (a subagent)"),
    ] {
        let by_question = Scene {
            questions: question(held),
            sessions: family(),
            status: status_of("busy"),
            ..Scene::default()
        };
        assert_eq!(
            activity(&by_question),
            Ok(Activity::HoldingQuestion),
            "a question, {what}"
        );
        let by_permission = Scene {
            questions: ok("[]"),
            permissions: permission(held),
            ..by_question
        };
        assert_eq!(
            activity(&by_permission),
            Ok(Activity::HoldingQuestion),
            "a permission request, {what}"
        );
    }
}

/// D7(a): an item of another session, of a child of another session, or of a grandchild does
/// not count, and the status decides (`busy` here).
#[test]
fn ac8_an_item_outside_the_session_and_its_children_does_not_count() {
    let sessions = ok(format!(
        r#"[{{"id":"{ID}"}},{{"id":"{CHILD}","parentID":"{OTHER}"}},{{"id":"{GRANDCHILD}","parentID":"{CHILD}"}}]"#
    ));
    for (held, what) in [
        (OTHER, "another session"),
        (CHILD, "a child of another session"),
    ] {
        let scene = Scene {
            questions: question(held),
            permissions: permission(held),
            sessions: sessions.clone(),
            status: status_of("busy"),
            ..Scene::default()
        };
        assert_eq!(activity(&scene), Ok(Activity::Busy), "{what}");
    }
    let grandchild = Scene {
        questions: question(GRANDCHILD),
        permissions: permission(GRANDCHILD),
        sessions: family(),
        status: status_of("busy"),
        ..Scene::default()
    };
    assert_eq!(activity(&grandchild), Ok(Activity::Busy), "a grandchild");
}

/// D7(a): when `GET /session` is not a list, only an exact `sessionID` match counts.
#[test]
fn ac8_with_no_session_list_only_an_exact_match_counts() {
    let exact = Scene {
        questions: question(ID),
        sessions: Answer::Html,
        status: status_of("busy"),
        ..Scene::default()
    };
    assert_eq!(activity(&exact), Ok(Activity::HoldingQuestion), "exact");
    let child = Scene {
        questions: question(CHILD),
        ..exact
    };
    assert_eq!(activity(&child), Ok(Activity::Busy), "a child, unlisted");
}

/// Plan review W-2: a `/question` or `/permission` answer that is not a `200` JSON array (the
/// web app's page, a `500`) holds nothing, as `holler-body` reads it; the status decides.
#[test]
fn ac8_a_list_that_cannot_be_read_holds_nothing() {
    for (bad, what) in [
        (Answer::Html, "an HTML 200"),
        (
            Answer::Json(500, r#"{"name":"UnknownError"}"#.to_owned()),
            "a 500",
        ),
    ] {
        let questions = Scene {
            questions: bad.clone(),
            status: status_of("busy"),
            ..Scene::default()
        };
        assert_eq!(
            activity(&questions),
            Ok(Activity::Busy),
            "/question, {what}"
        );
        let permissions = Scene {
            permissions: bad,
            status: status_of("busy"),
            ..Scene::default()
        };
        assert_eq!(
            activity(&permissions),
            Ok(Activity::Busy),
            "/permission, {what}"
        );
    }
}

/// D7(b): an entry for the session whose `type` is not `idle` (`busy`, `retry`, or none) is
/// `Busy`; no entry, an `idle` one, or another session's is `Idle`.
#[test]
fn ac8_the_status_entry_decides_busy_or_idle() {
    let retry = ok(format!(
        r#"{{"{ID}":{{"type":"retry","attempt":1,"message":"x","next":1}}}}"#
    ));
    let untyped = ok(format!(r#"{{"{ID}":{{}}}}"#));
    let cases = [
        (status_of("busy"), Activity::Busy, "busy"),
        (retry, Activity::Busy, "retry"),
        (untyped, Activity::Busy, "no type"),
        (ok("{}"), Activity::Idle, "no entry"),
        (status_of("idle"), Activity::Idle, "idle"),
        (
            ok(format!(r#"{{"{OTHER}":{{"type":"busy"}}}}"#)),
            Activity::Idle,
            "another session busy",
        ),
    ];
    for (status, want, what) in cases {
        let scene = Scene {
            status,
            ..Scene::default()
        };
        assert_eq!(activity(&scene), Ok(want), "{what}");
    }
}

/// An assistant message, completed or not, after a user message (E21's shape).
fn messages(completed: bool) -> Answer {
    let time = if completed {
        r#"{"created":2,"completed":3}"#
    } else {
        r#"{"created":2}"#
    };
    ok(format!(
        r#"[{{"info":{{"role":"user","time":{{"created":1}}}}}},{{"info":{{"role":"assistant","time":{time}}}}}]"#
    ))
}

/// D7(c): when `/session/status` does not answer a JSON object (the web app's page, a `500`
/// whose body is an object, a `200` whose JSON is `[]`), the message list decides: the last
/// assistant message without `time.completed` is `Busy`, a completed one `Idle`. An error
/// object is never taken for an empty status.
#[test]
fn ac8_the_message_list_decides_when_the_status_is_not_an_object() {
    for (status, what) in [
        (Answer::Html, "an HTML 200"),
        (Answer::Json(500, "{}".to_owned()), "a 500 object"),
        (ok("[]"), "a 200 array"),
    ] {
        let running = Scene {
            status: status.clone(),
            messages: messages(false),
            ..Scene::default()
        };
        assert_eq!(activity(&running), Ok(Activity::Busy), "{what}, running");
        let ended = Scene {
            status,
            messages: messages(true),
            ..Scene::default()
        };
        assert_eq!(activity(&ended), Ok(Activity::Idle), "{what}, ended");
    }
}

/// D7(c): with neither the status nor the message list usable, the call is `unavailable`.
#[test]
fn ac8_with_no_usable_status_or_message_list_it_is_unavailable() {
    let scene = Scene {
        status: Answer::Html,
        messages: Answer::Html,
        ..Scene::default()
    };
    unavailable(activity(&scene), "no status and no message list");
}
