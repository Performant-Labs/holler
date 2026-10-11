//! `holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]` (story #664, epic
//! #633): converge the live panes on a profile, declaratively and idempotently.
//!
//! The verb types the profile's name (`usage`), runs the engine
//! (`holler_pane::tx_apply::apply`, with `TxOptions::default()`, as `pane launch` does) and
//! prints what it answered, by its exit code:
//!
//! - **0**: the steps. In JSON the envelope's `data` is the engine's `{"profile", "dry_run",
//!   "steps"}` itself; for a dry run those steps are the plan.
//! - **1, a `failed` step** (the decision on O6): only the failure, coded as that step's error,
//!   with the message `apply stopped at <pane> (<action>): <step error>; <N> later step(s)
//!   skipped`. No plan and no `skipped` list is printed, for a dry run too; the engine's result
//!   holds them.
//! - **An error before any plan** (a bad name, P not found, a registry that cannot be read, a
//!   `create` without `--herdr-session`): that error alone.
//!
//! **The reconcile step.** A step whose launch or relaunch had started its act before it was
//! refused or failed (since #802 the act can refuse: the port's re-check, `verify_serving`) may
//! leave the records and the live panes apart. So the verb ends that step's detail with the
//! reconcile step of P, as `pane launch` ends an acted failure: a `refused` step prints it with
//! its detail, and a `failed` step's message above carries it (ADR-0021 section 8, step 6).

use clap::Args;
use holler_pane::findings::quoted;
use holler_pane::tx_apply::{apply, Applied, ApplyRequest, Outcome, Step};
use holler_pane::tx_launch::TxOptions;
use holler_pane::{PaneError, ProfileName};

use super::list::count;
use super::show::{difference_line, text};
use crate::output::{emit, ErrorBody, ErrorCode, VerbCtx};
use crate::pane::profile_scope::reconcile_step;

/// Converge the live panes on a profile: create, adopt or relaunch, never kill a healthy pane.
///
/// One step per spec of the profile, in workspace, row and column order. A spec pane with no
/// record is created (through the launch transaction, which runs its health check and refuses an
/// occupied cell). A recorded pane that is healthy (its harness server answers, Herdr lists its
/// pane, and its TUI does not show another session than its session of record) is adopted, made a
/// member of the profile with nothing live touched, when it matches its spec, and reported when
/// it differs. An agent that is not healthy is relaunched in place; the orchestrator is never
/// relaunched. A pane that belongs to another profile is refused unless --take-over, which moves
/// it to this profile. A refused or reported step does not change the exit code; a failed step
/// stops the run (exit 1). --dry-run prints the plan and changes nothing.
///
/// Prints `profile NAME: N steps, applied` (or `planned (dry run; nothing changed)`), then one
/// line per step, `PANE: ACTION, OUTCOME`, with the code and the detail of a refused or failed
/// step, and under a reported step one `FIELD: spec VALUE, live VALUE` line per difference.
///
/// `--format=json` prints one envelope whose data is `{"profile", "dry_run", "steps"}`, each step
/// `{"pane", "action", "outcome", "code", "diff", "detail"}`.
#[derive(Args, Debug)]
pub struct ProfileApply {
    /// The profile's name (spaces allowed).
    pub name: String,
    /// Print the plan and change nothing: no probe, no split, no write.
    #[arg(long)]
    pub dry_run: bool,
    /// Take over a pane that belongs to another profile.
    #[arg(long)]
    pub take_over: bool,
    /// The Herdr session a spec pane with no record is created in. Required when there is one.
    #[arg(long, value_name = "NAME")]
    pub herdr_session: Option<String>,
}

/// Run `holler profile apply`: type the name, run the engine and print what it answered.
pub fn run(args: &ProfileApply, ctx: &mut VerbCtx<'_>) -> i32 {
    let ports = ctx.ports;
    let applied = ProfileName::parse(&args.name).and_then(|profile| {
        let request = ApplyRequest {
            profile,
            herdr_session: args.herdr_session.clone(),
            take_over: args.take_over,
            dry_run: args.dry_run,
        };
        apply(ports, &request, &TxOptions::default())
    });
    let result = applied
        .map(reconciled)
        .map_err(|error: PaneError| ErrorBody::from(&error))
        .and_then(|applied| match stopped(&applied) {
            Some(failure) => Err(failure),
            None => Ok(applied),
        });
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// `applied` with P's reconcile step at the end of the detail of every step whose launch or
/// relaunch had started its act, `refused` or `failed` (see the module docs). As `pane launch`
/// appends it to an acted failure, it follows `"; "`, and only when the detail lacks it (the
/// profile scope's own errors carry it). A step that acted nothing (a dry run's, row a's, a
/// refusal of launch's plan) is left as it is.
fn reconciled(mut applied: Applied) -> Applied {
    let reconcile = reconcile_step(Some(&applied.profile));
    for step in applied.steps.iter_mut().filter(|step| step.acted) {
        let detail = step
            .detail
            .take()
            .or_else(|| step.error.as_ref().map(ToString::to_string));
        step.detail = Some(match detail {
            Some(detail) if detail.contains(&reconcile) => detail,
            Some(detail) => format!("{detail}; {reconcile}"),
            None => reconcile.clone(),
        });
    }
    applied
}

/// The failure a run with a `failed` step prints, and nothing else (the decision on O6):
/// `apply stopped at <pane> (<action>): <step error>; <N> later step(s) skipped`, coded as the
/// step's error, whose text is the step's detail (with the reconcile step when it had acted).
/// `None` when no step failed.
fn stopped(applied: &Applied) -> Option<ErrorBody> {
    let (failed, error) = applied
        .steps
        .iter()
        .find(|step| step.outcome == Outcome::Failed)
        .and_then(|step| Some((step, step.error.as_ref()?)))?;
    let skipped = applied
        .steps
        .iter()
        .filter(|step| step.outcome == Outcome::Skipped)
        .count();
    let message = failed.detail.clone().unwrap_or_else(|| error.to_string());
    Some(ErrorBody {
        code: ErrorCode::from(error),
        message: format!(
            "apply stopped at {} ({}): {message}; {skipped} later step(s) skipped",
            quoted(&failed.pane),
            failed.action.as_str()
        ),
    })
}

/// The text form: the header, then one line per step, with a reported step's differences.
fn render(applied: &Applied) -> String {
    let done = if applied.dry_run {
        "planned (dry run; nothing changed)"
    } else {
        "applied"
    };
    let mut lines = vec![format!(
        "profile {}: {}, {done}",
        applied.profile,
        count(applied.steps.len(), "step")
    )];
    for step in &applied.steps {
        lines.push(step_line(step));
        if let Some(diff) = &step.diff {
            lines.extend(diff.differences.iter().map(difference_line));
        }
    }
    lines.join("\n")
}

/// `PANE: ACTION, OUTCOME`, then ` (CODE)` and `: DETAIL` when the step has them. The pane and
/// the detail are escaped as `profile show` escapes a stored string.
fn step_line(step: &Step) -> String {
    let mut line = format!(
        "{}: {}, {}",
        text(&step.pane),
        step.action.as_str(),
        step.outcome.as_str()
    );
    if let Some(code) = &step.code {
        line.push_str(&format!(" ({code})"));
    }
    if let Some(detail) = &step.detail {
        line.push_str(&format!(": {}", text(detail)));
    }
    line
}
