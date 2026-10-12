//! The `rename` cases of the `ProfileStore` suite (cases 23 to 28, #665): the record half
//! of a rename, which every `ProfileStore` keeps, the fake, the hub's registry called as a
//! port and the hub's `profile/rename` over the wire alike (a registry with no member pane
//! has nothing else to move; the members' half is the hub's and is pinned there).
//!
//! A rename moves the record from the old name's slug to the new one's: the new name and
//! slug, the generation + 1, the old `created` and the same specs. The logs are filed by
//! slug, so the old name's log is closed with one `deleted` entry and the new name's gains
//! exactly one `renamed {from}` entry, both at the new generation; no entry is copied. The
//! feed yields the deletion of the old name, then the put of the new one. A rename that
//! changes only the spelling (one slug) is one `renamed` entry and one put.
//!
//! A refused rename changes nothing, as every refused write: `profile-not-found` for an old
//! name never stored (whatever the generation), `usage` for a new name equal to the stored
//! one, `profile-exists` for a new name whose slug another live profile holds (whatever the
//! generation), and `generation-conflict` for a stale or an ahead generation.

use holler_pane::{Profile, ProfileChange, ProfileName, ProfileStore};

use super::watch::{last_cursor, open};
use super::{actor, entries, history, put, revised, sample, shown, unchanged, Shown};
use super::{ALPHA, ALPHA_SHOUTED, BETA, C1, C2, CONFLICT, CREATED, DELETED, EXISTS, NOT_FOUND};
use super::{RENAMED, UPDATED};
use crate::conformance::pane_store::{expect_change, increasing, profile_name};
use crate::conformance::{expect_code, expect_eq, next_item, succeeds};

const USAGE: &str = "usage";

/// `BETA`'s slug in other spellings.
const BETA_SHOUTED: &str = "DEMO-BETA";

/// Case 23: a rename moves the record: `get` finds it under the new name, and under any
/// spelling of the new slug, and no longer under the old one; it has the new name and slug,
/// the generation + 1, the old `created` and the same specs, and the reply is the stored
/// record.
pub(super) fn rename_moves_the_record(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1, C2])?;
    put(store, &p, 0, &who)?;
    let before = put(store, &revised(&p, 1), 1, &who)?;
    let q = profile_name(BETA)?;
    let renamed = succeeds("rename(p, q, 2)", store.rename(&p.name, &q, 2, &who))?;
    expect_eq(
        "the record a rename returns, but for `updated`",
        &Profile {
            updated: before.updated,
            ..renamed.clone()
        },
        &Profile {
            name: q.clone(),
            slug: q.slug(),
            generation: 3,
            ..before.clone()
        },
    )?;
    if renamed.updated < before.updated {
        return Err(format!(
            "the renamed record's `updated` ({}) is before the last write's ({})",
            renamed.updated, before.updated
        ));
    }
    for spelling in [BETA, BETA_SHOUTED] {
        expect_eq(
            &format!("get({spelling}) after the rename"),
            succeeds("get", store.get(&profile_name(spelling)?))?,
            Some(renamed.clone()),
        )?;
    }
    expect_eq(
        "get of the old name after the rename",
        succeeds("get", store.get(&p.name))?,
        None,
    )?;
    expect_eq(
        "list after the rename",
        succeeds("list", store.list())?,
        vec![renamed],
    )
}

/// Case 24: the old name's log is its entries followed by one `deleted` entry at the new
/// generation, and the new name's log is exactly one `renamed` entry, from the old stored
/// name, at the new generation, by the writer of the rename. No entry is copied, and `at`
/// never decreases.
pub(super) fn rename_logs_renamed_and_closes_the_old_log(
    store: &dyn ProfileStore,
) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    put(store, &revised(&p, 1), 1, &who)?;
    let old_before = entries(store, &p.name)?;
    let q = profile_name(BETA)?;
    succeeds("rename(p, q, 2)", store.rename(&p.name, &q, 2, &who))?;
    let old_after = entries(store, &p.name)?;
    expect_eq(
        "the old name's log, but for its last entry",
        old_after.get(..old_before.len()),
        Some(old_before.as_slice()),
    )?;
    expect_eq(
        "the old name's log after the rename",
        history(store, &p.name)?,
        vec![
            (1, who.clone(), CREATED),
            (2, who.clone(), UPDATED),
            (3, who.clone(), DELETED),
        ],
    )?;
    let new_log = entries(store, &q)?;
    expect_eq(
        "the new name's log after the rename",
        history(store, &q)?,
        vec![(3, who, RENAMED)],
    )?;
    expect_eq(
        "the change of the new name's one entry",
        new_log.first().map(|entry| &entry.change),
        Some(&ProfileChange::Renamed {
            from: p.name.clone(),
        }),
    )?;
    let closed = old_after.last().map(|entry| entry.at);
    let opened = new_log.first().map(|entry| entry.at);
    expect_eq(
        "the `at` of the deleted entry and of the renamed entry (one stamp)",
        closed,
        opened,
    )?;
    if old_after.windows(2).any(|pair| pair[0].at > pair[1].at) {
        return Err(format!("the old name's `at` decreases: {old_after:?}"));
    }
    Ok(())
}

/// Case 25: a rename onto the name of a profile deleted earlier continues that name's log:
/// its old entries, then the `renamed` entry.
pub(super) fn rename_onto_a_deleted_name_continues_its_log(
    store: &dyn ProfileStore,
) -> Result<(), String> {
    let who = actor()?;
    let gone = put(store, &sample(BETA, &[C2])?, 0, &who)?;
    succeeds("delete(q, 1)", store.delete(&gone.name, 1, &who))?;
    let p = put(store, &sample(ALPHA, &[C1])?, 0, &who)?;
    let renamed = succeeds(
        "rename(p, q, 1)",
        store.rename(&p.name, &gone.name, 1, &who),
    )?;
    expect_eq("the renamed record's generation", renamed.generation, 2)?;
    expect_eq(
        "the log of the reused name",
        history(store, &gone.name)?,
        vec![
            (1, who.clone(), CREATED),
            (2, who.clone(), DELETED),
            (2, who, RENAMED),
        ],
    )
}

/// Case 26: a rename that only changes the spelling (one slug) is one record, one
/// `renamed` entry on the one log, and one put on the feed. The watch is opened at the head
/// and read idle before the rename, as in case 19: a store may answer the first `next()`
/// with the idle window that passed while `watch()` opened (the hub client does, #649).
pub(super) fn rename_of_one_slug_is_one_put(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = put(store, &sample(ALPHA, &[C1])?, 0, &who)?;
    let mut watch = open(store, last_cursor(store)?)?;
    expect_eq(
        "the first next() of a watch at the head",
        next_item(&mut watch)?,
        None,
    )?;
    let shouted = profile_name(ALPHA_SHOUTED)?;
    let renamed = succeeds(
        "rename(p, DEMO-ALPHA, 1)",
        store.rename(&p.name, &shouted, 1, &who),
    )?;
    expect_eq("the renamed record's name", &renamed.name, &shouted)?;
    expect_eq("the renamed record's slug", &renamed.slug, &p.slug)?;
    expect_eq("the renamed record's generation", renamed.generation, 2)?;
    expect_eq(
        "get of either spelling after the rename",
        succeeds("get", store.get(&p.name))?,
        Some(renamed.clone()),
    )?;
    expect_eq(
        "the one log after the rename",
        history(store, &p.name)?,
        vec![(1, who.clone(), CREATED), (2, who, RENAMED)],
    )?;
    expect_change(&mut watch, &p.slug, Some(&renamed), "a one-slug rename")?;
    expect_eq(
        "next() after the one put of a one-slug rename",
        next_item(&mut watch)?,
        None,
    )
}

/// Case 27: the feed yields the deletion of the old name, then the put of the new one, at
/// increasing cursors, and nothing else. The watch is read idle before the rename, as in
/// case 26.
pub(super) fn rename_feed_is_deletion_then_put(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = put(store, &sample(ALPHA, &[C1])?, 0, &who)?;
    let mut watch = open(store, last_cursor(store)?)?;
    expect_eq(
        "the first next() of a watch at the head",
        next_item(&mut watch)?,
        None,
    )?;
    let q = profile_name(BETA)?;
    let renamed = succeeds("rename(p, q, 1)", store.rename(&p.name, &q, 1, &who))?;
    let deleted = expect_change(&mut watch, &p.slug, None, "the rename (the old name)")?;
    let added = expect_change(
        &mut watch,
        &q.slug(),
        Some(&renamed),
        "the rename (the new)",
    )?;
    increasing(&[deleted, added])?;
    expect_eq(
        "next() after the two changes of a rename",
        next_item(&mut watch)?,
        None,
    )
}

/// Case 28: a refused rename changes nothing (the record, the logs, `list`) and publishes no
/// event: an old name never stored at 0 and at 7 (`profile-not-found`), the stored name
/// itself (`usage`), a new name whose slug another live profile holds, in two spellings and
/// at a stale generation too (`profile-exists`), and a stale and an ahead generation
/// (`generation-conflict`).
pub(super) fn rename_refusals_change_nothing(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    put(store, &revised(&p, 1), 1, &who)?;
    let taken = put(store, &sample(BETA, &[C2])?, 0, &who)?;
    let mut watch = open(store, last_cursor(store)?)?;
    let before = both_shown(store, &p.name, &taken.name)?;
    let missing = profile_name("Demo Missing")?;
    let fresh = profile_name("Demo Fresh")?;
    let shouted = profile_name(BETA_SHOUTED)?;
    let refusals: [(&str, &ProfileName, &ProfileName, u64, &str); 8] = [
        ("rename(missing, fresh, 0)", &missing, &fresh, 0, NOT_FOUND),
        ("rename(missing, fresh, 7)", &missing, &fresh, 7, NOT_FOUND),
        ("rename(p, p, 2)", &p.name, &p.name, 2, USAGE),
        ("rename(p, Demo Beta, 2)", &p.name, &taken.name, 2, EXISTS),
        ("rename(p, DEMO-BETA, 2)", &p.name, &shouted, 2, EXISTS),
        ("rename(p, Demo Beta, 1)", &p.name, &taken.name, 1, EXISTS),
        (
            "rename(p, fresh, 1) of a profile at 2",
            &p.name,
            &fresh,
            1,
            CONFLICT,
        ),
        (
            "rename(p, fresh, 5) of a profile at 2",
            &p.name,
            &fresh,
            5,
            CONFLICT,
        ),
    ];
    for (call, from, to, expected, code) in refusals {
        expect_code(call, store.rename(from, to, expected, &who), code)?;
        let now = both_shown(store, &p.name, &taken.name)?;
        expect_eq(&format!("both profiles after {call}"), &now, &before)?;
        expect_eq(
            &format!("get of the new name after {call}"),
            succeeds("get", store.get(&fresh))?,
            None,
        )?;
    }
    unchanged(store, &p.name, &before.0, "the refused renames")?;
    expect_eq(
        "next() after eight refused renames",
        next_item(&mut watch)?,
        None,
    )
}

/// What `get`, `list` and `log` show of two names.
fn both_shown(
    store: &dyn ProfileStore,
    one: &ProfileName,
    other: &ProfileName,
) -> Result<(Shown, Shown), String> {
    Ok((shown(store, one)?, shown(store, other)?))
}
