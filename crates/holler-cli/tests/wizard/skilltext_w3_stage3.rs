//! #787: Stage 3 and Stage 10 text (one collide line per host, no serve port for a local-only
//! run, the scoped hub check, the `default` session entry, the done wording), epic #769.
//!
//! Plain text assertions over `agent-skills/setup-wizard/SKILL.md` and `docs/setup-wizard.md`.
//! Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #787

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

fn skill() -> String {
    read("agent-skills/setup-wizard/SKILL.md")
}

/// Whitespace folded to one space, for phrases that wrap.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text of one `## ` section, from its heading to the next `## ` heading.
fn section(text: &str, heading_start: &str) -> String {
    let from = text.find(heading_start).expect("section heading");
    let rest = &text[from + 3..];
    rest[..rest.find("\n## ").unwrap_or(rest.len())].to_owned()
}

fn collide_lines() -> Vec<String> {
    skill()
        .lines()
        .filter(|l| l.contains("bash $WIZARD_LIB/collide.sh"))
        .map(|l| l.trim().to_owned())
        .collect()
}

#[test]
fn there_are_two_collide_lines_one_local_and_one_remote() {
    let lines = collide_lines();
    assert_eq!(lines.len(), 2, "one local and one per-remote-host line: {lines:?}");
    let (local, remote): (Vec<_>, Vec<_>) = lines
        .iter()
        .partition(|l| l.contains("WIZARD_HERDR_HOST=1"));
    assert_eq!(local.len(), 1, "exactly one local line");
    assert_eq!(remote.len(), 1, "exactly one remote line");

    let local = local[0];
    for var in [
        "WIZARD_HUB_PORT=",
        "WIZARD_SERVE_HTTPS_PORT=",
        "WIZARD_HERDR_SESSION=",
    ] {
        assert!(local.contains(var), "local line lacks {var}");
    }
    assert!(
        local.contains("WIZARD_LEDGER=<state_dir>/herdr/setup/wizard-ledger.toml"),
        "the local ledger is in place: {local}"
    );
    assert!(!local.contains("ledger-<host>"), "no fetched ledger locally");

    let remote = remote[0];
    for var in [
        "WIZARD_HERDR_HOST",
        "WIZARD_HUB_PORT",
        "WIZARD_SERVE_HTTPS_PORT",
        "WIZARD_HERDR_SESSION",
    ] {
        assert!(!remote.contains(var), "remote line carries {var}: {remote}");
    }
    assert!(
        remote.contains("WIZARD_LEDGER=<scratch>/ledger-<host>.toml"),
        "the remote line uses the fetched ledger: {remote}"
    );
}

#[test]
fn a_local_only_run_leaves_the_serve_port_empty_on_the_local_call() {
    let s = flat(&section(&skill(), "## Stage 3 "));
    assert!(
        s.contains("leave `WIZARD_SERVE_HTTPS_PORT` empty on the local call"),
        "the N=0 sentence"
    );
}

#[test]
fn stage10_checks_the_hub_with_the_scoped_command() {
    let s = flat(&section(&skill(), "## Stage 10 "));
    assert!(s.contains("`HOLLER_STATE_DIR=<state_dir> holler hub status`"));
    assert!(s.contains("`listening: 127.0.0.1:<hub_port>`"));
    assert!(s.contains("herdr.sh run pane read <orch-pane>"));
    assert!(!s.contains("(`holler hub status`)"), "the bare command is gone");
}

#[test]
fn the_default_instance_looks_for_the_entry_named_default() {
    let s = flat(&section(&skill(), "## Stage 3 "));
    assert!(
        s.contains("for the default instance (no `herdr_session`) the entry named `default`"),
        "which entry of herdr session list"
    );
}

#[test]
fn done_means_every_stage_green_or_skipped() {
    let s = flat(&skill());
    assert!(s.contains("**Every stage green or skipped = done.**"));
    assert!(!s.contains("All 10 stages green = done"));
}

#[test]
fn the_docs_say_the_remote_call_has_no_hub_only_variables() {
    let d = flat(&read("docs/setup-wizard.md"));
    assert!(d.contains("local `collide.sh` call only, never on a remote call"), "docs name both calls");
}
