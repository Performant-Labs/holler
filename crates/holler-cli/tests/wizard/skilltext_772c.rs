//! #776: Stage 1's default `r<row>c<col>` pane names and first-run notes (epic #769).
//!
//! Plain text assertions over `agent-skills/herdr-setup/SKILL.md`, `docs/setup-wizard.md` and
//! `README.md`. Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #776

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

/// Whitespace folded to one space and lower-cased, for phrases that wrap.
fn flat(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).unwrap_or_else(|| panic!("no `{start}`"));
    let rest = &text[from + start.len()..];
    rest[..rest.find(end).unwrap_or(rest.len())].to_owned()
}

fn stage1() -> String {
    flat(&between(
        &read("agent-skills/herdr-setup/SKILL.md"),
        "## Stage 1 \u{2014}",
        "## Stage 2 \u{2014}",
    ))
}

fn skill_config_section() -> String {
    flat(&between(
        &read("agent-skills/herdr-setup/SKILL.md"),
        "## The config file",
        "\n## ",
    ))
}

fn docs_config_section() -> String {
    flat(&between(
        &read("docs/setup-wizard.md"),
        "## The config file",
        "\n## ",
    ))
}

fn readme_wizard_section() -> String {
    flat(&between(
        &read("README.md"),
        "## Set up a Herdr workspace with an agent",
        "\n## ",
    ))
}

fn has_all(label: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(text.contains(needle), "{label}: missing `{needle}`");
    }
}

#[test]
fn stage1_names_default_panes_by_row_then_column() {
    has_all(
        "Stage 1",
        &stage1(),
        &[
            "row then column",
            "`r1c1`, `r1c2`, `r1c3`, `r1c4`",
            "`r2c1`",
            "the user's own names always win",
        ],
    );
}

#[test]
fn stage1_proposes_the_squarest_grid_for_a_bare_pane_count() {
    has_all(
        "Stage 1",
        &stage1(),
        &[
            "squarest grid",
            "two rows for more than four panes",
            "one confirm",
        ],
    );
}

#[test]
fn stage1_states_the_first_run_notes() {
    has_all(
        "Stage 1",
        &stage1(),
        &[
            "a first run has no skill directory",
            "ask, then install",
            "an existing global `sessions.toml` skips the interview",
            "moves it aside",
            "never deletes it",
            "\"no config found\" message names the config path",
        ],
    );
}

#[test]
fn the_config_file_sections_and_readme_carry_the_naming_rule() {
    for (label, text) in [
        ("SKILL.md config section", skill_config_section()),
        ("docs config section", docs_config_section()),
        ("README wizard section", readme_wizard_section()),
    ] {
        has_all(label, &text, &["`r1c1`", "`r2c1`", "row then column"]);
    }
}

#[test]
fn the_docs_explain_how_to_start_clean() {
    has_all(
        "docs config section",
        &docs_config_section(),
        &["skips the interview", "move it aside", "never deletes it"],
    );
}
