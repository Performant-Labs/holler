//! #819: Stage 0 is the Preflight and ends with "All systems go. Proceed?"; the stage checklist is a
//! vertical table printed after the status table; Stage 2 is "Config checks", epic #769.
//!
//! Plain text assertions over the skill, the docs and the README. Nothing here runs a script.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #819

use std::path::PathBuf;

const SKILL: &str = "agent-skills/herdr-setup/SKILL.md";
const NEW: &str = "All systems go. Proceed?";

/// A file with every run of whitespace folded to one space, for phrases that wrap.
fn flat(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join(rel)).unwrap();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).unwrap_or_else(|| panic!("no `{start}`"));
    let rest = &text[from..];
    let to = rest[start.len()..]
        .find(end)
        .map_or(rest.len(), |i| i + start.len());
    rest[..to].to_owned()
}

fn stage0() -> String {
    between(&flat(SKILL), "## Stage 0 \u{2014} Preflight", "## Stage 1 ")
}

fn stage2() -> String {
    between(
        &flat(SKILL),
        "## Stage 2 \u{2014} Config checks",
        "## Stage 3 ",
    )
}

fn has_all(what: &str, text: &str, needles: &[&str]) {
    for n in needles {
        assert!(text.contains(n), "{what} lacks `{n}`");
    }
}

#[test]
fn the_headings_are_preflight_and_config_checks() {
    let text = flat(SKILL);
    assert!(text.contains("## Stage 0 \u{2014} Preflight"));
    assert!(text.contains("## Stage 2 \u{2014} Config checks"));
    assert!(!text.contains("Stage 0 \u{2014} Install Herdr"));
    assert!(!text.contains("Stage 2 \u{2014} Preflight"));
}

#[test]
fn stage0_ends_with_the_question_and_holds_the_install_question_before_it() {
    let s = stage0();
    let install = s
        .find("Herdr isn't installed on this machine \u{2014} do you want to install it?")
        .expect("the install question is inside Stage 0");
    let proceed = s.find(&format!("**\"{NEW}\"** (yes/no)")).unwrap();
    assert!(install < proceed, "install question comes first");
    assert!(
        s.trim_end().ends_with("moves on to Stage 1."),
        "the question closes the stage: {}",
        &s[s.len().saturating_sub(120)..]
    );
    has_all(
        "Stage 0",
        &s,
        &[
            "re-checks",
            "ends the wizard cleanly right here",
            "A no to installing",
        ],
    );
}

#[test]
fn stage0_needs_no_config() {
    let s = stage0();
    has_all(
        "Stage 0",
        &s,
        &["machine-level", "read-only", "needs no config"],
    );
}

#[test]
fn stage0_prints_one_vertical_status_table_with_every_row() {
    let s = stage0();
    assert!(s.contains("| Check | Result | Detail |"));
    has_all(
        "Stage 0 status table",
        &s,
        &[
            "| `herdr` installed",
            "| `holler` is the current single binary",
            "| The skill's `lib/` scripts present",
            "| Driving agent not inside a Herdr pane",
            "| Herdr server state",
            "| A Holler hub running here",
            "| An existing wizard ledger",
            "| An existing config at the three tiers",
            "ok / missing / legacy",
            "ok / absent",
            "ok / stop",
        ],
    );
}

#[test]
fn stage0_runs_the_machine_level_commands() {
    let s = stage0();
    has_all(
        "Stage 0",
        &s,
        &[
            "command -v herdr",
            "holler --help",
            "holler hub status",
            "herdr.sh check-pane",
            "herdr.sh run status",
            "herdr session list",
            "wizard-ledger.toml",
            "WIZARD_LIB=",
        ],
    );
}

#[test]
fn the_checklist_is_a_table_printed_directly_after_the_status_table() {
    let s = stage0();
    let status = s.find("| Check | Result | Detail |").unwrap();
    let after = s.find("Directly after that table").unwrap();
    let checklist = s[after..].find("| Stage | Name | Status |").unwrap() + after;
    assert!(status < after && after < checklist, "order in Stage 0");
    let text = flat(SKILL);
    let how = between(&text, "Keep a running status checklist", "## Stage 0");
    assert!(how.contains("| Stage | Name | Status |"), "{how}");
    assert!(how.contains("vertical table"));
    for stage in [
        "| 0 | Preflight |",
        "| 1 | Load the session config |",
        "| 2 | Config checks |",
        "| 10 | Verify end-to-end |",
        "| 11 | Basic Herdr commands |",
    ] {
        assert!(how.contains(stage), "checklist lacks `{stage}`");
    }
}

#[test]
fn the_inline_checklist_and_its_sentences_are_gone() {
    let text = flat(SKILL);
    assert!(!text.contains("\u{2705} 0. Install Herdr"));
    assert!(!text.contains("\u{2705} 2. Preflight"));
    assert!(!text.contains("Print the whole checklist"));
    assert!(text.contains("one line under the first table"));
}

#[test]
fn stage2_holds_no_machine_level_check() {
    let s = stage2();
    for gone in [
        "herdr.sh run status",
        "holler hub status",
        "command -v ssh",
        "command -v tailscale",
        "tailscale status --self",
    ] {
        assert!(!s.contains(gone), "Stage 2 still holds `{gone}`");
    }
    has_all("Stage 2", &s, &["Stage 0 already", "check-session"]);
}

#[test]
fn no_sentence_calls_stage2_the_preflight_or_stage0_the_install() {
    for rel in [SKILL, "docs/setup-wizard.md", "README.md"] {
        let text = flat(rel);
        for bad in [
            "Stage 2 \u{2014} Preflight",
            "Stage 2's preflight",
            "Stage 2 preflight",
            "preflight (Stage 2)",
            "Stage 0 \u{2014} Install Herdr",
            "Stage 0 installs",
            "Stage 0 stands apart",
            "its own Stage 0, which only installs",
        ] {
            assert!(!text.contains(bad), "{rel} still says `{bad}`");
        }
    }
}

#[test]
fn the_intro_frontmatter_docs_and_readme_name_the_preflight_first() {
    let text = flat(SKILL);
    let front = &text[..text.find("# Herdr").unwrap()];
    assert!(front.contains("preflight"), "{front}");
    assert!(front.contains(NEW));
    assert!(!front.contains("asked as its own up-front yes/no"));
    assert!(text.contains("Stage 0 is the Preflight"));
    let docs = flat("docs/setup-wizard.md");
    assert!(docs.contains("Stage 0 is the preflight"), "docs");
    assert!(docs.contains("\"All systems go. Proceed?\""));
    let readme = flat("README.md");
    assert!(readme.contains("its own Stage 0, the preflight"), "README");
    assert!(readme.contains("\"All systems go. Proceed?\""));
}
