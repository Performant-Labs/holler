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

use holler_cli::output::Format;
use holler_cli::pane::wiring::Wiring;
use verb_harness::{one_envelope, run_verb};

/// The seam: a stub's `run` reached through `pane::run`, over the stub wiring's
/// `Ports` and captured writers. Text mode refuses on `err` only; JSON mode writes one
/// envelope to `out` only. (The signatures of `VerbCtx`, `Sink` and `emit*` are
/// provisional until this compiles.)
#[test]
fn seam_pane_stub_routes_text_to_err_and_json_to_out() {
    verb_harness::assert_stub_routes(&["pane", "list"], 643);
}

/// The stub wiring is the one not-implemented port set (#649 replaces the body of its
/// constructor): every port it hands a verb answers `not-implemented`, so a verb that
/// runs before its wiring exists fails loudly instead of acting.
#[test]
fn stub_wiring_ports_answer_not_implemented() {
    let wiring = Wiring::connect().expect("the stub wiring connects without a hub");
    let ports = wiring.ports();
    let codes = [
        ports.pane_store.list().err().map(|e| e.code().to_string()),
        ports.herdr.version().err().map(|e| e.code().to_string()),
        ports.harness.health(1).err().map(|e| e.code().to_string()),
    ];
    for code in codes {
        assert_eq!(code.as_deref(), Some("not-implemented"));
    }
}

/// A verb reached with `--format` JSON never writes a second line: the seam holds for
/// a verb that is given its shared flags too.
#[test]
fn seam_json_mode_with_shared_flags_is_still_one_envelope() {
    let run = run_verb(
        &["pane", "launch", "--profile", "demo", "--spec-only"],
        Format::Json,
    );
    assert_eq!(run.code, 1);
    assert_eq!(one_envelope(&run.out)["error"]["code"], "not-implemented");
    assert!(run.err.is_empty(), "{run:?}");
}
