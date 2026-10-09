#![allow(clippy::unwrap_used, clippy::expect_used)] // #639
#![allow(clippy::panic, clippy::unreachable)] // #639
//! Issue #639: the hub's pane registry, driven through the `PaneStore` port from plain
//! threads, with no socket (epic #633). These are the store tests: compare-and-swap,
//! persistence, fail-closed loading and concurrency. The change feed is in
//! `pane_feed_test.rs` and the wire handlers are in `pane_handlers_test.rs`.
//!
//! Every wait is a bounded one. There is no `thread::sleep`: a race is asserted as an
//! invariant ("exactly one wins", "no update is lost"), never as a duration.

mod pane_support;

use std::sync::{Barrier, Mutex};

use holler_hub::panes::PaneState;
use holler_pane::{Cursor, Pane, PaneError, PaneStore};
use pane_support::{
    create, dir_listing, drain, head, load, name, registry_file, sample_pane, temp_state,
};
use serde_json::{json, Value};

// --- Store: CAS, records, the no-inference rule --------------------------------------

#[test]
fn cas_put_creates_at_generation_one_and_get_returns_it() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    assert_eq!(
        store.get(&name("hj-c1r1")).unwrap(),
        None,
        "nothing stored yet"
    );

    let stored = store.cas_put(&sample_pane("hj-c1r1", None), 0).unwrap();
    assert_eq!(stored.generation, 1, "a new pane is stored at generation 1");
    assert_eq!(store.get(&name("hj-c1r1")).unwrap(), Some(stored));
}

#[test]
fn cas_put_with_a_stale_generation_is_a_conflict_and_changes_nothing() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let created = create(&store, "hj-c1r1");
    let updated = store.cas_put(&created, 1).unwrap();
    assert_eq!(updated.generation, 2);
    let bytes = std::fs::read(registry_file(&state)).unwrap();
    let head_before = head(&store);

    // 0 on an existing record, an old generation and a future one are all stale.
    for expected in [0, 1, 3] {
        let mut attempt = updated.clone();
        attempt.context.soft += 1;
        let err = store.cas_put(&attempt, expected).unwrap_err();
        assert_eq!(err, PaneError::Conflict, "expected_generation {expected}");
        assert_eq!(err.code(), "generation-conflict");
    }
    assert_eq!(store.get(&name("hj-c1r1")).unwrap(), Some(updated));
    assert_eq!(std::fs::read(registry_file(&state)).unwrap(), bytes);
    assert_eq!(head(&store), head_before, "a refused write takes no cursor");
}

#[test]
fn cas_put_stores_the_record_verbatim_except_generation() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    // The submitted generation (7 in the sample) is ignored; the sample has
    // `session_of_record: None` next to a `shown` session, and the hub never infers one.
    let mut submitted = sample_pane("hj-c1r1", Some("Night Shift"));
    assert!(submitted.session_of_record.is_none() && submitted.last_observed.shown.is_some());
    let stored = store.cas_put(&submitted, 0).unwrap();
    submitted.generation = 1;
    assert_eq!(stored, submitted);
    assert!(
        stored.session_of_record.is_none(),
        "the hub must not infer a session"
    );

    submitted.generation = 42;
    submitted.context.soft = 90_000;
    let again = store.cas_put(&submitted, 1).unwrap();
    submitted.generation = 2;
    assert_eq!(again, submitted);
    assert_eq!(store.get(&name("hj-c1r1")).unwrap(), Some(submitted));
}

#[test]
fn delete_checks_missing_before_generation() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let missing = |store: &PaneState, expected: u64| {
        let err = store.delete(&name("hj-c1r1"), expected).unwrap_err();
        assert!(matches!(err, PaneError::PaneNotFound { .. }), "got {err:?}");
        assert_eq!(err.code(), "pane-not-found");
    };
    missing(&store, 0);
    missing(&store, 5);

    create(&store, "hj-c1r1");
    assert_eq!(
        store.delete(&name("hj-c1r1"), 7).unwrap_err(),
        PaneError::Conflict
    );
    assert!(
        store.get(&name("hj-c1r1")).unwrap().is_some(),
        "a conflict keeps the record"
    );

    store.delete(&name("hj-c1r1"), 1).unwrap();
    assert_eq!(store.get(&name("hj-c1r1")).unwrap(), None);
    assert!(store.list().unwrap().is_empty());
    // After an earlier delete the record is missing again, whatever generation is named.
    missing(&store, 1);
    missing(&store, 99);

    let recreated = store.cas_put(&sample_pane("hj-c1r1", None), 0).unwrap();
    assert_eq!(
        recreated.generation, 1,
        "a deleted pane comes back at generation 1"
    );
}

#[test]
fn list_returns_every_live_record_sorted_by_name() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    assert!(store.list().unwrap().is_empty());
    for pane in ["hj-c1r3", "hj-c1r1", "hj-c2r1", "hj-c1r2"] {
        create(&store, pane);
    }
    store.delete(&name("hj-c1r2"), 1).unwrap();
    let names: Vec<String> = store
        .list()
        .unwrap()
        .iter()
        .map(|p| p.name.to_string())
        .collect();
    assert_eq!(names, ["hj-c1r1", "hj-c1r3", "hj-c2r1"]);
}

// --- Persistence ---------------------------------------------------------------------

#[test]
fn records_survive_a_restart_unchanged() {
    let (_dir, state) = temp_state();
    let store = load(&state);
    let mut a = sample_pane("hj-c1r1", Some("Night Shift"));
    a.herdr.grid = holler_pane::GridPos::parse("r2c1").unwrap();
    store.cas_put(&a, 0).unwrap();
    create(&store, "hj-c1r2");
    let b2 = store.get(&name("hj-c1r2")).unwrap().unwrap();
    store.cas_put(&b2, 1).unwrap();
    create(&store, "hj-c1r3");
    store.delete(&name("hj-c1r3"), 1).unwrap();
    let before = store.list().unwrap();
    let head_before = head(&store);
    drop(store);

    let reloaded = load(&state);
    let after = reloaded.list().unwrap();
    assert_eq!(after, before, "every record is equal, field for field");
    assert_eq!(
        after[0].profile.as_ref().map(|p| p.as_str()),
        Some("Night Shift")
    );
    assert_eq!(after[1].generation, 2, "generations are kept");
    assert_eq!(
        serde_json::to_value(after[0].herdr.grid).unwrap(),
        json!({"row": 2, "col": 1, "pos": "r2c1"})
    );
    assert_eq!(
        reloaded.get(&name("hj-c1r3")).unwrap(),
        None,
        "a deleted pane stays deleted"
    );

    assert_eq!(
        head(&reloaded),
        head_before,
        "the head survives, tombstones included"
    );
    create(&reloaded, "hj-c1r4");
    let mut w = reloaded.watch(head_before).unwrap();
    let events = drain(&mut w);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].cursor,
        Cursor(head_before.0 + 1),
        "the cursor never goes backwards"
    );
}

/// A v1 file with one record, built by hand: `pane` as JSON, entry and head at cursor 1.
fn v1_file(pane: &Value) -> Value {
    json!({"version": 1, "cursor": 1,
           "entries": [{"name": pane["name"], "cursor": 1, "pane": pane}]})
}

fn write_registry(state: &holler_hub::state::HubState, doc: &Value) {
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    std::fs::write(
        registry_file(state),
        serde_json::to_vec_pretty(doc).unwrap(),
    )
    .unwrap();
}

#[test]
fn a_record_written_before_the_profile_field_loads_with_profile_none() {
    let (_dir, state) = temp_state();
    let mut pane = serde_json::to_value(sample_pane("hj-c1r1", Some("Night Shift"))).unwrap();
    pane.as_object_mut().unwrap().remove("profile");
    pane["generation"] = json!(3);
    write_registry(&state, &v1_file(&pane));

    let store = load(&state);
    let loaded = store
        .get(&name("hj-c1r1"))
        .unwrap()
        .expect("the record loads");
    assert_eq!(loaded.profile, None);
    assert_eq!(loaded.generation, 3);
    assert_eq!(loaded, sample_pane_at("hj-c1r1", 3), "nothing else changed");
}

fn sample_pane_at(pane_name: &str, generation: u64) -> Pane {
    let mut pane = sample_pane(pane_name, None);
    pane.generation = generation;
    pane
}

#[cfg(unix)]
#[test]
fn the_file_is_written_atomically_at_mode_0600() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, state) = temp_state();
    assert!(!state.hub_dir.exists(), "the hub dir does not exist yet");
    let store = load(&state);
    create(&store, "hj-c1r1");
    create(&store, "hj-c1r2");

    let file = registry_file(&state);
    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the registry file is private");
    let doc: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(doc["version"], json!(1));
    assert_eq!(doc["cursor"], json!(2));
    let entries = doc["entries"].as_array().unwrap();
    let names: Vec<&str> = entries
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["hj-c1r1", "hj-c1r2"], "entries are sorted by name");
    assert_eq!(entries[0]["cursor"], json!(1));
    assert_eq!(entries[1]["pane"]["generation"], json!(1));
    let leftovers: Vec<String> = dir_listing(&state.hub_dir)
        .into_iter()
        .filter(|n| n.starts_with(".panes.json.") && n.ends_with(".tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "no temp file is left behind: {leftovers:?}"
    );
}

// --- Fail closed ---------------------------------------------------------------------

/// Every method of the port, run against `store`: each must answer `store-corrupt` with
/// a `what` that names the registry file.
fn assert_every_method_is_store_corrupt(store: &PaneState, label: &str) {
    let n = name("hj-c1r1");
    let errors = [
        store.get(&n).unwrap_err(),
        store.list().unwrap_err(),
        store.cas_put(&sample_pane("hj-c1r1", None), 0).unwrap_err(),
        store.delete(&n, 1).unwrap_err(),
        store.watch(Cursor(0)).err().expect("watch must refuse"),
    ];
    for err in errors {
        let PaneError::StoreCorrupt { what } = &err else {
            panic!("{label}: expected store-corrupt, got {err:?}");
        };
        assert!(
            what.contains("panes.json"),
            "{label}: `what` must name the file: {what}"
        );
        assert_eq!(err.code(), "store-corrupt");
    }
}

fn corrupt_cases() -> Vec<(&'static str, Vec<u8>)> {
    let pane = serde_json::to_value(sample_pane_at("hj-c1r1", 1)).unwrap();
    let other = serde_json::to_value(sample_pane_at("hj-c1r2", 1)).unwrap();
    let mut with_unknown = pane.clone();
    with_unknown["surprise"] = json!(true);
    let mut v2 = v1_file(&pane);
    v2["version"] = json!(2);
    let mut duplicate_name = v1_file(&pane);
    duplicate_name["cursor"] = json!(2);
    duplicate_name["entries"] = json!([
        {"name": "hj-c1r1", "cursor": 1, "pane": pane},
        {"name": "hj-c1r1", "cursor": 2, "pane": pane}]);
    let mut mismatch = v1_file(&pane);
    mismatch["entries"][0]["name"] = json!("hj-c1r2");
    // Not in the brief's five-case table: A's finding W-5 (distinct entry cursors).
    let mut same_cursor = v1_file(&pane);
    same_cursor["entries"] = json!([
        {"name": "hj-c1r1", "cursor": 1, "pane": pane},
        {"name": "hj-c1r2", "cursor": 1, "pane": other}]);
    let bytes = |v: &Value| serde_json::to_vec_pretty(v).unwrap();
    vec![
        ("not JSON", b"this is { not json".to_vec()),
        ("version 2", bytes(&v2)),
        ("unknown field in a record", bytes(&v1_file(&with_unknown))),
        ("duplicate pane name", bytes(&duplicate_name)),
        ("entry name differs from its record", bytes(&mismatch)),
        ("two entries share a cursor", bytes(&same_cursor)),
    ]
}

#[test]
fn a_corrupt_file_fails_closed_and_is_never_rewritten() {
    for (label, content) in corrupt_cases() {
        let (_dir, state) = temp_state();
        std::fs::create_dir_all(&state.hub_dir).unwrap();
        std::fs::write(registry_file(&state), &content).unwrap();
        let listing = dir_listing(&state.hub_dir);

        let store = load(&state);
        assert_every_method_is_store_corrupt(&store, label);

        assert_eq!(
            std::fs::read(registry_file(&state)).unwrap(),
            content,
            "{label}: rewritten"
        );
        assert_eq!(
            dir_listing(&state.hub_dir),
            listing,
            "{label}: a file was moved or created"
        );
    }
}

#[cfg(unix)]
#[test]
fn an_unreadable_file_fails_closed_and_is_never_rewritten() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, state) = temp_state();
    let pane = serde_json::to_value(sample_pane_at("hj-c1r1", 1)).unwrap();
    write_registry(&state, &v1_file(&pane));
    let file = registry_file(&state);
    let content = std::fs::read(&file).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&file).is_ok() {
        return; // running as a user that can read anything
    }
    let listing = dir_listing(&state.hub_dir);

    let store = load(&state);
    assert_every_method_is_store_corrupt(&store, "unreadable");

    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        std::fs::read(&file).unwrap(),
        content,
        "the unreadable file was rewritten"
    );
    assert_eq!(
        dir_listing(&state.hub_dir),
        listing,
        "a file was moved or created"
    );
}

#[test]
fn the_corrupt_reason_never_echoes_file_content() {
    const SENTINEL: &str = "SENTINEL-639";
    let pane = serde_json::to_value(sample_pane_at("hj-c1r1", 1)).unwrap();
    // A sentinel as a value of the wrong type, and as the NAME of an unknown field:
    // serde's own messages quote both.
    let mut wrong_type = pane.clone();
    wrong_type["harness"]["port"] = json!(SENTINEL);
    let mut unknown_key = pane.clone();
    unknown_key[SENTINEL] = json!(1);
    let mut bad_version = v1_file(&pane);
    bad_version["version"] = json!(SENTINEL);
    for doc in [v1_file(&wrong_type), v1_file(&unknown_key), bad_version] {
        let (_dir, state) = temp_state();
        write_registry(&state, &doc);
        let store = load(&state);
        let err = store.list().unwrap_err();
        let PaneError::StoreCorrupt { what } = &err else {
            panic!("expected store-corrupt, got {err:?}");
        };
        assert!(
            !what.contains(SENTINEL),
            "`what` echoes file content: {what}"
        );
        assert!(
            !err.to_string().contains(SENTINEL),
            "Display echoes file content: {err}"
        );
    }
}

#[cfg(unix)]
#[test]
fn an_unwritable_directory_refuses_the_write_and_keeps_the_old_state() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, state) = temp_state();
    let store = load(&state);
    let created = create(&store, "hj-c1r1");
    let head_before = head(&store);
    let mut next = created.clone();
    next.context.soft += 1;

    std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    if std::fs::File::create(state.hub_dir.join("probe")).is_ok() {
        std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        return; // root ignores directory modes
    }
    let err = store.cas_put(&next, 1).unwrap_err();
    assert!(matches!(err, PaneError::Unavailable { .. }), "got {err:?}");
    assert_eq!(err.code(), "unavailable");
    assert_eq!(
        store.get(&name("hj-c1r1")).unwrap(),
        Some(created),
        "memory keeps the old record"
    );
    assert_eq!(head(&store), head_before, "the cursor did not move");

    std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let healed = store.cas_put(&next, 1).unwrap();
    assert_eq!(
        healed.generation, 2,
        "the same write succeeds once the directory is writable"
    );
    assert_eq!(load(&state).get(&name("hj-c1r1")).unwrap(), Some(healed));
}

// --- Concurrency, as invariants ------------------------------------------------------

#[test]
fn racing_writers_exactly_one_wins() {
    const WRITERS: usize = 16;
    let (_dir, state) = temp_state();
    let store = load(&state);
    let base = create(&store, "hj-c1r1");
    let barrier = Barrier::new(WRITERS);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for i in 0..WRITERS {
            let (store, barrier, results, base) = (&store, &barrier, &results, &base);
            scope.spawn(move || {
                let mut mine = base.clone();
                mine.context.soft = 1_000 + u32::try_from(i).unwrap();
                barrier.wait();
                let outcome = store.cas_put(&mine, base.generation);
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

    let stored = store.get(&name("hj-c1r1")).unwrap().unwrap();
    assert_eq!(stored.generation, base.generation + 1);
    assert_eq!(
        load(&state).list().unwrap(),
        store.list().unwrap(),
        "the file equals memory"
    );
}

#[test]
fn concurrent_read_modify_write_loses_no_update() {
    const THREADS: u32 = 8;
    const ROUNDS: u32 = 25;
    let (_dir, state) = temp_state();
    let store = load(&state);
    let base = create(&store, "hj-c1r1");
    let start = head(&store);
    std::thread::scope(|scope| {
        for _ in 0..THREADS {
            let (store, n) = (&store, name("hj-c1r1"));
            scope.spawn(move || {
                for _ in 0..ROUNDS {
                    loop {
                        let mut pane = store.get(&n).unwrap().unwrap();
                        pane.context.soft += 1;
                        match store.cas_put(&pane, pane.generation) {
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
    let end = store.get(&name("hj-c1r1")).unwrap().unwrap();
    assert_eq!(
        end.context.soft,
        base.context.soft + total,
        "an update was lost"
    );
    assert_eq!(end.generation, base.generation + u64::from(total));

    let events = drain(&mut store.watch(start).unwrap());
    let cursors: Vec<u64> = events.iter().map(|e| e.cursor.0).collect();
    let expected: Vec<u64> = (start.0 + 1..=start.0 + u64::from(total)).collect();
    assert_eq!(
        cursors, expected,
        "one put event per successful write, consecutive"
    );
    assert!(events.iter().all(|e| e.pane.is_some()));
}
