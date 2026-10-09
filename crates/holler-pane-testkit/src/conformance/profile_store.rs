//! The `ProfileStore` conformance suite: [`run_profile_store_conformance`] runs the
//! cases [`profile_store_cases`] lists against any `ProfileStore`, each against a
//! fresh, empty store.
//!
//! The cases pin the port's rules (`holler_pane::profile`; ADR-0021 sections 7 and 8):
//! the compare-and-swap on a profile's generation, `delete` checking that the profile
//! exists before it checks the generation, one log entry per applied write and none
//! for a refused one, `rename` answering `not-implemented` while it is PROPOSED (#665),
//! an environment of names only, and the watch stream (the current state from
//! `Cursor(0)`, a resume with no gap and no repeat, idle as `Ok(None)`).
//!
//! They also fix four rules the port leaves open, which every profile registry (#661)
//! keeps too:
//!
//! - **The slug is the identity.** Two names with one slug (`ProfileName::slug`) are one
//!   profile, so a write of a name whose slug is stored under another name is
//!   `profile-exists`. That is checked before the generation: a create of such a name
//!   at 0 is `profile-exists`, not `generation-conflict`. A stored record's slug is its
//!   name's. The suite compares watch events by slug and pins nothing about how a store
//!   files its records.
//! - **A `Deleted` entry carries the deleted generation + 1.** Each applied write adds
//!   one (ADR-0021 section 8), a delete included, so the generations of one life of a
//!   profile rise strictly in its log. A re-create still names 0, as every create does.
//! - **A profile's log is never cut.** It is still readable after a delete and goes on
//!   across a re-create, so a name's history outlives the profile. Only `log` of a name
//!   never created is `profile-not-found`.
//! - **`list` has no order** (ADR-0021 pins the order of `pane/list` only), so the suite
//!   sorts it by slug before it compares.
//!
//! "Stored as" in a case means the record a write returns equals the profile written,
//! with the generation the case expects, the name's slug, and the `created` and
//! `updated` of the store's own reply: the suite pins no timestamp. "Unchanged" means
//! that `get` and `log` of the name and `list` are each equal to before. A case reads
//! the log as `(generation, actor, kind)`, and every `Updated` summary must be one
//! non-empty line; its wording is the store's. The profiles come from
//! [`sample_profile`], under neutral names, and every write is made by the actor
//! `conformance`.
//!
//! A store's own idle wait bounds each idle read, so a store with a long-poll window is
//! run with a short one. This file holds cases 1 to 14 and the helpers every case
//! uses. The child module `log` holds cases 15 to 18 (the log, `rename`, the env), and
//! `watch` holds the watch cases, 19 to 23, which use the watch helpers of the pane
//! suite.

mod log;
mod watch;

use holler_pane::{Actor, Profile, ProfileChange, ProfileLogEntry, ProfileName, ProfileStore};

use super::pane_store::profile_name;
use super::{expect_code, expect_eq, run_cases, succeeds, Conformance};
use crate::fixture::sample_profile;

/// One case: `Err` with the reason when it does not hold.
type Case = fn(&dyn ProfileStore) -> Result<(), String>;

/// What `get` of one name, `list` (sorted by slug) and `log` of that name show,
/// compared before and after a refused write.
type Shown = (Option<Profile>, Vec<Profile>, Vec<ProfileLogEntry>);

/// One log entry as a case reads it: the generation, the actor and the kind of change.
type Step = (u64, Actor, &'static str);

const CONFLICT: &str = "generation-conflict";
const NOT_FOUND: &str = "profile-not-found";
const EXISTS: &str = "profile-exists";
const NOT_IMPLEMENTED: &str = "not-implemented";

/// The kinds of change a case reads in a log.
const CREATED: &str = "created";
const UPDATED: &str = "updated";
const RENAMED: &str = "renamed";
const DELETED: &str = "deleted";

/// Who makes every write of the suite.
const ACTOR: &str = "conformance";

/// The profile names of the cases: neutral ones. `ALPHA_SHOUTED` has `ALPHA`'s slug.
const ALPHA: &str = "Demo Alpha";
const ALPHA_SHOUTED: &str = "DEMO-ALPHA";
const BETA: &str = "Demo Beta";
const GAMMA: &str = "Demo Gamma";

/// The pane names of the specs: neutral ones, never a live session's.
const C1: &str = "demo-c1r1";
const C2: &str = "demo-c2r1";
const C3: &str = "demo-c3r1";

/// The suite, in order: the one table that the runner iterates and
/// [`profile_store_cases`] lists.
const CASES: [(&str, Case); 23] = [
    ("get-missing-is-none", get_missing_is_none),
    ("list-empty", list_empty),
    ("create-at-zero-stored-at-one", create_at_zero_stored_at_one),
    ("submitted-generation-ignored", submitted_generation_ignored),
    (
        "create-over-existing-conflicts",
        create_over_existing_conflicts,
    ),
    (
        "same-slug-other-name-is-profile-exists",
        same_slug_other_name_is_profile_exists,
    ),
    ("update-bumps-by-one", update_bumps_by_one),
    ("stale-generation-conflicts", stale_generation_conflicts),
    ("expected-ahead-conflicts", expected_ahead_conflicts),
    ("list-holds-every-profile", list_holds_every_profile),
    ("delete-at-current-generation", delete_at_current_generation),
    ("delete-stale-conflicts", delete_stale_conflicts),
    (
        "delete-missing-is-profile-not-found",
        delete_missing_is_profile_not_found,
    ),
    (
        "recreate-after-delete-starts-at-one",
        recreate_after_delete_starts_at_one,
    ),
    (
        "log-is-append-only-and-oldest-first",
        log::log_is_append_only_and_oldest_first,
    ),
    (
        "log-of-never-created-is-profile-not-found",
        log::log_of_never_created_is_profile_not_found,
    ),
    ("rename-is-not-implemented", log::rename_is_not_implemented),
    ("env-is-names-only", log::env_is_names_only),
    (
        "watch-from-zero-yields-current-state",
        watch::watch_from_zero_yields_current_state,
    ),
    (
        "watch-follows-each-change",
        watch::watch_follows_each_change,
    ),
    (
        "watch-resumes-without-gap-or-repeat",
        watch::watch_resumes_without_gap_or_repeat,
    ),
    (
        "watch-idle-is-ok-none-and-stays-usable",
        watch::watch_idle_is_ok_none_and_stays_usable,
    ),
    (
        "failed-write-changes-nothing",
        watch::failed_write_changes_nothing,
    ),
];

/// The ids of the cases [`run_profile_store_conformance`] runs, in the order it runs
/// them.
pub fn profile_store_cases() -> Vec<&'static str> {
    CASES.iter().map(|&(id, _)| id).collect()
}

/// Run every case of [`profile_store_cases`], in order, each against a fresh, empty
/// store, and return every case that did not hold.
///
/// `fresh` is called once per case. It returns the store and a guard that the suite
/// keeps alive for that case only: the temporary directory a store lives in, or `()`
/// for the fake. The store is dropped before its guard. How each implementation runs
/// the suite (the test kit cannot name the hub):
///
/// ```text
/// // the fake:
/// assert_eq!(run_profile_store_conformance(|| (FakeProfileStore::new(), ())), Ok(()));
/// // the hub's profile registry (#661, in holler-hub/tests/): a fresh registry in a
/// // temporary directory per case, with a short watch window, and the temporary
/// // directory as the guard.
/// ```
///
/// The case `rename-is-not-implemented` pins `rename` as PROPOSED; #665 replaces it
/// when `rename` is confirmed. The suite offers no way to skip a case.
pub fn run_profile_store_conformance<S, K, F>(fresh: F) -> Conformance
where
    S: ProfileStore,
    F: FnMut() -> (S, K),
{
    run_cases(&CASES, fresh, |case, store| case(store))
}

// --- the cases ---

/// Case 1: `get` of a name never stored is `Ok(None)`.
fn get_missing_is_none(store: &dyn ProfileStore) -> Result<(), String> {
    let missing = profile_name(ALPHA)?;
    expect_eq(
        "get of a name never stored",
        succeeds("get", store.get(&missing))?,
        None,
    )
}

/// Case 2: `list` of an empty store is empty.
fn list_empty(store: &dyn ProfileStore) -> Result<(), String> {
    expect_eq(
        "list of an empty store",
        succeeds("list", store.list())?,
        Vec::new(),
    )
}

/// Case 3: a create names generation 0, the profile is stored at 1, and the log holds
/// one `created` entry by the writer.
fn create_at_zero_stored_at_one(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    let stored = put(store, &p, 0, &who)?;
    expect_eq(
        "the record a create returns",
        &stored,
        &stored_as(&p, 1, &stored),
    )?;
    expect_eq(
        "get after the create",
        succeeds("get", store.get(&p.name))?,
        Some(stored),
    )?;
    expect_eq(
        "the log after the create",
        history(store, &p.name)?,
        vec![(1, who, CREATED)],
    )
}

/// Case 4: the store sets the generation; the one a client submits is ignored.
fn submitted_generation_ignored(store: &dyn ProfileStore) -> Result<(), String> {
    let p = Profile {
        generation: 99,
        ..sample(ALPHA, &[C1])?
    };
    let stored = put(store, &p, 0, &actor()?)?;
    expect_eq(
        "the generation a create submitted at 99 returns",
        stored.generation,
        1,
    )?;
    expect_eq(
        "get after the create",
        succeeds("get", store.get(&p.name))?,
        Some(stored_as(&p, 1, &stored)),
    )
}

/// Case 5: a create over a stored profile is `generation-conflict` and changes
/// nothing, its log included.
fn create_over_existing_conflicts(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(p2, 0) over a profile at 1";
    expect_code(call, store.cas_put(&revised(&p, 1), 0, &who), CONFLICT)?;
    unchanged(store, &p.name, &before, call)
}

/// Case 6: a create of another name with a stored profile's slug is `profile-exists`
/// (the name rule comes before the generation) and changes nothing.
fn same_slug_other_name_is_profile_exists(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(DEMO-ALPHA, 0) with Demo Alpha at 1";
    let shouted = sample(ALPHA_SHOUTED, &[C1])?;
    expect_code(call, store.cas_put(&shouted, 0, &who), EXISTS)?;
    unchanged(store, &p.name, &before, call)
}

/// Case 7: a write at the current generation stores the profile at the next one and
/// logs an `updated` entry.
fn update_bumps_by_one(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    let p2 = revised(&p, 1);
    let stored = put(store, &p2, 1, &who)?;
    expect_eq(
        "the record an update at 1 returns",
        &stored,
        &stored_as(&p2, 2, &stored),
    )?;
    expect_eq(
        "get after the update",
        succeeds("get", store.get(&p.name))?,
        Some(stored),
    )?;
    expect_eq(
        "the log after the update",
        history(store, &p.name)?,
        vec![(1, who.clone(), CREATED), (2, who, UPDATED)],
    )
}

/// Case 8: a write at a generation the profile has moved past is
/// `generation-conflict` and changes nothing; the log gets no entry.
fn stale_generation_conflicts(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    put(store, &revised(&p, 1), 1, &who)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(p3, 1) on a profile at 2";
    expect_code(call, store.cas_put(&revised(&p, 2), 1, &who), CONFLICT)?;
    unchanged(store, &p.name, &before, call)
}

/// Case 9: a generation ahead of the profile's is `generation-conflict` too, for a
/// stored profile and for a name never stored, which then has neither a record nor a
/// log.
fn expected_ahead_conflicts(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    let before = shown(store, &p.name)?;
    let call = "cas_put(p2, 5) on a profile at 1";
    expect_code(call, store.cas_put(&revised(&p, 1), 5, &who), CONFLICT)?;
    unchanged(store, &p.name, &before, call)?;
    let q = sample(BETA, &[C2])?;
    let call = "cas_put(q, 3) of a name never stored";
    expect_code(call, store.cas_put(&q, 3, &who), CONFLICT)?;
    expect_eq(
        "get of q after its refused create",
        succeeds("get", store.get(&q.name))?,
        None,
    )?;
    expect_code(
        "log of q after its refused create",
        store.log(&q.name),
        NOT_FOUND,
    )
}

/// Case 10: `list` holds every stored profile, whatever order they were created in.
fn list_holds_every_profile(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let gamma = put(store, &sample(GAMMA, &[C3])?, 0, &who)?;
    let alpha = put(store, &sample(ALPHA, &[C1])?, 0, &who)?;
    let beta = put(store, &sample(BETA, &[C2])?, 0, &who)?;
    expect_eq(
        "list after creating Gamma, Alpha and Beta, sorted by slug",
        listed(store)?,
        vec![alpha, beta, gamma],
    )
}

/// Case 11: a delete at the current generation removes the profile, and its log,
/// still readable, ends with a `deleted` entry at the next generation.
fn delete_at_current_generation(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = put(store, &sample(ALPHA, &[C1])?, 0, &who)?;
    succeeds("delete(p, 1)", store.delete(&p.name, 1, &who))?;
    expect_eq(
        "get after the delete",
        succeeds("get", store.get(&p.name))?,
        None,
    )?;
    expect_eq(
        "list after the delete",
        succeeds("list", store.list())?,
        Vec::new(),
    )?;
    expect_eq(
        "the log after the delete",
        history(store, &p.name)?,
        vec![(1, who.clone(), CREATED), (2, who, DELETED)],
    )
}

/// Case 12: a delete at a stale generation is `generation-conflict` and changes
/// nothing: the profile is still at 2, with the same log.
fn delete_stale_conflicts(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    put(store, &revised(&p, 1), 1, &who)?;
    let before = shown(store, &p.name)?;
    let call = "delete(p, 1) of a profile at 2";
    expect_code(call, store.delete(&p.name, 1, &who), CONFLICT)?;
    unchanged(store, &p.name, &before, call)
}

/// Case 13: a delete of a missing profile is `profile-not-found` whatever the
/// generation, because existence is checked first.
fn delete_missing_is_profile_not_found(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let missing = profile_name(ALPHA)?;
    let at_zero = store.delete(&missing, 0, &who);
    expect_code("delete of a missing name at 0", at_zero, NOT_FOUND)?;
    let at_seven = store.delete(&missing, 7, &who);
    expect_code("delete of a missing name at 7", at_seven, NOT_FOUND)
}

/// Case 14: a profile deleted and created again restarts at generation 1 (the known
/// gap of ADR-0021 section 8, pinned), and its log goes on across the two lives.
fn recreate_after_delete_starts_at_one(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    succeeds("delete(p, 1)", store.delete(&p.name, 1, &who))?;
    let again = put(store, &p, 0, &who)?;
    expect_eq(
        "the generation of the profile created again",
        again.generation,
        1,
    )?;
    expect_eq(
        "the log after a create, a delete and a create",
        history(store, &p.name)?,
        vec![
            (1, who.clone(), CREATED),
            (2, who.clone(), DELETED),
            (1, who, CREATED),
        ],
    )
}

// --- helpers (the cases of `log` and `watch` use them too) ---

/// The actor of every write of the suite.
fn actor() -> Result<Actor, String> {
    Actor::parse(ACTOR).map_err(|e| format!("{ACTOR:?} is not an actor: {e}"))
}

/// The fixture profile named `name`, with one sample spec per entry of `panes`.
fn sample(name: &str, panes: &[&str]) -> Result<Profile, String> {
    sample_profile(name, panes)
        .map_err(|e| format!("the fixture profile {name:?} cannot be built: {e}"))
}

/// `profile` as a store holds it at `generation`: the name's slug, and the `created`
/// and `updated` of `reply`, the store's own record, because the suite pins no
/// timestamp.
fn stored_as(profile: &Profile, generation: u64, reply: &Profile) -> Profile {
    Profile {
        slug: profile.name.slug(),
        generation,
        created: reply.created,
        updated: reply.updated,
        ..profile.clone()
    }
}

/// `profile` with one spec field changed, so that a write of it shows: revision `rev`
/// moves the soft context ceiling of its first spec up by `rev`.
fn revised(profile: &Profile, rev: u32) -> Profile {
    let mut next = profile.clone();
    if let Some(spec) = next.panes.first_mut() {
        spec.context.soft = spec.context.soft.saturating_add(rev);
    }
    next
}

/// Write `profile` at `expected` (0 for a create) as `actor`, which must succeed;
/// returns the stored record.
fn put(
    store: &dyn ProfileStore,
    profile: &Profile,
    expected: u64,
    actor: &Actor,
) -> Result<Profile, String> {
    let call = format!("cas_put({}, {expected})", profile.name);
    succeeds(&call, store.cas_put(profile, expected, actor))
}

/// `list`, sorted by slug: the suite pins no order.
fn listed(store: &dyn ProfileStore) -> Result<Vec<Profile>, String> {
    let mut profiles = succeeds("list", store.list())?;
    profiles.sort_by_key(|profile| profile.name.slug());
    Ok(profiles)
}

/// The log of `name`, which must be readable.
fn entries(store: &dyn ProfileStore, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, String> {
    succeeds(&format!("log({name})"), store.log(name))
}

/// The log of `name` as `(generation, actor, kind)`. An `Updated` summary that is
/// empty or more than one line fails.
fn history(store: &dyn ProfileStore, name: &ProfileName) -> Result<Vec<Step>, String> {
    entries(store, name)?.iter().map(step).collect()
}

/// One log entry as `(generation, actor, kind)`.
fn step(entry: &ProfileLogEntry) -> Result<Step, String> {
    let kind = match &entry.change {
        ProfileChange::Created => CREATED,
        ProfileChange::Updated { summary } if summary.is_empty() => {
            return Err("an Updated entry's summary is empty".to_owned())
        }
        ProfileChange::Updated { summary } if summary.contains(['\n', '\r']) => {
            return Err(format!(
                "an Updated entry's summary is more than one line: {summary:?}"
            ))
        }
        ProfileChange::Updated { .. } => UPDATED,
        ProfileChange::Renamed { .. } => RENAMED,
        ProfileChange::Deleted => DELETED,
    };
    Ok((entry.generation, entry.actor.clone(), kind))
}

/// `get` and `log` of `name` and `list` (sorted by slug), as the store shows them now.
fn shown(store: &dyn ProfileStore, name: &ProfileName) -> Result<Shown, String> {
    Ok((
        succeeds("get", store.get(name))?,
        listed(store)?,
        entries(store, name)?,
    ))
}

/// `Ok` when the store shows `before` again after the refused `call`.
fn unchanged(
    store: &dyn ProfileStore,
    name: &ProfileName,
    before: &Shown,
    call: &str,
) -> Result<(), String> {
    let now = shown(store, name)?;
    expect_eq(&format!("get, list and log after {call}"), &now, before)
}
