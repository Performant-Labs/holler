#![allow(clippy::unwrap_used, clippy::expect_used)] // #661
#![allow(clippy::panic, clippy::unreachable)] // #661
//! Issue #661: the profile registry's persistence and failure modes, driven through the
//! `ProfileStore` port (epic #633): the file `<state dir>/hub/profiles.json`, restart,
//! atomic 0600 writes, fail-closed loading, an unwritable directory, and the two
//! registries failing independently. The conformance suite, the CAS rules and the
//! concurrency tests are in `profile_registry_test.rs`.
//!
//! There is no `thread::sleep`.

mod pane_support;

use holler_hub::profile::ProfileState;
use holler_pane::{
    Actor, Cursor, EnvVarName, GridPos, PaneError, Profile, ProfileName, ProfileStore,
};
use pane_support::{
    actor, create_profile, dir_listing, load_profiles, profile, profile_head, profiles_doc,
    profiles_file, temp_state, write_profiles_doc,
};
use serde_json::{json, Value};

fn pname(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

fn who(text: &str) -> Actor {
    Actor::parse(text).unwrap()
}

fn by_slug(mut profiles: Vec<Profile>) -> Vec<Profile> {
    profiles.sort_by(|a, b| a.slug.cmp(&b.slug));
    profiles
}

// --- Persistence (AC 10-17) ----------------------------------------------------------

#[test]
fn records_logs_and_tombstones_survive_a_restart_unchanged() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let mut rich = profile("Night Shift", &["hj-c1r1", "hj-c1r2"]);
    rich.panes[0].env = vec![
        EnvVarName::parse("ANTHROPIC_API_KEY").unwrap(),
        EnvVarName::parse("HOME").unwrap(),
    ];
    rich.panes[1].herdr.grid = GridPos::parse("r2c1").unwrap();
    store.cas_put(&rich, 0, &who("alice")).unwrap();
    let day = create_profile(&store, "Day Shift");
    let mut day2 = day.clone();
    day2.panes[0].context.soft += 1;
    store.cas_put(&day2, 1, &who("bob")).unwrap();
    let dusk = create_profile(&store, "Dusk Shift");
    store.delete(&dusk.name, 1, &who("carol")).unwrap();

    let list = by_slug(store.list().unwrap());
    let logs: Vec<_> = ["Night Shift", "Day Shift", "Dusk Shift"]
        .iter()
        .map(|n| store.log(&pname(n)).unwrap())
        .collect();
    let head = profile_head(&store);
    drop(store);

    let reloaded = load_profiles(&state);
    let after = by_slug(reloaded.list().unwrap());
    assert_eq!(after, list, "every record is equal, field for field");
    assert_eq!(after.len(), 2);
    let night = reloaded.get(&pname("Night Shift")).unwrap().unwrap();
    assert_eq!(night.panes[0].env.len(), 2);
    assert_eq!(
        serde_json::to_value(night.panes[1].herdr.grid).unwrap(),
        json!({"row": 2, "col": 1, "pos": "r2c1"})
    );
    for (n, log) in ["Night Shift", "Day Shift", "Dusk Shift"].iter().zip(&logs) {
        assert_eq!(&reloaded.log(&pname(n)).unwrap(), log, "log of {n}");
    }
    assert_eq!(
        reloaded.get(&dusk.name).unwrap(),
        None,
        "a delete stays deleted"
    );
    assert_eq!(
        profile_head(&reloaded),
        head,
        "the head survives, tombstones included"
    );

    create_profile(&reloaded, "Late Shift");
    let events = pane_support::drain(&mut reloaded.watch(head).unwrap());
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].cursor,
        Cursor(head.0 + 1),
        "the cursor never goes backwards"
    );
}

#[cfg(unix)]
#[test]
fn the_file_is_written_atomically_at_mode_0600() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, state) = temp_state();
    assert!(!state.hub_dir.exists(), "the hub dir does not exist yet");
    let store = load_profiles(&state);
    create_profile(&store, "Night Shift");
    create_profile(&store, "Day Shift");

    let file = profiles_file(&state);
    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the registry file is private");
    let doc = profiles_doc(&state);
    assert_eq!(doc["version"], json!(1));
    assert_eq!(doc["cursor"], json!(2));
    let entries = doc["entries"].as_array().unwrap();
    let slugs: Vec<&str> = entries
        .iter()
        .map(|e| e["slug"].as_str().unwrap())
        .collect();
    assert_eq!(
        slugs,
        ["day-shift", "night-shift"],
        "entries are sorted by slug"
    );
    assert_eq!(entries[0]["event"]["cursor"], json!(2));
    assert_eq!(entries[0]["event"]["profile"]["generation"], json!(1));
    assert_eq!(entries[0]["log"].as_array().unwrap().len(), 1);
    let leftovers: Vec<String> = dir_listing(&state.hub_dir)
        .into_iter()
        .filter(|n| n.starts_with(".profiles.json.") && n.ends_with(".tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "no temp file is left behind: {leftovers:?}"
    );
}

#[test]
fn loading_never_writes() {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let night = create_profile(&store, "Night Shift");
    store
        .delete(&create_profile(&store, "Day Shift").name, 1, &actor())
        .unwrap();
    drop(store);
    let file = profiles_file(&state);
    let bytes = std::fs::read(&file).unwrap();
    let listing = dir_listing(&state.hub_dir);
    let modified = std::fs::metadata(&file).unwrap().modified().unwrap();

    let reloaded = load_profiles(&state);
    reloaded.get(&night.name).unwrap();
    reloaded.list().unwrap();
    reloaded.log(&night.name).unwrap();
    pane_support::drain(&mut reloaded.watch(Cursor(0)).unwrap());
    profile_head(&reloaded);

    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    assert_eq!(dir_listing(&state.hub_dir), listing);
    assert_eq!(
        std::fs::metadata(&file).unwrap().modified().unwrap(),
        modified
    );
}

/// A valid document with three profiles: `Night Shift` (cursor 1), `Day Shift` (cursor 2)
/// and the tombstone `Dusk Shift` (created at 3, deleted at 4).
fn valid_doc() -> Value {
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    create_profile(&store, "Night Shift");
    create_profile(&store, "Day Shift");
    let dusk = create_profile(&store, "Dusk Shift");
    store.delete(&dusk.name, 1, &actor()).unwrap();
    profiles_doc(&state)
}

fn entry_mut<'a>(doc: &'a mut Value, slug: &str) -> &'a mut Value {
    doc["entries"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["slug"] == json!(slug))
        .unwrap_or_else(|| panic!("no entry {slug}"))
}

fn corrupt_cases() -> Vec<(&'static str, Vec<u8>)> {
    let base = valid_doc();
    let bytes = |v: &Value| serde_json::to_vec_pretty(v).unwrap();
    let mutated = |edit: &dyn Fn(&mut Value)| {
        let mut doc = base.clone();
        edit(&mut doc);
        bytes(&doc)
    };
    vec![
        ("not JSON", b"this is { not json".to_vec()),
        ("version 2", mutated(&|d| d["version"] = json!(2))),
        (
            "unknown field in a record",
            mutated(&|d| entry_mut(d, "night-shift")["event"]["profile"]["surprise"] = json!(true)),
        ),
        (
            "two entries share a slug",
            mutated(&|d| {
                let mut twin = entry_mut(d, "night-shift").clone();
                twin["event"]["cursor"] = json!(2);
                d["entries"] = json!([entry_mut(d, "night-shift").clone(), twin]);
            }),
        ),
        (
            "entry slug differs from its record's",
            mutated(&|d| entry_mut(d, "night-shift")["slug"] = json!("someone-else")),
        ),
        (
            "tombstone event name has another slug",
            mutated(&|d| entry_mut(d, "dusk-shift")["event"]["name"] = json!("Other Name")),
        ),
        (
            "live event name differs from its record's name",
            mutated(&|d| entry_mut(d, "night-shift")["event"]["name"] = json!("night shift")),
        ),
        (
            "entry with an empty log",
            mutated(&|d| entry_mut(d, "night-shift")["log"] = json!([])),
        ),
        (
            "live last log generation differs from the record's",
            mutated(&|d| entry_mut(d, "night-shift")["log"][0]["generation"] = json!(99)),
        ),
        (
            "tombstone last log entry is not Deleted",
            mutated(&|d| entry_mut(d, "dusk-shift")["log"][1]["change"] = json!("created")),
        ),
    ]
}

/// Every method of the port, run against `store`: each must answer `store-corrupt` with
/// a `what` that names the registry file.
fn assert_every_method_is_store_corrupt(store: &ProfileState, file: &std::path::Path, label: &str) {
    let n = pname("Night Shift");
    let errors = [
        store.get(&n).unwrap_err(),
        store.list().unwrap_err(),
        store
            .cas_put(&profile("Night Shift", &[]), 0, &actor())
            .unwrap_err(),
        store.delete(&n, 1, &actor()).unwrap_err(),
        store.watch(Cursor(0)).err().expect("watch must refuse"),
        store.log(&n).unwrap_err(),
    ];
    for err in errors {
        let PaneError::StoreCorrupt { what } = &err else {
            panic!("{label}: expected store-corrupt, got {err:?}");
        };
        assert!(
            what.starts_with(&format!("profile registry {}", file.display())),
            "{label}: `what` must name the registry and the file: {what}"
        );
        assert_eq!(err.code(), "store-corrupt");
    }
}

#[test]
fn a_corrupt_file_fails_closed_and_is_never_rewritten() {
    for (label, content) in corrupt_cases() {
        let (_dir, state) = temp_state();
        std::fs::create_dir_all(&state.hub_dir).unwrap();
        std::fs::write(profiles_file(&state), &content).unwrap();
        let listing = dir_listing(&state.hub_dir);

        let store = load_profiles(&state);
        assert_every_method_is_store_corrupt(&store, &profiles_file(&state), label);

        assert_eq!(
            std::fs::read(profiles_file(&state)).unwrap(),
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
    let content = write_profiles_doc(&state, &valid_doc());
    let file = profiles_file(&state);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&file).is_ok() {
        return; // running as a user that can read anything
    }
    let listing = dir_listing(&state.hub_dir);

    let store = load_profiles(&state);
    assert_every_method_is_store_corrupt(&store, &file, "unreadable");

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
fn a_stored_secret_value_fails_closed_without_echoing_it() {
    const SENTINEL: &str = "SENTINEL-661";
    let mut doc = valid_doc();
    entry_mut(&mut doc, "night-shift")["event"]["profile"]["panes"][0]["env"] =
        json!([format!("TOKEN={SENTINEL}")]);
    let (_dir, state) = temp_state();
    write_profiles_doc(&state, &doc);

    let store = load_profiles(&state);
    let err = store.list().unwrap_err();
    let PaneError::StoreCorrupt { what } = &err else {
        panic!("a stored secret must load as store-corrupt, got {err:?}");
    };
    assert!(!what.contains(SENTINEL), "`what` echoes the value: {what}");
    assert!(
        !err.to_string().contains(SENTINEL),
        "Display echoes the value: {err}"
    );
}

#[cfg(unix)]
#[test]
fn an_unwritable_directory_refuses_the_write_and_keeps_the_old_state() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, state) = temp_state();
    let store = load_profiles(&state);
    let created = create_profile(&store, "Night Shift");
    let head = profile_head(&store);
    let log = store.log(&created.name).unwrap();
    let mut next = created.clone();
    next.panes[0].context.soft += 1;

    std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    if std::fs::File::create(state.hub_dir.join("probe")).is_ok() {
        std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        return; // root ignores directory modes
    }
    for err in [
        store.cas_put(&next, 1, &actor()).unwrap_err(),
        store.delete(&created.name, 1, &actor()).unwrap_err(),
    ] {
        assert!(matches!(err, PaneError::Unavailable { .. }), "got {err:?}");
        assert_eq!(err.code(), "unavailable");
    }
    assert_eq!(store.get(&created.name).unwrap(), Some(created.clone()));
    assert_eq!(store.log(&created.name).unwrap(), log);
    assert_eq!(profile_head(&store), head, "the cursor did not move");

    std::fs::set_permissions(&state.hub_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let healed = store.cas_put(&next, 1, &actor()).unwrap();
    assert_eq!(
        healed.generation, 2,
        "the same write succeeds once the directory is writable"
    );
    assert_eq!(
        load_profiles(&state).get(&created.name).unwrap(),
        Some(healed)
    );
}

#[test]
fn the_two_registries_fail_independently() {
    use holler_hub::panes::PaneState;
    use holler_pane::PaneStore;

    // A corrupt profiles.json leaves the pane methods working for a pane outside a profile.
    let (_dir, state) = temp_state();
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    std::fs::write(profiles_file(&state), b"{ not json").unwrap();
    let panes = PaneState::load_with(&state, pane_support::short_opts());
    let pane = pane_support::sample_pane("hj-c1r1", None);
    let stored = panes.cas_put(&pane, 0).unwrap();
    assert_eq!(panes.get(&stored.name).unwrap(), Some(stored.clone()));
    assert_eq!(panes.list().unwrap().len(), 1);
    panes.delete(&stored.name, 1).unwrap();
    assert!(profiles_file(&state).exists());

    // A corrupt panes.json leaves every profile method working.
    let (_dir2, state2) = temp_state();
    std::fs::create_dir_all(&state2.hub_dir).unwrap();
    std::fs::write(pane_support::registry_file(&state2), b"{ not json").unwrap();
    let profiles = load_profiles(&state2);
    let created = create_profile(&profiles, "Night Shift");
    assert_eq!(profiles.get(&created.name).unwrap(), Some(created.clone()));
    assert_eq!(profiles.list().unwrap().len(), 1);
    assert_eq!(profiles.log(&created.name).unwrap().len(), 1);
    profiles.delete(&created.name, 1, &actor()).unwrap();
}
