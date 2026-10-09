//! `holler pane get`: the stub case of story #670. Story #643 owns the real verb and
//! replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn pane_get_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "get"], 643);
}
