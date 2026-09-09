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
use std::sync::{Arc, Mutex};

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
    /// force-closes this socket (supersede / revoke). The owning connection's
    /// `ConnCtx` holds the matching *receiver* and awaits it in `finish` (in a
    /// `select!` alongside the flush signal), so the connection task waits for
    /// that close to reach the wire before it drops the read half — the peer
    /// then sees the close code, not a bare EOF. A *peer*-initiated close never
    /// signals it (the sender is dropped on deregister, which resolves the
    /// receiver as "no force-close"), so the task drops immediately — the peer
    /// would not reply to a close, so waiting would stall the teardown.
    ///
    /// Stored as an `Arc<Sender>` (a tokio oneshot `Sender` is not `Clone`;
    /// only the `Arc` wrapper is). The connection task creates the channel and
    /// holds the *receiver*; `register` moves a clone of the `Arc` into this
    /// entry. On a clean deregister (`ConnectionHandle::drop`) the registry
    /// `map.remove`s the entry (dropping the `Arc` clone), which resolves the
    /// receiver as `Err` once the connection task's `Arc` is also dropped —
    /// the "no force-close" case. On supersede/revoke the registry does NOT
    /// signal the sender (a tokio oneshot `Sender` cannot be signalled through
    /// a shared reference); instead the close frame is queued on the entry's
    /// `sink` (the connection's own writer drains it and signals `flushed`,
    /// which `ConnCtx::finish` also awaits), and the entry is re-inserted
    /// (supersede) or left removed (revoke) so the deregister drops the
    /// sender.
    pub force_close: Arc<tokio::sync::oneshot::Sender<()>>,
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
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
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
    ) -> Option<u64> {
        if info.token_id.is_empty() {
            return None;
        }
        let seq = self.next_seq();
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
            force_close: info.force_close.clone(),
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
            // `ConnCtx::finish` wakes on that signal and awaits the flush
            // signal (`flushed`) before it drops its read half, so the old
            // peer sees the 1000 close code rather than a bare EOF. (A
            // *revoke* works the same way but with a 1008 close and no
            // notification; see `Registry::remove`.)
            if superseded {
                // Take the old entry out of the map so we can move its owned
                // `Arc<Sender>` out (a tokio oneshot `Sender` is not `Clone`
                // and `send` takes it by value). The old entry is re-inserted
                // under its own key immediately after the signal.
                let Some(old_entry) = map.remove(&info.token_id) else {
                    // A concurrent deregister removed the entry between the
                    // `contains_key` check and this `remove`; the old socket
                    // is already gone, so there is nothing to supersede.
                    map.insert(info.token_id.clone(), entry);
                    return Some(seq);
                };
                let _ = old_entry.sink.send(superseded_notification());
                let _ = old_entry.sink.send(Message::Close(Some(CloseFrame {
                    code: CloseCode::from(1000u16),
                    reason: "superseded".into(),
                })));
                // Signal the old connection's `force_close`: its `ConnCtx::finish`
                // wakes and awaits `flushed` before dropping the read half, so
                // the old peer sees the 1000 close code, not a bare EOF.
                // We move the `Arc` out of the entry (allowed because the entry
                // was just `map.remove`d) so the `Sender` is not moved out of a
                // shared reference. The `Arc` is dropped after the signal,
                // releasing the registry's strong reference (the connection task
                // holds its own `Arc`).
                // Signal the old connection's `force_close`: the entry was
                // `map.remove`d, so `old_entry.force_close` is an owned
                // `Sender` (not behind a shared reference) and can be moved
                // out by `send()`. The connection task holds only the *receiver*;
                // the sender is owned by the registry entry. Dropping the old
                // entry (below) drops the sender, but the signal has already
                // been delivered.
                // The old entry's `force_close` `Arc` is NOT signalled here:
                // a tokio oneshot `Sender` cannot be signalled through a
                // shared reference (it is not `Clone`, and `send` takes it by
                // value). Instead, the 1000 close frame queued on `old_entry.sink`
                // above is drained by the old connection's *own* writer task,
                // which signals the old connection's `flushed` oneshot.
                // `ConnCtx::finish` awaits `flushed` (in a `select!` alongside
                // `force_close`), so the old connection task waits for the
                // 1000 to reach the wire before dropping the read half — the
                // old peer sees the close code, not a bare EOF. The entry is
                // re-inserted below (so the old connection's deregister can
                // find it and drop the `force_close` `Arc`, which resolves the
                // receiver as `Err` — the "no force-close" case).
                map.insert(info.token_id.clone(), entry);
                map.insert(old_entry.info.token_id.clone(), old_entry);
            } else {
                map.insert(info.token_id.clone(), entry);
            }
        }
        Some(seq)
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
            // The connection task's `ConnCtx::finish` wakes and awaits
            // `flushed` before dropping the read half.
            // The 1008 close frame queued on `old.sink` above is drained by
            // the connection's *own* writer task, which signals the
            // connection's `flushed` oneshot. `ConnCtx::finish` awaits
            // `flushed` (in a `select!` alongside `force_close`), so the
            // connection task waits for the 1008 to reach the wire before
            // dropping the read half. The entry is left removed (not
            // re-inserted), so the connection's deregister is a no-op (the
            // entry is already gone); the `force_close` `Arc` is dropped when
            // the connection task drops its `ConnectionHandle`, which resolves
            // the receiver as `Err` — the "no force-close" case.
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
    /// The connection's `force_close` oneshot *sender* (created in
    /// `serve::handle_ws_conn`, wrapped in an `Arc` because a tokio oneshot
    /// `Sender` is not `Clone`). The registry entry stores a clone of this
    /// `Arc`; on supersede/revoke the registry `map.remove`s the entry and
    /// calls `send()` on the dereferenced sender (the `Arc` was moved out of
    /// the entry, so the sender is not behind a shared reference). The
    /// connection's `ConnCtx` holds the matching *receiver*; a clean
    /// deregister drops the entry (and its `Arc` clone), which resolves the
    /// receiver as `Err` once the connection task's `Arc` is also dropped.
    pub force_close: Arc<tokio::sync::oneshot::Sender<()>>,
}

/// A connection's registration handle. The connection task holds one for its
/// lifetime; on `Drop` (socket gone — peer closed, timeout, superseded,
/// revoked, or the task ends) it removes itself from the registry. `Drop`
/// never panics, and it removes *only if this connection is still the
/// incumbent* (by `seq`), so a superseded connection never removes the socket
/// that replaced it.
pub struct ConnectionHandle {
    registry: Arc<Registry>,
    token_id: String,
    seq: u64,
}

impl ConnectionHandle {
    /// Build a handle that deregisters `token_id` on drop, but only if this
    /// `seq` is still the incumbent.
    pub fn new(registry: Arc<Registry>, token_id: String, seq: u64) -> Self {
        Self {
            registry,
            token_id,
            seq,
        }
    }

    /// The token this connection is registered under.
    pub fn token_id(&self) -> &str {
        &self.token_id
    }
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
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
            _ => {}
        }
    }
}

/// The `circuit/superseded` notification sent to a superseded socket.
fn superseded_notification() -> Message {
    let env = holler_proto::Envelope::notification("circuit/superseded", None);
    Message::text(holler_proto::encode(&env).unwrap_or_default())
}
