//! The profile registry's store (D1): the table of entries behind one lock, its file, and
//! the long-poll wait of its change feed.
//!
//! It is the twin of `panes::store` and keeps its rules. One `std::sync::Mutex` guards the
//! whole table, and one `Condvar` wakes the watchers, which wait without holding the lock
//! ([`Store::poll`]). Every method is synchronous. The handlers reach the store through
//! `spawn_blocking`, and no `.await` ever happens while the lock is held. Only a rename
//! takes the pane registry's lock, after its own (ADR-0021 §7; see "Renames" below).
//!
//! What differs is the entry (`entry.rs`: the record or its tombstone, and the change log,
//! filed by slug) and the write itself. The lock, `read`, `commit`, `apply`, `save`, `watch`,
//! `poll`, the `Feed` iterator and `load_table` mirror their pane twins item for item, under
//! the same names, and each says so. #639's store is concrete over the pane table, and this
//! story may change it by one comparison only, so making the two stores one generic store is
//! left to a follow-up (D10).
//!
//! # Writes
//!
//! A write runs entirely under the lock, in this order:
//!
//! 1. the name rule (`check_name`): a name whose slug is filed under another live name is
//!    `profile-exists`, whatever the generation;
//! 2. the compare-and-swap (`holler_pane::next_generation`);
//! 3. the stored record and its log entry, stamped once (`entry::stamp`, D4);
//! 4. the next document, and the save;
//! 5. only then, the commit to memory, the event and the wake-up.
//!
//! A refused write therefore logs nothing and takes no cursor, and a failed save changes
//! nothing (memory, file and cursor): the write answers `unavailable`, and the same write
//! with the same generation succeeds once the file can be written again.
//!
//! # Renames (#665)
//!
//! [`Store::rename`] is one write of two entries: OLD's slug becomes a tombstone whose log
//! gets one `deleted` entry, and NEW's slug the record and one `renamed {from}` entry, both
//! at the new generation and stamped once (one entry and one event when the slug does not
//! change). Given the pane registry (`profile/rename`, ADR-0021 section 8, "Rename as
//! built"), it is also the one transaction that spans the two registries: under this lock it
//! takes the pane lock (`PaneState::locked`), checks that both registries loaded, runs the
//! rename's own checks (in the order [`Store::rename`] gives), saves the pane document with
//! the member panes' moves, then this one, and only then commits both to memory. When this
//! save fails after the pane save, the pane document is saved back (`Locked::restore`) and
//! nothing changed. The port, given no pane registry, renames the record only.
//!
//! # A failed load
//!
//! A file that cannot be loaded (`entry.rs`, `panes::persist`) leaves the store failed for
//! the life of the process. Every method, `watch` included, answers the same
//! `store-corrupt`, and nothing is ever written, so the operator finds the file exactly as
//! it was. The hub logs one `error` event, `profile_registry_corrupt`, when it loads the
//! file, and `profile_registry_write_failed` when a write cannot be saved.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use holler_pane::{
    next_generation, Actor, Cursor, PaneError, Profile, ProfileChange, ProfileEvent,
    ProfileLogEntry, ProfileName, Watch, WatchReply,
};
use holler_proto::clock::now_millis;

use super::entry::{self, ProfileEntry};
use crate::panes::feed::{self, Ring};
use crate::panes::persist::{self, Doc};
use crate::panes::{log_fault, PaneState, PaneStoreOptions};

/// How the registry names itself in the `what` of an error.
const LABEL: &str = "profile registry";

/// What a file that cannot be loaded means for the hub, as the log states it.
const CORRUPT_EFFECT: &str = "every profile method answers store-corrupt until the file is \
    repaired and the hub restarted; the file is left as it is";

/// What a failed save means, as the log states it.
const WRITE_EFFECT: &str = "the write was refused and the registry is unchanged";

/// The registry's entries and change feed.
struct Table {
    /// The cursor of the last change (0: no change yet).
    head: Cursor,
    /// The entry filed under each slug: the record or its tombstone, and the change log.
    entries: BTreeMap<String, ProfileEntry>,
    /// The recent events, without the logs.
    ring: Ring<ProfileEvent>,
}

impl Table {
    /// The table a loaded document describes, or the empty table when there is no file.
    /// Twin of `panes::store::Table::from_doc`.
    fn from_doc(doc: Option<Doc<ProfileEntry>>, retained: usize) -> Self {
        let (head, entries) = doc.map_or((Cursor(0), Vec::new()), |doc| (doc.cursor, doc.entries));
        Self {
            head,
            entries: entries
                .into_iter()
                .map(|entry| (entry.slug.clone(), entry))
                .collect(),
            ring: Ring::new(head, retained),
        }
    }

    /// The live record filed under the slug of `name`, if there is one.
    fn record(&self, name: &ProfileName) -> Option<&Profile> {
        self.entries
            .get(&name.slug())
            .and_then(ProfileEntry::profile)
    }

    /// The cursor the next change takes. Twin of `panes::store::Table::next_cursor`.
    fn next_cursor(&self) -> Result<Cursor, PaneError> {
        after(self.head)
    }

    /// The entries of the next document: every entry, with each of `changes` filed under its
    /// slug, in slug order. Twin of `panes::store::Table::entries_with`.
    fn entries_with<'a>(&'a self, changes: &'a [ProfileEntry]) -> Vec<&'a ProfileEntry> {
        let mut entries: BTreeMap<&str, &ProfileEntry> = self
            .entries
            .iter()
            .map(|(slug, entry)| (slug.as_str(), entry))
            .collect();
        for change in changes {
            entries.insert(&change.slug, change);
        }
        entries.into_values().collect()
    }
}

/// The cursor after `cursor`: the next change's, or the one a write's later change takes.
/// Twin of `panes::store::after`.
fn after(cursor: Cursor) -> Result<Cursor, PaneError> {
    cursor
        .0
        .checked_add(1)
        .map(Cursor)
        .ok_or_else(|| PaneError::StoreCorrupt {
            what: format!("{LABEL}: the change cursor overflowed"),
        })
}

/// The profile registry's store. `ProfileState` holds it behind an `Arc`, which the
/// handlers and every `Watch` iterator share. Twin of `panes::store::Store`.
pub(crate) struct Store {
    path: PathBuf,
    options: PaneStoreOptions,
    /// The table, or the failure that keeps the store closed.
    table: Mutex<Result<Table, PaneError>>,
    /// Notified after every committed write, for the waiting watchers.
    changed: Condvar,
}

impl Store {
    /// Load the registry from the file at `path`. A file that cannot be loaded leaves the
    /// store failed (see the module docs); this never fails itself. Twin of
    /// `panes::store::Store::open`.
    pub(crate) fn open(path: PathBuf, options: PaneStoreOptions) -> Self {
        let table = load_table(&path, options.feed_retained);
        Self {
            path,
            options,
            table: Mutex::new(table),
            changed: Condvar::new(),
        }
    }

    /// Twin of `panes::store::Store::lock`.
    fn lock(&self) -> MutexGuard<'_, Result<Table, PaneError>> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `view` applied to the table, or the store's failure. Twin of
    /// `panes::store::Store::read`.
    fn read<T>(&self, view: impl FnOnce(&Table) -> T) -> Result<T, PaneError> {
        match &*self.lock() {
            Ok(table) => Ok(view(table)),
            Err(err) => Err(err.clone()),
        }
    }

    /// The profile filed under the slug of `name`, or `None`. Another spelling of the name
    /// finds it (D6).
    pub(crate) fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        self.read(|table| table.record(name).cloned())
    }

    /// Every live profile, in slug order (the port pins no order).
    pub(crate) fn list(&self) -> Result<Vec<Profile>, PaneError> {
        self.read(|table| {
            table
                .entries
                .values()
                .filter_map(ProfileEntry::profile)
                .cloned()
                .collect()
        })
    }

    /// The change log of the profile filed under the slug of `name`, oldest first. A
    /// deleted profile keeps its log, so only a name never created is `profile-not-found`.
    pub(crate) fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> {
        let slug = name.slug();
        self.read(|table| table.entries.get(&slug).map(|entry| entry.log.clone()))?
            .ok_or_else(|| PaneError::ProfileNotFound {
                what: name.to_string(),
            })
    }

    /// Store `profile` as `actor` if the stored record is still at `expected` (0 for a new
    /// profile), and log the write. The stored record is `profile` with the name's slug,
    /// the next generation and the hub's stamps: the submitted slug, generation, `created`
    /// and `updated` are ignored. Returns the stored record.
    pub(crate) fn cas_put(
        &self,
        profile: &Profile,
        expected: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        let mut guard = self.lock();
        let table = guard.as_mut().map_err(|err| err.clone())?;
        let slug = profile.name.slug();
        let previous = table.entries.get(&slug);
        let stored = previous.and_then(ProfileEntry::profile);
        check_name(stored, &profile.name, &slug)?;
        let generation = next_generation(stored.map_or(0, |stored| stored.generation), expected)?;
        let at = entry::stamp(previous, now_millis());
        let record = Profile {
            slug: slug.clone(),
            generation,
            created: stored.map_or(at, |stored| stored.created),
            updated: at,
            ..profile.clone()
        };
        let change = stored.map_or(ProfileChange::Created, |stored| ProfileChange::Updated {
            summary: entry::summary(&stored.panes, &record.panes),
        });
        let event = ProfileEvent {
            cursor: table.next_cursor()?,
            name: record.name.clone(),
            profile: Some(Box::new(record.clone())),
        };
        let logged = log_entry(at, generation, actor, change);
        let next = ProfileEntry::next(previous, slug, event, logged);
        self.commit(table, vec![next])?;
        Ok(record)
    }

    /// Delete the profile filed under the slug of `name` as `actor` if it is still at
    /// `expected`, and log the delete at the deleted generation + 1. A missing profile is
    /// `profile-not-found` whatever `expected` is: that is checked before the generation.
    /// The tombstone keeps the stored display name, whatever spelling `name` used.
    pub(crate) fn delete(
        &self,
        name: &ProfileName,
        expected: u64,
        actor: &Actor,
    ) -> Result<(), PaneError> {
        let mut guard = self.lock();
        let table = guard.as_mut().map_err(|err| err.clone())?;
        let slug = name.slug();
        let previous = table.entries.get(&slug);
        let Some(stored) = previous.and_then(ProfileEntry::profile) else {
            return Err(PaneError::ProfileNotFound {
                what: name.to_string(),
            });
        };
        let generation = next_generation(stored.generation, expected)?;
        let at = entry::stamp(previous, now_millis());
        let event = ProfileEvent {
            cursor: table.next_cursor()?,
            name: stored.name.clone(),
            profile: None,
        };
        let logged = log_entry(at, generation, actor, ProfileChange::Deleted);
        let next = ProfileEntry::next(previous, slug, event, logged);
        self.commit(table, vec![next])
    }

    /// Rename the profile filed under the slug of `from` to `to` as `actor` if it is still at
    /// `expected`, and return the renamed record (see "Renames" in the module docs). With
    /// `panes` this is `profile/rename`'s whole transaction, every member pane moved to `to`;
    /// without, the record half the port promises.
    ///
    /// Nothing is written before every check passed, in this order: either registry failed to
    /// load (`store-corrupt`), `from` is not found by slug (`profile-not-found`, whatever
    /// `expected`), `to` is the stored name already (`usage`), `to`'s slug is another live
    /// profile's (`profile-exists`, whatever `expected`), and the compare-and-swap
    /// (`generation-conflict`).
    pub(crate) fn rename(
        &self,
        from: &ProfileName,
        to: &ProfileName,
        expected: u64,
        actor: &Actor,
        panes: Option<&PaneState>,
    ) -> Result<Profile, PaneError> {
        let mut guard = self.lock();
        // The profile lock is held: the pane lock comes second (ADR-0021 section 7).
        let mut members = panes.map(PaneState::locked).transpose()?;
        let table = guard.as_mut().map_err(|err| err.clone())?;
        let (record, changes) = renamed(table, from, to, expected, actor)?;
        let moves = match &members {
            Some(members) => members.moves(from, to)?,
            None => Vec::new(),
        };
        // The panes first: a crash between the two saves leaves the members naming `to`
        // while `from` stands, and the same rename run again completes it.
        if let Some(members) = &members {
            members.save(&moves)?;
        }
        if let Err(err) = self.save(table, &changes) {
            if let Some(members) = &members {
                members.restore(&moves);
            }
            return Err(err);
        }
        self.apply(table, changes);
        if let Some(members) = members.as_mut() {
            members.apply(moves);
        }
        Ok(record)
    }

    /// Save the table with `changes` applied, then apply them to memory and wake the
    /// watchers. Twin of `panes::store::Store::commit`.
    fn commit(&self, table: &mut Table, changes: Vec<ProfileEntry>) -> Result<(), PaneError> {
        self.save(table, &changes)?;
        self.apply(table, changes);
        Ok(())
    }

    /// Apply `changes`, which are saved, to memory in order, and wake the watchers. Twin of
    /// `panes::store::Store::apply`.
    fn apply(&self, table: &mut Table, changes: Vec<ProfileEntry>) {
        for change in changes {
            table.head = change.event.cursor;
            table.ring.push(change.event.clone());
            table.entries.insert(change.slug.clone(), change);
        }
        self.changed.notify_all();
    }

    /// Write the document that `table` with `changes` applied describes. A failure is
    /// logged and answered `unavailable`; the file is as it was. Twin of
    /// `panes::store::Store::save`.
    fn save(&self, table: &Table, changes: &[ProfileEntry]) -> Result<(), PaneError> {
        let doc = Doc {
            version: persist::VERSION,
            cursor: changes
                .last()
                .map_or(table.head, |change| change.event.cursor),
            entries: table.entries_with(changes),
        };
        persist::save_doc(&self.path, &doc).map_err(|problem| {
            log_fault(
                "profile_registry_write_failed",
                &self.path,
                &problem,
                WRITE_EFFECT,
            );
            PaneError::Unavailable {
                what: problem.what(LABEL, &self.path),
            }
        })
    }

    /// A watch from `since`, for the `ProfileStore` port. It is refused at once when the
    /// store has failed or `since` is ahead of the head (rule 1); otherwise each `next()`
    /// reads from [`Store::poll`]. Twin of `panes::store::Store::watch`.
    pub(crate) fn watch(
        store: &Arc<Store>,
        since: Cursor,
    ) -> Result<Watch<ProfileEvent>, PaneError> {
        let head = store.read(|table| table.head)?;
        feed::check_since(since, head)?;
        Ok(Box::new(Feed {
            store: Some(Arc::clone(store)),
            since,
            batch: VecDeque::new(),
        }))
    }

    /// One long-poll of the feed from `since`: the events owed now, or, when there are
    /// none, the first ones written within the window. When the window ends first, the
    /// batch is empty. The reply's cursor is the head when the reply is made.
    ///
    /// The wait releases the lock (`Condvar::wait_timeout`), and `feed::select` checks rule 1
    /// on every pass, so `profile/watch`, which passes the client's cursor straight here,
    /// is refused for a cursor ahead of the head. The feed's rules read the entries' events
    /// only, never their logs. Twin of `panes::store::Store::poll`.
    pub(crate) fn poll(&self, since: Cursor) -> Result<WatchReply<ProfileEvent>, PaneError> {
        let window = self.options.watch_wait;
        let deadline = Instant::now().checked_add(window);
        let mut guard = self.lock();
        loop {
            let table = match &*guard {
                Ok(table) => table,
                Err(err) => return Err(err.clone()),
            };
            let entries = table.entries.values().map(|entry| &entry.event);
            let events = feed::select(since, table.head, &table.ring, entries)?;
            let left = deadline.map_or(window, |at| at.saturating_duration_since(Instant::now()));
            if !events.is_empty() || left.is_zero() {
                return Ok(WatchReply {
                    events,
                    cursor: table.head,
                });
            }
            guard = self
                .changed
                .wait_timeout(guard, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// The `Watch` iterator: the batches of [`Store::poll`], one event at a time. It yields
/// `Ok(None)` when a window passes with nothing new, and the stream stays usable. The first
/// error ends the stream. Twin of `panes::store::Feed`.
struct Feed {
    /// `None` once an error has ended the stream.
    store: Option<Arc<Store>>,
    /// Where the next poll resumes: the cursor of the last reply.
    since: Cursor,
    /// The rest of the last batch.
    batch: VecDeque<ProfileEvent>,
}

impl Iterator for Feed {
    type Item = Result<Option<ProfileEvent>, PaneError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(event) = self.batch.pop_front() {
            return Some(Ok(Some(event)));
        }
        let store = self.store.as_ref()?;
        match store.poll(self.since) {
            Ok(reply) => {
                self.since = reply.cursor;
                self.batch = reply.events.into();
                Some(Ok(self.batch.pop_front()))
            }
            Err(err) => {
                self.store = None;
                Some(Err(err))
            }
        }
    }
}

/// The table in the file at `path`, or the failure that keeps the store closed. The failure
/// is logged here, once. Twin of `panes::store::load_table`.
fn load_table(path: &Path, retained: usize) -> Result<Table, PaneError> {
    let doc = persist::load_doc::<ProfileEntry>(path).map_err(|problem| {
        log_fault("profile_registry_corrupt", path, &problem, CORRUPT_EFFECT);
        PaneError::StoreCorrupt {
            what: problem.what(LABEL, path),
        }
    })?;
    Ok(Table::from_doc(doc, retained))
}

/// The name rule of a write (D6): the slug is a profile's identity, so a write of `name`
/// while its slug is filed under another live name is `profile-exists`, whatever the
/// generation. A display name changes only through a rename (#665). A tombstone holds no
/// name: after a delete, a create may spell the name another way, and its log goes on.
fn check_name(stored: Option<&Profile>, name: &ProfileName, slug: &str) -> Result<(), PaneError> {
    match stored {
        Some(stored) if stored.name != *name => Err(taken(name, slug, stored)),
        _ => Ok(()),
    }
}

/// `profile-exists`: `name`, of slug `slug`, cannot be written while `stored` holds the slug.
fn taken(name: &ProfileName, slug: &str, stored: &Profile) -> PaneError {
    PaneError::ProfileExists {
        what: format!(
            "{:?} has the slug {slug:?} of the stored profile {:?}",
            name.as_str(),
            stored.name.as_str()
        ),
    }
}

/// The checks of a rename of `from` to `to` at `expected` (the order of [`Store::rename`]'s
/// docs, after the loads), and, when they pass, the renamed record and the entries it files:
/// OLD's tombstone, then NEW's entry, at consecutive cursors (NEW's alone when the slug does
/// not change). Both log entries are at the new generation, by `actor`, with one stamp that
/// is never before either entry's last time (`entry::stamp`), so `at` never decreases in
/// either log. Nothing is written.
fn renamed(
    table: &Table,
    from: &ProfileName,
    to: &ProfileName,
    expected: u64,
    actor: &Actor,
) -> Result<(Profile, Vec<ProfileEntry>), PaneError> {
    let (old_slug, new_slug) = (from.slug(), to.slug());
    let moves_slug = new_slug != old_slug;
    let previous = table.entries.get(&old_slug);
    let Some(stored) = previous.and_then(ProfileEntry::profile) else {
        return Err(PaneError::ProfileNotFound {
            what: from.to_string(),
        });
    };
    if *to == stored.name {
        return Err(PaneError::Usage {
            message: format!("{:?} is the profile's name already", to.as_str()),
        });
    }
    let target = moves_slug.then(|| table.entries.get(&new_slug)).flatten();
    if let Some(other) = target.and_then(ProfileEntry::profile) {
        return Err(taken(to, &new_slug, other));
    }
    let generation = next_generation(stored.generation, expected)?;
    let now = now_millis();
    let at = entry::stamp(previous, now).max(entry::stamp(target, now));
    let record = Profile {
        name: to.clone(),
        slug: new_slug.clone(),
        generation,
        updated: at,
        ..stored.clone()
    };
    let mut cursor = table.next_cursor()?;
    let mut changes = Vec::with_capacity(2);
    let mut base = previous;
    if moves_slug {
        let event = ProfileEvent {
            cursor,
            name: stored.name.clone(),
            profile: None,
        };
        let closed = log_entry(at, generation, actor, ProfileChange::Deleted);
        changes.push(ProfileEntry::next(previous, old_slug, event, closed));
        cursor = after(cursor)?;
        base = target;
    }
    let event = ProfileEvent {
        cursor,
        name: to.clone(),
        profile: Some(Box::new(record.clone())),
    };
    let change = ProfileChange::Renamed {
        from: stored.name.clone(),
    };
    let opened = log_entry(at, generation, actor, change);
    changes.push(ProfileEntry::next(base, new_slug, event, opened));
    Ok((record, changes))
}

/// The log entry of a write by `actor`, stamped `at`, that left the profile at `generation`.
fn log_entry(at: i64, generation: u64, actor: &Actor, change: ProfileChange) -> ProfileLogEntry {
    ProfileLogEntry {
        at,
        generation,
        actor: actor.clone(),
        change,
    }
}
