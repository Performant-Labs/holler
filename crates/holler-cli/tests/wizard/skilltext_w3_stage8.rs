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

/// #803: the panes are built in Stage 7.
fn stage7() -> String {
    flat(&section(
        &read("agent-skills/herdr-setup/SKILL.md"),
        "## Stage 7 ",
    ))
}

fn stage3() -> String {
    flat(&section(
        &read("agent-skills/herdr-setup/SKILL.md"),
        "## Stage 3 ",
    ))
}

#[test]
fn stage7_build_in_it_is_a_new_workspace_holler_creates() {
    let s = stage7();
    assert!(s.contains("a new workspace in that server"), "{s}");
    assert!(
        !s.contains("workspace create"),
        "the adapter creates it (#803 F4)"
    );
}

#[test]
fn stage3_build_in_it_means_a_new_workspace() {
    let s = stage3();
    assert!(s.contains("build in it"));
    assert!(s.contains("a new workspace in that server"));
    assert!(
        !s.contains("workspace create"),
        "#803: no direct Herdr control"
    );
    assert!(!s.contains("then Stage 8 reuses it"));
}

#[test]
fn docs_never_reuse_restored_panes() {
    let d = flat(&read("docs/setup-wizard.md"));
    assert!(
        !d.contains("if the user asked to reuse them"),
        "the reuse branch is retired (#803)"
    );
    assert!(d.contains("a new workspace in that server"));
}
