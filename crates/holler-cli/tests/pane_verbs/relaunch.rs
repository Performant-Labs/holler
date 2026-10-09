//! `holler pane relaunch`: the stub cases of story #670. Story #644 owns the real verb and
//! replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;
use crate::verb_harness::parse::assert_spec_flags_accepted;

#[test]
fn pane_relaunch_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "relaunch"], 644);
}

/// AC 5: every shared spec flag (and all of them together) parses on `relaunch`.
#[test]
fn pane_relaunch_accepts_every_spec_flag() {
    assert_spec_flags_accepted("relaunch");
}
