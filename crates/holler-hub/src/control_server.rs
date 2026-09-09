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
        Some("control/say") => say(&cid, &obj, registry).await,
        // `control/interrupt` (issue #191): the CLI's `interrupt` verb.
        Some("control/interrupt") => interrupt(&cid, &obj, registry).await,
        Some("control/answer") => answer(&cid, &obj, registry).await,
        // `control/roster` (issue #186): read the hub's own roster and return
        // `{rows: [...]}` (the live-only view; the CLI's `--all` reads the
        // same socket and asks for the full set, which the server honors here).
        Some("control/roster") => roster_control(&cid, &obj, roster).await,
        Some(other) => encode_error(&cid, Code::MethodNotFound, format!("unknown control method: {other}")),
        // No method: not a call (a stray response/notification or empty frame).
        None => unkeyed_error_line(Code::InvalidRequest, "a control frame must be a request with a method"),
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
async fn say(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
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
    match crate::talk::say(registry, &state, session, text, queue, timeout).await {
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
        Err(crate::talk::SayError::NotConnected) => {
            encode_error(cid, Code::NotConnected, format!("{session}'s body is not connected"))
        }
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
async fn interrupt(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry) -> String {
    let params = obj.get("params");
    let Some(session) = params.and_then(|p| p.get("session")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/interrupt needs params.session".to_string());
    };
    let text = params.and_then(|p| p.get("text")).and_then(|v| v.as_str());

    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    match crate::interrupt::interrupt(registry, &state, session, text).await {
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

/// The bound listen addresses, read the same way [`status_doc`] does (from
/// `hub/listening.json`, written once at `hub serve` startup) — the shared
/// helper both `control/status` and the new `control/caps`/`control/
/// query_local {method:"query/status"}` need.
fn read_listening_here() -> Vec<String> {
    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    read_listening(&state)
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

    serde_json::json!({
        "role": "hub",
        "protocol": 2,
        "version": version,
        "hostname": hostname,
        "listening": listening,
        "advertise": advertise,
        "clients": registry.len().await,
        "sessions": registry.total_sessions().await,
        "harnesses_known": registry.harnesses_known().await,
        "harnesses_confirmed": registry.harnesses_confirmed().await,
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
