//! #775: Stage 8's text (pane labels, restored saved sessions, the 0.8.x status format, the
//! row-first build) and the `WIZARD_HERDR_HOST=1` collide line (epic #769).
//!
//! Plain text assertions over `agent-skills/setup-wizard/SKILL.md` and `docs/setup-wizard.md`.
//! Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #775

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

fn stage8() -> String {
    section(&skill(), "## Stage 8 ")
}

fn stage3() -> String {
    section(&skill(), "## Stage 3 ")
}

#[test]
fn stage8_labels_every_pane_with_its_slot_name() {
    let s = flat(&stage8());
    assert!(
        s.contains("herdr.sh label <pane-id> <label>"),
        "the label call is missing"
    );
    assert!(s.contains("right after the splits"), "when to label");
    assert!(s.contains("slot name"), "what the label is");
    assert!(
        s.contains("every pane's label equals its slot"),
        "the Verify"
    );
}

#[test]
fn stage8_reads_the_status_line_of_herdr_0_8() {
    let s = flat(&stage8());
    assert!(s.contains("status: not running"));
    assert!(s.contains("status: running"));
    assert!(s.contains("under `server:`"));
    assert!(
        s.contains("`server: not running`"),
        "old text stays accepted"
    );
    assert!(s.contains("stays accepted"));
}

#[test]
fn stage8_handles_a_restored_saved_session() {
    let s = flat(&stage8());
    assert!(s.contains("pane list"));
    assert!(s.contains("the wizard did not create"));
    assert!(s.contains("never split or close them"));
    assert!(
        s.contains("workspace create"),
        "the new workspace beside them"
    );
    assert!(s.contains("with the user's yes"));
    assert!(s.contains("only if the user asks"), "reuse is on request");
}

#[test]
fn stage3_plan_names_a_stopped_saved_session() {
    let s = flat(&stage3());
    assert!(
        s.contains("shows a stopped session of the instance's name"),
        "the plan sentence"
    );
    assert!(s.contains("restores"), "says what starting the server does");
    assert!(s.contains("workspace create"));
}

#[test]
fn stage8_builds_the_grid_row_first() {
    let s = flat(&stage8()).to_lowercase();
    assert!(s.contains("first row"), "first row is built first");
    assert!(
        s.contains("splitting right from the last pane of the row"),
        "row order"
    );
    assert!(
        s.contains("then split each first-row pane down"),
        "columns after"
    );
    assert!(
        s.contains("column-by-column order nests unevenly"),
        "the reason"
    );
    assert!(
        !s.contains("split **right** from the previous column's first-row pane"),
        "the old column-first rule is gone"
    );
}

#[test]
fn only_the_local_collide_line_sets_the_herdr_host_flag() {
    let text = skill();
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("bash $WIZARD_LIB/collide.sh"))
        .collect();
    assert!(!lines.is_empty());
    let (local, remote): (Vec<&str>, Vec<&str>) = lines
        .iter()
        .partition(|l| l.contains("WIZARD_HERDR_HOST=1"));
    assert_eq!(local.len(), 1, "the local line has the flag");
    assert!(!remote.is_empty(), "a remote line without it exists");
    for l in &remote {
        assert!(!l.contains("WIZARD_HERDR_HOST"), "remote collide call: {l}");
    }
    assert!(
        flat(&text).contains("local call only"),
        "remote calls never set it"
    );
    // The flag is never put on an inventory or remote line.
    for l in text.lines().filter(|l| l.contains("inventory.sh")) {
        assert!(!l.contains("WIZARD_HERDR_HOST"), "inventory line: {l}");
    }
}

#[test]
fn the_docs_herdr_section_matches() {
    let d = flat(&read("docs/setup-wizard.md"));
    assert!(d.contains("workspace create"));
    assert!(d.contains("WIZARD_HERDR_HOST=1"));
    assert!(d.contains("label"));
}
