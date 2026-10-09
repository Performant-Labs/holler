//! `holler pane list`: the stub case of story #670. Story #643 owns the real verb and
//! replaces this file's cases with its own.

use crate::verb_harness::assert_stub_routes;

// The consumer of `holler-pane-testkit` (a dev-dependency declared by #670 so that
// #638 and #643-#647 add no manifest line). #643 replaces this with its real use of the
// fakes; the link itself is the assertion: this target does not build without it.
use holler_pane_testkit as _;

#[test]
fn testkit_links() {}

#[test]
fn pane_list_stub_routes_text_to_err_and_json_to_out() {
    assert_stub_routes(&["pane", "list"], 643);
}
