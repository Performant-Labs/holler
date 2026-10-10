//! `holler pane switch PANE SESSION [--as-operator] [--profile NAME]` (story #645): point a
//! pane at a session its harness server already has, its TUI and its record together, through
//! the engine `holler_pane::tx_switch`. `pane reset` runs the same engine, and this file holds
//! what the two verbs share: typing their arguments, running the engine and printing the
//! result ([`execute`], [`emit_outcome`]).
//!
//! - **Arguments.** A bad `PANE`, `--profile` or `SESSION` value is `usage` (exit 2), printed
//!   before any port is called.
//! - **Success** (exit 0). Text mode prints one line, `switched <pane> to session "<id>" (was
//!   "<id>")`, or `(was none)` when the pane had no session of record. JSON mode's data is
//!   `{"verb", "pane", "previous"}`: the record as stored, and the session of record before
//!   the run, so a script can undo a switch with a second one.
//! - **Failure.** The engine's one-line message (`SwitchFailure::message`), which ends with
//!   the reconcile step once the TUI may have moved; the exit code is its code's class.
//!
//! Session ids are printed quoted (`findings::quoted`): they come from a person or the
//! harness, so no control sequence reaches the terminal.

use clap::Args;
use holler_pane::findings::quoted;
use holler_pane::tx_switch::{
    self, parse_session_id, SwitchFailure, SwitchRequest, Switched, Target,
};
use holler_pane::{Pane, PaneError, PaneName, ProfileName};
use holler_proto::clock::now_millis;
use serde::Serialize;

use super::args::ProfileOpt;
use crate::output::{emit, emit_error, ErrorBody, ErrorCode, VerbCtx};

/// Point a pane at an existing session: its TUI and its record together.
#[derive(Args, Debug)]
pub struct PaneSwitch {
    /// The pane, e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: String,
    /// The harness session to show and record (it must exist on the pane's server).
    #[arg(value_name = "SESSION")]
    pub session: String,
    /// Allow the orchestrator's own pane (the operator's deliberate act).
    #[arg(long)]
    pub as_operator: bool,
    #[command(flatten)]
    pub profile: ProfileOpt,
}

/// Run `holler pane switch`: move the pane to `SESSION`, then print the result.
pub fn run(args: &PaneSwitch, ctx: &mut VerbCtx<'_>) -> i32 {
    let target = parse_session_id(&args.session).map(Target::Existing);
    execute(
        ctx,
        Verb::Switch,
        &args.pane,
        &args.profile,
        args.as_operator,
        target,
    )
}

/// The verb a run is; its JSON form (`data.verb`) is its lower-case name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Verb {
    Switch,
    Reset,
}

/// Run one switch or reset and print it; returns the exit code. `pane` and `profile` are the
/// verb's `PANE` and `--profile`, typed here in that order, then `target` (switch's typed
/// `SESSION`, or reset's fresh session). The first that fails is printed as `usage` before
/// any port is called.
pub(crate) fn execute(
    ctx: &mut VerbCtx<'_>,
    verb: Verb,
    pane: &str,
    profile: &ProfileOpt,
    as_operator: bool,
    target: Result<Target, PaneError>,
) -> i32 {
    match request(pane, profile, as_operator, target) {
        Ok(request) => {
            let result = tx_switch::switch(ctx.ports, &request);
            emit_outcome(ctx, verb, &request.pane, result)
        }
        Err(error) => emit_error(&mut ctx.sink, ctx.format, ErrorBody::from(&error)),
    }
}

/// The engine's request: the arguments typed, and the clock now.
fn request(
    pane: &str,
    profile: &ProfileOpt,
    as_operator: bool,
    target: Result<Target, PaneError>,
) -> Result<SwitchRequest, PaneError> {
    let pane = PaneName::parse(pane)?;
    let profile = profile
        .profile
        .as_deref()
        .map(ProfileName::parse)
        .transpose()?;
    Ok(SwitchRequest {
        pane,
        profile,
        target: target?,
        as_operator,
        now_ms: now_millis(),
    })
}

/// The `data` of a run that recorded its target.
#[derive(Debug, Serialize)]
struct Outcome {
    verb: Verb,
    /// The record as stored.
    pane: Pane,
    /// The session of record before the run; `null` when there was none.
    previous: Option<String>,
}

/// Print a run's result through `output::emit` (shared with reset.rs) and return the exit
/// code: the data on success, else the failure's code with its message for `pane`.
pub(crate) fn emit_outcome(
    ctx: &mut VerbCtx<'_>,
    verb: Verb,
    pane: &PaneName,
    result: Result<Switched, SwitchFailure>,
) -> i32 {
    let result = match result {
        Ok(Switched {
            pane: stored,
            previous,
        }) => Ok(Outcome {
            verb,
            pane: stored,
            previous,
        }),
        Err(failure) => Err(ErrorBody {
            code: ErrorCode::from(&failure.error),
            message: failure.message(pane),
        }),
    };
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// The text form of a run that recorded its target: one line.
fn render(outcome: &Outcome) -> String {
    let name = &outcome.pane.name;
    let now = session_text(outcome.pane.session_of_record.as_deref());
    let was = session_text(outcome.previous.as_deref());
    match outcome.verb {
        Verb::Switch => format!("switched {name} to session {now} (was {was})"),
        Verb::Reset => format!("reset {name} to a new session {now} (was {was})"),
    }
}

/// A session id as a line of text shows it: quoted, or `none`.
fn session_text(session: Option<&str>) -> String {
    session.map_or_else(|| "none".to_owned(), quoted)
}
