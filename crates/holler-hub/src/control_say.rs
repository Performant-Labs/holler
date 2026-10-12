//! `control/say {session, text, timeout_ms?, queue?, grant?, check_only?}` (issue #190;
//! `check_only`, #839): the CLI's `say` verb relayed to [`crate::talk::say`], and the question
//! `pane reset --first` asks before it acts. `admin/say` shares it (`dispatch_allowlisted`).
//! Moved out of `control_server.rs` whole (#839), with the one mapping of a [`SayError`] to its
//! wire error, which both branches use.
//!
//! - **`check_only`** runs [`crate::talk::admit`], `say`'s own prelude (resolve, the hold's fast
//!   path, the pane gate, the busy check), and stops: `{admitted: true}`, or the refusal `say`
//!   would answer. Nothing reaches the body, no TalkLog line is written, no turn id moves and no
//!   grant is consumed. It is advisory: `send_prompt` stays the authority.
//! - **`session`** is a routable name, or at the hub the `harness_session_id` a live attach-mode
//!   session advertises (ADR-0021 section 11 point 1); the `holler` CLI sends an id only for
//!   `--pane`.
//! - **The offline body** (#839, P7): a target whose body is offline is `-32004 not_connected`
//!   with `data.reason: "offline"` (`WireError::offline`), which is how a `--pane` verb tells a
//!   pane no body drives from a turn that outlived its wait or was cancelled: those stay a bare
//!   `-32004`.

use std::time::Duration;

use holler_proto::{Code, WireError};

use crate::control_server::{encode_error, encode_error_frame, encode_response};
use crate::live::Registry;
use crate::roster::Roster;
use crate::state::{resolve_state_dir, HubState};
use crate::talk::{self, SayError};

/// `control/say` (see the module docs): the reply line.
pub(crate) async fn say(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry, roster: &Roster) -> String {
    let params = obj.get("params");
    let Some(session) = params.and_then(|p| p.get("session")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/say needs params.session".to_string());
    };
    let Some(text) = params.and_then(|p| p.get("text")).and_then(|v| v.as_str()) else {
        return encode_error(cid, Code::InvalidParams, "control/say needs params.text".to_string());
    };
    let flag = |name: &str| params.and_then(|p| p.get(name)).and_then(|v| v.as_bool()).unwrap_or(false);
    let (queue, check_only) = (flag("queue"), flag("check_only"));
    let timeout_ms = params.and_then(|p| p.get("timeout_ms")).and_then(|v| v.as_u64()).unwrap_or(600_000);
    let timeout = Duration::from_millis(timeout_ms);
    let grant = params.and_then(|p| p.get("grant")).and_then(|v| v.as_str());

    if check_only {
        return match talk::admit(registry, roster, session, queue, grant).await {
            Ok(_) => encode_response(cid, serde_json::json!({ "admitted": true })),
            Err(e) => encode_error_frame(cid, &refusal(session, timeout, e)),
        };
    }
    let state = HubState::from_root(resolve_state_dir().unwrap_or_default());
    match talk::say(registry, roster, &state, talk::SayArgs { session, text, queue, grant, timeout }).await {
        Ok(outcome) => encode_response(cid, serde_json::json!({
            "session": outcome.session,
            "stop_reason": outcome.stop_reason,
            "state": outcome.state,
            "updates": outcome.updates,
            "elapsed_ms": outcome.elapsed_ms,
            "text": talk::reply_text(&outcome.message),
            "message": outcome.message,
        })),
        Err(e) => encode_error_frame(cid, &refusal(session, timeout, e)),
    }
}

/// The wire error of a `say` to `session` that produced no reply, with `timeout` its turn
/// budget: the one mapping, for `say` and for its `check_only` question alike.
fn refusal(session: &str, timeout: Duration, error: SayError) -> WireError {
    match error {
        SayError::UnknownSession => WireError::new(Code::UnknownSession, format!("unknown session: {session}"), None),
        // Issue #192, rule 4: `detail` already carries the roster's own `conn_state` (and,
        // for `reconnecting`, the last-seen age) when the roster still remembers this name
        // (`talk::not_connected_detail`). #839 (P7): marked `offline`, the resolve-time case.
        SayError::NotConnected(detail) => WireError::offline(detail),
        SayError::Ambiguous(candidates) => {
            let message = format!("ambiguous session {session}: candidates are {}", candidates.join(", "));
            WireError::new(Code::UnknownSession, message, Some("ambiguous"))
        }
        SayError::Busy { state, turn_age_ms, last_update_age_ms } => {
            WireError::session_busy(state, turn_age_ms, last_update_age_ms)
        }
        // Issue #151: reported as `-32009 session_busy` with `data.state = "input-required"`
        // (distinct from `working`/`stalled`) so the CLI's own busy hint
        // (`say_cmd::busy_hint`) can point at `answer` instead of `interrupt`/`--queue` —
        // neither of which resolves a held permission/elicitation.
        SayError::InputRequired { question, turn_age_ms, last_update_age_ms } => {
            let mut err = WireError::session_busy("input-required", turn_age_ms, last_update_age_ms);
            if let Some(q) = question {
                err.message = q;
            }
            err
        }
        SayError::Refused(err) => err,
        SayError::ConnectionLost => {
            WireError::new(Code::ConnectionLost, format!("{session} io disconnected mid-turn; ask again"), None)
        }
        SayError::Timeout => {
            let secs = timeout.as_secs();
            WireError::new(Code::NotConnected, format!("no reply from {session} within {secs}s"), None)
        }
        SayError::Cancelled => {
            WireError::new(Code::NotConnected, "prompt was interrupted before it completed", None)
        }
    }
}
