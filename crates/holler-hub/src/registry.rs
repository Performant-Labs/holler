//! The hub's live-connection registry (story #184).
//!
//! One live WebSocket per token, keyed by `token_id` (the token store makes
//! token ids unique, so the map is keyed by one).
//!
//! Supersede-on-insert: when a body (re)connects on a token that already has
//! a live socket, the **old** socket is superseded — it receives a
//! `circuit/superseded` notification and is closed with code **1000** — and
//! the new socket takes the slot. A token that is revoked (or deleted) is
//! removed from the registry; if it was live, its socket is closed with code
//! **1008**.
//!
//! Ownership: the registry holds each connection's write half (`sink`) and a
//! monotonic `seq`. A connection's task is given a [`ConnectionHandle`]: the
//! handle's `Drop` deregisters the connection, so a socket that goes away
//! (peer closed, pre-auth timeout, …) removes itself from the registry without
//! the caller remembering to. `Drop` never panics, and it removes *only if
//! this connection is still the incumbent* (by `seq`), so a connection that
//! was superseded never removes the socket that replaced it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{
    protocol::{
        frame::{coding::CloseCode, CloseFrame},
        Message,
    },
};

/// The public, read-only view of a live connection (the fields `hub status
/// --json` reports, plus the map key). `Clone` because the registry's
/// `get()` hands a copy out without holding the map's lock — and because the
/// per-connection channel/oneshot handles live *outside* this struct (in
/// [`RegistryEntry`]) they do not constrain its `Clone`.
#[derive(Clone)]
pub struct ClientInfo {
    pub seq: u64,
    pub token_id: String,
    pub hostname: String,
    pub peer: String,
    pub client_id: String,
    pub features: Vec<String>,
    pub harnesses: Vec<String>,
    pub connected_at: u64,
    pub last_frame_at: u64,
}

/// One live body connection in the registry. The public view is a
/// [`ClientInfo`]; the rest are the per-connection channel/oneshot handles the
/// registry owns (none are `Clone`-able, so this struct is **not** `Clone` —
/// `get()` returns the cloneable [`ClientInfo`] view instead).
pub struct RegistryEntry {
    /// The public, read-only view (map key + the status-reported fields).
    pub info: ClientInfo,
    /// The write half of the connection's WebSocket. The registry owns it;
    /// sending a close frame here makes the connection task tear the socket
    /// down (the read half observes the peer-closed condition).
    pub sink: mpsc::UnboundedSender<Message>,
    /// The oneshot *sender* the registry holds when *it* (not the peer)
    /// force-closes this socket (supersede / revoke). The connection task
    /// holds the matching *receiver* in `ConnCtx` (as `force_close_rx`); on
    /// supersede or revoke the registry moves the entry out of the map and
    /// calls `send()` on this owned sender, waking the connection task's
    /// `post_auth_loop` (which races it against the read half) and
    /// `ConnCtx::finish` (which races it against `flushed`). A
    /// *peer*-initiated close never signals it: the registry removes the entry
    /// on deregister (dropping the sender), which resolves the receiver as
    /// `Err` — the "no force-close" case — so `finish` returns immediately.
    pub force_close: tokio::sync::oneshot::Sender<()>,
}

/// The shared registry: one entry per live token, behind an `Arc` (the accept
/// loop, the connection tasks, and the token store all share it).
pub struct Registry {
    entries: Mutex<HashMap<String, RegistryEntry>>,
    /// Monotonic counter that stamps each connection's `seq`.
    seq: AtomicU64,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            entries: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
        })
    }

    /// The next connection `seq` number (monotonic, per registry).
    pub fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed)
    }

    /// Register a live connection for `token_id` (superseding any existing
    /// socket on that token). Supersede sends a `circuit/superseded`
    /// notification and a **1000** close to the old socket. Returns the `seq`
    /// stamped on the new entry (the caller builds the [`ConnectionHandle`]
    /// with it). Returns `None` (and registers nothing) if `token_id` is empty.
    pub fn register(
        &self,
        info: &ConnInfo,
        sink: mpsc::UnboundedSender<Message>,
    ) -> Option<(u64, tokio::sync::oneshot::Receiver<()>)> {
        if info.token_id.is_empty() {
            return None;
        }
        let seq = self.next_seq();
        // Create a NEW force_close oneshot pair for this entry (a tokio
        // oneshot `Sender` is not `Clone`; we create a fresh pair rather than
        // cloning the connection's own pair). The *sender* goes into the
        // registry entry (the registry owns it and signals it on
        // supersede/revoke); the *receiver* is returned to the caller, who
        // hands it to the connection task's `ConnCtx` (as `force_close_rx`,
        // raced against the read half in `post_auth_loop`) and to `finish`
        // (as `force_close`, raced against `flushed`).
        let (fc_tx, fc_rx) = tokio::sync::oneshot::channel::<()>();
        let entry = RegistryEntry {
            info: ClientInfo {
                seq,
                token_id: info.token_id.clone(),
                hostname: info.hostname.clone(),
                peer: info.peer.clone(),
                client_id: info.client_id.clone(),
                features: info.features.clone(),
                harnesses: info.harnesses.clone(),
                connected_at: info.now_ms,
                last_frame_at: info.now_ms,
            },
            sink,
            force_close: fc_tx,
        };

        {
            let mut map = self.entries.lock().unwrap_or_else(|e| e.into_inner());
            let superseded = map.contains_key(&info.token_id);
            // Supersede the previous connection on this token (if any) *before*
            // inserting the new entry (the insert replaces the old one under the
            // same key). The registry sends the `circuit/superseded`
            // notification + 1000 close through the OLD entry's sink (the old
            // connection's writer task drains that channel onto the old socket
            // and flushes the close frame to the wire) and then signals the
            // old connection's `force_close`. The old connection's
            // `post_auth_loop` wakes on that signal (it races the read half
            // against the oneshot) and `ConnCtx::finish` awaits the flush
            // signal (`flushed`) before it drops its read half, so the old
            // peer sees the 1000 close code rather than a bare EOF. (A
            // *revoke* works the same way but with a 1008 close and no
            // notification; see `Registry::remove`.)
            if superseded {
                // Take the old entry out of the map so we can move its owned
                // `force_close` sender out (a tokio oneshot `Sender` is not
                // `Clone` and `send` takes it by value). The old entry is NOT
                // re-inserted: the new entry takes the token's slot, and the
                // old connection's `ConnectionHandle::drop` (which fires when
                // the old connection's task ends) finds the new entry in the
                // map, sees that its `seq` is not the incumbent, and does not
                // remove the new entry.
                let Some(old_entry) = map.remove(&info.token_id) else {
                    // A concurrent deregister removed the entry between the
                    // `contains_key` check and this `remove`; the old socket
                    // is already gone, so there is nothing to supersede.
                    map.insert(info.token_id.clone(), entry);
                    return Some((seq, fc_rx));
                };
                let _ = old_entry.sink.send(superseded_notification());
                let _ = old_entry.sink.send(Message::Close(Some(CloseFrame {
                    code: CloseCode::from(1000u16),
                    reason: "superseded".into(),
                })));
                // Signal the old connection's `force_close` so its
                // `post_auth_loop` wakes up (it races the read half against
                // this oneshot) and its `ConnCtx::finish` resolves (it
                // races `flushed` against `force_close`). The sender is
                // owned by the old entry (not shared via `Arc`), so we can
                // call `send()` directly.

                let _ = old_entry.force_close.send(());
                map.insert(info.token_id.clone(), entry);

            } else {
                map.insert(info.token_id.clone(), entry);
            }
        }
        Some((seq, fc_rx))
    }

    /// Remove a token from the registry (revoked or deleted). If the token was
    /// live, close its socket with code **1008** and return `true` (the
    /// caller updates the roster to `gone`); `false` if it was not live.
    pub fn remove(&self, token_id: &str) -> bool {
        if token_id.is_empty() {
            return false;
        }
        let mut map = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = map.remove(token_id) {
            let _ = old.sink.send(Message::Close(Some(CloseFrame {
                code: CloseCode::from(1008u16),
                reason: "revoked".into(),
            })));
            // The registry (not the peer) closed this socket: the entry was
            // `map.remove`d, so `old.force_close` is an owned `Sender` (not
            // behind a shared reference) and can be moved out by `send()`.
            // The connection task's `post_auth_loop` wakes on the signal (it
            // races the read half against `force_close_rx`) and breaks, then
            // `ConnCtx::finish` resolves (it races `flushed` against
            // `force_close`). The 1008 close frame queued on `old.sink` above
            // is drained by the connection's *own* writer task, which signals
            // the connection's `flushed` oneshot. The entry is left removed
            // (not re-inserted), so the connection's deregister is a no-op
            // (the entry is already gone).
            let _ = old.force_close.send(());
            true
        } else {
            false
        }
    }

    /// Remove a token from the registry **without** closing its socket (used
    /// on a clean peer disconnect, where the socket is already gone).
    pub fn forget(&self, token_id: &str) {
        if token_id.is_empty() {
            return;
        }
        let mut map = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(token_id);
    }

    /// The number of live connections.
    pub fn client_count(&self) -> usize {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// The token ids that currently have a live connection (the registry map's
    /// keys). The revoke watcher iterates these to find live tokens that the
    /// store has since marked `revoked`.
    pub fn live_token_ids(&self) -> Vec<String> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).keys().cloned().collect()
    }

    /// A JSON array of `clients` objects for `hub status --json` (each with
    /// `token_id`, `hostname`, `peer`, `client_id`, `features`,
    /// `harnesses`, `connected_at`, `last_frame_at`). Built from each entry's
    /// public [`ClientInfo`] view (the map's per-connection channel/oneshot
    /// handles are not serialized).
    pub fn clients_json(&self) -> serde_json::Value {
        let map = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let items: Vec<serde_json::Value> =
            map.values().map(|e| e.info()).map(|i| {
                serde_json::json!({
                    "token_id": i.token_id,
                    "hostname": i.hostname,
                    "peer": i.peer,
                    "client_id": i.client_id,
                    "features": i.features,
                    "harnesses": i.harnesses,
                    "connected_at": i.connected_at,
                    "last_frame_at": i.last_frame_at,
                })
            }).collect();
        serde_json::json!(items)
    }

    /// Whether a token currently has a live connection.
    pub fn is_live(&self, token_id: &str) -> bool {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(token_id)
    }

    /// The public view of a token's live connection, if live (a copy of its
    /// cloneable [`ClientInfo`]; the per-connection channel/oneshot handles are
    /// not exposed).
    pub fn get(&self, token_id: &str) -> Option<ClientInfo> {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(token_id)
            .map(|e| e.info().clone())
    }
}

impl RegistryEntry {
    /// A copy of the public read-only view.
    pub fn info(&self) -> &ClientInfo {
        &self.info
    }
}

/// The information the accept loop knows about a new connection at
/// registration time (before or after auth).
pub struct ConnInfo {
    pub token_id: String,
    pub hostname: String,
    pub peer: String,
    pub client_id: String,
    pub features: Vec<String>,
    pub harnesses: Vec<String>,
    pub now_ms: u64,
}

/// A connection's registration handle. The connection task holds one for its
/// lifetime; on `Drop` (socket gone — peer closed, timeout, superseded,
/// revoked, or the task ends) it removes itself from the registry. `Drop`
/// never panics, and it removes *only if this connection is still the
/// incumbent* (by `seq`), so a superseded connection never removes the socket
/// that replaced it.
pub struct ConnectionHandle {
    registry: std::sync::Arc<Registry>,
    token_id: String,
    seq: u64,
    /// Signaled when this handle is dropped. `Option` so `Drop` can `take()`
    /// it out (a tokio oneshot `Sender` is not `Copy` and `send` takes it by
    /// value, so we can't call `send` through a `&mut` reference).
    dropped_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ConnectionHandle {
    /// Build a handle that deregisters `token_id` on drop, but only if this
    /// `seq` is still the incumbent. Returns the handle and the drop
    /// oneshot's *receiver* (the connection task holds it in `handle_ws_conn`
    /// and awaits it in a `tokio::select!` alongside the writer's
    /// `flushed_tx`: if the handle drops before the writer processes a
    /// queued close, the select still resolves so `ConnCtx::finish`
    /// doesn't stall).
    pub fn new(registry: std::sync::Arc<Registry>, token_id: String, seq: u64) -> (Self, tokio::sync::oneshot::Receiver<()>) {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let handle = Self {
            registry,
            token_id,
            seq,
            dropped_tx: Some(tx),
        };
        (handle, rx)
    }

    /// The token this connection is registered under.
    pub fn token_id(&self) -> &str {
        &self.token_id
    }
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        // Signal the connection task that this handle has been dropped.
        // `take()` moves the `Sender` out of the `Option`, so we can call
        // `send()` (which takes `self` by value) without a move-out-of-`&mut`.
        if let Some(tx) = self.dropped_tx.take() {
            let _ = tx.send(());
        }
        let reg = &self.registry;
        let mut map = reg.entries.lock().unwrap_or_else(|e| e.into_inner());
        // Remove only if we are still the incumbent for our token (the map's
        // entry points at our seq). If a newer connection superseded us, the
        // map holds that newer entry and we must not remove it. If no entry
        // remains (already revoked / forgotten), there is nothing to do.
        match map.get(&self.token_id) {
            Some(entry) if entry.info.seq == self.seq => {
                map.remove(&self.token_id);
            }
            Some(_) => {
                // A newer connection superseded us; the map holds that newer
                // entry and we must not remove it.
            }
            None => {
                // Already revoked / forgotten; nothing to do.
            }
        }
    }
}

/// The `circuit/superseded` notification sent to a superseded socket.
fn superseded_notification() -> Message {
    let env = holler_proto::Envelope::notification("circuit/superseded", None);
    Message::text(holler_proto::encode(&env).unwrap_or_default())
}
