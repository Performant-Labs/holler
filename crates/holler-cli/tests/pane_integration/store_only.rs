//! AC 2 and D2: the verbs that read only the registries work against a hub with no Herdr, no
//! tmux and no OpenCode present; and D13: a hub restarted on the same state dir serves the
//! registry it had.
//!
//! "Not present" is made so: the binary runs with `PATH` set to an empty directory and every
//! Herdr, tmux and OpenCode setting removed, so a verb that reached an adapter would fail.
//! The pane record is written through the wiring's own client (`HubPaneStore`), the profile
//! through `holler profile create`.

use std::time::Duration;

use holler_cli::pane::wiring::store::HubPaneStore;
use holler_pane::PaneStore;
use holler_pane_testkit::fixture::sample_pane;
use serde_json::Value;

use crate::support::{Hub, StateDir};
use crate::{holler_on, with_state_dir, Out};

pub const PANE: &str = "hlr649-c1r1";
const PROFILE: &str = "hlr649";

/// A hub on a fresh state dir, and a directory to use as an empty `PATH`.
pub struct Bare {
    pub state: StateDir,
    pub hub: Hub,
    empty: tempfile::TempDir,
}

impl Bare {
    pub fn start() -> Self {
        let state = StateDir::new();
        let hub = Hub::start(&state);
        let empty = tempfile::tempdir().unwrap();
        Self { state, hub, empty }
    }

    /// `holler <args>` with no adapter reachable: an empty `PATH`, nothing configured.
    pub fn holler(&self, args: &[&str]) -> Out {
        let path = self.empty.path().display().to_string();
        holler_on(&self.state, args, &[("PATH", path.as_str())])
    }

    /// Write the sample pane `name` through the wiring's pane client.
    pub fn seed_pane(&self, name: &str) {
        with_state_dir(self.state.path(), || {
            HubPaneStore::new()
                .cas_put(&sample_pane(name).unwrap(), 0)
                .unwrap_or_else(|e| panic!("cas_put {name} through the client: {e}"))
        });
    }
}

/// `pane list` and `pane get` read the record; `pane watch --until-idle` prints its put;
/// `profile create`, `list` and `show` read and write the profile registry. Each succeeds
/// with nothing but the hub.
#[test]
fn the_store_only_verbs_work_with_no_herdr_tmux_or_opencode() {
    let bare = Bare::start();
    bare.seed_pane(PANE);

    let list = bare.holler(&["pane", "list", "--format=json"]);
    assert_eq!(list.code, 0, "pane list: {list:?}");
    assert!(
        list.data().to_string().contains(PANE),
        "pane list shows the record: {list:?}"
    );

    let get = bare.holler(&["pane", "get", PANE, "--format=json"]);
    assert_eq!(get.code, 0, "pane get: {get:?}");
    assert!(get.data().to_string().contains(PANE), "pane get: {get:?}");

    let watch = bare.holler(&["pane", "watch", "--until-idle", "--format=json"]);
    assert_eq!(watch.code, 0, "pane watch: {watch:?}");
    assert!(watch.stdout.contains(PANE), "pane watch prints the put: {watch:?}");

    let create = bare.holler(&["profile", "create", PROFILE, "--format=json"]);
    assert_eq!(create.code, 0, "profile create: {create:?}");

    let profiles = bare.holler(&["profile", "list", "--format=json"]);
    assert_eq!(profiles.code, 0, "profile list: {profiles:?}");
    assert!(
        profiles.data().to_string().contains(PROFILE),
        "profile list shows the profile: {profiles:?}"
    );

    let show = bare.holler(&["profile", "show", PROFILE, "--format=json"]);
    assert_eq!(show.code, 0, "profile show: {show:?}");
}

/// A verb that needs Herdr, with none configured, fails at its first Herdr call with
/// `usage` naming the two session variables (D5); the store-only verbs above are unaffected
/// by the same missing setting.
#[test]
fn a_herdr_verb_with_no_session_configured_is_usage_naming_the_variables() {
    let bare = Bare::start();
    let out = bare.holler(&[
        "pane",
        "launch",
        PANE,
        "--herdr-session",
        "holler649-00000000",
        "--project",
        "/tmp",
        "--workspace",
        "hlr649",
        "--grid",
        "r1c1",
        "--model",
        "deadend/none",
        "--effort",
        "medium",
        "--ctx-soft",
        "100000",
        "--ctx-hard",
        "150000",
        "--port-policy",
        "fixed:48100",
        "--format=json",
    ]);
    let error = out.envelope().error.unwrap_or_else(|| panic!("a failure: {out:?}"));
    assert_eq!(error.code, "usage", "{out:?}");
    for variable in ["HOLLER_HERDR_SESSION", "HERDR_SESSION"] {
        assert!(
            error.message.contains(variable),
            "the message names {variable}: {out:?}"
        );
    }
    let list = bare.holler(&["pane", "list", "--format=json"]);
    assert_eq!(list.code, 0, "nothing was written: {list:?}");
    assert_eq!(list.data()["panes"], Value::Array(Vec::new()), "{list:?}");
}

/// D13: a hub restarted on the same state dir serves the same registry: `pane list` after
/// the restart equals `pane list` before it.
#[test]
fn a_restarted_hub_serves_the_registry_it_had() {
    let bare = Bare::start();
    bare.seed_pane("hlr649-c1r1");
    bare.seed_pane("hlr649-c2r1");
    let before = bare.holler(&["pane", "list", "--format=json"]);
    assert_eq!(before.code, 0, "{before:?}");
    assert_eq!(
        before.data()["panes"].as_array().map(Vec::len),
        Some(2),
        "two records: {before:?}"
    );

    let Bare { state, hub, empty } = bare;
    hub.stop(Duration::from_secs(5));
    let gone = holler_on(&state, &["pane", "list", "--format=json"], &[]);
    assert_eq!(
        gone.error_code().as_deref(),
        Some("unavailable"),
        "the hub is down between the two runs: {gone:?}"
    );
    let hub = Hub::start(&state);
    let bare = Bare { state, hub, empty };

    let after = bare.holler(&["pane", "list", "--format=json"]);
    assert_eq!(after.code, 0, "{after:?}");
    assert_eq!(after.data(), before.data(), "the registry is intact");
}
