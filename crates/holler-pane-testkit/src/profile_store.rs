//! `FakeProfileStore`, the in-memory `ProfileStore` that profile tests run against
//! instead of the hub's profile registry, and `ProfileStoreOp`, the port's methods as
//! its faults and its call log name them.
//!
//! The fake passes the `ProfileStore` conformance suite
//! ([`crate::conformance::profile_store`]). What only the fake has is fault injection
//! ([`FakeProfileStore::faults`]), another writer's changes (`concurrent_put` and
//! `concurrent_delete`), a settable clock ([`FakeProfileStore::set_now`]) and the idle
//! wait of its watch.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use holler_pane::{
    next_generation, Actor, Cursor, PaneError, Profile, ProfileChange, ProfileEvent,
    ProfileLogEntry, ProfileName, ProfileStore, Watch,
};

use crate::fault::{FaultSwitch, PortOp};
use crate::feed::{lock, Change, Feed, Writer};

/// A method of the `ProfileStore` port, as a fault targets it and the call log
/// records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileStoreOp {
    Get,
    List,
    CasPut,
    Delete,
    Watch,
    /// One `next()` of an open watch.
    WatchNext,
    Log,
    Rename,
}

impl PortOp for ProfileStoreOp {
    fn as_str(self) -> &'static str {
        match self {
            ProfileStoreOp::Get => "profile_store.get",
            ProfileStoreOp::List => "profile_store.list",
            ProfileStoreOp::CasPut => "profile_store.cas_put",
            ProfileStoreOp::Delete => "profile_store.delete",
            ProfileStoreOp::Watch => "profile_store.watch",
            ProfileStoreOp::WatchNext => "profile_store.watch_next",
            ProfileStoreOp::Log => "profile_store.log",
            ProfileStoreOp::Rename => "profile_store.rename",
        }
    }
}

/// An in-memory `ProfileStore`.
///
/// It keeps the port's rules (`holler_pane::profile`; ADR-0021 sections 7 and 8):
///
/// - Every write is a compare-and-swap through `holler_pane::next_generation`: a
///   create names generation 0 and is stored at 1, the submitted generation is
///   ignored, and a stale or an ahead generation is `generation-conflict`.
/// - Each applied write appends one entry to the profile's change log: when, the
///   generation after the write, who, and what changed. A refused write appends
///   nothing and publishes no event.
/// - `delete` of a missing profile is `profile-not-found`, whatever the generation.
/// - `rename` is PROPOSED (#665) and answers `not-implemented`, changing nothing.
/// - `watch` follows the feed's rules: a cursor ahead of the head is `usage`, and a
///   watch from `Cursor(0)` yields the current state and then resumes from the head as
///   of that snapshot, as the hub's registry does.
///
/// It keeps the rules the conformance suite adds to the port, too (the suite's module
/// docs give the reasons):
///
/// - Records, events and the log are filed by the slug of the profile's name, and the
///   stored slug is always the name's: a submitted slug is ignored, as the submitted
///   generation is.
/// - The name rule: a `cas_put` of a name whose slug is stored under another name is
///   `profile-exists`, whatever the generation. It is checked before the generation,
///   so a create of such a name at 0 is `profile-exists`, not `generation-conflict`.
/// - A `Deleted` entry carries the deleted generation + 1.
/// - A profile's log is never cut: it is still readable after a delete and goes on
///   across a re-create. Only a name never created is `profile-not-found`.
///
/// Three behaviours are the fake's own, not the port's:
///
/// - The clock ([`FakeProfileStore::set_now`], 0 by default): a create stamps
///   `created` and `updated`, an update keeps `created` and stamps `updated`, and each
///   log entry's `at` is the time of its write.
/// - An `Updated` entry's summary is `pane specs: <before> -> <after>`, the number of
///   specs before and after the write.
/// - `list` is in slug order. The port pins no order.
///
/// Every port method first passes [`FakeProfileStore::faults`], and when that fails,
/// it returns the error and changes nothing. A watch's `next()` passes it too, as
/// [`ProfileStoreOp::WatchNext`]. The fake keeps its whole history, so a watch never
/// collapses writes, and an idle `next()` waits [`FakeProfileStore::set_idle_wait`]
/// (zero by default) before it yields `Ok(None)`. The fake runs no env check of its
/// own: a `ProfileSpec` holds `EnvVarName`s, which cannot carry a value.
///
/// It is not `Clone`: share it behind an `Arc`, because a copy would split the store.
pub struct FakeProfileStore {
    feed: Arc<Feed<ProfileEvent>>,
    faults: Arc<FaultSwitch<ProfileStoreOp>>,
    /// Each profile's change log, by slug, oldest first. Lock order: the feed's lock
    /// first, then this one, never the reverse. A write takes it inside its
    /// `feed.write`, and `log` takes it alone.
    change_logs: Mutex<BTreeMap<String, Vec<ProfileLogEntry>>>,
    /// The clock every write stamps, in milliseconds since the Unix epoch.
    now: AtomicI64,
}

impl FakeProfileStore {
    /// An empty store, with no fault, an idle wait of zero and the clock at 0.
    pub fn new() -> Self {
        Self {
            feed: Arc::new(Feed::new()),
            faults: Arc::new(FaultSwitch::new()),
            change_logs: Mutex::new(BTreeMap::new()),
            now: AtomicI64::new(0),
        }
    }

    /// A store holding `profiles`, each created by `actor` at expected generation 0
    /// and so stored at 1 with one `Created` entry, in order. Seeding bypasses the
    /// faults and the call log. Two seeds with one name are `generation-conflict`, as
    /// a second create would be, and a seed whose name has the slug of an earlier
    /// seed's other name is `profile-exists`.
    pub fn seeded(
        profiles: impl IntoIterator<Item = Profile>,
        actor: &Actor,
    ) -> Result<Self, PaneError> {
        let store = Self::new();
        for profile in profiles {
            store.put(&profile, Writer::Port(0), actor)?;
        }
        Ok(store)
    }

    /// How long a watch's `next()` waits for a write when nothing is owed before it
    /// yields `Ok(None)` (zero by default, so an idle `next()` answers at once).
    pub fn set_idle_wait(&self, wait: Duration) {
        self.feed.set_idle_wait(wait);
    }

    /// Set the clock: every later write stamps `millis` (milliseconds since the Unix
    /// epoch). It starts at 0, so a test that never sets it is deterministic.
    pub fn set_now(&self, millis: i64) {
        self.now.store(millis, Ordering::SeqCst);
    }

    /// The fault switch of every port method and the log of the calls made through
    /// the port.
    pub fn faults(&self) -> &FaultSwitch<ProfileStoreOp> {
        &self.faults
    }

    /// Another writer stores `profile` unconditionally, at the stored generation + 1
    /// (or at 1 for a new profile), without the name rule, logs its entry with `actor`
    /// and publishes its event. It bypasses the faults and the call log. Returns the
    /// stored record.
    pub fn concurrent_put(&self, profile: &Profile, actor: &Actor) -> Result<Profile, PaneError> {
        self.put(profile, Writer::Other, actor)
    }

    /// Another writer deletes the profile `name`: it logs a `Deleted` entry with
    /// `actor` and publishes the delete (an event with no record). It bypasses the
    /// faults and the call log. `profile-not-found` when there is no such profile.
    pub fn concurrent_delete(&self, name: &ProfileName, actor: &Actor) -> Result<(), PaneError> {
        self.remove(name, Writer::Other, actor)
    }

    /// The one write path of a put (`cas_put`, `seeded`, `concurrent_put`): the name
    /// rule for a port writer, then the compare-and-swap. The stored record is
    /// `profile` with the name's slug, the next generation and the clock's stamps.
    fn put(&self, profile: &Profile, writer: Writer, actor: &Actor) -> Result<Profile, PaneError> {
        let slug = profile.name.slug();
        self.feed.write(|log| {
            let now = self.now();
            let stored = log.get(&slug);
            if let Writer::Port(_) = writer {
                check_name(stored, profile, &slug)?;
            }
            let current = stored.map_or(0, |stored| stored.generation);
            let generation = next_generation(current, writer.expected(current))?;
            let next = Profile {
                slug: slug.clone(),
                generation,
                created: stored.map_or(now, |stored| stored.created),
                updated: now,
                ..profile.clone()
            };
            let change = match stored {
                None => ProfileChange::Created,
                Some(stored) => ProfileChange::Updated {
                    summary: summary(stored, &next),
                },
            };
            log.append(|cursor| ProfileEvent {
                cursor,
                name: next.name.clone(),
                profile: Some(Box::new(next.clone())),
            })?;
            self.record(slug, entry(now, generation, actor, change));
            Ok(next)
        })
    }

    /// The one write path of a delete (`delete`, `concurrent_delete`): a missing
    /// profile is `profile-not-found` before the generation is looked at.
    fn remove(&self, name: &ProfileName, writer: Writer, actor: &Actor) -> Result<(), PaneError> {
        let slug = name.slug();
        self.feed.write(|log| {
            let now = self.now();
            let (stored_name, current) = log
                .get(&slug)
                .map(|stored| (stored.name.clone(), stored.generation))
                .ok_or_else(|| PaneError::ProfileNotFound {
                    what: name.to_string(),
                })?;
            let generation = next_generation(current, writer.expected(current))?;
            log.append(|cursor| ProfileEvent {
                cursor,
                name: stored_name,
                profile: None,
            })?;
            self.record(slug, entry(now, generation, actor, ProfileChange::Deleted));
            Ok(())
        })
    }

    /// Append the log entry of an applied write to the log of `slug`. A write calls it
    /// inside its `feed.write` and after its event is published, so a write whose event
    /// cannot be published logs nothing.
    fn record(&self, slug: String, entry: ProfileLogEntry) {
        lock(&self.change_logs).entry(slug).or_default().push(entry);
    }

    /// The clock's time now.
    fn now(&self) -> i64 {
        self.now.load(Ordering::SeqCst)
    }
}

impl Default for FakeProfileStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileStore for FakeProfileStore {
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        self.faults.enter(ProfileStoreOp::Get)?;
        Ok(self.feed.read(|log| log.get(&name.slug()).cloned()))
    }

    fn list(&self) -> Result<Vec<Profile>, PaneError> {
        self.faults.enter(ProfileStoreOp::List)?;
        Ok(self.feed.read(|log| log.records().cloned().collect()))
    }

    fn cas_put(
        &self,
        profile: &Profile,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        self.faults.enter(ProfileStoreOp::CasPut)?;
        self.put(profile, Writer::Port(expected_generation), actor)
    }

    fn delete(
        &self,
        name: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<(), PaneError> {
        self.faults.enter(ProfileStoreOp::Delete)?;
        self.remove(name, Writer::Port(expected_generation), actor)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError> {
        self.faults.enter(ProfileStoreOp::Watch)?;
        Feed::watch(&self.feed, since, &self.faults, ProfileStoreOp::WatchNext)
    }

    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> {
        self.faults.enter(ProfileStoreOp::Log)?;
        lock(&self.change_logs)
            .get(&name.slug())
            .cloned()
            .ok_or_else(|| PaneError::ProfileNotFound {
                what: name.to_string(),
            })
    }

    /// PROPOSED (#665): after the fault switch, `not-implemented`, changing nothing.
    fn rename(
        &self,
        _from: &ProfileName,
        _to: &ProfileName,
        _expected_generation: u64,
        _actor: &Actor,
    ) -> Result<Profile, PaneError> {
        self.faults.enter(ProfileStoreOp::Rename)?;
        Err(PaneError::NotImplemented)
    }
}

impl Change for ProfileEvent {
    /// A profile is filed under the slug of its name.
    type Key = String;
    type Record = Profile;

    fn key(&self) -> String {
        self.name.slug()
    }

    fn cursor(&self) -> Cursor {
        self.cursor
    }

    fn record(&self) -> Option<&Profile> {
        self.profile.as_deref()
    }
}

/// The name rule of a port write: the slug of a profile is its identity, so a profile
/// whose slug is stored under another name cannot be written (`profile-exists`),
/// whatever the generation. A display name changes only through `rename` (#665), which
/// logs it as a rename.
fn check_name(stored: Option<&Profile>, submitted: &Profile, slug: &str) -> Result<(), PaneError> {
    match stored {
        Some(stored) if stored.name != submitted.name => Err(PaneError::ProfileExists {
            what: format!(
                "{:?} has the slug {slug:?} of the stored profile {:?}",
                submitted.name.as_str(),
                stored.name.as_str()
            ),
        }),
        _ => Ok(()),
    }
}

/// The one-line summary of an update: the number of specs before and after it.
fn summary(before: &Profile, after: &Profile) -> String {
    format!(
        "pane specs: {} -> {}",
        before.panes.len(),
        after.panes.len()
    )
}

/// The log entry of a write by `actor` at `at` that left the profile at `generation`.
fn entry(at: i64, generation: u64, actor: &Actor, change: ProfileChange) -> ProfileLogEntry {
    ProfileLogEntry {
        at,
        generation,
        actor: actor.clone(),
        change,
    }
}
