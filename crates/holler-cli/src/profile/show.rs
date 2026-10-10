//! `holler profile show` (story #662, epic #633): a profile's specs, its live panes, and
//! for each live pane every field where it differs from its spec.
//!
//! A profile's live panes are its members, the pane records whose `profile` has its slug
//! (`holler_pane::profile_diff::is_member`, ADR-0021 section 3). The comparison is
//! `holler_pane::profile_diff::diff_profile`, the one `profile apply` (#664) and the
//! `profile-drift` finding (#665) reuse, so there is no `diff` verb. The probe result is
//! each member's `probe.last`, read and never run: a read verb that ran stored argv would
//! run a command as a side effect of looking.
//!
//! The verb reads the two registries (`ProfileStore::get`, then one `PaneStore::list`)
//! and nothing else: no adapter, no probe, no write.

use clap::Args;
use holler_pane::profile_diff::{
    diff_profile, is_member, FieldDiff, FieldValue, PaneDiff, PaneStatus, SpecField,
};
use holler_pane::{Pane, PaneError, Ports, ProbeResult, Profile, ProfileName, ProfileSpec};
use serde::Serialize;

use super::list::count;
use crate::output::{emit, ErrorBody, VerbCtx};

/// Show a profile and where live panes differ from it.
///
/// The text form starts with `profile NAME (SLUG): generation G, N specs, L live`, where
/// the live panes are the profile's members (the panes whose profile is this one). Then
/// one block per spec, `spec PANE` and one `FIELD: VALUE` line per field. Then one line
/// per pane: `matches` or `differs` (a spec and a live pane of its name), `missing (no
/// live pane)` (a spec alone) or `extra (no spec)` (a live pane alone). Under a pane that
/// differs, one `FIELD: spec VALUE, live VALUE` line per field that differs; under every
/// live pane, its last probe result (`ok`, `failed (missing "...")`, `error ("...")` or
/// `none`), which this verb reads and never runs. A position is `r<row>c<col>`, a command
/// or a check a JSON array, and a control character in a stored string is escaped.
///
/// `--format=json` prints one envelope whose data is `{"profile": PROFILE, "comparison":
/// [ROW, ...]}`: the profile as stored, and per pane `{"pane", "status", "differences",
/// "probe"}`, where each difference is `{"field", "spec", "live"}` and `probe` is the
/// last probe result or null.
#[derive(Args, Debug)]
pub struct ProfileShow {
    /// The profile's name (spaces allowed).
    pub name: String,
}

/// Run `holler profile show`: read the profile and its members, compare them, and print
/// the comparison.
pub fn run(args: &ProfileShow, ctx: &mut VerbCtx<'_>) -> i32 {
    let result = view(args, ctx.ports).map_err(|e| ErrorBody::from(&e));
    emit(&mut ctx.sink, ctx.format, result, render)
}

/// The data of `profile show`.
#[derive(Debug, Serialize)]
struct ProfileView {
    /// The profile as stored.
    profile: Profile,
    comparison: Vec<ComparisonRow>,
    /// The number of member panes, for the text header.
    #[serde(skip)]
    live: usize,
}

/// One row of the comparison, with the last probe result of its live pane.
#[derive(Debug, Serialize)]
struct ComparisonRow {
    #[serde(flatten)]
    diff: PaneDiff,
    /// `Pane.probe.last` of the live pane; `None` for a spec with no live pane, or a pane
    /// whose probe never ran.
    probe: Option<ProbeResult>,
}

/// The profile named in `args` and its comparison with its members. A bad name is
/// `usage`, a profile that does not exist is `profile-not-found`, and a store error passes
/// through with its own code.
fn view(args: &ProfileShow, ports: Ports<'_>) -> Result<ProfileView, PaneError> {
    let name = ProfileName::parse(&args.name)?;
    let profile = ports
        .profile_store
        .get(&name)?
        .ok_or_else(|| PaneError::ProfileNotFound {
            what: format!("{:?}", name.as_str()),
        })?;
    let members: Vec<Pane> = ports
        .pane_store
        .list()?
        .into_iter()
        .filter(|pane| is_member(pane, &profile.name))
        .collect();
    let comparison = diff_profile(&profile, &members)
        .into_iter()
        .map(|diff| ComparisonRow {
            probe: last_probe(&diff, &members),
            diff,
        })
        .collect();
    Ok(ProfileView {
        live: members.len(),
        profile,
        comparison,
    })
}

/// The last probe result of the member a row names. A `Missing` row names no member, so
/// it has none.
fn last_probe(diff: &PaneDiff, members: &[Pane]) -> Option<ProbeResult> {
    members
        .iter()
        .find(|pane| pane.name.as_str() == diff.pane)
        .and_then(|pane| pane.probe.last.clone())
}

/// The text form: the header, the spec blocks, then the pane rows.
fn render(view: &ProfileView) -> String {
    let profile = &view.profile;
    let mut lines = vec![format!(
        "profile {} ({}): generation {}, {}, {} live",
        profile.name,
        profile.slug,
        profile.generation,
        count(profile.panes.len(), "spec"),
        view.live
    )];
    for spec in &profile.panes {
        push_spec(&mut lines, spec);
    }
    for row in &view.comparison {
        push_row(&mut lines, row);
    }
    lines.join("\n")
}

/// `spec PANE`, then one `FIELD: VALUE` line per field.
fn push_spec(lines: &mut Vec<String>, spec: &ProfileSpec) {
    lines.push(format!("spec {}", text(&spec.pane)));
    lines.extend(
        SpecField::ALL
            .into_iter()
            .map(|field| format!("  {}: {}", field.as_str(), field.value(spec))),
    );
}

/// `pane PANE: STATUS`, then one line per difference, then the probe line (every row but
/// a `Missing` one, which has no live pane).
fn push_row(lines: &mut Vec<String>, row: &ComparisonRow) {
    let diff = &row.diff;
    lines.push(format!(
        "pane {}: {}",
        text(&diff.pane),
        status_text(diff.status)
    ));
    lines.extend(diff.differences.iter().map(difference_line));
    if diff.status != PaneStatus::Missing {
        lines.push(format!("  probe: {}", probe_text(row.probe.as_ref())));
    }
}

/// `  FIELD: spec VALUE, live VALUE`.
fn difference_line(difference: &FieldDiff) -> String {
    format!(
        "  {}: spec {}, live {}",
        difference.field.as_str(),
        difference.spec,
        difference.live
    )
}

/// A row's status for a person.
fn status_text(status: PaneStatus) -> &'static str {
    match status {
        PaneStatus::Matches => "matches",
        PaneStatus::Differs => "differs",
        PaneStatus::Missing => "missing (no live pane)",
        PaneStatus::Extra => "extra (no spec)",
    }
}

/// A probe result for a person. Each stored string is in Rust's `{:?}` quoting, which
/// escapes control characters: `failed (missing "a", "b")`, `error ("reason")`.
fn probe_text(probe: Option<&ProbeResult>) -> String {
    match probe {
        None => "none".to_owned(),
        Some(ProbeResult::Ok) => "ok".to_owned(),
        Some(ProbeResult::Failed { missing }) if missing.is_empty() => "failed".to_owned(),
        Some(ProbeResult::Failed { missing }) => {
            let quoted: Vec<String> = missing.iter().map(|s| format!("{s:?}")).collect();
            format!("failed (missing {})", quoted.join(", "))
        }
        Some(ProbeResult::Error(reason)) => format!("error ({reason:?})"),
    }
}

/// A stored string for a text line, with its control characters escaped. A spec's pane
/// is unchecked text, so it goes through the same escape as every stored field value.
fn text(stored: &str) -> String {
    FieldValue::Text(stored.to_owned()).to_string()
}
