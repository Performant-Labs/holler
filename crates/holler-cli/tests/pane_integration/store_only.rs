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
    pub empty: tempfile::TempDir,
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
    // D33 (#832): with no Herdr configured the unmanaged panes are not listed, and the run
    // says why, by the Herdr port's own code (JSON) and the variable to set (text).
    assert_eq!(list.data()["unmanaged_unavailable"], "usage", "{list:?}");
    let text = bare.holler(&["pane", "list"]);
    assert_eq!(text.code, 0, "pane list in text: {text:?}");
    assert!(
        text.stderr.contains("warning: unmanaged panes not listed")
            && text.stderr.contains("HOLLER_HERDR_SESSION"),
        "the text run warns, naming HOLLER_HERDR_SESSION: {text:?}"
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

/// A verb that needs Herdr, with no Herdr session configured but tmux and OpenCode
/// configured (each a trap that is never reached), fails at its first Herdr call with `usage`
/// naming `HOLLER_HERDR_SESSION`, and only that variable: `HERDR_SESSION` is no fallback (D5,
/// audit). The store-only verbs above are unaffected by the same missing setting, and nothing
/// is written. `HOME` and `XDG_CONFIG_HOME` are scratch, so a wiring that did fall back could
/// only resolve a Herdr socket inside the trap directory.
#[test]
fn a_herdr_verb_with_no_session_configured_is_usage_naming_holler_herdr_session() {
    let bare = Bare::start();
    let traps = tempfile::Builder::new().prefix("hlr649-").tempdir_in("/tmp").unwrap();
    let opencode = traps.path().join("opencode").display().to_string();
    let tmux = traps.path().join("tmux.sock").display().to_string();
    let empty = bare.empty.path().display().to_string();
    let home = traps.path().display().to_string();
    let out = holler_on(
        &bare.state,
        &[
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
        ],
        &[
            ("PATH", empty.as_str()),
            ("HOME", home.as_str()),
            ("XDG_CONFIG_HOME", home.as_str()),
            ("HERDR_SESSION", "holler649-0000000b"),
            ("HOLLER_TMUX_SOCKET", tmux.as_str()),
            ("HOLLER_OPENCODE_BIN", opencode.as_str()),
        ],
    );
    let error = out.envelope().error.unwrap_or_else(|| panic!("a failure: {out:?}"));
    assert_eq!(error.code, "usage", "{out:?}");
    assert!(
        error.message.contains("HOLLER_HERDR_SESSION")
            && !error.message.replace("HOLLER_HERDR_SESSION", "").contains("HERDR_SESSION"),
        "the message names HOLLER_HERDR_SESSION and offers no fallback: {out:?}"
    );
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
