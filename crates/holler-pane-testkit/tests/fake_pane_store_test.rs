#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #638
//! `FakePaneStore`'s own mechanisms: fault injection (a wedged or corrupt store, a
//! one-shot error, a slow call), the call log, another writer's change, the idle
//! wait and the fixture. The generic port rules (CAS, ordering, watch) are the
//! conformance suite's job; this file pins what only the fake has (#638, slice a).

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use holler_pane::{Cursor, Pane, PaneError, PaneEvent, PaneName, PaneStore, Watch};
use holler_pane_testkit::fault::{Fault, FaultSwitch, PortOp};
use holler_pane_testkit::fixture::sample_pane;
use holler_pane_testkit::pane_store::{FakePaneStore, PaneStoreOp};

fn pane(name: &str) -> Pane {
    sample_pane(name).unwrap()
}

fn name(text: &str) -> PaneName {
    PaneName::parse(text).unwrap()
}

fn timeout(op: &str) -> PaneError {
    PaneError::Timeout { op: op.to_owned() }
}

fn corrupt() -> PaneError {
    PaneError::StoreCorrupt {
        what: "pane store".to_owned(),
    }
}

fn unavailable(what: &str) -> PaneError {
    PaneError::Unavailable {
        what: what.to_owned(),
    }
}

/// `watch` without needing `Watch` to be `Debug`.
fn open_watch(store: &FakePaneStore, since: u64) -> Watch<PaneEvent> {
    match store.watch(Cursor(since)) {
        Ok(watch) => watch,
        Err(e) => panic!("watch({since}) failed: {e:?}"),
    }
}

fn watch_error(store: &FakePaneStore, since: u64) -> Option<PaneError> {
    store.watch(Cursor(since)).err()
}

/// Everything the watch yields until the first idle `Ok(None)`.
fn drain(watch: &mut Watch<PaneEvent>) -> Vec<PaneEvent> {
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

// --- wedged ---

#[test]
fn a_wedged_store_times_out_every_method() {
    let store = FakePaneStore::new();
    let a = pane("demo-c1r1");
    store.faults().set(Some(Fault::Wedged));

    assert_eq!(store.get(&a.name), Err(timeout("pane_store.get")));
    assert_eq!(store.list(), Err(timeout("pane_store.list")));
    assert_eq!(store.cas_put(&a, 0), Err(timeout("pane_store.cas_put")));
    assert_eq!(store.delete(&a.name, 1), Err(timeout("pane_store.delete")));
    assert_eq!(watch_error(&store, 0), Some(timeout("pane_store.watch")));

    store.faults().set(None);
    assert_eq!(store.list(), Ok(vec![]), "the wedged cas_put wrote nothing");
    assert_eq!(store.cas_put(&a, 0).unwrap().generation, 1);
    assert!(store.get(&a.name).unwrap().is_some());
}

#[test]
fn a_wedged_store_ends_an_open_watch() {
    let store = FakePaneStore::seeded([pane("demo-c1r1")]).unwrap();
    let mut watch = open_watch(&store, 0);

    store.faults().set(Some(Fault::Wedged));
    assert_eq!(
        watch.next(),
        Some(Err(timeout("pane_store.watch_next"))),
        "a wedged store is a timeout, never idle"
    );

    store.faults().set(None);
    assert_eq!(watch.next(), None, "any error ends the stream");
}

// --- corrupt ---

#[test]
fn a_corrupt_store_fails_closed_everywhere() {
    let a = pane("demo-c1r1");
    let store = FakePaneStore::seeded([a.clone()]).unwrap();
    let before = store.get(&a.name).unwrap().unwrap();

    store.faults().set(Some(Fault::Fail(corrupt())));
    assert_eq!(store.get(&a.name), Err(corrupt()));
    assert_eq!(store.list(), Err(corrupt()));
    assert_eq!(store.cas_put(&a, 1), Err(corrupt()));
    assert_eq!(store.delete(&a.name, 1), Err(corrupt()));
    assert_eq!(watch_error(&store, 0), Some(corrupt()));

    store.faults().set(None);
    assert_eq!(
        store.get(&a.name),
        Ok(Some(before)),
        "the records written before are intact"
    );
}

// --- one-shot errors ---

#[test]
fn fail_next_is_one_shot_and_per_op() {
    let store = FakePaneStore::new();
    let a = pane("demo-c1r1");
    store
        .faults()
        .fail_next(PaneStoreOp::CasPut, unavailable("hub"));

    assert_eq!(store.cas_put(&a, 0), Err(unavailable("hub")));
    assert_eq!(store.get(&a.name), Ok(None), "get is not the targeted op");
    assert_eq!(store.list(), Ok(vec![]), "the failed cas_put wrote nothing");

    let stored = store.cas_put(&a, 0).unwrap();
    assert_eq!(stored.generation, 1, "the second cas_put succeeds");
}

#[test]
fn queued_one_shot_errors_come_out_in_order() {
    let store = FakePaneStore::new();
    let missing = name("demo-c1r1");
    store
        .faults()
        .fail_next(PaneStoreOp::Get, unavailable("first"));
    store
        .faults()
        .fail_next(PaneStoreOp::Get, unavailable("second"));

    assert_eq!(store.get(&missing), Err(unavailable("first")));
    assert_eq!(store.get(&missing), Err(unavailable("second")));
    assert_eq!(store.get(&missing), Ok(None));
}

// --- slow ---

#[test]
fn a_slow_call_takes_at_least_the_delay() {
    let store = FakePaneStore::new();
    store.faults().set_delay(Some(Duration::from_millis(50)));

    let started = Instant::now();
    let result = store.get(&name("demo-c1r1"));
    let took = started.elapsed();

    assert_eq!(result, Ok(None));
    assert!(took >= Duration::from_millis(50), "took only {took:?}");
}

// --- the call log ---

#[test]
fn calls_are_recorded_in_order() {
    let store = FakePaneStore::new();
    let a = pane("demo-c1r1");
    store
        .faults()
        .fail_next(PaneStoreOp::CasPut, unavailable("hub"));

    store.get(&a.name).unwrap();
    assert!(store.cas_put(&a, 0).is_err());
    store.list().unwrap();
    let expected = vec![PaneStoreOp::Get, PaneStoreOp::CasPut, PaneStoreOp::List];
    assert_eq!(
        store.faults().calls(),
        expected,
        "failed calls are recorded too"
    );

    store.concurrent_put(&a).unwrap();
    store.concurrent_delete(&a.name).unwrap();
    assert_eq!(
        store.faults().calls(),
        expected,
        "another writer is not a call through the port"
    );
}

#[test]
fn a_seeded_store_starts_with_an_empty_call_log() {
    let store = FakePaneStore::seeded([pane("demo-c1r1")]).unwrap();
    assert_eq!(store.faults().calls(), vec![]);
}

#[test]
fn port_op_names_are_port_dot_method() {
    assert_eq!(PaneStoreOp::Get.as_str(), "pane_store.get");
    assert_eq!(PaneStoreOp::List.as_str(), "pane_store.list");
    assert_eq!(PaneStoreOp::CasPut.as_str(), "pane_store.cas_put");
    assert_eq!(PaneStoreOp::Delete.as_str(), "pane_store.delete");
    assert_eq!(PaneStoreOp::Watch.as_str(), "pane_store.watch");
    assert_eq!(PaneStoreOp::WatchNext.as_str(), "pane_store.watch_next");
}

// --- another writer ---

#[test]
fn a_concurrent_put_makes_the_next_cas_stale() {
    let store = FakePaneStore::new();
    let a = pane("demo-c1r1");
    let read = store.cas_put(&a, 0).unwrap();
    let g = read.generation;

    let mut watch = open_watch(&store, 0);
    let seen = drain(&mut watch);
    assert_eq!(seen.len(), 1);

    let theirs = store.concurrent_put(&a).unwrap();
    assert_eq!(theirs.generation, g + 1);

    assert_eq!(store.cas_put(&a, g), Err(PaneError::Conflict));
    let event = watch.next();
    assert_eq!(
        event.map(|r| r.map(|o| o.map(|e| e.pane))),
        Some(Ok(Some(Some(Box::new(theirs))))),
        "a watch sees the concurrent write"
    );
}

#[test]
fn a_concurrent_put_creates_a_record_at_one() {
    let store = FakePaneStore::new();
    let stored = store.concurrent_put(&pane("demo-c1r1")).unwrap();
    assert_eq!(stored.generation, 1);
    assert_eq!(store.get(&stored.name), Ok(Some(stored)));
}

#[test]
fn a_concurrent_delete_makes_the_record_vanish() {
    let store = FakePaneStore::new();
    let a = pane("demo-c1r1");
    let g = store.cas_put(&a, 0).unwrap().generation;
    let mut watch = open_watch(&store, 0);
    drain(&mut watch);

    store.concurrent_delete(&a.name).unwrap();

    assert_eq!(store.get(&a.name), Ok(None));
    assert_eq!(
        store.delete(&a.name, g),
        Err(PaneError::PaneNotFound {
            what: a.name.to_string()
        })
    );
    assert_eq!(
        store.concurrent_delete(&a.name),
        Err(PaneError::PaneNotFound {
            what: a.name.to_string()
        })
    );
    let events = drain(&mut watch);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].name, a.name);
    assert_eq!(events[0].pane, None, "a delete carries no record");
}

// --- watch ---

#[test]
fn a_watch_ahead_of_the_head_is_usage() {
    let store = FakePaneStore::new();
    assert!(matches!(
        watch_error(&store, 5),
        Some(PaneError::Usage { .. })
    ));
}

#[test]
fn a_watch_at_the_head_is_idle_not_usage() {
    let store = FakePaneStore::new();
    store.cas_put(&pane("demo-c1r1"), 0).unwrap();
    let head = drain(&mut open_watch(&store, 0)).last().unwrap().cursor;

    let mut watch = open_watch(&store, head.0);
    assert_eq!(watch.next(), Some(Ok(None)));
}

#[test]
fn a_watch_from_zero_resumes_from_the_head_as_of_the_snapshot() {
    // The hub replies with the head after a snapshot, so a record deleted last
    // leaves no put and no delete in the stream (a fake that resumed from the last
    // yielded cursor would replay it and differ from production).
    let store = FakePaneStore::new();
    store.cas_put(&pane("demo-c1r1"), 0).unwrap();
    store.cas_put(&pane("demo-c3r1"), 0).unwrap();
    let b = store.cas_put(&pane("demo-c2r1"), 0).unwrap();
    store.delete(&b.name, 1).unwrap();

    let events = drain(&mut open_watch(&store, 0));

    let names: Vec<&str> = events.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["demo-c1r1", "demo-c3r1"]);
    assert!(events.iter().all(|e| e.pane.is_some()), "puts only");
    assert!(events.windows(2).all(|w| w[0].cursor < w[1].cursor));
}

#[test]
fn the_idle_wait_wakes_on_a_write() {
    let store = Arc::new(FakePaneStore::new());
    store.set_idle_wait(Duration::from_secs(5));
    let mut watch = open_watch(&store, 0);

    let writer = {
        let store = Arc::clone(&store);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            store.cas_put(&pane("demo-c1r1"), 0).unwrap()
        })
    };

    let started = Instant::now();
    let next = watch.next();
    let waited = started.elapsed();
    let written = writer.join().unwrap();

    assert_eq!(
        next.map(|r| r.map(|o| o.map(|e| e.pane))),
        Some(Ok(Some(Some(Box::new(written))))),
        "next() yields the write"
    );
    assert!(
        waited < Duration::from_secs(4),
        "waited the whole {waited:?}"
    );
}

#[test]
fn the_default_idle_wait_is_zero() {
    let store = FakePaneStore::new();
    let mut watch = open_watch(&store, 0);

    let started = Instant::now();
    let next = watch.next();
    let waited = started.elapsed();

    assert_eq!(next, Some(Ok(None)));
    assert!(waited < Duration::from_secs(2), "took {waited:?}");
}

// --- seeding and the fixture ---

#[test]
fn seeded_stores_hold_each_pane_at_generation_one() {
    let a = pane("demo-c1r1");
    let b = pane("demo-c2r1");
    let store = FakePaneStore::seeded([b.clone(), a.clone()]).unwrap();

    assert_eq!(store.get(&a.name).unwrap().unwrap().generation, 1);
    assert_eq!(store.get(&b.name).unwrap().unwrap().generation, 1);
    let listed: Vec<String> = store
        .list()
        .unwrap()
        .iter()
        .map(|p| p.name.to_string())
        .collect();
    assert_eq!(listed, ["demo-c1r1", "demo-c2r1"]);
}

#[test]
fn seeding_the_same_name_twice_is_a_conflict() {
    // Each seed is a create at expected generation 0.
    let result = FakePaneStore::seeded([pane("demo-c1r1"), pane("demo-c1r1")]);
    assert_eq!(result.err(), Some(PaneError::Conflict));
}

#[test]
fn sample_pane_is_valid_and_deterministic() {
    let first = sample_pane("demo-c1r1").unwrap();
    assert_eq!(first, sample_pane("demo-c1r1").unwrap());
    assert_eq!(first.name.as_str(), "demo-c1r1");
    assert_eq!(first.generation, 0);
    assert_eq!(first.profile, None);
    assert_eq!(first.session_of_record, None);
    assert_eq!(first.harness.port, 48100);
    assert_eq!(first.herdr.grid.to_string(), "r1c1");

    assert_ne!(first, sample_pane("demo-c2r1").unwrap());
    assert!(matches!(
        sample_pane("not a name"),
        Err(PaneError::Usage { .. })
    ));
}

#[test]
fn sample_pane_never_names_a_live_session() {
    let p = sample_pane("demo-c1r1").unwrap();
    for field in [&p.herdr.session, &p.herdr.workspace, &p.host.tmux] {
        assert!(!field.starts_with("hj"), "`{field}` looks like a live name");
    }
}

// --- bounds ---

#[test]
fn the_fake_is_send_and_sync_and_its_watch_is_send() {
    fn send_sync<T: Send + Sync>() {}
    fn send<T: Send>() {}
    send_sync::<FakePaneStore>();
    send_sync::<FaultSwitch<PaneStoreOp>>();
    send::<Watch<PaneEvent>>();
    fn is_port_op<T: PortOp>() {}
    is_port_op::<PaneStoreOp>();
}
