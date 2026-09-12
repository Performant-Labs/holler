//! The control-socket **server** side: newline-delimited JSON-RPC handling
//! for the one-shot CLI commands (`hub status`, `hub token ping`, …) that
//! reach the live hub process over the control Unix socket (ADR 0006). The
//! client side lives in [`crate::control`]; the accept loop that spawns
//! [`handle_control_conn`] per connection lives in [`crate::serve`].
//!
//! Moved out of `serve.rs` (issue #182) once that file's authenticated
//! live-session path pushed it toward the 900-line build guard — this module
//! is the whole control-socket **server**, `serve.rs` is the WS accept loop
//! and handshake.

use holler_proto::{Code, Envelope, WireError};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::live::Registry;
use crate::roster::Roster;
use crate::state::{advertise_path, resolve_state_dir, HubState};

/// Handle one control-socket connection: newline-delimited JSON-RPC. A frame
/// that does not decode is `-32700`/`-32600`; an unknown `control/…` method is
/// `-32601 method_not_found`.
pub async fn handle_control_conn(
    stream: UnixStream,
    registry: Registry,
    roster: std::sync::Arc<Roster>,
) {
    let (read_half, write_half) = tokio::io::split(stream);
    let mut write_half = write_half;
    let mut lines = tokio::io::BufReader::new(read_half).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let line = line.trim_end().to_string();
                if line.is_empty() {
                    continue;
                }
                let reply = dispatch_control(&line, &registry, &roster).await;
                let bytes = format!("{reply}\n");
                if write_half.write_all(bytes.as_bytes()).await.is_err() {
                    return; // client went away.
                }
            }
            Ok(None) => return, // client closed.
            Err(_) => return,
        }
    }
}

/// `component=control` debug event: one `control/…` request received on the
/// socket (issue #197 — `dispatch_control` previously logged nothing at all;
/// `Component::Control` elsewhere in the hub only ever covers `serve.rs`'s
/// own bind/lock lifecycle events, never a per-request trace of the control
/// socket itself).
fn log_control(method: &str, id: Option<&str>) {
    let mut fields = vec![("method", method.to_string())];
    if let Some(i) = id {
        fields.push(("id", i.to_string()));
    }
    holler_proto::log::emit(&holler_proto::log::Event {
        component: holler_proto::log::Component::Control,
        severity: holler_proto::log::Severity::Debug,
        direction: holler_proto::log::Direction::In,
        method: "control_request",
        id: None,
        peer: None,
        fields,
        frame: None,
    });
}

/// Parse one control frame, dispatch it, and return the reply as a single
/// line (no trailing newline; the caller adds it).
async fn dispatch_control(line: &str, registry: &Registry, roster: &Roster) -> String {
    // The control socket is **internal, non-wire**: it is not validated
    // against the v2 wire catalog (those are the `control/…` methods, which
    // live only here). We still parse the frame as a JSON-RPC object so we can
    // echo the request's id and answer with the right envelope shape.
    let obj: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return unkeyed_error_line(Code::ParseError, "the control frame is not JSON"),
    };
    // A batch (array) on the control socket is also a parse-shape error.
    if obj.is_array() {
        return unkeyed_error_line(Code::InvalidRequest, "a batch is not a control frame");
    }
    let id = obj.get("id").and_then(|v| v.as_str()).map(str::to_owned);
    let method = obj.get("method").and_then(|v| v.as_str());
    let cid = resolve_cid(id.as_deref());

    log_control(method.unwrap_or("<none>"), id.as_deref());

    match method {
        Some("control/status") => {
            let doc = status_doc(registry).await;
            encode_response(&cid, doc)
        }
        Some("control/token_ping") => token_ping(&cid, &obj, registry).await,
        Some("control/caps") => {
            let listening = read_listening_here();
            let doc = crate::query::local_caps(registry, listening).await;
            encode_response(&cid, serde_json::to_value(doc).unwrap_or_default())
        }
        Some("control/support") => hub_support(&cid, &obj, registry).await,
        Some("control/query_local") => hub_query_local(&cid, &obj, registry).await,
        Some("control/query_remote") => hub_query_remote(&cid, &obj, registry).await,
        Some(other) if other.starts_with("control/") => {
            dispatch_session_control(other, &cid, &obj, registry, roster).await
        }
        Some(other) => encode_error(&cid, Code::MethodNotFound, format!("unknown control method: {other}")),
        // No method: not a call (a stray response/notification or empty frame).
        None => unkeyed_error_line(Code::InvalidRequest, "a control frame must be a request with a method"),
    }
}

/// The session/roster half of [`dispatch_control`]'s match — split out
/// purely to keep `dispatch_control`'s own cognitive-complexity score under
/// the workspace's `clippy.toml` threshold of 15 once issues #191/#142 added
/// their own arms alongside #150/#186/#192's existing ones; no behavior
/// change, just fewer arms in one match.
async fn dispatch_session_control(
    method: &str,
    cid: &holler_proto::CorrelationId,
    obj: &serde_json::Value,
    registry: &Registry,
    roster: &Roster,
) -> String {
    match method {
        "control/say" => say(cid, obj, registry, roster).await,
        // `control/interrupt` (issue #191): the CLI's `interrupt` verb.
        "control/interrupt" => interrupt(cid, obj, registry, roster).await,
        "control/answer" => answer(cid, obj, registry).await,
        // `control/roster` (issue #186): read the hub's own roster and return
        // `{rows: [...]}` (the live-only view; the CLI's `--all` reads the
        // same socket and asks for the full set, which the server honors here).
        "control/roster" => roster_control(cid, obj, roster).await,
        // Issue #192's test-only hook: forcibly end a body's live connection
        // to simulate an abrupt drop. Gated behind `HOLLER_TEST_HOOKS=1` (an
        // env var, not `cfg(test)`, since this dispatch runs inside the real
        // compiled `holler` binary a test spawns as a subprocess — a
        // `cfg(test)` gate would never be reachable there at all). Answered
        // as a plain unknown method when the flag is unset, so the hook is
        // indistinguishable from not existing in a production hub.
        "control/test_drop" if test_hooks_enabled() => test_drop(cid, obj, registry).await,
        // `control/wait` (issue #142): block until any named session (or
        // every row under `--prefix`) matches one of the target states, or
        // `params.timeout_ms` elapses. Edge-triggered on `Roster::subscribe`
        // — no polling loop anywhere in this path.
        "control/wait" => wait(cid, obj, roster).await,
        // `control/revoke` (issue #184): `hub token revoke`/`delete`'s own
        // process already flipped the token store to `revoked`; this call
        // (from that same CLI invocation) force-closes the live socket, if
        // any, so the spec's "closes the socket immediately" is not left
        // waiting on the body's own liveness timeout.
        "control/revoke" => revoke(cid, obj, registry).await,
        other => encode_error(cid, Code::MethodNotFound, format!("unknown control method: {other}")),
    }
}

/// `control/support {feature}` (issue #185): a single `query/support` answer
/// for the hub role (see [`crate::query::local_support`]).
async fn hub_support(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let Some(feature) = obj.get("params").and_then(|p| p.get("feature")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/support needs params.feature".to_string());
    };
    match crate::query::local_support(feature, registry).await {
        Ok(doc) => encode_response(cid, serde_json::to_value(doc).unwrap_or_default()),
        Err(e) => encode_error(cid, Code::UnknownFeature, e.message),
    }
}

/// `control/query_local {method, params?}` (issue #185): `holler hub query
/// CMD [ARGS...]` — the local form, answered from this hub's own state
/// without touching any body.
async fn hub_query_local(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let Some(method) = obj.get("params").and_then(|p| p.get("method")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/query_local needs params.method".to_string());
    };
    let inner_params = obj.get("params").and_then(|p| p.get("params")).cloned();
    match method {
        "query/status" => {
            let doc = crate::query::local_status(registry, read_listening_here()).await;
            encode_response(cid, serde_json::to_value(doc).unwrap_or_default())
        }
        "query/caps" => {
            let doc = crate::query::local_caps(registry, read_listening_here()).await;
            encode_response(cid, serde_json::to_value(doc).unwrap_or_default())
        }
        "query/support" => {
            let feature = inner_params.as_ref().and_then(|p| p.get("feature")).and_then(|v| v.as_str()).unwrap_or_default();
            match crate::query::local_support(feature, registry).await {
                Ok(doc) => encode_response(cid, serde_json::to_value(doc).unwrap_or_default()),
                Err(e) => encode_error(cid, Code::UnknownFeature, e.message),
            }
        }
        "query/protocol" => match holler_proto::ProtocolParams::parse_version(inner_params.as_ref()) {
            Ok(version) => {
                let doc = crate::query::local_protocol(version);
                encode_response(cid, serde_json::to_value(doc).unwrap_or_default())
            }
            Err(e) => encode_error_frame(cid, &e),
        },
        other => encode_error(cid, Code::MethodNotFound, format!("unknown query method: {other}")),
    }
}

/// `control/query_remote {target, method, params?}` (issue #185): `holler hub
/// query TARGET CMD [ARGS...]` — resolve `target` against the live registry
/// and forward the `query/*` request over that body's socket. No live body
/// for `target` is `-32004 not_connected` (exit 1 at the CLI); more than one
/// match is reported as an ambiguous refusal (the CLI maps that to exit 2,
/// ADR 0003's usage-error code for an unresolvable target).
async fn hub_query_remote(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let params = obj.get("params");
    let (Some(target), Some(method)) = (
        params.and_then(|p| p.get("target")).and_then(|v| v.as_str()),
        params.and_then(|p| p.get("method")).and_then(|v| v.as_str()),
    ) else {
        return encode_error(cid, Code::InvalidParams, "control/query_remote needs params.target and params.method".to_string());
    };
    let inner_params = params.and_then(|p| p.get("params")).cloned();

    match registry.find_target(target).await {
        crate::live::TargetLookup::Found(handle) => {
            match handle.query(method, inner_params, std::time::Duration::from_secs(10)).await {
                Some(Ok(value)) => encode_response(cid, value),
                // The body itself raised a wire error (e.g. `query/support`'s
                // `-32006 unknown_feature`) — forward it verbatim rather than
                // re-wrapping it in a fresh `Code`.
                Some(Err(e)) => holler_proto::encode(&Envelope::error_frame(cid, &e)).unwrap_or_default(),
                None => encode_error(cid, Code::NotConnected, format!("{target} did not answer")),
            }
        }
        crate::live::TargetLookup::NotConnected => {
            encode_error(cid, Code::NotConnected, format!("{target} is not connected"))
        }
        crate::live::TargetLookup::Ambiguous => {
            encode_error(cid, Code::InvalidRequest, format!("{target} is ambiguous — matches more than one live body"))
        }
    }
}

/// `control/say {session, text, timeout_ms, queue}` (issue #190): the CLI's
/// `say` verb, relayed to [`crate::talk::say`]. See that module for the full
/// resolve → busy-check → send → collect → TalkLog flow; this is only the
/// control-socket param/result mapping.
async fn say(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry, roster: &Roster) -> String {
    let params = obj.get("params");
    let Some(session) = params.and_then(|p| p.get("session")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/say needs params.session".to_string());
    };
    let Some(text) = params.and_then(|p| p.get("text")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/say needs params.text".to_string());
    };
    let queue = params.and_then(|p| p.get("queue")).and_then(|v| v.as_bool()).unwrap_or(false);
    let timeout_ms = params.and_then(|p| p.get("timeout_ms")).and_then(|v| v.as_u64()).unwrap_or(600_000);
    let timeout = std::time::Duration::from_millis(timeout_ms);

    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    match crate::talk::say(registry, roster, &state, session, text, queue, timeout).await {
        Ok(outcome) => encode_response(cid, serde_json::json!({
            "session": outcome.session,
            "stop_reason": outcome.stop_reason,
            "state": outcome.state,
            "updates": outcome.updates,
            "elapsed_ms": outcome.elapsed_ms,
            "text": crate::talk::reply_text(&outcome.message),
            "message": outcome.message,
        })),
        Err(crate::talk::SayError::UnknownSession) => {
            encode_error(cid, Code::UnknownSession, format!("unknown session: {session}"))
        }
        // Issue #192, rule 4: `detail` already carries the roster's own
        // `conn_state` (and, for `reconnecting`, the last-seen age) when the
        // roster still remembers this name — see `talk::not_connected_detail`.
        Err(crate::talk::SayError::NotConnected(detail)) => encode_error(cid, Code::NotConnected, detail),
        Err(crate::talk::SayError::Ambiguous(candidates)) => {
            let message = format!("ambiguous session {session}: candidates are {}", candidates.join(", "));
            let err = WireError::new(Code::UnknownSession, message, Some("ambiguous"));
            encode_error_frame(cid, &err)
        }
        Err(crate::talk::SayError::Busy { state, turn_age_ms, last_update_age_ms }) => {
            let err = WireError::session_busy(state, turn_age_ms, last_update_age_ms);
            encode_error_frame(cid, &err)
        }
        // Issue #151: reported as `-32009 session_busy` with `data.state
        // = "input-required"` (distinct from `working`/`stalled`) so the
        // CLI's own busy hint (`say_cmd::busy_hint`) can point at `answer`
        // instead of `interrupt`/`--queue` — neither of which resolves a
        // held permission/elicitation.
        Err(crate::talk::SayError::InputRequired { question, turn_age_ms, last_update_age_ms }) => {
            let mut err = WireError::session_busy("input-required", turn_age_ms, last_update_age_ms);
            if let Some(q) = question {
                err.message = q;
            }
            encode_error_frame(cid, &err)
        }
        Err(crate::talk::SayError::Refused(err)) => encode_error_frame(cid, &err),
        Err(crate::talk::SayError::ConnectionLost) => {
            encode_error(cid, Code::ConnectionLost, format!("{session} io disconnected mid-turn; ask again"))
        }
        Err(crate::talk::SayError::Timeout) => {
            let secs = timeout.as_secs();
            encode_error(cid, Code::NotConnected, format!("no reply from {session} within {secs}s"))
        }
        Err(crate::talk::SayError::Cancelled) => {
            encode_error(cid, Code::NotConnected, "prompt was interrupted before it completed".to_string())
        }
    }
}

/// `control/interrupt {session, text?}` (issue #191): the CLI's `interrupt`
/// verb, relayed to [`crate::interrupt::interrupt`]. Without `text` the
/// result is `{session, applied: true}`; with `text` it also carries the
/// redirect turn's own reply, in exactly `control/say`'s own result shape
/// (`stop_reason`/`state`/`updates`/`elapsed_ms`/`text`/`message`) so the CLI
/// can share `say_cmd.rs`'s own printing logic.
async fn interrupt(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry, roster: &Roster) -> String {
    let params = obj.get("params");
    let Some(session) = params.and_then(|p| p.get("session")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/interrupt needs params.session".to_string());
    };
    let text = params.and_then(|p| p.get("text")).and_then(|v| v.as_str());

    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    match crate::interrupt::interrupt(registry, roster, &state, session, text).await {
        Ok(outcome) => {
            let mut result = serde_json::json!({ "session": outcome.session, "applied": true });
            if let Some(reply) = &outcome.reply {
                result["stop_reason"] = serde_json::Value::String(reply.stop_reason.clone());
                result["state"] = serde_json::Value::String(reply.state.clone());
                result["updates"] = serde_json::json!(reply.updates);
                result["elapsed_ms"] = serde_json::json!(reply.elapsed_ms);
                result["text"] = serde_json::Value::String(crate::talk::reply_text(&reply.message));
                result["message"] = serde_json::to_value(&reply.message).unwrap_or_default();
            }
            encode_response(cid, result)
        }
        Err(crate::interrupt::InterruptError::UnknownSession) => {
            encode_error(cid, Code::UnknownSession, format!("unknown session: {session}"))
        }
        Err(crate::interrupt::InterruptError::NotConnected) => {
            encode_error(cid, Code::NotConnected, format!("{session}'s body is not connected"))
        }
        Err(crate::interrupt::InterruptError::Ambiguous(candidates)) => {
            let message = format!("ambiguous session {session}: candidates are {}", candidates.join(", "));
            let err = WireError::new(Code::UnknownSession, message, Some("ambiguous"));
            encode_error_frame(cid, &err)
        }
        Err(crate::interrupt::InterruptError::AckTimeout { secs }) => encode_error(
            cid,
            Code::NotConnected,
            format!(
                "interrupt sent but not confirmed within {secs}s (body may be stalled); \
                 the session is still on the roster"
            ),
        ),
        Err(crate::interrupt::InterruptError::Refused(err)) => encode_error_frame(cid, &err),
        Err(crate::interrupt::InterruptError::ConnectionLost) => {
            encode_error(cid, Code::ConnectionLost, format!("{session} io disconnected mid-turn; ask again"))
        }
        Err(crate::interrupt::InterruptError::PromptFailed(e)) => encode_error(cid, Code::NotConnected, e.message()),
    }
}

/// `control/answer {session, choice, timeout_ms?}` (issue #151): the CLI's
/// `answer` verb, relayed to [`crate::talk::answer`]. Mirrors [`say`]'s own
/// param/result mapping; the body-side refusal (`-32010 nothing_pending`,
/// `-32602 invalid_params`, …) is forwarded verbatim via
/// [`crate::talk::AnswerError::Refused`], never re-coded here.
async fn answer(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let params = obj.get("params");
    let Some(session) = params.and_then(|p| p.get("session")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/answer needs params.session".to_string());
    };
    let Some(choice) = params.and_then(|p| p.get("choice")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/answer needs params.choice".to_string());
    };
    let timeout_ms = params.and_then(|p| p.get("timeout_ms")).and_then(|v| v.as_u64()).unwrap_or(10_000);
    let timeout = std::time::Duration::from_millis(timeout_ms);

    match crate::talk::answer(registry, session, choice, timeout).await {
        Ok(outcome) => encode_response(cid, serde_json::json!({ "session": outcome.session, "applied": outcome.applied })),
        Err(crate::talk::AnswerError::UnknownSession) => {
            encode_error(cid, Code::UnknownSession, format!("unknown session: {session}"))
        }
        Err(crate::talk::AnswerError::NotConnected) => {
            encode_error(cid, Code::NotConnected, format!("{session}'s body is not connected"))
        }
        Err(crate::talk::AnswerError::Ambiguous(candidates)) => {
            let message = format!("ambiguous session {session}: candidates are {}", candidates.join(", "));
            let err = WireError::new(Code::UnknownSession, message, Some("ambiguous"));
            encode_error_frame(cid, &err)
        }
        Err(crate::talk::AnswerError::Refused(err)) => encode_error_frame(cid, &err),
        Err(crate::talk::AnswerError::ConnectionLost) => {
            encode_error(cid, Code::ConnectionLost, format!("{session} io disconnected; ask again"))
        }
    }
}

/// `control/roster {all?, prefix?}` (issue #186; `prefix` added by issue
/// #236, ADR 0005 §4): the CLI's `holler roster` verb, answered straight from
/// this hub's in-process roster. Without `params.all` it returns the
/// live-only view (every row except `gone`, matching [`Roster::rows`]); with
/// `params.all == true` it includes `gone` rows so the operator can see what
/// fell off and how recently. `params.prefix`, when present and non-empty,
/// narrows to rows named exactly that prefix or nested under it
/// (`Roster::rows_matching`'s grammar) — done hub-side (not by the CLI
/// filtering a full fetch locally) so a hub with many sessions doesn't ship
/// the whole roster over the control socket just to throw most of it away.
/// `stalled` is not a filter dimension here (it is a display-only column in
/// the CLI), and `hostname` narrowing is still the CLI's job (it can read the
/// roster once and filter locally).
async fn roster_control(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, roster: &Roster) -> String {
    let params = obj.get("params");
    let all = params.and_then(|p| p.get("all")).and_then(|v| v.as_bool()).unwrap_or(false);
    let prefix = params.and_then(|p| p.get("prefix")).and_then(|v| v.as_str());
    let rows = roster.rows_matching(Option::from(all), prefix);
    encode_response(
        cid,
        serde_json::json!({ "rows": rows }),
    )
}

/// `control/wait {sessions?, prefix?, until?, after?, timeout_ms?}` (issue
/// #142): block until a named session (or every row under `prefix`) matches
/// one of `until`'s target states, or `timeout_ms` elapses. Edge-triggered on
/// [`Roster::subscribe`] — the loop below only ever wakes on an actual roster
/// change (`rx.changed()`) or the deadline, never a fixed poll tick, so there
/// is no disguised polling loop here (see `Roster::read_count`, which the
/// `wait_uses_no_polling` test reads to prove it).
///
/// The result is always a **success** envelope, `{matched, rows}`: a timeout
/// is `matched:false, rows:[]`, not a JSON-RPC error — "nothing happened in
/// this window" is an ordinary outcome of a wait, not a hub-side refusal, and
/// giving it its own wire error code would be one more code the CLI has to
/// special-case instead of just reading `matched`. `rows` is one entry per
/// row that currently satisfies `until` (already narrowed to the caller's
/// `sessions`/`prefix`), each `{session, state, conn_state, turn_id,
/// stop_reason, last_turn}` — the shape `wait_cmd.rs` renders and `--json`
/// forwards.
async fn wait(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, roster: &Roster) -> String {
    let params = obj.get("params");
    let sessions = comma_list(params, "sessions");
    let prefix = params.and_then(|p| p.get("prefix")).and_then(|v| v.as_str()).map(str::to_string);
    if sessions.is_empty() && prefix.is_none() {
        return encode_error(cid, Code::InvalidParams, "control/wait needs params.sessions or params.prefix".to_string());
    }
    let until = {
        let given = comma_list(params, "until");
        if given.is_empty() { default_until() } else { given }
    };
    let after = params.and_then(|p| p.get("after")).and_then(|v| v.as_str()).map(str::to_string);
    let timeout_ms = params.and_then(|p| p.get("timeout_ms")).and_then(|v| v.as_u64()).unwrap_or(600_000);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

    // Subscribe *before* the first check: a change that lands between "check"
    // and "subscribe" would otherwise be missed (the classic wait/notify
    // race). `watch::Receiver::subscribe` always yields the sender's current
    // value first, so the loop's very first `changed()` (if reached) still
    // reports the latest generation, not a stale one.
    let mut rx = roster.subscribe();
    loop {
        let rows = matching_rows(roster, &sessions, prefix.as_deref(), &until, after.as_deref());
        if !rows.is_empty() {
            return encode_response(cid, serde_json::json!({ "matched": true, "rows": rows }));
        }
        if tokio::time::Instant::now() >= deadline {
            return encode_response(cid, serde_json::json!({ "matched": false, "rows": [] }));
        }
        tokio::select! {
            changed = rx.changed() => {
                if changed.is_err() {
                    // The roster outlives every connection in the live hub;
                    // a dropped sender is unreachable in production. Fail
                    // like a timeout rather than looping forever on an error.
                    return encode_response(cid, serde_json::json!({ "matched": false, "rows": [] }));
                }
            }
            () = tokio::time::sleep_until(deadline) => {
                return encode_response(cid, serde_json::json!({ "matched": false, "rows": [] }));
            }
        }
    }
}

/// The default `--until` set (ADR 0003 / holler#142's spec): every terminal
/// turn outcome, `input-required`, and `gone` — deliberately **not** bare
/// `idle` (a session goes idle after join, an interrupt, a refusal, *and* a
/// success, so it would fire far too eagerly as a default).
fn default_until() -> Vec<String> {
    ["completed", "failed", "rejected", "input-required", "gone"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// Split a `params.<field>` comma-separated string into trimmed, non-empty
/// segments (`""`/absent → empty `Vec`).
fn comma_list(params: Option<&serde_json::Value>, field: &str) -> Vec<String> {
    params
        .and_then(|p| p.get(field))
        .and_then(|v| v.as_str())
        .map(|s| s.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect())
        .unwrap_or_default()
}

/// The rows (in [`Roster::rows_matching`]'s display shape — `stalled`
/// already applied) that currently match both the caller's `sessions`/
/// `prefix` filter and one of `until`'s target states. `all` rows (`gone`
/// included) are always read here — `gone` is itself a legal `--until`
/// target, so a wait watching for it must see rows a plain `roster` listing
/// would hide by default.
fn matching_rows(
    roster: &Roster,
    sessions: &[String],
    prefix: Option<&str>,
    until: &[String],
    after: Option<&str>,
) -> Vec<serde_json::Value> {
    roster
        .rows_matching(Some(true), prefix)
        .into_iter()
        .filter(|row| prefix.is_some() || sessions.iter().any(|s| row.name == *s || row.name.ends_with(&format!("/{s}"))))
        .filter(|row| row_matches_until(row, until, after))
        .map(row_to_json)
        .collect()
}

/// True iff `row` currently satisfies any of `until`'s target states.
/// `idle`/`working`/`input-required`/`stalled` compare the row's own
/// (already stall-derived) `state`; `gone` compares `conn_state`;
/// `completed`/`canceled`/`failed`/`rejected` compare `last_turn.state`,
/// additionally requiring `last_turn.turn_id != after` when `after` is given
/// (the anti-spin rule: a caller re-waiting on a turn it already saw settle
/// does not instantly re-match on that same turn).
fn row_matches_until(row: &crate::roster::Row, until: &[String], after: Option<&str>) -> bool {
    until.iter().any(|state| match state.as_str() {
        "gone" => row.conn_state == "gone",
        "idle" | "working" | "input-required" | "stalled" => row.state == *state,
        "completed" | "canceled" | "failed" | "rejected" => row.last_turn.as_ref().is_some_and(|lt| {
            lt.state.as_str() == state && after.is_none_or(|a| lt.turn_id != a)
        }),
        _ => false,
    })
}

/// One matched row, in the shape `wait_cmd.rs` renders (and `--json`
/// forwards verbatim). `age_secs` (a terminal match only) is computed here,
/// server-side, from `last_turn.ended_at` — reusing [`crate::roster::
/// parse_rfc3339`] rather than giving the CLI a second RFC3339 parser (or a
/// new `time`-crate dependency) just to render "how long ago".
fn row_to_json(row: crate::roster::Row) -> serde_json::Value {
    let age_secs = row.last_turn.as_ref().and_then(|lt| crate::roster::parse_rfc3339_secs_since(&lt.ended_at));
    serde_json::json!({
        "session": row.name,
        "state": row.state,
        "conn_state": row.conn_state,
        "turn_id": row.turn_id,
        "stop_reason": row.last_turn.as_ref().map(|lt| lt.stop_reason.clone()),
        "last_turn": row.last_turn,
        "age_secs": age_secs,
    })
}

/// The bound listen addresses, read the same way [`status_doc`] does (from
/// `hub/listening.json`, written once at `hub serve` startup) — the shared
/// helper both `control/status` and the new `control/caps`/`control/
/// query_local {method:"query/status"}` need.
fn read_listening_here() -> Vec<String> {
    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    read_listening(&state)
}

/// Whether the issue #192 test-only control hooks (`control/test_drop`) are
/// enabled on this hub process. `HOLLER_TEST_HOOKS=1` only — checked at
/// request time (not cached), so a test that spawns its own hub with this
/// env var never affects any other concurrently-running hub process.
fn test_hooks_enabled() -> bool {
    std::env::var("HOLLER_TEST_HOOKS").map(|v| v == "1").unwrap_or(false)
}

/// `control/test_drop {token}` (issue #192, test-only): resolve `token`
/// (a token id, client id, or hostname/label — the same grammar
/// `control/query_remote`'s `target` uses) against the live registry and ask
/// that connection to forcibly end itself, simulating an abrupt network drop.
/// No live match is `-32004 not_connected`; more than one is `-32600` (the
/// CLI's own exit-2 ambiguity shape), matching `hub_query_remote`'s own
/// mapping.
async fn test_drop(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let Some(token) = obj.get("params").and_then(|p| p.get("token")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/test_drop needs params.token".to_string());
    };
    match registry.find_target(token).await {
        crate::live::TargetLookup::Found(handle) => {
            let dropped = handle.force_drop().await;
            encode_response(cid, serde_json::json!({ "dropped": dropped }))
        }
        crate::live::TargetLookup::NotConnected => {
            encode_error(cid, Code::NotConnected, format!("{token} has no live connection"))
        }
        crate::live::TargetLookup::Ambiguous => {
            encode_error(cid, Code::InvalidRequest, format!("{token} is ambiguous — matches more than one live body"))
        }
    }
}
/// `control/revoke {token_id}` (issue #184): if `token_id` currently has a
/// live socket, ask it to close with WS code 1008 and mark the roster row
/// `gone` at once. `{closed: bool}` reports whether a live socket was found
/// (`false` is not an error — the token may simply not have been connected,
/// which is the common case; the store-side revoke already took effect in
/// the CLI process that called this).
async fn revoke(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let Some(token_id) = obj.get("params").and_then(|p| p.get("token_id")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/revoke needs params.token_id".to_string());
    };
    match registry.find_by_token(token_id).await {
        Some(handle) => {
            let closed = handle.revoke().await;
            encode_response(cid, serde_json::json!({ "closed": closed }))
        }
        None => encode_response(cid, serde_json::json!({ "closed": false })),
    }
}

/// `control/token_ping {token_id}` (issue #182): find the live circuit bound
/// to `token_id` in the registry and ask it to answer a `circuit/ping`,
/// returning `{hostname, rtt_ms}`. No live socket for that token is
/// `-32004 not_connected` — the spec's exact fail-closed shape for `hub token
/// ping` against a body that is not currently connected.
async fn token_ping(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let Some(token_id) = obj.get("params").and_then(|p| p.get("token_id")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/token_ping needs params.token_id".to_string());
    };
    let Some(handle) = registry.find_by_token(token_id).await else {
        return encode_error(cid, Code::NotConnected, format!("{token_id} has no live connection"));
    };
    let started = std::time::Instant::now();
    match handle.ping(std::time::Duration::from_secs(5)).await {
        Some(ack) => {
            let elapsed = started.elapsed();
            // Issue #191: this measured RTT is the input to the *next*
            // `interrupt`'s ack-timeout scale (`interrupt::ack_timeout`).
            handle.record_rtt(elapsed).await;
            let rtt_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
            encode_response(cid, serde_json::json!({ "hostname": ack.hostname, "rtt_ms": rtt_ms }))
        }
        None => encode_error(cid, Code::NotConnected, format!("{token_id} did not answer the ping")),
    }
}

fn encode_response(cid: &holler_proto::CorrelationId, result: serde_json::Value) -> String {
    let env = Envelope::response(cid, Some(result));
    holler_proto::encode(&env).unwrap_or_default()
}

fn encode_error(cid: &holler_proto::CorrelationId, code: Code, message: String) -> String {
    let env = Envelope::error_frame(cid, &WireError::new(code, message, None));
    holler_proto::encode(&env).unwrap_or_default()
}

/// Encode an already-built [`WireError`] verbatim (issue #190: `say`'s own
/// `session_busy`/body-refused errors already carry the exact `data` the CLI
/// needs — `encode_error` would flatten that away).
fn encode_error_frame(cid: &holler_proto::CorrelationId, error: &WireError) -> String {
    let env = Envelope::error_frame(cid, error);
    holler_proto::encode(&env).unwrap_or_default()
}

/// Build a hub-minted unkeyed error line (`id: null`) carrying `code`.
fn unkeyed_error_line(code: Code, message: &str) -> String {
    let env = Envelope::Error {
        id: None,
        error: WireError::new(code, message, None),
    };
    holler_proto::encode(&env).unwrap_or_default()
}

/// A synthetic hub id for control replies whose request id is missing or
/// unparsable (defensive; the control protocol expects a request id).
// The literal is a well-formed `h-` id, so the parse is infallible.
#[allow(clippy::expect_used)] // #143
fn fallback_id() -> holler_proto::CorrelationId {
    holler_proto::CorrelationId::parse("h-000000000000000000000000")
        .expect("a well-formed synthetic hub id")
}

/// Resolve a request id (a wire string, possibly absent) to a
/// [`CorrelationId`], falling back to a synthetic hub id when it is missing or
/// does not carry the `h-`/`b-` prefix.
fn resolve_cid(id: Option<&str>) -> holler_proto::CorrelationId {
    match id.and_then(|s| holler_proto::CorrelationId::parse(s).ok()) {
        Some(cid) => cid,
        None => fallback_id(),
    }
}

/// Build the hub's status document for a `control/status` answer. Per the
/// story spec the doc has `role:"hub"`, a `listening` **array** of bound
/// addresses, an optional `advertise`, `clients` (bodies), `sessions`,
/// `harnesses_known`, `harnesses_confirmed`, `protocol:2`, and `version`.
/// `clients`/`sessions`/`harnesses_known`/`harnesses_confirmed` are now the
/// live registry's real counts (issue #182 landed `clients`; issue #185 adds
/// the rest — previously always `0`/`[]`, since no body could yet report a
/// session count or a confirmed harness).
async fn status_doc(registry: &Registry) -> serde_json::Value {
    // Only ever called by the live hub's own control dispatch, where the state
    // dir is always resolvable; `unwrap_or_default` is a defensive no-op.
    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    let listening = read_listening(&state);
    let advertise = std::fs::read_to_string(advertise_path(&state))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("advertise")
                .and_then(|a| a.as_str())
                .map(str::to_owned)
        });
    let version = env!("CARGO_PKG_VERSION");
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string());
    // Issue #322: the hub's X25519 public key, so an operator can compare it
    // out-of-band without re-minting a token. `identity::ensure` is
    // idempotent (loads the existing key once `hub serve`/`hub token mint`
    // generated one); `None` only if it cannot even be resolved (an
    // unwritable state dir), which `hub status` should still answer despite.
    let hub_pubkey = crate::identity::ensure(&state).ok().map(|i| i.public_hex());

    serde_json::json!({
        "role": "hub",
        "protocol": 2,
        "version": version,
        "hostname": hostname,
        "listening": listening,
        "advertise": advertise,
        "hub_pubkey": hub_pubkey,
        "clients": registry.len().await,
        // Issue #184: the per-connection detail (`token_id`/`client_id`/
        // `hostname`/`peer`/`connected_at`) `hub status --json` now also
        // reports. Additive — `clients` itself stays the plain live count
        // the existing test suite already asserts `as_u64()` against.
        "clients_detail": registry.clients_detail().await,
        "sessions": registry.total_sessions().await,
        "harnesses_known": registry.harnesses_known().await,
        "harnesses_confirmed": registry.harnesses_confirmed().await,
        // Issue #184's acceptance: the hygiene/lockout limits documented in
        // `hub status --json`'s `limits{}`.
        "limits": crate::hygiene::HygieneLimits::resolve().to_json(crate::lockout::LockoutLimits::resolve()),
    })
}

/// The bound listen addresses for the live hub, read from the listening event
/// we already emitted on stderr at startup. We re-derive them from the live
/// listeners' state: there is no persisted listener list, so a re-bind is
/// wrong. Instead the hub records its bound addrs in memory; `status_doc` runs
/// on the same process, so we read them from a file the start path writes.
fn read_listening(state: &HubState) -> Vec<String> {
    // The start path writes the bound addresses to `hub/listening.json` so a
    // `control/status` (running in the same process) can report them.
    let path = state.hub_dir.join("listening.json");
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod wait_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #142
    //! Issue #142's `wait_uses_no_polling`: proving the `control/wait`
    //! handler is genuinely event-driven — it calls the private `wait` fn
    //! directly (same module, same crate) against a real [`Roster`], rather
    //! than driving a whole hub subprocess, because the property under test
    //! (how many times the roster's row table was *read* during a multi-
    //! hundred-millisecond wait) is an in-process fact `Roster::read_count`
    //! exposes — there is no wire-level way to observe it from a CLI
    //! subprocess test.
    use super::*;
    use crate::roster::{Config, Roster};
    use holler_proto::{docs::LastTurn, CorrelationId, Mode, Presence, SessionAd, SessionState};

    fn idle_presence(name: &str) -> Presence {
        Presence {
            hostname: "h1".to_string(),
            sessions: vec![SessionAd {
                name: name.to_string(),
                harness: "opencode".to_string(),
                state: SessionState::Idle,
                mode: Mode::Spawn,
                harness_session_id: None,
                turn_started_at: None,
                last_update_at: None,
                pending: None,
                turn_id: None,
                last_turn: None,
            }],
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn wait_uses_no_polling() {
        let roster = Roster::with_system_clock(&Config::default());
        roster.set_token("tok1", "cli1");
        roster.set_label("tok1", "io");
        roster.advertise("tok1", &idle_presence("alpha"));

        let cid = CorrelationId::parse("b-test-wait").expect("well-formed test id");
        let obj = serde_json::json!({
            "id": "b-test-wait",
            "method": "control/wait",
            "params": { "sessions": "io/alpha", "until": "completed", "timeout_ms": 5_000 },
        });

        let before = roster.read_count();
        let roster_task = roster.clone();
        let handle = tokio::spawn(async move { wait(&cid, &obj, &roster_task).await });

        // Give the task time to reach its first check and start blocking on
        // `rx.changed()` (a real event-driven wait spends this whole span
        // parked, not looping); then deliver exactly one real change.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        roster.set_last_turn(
            "tok1",
            "alpha",
            LastTurn {
                turn_id: "h-000000000000000000000001".to_string(),
                state: SessionState::Completed,
                stop_reason: "end_turn".to_string(),
                ended_at: "2026-09-09T00:00:00Z".to_string(),
            },
        );

        let reply = tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("the wait task finished promptly after the change")
            .expect("the wait task did not panic");
        let env = holler_proto::decode(&reply).expect("a valid v2 envelope");
        let result = env.result().expect("a result, not an error");
        assert_eq!(result["matched"], serde_json::json!(true), "must match once last_turn lands: {result}");

        let after = roster.read_count();
        // A real 50ms poll loop over this ~300ms span would already have run
        // 6+ checks; an event-driven wait runs one check up front, blocks,
        // and re-checks once per actual change — at most a handful of reads
        // regardless of how long the blocking span was.
        assert!(
            after.saturating_sub(before) <= 5,
            "too many roster reads ({} -> {}) for one real change — looks like a disguised poll loop",
            before,
            after
        );
    }
}
