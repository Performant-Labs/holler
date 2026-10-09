//! The pane registry's store (D1, D4): the table of records behind one lock, its file, and
//! the long-poll wait of its change feed.
//!
//! One `std::sync::Mutex` guards the whole table, and one `Condvar` wakes the watchers,
//! which wait without holding the lock ([`Store::poll`]). Every method is synchronous. The
//! handlers reach the store through `spawn_blocking`, and no `.await` ever happens while
//! the lock is held.
//!
//! # Writes
//!
//! A write runs entirely under the lock, in this order:
//!
//! 1. the compare-and-swap (`holler_pane::next_generation`), then, for a put, the
//!    membership rule (`refuse_profile_move`, #661);
//! 2. the next document;
//! 3. the save;
//! 4. only then, the commit to memory, the event and the wake-up.
//!
//! A failed save therefore changes nothing (memory, file and cursor), and the file always
//! holds a committed state. The write answers `unavailable`, and the same write with the
//! same generation succeeds once the file can be written again (D4). `holds.rs` keeps a
//! hold in memory when it cannot save it; a pane record that would not survive a restart
//! would defeat the registry's purpose, so this store refuses the write instead.
//!
//! # A failed load
//!
//! A file that cannot be loaded (D3, `persist.rs`) leaves the store failed for the life of
//! the process. Every method, `watch` included, answers the same `store-corrupt`, and
//! nothing is ever written, so the operator finds the file exactly as it was. The hub logs
//! one `error` event, `pane_registry_corrupt`, when it loads the file.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use holler_pane::{
    next_generation, Cursor, Pane, PaneError, PaneEvent, PaneName, Watch, WatchReply,
};
use holler_proto::log::{self, Component, Direction, Event, Severity};

use super::feed::{self, Ring};
use super::persist::{self, Doc, Problem};
use super::PaneStoreOptions;

/// How the registry names itself in the `what` of an error.
const LABEL: &str = "pane registry";

/// What a file that cannot be loaded means for the hub, as the log states it.
const CORRUPT_EFFECT: &str = "every pane method answers store-corrupt until the file is \
    repaired and the hub restarted; the file is left as it is";

/// What a failed save means, as the log states it.
const WRITE_EFFECT: &str = "the write was refused and the registry is unchanged";

/// The registry's records and change feed.
struct Table {
    /// The cursor of the last change (0: no change yet).
    head: Cursor,
    /// The last change filed under each name: the record, or a tombstone after a delete.
    entries: BTreeMap<PaneName, PaneEvent>,
    ring: Ring<PaneEvent>,
}

impl Table {
    /// The table a loaded document describes, or the empty table when there is no file.
    fn from_doc(doc: Option<Doc<PaneEvent>>, retained: usize) -> Self {
        let (head, entries) = doc.map_or((Cursor(0), Vec::new()), |doc| (doc.cursor, doc.entries));
        Self {
            head,
            entries: entries
                .into_iter()
                .map(|entry| (entry.name.clone(), entry))
                .collect(),
            ring: Ring::new(head, retained),
        }
    }

    /// The live record named `name`, if there is one.
    fn record(&self, name: &PaneName) -> Option<&Pane> {
        self.entries
            .get(name)
            .and_then(|entry| entry.pane.as_deref())
    }

    /// The cursor the next change takes.
    fn next_cursor(&self) -> Result<Cursor, PaneError> {
        self.head
            .0
            .checked_add(1)
            .map(Cursor)
            .ok_or_else(|| PaneError::StoreCorrupt {
                what: format!("{LABEL}: the change cursor overflowed"),
            })
    }

    /// The entries of the next document: every entry, with `change` filed under its name,
    /// in name order.
    fn entries_with<'a>(&'a self, change: &'a PaneEvent) -> Vec<&'a PaneEvent> {
        let mut entries: Vec<&PaneEvent> = self
            .entries
            .values()
            .filter(|entry| entry.name != change.name)
            .collect();
        let at = entries.partition_point(|entry| entry.name < change.name);
        entries.insert(at, change);
        entries
    }
}

/// The pane registry's store. `PaneState` holds it behind an `Arc`, which the handlers and
/// every `Watch` iterator share.
pub(crate) struct Store {
    path: PathBuf,
    options: PaneStoreOptions,
    /// The table, or the failure that keeps the store closed (D3).
    table: Mutex<Result<Table, PaneError>>,
    /// Notified after every committed write, for the waiting watchers.
    changed: Condvar,
}

impl Store {
    /// Load the registry from the file at `path`. A file that cannot be loaded leaves the
    /// store failed (see the module docs); this never fails itself.
    pub(crate) fn open(path: PathBuf, options: PaneStoreOptions) -> Self {
        let table = load_table(&path, options.feed_retained);
        Self {
            path,
            options,
            table: Mutex::new(table),
            changed: Condvar::new(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Result<Table, PaneError>> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `view` applied to the table, or the store's failure.
    fn read<T>(&self, view: impl FnOnce(&Table) -> T) -> Result<T, PaneError> {
        match &*self.lock() {
            Ok(table) => Ok(view(table)),
            Err(err) => Err(err.clone()),
        }
    }

    /// The pane named `name`, or `None`.
    pub(crate) fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        self.read(|table| table.record(name).cloned())
    }

    /// Every live pane, sorted by name.
    pub(crate) fn list(&self) -> Result<Vec<Pane>, PaneError> {
        self.read(|table| {
            table
                .entries
                .values()
                .filter_map(|entry| entry.pane.as_deref().cloned())
                .collect()
        })
    }

    /// Store `pane` if the stored record is still at `expected` (0 for a new pane). The
    /// stored record is `pane` with only its generation replaced: the submitted generation
    /// is ignored and nothing else is inferred. Returns the stored record.
    ///
    /// The membership rule runs here, after the generation and under the lock (#661, see
    /// [`refuse_profile_move`]), so it holds on every path into the registry, the port and
    /// `pane/cas_put` alike, and no writer can slip in between the check and the write.
    pub(crate) fn cas_put(&self, pane: &Pane, expected: u64) -> Result<Pane, PaneError> {
        let mut guard = self.lock();
        let table = guard.as_mut().map_err(|err| err.clone())?;
        let current = table.record(&pane.name);
        let generation = next_generation(current.map_or(0, |stored| stored.generation), expected)?;
        refuse_profile_move(current, pane)?;
        let stored = Pane {
            generation,
            ..pane.clone()
        };
        let change = PaneEvent {
            cursor: table.next_cursor()?,
            name: pane.name.clone(),
            pane: Some(Box::new(stored.clone())),
        };
        self.commit(table, change)?;
        Ok(stored)
    }

    /// Remove the record named `name` if it is still at `expected`. A missing record is
    /// `pane-not-found` whatever `expected` is: that is checked before the generation.
    pub(crate) fn delete(&self, name: &PaneName, expected: u64) -> Result<(), PaneError> {
        let mut guard = self.lock();
        let table = guard.as_mut().map_err(|err| err.clone())?;
        let Some(current) = table.record(name).map(|stored| stored.generation) else {
            return Err(PaneError::PaneNotFound {
                what: name.to_string(),
            });
        };
        next_generation(current, expected)?;
        let change = PaneEvent {
            cursor: table.next_cursor()?,
            name: name.clone(),
            pane: None,
        };
        self.commit(table, change)
    }

    /// Save the table with `change` applied, then apply it to memory and wake the watchers.
    fn commit(&self, table: &mut Table, change: PaneEvent) -> Result<(), PaneError> {
        self.save(table, &change)?;
        table.head = change.cursor;
        table.ring.push(change.clone());
        table.entries.insert(change.name.clone(), change);
        self.changed.notify_all();
        Ok(())
    }

    /// Write the document that `table` with `change` applied describes. A failure is
    /// logged and answered `unavailable`; the file is as it was.
    fn save(&self, table: &Table, change: &PaneEvent) -> Result<(), PaneError> {
        let doc = Doc {
            version: persist::VERSION,
            cursor: change.cursor,
            entries: table.entries_with(change),
        };
        persist::save_doc(&self.path, &doc).map_err(|problem| {
            log_fault(
                "pane_registry_write_failed",
                &self.path,
                &problem,
                WRITE_EFFECT,
            );
            PaneError::Unavailable {
                what: problem.what(LABEL, &self.path),
            }
        })
    }

    /// A watch from `since`, for the `PaneStore` port. It is refused at once when the store
    /// has failed or `since` is ahead of the head (rule 1); otherwise each `next()` reads
    /// from [`Store::poll`].
    pub(crate) fn watch(store: &Arc<Store>, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        let head = store.read(|table| table.head)?;
        feed::check_since(since, head)?;
        Ok(Box::new(Feed {
            store: Some(Arc::clone(store)),
            since,
            batch: VecDeque::new(),
        }))
    }

    /// One long-poll of the feed from `since` (D6): the events owed now, or, when there are
    /// none, the first ones written within the window. When the window ends first, the
    /// batch is empty. The reply's cursor is the head when the reply is made.
    ///
    /// The wait does not hold the lock: `Condvar::wait_timeout` releases it while the thread
    /// waits and takes it back before it returns. Every other method, and every other
    /// watcher, runs during the window. `a_waiting_watch_wakes_on_the_next_write`
    /// (`tests/pane_feed_test.rs`) writes while a watcher waits out a window far longer than
    /// the test.
    ///
    /// Rule 1 (a cursor ahead of the head is `usage`) is checked by `feed::select` on every
    /// pass. `pane/watch` passes the client's cursor straight here, so for that method this
    /// is the only check. A cursor that passed it once never fails it later, because the
    /// head only grows: each write takes `head + 1`.
    pub(crate) fn poll(&self, since: Cursor) -> Result<WatchReply<PaneEvent>, PaneError> {
        let window = self.options.watch_wait;
        let deadline = Instant::now().checked_add(window);
        let mut guard = self.lock();
        loop {
            let table = match &*guard {
                Ok(table) => table,
                Err(err) => return Err(err.clone()),
            };
            let events = feed::select(since, table.head, &table.ring, table.entries.values())?;
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
/// error ends the stream.
struct Feed {
    /// `None` once an error has ended the stream.
    store: Option<Arc<Store>>,
    /// Where the next poll resumes: the cursor of the last reply.
    since: Cursor,
    /// The rest of the last batch.
    batch: VecDeque<PaneEvent>,
}

impl Iterator for Feed {
    type Item = Result<Option<PaneEvent>, PaneError>;

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
/// is logged here, once.
fn load_table(path: &Path, retained: usize) -> Result<Table, PaneError> {
    let doc = persist::load_doc::<PaneEvent>(path).map_err(|problem| {
        log_fault("pane_registry_corrupt", path, &problem, CORRUPT_EFFECT);
        PaneError::StoreCorrupt {
            what: problem.what(LABEL, path),
        }
    })?;
    Ok(Table::from_doc(doc, retained))
}

/// The membership rule of a write (#661; ADR-0021 §8 and "Decisions taken", item 2): a
/// pane stored in one profile cannot be written into another, `pane-in-other-profile`.
/// Leaving (`None`), joining from `None` and keeping the profile are allowed, so a move
/// takes two writes, leave and then join. Profiles are compared by slug, the profile
/// registry's identity, so another spelling of the same profile keeps it. Whether the
/// profile exists is the `pane/cas_put` hook's check (`crate::profile::check_membership`).
fn refuse_profile_move(stored: Option<&Pane>, next: &Pane) -> Result<(), PaneError> {
    let current = stored.and_then(|pane| pane.profile.as_ref());
    match (current, next.profile.as_ref()) {
        (Some(current), Some(other)) if current.slug() != other.slug() => {
            Err(PaneError::PaneInOtherProfile {
                what: format!(
                    "{} is in profile {:?}, not {:?}",
                    next.name,
                    current.as_str(),
                    other.as_str()
                ),
            })
        }
        _ => Ok(()),
    }
}

/// Log one of a registry's two faults: `<registry>_corrupt` when its file cannot be loaded,
/// and `<registry>_write_failed` when a write cannot be saved. The pane registry logs
/// `pane_registry_corrupt` and `pane_registry_write_failed`, and the profile registry (#661)
/// `profile_registry_corrupt` and `profile_registry_write_failed`. Each is a fault on the
/// hub's own side, so each is an `error` event. The events share every field but the event
/// name and the effect, so they share this one helper.
pub(crate) fn log_fault(method: &'static str, path: &Path, problem: &Problem, effect: &str) {
    log::emit(&Event {
        component: Component::Control,
        severity: Severity::Error,
        direction: Direction::Local,
        method,
        id: None,
        peer: None,
        fields: vec![
            ("path", path.display().to_string()),
            ("problem", problem.to_string()),
            ("effect", effect.to_owned()),
        ],
        frame: None,
    });
}
