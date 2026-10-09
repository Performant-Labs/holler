//! The change feed every fake store of this crate shares (crate-private): the store's
//! records and its change log behind one lock, the cursor each change takes, what a
//! watcher at a cursor is owed, the idle wait and the `Watch` iterator.
//!
//! It is generic over the event a store publishes ([`Change`]), and nothing here
//! depends on the record type. `pane_store.rs` implements the trait for `PaneEvent`,
//! and `profile_store.rs` implements it for `ProfileEvent`, so the crate has one feed.
//! The two fake stores also share the write-side helpers kept here: [`Writer`], who
//! makes a write (a caller of the port or another writer), and [`lock`], which takes
//! over a poisoned lock.
//!
//! # Cursors
//!
//! Every change takes the next cursor, `head + 1`. An overflow is `store-corrupt` and
//! changes nothing. Unlike the hub's registry, the feed keeps its whole history, so it
//! never collapses writes.
//!
//! # What a watcher at `since` is owed
//!
//! 1. A cursor ahead of the head is `usage`, refused when the watch opens (the hub's
//!    rule). A cursor that passed once never fails later, because the head only grows.
//! 2. From `Cursor(0)`: one put for each live record, carrying the cursor of that
//!    record's last change, in cursor order. The stream then resumes from the head as
//!    of that snapshot, as the hub's does, so a record deleted before the snapshot
//!    leaves no event at all.
//! 3. From any other cursor: every change after it, in order.
//!
//! Each `next()` first calls the port's fault hook (the store's `watch_next` op), so a
//! fault set while the stream is open reaches it. It then yields the next change owed.
//! When none is owed, it waits up to the idle wait for a write and yields `Ok(None)`
//! (idle) if none came; the stream stays usable. The first error ends the stream.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use holler_pane::{Cursor, PaneError, Watch};

use crate::fault::{FaultSwitch, PortOp};

/// One change a store publishes: the put or the delete of one record.
pub(crate) trait Change: Clone + Send + 'static {
    /// What a record is filed under, e.g. a pane's name.
    type Key: Ord + Send + 'static;
    /// The record a put carries.
    type Record;

    /// The key of the record the change is about.
    fn key(&self) -> Self::Key;

    /// The change's cursor.
    fn cursor(&self) -> Cursor;

    /// The record after the change, or `None` for a delete.
    fn record(&self) -> Option<&Self::Record>;
}

/// A store's records and its change log. The [`Feed`]'s lock guards it.
pub(crate) struct Log<E: Change> {
    /// The cursor of the last change, or `Cursor(0)` before the first.
    head: Cursor,
    /// Every change, oldest first.
    history: Vec<E>,
    /// The last change of each live record, by key. It is always a put, because a
    /// delete removes the record's entry.
    live: BTreeMap<E::Key, E>,
}

impl<E: Change> Log<E> {
    /// The live record filed under `key`, if any.
    pub(crate) fn get(&self, key: &E::Key) -> Option<&E::Record> {
        self.live.get(key).and_then(|change| change.record())
    }

    /// Every live record, in key order.
    pub(crate) fn records(&self) -> impl Iterator<Item = &E::Record> {
        self.live.values().filter_map(|change| change.record())
    }

    /// Publish the change that `make` builds from the next cursor: add it to the
    /// history, and file or remove its record. A cursor overflow is `store-corrupt`
    /// and changes nothing.
    pub(crate) fn append(&mut self, make: impl FnOnce(Cursor) -> E) -> Result<(), PaneError> {
        let overflowed = || PaneError::StoreCorrupt {
            what: "the change cursor overflowed".to_owned(),
        };
        let cursor = Cursor(self.head.0.checked_add(1).ok_or_else(overflowed)?);
        let change = make(cursor);
        if change.record().is_some() {
            self.live.insert(change.key(), change.clone());
        } else {
            self.live.remove(&change.key());
        }
        self.history.push(change);
        self.head = cursor;
        Ok(())
    }

    /// Rule 1: refuse a cursor ahead of the head, which names a change that has not
    /// happened.
    fn check_since(&self, since: Cursor) -> Result<(), PaneError> {
        if since > self.head {
            return Err(PaneError::Usage {
                message: format!(
                    "watch cursor {} is ahead of the store's last change ({})",
                    since.0, self.head.0
                ),
            });
        }
        Ok(())
    }

    /// Rules 2 and 3: what a watcher at `since` is owed now, oldest first, and the
    /// cursor it resumes from (the head).
    fn owed(&self, since: Cursor) -> (Vec<E>, Cursor) {
        let owed = if since == Cursor(0) {
            let mut current: Vec<E> = self.live.values().cloned().collect();
            current.sort_by_key(E::cursor);
            current
        } else {
            self.history
                .iter()
                .skip_while(|change| change.cursor() <= since)
                .cloned()
                .collect()
        };
        (owed, self.head)
    }
}

/// A store's [`Log`] behind one lock, the condition variable that wakes its watchers,
/// and how long an idle `next()` waits for a write.
pub(crate) struct Feed<E: Change> {
    log: Mutex<Log<E>>,
    /// Notified after every change.
    changed: Condvar,
    idle_wait: Mutex<Duration>,
}

impl<E: Change> Feed<E> {
    /// An empty store whose idle wait is zero.
    pub(crate) fn new() -> Self {
        Self {
            log: Mutex::new(Log {
                head: Cursor(0),
                history: Vec::new(),
                live: BTreeMap::new(),
            }),
            changed: Condvar::new(),
            idle_wait: Mutex::new(Duration::ZERO),
        }
    }

    /// How long a `next()` that is owed nothing waits for a write before it yields
    /// `Ok(None)`.
    pub(crate) fn set_idle_wait(&self, wait: Duration) {
        *lock(&self.idle_wait) = wait;
    }

    /// `view` applied to the log, under the lock.
    pub(crate) fn read<T>(&self, view: impl FnOnce(&Log<E>) -> T) -> T {
        view(&lock(&self.log))
    }

    /// `change` applied to the log, under the lock. When it succeeds, the watchers
    /// are woken.
    pub(crate) fn write<T>(
        &self,
        change: impl FnOnce(&mut Log<E>) -> Result<T, PaneError>,
    ) -> Result<T, PaneError> {
        let result = change(&mut lock(&self.log));
        if result.is_ok() {
            self.changed.notify_all();
        }
        result
    }

    /// A watch of `feed` from `since`, refused at once when `since` is ahead of the
    /// head (rule 1). Each `next()` first calls `faults.enter(next_op)`.
    pub(crate) fn watch<Op: PortOp>(
        feed: &Arc<Self>,
        since: Cursor,
        faults: &Arc<FaultSwitch<Op>>,
        next_op: Op,
    ) -> Result<Watch<E>, PaneError> {
        feed.read(|log| log.check_since(since))?;
        Ok(Box::new(Stream {
            feed: Some(Arc::clone(feed)),
            faults: Arc::clone(faults),
            next_op,
            since,
            owed: VecDeque::new(),
        }))
    }

    /// One poll from `since`: what is owed now, or else what the first write within
    /// the idle wait brings, with the cursor to resume from. The changes are empty when
    /// the wait ends first. The wait releases the lock, so writers proceed during it.
    fn poll(&self, since: Cursor) -> (Vec<E>, Cursor) {
        let wait = *lock(&self.idle_wait);
        let deadline = Instant::now().checked_add(wait);
        let mut log = lock(&self.log);
        loop {
            let (owed, resume) = log.owed(since);
            let left = deadline.map_or(wait, |at| at.saturating_duration_since(Instant::now()));
            if !owed.is_empty() || left.is_zero() {
                return (owed, resume);
            }
            log = self
                .changed
                .wait_timeout(log, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

/// Who makes a write to a fake store.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Writer {
    /// A caller of the port, or a seed: a compare-and-swap at the generation it read,
    /// and the store's port-only rule (membership for panes, the name rule for
    /// profiles).
    Port(u64),
    /// Another writer (`concurrent_*`): applied to whatever is stored now, without the
    /// store's port-only rule.
    Other,
}

impl Writer {
    /// The generation the write expects, the record being at `current` now.
    pub(crate) fn expected(self, current: u64) -> u64 {
        match self {
            Writer::Port(expected) => expected,
            Writer::Other => current,
        }
    }
}

/// `mutex`, locked. A poisoned lock is taken over: a test that panicked while holding
/// it must not wedge every later call. A fake store locks its own state beside the
/// feed with it too.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The `Watch` iterator over a [`Feed`].
struct Stream<E: Change, Op: PortOp> {
    /// `None` once an error has ended the stream.
    feed: Option<Arc<Feed<E>>>,
    faults: Arc<FaultSwitch<Op>>,
    /// The op of one `next()`, for the fault hook and the call log.
    next_op: Op,
    /// Where the next poll resumes.
    since: Cursor,
    /// The rest of the last poll's changes.
    owed: VecDeque<E>,
}

impl<E: Change, Op: PortOp> Stream<E, Op> {
    /// One `next()` of an open stream: the fault hook, then the next change owed,
    /// polling the feed when none is left.
    fn step(&mut self, feed: &Feed<E>) -> Result<Option<E>, PaneError> {
        self.faults.enter(self.next_op)?;
        if self.owed.is_empty() {
            let (owed, resume) = feed.poll(self.since);
            self.owed = owed.into();
            self.since = resume;
        }
        Ok(self.owed.pop_front())
    }
}

impl<E: Change, Op: PortOp> Iterator for Stream<E, Op> {
    type Item = Result<Option<E>, PaneError>;

    fn next(&mut self) -> Option<Self::Item> {
        let feed = self.feed.take()?;
        let item = self.step(&feed);
        if item.is_ok() {
            self.feed = Some(feed);
        }
        Some(item)
    }
}
