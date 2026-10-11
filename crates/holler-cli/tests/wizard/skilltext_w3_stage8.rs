//! #788: Stage 8 text. Restored panes are reused only on the user's request, and "build in it"
//! for an unnamed Herdr server the wizard did not start means a new workspace (epic #769).
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`.
//! Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #788

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
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
    flat(&section(&read("agent-skills/herdr-setup/SKILL.md"), "## Stage 8 "))
}

fn stage3() -> String {
    flat(&section(&read("agent-skills/herdr-setup/SKILL.md"), "## Stage 3 "))
}

#[test]
fn stage8_restored_panes_are_a_reuse_branch_on_request() {
    let s = stage8();
    assert!(
        s.contains("if the user asked to reuse them and the count equals M+N"),
        "the reuse branch"
    );
    assert!(
        !s.contains("if the pane count already equals M+N, reuse the restored structure"),
        "the unconditional reuse sentence is gone"
    );
    assert!(
        !s.contains("reusing that structure is fine"),
        "the second unconditional reuse sentence is gone"
    );
}

#[test]
fn stage8_build_in_it_is_a_new_workspace_with_a_way_to_find_its_pane() {
    let s = stage8();
    assert!(s.contains("`herdr.sh run workspace create`"));
    assert!(s.contains("a new workspace in that server"));
    assert!(s.contains("filtered by the new workspace id"), "first pane id");
    assert!(s.contains("`<base-pane>`"));
    assert!(s.contains("never split a pane the wizard did not create"));
}

#[test]
fn stage3_build_in_it_means_a_new_workspace() {
    let s = stage3();
    assert!(s.contains("build in it"));
    assert!(s.contains("a new workspace in that server"));
    assert!(s.contains("`herdr.sh run workspace create`"));
    assert!(!s.contains("then Stage 8 reuses it"));
}

#[test]
fn stage8_says_to_check_column_widths_after_the_build() {
    let s = stage8();
    assert!(s.contains("1/2, 1/4, 1/8, 1/8"));
    assert!(s.contains("tell the user if the widths look uneven"));
    assert!(s.contains("#734") && s.contains("#778"), "who verifies it");
}

#[test]
fn docs_reuse_restored_panes_only_on_request() {
    let d = flat(&read("docs/setup-wizard.md"));
    assert!(d.contains("if the user asked to reuse them and the count equals orchestrators + sessions"));
    assert!(!d.contains("if the pane count already equals orchestrators + sessions, reuse the structure"));
    assert!(d.contains("a new workspace in that server"));
}
