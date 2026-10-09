//! The registry's change feed (D5, D6): the ring of recent events, and the rule that picks
//! what a watcher at a cursor is owed.
//!
//! This file is pure: no lock, no wait and no I/O. `store.rs` calls [`select`] under the
//! registry's lock and does the waiting itself (`Store::poll`), so the dependency runs one
//! way, from the store to the feed. Both items are generic over the event type, so #661's
//! profile feed reuses them.
//!
//! # Cursors
//!
//! Every put and every delete takes the next cursor, `head + 1`. The head is saved in the
//! registry file, so a cursor never goes backwards across a restart. The [`Ring`] keeps the
//! last `feed_retained` events exactly as they were written. Its **floor** is the cursor of
//! the newest event it no longer holds, so every event after the floor is in the ring. After
//! a restart the ring starts empty, with its floor at the loaded head.
//!
//! # What a watcher is owed (D6)
//!
//! [`select`] applies these rules under the lock. The store applies them again after each
//! wake-up, until there are events or the long-poll window ends:
//!
//! 1. If `since` is ahead of the head: `usage`.
//! 2. If `since` is 0: a put for each live record, ordered by the cursor of its last change
//!    (the port's "the current state first").
//! 3. If `since` is at or after the ring's floor: the ring's events after `since`. Every
//!    write is delivered once, in order.
//! 4. Otherwise (after a restart, or when the watcher has fallen behind the ring): one event
//!    per entry whose cursor is greater than `since`, ordered by cursor. This is the latest
//!    state of each changed pane, and a **tombstone** yields a delete event. Intermediate
//!    writes are collapsed. A watcher that keeps up never reaches this case.
//!
//! The reply's cursor is always the head, read when the reply is made. Resuming from it
//! neither repeats nor skips a change.
//!
//! Rule 4 refines the `Watch` contract in `holler_pane::ports`, which says a watch "yields
//! every change after `since`". For a watcher the ring no longer covers, it delivers every
//! final state and every deletion, but not every intermediate write. ADR-0021 §7 (#634,
//! accepted) ratifies this refinement.

use std::collections::VecDeque;

use holler_pane::{Cursor, PaneError};

use super::RegistryEntry;

/// The last events of a registry's feed, oldest first, at most `capacity` of them.
pub(crate) struct Ring<E> {
    events: VecDeque<E>,
    /// Every event after this cursor is in the ring.
    floor: Cursor,
    capacity: usize,
}

impl<E: RegistryEntry> Ring<E> {
    /// An empty ring, at most `capacity` long, whose floor is `head`: the registry's head
    /// when it was loaded.
    pub(crate) fn new(head: Cursor, capacity: usize) -> Self {
        Self {
            events: VecDeque::new(),
            floor: head,
            capacity,
        }
    }

    /// Append the newest event, and drop the oldest ones beyond the capacity.
    pub(crate) fn push(&mut self, event: E) {
        self.events.push_back(event);
        while self.events.len() > self.capacity {
            let Some(oldest) = self.events.pop_front() else {
                break;
            };
            self.floor = oldest.cursor();
        }
    }

    /// The events after `since`, which is at or after the floor.
    fn after(&self, since: Cursor) -> Vec<E> {
        self.events
            .iter()
            .filter(|event| event.cursor() > since)
            .cloned()
            .collect()
    }
}

/// Rule 1: refuse a cursor ahead of the head, because it names a change that has not
/// happened.
pub(crate) fn check_since(since: Cursor, head: Cursor) -> Result<(), PaneError> {
    if since > head {
        return Err(PaneError::Usage {
            message: format!(
                "watch cursor {} is ahead of the registry's last change ({})",
                since.0, head.0
            ),
        });
    }
    Ok(())
}

/// The events a watcher at `since` is owed now (rules 1-4), oldest first. The result is
/// empty when the watcher is up to date. `entries` is every entry of the registry,
/// tombstones included.
pub(crate) fn select<'a, E>(
    since: Cursor,
    head: Cursor,
    ring: &Ring<E>,
    entries: impl Iterator<Item = &'a E>,
) -> Result<Vec<E>, PaneError>
where
    E: RegistryEntry + 'a,
{
    check_since(since, head)?;
    Ok(if since == Cursor(0) {
        by_cursor(entries.filter(|entry| entry.record().is_some()))
    } else if since >= ring.floor {
        ring.after(since)
    } else {
        by_cursor(entries.filter(|entry| entry.cursor() > since))
    })
}

/// `entries`, ordered by cursor and cloned.
fn by_cursor<'a, E: RegistryEntry + 'a>(entries: impl Iterator<Item = &'a E>) -> Vec<E> {
    let mut picked: Vec<&E> = entries.collect();
    picked.sort_unstable_by_key(|entry| entry.cursor());
    picked.into_iter().cloned().collect()
}
