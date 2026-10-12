//! #787: Stage 3 and Stage 10 text (one collide line per host, no serve port for a local-only
//! run, the scoped hub check, the `default` session entry, the done wording), epic #769.
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`.
//! Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #787

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

fn skill() -> String {
    read("agent-skills/herdr-setup/SKILL.md")
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

#[test]
fn the_collide_call_leaves_the_serve_port_empty() {
    // #803: there is no `tailscale serve`, so the one collide call has no serve port (D7).
    let s = flat(&section(&skill(), "## Stage 3 "));
    assert!(s.contains("`WIZARD_SERVE_HTTPS_PORT` empty"), "{s}");
}

#[test]
fn stage10_checks_the_hub_with_the_scoped_command() {
    let s = flat(&section(&skill(), "## Stage 10 "));
    assert!(s.contains("`HOLLER_STATE_DIR=<state_dir> holler hub status`"));
    assert!(s.contains("`listening: 127.0.0.1:<hub_port>`"));
    assert!(
        !s.contains("(`holler hub status`)"),
        "the bare command is gone"
    );
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
