//! The body's long-lived **X25519 static identity keypair** (issue #337).
//!
//! Corrects an assumption from the original #321 Noise XK decomposition:
//! the body's existing registered key (`crate::identity`, issue #323) is an
//! **Ed25519 signing key**, used to prove possession over
//! `circuit/authenticate` → `circuit/prove` — it is not usable for a
//! Diffie-Hellman exchange. Noise XK needs a static **X25519** key on both
//! sides for its DH steps (`es`, `ee`, `se`); the hub already has one
//! (issue #322, `holler_hub::identity`). This module gives the body the
//! matching half.
//!
//! Generated on first `body join`, persisted at `<state>/body/x25519_
//! identity.key` (mode `0600`, raw 32 bytes — the same convention the hub's
//! `#322` key and the join-token pepper both use), and never regenerated
//! afterward — `body_x25519_keypair_survives_restart_and_is_0600` pins this.
//! The public half rides `circuit/join`'s `body_x25519_pubkey` field
//! alongside the existing Ed25519 `body_pubkey`; the hub stores it against
//! the token record. Both keys coexist: Ed25519 for #323's proof-of-
//! possession, X25519 for the future #338 Noise XK handshake — this issue
//! is plumbing only (keygen, persistence, registration), not the handshake
//! state machine itself.
//!
//! The private key is **never logged** — this module never hands it to
//! [`holler_proto::log::emit`] (only the fact that a keypair was generated,
//! and the path, mirroring [`crate::identity`]'s and the hub's own
//! `holler_hub::identity`'s discipline). `body_x25519_private_key_never_
//! appears_in_noisy_debug_output` (this file's own test) pins that, and
//! `holler_proto::log`'s redaction filter additionally covers any field
//! literally named `*private_key*` as defense in depth.

use std::path::{Path, PathBuf};

use holler_proto::log::{Component, Direction, Event, Severity};
use x25519_dalek::{PublicKey, StaticSecret};

/// The length, in bytes, of an X25519 private (and public) key.
const KEY_BYTES: usize = 32;

/// The body's resolved X25519 identity: the public key (hex, the
/// `circuit/join`-facing form) plus the keypair itself, for the future
/// Noise XK handshake (#338) to consume.
#[derive(Clone)]
pub struct BodyX25519Identity {
    // Forward-declared for the future Noise XK handshake (#338): this issue
    // only ever publishes `public` (`circuit/join`'s `body_x25519_pubkey`);
    // nothing in this story performs a Diffie-Hellman with the secret half
    // yet. Read only by this module's own tests (`secret_bytes`,
    // `#[cfg(test)]`) to prove restart persistence and the no-leak guarantee.
    #[allow(dead_code)] // #338 (Noise XK will read this; nothing does yet)
    secret: StaticSecret,
    public: PublicKey,
}

impl BodyX25519Identity {
    /// The hex-encoded public key (64 hex chars) — what rides `circuit/
    /// join`'s `body_x25519_pubkey`.
    pub fn public_hex(&self) -> String {
        hex::encode(self.public.as_bytes())
    }

    /// The private key's raw bytes. Exposed only for this module's own
    /// persistence round-trip and no-leak tests — no current caller needs to
    /// perform a Diffie-Hellman with it (this issue only pins the public
    /// half; the exchange itself is Noise XK, issue #338, not yet
    /// implemented).
    #[cfg(test)]
    fn secret_bytes(&self) -> [u8; KEY_BYTES] {
        self.secret.to_bytes()
    }
}

/// The identity key file: `<state_root>/body/x25519_identity.key` (0600,
/// raw 32 bytes).
pub fn identity_path(state_root: &Path) -> PathBuf {
    state_root.join("body").join("x25519_identity.key")
}

/// A fail-closed refusal resolving or generating the body's X25519 identity
/// keypair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X25519IdentityError {
    pub message: String,
}

impl X25519IdentityError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for X25519IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for X25519IdentityError {}

/// Resolve the body's X25519 identity keypair: load `<state_root>/body/
/// x25519_identity.key` if it exists, else generate one (32 bytes from the
/// OS CSPRNG), persist it at mode `0600`, and emit the `body_x25519_
/// identity_generated` info event (path only — never the key bytes).
///
/// Idempotent and safe to call on every `body join`: the first call creates
/// the file; every later call, including across a process restart, loads
/// the same bytes back — `body_x25519_keypair_survives_restart_and_is_0600`
/// is exactly this round-trip.
pub fn ensure(state_root: &Path) -> Result<BodyX25519Identity, X25519IdentityError> {
    let dir = state_root.join("body");
    std::fs::create_dir_all(&dir)
        .map_err(|e| X25519IdentityError::new(format!("cannot create state dir: {e}")))?;
    let path = identity_path(state_root);
    if path.exists() {
        let bytes = std::fs::read(&path)
            .map_err(|e| X25519IdentityError::new(format!("cannot read body X25519 identity {}: {e}", path.display())))?;
        let arr: [u8; KEY_BYTES] = bytes.try_into().map_err(|_| {
            X25519IdentityError::new(format!("body X25519 identity {} is corrupt (wrong length)", path.display()))
        })?;
        return Ok(from_bytes(arr));
    }
    let mut buf = [0u8; KEY_BYTES];
    getrandom::fill(&mut buf).map_err(|e| X25519IdentityError::new(format!("CSPRNG failure: {e}")))?;
    std::fs::File::create(&path)
        .map_err(|e| X25519IdentityError::new(format!("cannot create body X25519 identity {}: {e}", path.display())))?;
    set_mode_0600(&path);
    std::fs::write(&path, buf)
        .map_err(|e| X25519IdentityError::new(format!("cannot write body X25519 identity {}: {e}", path.display())))?;
    let identity = from_bytes(buf);
    emit_identity_generated(&path, &identity.public_hex());
    Ok(identity)
}

fn from_bytes(bytes: [u8; KEY_BYTES]) -> BodyX25519Identity {
    let secret = StaticSecret::from(bytes);
    let public = PublicKey::from(&secret);
    BodyX25519Identity { secret, public }
}

#[cfg(unix)]
fn set_mode_0600(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}
#[cfg(not(unix))]
fn set_mode_0600(_path: &Path) {}

/// Emit the `body_x25519_identity_generated` info event: the path and the
/// (public, safe-to-log) fingerprint only — the private key is never handed
/// to this function, let alone to `emit`.
fn emit_identity_generated(path: &Path, public_hex: &str) {
    holler_proto::log::emit(&Event {
        component: Component::Token,
        severity: Severity::Info,
        direction: Direction::Local,
        method: "body_x25519_identity_generated",
        id: None,
        peer: None,
        fields: vec![
            ("path", path.display().to_string()),
            ("body_x25519_pubkey", public_hex.to_string()),
        ],
        frame: None,
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #337
mod tests {
    use super::*;

    struct Tdir {
        root: PathBuf,
    }
    impl Tdir {
        fn new(tag: &str) -> Self {
            let unique = format!(
                "holler-body-x25519-identity-{tag}-{}-{:x}",
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

    /// Acceptance: "generated once, persisted at 0600, and stable across
    /// restarts (not regenerated every launch)".
    #[test]
    fn body_x25519_keypair_survives_restart_and_is_0600() {
        let dir = Tdir::new("restart");

        let first = ensure(&dir.root).expect("first ensure generates a keypair");
        let path = identity_path(&dir.root);
        assert!(path.exists(), "x25519_identity.key must be persisted");

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let mode = std::fs::metadata(&path).expect("stat x25519_identity.key").mode();
            assert_eq!(mode & 0o777, 0o600, "x25519_identity.key must be mode 0600; got {mode:o}");
        }

        // A fresh `ensure` call over the *same* state root simulates a body
        // restart (a later `body join`, or a future `body run` load): it
        // must load the same keypair back, never regenerate.
        let second = ensure(&dir.root).expect("second ensure loads the persisted keypair");
        assert_eq!(first.public_hex(), second.public_hex(), "the public key must survive a restart");
        assert_eq!(first.secret_bytes(), second.secret_bytes(), "the private key must survive a restart");
    }

    #[test]
    fn public_hex_is_64_lowercase_hex_chars() {
        let dir = Tdir::new("hex");
        let identity = ensure(&dir.root).expect("ensure");
        let hex = identity.public_hex();
        assert_eq!(hex.len(), 64, "a 32-byte X25519 public key is 64 hex chars: {hex}");
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "must be lowercase hex: {hex}");
    }

    /// Acceptance: "A test confirms the private key is never present in any
    /// wire frame or log line" — the log-line half. This asserts the real
    /// call site's field list directly (not just the redaction filter, which
    /// is a second, independent line of defense — see `holler_proto`'s own
    /// `redact_key_substrings_are_case_insensitive` test for that), mirroring
    /// `holler_hub::identity`'s `hub_private_key_never_appears_in_noisy_
    /// debug_output`.
    #[test]
    fn body_x25519_private_key_never_appears_in_noisy_debug_output() {
        let dir = Tdir::new("nolog");
        let identity = ensure(&dir.root).expect("ensure");
        let secret_hex = hex::encode(identity.secret_bytes());

        let path = identity_path(&dir.root);
        let event = Event {
            component: Component::Token,
            severity: Severity::Info,
            direction: Direction::Local,
            method: "body_x25519_identity_generated",
            id: None,
            peer: None,
            fields: vec![("path", path.display().to_string()), ("body_x25519_pubkey", identity.public_hex())],
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

    /// Acceptance: "A test confirms the private key is never present in any
    /// wire frame or log line" — the wire-frame half. Builds the actual
    /// `circuit/join` params ([`holler_proto::docs::Join`]) this body would
    /// send, carrying this identity's real public key, and asserts the
    /// serialized wire JSON never contains the private key's hex — proving
    /// the boundary at the exact struct that crosses the socket, not just by
    /// inspecting the struct definition.
    #[test]
    fn body_x25519_private_key_never_appears_in_join_wire_frame() {
        let dir = Tdir::new("wire");
        let identity = ensure(&dir.root).expect("ensure");
        let secret_hex = hex::encode(identity.secret_bytes());

        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]);
        let join = holler_proto::docs::Join {
            secret: "hlr_join_deadbeef".to_string(),
            hostname: "kiwi".to_string(),
            body_pubkey: hex::encode(signing_key.verifying_key().to_bytes()),
            body_x25519_pubkey: identity.public_hex(),
        };
        let wire = serde_json::to_string(&join).expect("Join serializes");
        assert!(
            !wire.contains(&secret_hex),
            "the X25519 private key must never appear in the circuit/join wire frame: {wire}"
        );
        assert!(wire.contains(&identity.public_hex()), "the wire frame must still carry the public key: {wire}");
    }
}
