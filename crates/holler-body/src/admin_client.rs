//! A one-shot **admin** client (issue #508/#509, epic #506): dial a hub, run
//! the existing body handshake unchanged (`connection::handshake::
//! authenticate`), send one `circuit/hello` with `role: "admin"`
//! (`connection::handshake::hello_exchange`, given a role parameter by this
//! same story so the hub-key pinning is not duplicated), then send exactly
//! one `admin/*` request and read its response.
//!
//! Reuses this process's own persisted identity (MO 7: "the same x25519
//! identity + token, not a new credential") — this module never mints or
//! persists anything of its own, and the identity file on disk is never
//! rewritten. The `--server` URL a caller passes may differ from the one the
//! body joined with (the brief's Clarifications): this fn clones the loaded
//! [`BodyIdentity`] and only overrides the clone's `server_url` before
//! dialling, so `handshake::authenticate`'s Noise prologue and the dial
//! target both use the given `--server`.

use std::path::Path;
use std::time::Duration;

use futures_util::{SinkExt, Stream, StreamExt};
use holler_proto::{CorrelationId, Envelope, HelloRole};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::connection::handshake;
use crate::identity::BodyIdentity;

/// Why an admin request could not be sent, or its reply read (issue #508).
#[derive(Debug)]
pub enum AdminClientError {
    /// This process has never `body join`ed — no `body/credential.json` at
    /// the named path.
    NotJoined(std::path::PathBuf),
    /// The persisted identity, or its X25519 key, is corrupt/unreadable, or
    /// (A's Phase 3 finding W-3) the X25519 key file is simply missing —
    /// checked before dialling, so a half-joined state dir is never written
    /// to by this path.
    Identity(String),
    /// Could not connect (DNS, refused, TLS failure, …). Carries the target
    /// URL and the transport's own message.
    Connect(String),
    /// The handshake failed: a `-32002`, a hub-key mismatch, a protocol
    /// mismatch, … — the human words `handshake::authenticate`/
    /// `hello_exchange` already build (the same ones `body run` prints).
    Refused(String),
    /// The socket dropped, or a frame did not parse, after a successful
    /// handshake — including "no reply within the timeout".
    Dropped(String),
    /// The hub answered the `admin/*` request with a JSON-RPC error.
    Wire(holler_proto::WireError),
}

impl std::fmt::Display for AdminClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminClientError::NotJoined(p) => write!(f, "{} not found; run `holler body join` first", p.display()),
            AdminClientError::Identity(m)
            | AdminClientError::Connect(m)
            | AdminClientError::Refused(m)
            | AdminClientError::Dropped(m) => write!(f, "{m}"),
            AdminClientError::Wire(e) => write!(f, "{}", e.message),
        }
    }
}

/// Send one `admin/*` request to `server_url`, using this process's own body
/// identity (read from `state_root` — there is no reusable credential string
/// to pass on the command line, per the brief's flag decision), and return
/// its `result`. `method`/`params` are the caller-mapped `admin/…` wire
/// shape (`holler-cli`'s `transport` module builds these from a
/// `holler_hub::control::ControlCall`); this fn only carries them over the
/// wire and back.
pub async fn call(
    state_root: &Path,
    server_url: &str,
    method: &str,
    params: Option<serde_json::Value>,
    timeout: Duration,
) -> Result<serde_json::Value, AdminClientError> {
    let mut identity = load_identity(state_root)?;
    identity.server_url = server_url.to_string();

    let (ws, _) = tokio_tungstenite::connect_async(server_url)
        .await
        .map_err(|e| AdminClientError::Connect(format!("could not reach the hub at {server_url}: {e}")))?;
    let (mut sink, mut stream) = ws.split();

    handshake::authenticate(&mut sink, &mut stream, &identity, state_root)
        .await
        .map_err(attempt_words)
        .map_err(AdminClientError::Refused)?;
    handshake::hello_exchange(&mut sink, &mut stream, &identity, &[], HelloRole::Admin)
        .await
        .map_err(attempt_words)
        .map_err(AdminClientError::Refused)?;

    let cid = CorrelationId::mint_body();
    let req = Envelope::request(&cid, method, params);
    let text = holler_proto::encode(&req).map_err(|e| AdminClientError::Dropped(format!("encode request: {e}")))?;
    sink.send(Message::text(text)).await.map_err(|e| AdminClientError::Dropped(format!("send request: {e}")))?;
    sink.flush().await.map_err(|e| AdminClientError::Dropped(format!("send request: {e}")))?;

    let result = tokio::time::timeout(timeout, read_reply(&mut stream, &cid))
        .await
        .map_err(|_| AdminClientError::Dropped(format!("no reply from the hub within {}s", timeout.as_secs())))??;

    let _ = sink.send(Message::Close(None)).await;
    let _ = sink.flush().await;
    Ok(result)
}

/// Resolve this process's own body identity, checked in the same fail-closed
/// order `handshake::authenticate` itself would fail in, but *before* any
/// dial: a missing credential is [`AdminClientError::NotJoined`]; a present
/// credential whose X25519 identity key is missing is
/// [`AdminClientError::Identity`] (A's Phase 3 finding W-3) — without this
/// check, `handshake::authenticate` would call `x25519_identity::ensure`,
/// which **generates and persists a new key** into a half-joined state dir
/// before failing on the mismatch it cannot avoid. This path must never write
/// to the state dir.
fn load_identity(state_root: &Path) -> Result<BodyIdentity, AdminClientError> {
    let identity = match crate::identity::load(state_root) {
        None => return Err(AdminClientError::NotJoined(BodyIdentity::path(state_root))),
        Some(Err(e)) => return Err(AdminClientError::Identity(e.to_string())),
        Some(Ok(identity)) => identity,
    };
    let key_path = crate::x25519_identity::identity_path(state_root);
    if !key_path.exists() {
        return Err(AdminClientError::Identity(format!(
            "{} is missing; run `holler body join`",
            key_path.display()
        )));
    }
    Ok(identity)
}

/// The plain-words message an [`crate::connection::Attempt`] carries — both
/// of `handshake::authenticate`/`hello_exchange`'s error arms only ever
/// return `AuthFailed`/`Dropped` (never `Ended`, which belongs to the live
/// reconnect loop this one-shot client does not run); the third arm stays
/// for exhaustiveness, fail-closed rather than a `match` that could panic.
fn attempt_words(a: crate::connection::Attempt) -> String {
    match a {
        crate::connection::Attempt::AuthFailed(m) | crate::connection::Attempt::Dropped(m) => m,
        crate::connection::Attempt::Ended(_) => "the connection ended unexpectedly".to_string(),
    }
}

/// Read frames until the response (or error) matching `cid` arrives,
/// skipping ping/pong/raw/binary frames — mirrors `connection::
/// next_envelope`, but this module is a short-lived one-shot client with no
/// live loop of its own to fold it into.
async fn read_reply<St>(stream: &mut St, cid: &CorrelationId) -> Result<serde_json::Value, AdminClientError>
where
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    loop {
        match stream.next().await {
            Some(Ok(Message::Text(t))) => {
                let env = holler_proto::decode(&t).map_err(|e| AdminClientError::Dropped(e.to_string()))?;
                match env {
                    Envelope::Response { id, result } if id == cid.as_str() => return Ok(result.unwrap_or_default()),
                    Envelope::Error { id, error } if id.as_deref() == Some(cid.as_str()) => {
                        return Err(AdminClientError::Wire(error));
                    }
                    _ => continue,
                }
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_) | Message::Binary(_))) => continue,
            Some(Ok(Message::Close(_))) | None => {
                return Err(AdminClientError::Dropped("the hub closed the socket before replying".to_string()));
            }
            Some(Err(e)) => return Err(AdminClientError::Dropped(e.to_string())),
        }
    }
}
