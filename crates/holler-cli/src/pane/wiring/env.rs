//! What the wiring reads from the environment (#649, D5 to D7): the Herdr session, its socket
//! and its workspaces, the tmux socket, and the OpenCode binary and server arguments. The
//! parent module's doc lists every variable.
//!
//! Each function reads through the [`Lookup`] it is given (`std::env::var` in production), so
//! a test needs no process environment. A malformed or missing value is an error that names
//! its variable. The wiring keeps it, and the port the value configures answers it at its first
//! use, never at `connect` (D26), so the verbs that read only the registries work whatever these
//! variables say.
//!
//! Every path a variable gives must be absolute: a relative or empty one is `usage` naming the
//! variable, never a path resolved against the working directory, and never a fallback (D6:
//! a bad `HOLLER_TMUX_SOCKET` must not reach tmux's default server, the operator's live one).

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use holler_adapter_herdr::adapter::HerdrConfig;
use holler_adapter_herdr::plan::Extent;
use holler_adapter_host::TmuxSocket;
use holler_pane::PaneError;

/// How the wiring reads a variable: its value, or `None` when it is unset (or not UTF-8).
pub type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The one Herdr session the panes live in.
const HERDR_SESSION: &str = "HOLLER_HERDR_SESSION";
/// That session's socket.
const HERDR_SOCKET: &str = "HOLLER_HERDR_SOCKET";
/// Herdr's own name for its socket, set inside a Herdr pane (E25): the socket's fallback.
const HERDR_SOCKET_PATH: &str = "HERDR_SOCKET_PATH";
/// The rows by columns of each workspace (`label=RxC`, comma-separated).
const HERDR_WORKSPACES: &str = "HOLLER_HERDR_WORKSPACES";
/// The tmux server's socket, for the host and the harness both.
const TMUX_SOCKET: &str = "HOLLER_TMUX_SOCKET";
/// The `opencode` binary.
const OPENCODE_BIN: &str = "HOLLER_OPENCODE_BIN";
/// The arguments appended to `opencode serve`.
const OPENCODE_SERVE_ARGS: &str = "HOLLER_OPENCODE_SERVE_ARGS";

/// The Herdr adapter's configuration (D5): the session `HOLLER_HERDR_SESSION` names, its socket,
/// the workspaces `HOLLER_HERDR_WORKSPACES` lists, and the adapter's own 10 s bound (D9).
///
/// The session is `HOLLER_HERDR_SESSION` only: Herdr's `HERDR_SESSION` is no fallback, since no
/// source shows that Herdr sets it in its panes. The socket is `HOLLER_HERDR_SOCKET`, else
/// `HERDR_SOCKET_PATH`, else `$XDG_CONFIG_HOME/herdr/sessions/<session>/herdr.sock`, with
/// `$HOME/.config` for an `XDG_CONFIG_HOME` that is unset, empty or relative (the XDG rule).
///
/// `usage`, naming the variable: no session (or an empty one), a socket variable that is not an
/// absolute path, no absolute directory to find the session's socket under, and a malformed
/// workspace list: an item that is empty, has no `=`, more than one `=` or no `x`, has an empty
/// label or a row or column count that is not a whole number from 1 to 65535, or repeats a label.
/// The empty list is no workspace.
pub fn herdr_config(env: Lookup<'_>) -> Result<HerdrConfig, PaneError> {
    let session = env(HERDR_SESSION)
        .filter(|session| !session.is_empty())
        .ok_or_else(|| {
            usage(format!(
                "no Herdr session is configured: set {HERDR_SESSION} to the Herdr session the \
                 panes live in"
            ))
        })?;
    let socket = herdr_socket(env, &session)?;
    let workspaces = match env(HERDR_WORKSPACES) {
        Some(list) => workspaces(&list)?,
        None => BTreeMap::new(),
    };
    Ok(HerdrConfig {
        workspaces,
        ..HerdrConfig::new(session, socket)
    })
}

/// The tmux server of the host and the harness (D6): `HOLLER_TMUX_SOCKET` as a socket path, or
/// tmux's default socket when it is unset. A relative or empty value is `usage` naming the
/// variable; it never falls back to the default socket.
pub fn tmux_socket(env: Lookup<'_>) -> Result<TmuxSocket, PaneError> {
    match env(TMUX_SOCKET) {
        Some(value) => absolute(TMUX_SOCKET, value).map(TmuxSocket::Path),
        None => Ok(TmuxSocket::Default),
    }
}

/// The `opencode` binary (D7), always an absolute path, since tmux starts it: `HOLLER_OPENCODE_BIN`
/// (`usage` when it is not absolute), else the first executable `opencode` in an absolute
/// directory of `PATH`. None found is `unavailable` naming `HOLLER_OPENCODE_BIN`.
pub fn opencode_bin(env: Lookup<'_>) -> Result<PathBuf, PaneError> {
    if let Some(value) = env(OPENCODE_BIN) {
        return absolute(OPENCODE_BIN, value);
    }
    let path = env("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join("opencode"))
        .find(|candidate| is_executable(candidate))
        .ok_or_else(|| PaneError::Unavailable {
            what: format!(
                "no opencode binary: none is executable on PATH; set {OPENCODE_BIN} to its \
                 absolute path"
            ),
        })
}

/// What the harness appends to `opencode serve --port P --hostname 127.0.0.1` (D7):
/// `HOLLER_OPENCODE_SERVE_ARGS` as a JSON array of strings, or nothing when it is unset. Any
/// other value is `usage` naming the variable; the message quotes none of it.
pub fn serve_args(env: Lookup<'_>) -> Result<Vec<String>, PaneError> {
    let Some(text) = env(OPENCODE_SERVE_ARGS) else {
        return Ok(Vec::new());
    };
    serde_json::from_str(&text).map_err(|_| {
        usage(format!(
            "{OPENCODE_SERVE_ARGS} must be a JSON array of strings, such as [\"--pure\"]"
        ))
    })
}

/// D5's socket: the first of `HOLLER_HERDR_SOCKET` and `HERDR_SOCKET_PATH` that is set (it must be
/// absolute), else the session's socket under the configuration directory.
fn herdr_socket(env: Lookup<'_>, session: &str) -> Result<PathBuf, PaneError> {
    for variable in [HERDR_SOCKET, HERDR_SOCKET_PATH] {
        if let Some(value) = env(variable) {
            return absolute(variable, value);
        }
    }
    let config = config_home(env).ok_or_else(|| {
        usage(format!(
            "no Herdr socket for the session {session:?}: neither XDG_CONFIG_HOME nor HOME is an \
             absolute directory; set {HERDR_SOCKET}"
        ))
    })?;
    Ok(config
        .join("herdr")
        .join("sessions")
        .join(session)
        .join("herdr.sock"))
}

/// `$XDG_CONFIG_HOME`, else `$HOME/.config`, each only when it is an absolute path: the XDG base
/// directory rule ignores a value that is empty or relative.
fn config_home(env: Lookup<'_>) -> Option<PathBuf> {
    let directory = |variable: &str| {
        env(variable)
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
    };
    directory("XDG_CONFIG_HOME").or_else(|| directory("HOME").map(|home| home.join(".config")))
}

/// `HOLLER_HERDR_WORKSPACES`: items `label=RxC` (rows by columns) joined by `,`, the empty value
/// being no workspace. `usage` naming the variable for the first item that is empty, has no `=`,
/// more than one `=` or no `x`, has an empty label or a count that is not a whole number from 1 to
/// 65535, or repeats a label.
fn workspaces(list: &str) -> Result<BTreeMap<String, Extent>, PaneError> {
    let mut workspaces = BTreeMap::new();
    if list.is_empty() {
        return Ok(workspaces);
    }
    for item in list.split(',') {
        let (label, extent) = workspace(item)?;
        if workspaces.insert(label.to_owned(), extent).is_some() {
            return Err(bad_workspace(item, "its label is given twice"));
        }
    }
    Ok(workspaces)
}

/// One item of [`workspaces`]: its label and its extent.
fn workspace(item: &str) -> Result<(&str, Extent), PaneError> {
    if item.is_empty() {
        return Err(bad_workspace(item, "it is empty (a stray `,`)"));
    }
    let (label, size) = item
        .split_once('=')
        .ok_or_else(|| bad_workspace(item, "it has no `=`"))?;
    if label.is_empty() {
        return Err(bad_workspace(item, "its label is empty"));
    }
    if size.contains('=') {
        return Err(bad_workspace(
            item,
            "it has more than one `=`, and a label holds none",
        ));
    }
    let (rows, cols) = size
        .split_once('x')
        .ok_or_else(|| bad_workspace(item, "its size has no `x` between the rows and columns"))?;
    let extent = Extent {
        rows: count(item, rows)?,
        cols: count(item, cols)?,
    };
    Ok((label, extent))
}

/// A row or column count of the item `item`: ASCII digits only, from 1 to 65535.
fn count(item: &str, text: &str) -> Result<u16, PaneError> {
    let digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    match text.parse::<u16>() {
        Ok(count) if digits && count > 0 => Ok(count),
        _ => Err(bad_workspace(
            item,
            "its rows and columns must be whole numbers from 1 to 65535",
        )),
    }
}

/// `usage`: the `HOLLER_HERDR_WORKSPACES` item `item` is malformed, for the reason `why`.
fn bad_workspace(item: &str, why: &str) -> PaneError {
    usage(format!(
        "{HERDR_WORKSPACES} is a comma-separated list of label=RxC (rows by columns), and the \
         item {item:?} is not one: {why}"
    ))
}

/// The value of `variable` as a path: `usage` naming the variable when it is not absolute.
fn absolute(variable: &str, value: String) -> Result<PathBuf, PaneError> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(usage(format!(
            "{variable} must be an absolute path, not {path:?}"
        )))
    }
}

/// Whether `path` is a file someone may execute.
fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// `usage` with `message`.
fn usage(message: String) -> PaneError {
    PaneError::Usage { message }
}
