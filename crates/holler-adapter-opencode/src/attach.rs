//! The TUI side of `HarnessPort` (#642): `attach_tui`, `select_session` and
//! `shown_session`, to which the crate root delegates in one line each, as it delegates
//! `serve` to `server`. A pane's TUI is an `opencode attach` that runs in the pane's tmux
//! session (the one `tui_session` names). The adapter reaches it only through tmux, by the
//! calls [`crate::tui`] builds, and reads what it shows only from its terminal title.
//!
//! - **One deadline per method**, taken at its entry: every tmux call and every request of
//!   the method runs within it, and past it the answer is `timeout` with the method's `op`.
//! - **Observed, never remembered** (I6): the port of a pane's TUI is read from the pane's
//!   start command (`#{pane_start_command}`) by every call that needs it.
//! - **An attach or a switch is trusted only once seen** (I3). `select-session` reaches
//!   every TUI of a server and answers `true` with no TUI at all
//!   (opencode-pane-spike.md:117-124), so its `true` proves nothing. Both methods query
//!   the pane every `SETTLE_POLL` (the crate's one poll), within `settle`, until it is a
//!   live attach to that server whose title shows the session; otherwise the answer is
//!   `timeout`.
//! - **No keystroke is ever sent** (I4): the TUI is switched only over the API.

use std::time::{Duration, Instant};

use holler_pane::{PaneError, PaneId, PaneName};
use serde_json::{json, Value};

use crate::exec;
use crate::tui::{self, Query, Refusal, TitleShows};
use crate::{deadline_after, json_of, known, one_line, poll, session_path, session_reply};
use crate::{string_at, Call, OpenCodeHarness, TmuxConfig, GET_SESSION, SELECT};
use crate::{OP_ATTACH_TUI, OP_SELECT_SESSION, OP_SHOWN_SESSION};

/// `HarnessPort::attach_tui`: one existence check, which also gives the session's
/// directory (until it holds, the pane is not touched: cases 6 and 11); then the pane is
/// kept when its program exits (`remain-on-exit on`), its program is replaced by the TUI,
/// and the title is watched until it shows the session.
pub(crate) fn attach_tui(
    harness: &OpenCodeHarness,
    pane: &PaneId,
    port: u16,
    session: &str,
) -> Result<(), PaneError> {
    let config = &harness.config;
    let call = harness.call(port, OP_ATTACH_TUI);
    let path = session_path(session);
    let (reply, found) = session_reply(&call, &path, session)?;
    let directory = string_at(Some(&found), "directory")
        .ok_or_else(|| call.unexpected(GET_SESSION, &reply, "that session with its directory"))?;
    let bin = config
        .opencode_bin
        .to_str()
        .ok_or_else(|| PaneError::Unavailable {
            what: "the path of the opencode binary is not valid UTF-8".to_owned(),
        })?;
    let argv = tui::tui_argv(bin, &config.env, port, directory, session);
    let tui = Tui::of(harness, pane, OP_ATTACH_TUI, call.deadline)?;
    tui.change(&tui::remain_on_exit_args(&tui.session))?;
    tui.change(&tui::respawn_args(&tui.session, directory, &argv))?;
    match tui.watch(port, session, config.timeouts.settle)? {
        Seen::Shown => Ok(()),
        Seen::Dead(status) => Err(exited(&call, &path, session, &tui, status)),
        Seen::Gone => Err(tui.no_session()),
    }
}

/// `HarnessPort::select_session`. The method takes no port: the pane's TUI names the
/// server, so the TUI is found first, and a pane with no TUI is `unavailable` before
/// anything is sent (case 14). Then the session is checked (an unknown one is
/// `session-not-found`, and the screen stays: case 13), the switch is asked for, and the
/// title is watched until it shows the session.
pub(crate) fn select_session(
    harness: &OpenCodeHarness,
    pane: &PaneId,
    session: &str,
) -> Result<(), PaneError> {
    let config = &harness.config;
    let deadline = deadline_after(config.timeouts.call);
    let tui = Tui::of(harness, pane, OP_SELECT_SESSION, deadline)?;
    let port = match tui.query(deadline)? {
        Query::Live { start_command, .. } => tui::attach_port(&start_command),
        Query::Dead(_) | Query::NoPane => None,
    }
    .ok_or_else(|| tui.no_tui())?;
    let call = harness.call_until(port, OP_SELECT_SESSION, deadline);
    known(&call, &session_path(session), session)?;
    let reply = call.send(SELECT, SELECT.label, Some(&json!({ "sessionID": session })))?;
    if reply.status == 404 {
        return Err(PaneError::SessionNotFound {
            what: session.to_owned(),
        });
    }
    if json_of(&reply) != Some(Value::Bool(true)) {
        return Err(call.unexpected(SELECT, &reply, "true"));
    }
    match tui.watch(port, session, config.timeouts.settle)? {
        Seen::Shown => Ok(()),
        Seen::Dead(_) | Seen::Gone => Err(tui.no_tui()),
    }
}

/// `HarnessPort::shown_session`: one query of the pane, read by [`showing`]. It never asks
/// the server: a TUI keeps its screen while its server is frozen or dead
/// (opencode-pane-spike.md:193-194).
pub(crate) fn shown_session(
    harness: &OpenCodeHarness,
    pane: &PaneId,
) -> Result<Option<String>, PaneError> {
    let deadline = deadline_after(harness.config.timeouts.call);
    let tui = Tui::of(harness, pane, OP_SHOWN_SESSION, deadline)?;
    Ok(showing(&tui.query(deadline)?).map(|(_, id)| id))
}

/// The port and the session of a pane that runs an `opencode attach` to a loopback server
/// and whose title holds a whole session id; `None` for anything else, so nothing is
/// guessed: no tmux pane, a dead pane, another program, the home screen, a title that is
/// not an id.
fn showing(query: &Query) -> Option<(u16, String)> {
    let Query::Live {
        start_command,
        title,
    } = query
    else {
        return None;
    };
    let port = tui::attach_port(start_command)?;
    match tui::parse_title(title) {
        TitleShows::Session(id) => Some((port, id)),
        TitleShows::Home | TitleShows::Unrecognised => None,
    }
}

/// The TUI exited before its title showed `id`. The session is asked for again: one that
/// went away since the existence check is `session-not-found`; otherwise the exit is
/// `unavailable`, with its status.
fn exited(call: &Call, path: &str, id: &str, tui: &Tui<'_>, status: Option<i32>) -> PaneError {
    if let Err(gone @ PaneError::SessionNotFound { .. }) = known(call, path, id) {
        return gone;
    }
    let what = match status {
        Some(status) => format!("the TUI in pane {} exited with status {status}", tui.pane),
        None => format!("the TUI in pane {} exited, with no exit status", tui.pane),
    };
    PaneError::Unavailable { what }
}

/// What the watch of an attach or a switch saw.
enum Seen {
    /// A live attach to the server, whose title shows the session.
    Shown,
    /// The pane died (it stays because of `remain-on-exit on`), with its exit status when
    /// tmux gave one.
    Dead(Option<i32>),
    /// No tmux pane of the session.
    Gone,
}

/// A pane's TUI as tmux reaches it: how tmux is run, the tmux session `tui_session` named
/// for the pane, and the method's `op` and deadline.
struct Tui<'a> {
    tmux: &'a TmuxConfig,
    /// The pane, as a message names it: on one line.
    pane: String,
    session: PaneName,
    op: &'static str,
    deadline: Instant,
}

impl<'a> Tui<'a> {
    /// The TUI of `pane`, through the `tui_session` resolver, whose `Err` is returned as it
    /// is.
    fn of(
        harness: &'a OpenCodeHarness,
        pane: &PaneId,
        op: &'static str,
        deadline: Instant,
    ) -> Result<Self, PaneError> {
        let session = (harness.config.tui_session)(pane)?;
        Ok(Self {
            tmux: &harness.config.tmux,
            pane: one_line(pane.as_str()),
            session,
            op,
            deadline,
        })
    }

    /// The one query of the pane, within `until`. tmux saying that the session, its pane
    /// or the server is missing reads as [`Query::NoPane`].
    fn query(&self, until: Instant) -> Result<Query, PaneError> {
        Ok(match self.run(&tui::query_args(&self.session), until)? {
            Some(stdout) => tui::parse_query(&self.session, &stdout),
            None => Query::NoPane,
        })
    }

    /// A tmux call that changes the pane, within the method's deadline. A missing session
    /// or pane is `unavailable` ("no tmux session for pane P").
    fn change(&self, args: &[String]) -> Result<(), PaneError> {
        match self.run(args, self.deadline)? {
            Some(_) => Ok(()),
            None => Err(self.no_session()),
        }
    }

    /// Query the pane every `SETTLE_POLL`, through the crate's one poll, until it is a live
    /// attach to the server on `port` whose title shows `id`, a dead pane or no pane, within
    /// `settle` and the method's deadline. None of them in time is `timeout`.
    fn watch(&self, port: u16, id: &str, settle: Duration) -> Result<Seen, PaneError> {
        poll(settle, self.deadline, self.op, |until| {
            let query = self.query(until)?;
            Ok(match query {
                Query::NoPane => Some(Seen::Gone),
                Query::Dead(status) => Some(Seen::Dead(status)),
                Query::Live { .. } => showing(&query)
                    .is_some_and(|(at, shows)| at == port && shows == id)
                    .then_some(Seen::Shown),
            })
        })
    }

    /// One tmux call, within `until`: its stdout, or `None` when tmux says the session,
    /// its pane or the server is missing. Any other failure is `unavailable` with tmux's
    /// first stderr line, never one of the call's arguments.
    fn run(&self, args: &[String], until: Instant) -> Result<Option<String>, PaneError> {
        let ran = exec::capture(tui::tmux_command(self.tmux, args), self.op, until)?;
        if ran.status.success() {
            return Ok(Some(ran.stdout));
        }
        match tui::classify(&ran.stderr) {
            Refusal::Missing => Ok(None),
            Refusal::Other => Err(PaneError::Unavailable {
                what: format!("tmux failed for pane {}: {}", self.pane, ran.reason()),
            }),
        }
    }

    fn no_session(&self) -> PaneError {
        PaneError::Unavailable {
            what: format!("no tmux session for pane {}", self.pane),
        }
    }

    fn no_tui(&self) -> PaneError {
        PaneError::Unavailable {
            what: format!("no TUI in pane {}", self.pane),
        }
    }
}
