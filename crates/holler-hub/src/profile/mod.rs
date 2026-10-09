//! The hub's profile registry (epic #633, story #661). It keeps one record per profile in a
//! file, filed by slug, makes every write a compare-and-swap on the record's generation,
//! appends each applied write to the profile's change log, and offers a change feed.
//!
//! The control socket forwards `profile/get`, `list`, `cas_put`, `delete`, `watch` and `log`
//! here (`crate::pane_dispatch`, #669). [`ProfileState`] is the registry and implements the
//! `holler_pane::ProfileStore` port. [`dispatch`] answers the six methods with a
//! `PaneReply`, carried as a JSON-RPC result, and routes `profile/rename` (PROPOSED, #665)
//! to [`rename`], which answers `not-implemented`. The hub is the store only: it keeps what
//! a verb wrote and never reads the pane registry on a profile write. Unlike the pane
//! registry it stamps times itself (D4): a record's `created` and `updated` and each log
//! entry's `at`. The submitted times, slug and generation are ignored.
//!
//! - `store.rs`: the table behind one lock, its compare-and-swap writes and their log
//!   entries, and the long-poll wait. It is the twin of `panes::store`.
//! - `entry.rs`: the file's entry, `<state dir>/hub/profiles.json` (D2), and the checks
//!   that make a corrupt one fail closed; the stamps (D4) and an update's summary (D5).
//! - `handlers.rs`: the six handlers, each #639's `panes::handlers::run` with a closure.
//!
//! The registry is built from #639's shared parts and copies none of them: the file's load
//! and save (`panes::persist`), the feed's ring and its rules (`panes::feed`), the handler
//! pipeline (`panes::handlers::run`), the options and the long-poll window
//! (`PaneStoreOptions`, `WATCH_WAIT`), the fault log (`panes::log_fault`) and the
//! compare-and-swap rule (`holler_pane::next_generation`).
//!
//! # The rules
//!
//! - **The slug is the identity** (`ProfileName::slug`). `get`, `delete` and `log` find a
//!   profile by the slug of the name they are given, so `NIGHT-SHIFT` finds `Night Shift`.
//!   A write of a name whose slug is filed under another live name is `profile-exists`,
//!   whatever the generation, and a submitted slug is replaced by the name's.
//! - **Every write is a compare-and-swap** on the generation: a create names 0 and is
//!   stored at 1, and a stale or an ahead generation is `generation-conflict`. A delete of a
//!   profile that does not exist is `profile-not-found`, whatever the generation.
//! - **The change log is append-only.** Each applied write appends one entry (when, the
//!   generation after the write, who, and what changed), and a refused write appends
//!   nothing. A delete is logged at the deleted generation + 1. The log is never cut: it is
//!   still readable after a delete and goes on across a re-create, so `log` is
//!   `profile-not-found` only for a name never created. No method takes a log.
//! - **A profile holds no secret** (I7): an env entry is an `EnvVarName`, whose own decode
//!   refuses a value, in a request and in the file alike.
//! - **The membership rule** (ADR-0021 §8): a pane belongs to at most one profile, and
//!   names only a profile that exists. [`check_membership`], the `pane/cas_put` hook, checks
//!   that the named profile exists. The refusal of a move from one profile to another
//!   (`pane-in-other-profile`) is not here: it needs the stored pane, so it runs inside the
//!   pane registry's compare-and-swap, under the pane lock ("Decisions taken", item 2). A
//!   spec may name a pane of another profile, or one that does not exist (a detached
//!   spec): that is not refused.
//!
//! # Operating limits
//!
//! - Each write rewrites the whole file, every log included, and wakes every watcher.
//!   Profile writes are paced by an operator's verbs, never by a timer, and nothing prunes a
//!   log or a tombstone, so the file grows with the history it keeps.
//! - The lock is held across the save, as in the pane registry: a wedged save blocks every
//!   caller, with no `timeout`.
//! - Each pending `profile/watch` holds one blocking-pool thread for up to `WATCH_WAIT`, the
//!   pane registry's window.
//!
//! # Shape
//!
//! As `PaneState` does, `ProfileState` holds its store behind an `Arc` of its own: the
//! port's `watch(&self)` returns a `'static` iterator, which has to own a handle on the
//! store, and the handlers clone the same inner `Arc` into the blocking pool.
//!
//! The module root is `mod.rs`, which no other hub module is: the root has to sit
//! inside the `profile/**` blast radius that #661 owns, and a sibling `profile.rs`
//! would not.

mod entry;
mod handlers;
pub mod rename;
mod store;

use std::sync::Arc;

use holler_pane::{
    Actor, Cursor, Pane, PaneError, PaneReply, Profile, ProfileEvent, ProfileLogEntry, ProfileName,
    ProfileStore, Watch,
};
use holler_proto::CorrelationId;
use serde_json::Value;

use crate::pane_dispatch::reply_line;
use crate::panes::{PaneState, PaneStoreOptions};
use crate::state::HubState;
use store::Store;

/// The registry file, in the hub's state dir (`<state dir>/hub/`).
const FILE_NAME: &str = "profiles.json";

/// The profile registry's state, shared by every control connection behind one `Arc`
/// ([`crate::pane_dispatch::PaneDeps`]).
///
/// It does not derive `Clone`. A copy per connection would split the registry, and a
/// split registry cannot make a compare-and-swap hold: share the `Arc`, never the
/// value.
pub struct ProfileState {
    store: Arc<Store>,
}

impl ProfileState {
    /// Load the registry from `<state dir>/hub/profiles.json`, once, before the first
    /// connection. It never fails: a missing file is an empty registry, and a file that
    /// cannot be loaded leaves the registry failed. Every method then answers
    /// `store-corrupt`, and the file is never rewritten or moved (see `store.rs`).
    pub fn load(state: &HubState) -> Self {
        Self::load_with(state, PaneStoreOptions::default())
    }

    /// [`ProfileState::load`] with `options` in place of the defaults. The options are the
    /// pane registry's: the long-poll window and the length of the feed's ring.
    pub fn load_with(state: &HubState, options: PaneStoreOptions) -> Self {
        Self {
            store: Arc::new(Store::open(state.hub_dir.join(FILE_NAME), options)),
        }
    }
}

impl ProfileStore for ProfileState {
    fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
        self.store.get(name)
    }

    fn list(&self) -> Result<Vec<Profile>, PaneError> {
        self.store.list()
    }

    fn cas_put(
        &self,
        profile: &Profile,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<Profile, PaneError> {
        self.store.cas_put(profile, expected_generation, actor)
    }

    fn delete(
        &self,
        name: &ProfileName,
        expected_generation: u64,
        actor: &Actor,
    ) -> Result<(), PaneError> {
        self.store.delete(name, expected_generation, actor)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<ProfileEvent>, PaneError> {
        Store::watch(&self.store, since)
    }

    fn log(&self, name: &ProfileName) -> Result<Vec<ProfileLogEntry>, PaneError> {
        self.store.log(name)
    }

    /// PROPOSED (#665): `not-implemented`, touching nothing, whatever the registry holds.
    /// #665 adds the rename to the store when it is confirmed (D12).
    fn rename(
        &self,
        _from: &ProfileName,
        _to: &ProfileName,
        _expected_generation: u64,
        _actor: &Actor,
    ) -> Result<Profile, PaneError> {
        Err(PaneError::NotImplemented)
    }
}

/// Answer one `profile/*` request with its handler. `obj` is the whole request frame.
///
/// `panes` is the pane registry, which `profile/rename` (#665) reaches to move a profile's
/// member panes to the new name; no other method reads it. `profile/watch` is long-poll:
/// one `{events, cursor}` reply per request, after at most `WATCH_WAIT`.
pub async fn dispatch(
    method: &str,
    cid: &CorrelationId,
    obj: &Value,
    profiles: &ProfileState,
    panes: &PaneState,
) -> String {
    let store = Arc::clone(&profiles.store);
    match method {
        "profile/get" => handlers::get(cid, obj, store).await,
        "profile/list" => handlers::list(cid, obj, store).await,
        "profile/cas_put" => handlers::cas_put(cid, obj, store).await,
        "profile/delete" => handlers::delete(cid, obj, store).await,
        "profile/watch" => handlers::watch(cid, obj, store).await,
        "profile/log" => handlers::log(cid, obj, store).await,
        "profile/rename" => rename::dispatch(cid, obj, profiles, panes).await,
        // `forward` sends only the methods of `PROFILE_METHODS`, and each has an arm above.
        // A method added to that list before its handler answers like a stub.
        _ => reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented)),
    }
}

/// The membership hook of a pane write (D8): `pane/cas_put` (#639) calls it before the
/// compare-and-swap, on the blocking pool and before the pane lock is taken, so no lock is
/// held inside another.
///
/// A pane outside any profile is accepted without reading the profile registry. A pane
/// that names a profile needs that profile to exist, found by slug as every lookup is:
/// `profile-not-found` when it does not, and the registry's own `store-corrupt` when it
/// has failed (fail closed, never accepted). Every write naming a profile is checked, not
/// only one that changes `Pane.profile`, because the hook does not see the stored pane
/// (D9). Moving a pane from one profile to another is the pane registry's refusal, made
/// inside its compare-and-swap (see the module docs).
pub fn check_membership(pane: &Pane, profiles: &ProfileState) -> Result<(), PaneError> {
    let Some(name) = &pane.profile else {
        return Ok(());
    };
    match profiles.store.get(name)? {
        Some(_) => Ok(()),
        None => Err(PaneError::ProfileNotFound {
            what: name.to_string(),
        }),
    }
}
