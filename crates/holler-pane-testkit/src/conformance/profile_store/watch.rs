//! The watch cases of the `ProfileStore` suite (cases 19 to 23): the current state
//! from `Cursor(0)`, one change per write, a resume with no gap and no repeat, idle as
//! `Ok(None)`, and no event for a refused write.
//!
//! They mirror cases 14 to 18 of the `PaneStore` suite and use its watch helpers
//! (`expect_change`, `changes`, `cursors` and `increasing`), which are generic over the
//! event. A profile's changes are keyed by the slug of its name.

use holler_pane::{Cursor, ProfileEvent, ProfileStore, Watch};

use super::{actor, put, revised, sample, shown, unchanged};
use super::{
    ALPHA, ALPHA_SHOUTED, BETA, C1, C2, C3, CONFLICT, EXISTS, GAMMA, NOT_FOUND, NOT_IMPLEMENTED,
};
use crate::conformance::pane_store::{changes, cursors, expect_change, increasing, profile_name};
use crate::conformance::{drain, expect_code, expect_eq, next_item, succeeds};

/// Case 19: a watch from `Cursor(0)` yields a put of each profile held now, keyed by
/// slug and equal to the stored record, with increasing cursors, and nothing for a
/// profile deleted before it; then it goes idle. The delete is the last write, so a
/// store that replays the history after the current state fails.
pub(super) fn watch_from_zero_yields_current_state(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let (a, b, c) = (
        sample(ALPHA, &[C1])?,
        sample(BETA, &[C2])?,
        sample(GAMMA, &[C3])?,
    );
    put(store, &a, 0, &who)?;
    put(store, &b, 0, &who)?;
    let a2 = put(store, &revised(&a, 1), 1, &who)?;
    let c1 = put(store, &c, 0, &who)?;
    succeeds("delete(b, 1)", store.delete(&b.name, 1, &who))?;
    let events = drain(&mut open(store, Cursor(0))?)?;
    increasing(&cursors(&events))?;
    expect_eq(
        "what watch(Cursor(0)) yields, by slug",
        changes(events),
        vec![(a.name.slug(), Some(a2)), (c.name.slug(), Some(c1))],
    )
}

/// Case 20: on an empty store, a watch from `Cursor(0)` goes idle. Then after each
/// write, one `next()` yields exactly that change: the stored record for a put, no
/// record for the delete, with increasing cursors.
pub(super) fn watch_follows_each_change(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    let slug = p.name.slug();
    let mut watch = open(store, Cursor(0))?;
    expect_eq(
        "the first next() on an empty store",
        next_item(&mut watch)?,
        None,
    )?;
    let created = put(store, &p, 0, &who)?;
    let first = expect_change(&mut watch, &slug, Some(&created), "the create")?;
    let updated = put(store, &revised(&p, 1), 1, &who)?;
    let second = expect_change(&mut watch, &slug, Some(&updated), "the update")?;
    succeeds("delete(p, 2)", store.delete(&p.name, 2, &who))?;
    let third = expect_change(&mut watch, &slug, None, "the delete")?;
    increasing(&[first, second, third])
}

/// Case 21: a watch resumed from the cursor of the last change seen yields exactly the
/// changes made since, with no gap and no repeat.
pub(super) fn watch_resumes_without_gap_or_repeat(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    put(store, &sample(ALPHA, &[C1])?, 0, &who)?;
    put(store, &sample(BETA, &[C2])?, 0, &who)?;
    let last = last_cursor(store)?;
    let d = put(store, &sample(GAMMA, &[C3])?, 0, &who)?;
    let resumed = drain(&mut open(store, last)?)?;
    expect_eq(
        "what a watch resumed from the last cursor seen yields",
        changes(resumed),
        vec![(d.name.slug(), Some(d))],
    )
}

/// Case 22: an idle watch yields `Ok(None)`, neither ending nor failing, and the same
/// iterator then yields the next write.
pub(super) fn watch_idle_is_ok_none_and_stays_usable(
    store: &dyn ProfileStore,
) -> Result<(), String> {
    let who = actor()?;
    put(store, &sample(ALPHA, &[C1])?, 0, &who)?;
    let mut watch = open(store, Cursor(0))?;
    drain(&mut watch)?;
    expect_eq("next() on a drained watch", next_item(&mut watch)?, None)?;
    let b = put(store, &sample(BETA, &[C2])?, 0, &who)?;
    let after = "a write to an idle watch";
    expect_change(&mut watch, &b.name.slug(), Some(&b), after)?;
    Ok(())
}

/// Case 23: a refused write (a stale `cas_put`, a stale `delete`, a `delete` of a
/// missing name, a create of another name with the profile's slug, a `rename`) changes
/// nothing, the log included, and publishes no event.
pub(super) fn failed_write_changes_nothing(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    put(store, &revised(&p, 1), 1, &who)?;
    let mut watch = open(store, last_cursor(store)?)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(p3, 1) on a profile at 2";
    expect_code(call, store.cas_put(&revised(&p, 2), 1, &who), CONFLICT)?;
    unchanged(store, &p.name, &before, call)?;
    let call = "delete(p, 1) of a profile at 2";
    expect_code(call, store.delete(&p.name, 1, &who), CONFLICT)?;
    unchanged(store, &p.name, &before, call)?;
    let call = "delete of a missing name";
    let missing = profile_name(BETA)?;
    expect_code(call, store.delete(&missing, 0, &who), NOT_FOUND)?;
    unchanged(store, &p.name, &before, call)?;
    let call = "cas_put(DEMO-ALPHA, 0) with Demo Alpha at 2";
    let shouted = sample(ALPHA_SHOUTED, &[C1])?;
    expect_code(call, store.cas_put(&shouted, 0, &who), EXISTS)?;
    unchanged(store, &p.name, &before, call)?;
    let call = "rename(p, q, 2) while rename is PROPOSED (#665)";
    let renamed = store.rename(&p.name, &missing, 2, &who);
    expect_code(call, renamed, NOT_IMPLEMENTED)?;
    unchanged(store, &p.name, &before, call)?;
    expect_eq(
        "next() after five refused writes",
        next_item(&mut watch)?,
        None,
    )
}

// --- helpers ---

fn open(store: &dyn ProfileStore, since: Cursor) -> Result<Watch<ProfileEvent>, String> {
    succeeds(&format!("watch(Cursor({}))", since.0), store.watch(since))
}

/// The cursor of the last change a watch from `Cursor(0)` yields before it goes idle.
fn last_cursor(store: &dyn ProfileStore) -> Result<Cursor, String> {
    let seen = drain(&mut open(store, Cursor(0))?)?;
    seen.last()
        .map(|event| event.cursor)
        .ok_or_else(|| "watch(Cursor(0)) yielded nothing on a store holding profiles".to_owned())
}
