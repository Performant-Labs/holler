#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #850
#![allow(dead_code)] // #850: each test target that includes this rig uses a subset of it
//! The fake Claude Code process a real `holler body run` drives (#850, Acceptance 4 and 5): the
//! program that stands for `claude`, and the socket that stands for a process's inbox. Nothing
//! here runs a real Claude Code, reads a login or touches a pane.
//!
//! - **The program** is a `#!/bin/sh` script in the state directory, named to the body by
//!   `HOLLER_CLAUDE_BIN`. `agents --json` prints `agents.json`, which the test rewrites to move
//!   the process through `idle`, `busy` and `waiting`; every run's arguments are appended to
//!   `argv.log` and the names of its environment to `env.log`; any other command exits 2, so a
//!   body that ran something else (`auth status`, a login read) is seen in `argv.log`.
//! - **The inbox** is a listener on a unix socket, whose path the rig writes to
//!   `<state dir>/claude-inbox/<pid>` (the directory `0700`, the file `0600`), as the launch hook
//!   would. Every line written to it is kept for the test.
//!
//! The including test declares `mod support;` first (this module uses it).

use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::support::{wait_for, StateDir};

/// The pid the fake process runs as, and a placeholder session of record (a UUID).
pub const PID: u32 = 4_242_424;
pub const SESSION: &str = "0b6f8f2e-5c1d-4a43-9d51-2f1f6a7f0a11";
/// A second process and session, for a two-pane rig.
pub const PID_TWO: u32 = 4_242_425;
pub const SESSION_TWO: &str = "1c7a9e30-8d2b-4f65-a1c4-3e5d7b9f2c42";

/// A sentinel that stands for a credential in the body's environment. It must never reach the
/// fake program, a log or an output.
pub const SENTINEL_KEY: &str = "sk-ant-test-sentinel-not-a-real-key";

/// The variables that change a Claude Code login (ADR-0021 section 15 item 10).
pub const LOGIN_VARIABLES: [&str; 7] = [
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

/// What the fake process says about itself in `agents --json`.
#[derive(Debug, Clone, Copy)]
pub struct Process<'a> {
    pub pid: u32,
    pub session: &'a str,
    pub status: &'a str,
    pub waiting_for: Option<&'a str>,
}

impl<'a> Process<'a> {
    pub fn idle(pid: u32, session: &'a str) -> Self {
        Self { pid, session, status: "idle", waiting_for: None }
    }

    pub fn busy(pid: u32, session: &'a str) -> Self {
        Self { pid, session, status: "busy", waiting_for: None }
    }

    fn json(&self) -> String {
        let waiting = self
            .waiting_for
            .map(|w| format!(",\"waitingFor\":\"{w}\""))
            .unwrap_or_default();
        format!(
            "{{\"cwd\":\"/srv/demo\",\"kind\":\"interactive\",\"name\":\"demo\",\"pid\":{},\
             \"sessionId\":\"{}\",\"startedAt\":1,\"status\":\"{}\"{waiting}}}",
            self.pid, self.session, self.status
        )
    }
}

/// The program, its scripted state and the inbox listener of one test.
pub struct FakeProgram {
    dir: PathBuf,
    inbox_dir: PathBuf,
    lines: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    listener: Option<JoinHandle<()>>,
}

impl FakeProgram {
    /// A program that lists no process, and an inbox listener, under `state`.
    pub fn new(state: &StateDir) -> Self {
        let dir = state.path().join("fake-claude");
        std::fs::create_dir_all(&dir).unwrap();
        let inbox_dir = state.path().join("claude-inbox");
        std::fs::create_dir_all(&inbox_dir).unwrap();
        std::fs::set_permissions(&inbox_dir, std::fs::Permissions::from_mode(0o700)).unwrap();

        let script = format!(
            "#!/bin/sh\n\
             printf '%s\\n' \"$*\" >> '{dir}/argv.log'\n\
             env | cut -d= -f1 | sort >> '{dir}/env.log'\n\
             if [ \"$1\" = agents ] && [ \"$2\" = --json ]; then\n  cat '{dir}/agents.json'\n  exit 0\nfi\n\
             echo \"fake claude: unexpected arguments\" >&2\nexit 2\n",
            dir = dir.display()
        );
        let bin = dir.join("claude");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

        let socket = dir.join("inbox.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        listener.set_nonblocking(true).unwrap();
        let lines: Arc<Mutex<Vec<String>>> = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let (lines, stop) = (Arc::clone(&lines), Arc::clone(&stop));
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            stream.set_nonblocking(false).unwrap();
                            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                                lines.lock().unwrap_or_else(PoisonError::into_inner).push(line);
                            }
                        }
                        Err(_) => std::thread::sleep(Duration::from_millis(10)),
                    }
                }
            })
        };
        let rig = Self { dir, inbox_dir, lines, stop, listener: Some(handle) };
        rig.set_processes(&[]);
        rig
    }

    /// The program's path, for `HOLLER_CLAUDE_BIN`.
    pub fn bin(&self) -> PathBuf {
        self.dir.join("claude")
    }

    /// The environment a body needs to drive this program, plus a credential sentinel that must
    /// never reach it (the adapter removes every login variable from the program's environment).
    pub fn env(&self) -> Vec<(String, String)> {
        vec![
            ("HOLLER_CLAUDE_BIN".to_owned(), self.bin().display().to_string()),
            ("ANTHROPIC_API_KEY".to_owned(), SENTINEL_KEY.to_owned()),
        ]
    }

    /// What `agents --json` prints from now on (written whole, then renamed into place).
    pub fn set_processes(&self, processes: &[Process<'_>]) {
        let body: Vec<String> = processes.iter().map(Process::json).collect();
        let tmp = self.dir.join("agents.json.tmp");
        std::fs::write(&tmp, format!("[{}]", body.join(","))).unwrap();
        std::fs::rename(&tmp, self.dir.join("agents.json")).unwrap();
    }

    /// Record the inbox socket of the process `pid`, as its launch hook would.
    pub fn register(&self, pid: u32) {
        let file = self.inbox_dir.join(pid.to_string());
        std::fs::write(&file, self.dir.join("inbox.sock").display().to_string()).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// Every line written to the inbox so far.
    pub fn frames(&self) -> Vec<String> {
        self.lines.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Every run of the program so far: its arguments, one line each.
    pub fn runs(&self) -> Vec<String> {
        read_lines(&self.dir.join("argv.log"))
    }

    /// How many times the program listed the processes (`agents --json`).
    pub fn listings(&self) -> usize {
        self.runs().iter().filter(|run| run.as_str() == "agents --json").count()
    }

    /// The names of the environment every run saw.
    pub fn env_names(&self) -> Vec<String> {
        read_lines(&self.dir.join("env.log"))
    }

    /// Wait, bounded, until the program has listed the processes `more` more times than it has
    /// now: a body that polls has then read what the test wrote before this call, twice over.
    pub fn until_listed(&self, more: usize, timeout: Duration) -> bool {
        let target = self.listings() + more;
        wait_for(timeout, || (self.listings() >= target).then_some(())).is_some()
    }

    /// Wait, bounded, for the inbox to hold `count` lines.
    pub fn wait_frames(&self, count: usize, timeout: Duration) -> Vec<String> {
        wait_for(timeout, || {
            let frames = self.frames();
            (frames.len() >= count).then_some(frames)
        })
        .unwrap_or_else(|| panic!("the inbox never held {count} line(s): {:?}", self.frames()))
    }
}

impl Drop for FakeProgram {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.listener.take() {
            let _ = handle.join();
        }
    }
}

fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// A `[[session]]` block for a Claude Code pane's row: no endpoint, the pane's session of record.
pub fn claude_row(name: &str, session: &str) -> String {
    format!("[[session]]\nname = \"{name}\"\nharness = \"claude\"\nmode = \"attach\"\nsession_id = \"{session}\"\n\n")
}

/// The frame the adapter writes to an inbox socket for `text`, as a value (the key order of the
/// line is serde's, which a test must not depend on). Read from the client's own startup log, not
/// documented: the adapter's opt-in real test is its guard.
pub fn frame_of(text: &str) -> serde_json::Value {
    serde_json::json!({"type": "user", "message": {"role": "user", "content": text}})
}

/// The lines of the inbox as values.
pub fn parsed(frames: &[String]) -> Vec<serde_json::Value> {
    frames.iter().map(|line| serde_json::from_str(line).unwrap()).collect()
}
