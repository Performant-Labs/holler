//! #809: Stage 0 closes with "All systems go. Proceed?", epic #769.
//!
//! Plain text assertions over the skill, the docs and the README. Nothing here runs a script.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #809

use std::path::PathBuf;

const NEW: &str = "All systems go. Proceed?";
const OLD: &str = "configure Herdr now and launch a workspace";

/// A file with every run of whitespace folded to one space, for phrases that wrap.
fn flat(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join(rel)).unwrap();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn stage0() -> String {
    let text = flat("agent-skills/herdr-setup/SKILL.md");
    let from = text.find("## Stage 0").expect("no Stage 0");
    let rest = &text[from..];
    let to = rest.find("## Stage 1 ").expect("no Stage 1");
    rest[..to].to_owned()
}

#[test]
fn stage0_asks_the_new_question() {
    assert!(stage0().contains(&format!("**\"{NEW}\"** (yes/no)")));
}

#[test]
fn old_wording_is_gone_everywhere() {
    for rel in [
        "agent-skills/herdr-setup/SKILL.md",
        "docs/setup-wizard.md",
        "README.md",
    ] {
        let text = flat(rel).to_lowercase();
        assert!(!text.contains(&OLD.to_lowercase()), "{rel}");
        assert!(
            !text.contains("configure holler and launch a workspace now"),
            "{rel}"
        );
    }
}

#[test]
fn stage0_keeps_its_other_rules() {
    let s = stage0();
    assert!(s.contains("ask a second, separate, explicit question, never assumed"));
    assert!(s.contains("ends the wizard cleanly right here"));
    assert!(s.contains("Only a clear yes moves on to Stage 1"));
    assert!(s.contains("only once `herdr` is confirmed present"));
}

#[test]
fn frontmatter_docs_and_readme_use_the_new_question() {
    let skill = flat("agent-skills/herdr-setup/SKILL.md");
    let front = &skill[..skill.find("# Herdr: M local").unwrap()];
    assert!(front.contains(NEW));
    assert!(flat("docs/setup-wizard.md").contains(&format!("\"{NEW}\"")));
    assert!(flat("README.md").contains(&format!("\"{NEW}\"")));
}
