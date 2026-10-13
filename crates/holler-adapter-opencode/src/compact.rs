//! `HarnessPort::compact` over OpenCode (#878, ADR-0021 "Compaction"): the context of a
//! session is compacted by `POST /session/:id/summarize`, never by typing `/compact` into the
//! TUI (I4), on the model the server's own config pins (`agent.compaction.model`, as the
//! restart script writes it) unless the caller names one.
//!
//! **Written from the OpenCode 1.19.0 OpenAPI document, never run** (read 2026-10-12):
//! `summarize` takes `{providerID, modelID}` and answers `200`; an assistant message carries
//! `info.tokens` (`input`, `output`, `reasoning`, `cache.read`, `cache.write`), and the context's
//! size is the sum of those counts of the session's last assistant message. The spike of #878
//! (Part B.1) confirms or corrects each of these against a real server.
//!
//! In this order, each step stopping the call:
//!
//! 1. On the ordinary call ([`Timeouts::call`], 10 s): the existence check (`known`: an unknown
//!    session is `session-not-found`, and nothing else is asked).
//! 2. The session's activity (`prompt::session_activity`, which builds its own ordinary call):
//!    a session that is not idle is refused with `activity_refusal`'s code. The engine has
//!    checked already; this re-check is deliberate, because a turn can start in between, and
//!    the port's contract ("a busy session is refused and nothing is sent") holds without the
//!    engine.
//! 3. The model: the one given, and then `GET /config` is not read; else the server's pin,
//!    split at the **first** `/` (a routed provider's model id may hold slashes), both halves
//!    non-empty. No usable pin is [`COMPACT_MODEL_UNPINNED`], with nothing posted.
//! 4. On the compaction's call ([`Timeouts::compact`], 120 s, which is also each request's
//!    bound there, so the 5 s [`Timeouts::request`] does not cut a model call): the context
//!    before, one `POST /session/:id/summarize` naming the model (no `auto`, no `variant`),
//!    accepted only on `200`, and the context after. A count that cannot be read is
//!    `unavailable`, never zero; whether the context shrank is the engine's to judge.
//!
//! **No reply is quoted.** `GET /config` is a whole server config that can hold a provider key,
//! and the message list is conversation text, so every reply that is not the wanted one is
//! reported with `Call::unexpected_status` (the route, the port and the status only). The only
//! names a message of this file carries are a model's provider and id, which are never secrets.
//!
//! [`Timeouts::call`]: crate::Timeouts::call
//! [`Timeouts::compact`]: crate::Timeouts::compact
//! [`Timeouts::request`]: crate::Timeouts::request

use holler_pane::ports::{CompactModel, CompactOutcome, COMPACT_MODEL_UNPINNED};
use holler_pane::tx_switch::activity_refusal;
use holler_pane::PaneError;
use serde_json::{json, Value};

use crate::prompt::{messages, session_activity};
use crate::{deadline_after, json_of, known, session_path, string_at};
use crate::{Call, OpenCodeHarness, Route, GET_CONFIG, GET_MESSAGES};

/// The `op` of the method's `timeout`: the test kit's `HarnessOp::Compact`.
const OP_COMPACT: &str = "harness.compact";

/// OpenCode's route for a compaction of a session's context.
const SUMMARIZE: Route = Route {
    method: "POST",
    label: "/session/:id/summarize",
};

/// `HarnessPort::compact`, in the module's order.
pub(crate) fn compact(
    harness: &OpenCodeHarness,
    port: u16,
    session: &str,
    model: Option<&CompactModel>,
) -> Result<CompactOutcome, PaneError> {
    let call = harness.call(port, OP_COMPACT);
    let path = session_path(session);
    known(&call, &path, session)?;
    if let Some((code, doing)) = activity_refusal(session_activity(harness, port, session)?) {
        return Err(PaneError::Refused {
            code,
            message: format!("session {session} on port {port} {doing}"),
        });
    }
    let model = match model {
        Some(model) => model.clone(),
        None => pinned(&call)?,
    };
    let bound = harness.config.timeouts.compact;
    let long = Call {
        request: bound,
        ..harness.call_until(port, OP_COMPACT, deadline_after(bound))
    };
    let tokens_before = context_size(&long, session)?;
    let body = json!({ "providerID": model.provider, "modelID": model.model_id });
    let reply = long.send(SUMMARIZE, &format!("{path}/summarize"), Some(&body))?;
    if reply.status != 200 {
        return Err(long.unexpected_status(SUMMARIZE, &reply, "200"));
    }
    let tokens_after = context_size(&long, session)?;
    Ok(CompactOutcome {
        tokens_before,
        tokens_after,
    })
}

/// The compaction model the server's config pins: `agent.compaction.model` of `GET /config`,
/// split at the first `/`. A config that is not a JSON object is `unavailable` (quoting none
/// of it); one with no usable pin is [`COMPACT_MODEL_UNPINNED`], naming none of it either.
fn pinned(call: &Call) -> Result<CompactModel, PaneError> {
    let reply = call.send(GET_CONFIG, GET_CONFIG.label, None)?;
    let config = json_of(&reply)
        .filter(Value::is_object)
        .ok_or_else(|| call.unexpected_status(GET_CONFIG, &reply, "the server's config"))?;
    let pin = config
        .get("agent")
        .and_then(|agent| agent.get("compaction"))
        .and_then(|compaction| string_at(Some(compaction), "model"));
    match pin.and_then(|pin| pin.split_once('/')) {
        Some((provider, model_id)) if !provider.is_empty() && !model_id.is_empty() => {
            Ok(CompactModel {
                provider: provider.to_owned(),
                model_id: model_id.to_owned(),
            })
        }
        _ => Err(PaneError::Refused {
            code: COMPACT_MODEL_UNPINNED,
            message: format!(
                "no model was named and the config of the harness server on port {} pins no \
                 usable compaction model (agent.compaction.model, provider/model); nothing was \
                 sent",
                call.port
            ),
        }),
    }
}

/// The context size of `session`: the sum of the token counts of its last assistant message
/// (`input` and `output` required; `reasoning`, `cache.read` and `cache.write` counted when
/// present). A list with no assistant message, or whose last one has no readable counts, is
/// `unavailable` naming the route and the status, never a zero.
fn context_size(call: &Call, session: &str) -> Result<u64, PaneError> {
    let (reply, messages) = messages(call, session)?;
    messages
        .iter()
        .filter_map(|message| message.get("info"))
        .rfind(|info| string_at(Some(info), "role") == Some("assistant"))
        .and_then(|info| info.get("tokens"))
        .and_then(token_sum)
        .ok_or_else(|| call.unexpected_status(GET_MESSAGES, &reply, "a context size"))
}

/// The sum of an `info.tokens` object's counts; `None` when `input` or `output` is not a
/// count, or an optional count is present and is not one.
fn token_sum(tokens: &Value) -> Option<u64> {
    let required = |key: &str| tokens.get(key).and_then(Value::as_u64);
    let optional = |value: Option<&Value>| match value {
        None | Some(Value::Null) => Some(0),
        Some(count) => count.as_u64(),
    };
    let cache = tokens.get("cache");
    let counts = [
        required("input")?,
        required("output")?,
        optional(tokens.get("reasoning"))?,
        optional(cache.and_then(|cache| cache.get("read")))?,
        optional(cache.and_then(|cache| cache.get("write")))?,
    ];
    counts.into_iter().try_fold(0_u64, u64::checked_add)
}
