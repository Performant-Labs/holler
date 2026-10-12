//! `holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]` (story #664, epic
//! #633): converge the live panes on a profile, declaratively and idempotently.
//!
//! The verb types the profile's name (`usage`), runs the engine
//! (`holler_pane::tx_apply::apply`, with `TxOptions::default()`, as `pane launch` does) and
//! prints what it answered, by its exit code, decided in this order:
//!
//! - **1, a `failed` step** (the decision on O6): only the failure, coded as that step's error,
//!   with the message `apply stopped at <pane> (<action>): <step error>; <N> later step(s)
//!   skipped`. No plan and no `skipped` list is printed, for a dry run too; the engine's result
//!   holds them.
//! - **3, a real run with a `refused` step** (#853, ADR-0021 decision 33): only the refusal,
//!   coded as the first refused step's error, with the message `apply refused <N> step(s):
//!   <pane> (<code>), ...; <M> other step(s) ran`, then `; <pane>: <detail>` for each refused
//!   step that had started its act. Refusals do not stop the run, so every step ran; the
//!   steps are not printed (an envelope carries no data with a failure), and a script that
//!   needs them runs `--dry-run` first.
//! - **0**: the steps. In JSON the envelope's `data` is the engine's `{"profile", "dry_run",
//!   "steps"}` itself; for a dry run those steps are the plan, refused steps included.
//! - **An error before any plan** (a bad name, P not found, a registry that cannot be read, a
//!   `create` without `--herdr-session`): that error alone.
//!
//! **The reconcile step.** A step whose launch or relaunch had started its act before it was
//! refused or failed (since #802 the act can refuse: the port's re-check, `verify_serving`) may
//! leave the records and the live panes apart. So the verb ends that step's detail with the
//! reconcile step of P, as `pane launch` ends an acted failure, and the line the run prints
//! carries that detail: the refusal line for a `refused` step, the failure line for a `failed`
//! one (ADR-0021 section 8, step 6). A dry run acts nothing, so its plan carries none.

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
/// One step per spec of the profile, in creation order: by workspace, then its column 1 by row,
/// then its other cells by row and column, so a whole grid can be built from an empty workspace
/// (Herdr adds a row only below a row of one pane). A spec pane with no record is created
/// (through the launch transaction, which runs its health check and refuses an occupied cell). A
/// recorded pane that is healthy (its harness server answers, Herdr lists its pane, and its TUI
/// does not show another session than its session of record) is adopted, made a member of the
/// profile with nothing live touched, when it matches its spec, and reported when it differs. An
/// agent that is not healthy is relaunched in place; the orchestrator is never relaunched. A pane
/// that belongs to another profile is refused unless --take-over, which moves it to this profile.
/// A refused step does not stop the run, but the run then exits 3, printing one line that names
/// the refused steps; a reported step leaves the exit code at 0; a failed step stops the run
/// (exit 1). --dry-run prints the plan, exits 0 and changes nothing.
///
/// Prints `profile NAME: N steps, applied` (or `planned (dry run; nothing changed)`), then one
/// line per step, `PANE: ACTION, OUTCOME`, with the code and the detail of a refused step, and
/// under a reported step one `FIELD: spec VALUE, live VALUE` line per difference. A run that
/// exits 1 or 3 prints one `error:` line instead.
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
        .and_then(
            |applied| match stopped(&applied).or_else(|| refused(&applied)) {
                Some(failure) => Err(failure),
                None => Ok(applied),
            },
        );
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

/// The refusal a real run with a `refused` step and no `failed` one prints, and nothing else
/// (#853, decision 33): `apply refused <N> step(s): <pane> (<code>), ...; <M> other step(s)
/// ran`, naming every refused step in step order, coded as the first one's error (a refusal, so
/// exit 3). Then each refused step whose launch or relaunch had started its act adds
/// `; <pane>: <detail>`, its detail as [`reconciled`] left it, ending with the reconcile step; a
/// step that acted nothing adds nothing beyond its pane and code. `None` for a dry run, which
/// prints its plan whatever it refuses, and for a run with no refused step.
fn refused(applied: &Applied) -> Option<ErrorBody> {
    if applied.dry_run {
        return None;
    }
    let refusals: Vec<(&Step, &PaneError)> = applied
        .steps
        .iter()
        .filter(|step| step.outcome == Outcome::Refused)
        .filter_map(|step| Some((step, step.error.as_ref()?)))
        .collect();
    let &(_, first) = refusals.first()?;
    let named: Vec<String> = refusals
        .iter()
        .map(|(step, error)| format!("{} ({})", quoted(&step.pane), error.code()))
        .collect();
    let ran = applied.steps.len() - refusals.len();
    let carried: String = refusals
        .iter()
        .filter(|(step, _)| step.acted)
        .filter_map(|(step, _)| Some((quoted(&step.pane), step.detail.as_deref()?)))
        .map(|(pane, detail)| format!("; {pane}: {detail}"))
        .collect();
    Some(ErrorBody {
        code: ErrorCode::from(first),
        message: format!(
            "apply refused {} step(s): {}; {ran} other step(s) ran{carried}",
            refusals.len(),
            named.join(", ")
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
