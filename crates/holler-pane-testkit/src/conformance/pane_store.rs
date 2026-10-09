//! The `PaneStore` conformance suite: [`run_pane_store_conformance`] runs the cases
//! [`pane_store_cases`] lists against any `PaneStore`, each against a fresh, empty
//! store.
//!
//! The cases pin the port's rules (`holler_pane::ports`; ADR-0021 sections 2, 6 and 8):
//! the compare-and-swap on a pane's generation, `delete` checking that the record
//! exists before it checks the generation, `list` in name order, the watch stream (the
//! current state from `Cursor(0)`, a resume with no gap and no repeat, idle as
//! `Ok(None)`), and the membership rule of `cas_put`. "Unchanged" in a case means that
//! `get` returns the record exactly as before and `list` is equal to before. The panes
//! come from [`sample_pane`], under neutral names.
//!
//! A store's own idle wait bounds each idle read, so a store with a long-poll window
//! (the hub's `watch_wait`) is run with a short one. The watch cases read one change
//! after each write: a watcher that resumes from `Cursor(0)` gets the current state,
//! not the writes in between, and that is all every correct store promises.

use std::fmt::Debug;

use holler_pane::{Cursor, Pane, PaneEvent, PaneName, PaneStore, ProfileName, Watch};

use super::{drain, expect_code, expect_eq, next_item, run_cases, succeeds, Conformance};
use crate::feed::Change;
use crate::fixture::sample_pane;

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&dyn PaneStore) -> Result<(), String>;

/// What `get` of one name and `list` show, compared before and after a refused write.
type Shown = (Option<Pane>, Vec<Pane>);

const CONFLICT: &str = "generation-conflict";
const NOT_FOUND: &str = "pane-not-found";
const IN_OTHER_PROFILE: &str = "pane-in-other-profile";

/// The pane names of the cases: neutral ones, never a live session's.
const C1: &str = "demo-c1r1";
const C2: &str = "demo-c2r1";
const C3: &str = "demo-c3r1";

/// The suite, in order: the one table that the runner iterates and
/// [`pane_store_cases`] lists.
const CASES: [(&str, Case); 19] = [
    ("get-missing-is-none", get_missing_is_none),
    ("list-empty", list_empty),
    ("create-at-zero-stored-at-one", create_at_zero_stored_at_one),
    ("submitted-generation-ignored", submitted_generation_ignored),
    (
        "create-over-existing-conflicts",
        create_over_existing_conflicts,
    ),
    ("update-bumps-by-one", update_bumps_by_one),
    ("stale-generation-conflicts", stale_generation_conflicts),
    ("expected-ahead-conflicts", expected_ahead_conflicts),
    ("list-sorted-by-name", list_sorted_by_name),
    ("delete-at-current-generation", delete_at_current_generation),
    ("delete-stale-conflicts", delete_stale_conflicts),
    (
        "delete-missing-is-pane-not-found",
        delete_missing_is_pane_not_found,
    ),
    (
        "recreate-after-delete-starts-at-one",
        recreate_after_delete_starts_at_one,
    ),
    (
        "watch-from-zero-yields-current-state",
        watch_from_zero_yields_current_state,
    ),
    ("watch-follows-each-change", watch_follows_each_change),
    (
        "watch-resumes-without-gap-or-repeat",
        watch_resumes_without_gap_or_repeat,
    ),
    (
        "watch-idle-is-ok-none-and-stays-usable",
        watch_idle_is_ok_none_and_stays_usable,
    ),
    ("failed-write-changes-nothing", failed_write_changes_nothing),
    ("pane-in-other-profile", pane_in_other_profile),
];

/// The ids of the cases [`run_pane_store_conformance`] runs, in the order it runs
/// them.
pub fn pane_store_cases() -> Vec<&'static str> {
    CASES.iter().map(|&(id, _)| id).collect()
}

/// Run every case of [`pane_store_cases`], in order, each against a fresh, empty
/// store, and return every case that did not hold.
///
/// `fresh` is called once per case. It returns the store and a guard that the suite
/// keeps alive for that case only: the temporary directory a store lives in, or `()`
/// for the fake. The store is dropped before its guard. How each implementation runs
/// the suite (the test kit cannot name the hub):
///
/// ```text
/// // the fake:
/// assert_eq!(run_pane_store_conformance(|| (FakePaneStore::new(), ())), Ok(()));
/// // the hub's registry (#649 or #661, in holler-hub/tests/):
/// run_pane_store_conformance(|| {
///     let dir = tempfile::tempdir().expect("temp dir");
///     let opts = PaneStoreOptions { watch_wait: Duration::from_millis(50), feed_retained: 1024 };
///     (PaneState::load_with(&HubState::from_root(dir.path().to_path_buf()), opts), dir)
/// })
/// // the CLI's pane/* client (#649): a fresh hub per case, the client as the store and
/// // the hub's handle as the guard.
/// ```
///
/// The case `pane-in-other-profile` pins the membership rule that ADR-0021 puts inside
/// the registry's compare-and-swap. The hub's registry gains it with #661, so the hub
/// passes every case only once #661 has merged. The suite offers no way to skip a case.
pub fn run_pane_store_conformance<S, K, F>(fresh: F) -> Conformance
where
    S: PaneStore,
    F: FnMut() -> (S, K),
{
    run_cases(&CASES, fresh, |case, store| case(store))
}

// --- the cases ---

/// Case 1: `get` of a name never stored is `Ok(None)`.
fn get_missing_is_none(store: &dyn PaneStore) -> Result<(), String> {
    let missing = pane_name(C1)?;
    expect_eq(
        "get of a name never stored",
        succeeds("get", store.get(&missing))?,
        None,
    )
}

/// Case 2: `list` of an empty store is empty.
fn list_empty(store: &dyn PaneStore) -> Result<(), String> {
    expect_eq(
        "list of an empty store",
        succeeds("list", store.list())?,
        Vec::new(),
    )
}

/// Case 3: a create names generation 0, and the record is stored at 1.
fn create_at_zero_stored_at_one(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    let stored = put(store, &p, 0)?;
    expect_eq("the record a create returns", &stored, &at(&p, 1))?;
    expect_eq(
        "get after the create",
        succeeds("get", store.get(&p.name))?,
        Some(stored),
    )
}

/// Case 4: the store sets the generation; the one a client submits is ignored.
fn submitted_generation_ignored(store: &dyn PaneStore) -> Result<(), String> {
    let p = Pane {
        generation: 99,
        ..sample(C1)?
    };
    let stored = put(store, &p, 0)?;
    expect_eq(
        "the generation a create submitted at 99 returns",
        stored.generation,
        1,
    )?;
    expect_eq(
        "get after the create",
        succeeds("get", store.get(&p.name))?,
        Some(at(&p, 1)),
    )
}

/// Case 5: a create over a stored record is `generation-conflict` and changes nothing.
fn create_over_existing_conflicts(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    put(store, &p, 0)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(p2, 0) over a record at 1";
    expect_code(call, store.cas_put(&revised(&p, 1), 0), CONFLICT)?;
    unchanged(store, &p.name, &before, call)
}

/// Case 6: a write at the current generation stores the record at the next one.
fn update_bumps_by_one(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    put(store, &p, 0)?;
    let p2 = revised(&p, 1);
    let stored = put(store, &p2, 1)?;
    expect_eq("the record an update at 1 returns", &stored, &at(&p2, 2))?;
    expect_eq(
        "get after the update",
        succeeds("get", store.get(&p.name))?,
        Some(stored),
    )
}

/// Case 7: a write at a generation the record has moved past is `generation-conflict`
/// and changes nothing.
fn stale_generation_conflicts(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    put(store, &p, 0)?;
    put(store, &revised(&p, 1), 1)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(p3, 1) on a record at 2";
    expect_code(call, store.cas_put(&revised(&p, 2), 1), CONFLICT)?;
    unchanged(store, &p.name, &before, call)
}

/// Case 8: a generation ahead of the record's is `generation-conflict` too, for a
/// stored record and for a name never stored.
fn expected_ahead_conflicts(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    put(store, &p, 0)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(p2, 5) on a record at 1";
    expect_code(call, store.cas_put(&revised(&p, 1), 5), CONFLICT)?;
    unchanged(store, &p.name, &before, call)?;
    let q = sample(C2)?;
    let call = "cas_put(q, 3) of a name never stored";
    expect_code(call, store.cas_put(&q, 3), CONFLICT)?;
    expect_eq(
        "get of q after its refused create",
        succeeds("get", store.get(&q.name))?,
        None,
    )
}

/// Case 9: `list` returns the stored records in name order (the `pane/list` reply of
/// ADR-0021 section 6), whatever order they were created in.
fn list_sorted_by_name(store: &dyn PaneStore) -> Result<(), String> {
    let c3 = put(store, &sample(C3)?, 0)?;
    let c1 = put(store, &sample(C1)?, 0)?;
    let c2 = put(store, &sample(C2)?, 0)?;
    expect_eq(
        "list after creating c3, c1 and c2 in that order",
        succeeds("list", store.list())?,
        vec![c1, c2, c3],
    )
}

/// Case 10: a delete at the current generation removes the record.
fn delete_at_current_generation(store: &dyn PaneStore) -> Result<(), String> {
    let p = put(store, &sample(C1)?, 0)?;
    succeeds("delete(p, 1)", store.delete(&p.name, 1))?;
    expect_eq(
        "get after the delete",
        succeeds("get", store.get(&p.name))?,
        None,
    )?;
    expect_eq(
        "list after the delete",
        succeeds("list", store.list())?,
        Vec::new(),
    )
}

/// Case 11: a delete at a stale generation is `generation-conflict` and changes
/// nothing.
fn delete_stale_conflicts(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    put(store, &p, 0)?;
    put(store, &revised(&p, 1), 1)?;
    let before = shown(store, &p.name)?;
    let call = "delete(p, 1) of a record at 2";
    expect_code(call, store.delete(&p.name, 1), CONFLICT)?;
    unchanged(store, &p.name, &before, call)
}

/// Case 12: a delete of a missing record is `pane-not-found` whatever the generation,
/// because existence is checked first.
fn delete_missing_is_pane_not_found(store: &dyn PaneStore) -> Result<(), String> {
    let missing = pane_name(C1)?;
    let at_zero = store.delete(&missing, 0);
    expect_code("delete of a missing name at 0", at_zero, NOT_FOUND)?;
    let at_seven = store.delete(&missing, 7);
    expect_code("delete of a missing name at 7", at_seven, NOT_FOUND)
}

/// Case 13: a record deleted and created again restarts at generation 1 (the known gap
/// of ADR-0021 section 8, pinned).
fn recreate_after_delete_starts_at_one(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    put(store, &p, 0)?;
    succeeds("delete(p, 1)", store.delete(&p.name, 1))?;
    let again = put(store, &p, 0)?;
    expect_eq(
        "the generation of the record created again",
        again.generation,
        1,
    )
}

/// Case 14: a watch from `Cursor(0)` yields a put of each record held now, equal to the
/// stored record, with increasing cursors, and nothing for a record deleted before it;
/// then it goes idle. The delete is the last write, so a store that replays the
/// history after the current state fails.
fn watch_from_zero_yields_current_state(store: &dyn PaneStore) -> Result<(), String> {
    let (a, b, c) = (sample(C1)?, sample(C2)?, sample(C3)?);
    put(store, &a, 0)?;
    put(store, &b, 0)?;
    let a2 = put(store, &revised(&a, 1), 1)?;
    let c1 = put(store, &c, 0)?;
    succeeds("delete(b, 1)", store.delete(&b.name, 1))?;
    let events = drain(&mut open(store, Cursor(0))?)?;
    increasing(&cursors(&events))?;
    expect_eq(
        "what watch(Cursor(0)) yields, by name",
        changes(events),
        vec![(a.name, Some(a2)), (c.name, Some(c1))],
    )
}

/// Case 15: on an empty store, a watch from `Cursor(0)` goes idle. Then after each
/// write, one `next()` yields exactly that change: the stored record for a put, no
/// record for the delete, with increasing cursors.
fn watch_follows_each_change(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    let mut watch = open(store, Cursor(0))?;
    expect_eq(
        "the first next() on an empty store",
        next_item(&mut watch)?,
        None,
    )?;
    let created = put(store, &p, 0)?;
    let first = expect_change(&mut watch, &p.name, Some(&created), "the create")?;
    let updated = put(store, &revised(&p, 1), 1)?;
    let second = expect_change(&mut watch, &p.name, Some(&updated), "the update")?;
    succeeds("delete(p, 2)", store.delete(&p.name, 2))?;
    let third = expect_change(&mut watch, &p.name, None, "the delete")?;
    increasing(&[first, second, third])
}

/// Case 16: a watch resumed from the cursor of the last change seen yields exactly the
/// changes made since, with no gap and no repeat.
fn watch_resumes_without_gap_or_repeat(store: &dyn PaneStore) -> Result<(), String> {
    put(store, &sample(C1)?, 0)?;
    put(store, &sample(C2)?, 0)?;
    let last = last_cursor(store)?;
    let d = put(store, &sample(C3)?, 0)?;
    let resumed = drain(&mut open(store, last)?)?;
    expect_eq(
        "what a watch resumed from the last cursor seen yields",
        changes(resumed),
        vec![(d.name.clone(), Some(d))],
    )
}

/// Case 17: an idle watch yields `Ok(None)`, neither ending nor failing, and the same
/// iterator then yields the next write.
fn watch_idle_is_ok_none_and_stays_usable(store: &dyn PaneStore) -> Result<(), String> {
    put(store, &sample(C1)?, 0)?;
    let mut watch = open(store, Cursor(0))?;
    drain(&mut watch)?;
    expect_eq("next() on a drained watch", next_item(&mut watch)?, None)?;
    let b = put(store, &sample(C2)?, 0)?;
    expect_change(&mut watch, &b.name, Some(&b), "a write to an idle watch")?;
    Ok(())
}

/// Case 18: a refused write (a stale `cas_put`, a stale `delete`, a `delete` of a
/// missing name) changes nothing and publishes no event.
fn failed_write_changes_nothing(store: &dyn PaneStore) -> Result<(), String> {
    let p = sample(C1)?;
    put(store, &p, 0)?;
    put(store, &revised(&p, 1), 1)?;
    let mut watch = open(store, last_cursor(store)?)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(p3, 1) on a record at 2";
    expect_code(call, store.cas_put(&revised(&p, 2), 1), CONFLICT)?;
    unchanged(store, &p.name, &before, call)?;
    let call = "delete(p, 1) of a record at 2";
    expect_code(call, store.delete(&p.name, 1), CONFLICT)?;
    unchanged(store, &p.name, &before, call)?;
    let call = "delete of a missing name";
    expect_code(call, store.delete(&pane_name(C2)?, 0), NOT_FOUND)?;
    unchanged(store, &p.name, &before, call)?;
    expect_eq(
        "next() after three refused writes",
        next_item(&mut watch)?,
        None,
    )
}

/// Case 19: a pane of one profile cannot be written into another
/// (`pane-in-other-profile`, and unchanged). It can leave its profile, and then join
/// the other one.
fn pane_in_other_profile(store: &dyn PaneStore) -> Result<(), String> {
    let p = Pane {
        profile: Some(profile_name("Alpha")?),
        ..sample(C1)?
    };
    put(store, &p, 0)?;
    let before = shown(store, &p.name)?;
    let to_beta = Pane {
        profile: Some(profile_name("Beta")?),
        ..p.clone()
    };
    let call = "cas_put moving a pane of Alpha to Beta";
    expect_code(call, store.cas_put(&to_beta, 1), IN_OTHER_PROFILE)?;
    unchanged(store, &p.name, &before, call)?;
    let left = Pane {
        profile: None,
        ..p.clone()
    };
    let stored = put(store, &left, 1)?;
    expect_eq("the record after leaving Alpha", &stored, &at(&left, 2))?;
    let stored = put(store, &to_beta, 2)?;
    expect_eq("the record after joining Beta", &stored, &at(&to_beta, 3))
}

// --- helpers ---

/// The fixture pane named `name`.
fn sample(name: &str) -> Result<Pane, String> {
    sample_pane(name).map_err(|e| format!("the fixture pane {name:?} cannot be built: {e}"))
}

fn pane_name(text: &str) -> Result<PaneName, String> {
    PaneName::parse(text).map_err(|e| format!("{text:?} is not a pane name: {e}"))
}

/// The profile name `text`. The profile store suite reuses it.
pub(super) fn profile_name(text: &str) -> Result<ProfileName, String> {
    ProfileName::parse(text).map_err(|e| format!("{text:?} is not a profile name: {e}"))
}

/// `pane` as a store holds it at `generation`.
fn at(pane: &Pane, generation: u64) -> Pane {
    Pane {
        generation,
        ..pane.clone()
    }
}

/// `pane` with one field changed, so that a write of it shows: revision `rev` moves
/// the harness port up by `rev`.
fn revised(pane: &Pane, rev: u16) -> Pane {
    let mut next = pane.clone();
    next.harness.port = pane.harness.port.saturating_add(rev);
    next
}

/// Write `pane` at `expected` (0 for a create), which must succeed; returns the
/// stored record.
fn put(store: &dyn PaneStore, pane: &Pane, expected: u64) -> Result<Pane, String> {
    let call = format!("cas_put({}, {expected})", pane.name);
    succeeds(&call, store.cas_put(pane, expected))
}

/// `get` of `name` and `list`, as the store shows them now.
fn shown(store: &dyn PaneStore, name: &PaneName) -> Result<Shown, String> {
    Ok((
        succeeds("get", store.get(name))?,
        succeeds("list", store.list())?,
    ))
}

/// `Ok` when the store shows `before` again after the refused `call`.
fn unchanged(
    store: &dyn PaneStore,
    name: &PaneName,
    before: &Shown,
    call: &str,
) -> Result<(), String> {
    let now = shown(store, name)?;
    expect_eq(&format!("get and list after {call}"), &now, before)
}

fn open(store: &dyn PaneStore, since: Cursor) -> Result<Watch<PaneEvent>, String> {
    succeeds(&format!("watch(Cursor({}))", since.0), store.watch(since))
}

/// The cursor of the last change a watch from `Cursor(0)` yields before it goes idle.
fn last_cursor(store: &dyn PaneStore) -> Result<Cursor, String> {
    let seen = drain(&mut open(store, Cursor(0))?)?;
    seen.last()
        .map(|event| event.cursor)
        .ok_or_else(|| "watch(Cursor(0)) yielded nothing on a store holding records".to_owned())
}

/// The next item of `watch` must be the change of the record filed under `key` (a
/// pane's name, a profile's slug) to `record` (`None` for its delete); returns the
/// change's cursor. `after` names the write, for the detail. The profile store suite
/// reuses it.
pub(super) fn expect_change<E>(
    watch: &mut Watch<E>,
    key: &E::Key,
    record: Option<&E::Record>,
    after: &str,
) -> Result<Cursor, String>
where
    E: Change + Debug,
    E::Key: Debug,
    E::Record: PartialEq + Debug,
{
    match next_item(watch)? {
        Some(event) if event.key() == *key && event.record() == record => Ok(event.cursor()),
        other => Err(format!(
            "next() after {after}: expected the change of {key:?} to {record:?}, got {other:?}"
        )),
    }
}

/// The changes `events` carry, as `(key, record)` pairs sorted by key (`None` for a
/// delete). The profile store suite reuses it.
pub(super) fn changes<E>(events: Vec<E>) -> Vec<(E::Key, Option<E::Record>)>
where
    E: Change,
    E::Record: Clone,
{
    let mut changes: Vec<_> = events
        .into_iter()
        .map(|event| (event.key(), event.record().cloned()))
        .collect();
    changes.sort_by(|x, y| x.0.cmp(&y.0));
    changes
}

/// The cursors of `events`, in order. The profile store suite reuses it.
pub(super) fn cursors<E: Change>(events: &[E]) -> Vec<Cursor> {
    events.iter().map(Change::cursor).collect()
}

/// `Ok` when `cursors` strictly increase. The profile store suite reuses it.
pub(super) fn increasing(cursors: &[Cursor]) -> Result<(), String> {
    if cursors
        .windows(2)
        .all(|pair| matches!(pair, [earlier, later] if earlier < later))
    {
        Ok(())
    } else {
        Err(format!("the cursors do not strictly increase: {cursors:?}"))
    }
}
