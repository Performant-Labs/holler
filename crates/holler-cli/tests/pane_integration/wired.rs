//! D6 and A's W-E: the ports `Wiring::connect` builds, read through `Wiring::ports`, with a
//! malformed `HOLLER_TMUX_SOCKET`. The host and the harness each answer `usage` naming the
//! variable at their first use: neither falls back to `TmuxSocket::Default` (the operator's
//! live tmux server) and neither is an `Unwired` port (which would answer `not-implemented`).
//! Two malformations: a relative path, and a value that is not UTF-8 (A's W-T: `connect` must
//! read it as malformed, never as unset).
//!
//! `connect` reads its settings from this process's environment, so the test sets them under
//! the state-dir lock ([`with_env`]) against a real hub. "Never reaches the default socket"
//! is observed, not assumed: `TMUX_TMPDIR` points at a trap directory (tmux puts its default
//! socket there, as `tmux-<uid>/default`), `TMUX` and `TMUX_PANE` are removed, and the
//! OpenCode binary is a script that leaves a mark when it runs. A tmux server that a broken
//! wiring starts in the trap is killed before the test reports.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Stdio};

use holler_cli::pane::wiring::Wiring;
use holler_pane::{HarnessEndpoint, PaneError, PaneId, PaneName};

use crate::support::{Hub, StateDir};
use crate::traps::{opencode_trap, OPENCODE_MARK};
use crate::with_env;

/// What the run touched in the trap directory: a tmux socket directory, or the OpenCode mark.
fn touched(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("tmux-") || name == OPENCODE_MARK)
        .collect()
}

/// Kill every tmux server listening in the trap's `tmux-<uid>/` directories, so a wiring that
/// did reach the default socket leaves no server behind when the trap is removed.
fn kill_trapped_servers(dir: &Path) {
    let sockets = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("tmux-"))
        .flat_map(|entry| std::fs::read_dir(entry.path()).into_iter().flatten())
        .filter_map(Result::ok);
    for socket in sockets {
        let _ = Command::new("tmux")
            .arg("-S")
            .arg(socket.path())
            .arg("kill-server")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn usage_naming_the_socket(method: &str, result: Result<impl std::fmt::Debug, PaneError>) -> Option<String> {
    match result {
        Err(error) if error.code() == "usage" && error.to_string().contains("HOLLER_TMUX_SOCKET") => None,
        other => Some(format!(
            "{method}: want `usage` naming HOLLER_TMUX_SOCKET; got {other:?}"
        )),
    }
}

/// With `HOLLER_TMUX_SOCKET` set to `socket`: `connect` still succeeds (configuration errors
/// are lazy, D26); then every host and harness call tried answers `usage` naming the variable,
/// and nothing reached a tmux server or ran OpenCode.
fn assert_the_socket_is_usage_never_the_default_server(socket: &OsStr) {
    let state = StateDir::new();
    let _hub = Hub::start(&state);
    // The OpenCode trap's directory; `TMUX_TMPDIR` points at it too.
    let trap = opencode_trap();
    let tmpdir = trap.path().as_os_str().to_owned();
    let opencode = trap.path().join("opencode").into_os_string();
    let vars: &[(&str, Option<&OsStr>)] = &[
        ("HOLLER_TMUX_SOCKET", Some(socket)),
        ("HOLLER_OPENCODE_BIN", Some(&opencode)),
        ("HOLLER_OPENCODE_SERVE_ARGS", None),
        ("HOLLER_HERDR_SESSION", Some(OsStr::new("holler649-00000000"))),
        ("HOLLER_HERDR_SOCKET", Some(OsStr::new("/nonexistent-hlr649/herdr.sock"))),
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
            usage_naming_the_socket("host.foreground_commands", ports.host.foreground_commands(&name)),
            usage_naming_the_socket("harness.health", ports.harness.health(&HarnessEndpoint::Port(48199))),
            usage_naming_the_socket("harness.shown_session", ports.harness.shown_session(&pane, &HarnessEndpoint::Port(48199))),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    });
    let touched = touched(trap.path());
    kill_trapped_servers(trap.path());
    assert!(
        failures.is_empty() && touched.is_empty(),
        "HOLLER_TMUX_SOCKET={socket:?}\n{}\ntouched in the trap directory: {touched:?}",
        failures.join("\n")
    );
}

/// D6: a relative `HOLLER_TMUX_SOCKET` is `usage` from the host and the harness.
#[test]
fn a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server() {
    assert_the_socket_is_usage_never_the_default_server(OsStr::new("hlr649/tmux.sock"));
}

/// A's W-T, under D6: a `HOLLER_TMUX_SOCKET` that is not UTF-8 is malformed, not unset. It is
/// absolute, so only the encoding makes it malformed; read as unset, it would be tmux's
/// default server.
#[test]
fn a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server() {
    assert_the_socket_is_usage_never_the_default_server(OsStr::from_bytes(
        b"/tmp/hlr649-\xff/tmux.sock",
    ));
}
