//! The `holler profile apply` transaction (story #664): converge the live panes on a profile P,
//! declaratively and idempotently. It creates a missing pane through the launch transaction,
//! adopts a healthy pane that matches its spec, relaunches an agent that is not healthy, and
//! never kills, rewrites or moves a healthy pane: what it will not change, it reports. It works
//! only through the [`Ports`] (ADR-0021 section 5), calls [`tx_launch`] unchanged, and holds no
//! I/O of its own.
//!
//! - **What it looks at** (I6): the registry and live observation, never a file. A spec pane's
//!   record is the registry entry of its name, in any profile. A Herdr pane, tmux session or
//!   harness server that no record names is never adopted (`holler pane import` records those).
//! - **The order of a run.** Read P (`profile-not-found`) and the registry. Without
//!   `--herdr-session`, a spec pane with no record (a `create`) is `usage` before any Herdr or
//!   harness call, in a dry run too. Observe every recorded spec pane at once. Choose every
//!   pane's step: that is the plan, where a dry run stops. Otherwise run the steps one at a time.
//! - **Healthy** is observed, never the stored `harness.health`: `health` answers `Ok(true)`,
//!   Herdr's snapshot lists the record's Herdr session and pane id (that pair alone, never the
//!   cell or the label), and the TUI does not show a session other than the session of record.
//!   A shown session that cannot be read (`Ok(None)` or an error) cannot tell, so it does not
//!   make a pane unhealthy, and the step's detail says so. A `health` or `snapshot` call that
//!   fails makes the step of a recorded pane `failed`, unless row a refuses it.
//! - **The steps**, one per spec pane in `(workspace, row, col)` order; the first row that
//!   matches applies. (a) A record of another profile, without `--take-over`: `refused`
//!   (`pane-in-other-profile`). A record whose spec names another harness than the pane runs:
//!   `refused` (`harness-change-refused`, #831: a live pane's harness is never rewritten, by
//!   apply or by a relaunch). (b) No record: `create`, by `tx_launch::launch`. (c) Healthy and
//!   matching its spec: `adopt`, which records the pane as a member of P and touches nothing
//!   live. (d) Healthy and differing: `reported`, with its row of `diff_profile`. (e) Not
//!   healthy, the orchestrator: `refused` ([`ORCHESTRATOR_NOT_RELAUNCHED`]). (f) Not healthy, an
//!   agent: `relaunch` in place, by `tx_launch::relaunch`. Under `--take-over`, a record of
//!   another profile leaves it and then joins P, two writes, before anything else rows c, d
//!   and f do; row e never writes.
//! - **Outcomes.** A refusal from launch or relaunch is a `refused` step with its code, and a
//!   `usage` answer a `refused` step coded [`SPEC_NOT_LAUNCHABLE`]; the run goes on. A
//!   failure-class answer is a `failed` step, which stops the run: the steps after it are
//!   `skipped`. A `refused` or `failed` step whose launch or relaunch had started its act (a
//!   refusal can come from inside the act: the port's re-check, `verify_serving`) is marked
//!   [`Step::acted`], for the reconcile step the verb prints. In a dry run a step that would act
//!   is `planned`, and the run calls only its reads: no probe, no split, no write.
//! - **Each step re-reads** its record and P before it acts, so a stale spec is never written
//!   back: the spec handed to launch or relaunch is P's at that moment, and a membership write
//!   is a compare-and-swap at the generation re-read (a conflict is a `failed` step).
//! - **Bound (I5).** A run takes, in order: its two reads (P and the registry), each within I5's
//!   per-call bound; the up-front observation, whose calls (Herdr's snapshot, and each recorded
//!   spec pane's `health` and `shown_session`) all run at once, one thread each, so it takes at
//!   most one port call's own bound, I5's per-call bound (default 10 s); then the steps in
//!   turn, each under its own I5 bound. A `create` or `relaunch` step is one `tx_launch` run
//!   under the `TxOptions` budget its caller passes (the verb passes
//!   [`DEFAULT_BUDGET`](crate::tx_launch::DEFAULT_BUDGET)), which starts when that run starts,
//!   so it takes at most the budget plus one port call's own bound plus its rollback calls. An
//!   `adopt`, or a take-over's move, makes at most two record writes. Each of those steps first
//!   re-reads its record and P, two calls; a step refused or failed at planning calls nothing.
//!   There is no overall cap: a run of N steps takes at most 10 s for the observation plus N
//!   step bounds, beyond its reads.
//!
//! Untrusted text (a spec's pane, profile names, session and Herdr ids) reaches a message only
//! through `findings::quoted`. A [`PaneName`] is checked text (1 to 32 lowercase letters, digits
//! and inner `-`, `SessionName`'s grammar), so it is printed as it is.

mod observe;
mod run;

use serde::{Serialize, Serializer};

use crate::error::{class_of, ErrorClass, RefusalCode};
use crate::findings::quoted;
use crate::pane::PaneRole;
use crate::profile_diff::{diff_profile, is_member, PaneDiff, PaneStatus};
use crate::tx_launch::{self, check_harness_unchanged, TxOptions};
use crate::{Pane, PaneError, PaneName, Ports, Profile, ProfileName, ProfileSpec};

use observe::{Judged, Observed};

/// `orchestrator-not-relaunched`: the orchestrator's pane is not healthy, and `apply` never
/// relaunches the orchestrator (row e); the operator relaunches it. A refusal, exit 3.
pub const ORCHESTRATOR_NOT_RELAUNCHED: RefusalCode =
    RefusalCode::from_static("orchestrator-not-relaunched");
/// `spec-not-launchable`: the launch transaction answered `usage` for a step's spec (the relaunch
/// rules refuse a new directory or cell, a port policy names no port), or the spec's pane is not
/// a pane name. A per-pane refusal, not a run failure (exit 3).
pub const SPEC_NOT_LAUNCHABLE: RefusalCode = RefusalCode::from_static("spec-not-launchable");

/// What `apply` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRequest {
    /// The profile to converge on, as the caller named it (found by slug).
    pub profile: ProfileName,
    /// The Herdr session a spec pane with no record is created in: required when there is such
    /// a pane (`usage`), unused otherwise (a relaunch keeps its record's session).
    pub herdr_session: Option<String>,
    /// Take over a pane whose record belongs to another profile (rows c, d and f move it to P).
    pub take_over: bool,
    /// Choose every step and change nothing.
    pub dry_run: bool,
}

/// What a run did, or for a dry run would do. Its JSON is `profile apply`'s `data`:
/// `{"profile", "dry_run", "steps"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Applied {
    /// P's stored name.
    pub profile: ProfileName,
    pub dry_run: bool,
    /// One per spec pane, in `(workspace, row, col)` order.
    pub steps: Vec<Step>,
}

/// One spec pane's step. Its JSON is `{"pane", "action", "outcome", "code", "diff", "detail"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Step {
    /// The spec's pane, as P holds it (`ProfileSpec.pane`, unchecked text).
    pub pane: String,
    pub action: Action,
    pub outcome: Outcome,
    /// The code of a `refused` or `failed` step: its error's.
    pub code: Option<String>,
    /// A `reported` step's comparison: its spec's row of `diff_profile`, unchanged.
    pub diff: Option<PaneDiff>,
    /// One line for a person: a `refused` or `failed` step's error, why a pane is relaunched, or
    /// what its observation could not tell.
    pub detail: Option<String>,
    /// The error of a `refused` or `failed` step, and of no other.
    #[serde(skip)]
    pub error: Option<PaneError>,
    /// A `refused` or `failed` step whose launch or relaunch had started its act
    /// (`TxFailure.acted`), so the live panes and the records may be apart: the verb ends the
    /// step's detail with the reconcile step. False for every step that acted nothing.
    #[serde(skip)]
    pub acted: bool,
}

/// What a step does: its row of the step table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Row b: launch the pane.
    Create,
    /// Row c: record the pane as a member of P.
    Adopt,
    /// Row d: report how the pane differs from its spec.
    Report,
    /// Row f: relaunch the agent in place.
    Relaunch,
    /// Rows a and e, and a spec whose pane is not a pane name.
    Refuse,
    /// A recorded pane whose `health` or snapshot call failed, so no row could be chosen.
    Observe,
}

impl Action {
    /// Its JSON form, which the verb's text and messages also print.
    pub const fn as_str(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Adopt => "adopt",
            Action::Report => "report",
            Action::Relaunch => "relaunch",
            Action::Refuse => "refuse",
            Action::Observe => "observe",
        }
    }
}

impl Serialize for Action {
    /// [`Action::as_str`].
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// How a step ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Created,
    Relaunched,
    Adopted,
    Reported,
    /// A refusal, or `usage` for the step's own spec: the run goes on.
    Refused,
    /// A failure-class answer: the run stops at this step.
    Failed,
    /// Not attempted, because an earlier step failed.
    Skipped,
    /// A dry run's step that would act (a create, an adopt or a relaunch); nothing was done.
    Planned,
}

impl Outcome {
    /// Its JSON form, which the verb's text also prints.
    pub const fn as_str(self) -> &'static str {
        match self {
            Outcome::Created => "created",
            Outcome::Relaunched => "relaunched",
            Outcome::Adopted => "adopted",
            Outcome::Reported => "reported",
            Outcome::Refused => "refused",
            Outcome::Failed => "failed",
            Outcome::Skipped => "skipped",
            Outcome::Planned => "planned",
        }
    }
}

impl Serialize for Outcome {
    /// [`Outcome::as_str`].
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Converge the live panes on `request.profile` (see the module docs). `Err` is an error with no
/// plan: `usage` for a create without `--herdr-session`, `profile-not-found`, or a read of P or
/// the registry that fails. Every step's outcome, `failed` included, is in `Ok`.
pub fn apply(
    ports: Ports<'_>,
    request: &ApplyRequest,
    options: &TxOptions,
) -> Result<Applied, PaneError> {
    let profile = stored_profile(ports, &request.profile)?;
    let records = spec_records(&profile, ports.pane_store.list()?);
    require_herdr_session(request, &profile, &records)?;
    let observed = observe::observe(ports, &records);
    let planned = plan(&profile, &records, &observed, request.take_over);
    let steps = if request.dry_run {
        until_failed(&planned, Planned::dry)
    } else {
        let cx = run::Cx {
            ports,
            request,
            options,
        };
        until_failed(&planned, |planned| run::step(&cx, planned))
    };
    Ok(Applied {
        profile: profile.name,
        dry_run: request.dry_run,
        steps,
    })
}

/// `profile` as stored (found by slug): `profile-not-found` when there is none.
fn stored_profile(ports: Ports<'_>, profile: &ProfileName) -> Result<Profile, PaneError> {
    let stored = ports.profile_store.get(profile)?;
    stored.ok_or_else(|| PaneError::ProfileNotFound {
        what: quoted(profile.as_str()),
    })
}

/// The records a spec of `profile` names, in any profile, in the registry's order.
fn spec_records(profile: &Profile, all: Vec<Pane>) -> Vec<Pane> {
    all.into_iter()
        .filter(|record| {
            profile
                .panes
                .iter()
                .any(|spec| spec.pane == record.name.as_str())
        })
        .collect()
}

/// §4's gate, before any observation: a spec pane with no record is a `create`, which needs the
/// Herdr session, so without `--herdr-session` the run is `usage` and no step runs. A spec whose
/// pane is not a pane name is refused at planning, never created, so it needs no session.
fn require_herdr_session(
    request: &ApplyRequest,
    profile: &Profile,
    records: &[Pane],
) -> Result<(), PaneError> {
    if request.herdr_session.is_some() {
        return Ok(());
    }
    let creates: Vec<String> = profile
        .panes
        .iter()
        .filter(|spec| PaneName::parse(&spec.pane).is_ok())
        .filter(|spec| !records.iter().any(|r| r.name.as_str() == spec.pane))
        .map(|spec| quoted(&spec.pane))
        .collect();
    if creates.is_empty() {
        return Ok(());
    }
    Err(PaneError::Usage {
        message: format!(
            "--herdr-session is required: apply creates the spec panes that have no record ({}), \
             and the flag names the Herdr session they go in",
            creates.join(", ")
        ),
    })
}

/// One spec pane of the plan: its pane as P holds it, and its row.
struct Planned {
    pane: String,
    row: Row,
}

/// A spec pane's row of the step table, chosen from step 1's reads and the observation.
enum Row {
    /// Refused at planning: row a, row e, or a spec whose pane is not a pane name.
    Refuse(PaneError),
    /// A recorded pane whose observation failed (always a failure-class error): `failed`.
    Unobserved(PaneError),
    /// Row b.
    Create(PaneName),
    /// Row c, with what the observation could not tell.
    Adopt {
        name: PaneName,
        note: Option<String>,
    },
    /// Row d: its row of `diff_profile`, and what the observation could not tell.
    Report {
        name: PaneName,
        diff: PaneDiff,
        note: Option<String>,
    },
    /// Row f, with why the pane is not healthy.
    Relaunch { name: PaneName, why: String },
}

impl Row {
    fn action(&self) -> Action {
        match self {
            Row::Refuse(_) => Action::Refuse,
            Row::Unobserved(_) => Action::Observe,
            Row::Create(_) => Action::Create,
            Row::Adopt { .. } => Action::Adopt,
            Row::Report { .. } => Action::Report,
            Row::Relaunch { .. } => Action::Relaunch,
        }
    }
}

/// Every spec's row (§3), in `(workspace, row, col)` order: P's order is not the steps' (C6).
/// The comparison is `diff_profile`'s, whose rows come one per spec in P's order, so they pair
/// with the specs; a sort by cell is stable, so specs of one cell keep P's order.
fn plan(profile: &Profile, records: &[Pane], observed: &Observed, take_over: bool) -> Vec<Planned> {
    let mut chosen: Vec<(&ProfileSpec, Planned)> = profile
        .panes
        .iter()
        .zip(diff_profile(profile, records))
        .map(|(spec, diff)| {
            let row = choose(spec, diff, records, observed, &profile.name, take_over);
            let pane = spec.pane.clone();
            (spec, Planned { pane, row })
        })
        .collect();
    chosen.sort_by(|(a, _), (b, _)| cell_key(a).cmp(&cell_key(b)));
    chosen.into_iter().map(|(_, planned)| planned).collect()
}

/// A spec's place in the run: its workspace, then its row and column.
fn cell_key(spec: &ProfileSpec) -> (&str, u16, u16) {
    let grid = spec.herdr.grid;
    (spec.herdr.workspace.as_str(), grid.row, grid.col)
}

/// The row of `spec`, whose row of `diff_profile` is `diff`: the first of the step table that
/// matches (§3). Row a is checked before any observation is looked at (C1, R5 B-3).
fn choose(
    spec: &ProfileSpec,
    diff: PaneDiff,
    records: &[Pane],
    observed: &Observed,
    profile: &ProfileName,
    take_over: bool,
) -> Row {
    let name = match PaneName::parse(&spec.pane) {
        Ok(name) => name,
        Err(error) => return Row::Refuse(not_launchable(&error)),
    };
    let Some((record, seen)) = records
        .iter()
        .zip(&observed.panes)
        .find(|(record, _)| record.name == name)
    else {
        return Row::Create(name);
    };
    if in_other_profile(record, profile) && !take_over {
        return Row::Refuse(other_profile(record, profile));
    }
    if let Err(changed) = check_harness_unchanged(record, spec) {
        return Row::Refuse(changed);
    }
    match observe::judge(record, seen, &observed.snapshot) {
        Err(error) => Row::Unobserved(error),
        Ok(Judged::Healthy { note }) if diff.status == PaneStatus::Matches => {
            Row::Adopt { name, note }
        }
        Ok(Judged::Healthy { note }) => Row::Report { name, diff, note },
        Ok(Judged::Unhealthy(why)) if record.role == PaneRole::Orchestrator => {
            Row::Refuse(orchestrator(&name, &why))
        }
        Ok(Judged::Unhealthy(why)) => Row::Relaunch { name, why },
    }
}

/// Whether `record` belongs to a profile other than `profile` (slugs compared, as membership is).
fn in_other_profile(record: &Pane, profile: &ProfileName) -> bool {
    record.profile.is_some() && !is_member(record, profile)
}

/// Row a's refusal, and a step's when its pane moved to another profile during the run.
fn other_profile(record: &Pane, profile: &ProfileName) -> PaneError {
    let other = record
        .profile
        .as_ref()
        .map_or_else(String::new, |other| quoted(other.as_str()));
    PaneError::PaneInOtherProfile {
        what: format!(
            "{} is in profile {other}, not {}; --take-over moves it",
            record.name,
            quoted(profile.as_str())
        ),
    }
}

/// Row e's refusal.
fn orchestrator(name: &PaneName, why: &str) -> PaneError {
    PaneError::Refused {
        code: ORCHESTRATOR_NOT_RELAUNCHED,
        message: format!(
            "{name} is the orchestrator and is not healthy ({why}); apply never relaunches the \
             orchestrator: relaunch it with holler pane relaunch {name}"
        ),
    }
}

/// A `usage` answer for a step's own spec, as the `refused` step's error (§4).
fn not_launchable(error: &PaneError) -> PaneError {
    PaneError::Refused {
        code: SPEC_NOT_LAUNCHABLE,
        message: error.to_string(),
    }
}

/// The step of each of `planned`, made by `step` in order, until one is `failed`: the steps after
/// it are `skipped` (§5).
fn until_failed(planned: &[Planned], mut step: impl FnMut(&Planned) -> Step) -> Vec<Step> {
    let mut stopped = false;
    planned
        .iter()
        .map(|planned| {
            if stopped {
                return planned.skipped();
            }
            let made = step(planned);
            stopped = made.outcome == Outcome::Failed;
            made
        })
        .collect()
}

/// Why a step ended short: its error, a note on what it had changed by then, and whether its
/// launch or relaunch had started its act.
struct Halt {
    error: PaneError,
    note: Option<String>,
    acted: bool,
}

impl From<PaneError> for Halt {
    fn from(error: PaneError) -> Self {
        Self {
            error,
            note: None,
            acted: false,
        }
    }
}

impl From<tx_launch::TxFailure> for Halt {
    fn from(failure: tx_launch::TxFailure) -> Self {
        Self {
            error: failure.error,
            note: None,
            acted: failure.acted,
        }
    }
}

impl Planned {
    /// This pane's step, with no code, diff or detail.
    fn step(&self, action: Action, outcome: Outcome) -> Step {
        Step {
            pane: self.pane.clone(),
            action,
            outcome,
            code: None,
            diff: None,
            detail: None,
            error: None,
            acted: false,
        }
    }

    /// The step a dry run gives this pane; a live run gives the same to a row refused or failed
    /// at planning. A step that would act is `planned`; row d is `reported`.
    fn dry(&self) -> Step {
        let action = self.row.action();
        match &self.row {
            Row::Refuse(error) | Row::Unobserved(error) => {
                self.halted(action, Halt::from(error.clone()))
            }
            Row::Create(_) => self.step(action, Outcome::Planned),
            Row::Adopt { note, .. } => self.step(action, Outcome::Planned).noted(note.clone()),
            Row::Report { diff, note, .. } => Step {
                diff: Some(diff.clone()),
                ..self.step(action, Outcome::Reported)
            }
            .noted(note.clone()),
            Row::Relaunch { why, .. } => {
                self.step(action, Outcome::Planned).noted(Some(why.clone()))
            }
        }
    }

    /// This pane's step when it ended with `halt`, sorted through `class_of` (§4, §5): a failure
    /// is `failed`, a refusal `refused` with its own code, and `usage` (the step's own spec)
    /// `refused` with [`SPEC_NOT_LAUNCHABLE`].
    fn halted(&self, action: Action, halt: Halt) -> Step {
        let error = match class_of(halt.error.code()) {
            ErrorClass::Usage => not_launchable(&halt.error),
            ErrorClass::Refusal | ErrorClass::Failure => halt.error,
        };
        let outcome = if class_of(error.code()) == ErrorClass::Failure {
            Outcome::Failed
        } else {
            Outcome::Refused
        };
        let detail = match halt.note {
            Some(note) => format!("{error}; {note}"),
            None => error.to_string(),
        };
        Step {
            code: Some(error.code().to_owned()),
            detail: Some(detail),
            error: Some(error),
            acted: halt.acted,
            ..self.step(action, outcome)
        }
    }

    /// This pane's step after an earlier step failed.
    fn skipped(&self) -> Step {
        self.step(self.row.action(), Outcome::Skipped)
    }
}

impl Step {
    /// The step with `note`, when there is one, as its detail.
    fn noted(self, note: Option<String>) -> Self {
        Self {
            detail: note,
            ..self
        }
    }
}
