//! The hub under test: starting a real `holler hub serve`, locating the
//! binaries beside this one, minting each client's join token, and reading the
//! hub's own `hub status --json`.
//!
//! Issue #369 harness bullet 1 ("starts a real `holler hub serve`, or points
//! at an already-running one") lives here. Nothing in this module is a mock:
//! the hub is the shipping binary, the token store is the shipping token
//! store, and `hub status` is read by running the shipping CLI.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::{kill_tree, Res};

/// A scratch `HOLLER_STATE_DIR`, removed when the harness exits.
pub struct StateDir {
    path: PathBuf,
    owned: bool,
}

impl StateDir {
    /// A fresh, uniquely-named state dir under the OS temp dir.
    pub fn fresh() -> Res<Self> {
        // Deliberately terse. The hub binds a Unix-domain control socket at
        // `<state>/hub/control.sock`, and a Unix socket path is capped at
        // `SUN_LEN` (104 bytes on macOS) — where `std::env::temp_dir()` is
        // already ~49 characters. A descriptive `holler-load-test-<pid>-<nanos>`
        // name overflows that cap and the hub fails closed at startup, which
        // is exactly what this shorter name avoids. Uniqueness is unchanged
        // (pid plus a per-call counter plus the low bits of the clock).
        static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let unique = format!(
            "hlr-lt-{}-{:x}-{:x}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0)
                & 0xFFFF_FFFF,
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path)?;
        Ok(Self { path, owned: true })
    }

    /// Adopt an existing state dir (`--hub-state`, for a hub the operator
    /// already has running). Never removed on drop — it is not ours.
    pub fn adopt(path: PathBuf) -> Self {
        Self { path, owned: false }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StateDir {
    fn drop(&mut self) {
        if self.owned {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// The directory this harness binary was built into — where cargo also puts
/// `holler` and `stub-acp`.
///
/// A `[[bin]]` target (unlike a test target) never gets `CARGO_BIN_EXE_*`, so
/// sibling discovery is the substitute: every binary in a cargo workspace
/// lands in the same profile directory, so "next to me" is exactly right for a
/// `cargo build --workspace` or `cargo run -p holler-load-test`. `--holler-bin`
/// / `--stub-acp-bin` override it for anything else (an installed binary, a
/// cross-built one).
pub fn sibling_bin(name: &str) -> Res<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or("this binary has no parent directory, so its siblings cannot be located")?;
    let candidate = dir.join(name);
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(format!(
        "could not find `{name}` next to this binary ({}) — build the workspace \
         (`cargo build --workspace`) or pass an explicit path",
        dir.display()
    )
    .into())
}

/// How the harness reaches its hub.
pub struct Hub {
    /// `ws://127.0.0.1:<port>` — what every client dials.
    pub ws_url: String,
    /// The hub's PID, for resource sampling. `None` when attached to a hub
    /// this harness did not start (we know its URL, not its process).
    pub pid: Option<u32>,
    child: Option<Child>,
    holler_bin: PathBuf,
    state: StateDir,
}

impl Hub {
    /// Start a real `holler hub serve` on a free loopback port and wait for it
    /// to report the port it bound.
    ///
    /// Readiness is *observed* (the hub's own `listening` event on stderr),
    /// never slept on — ADR 0002's rule, and the same mechanism
    /// `tests/support/mod.rs` uses. The stderr pipe keeps being drained on a
    /// background thread for the hub's whole life: stopping at the `listening`
    /// line would let the pipe fill and block the hub's runtime.
    pub fn start(holler_bin: PathBuf, state: StateDir, env: &[(String, String)]) -> Res<Self> {
        let mut cmd = Command::new(&holler_bin);
        cmd.env("HOLLER_STATE_DIR", state.path())
            .env("HOLLER_DEBUG", "quiet")
            .env("HOLLER_LOG_FORMAT", "json");
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.args(["hub", "serve", "--listen", "127.0.0.1:0"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        own_process_group(&mut cmd);
        let mut child = cmd.spawn()?;

        let stderr = child.stderr.take().ok_or("hub stderr was not piped")?;
        let (tx, rx) = std::sync::mpsc::channel::<u16>();
        // The hub's own log stream is invisible by default (it would drown the
        // report), but a hub that fails to come up leaves nothing to diagnose
        // from — `HOLLER_LOAD_TEST_ECHO_HUB=1` relays it to this process's
        // stderr.
        let echo = std::env::var("HOLLER_LOAD_TEST_ECHO_HUB").is_ok();
        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stderr);
            let mut line = String::new();
            let mut sent = false;
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                if echo {
                    eprint!("[hub] {line}");
                }
                if sent {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
                if v.get("event").and_then(serde_json::Value::as_str) != Some("listening") {
                    continue;
                }
                if let Some(port) = v
                    .get("addr")
                    .or_else(|| v.get("address"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(|a| a.rsplit(':').next())
                    .and_then(|p| p.parse::<u16>().ok())
                {
                    sent = true;
                    let _ = tx.send(port);
                }
            }
        });

        let port = match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(p) => p,
            Err(e) => {
                kill_tree(&mut child);
                return Err(format!("hub never reported a listening port: {e}").into());
            }
        };

        Ok(Self {
            ws_url: format!("ws://127.0.0.1:{port}"),
            pid: Some(child.id()),
            child: Some(child),
            holler_bin,
            state,
        })
    }

    /// Point at a hub someone else is running. Its state dir is still needed:
    /// minting a join token means writing that hub's own token store.
    pub fn attach(holler_bin: PathBuf, state: StateDir, ws_url: String) -> Self {
        Self {
            ws_url,
            pid: None,
            child: None,
            holler_bin,
            state,
        }
    }

    pub fn state(&self) -> &StateDir {
        &self.state
    }

    /// The hub's X25519 static public key — what a Noise XK initiator needs in
    /// advance, exactly as a real body gets it from `body join --hub-key`.
    pub fn x25519_pubkey(&self) -> Res<[u8; 32]> {
        let hub_state = holler_hub::state::HubState::from_root(self.state.path().to_path_buf());
        let identity = holler_hub::identity::ensure(&hub_state)?;
        let bytes = hex::decode(identity.public_hex())?;
        <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| "hub pubkey is not 32 bytes".into())
    }

    /// Mint a join token and redeem it for `label`, in-process against the
    /// hub's real token store.
    ///
    /// In-process rather than one `hub token mint` + `body join` subprocess
    /// pair per client, for one measurable reason: at N=200 that is 400 extra
    /// process spawns whose cost would land inside the very connect window
    /// this harness is trying to measure. The code path is the shipping one
    /// either way — `token::mint`/`token::redeem` are what the hub's own
    /// `circuit/join` handler calls.
    pub fn provision(&self, label: &str, identity: &crate::wire::ClientIdentity) -> Res<String> {
        let hub_state = holler_hub::state::HubState::from_root(self.state.path().to_path_buf());
        let minted = holler_hub::token::mint(label, 24 * 3600, &hub_state)?;
        holler_hub::token::redeem(
            &minted.secret,
            label,
            &identity.ed25519_pubkey_hex(),
            &identity.x25519_pubkey_hex(),
            &hub_state,
        )
        // `RedeemError` is not a `std::error::Error`, so `?` cannot widen it
        // into this harness's boxed error type on its own.
        .map_err(|e| format!("redeeming {label}'s join token failed: {e:?}"))?;
        Ok(minted.record.token_id)
    }

    /// `holler --json hub status`, parsed. Runs the shipping CLI over the
    /// hub's control socket — the same thing an operator would type.
    pub fn status_json(&self) -> Res<serde_json::Value> {
        let out = Command::new(&self.holler_bin)
            .env("HOLLER_STATE_DIR", self.state.path())
            .env("HOLLER_DEBUG", "quiet")
            .args(["--json", "hub", "status"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        if !out.status.success() {
            return Err(format!("`hub status --json` failed: {}", String::from_utf8_lossy(&out.stderr)).into());
        }
        Ok(serde_json::from_slice(&out.stdout)?)
    }

    /// The live client count the hub itself reports — #370's "no silent
    /// drops" check reads this.
    pub fn client_count(&self) -> Res<u64> {
        self.status_json()?
            .get("clients")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "hub status --json carries no numeric `clients`".into())
    }

    /// Stop the hub (only if this harness started it).
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            kill_tree(&mut child);
        }
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Put a spawned child in its own process group so [`kill_tree`] can reap the
/// whole tree. Unix-only; a no-op elsewhere.
pub fn own_process_group(cmd: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    {
        let _ = cmd;
    }
}
