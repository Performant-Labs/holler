#![allow(clippy::unwrap_used, clippy::expect_used)] // #661
#![allow(clippy::panic, clippy::unreachable)] // #661
//! Issue #661: the profile registry's change feed (`ProfileStore::watch`), driven from
//! plain threads with no socket (epic #633). The twin of `pane_feed_test.rs`: the feed
//! rules are #639's (`panes::feed`), so these tests pin that the profile registry feeds
//! them the right events, including a delete's stored display name.
//!
//! Every wait is bounded (`recv_timeout`, or the short `watch_wait` of the options). There
//! is no `thread::sleep`.

mod pane_support;

use holler_hub::panes::PaneStoreOptions;
use holler_hub::profile::ProfileState;
use holler_pane::{Cursor, PaneError, ProfileEvent, ProfileName, ProfileStore};
use pane_support::{
    actor, create_profile, drain, load_profiles, profile_head, short_opts, temp_state,
};

fn pname(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

/// The (display name, generation or `None` for a delete) of each event, in order.
fn summary(events: &[ProfileEvent]) -> Vec<(String, Option<u64>)> {
    events
        .iter()
        .map(|e| (e.name.to_string(), e.profile.as_ref().map(|p| p.generation)))
        .collect()
}

fn entry(name: &str, generation: Option<u64>) -> (String, Option<u64>) {
    (name.to_owned(), generation)
}

/// Update `name` at `expected` (its spec's soft ceiling moves), returning the stored record.
fn touch(store: &ProfileState, name: &str, expected: u64) -> holler_pane::Profile {
    let mut p = store.get(&pname(name)).unwrap().unwrap();
    p.panes[0].context.soft += 1;
    store.cas_put(&p, expected, &actor()).unwrap()
}

#[test]
fn the_feed_delivers_every_write_exactly_once_in_order() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    create_profile(&store, "Night Shift");
    let c = profile_head(&store);
    let mut watch = store.watch(c).unwrap();

    create_profile(&store, "Day Shift"); // c+1
    touch(&store, "Night Shift", 1); // c+2
    touch(&store, "Night Shift", 2); // c+3
    store.delete(&pname("DAY SHIFT"), 1, &actor()).unwrap(); // c+4, another spelling
    create_profile(&store, "Day Shift"); // c+5, a re-create

    let events = drain(&mut watch);
    let cursors: Vec<u64> = events.iter().map(|e| e.cursor.0).collect();
    assert_eq!(cursors, (c.0 + 1..=c.0 + 5).collect::<Vec<_>>());
    assert_eq!(
        summary(&events),
        [
            entry("Day Shift", Some(1)),
            entry("Night Shift", Some(2)),
            entry("Night Shift", Some(3)),
            entry("Day Shift", None),
            entry("Day Shift", Some(1)),
        ]
    );
    assert!(events[3].profile.is_none(), "a delete carries no record");
    assert!(
        drain(&mut watch).is_empty(),
        "idle after the last event: Ok(None), not an end"
    );
}

#[test]
fn watch_from_zero_yields_every_current_profile_then_later_changes() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    for name in ["Alpha", "Beta", "Gamma"] {
        create_profile(&store, name);
    }
    store.delete(&pname("Gamma"), 1, &actor()).unwrap(); // cursor 4: gone, not a current profile
    touch(&store, "Alpha", 1); // cursor 5: Alpha changed last

    let mut watch = store.watch(Cursor(0)).unwrap();
    let first = drain(&mut watch);
    assert_eq!(
        summary(&first),
        [entry("Beta", Some(1)), entry("Alpha", Some(2))],
        "one put per live profile, ordered by its last change"
    );
    assert_eq!(first.iter().map(|e| e.cursor.0).collect::<Vec<_>>(), [2, 5]);

    create_profile(&store, "Delta");
    let later = drain(&mut watch);
    assert_eq!(
        summary(&later),
        [entry("Delta", Some(1))],
        "the stream stays usable"
    );
    assert_eq!(later[0].cursor, Cursor(6));
}

#[test]
fn a_waiting_watch_wakes_on_the_next_write() {
    let (_dir, state) = temp_state();
    // A window far longer than the test: only a wake-up can answer in time.
    let opts = PaneStoreOptions {
        watch_wait: std::time::Duration::from_secs(20),
        ..short_opts()
    };
    let store = std::sync::Arc::new(ProfileState::load_with(&state, opts));
    for round in 0..20_u64 {
        let watch = store.watch(profile_head(&store)).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut watch = watch;
            let _ = tx.send(watch.next());
        });
        // Either order is fine: a write before the wait begins is returned at once.
        create_profile(&store, &format!("Shift {round}"));
        let got = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the waiting watch never woke");
        let event = got
            .expect("the stream ended")
            .expect("watch failed")
            .expect("idle");
        assert_eq!(event.name.to_string(), format!("Shift {round}"));
    }
}

#[test]
fn a_cursor_ahead_of_the_store_is_usage() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    create_profile(&store, "Night Shift");
    let h = profile_head(&store);
    assert!(store.watch(h).is_ok(), "the head itself is a valid cursor");
    let err = store
        .watch(Cursor(h.0 + 1))
        .err()
        .expect("a cursor ahead must be refused");
    assert!(matches!(err, PaneError::Usage { .. }), "got {err:?}");
    assert_eq!(err.code(), "usage");
}

/// Seed one profile, note the cursor, then: put A, put B, put A again, delete B.
fn history_since_a_cursor(store: &ProfileState) -> Cursor {
    create_profile(store, "Seed");
    let c = profile_head(store);
    create_profile(store, "Alpha");
    create_profile(store, "Beta");
    touch(store, "Alpha", 1);
    store.delete(&pname("Beta"), 1, &actor()).unwrap();
    c
}

fn assert_collapsed(events: &[ProfileEvent], c: Cursor) {
    assert_eq!(
        summary(events),
        [entry("Alpha", Some(2)), entry("Beta", None)],
        "A's latest put and B's delete, the intermediate writes collapsed"
    );
    assert_eq!(events[0].cursor, Cursor(c.0 + 3));
    assert_eq!(events[1].cursor, Cursor(c.0 + 4));
}

#[test]
fn after_a_restart_an_old_cursor_gets_each_changed_profile_once_including_deletions() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let c = history_since_a_cursor(&store);
    drop(store);

    let restarted = load_profiles(&state);
    assert_collapsed(&drain(&mut restarted.watch(c).unwrap()), c);
}

#[test]
fn a_watcher_behind_the_retained_window_gets_the_compacted_changes() {
    let (_dir, state) = temp_state();
    let opts = PaneStoreOptions {
        feed_retained: 2,
        ..short_opts()
    };
    let store = ProfileState::load_with(&state, opts);
    let c = history_since_a_cursor(&store);
    assert_collapsed(&drain(&mut store.watch(c).unwrap()), c);
}
