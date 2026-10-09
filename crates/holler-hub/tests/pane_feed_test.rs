#![allow(clippy::unwrap_used, clippy::expect_used)] // #639
#![allow(clippy::panic, clippy::unreachable)] // #639
//! Issue #639: the pane registry's change feed (`PaneStore::watch`), driven from plain
//! threads with no socket (epic #633). Split from `pane_registry_test.rs` to keep both
//! files under the size gate.
//!
//! Every wait is bounded (`recv_timeout`, or the short `watch_wait` of the options). There
//! is no `thread::sleep`.

mod pane_support;

use holler_hub::panes::PaneState;
use holler_pane::{Cursor, PaneError, PaneStore};
use pane_support::{create, drain, head, load, name, temp_state};

// --- The change feed -----------------------------------------------------------------

/// The (name, generation or `None` for a delete) of each event, in order.
fn summary(events: &[holler_pane::PaneEvent]) -> Vec<(String, Option<u64>)> {
    events
        .iter()
        .map(|e| (e.name.to_string(), e.pane.as_ref().map(|p| p.generation)))
        .collect()
}

fn entry(name: &str, generation: Option<u64>) -> (String, Option<u64>) {
    (name.to_owned(), generation)
}

#[test]
fn watch_from_zero_yields_every_current_record_then_later_changes() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    for pane in ["hj-c1r1", "hj-c1r2", "hj-c1r3"] {
        create(&store, pane);
    }
    store.delete(&name("hj-c1r3"), 1).unwrap(); // cursor 4: gone, so not a current record
    let a = store.get(&name("hj-c1r1")).unwrap().unwrap();
    store.cas_put(&a, 1).unwrap(); // cursor 5: hj-c1r1 changed last

    let mut watch = store.watch(Cursor(0)).unwrap();
    let first = drain(&mut watch);
    assert_eq!(
        summary(&first),
        [entry("hj-c1r2", Some(1)), entry("hj-c1r1", Some(2))],
        "one put per live record, ordered by its last change"
    );
    assert_eq!(first.iter().map(|e| e.cursor.0).collect::<Vec<_>>(), [2, 5]);

    create(&store, "hj-c1r4");
    let later = drain(&mut watch);
    assert_eq!(
        summary(&later),
        [entry("hj-c1r4", Some(1))],
        "the stream stays usable"
    );
    assert_eq!(later[0].cursor, Cursor(6));
}

#[test]
fn the_feed_delivers_every_write_exactly_once_in_order() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    create(&store, "hj-c1r1");
    let c = head(&store);
    let mut watch = store.watch(c).unwrap();

    create(&store, "hj-c1r2");
    let a = store.get(&name("hj-c1r1")).unwrap().unwrap();
    let a2 = store.cas_put(&a, 1).unwrap();
    store.cas_put(&a2, 2).unwrap();
    store.delete(&name("hj-c1r2"), 1).unwrap();
    create(&store, "hj-c1r3");

    let events = drain(&mut watch);
    let cursors: Vec<u64> = events.iter().map(|e| e.cursor.0).collect();
    assert_eq!(cursors, (c.0 + 1..=c.0 + 5).collect::<Vec<_>>());
    assert_eq!(
        summary(&events),
        [
            entry("hj-c1r2", Some(1)),
            entry("hj-c1r1", Some(2)),
            entry("hj-c1r1", Some(3)),
            entry("hj-c1r2", None),
            entry("hj-c1r3", Some(1)),
        ]
    );
    assert!(events[3].pane.is_none(), "a delete carries no record");
    assert!(
        drain(&mut watch).is_empty(),
        "idle after the last event: Ok(None), not an end"
    );
}

#[test]
fn resuming_from_the_last_cursor_neither_repeats_nor_skips() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    for pane in ["hj-c1r1", "hj-c1r2", "hj-c1r3"] {
        create(&store, pane);
    }
    let mut first = store.watch(Cursor(0)).unwrap();
    let seen: Vec<_> = (0..2)
        .map(|_| first.next().unwrap().unwrap().unwrap())
        .collect();
    let last = seen[1].cursor;
    create(&store, "hj-c1r4");
    store.delete(&name("hj-c1r1"), 1).unwrap();

    let resumed = drain(&mut store.watch(last).unwrap());
    assert_eq!(
        summary(&resumed),
        [
            entry("hj-c1r3", Some(1)),
            entry("hj-c1r4", Some(1)),
            entry("hj-c1r1", None)
        ]
    );
    let all_cursors: Vec<u64> = seen.iter().chain(&resumed).map(|e| e.cursor.0).collect();
    assert_eq!(
        all_cursors,
        [1, 2, 3, 4, 5],
        "no gap and no repeat across the resume"
    );
}

#[test]
fn a_waiting_watch_wakes_on_the_next_write() {
    let (_dir, state) = temp_state();
    // A window far longer than the test: only a wake-up can answer in time.
    let opts = holler_hub::panes::PaneStoreOptions {
        watch_wait: std::time::Duration::from_secs(20),
        ..pane_support::short_opts()
    };
    let store = std::sync::Arc::new(PaneState::load_with(&state, opts));
    for round in 0..20_u64 {
        let watch = store.watch(head(&store)).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut watch = watch;
            let _ = tx.send(watch.next());
        });
        // Either order is fine: a write before the wait begins is returned at once.
        create(&store, &format!("hj-c1r{}", round + 1));
        let got = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the waiting watch never woke");
        let event = got
            .expect("the stream ended")
            .expect("watch failed")
            .expect("idle");
        assert_eq!(event.name.to_string(), format!("hj-c1r{}", round + 1));
    }
}

#[test]
fn a_cursor_ahead_of_the_store_is_usage() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    create(&store, "hj-c1r1");
    let h = head(&store);
    assert!(store.watch(h).is_ok(), "the head itself is a valid cursor");
    let err = store
        .watch(Cursor(h.0 + 1))
        .err()
        .expect("a cursor ahead must be refused");
    assert!(matches!(err, PaneError::Usage { .. }), "got {err:?}");
    assert_eq!(err.code(), "usage");
}

/// Seed one pane, note the cursor, then: put A, put B, put A again, delete B.
fn history_since_a_cursor(store: &PaneState) -> Cursor {
    create(store, "hj-c1r9");
    let c = head(store);
    let a = create(store, "hj-c1r1");
    create(store, "hj-c1r2");
    store.cas_put(&a, 1).unwrap();
    store.delete(&name("hj-c1r2"), 1).unwrap();
    c
}

fn assert_collapsed(events: &[holler_pane::PaneEvent], c: Cursor) {
    assert_eq!(
        summary(events),
        [entry("hj-c1r1", Some(2)), entry("hj-c1r2", None)],
        "A's latest put and B's delete, the intermediate writes collapsed"
    );
    assert_eq!(events[0].cursor, Cursor(c.0 + 3));
    assert_eq!(events[1].cursor, Cursor(c.0 + 4));
}

#[test]
fn after_a_restart_an_old_cursor_gets_each_changed_pane_once_including_deletions() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let c = history_since_a_cursor(&store);
    drop(store);

    let restarted = load(&state);
    assert_collapsed(&drain(&mut restarted.watch(c).unwrap()), c);
}

#[test]
fn a_watcher_behind_the_retained_window_gets_the_compacted_changes() {
    let (_dir, state) = temp_state();
    let opts = holler_hub::panes::PaneStoreOptions {
        feed_retained: 2,
        ..pane_support::short_opts()
    };
    let store = PaneState::load_with(&state, opts);
    let c = history_since_a_cursor(&store);
    assert_collapsed(&drain(&mut store.watch(c).unwrap()), c);
}
