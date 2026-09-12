//! The hub's long-lived **X25519 static identity keypair** (issue #322).
//!
//! Generated on first use (`hub serve` or `hub token mint`, whichever runs
//! first), persisted at `<state>/hub/identity.key` (mode `0600`), and never
//! regenerated afterward — `hub_keypair_survives_restart_and_is_0600` pins
//! this. The public half rides `hub token mint`'s join line (and its
//! `--json` output) and every `circuit/hello` a hub sends; a body pins it at
//! `body join` and hard-fails any later connection whose hub hello carries a
//! different key (no prompt, no trust-on-first-use — see `holler_body::
//! connection`).
//!
//! This is **X25519**, not Ed25519: the hub never signs anything in this
//! design (it issues `circuit/authenticate` nonces and verifies a body's
//! `circuit/prove` signature — see [`crate::circuit`] and issue #323's own
//! ADR amendment). X25519 is deliberately chosen as the pre-message key for
//! a future Noise XK handshake ([#321](https://github.com/Performant-Labs/holler/issues/321)) —
//! landing the identity now, ahead of Noise itself, is what makes that later
//! handshake an incremental addition rather than a redesign.
//!
//! The private key is **never logged** — this module never hands it to
//! [`holler_proto::log::emit`] (only the fact that a keypair was generated,
//! and the path, mirroring [`crate::token::emit_pepper_generated`]'s own
//! discipline for the HMAC pepper). `hub_private_key_never_appears_in_noisy_
//! debug_output` (this file's own test) pins that, and `holler_proto::log`'s
//! redaction filter additionally covers any field literally named
//! `*private_key*` as defense in depth.

use std::path::{Path, PathBuf};

use holler_proto::log::{Component, Direction, Event, Severity};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::state::{ensure_dirs, HubState};

/// The length, in bytes, of an X25519 private (and public) key.
const KEY_BYTES: usize = 32;

/// The hub's resolved identity: the public key (hex, the wire/CLI-facing
/// form) plus the keypair itself if a future caller ever needs the private
/// half (no current caller does — the hub only ever publishes the public
/// half; it never performs a Diffie-Hellman in v2).
#[derive(Clone)]
pub struct HubIdentity {
    // Forward-declared for the future Noise XK handshake (#321): v2 only
    // ever publishes `public` (the join line, `circuit/hello`, `hub
    // status`); nothing in this story performs a Diffie-Hellman with the
    // secret half yet. Read only by this module's own tests
    // (`secret_bytes`, `#[cfg(test)]`) to prove restart persistence.
    #[allow(dead_code)] // #321 (Noise XK will read this; nothing does yet)
    secret: StaticSecret,
    public: PublicKey,
}

impl HubIdentity {
    /// The hex-encoded public key (64 hex chars) — what rides the join line,
    /// `circuit/hello`, and `hub status`'s fingerprint.
    pub fn public_hex(&self) -> String {
        hex::encode(self.public.as_bytes())
    }

    /// The private key's raw bytes. Exposed only for this module's own
    /// persistence round-trip and tests — no current caller needs to perform
    /// a Diffie-Hellman with it (v2 only pins the public half; the exchange
    /// itself is Noise XK, issue #321, not yet implemented).
    #[cfg(test)]
    fn secret_bytes(&self) -> [u8; KEY_BYTES] {
        self.secret.to_bytes()
    }
}

/// The identity key file: `<root>/hub/identity.key` (0600, raw 32 bytes).
pub fn identity_path(state: &HubState) -> PathBuf {
    state.hub_dir.join("identity.key")
}

/// A fail-closed refusal resolving or generating the hub's identity keypair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityError {
    pub message: String,
}

impl IdentityError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for IdentityError {}

/// Resolve the hub's identity keypair: load `<state>/hub/identity.key` if it
/// exists, else generate one (32 bytes from the OS CSPRNG), persist it at
/// mode `0600`, and emit the `hub_identity_generated` info event (path only —
/// never the key bytes).
///
/// Idempotent and safe to call from both `hub serve` and `hub token mint`
/// (whichever runs first creates the file; every later caller, including a
/// restart, loads the same bytes back — `hub_keypair_survives_restart_and_
/// is_0600` is exactly this round-trip).
pub fn ensure(state: &HubState) -> Result<HubIdentity, IdentityError> {
    ensure_dirs(state).map_err(|e| IdentityError::new(format!("cannot create state dir: {e}")))?;
    let path = identity_path(state);
    if path.exists() {
        let bytes = std::fs::read(&path)
            .map_err(|e| IdentityError::new(format!("cannot read hub identity {}: {e}", path.display())))?;
        let arr: [u8; KEY_BYTES] = bytes
            .try_into()
            .map_err(|_| IdentityError::new(format!("hub identity {} is corrupt (wrong length)", path.display())))?;
        return Ok(from_bytes(arr));
    }
    let mut buf = [0u8; KEY_BYTES];
    getrandom::fill(&mut buf).map_err(|e| IdentityError::new(format!("CSPRNG failure: {e}")))?;
    std::fs::File::create(&path)
        .map_err(|e| IdentityError::new(format!("cannot create hub identity {}: {e}", path.display())))?;
    set_mode_0600(&path);
    std::fs::write(&path, buf)
        .map_err(|e| IdentityError::new(format!("cannot write hub identity {}: {e}", path.display())))?;
    let identity = from_bytes(buf);
    emit_identity_generated(&path, &identity.public_hex());
    Ok(identity)
}

fn from_bytes(bytes: [u8; KEY_BYTES]) -> HubIdentity {
    let secret = StaticSecret::from(bytes);
    let public = PublicKey::from(&secret);
    HubIdentity { secret, public }
}

#[cfg(unix)]
fn set_mode_0600(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}
#[cfg(not(unix))]
fn set_mode_0600(_path: &Path) {}

/// Emit the `hub_identity_generated` info event: the path and the (public,
/// safe-to-log) fingerprint only — the private key is never handed to this
/// function, let alone to `emit`.
fn emit_identity_generated(path: &Path, public_hex: &str) {
    holler_proto::log::emit(&Event {
        component: Component::Token,
        severity: Severity::Info,
        direction: Direction::Local,
        method: "hub_identity_generated",
        id: None,
        peer: None,
        fields: vec![
            ("path", path.display().to_string()),
            ("hub_pubkey", public_hex.to_string()),
        ],
        frame: None,
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #322
mod tests {
    use super::*;

    struct Tdir {
        root: PathBuf,
    }
    impl Tdir {
        fn new(tag: &str) -> Self {
            let unique = format!(
                "holler-hub-identity-{tag}-{}-{:x}",
                std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0),
            );
            let root = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&root).expect("create scratch state dir");
            Self { root }
        }
    }
    impl Drop for Tdir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn hub_keypair_survives_restart_and_is_0600() {
        let dir = Tdir::new("restart");
        let state = HubState::from_root(dir.root.clone());

        let first = ensure(&state).expect("first ensure generates a keypair");
        let path = identity_path(&state);
        assert!(path.exists(), "identity.key must be persisted");

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let mode = std::fs::metadata(&path).expect("stat identity.key").mode();
            assert_eq!(mode & 0o777, 0o600, "identity.key must be mode 0600; got {mode:o}");
        }

        // A fresh `HubState` over the *same* root simulates a hub restart:
        // `ensure` must load the same keypair back, never regenerate.
        let restarted_state = HubState::from_root(dir.root.clone());
        let second = ensure(&restarted_state).expect("second ensure loads the persisted keypair");
        assert_eq!(first.public_hex(), second.public_hex(), "the public key must survive a restart");
        assert_eq!(first.secret_bytes(), second.secret_bytes(), "the private key must survive a restart");
    }

    #[test]
    fn public_hex_is_64_lowercase_hex_chars() {
        let dir = Tdir::new("hex");
        let state = HubState::from_root(dir.root.clone());
        let identity = ensure(&state).expect("ensure");
        let hex = identity.public_hex();
        assert_eq!(hex.len(), 64, "a 32-byte X25519 public key is 64 hex chars: {hex}");
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "must be lowercase hex: {hex}");
    }

    /// #322's `hub_private_key_never_appears_in_noisy_debug_output`: the one
    /// event this module ever emits about the keypair (`hub_identity_
    /// generated`) carries the path and the *public* fingerprint only — the
    /// private key bytes are never passed to `emit_identity_generated`, so
    /// they can never appear in a rendered log line at any debug level. This
    /// asserts the real call site's field list directly (not just the
    /// redaction filter, which is a second, independent line of defense —
    /// see `holler_proto`'s own `redact_key_substrings_are_case_insensitive`
    /// test for that).
    #[test]
    fn hub_private_key_never_appears_in_noisy_debug_output() {
        let dir = Tdir::new("nolog");
        let state = HubState::from_root(dir.root.clone());
        let identity = ensure(&state).expect("ensure");
        let secret_hex = hex::encode(identity.secret_bytes());

        // Render the exact event this module emits on generation, at every
        // level, and confirm the private key's hex never appears in it.
        let path = identity_path(&state);
        let event = Event {
            component: Component::Token,
            severity: Severity::Info,
            direction: Direction::Local,
            method: "hub_identity_generated",
            id: None,
            peer: None,
            fields: vec![("path", path.display().to_string()), ("hub_pubkey", identity.public_hex())],
            frame: None,
        };
        for debug in [holler_proto::log::DebugLevel::None, holler_proto::log::DebugLevel::Quiet, holler_proto::log::DebugLevel::Noisy] {
            for format in [holler_proto::log::LogFormat::Text, holler_proto::log::LogFormat::Json] {
                let config = holler_proto::log::Config { debug, format };
                let rendered = event.render(&config);
                assert!(
                    !rendered.contains(&secret_hex),
                    "the private key must never appear in a rendered log line (debug={debug:?}, format={format:?}): {rendered}"
                );
            }
        }
    }
}
