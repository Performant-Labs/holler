//! The two pre-auth handlers (`circuit/join` and `circuit/authenticate`),
//! split out of `serve.rs` so that file stays under the 900-line lint gate.
//! Each decodes its params, touches the token store (on the blocking pool),
//! registers the socket in the registry (superseding any older socket on the
//! same token), and sends the response frame. On failure they return the
//! `(code, message)` the caller refuses with. ADR 0006.

use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;

use holler_proto::{Envelope, Code};

use crate::state::HubState;
use crate::conn::ConnCtx;
use crate::registry::{ConnectionHandle, ConnInfo, Registry};
use crate::wire_io::{now_ms, send_error, send_result};

/// Decode a `circuit/join`'s params (`{secret, hostname}`), redeem the secret
/// from the token store, register the socket in the registry (superseding any
/// older socket on the same token), and send the `JoinResult`
/// (`{client_id, credential}`) response.
///
/// Per protocol v2 §3, `circuit/join` is a **one-shot bootstrap**: it redeems
/// a one-time join secret and, on success, the hub **closes the socket after
/// replying** — join never leads into talk on the same socket. So this handler
/// owns the terminal close: on success it sends the result and flags
/// `ctx.close_pending` (the caller's `finish` then flushes a normal close
/// before the read half is dropped), and on failure it sends the codec's error
/// + close. It returns `()` (no `ConnectionHandle` to keep the socket alive).
///
/// A failed join records **no** lockout strike (the redeem is a one-time
/// secret exchange, not a long-lived credential check — lockout strikes
/// belong to `circuit/authenticate`), so `ctx.guard.armed` is left to the
/// caller.
pub async fn handle_join(
    env: &Envelope,
    ctx: &mut ConnCtx<'_>,
    tx: &UnboundedSender<Message>,
    state: &HubState,
    registry: &Arc<Registry>,
    peer: &str,
) {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct JoinParams {
        secret: String,
        hostname: String,
    }
    let params: JoinParams = match env
        .params()
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
    {
        Some(p) => p,
        None => {
            send_error(tx, env.id(), Code::InvalidParams, "join params need {secret, hostname}");
            ctx.close_pending = true;
            return;
        }
    };
    let (client_id, credential) =
        match crate::token::redeem_async(&params.secret, &params.hostname, state).await {
            Ok(rc) => rc,
            Err(e) => {
                send_error(tx, env.id(), Code::Unauthenticated, e.message());
                ctx.close_pending = true;
                return;
            }
        };
    // `redeem` returns only (client_id, credential); the token id is the
    // store's id, so look the (now bound) record up by the client id. This
    // goes through the async twin (`list_async`) so it blocks on the store
    // flock on the blocking pool — a synchronous `list` here would do a
    // non-blocking flock try on the tokio thread and spuriously fail ("no
    // matching token") whenever a sibling connection's slow store op is
    // holding the lock (see `list_async`'s doc / #184).
    let token_id = match crate::token::list_async(state)
        .await
        .ok()
        .and_then(|rows| rows.into_iter().find(|r| r.client_id.as_deref() == Some(&client_id)))
        .map(|r| r.token_id)
    {
        Some(id) => id,
        None => {
            send_error(tx, env.id(), Code::Unauthenticated, "no matching token");
            ctx.close_pending = true;
            return;
        }
    };
    let info = ConnInfo {
        token_id: token_id.clone(),
        hostname: params.hostname,
        peer: peer.to_owned(),
        client_id: client_id.clone(),
        features: Vec::new(),
        harnesses: Vec::new(),
        now_ms: now_ms(),
    };
    let Some((seq, _fc_rx)) = registry.register(&info, tx.clone()) else {
        send_error(tx, env.id(), Code::Unauthenticated, "no matching token");
        ctx.close_pending = true;
        return;
    };
    // The join response, then a normal close (join is one-shot: the socket is
    // closed after replying; the caller's `finish` flushes it). `close_pending`
    // is set so `finish` waits for the writer to put the result + close on the
    // wire before the read half is dropped.
    let doc = serde_json::json!({ "client_id": client_id, "credential": credential });
    send_result(tx, env.id(), Some(&doc));
    // Hold the connection in the registry (so `is_live` / status reflect the
    // join) but close the socket: the `ConnectionHandle` is dropped here, which
    // deregisters the join's registry entry — the socket is one-shot, so the
    // join's entry does not survive the close. A follow-up `circuit/authenticate`
    // on the same socket re-registers the token (superseding the now-gone join
    // entry, a no-op), and *that* socket's entry is the one that stays live.
    let (_handle, _dropped_rx) = ConnectionHandle::new(registry.clone(), token_id, seq);
    ctx.close_pending = true;
}

/// Decode a `circuit/authenticate`'s params (`{token_id, credential,
/// hostname}`), verify the credential, register the socket in the registry
/// (superseding), and send the `AuthOk {ok:true}` response. Returns the
/// connection's [`ConnectionHandle`] on success, or the `(code, message)` to
/// refuse with.
pub async fn handle_auth(
    env: &Envelope,
    _ctx: &mut ConnCtx<'_>,
    tx: &UnboundedSender<Message>,
    state: &HubState,
    registry: &Arc<Registry>,
    peer: &str,
) -> Result<(ConnectionHandle, tokio::sync::oneshot::Receiver<()>, tokio::sync::oneshot::Receiver<()>), (Code, String)> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct AuthParams {
        token_id: String,
        credential: String,
        #[serde(default)]
        hostname: Option<String>,
    }
    let params: AuthParams = match env
        .params()
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
    {
        Some(p) => p,
        None => {
            return Err((
                Code::InvalidParams,
                "authenticate params need {token_id, credential}".to_owned(),
            ))
        }
    };
    let record = match crate::token::verify_credential_async(
        &params.token_id,
        &params.credential,
        state,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return Err((Code::Unauthenticated, e.message.clone()))
        }
    };
    let client_id = record.client_id.clone().unwrap_or_default();
    let hostname = params
        .hostname
        .or_else(|| record.hostname.clone())
        .unwrap_or_default();
    let info = ConnInfo {
        token_id: record.token_id.clone(),
        hostname,
        peer: peer.to_owned(),
        client_id,
        features: Vec::new(),
        harnesses: Vec::new(),
        now_ms: now_ms(),
    };
    let Some((seq, fc_rx)) = registry.register(&info, tx.clone()) else {
        return Err((
            Code::Unauthenticated,
            "no matching token".to_owned(),
        ));
    };
    send_result(tx, env.id(), Some(&serde_json::json!({ "ok": true })));
    let (handle, dropped_rx) = ConnectionHandle::new(registry.clone(), record.token_id, seq);
    Ok((handle, dropped_rx, fc_rx))
}

/// Handle a `circuit/ping` request: reply with `{hostname, ts}`.
pub fn handle_ping(env: &Envelope, tx: &UnboundedSender<Message>, hostname: &str) {
    send_result(
        tx,
        env.id(),
        Some(&serde_json::json!({
            "hostname": hostname,
            "ts": now_ms(),
        })),
    );
}
