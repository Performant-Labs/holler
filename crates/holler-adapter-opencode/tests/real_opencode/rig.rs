//! The rig of `real_opencode_test.rs` (#642b, the brief's "Real OpenCode" rules): one per
//! test, and one per conformance case. It is the spike's shell rig
//! (`scripts/spikes/opencode-lib.sh`) in Rust:
//!
//! - a fresh scratch dir in `/tmp`, short enough for a tmux socket path; `HOME` and the
//!   `XDG_*` dirs inside it, so no real OpenCode config, auth or session store is read;
//! - the dead-end provider config, verbatim from the spike, so no model can be reached:
//!   127.0.0.1:9 must refuse, and after every `serve` the server must report exactly that
//!   provider, or the test fails without going on. No prompt is ever sent;
//! - two ports from 48100-48199 only, each refused and bindable when picked and not yet
//!   handed out in this process;
//! - a private tmux server (`-S <scratch>/tmux.sock -f /dev/null`) with sessions
//!   `demo-c1r1` and `demo-c2r1` running a placeholder (`sleep 3600`), for the panes
//!   `w9:p1` and `w9:p2`; never the default tmux server.
//!
//! The [`Guard`] is built before the tmux server starts. On drop it SIGKILLs the process
//! group of every pid `serve` returned through [`Recording`], kills the private tmux server
//! by its socket (`kill-server`, never a signal), and removes the scratch dir. It never
//! signals a process it did not start and never looks processes up.

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use holler_adapter_opencode::http::{self, Reply};
use holler_adapter_opencode::{
    OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig, TmuxSocket,
};
use holler_pane::{HarnessPort, PaneError, PaneId, PaneName};
use holler_pane_testkit::conformance::harness::HarnessRig;
use serde_json::{json, Value};
use tempfile::TempDir;

/// The variable that opts in.
pub const OPT_IN: &str = "HOLLER_TEST_OPENCODE";
/// The rig's panes and the tmux sessions their TUIs run in.
pub const PANES: [&str; 2] = ["w9:p1", "w9:p2"];
pub const SESSIONS: [&str; 2] = ["demo-c1r1", "demo-c2r1"];
/// A unix socket path must stay under this.
const SUN_PATH: usize = 100;
/// The only ports the rig ever uses.
const FIRST_PORT: u16 = 48100;
const PORT_COUNT: u16 = 100;
/// The dead-end provider's address: a closed local port.
const DEAD_END: u16 = 9;

/// `scripts/spikes/opencode-lib.sh:68-85`, verbatim.
const CONFIG: &str = r#"{
  "$schema": "https://opencode.ai/config.json",
  "enabled_providers": ["deadend"],
  "disabled_providers": ["opencode"],
  "model": "deadend/none",
  "small_model": "deadend/none",
  "autoupdate": false,
  "share": "disabled",
  "provider": {
    "deadend": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "dead end (closed local port)",
      "options": { "baseURL": "http://127.0.0.1:9/v1" },
      "models": { "none": { "name": "none" } }
    }
  }
}
"#;

/// Whether this run opted in. When it did not, the test returns at once, saying why.
pub fn opted_in(test: &str) -> bool {
    let on = std::env::var(OPT_IN).is_ok_and(|v| v == "1");
    if !on {
        eprintln!("{test}: skipped: set {OPT_IN}=1 to run it against a real OpenCode and tmux");
    }
    on
}

/// `name` on `PATH`, as an absolute path.
fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// The binaries: `OPENCODE_BIN` or `opencode` on `PATH`, and `tmux` on `PATH`. Opted in, a
/// missing one is a failure, not a skip.
fn binaries() -> (PathBuf, PathBuf) {
    let opencode = std::env::var_os("OPENCODE_BIN")
        .map(PathBuf::from)
        .or_else(|| on_path("opencode"))
        .unwrap_or_else(|| panic!("{OPT_IN}=1, but no opencode: set OPENCODE_BIN or PATH"));
    let tmux = on_path("tmux").unwrap_or_else(|| panic!("{OPT_IN}=1, but no tmux on PATH"));
    (opencode, tmux)
}

/// Whether a connect to `127.0.0.1:port` is refused (nothing listens).
pub fn refused(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    matches!(
        TcpStream::connect_timeout(&addr, Duration::from_millis(500)),
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused
    )
}

/// Whether `127.0.0.1:port` can be bound now. The range lies inside Linux's ephemeral
/// range, so a socket that does not listen (a client end) can hold a port that
/// [`refused`] reports free, and `opencode serve` then cannot bind it. The listener is
/// dropped at once; one that never accepted leaves no `TIME_WAIT`.
fn bindable(port: u16) -> bool {
    std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok()
}

/// A port in 48100-48199 that refuses now, can be bound now, and that this process has
/// not handed out.
pub fn free_port() -> u16 {
    static TAKEN: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
    let mut taken = TAKEN
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let start = (nanos ^ std::process::id()) % u32::from(PORT_COUNT);
    for i in 0..u32::from(PORT_COUNT) {
        let port = FIRST_PORT + u16::try_from((start + i) % u32::from(PORT_COUNT)).unwrap();
        if !taken.contains(&port) && refused(port) && bindable(port) {
            taken.insert(port);
            return port;
        }
    }
    panic!("no free port in 48100-48199");
}

/// Run tmux on the private server only: `TMUX` and `TMUX_PANE` removed, stdin null.
pub fn tmux(bin: &Path, sock: &Path, args: &[&str]) -> Output {
    Command::new(bin)
        .arg("-S")
        .arg(sock)
        .args(args)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", bin.display()))
}

/// SIGKILLs what the rig started, kills its tmux server, then removes its scratch dir.
pub struct Guard {
    pids: Arc<Mutex<Vec<u32>>>,
    tmux: PathBuf,
    sock: PathBuf,
    /// Dropped after `drop` has run: the scratch dir goes last.
    _dir: TempDir,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let pids = self
            .pids
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        for pid in pids.into_iter().filter(|&pid| pid > 1) {
            let _ = signal_group(pid, "KILL");
        }
        // `no server running` or a missing socket: already done.
        let _ = tmux(&self.tmux, &self.sock, &["kill-server"]);
    }
}

/// `kill -s <signal> -- -<pgid>`: only ever a group `serve` returned.
pub fn signal_group(pgid: u32, signal: &str) -> bool {
    assert!(pgid > 1, "refusing to signal the process group {pgid}");
    Command::new("kill")
        .args(["-s", signal, "--", &format!("-{pgid}")])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// The adapter, with every pid `serve` returns recorded for the guard and the model guard
/// run after every `serve`.
pub struct Recording {
    inner: OpenCodeHarness,
    pids: Arc<Mutex<Vec<u32>>>,
}

impl HarnessPort for Recording {
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError> {
        let pid = self.inner.serve(name, port)?;
        self.pids
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(pid);
        no_model_can_be_reached(port);
        Ok(pid)
    }
    fn health(&self, port: u16) -> Result<bool, PaneError> {
        self.inner.health(port)
    }
    fn create_session(&self, port: u16) -> Result<String, PaneError> {
        self.inner.create_session(port)
    }
    fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
        self.inner.list_sessions(port)
    }
    fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
        self.inner.abort(port, session)
    }
    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
        self.inner.attach_tui(pane, port, session)
    }
    fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
        self.inner.select_session(pane, session)
    }
    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
        self.inner.shown_session(pane)
    }
}

/// The model guard: the server on `port` lists exactly the dead-end provider, and
/// 127.0.0.1:9 still refuses. Otherwise the test stops here.
fn no_model_can_be_reached(port: u16) {
    let reply = raw(port, "GET", "/config/providers", None);
    let providers: Option<Vec<Value>> = serde_json::from_slice::<Value>(&reply.body)
        .ok()
        .and_then(|v| v.get("providers")?.as_array().cloned())
        .map(|list| {
            list.iter()
                .map(|p| json!({ "id": p["id"], "url": p["options"]["baseURL"] }))
                .collect()
        });
    let expected = vec![json!({ "id": "deadend", "url": "http://127.0.0.1:9/v1" })];
    assert!(
        providers.as_ref() == Some(&expected) && refused(DEAD_END),
        "model guard: the server on {port} must list only the dead end on a closed \
         127.0.0.1:9; got {providers:?}. Refusing to go on."
    );
}

/// One raw request to OpenCode (no adapter in between), which must get a reply.
pub fn raw(port: u16, method: &str, path: &str, body: Option<&Value>) -> Reply {
    http::request(port, method, path, body, Duration::from_secs(10))
        .unwrap_or_else(|e| panic!("raw {method} {path} on {port}: {e:?}"))
}

/// One test's rig. The harness is dropped before the guard.
pub struct Rig {
    pub harness: Recording,
    pub ports: [u16; 2],
    pub panes: [PaneId; 2],
    pub config: OpenCodeConfig,
    tmux: PathBuf,
    sock: PathBuf,
    guard: Guard,
}

impl Rig {
    pub fn start() -> Rig {
        let (opencode, tmux_bin) = binaries();
        let dir = tempfile::Builder::new()
            .prefix("hlr642r-")
            .tempdir_in("/tmp")
            .unwrap();
        let root = dir.path().to_path_buf();
        let sock = root.join("tmux.sock");
        assert!(
            sock.as_os_str().len() < SUN_PATH,
            "{} is too long",
            sock.display()
        );
        let pids = Arc::new(Mutex::new(Vec::new()));
        // Built before the tmux server, so a setup that fails part-way still cleans up.
        let guard = Guard {
            pids: Arc::clone(&pids),
            tmux: tmux_bin.clone(),
            sock: sock.clone(),
            _dir: dir,
        };
        assert!(
            refused(DEAD_END),
            "127.0.0.1:9 accepts connections; the dead-end provider would not be dead. Refusing."
        );
        for sub in ["home", "data", "config/opencode", "state", "cache", "proj"] {
            std::fs::create_dir_all(root.join(sub)).unwrap();
        }
        std::fs::write(root.join("config/opencode/opencode.json"), CONFIG).unwrap();
        for session in SESSIONS {
            let out = tmux(
                &tmux_bin,
                &sock,
                &[
                    "-f",
                    "/dev/null",
                    "new-session",
                    "-d",
                    "-s",
                    session,
                    "sleep",
                    "3600",
                ],
            );
            assert!(out.status.success(), "tmux new-session {session}: {out:?}");
        }
        let config = config(&opencode, &tmux_bin, &sock, &root);
        Rig {
            harness: Recording {
                inner: OpenCodeHarness::new(config.clone()),
                pids,
            },
            ports: [free_port(), free_port()],
            panes: PANES.map(PaneId::new),
            config,
            tmux: tmux_bin,
            sock,
            guard,
        }
    }

    /// What the conformance suite takes for one case.
    pub fn into_case(self) -> (Recording, HarnessRig, Guard) {
        let rig = HarnessRig {
            ports: self.ports,
            panes: self.panes.clone(),
        };
        (self.harness, rig, self.guard)
    }

    /// Serve `SESSIONS[i]` on `ports[i]`. A port another process took since it was picked
    /// ("port N is in use") is replaced by a new one from the range, not a failure (AC 31).
    pub fn serve(&mut self, i: usize) -> u32 {
        let name = PaneName::parse(SESSIONS[i]).unwrap();
        for _ in 0..3 {
            match self.harness.serve(&name, self.ports[i]) {
                Ok(pid) => return pid,
                Err(PaneError::Unavailable { what }) if what.contains("is in use") => {
                    self.ports[i] = free_port();
                }
                Err(other) => panic!("serve({name}, {}): {other:?}", self.ports[i]),
            }
        }
        panic!("serve({name}): three ports in a row were taken by another process");
    }

    pub fn create(&self, i: usize) -> String {
        self.harness
            .create_session(self.ports[i])
            .unwrap_or_else(|e| panic!("create_session({}): {e:?}", self.ports[i]))
    }

    /// One `#{...}` format of a rig session's pane, read by raw tmux.
    pub fn pane_format(&self, session: &str, format: &str) -> String {
        let target = format!("={session}:");
        let out = tmux(
            &self.tmux,
            &self.sock,
            &["display-message", "-p", "-t", &target, format],
        );
        assert!(
            out.status.success(),
            "tmux display-message {format}: {out:?}"
        );
        String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_owned()
    }

    /// Poll `shown_session(pane)` until it is `want`, within `within`; the last answer.
    pub fn shown_within(
        &self,
        pane: &PaneId,
        want: &Option<String>,
        within: Duration,
    ) -> Result<Option<String>, PaneError> {
        let until = Instant::now() + within;
        loop {
            let shown = self.harness.shown_session(pane);
            if shown.as_ref() == Ok(want) || Instant::now() >= until {
                return shown;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// The adapter's config for a rig: the isolated env of `opencode-lib.sh:98-111`, `--pure`,
/// one scratch project dir for every pane, the private tmux socket, and a `tui_session`
/// that maps the rig's panes to their sessions and anything else to `pane-not-found`.
fn config(opencode: &Path, tmux_bin: &Path, sock: &Path, root: &Path) -> OpenCodeConfig {
    let dir = |sub: &str| root.join(sub).display().to_string();
    let bin_dir = opencode
        .parent()
        .map_or_else(String::new, |d| d.display().to_string());
    let mut vars: Vec<(String, String)> = [
        ("PATH", format!("{bin_dir}:/usr/bin:/bin")),
        ("HOME", dir("home")),
        ("XDG_DATA_HOME", dir("data")),
        ("XDG_CONFIG_HOME", dir("config")),
        ("XDG_STATE_HOME", dir("state")),
        ("XDG_CACHE_HOME", dir("cache")),
        ("TERM", "xterm-256color".to_owned()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    for flag in [
        "AUTOUPDATE",
        "MODELS_FETCH",
        "CLAUDE_CODE",
        "LSP_DOWNLOAD",
        "SHARE",
    ] {
        vars.push((format!("OPENCODE_DISABLE_{flag}"), "1".to_owned()));
    }
    let project = root.join("proj");
    OpenCodeConfig {
        opencode_bin: opencode.to_path_buf(),
        serve_args: vec!["--pure".to_owned()],
        env: ProcessEnv::Isolated(vars),
        tmux: TmuxConfig {
            tmux_bin: tmux_bin.to_path_buf(),
            socket: TmuxSocket::Path(sock.to_path_buf()),
        },
        workdir: Arc::new(move |_: &PaneName| Ok(project.clone())),
        tui_session: sessions_for(&[(PANES[0], SESSIONS[0]), (PANES[1], SESSIONS[1])]),
        timeouts: Timeouts::default(),
    }
}

/// A `tui_session` that maps each pane of `map` to its session name, and anything else to
/// `pane-not-found`.
pub fn sessions_for(map: &[(&str, &str)]) -> holler_adapter_opencode::Resolver<PaneId, PaneName> {
    let map: Vec<(String, PaneName)> = map
        .iter()
        .map(|&(pane, session)| (pane.to_owned(), PaneName::parse(session).unwrap()))
        .collect();
    Arc::new(move |pane: &PaneId| {
        map.iter()
            .find(|(known, _)| known == pane.as_str())
            .map(|(_, session)| session.clone())
            .ok_or_else(|| PaneError::PaneNotFound {
                what: pane.as_str().to_owned(),
            })
    })
}
