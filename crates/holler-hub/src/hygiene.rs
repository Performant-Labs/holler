//! Connection hygiene limits (issue #184): the accept path's own hardening,
//! independent of the [`crate::lockout`] failed-auth cooldown.
//!
//! - **Max frame/message size** — `HOLLER_MAX_FRAME_BYTES`, default 2 MiB —
//!   applied via [`tokio_tungstenite::tungstenite::protocol::WebSocketConfig`]
//!   on every accepted socket; an oversized frame closes the socket with
//!   **1009** (message too big).
//! - **Pre-auth timeout** — `HOLLER_PRE_AUTH_TIMEOUT_MS`, default 20 s — a
//!   socket that has sent nothing at all by this deadline is closed.
//! - **Unauthenticated connection cap** — `HOLLER_MAX_PREAUTH_CONNECTIONS`,
//!   default 64 — a semaphore around the pre-auth phase (accept through the
//!   end of the auth handshake); a socket over the cap is refused at once
//!   with **1013** (try again later), before reading a frame.
//!
//! All three are read fresh from the environment at `hub serve` startup
//! (not per-connection — the values are fixed for the process's life, the
//! same discipline [`crate::lockout::LockoutLimits`] uses).

use std::time::Duration;

/// Default max WebSocket frame/message size: 2 MiB.
pub const DEFAULT_MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;
/// Default pre-auth timeout: 20 s.
pub const DEFAULT_PRE_AUTH_TIMEOUT_MS: u64 = 20_000;
/// Default unauthenticated-connection cap: 64.
pub const DEFAULT_MAX_PREAUTH_CONNECTIONS: usize = 64;

/// The resolved hygiene tunables for one `hub serve` process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HygieneLimits {
    pub max_frame_bytes: usize,
    pub pre_auth_timeout_ms: u64,
    pub max_preauth_connections: usize,
}

impl Default for HygieneLimits {
    fn default() -> Self {
        Self::resolve()
    }
}

impl HygieneLimits {
    /// Resolve the tunables from the environment, falling back to defaults.
    pub fn resolve() -> Self {
        Self {
            max_frame_bytes: std::env::var("HOLLER_MAX_FRAME_BYTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_MAX_FRAME_BYTES),
            pre_auth_timeout_ms: std::env::var("HOLLER_PRE_AUTH_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_PRE_AUTH_TIMEOUT_MS),
            max_preauth_connections: std::env::var("HOLLER_MAX_PREAUTH_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_MAX_PREAUTH_CONNECTIONS),
        }
    }

    /// The pre-auth timeout as a [`Duration`].
    pub fn pre_auth_timeout(&self) -> Duration {
        Duration::from_millis(self.pre_auth_timeout_ms)
    }

    /// The [`tokio_tungstenite::tungstenite::protocol::WebSocketConfig`] this
    /// hub applies to every accepted socket (max frame *and* max message size
    /// both pinned to the same limit — a single-frame text message is the
    /// only shape the v2 wire ever sends, so the two limits coincide here).
    pub fn ws_config(&self) -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
        tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
            .max_frame_size(Some(self.max_frame_bytes))
            .max_message_size(Some(self.max_frame_bytes))
    }

    /// The `limits{}` document `control/status` reports (issue #184's
    /// acceptance: the hygiene values are documented in `hub status --json`).
    pub fn to_json(self, lockout: crate::lockout::LockoutLimits) -> serde_json::Value {
        serde_json::json!({
            "max_frame_bytes": self.max_frame_bytes,
            "pre_auth_timeout_ms": self.pre_auth_timeout_ms,
            "max_preauth_connections": self.max_preauth_connections,
            "lockout": {
                "max_failures": lockout.max_failures,
                "window_ms": lockout.window_ms,
                "duration_ms": lockout.duration_ms,
            },
        })
    }
}
