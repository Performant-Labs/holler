#![allow(clippy::unwrap_used, clippy::expect_used)] // #661
#![allow(clippy::panic, clippy::unreachable)] // #661
//! Issue #661: the CAS half of the membership rule. A pane belongs to at most one
//! profile, and the refusal (`pane-in-other-profile`) runs inside the pane registry's
//! compare-and-swap, under the pane lock (ADR-0021 section 8, "Decisions taken" 2).
//!
//! These tests drive `PaneState` through the `PaneStore` port only. They never go
//! through the `pane/cas_put` hook, so they would fail if the comparison lived there.
//! The hook half (`profile-not-found`) is in `profile_handlers_test.rs`.
//!
//! Every wait is a bounded one. There is no `thread::sleep`.

mod pane_support;

use std::sync::Barrier;

use holler_hub::panes::PaneState;
use holler_pane::{Pane, PaneError, PaneStore, ProfileName};
use holler_pane_testkit::conformance::pane_store::{pane_store_cases, run_pane_store_conformance};
use pane_support::{create, dir_listing, head, load, name, registry_file, sample_pane, temp_state};

/// The pane `hj-c1r1`, stored in `profile` at generation 1.
fn stored_in(store: &PaneState, profile: &str) -> Pane {
    store
        .cas_put(&sample_pane("hj-c1r1", Some(profile)), 0)
        .unwrap()
}

// --- AC 34 ---------------------------------------------------------------------------

#[test]
fn the_hub_pane_registry_passes_the_pane_store_conformance_suite() {
    assert_eq!(pane_store_cases().len(), 19, "the suite is the 19-case one");
    let result = run_pane_store_conformance(|| {
        let (dir, state) = temp_state();
        (
            PaneState::load_with(&state, pane_support::short_opts()),
            dir,
        )
    });
    if let Err(failures) = &result {
        let ids: Vec<&str> = failures.iter().map(|f| f.case).collect();
        panic!(
            "the pane registry fails {} case(s): {ids:?}: {failures:#?}",
            ids.len()
        );
    }
}

// --- AC 35 ---------------------------------------------------------------------------

#[test]
fn moving_a_pane_to_another_profile_is_refused_inside_the_cas() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let stored = stored_in(&store, "Night Shift");
    assert_eq!(stored.generation, 1);
    let bytes = std::fs::read(registry_file(&state)).unwrap();
    let listing = dir_listing(&state.hub_dir);
    let head_before = head(&store);

    let moved = Pane {
        profile: Some(ProfileName::parse("Day Shift").unwrap()),
        ..stored.clone()
    };
    let err = store.cas_put(&moved, 1).unwrap_err();
    let PaneError::PaneInOtherProfile { what } = &err else {
        panic!("a move to another profile must be pane-in-other-profile, got {err:?}");
    };
    assert_eq!(err.code(), "pane-in-other-profile");
    assert!(
        what.contains("Night Shift") && what.contains("Day Shift"),
        "the refusal names both profiles: {what:?}"
    );
    assert_eq!(store.get(&name("hj-c1r1")).unwrap(), Some(stored.clone()));
    assert_eq!(std::fs::read(registry_file(&state)).unwrap(), bytes);
    assert_eq!(dir_listing(&state.hub_dir), listing);
    assert_eq!(head(&store), head_before, "a refused write takes no cursor");

    // Leaving first (None), then joining the other profile, is allowed.
    let left = Pane {
        profile: None,
        ..stored.clone()
    };
    assert_eq!(store.cas_put(&left, 1).unwrap().generation, 2);
    let joined = store.cas_put(&moved, 2).unwrap();
    assert_eq!(joined.generation, 3);
    assert_eq!(joined.profile.as_ref().unwrap().as_str(), "Day Shift");
}

#[test]
fn keeping_the_profile_or_respelling_it_with_the_same_slug_is_allowed() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let stored = stored_in(&store, "Night Shift");

    let mut kept = stored.clone();
    kept.context.soft += 1;
    let kept = store.cas_put(&kept, 1).unwrap();
    assert_eq!(kept.generation, 2, "keeping the profile is allowed");

    // The comparison is by slug (the profile registry's identity), not by spelling.
    let respelled = Pane {
        profile: Some(ProfileName::parse("night shift").unwrap()),
        ..kept
    };
    let respelled = store.cas_put(&respelled, 2).unwrap();
    assert_eq!(respelled.generation, 3);
    assert_eq!(respelled.profile.as_ref().unwrap().as_str(), "night shift");
}

#[test]
fn joining_a_profile_from_none_is_allowed() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let created = create(&store, "hj-c1r1");
    let joined = Pane {
        profile: Some(ProfileName::parse("Day Shift").unwrap()),
        ..created
    };
    assert_eq!(store.cas_put(&joined, 1).unwrap().generation, 2);
}

// --- AC 36 ---------------------------------------------------------------------------

#[test]
fn a_stale_generation_wins_over_the_profile_rule() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let stored = stored_in(&store, "Night Shift");
    let moved = Pane {
        profile: Some(ProfileName::parse("Day Shift").unwrap()),
        ..stored
    };
    // 0 on an existing record, and a future generation, are both stale.
    for expected in [0, 5] {
        assert_eq!(
            store.cas_put(&moved, expected).unwrap_err(),
            PaneError::Conflict,
            "expected_generation {expected}: the CAS is checked before the profile rule"
        );
    }
}

// --- AC 37 ---------------------------------------------------------------------------

#[test]
fn two_writers_joining_different_profiles_exactly_one_wins_and_the_loser_is_refused_on_retry() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let created = create(&store, "hj-c1r1");
    assert_eq!(created.generation, 1);
    let barrier = Barrier::new(2);

    let results: Vec<(&str, Result<Pane, PaneError>)> = std::thread::scope(|scope| {
        let handles: Vec<_> = ["Night Shift", "Day Shift"]
            .into_iter()
            .map(|profile| {
                let (store, barrier, created) = (&store, &barrier, &created);
                scope.spawn(move || {
                    let attempt = Pane {
                        profile: Some(ProfileName::parse(profile).unwrap()),
                        ..created.clone()
                    };
                    barrier.wait();
                    (profile, store.cas_put(&attempt, 1))
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let winners: Vec<_> = results.iter().filter(|(_, r)| r.is_ok()).collect();
    let losers: Vec<_> = results.iter().filter(|(_, r)| r.is_err()).collect();
    assert_eq!(winners.len(), 1, "exactly one writer wins: {results:?}");
    assert_eq!(losers.len(), 1);
    assert_eq!(losers[0].1, Err(PaneError::Conflict));

    // The loser re-reads and retries at the new generation: it would move the pane out
    // of the winner's profile, which is the rule the pane lock makes race-free.
    let current = store.get(&name("hj-c1r1")).unwrap().unwrap();
    assert_eq!(current.generation, 2);
    let retry = Pane {
        profile: Some(ProfileName::parse(losers[0].0).unwrap()),
        ..current.clone()
    };
    let err = store.cas_put(&retry, 2).unwrap_err();
    assert_eq!(err.code(), "pane-in-other-profile", "got {err:?}");
    assert_eq!(store.get(&name("hj-c1r1")).unwrap(), Some(current));
}
