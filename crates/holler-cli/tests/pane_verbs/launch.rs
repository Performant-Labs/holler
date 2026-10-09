//! `holler pane launch`: the stub cases of story #670. Story #644 owns the real verb and
//! replaces this file's cases with its own.

use holler_cli::output::Format;

use crate::verb_harness::parse::assert_spec_flags_accepted;
use crate::verb_harness::{assert_stub_routes, one_envelope, run_verb};

#[test]
fn pane_launch_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "launch"], 644);
}

/// A verb reached with `--format` JSON never writes a second line: the seam holds for a
/// verb that is given its shared flags too.
#[test]
fn pane_launch_json_mode_with_shared_flags_is_still_one_envelope() {
    let run = run_verb(
        &["pane", "launch", "--profile", "demo", "--spec-only"],
        Format::Json,
    );
    assert_eq!(run.code, 1);
    assert_eq!(one_envelope(&run.out)["error"]["code"], "not-implemented");
    assert!(run.err.is_empty(), "{run:?}");
}

/// AC 5: every shared spec flag (and all of them together) parses on `launch`.
#[test]
fn pane_launch_accepts_every_spec_flag() {
    assert_spec_flags_accepted("launch");
}
