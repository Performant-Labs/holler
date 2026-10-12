//! A single-process pane asks the hub nothing (#839, D3 item 8 and D6; Acceptance 22): until a
//! body drives a Claude Code pane (#850) it has no driver, so its switch and reset are the
//! relaunch they were (#831), make no `hub.*` call and leave the stored `last_observed.driven`
//! as it was; `pane doctor` observes no DRIVEN for it. Its `reset --first` is in `lifecycle.rs`.

use holler_cli::output::Format;
use holler_pane::ports::Activity;
use holler_pane::HarnessPort;

use crate::rig::{cell, Rig, CLAUDE};

/// A DRIVEN a pass of the old era might have stored, which nothing here may infer or clear.
const STORED: &str = "4f1c2b8e-0000-4000-8000-00000000d1d1";

/// One live, idle Claude pane with a conversation and a stored DRIVEN; and a body that
/// advertises its session of record, which would be its driver by the session of record were the
/// pane not skipped.
fn claude_pane_beside_a_body() -> Rig {
    let rig = Rig::new();
    rig.seed_claude(CLAUDE, cell(1, 2));
    rig.converse(CLAUDE);
    let record = rig.record(CLAUDE);
    let mut stored = record.clone();
    stored.last_observed.driven = Some(STORED.to_owned());
    rig.panes.concurrent_put(&stored).unwrap();
    let sor = record.session_of_record.unwrap();
    rig.hub
        .with_body("tok_x", "kiwi/claude", &sor, Activity::Idle);
    rig
}

fn driven(rig: &Rig) -> Option<String> {
    rig.record(CLAUDE).last_observed.driven
}

#[test]
fn switch_and_reset_of_a_claude_pane_ask_the_hub_nothing_and_keep_driven() {
    let rig = claude_pane_beside_a_body();
    let target = rig.claude().new_session_id().unwrap();
    rig.claude().seed_conversation(&target);
    let (switched, calls) = rig.run_logged(&["pane", "switch", CLAUDE, &target], Format::Json);
    assert_eq!(switched.code, 0, "{switched:?}");
    assert!(calls.hub.is_empty(), "switch: no hub call: {:?}", calls.hub);
    assert_eq!(
        driven(&rig).as_deref(),
        Some(STORED),
        "switch keeps the stored DRIVEN"
    );

    let (reset, calls) = rig.run_logged(&["pane", "reset", CLAUDE], Format::Json);
    assert_eq!(reset.code, 0, "{reset:?}");
    assert!(calls.hub.is_empty(), "reset: no hub call: {:?}", calls.hub);
    assert_eq!(
        driven(&rig).as_deref(),
        Some(STORED),
        "reset keeps the stored DRIVEN"
    );
    assert!(
        rig.hub.log().is_empty(),
        "the body was never asked anything"
    );
}

/// `pane doctor` observes no DRIVEN for a single process: its stored value stands, and a body on
/// its session of record does not become its DRIVEN.
#[test]
fn doctor_leaves_a_claude_pane_s_driven_as_stored() {
    let rig = claude_pane_beside_a_body();
    let run = rig.run(&["pane", "doctor"], Format::Json);
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(driven(&rig).as_deref(), Some(STORED));
}
