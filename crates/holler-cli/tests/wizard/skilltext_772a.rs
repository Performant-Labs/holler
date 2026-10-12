//! #772: the skill text for a local-only run (zero `[[session]]` tables), epic #769. Since #803
//! every run is local and the session-less tests are retired; the checklist legend and Stage 10 stay.
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md`, `docs/setup-wizard.md` and
//! `README.md`. Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #772

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

/// The skill with every run of whitespace folded to one space, for phrases that wrap.
fn skill() -> String {
    read("agent-skills/herdr-setup/SKILL.md")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text from `start` up to (not including) the next `end`, or to the end.
fn between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).unwrap_or_else(|| panic!("no `{start}`"));
    let rest = &text[from..];
    let to = rest[start.len()..]
        .find(end)
        .map_or(rest.len(), |i| i + start.len());
    rest[..to].to_owned()
}

fn stage(n: u32) -> String {
    let next = if n == 10 {
        "## If a stage fails".to_owned()
    } else {
        format!("## Stage {} ", n + 1)
    };
    between(&skill(), &format!("## Stage {n} "), &next)
}

#[test]
fn the_checklist_shows_skipped_stages_with_a_distinct_symbol() {
    let text = skill();
    let list = between(&text, "Keep a running status checklist", "## Stage 0");
    assert!(
        list.contains("⏭️ skipped"),
        "the legend defines the skipped symbol"
    );
    for sym in ["✅", "🔄", "⬜", "❌"] {
        assert!(!"⏭️".contains(sym), "the skipped symbol differs from {sym}");
    }
}

#[test]
fn stage_ten_verifies_the_hub_and_the_orchestrator_panes() {
    // #803: there is no session-less run any more; every pane is a registry pane.
    let s = stage(10);
    assert!(s.contains("the hub answers"), "Stage 10 verifies the hub");
    assert!(s.contains("every orchestrator pane is alive"));
}
