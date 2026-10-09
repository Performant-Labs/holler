//! `FakePaneStore`, the in-memory `PaneStore` that pane-control tests run against
//! instead of the hub's registry, and `PaneStoreOp`, the port's methods as its faults
//! and its call log name them.
//!
//! The fake passes the `PaneStore` conformance suite
//! ([`crate::conformance::pane_store`]). What only the fake has is fault injection
//! ([`FakePaneStore::faults`]), another writer's changes (`concurrent_put` and
//! `concurrent_delete`) and the idle wait of its watch.

use std::sync::Arc;
use std::time::Duration;

use holler_pane::{
    next_generation, Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, Watch,
};

use crate::fault::{FaultSwitch, PortOp};
use crate::feed::{Change, Feed, Writer};

/// A method of the `PaneStore` port, as a fault targets it and the call log records
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneStoreOp {
    Get,
    List,
    CasPut,
    Delete,
    Watch,
    /// One `next()` of an open watch.
    WatchNext,
}

impl PortOp for PaneStoreOp {
    fn as_str(self) -> &'static str {
        match self {
            PaneStoreOp::Get => "pane_store.get",
            PaneStoreOp::List => "pane_store.list",
            PaneStoreOp::CasPut => "pane_store.cas_put",
            PaneStoreOp::Delete => "pane_store.delete",
            PaneStoreOp::Watch => "pane_store.watch",
            PaneStoreOp::WatchNext => "pane_store.watch_next",
        }
    }
}

/// An in-memory `PaneStore`.
///
/// It keeps the port's rules (`holler_pane::ports`; ADR-0021 sections 2, 6 and 8):
///
/// - Every write is a compare-and-swap through `holler_pane::next_generation`: a
///   create names generation 0 and is stored at 1, the submitted generation is
///   ignored, and a stale or an ahead generation is `generation-conflict`.
/// - `delete` of a missing record is `pane-not-found`, whatever the generation.
/// - `list` is sorted by name.
/// - A pane that belongs to a profile cannot move to another profile in one write
///   (`pane-in-other-profile`); it leaves its profile (`None`) first. Profiles are
///   compared by slug.
/// - `watch` follows the feed's rules: a cursor ahead of the head is `usage`, and a
///   watch from `Cursor(0)` yields the current state and then resumes from the head as
///   of that snapshot, as the hub's registry does.
///
/// Two behaviours are the fake's own, not the port's:
///
/// - A `cas_put` is checked for its generation first and for membership second, so a
///   stale write that also changes the profile is `generation-conflict`.
/// - The fake does not check that the profile a pane names exists. The hub does that
///   in `check_membership`, outside the port, and answers `profile-not-found`.
///
/// Every port method first passes [`FakePaneStore::faults`], and when that fails, it
/// returns the error and changes nothing. A watch's `next()` passes it too, as
/// [`PaneStoreOp::WatchNext`]. The fake keeps its whole history, so a watch never
/// collapses writes, and an idle `next()` waits [`FakePaneStore::set_idle_wait`] (zero
/// by default) before it yields `Ok(None)`.
///
/// It is not `Clone`: share it behind an `Arc`, because a copy would split the store.
pub struct FakePaneStore {
    feed: Arc<Feed<PaneEvent>>,
    faults: Arc<FaultSwitch<PaneStoreOp>>,
}

impl FakePaneStore {
    /// An empty store, with no fault and an idle wait of zero.
    pub fn new() -> Self {
        Self {
            feed: Arc::new(Feed::new()),
            faults: Arc::new(FaultSwitch::new()),
        }
    }

    /// A store holding `panes`, each created at expected generation 0 and so stored
    /// at 1, in order. Seeding bypasses the faults and the call log. Two seeds with one
    /// name are `generation-conflict`, as a second create would be.
    pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError> {
        let store = Self::new();
        for pane in panes {
            store.put(&pane, Writer::Port(0))?;
        }
        Ok(store)
    }

    /// How long a watch's `next()` waits for a write when nothing is owed before it
    /// yields `Ok(None)` (zero by default, so an idle `next()` answers at once).
    pub fn set_idle_wait(&self, wait: Duration) {
        self.feed.set_idle_wait(wait);
    }

    /// The fault switch of every port method and the log of the calls made through
    /// the port.
    pub fn faults(&self) -> &FaultSwitch<PaneStoreOp> {
        &self.faults
    }

    /// Another writer stores `pane` unconditionally, at the stored generation + 1 (or
    /// at 1 for a new record), without the membership rule, and publishes its event.
    /// It bypasses the faults and the call log. Returns the stored record.
    pub fn concurrent_put(&self, pane: &Pane) -> Result<Pane, PaneError> {
        self.put(pane, Writer::Other)
    }

    /// Another writer removes the record named `name` and publishes its delete (an
    /// event with no record). It bypasses the faults and the call log.
    /// `pane-not-found` when there is no such record.
    pub fn concurrent_delete(&self, name: &PaneName) -> Result<(), PaneError> {
        self.remove(name, Writer::Other)
    }

    /// The one write path of a put (`cas_put`, `seeded`, `concurrent_put`): the
    /// stored record is `pane` with only its generation replaced.
    fn put(&self, pane: &Pane, writer: Writer) -> Result<Pane, PaneError> {
        self.feed.write(|log| {
            let stored = log.get(&pane.name);
            let current = stored.map_or(0, |stored| stored.generation);
            let generation = next_generation(current, writer.expected(current))?;
            if let Writer::Port(_) = writer {
                check_membership(stored, pane)?;
            }
            let next = Pane {
                generation,
                ..pane.clone()
            };
            log.append(|cursor| PaneEvent {
                cursor,
                name: next.name.clone(),
                pane: Some(Box::new(next.clone())),
            })?;
            Ok(next)
        })
    }

    /// The one write path of a delete (`delete`, `concurrent_delete`): a missing
    /// record is `pane-not-found` before the generation is looked at.
    fn remove(&self, name: &PaneName, writer: Writer) -> Result<(), PaneError> {
        self.feed.write(|log| {
            let current = log
                .get(name)
                .map(|stored| stored.generation)
                .ok_or_else(|| PaneError::PaneNotFound {
                    what: name.to_string(),
                })?;
            next_generation(current, writer.expected(current))?;
            log.append(|cursor| PaneEvent {
                cursor,
                name: name.clone(),
                pane: None,
            })
        })
    }
}

impl Default for FakePaneStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneStore for FakePaneStore {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        self.faults.enter(PaneStoreOp::Get)?;
        Ok(self.feed.read(|log| log.get(name).cloned()))
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        self.faults.enter(PaneStoreOp::List)?;
        Ok(self.feed.read(|log| log.records().cloned().collect()))
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        self.faults.enter(PaneStoreOp::CasPut)?;
        self.put(pane, Writer::Port(expected_generation))
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        self.faults.enter(PaneStoreOp::Delete)?;
        self.remove(name, Writer::Port(expected_generation))
    }

    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        self.faults.enter(PaneStoreOp::Watch)?;
        Feed::watch(&self.feed, since, &self.faults, PaneStoreOp::WatchNext)
    }
}

impl Change for PaneEvent {
    type Key = PaneName;
    type Record = Pane;

    fn key(&self) -> PaneName {
        self.name.clone()
    }

    fn cursor(&self) -> Cursor {
        self.cursor
    }

    fn record(&self) -> Option<&Pane> {
        self.pane.as_deref()
    }
}

/// The membership rule of a `cas_put` (ADR-0021 section 8, decided to run inside the
/// registry's compare-and-swap): a pane stored with profile P cannot be written with
/// another profile Q. Leaving (`None`), joining from `None` and keeping P are allowed.
fn check_membership(stored: Option<&Pane>, submitted: &Pane) -> Result<(), PaneError> {
    let current = stored.and_then(|pane| pane.profile.as_ref());
    match (current, submitted.profile.as_ref()) {
        (Some(current), Some(next)) if current.slug() != next.slug() => {
            Err(PaneError::PaneInOtherProfile {
                what: format!(
                    "{} is in profile {:?}, not {:?}",
                    submitted.name,
                    current.as_str(),
                    next.as_str()
                ),
            })
        }
        _ => Ok(()),
    }
}
