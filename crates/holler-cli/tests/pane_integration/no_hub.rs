//! AC 1 and D2: with no hub running, every `holler pane ...` and `holler profile ...` verb
//! exits 1 with `unavailable`, naming the missing hub, and touches no adapter.
//!
//! "Touches no adapter" is observed, not assumed: every adapter setting points at a trap in
//! a scratch directory. The Herdr socket is a listener the test holds (a connect to it is
//! seen), the tmux socket is a path no tmux server may create, and the OpenCode binary is a
//! script that leaves a mark when it runs.

use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use crate::support::StateDir;
use crate::{holler_on, Out};

/// Every verb of both namespaces, with arguments that parse. The stubs are here too: with no
/// hub, `connect` fails before any verb runs, stub or not.
pub const EVERY_VERB: &[&[&str]] = &[
    &["pane", "list"],
    &["pane", "get", "hlr649-c1r1"],
    &["pane", "watch", "--until-idle"],
    &["pane", "launch", "hlr649-c1r1", "--herdr-session", "holler649-00000000"],
    &["pane", "relaunch", "hlr649-c1r1"],
    &["pane", "switch", "hlr649-c1r1", "ses_hlr649"],
    &["pane", "reset", "hlr649-c1r1", "--first", "hello"],
    &["pane", "park", "hlr649-c1r1", "--reason", "test", "--release-when", "manual"],
    &["pane", "unpark", "hlr649-c1r1"],
    &["pane", "close", "hlr649-c1r1"],
    &["pane", "doctor"],
    &["pane", "import"],
    &["profile", "create", "hlr649"],
    &["profile", "delete", "hlr649"],
    &["profile", "list"],
    &["profile", "show", "hlr649"],
    &["profile", "apply"],
    &["profile", "rename"],
    &["profile", "export"],
    &["profile", "import"],
];

/// The phrase the CLI prints for a missing hub (`say_cmd.rs`, `hub_cmd.rs`; A's W-9).
const NO_HUB: &str = "no live holler hub reachable";

/// The adapter traps of one run, in a scratch directory.
struct Traps {
    dir: tempfile::TempDir,
    herdr: UnixListener,
}

impl Traps {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("hlr649-")
            .tempdir_in("/tmp")
            .unwrap();
        let herdr = UnixListener::bind(dir.path().join("herdr.sock")).unwrap();
        herdr.set_nonblocking(true).unwrap();
        let script = dir.path().join("opencode");
        std::fs::write(
            &script,
            format!("#!/bin/sh\ntouch '{}'\n", dir.path().join("opencode-ran").display()),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir, herdr }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn text(&self, name: &str) -> String {
        self.path(name).display().to_string()
    }

    /// The wiring's whole environment, every value a trap.
    fn env(&self) -> Vec<(&'static str, String)> {
        vec![
            ("HOLLER_HERDR_SESSION", "holler649-00000000".to_owned()),
            ("HOLLER_HERDR_SOCKET", self.text("herdr.sock")),
            ("HOLLER_HERDR_WORKSPACES", "hlr649=2x1".to_owned()),
            ("HOLLER_TMUX_SOCKET", self.text("tmux.sock")),
            ("HOLLER_OPENCODE_BIN", self.text("opencode")),
        ]
    }

    /// What the run touched, if anything.
    fn touched(&self) -> Vec<String> {
        let mut touched = Vec::new();
        if self.herdr.accept().is_ok() {
            touched.push("the Herdr socket was connected to".to_owned());
        }
        if self.path("tmux.sock").exists() {
            touched.push("a tmux server was started on the tmux socket".to_owned());
        }
        if self.path("opencode-ran").exists() {
            touched.push("the OpenCode binary was run".to_owned());
        }
        touched
    }
}

fn run(state: &StateDir, traps: &Traps, argv: &[&str]) -> Out {
    let env = traps.env();
    let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    holler_on(state, argv, &env)
}

fn names_the_missing_hub(message: &str, state: &Path) -> bool {
    message.contains(NO_HUB) && message.contains(&state.display().to_string())
}

/// JSON mode: one `unavailable` envelope whose message names the missing hub and its state
/// dir, exit 1, and no adapter touched.
#[test]
fn every_verb_with_no_hub_is_unavailable_naming_the_hub_in_json() {
    let state = StateDir::new();
    let traps = Traps::new();
    let mut failures = Vec::new();
    for verb in EVERY_VERB {
        let mut argv = verb.to_vec();
        argv.push("--format=json");
        let out = run(&state, &traps, &argv);
        let envelope = holler_pane_testkit::envelope::check_envelope(&out.stdout, out.code);
        let error = envelope.ok().and_then(|e| e.error);
        let good = out.code == 1
            && error.as_ref().is_some_and(|e| {
                e.code == "unavailable" && names_the_missing_hub(&e.message, state.path())
            });
        if !good {
            failures.push(format!(
                "holler {argv:?}: want exit 1 and one `unavailable` envelope naming {NO_HUB:?} \
                 at {}; got {out:?}",
                state.path().display()
            ));
        }
        for touch in traps.touched() {
            failures.push(format!("holler {argv:?}: {touch}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Text mode: nothing on stdout, a stderr line with the `unavailable` code naming the missing
/// hub, exit 1, and no adapter touched.
#[test]
fn every_verb_with_no_hub_is_unavailable_naming_the_hub_in_text() {
    let state = StateDir::new();
    let traps = Traps::new();
    let mut failures = Vec::new();
    for verb in EVERY_VERB {
        let out = run(&state, &traps, verb);
        let line = out
            .stderr
            .lines()
            .any(|l| l.contains("unavailable") && names_the_missing_hub(l, state.path()));
        if out.code != 1 || !out.stdout.is_empty() || !line {
            failures.push(format!(
                "holler {verb:?}: want exit 1, empty stdout and an `unavailable` line naming \
                 {NO_HUB:?}; got {out:?}"
            ));
        }
        for touch in traps.touched() {
            failures.push(format!("holler {verb:?}: {touch}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
