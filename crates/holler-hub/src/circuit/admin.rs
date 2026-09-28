//! The hub side of an **admin** connection (issue #508/#509, epic #506):
//! once `authenticate_and_hello` completes with `hello.role == "admin"`,
//! [`super::handle_authenticated`] branches here instead of the normal body
//! session loop ([`super::SessionConnection`]) — no supersede, no
//! `registry.insert`, no roster row, ever (AC 3-4). This module never
//! touches [`crate::live::Registry`]'s body-slot bookkeeping or
//! [`crate::roster::Roster`] at all — an admin socket is deliberately kept
//! out of both (A's Phase 3 finding #6).
//!
//! Each `admin/*` request is mapped to the matching allowlisted verb and
//! dispatched through [`crate::control_server::dispatch_allowlisted`] — the
//! exact same code the Unix control socket runs (MO 3: an allowlist, not a
//! generic passthrough). Every request runs on its own spawned task (A's
//! Phase 3 finding #4): the handlers it calls borrow `&Registry`/`&Roster`,
//! which are cheap to clone, so a 600s `admin/say` never blocks a concurrent
//! `admin/roster` on the same socket, and this loop's own WS-ping and
//! liveness handling keep running while one is in flight (MO 5, AC 16).
//! Replies return over an mpsc channel back to this loop — the socket's sole
//! writer; a send is silently dropped once the socket itself is gone (the
//! request still ran to completion hub-side — the same "finish the turn,
//! discard the reply" contract the Unix control socket already has for a
//! client that goes away mid-`say`, `control_server.rs`'s own
//! `handle_control_conn` doc).

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use holler_proto::log::Severity;
use holler_proto::{Code, CorrelationId, Envelope, EnvelopeError, PingAck, WireError};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use crate::live::Registry;
use crate::lockout::Lockout;
use crate::roster::Roster;

use super::{liveness_timeout, log, ws_ping_interval};

/// The shared, cheap-to-clone hub state one admin request needs to reach its
/// allowlisted handler (issue #508) — bundled so [`run`] stays under
/// clippy's argument-count gate, the same discipline `AuthDeps`/
/// `CommandChannels` use elsewhere in this module's own parent.
#[derive(Clone)]
pub(super) struct AdminDeps {
    pub(super) registry: Registry,
    pub(super) roster: std::sync::Arc<Roster>,
    pub(super) lockout: std::sync::Arc<Lockout>,
}

/// The seven `admin/*` names (MO 2) mapped to the allowlisted verb
/// [`crate::control_server::dispatch_allowlisted`] takes — `admin/query`
/// resolves to `"query_local"`/`"query_remote"` in [`handle_request`]
/// itself (MO 2: it maps to `control/query_remote` when `params.target` is
/// present, `control/query_local` otherwise), so it is not listed here.
fn allowlisted_verb(method: &str) -> Option<&'static str> {
    match method {
        "admin/status" => Some("status"),
        "admin/roster" => Some("roster"),
        "admin/say" => Some("say"),
        "admin/interrupt" => Some("interrupt"),
        "admin/answer" => Some("answer"),
        "admin/wait" => Some("wait"),
        _ => None,
    }
}

/// Run the admin session loop until the socket ends (issue #508). Unlike
/// [`super::SessionConnection`], this never touches the roster or the
/// registry's body-slot bookkeeping — `record`/`peer`/`sas` are used only
/// for the `admin_connected`/`admin_dropped` log lines (MO 5).
pub(super) async fn run<Snk, St>(sink: &mut Snk, stream: &mut St, record: &crate::token::Record, peer: &str, sas: &str, deps: AdminDeps)
where
    Snk: Sink<Message, Error = WsError> + Unpin,
    St: Stream<Item = Result<Message, WsError>> + Unpin,
{
    log(
        Severity::Info,
        "admin_connected",
        vec![
            ("token_id", record.token_id.clone()),
            ("label", record.label.clone()),
            ("peer", peer.to_string()),
            ("sas", sas.to_string()),
        ],
    );

    let (reply_tx, mut reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let mut ws_ping = tokio::time::interval(ws_ping_interval());
    ws_ping.tick().await; // the first tick fires immediately; consume it.
    let mut last_frame_at = tokio::time::Instant::now();
    // How many spawned requests have not yet sent their reply back (MO 5:
    // "An admin connection is never timed out while a request is in
    // flight") — the liveness-timeout arm below is disabled whenever this
    // is non-zero, exactly mirroring AC 16(b).
    let mut inflight: u32 = 0;

    loop {
        tokio::select! {
            biased;
            frame = stream.next() => {
                match frame_outcome(frame, &deps, &reply_tx) {
                    FrameOutcome::Continue { fresh_frame } => last_frame_at = fresh_frame,
                    FrameOutcome::Spawned { fresh_frame } => {
                        last_frame_at = fresh_frame;
                        inflight = inflight.saturating_add(1);
                    }
                    FrameOutcome::Rejected => {}
                    FrameOutcome::Ended => break,
                }
            }
            _ = tokio::time::sleep_until(last_frame_at + liveness_timeout()), if inflight == 0 => break,
            _ = ws_ping.tick() => {
                if send_ping(sink).await.is_err() { break; }
            }
            reply = reply_rx.recv() => {
                let Some(reply) = reply else { continue };
                inflight = inflight.saturating_sub(1);
                if send_reply(sink, reply).await.is_err() { break; }
            }
        }
    }

    log(
        Severity::Warn,
        "admin_dropped",
        vec![
            ("token_id", record.token_id.clone()),
            ("label", record.label.clone()),
            ("peer", peer.to_string()),
            ("sas", sas.to_string()),
        ],
    );
}

/// One inbound WS poll's outcome for [`run`]'s own `select!` arm — split out
/// so that arm stays a single-line match instead of nested nested `match`es,
/// keeping [`run`]'s cognitive-complexity score under the workspace
/// threshold (the same reason `circuit.rs`'s own `SessionConnection::run`
/// delegates each of its arms to a small helper).
enum FrameOutcome {
    /// A ping/pong/raw control frame: only the liveness clock moved.
    Continue { fresh_frame: tokio::time::Instant },
    /// A request frame was serviced (directly, or as a spawned task).
    Spawned { fresh_frame: tokio::time::Instant },
    /// Not a decodable JSON-RPC frame (issue #508, handoff-S REWORK item 1):
    /// [`handle_request`] already answered it with an error reply, but the
    /// liveness clock deliberately does not move — untrusted noise a client
    /// never meant as a request must not keep an otherwise-idle socket alive.
    Rejected,
    /// The socket closed, errored, or hit EOF.
    Ended,
}

fn frame_outcome(frame: Option<Result<Message, WsError>>, deps: &AdminDeps, reply_tx: &tokio::sync::mpsc::UnboundedSender<String>) -> FrameOutcome {
    let now = tokio::time::Instant::now();
    match frame {
        Some(Ok(Message::Text(t))) => match handle_request(&t, deps, reply_tx) {
            RequestOutcome::Spawned => FrameOutcome::Spawned { fresh_frame: now },
            RequestOutcome::Inline => FrameOutcome::Continue { fresh_frame: now },
            RequestOutcome::Rejected => FrameOutcome::Rejected,
        },
        Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => FrameOutcome::Continue { fresh_frame: now },
        _ => FrameOutcome::Ended,
    }
}

/// Send a WS-level Ping control frame (mirrors `circuit.rs`'s own
/// `SessionConnection::send_ws_ping`).
async fn send_ping<Snk>(sink: &mut Snk) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    sink.send(Message::Ping(Vec::new().into())).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())
}

/// Write one already-encoded reply frame to the socket.
async fn send_reply<Snk>(sink: &mut Snk, body: String) -> Result<(), ()>
where
    Snk: Sink<Message, Error = WsError> + Unpin,
{
    sink.send(Message::text(body)).await.map_err(|_| ())?;
    sink.flush().await.map_err(|_| ())
}

/// [`handle_request`]'s outcome, for [`frame_outcome`] to turn into a
/// [`FrameOutcome`]. Distinct from `FrameOutcome` because a ping/pong/raw WS
/// frame (handled directly in `frame_outcome`) never reaches `handle_request`
/// at all.
enum RequestOutcome {
    /// A task was spawned (the caller's `inflight` counter tracks exactly
    /// these).
    Spawned,
    /// Answered without spawning a task (a `circuit/ping`, or a decodable
    /// request naming a method outside the allowlist).
    Inline,
    /// Not a decodable JSON-RPC frame at all (issue #508, handoff-S REWORK
    /// item 1) — answered with an error reply by [`reply_decode_error`], but
    /// the caller must not treat this as proof of a live request.
    Rejected,
}

/// Decode one inbound text frame and either answer it directly (a
/// `circuit/ping`, a decodable-but-unrecognised method's `-32601`, or an
/// undecodable frame's own error reply, issue #508 handoff-S REWORK item 1)
/// or spawn it as its own task (A's Phase 3 finding #4). A notification —
/// `session/presence`/`session/update` included — has no `id` and is
/// dropped without being serviced or touching the roster (MO 4: an admin
/// socket can never publish presence).
fn handle_request(text: &str, deps: &AdminDeps, reply_tx: &tokio::sync::mpsc::UnboundedSender<String>) -> RequestOutcome {
    let env = match holler_proto::decode(text) {
        Ok(env) => env,
        Err(e) => {
            reply_decode_error(text, &e, reply_tx);
            return RequestOutcome::Rejected;
        }
    };
    let Some(id) = env.id() else { return RequestOutcome::Inline };
    let Some(method) = env.method() else { return RequestOutcome::Inline };
    let Ok(cid) = CorrelationId::parse(id) else { return RequestOutcome::Inline };

    // MO 4: anything other than `admin/*` and `circuit/ping` is `-32601` on
    // an admin connection — a body-shaped request (`session/prompt`, a bare
    // `control/*` name, …) never reaches a handler here.
    if method == "circuit/ping" {
        let ack = PingAck { hostname: "hub-admin".to_string(), ts: holler_proto::now_millis() };
        let body = holler_proto::encode(&Envelope::response(&cid, serde_json::to_value(ack).ok())).unwrap_or_default();
        let _ = reply_tx.send(body);
        return RequestOutcome::Inline;
    }

    let Ok(obj) = serde_json::from_str::<serde_json::Value>(text) else { return RequestOutcome::Inline };
    let verb = if method == "admin/query" {
        // MO 2: maps to `control/query_remote` when `params.target` is
        // present, `control/query_local` when it is absent — the same
        // shape `hub_query_remote`/`hub_query_local` already read `obj`
        // for, unchanged.
        let has_target = obj.get("params").and_then(|p| p.get("target")).and_then(|v| v.as_str()).is_some();
        Some(if has_target { "query_remote" } else { "query_local" })
    } else {
        allowlisted_verb(method)
    };
    let Some(verb) = verb else {
        let err = WireError::new(Code::MethodNotFound, "unknown method", None);
        let body = holler_proto::encode(&Envelope::error_frame(&cid, &err)).unwrap_or_default();
        let _ = reply_tx.send(body);
        return RequestOutcome::Inline;
    };

    let (deps, reply_tx, verb) = (deps.clone(), reply_tx.clone(), verb.to_string());
    tokio::spawn(async move {
        let body = crate::control_server::dispatch_allowlisted(&verb, &cid, &obj, &deps.registry, &deps.roster, &deps.lockout)
            .await
            .unwrap_or_default();
        // Discarded (never observed) once the loop above has already ended
        // and dropped its receiver — MO 5's "if the client disconnects
        // mid-`say`, the hub finishes the turn and discards the reply".
        let _ = reply_tx.send(body);
    });
    RequestOutcome::Spawned
}

/// Reply to a frame `holler_proto::decode` could not turn into an
/// [`Envelope`] at all (issue #508, handoff-S REWORK item 1) instead of
/// dropping it silently. Five of the six methods AC 5 names —
/// `control/revoke`, `control/test_drop`, `admin/revoke`, `admin/hold`,
/// `admin/release` — are not in `methods.rs`'s `CATALOG`, so `decode` fails
/// them with [`EnvelopeError::UnknownMethod`] before a method-name `match`
/// ever runs; without this, a hand-rolled client sending one got no reply at
/// all and hung on its own timeout. Carries the request `id` back when the
/// raw JSON has one — a well-formed request naming an uncatalogued method
/// still has a valid id — via a plain best-effort read of the raw text,
/// since `decode` already failed and there is no [`Envelope`] to read an id
/// off. Uses the code the decode failure itself maps to (docs §8; `-32601`
/// for an unlisted method), the same way the body loop's own decode-failure
/// reply does at `circuit.rs:812-817` (that path always sends an unkeyed
/// error; this one recovers the id when it can, since an admin client has no
/// other way to correlate the reply with its request).
fn reply_decode_error(text: &str, err: &EnvelopeError, reply_tx: &tokio::sync::mpsc::UnboundedSender<String>) {
    let id = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v.get("id").and_then(|i| i.as_str()).map(str::to_owned));
    let error = WireError::new(err.code(), err.to_string(), None);
    let frame = match id.as_deref().and_then(|s| CorrelationId::parse(s).ok()) {
        Some(cid) => Envelope::error_frame(&cid, &error),
        None => Envelope::Error { id, error },
    };
    let body = holler_proto::encode(&frame).unwrap_or_default();
    let _ = reply_tx.send(body);
}

#[cfg(test)]
mod tests {
    use super::allowlisted_verb;

    /// **AC 5 (third bullet).** [`allowlisted_verb`] plus `handle_request`'s
    /// own `admin/query` special-case (not listed here — it never reaches
    /// this function, see its own doc comment) is the complete admin
    /// allowlist: exactly the seven `admin/*` catalog rows `methods.rs`
    /// defines for issue #508/#509, no more and no fewer. This pins the
    /// allowlist's shape directly, so a future accidental addition (or
    /// removal) of a mapped verb fails here instead of only being visible as
    /// a missing/extra wire refusal.
    #[test]
    fn allowlist_is_exactly_the_six_delegated_admin_verbs() {
        let mapped: Vec<(&str, &str)> = [
            ("admin/status", "status"),
            ("admin/roster", "roster"),
            ("admin/say", "say"),
            ("admin/interrupt", "interrupt"),
            ("admin/answer", "answer"),
            ("admin/wait", "wait"),
        ]
        .to_vec();
        for (method, verb) in &mapped {
            assert_eq!(allowlisted_verb(method), Some(*verb), "{method} must map to allowlisted verb {verb}");
        }

        // Nothing else — not `admin/query` (handled separately in
        // `handle_request`), not a plain `control/*` name, not a body-shaped
        // method — is in this map.
        for other in [
            "admin/query",
            "admin/revoke",
            "admin/hold",
            "admin/release",
            "control/revoke",
            "control/test_drop",
            "session/prompt",
            "circuit/ping",
            "",
        ] {
            assert_eq!(allowlisted_verb(other), None, "{other} must not be in the allowlisted-verb map");
        }
    }
}
