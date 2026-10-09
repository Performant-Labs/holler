//! `holler profile delete`: the stub case of story #670. Story #662 owns the real verb
//! and replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn profile_delete_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["profile", "delete"], 662);
}
