//! `control/rebind {token_id, session, harness_session_id}` (#839, ADR-0021 section 11 point 2):
//! ask the body bound to `token_id` to re-point its attach session `session` to another harness
//! session on the same endpoint (`session/rebind`). `holler pane switch` and `holler pane reset`
//! call it through the CLI's `HubPort` after the TUI is seen showing the target. The precedent is
//! `control_hold.rs`: one control method in a file of its own.
//!
//! - **The local control socket only.** There is no `admin/rebind`, so an admin-role connection
//!   (remote, less trusted than the local socket) cannot re-point a body, and the generic
//!   forward (`LiveHandle::query`, which `control/query_remote` and `admin/query` reach) refuses
//!   `session/rebind`: the request rides `LiveHandle::rebind`, its own forward.
//! - **Fail closed before a body is asked**: the three params are required, the id must be a
//!   harness session id (the grammar `pane switch` checks a typed one with: no `/`, no space), the
//!   token must have a live connection, and `session` must be in that body's cached presence.
//! - **Bounded** by [`REBIND_FORWARD`], the wait for the connection's one reply slot included:
//!   above the body's own 6 s reply wait, below the CLI's 10 s call, so each layer sees the
//!   answer of the one below it. A body's refusal is passed through verbatim.
//! - **Before the reply**, the hub's cached presence and the roster row name the id the body
//!   acknowledged, so a `control/roster` or a prompt resolved right after already sees it; each is
//!   a forward-fill of the one row (as `talk::say` fills `last_turn`), so the body's other rows,
//!   and their state, are untouched. The next presence beat confirms it. A rebind sets no hold:
//!   the session's name, and so its hold key, does not change.

use std::time::Duration;

use holler_pane::findings::parse_session_id;
use holler_proto::rebind::{Rebind, RebindResult};
use holler_proto::{Code, SessionAd};

use crate::control_server::{encode_error, encode_error_frame, encode_response};
use crate::live::{LiveHandle, Registry};
use crate::roster::Roster;

/// How long the hub waits for a body to answer a `session/rebind`, the wait for the connection's
/// one reply slot included: strictly above the body's reply wait (6 s) and below the CLI's call
/// (`ControlCall::rebind`, 10 s).
pub const REBIND_FORWARD: Duration = Duration::from_secs(8);

/// `control/rebind` (see the module docs): the reply line.
pub(crate) async fn rebind(cid: &holler_proto::CorrelationId, obj: &serde_json::Value, registry: &Registry, roster: &Roster) -> String {
    let params = obj.get("params");
    let field = |name: &str| params.and_then(|p| p.get(name)).and_then(|v| v.as_str());
    let (Some(token_id), Some(session), Some(id)) = (field("token_id"), field("session"), field("harness_session_id")) else {
        let message = "control/rebind needs params.token_id, params.session and params.harness_session_id";
        return encode_error(cid, Code::InvalidParams, message.to_owned());
    };
    if let Err(invalid) = parse_session_id(id) {
        return encode_error(cid, Code::InvalidParams, invalid.to_string());
    }
    let Some(handle) = registry.find_by_token(token_id).await else {
        return encode_error(cid, Code::NotConnected, format!("{token_id} has no live connection"));
    };
    let cached = handle.presence().await.unwrap_or_default();
    let Some(ad) = cached.into_iter().find(|ad| ad.name == session) else {
        return encode_error(cid, Code::UnknownSession, format!("{token_id} has no session {session}"));
    };
    let request = Rebind { session: session.to_owned(), harness_session_id: id.to_owned() };
    let Ok(request) = serde_json::to_value(&request) else {
        return encode_error(cid, Code::InvalidParams, "the rebind request did not encode".to_owned());
    };
    match handle.rebind(request, REBIND_FORWARD).await {
        None => {
            let secs = REBIND_FORWARD.as_secs();
            encode_error(cid, Code::NotConnected, format!("{token_id} did not answer the rebind of {session} within {secs}s"))
        }
        Some(Err(refusal)) => encode_error_frame(cid, &refusal),
        Some(Ok(result)) => {
            if let Ok(acknowledged) = serde_json::from_value::<RebindResult>(result.clone()) {
                record(registry, roster, &handle, ad, &acknowledged).await;
            }
            encode_response(cid, result)
        }
    }
}

/// Before the reply: the one cached row and the one roster row of the rebound session name the
/// id the body acknowledged.
async fn record(registry: &Registry, roster: &Roster, handle: &LiveHandle, mut ad: SessionAd, acknowledged: &RebindResult) {
    ad.harness_session_id = Some(acknowledged.harness_session_id.clone());
    registry.replace_session(handle, ad).await;
    roster.set_harness_session_id(&handle.token_id, &acknowledged.session, &acknowledged.harness_session_id);
}
