//! The body's persisted identity: `<state>/body/credential.json` (story #176).
//!
//! A `body join` redeems a one-time join secret over the wire for a `client_id` + a long-lived `credential`; the body then persists **the credential** (and the identity it names) in this file, mode 0600, so a later `body run` can re-authenticate with it.
//! The file holds the **credential, never the join secret** — the secret is one-time bootstrap and is never written to disk (the hub's token store keeps only its HMAC).
//!
//! Everything here is **synchronous** (plain `std::fs`): `body join` is a
//! short-lived CLI process with no tokio runtime in scope, so a blocking file
//! write is correct (the hub's `spawn_blocking` wrappers exist only because
//! *it* runs inside a runtime).

use std::path::Path;

use serde::{Deserialize, Serialize};

/// The one-time join secret (and therefore the join) is refused after this
/// long without the body being seen again. It doubles as [`BodyIdentity`]::connected's
/// staleness window for `body status` (story #176: "stale > 45s = false").
pub const STALE_AFTER_SECS: i64 = 45;

/// The body's identity, as persisted in `<state>/body/credential.json`.
///
/// This is the file the spec pins: after a successful join it exists, is mode
/// 0600, and names the token the body joined with. `client_id` + `credential`
/// are exactly what the hub returns from `circuit/join`; `token_id` is what
/// the CLI's `--token ID:SECRET` carried (the body needs it later to
/// re-authenticate); `hostname` is what the body claimed on the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyIdentity {
    pub client_id: String,
    /// The long-lived credential (never the one-time join secret).
    pub credential: String,
    /// The token id the body joined with.
    pub token_id: String,
    /// The hostname the body claimed.
    #[serde(default)]
    pub hostname: String,
    /// The hub address the body joined (`ws://…` / `wss://…`), for `status`.
    #[serde(default)]
    pub server_url: String,
    /// Unix seconds the join succeeded (the last time this body was seen).
    #[serde(default)]
    pub joined_at: i64,
}

impl BodyIdentity {
    /// The file this identity is persisted at under the state root.
    pub fn path(state_root: &Path) -> std::path::PathBuf {
        state_root.join("body").join("credential.json")
    }

    /// `connected`, per story #176: the body counts as connected while the
    /// join is recent (within [`STALE_AFTER_SECS`]); a missing/stale
    /// `joined_at` is not connected. (A `body run` that is live holds the
    /// connection; `status` reads the file, so recency is the observable proxy.)
    pub fn connected(&self, now: i64) -> bool {
        self.joined_at > 0 && now.saturating_sub(self.joined_at) <= STALE_AFTER_SECS
    }
}

/// Why a join could not complete (and the CLI must exit 1). The `message` is
/// the one line the CLI prints on stderr — on a hub refusal it is the hub's
/// own `join_failed` reason (e.g. "token already redeemed").
#[derive(Debug, Clone, PartialEq)]
pub enum JoinError {
    /// The `--server` address is not a usable `ws`/`wss` URL.
    BadAddress(String),
    /// The join secret could not be split as `ID:SECRET`.
    MalformedToken(String),
    /// The WebSocket connection failed (DNS, refused, TLS failure, …).
    Connect(String),
    /// The hub answered `circuit/join` with a `join_failed` (or other) error.
    HubRefused(String),
    /// The socket closed before a `circuit/join` answer arrived.
    Closed,
    /// A file write/read under the state dir failed.
    Io(String),
    /// A join frame the body sent or the hub sent was not valid JSON-RPC.
    Wire(String),
}

impl JoinError {
    /// The human-facing one-line reason (the CLI prints this on stderr).
    pub fn message(&self) -> String {
        match self {
            JoinError::BadAddress(m) => format!("bad --server address: {m}"),
            JoinError::MalformedToken(m) => format!("bad --token: {m}"),
            JoinError::Connect(m) => format!("could not reach the hub: {m}"),
            JoinError::HubRefused(m) => m.clone(),
            JoinError::Closed => "the hub closed the connection before completing the join".to_string(),
            JoinError::Io(m) => format!("state dir: {m}"),
            JoinError::Wire(m) => format!("wire: {m}"),
        }
    }
}

/// Persist `identity` to `<state>/body/credential.json`, creating the
/// `body/` directory as needed and setting the file mode to 0600.
pub fn save(identity: &BodyIdentity, state_root: &Path) -> std::io::Result<()> {
    let path = BodyIdentity::path(state_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(identity).map_err(io_err)?;
    std::fs::write(&path, json)?;
    set_mode_600(&path)?;
    Ok(())
}

/// Set `path` to mode 0600 (owner read/write only) — the credential is
/// secret material and must not be world/group-readable.
#[cfg(unix)]
fn set_mode_600(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // Set the mode to 0600 (owner read/write only) via the `mode`-based
    // constructor. (The test harness asserts `file.mode() & 0o777 == 0o600`,
    // which is exactly what a 0600 file reports on Unix.)
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}
#[cfg(not(unix))]
fn set_mode_600(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Read the persisted identity from `<state>/body/credential.json`; `None` if
/// it is absent (a body that has never joined). A present-but-unreadable or
/// corrupt file is an `Err` (the CLI surfaces it rather than pretending the
/// body never joined — a corrupt credential is a real, reportable fault).
pub fn load(state_root: &Path) -> Option<Result<BodyIdentity, std::io::Error>> {
    let path = BodyIdentity::path(state_root);
    if !path.exists() {
        return None;
    }
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => return Some(Err(io_err(e))),
    };
    Some(serde_json::from_slice::<BodyIdentity>(&bytes).map_err(io_err))
}

/// Delete the persisted identity (the no-run `body detach` path). A missing
/// file is a no-op success (detach is idempotent).
pub fn clear(state_root: &Path) -> std::io::Result<()> {
    let path = BodyIdentity::path(state_root);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn io_err<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
}
