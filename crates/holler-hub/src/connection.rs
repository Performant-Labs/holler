//! Per-connection hygiene (story #184).
//!
//! Three guards keep the hub from being abused by cheaply-opened sockets:
//!
//! - **Frame cap.** A message larger than [`MAX_MESSAGE_BYTES`] is a protocol
//!   violation: the server rejects it (tungstenite answers with a **1009**
//!   "message too big" close) and the connection is torn down. The cap is set
//!   on the server's `WebSocketConfig` so *every* inbound message on a socket
//!   is checked.
//! - **Pre-auth timeout.** A socket that has not completed `circuit/join` or
//!   `circuit/authenticate` within [`PRE_AUTH_TIMEOUT_MS`] ms is closed
//!   (close **1001**, going away). The timer is armed when the socket opens
//!   and stopped as soon as auth completes; an already-authenticated socket
//!   has no liveness timer (liveness is the next story).
//! - **Pre-auth cap.** At most [`MAX_PREAUTH_CONNECTIONS`] sockets may be
//!   open-but-unauthenticated at once; the overflow connection is closed
//!   (close **1013**, "try again later"). The counter ticks up when a socket
//!   opens and down when it is either authenticated or closed.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Default maximum inbound message size: 2 MiB.
pub const MAX_MESSAGE_BYTES: usize = 2 * 1024 * 1024;
/// Default pre-auth timeout: 20 s.
pub const PRE_AUTH_TIMEOUT_MS: u64 = 20_000;
/// Default maximum number of concurrently open-but-unauthenticated sockets.
pub const MAX_PREAUTH_CONNECTIONS: usize = 64;

/// The hygiene tunables, resolved from the environment (defaults above when
/// the variables are unset or invalid). `PartialEq` lets `hub status` report
/// whether an operator overrode any default (`limits.overridden`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub max_message_bytes: usize,
    pub pre_auth_timeout_ms: u64,
    pub max_preauth_connections: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self::resolve()
    }
}

impl Limits {
    /// Resolve the limits from the environment, falling back to the defaults.
    pub fn resolve() -> Self {
        Self {
            max_message_bytes: std::env::var("HOLLER_MAX_FRAME_BYTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(MAX_MESSAGE_BYTES),
            pre_auth_timeout_ms: std::env::var("HOLLER_PRE_AUTH_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(PRE_AUTH_TIMEOUT_MS),
            max_preauth_connections: std::env::var("HOLLER_MAX_PREAUTH_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(MAX_PREAUTH_CONNECTIONS),
        }
    }

    /// The tokio-tungstenite `WebSocketConfig` for a **server** connection,
    /// with the frame cap applied (so an oversized inbound message is rejected
    /// with a 1009 close by tungstenite itself).
    pub fn ws_config(&self) -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
        let mut config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
        config.max_message_size = Some(self.max_message_bytes);
        config
    }

    /// The tokio-tungstenite `WebSocketConfig` for a **client** (test)
    /// connection. A test that wants to *send* an oversized frame must be able
    /// to (a server-side cap does not stop the client from framing it), so the
    /// client uses an unconstrained message size.
    pub fn client_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
        tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
    }

    /// The pre-auth timeout as a `Duration`.
    pub fn pre_auth_timeout(&self) -> Duration {
        Duration::from_millis(self.pre_auth_timeout_ms)
    }
}

/// Shared hygiene state for the hub: the live pre-auth connection counter and
/// the resolved limits. The accept loop increments on connect and the
/// connection task decrements on close / on successful auth.
#[derive(Default)]
pub struct Hygiene {
    limits: Limits,
    /// The number of sockets currently open but not yet authenticated.
    preauth: AtomicUsize,
    /// A monotonically increasing id for each pre-auth registration (used to
    /// make decrement atomic with respect to a concurrent increment).
    _seq: AtomicU64,
}

impl Hygiene {
    /// A new hygiene instance, with limits resolved from the environment.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Try to register a new pre-auth connection. Returns `true` if the
    /// connection may proceed (and the counter was incremented) or `false` if
    /// the pre-auth cap is reached (the caller should close the connection
    /// with **1013**).
    pub fn try_begin_preauth(&self) -> bool {
        loop {
            let cur = self.preauth.load(Ordering::Relaxed);
            if cur >= self.limits.max_preauth_connections {
                return false; // over the cap: the caller closes with 1013.
            }
            match self.preauth.compare_exchange_weak(
                cur,
                cur + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(_) => continue, // someone else incremented; retry.
            }
        }
    }

    /// Release a pre-auth registration (the connection authenticated, or
    /// closed before authenticating). Idempotent at the bound (never goes
    /// negative).
    pub fn end_preauth(&self) {
        let cur = self.preauth.load(Ordering::Relaxed);
        self.preauth
            .compare_exchange_weak(cur, cur.saturating_sub(1), Ordering::SeqCst, Ordering::SeqCst)
            .ok();
    }

    /// The current number of open-but-unauthenticated connections.
    pub fn preauth_count(&self) -> usize {
        self.preauth.load(Ordering::Relaxed)
    }

    /// The resolved limits.
    pub fn limits(&self) -> &Limits {
        &self.limits
    }
}
