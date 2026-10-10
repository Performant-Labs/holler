//! `holler pane relaunch PANE [SPEC FLAGS] [--profile NAME] [--spec-only]` (story #644): launch
//! a pane again after stopping only what it owns, keeping its directory, its cell unless `--grid`
//! moves it, and its session of record when the restarted server still lists it (ADR-0021
//! section 8, "Launch and relaunch as built").
//!
//! In order: `PANE`, the spec flags and `--profile` are typed as `launch.rs` types them; the
//! pane's record is read (`pane-not-found`); with `--profile P`, P is read
//! (`profile-not-found`). The base the flags overlay is P's spec for the pane when it has one,
//! else the record's own (`profile_snapshot::spec_from_pane`, the one mapping, so `profile show`
//! sees no false drift). The relaunch rules are the engine's (`tx_launch::relaunch`, E0), not
//! checked here, so every caller is held to them. The overlay and the output are `launch.rs`'s.

use clap::Args;
use holler_pane::profile_snapshot::spec_from_pane;
use holler_pane::tx_launch::{relaunch, RelaunchRequest, TxOptions};
use holler_pane::{PaneError, PaneName, Ports};

use super::args::{ProfileOpt, SpecFlags, SpecOnly};
use super::launch::{effective_spec, emit_outcome, spec_of, stored_profile, Verb};
use super::list::profile_name;
use crate::output::{emit_error, ErrorBody, VerbCtx};

/// Launch a pane again, replacing its processes.
///
/// It stops only the processes the pane owns and checks that its old harness server no longer
/// answers (one that survives fails the run loudly), then brings the pane up as launch does:
/// the Herdr pane at the same cell (recreated there if it vanished), the tmux session, the
/// command, the server, the session of record (the same one when the restarted server still
/// has it, a new one otherwise) and the TUI attached to it, observed, then recorded. The pane
/// keeps its directory (a new --project is refused: close it and launch it again) and its cell
/// (a new --workspace or cell is refused without --grid; with --grid the pane moves, and its old
/// Herdr pane is closed once the record names the new one). Flags not given keep the record's
/// values, or with `--profile P` the values of P's spec for the pane.
///
/// `--format=json` prints one envelope whose data is `{"verb", "pane", "profile", "spec_only"}`.
#[derive(Args, Debug)]
pub struct PaneRelaunch {
    /// The pane's name, e.g. demo-c1r1.
    #[arg(value_name = "PANE")]
    pub pane: String,
    /// Boxed so `PaneCmd` (inside the frozen `Command`) stays under clippy's
    /// `large_enum_variant` bound now that the verb has its own positional.
    #[command(flatten)]
    pub spec: Box<SpecFlags>,
    #[command(flatten)]
    pub profile: ProfileOpt,
    #[command(flatten)]
    pub spec_only: SpecOnly,
}

/// Run `holler pane relaunch`: type the arguments, run the engine and print what it answered.
pub fn run(args: &PaneRelaunch, ctx: &mut VerbCtx<'_>) -> i32 {
    match relaunch_request(args, ctx.ports) {
        Ok(request) => {
            let result = relaunch(ctx.ports, &request, &TxOptions::default());
            let name = &request.record.name;
            emit_outcome(ctx, Verb::Relaunch, name, request.profile.as_ref(), result)
        }
        Err(error) => emit_error(&mut ctx.sink, ctx.format, ErrorBody::from(&error)),
    }
}

/// The request the engine runs (see the module docs for the order of the reads).
fn relaunch_request(args: &PaneRelaunch, ports: Ports<'_>) -> Result<RelaunchRequest, PaneError> {
    let name = PaneName::parse(&args.pane)?;
    let values = args.spec.validate()?;
    let profile = profile_name(&args.profile)?;
    let record = ports
        .pane_store
        .get(&name)?
        .ok_or_else(|| PaneError::PaneNotFound {
            what: name.to_string(),
        })?;
    let profile = stored_profile(ports, profile.as_ref())?;
    let from_record = spec_from_pane(&record);
    let base = profile
        .as_ref()
        .and_then(|profile| spec_of(profile, &name))
        .unwrap_or(&from_record);
    let spec = effective_spec(Some(base), &name, &values)?;
    Ok(RelaunchRequest {
        record,
        spec,
        grid_given: values.grid.is_some(),
        profile: profile.map(|profile| profile.name),
        spec_only: args.spec_only.spec_only,
    })
}
