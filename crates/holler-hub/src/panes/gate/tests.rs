//! `pane_gate` alone (#839, D1; Acceptance 4 and 14): the `-32011` frame it answers for the pane
//! `holler_pane::gate::first_refusal` names, keyed on the advertised id of the session being
//! prompted. The enforcement point (every variant, nothing on the socket, the hold first) is
//! `circuit/dispatch.rs`'s `hold_tests`; the selection's two keys and its order are
//! `holler-pane`'s `gate_test.rs`. Ids are `ses_`-shaped (D8).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #839

use std::sync::Arc;

use holler_pane::gate::prompt_refusal;
use holler_pane::pane::{Health, Hold, LastObserved};
use holler_pane::{Pane, PaneStore as _};
use holler_proto::WireError;

use super::pane_gate;
use crate::panes::PaneState;

const ID: &str = "ses_000000000000000000000000a1";
const OTHER: &str = "ses_000000000000000000000000b2";

/// A healthy pane `name` on `sor`, SHOWN and DRIVEN `driven`.
fn pane(name: &str, sor: &str, driven: &str) -> Pane {
    let mut pane = holler_pane_testkit::fixture::sample_pane(name).unwrap();
    pane.session_of_record = Some(sor.to_owned());
    pane.harness.health = Health::Healthy;
    pane.last_observed = LastObserved {
        shown: Some(sor.to_owned()),
        driven: Some(driven.to_owned()),
        at: 1,
    };
    pane
}

fn parked(mut pane: Pane) -> Pane {
    pane.hold = Hold::Parked {
        reason: "disk full".to_owned(),
        release_when: "later".to_owned(),
        since: 5,
    };
    pane
}

fn unhealthy(mut pane: Pane) -> Pane {
    pane.harness.health = Health::Unhealthy("server-down".to_owned());
    pane
}

/// A registry seeded with `panes` in a throwaway state dir, which must outlive it.
fn registry(panes: &[Pane]) -> (Arc<PaneState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::state::HubState::from_root(dir.path().to_path_buf());
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    let registry = Arc::new(PaneState::load(&state));
    for pane in panes {
        registry.cas_put(pane, 0).unwrap();
    }
    (registry, dir)
}

/// `-32011`, `hold_kind: "pane"`, the pane's code as `data.reason`, no `data.since`, and one line
/// naming the pane and the code.
fn assert_frame(error: &WireError, pane: &str, code: &str) {
    assert_eq!(error.code, -32011, "{error:?}");
    let data = error.data.as_deref().expect("data");
    assert_eq!(data.hold_kind.as_deref(), Some("pane"), "{error:?}");
    assert_eq!(data.reason.as_deref(), Some(code), "{error:?}");
    assert!(data.since.is_none(), "{error:?}");
    assert!(!error.message.contains('\n'), "{}", error.message);
    assert!(
        error.message.contains(pane) && error.message.contains(code),
        "{}",
        error.message
    );
}

/// The frame carries the code `holler_pane::gate::prompt_refusal` gives the same record (one
/// predicate: the CLI's side is `pane_verbs/driven.rs`).
#[test]
fn the_frame_is_the_one_predicate_s_code_for_the_pane_on_the_id() {
    let records = [
        parked(pane("demo-c1r1", ID, ID)),
        unhealthy(pane("demo-c1r1", ID, ID)),
        pane("demo-c1r1", ID, OTHER),
    ];
    for record in records {
        let want = prompt_refusal(&record).expect("a refused record");
        let (panes, _dir) = registry(&[record]);
        let error = pane_gate(&panes, ID).expect_err("refused");
        assert_frame(&error, "demo-c1r1", want.code().as_str());
    }
}

/// The recorded DRIVEN is a key: a pane whose stale DRIVEN is the id is named.
#[test]
fn a_pane_whose_recorded_driven_is_the_id_is_named() {
    let (panes, _dir) = registry(&[pane("demo-c1r1", OTHER, ID)]);
    let error = pane_gate(&panes, ID).expect_err("the DRIVEN key refuses");
    assert_frame(&error, "demo-c1r1", "pane-shown-driven-mismatch");
}

/// Two bad panes on the id: the first in name order is named.
#[test]
fn the_first_bad_of_two_is_named() {
    let (panes, _dir) = registry(&[
        parked(pane("demo-c2r1", ID, ID)),
        unhealthy(pane("demo-c1r1", ID, ID)),
    ]);
    let error = pane_gate(&panes, ID).expect_err("refused");
    assert_frame(&error, "demo-c1r1", "pane-unhealthy");
}

/// An id no pane names is admitted, whatever other panes record.
#[test]
fn an_id_no_pane_names_is_admitted() {
    let (panes, _dir) = registry(&[parked(pane("demo-c1r1", OTHER, OTHER))]);
    assert!(pane_gate(&panes, ID).is_ok());
}

/// A registry that cannot be read refuses nothing (a broken pane file must not stop every prompt).
#[test]
fn an_unreadable_registry_refuses_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = crate::state::HubState::from_root(dir.path().to_path_buf());
    std::fs::create_dir_all(&state.hub_dir).unwrap();
    std::fs::write(state.hub_dir.join("panes.json"), "{ not json").unwrap();
    let panes = PaneState::load(&state);
    assert!(panes.list().is_err(), "the registry failed to load");
    assert!(pane_gate(&panes, ID).is_ok());
}
