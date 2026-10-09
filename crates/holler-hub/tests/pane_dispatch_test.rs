#![allow(clippy::unwrap_used, clippy::expect_used)] // #669
#![allow(clippy::panic, clippy::unreachable)] // #669
//! Issue #669: the hub's plumbing for the `pane/*` and `profile/*` control methods
//! (epic #633, skeleton slice b).
//!
//! A `pane/*` or `profile/*` request on the control socket used to fall through to
//! `method_not_found`. This slice forwards both method lists to stub dispatchers
//! that answer `not-implemented` as a `PaneReply` in a JSON-RPC **result**, and
//! hands the two state handles to every connection as shared `Arc`s. The stories
//! #639 (panes) and #661 (profiles) replace the stubs; these tests pin the plumbing
//! they build on.
//!
//! Issue #639 filled the `pane/*` stub, so the pane-method assertions below expect the
//! real registry's answers (an empty list, a decode error, an empty batch) rather than
//! `not-implemented`. Issue #661 filled the `profile/*` stub the same way, so the
//! profile-method assertions expect the profile registry's answers too; only
//! `profile/rename` (PROPOSED, #665) still answers `not-implemented`. Both registries are
//! loaded with the short long-poll window, so no test here waits the real one.
//!
//! The tests drive `handle_control_conn` in process over a `UnixStream::pair()`
//! with a real `Registry`, `Roster` and `Lockout`. They spawn no `holler` binary,
//! set no `HOLLER_STATE_DIR` and never sleep: every read is a bounded wait on the
//! reply line itself.

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use holler_hub::control_server::handle_control_conn;
use holler_hub::live::Registry;
use holler_hub::lockout::Lockout;
use holler_hub::pane_dispatch::PaneDeps;
use holler_hub::panes::{self, PaneState};
use holler_hub::profile::{self, check_membership, ProfileState};
use holler_hub::roster::{Config, Roster};
use holler_pane::{PaneError, PaneReply};
use holler_proto::methods::{PANE_METHODS, PROFILE_METHODS};
use holler_proto::CorrelationId;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::task::JoinHandle;

mod pane_support;
use pane_support::{pane_outcome, sample_pane, short_opts, temp_state};

// The test-kit crate is an empty skeleton today; linking it here is what makes the
// dev-dependency real, so #638, #639 and #661 add no manifest line.
use holler_pane_testkit as _;

/// JSON-RPC `-32601`, the wire code of `method_not_found`.
const METHOD_NOT_FOUND: i64 = -32601;

/// How long a reply may take. A bounded wait on the line, not a sleep: a hang fails
/// the test instead of the run.
const REPLY_WITHIN: Duration = Duration::from_secs(10);

/// One in-process control connection: the client end of the pair, and the task
/// running `handle_control_conn` on the other end.
struct Conn {
    client: BufReader<UnixStream>,
    _server: JoinHandle<()>,
}

/// Start a connection served with `deps`. Registry, roster and lockout are fresh and
/// real; the methods these tests send never resolve the state dir, so none is touched.
fn connect(deps: PaneDeps) -> Conn {
    let (client, server) = UnixStream::pair().unwrap();
    let registry = Registry::new();
    let roster = Arc::new(Roster::with_system_clock(&Config::default()));
    let lockout = Lockout::new();
    let task = tokio::spawn(handle_control_conn(server, registry, roster, lockout, deps));
    Conn {
        client: BufReader::new(client),
        _server: task,
    }
}

impl Conn {
    /// Send one request line and return the one reply line, parsed.
    async fn call(&mut self, id: &str, method: &str) -> Value {
        let req = json!({"jsonrpc": "2.0", "id": id, "method": method});
        let line = format!("{req}\n");
        self.client
            .get_mut()
            .write_all(line.as_bytes())
            .await
            .unwrap();
        let mut reply = String::new();
        let n = tokio::time::timeout(REPLY_WITHIN, self.client.read_line(&mut reply))
            .await
            .unwrap_or_else(|_| panic!("no reply to {method} within {REPLY_WITHIN:?}"))
            .unwrap();
        assert!(
            n > 0,
            "the hub closed the connection instead of answering {method}"
        );
        serde_json::from_str(&reply)
            .unwrap_or_else(|e| panic!("reply to {method} is not JSON ({e}): {reply:?}"))
    }
}

/// Fresh, empty state handles loaded from a throwaway state dir. Both registries have the
/// short long-poll window.
fn fresh_deps() -> (PaneDeps, tempfile::TempDir) {
    let (dir, state) = temp_state();
    let deps = PaneDeps {
        panes: Arc::new(PaneState::load_with(&state, short_opts())),
        profiles: Arc::new(ProfileState::load_with(&state, short_opts())),
    };
    (deps, dir)
}

fn error_code(reply: &Value) -> i64 {
    reply
        .pointer("/error/code")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("expected a JSON-RPC error with a numeric code: {reply}"))
}

// --- AC 1: forwarding -------------------------------------------------------------

/// What each pane method answers to a request that carries no `params`, now that the
/// registry is real: a decode error, an empty list, or an empty batch. Never
/// `not-implemented`, never `method_not_found`.
fn assert_answers_an_empty_request(method: &str, outcome: Result<Option<Value>, PaneError>) {
    match method {
        "pane/list" => assert_eq!(outcome, Ok(Some(json!([]))), "{method}"),
        "pane/watch" => assert_eq!(
            outcome,
            Ok(Some(json!({"events": [], "cursor": 0}))),
            "{method}"
        ),
        "pane/get" | "pane/cas_put" | "pane/delete" => assert!(
            matches!(outcome, Err(PaneError::Usage { .. })),
            "{method} without params must be a usage error, got {outcome:?}"
        ),
        other => panic!("{other} is a new pane method: add its expectation here"),
    }
}

#[tokio::test]
async fn every_pane_method_is_forwarded_to_the_registry_not_method_not_found() {
    let (deps, _dir) = fresh_deps();
    let mut conn = connect(deps);
    // One connection for the whole table: every method, `pane/watch` (long-poll, last
    // in the list) included, answers once and the connection stays usable for the
    // next request.
    for (i, method) in PANE_METHODS.iter().enumerate() {
        let id = format!("h-pane-{i}");
        let reply = conn.call(&id, method).await;
        assert_eq!(reply["id"], json!(id), "{method} must echo the request id");
        assert_answers_an_empty_request(method, pane_outcome(&reply, method));
    }
}

/// What each profile method answers to a request that carries no `params`, now that the
/// registry is real: a decode error, an empty list, or an empty batch. Only
/// `profile/rename` (#665) is still `not-implemented`. Never `method_not_found`.
fn assert_answers_an_empty_profile_request(
    method: &str,
    outcome: Result<Option<Value>, PaneError>,
) {
    match method {
        "profile/list" => assert_eq!(outcome, Ok(Some(json!([]))), "{method}"),
        "profile/watch" => assert_eq!(
            outcome,
            Ok(Some(json!({"events": [], "cursor": 0}))),
            "{method}"
        ),
        "profile/get" | "profile/cas_put" | "profile/delete" | "profile/log" => assert!(
            matches!(outcome, Err(PaneError::Usage { .. })),
            "{method} without params must be a usage error, got {outcome:?}"
        ),
        "profile/rename" => assert_eq!(outcome, Err(PaneError::NotImplemented), "{method}"),
        other => panic!("{other} is a new profile method: add its expectation here"),
    }
}

#[tokio::test]
async fn every_profile_method_is_forwarded_to_the_registry_not_method_not_found() {
    let (deps, _dir) = fresh_deps();
    let mut conn = connect(deps);
    for (i, method) in PROFILE_METHODS.iter().enumerate() {
        let id = format!("h-profile-{i}");
        let reply = conn.call(&id, method).await;
        assert_eq!(reply["id"], json!(id), "{method} must echo the request id");
        assert_answers_an_empty_profile_request(method, pane_outcome(&reply, method));
    }
}

// --- AC 2: nothing else changes ---------------------------------------------------

#[tokio::test]
async fn unknown_methods_still_answer_method_not_found() {
    let (deps, _dir) = fresh_deps();
    let mut conn = connect(deps);
    // `pane/frobnicate` and `profile/frobnicate` pin that the arm forwards the two
    // method LISTS, not every method with a `pane/` or `profile/` prefix.
    for method in [
        "control/x",
        "foo/bar",
        "pane/frobnicate",
        "profile/frobnicate",
    ] {
        let reply = conn.call("h-unknown-1", method).await;
        assert_eq!(
            error_code(&reply),
            METHOD_NOT_FOUND,
            "{method} must stay method_not_found: {reply}"
        );
    }
}

#[tokio::test]
async fn an_existing_control_method_still_answers_through_the_new_dispatcher() {
    let (deps, _dir) = fresh_deps();
    let mut conn = connect(deps);
    // `control/roster` reads only the in-memory roster, and it sits in the `control/`
    // arm right behind the new pane arm. The other control handlers resolve the state
    // dir from the environment (and `control/status` can mint a hub identity key
    // there), which an in-process test must not touch.
    let reply = conn.call("h-roster-1", "control/roster").await;
    assert!(
        reply.get("error").is_none(),
        "control/roster must still succeed: {reply}"
    );
    assert_eq!(
        reply["result"]["rows"],
        json!([]),
        "control/roster on a fresh roster must list no rows: {reply}"
    );
    // The control reply is the hub's own document, not a PaneReply.
    assert!(
        serde_json::from_value::<PaneReply>(reply["result"].clone()).is_err(),
        "control/roster was swallowed by the pane arm: {reply}"
    );
}

// --- AC 3: the state handles ------------------------------------------------------

#[tokio::test]
async fn two_connections_share_the_one_pair_of_state_handles() {
    let (deps, _dir) = fresh_deps();
    // What `serve.rs` does per accepted connection: clone the bundle of `Arc`s.
    let deps_a = deps.clone();
    let deps_b = deps.clone();
    assert!(
        Arc::ptr_eq(&deps_a.panes, &deps_b.panes),
        "the PaneState must be one shared handle"
    );
    assert!(
        Arc::ptr_eq(&deps_a.profiles, &deps_b.profiles),
        "the ProfileState must be one shared handle"
    );

    let mut conn_a = connect(deps_a);
    let mut conn_b = connect(deps_b);
    // Both connections are live and each serves a request off the shared handles.
    let a = conn_a.call("h-a-1", "pane/list").await;
    let b = conn_b.call("h-b-1", "profile/list").await;
    assert_eq!(pane_outcome(&a, "pane/list"), Ok(Some(json!([]))));
    assert_eq!(pane_outcome(&b, "profile/list"), Ok(Some(json!([]))));

    // The original plus one clone held by each live connection task: three owners of
    // ONE allocation. A per-connection copy of the state would leave this at one.
    assert_eq!(
        Arc::strong_count(&deps.panes),
        3,
        "each connection must hold the shared PaneState, not a copy"
    );
    assert_eq!(
        Arc::strong_count(&deps.profiles),
        3,
        "each connection must hold the shared ProfileState, not a copy"
    );
}

/// Reports whether `T: Clone` without naming a `Clone` bound at the call site: the
/// inherent const wins when the bound holds, the trait default applies otherwise.
trait NotClone {
    const IS_CLONE: bool = false;
}
impl<T> NotClone for T {}
struct IsClone<T>(PhantomData<T>);
#[allow(dead_code)] // #669 (the const is read through the type, which the lint does not see)
impl<T: Clone> IsClone<T> {
    const IS_CLONE: bool = true;
}

#[test]
fn the_state_types_do_not_derive_clone_so_a_connection_cannot_fork_a_copy() {
    // A `Clone` here would let a later inline-state change copy the state per
    // connection and defeat the compare-and-swap; sharing goes through `Arc`.
    assert!(
        !std::hint::black_box(IsClone::<PaneState>::IS_CLONE),
        "PaneState must not be Clone"
    );
    assert!(
        !std::hint::black_box(IsClone::<ProfileState>::IS_CLONE),
        "ProfileState must not be Clone"
    );
    // The bundle of `Arc`s is what is cloned per connection.
    assert!(
        std::hint::black_box(IsClone::<PaneDeps>::IS_CLONE),
        "PaneDeps must be Clone (it is cloned per connection)"
    );
}

// --- AC 4: the dispatcher signatures ---------------------------------------------

#[tokio::test]
async fn panes_dispatch_takes_both_handles_and_answers_as_a_result() {
    let (deps, _dir) = fresh_deps();
    let cid = CorrelationId::parse("h-direct-1").unwrap();
    let line = panes::dispatch("pane/watch", &cid, &json!({}), &deps.panes, &deps.profiles).await;
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(
        pane_outcome(&reply, "pane/watch"),
        Ok(Some(json!({"events": [], "cursor": 0}))),
        "an idle registry answers an empty batch: {reply}"
    );
}

#[tokio::test]
async fn profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub() {
    let (deps, _dir) = fresh_deps();
    let cid = CorrelationId::parse("h-direct-2").unwrap();
    // The handles arrive in the opposite order to `panes::dispatch`: the profile
    // store first, the pane store second (`profile/rename` reaches the panes). With no
    // params `profile/get` is a usage error from the real registry; `profile/rename` is
    // still #665's stub.
    for method in ["profile/get", "profile/rename"] {
        let line = profile::dispatch(method, &cid, &json!({}), &deps.profiles, &deps.panes).await;
        let reply: Value = serde_json::from_str(&line).unwrap();
        let outcome = pane_outcome(&reply, method);
        match method {
            "profile/get" => assert!(
                matches!(outcome, Err(PaneError::Usage { .. })),
                "{method}: {reply}"
            ),
            _ => assert_eq!(outcome, Err(PaneError::NotImplemented), "{method}: {reply}"),
        }
    }
}

// --- AC 5: the membership hook ----------------------------------------------------

#[test]
fn check_membership_requires_the_named_profile_to_exist() {
    let (_dir, state) = temp_state();
    let profiles = ProfileState::load_with(&state, short_opts());
    // A pane outside any profile is accepted without touching the profile registry.
    assert_eq!(
        check_membership(&sample_pane("hj-c1r1", None), &profiles),
        Ok(())
    );
    // A pane naming a profile that does not exist is refused.
    let named = sample_pane("hj-c1r1", Some("No Such Profile"));
    let err = check_membership(&named, &profiles).unwrap_err();
    assert!(
        matches!(err, PaneError::ProfileNotFound { .. }),
        "got {err:?}"
    );
    assert_eq!(err.code(), "profile-not-found");
    // Once the profile exists, the same pane is accepted.
    pane_support::create_profile(&profiles, "No Such Profile");
    assert_eq!(check_membership(&named, &profiles), Ok(()));
}

// --- AC 6: the manifest -----------------------------------------------------------

#[test]
fn testkit_links() {
    // The `use holler_pane_testkit as _;` above is the consumer: this test exists so
    // the dev-dependency is exercised by name, and fails to build without it.
    let _linked: PhantomData<()> = PhantomData;
}
