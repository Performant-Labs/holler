//! `holler pane close`: the stub case of story #670. Story #646 owns the real verb and
//! replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn pane_close_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "close"], 646);
}
