//! The prompt and activity seam of `HarnessPort` (ADR-0021 section 8): #645 part 2 armed it
//! with defaults, and #642 part 3 makes it real. `send_prompt` queues a prompt over OpenCode's
//! HTTP API, never by typing into a TUI (I4), and `session_activity` reads what a session is
//! doing from the server's live state, each within one call's bound.
//!
//! Both are pure transport. A hold, a limit or a park belongs at the hub's one prompt path,
//! never in an adapter. Since #839 no verb and no engine calls `send_prompt`: `pane reset
//! --first` goes through the hub's prompt path too (ADR-0021 section 11), and the method stays
//! as the adapter's transport.

use holler_pane::ports::Activity;
use holler_pane::PaneError;
use serde_json::{json, Value};

use crate::{child_ids, json_of, known, running, session_path, status_object, string_at};
use crate::{Call, OpenCodeHarness, Route, OP_SEND_PROMPT, OP_SESSION_ACTIVITY};
use crate::{GET_MESSAGES, GET_PERMISSION, GET_QUESTION, LIST, PROMPT_ASYNC, STATUS};

/// `HarnessPort::send_prompt`: the existence check `abort` makes (an unknown session is
/// `session-not-found`, and nothing is posted), then `POST /session/:id/prompt_async` with the
/// text part alone, the body and the bare route `holler-body` sends. It names no agent: the
/// server's `default_agent` is the pane's, checked when the server started. `Ok` exactly when
/// the reply is `204`, OpenCode's answer once the prompt is queued and the turn runs on. Any
/// other reply is `unavailable`, naming the route, the port and the status only
/// (`Call::unexpected_status`), so no message carries the prompt, not even from a server that
/// echoes the request in its error body.
pub(crate) fn send_prompt(
    harness: &OpenCodeHarness,
    port: u16,
    session: &str,
    text: &str,
) -> Result<(), PaneError> {
    let call = harness.call(port, OP_SEND_PROMPT);
    let path = session_path(session);
    known(&call, &path, session)?;
    let body = json!({ "parts": [{ "type": "text", "text": text }] });
    let reply = call.send(PROMPT_ASYNC, &format!("{path}/prompt_async"), Some(&body))?;
    if reply.status == 204 {
        Ok(())
    } else {
        Err(call.unexpected_status(PROMPT_ASYNC, &reply, "204"))
    }
}

/// `HarnessPort::session_activity`, from the server's live state, in this order:
///
/// 1. [`Activity::HoldingQuestion`] while a question or a permission request of `session`, or
///    of a direct child (subagent) session of it, is pending ([`holds_a_question`]): its turn
///    waits on an answer, and a subagent's question belongs to its parent's turn.
/// 2. Otherwise the server's own account of its turns, whenever `GET /session/status` answers
///    a JSON object: [`Activity::Busy`] while it has an entry for `session` that is not `idle`
///    (`busy`, `retry`, or no `type`), else [`Activity::Idle`].
/// 3. Only when that route answers no object (an older server, the web app's page, an error
///    status): the session's message list ([`from_messages`]).
pub(crate) fn session_activity(
    harness: &OpenCodeHarness,
    port: u16,
    session: &str,
) -> Result<Activity, PaneError> {
    let call = harness.call(port, OP_SESSION_ACTIVITY);
    if holds_a_question(&call, session)? {
        return Ok(Activity::HoldingQuestion);
    }
    let reply = call.send(STATUS, STATUS.label, None)?;
    match status_object(&reply) {
        Some(status) => Ok(busy_or_idle(running(&status, session))),
        None => from_messages(&call, session),
    }
}

/// Whether a pending item of `GET /question` or `GET /permission` (both global lists) is held
/// by `session` or by a direct child session of it ([`children_of`]). A permission request
/// counts as a question: the turn waits for an answer either way. The children are asked for
/// only when an item is held by another session.
fn holds_a_question(call: &Call, session: &str) -> Result<bool, PaneError> {
    let mut holders = held_by(call, GET_QUESTION)?;
    holders.extend(held_by(call, GET_PERMISSION)?);
    if holders.iter().any(|holder| holder == session) {
        return Ok(true);
    }
    if holders.is_empty() {
        return Ok(false);
    }
    let children = children_of(call, session)?;
    Ok(holders.iter().any(|holder| children.contains(holder)))
}

/// The `sessionID` of every item of the pending list `route` answers. The list is read
/// best-effort, as `holler-body` reads it: an answer that is not a `200` JSON array holds
/// nothing. That is safe, because a turn that waits on an answer is still running, so its
/// status entry reads `Busy`, and a reset is refused all the same. A refused connection or a
/// missed deadline stays an error.
fn held_by(call: &Call, route: Route) -> Result<Vec<String>, PaneError> {
    let reply = call.send(route, route.label, None)?;
    let list = json_of(&reply);
    let items = list.as_ref().and_then(Value::as_array);
    Ok(items
        .into_iter()
        .flatten()
        .filter_map(|item| string_at(Some(item), "sessionID"))
        .map(str::to_owned)
        .collect())
}

/// The direct children of `session` in `GET /session` ([`crate::child_ids`]). Best-effort, as
/// `holler-body`'s listing is: when the answer is not such a list there are none, so only an
/// exact `sessionID` match counts.
fn children_of(call: &Call, session: &str) -> Result<Vec<String>, PaneError> {
    let reply = call.send(LIST, LIST.label, None)?;
    Ok(json_of(&reply)
        .and_then(|list| child_ids(&list, session))
        .unwrap_or_default())
}

/// The fallback for a server whose status route answers no JSON object: the last message of
/// `session` whose `info.role` is `assistant` is [`Activity::Busy`] while it has no
/// `info.time.completed`; a completed one, or none, is [`Activity::Idle`]. A message list
/// that is not a `200` JSON array either is `unavailable`.
fn from_messages(call: &Call, session: &str) -> Result<Activity, PaneError> {
    let path = format!("{}/message", session_path(session));
    let reply = call.send(GET_MESSAGES, &path, None)?;
    let list = json_of(&reply);
    let Some(messages) = list.as_ref().and_then(Value::as_array) else {
        return Err(call.unexpected(GET_MESSAGES, &reply, "a list of messages"));
    };
    let last = messages
        .iter()
        .filter_map(|message| message.get("info"))
        .rfind(|info| string_at(Some(info), "role") == Some("assistant"));
    Ok(busy_or_idle(last.is_some_and(|info| !completed(info))))
}

/// Whether a message's `info` carries `time.completed`.
fn completed(info: &Value) -> bool {
    info.get("time")
        .and_then(|time| time.get("completed"))
        .is_some_and(|completed| !completed.is_null())
}

fn busy_or_idle(busy: bool) -> Activity {
    if busy {
        Activity::Busy
    } else {
        Activity::Idle
    }
}
