//! D6 and A's W-E: the ports `Wiring::connect` builds, read through `Wiring::ports`, with a
//! malformed `HOLLER_TMUX_SOCKET`. The host and the harness each answer `usage` naming the
//! variable at their first use: neither falls back to `TmuxSocket::Default` (the operator's
//! live tmux server) and neither is an `Unwired` port (which would answer `not-implemented`).
//!
//! `connect` reads its settings from this process's environment, so the test sets them under
//! the state-dir lock ([`with_env`]) against a real hub. "Never reaches the default socket"
//! is observed, not assumed: `TMUX_TMPDIR` points at a trap directory (tmux puts its default
//! socket there, as `tmux-<uid>/default`), `TMUX` and `TMUX_PANE` are removed, and the
//! OpenCode binary is a script that leaves a mark when it runs.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use holler_cli::pane::wiring::Wiring;
use holler_pane::{PaneError, PaneId, PaneName};

use crate::support::{Hub, StateDir};
use crate::with_env;

/// A scratch directory holding the OpenCode trap; `TMUX_TMPDIR` points at it too.
fn trap_dir() -> tempfile::TempDir {
    let dir = tempfile::Builder::new()
        .prefix("hlr649-")
        .tempdir_in("/tmp")
        .unwrap();
    let script = dir.path().join("opencode");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", dir.path().join("opencode-ran").display()),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

/// What the run touched in the trap directory: a tmux socket directory, or the OpenCode mark.
fn touched(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("tmux-") || name == "opencode-ran")
        .collect()
}

fn usage_naming_the_socket(method: &str, result: Result<impl std::fmt::Debug, PaneError>) -> Option<String> {
    match result {
        Err(error) if error.code() == "usage" && error.to_string().contains("HOLLER_TMUX_SOCKET") => None,
        other => Some(format!(
            "{method}: want `usage` naming HOLLER_TMUX_SOCKET; got {other:?}"
        )),
    }
}

/// A relative `HOLLER_TMUX_SOCKET`: `connect` still succeeds (configuration errors are lazy,
/// D26); then every host and harness call tried answers `usage` naming the variable, and
/// nothing reached a tmux server or ran OpenCode.
#[test]
fn a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server() {
    let state = StateDir::new();
    let _hub = Hub::start(&state);
    let trap = trap_dir();
    let tmpdir = trap.path().display().to_string();
    let opencode = trap.path().join("opencode").display().to_string();
    let vars: &[(&str, Option<&str>)] = &[
        ("HOLLER_TMUX_SOCKET", Some("hlr649/tmux.sock")),
        ("HOLLER_OPENCODE_BIN", Some(&opencode)),
        ("HOLLER_OPENCODE_SERVE_ARGS", None),
        ("HOLLER_HERDR_SESSION", Some("holler649-00000000")),
        ("HOLLER_HERDR_SOCKET", Some("/nonexistent-hlr649/herdr.sock")),
        ("HOLLER_HERDR_WORKSPACES", None),
        ("TMUX_TMPDIR", Some(&tmpdir)),
        ("TMUX", None),
        ("TMUX_PANE", None),
    ];
    let failures = with_env(state.path(), vars, || {
        let wiring = Wiring::connect().expect("connect needs only the hub (D2, D26)");
        let ports = wiring.ports();
        let name = PaneName::parse("hlr649-c1r1").unwrap();
        let pane = PaneId::new("w1:p1");
        [
            usage_naming_the_socket("host.ensure_session", ports.host.ensure_session(&name, "/tmp")),
            usage_naming_the_socket("host.ps", ports.host.ps(&name)),
            usage_naming_the_socket("harness.health", ports.harness.health(48199)),
            usage_naming_the_socket("harness.shown_session", ports.harness.shown_session(&pane)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    });
    let touched = touched(trap.path());
    assert!(
        failures.is_empty() && touched.is_empty(),
        "{}\ntouched in the trap directory: {touched:?}",
        failures.join("\n")
    );
}
