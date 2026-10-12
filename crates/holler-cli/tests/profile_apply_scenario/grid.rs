//! #853's scratch test (Acceptance 3), modelled on the issue's spike: `profile apply` of a
//! profile of four panes, none of them recorded, builds the whole 2-by-2 grid in the empty
//! scratch workspace `hlr649sq`, column 1 first (`r1c1`, `r2c1`, `r1c2`, `r2c2`), and a second
//! apply adopts all four. Before #853 the spike's run created `r1c1` and `r1c2`, then refused
//! `r2c1` and `r2c2` as `grid-unreachable`.
//!
//! It needs a Herdr that speaks protocol 22 (0.9.1) first on `PATH`: opted in on a machine
//! where an older `herdr` comes first, it fails at its first connect, and the message says so.

use holler_pane::tx_apply::{Action, Outcome};
use holler_pane::GridPos;
use serde_json::json;

use crate::plan::Applied;
use crate::scratch::rig::{free_port, no_model_can_be_reached};
use crate::scratch::Scratch;

const PROFILE: &str = "hlr649-grid";
const WORKSPACE: &str = "hlr649sq";

/// The four panes in the order apply takes them, each with its cell.
const PANES: [(&str, &str, GridPos); 4] = [
    ("hlr649-g11", "r1c1", GridPos { row: 1, col: 1 }),
    ("hlr649-g21", "r2c1", GridPos { row: 2, col: 1 }),
    ("hlr649-g12", "r1c2", GridPos { row: 1, col: 2 }),
    ("hlr649-g22", "r2c2", GridPos { row: 2, col: 2 }),
];

/// Herdr's own snapshot of `WORKSPACE`: each pane's id and cell (as JSON text), sorted.
fn listed(scratch: &Scratch) -> Vec<(String, String)> {
    let mut panes: Vec<(String, String)> = scratch
        .herdr_snapshot()
        .into_iter()
        .filter(|pane| pane.workspace == WORKSPACE)
        .map(|pane| (pane.pane_id.as_str().to_owned(), json!(pane.grid).to_string()))
        .collect();
    panes.sort();
    panes
}

/// Acceptance 3: from an empty workspace, one apply creates the four panes, Herdr lists them
/// at their cells (the adapter's snapshot agrees with the records), and a second apply adopts
/// all four and changes nothing.
#[test]
#[ignore = "opt-in: scratch Herdr 0.9.1, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#853)"]
fn apply_builds_a_whole_2x2_grid_from_an_empty_workspace() {
    let Some(scratch) = Scratch::start() else {
        return;
    };
    scratch.assert_tmux_env_is_scratch();
    assert_eq!(listed(&scratch), vec![], "{WORKSPACE} starts empty");
    let ports = PANES.map(|(pane, _, _)| (pane, free_port()));
    scratch.json(&["profile", "create", PROFILE], 0);
    // P lists the specs in the reverse of apply's order, which no split reaches.
    for ((pane, grid, _), (_, port)) in PANES.iter().zip(ports).rev() {
        let flags = ["--profile", PROFILE, "--spec-only"];
        let argv = scratch.launch_argv(pane, WORKSPACE, grid, port, &flags);
        scratch.json(&argv.iter().map(String::as_str).collect::<Vec<_>>(), 0);
    }

    let session = scratch.settings.session.clone();
    let argv = ["profile", "apply", PROFILE, "--herdr-session", &session];
    let applied = Applied::of(&scratch.json(&argv, 0));
    for (pane, _, _) in PANES {
        assert_eq!(
            applied.step(pane).row(),
            (Action::Create, Outcome::Created),
            "{pane}: {applied:#?}"
        );
    }
    let order: Vec<&str> = PANES.iter().map(|(pane, _, _)| *pane).collect();
    assert_eq!(applied.panes(), order, "column 1 first");
    for (_, port) in ports {
        no_model_can_be_reached(port);
    }
    let mut recorded: Vec<(String, String)> = PANES
        .iter()
        .map(|(pane, _, cell)| {
            let herdr = scratch.record(pane)["herdr"].clone();
            assert_eq!(herdr["grid"], json!(cell), "{pane}'s record");
            let id = herdr["pane_id"].as_str().expect("a pane id").to_owned();
            (id, json!(cell).to_string())
        })
        .collect();
    recorded.sort();
    assert_eq!(listed(&scratch), recorded, "Herdr lists each at its cell");

    let again = Applied::of(&scratch.json(&argv, 0));
    for (pane, _, _) in PANES {
        assert_eq!(
            again.step(pane).row(),
            (Action::Adopt, Outcome::Adopted),
            "{pane}"
        );
    }
    assert_eq!(listed(&scratch), recorded, "the second apply made nothing");

    scratch.assert_end_state(&order, &[WORKSPACE]);
    scratch.assert_tmux_env_is_scratch();
    scratch.assert_no_credential_leaked();
}
