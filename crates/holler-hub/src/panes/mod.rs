//! The hub's pane registry (epic #633, story #639). It keeps one record per pane in a file,
//! makes every write a compare-and-swap on the record's generation, and offers a change
//! feed.
//!
//! The control socket forwards `pane/get`, `list`, `cas_put`, `delete` and `watch` here
//! (`crate::pane_dispatch`, #669). [`PaneState`] is the registry and implements the
//! `holler_pane::PaneStore` port. [`dispatch`] answers the five methods with a `PaneReply`,
//! carried as a JSON-RPC result. The hub is the store only: it keeps what a verb recorded and
//! what reconcile observed. It never infers a pane's session, stamps nothing and runs no
//! adapter.
//!
//! - `store.rs`: the table behind one lock, its compare-and-swap writes, and the long-poll
//!   wait.
//! - `persist.rs`: the file, `<state dir>/hub/panes.json` (D2), and how loading it fails
//!   closed (D3).
//! - `feed.rs`: the ring of recent events, and the rule of what a watcher is owed (D5, D6).
//! - `handlers.rs`: the five handlers and the one pipeline they share.
//!
//! #661's profile registry reuses `persist`, `feed` and `handlers::run` from `profile/`,
//! through the crate-private `RegistryEntry` trait. #661 also adds one comparison here: the
//! `pane-in-other-profile` check runs inside the pane registry's compare-and-swap
//! (`store.rs`), under the pane lock (ADR-0021 §8 and "Decisions taken", item 2).
//!
//! # Operating limits
//!
//! - Each write rewrites the whole file and wakes every watcher. A periodic writer, such as
//!   reconcile writing `last_observed` (#647), must skip a record that has not changed.
//! - The lock is held across the save. The port's bound on a call (I5, in
//!   `holler_pane::ports`) therefore holds only while saves are fast: a wedged save blocks
//!   every caller, with no `timeout`. That is acceptable for a local state dir, where the
//!   client's own timeout bounds the verb.
//! - Each pending `pane/watch` holds one blocking-pool thread for up to [`WATCH_WAIT`], and
//!   the runtime's shutdown waits for it. Do not raise the window without revisiting this.
//!
//! # Shape
//!
//! `PaneState` holds its store behind an `Arc` of its own, rather than relying on the `Arc`
//! that `pane_dispatch.rs` wraps it in. The port forces this: `watch(&self)` returns a
//! `'static` iterator, which has to own a handle on the store. The handlers clone the same
//! inner `Arc` into the blocking pool.
//!
//! The module root is `mod.rs`, which no other hub module is: the root has to sit inside
//! the `panes/**` blast radius that #639 owns, and a sibling `panes.rs` would not.

pub(crate) mod feed;
pub(crate) mod handlers;
pub(crate) mod persist;
mod store;

use std::sync::Arc;
use std::time::Duration;

use holler_pane::{Cursor, Pane, PaneError, PaneEvent, PaneName, PaneReply, PaneStore, Watch};
use holler_proto::CorrelationId;
use serde_json::Value;

use crate::pane_dispatch::reply_line;
use crate::profile::ProfileState;
use crate::state::HubState;
use store::Store;

/// The registry file, in the hub's state dir (`<state dir>/hub/`).
const FILE_NAME: &str = "panes.json";

/// How many recent events the change feed keeps by default (D5).
const FEED_RETAINED: usize = 1024;

/// The long-poll window of `pane/watch`, and of `next()` on a `Watch` iterator: how long a
/// request waits for a change before it answers an empty batch (D5).
///
/// It is below the control client's default timeout (5 s) and within I5's 10 s. A client
/// that sends `pane/watch` must set its own timeout to this window plus a margin. The
/// control precedent adds 5 s (`control/say`, `control/wait`), because the 1 s left under
/// the default can be used up by a writer that holds the lock across a slow save. #649
/// reads this constant to size `ControlCall.timeout`.
pub const WATCH_WAIT: Duration = Duration::from_secs(4);

/// The tunable limits of a registry. [`PaneState::load`] uses the defaults. Tests use a
/// short window and a small ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneStoreOptions {
    /// The long-poll window ([`WATCH_WAIT`] by default).
    pub watch_wait: Duration,
    /// How many recent events the feed keeps exactly (1024 by default). A watcher further
    /// behind gets the latest state of each changed pane instead (`feed.rs`, rule 4).
    pub feed_retained: usize,
}

impl Default for PaneStoreOptions {
    fn default() -> Self {
        Self {
            watch_wait: WATCH_WAIT,
            feed_retained: FEED_RETAINED,
        }
    }
}

/// The pane registry's state, shared by every control connection behind one `Arc`
/// ([`crate::pane_dispatch::PaneDeps`]).
///
/// It does not derive `Clone`. A copy per connection would split the registry, and a
/// split registry cannot make a compare-and-swap hold: share the `Arc`, never the
/// value.
pub struct PaneState {
    store: Arc<Store>,
}

impl PaneState {
    /// Load the registry from `<state dir>/hub/panes.json`, once, before the first
    /// connection. It never fails: a missing file is an empty registry, and a file that
    /// cannot be loaded leaves the registry failed. Every method then answers
    /// `store-corrupt`, and the file is never rewritten (see `persist.rs` and `store.rs`).
    pub fn load(state: &HubState) -> Self {
        Self::load_with(state, PaneStoreOptions::default())
    }

    /// [`PaneState::load`] with `options` in place of the defaults.
    pub fn load_with(state: &HubState, options: PaneStoreOptions) -> Self {
        Self {
            store: Arc::new(Store::open(state.hub_dir.join(FILE_NAME), options)),
        }
    }
}

impl PaneStore for PaneState {
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError> {
        self.store.get(name)
    }

    fn list(&self) -> Result<Vec<Pane>, PaneError> {
        self.store.list()
    }

    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        self.store.cas_put(pane, expected_generation)
    }

    fn delete(&self, name: &PaneName, expected_generation: u64) -> Result<(), PaneError> {
        self.store.delete(name, expected_generation)
    }

    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        Store::watch(&self.store, since)
    }
}

/// One entry of a registry: the last change filed under a name. The file stores entries
/// and the feed replays them. A put carries the record, and a delete leaves a tombstone.
///
/// `persist` checks entries and `feed` selects them only through this trait. #661's
/// profile registry reuses both by implementing it for `ProfileEvent`.
pub(crate) trait RegistryEntry: Clone {
    /// The name the entry is filed under.
    fn name(&self) -> &str;
    /// The cursor of the entry's change.
    fn cursor(&self) -> Cursor;
    /// The record's own name and generation, or `None` for a tombstone.
    fn record(&self) -> Option<(&str, u64)>;
}

impl RegistryEntry for PaneEvent {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn cursor(&self) -> Cursor {
        self.cursor
    }

    fn record(&self) -> Option<(&str, u64)> {
        self.pane
            .as_deref()
            .map(|pane| (pane.name.as_str(), pane.generation))
    }
}

/// Answer one `pane/*` request with its handler. `obj` is the whole request frame.
///
/// `profiles` is the profile registry. `pane/cas_put` calls
/// [`crate::profile::check_membership`] with it before the compare-and-swap, so a refusal
/// is the reply and nothing is written. The parameter is an `&Arc`, so the handler can move
/// a handle into the blocking pool, where the check runs. #661's check reads the profile
/// registry through its blocking port, which must never run on an executor thread.
/// `pane/watch` is long-poll: one `{events, cursor}` reply per request, after at most
/// [`WATCH_WAIT`].
pub async fn dispatch(
    method: &str,
    cid: &CorrelationId,
    obj: &Value,
    panes: &PaneState,
    profiles: &Arc<ProfileState>,
) -> String {
    let store = Arc::clone(&panes.store);
    match method {
        "pane/get" => handlers::get(cid, obj, store).await,
        "pane/list" => handlers::list(cid, obj, store).await,
        "pane/cas_put" => handlers::cas_put(cid, obj, store, Arc::clone(profiles)).await,
        "pane/delete" => handlers::delete(cid, obj, store).await,
        "pane/watch" => handlers::watch(cid, obj, store).await,
        // `forward` sends only the methods of `PANE_METHODS`, and each has an arm above.
        // A method added to that list before its handler answers like a stub.
        _ => reply_line(cid, &PaneReply::failure(&PaneError::NotImplemented)),
    }
}
