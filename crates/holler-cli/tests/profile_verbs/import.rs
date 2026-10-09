//! `holler profile import`: the stub case of story #670. Story #665 owns the real verb (proposed: the operator confirms #665 first)
//! and replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

#[test]
fn profile_import_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["profile", "import"], 665);
}
