//! D3 and D13's round trips, as A's W-6 binds them: the wiring's two store clients pass the
//! test kit's `PaneStore` and `ProfileStore` conformance suites against a real hub, as
//! ADR-0021 section 2 asks of every real implementation. Each case gets a fresh hub on a
//! fresh state dir; the hub's handle and its state dir are the case's guard.
//!
//! The hub's long-poll window is `WATCH_WAIT` (4 s), and a hub process takes no shorter one,
//! so each idle read of a watch case waits that long: the two suites take tens of seconds.
//!
//! Over the wire, `pane/cas_put` checks membership against the hub's profile registry
//! (#661), so a pane naming a profile is stored only when that profile exists. The suite's
//! case `pane-in-other-profile` moves a pane between the profiles `Alpha` and `Beta`: each
//! case's hub is given both, empty, before the case runs (the in-process registry of the
//! hub's own run has no profile registry to check).

use holler_cli::pane::wiring::store::{HubPaneStore, HubProfileStore};
use holler_pane::{Actor, ProfileStore};
use holler_pane_testkit::conformance::pane_store::run_pane_store_conformance;
use holler_pane_testkit::conformance::profile_store::run_profile_store_conformance;
use holler_pane_testkit::conformance::Conformance;
use holler_pane_testkit::fixture::sample_profile;

use crate::state_dir_lock;
use crate::support::{Hub, StateDir};

/// A fresh hub, with this process's `HOLLER_STATE_DIR` pointed at it. The caller holds the
/// state-dir lock.
fn fresh_hub() -> (Hub, StateDir) {
    let state = StateDir::new();
    let hub = Hub::start(&state);
    std::env::set_var("HOLLER_STATE_DIR", state.path());
    (hub, state)
}

fn assert_conforms(suite: &str, result: Conformance) {
    if let Err(failures) = result {
        let ids: Vec<&str> = failures.iter().map(|f| f.case).collect();
        panic!("the {suite} client fails case(s) {ids:?}: {failures:#?}");
    }
}

/// [`fresh_hub`], holding the empty profiles `Alpha` and `Beta`.
fn fresh_hub_with_profiles() -> (Hub, StateDir) {
    let guard = fresh_hub();
    let actor = Actor::parse("holler pane").unwrap();
    for name in ["Alpha", "Beta"] {
        HubProfileStore::new()
            .cas_put(&sample_profile(name, &[]).unwrap(), 0, &actor)
            .unwrap_or_else(|e| panic!("create the profile {name} through the client: {e}"));
    }
    guard
}

#[test]
fn the_pane_client_passes_the_pane_store_conformance_suite() {
    let _held = state_dir_lock();
    let result = run_pane_store_conformance(|| (HubPaneStore::new(), fresh_hub_with_profiles()));
    assert_conforms("pane/*", result);
}

#[test]
fn the_profile_client_passes_the_profile_store_conformance_suite() {
    let _held = state_dir_lock();
    let result = run_profile_store_conformance(|| (HubProfileStore::new(), fresh_hub()));
    assert_conforms("profile/*", result);
}
