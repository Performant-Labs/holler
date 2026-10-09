//! Cases 15 to 18 of the `ProfileStore` suite, in a file of their own so that no file
//! of the suite nears the 600-line lint: the change log (it only grows, oldest first,
//! and is `profile-not-found` only for a name never created), `rename` while it is
//! PROPOSED (#665), and an environment of names only.

use holler_pane::{EnvVarName, Profile, ProfileLogEntry, ProfileSpec, ProfileStore};

use super::{actor, entries, history, put, revised, sample, shown, stored_as, unchanged};
use super::{ALPHA, BETA, C1, DELETED, NOT_FOUND, NOT_IMPLEMENTED};
use crate::conformance::pane_store::profile_name;
use crate::conformance::{expect_code, expect_eq, succeeds};
use crate::fixture::sample_spec;

const SECRET_REFUSED: &str = "profile-secret-refused";
const NAME_INVALID: &str = "env-name-invalid";

/// The env names case 18 stores: names only, one of them in lower case with a dot.
const ENV_NAMES: [&str; 3] = ["ANTHROPIC_API_KEY", "TOKEN", "lower.dotted"];

/// Case 15: the log only grows, oldest first: a later read starts with every entry of
/// an earlier one, the generations rise one per write, `at` never decreases, and a
/// delete leaves the log readable with its `deleted` entry last.
pub(super) fn log_is_append_only_and_oldest_first(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    put(store, &revised(&p, 1), 1, &who)?;
    put(store, &revised(&p, 2), 2, &who)?;
    let earlier = entries(store, &p.name)?;
    let what = "the entries after a create and two updates";
    expect_eq(what, earlier.len(), 3)?;
    put(store, &revised(&p, 3), 3, &who)?;
    let later = entries(store, &p.name)?;
    expect_eq(
        "the first three entries after one more update",
        later.get(..3),
        Some(earlier.as_slice()),
    )?;
    let generations: Vec<u64> = later.iter().map(|entry| entry.generation).collect();
    expect_eq("the generations of the log", generations, vec![1, 2, 3, 4])?;
    non_decreasing(&later)?;
    succeeds("delete(p, 4)", store.delete(&p.name, 4, &who))?;
    let after = history(store, &p.name)?;
    expect_eq("the entries after the delete", after.len(), 5)?;
    expect_eq(
        "the kind of the last entry after the delete",
        after.last().map(|&(_, _, kind)| kind),
        Some(DELETED),
    )
}

/// Case 16: the log of a name never stored is `profile-not-found`.
pub(super) fn log_of_never_created_is_profile_not_found(
    store: &dyn ProfileStore,
) -> Result<(), String> {
    let missing = profile_name(ALPHA)?;
    expect_code("log of a name never stored", store.log(&missing), NOT_FOUND)
}

/// Case 17: `rename` is PROPOSED (#665) and answers `not-implemented`, changing
/// nothing: the profile, its log and the absence of the new name. #665 replaces this
/// case.
pub(super) fn rename_is_not_implemented(store: &dyn ProfileStore) -> Result<(), String> {
    let who = actor()?;
    let p = sample(ALPHA, &[C1])?;
    put(store, &p, 0, &who)?;
    let before = shown(store, &p.name)?;
    let q = profile_name(BETA)?;
    let call = "rename(p, q, 1) while rename is PROPOSED (#665)";
    expect_code(call, store.rename(&p.name, &q, 1, &who), NOT_IMPLEMENTED)?;
    unchanged(store, &p.name, &before, call)?;
    expect_eq(
        "get of q after the refused rename",
        succeeds("get", store.get(&q))?,
        None,
    )
}

/// Case 18: an env entry is a name, and the one guard is the type: `EnvVarName`
/// refuses a value (`profile-secret-refused`) and a blank name (`env-name-invalid`).
/// The store adds no check of its own, so a spec's env names are stored and read back
/// verbatim.
pub(super) fn env_is_names_only(store: &dyn ProfileStore) -> Result<(), String> {
    let call = "EnvVarName::parse(\"TOKEN=x\")";
    expect_code(call, EnvVarName::parse("TOKEN=x"), SECRET_REFUSED)?;
    let call = "EnvVarName::parse(\" \")";
    expect_code(call, EnvVarName::parse(" "), NAME_INVALID)?;
    let call = "EnvVarName::parse(\"\")";
    expect_code(call, EnvVarName::parse(""), NAME_INVALID)?;
    let env = ENV_NAMES
        .iter()
        .map(|text| {
            let call = format!("EnvVarName::parse({text:?})");
            succeeds(&call, EnvVarName::parse(text))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let spec = ProfileSpec {
        env,
        ..sample_spec(C1)
    };
    let p = Profile {
        panes: vec![spec],
        ..sample(ALPHA, &[])?
    };
    let stored = put(store, &p, 0, &actor()?)?;
    expect_eq(
        "the record a create with env names returns",
        &stored,
        &stored_as(&p, 1, &stored),
    )?;
    expect_eq(
        "get of the profile with env names",
        succeeds("get", store.get(&p.name))?,
        Some(stored),
    )
}

/// `Ok` when the `at` of `log_entries` never decreases.
fn non_decreasing(log_entries: &[ProfileLogEntry]) -> Result<(), String> {
    if log_entries
        .windows(2)
        .all(|pair| matches!(pair, [earlier, later] if earlier.at <= later.at))
    {
        Ok(())
    } else {
        let at: Vec<i64> = log_entries.iter().map(|entry| entry.at).collect();
        Err(format!("the log's `at` decreases: {at:?}"))
    }
}
