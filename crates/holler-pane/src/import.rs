//! The migration import (#650): the plan of `holler pane import`, which adopts the fleet the
//! old launcher runs. It reads what the launcher's files say about each pane (`holler-cli`
//! reads the files, [`files`]), joins each pane to what Herdr, tmux and the harness show, and
//! plans one record per pane and the profile `fleet` (ADR-0021 section 13). It writes
//! nothing: a real run's writes and their undo are the CLI's (they go through `profile
//! create`'s `insert_profile`), so a dry run and a real run plan exactly the same.
//!
//! [`plan`] reads only, through the [`Ports`]:
//!
//! - `herdr.version()` and `herdr.snapshot()`, once each. Either one's error ends the run
//!   with that error, `herdr-version-unsupported` included: nothing can be joined without
//!   them, so it is never a finding.
//! - `pane_store.get(name)` per pane: a pane that already has a record is `already-imported`,
//!   skipped before any other check. A pane whose `sessions.toml` names a harness that is not
//!   OpenCode is `harness-not-opencode`, skipped too.
//! - For every other pane, at once (one scoped thread per pane): `host.ps`, then
//!   `host.foreground_commands`, then `harness.health` and `harness.list_sessions`; for a pane
//!   whose tmux session runs Claude Code and that the run adopts (#831), `host.foreground_pids`
//!   and `harness.shown_session` per pid instead of the last two (`observe`'s docs count them).
//! - `profile_store.get("fleet")`, once, only with `--adopt-claude-code` and a record to write
//!   (#831): an existing `fleet` is then extended ([`FleetPlan::Extend`]), and its error ends
//!   the run as a pane store's does.
//!
//! Nothing else is called: it never starts, stops, types into or moves anything. The join of
//! each pane is `assess`'s (the label finds the Herdr pane; the files and a healthy server's
//! list give the session of record). The report lists the panes in name order, whatever order
//! the threads finish in.
//!
//! **A pane that runs Claude Code is left alone (#835), unless the run adopts it (#831).** The
//! old launcher's files cannot tell a Claude Code pane from an OpenCode one, so the import reads
//! the command name of the foreground process of each tmux pane in the pane's tmux session: one
//! named exactly `claude` ([`crate::pane::is_claude_code_command`]) leaves the pane alone,
//! `harness-claude-code` (`info`, the row `skipped`), with no record, no other finding and no
//! harness call. With `--adopt-claude-code` such a pane is adopted instead, with the Claude Code
//! harness (`adopt`'s docs), and the switch also lets the run extend an existing `fleet`
//! (decision D1). A pane whose `sessions.toml` names `claude` is still `harness-not-opencode`,
//! switch or not: only a pane the classification finds running Claude Code is adopted. Any other
//! name (`bash`, which the launcher's OpenCode panes show), or no process, is imported as
//! OpenCode, as before, so a pane whose OpenCode server is merely down is still recorded and
//! flagged `server-down`. A pane whose session could not be read, or whose chain did not
//! finish, is never recorded on a guess: `harness-unreadable`, a problem.
//!
//! Only a [`Level::Problem`](findings::Level) blocks: a pane with one gets no record, and a
//! real run that meets one is refused as [`IMPORT_BLOCKED`] ([`Plan::refusal`]).

mod adopt;
mod assess;
pub mod files;
pub mod findings;
mod observe;

use serde::{Serialize, Serializer};

use crate::error::RefusalCode;
use crate::pane::PaneRole;
use crate::profile_snapshot::{profile_from_panes, spec_from_pane};
use crate::{GridPos, Pane, PaneError, PaneName, Ports, Profile, ProfileName};

use assess::{Context, Outcome};
use files::PaneFiles;
use findings::{Finding, ImportFinding, Level};

/// `import-blocked`: a real run refused because a pane has a problem; nothing was written.
/// The verb's own open code (#650): a refusal, exit 3.
pub const IMPORT_BLOCKED: RefusalCode = RefusalCode::from_static("import-blocked");

/// The name of the profile a real run creates, with one spec per imported pane.
pub const FLEET: &str = "fleet";

/// What one import is asked to plan.
#[derive(Debug, Clone, Copy)]
pub struct ImportRequest<'a> {
    /// The Herdr session the panes live in (`--herdr-session`): a pane's Herdr pane is looked
    /// for there only.
    pub herdr_session: &'a str,
    /// The panes `--orchestrator` names; every other pane is an agent.
    pub orchestrators: &'a [PaneName],
    /// What each pane directory's files say.
    pub panes: &'a [PaneFiles],
    /// The findings about no pane the CLI met listing the state root: a directory that holds
    /// no pane file (`ignored-entry`), or holds one under a name that is not a pane name
    /// (`source-unreadable`).
    pub others: &'a [Finding],
    /// `--adopt-claude-code` (#831): adopt the panes whose tmux session runs Claude Code, and
    /// extend `fleet` when it exists.
    pub adopt: bool,
}

/// What the import does with a pane directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A real run writes its record.
    Record,
    /// It already has a record and is left as it is.
    AlreadyImported,
    /// It is not an OpenCode pane, or its tmux session runs Claude Code (#835), and is left
    /// out.
    Skipped,
    /// It has a problem and gets no record.
    Blocked,
}

impl Action {
    /// The action's name, which is also its JSON form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Action::Record => "record",
            Action::AlreadyImported => "already-imported",
            Action::Skipped => "skipped",
            Action::Blocked => "blocked",
        }
    }
}

impl Serialize for Action {
    /// The action's [`as_str`](Action::as_str).
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// One pane directory in the report. Its JSON keys are its fields, in this order; an absent
/// value is `null`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneRow {
    pub name: PaneName,
    pub action: Action,
    /// The role the record takes (from `--orchestrator`); `None` for a pane left alone.
    pub role: Option<PaneRole>,
    /// Where Herdr shows the pane; `None` when no single Herdr pane carries its label, or the
    /// pane was left alone.
    pub pos: Option<GridPos>,
    /// The record's session of record; `None` when it has none or gets no record.
    pub session_of_record: Option<String>,
}

/// What a real run writes, and the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// One row per pane directory, in name order.
    pub panes: Vec<PaneRow>,
    /// Every finding: those about no pane first, then by pane, by kind (in
    /// [`findings::ImportFinding::ALL`] order) and by message.
    pub findings: Vec<Finding>,
    /// The records a real run writes, in name order: one per row whose action is `record`,
    /// each at generation 0 and in the profile `fleet`.
    pub records: Vec<Pane>,
    /// What a real run does with the profile `fleet`.
    pub fleet: FleetPlan,
}

/// What a real run does with the profile `fleet` (#831).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetPlan {
    /// There is no record to write: `fleet` is neither read nor written.
    None,
    /// Create `fleet` with one spec per record, as `profile_from_panes` builds it (generation
    /// 0, and `created` and `updated` 0: the store stamps them). Without the switch this is the
    /// plan whatever exists, and the write refuses an existing `fleet` (`profile-exists`).
    Create(Profile),
    /// With the switch, `fleet` exists: extend it by one compare-and-swap at the generation
    /// read.
    Extend {
        /// The profile as read, with a spec appended for each recorded pane it has none for. A
        /// spec it has already is kept as it is (`spec-kind-differs` when its harness kind is not
        /// the record's).
        profile: Profile,
        /// The generation `fleet` was read at, which the write expects.
        generation_read: u64,
        /// The panes whose specs were appended, in name order. Empty when `fleet` already has
        /// every spec: the records are written and `fleet` is not.
        added: Vec<PaneName>,
    },
}

impl FleetPlan {
    /// The profile a dry run shows: the one a real run would create, or `fleet` as it would be
    /// extended (at the generation read).
    pub fn profile(&self) -> Option<&Profile> {
        match self {
            FleetPlan::None => None,
            FleetPlan::Create(profile) | FleetPlan::Extend { profile, .. } => Some(profile),
        }
    }
}

impl Plan {
    /// How many panes have a problem: each pane directory with one, counted once, and each
    /// problem about no pane (a directory of pane files whose name is not a pane name).
    pub fn blocked(&self) -> usize {
        let panes = self
            .panes
            .iter()
            .filter(|row| row.action == Action::Blocked)
            .count();
        let others = self
            .findings
            .iter()
            .filter(|f| f.pane.is_none() && f.level() == Level::Problem)
            .count();
        panes + others
    }

    /// `import-blocked` when a pane has a problem: a real run then writes nothing.
    pub fn refusal(&self) -> Option<PaneError> {
        let blocked = self.blocked();
        (blocked > 0).then(|| PaneError::Refused {
            code: IMPORT_BLOCKED,
            message: format!(
                "import blocked: {blocked} pane(s) have problems; run with --dry-run for the report"
            ),
        })
    }
}

/// Plan the import of `request.panes` (see the module docs). It fails only when it cannot
/// plan: a `herdr.version()` or `herdr.snapshot()` error, a pane store that cannot be read,
/// or, with `--adopt-claude-code` and a record to write, a profile store that cannot be read.
/// Everything else is a finding of the plan.
pub fn plan(ports: Ports<'_>, request: &ImportRequest<'_>) -> Result<Plan, PaneError> {
    let fleet = ProfileName::parse(FLEET)?;
    let version = ports.herdr.version()?;
    let snapshot = ports.herdr.snapshot()?;
    let mut panes: Vec<&PaneFiles> = request.panes.iter().collect();
    panes.sort_by(|a, b| a.name.cmp(&b.name));
    let mut outcomes = Vec::with_capacity(panes.len());
    let mut candidates = Vec::with_capacity(panes.len());
    for files in panes {
        if ports.pane_store.get(&files.name)?.is_some() {
            outcomes.push(Outcome::already_imported(&files.name));
        } else if files.sessions.not_opencode() {
            outcomes.push(Outcome::not_opencode(&files.name));
        } else {
            candidates.push(files);
        }
    }
    let context = Context {
        session: request.herdr_session,
        snapshot: &snapshot.panes,
        version: &version,
        orchestrators: request.orchestrators,
        fleet: &fleet,
        adopt: request.adopt,
    };
    outcomes.extend(assess::assess_all(ports, &context, &candidates));
    let to_record = outcomes.iter().any(|outcome| outcome.record.is_some());
    let existing = if request.adopt && to_record {
        ports.profile_store.get(&fleet)?
    } else {
        None
    };
    Ok(assemble(&fleet, request.others, outcomes, existing))
}

/// The plan of `outcomes` and the findings about no pane, each list in its order; `existing`
/// is `fleet` as read, when the run extends it.
fn assemble(
    fleet: &ProfileName,
    others: &[Finding],
    mut outcomes: Vec<Outcome>,
    existing: Option<Profile>,
) -> Plan {
    outcomes.sort_by(|a, b| a.row.name.cmp(&b.row.name));
    let mut findings = others.to_vec();
    let mut panes = Vec::with_capacity(outcomes.len());
    let mut records = Vec::new();
    for outcome in outcomes {
        findings.extend(outcome.findings);
        panes.push(outcome.row);
        records.extend(outcome.record);
    }
    let fleet = match existing {
        _ if records.is_empty() => FleetPlan::None,
        None => FleetPlan::Create(profile_from_panes(fleet, &records)),
        Some(profile) => extend(profile, &records, &mut findings),
    };
    findings.sort_by(|a, b| (&a.pane, a.kind, &a.message).cmp(&(&b.pane, b.kind, &b.message)));
    Plan {
        panes,
        findings,
        records,
        fleet,
    }
}

/// The extension of the existing `profile` by `records` (#831): a spec appended for each record
/// it has none for (`spec_from_pane`, as `profile_from_panes` builds one), an existing spec kept
/// as it is, and `spec-kind-differs` for one whose harness kind is not the record's.
fn extend(mut profile: Profile, records: &[Pane], findings: &mut Vec<Finding>) -> FleetPlan {
    let generation_read = profile.generation;
    let mut added = Vec::new();
    for record in records {
        let kept = profile
            .panes
            .iter()
            .find(|spec| spec.pane == record.name.as_str())
            .map(|spec| spec.harness.kind);
        match kept {
            None => {
                profile.panes.push(spec_from_pane(record));
                added.push(record.name.clone());
            }
            Some(kind) if kind != record.harness.kind => {
                let message = format!(
                    "the profile {} has a spec for the pane with the harness kind {}, and the \
                     record's is {}; the spec is kept as it is, so profile apply reports the \
                     pane as differing from it",
                    profile.name,
                    kind.as_str(),
                    record.harness.kind.as_str()
                );
                findings.push(Finding::about(
                    &record.name,
                    ImportFinding::SpecKindDiffers,
                    message,
                ));
            }
            Some(_) => {}
        }
    }
    FleetPlan::Extend {
        profile,
        generation_read,
        added,
    }
}
