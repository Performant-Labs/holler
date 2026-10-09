#![allow(clippy::unwrap_used)] // #670
#![allow(clippy::expect_used)] // #670
#![allow(clippy::panic)] // #670
#![allow(clippy::unreachable)] // #670
#![allow(dead_code)] // #670
//! In-process tests of the `holler pane` verbs (story #670, epic #633).
//!
//! One module per verb, so a verb story edits only `pane_verbs/<verb>.rs` and never
//! this file or the manifest. The shared pieces: the harness that runs a verb over
//! captured writers (`tests/verb_harness/mod.rs`), the output API (`output_api`), the
//! shared spec-flag validation (`spec_flags`) and the `say`/`interrupt`/`answer`
//! target accessors (`target_flags`).
//!
//! The process-level tests of the whole surface are a separate target,
//! `pane_cli_process`.

#[path = "../verb_harness/mod.rs"]
mod verb_harness;

mod output_api;
mod spec_flags;
mod target_flags;

mod close;
mod doctor;
mod get;
mod import;
mod launch;
mod list;
mod park;
mod relaunch;
mod reset;
mod switch;
mod unpark;
mod watch;

/// The unwired port set is the one not-implemented port set (#649 builds the real wiring
/// and keeps `Unwired`): every port it hands a verb answers
/// `not-implemented`, so a verb that runs before its ports exist fails loudly instead of
/// acting. The harness's `run_verb` runs over it, and nothing here builds the real wiring.
#[test]
fn unwired_ports_answer_not_implemented() {
    let ports = verb_harness::unwired_ports();
    let codes = [
        ports.pane_store.list().err().map(|e| e.code().to_string()),
        ports.herdr.version().err().map(|e| e.code().to_string()),
        ports.harness.health(1).err().map(|e| e.code().to_string()),
    ];
    for code in codes {
        assert_eq!(code.as_deref(), Some("not-implemented"));
    }
}
