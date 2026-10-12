#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #835
//! `TmuxHost::foreground_commands` against the fake `tmux` (#835, brief section 1): the
//! command name of the foreground process of every live pane of the session, read from the
//! same exact-target `list-panes -s` listing `ps` uses, through a strict parser that drops a
//! dead pane as `ps` does. Names only: the listing asks tmux for no command line.
//!
//! The fake answers every listing by filling in the `-F` format it was given, so these tests
//! pin what the adapter reads, not how it spells its format. The bound and a missing binary
//! are pinned with the other methods in `fake_tmux_test.rs` (`METHODS`).

mod common;

use common::{answer, flag_value, host, name, subcommand, Fake};
use holler_pane::{HostPort, PaneError};

const C1: &str = "demo-c1r1";

/// A fake `tmux` that answers a listing with one line per `(pid, dead, command)` of `panes`,
/// each the `-F` format it was given with tmux's three pane variables filled in.
fn listing(panes: &[(&str, &str, &str)]) -> Fake {
    let mut body = String::from(
        r#"fmt=; prev=
for a in "$@"; do [ "$prev" = -F ] && fmt=$a; prev=$a; done
row() { printf '%s\n' "$fmt" | sed -e "s/#{pane_pid}/$1/g" -e "s/#{pane_dead}/$2/g" -e "s/#{pane_current_command}/$3/g"; }
"#,
    );
    for (pid, dead, command) in panes {
        body.push_str(&format!("row {pid} {dead} {command}\n"));
    }
    body.push_str("exit 0");
    Fake::new(&body)
}

#[test]
fn foreground_commands_names_every_live_pane_in_tmux_order() {
    let fake = listing(&[
        ("101", "0", "sh"),
        ("202", "0", "claude"),
        ("303", "1", "opencode"),
        ("404", "0", "sleep"),
    ]);
    assert_eq!(
        host(&fake).foreground_commands(&name(C1)),
        Ok(vec!["sh".to_owned(), "claude".to_owned(), "sleep".to_owned()]),
        "every live pane, in the order tmux lists them; the dead one is dropped"
    );
    let calls = fake.calls();
    assert_eq!(calls.len(), 1, "one tmux call: {calls:?}");
    let call = &calls[0];
    assert_eq!(subcommand(call), "list-panes", "{call:?}");
    assert!(call.iter().any(|a| a == "-s"), "every window of the session: {call:?}");
    assert_eq!(flag_value(call, "-t"), Some("=demo-c1r1:"), "the exact target: {call:?}");
    let format = flag_value(call, "-F").expect("a -F format");
    assert!(format.contains("#{pane_current_command}"), "{format:?}");
    assert!(
        !format.contains("start_command"),
        "names only, never a command line (I7): {format:?}"
    );
}

#[test]
fn foreground_commands_of_a_missing_session_is_pane_not_found() {
    for err in [
        "can't find session: demo-c1r1",
        "can't find window: demo-c1r1",
        "no server running on /s",
    ] {
        let fake = Fake::new(&answer("", &format!("{err}\n"), 1));
        let got = host(&fake).foreground_commands(&name(C1));
        assert!(
            matches!(got, Err(PaneError::PaneNotFound { .. })),
            "{err:?}: {got:?}"
        );
    }
}

#[test]
fn another_tmux_failure_is_unavailable_with_its_first_line() {
    let fake = Fake::new(&answer("", "server exploded: protocol mismatch\nsecond line\n", 1));
    match host(&fake).foreground_commands(&name(C1)) {
        Err(PaneError::Unavailable { what }) => {
            assert!(what.contains("server exploded: protocol mismatch"), "{what:?}");
            assert!(!what.contains("second line"), "{what:?}");
        }
        other => panic!("expected unavailable, got {other:?}"),
    }
}

/// The parser is strict, as `ps`'s is: a pane whose dead flag is neither `0` nor `1` makes
/// the whole answer `unavailable`, never a partial list.
#[test]
fn a_malformed_listing_is_unavailable() {
    let fake = listing(&[("101", "0", "sh"), ("202", "x", "claude")]);
    let got = host(&fake).foreground_commands(&name(C1));
    assert!(matches!(got, Err(PaneError::Unavailable { .. })), "{got:?}");
}
