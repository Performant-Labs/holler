#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
//! `TmuxHost::attached_clients` against the fake `tmux` (#647 part 2, ADR-0021 section 12,
//! brief D15): the number of clients attached to the session, `#{session_attached}`, read
//! from the same exact-target `list-panes -s` listing `ps` uses (with `-s`, tmux prints the
//! session's one count on every pane's line), through a parser as strict as `ps`'s.
//!
//! The fake answers a listing by filling in the `-F` format it was given, so these tests pin
//! what the adapter reads, not how it spells its format. The bound and a missing binary are
//! pinned with the other methods in `fake_tmux_test.rs` (`METHODS`).

mod common;

use common::{answer, flag_value, host, name, subcommand, Fake};
use holler_pane::{HostPort, PaneError};

const C1: &str = "demo-c1r1";

/// A fake `tmux` that answers a listing with one line per pane of `panes`, each the `-F`
/// format it was given with `#{session_attached}` as `attached` and the pane variables `ps`
/// and `foreground_commands` read filled in as a live pane's.
fn listing(attached: &str, panes: usize) -> Fake {
    let mut body = String::from(
        r#"fmt=; prev=
for a in "$@"; do [ "$prev" = -F ] && fmt=$a; prev=$a; done
row() { printf '%s\n' "$fmt" | sed -e "s/#{session_attached}/$1/g" -e "s/#{pane_pid}/$2/g" -e "s/#{pane_dead}/0/g" -e "s/#{pane_current_command}/sh/g"; }
"#,
    );
    for pane in 0..panes {
        body.push_str(&format!("row '{attached}' {}\n", 100 + pane));
    }
    body.push_str("exit 0");
    Fake::new(&body)
}

#[test]
fn attached_clients_reads_the_sessions_count() {
    for (value, count) in [("0", 0), ("1", 1), ("2", 2)] {
        let fake = listing(value, 2);
        assert_eq!(
            host(&fake).attached_clients(&name(C1)),
            Ok(count),
            "#{{session_attached}} = {value}, printed on each of two panes' lines"
        );
    }
}

/// One `list-panes -s` call on the exact target `=NAME:`, as `ps`'s, run as an argv (no
/// shell), whose format asks for the count and for no command line or client (I7).
#[test]
fn the_listing_is_ps_s_exact_target_argv() {
    let fake = listing("1", 1);
    host(&fake).attached_clients(&name(C1)).unwrap();
    let calls = fake.calls();
    assert_eq!(calls.len(), 1, "one tmux call: {calls:?}");
    let call = &calls[0];
    assert_eq!(subcommand(call), "list-panes", "{call:?}");
    assert!(
        call.iter().any(|a| a == "-s"),
        "the whole session: {call:?}"
    );
    assert_eq!(
        flag_value(call, "-t"),
        Some("=demo-c1r1:"),
        "the exact target, one argv element: {call:?}"
    );
    assert!(
        !call.iter().any(|a| a == "-c" || a == "sh"),
        "no shell: {call:?}"
    );
    let format = flag_value(call, "-F").expect("a -F format");
    assert!(format.contains("#{session_attached}"), "{format:?}");
    assert!(
        !format.contains("start_command") && !format.contains("client_"),
        "a count only, never a command line or a client's name (I7): {format:?}"
    );
}

/// The parser is strict: a value that is not an unsigned integer makes the answer
/// `unavailable`, naming the call (as `BAD_PANE_LIST` names `tmux list-panes`) and quoting none
/// of tmux's output.
#[test]
fn a_value_that_is_not_a_count_is_unavailable_and_quotes_nothing() {
    for value in ["", "SECRETVALUE", "-1", "1.5"] {
        let fake = listing(value, 2);
        match host(&fake).attached_clients(&name(C1)) {
            Err(PaneError::Unavailable { what }) => {
                assert!(
                    what.contains("list-panes") || what.contains("attached"),
                    "names the call, as the adapter's other parse failures do: {value:?}: {what:?}"
                );
                assert!(!what.contains("SECRETVALUE"), "{value:?}: {what:?}");
            }
            other => panic!("{value:?}: expected unavailable, got {other:?}"),
        }
    }
}

#[test]
fn attached_clients_of_a_missing_session_is_pane_not_found() {
    for err in [
        "can't find session: demo-c1r1",
        "can't find window: demo-c1r1",
        "no server running on /s",
    ] {
        let fake = Fake::new(&answer("", &format!("{err}\n"), 1));
        let got = host(&fake).attached_clients(&name(C1));
        assert!(
            matches!(got, Err(PaneError::PaneNotFound { .. })),
            "{err:?}: {got:?}"
        );
    }
}

#[test]
fn another_tmux_failure_is_unavailable_with_its_first_line() {
    let fake = Fake::new(&answer(
        "",
        "server exploded: protocol mismatch\nsecond line\n",
        1,
    ));
    match host(&fake).attached_clients(&name(C1)) {
        Err(PaneError::Unavailable { what }) => {
            assert!(
                what.contains("server exploded: protocol mismatch"),
                "{what:?}"
            );
            assert!(!what.contains("second line"), "{what:?}");
        }
        other => panic!("expected unavailable, got {other:?}"),
    }
}
