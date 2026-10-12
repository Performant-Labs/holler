//! Unmanaged panes (story #832, epic #633 decision 9): a Herdr pane no record names, shown
//! with its state `unmanaged` instead of being omitted.
//!
//! A pane that runs something Holler does not drive (today a Claude Code session set up by
//! hand) has no record, so the registry cannot list it, and nobody sees that its screen has
//! not changed for hours. `pane list` and `pane get` show such a pane with the pane id, its
//! label, its working directory and **when its screen last changed**, and `pane doctor`
//! reports a long unchanged screen as `unmanaged-stalled`.
//!
//! - `unmanaged` is **observed, never stored** (decision 2: the registry is the only record
//!   of a pane). Nothing here writes anything.
//! - It is a visible state, **not a hold**: no verb that acts on a pane accepts one
//!   ([`PANE_UNMANAGED`], exit 3, from [`refuse_if_unmanaged`]).
//! - The time of the last screen change is whatever [`HerdrPort::observe_pane`] reports,
//!   and `None` when Herdr reports none. Herdr 0.8.2 (protocol 20) reports none, so the
//!   real adapter leaves it empty: this module never guesses one from an earlier read or a
//!   file, and the idle time is `None` with it.
//!
//! [`HerdrPort::observe_pane`]: crate::ports::HerdrPort::observe_pane

use serde::Serialize;

use crate::error::{PaneError, RefusalCode};
use crate::findings::{quoted, Finding, FindingKind};
use crate::pane::{HerdrPane, Pane, PaneId, PaneName};
use crate::ports::{HerdrSnapshot, PaneObservation, Ports};
use crate::GridPos;

/// The state word of a pane no record names.
pub const STATE: &str = "unmanaged";

/// `pane-unmanaged`: a verb that acts on a pane was given a pane Holler does not drive
/// (a Herdr pane no record names). Refusal, exit 3.
pub const PANE_UNMANAGED: RefusalCode = RefusalCode::from_static("pane-unmanaged");

/// One Herdr pane that no record names, as `pane list` and `pane get` show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnmanagedPane {
    /// Always [`STATE`].
    pub state: &'static str,
    /// Herdr's id for the pane.
    pub pane_id: PaneId,
    /// The Herdr session the pane is in.
    pub session: String,
    /// The Herdr workspace the pane is in.
    pub workspace: String,
    /// The pane's cell.
    pub pos: GridPos,
    /// The label Herdr reports for the pane, if it reports one.
    pub label: Option<String>,
    /// The pane's working directory, if Herdr reports one.
    pub cwd: Option<String>,
    /// When the pane's screen last changed (milliseconds since the Unix epoch), if Herdr
    /// reports it.
    pub last_screen_change: Option<i64>,
    /// Whole seconds from `last_screen_change` to the clock of the call; `None` with it.
    pub idle_seconds: Option<u64>,
    /// The snapshot entry, kept for the findings that name it.
    #[serde(skip)]
    pub(crate) herdr: HerdrPane,
}

/// The panes of `snapshot` that no record names: **the one rule of "a pane Holler does not
/// drive"**, public so that no story writes its own copy. `pane list`/`get`, the
/// `pane-unmanaged` refusal and the `unregistered-herdr-pane` finding of whole-fleet reconcile
/// passes call it, and so do #831 (a second harness kind) and #835 (import decides whether
/// Holler drives a pane). A record names a pane by Herdr session and pane id.
///
/// Today "no record names it" is the only way a pane is undriveable, because
/// `HarnessKind` has one variant. The other half of the story's wording, "or whose harness
/// Holler cannot drive", has no case yet; #831 adds it here (a `HarnessKind` that is not
/// drivable), so the rule stays in this one function.
pub fn unregistered<'a>(
    snapshot: &'a HerdrSnapshot,
    records: &'a [Pane],
) -> impl Iterator<Item = &'a HerdrPane> + 'a {
    snapshot
        .panes
        .iter()
        .filter(move |herdr| is_unmanaged(herdr, records))
}

/// Whether `herdr` is a pane Holler does not drive, given every record: [`unregistered`] for
/// one pane. The question #831 and #835 ask; see [`unregistered`] for who else calls it.
pub fn is_unmanaged(herdr: &HerdrPane, records: &[Pane]) -> bool {
    !records
        .iter()
        .any(|pane| pane.herdr.session == herdr.session && pane.herdr.pane_id == herdr.pane_id)
}

/// Every pane of the live Herdr session that no record names, sorted by workspace and
/// cell, each with what Herdr reports about it. `now_ms` is the clock: idle time is counted
/// to it.
///
/// A failed `snapshot` or `pane_store.list` is the call's own error. A pane Herdr cannot
/// observe leaves its `cwd` and `last_screen_change` empty: the pane is still listed. The
/// panes are observed together (`HerdrPort::observe_panes`), so many of them cost one call.
pub fn observe(ports: Ports<'_>, now_ms: i64) -> Result<Vec<UnmanagedPane>, PaneError> {
    let snapshot = ports.herdr.snapshot()?;
    let records = ports.pane_store.list()?;
    Ok(from_snapshot(ports, &snapshot, &records, now_ms))
}

/// [`observe`] over a snapshot and the records the caller already has.
pub(crate) fn from_snapshot(
    ports: Ports<'_>,
    snapshot: &HerdrSnapshot,
    records: &[Pane],
    now_ms: i64,
) -> Vec<UnmanagedPane> {
    let listed: Vec<&HerdrPane> = unregistered(snapshot, records).collect();
    let ids: Vec<PaneId> = listed.iter().map(|herdr| herdr.pane_id.clone()).collect();
    // One call for all of them: a list of many unmanaged panes is one read of Herdr.
    let seen = if ids.is_empty() {
        Vec::new()
    } else {
        ports.herdr.observe_panes(&ids)
    };
    let mut panes: Vec<UnmanagedPane> = listed
        .into_iter()
        .zip(
            seen.iter()
                .chain(std::iter::repeat(&PaneObservation::default())),
        )
        .map(|(herdr, seen)| UnmanagedPane::new(herdr, seen, now_ms))
        .collect();
    panes.sort_by(|a, b| {
        let key = |pane: &UnmanagedPane| {
            (
                pane.session.clone(),
                pane.workspace.clone(),
                pane.pos.row,
                pane.pos.col,
                pane.pane_id.as_str().to_owned(),
            )
        };
        key(a).cmp(&key(b))
    });
    panes
}

/// The unmanaged pane `arg` names by its label or its Herdr pane id, observed now: `None`
/// when no pane no record names answers to `arg`. A failed `snapshot` or `pane_store.list`
/// is the call's own error.
pub fn find(ports: Ports<'_>, arg: &str, now_ms: i64) -> Result<Option<UnmanagedPane>, PaneError> {
    Ok(observe(ports, now_ms)?
        .into_iter()
        .find(|pane| pane.pane_id.as_str() == arg || pane.label.as_deref() == Some(arg)))
}

/// The error for a pane `arg` that a verb found **no record** for: `pane-unmanaged` when
/// `arg` is the label or the Herdr pane id of a pane no record names, and `pane-not-found`
/// otherwise. A verb calls it where it would report the missing record, so a pane that has a
/// record costs no extra read.
pub fn missing(ports: Ports<'_>, arg: &str) -> PaneError {
    match refuse_unrecorded(ports, arg) {
        Err(refusal) => refusal,
        Ok(()) => PaneError::PaneNotFound {
            what: arg.to_owned(),
        },
    }
}

/// `Err(pane-unmanaged)` when `arg` (what a verb was given to name a pane) is the label or
/// the Herdr pane id of a pane no record names, and `Ok` otherwise: when a record is
/// named `arg`, when Herdr has no such pane, and when Herdr or the pane store cannot be
/// read (the verb's own error then stands).
pub fn refuse_if_unmanaged(ports: Ports<'_>, arg: &str) -> Result<(), PaneError> {
    if let Ok(name) = PaneName::parse(arg) {
        if !matches!(ports.pane_store.get(&name), Ok(None)) {
            return Ok(());
        }
    }
    refuse_unrecorded(ports, arg)
}

/// [`refuse_if_unmanaged`] for an `arg` the caller knows no record is named.
fn refuse_unrecorded(ports: Ports<'_>, arg: &str) -> Result<(), PaneError> {
    let (Ok(snapshot), Ok(records)) = (ports.herdr.snapshot(), ports.pane_store.list()) else {
        return Ok(());
    };
    let found = unregistered(&snapshot, &records)
        .find(|herdr| herdr.pane_id.as_str() == arg || herdr.label.as_deref() == Some(arg));
    match found {
        None => Ok(()),
        Some(herdr) => Err(PaneError::Refused {
            code: PANE_UNMANAGED,
            message: format!(
                "{} is a Herdr pane no record names (pane {}), so Holler does not drive it \
                 and no verb acts on it",
                quoted(arg),
                quoted(herdr.pane_id.as_str())
            ),
        }),
    }
}

/// `unmanaged-stalled` for each pane of `panes` whose screen has not changed for at least
/// `after_secs`. A pane with no known idle time has no finding.
pub fn stalled_findings(panes: &[UnmanagedPane], after_secs: u64) -> Vec<Finding> {
    panes
        .iter()
        .filter_map(|pane| {
            let idle = pane.idle_seconds.filter(|idle| *idle >= after_secs)?;
            let label = pane.label.as_deref().map_or_else(|| "-".to_owned(), quoted);
            let message = format!(
                "Herdr pane {} (label {label}) has shown the same screen for {} (the threshold \
                 is {}); no holler verb drives it",
                quoted(pane.pane_id.as_str()),
                duration_text(idle),
                duration_text(after_secs)
            );
            Some(Finding::new(FindingKind::UnmanagedStalled, message).with_herdr_pane(&pane.herdr))
        })
        .collect()
}

/// A span of seconds in a few characters: `45s`, `35m`, `4h05m`, `2d3h`. Under a minute it
/// is seconds; under an hour whole minutes; under a day hours and zero-padded minutes;
/// otherwise days and hours.
pub fn duration_text(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m", secs / 60),
        3600..=86_399 => format!("{}h{:02}m", secs / 3600, (secs % 3600) / 60),
        _ => format!("{}d{}h", secs / 86_400, (secs % 86_400) / 3600),
    }
}

impl UnmanagedPane {
    /// The pane `herdr` as the snapshot lists it, with what `seen` reports and the idle time
    /// counted to `now_ms`.
    fn new(herdr: &HerdrPane, seen: &PaneObservation, now_ms: i64) -> Self {
        let idle_seconds = seen
            .last_screen_change
            .map(|at| u64::try_from(now_ms.saturating_sub(at)).unwrap_or(0) / 1000);
        Self {
            state: STATE,
            pane_id: herdr.pane_id.clone(),
            session: herdr.session.clone(),
            workspace: herdr.workspace.clone(),
            pos: herdr.grid,
            label: herdr.label.clone(),
            cwd: seen.cwd.clone(),
            last_screen_change: seen.last_screen_change,
            idle_seconds,
            herdr: herdr.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn herdr(label: Option<&str>) -> HerdrPane {
        HerdrPane {
            session: "s".to_owned(),
            workspace: "w".to_owned(),
            pane_id: PaneId::new("w1:p1"),
            grid: GridPos { row: 1, col: 2 },
            label: label.map(str::to_owned),
        }
    }

    #[test]
    fn idle_time_is_whole_seconds_from_the_screen_change_to_the_clock() {
        let seen = PaneObservation::default().with_last_screen_change(1_000_000);
        let pane = UnmanagedPane::new(&herdr(None), &seen, 1_000_000 + 4 * 3_600_000 + 999);
        assert_eq!(pane.idle_seconds, Some(14_400));
        assert_eq!(pane.state, "unmanaged");
    }

    #[test]
    fn a_screen_change_in_the_future_is_idle_zero_not_a_wrapped_number() {
        let seen = PaneObservation::default().with_last_screen_change(5_000);
        assert_eq!(
            UnmanagedPane::new(&herdr(None), &seen, 1_000).idle_seconds,
            Some(0)
        );
    }

    #[test]
    fn no_time_from_herdr_is_no_idle_time() {
        let pane = UnmanagedPane::new(&herdr(None), &PaneObservation::default(), 1_000);
        assert_eq!((pane.last_screen_change, pane.idle_seconds), (None, None));
    }

    #[test]
    fn the_threshold_is_inclusive_and_an_unknown_idle_time_has_no_finding() {
        let at = |idle: Option<u64>| UnmanagedPane {
            idle_seconds: idle,
            ..UnmanagedPane::new(&herdr(Some("p")), &PaneObservation::default(), 0)
        };
        let panes = [at(Some(1799)), at(Some(1800)), at(Some(9000)), at(None)];
        assert_eq!(stalled_findings(&panes, 1800).len(), 2);
        let finding = &stalled_findings(&panes, 1800)[0];
        assert_eq!(finding.kind.code(), "unmanaged-stalled");
        assert_eq!(
            finding.herdr_pane.as_ref().map(PaneId::as_str),
            Some("w1:p1")
        );
    }

    #[test]
    fn durations_are_short_and_stable() {
        for (secs, want) in [
            (0, "0s"),
            (59, "59s"),
            (60, "1m"),
            (1800, "30m"),
            (3599, "59m"),
            (3600, "1h00m"),
            (14_700, "4h05m"),
            (86_399, "23h59m"),
            (86_400, "1d0h"),
            (183_600, "2d3h"),
        ] {
            assert_eq!(duration_text(secs), want, "{secs}");
        }
    }

    #[test]
    fn the_refusal_code_is_the_published_one() {
        assert_eq!(PANE_UNMANAGED.as_str(), "pane-unmanaged");
    }
}
