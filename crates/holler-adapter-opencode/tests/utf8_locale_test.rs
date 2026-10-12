#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #844
//! The adapter's tmux query survives a process with no UTF-8 locale (#844, AC 2). Real tmux,
//! a private server, no OpenCode.
//!
//! Without `-u`, a tmux client under `LANG=C` or `LC_ALL=C` prints each TAB of a
//! `display-message -p` reply as `_`, so `parse_query` finds one field, not five, and every
//! `attach_tui`, `select_session` and `shown_session` reports "no tmux session" for a pane
//! that is fine. This test builds the call exactly as the adapter does (`tmux_command` over
//! `query_args`), runs it in each such locale, and reads the reply with `parse_query`.
//!
//! It runs in the default `cargo test`, not behind `#[ignore]`: a regression pin that is
//! ignored never fails CI. It skips, saying so, when no `tmux` is on `PATH`. The locale is
//! set on the child `Command` only, never with `set_var`, so no other test sees it.
//!
//! The two helpers below (a `PATH` lookup, a private-socket runner) are small copies of
//! `real_opencode/rig.rs`'s on purpose: that file cannot be included here, because the
//! workspace's `dead_code = "deny"` rejects its unused items in a second test target and
//! `rig::binaries()` panics without opencode.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use holler_adapter_opencode::tui::{parse_query, query_args, tmux_command, Query};
use holler_adapter_opencode::{TmuxConfig, TmuxSocket};
use holler_pane::PaneName;

/// The title a TUI shows for a session.
const TITLE: &str = "OC | ses_x";

/// `name` on `PATH`, as an absolute path.
fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Run tmux on the private server only: `TMUX` and `TMUX_PANE` removed, stdin null.
fn tmux(bin: &Path, sock: &Path, args: &[&str]) -> Output {
    Command::new(bin)
        .arg("-S")
        .arg(sock)
        .args(args)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("cannot run {}: {e}", bin.display()))
}

/// Kills the private tmux server, then removes its scratch dir. Built before the server
/// starts, so a failed assertion or a setup that stops part-way leaks nothing.
struct Guard {
    tmux: PathBuf,
    sock: PathBuf,
    _dir: tempfile::TempDir,
}

impl Drop for Guard {
    fn drop(&mut self) {
        // `no server running` or a missing socket: already done.
        let _ = tmux(&self.tmux, &self.sock, &["kill-server"]);
    }
}

/// The adapter's query of `session`, run as a process with exactly this locale (`LANG` and
/// `LC_ALL`, the other removed, and `LC_CTYPE` removed: tmux reads `LC_ALL`, then
/// `LC_CTYPE`, then `LANG`, so an ambient UTF-8 value would hide the failure).
fn query_in_locale(
    config: &TmuxConfig,
    session: &PaneName,
    lang: Option<&str>,
    lc_all: Option<&str>,
) -> Query {
    let mut command = tmux_command(config, &query_args(session));
    command.env_remove("LC_CTYPE");
    match lang {
        Some(value) => command.env("LANG", value),
        None => command.env_remove("LANG"),
    };
    match lc_all {
        Some(value) => command.env("LC_ALL", value),
        None => command.env_remove("LC_ALL"),
    };
    let output = command
        .stdin(Stdio::null())
        .output()
        .expect("the adapter's tmux call runs");
    assert!(
        output.status.success(),
        "tmux exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_query(session, &String::from_utf8(output.stdout).unwrap())
}

#[test]
fn the_query_reads_a_live_pane_in_a_process_with_no_utf8_locale() {
    let Some(tmux_bin) = on_path("tmux") else {
        eprintln!("utf8_locale_test: skipped: no tmux on PATH");
        return;
    };
    let dir = tempfile::Builder::new()
        .prefix("hlr844-")
        .tempdir_in("/tmp")
        .unwrap();
    let sock = dir.path().join("tmux.sock");
    let guard = Guard {
        tmux: tmux_bin.clone(),
        sock: sock.clone(),
        _dir: dir,
    };

    let session = PaneName::parse("utf8-c1r1").unwrap();
    let target = format!("={}:", session.as_str());
    let started = tmux(
        &tmux_bin,
        &sock,
        &[
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            session.as_str(),
            "sleep",
            "300",
        ],
    );
    assert!(
        started.status.success(),
        "new-session: {}",
        String::from_utf8_lossy(&started.stderr)
    );
    let titled = tmux(
        &tmux_bin,
        &sock,
        &["select-pane", "-t", &target, "-T", TITLE],
    );
    assert!(
        titled.status.success(),
        "select-pane: {}",
        String::from_utf8_lossy(&titled.stderr)
    );

    let config = TmuxConfig {
        tmux_bin: tmux_bin.clone(),
        socket: TmuxSocket::Path(sock.clone()),
    };
    for (what, lang, lc_all) in [
        ("LANG=C, LC_ALL unset", Some("C"), None),
        ("LC_ALL=C, LANG unset", None, Some("C")),
        ("no locale variable at all", None, None),
    ] {
        match query_in_locale(&config, &session, lang, lc_all) {
            Query::Live { title, .. } => assert_eq!(title, TITLE, "{what}"),
            other => panic!("{what}: the pane is live, but the query read {other:?}"),
        }
    }
    drop(guard);
}
