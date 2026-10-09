#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #682
//! `FakeProfileStore`'s own mechanisms: fault injection (a wedged or corrupt store, a
//! one-shot error, a slow call), the call log, the settable clock, the change log's
//! entries, another writer's change, the idle wait, seeding and the fixtures. The
//! generic port rules (CAS, ordering, watch) are the conformance suite's job; this
//! file pins what only the fake has (#638, slice c part 1, #682).

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use holler_pane::pane::{HarnessKind, PaneRole};
use holler_pane::{
    Actor, Cursor, PaneError, Profile, ProfileChange, ProfileEvent, ProfileLogEntry, ProfileName,
    ProfileStore, Watch,
};
use holler_pane_testkit::fault::{Fault, FaultSwitch, PortOp};
use holler_pane_testkit::fixture::{sample_pane, sample_profile, sample_spec};
use holler_pane_testkit::profile_store::{FakeProfileStore, ProfileStoreOp};

fn actor(text: &str) -> Actor {
    Actor::parse(text).unwrap()
}

fn profile(name: &str, panes: &[&str]) -> Profile {
    sample_profile(name, panes).unwrap()
}

fn name(text: &str) -> ProfileName {
    ProfileName::parse(text).unwrap()
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}

fn corrupt() -> PaneError {
    PaneError::StoreCorrupt {
        what: "profile store".to_owned(),
    }
}

fn unavailable(what: &str) -> PaneError {
    PaneError::Unavailable {
        what: what.to_owned(),
    }
}

fn not_found(n: &ProfileName) -> PaneError {
    PaneError::ProfileNotFound {
        what: n.to_string(),
    }
}

/// `watch` without needing `Watch` to be `Debug`.
fn open_watch(store: &FakeProfileStore, since: u64) -> Watch<ProfileEvent> {
    match store.watch(Cursor(since)) {
        Ok(watch) => watch,
        Err(e) => panic!("watch({since}) failed: {e:?}"),
    }
}

fn watch_error(store: &FakeProfileStore, since: u64) -> Option<PaneError> {
    store.watch(Cursor(since)).err()
}

/// Everything the watch yields until the first idle `Ok(None)`.
fn drain(watch: &mut Watch<ProfileEvent>) -> Vec<ProfileEvent> {
    let mut events = Vec::new();
    for _ in 0..1000 {
        match watch.next() {
            Some(Ok(Some(event))) => events.push(event),
            Some(Ok(None)) => return events,
            other => panic!("expected an event or idle, got {other:?}"),
        }
    }
    panic!("watch did not go idle within 1000 items");
}

fn entry(at: i64, generation: u64, who: &Actor, change: ProfileChange) -> ProfileLogEntry {
    ProfileLogEntry {
        at,
        generation,
        actor: who.clone(),
        change,
    }
}

// --- wedged ---

#[test]
fn a_wedged_store_times_out_every_method() {
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let who = actor("tester");
    let store = FakeProfileStore::new();
    store.faults().set(Some(Fault::Wedged));

    assert_eq!(store.get(&a.name), Err(timeout("profile_store.get")));
    assert_eq!(store.list(), Err(timeout("profile_store.list")));
    assert_eq!(
        store.cas_put(&a, 0, &who),
        Err(timeout("profile_store.cas_put"))
    );
    assert_eq!(
        store.delete(&a.name, 1, &who),
        Err(timeout("profile_store.delete"))
    );
    assert_eq!(watch_error(&store, 0), Some(timeout("profile_store.watch")));
    assert_eq!(store.log(&a.name), Err(timeout("profile_store.log")));
    assert_eq!(
        store.rename(&a.name, &name("Demo Beta"), 1, &who),
        Err(timeout("profile_store.rename"))
    );

    store.faults().set(None);
    assert_eq!(store.list(), Ok(vec![]), "the wedged cas_put wrote nothing");
    assert_eq!(store.log(&a.name), Err(not_found(&a.name)));
    assert_eq!(store.cas_put(&a, 0, &who).unwrap().generation, 1);
    assert!(store.get(&a.name).unwrap().is_some());
}

#[test]
fn a_wedged_store_ends_an_open_watch() {
    let store =
        FakeProfileStore::seeded([profile("Demo Alpha", &["demo-c1r1"])], &actor("seed")).unwrap();
    let mut watch = open_watch(&store, 0);

    store.faults().set(Some(Fault::Wedged));
    assert_eq!(
        watch.next(),
        Some(Err(timeout("profile_store.watch_next"))),
        "a wedged store is a timeout, never idle"
    );

    store.faults().set(None);
    assert_eq!(watch.next(), None, "any error ends the stream");
}

// --- corrupt ---

#[test]
fn a_corrupt_store_fails_closed_everywhere() {
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let who = actor("seed");
    let store = FakeProfileStore::seeded([a.clone()], &who).unwrap();
    let record = store.get(&a.name).unwrap().unwrap();
    let history = store.log(&a.name).unwrap();

    store.faults().set(Some(Fault::Fail(corrupt())));
    assert_eq!(store.get(&a.name), Err(corrupt()));
    assert_eq!(store.list(), Err(corrupt()));
    assert_eq!(store.cas_put(&a, 1, &who), Err(corrupt()));
    assert_eq!(store.delete(&a.name, 1, &who), Err(corrupt()));
    assert_eq!(watch_error(&store, 0), Some(corrupt()));
    assert_eq!(store.log(&a.name), Err(corrupt()));
    assert_eq!(
        store.rename(&a.name, &name("Demo Beta"), 1, &who),
        Err(corrupt())
    );

    store.faults().set(None);
    assert_eq!(
        store.get(&a.name),
        Ok(Some(record)),
        "the records written before are intact"
    );
    assert_eq!(store.log(&a.name), Ok(history), "and so is the log");
}

// --- one-shot errors ---

#[test]
fn fail_next_is_one_shot_and_per_op() {
    let store = FakeProfileStore::new();
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let who = actor("tester");
    store
        .faults()
        .fail_next(ProfileStoreOp::CasPut, unavailable("hub"));

    assert_eq!(store.cas_put(&a, 0, &who), Err(unavailable("hub")));
    assert_eq!(store.get(&a.name), Ok(None), "get is not the targeted op");
    assert_eq!(store.list(), Ok(vec![]), "the failed cas_put wrote nothing");
    assert_eq!(
        store.log(&a.name),
        Err(not_found(&a.name)),
        "and logged nothing"
    );

    let stored = store.cas_put(&a, 0, &who).unwrap();
    assert_eq!(stored.generation, 1, "the second cas_put succeeds");
}

// --- slow ---

#[test]
fn a_slow_call_takes_at_least_the_delay() {
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let store = FakeProfileStore::seeded([a.clone()], &actor("seed")).unwrap();
    store.faults().set_delay(Some(Duration::from_millis(50)));

    let started = Instant::now();
    let result = store.log(&a.name);
    let took = started.elapsed();

    assert_eq!(result.map(|l| l.len()), Ok(1));
    assert!(took >= Duration::from_millis(50), "took only {took:?}");
}

// --- the call log ---

#[test]
fn calls_are_recorded_in_order() {
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let who = actor("tester");
    let store = FakeProfileStore::seeded([a.clone()], &who).unwrap();
    assert_eq!(store.faults().calls(), vec![], "seeding is not a call");
    store
        .faults()
        .fail_next(ProfileStoreOp::CasPut, unavailable("hub"));

    store.get(&a.name).unwrap();
    assert!(store.cas_put(&a, 1, &who).is_err());
    store.log(&a.name).unwrap();
    store.list().unwrap();
    let expected = vec![
        ProfileStoreOp::Get,
        ProfileStoreOp::CasPut,
        ProfileStoreOp::Log,
        ProfileStoreOp::List,
    ];
    assert_eq!(
        store.faults().calls(),
        expected,
        "failed calls are recorded too"
    );

    store.concurrent_put(&a, &actor("other")).unwrap();
    store.concurrent_delete(&a.name, &actor("other")).unwrap();
    assert_eq!(
        store.faults().calls(),
        expected,
        "another writer is not a call through the port"
    );
}

#[test]
fn port_op_names_are_port_dot_method() {
    assert_eq!(ProfileStoreOp::Get.as_str(), "profile_store.get");
    assert_eq!(ProfileStoreOp::List.as_str(), "profile_store.list");
    assert_eq!(ProfileStoreOp::CasPut.as_str(), "profile_store.cas_put");
    assert_eq!(ProfileStoreOp::Delete.as_str(), "profile_store.delete");
    assert_eq!(ProfileStoreOp::Watch.as_str(), "profile_store.watch");
    assert_eq!(
        ProfileStoreOp::WatchNext.as_str(),
        "profile_store.watch_next"
    );
    assert_eq!(ProfileStoreOp::Log.as_str(), "profile_store.log");
    assert_eq!(ProfileStoreOp::Rename.as_str(), "profile_store.rename");
}

// --- the clock and the log's entries ---

#[test]
fn the_clock_stamps_created_updated_and_at() {
    let who = actor("tester");
    let store = FakeProfileStore::new();
    let one = profile("Demo Alpha", &["demo-c1r1"]);
    let two = profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"]);

    store.set_now(1_000);
    let created = store.cas_put(&one, 0, &who).unwrap();
    assert_eq!((created.created, created.updated), (1_000, 1_000));

    store.set_now(2_000);
    let updated = store.cas_put(&two, 1, &who).unwrap();
    assert_eq!(updated.created, 1_000, "created is kept on update");
    assert_eq!(updated.updated, 2_000);

    store.set_now(3_000);
    store.delete(&one.name, 2, &who).unwrap();

    let log = store.log(&one.name).unwrap();
    assert_eq!(log.len(), 3);
    assert_eq!((log[0].at, log[0].generation), (1_000, 1));
    assert_eq!((log[1].at, log[1].generation), (2_000, 2));
    assert_eq!(log[2].change, ProfileChange::Deleted);
    assert_eq!(
        (log[2].at, log[2].generation),
        (3_000, 3),
        "a Deleted entry carries the deleted generation + 1"
    );

    let fresh = FakeProfileStore::new();
    let stamped = fresh.cas_put(&one, 0, &who).unwrap();
    assert_eq!(
        (stamped.created, stamped.updated),
        (0, 0),
        "the default clock"
    );
    assert_eq!(fresh.log(&one.name).unwrap()[0].at, 0);
}

#[test]
fn an_update_summarises_the_spec_count() {
    let who = actor("tester");
    let store = FakeProfileStore::new();
    store
        .cas_put(&profile("Demo Alpha", &["demo-c1r1"]), 0, &who)
        .unwrap();
    store
        .cas_put(&profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"]), 1, &who)
        .unwrap();

    let log = store.log(&name("Demo Alpha")).unwrap();
    assert_eq!(
        log,
        vec![
            entry(0, 1, &who, ProfileChange::Created),
            entry(
                0,
                2,
                &who,
                ProfileChange::Updated {
                    summary: "pane specs: 1 -> 2".to_owned()
                }
            ),
        ]
    );
}

#[test]
fn the_submitted_slug_is_replaced_by_the_names_slug() {
    let store = FakeProfileStore::new();
    let wrong = Profile {
        slug: "wrong".to_owned(),
        ..profile("Demo Alpha", &["demo-c1r1"])
    };

    let stored = store.cas_put(&wrong, 0, &actor("tester")).unwrap();

    assert_eq!(stored.slug, "demo-alpha");
    assert_eq!(store.get(&wrong.name).unwrap().unwrap().slug, "demo-alpha");
}

#[test]
fn a_same_slug_other_name_is_profile_exists_at_any_generation() {
    let who = actor("tester");
    let alpha = profile("Demo Alpha", &["demo-c1r1"]);
    let store = FakeProfileStore::seeded([alpha.clone()], &who).unwrap();

    let shouting = profile("DEMO-ALPHA", &["demo-c1r1"]);
    let result = store.cas_put(&shouting, 1, &who);

    assert!(
        matches!(result, Err(PaneError::ProfileExists { .. })),
        "{result:?}"
    );
    assert_eq!(
        store.get(&alpha.name).unwrap().unwrap().name.as_str(),
        "Demo Alpha",
        "the stored display name is untouched"
    );
    assert_eq!(
        store.log(&alpha.name).unwrap().len(),
        1,
        "and nothing is logged"
    );
}

// --- another writer ---

#[test]
fn a_concurrent_put_makes_the_next_cas_stale() {
    let store = FakeProfileStore::new();
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let me = actor("tester");
    let other = actor("other");
    let g = store.cas_put(&a, 0, &me).unwrap().generation;

    let mut watch = open_watch(&store, 0);
    assert_eq!(drain(&mut watch).len(), 1);

    let theirs = store.concurrent_put(&a, &other).unwrap();
    assert_eq!(theirs.generation, g + 1);

    assert_eq!(store.cas_put(&a, g, &me), Err(PaneError::Conflict));
    let log = store.log(&a.name).unwrap();
    assert_eq!(log.last().unwrap().actor, other, "the log names the writer");
    assert_eq!(log.last().unwrap().generation, g + 1);
    assert_eq!(
        watch.next().map(|r| r.map(|o| o.map(|e| e.profile))),
        Some(Ok(Some(Some(Box::new(theirs))))),
        "a watch sees the concurrent write"
    );
}

#[test]
fn a_concurrent_put_creates_a_profile_at_one() {
    let store = FakeProfileStore::new();
    let other = actor("other");
    let stored = store
        .concurrent_put(&profile("Demo Alpha", &["demo-c1r1"]), &other)
        .unwrap();

    assert_eq!(stored.generation, 1);
    assert_eq!(store.get(&stored.name), Ok(Some(stored.clone())));
    assert_eq!(
        store.log(&stored.name),
        Ok(vec![entry(0, 1, &other, ProfileChange::Created)])
    );
}

#[test]
fn a_concurrent_delete_makes_the_profile_vanish() {
    let store = FakeProfileStore::new();
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let me = actor("tester");
    let other = actor("other");
    let g = store.cas_put(&a, 0, &me).unwrap().generation;
    let mut watch = open_watch(&store, 0);
    drain(&mut watch);

    store.concurrent_delete(&a.name, &other).unwrap();

    assert_eq!(store.get(&a.name), Ok(None));
    assert_eq!(store.delete(&a.name, g, &me), Err(not_found(&a.name)));
    assert_eq!(
        store.concurrent_delete(&a.name, &other),
        Err(not_found(&a.name))
    );
    let log = store.log(&a.name).unwrap();
    assert_eq!(
        log.last(),
        Some(&entry(0, g + 1, &other, ProfileChange::Deleted)),
        "the log keeps the other writer's Deleted entry"
    );
    let events = drain(&mut watch);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].name, a.name);
    assert_eq!(events[0].profile, None, "a delete carries no record");
}

// --- watch ---

#[test]
fn a_watch_ahead_of_the_head_is_usage() {
    let store = FakeProfileStore::new();
    assert!(matches!(
        watch_error(&store, 5),
        Some(PaneError::Usage { .. })
    ));
}

#[test]
fn the_idle_wait_wakes_on_a_write() {
    let store = Arc::new(FakeProfileStore::new());
    store.set_idle_wait(Duration::from_secs(5));
    let mut watch = open_watch(&store, 0);

    let writer = {
        let store = Arc::clone(&store);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            store
                .cas_put(&profile("Demo Alpha", &["demo-c1r1"]), 0, &actor("tester"))
                .unwrap()
        })
    };

    let started = Instant::now();
    let next = watch.next();
    let waited = started.elapsed();
    let written = writer.join().unwrap();

    assert_eq!(
        next.map(|r| r.map(|o| o.map(|e| e.profile))),
        Some(Ok(Some(Some(Box::new(written))))),
        "next() yields the write"
    );
    assert!(
        waited < Duration::from_secs(4),
        "waited the whole {waited:?}"
    );
}

// --- seeding ---

#[test]
fn seeded_stores_hold_each_profile_at_generation_one() {
    let seed = actor("seed");
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let b = profile("Demo Beta", &["demo-c2r1"]);
    let store = FakeProfileStore::seeded([b.clone(), a.clone()], &seed).unwrap();
    assert_eq!(store.faults().calls(), vec![], "seeding is not a call");

    for p in [&a, &b] {
        assert_eq!(store.get(&p.name).unwrap().unwrap().generation, 1);
        assert_eq!(
            store.log(&p.name).unwrap(),
            vec![entry(0, 1, &seed, ProfileChange::Created)],
            "one Created entry by the seed actor"
        );
    }
    assert_eq!(store.list().unwrap().len(), 2);
}

#[test]
fn seeding_the_same_name_twice_is_a_conflict() {
    let a = profile("Demo Alpha", &["demo-c1r1"]);
    let result = FakeProfileStore::seeded([a.clone(), a], &actor("seed"));
    assert_eq!(result.err(), Some(PaneError::Conflict));
}

#[test]
fn seeding_a_same_slug_name_is_profile_exists() {
    let result = FakeProfileStore::seeded(
        [
            profile("Demo Alpha", &["demo-c1r1"]),
            profile("DEMO-ALPHA", &["demo-c1r1"]),
        ],
        &actor("seed"),
    );
    assert!(
        matches!(result.err(), Some(PaneError::ProfileExists { .. })),
        "a second name with the slug is profile-exists, not generation-conflict"
    );
}

// --- the fixtures ---

#[test]
fn sample_profile_is_valid_and_deterministic() {
    let first = sample_profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"]).unwrap();
    assert_eq!(
        first,
        sample_profile("Demo Alpha", &["demo-c1r1", "demo-c2r1"]).unwrap()
    );
    assert_eq!(first.name.as_str(), "Demo Alpha");
    assert_eq!(first.slug, first.name.slug());
    assert_eq!(first.slug, "demo-alpha");
    assert_eq!((first.generation, first.created, first.updated), (0, 0, 0));
    let panes: Vec<&str> = first.panes.iter().map(|s| s.pane.as_str()).collect();
    assert_eq!(
        panes,
        ["demo-c1r1", "demo-c2r1"],
        "the specs follow `panes`"
    );
    assert_eq!(first.panes[0], sample_spec("demo-c1r1"));

    assert_eq!(sample_profile("Demo Alpha", &[]).unwrap().panes, vec![]);
    assert!(matches!(
        sample_profile("!!", &[]),
        Err(PaneError::Usage { .. })
    ));
}

#[test]
fn sample_spec_is_deterministic_and_harmless() {
    let spec = sample_spec("demo-c1r1");
    assert_eq!(spec.pane, "demo-c1r1");
    assert_eq!(spec, sample_spec("demo-c1r1"));
    assert_ne!(spec, sample_spec("demo-c2r1"));

    assert_eq!(spec.herdr.workspace, "scratch");
    assert_eq!(spec.herdr.grid.to_string(), "r1c1");
    assert_eq!(spec.host.cwd, "/srv/demo");
    assert_eq!(spec.harness.kind, HarnessKind::Opencode);
    assert_eq!(spec.harness.port_policy, "fixed");
    assert_eq!(spec.role, PaneRole::Agent);
    assert!(spec.env.is_empty());
    assert_eq!((spec.command.as_ref(), spec.check.as_ref()), (None, None));
    assert!(spec.expect.is_empty());
}

#[test]
fn sample_spec_agrees_with_sample_pane() {
    let spec = sample_spec("demo-c1r1");
    // The two fixtures are meant to agree, so a pane snapshot of a sample pane is its sample spec.
    let pane = sample_pane("demo-c1r1").unwrap();
    assert_eq!(spec.model, pane.model);
    assert_eq!(spec.context, pane.context);
    assert_eq!(spec.host.cwd, pane.host.cwd);
}

// --- bounds ---

#[test]
fn the_fake_is_send_and_sync_and_its_watch_is_send() {
    fn send_sync<T: Send + Sync>() {}
    fn send<T: Send>() {}
    send_sync::<FakeProfileStore>();
    send_sync::<FaultSwitch<ProfileStoreOp>>();
    send::<Watch<ProfileEvent>>();
    fn is_port_op<T: PortOp>() {}
    is_port_op::<ProfileStoreOp>();
}
