//! #810: a named workspace (`workspace` config key), epic #769.
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`.
//! Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #810

use std::path::PathBuf;

/// A file with every run of whitespace folded to one space, for phrases that wrap.
fn flat(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel))
        .unwrap()
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

fn skill() -> String {
    flat("agent-skills/herdr-setup/SKILL.md")
}

fn has_all(what: &str, text: &str, needles: &[&str]) {
    for n in needles {
        assert!(text.contains(n), "{what} lacks `{n}`");
    }
}

#[test]
fn stage1_asks_for_the_name_and_suggests_two_words() {
    let s1 = between(&skill(), "## Stage 1 ", "## Stage 2 ");
    has_all(
        "Stage 1",
        &s1,
        &[
            "the workspace name",
            "two dictionary words related to the task",
            "joined with a hyphen",
            "`herald-relay`",
            "three alternatives on request",
            "workspace = \"<name>\"",
            "never a project's real name, never a person's name",
        ],
    );
}

#[test]
fn stage1_validator_text_and_config_example_know_the_key() {
    let s = skill();
    let cfg = between(&s, "## The config file", "## How to run this");
    has_all(
        "config section",
        &cfg,
        &["workspace = \"herald-relay\"", "1 to 40 characters"],
    );
}

#[test]
fn the_derived_body_config_drops_workspace() {
    let cfg = between(&skill(), "## The config file", "## How to run this");
    has_all(
        "derived body config paragraph",
        &cfg,
        &["**drops** the top-level `workspace` key"],
    );
}

#[test]
fn stage3_plan_names_the_workspace() {
    let s3 = between(&skill(), "## Stage 3 ", "## Stage 4 ");
    has_all(
        "Stage 3",
        &s3,
        &["**The workspace's name.**", "the plan names it"],
    );
}

#[test]
fn the_docs_list_workspace_among_the_keys_the_body_config_drops() {
    let d = flat("docs/setup-wizard.md");
    has_all(
        "docs",
        &d,
        &["the top-level `workspace` key", "`workspace = \"<name>\"`"],
    );
}
