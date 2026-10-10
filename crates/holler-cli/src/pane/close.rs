//! `holler pane close PANE [--profile NAME] [--spec-only]` (story #646, part 2): stop a pane's
//! owned processes, close its Herdr pane and delete its record (ADR-0021 section 8, "Close as
//! built").
//!
//! - **Plan.** `PANE` and `--profile` are typed (`usage`). With `--profile P`, P is read
//!   (`profile-not-found`) and must hold a spec for the pane (`pane-not-in-profile`): this is
//!   checked here, before any write, because `ProfileScope::edit_spec` removes a missing spec as
//!   a no-op that still writes P. A live close then reads the record (`pane-not-found`).
//! - **Act.** `HostPort::stop_owned`, then `HerdrPort::close` on the recorded Herdr pane, then
//!   `PaneStore::delete` at the generation the record was read at, each only after the one
//!   before succeeded. Nothing calls `ensure_pane`, so no pane moves in Herdr, and no keystroke
//!   is sent (I4).
//! - **`--profile P`** runs the act inside `ProfileScope::edit_spec(P, pane, Remove, act)` (I8):
//!   P loses the spec first, and an act that fails puts it back, so P's specs are unchanged (its
//!   generation then moved by two). `--spec-only` removes the spec and closes nothing; it needs no
//!   record, since a detached spec for a gone pane is what it cleans up. Without `--profile` no
//!   profile is read or written.
//! - **Failure.** A failure from `stop_owned` on names the pane and the step it reached, and ends
//!   with the reconcile step (`profile_scope::reconcile_step` of the run's profile), unless the
//!   profile scope's own message already carries it. A failure before it carries none: nothing
//!   live moved. Exit codes are `class_of`'s.
//! - **Output.** Text is one line: `closed <pane>`, `closed <pane> (removed from profile "<P>")`,
//!   or `removed <pane> from profile "<P>"` with `--spec-only`. JSON data is `{"pane"}`, with
//!   `"generation"`, P's after the run, when P was written.

use clap::Args;
use holler_pane::findings::quoted;
use holler_pane::{PaneError, PaneName, Ports, Profile, ProfileName, SpecEdit};
use serde::Serialize;

use super::args::{ProfileOpt, SpecOnly};
use super::launch::{spec_of, stored_profile};
use super::list::profile_name;
use super::profile_scope::reconcile_step;
use crate::output::{emit, ErrorBody, VerbCtx};

/// Close a pane: stop its processes, close its Herdr pane and remove its record.
#[derive(Args, Debug)]
pub struct PaneClose {
    /// The pane to close, e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: Option<String>,
    #[command(flatten)]
    pub profile: ProfileOpt,
    #[command(flatten)]
    pub spec_only: SpecOnly,
}

/// Run `holler pane close`: close the pane (or, with `--spec-only`, only remove its spec) and
/// print what was done.
pub fn run(args: &PaneClose, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = close(args, ctx.ports);
    emit(&mut ctx.sink, ctx.format, result, Closed::text)
}

/// What a run did. Its JSON keys are `pane` and, when P was written, `generation`.
#[derive(Debug, Serialize)]
struct Closed {
    pane: PaneName,
    /// P's generation after the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    generation: Option<u64>,
    /// The profile the spec was removed from, for the text line.
    #[serde(skip)]
    profile: Option<ProfileName>,
    /// Only the spec was removed.
    #[serde(skip)]
    spec_only: bool,
}

impl Closed {
    /// The text form, one line.
    fn text(&self) -> String {
        let name = &self.pane;
        match &self.profile {
            Some(profile) if self.spec_only => {
                format!("removed {name} from profile {}", quoted(profile.as_str()))
            }
            Some(profile) => format!(
                "closed {name} (removed from profile {})",
                quoted(profile.as_str())
            ),
            None => format!("closed {name}"),
        }
    }
}

/// The plan, then the act (or only the spec removal); see the module docs.
fn close(args: &PaneClose, ports: Ports<'_>) -> Result<Closed, ErrorBody> {
    let (name, profile) = plan(args, ports).map_err(|error| ErrorBody::from(&error))?;
    match (profile, args.spec_only.spec_only) {
        (Some(profile), true) => remove_spec(ports, name, profile),
        // clap requires `--profile` with `--spec-only`; never close live in its place.
        (None, true) => Err(ErrorBody::from(&PaneError::Usage {
            message: "--spec-only needs --profile NAME".to_owned(),
        })),
        (profile, false) => close_live(ports, name, profile),
    }
}

/// Type `PANE` and `--profile`, and with `--profile P` read P and check it holds a spec for the
/// pane. Returns the pane and P as stored.
fn plan(args: &PaneClose, ports: Ports<'_>) -> Result<(PaneName, Option<Profile>), PaneError> {
    let Some(pane) = args.pane.as_deref() else {
        return Err(PaneError::Usage {
            message: "pane close needs a PANE".to_owned(),
        });
    };
    let name = PaneName::parse(pane)?;
    let profile = stored_profile(ports, profile_name(&args.profile)?.as_ref())?;
    if let Some(profile) = &profile {
        if spec_of(profile, &name).is_none() {
            return Err(PaneError::PaneNotInProfile {
                what: format!(
                    "profile {} has no spec for {name}",
                    quoted(profile.name.as_str())
                ),
            });
        }
    }
    Ok((name, profile))
}

/// `--spec-only`: remove P's spec for the pane, with no act.
fn remove_spec(ports: Ports<'_>, name: PaneName, profile: Profile) -> Result<Closed, ErrorBody> {
    let mut nothing_live = || -> Result<(), PaneError> { Ok(()) };
    let written = ports
        .scope
        .edit_spec(
            Some(&profile.name),
            &name,
            &SpecEdit::Remove,
            &mut nothing_live,
        )
        .map_err(|error| ErrorBody::from(&error))?;
    Ok(Closed {
        pane: name,
        generation: written.map(|profile| profile.generation),
        profile: Some(profile.name),
        spec_only: true,
    })
}

/// The live steps of a close, in order. A failure names the one it reached.
#[derive(Debug, Clone, Copy)]
enum Step {
    StopOwned,
    Close,
    Delete,
}

impl Step {
    /// What failed, and what had already been done, when this step failed.
    fn failed(self) -> &'static str {
        match self {
            Step::StopOwned => "stopping its processes failed",
            Step::Close => "its processes are stopped, but closing its Herdr pane failed",
            Step::Delete => "it is closed, but its record was not deleted",
        }
    }
}

/// Read the record, then stop, close and delete, inside `edit_spec` with P when given.
fn close_live(
    ports: Ports<'_>,
    name: PaneName,
    profile: Option<Profile>,
) -> Result<Closed, ErrorBody> {
    let record = ports
        .pane_store
        .get(&name)
        .and_then(|record| {
            record.ok_or_else(|| PaneError::PaneNotFound {
                what: name.to_string(),
            })
        })
        .map_err(|error| ErrorBody::from(&error))?;
    let profile = profile.map(|profile| profile.name);
    let mut reached = None;
    let mut act = || -> Result<(), PaneError> {
        reached = Some(Step::StopOwned);
        ports.host.stop_owned(&name)?;
        reached = Some(Step::Close);
        ports.herdr.close(&record.herdr.pane_id)?;
        reached = Some(Step::Delete);
        ports.pane_store.delete(&name, record.generation)
    };
    let edited = ports
        .scope
        .edit_spec(profile.as_ref(), &name, &SpecEdit::Remove, &mut act);
    match edited {
        Ok(written) => Ok(Closed {
            pane: name,
            generation: written.map(|profile| profile.generation),
            profile,
            spec_only: false,
        }),
        Err(error) => Err(failure_body(&error, &name, reached, profile.as_ref())),
    }
}

/// A failed close's error body. Once the act began (`reached`), the message names the pane and
/// the step, and ends with the run's reconcile step unless it already holds it (the profile
/// scope's restore failures carry it); before, the error is as it is.
fn failure_body(
    error: &PaneError,
    name: &PaneName,
    reached: Option<Step>,
    profile: Option<&ProfileName>,
) -> ErrorBody {
    let mut body = ErrorBody::from(error);
    let Some(step) = reached else {
        return body;
    };
    body.message = format!("{name}: {}: {}", step.failed(), body.message);
    let reconcile = reconcile_step(profile);
    if !body.message.contains(reconcile.as_str()) {
        body.message = format!("{}; {reconcile}", body.message);
    }
    body
}
