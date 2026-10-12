//! #812: the skill command is `/herdr-setup`, not `/setup-wizard` (epic #769). The directory is
//! `agent-skills/herdr-setup`, the frontmatter name matches, and the install command's member
//! path and strip count agree with the directory. `docs/setup-wizard.md`, the `WIZARD_*`
//! variables and the ledger file keep their names.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #812

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap()
}

fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The old install location, written so this file does not trip its own scan.
fn old_skill_path() -> String {
    format!("skills/{}-{}", "setup", "wizard")
}

#[test]
fn directory_and_frontmatter_are_renamed() {
    assert!(root().join("agent-skills/herdr-setup/SKILL.md").is_file());
    assert!(root().join("agent-skills/herdr-setup/lib/ledger.sh").is_file());
    assert!(!root().join(format!("agent-skills/{}-{}", "setup", "wizard")).exists());
    let skill = read("agent-skills/herdr-setup/SKILL.md");
    assert_eq!(skill.lines().nth(1), Some("name: herdr-setup"));
}

/// Every tracked path, relative to `root`, as `git ls-files -z` lists it.
fn tracked_files(root: &Path) -> Vec<String> {
    let listing = Command::new("git").args(["ls-files", "-z"]).current_dir(root).output().unwrap();
    assert!(listing.status.success());
    String::from_utf8(listing.stdout)
        .unwrap()
        .split('\0')
        .filter(|rel| !rel.is_empty())
        .map(str::to_owned)
        .collect()
}

/// #848: pipeline handoffs (briefs, journals) under `docs/handoffs/` quote history verbatim and
/// are removed before a PR is pushed. Mirrors `docs_cli_test::is_cli_doc` (#462): `starts_with`
/// compares whole components, so `docs/research/handoffs-notes.md` is still scanned.
fn is_pipeline_scratch(rel: &str) -> bool {
    Path::new(rel).starts_with("docs/handoffs")
}

/// Lines of the files `rels` (relative to `root`) that contain `needle`, as `path:line: text`.
/// Skips this file itself and pipeline handoffs.
fn scan_files(root: &Path, rels: &[String], needle: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for rel in rels {
        if rel.ends_with("skilltext_w5_rename.rs") || is_pipeline_scratch(rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
            continue; // deleted in the working tree, or not UTF-8
        };
        for (i, line) in text.lines().enumerate() {
            if line.contains(needle) {
                hits.push(format!("{rel}:{}: {line}", i + 1));
            }
        }
    }
    hits
}

/// Tracked files that may still name the old install path, and only on a line that is the
/// rename note (it contains `~/.claude/skills/<old name>`).
fn scan_hits(needle: &str) -> Vec<String> {
    scan_files(&root(), &tracked_files(&root()), needle)
}

#[test]
fn no_old_command_or_directory_outside_the_rename_note() {
    let old = format!("{}-{}", "setup", "wizard");
    let note_marker = format!("~/.claude/{}", old_skill_path());
    for needle in [format!("agent-skills/{old}"), format!("/{old}")] {
        let bad: Vec<String> = scan_hits(&needle)
            .into_iter()
            .filter(|h| {
                // `docs/<old>.md` is the doc and keeps its name.
                let stripped = h.replace(&format!("docs/{old}"), "").replace(&format!("../{old}"), "");
                stripped.contains(&needle) && !h.contains(&note_marker)
            })
            .filter(|h| !h.contains(&format!(".config/{old}")))
            .collect();
        assert!(bad.is_empty(), "old name `{needle}` remains:\n{}", bad.join("\n"));
    }
}

/// #848: pipeline handoffs (briefs, journals) quote history verbatim, so the scan skips
/// `docs/handoffs/` at any depth (as `docs_cli_test` does, #462) and nothing else.
#[test]
fn rename_scan_skips_pipeline_handoffs_only() {
    let old = format!("{}-{}", "setup", "wizard");
    let needle = format!("agent-skills/{old}");
    let tmp = tempfile::tempdir().unwrap();
    let files = [
        "docs/handoffs/848-brief.md",
        "docs/handoffs/848/handoff-A.md",
        "docs/research/handoffs-notes.md",
    ];
    for rel in files {
        let path = tmp.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format!("quotes {needle}/SKILL.md\n")).unwrap();
    }
    let rels: Vec<String> = files.iter().map(|f| (*f).to_owned()).collect();
    let hits = scan_files(tmp.path(), &rels, &needle);
    assert_eq!(hits.len(), 1, "only the non-handoffs file is a hit, got: {hits:?}");
    assert!(hits[0].starts_with("docs/research/handoffs-notes.md:1:"), "got: {hits:?}");
}

#[test]
fn rename_note_is_in_readme_and_docs() {
    let marker = format!("~/.claude/{}", old_skill_path());
    for rel in ["README.md", "docs/setup-wizard.md"] {
        let text = flat(&read(rel));
        assert!(text.contains(&marker), "{rel} lacks the rename note");
        assert!(text.contains("by hand"), "{rel} note must say to remove it by hand");
    }
}

#[test]
fn docs_say_the_command_is_herdr_setup() {
    let docs = flat(&read("docs/setup-wizard.md"));
    assert!(docs.contains("`/herdr-setup`"));
    assert!(read("docs/setup-wizard.md").contains("`herdr-setup`"));
}

#[test]
fn the_wizard_prefix_names_that_stay_are_untouched() {
    let skill = read("agent-skills/herdr-setup/SKILL.md");
    assert!(skill.contains("WIZARD_LIB=<HOME>/.claude/skills/herdr-setup/lib"));
    assert!(skill.contains("wizard-ledger.toml"));
    assert!(root().join("docs/setup-wizard.md").is_file());
}

#[test]
fn stage_count_covers_stage_0_to_stage_11() {
    let skill = flat(&read("agent-skills/herdr-setup/SKILL.md"));
    assert!(skill.contains("`Stage N/11: <name>`"), "intro lacks the N/11 form");
    assert!(skill.contains("N running 0 through 11 — 12 stages total"));
    let docs = flat(&read("docs/setup-wizard.md"));
    assert!(docs.contains("a 12-stage wizard (Stage 0 through Stage 11)"));
}

/// The install command's member path and strip count, read from `SKILL.md`.
fn install_args(text: &str) -> (String, String) {
    let line = text
        .lines()
        .find(|l| l.contains("curl -fsSL") && l.contains("tar -xz"))
        .expect("install command");
    let after = &line[line.find("tar -xz").unwrap()..];
    let mut strip = None;
    let mut member = None;
    for word in after.split_whitespace() {
        if let Some(n) = word.strip_prefix("--strip-components=") {
            strip = Some(n.to_owned());
        } else if word.starts_with("holler-main/") {
            member = Some(word.to_owned());
        }
    }
    (strip.expect("--strip-components"), member.expect("member path"))
}

#[test]
fn install_command_lands_lib_in_herdr_setup() {
    let skill = read("agent-skills/herdr-setup/SKILL.md");
    let (strip, member) = install_args(&skill);
    assert_eq!(member, "holler-main/agent-skills/herdr-setup");
    assert_eq!(strip, "2");
    for rel in ["README.md", "docs/setup-wizard.md"] {
        assert_eq!(install_args(&read(rel)), (strip.clone(), member.clone()), "{rel}");
    }

    // A tarball shaped like GitHub's archive: everything under `holler-main/`.
    let tmp = tempfile::tempdir().unwrap();
    let tree = tmp.path().join("tree/holler-main/agent-skills");
    std::fs::create_dir_all(&tree).unwrap();
    let cp = Command::new("cp")
        .arg("-R")
        .arg(root().join("agent-skills/herdr-setup"))
        .arg(&tree)
        .status()
        .unwrap();
    assert!(cp.success());
    let tarball = tmp.path().join("main.tar.gz");
    let make = Command::new("tar")
        .arg("-czf")
        .arg(&tarball)
        .arg("-C")
        .arg(tmp.path().join("tree"))
        .arg("holler-main")
        .status()
        .unwrap();
    assert!(make.success());

    let skills = tmp.path().join("skills");
    std::fs::create_dir_all(&skills).unwrap();
    let extract = Command::new("tar")
        .arg("-xzf")
        .arg(&tarball)
        .arg("-C")
        .arg(&skills)
        .arg(format!("--strip-components={strip}"))
        .arg(&member)
        .status()
        .unwrap();
    assert!(extract.success());
    assert!(Path::new(&skills.join("herdr-setup/SKILL.md")).is_file());
    assert!(Path::new(&skills.join("herdr-setup/lib/ledger.sh")).is_file());
}
