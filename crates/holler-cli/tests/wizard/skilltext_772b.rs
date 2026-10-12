//! #775 (moved by #803 to Stages 3 and 5): Stage 8's text (pane labels, restored saved sessions, the 0.8.x status format, the
//! row-first build) and the `WIZARD_HERDR_HOST=1` collide line (epic #769).
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`.
//! Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #775

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

/// #803: the Herdr server starts in Stage 5.
fn stage5() -> String {
    section(&skill(), "## Stage 5 ")
}

fn stage3() -> String {
    section(&skill(), "## Stage 3 ")
}

#[test]
fn stage5_reads_the_status_line_of_herdr_0_8() {
    let s = flat(&stage5());
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
fn restored_panes_are_never_reused_and_the_workspace_is_built_beside_them() {
    // #803 (D18 item 5): restored panes have no record; nothing adopts, splits or closes them.
    let s = flat(&stage3());
    assert!(s.contains("restored"), "{s}");
    assert!(s.contains("beside them"), "{s}");
    assert!(s.contains("with the user's yes"), "{s}");
    let all = flat(&skill());
    assert!(
        !all.contains("reuse the restored"),
        "the reuse branch is retired"
    );
}

#[test]
fn stage3_plan_names_a_stopped_saved_session() {
    let s = flat(&stage3());
    assert!(
        s.contains("shows a stopped session of the instance's name"),
        "the plan sentence"
    );
    assert!(s.contains("restores"), "says what starting the server does");
}

#[test]
fn the_one_collide_line_sets_the_herdr_host_flag() {
    // #803: with no remote host there is one collide call, the local one.
    let text = skill();
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("bash $WIZARD_LIB/collide.sh"))
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("WIZARD_HERDR_HOST=1"), "{lines:?}");
    // The flag is never put on an inventory line.
    for l in text.lines().filter(|l| l.contains("inventory.sh")) {
        assert!(!l.contains("WIZARD_HERDR_HOST"), "inventory line: {l}");
    }
}

#[test]
fn the_docs_herdr_section_matches() {
    let d = flat(&read("docs/setup-wizard.md"));
    assert!(d.contains("WIZARD_HERDR_HOST=1"));
    assert!(d.contains("holler pane"), "panes are Holler's (#803)");
}
