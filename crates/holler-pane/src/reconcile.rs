//! The reconcile engine (#647): observe Herdr, tmux and the harness, compare what it saw with
//! the registry, repair what the record decides, and report the rest as typed
//! [`Finding`]s. It never infers state from files (I6), and it reaches the world only through
//! the [`Ports`], so `holler pane doctor` and any later caller (the roster, a periodic loop)
//! call [`reconcile`] unchanged.
//!
//! One pass, over every pane in scope at once (a thread per pane, so a pass takes about as
//! long as its slowest pane's chain, not the pane count times it: I5):
//!
//! 1. **Observe** each pane: `host.ps`, `harness.health`, `harness.list_sessions` and
//!    `harness.shown_session`; and once per pass `herdr.version` and `herdr.snapshot`. A
//!    healthy server's list is trusted. An unhealthy server's list only tells a wedged
//!    server (the list times out) from a down one. A rule that needs a value that was not
//!    observed is skipped, and the call that failed is an `observe-failed` finding: nothing
//!    is guessed.
//! 2. **Compare** (each [`FindingKind`] says when it is reported). Stray sessions are judged
//!    against every record, not only the scope, so another profile's session of record is
//!    never a stray; unregistered Herdr panes are reported by whole-fleet passes only.
//! 3. **Repair**, with `--fix`, one thing: a shown/driven mismatch, by selecting the session
//!    of record when the pane's healthy server has it, then observing the TUI again (I3). It
//!    never creates, starts, attaches, aborts or deletes anything, never types into a pane
//!    (I4) and never writes `session_of_record`. The orchestrator's pane is repaired only by
//!    a pass that names it.
//! 4. **Record** what was observed, in one compare-and-swap of the pane's record.
//!
//! **What the record write means.** Other verbs gate on these fields (#645, #646, #648):
//!
//! - `harness.health` is written from this pass's health check: `healthy`,
//!   `{"unhealthy": "server-wedged"}` or `{"unhealthy": "server-down"}`. A failed check
//!   leaves it as stored.
//! - `last_observed.shown` is the session the TUI showed (after a fix, the one it showed
//!   then). `None` with `at > 0` is an observed home screen, or a pane with no TUI.
//! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
//!   mismatch compares SHOWN with `session_of_record` ([`shown_differs`]), which I2 makes
//!   the session the hub drives.
//! - A record is written only when one of those values changed, or on its first
//!   observation (`at == 0`): the hub's rule for a periodic writer (`panes/mod.rs`). So
//!   `at` is when the current values were first observed, not a freshness stamp. A write
//!   that loses a race with another writer is reported and never retried: that writer's
//!   record wins.
//!
//! "Once per episode" is once per pass: a pass reports every current finding exactly once
//! and keeps no memory between passes.

mod observe;

use std::collections::{BTreeMap, BTreeSet};
use std::thread::{self, ScopedJoinHandle};

use serde::{Serialize, Serializer};

use crate::findings::{embedded, quoted, Finding, FindingKind};
use crate::pane::Health;
use crate::{GridPos, HerdrSnapshot, Pane, PaneError, PaneId, PaneName, Ports, ProfileName};

use observe::Checked;

/// The Herdr calls of a pass, named as `<port>.<method>` in an `observe-failed` message.
const HERDR_VERSION: &str = "herdr.version";
const HERDR_SNAPSHOT: &str = "herdr.snapshot";

/// What one reconcile pass is asked to do.
#[derive(Debug, Clone, Copy)]
pub struct ReconcileRequest<'a> {
    /// Scope the pass to this profile's panes.
    pub profile: Option<&'a ProfileName>,
    /// Scope the pass to this one pane.
    pub pane: Option<&'a PaneName>,
    /// Repair what the record decides.
    pub fix: bool,
    /// The clock: milliseconds since the Unix epoch, written as `last_observed.at`.
    pub now_ms: i64,
}

/// What one reconcile pass found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub scope: ScopeSummary,
    pub fix_requested: bool,
    pub herdr: HerdrSummary,
    /// The hosts the records in scope name, each once, sorted by name.
    pub hosts: Vec<HostSummary>,
    /// The panes in scope, sorted by name.
    pub panes: Vec<PaneSummary>,
    /// Sorted by pane (the findings about no pane first), then by kind in
    /// [`FindingKind::ALL`] order, then by session and Herdr pane.
    pub findings: Vec<Finding>,
}

/// The scope the pass ran over: a profile's panes, one pane, both, or (neither) every pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScopeSummary {
    pub profile: Option<ProfileName>,
    pub pane: Option<PaneName>,
}

/// What Herdr reported about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HerdrSummary {
    /// Herdr's API version; `None` when it was not observed or is unsupported (a finding
    /// says which).
    pub version: Option<String>,
}

/// One host of the panes in scope, as their records name it. Doctor shows the recorded
/// Herdr API version and writes neither field (a verb records it, ADR-0021 section 10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostSummary {
    pub name: String,
    pub herdr_api_version: Option<String>,
}

/// One pane in scope, as the pass observed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneSummary {
    pub name: PaneName,
    /// Where its record places it.
    pub grid: GridPos,
    pub session_of_record: Option<String>,
    /// The session the TUI showed at the end of the pane's chain (after a fix, the one it
    /// showed then). `None` is its home screen or no TUI, or that it was not observed (an
    /// `observe-failed` finding then names `harness.shown_session`).
    pub shown: Option<String>,
    pub health: ObservedHealth,
}

/// The harness server's health as the pass observed it, spelled as the finding kinds and the
/// record's `harness.health` reasons are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedHealth {
    /// It passed its health check (`healthy`).
    Healthy,
    /// It accepts a connection and never answers (`server-wedged`).
    ServerWedged,
    /// It fails its health check and is not wedged (`server-down`).
    ServerDown,
    /// The health check itself failed, so nothing is known (`unknown`).
    Unknown,
}

impl ObservedHealth {
    /// The JSON form.
    fn as_str(self) -> &'static str {
        match self {
            ObservedHealth::Healthy => "healthy",
            ObservedHealth::ServerWedged => FindingKind::ServerWedged.code(),
            ObservedHealth::ServerDown => FindingKind::ServerDown.code(),
            ObservedHealth::Unknown => "unknown",
        }
    }

    /// What the record's `harness.health` becomes; `None` (left as stored) when unknown.
    fn recorded(self) -> Option<Health> {
        match self {
            ObservedHealth::Healthy => Some(Health::Healthy),
            ObservedHealth::ServerWedged | ObservedHealth::ServerDown => {
                Some(Health::Unhealthy(self.as_str().to_owned()))
            }
            ObservedHealth::Unknown => None,
        }
    }
}

impl Serialize for ObservedHealth {
    /// `healthy`, `server-wedged`, `server-down` or `unknown`.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Whether a pane whose session of record is `session_of_record` shows something else: the
/// shown/driven mismatch, since I2 makes the session of record the session the hub drives.
/// `shown: None` is the home screen, a mismatch too. A pane with no session of record has
/// no mismatch (it is `no-session-of-record`).
///
/// The one form of this comparison: a pass calls it with what the TUI shows now, and a
/// reader of a record (the roster, `pane get`, the gate at `send_prompt`) with
/// `last_observed.shown`, provided `last_observed.at > 0` (`0` is never observed).
pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
    session_of_record.is_some_and(|record| shown != Some(record))
}

/// Run one reconcile pass over `ports` (see the module docs).
///
/// It fails only when it cannot run the pass: a named pane or profile that does not exist
/// or a pane outside the profile (`pane-not-found`, `profile-not-found`,
/// `pane-not-in-profile`), or a pane store or profile scope that cannot be read. Everything
/// else the pass observes, a failed call included, is a finding of the report.
pub fn reconcile(ports: Ports<'_>, request: &ReconcileRequest<'_>) -> Result<Report, PaneError> {
    let (scope, records) = resolve(ports, request)?;
    let (herdr, checked) = observe_all(ports, request, &scope);
    let whole_fleet = request.profile.is_none() && request.pane.is_none();
    let mut findings = herdr_findings(herdr.as_ref(), &scope, &records, whole_fleet);
    findings.extend(strays(&scope, &checked, &records));
    let mut panes = Vec::with_capacity(checked.len());
    for pane in checked {
        findings.extend(pane.findings);
        panes.push(pane.summary);
    }
    panes.sort_by(|a, b| a.name.cmp(&b.name));
    findings.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
    // A finding observed twice (Herdr listing one pane twice) is reported once.
    findings.dedup();
    Ok(Report {
        scope: ScopeSummary {
            profile: request.profile.cloned(),
            pane: request.pane.cloned(),
        },
        fix_requested: request.fix,
        herdr: HerdrSummary {
            version: herdr.and_then(|seen| seen.version.ok()),
        },
        hosts: hosts(&scope),
        panes,
        findings,
    })
}

/// The panes in scope, and every record (the stray rule judges against all of them). The
/// membership refusals are `ProfileScope::resolve`'s, and a named pane with no record is
/// `pane-not-found`.
fn resolve(
    ports: Ports<'_>,
    request: &ReconcileRequest<'_>,
) -> Result<(Vec<Pane>, Vec<Pane>), PaneError> {
    let scope = match (request.profile, request.pane) {
        (Some(profile), pane) => ports.scope.resolve(profile, pane)?.panes,
        (None, Some(name)) => {
            let pane = ports
                .pane_store
                .get(name)?
                .ok_or_else(|| PaneError::PaneNotFound {
                    what: name.to_string(),
                })?;
            vec![pane]
        }
        (None, None) => {
            let records = ports.pane_store.list()?;
            return Ok((records.clone(), records));
        }
    };
    Ok((scope, ports.pane_store.list()?))
}

/// What the pass saw of Herdr itself.
struct HerdrSeen {
    version: Result<String, PaneError>,
    snapshot: Result<HerdrSnapshot, PaneError>,
}

/// Observe Herdr and every pane of `scope` at once: one thread for the two Herdr calls and
/// one per pane, each pane's in `scope` order. A thread that cannot start or does not finish
/// leaves `None` for Herdr, and a lone `observe-failed` for a pane.
fn observe_all(
    ports: Ports<'_>,
    request: &ReconcileRequest<'_>,
    scope: &[Pane],
) -> (Option<HerdrSeen>, Vec<Checked>) {
    thread::scope(|threads| {
        let herdr = thread::Builder::new().spawn_scoped(threads, move || HerdrSeen {
            version: ports.herdr.version(),
            snapshot: ports.herdr.snapshot(),
        });
        let chains: Vec<_> = scope
            .iter()
            .map(|pane| {
                thread::Builder::new()
                    .spawn_scoped(threads, move || observe::check_pane(ports, request, pane))
            })
            .collect();
        let checked: Vec<Checked> = scope
            .iter()
            .zip(chains)
            .map(|(pane, chain)| joined(chain).unwrap_or_else(|| observe::lost(pane)))
            .collect();
        (joined(herdr), checked)
    })
}

/// What a thread of the pass returned, or `None` when it could not start or panicked.
fn joined<T>(thread: std::io::Result<ScopedJoinHandle<'_, T>>) -> Option<T> {
    thread.ok()?.join().ok()
}

/// The findings about Herdr itself, and those that need its snapshot: `herdr-pane-missing`
/// for a pane in scope, and (whole-fleet passes only) `unregistered-herdr-pane`.
fn herdr_findings(
    herdr: Option<&HerdrSeen>,
    scope: &[Pane],
    records: &[Pane],
    whole_fleet: bool,
) -> Vec<Finding> {
    let Some(herdr) = herdr else {
        let message = format!(
            "{HERDR_VERSION} and {HERDR_SNAPSHOT} were not observed: the check did not finish"
        );
        return vec![Finding::new(FindingKind::ObserveFailed, message)];
    };
    let mut findings = Vec::new();
    match &herdr.version {
        Ok(_) => {}
        Err(PaneError::HerdrVersionUnsupported { message }) => findings.push(Finding::new(
            FindingKind::HerdrVersionUnsupported,
            format!("{}; no holler verb upgrades Herdr", embedded(message)),
        )),
        Err(error) => findings.push(Finding::new(
            FindingKind::ObserveFailed,
            failed_message(HERDR_VERSION, error),
        )),
    }
    match &herdr.snapshot {
        Ok(snapshot) => {
            findings.extend(missing_herdr_panes(snapshot, scope));
            if whole_fleet {
                findings.extend(unregistered_herdr_panes(snapshot, records));
            }
        }
        Err(error) => findings.push(Finding::new(
            FindingKind::ObserveFailed,
            failed_message(HERDR_SNAPSHOT, error),
        )),
    }
    findings
}

/// `herdr-pane-missing` for each pane of `scope` whose Herdr session and pane id the
/// snapshot lacks.
fn missing_herdr_panes(snapshot: &HerdrSnapshot, scope: &[Pane]) -> Vec<Finding> {
    let live: BTreeSet<(&str, &PaneId)> = snapshot
        .panes
        .iter()
        .map(|herdr| (herdr.session.as_str(), &herdr.pane_id))
        .collect();
    scope
        .iter()
        .filter(|pane| !live.contains(&(pane.herdr.session.as_str(), &pane.herdr.pane_id)))
        .map(|pane| {
            let message = format!(
                "Herdr has no pane {} in session {}, where the record puts this pane",
                quoted(pane.herdr.pane_id.as_str()),
                quoted(&pane.herdr.session)
            );
            Finding::about(FindingKind::HerdrPaneMissing, pane, message)
                .with_herdr_pane(&pane.herdr)
        })
        .collect()
}

/// `unregistered-herdr-pane` for each pane of the snapshot that no record names.
fn unregistered_herdr_panes(snapshot: &HerdrSnapshot, records: &[Pane]) -> Vec<Finding> {
    let named: BTreeSet<(&str, &PaneId)> = records
        .iter()
        .map(|pane| (pane.herdr.session.as_str(), &pane.herdr.pane_id))
        .collect();
    snapshot
        .panes
        .iter()
        .filter(|herdr| !named.contains(&(herdr.session.as_str(), &herdr.pane_id)))
        .map(|herdr| {
            let message = format!(
                "Herdr pane {} (session {}, workspace {}) is in no pane record; no holler verb adopts a pane",
                quoted(herdr.pane_id.as_str()),
                quoted(&herdr.session),
                quoted(&herdr.workspace)
            );
            Finding::new(FindingKind::UnregisteredHerdrPane, message).with_herdr_pane(herdr)
        })
        .collect()
}

/// `stray-session` for each session a healthy in-scope server listed that is the session
/// of record of no record at all: one finding per session, with every port it was listed on
/// (the servers of a fleet can share one data directory).
fn strays(scope: &[Pane], checked: &[Checked], records: &[Pane]) -> Vec<Finding> {
    let recorded: BTreeSet<&str> = records
        .iter()
        .filter_map(|pane| pane.session_of_record.as_deref())
        .collect();
    let mut listed_on: BTreeMap<&str, BTreeSet<u16>> = BTreeMap::new();
    for (pane, chain) in scope.iter().zip(checked) {
        for session in chain.listed.iter().flatten() {
            if !recorded.contains(session.as_str()) {
                listed_on
                    .entry(session.as_str())
                    .or_default()
                    .insert(pane.harness.port);
            }
        }
    }
    listed_on
        .into_iter()
        .map(|(session, ports)| {
            let message = format!(
                "session {} is the session of record of no pane; no holler verb deletes a harness session",
                quoted(session)
            );
            Finding::new(FindingKind::StraySession, message)
                .with_session(Some(session))
                .with_ports(ports.into_iter().collect())
        })
        .collect()
}

/// The hosts the records of `scope` name, each `(name, herdr_api_version)` once, sorted.
fn hosts(scope: &[Pane]) -> Vec<HostSummary> {
    let hosts: BTreeSet<(&str, Option<&str>)> = scope
        .iter()
        .map(|pane| {
            (
                pane.host.name.as_str(),
                pane.host.herdr_api_version.as_deref(),
            )
        })
        .collect();
    hosts
        .into_iter()
        .map(|(name, version)| HostSummary {
            name: name.to_owned(),
            herdr_api_version: version.map(str::to_owned),
        })
        .collect()
}

/// The order of a report's findings (see [`Report::findings`]); the message breaks the
/// last ties, so the order is total.
fn sort_key(finding: &Finding) -> (&str, FindingKind, Option<&str>, Option<&PaneId>, &str) {
    (
        finding.pane.as_ref().map_or("", PaneName::as_str),
        finding.kind,
        finding.session.as_deref(),
        finding.herdr_pane.as_ref(),
        &finding.message,
    )
}

/// The message of an `observe-failed` finding: the call, as `<port>.<method>`, the error's
/// code and its message.
fn failed_message(op: &str, error: &PaneError) -> String {
    format!(
        "{op} failed ({}): {}",
        error.code(),
        embedded(&error.to_string())
    )
}
