//! What a harness call is keyed by, and the vocabulary of a single-process harness (#831,
//! ADR-0021 section 15 items 3 and 5).
//!
//! - [`HarnessEndpoint`]: where a harness is reached. An OpenCode harness is a server on a
//!   port; a Claude Code harness is one process, with no port. Every [`HarnessPort`] method that
//!   was keyed by `port: u16` is keyed by an endpoint, and `HarnessInfo::endpoint` is the one
//!   place a record becomes one.
//! - [`SessionChoice`]: the session a single process is started on, which carries the intent of
//!   the verb that starts it (a launch, a relaunch, a switch), so a switch to a conversation
//!   that exists nowhere is refused before anything stops (plan review W1).
//! - [`LoginStatus`]: what `HarnessPort::login_status` reads of a subscription login, and
//!   nothing else of it (I7).
//! - Two refusals for a call that reaches a harness of the other shape: [`PROCESS_UNSUPPORTED`]
//!   (a server harness asked about a process, the default of the four process methods) and
//!   [`SERVER_UNSUPPORTED`] (a single-process harness asked for a server's or a TUI's step).
//!
//! [`HarnessPort`]: super::HarnessPort

use std::fmt;

use crate::error::{PaneError, RefusalCode};
use crate::pane::PaneName;

/// `process-unsupported`: a harness that drives no single process was asked a process's
/// question: the default of `HarnessPort::process_command`, `new_session_id`,
/// `locate_process` and `login_status`, and the answer of a server harness to an endpoint
/// that names a process (#831). Refusal, exit 3.
pub const PROCESS_UNSUPPORTED: RefusalCode = RefusalCode::from_static("process-unsupported");

/// `server-unsupported`: a single-process harness was asked for a server's or a TUI's step
/// (`serve_command`, `verify_serving`, `create_session`, `list_sessions`, `attach_tui`,
/// `select_session`), or given an endpoint that names a port (#831). Its sessions are chosen
/// when its process starts, so the engines relaunch it instead. Refusal, exit 3.
pub const SERVER_UNSUPPORTED: RefusalCode = RefusalCode::from_static("server-unsupported");

/// `abort-unsupported`: `HarnessPort::abort` of a harness whose kind's abort style is a
/// relaunch (`AbortStyle::Relaunch`): it has no in-place abort, and a signal is not one (a
/// SIGINT ends a Claude Code process). The operator's decision of 2026-10-11: such a pane's
/// interrupt is a relaunch on its session of record, never a keypress. Refusal, exit 3.
pub const ABORT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("abort-unsupported");

/// `session-mismatch`: `HarnessPort::send_prompt` to a session that the single process at the
/// endpoint does not hold (a person's `/clear` or `/resume` moved it): nothing is delivered.
/// Refusal, exit 3.
pub const SESSION_MISMATCH: RefusalCode = RefusalCode::from_static("session-mismatch");

/// `inbound-held`: `HarnessPort::send_prompt` whose message the single process held behind a
/// dialog instead of running it (a process that does not accept a cross-session message, one
/// Holler did not start): its status turned to a wait. Refusal, exit 3.
pub const INBOUND_HELD: RefusalCode = RefusalCode::from_static("inbound-held");

/// Where a pane's harness is reached (#831, ADR-0021 section 15 item 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessEndpoint {
    /// A harness server on this port of 127.0.0.1 (OpenCode).
    Port(u16),
    /// The one process of a single-process harness (Claude Code): the pane whose tmux session
    /// runs it, and its pid, which is its identity.
    Process { pane: PaneName, pid: u32 },
}

impl HarnessEndpoint {
    /// The port of a server endpoint; [`PROCESS_UNSUPPORTED`] for a process endpoint, which a
    /// server harness cannot reach. The one conversion a server harness makes.
    pub fn port(&self) -> Result<u16, PaneError> {
        match self {
            HarnessEndpoint::Port(port) => Ok(*port),
            HarnessEndpoint::Process { pane, pid } => Err(PaneError::Refused {
                code: PROCESS_UNSUPPORTED,
                message: format!(
                    "the process {pid} of {pane} is not a harness server: this harness reaches \
                     a server on a port"
                ),
            }),
        }
    }

    /// The pane and the pid of a process endpoint; [`SERVER_UNSUPPORTED`] for a port, which a
    /// single-process harness has none of. The one conversion a single-process harness makes.
    pub fn process(&self) -> Result<(&PaneName, u32), PaneError> {
        match self {
            HarnessEndpoint::Process { pane, pid } => Ok((pane, *pid)),
            HarnessEndpoint::Port(port) => Err(PaneError::Refused {
                code: SERVER_UNSUPPORTED,
                message: format!(
                    "port {port} names a harness server: this harness is one process and has \
                     no port"
                ),
            }),
        }
    }
}

impl fmt::Display for HarnessEndpoint {
    /// `port 48100`, or `process 4242 of demo-c1r2`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarnessEndpoint::Port(port) => write!(f, "port {port}"),
            HarnessEndpoint::Process { pane, pid } => write!(f, "process {pid} of {pane}"),
        }
    }
}

/// The session a single process is started on, as `HarnessPort::process_command` receives it
/// (#831, plan review W1). Each choice names the session; they differ in what the harness does
/// when the session has no conversation yet (a session's conversation, its transcript, exists
/// only once it was first prompted):
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionChoice {
    /// A new session, on its chosen id (`launch`, `reset`): started on the id.
    Fresh(String),
    /// The session of record (`relaunch`, `interrupt`, doctor's repair): resumed when it has
    /// a conversation, started on its id when it has none.
    Keep(String),
    /// Another conversation (`switch`): resumed, and `session-not-found` when it has no
    /// conversation, so a switch to an unknown id starts and records nothing.
    Resume(String),
}

impl SessionChoice {
    /// The session the choice names.
    pub fn session(&self) -> &str {
        match self {
            SessionChoice::Fresh(session)
            | SessionChoice::Keep(session)
            | SessionChoice::Resume(session) => session,
        }
    }
}

/// What `HarnessPort::login_status` reads of the login a harness's processes use (ADR-0021
/// section 15 item 10). Only whether it is the subscription login is kept; nothing of the
/// login itself is read, copied or printed (I7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStatus {
    /// The processes use the subscription login.
    Ok,
    /// They do not: no login, or one that bills another way.
    Missing,
}

/// The default answer of a process method (`call`) of a harness that drives no single
/// process: [`PROCESS_UNSUPPORTED`].
pub(crate) fn process_unsupported(call: &str) -> PaneError {
    PaneError::Refused {
        code: PROCESS_UNSUPPORTED,
        message: format!("this harness drives no single process, so it does not answer {call}"),
    }
}
