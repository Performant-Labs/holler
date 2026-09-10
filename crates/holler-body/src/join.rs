//! `body join` (story #176): redeem a one-time join secret over the wire and
//! persist the body's identity.
//!
//! The flow is the protocol's one-shot bootstrap (docs §3): the body opens a
//! **fresh** WebSocket, sends **one** `circuit/join` request (`{secret,
//! hostname}`), and awaits **one** response — the hub's `{client_id,
//! credential}` — after which the hub **closes the socket**. Join never leads
//! into talk on the same socket.
//!
//! The exit-code semantics are the CLI's (ADR 0003), applied here:
//! - exit 0 — the join succeeded; the identity is persisted (0600) and the
//!   operator is told. The one-time join **secret is never printed or
//!   persisted** — the success line names the `client_id` and `token_id`,
//!   never the secret.
//! - exit 1 — a runtime failure: the hub refused the redeem (its
//!   `join_failed` message reaches stderr), the hub was unreachable, the
//!   socket closed early, or the state dir could not be written.
//! - exit 3 — a fail-closed **policy** refusal: a plaintext `ws://` to a
//!   non-loopback host (ADR 0002). This is checked **before** any connection
//!   is attempted.
//!
//! The `ws://` client transport is plain TCP; `wss://` is rustls with the
//! system native root CAs (pinned at the workspace) and **fails closed** on any
//! certificate problem (an unknown or invalid CA is a `Connect` error → exit
//! 1, never a silent plaintext downgrade).

use futures_util::{SinkExt, StreamExt};
use holler_proto::{CorrelationId, Envelope, Join, JoinResult};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{protocol::frame::Utf8Bytes, Message};

use crate::identity::JoinError;
use crate::server_address::{loopback_only_check, parse};

/// The CLI exit codes (ADR 0003). (Not `Eq` — the `Refused` variant carries a
/// `JoinError`, whose payloads compare with `PartialEq` only.)
#[derive(Debug, Clone, PartialEq)]
pub enum JoinExit {
    /// Joined; the identity is persisted. The bin exits 0.
    Ok,
    /// A runtime failure (the reason is in the `JoinError`); the bin exits 1.
    Refused(JoinError),
    /// A fail-closed policy refusal (a plaintext non-loopback `ws://`); the
    /// bin exits 3.
    Policy,
}

/// `body join --server <url> --token <ID:SECRET>`.
///
/// Returns the [`JoinExit`] the CLI bin turns into an exit code. A
/// [`JoinExit::Ok`] has already persisted `<state>/body/credential.json`
/// (0600). A `Refused` carries the one-line reason for stderr. A `Policy`
/// refusal made no connection and wrote nothing.
pub fn join(state_root: &std::path::Path, server: &str, token: &str, hostname: &str) -> JoinExit {
    // 1. Parse + policy-check the address *before* any I/O: a plaintext
    //    non-loopback `ws://` is exit 3, with no connection attempted.
    let addr = match parse(server) {
        Ok(a) => a,
        Err(e) => {
            let e = JoinError::BadAddress(e.to_string());
            eprintln!("error: {}", e.message());
            return JoinExit::Refused(e);
        }
    };
    if let Some(reason) = loopback_only_check(&addr) {
        eprintln!("error: {reason}");
        return JoinExit::Policy;
    }

    // 2. Split the `--token` as `ID:SECRET`.
    let (token_id, secret) = match split_token(token) {
        Some(pair) => pair,
        None => {
            let e = JoinError::MalformedToken("the join token must be ID:SECRET".to_string());
            eprintln!("error: {}", e.message());
            return JoinExit::Refused(e);
        }
    };

    // 3. Rebuild the canonical URL (explicit port, default 41807 when absent)
    //    so the connect uses what `parse` resolved, not the operator's raw arg.
    let url = format!("{}://{}:{}", addr.scheme, addr.host, addr.port);

    // 4. Connect and drive the one-shot handshake on a throwaway runtime
    //    (the body is a short-lived CLI with no runtime in scope).
    match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Err(e) => {
            let e = JoinError::Connect(format!("runtime: {e}"));
            eprintln!("error: {}", e.message());
            JoinExit::Refused(e)
        }
        Ok(rt) => {
            rt.block_on(connect_and_join(state_root, &url, token_id, secret, hostname, &url))
        }
    }
}

fn split_token(token: &str) -> Option<(&str, &str)> {
    let (id, secret) = token.split_once(':')?;
    if id.is_empty() || secret.is_empty() {
        return None;
    }
    Some((id, secret))
}

/// Connect to `url`, send the `circuit/join` request, await the (single)
/// response, and — on success — persist the identity. Returns a [`JoinExit`].
async fn connect_and_join(
    state_root: &std::path::Path,
    url: &str,
    token_id: &str,
    secret: &str,
    hostname: &str,
    server_url: &str,
) -> JoinExit {
    // `wss://` is TLS: rustls 0.23 takes its crypto from a process-global
    // provider that the CLI installs in `main` before the runtime. If none is
    // installed (the build is missing a rustls provider, or the caller bypassed
    // `main`), fail closed with a clear reason rather than letting the connect
    // die opaquely — and never downgrade to plaintext.
    if url.starts_with("wss://") && rustls::crypto::CryptoProvider::get_default().is_none() {
        let e = JoinError::Connect(
            "no TLS crypto provider is installed for a wss:// connection".to_string(),
        );
        eprintln!("error: {}", e.message());
        return JoinExit::Refused(e);
    }
    let ws = match connect_async(url).await {
        Ok((ws, _)) => ws,
        Err(e) => {
            let e = JoinError::Connect(e.to_string());
            eprintln!("error: {}", e.message());
            return JoinExit::Refused(e);
        }
    };

    // The body's correlation id for the join request.
    let cid = CorrelationId::mint_body();
    let params = match serde_json::to_value(Join {
        secret: secret.to_string(),
        hostname: hostname.to_string(),
    }) {
        Ok(p) => p,
        Err(e) => {
            let e = JoinError::Wire(format!("could not serialise the join request: {e}"));
            eprintln!("error: {}", e.message());
            return JoinExit::Refused(e);
        }
    };
    let req = Envelope::request(&cid, "circuit/join", Some(params));
    let text = match holler_proto::encode(&req) {
        Ok(t) => t,
        Err(e) => {
            let e = JoinError::Wire(format!("could not encode the join request: {e}"));
            eprintln!("error: {}", e.message());
            return JoinExit::Refused(e);
        }
    };
    let mut ws = match send_text(ws, text).await {
        Ok(ws) => ws,
        Err(e) => {
            let e = JoinError::Connect(format!("the socket closed while sending the join: {e}"));
            eprintln!("error: {}", e.message());
            return JoinExit::Refused(e);
        }
    };

    // Await the hub's single answer. The hub closes the socket after
    // replying (docs §3), so a read that hits EOF without a response is a
    // failure, not a hang: the loop below terminates on either.
    loop {
        // Normalize the next frame to a `&str` of its payload. The hub only
        // ever sends text, so the text arm is the normal path; a (defensive)
        // binary frame is converted to UTF-8, and a non-UTF-8 one is skipped
        // (the hub never sends binary). The owned payload is bound in
        // `payload` in the loop scope — *outside* the match whose arms borrow
        // it — so the `&str` below does not outlive the data it names
        // (a binding-pattern `Some(Ok(Message::Text(t)))` would drop `t` at
        // the end of its arm, while `next` still borrows it).
        let payload: Option<Utf8Bytes> = match ws.next().await {
            None => break JoinExit::Refused(JoinError::Closed),
            // `as_ref` gives a `&Utf8Bytes` that we keep (and borrow below)
            // instead of moving the frame out — the socket still owns the
            // stream and must not be drained of ownership per frame.
            Some(Ok(Message::Text(t))) => Some(t.clone()),
            Some(Ok(Message::Binary(b))) => match Utf8Bytes::try_from(b) {
                Ok(t) => Some(t),
                Err(_) => continue, // not UTF-8: ignore this frame.
            },
            Some(Ok(Message::Ping(_)))
            | Some(Ok(Message::Pong(_)))
            | Some(Ok(Message::Frame(_))) => continue,
            Some(Ok(Message::Close(_))) => break JoinExit::Refused(JoinError::Closed),
            Some(Err(e)) => {
                let e = JoinError::Connect(format!("connection lost: {e}"));
                eprintln!("error: {}", e.message());
                return JoinExit::Refused(e);
            }
        };
        // A frame reached the loop (the arms above `break`/`continue` on the
        // empty case), so `payload` is `Some` here; match on it so the borrow
        // of `next` does not outlive `payload`.
        let next: &str = match payload.as_deref() {
            Some(s) => s,
            None => continue, // defensive: never reached (bound above).
        };
        // Decode the frame and act on its shape.
        match holler_proto::decode(next) {
            Ok(Envelope::Response { id, result }) if id == cid.as_str() => {
                return on_response(state_root, token_id, hostname, server_url, result)
            }
            Ok(Envelope::Error { error, .. }) => {
                // A `join_failed` (or any other) error: report the hub's
                // message verbatim (exit 1). The hub then closes the socket.
                eprintln!("error: {}", error.message);
                return JoinExit::Refused(JoinError::HubRefused(error.message.clone()));
            }
            Ok(_) => continue, // a notification or a foreign frame — ignore.
            Err(e) => {
                let e = JoinError::Wire(e.to_string());
                eprintln!("error: {}", e.message());
                return JoinExit::Refused(e);
            }
        }
    }
}

/// Handle the hub's `circuit/join` response: on a result, deserialize
/// `{client_id, credential}`, persist the identity (0600), and report success
/// (without ever printing the secret).
fn on_response(
    state_root: &std::path::Path,
    token_id: &str,
    hostname: &str,
    server_url: &str,
    result: Option<serde_json::Value>,
) -> JoinExit {
    let result = match result {
        Some(v) => v,
        None => {
            eprintln!("error: the hub's join reply carried no result");
            return JoinExit::Refused(JoinError::HubRefused(
                "the hub's join reply carried no result".to_string(),
            ));
        }
    };
    let join_result: JoinResult = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("error: the hub's join reply was malformed");
            return JoinExit::Refused(JoinError::HubRefused(
                "the hub's join reply was malformed".to_string(),
            ));
        }
    };
    let identity = crate::identity::BodyIdentity {
        client_id: join_result.client_id.clone(),
        credential: join_result.credential,
        token_id: token_id.to_string(),
        hostname: hostname.to_string(),
        server_url: server_url.to_string(),
        joined_at: holler_proto::now_secs(),
    };
    if let Err(e) = crate::identity::save(&identity, state_root) {
        let e = JoinError::Io(e.to_string());
        eprintln!("error: {}", e.message());
        return JoinExit::Refused(e);
    }
    // Success — and the operator is told the *identity* (the server it joined
    // and the client id it minted), never the one-time join secret.
    println!("joined {} as client_id={}", server_url, join_result.client_id);
    JoinExit::Ok
}

/// Send one text frame and flush, returning the (flushed) socket.
async fn send_text<S>(mut ws: S, text: String) -> Result<S, S::Error>
where
    S: futures_util::Sink<Message> + Unpin,
{
    ws.send(Message::Text(text.into())).await?;
    ws.flush().await?;
    Ok(ws)
}

