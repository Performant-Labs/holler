//! D5 to D7 and A's W-13: what the wiring reads from the environment, through an injected
//! lookup (so no test here sets a process variable), and that a bad value fails closed at
//! the first use of the port it configures, never at `connect`.

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use holler_adapter_herdr::plan::Extent;
use holler_adapter_host::TmuxSocket;
use holler_cli::pane::wiring::env;
use holler_pane::PaneError;

use crate::store_only::{Bare, PANE};

/// A lookup over exactly `pairs`.
fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |name: &str| map.get(name).cloned()
}

fn usage_naming(result: Result<impl std::fmt::Debug, PaneError>, variable: &str) {
    let error = result.expect_err("refused");
    assert_eq!(error.code(), "usage", "{error}");
    assert!(error.to_string().contains(variable), "names {variable}: {error}");
}

// --- Herdr (D5) ------------------------------------------------------------------------

/// The message of `error` names `HOLLER_HERDR_SESSION` and offers no other variable for the
/// session: `HERDR_SESSION` is no fallback (D5, audit), so a message that suggests it is wrong.
fn names_only_hollers_session_variable(error: &PaneError) -> bool {
    let text = error.to_string();
    text.contains("HOLLER_HERDR_SESSION") && !text.replace("HOLLER_HERDR_SESSION", "").contains("HERDR_SESSION")
}

#[test]
fn the_herdr_session_is_hollers_variable() {
    let both = vars(&[
        ("HOLLER_HERDR_SESSION", "holler649-0000000a"),
        ("HERDR_SESSION", "holler649-0000000b"),
        ("HOLLER_HERDR_SOCKET", "/tmp/h/herdr.sock"),
    ]);
    assert_eq!(env::herdr_config(&both).unwrap().session, "holler649-0000000a");
}

/// D5 (audit): no `HERDR_SESSION` fallback. No source verifies that Herdr sets it in its
/// panes (the setup wizard marks it UNVERIFIED until #734), so a value there is ignored and
/// the session is still unconfigured.
#[test]
fn herdrs_own_session_variable_is_no_fallback() {
    let herdrs = vars(&[
        ("HERDR_SESSION", "holler649-0000000b"),
        ("HOLLER_HERDR_SOCKET", "/tmp/h/herdr.sock"),
    ]);
    let error = env::herdr_config(&herdrs).expect_err("HERDR_SESSION alone configures nothing");
    assert_eq!(error.code(), "usage", "{error}");
    assert!(names_only_hollers_session_variable(&error), "{error}");
}

#[test]
fn no_herdr_session_is_usage_naming_holler_herdr_session() {
    let none = vars(&[("HOLLER_HERDR_SOCKET", "/tmp/h/herdr.sock")]);
    let empty = vars(&[("HOLLER_HERDR_SESSION", ""), ("HOLLER_HERDR_SOCKET", "/tmp/h/herdr.sock")]);
    for (case, lookup) in [("unset", &none), ("empty", &empty)] {
        let error = env::herdr_config(lookup).expect_err(case);
        assert_eq!(error.code(), "usage", "{case}: {error}");
        assert!(names_only_hollers_session_variable(&error), "{case}: {error}");
    }
}

/// Every path a variable gives is absolute (the module doc's table): a relative or empty one is
/// `usage` naming that variable, never a path resolved against the working directory and never
/// the next fallback. D6's own row (`HOLLER_TMUX_SOCKET`) is pinned below.
#[test]
fn a_path_variable_that_is_not_absolute_is_usage_naming_it() {
    let s = ("HOLLER_HERDR_SESSION", "holler649-0000000a");
    for bad in ["", "herdr.sock", "./h/herdr.sock"] {
        usage_naming(
            env::herdr_config(&vars(&[s, ("HOLLER_HERDR_SOCKET", bad), ("HERDR_SOCKET_PATH", "/b/h.sock")])),
            "HOLLER_HERDR_SOCKET",
        );
        usage_naming(
            env::herdr_config(&vars(&[s, ("HERDR_SOCKET_PATH", bad), ("HOME", "/home/u")])),
            "HERDR_SOCKET_PATH",
        );
        usage_naming(env::opencode_bin(&vars(&[("HOLLER_OPENCODE_BIN", bad)])), "HOLLER_OPENCODE_BIN");
    }
}

#[test]
fn the_herdr_socket_is_hollers_variable_then_herdrs_then_the_session_default() {
    let s = ("HOLLER_HERDR_SESSION", "holler649-0000000a");
    let socket = |pairs: &[(&str, &str)]| env::herdr_config(&vars(pairs)).unwrap().socket;
    assert_eq!(
        socket(&[s, ("HOLLER_HERDR_SOCKET", "/a/h.sock"), ("HERDR_SOCKET_PATH", "/b/h.sock")]),
        PathBuf::from("/a/h.sock")
    );
    assert_eq!(
        socket(&[s, ("HERDR_SOCKET_PATH", "/b/h.sock"), ("XDG_CONFIG_HOME", "/x")]),
        PathBuf::from("/b/h.sock")
    );
    assert_eq!(
        socket(&[s, ("XDG_CONFIG_HOME", "/x"), ("HOME", "/home/u")]),
        PathBuf::from("/x/herdr/sessions/holler649-0000000a/herdr.sock")
    );
    assert_eq!(
        socket(&[s, ("HOME", "/home/u")]),
        PathBuf::from("/home/u/.config/herdr/sessions/holler649-0000000a/herdr.sock")
    );
}

#[test]
fn the_workspaces_are_label_rows_by_columns() {
    let config = env::herdr_config(&vars(&[
        ("HOLLER_HERDR_SESSION", "holler649-0000000a"),
        ("HOLLER_HERDR_SOCKET", "/tmp/h/herdr.sock"),
        ("HOLLER_HERDR_WORKSPACES", "hlr649=2x1,wide=3x4"),
    ]))
    .unwrap();
    assert_eq!(config.workspaces.get("hlr649"), Some(&Extent { rows: 2, cols: 1 }));
    assert_eq!(config.workspaces.get("wide"), Some(&Extent { rows: 3, cols: 4 }));
    assert_eq!(config.workspaces.len(), 2);
    assert_eq!(config.timeout, Duration::from_secs(10), "the adapter's own bound (D9)");

    let none = env::herdr_config(&vars(&[
        ("HOLLER_HERDR_SESSION", "holler649-0000000a"),
        ("HOLLER_HERDR_SOCKET", "/tmp/h/herdr.sock"),
    ]))
    .unwrap();
    assert!(none.workspaces.is_empty(), "unset is no workspace");

    let empty = env::herdr_config(&vars(&[
        ("HOLLER_HERDR_SESSION", "holler649-0000000a"),
        ("HOLLER_HERDR_SOCKET", "/tmp/h/herdr.sock"),
        ("HOLLER_HERDR_WORKSPACES", ""),
    ]))
    .unwrap();
    assert!(empty.workspaces.is_empty(), "the empty list is no workspace");
}

#[test]
fn malformed_workspaces_are_usage_naming_the_variable() {
    let mut failures = Vec::new();
    // D5 (audit): an item without `=` or `x`, an empty label, a label containing `=` (`,`
    // separates items, so an empty item stands for it), a zero or non-numeric extent, and a
    // repeated label.
    for bad in [
        "hlr649",
        "hlr649=2",
        "hlr649=2x",
        "hlr649=twoxone",
        "=2x1",
        "a=2x1;b=1x1",
        "a=b=2x1",
        "a=2x1,",
        "a=2x1,,b=1x1",
        "hlr649=0x1",
        "hlr649=2x0",
        "hlr649=-1x1",
        "hlr649=2x1,hlr649=1x1",
    ] {
        let answer = env::herdr_config(&vars(&[
            ("HOLLER_HERDR_SESSION", "holler649-0000000a"),
            ("HOLLER_HERDR_SOCKET", "/tmp/h/herdr.sock"),
            ("HOLLER_HERDR_WORKSPACES", bad),
        ]));
        match answer {
            Err(e) if e.code() == "usage" && e.to_string().contains("HOLLER_HERDR_WORKSPACES") => {}
            other => failures.push(format!("{bad:?}: want `usage` naming the variable; got {other:?}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// --- tmux (D6) ---------------------------------------------------------------------------

#[test]
fn the_tmux_socket_is_an_absolute_path_or_the_default() {
    assert_eq!(env::tmux_socket(&vars(&[])).unwrap(), TmuxSocket::Default);
    assert_eq!(
        env::tmux_socket(&vars(&[("HOLLER_TMUX_SOCKET", "/tmp/hlr649/tmux.sock")])).unwrap(),
        TmuxSocket::Path(PathBuf::from("/tmp/hlr649/tmux.sock"))
    );
}

/// W-13: a relative socket never falls back to the default server (the operator's live one).
#[test]
fn a_relative_tmux_socket_is_usage_never_the_default_server() {
    for bad in ["tmux.sock", "./tmux.sock", "hlr649/tmux.sock"] {
        usage_naming(
            env::tmux_socket(&vars(&[("HOLLER_TMUX_SOCKET", bad)])),
            "HOLLER_TMUX_SOCKET",
        );
    }
}

// --- OpenCode (D7) -----------------------------------------------------------------------

#[test]
fn the_opencode_binary_is_hollers_variable_else_opencode_on_path_made_absolute() {
    assert_eq!(
        env::opencode_bin(&vars(&[("HOLLER_OPENCODE_BIN", "/opt/oc/opencode")])).unwrap(),
        PathBuf::from("/opt/oc/opencode")
    );
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("opencode");
    std::fs::write(&bin, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("/nonexistent-hlr649:{}", dir.path().display());
    let found = env::opencode_bin(&vars(&[("PATH", &path)])).unwrap();
    assert!(found.is_absolute(), "{found:?}");
    assert_eq!(found, bin, "the one on PATH");
}

#[test]
fn no_opencode_binary_is_unavailable_naming_the_variable() {
    let empty = tempfile::tempdir().unwrap();
    let path = empty.path().display().to_string();
    let error = env::opencode_bin(&vars(&[("PATH", &path)])).expect_err("none found");
    assert_eq!(error.code(), "unavailable", "{error}");
    assert!(error.to_string().contains("HOLLER_OPENCODE_BIN"), "{error}");
}

#[test]
fn the_serve_args_are_a_json_array_of_strings_default_none() {
    assert_eq!(env::serve_args(&vars(&[])).unwrap(), Vec::<String>::new());
    assert_eq!(
        env::serve_args(&vars(&[("HOLLER_OPENCODE_SERVE_ARGS", r#"["--pure","--print-logs"]"#)]))
            .unwrap(),
        vec!["--pure".to_owned(), "--print-logs".to_owned()]
    );
    // "" is also how `connect` reads a value that is not UTF-8: refused, never the default.
    for bad in ["", "--pure", "[1]", r#"{"a":"b"}"#, r#"["--pure","#] {
        usage_naming(
            env::serve_args(&vars(&[("HOLLER_OPENCODE_SERVE_ARGS", bad)])),
            "HOLLER_OPENCODE_SERVE_ARGS",
        );
    }
}

// --- first use, never connect (W-13) -------------------------------------------------------

/// Every adapter setting malformed at once: the store-only verbs still work, because a bad
/// value fails at the first use of its port, never at `connect`.
#[test]
fn malformed_adapter_settings_do_not_stop_the_store_only_verbs() {
    let bare = Bare::start();
    bare.seed_pane(PANE);
    let out = crate::holler_on(
        &bare.state,
        &["pane", "list", "--format=json"],
        &[
            ("HOLLER_HERDR_SESSION", "holler649-0000000a"),
            ("HOLLER_HERDR_WORKSPACES", "not-a-workspace"),
            ("HOLLER_TMUX_SOCKET", "relative.sock"),
            ("HOLLER_OPENCODE_SERVE_ARGS", "not json"),
            ("HOLLER_OPENCODE_BIN", "/nonexistent-hlr649/opencode"),
        ],
    );
    assert_eq!(out.code, 0, "{out:?}");
    assert!(out.data().to_string().contains(PANE), "{out:?}");
}
