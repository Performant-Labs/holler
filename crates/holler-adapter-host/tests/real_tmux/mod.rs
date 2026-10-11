//! The private tmux server of the real-tmux tests (#641, AC 9), moved out of
//! `real_tmux_test.rs` so that a second test crate includes it with `#[path]` instead of
//! copying it (#649's pane integration scenario). It holds no `#[test]`.
//!
//! [`private`] builds every host of `real_tmux_test.rs`: its own tmux server on a socket in a
//! fresh `/tmp/hlr-tmux-*` directory, with a config that forces `/bin/sh`. [`isolated`] is the
//! same server in a directory under a caller's scratch base, started by the test itself with
//! exactly the environment it is given (#649, A's W-1): tmux gives a new session the server's
//! global environment, so what the server starts with is what every process under it sees.
//! Dropping the [`Server`] kills that server (by its socket, never a signal) and removes its
//! directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use holler_adapter_host::{TmuxHost, TmuxSocket};
use tempfile::TempDir;

/// The `sun_path` limit the socket path must stay under.
pub const SUN_PATH: usize = 100;

/// The config every private server runs with: `/bin/sh` panes (AC 9).
const CONFIG: &str = "set -g default-shell /bin/sh\n";

/// A private tmux server's directory; dropping it kills the server, then removes the
/// directory.
pub struct Server {
    dir: TempDir,
}

impl Server {
    /// The server's own directory.
    pub fn dir(&self) -> &Path {
        self.dir.path()
    }

    pub fn sock(&self) -> PathBuf {
        self.dir.path().join("s")
    }

    pub fn config(&self) -> PathBuf {
        self.dir.path().join("tmux.conf")
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Run tmux on this server only, with no inherited `TMUX`.
    pub fn tmux(&self, args: &[&str]) -> Output {
        Command::new("tmux")
            .arg("-S")
            .arg(self.sock())
            .arg("-f")
            .arg(self.config())
            .args(args)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    /// The trimmed stdout lines of a tmux call that must succeed.
    pub fn lines(&self, args: &[&str]) -> Vec<String> {
        let out = self.tmux(args);
        assert!(out.status.success(), "tmux {args:?}: {out:?}");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    pub fn sessions(&self) -> Vec<String> {
        self.lines(&["list-sessions", "-F", "#{session_name}"])
    }

    pub fn session_path(&self, session: &str) -> String {
        let target = format!("={session}:");
        self.lines(&["list-panes", "-t", &target, "-F", "#{session_path}"])
            .into_iter()
            .next()
            .unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.tmux(&["kill-server"]);
    }
}

pub fn tmux_available() -> bool {
    let found = Command::new("tmux")
        .arg("-V")
        .output()
        .is_ok_and(|o| o.status.success());
    if !found {
        eprintln!("skipped: tmux not found");
    }
    found
}

/// A server directory `hlr-tmux-*` in `base`, its config written, its socket path checked.
fn server_in(base: &Path) -> Server {
    let dir = tempfile::Builder::new()
        .prefix("hlr-tmux-")
        .tempdir_in(base)
        .unwrap();
    let server = Server { dir };
    let sock = server.sock();
    assert!(
        sock.as_os_str().len() < SUN_PATH,
        "{} is too long",
        sock.display()
    );
    fs::write(server.config(), CONFIG).unwrap();
    server
}

/// The host on `server`.
fn host_on(server: &Server) -> TmuxHost {
    TmuxHost::new(TmuxSocket::Path(server.sock())).with_config(server.config())
}

/// The one helper every real-tmux test builds its host with (AC 9).
pub fn private() -> (TmuxHost, Server) {
    let server = server_in(Path::new("/tmp"));
    (host_on(&server), server)
}

/// `tmux` on this process's `PATH`, as an absolute path.
fn tmux_on_path() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|dir| dir.join("tmux"))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("no tmux on PATH"))
}

/// A private server in `base`, started now by the test with exactly `env` (nothing
/// inherited) and one session, `anchor`, that keeps it running. The anchor runs `/bin/sh`
/// and is never a pane's session. `tmux` is found on this process's `PATH`, not `env`'s, and
/// runs in `base`, so the `PWD` tmux adds to its global environment is `base` too.
pub fn isolated(base: &Path, env: &[(String, String)], anchor: &str) -> (TmuxHost, Server) {
    let server = server_in(base);
    let out = Command::new(tmux_on_path())
        .arg("-S")
        .arg(server.sock())
        .arg("-f")
        .arg(server.config())
        .args(["new-session", "-d", "-s", anchor])
        .env_clear()
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .current_dir(base)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "tmux new-session {anchor}: {out:?}");
    (host_on(&server), server)
}
