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
use crate::registry::{ConnectionHandle, ConnInfo, Registry};
use crate::serve::ConnCtx;
use crate::wire_io::{now_ms, send_result};

/// Decode a `circuit/join`'s params (`{secret, hostname}`), redeem the secret
/// from the token store, register the socket in the registry (superseding any
/// older socket on the same token), and send the `JoinResult`
/// (`{client_id, credential}`) response. Returns the connection's
/// [`ConnectionHandle`] (and the bound client id) on success, or the
/// `(code, message)` to refuse with.
pub async fn handle_join(
    env: &Envelope,
    _ctx: &mut ConnCtx<'_>,
    tx: &UnboundedSender<Message>,
    state: &HubState,
    registry: &Arc<Registry>,
    peer: &str,
    force_close: &Arc<tokio::sync::oneshot::Sender<()>>,
) -> Result<(ConnectionHandle, String), (Code, String)> {
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
            return Err((
                Code::InvalidParams,
                "join params need {secret, hostname}".to_owned(),
            ))
        }
    };
    let (client_id, credential) =
        match crate::token::redeem_async(&params.secret, &params.hostname, state).await {
            Ok(rc) => rc,
            Err(e) => {
                return Err((Code::Unauthenticated, e.message().to_owned()))
            }
        };
    // `redeem` returns only (client_id, credential); the token id is the
    // store's id, so look the (now bound) record up by the client id.
    let token_id = match crate::token::list(state)
        .ok()
        .and_then(|rows| rows.into_iter().find(|r| r.client_id.as_deref() == Some(&client_id)))
        .map(|r| r.token_id)
    {
        Some(id) => id,
        None => {
            return Err((Code::Unauthenticated, "no matching token".to_owned()))
        }
    };
    // The connection's `force_close` sender (held in the connection task; see
    // `serve::handle_ws_conn`) goes on its registry entry so the registry can
    // signal *this* connection's task when it force-closes the socket
    // (supersede by a reauth / a token revoke).
    let info = ConnInfo {
        token_id: token_id.clone(),
        hostname: params.hostname,
        peer: peer.to_owned(),
        client_id: client_id.clone(),
        features: Vec::new(),
        harnesses: Vec::new(),
        now_ms: now_ms(),
        force_close: force_close.clone(),
    };
    let Some(seq) = registry.register(&info, tx.clone()) else {
        return Err((Code::Unauthenticated, "no matching token".to_owned()));
    };
    let doc = serde_json::json!({ "client_id": client_id, "credential": credential });
    send_result(tx, env.id(), Some(&doc));
    Ok((
        ConnectionHandle::new(registry.clone(), token_id, seq),
        client_id,
    ))
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
    force_close: &Arc<tokio::sync::oneshot::Sender<()>>,
) -> Result<ConnectionHandle, (Code, String)> {
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
    // The connection's `force_close` sender (held in the connection task; see
    // `serve::handle_ws_conn`) goes on its registry entry so the registry can
    // signal *this* connection's task when it force-closes the socket
    // (supersede by a reauth / a token revoke).
    let info = ConnInfo {
        token_id: record.token_id.clone(),
        hostname,
        peer: peer.to_owned(),
        client_id,
        features: Vec::new(),
        harnesses: Vec::new(),
        now_ms: now_ms(),
        force_close: force_close.clone(),
    };
    let Some(seq) = registry.register(&info, tx.clone()) else {
        return Err((
            Code::Unauthenticated,
            "no matching token".to_owned(),
        ));
    };
    send_result(tx, env.id(), Some(&serde_json::json!({ "ok": true })));
    Ok(ConnectionHandle::new(registry.clone(), record.token_id, seq))
}
