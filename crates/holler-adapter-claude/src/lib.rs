//! `holler_adapter_claude` — the Claude Code adapter: it implements [`holler_pane::HarnessPort`]
//! for an interactive Claude Code process in a pane's tmux session (#831, ADR-0021 section 15).
//! The facts about real Claude Code it relies on come from the spike #830
//! (`docs/research/claude-code-pane-spike.md`, Claude Code 2.1.296).
//!
//! **A Claude Code pane is one process**, the harness, the TUI and the session holder at once,
//! so the adapter answers the methods of a single-process harness and refuses a server's and a
//! TUI's steps (`server-unsupported`): its sessions are chosen when its process starts, and the
//! pane engine relaunches it to switch, reset or interrupt it. The process runs in the pane's
//! tmux session, started by the host adapter's `run` from the argv this adapter builds and
//! stopped by its `stop_owned`; the adapter owns no process and keeps no pid.
//!
//! **Blocking**, as the port is: every method is synchronous (call it from `spawn_blocking` or a
//! thread in async code) and returns within I5's bound (10 s): each run of `claude` and each
//! socket write has its own bound ([`Timeouts`]), well inside it.
//!
//! How it is built, three seams that each return raw I/O the adapter parses ([`AgentsList`],
//! [`InboxWrite`], [`LoginCheck`]; [`ClaudeHarness::new`] builds the real ones, and
//! [`ClaudeHarness::with_seams`] takes others), and what each method answers:
//!
//! - `process_command(name, spec, choice)`: the launch argv, pure (the `argv` module): `env -u`
//!   each login variable, the socket directory for the launch's `SessionStart` hook, the
//!   program, `--session-id` or `--resume` by whether the session's transcript exists (its
//!   existence alone is read), `--settings` (cross-session messages accepted, the hook),
//!   `--model` and `--effort` from the spec, and `--disallowed-tools AskUserQuestion`. A resume
//!   of a session with no transcript is `session-not-found`; an agent is `agent-unsupported`.
//! - `new_session_id`: a version 4 UUID.
//! - `locate_process(name, session)`: the pid of the `claude agents --json` entry holding
//!   `session`. `health(at)`: whether an entry lists the pid. `shown_session(_, at)`: the
//!   session that entry holds now. `session_activity(at, _)`: that entry's status (the `agents`
//!   module; `harness-dialog-open` for a process held by a dialog).
//! - `send_prompt(at, session, text)`: one frame line to the process's inbox socket (the
//!   `inbox` module), never a keypress (I4): `session-mismatch` when the entry holds another
//!   session, `unavailable` when no socket is known for the pid (nothing is guessed), and
//!   `inbound-held` when the entry turns `waiting` within [`Timeouts::held_window`] of the
//!   write, having not waited before it: the message was held behind a dialog.
//! - `abort`: `abort-unsupported` (the operator's decision of 2026-10-11: no keypress, and a
//!   signal ends the process; the pane engine's interrupt relaunches it).
//! - `login_status`: `claude auth status`'s `authMethod` alone: `claude.ai` is the subscription
//!   login, anything else is not. An answer with no `authMethod` is `unavailable`, quoting none
//!   of it: nothing of the login is read, kept or printed (I7).
//! - `agent_view(at, _, _)` (#647 part 2): the empty view for a process endpoint, with nothing
//!   read: one process has no server that lists agents or turns, so it cannot tell. A port is
//!   `server-unsupported`, the one conversion, so a call routed to the wrong harness is loud.
//!
//! **What it reads and writes on disk.** Whether a transcript file exists under Claude Code's
//! configuration directory, never its contents nor any other file there (the registry files
//! under `sessions/` are not a stable interface and outlive a killed process). The socket files
//! its launch hook writes under the adapter's own socket directory, which the reader trusts only
//! when the directory and the file are the user's alone.

mod agents;
mod argv;
mod exec;
mod inbox;
mod seams;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use holler_pane::ports::{
    Activity, LoginStatus, SessionChoice, ABORT_UNSUPPORTED, INBOUND_HELD, SERVER_UNSUPPORTED,
    SESSION_MISMATCH,
};
use holler_pane::{
    AgentView, Argv, HarnessEndpoint, HarnessPort, PaneError, PaneId, PaneName, ServeSpec,
};
use serde_json::Value;

use crate::agents::Entry;

pub use agents::HARNESS_DIALOG_OPEN;

/// The `authMethod` of the subscription login.
const SUBSCRIPTION: &str = "claude.ai";

/// How often `send_prompt` reads the process's status during its held window.
const HELD_POLL: Duration = Duration::from_millis(100);

/// `claude agents --json`: what every running Claude Code process of the user is doing. The
/// adapter matches its entries by pid or by session.
pub trait AgentsList: Send + Sync {
    /// The command's stdout, as printed.
    fn agents_json(&self) -> Result<String, PaneError>;
}

/// One line to a unix socket: the inbox socket of a Claude Code process.
pub trait InboxWrite: Send + Sync {
    /// Write `line`, which holds no newline, and end it with one.
    fn write_line(&self, socket: &Path, line: &str) -> Result<(), PaneError>;
}

/// `claude auth status`: which login the panes' processes use. The adapter reads its
/// `authMethod` alone.
pub trait LoginCheck: Send + Sync {
    /// The command's stdout, as printed.
    fn auth_status_json(&self) -> Result<String, PaneError>;
}

/// The adapter's bounds, each well inside I5's 10 s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// One run of `claude agents --json` (about 90 ms a call in the spike). Default 3 s.
    pub agents: Duration,
    /// One run of `claude auth status`. Default 5 s.
    pub login: Duration,
    /// One write to an inbox socket. Default 2 s.
    pub write: Duration,
    /// How long after a write `send_prompt` watches the process's status for a held message.
    /// Default 600 ms (the spike saw a status change within 100 to 300 ms of the event).
    pub held_window: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            agents: Duration::from_secs(3),
            login: Duration::from_secs(5),
            write: Duration::from_secs(2),
            held_window: Duration::from_millis(600),
        }
    }
}

/// How the adapter reaches Claude Code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeConfig {
    /// The `claude` program, for the launch argv and the adapter's own two reads. Its path must
    /// be valid UTF-8, since it goes on an argv. tmux starts the process, so a bare name is
    /// looked up on the `PATH` of the tmux server; the wiring passes an absolute path.
    pub bin: PathBuf,
    /// Claude Code's configuration directory (`$HOME/.claude`): the adapter checks only whether
    /// a transcript file exists under it. The launch argv unsets `CLAUDE_CONFIG_DIR`, so the
    /// process uses this one.
    pub config_dir: PathBuf,
    /// The adapter's own directory for the socket files its launch hook writes, under the Holler
    /// state directory, outside the repository and outside Claude Code's configuration
    /// directory. The hook makes it with mode `0700`.
    pub socket_dir: PathBuf,
    /// The bounds.
    pub timeouts: Timeouts,
}

impl ClaudeConfig {
    /// A configuration with the default bounds.
    pub fn new(
        bin: impl Into<PathBuf>,
        config_dir: impl Into<PathBuf>,
        socket_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            bin: bin.into(),
            config_dir: config_dir.into(),
            socket_dir: socket_dir.into(),
            timeouts: Timeouts::default(),
        }
    }
}

/// The `HarnessPort` over Claude Code.
pub struct ClaudeHarness {
    config: ClaudeConfig,
    agents: Arc<dyn AgentsList>,
    inbox: Arc<dyn InboxWrite>,
    login: Arc<dyn LoginCheck>,
}

impl ClaudeHarness {
    /// An adapter with `config` and the real seams. It starts nothing until a method is called.
    pub fn new(config: ClaudeConfig) -> Self {
        let timeouts = config.timeouts;
        let agents = seams::ClaudeAgents {
            bin: config.bin.clone(),
            bound: timeouts.agents,
        };
        let login = seams::ClaudeLogin {
            bin: config.bin.clone(),
            bound: timeouts.login,
        };
        let inbox = seams::SocketLine {
            bound: timeouts.write,
        };
        Self::with_seams(config, Arc::new(agents), Arc::new(inbox), Arc::new(login))
    }

    /// An adapter with `config` over the seams given.
    pub fn with_seams(
        config: ClaudeConfig,
        agents: Arc<dyn AgentsList>,
        inbox: Arc<dyn InboxWrite>,
        login: Arc<dyn LoginCheck>,
    ) -> Self {
        Self {
            config,
            agents,
            inbox,
            login,
        }
    }

    /// Every entry of `claude agents --json` now.
    fn entries(&self) -> Result<Vec<Entry>, PaneError> {
        agents::parse(&self.agents.agents_json()?)
    }

    /// The entry of the process `pid`, if it runs.
    fn entry(&self, pid: u32) -> Result<Option<Entry>, PaneError> {
        Ok(self.entries()?.into_iter().find(|entry| entry.pid == pid))
    }

    /// The entry of the process `at` names, which must run: `unavailable` otherwise.
    fn running(&self, at: &HarnessEndpoint) -> Result<Entry, PaneError> {
        let (pane, pid) = at.process()?;
        self.entry(pid)?.ok_or_else(|| PaneError::Unavailable {
            what: format!(
                "the harness process {pid} of {pane} does not run (claude agents lists no such \
                 process)"
            ),
        })
    }

    /// After a write to the process `pid`, whose status before it was `before`: watch its
    /// status for [`Timeouts::held_window`]. Turning `waiting`, having not waited before, is
    /// `inbound-held`; turning `busy` (the message started a turn), or the window passing, is
    /// delivered.
    fn watch_held(&self, pane: &PaneName, pid: u32, before: &str) -> Result<(), PaneError> {
        if before == "waiting" {
            return Ok(());
        }
        let until = Instant::now() + self.config.timeouts.held_window;
        while Instant::now() < until {
            thread::sleep(HELD_POLL.min(until.saturating_duration_since(Instant::now())));
            let Some(entry) = self.entry(pid)? else {
                return Ok(());
            };
            match entry.status.as_str() {
                "busy" => return Ok(()),
                "waiting" => {
                    return Err(PaneError::Refused {
                        code: INBOUND_HELD,
                        message: format!(
                            "the harness process {pid} of {pane} held the message behind a dialog \
                             a person must answer in its pane: it does not accept a cross-session \
                             message (it was not launched by holler)"
                        ),
                    })
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl HarnessPort for ClaudeHarness {
    fn serve_command(&self, _: &PaneName, _: u16, _: &ServeSpec) -> Result<Argv, PaneError> {
        Err(no_server("serve_command"))
    }

    fn verify_serving(&self, _: &HarnessEndpoint, _: &ServeSpec) -> Result<(), PaneError> {
        Err(no_server("verify_serving"))
    }

    fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
        let (_, pid) = at.process()?;
        Ok(self.entry(pid)?.is_some())
    }

    fn create_session(&self, _: &HarnessEndpoint) -> Result<String, PaneError> {
        Err(no_server("create_session"))
    }

    fn list_sessions(&self, _: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
        Err(no_server("list_sessions"))
    }

    fn abort(&self, at: &HarnessEndpoint, _session: &str) -> Result<(), PaneError> {
        let (pane, pid) = at.process()?;
        Err(PaneError::Refused {
            code: ABORT_UNSUPPORTED,
            message: format!(
                "the harness process {pid} of {pane} has no in-place abort, and a signal would end \
                 it; holler interrupt relaunches the pane on its session of record"
            ),
        })
    }

    fn attach_tui(&self, _: &PaneId, _: &HarnessEndpoint, _: &str) -> Result<(), PaneError> {
        Err(no_server("attach_tui"))
    }

    fn select_session(&self, _: &PaneId, _: &str) -> Result<(), PaneError> {
        Err(no_server("select_session"))
    }

    fn shown_session(&self, _: &PaneId, at: &HarnessEndpoint) -> Result<Option<String>, PaneError> {
        let (_, pid) = at.process()?;
        Ok(self.entry(pid)?.map(|entry| entry.session))
    }

    fn send_prompt(
        &self,
        at: &HarnessEndpoint,
        session: &str,
        text: &str,
    ) -> Result<(), PaneError> {
        let (pane, pid) = at.process()?;
        let entry = self.running(at)?;
        if entry.session != session {
            return Err(PaneError::Refused {
                code: SESSION_MISMATCH,
                message: format!(
                    "the harness process {pid} of {pane} holds another session than the one \
                     named; nothing was delivered"
                ),
            });
        }
        let socket = inbox::socket_of(&self.config.socket_dir, pid)?;
        self.inbox.write_line(&socket, &inbox::frame(text))?;
        self.watch_held(pane, pid, &entry.status)
    }

    fn session_activity(
        &self,
        at: &HarnessEndpoint,
        _session: &str,
    ) -> Result<Activity, PaneError> {
        agents::activity(&self.running(at)?)
    }

    fn process_command(
        &self,
        name: &PaneName,
        spec: &ServeSpec,
        session: &SessionChoice,
    ) -> Result<Argv, PaneError> {
        argv::process_command(&self.config, name, spec, session)
    }

    fn new_session_id(&self) -> Result<String, PaneError> {
        argv::new_session_id()
    }

    fn locate_process(&self, _name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
        Ok(self
            .entries()?
            .into_iter()
            .find(|entry| entry.session == session)
            .map(|entry| entry.pid))
    }

    fn login_status(&self) -> Result<LoginStatus, PaneError> {
        let text = self.login.auth_status_json()?;
        let method = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|answer| answer.get("authMethod")?.as_str().map(str::to_owned));
        match method.as_deref() {
            Some(SUBSCRIPTION) => Ok(LoginStatus::Ok),
            Some(_) => Ok(LoginStatus::Missing),
            None => Err(PaneError::Unavailable {
                what: "claude auth status did not answer a JSON object with an authMethod"
                    .to_owned(),
            }),
        }
    }

    fn agent_view(
        &self,
        at: &HarnessEndpoint,
        _session: &str,
        _max_turns: usize,
    ) -> Result<AgentView, PaneError> {
        at.process()?;
        Ok(AgentView::default())
    }
}

/// A server's or a TUI's step (`call`), which a Claude Code process has none of:
/// `server-unsupported`.
fn no_server(call: &str) -> PaneError {
    PaneError::Refused {
        code: SERVER_UNSUPPORTED,
        message: format!(
            "{call} is a server's or a TUI's step, and a Claude Code pane is one process: the pane \
             engine relaunches it instead"
        ),
    }
}
