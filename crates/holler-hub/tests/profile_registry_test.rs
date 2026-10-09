#![allow(clippy::unwrap_used, clippy::expect_used)] // #661
#![allow(clippy::panic, clippy::unreachable)] // #661
//! Issue #661: the hub's profile registry, driven through the `ProfileStore` port from
//! plain threads, with no socket (epic #633). These are the store tests: the conformance
//! suite and the rules the test kit fixes, compare-and-swap, the change log and
//! concurrency. Persistence and fail-closed loading are in `profile_persistence_test.rs`,
//! the change feed in `profile_feed_test.rs`, the wire handlers in
//! `profile_handlers_test.rs` and the membership rule's pane half in
//! `pane_membership_test.rs`.
//!
//! Every wait is a bounded one. There is no `thread::sleep`: a race is asserted as an
//! invariant ("exactly one wins", "no update is lost"), never as a duration.

mod pane_support;

use std::sync::{Barrier, Mutex};

use holler_hub::profile::ProfileState;
use holler_pane::{Actor, PaneError, ProfileChange, ProfileLogEntry, ProfileName, ProfileStore};
use holler_pane_testkit::conformance::profile_store::{
    profile_store_cases, run_profile_store_conformance,
};
use holler_proto::clock::now_millis;
use pane_support::{
    actor, create_profile, load_profiles, profile, profile_head, profiles_doc, profiles_file,
    temp_state,
};
use serde_json::json;

fn pname(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

fn who(text: &str) -> Actor {
    Actor::parse(text).unwrap()
}

/// The (generation, kind) of each log entry, oldest first.
fn shape(log: &[ProfileLogEntry]) -> Vec<(u64, &'static str)> {
    log.iter()
        .map(|e| {
            let kind = match e.change {
                ProfileChange::Created => "created",
                ProfileChange::Updated { .. } => "updated",
                ProfileChange::Renamed { .. } => "renamed",
                ProfileChange::Deleted => "deleted",
            };
            (e.generation, kind)
        })
        .collect()
}

// --- The conformance suite and the rules the test kit fixes (AC 1-6) ------------------

#[test]
fn the_registry_passes_the_profile_store_conformance_suite() {
    assert_eq!(
        profile_store_cases().len(),
        23,
        "the suite is the 23-case one"
    );
    let result = run_profile_store_conformance(|| {
        let (dir, state) = temp_state();
        (load_profiles(&state), dir)
    });
    if let Err(failures) = &result {
        let ids: Vec<&str> = failures.iter().map(|f| f.case).collect();
        panic!("the profile registry fails case(s) {ids:?}: {failures:#?}");
    }
}

#[test]
fn profiles_are_filed_by_slug_and_the_submitted_slug_is_ignored() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let mut submitted = profile("Night Shift", &["hj-c1r1"]);
    submitted.slug = "wrong".to_owned();
    let stored = store.cas_put(&submitted, 0, &actor()).unwrap();
    assert_eq!(stored.slug, "night-shift", "the store derives the slug");
    assert_eq!(stored.generation, 1);

    let other_spelling = store.get(&pname("NIGHT-SHIFT")).unwrap();
    assert_eq!(other_spelling, Some(stored), "lookups go by slug");
    let doc = profiles_doc(&state);
    let entries = doc["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["slug"], json!("night-shift"));
}

#[test]
fn the_name_rule_runs_before_the_generation() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let night = create_profile(&store, "Night Shift");
    let bytes = std::fs::read(profiles_file(&state)).unwrap();
    let log = store.log(&night.name).unwrap();
    let head = profile_head(&store);

    for expected in [0, 1] {
        let err = store
            .cas_put(&profile("NIGHT-SHIFT", &["hj-c1r1"]), expected, &actor())
            .unwrap_err();
        assert!(
            matches!(err, PaneError::ProfileExists { .. }),
            "expected_generation {expected}: got {err:?}"
        );
        assert_eq!(err.code(), "profile-exists");
    }
    assert_eq!(store.get(&night.name).unwrap(), Some(night.clone()));
    assert_eq!(store.log(&night.name).unwrap(), log);
    assert_eq!(std::fs::read(profiles_file(&state)).unwrap(), bytes);
    assert_eq!(profile_head(&store), head);
}

#[test]
fn a_delete_is_logged_at_the_deleted_generation_plus_one() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let created = store
        .cas_put(&profile("Night Shift", &["hj-c1r1"]), 0, &who("alice"))
        .unwrap();
    let mut next = created.clone();
    next.panes[0].context.soft += 1;
    let updated = store.cas_put(&next, 1, &who("bob")).unwrap();
    assert_eq!(updated.generation, 2);
    store.delete(&updated.name, 2, &who("carol")).unwrap();

    let log = store.log(&created.name).unwrap();
    assert_eq!(
        shape(&log),
        [(1, "created"), (2, "updated"), (3, "deleted")]
    );
    let actors: Vec<&str> = log.iter().map(|e| e.actor.as_str()).collect();
    assert_eq!(
        actors,
        ["alice", "bob", "carol"],
        "each entry names its writer"
    );
}

#[test]
fn the_log_survives_delete_and_recreate_and_a_restart() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let first = create_profile(&store, "Night Shift");
    store.delete(&first.name, 1, &actor()).unwrap();
    let second = create_profile(&store, "Night Shift");
    assert_eq!(second.generation, 1, "a re-create starts again at 1");
    let expected = [(1, "created"), (2, "deleted"), (1, "created")];
    assert_eq!(shape(&store.log(&first.name).unwrap()), expected);
    drop(store);

    let restarted = load_profiles(&state);
    assert_eq!(shape(&restarted.log(&first.name).unwrap()), expected);
    assert_eq!(restarted.get(&first.name).unwrap(), Some(second));

    // A name never created has no log, also after a refused create at a stale generation.
    let never = pname("Never Made");
    let err = restarted.log(&never).unwrap_err();
    assert!(matches!(err, PaneError::ProfileNotFound { .. }), "{err:?}");
    let refused = restarted
        .cas_put(&profile("Never Made", &[]), 5, &actor())
        .unwrap_err();
    assert_eq!(refused, PaneError::Conflict);
    let err = restarted.log(&never).unwrap_err();
    assert!(matches!(err, PaneError::ProfileNotFound { .. }), "{err:?}");
}

#[test]
fn the_hub_stamps_the_times_and_ignores_the_submitted_ones() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let mut submitted = profile("Night Shift", &["hj-c1r1"]);
    submitted.created = 5;
    submitted.updated = 6;
    let t0 = now_millis();
    let created = store.cas_put(&submitted, 0, &actor()).unwrap();
    let t1 = now_millis();
    assert_eq!(created.created, created.updated, "a create sets both");
    assert!(
        (t0..=t1).contains(&created.created),
        "{created:?} in {t0}..={t1}"
    );

    let mut next = created.clone();
    next.created = 7;
    next.updated = 8;
    next.panes[0].context.soft += 1;
    let t2 = now_millis();
    let updated = store.cas_put(&next, 1, &actor()).unwrap();
    let t3 = now_millis();
    assert_eq!(updated.created, created.created, "an update keeps created");
    assert!(updated.updated >= created.updated);
    assert!((t2..=t3).contains(&updated.updated));

    let log = store.log(&created.name).unwrap();
    assert!(
        (t0..=t1).contains(&log[0].at),
        "create entry at {}",
        log[0].at
    );
    assert!(
        (t2..=t3).contains(&log[1].at),
        "update entry at {}",
        log[1].at
    );
    assert!(
        log.windows(2).all(|w| w[0].at <= w[1].at),
        "at never decreases"
    );
}

// --- Store: CAS and the change log (AC 7-9) -----------------------------------------

#[test]
fn a_refused_write_changes_nothing_and_logs_nothing() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let night = create_profile(&store, "Night Shift");
    let mut next = night.clone();
    next.panes[0].context.soft += 1;
    store.cas_put(&next, 1, &actor()).unwrap();

    let bytes = std::fs::read(profiles_file(&state)).unwrap();
    let list = store.list().unwrap();
    let log = store.log(&night.name).unwrap();
    let head = profile_head(&store);

    type Attempt = Box<dyn Fn(&ProfileState) -> Result<(), PaneError>>;
    let table: Vec<(&str, &str, Attempt)> = vec![
        ("stale cas_put", "generation-conflict", {
            let n = next.clone();
            Box::new(move |s| s.cas_put(&n, 1, &actor()).map(|_| ()))
        }),
        ("cas_put ahead", "generation-conflict", {
            let n = next.clone();
            Box::new(move |s| s.cas_put(&n, 9, &actor()).map(|_| ()))
        }),
        ("stale delete", "generation-conflict", {
            let n = night.name.clone();
            Box::new(move |s| s.delete(&n, 1, &actor()))
        }),
        ("delete of a missing name", "profile-not-found", {
            Box::new(|s| s.delete(&pname("No Such"), 1, &actor()))
        }),
        ("same-slug create", "profile-exists", {
            Box::new(|s| {
                s.cas_put(&profile("night shift", &["hj-c1r1"]), 0, &actor())
                    .map(|_| ())
            })
        }),
    ];
    for (label, code, attempt) in table {
        let err = attempt(&store).unwrap_err();
        assert_eq!(err.code(), code, "{label}: {err:?}");
        assert_eq!(store.list().unwrap(), list, "{label}: list");
        assert_eq!(store.log(&night.name).unwrap(), log, "{label}: log");
        assert_eq!(
            std::fs::read(profiles_file(&state)).unwrap(),
            bytes,
            "{label}: file"
        );
        assert_eq!(profile_head(&store), head, "{label}: head");
    }
}

#[test]
fn an_update_summary_is_one_line_counting_specs() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let created = store
        .cas_put(
            &profile("Night Shift", &["hj-c1r1", "hj-c1r2"]),
            0,
            &actor(),
        )
        .unwrap();

    // [a, b] -> [b', c]: a removed, c added, b changed.
    let mut next = created.clone();
    next.panes.remove(0);
    next.panes[0].context.soft += 1;
    next.panes.push(profile("x", &["hj-c1r3"]).panes.remove(0));
    let updated = store.cas_put(&next, 1, &actor()).unwrap();
    // The same specs again: nothing added, removed or changed.
    store.cas_put(&updated, 2, &actor()).unwrap();

    let summaries: Vec<String> = store
        .log(&created.name)
        .unwrap()
        .into_iter()
        .filter_map(|e| match e.change {
            ProfileChange::Updated { summary } => Some(summary),
            _ => None,
        })
        .collect();
    assert_eq!(
        summaries,
        [
            "pane specs: 2 -> 2 (1 added, 1 removed, 1 changed)",
            "pane specs: 2 -> 2 (0 added, 0 removed, 0 changed)"
        ]
    );
}

#[test]
fn a_spec_pane_name_holding_a_newline_cannot_reach_the_summary() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let mut p = profile("Night Shift", &["hj-c1r1"]);
    p.panes[0].pane = "line-one\nline-two".to_owned();
    let created = store.cas_put(&p, 0, &actor()).unwrap();
    let mut next = created.clone();
    next.panes[0].context.soft += 1;
    store.cas_put(&next, 1, &actor()).unwrap();

    let log = store.log(&created.name).unwrap();
    let ProfileChange::Updated { summary } = &log[1].change else {
        panic!("expected an Updated entry: {log:?}");
    };
    assert!(
        !summary.contains('\n') && !summary.contains("line-"),
        "{summary:?}"
    );
    assert_eq!(
        summary,
        "pane specs: 1 -> 1 (0 added, 0 removed, 1 changed)"
    );
}

#[test]
fn the_log_cannot_be_rewritten() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let mut current = create_profile(&store, "Night Shift");
    for round in 1..=2 {
        current.panes[0].context.soft += 1;
        current = store.cas_put(&current, round, &actor()).unwrap();
    }
    let three = store.log(&current.name).unwrap();
    assert_eq!(three.len(), 3);
    current.panes[0].context.soft += 1;
    store.cas_put(&current, 3, &actor()).unwrap();
    let four = store.log(&current.name).unwrap();
    assert_eq!(four.len(), 4);
    assert_eq!(
        &four[..3],
        &three[..],
        "the first three entries are unchanged"
    );
}

// --- Concurrency, as invariants (AC 18-19) ------------------------------------------

#[test]
fn racing_writers_exactly_one_wins() {
    const WRITERS: usize = 16;
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let base = create_profile(&store, "Night Shift");
    let log_before = store.log(&base.name).unwrap().len();
    let barrier = Barrier::new(WRITERS);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for i in 0..WRITERS {
            let (store, barrier, results, base) = (&store, &barrier, &results, &base);
            scope.spawn(move || {
                let mut mine = base.clone();
                mine.panes[0].context.soft = 1_000 + u32::try_from(i).unwrap();
                barrier.wait();
                let outcome = store.cas_put(&mine, base.generation, &actor());
                results.lock().unwrap().push(outcome);
            });
        }
    });
    let results = results.into_inner().unwrap();
    let wins = results.iter().filter(|r| r.is_ok()).count();
    let conflicts = results
        .iter()
        .filter(|r| matches!(r, Err(PaneError::Conflict)))
        .count();
    assert_eq!((wins, conflicts), (1, WRITERS - 1), "results: {results:?}");

    let stored = store.get(&base.name).unwrap().unwrap();
    assert_eq!(stored.generation, base.generation + 1);
    assert_eq!(store.log(&base.name).unwrap().len(), log_before + 1);
    let reloaded = load_profiles(&state);
    assert_eq!(
        reloaded.list().unwrap(),
        store.list().unwrap(),
        "the file equals memory"
    );
    assert_eq!(
        reloaded.log(&base.name).unwrap(),
        store.log(&base.name).unwrap()
    );
}

#[test]
fn concurrent_read_modify_write_loses_no_update() {
    const THREADS: u32 = 8;
    const ROUNDS: u32 = 25;
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let base = create_profile(&store, "Night Shift");
    let n = base.name.clone();
    std::thread::scope(|scope| {
        for _ in 0..THREADS {
            let (store, n) = (&store, &n);
            scope.spawn(move || {
                for _ in 0..ROUNDS {
                    loop {
                        let mut p = store.get(n).unwrap().unwrap();
                        p.panes[0].context.soft += 1;
                        match store.cas_put(&p, p.generation, &actor()) {
                            Ok(_) => break,
                            Err(PaneError::Conflict) => {}
                            Err(other) => panic!("unexpected error: {other:?}"),
                        }
                    }
                }
            });
        }
    });
    let total = THREADS * ROUNDS;
    let end = store.get(&n).unwrap().unwrap();
    assert_eq!(
        end.panes[0].context.soft,
        base.panes[0].context.soft + total,
        "an update was lost"
    );
    assert_eq!(end.generation, base.generation + u64::from(total));

    let log = store.log(&n).unwrap();
    let new = &log[1..];
    assert_eq!(
        new.len(),
        total as usize,
        "one log entry per successful write"
    );
    assert!(new
        .iter()
        .all(|e| matches!(e.change, ProfileChange::Updated { .. })));
    let generations: Vec<u64> = new.iter().map(|e| e.generation).collect();
    let expected: Vec<u64> = (base.generation + 1..=base.generation + u64::from(total)).collect();
    assert_eq!(generations, expected, "consecutive generations");
}
