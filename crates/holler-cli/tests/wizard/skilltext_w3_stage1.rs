//! #786: Stage 1 text: `hub_host` for local-only runs, the `r1c1` default, one grid rule, and the
//! first-run notes, epic #769.
//!
//! Plain text assertions over `agent-skills/setup-wizard/SKILL.md`, `docs/setup-wizard.md` and
//! `README.md`. Nothing here runs a script or touches a host.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #786

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::read_to_string(root.join(rel)).unwrap()
}

/// A file with every run of whitespace folded to one space, for phrases that wrap.
fn flat(rel: &str) -> String {
    read(rel).split_whitespace().collect::<Vec<_>>().join(" ")
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
    flat("agent-skills/setup-wizard/SKILL.md")
}

fn stage(n: u32) -> String {
    between(&skill(), &format!("## Stage {n} "), &format!("## Stage {} ", n + 1))
}

fn has_all(what: &str, text: &str, needles: &[&str]) {
    for n in needles {
        assert!(text.contains(n), "{what} lacks `{n}`");
    }
}

fn readme_wizard() -> String {
    let text = read("README.md");
    let section = between(&text, "## Set up a Herdr workspace with an agent", "\n## ");
    section.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn hub_host_is_optional_for_zero_sessions_and_required_from_one() {
    let s1 = stage(1);
    has_all(
        "Stage 1",
        &s1,
        &[
            "`hub_host` is optional when N=0",
            "required when N>=1",
            "leave it out",
            "a missing/empty `hub_host` when N>=1",
        ],
    );
    assert!(
        !s1.contains("top-level `hub_host` (non-empty), at least one"),
        "Verify must not demand hub_host unconditionally"
    );
}

#[test]
fn stage_six_leaves_advertise_out_without_a_hub_host() {
    let s6 = stage(6);
    has_all(
        "Stage 6",
        &s6,
        &[
            "no `hub_host`",
            "leave `--advertise` out",
            "advertises nothing remote",
            "--advertise <hub_host>:<serve_https_port>",
        ],
    );
}

#[test]
fn scenarios_a_and_b_and_the_readme_propose_r1c1() {
    let s1 = stage(1);
    has_all("Stage 1", &s1, &["`name = \"r1c1\"`", "`[[\"r1c1\"]]`"]);
    assert!(!s1.contains("name = \"o1\""), "Stage 1 still proposes o1");
    assert!(!s1.contains("[[\"o1\"]]"), "Stage 1 still computes o1");
    let readme = flat("README.md");
    has_all("README", &readme, &["`name = \"r1c1\"`"]);
    assert!(!readme.contains("name = \"o1\""), "README still proposes o1");
}

#[test]
fn the_grid_rule_is_stated_once_per_file() {
    for (what, text) in [
        ("Stage 1", stage(1)),
        ("docs", flat("docs/setup-wizard.md")),
        ("README", readme_wizard()),
    ] {
        has_all(what, &text, &["two rows", "up to 8 panes", "above 8 panes"]);
        assert_eq!(text.matches("squarest grid").count(), 1, "{what}: one rule");
        assert!(
            !text.contains("propose the squarest grid"),
            "{what}: the old unconditional rule"
        );
    }
}

#[test]
fn the_docs_keep_the_global_config_at_the_default_path_for_every_instance() {
    let docs = flat("docs/setup-wizard.md");
    has_all(
        "docs",
        &docs,
        &[
            "even for a non-default instance",
            "`~/.holler/herdr/setup/sessions.toml`",
        ],
    );
    assert!(
        !docs.contains("and the global master config `sessions.toml`."),
        "the old claim that the global config lives in <state_dir>"
    );
}

#[test]
fn the_readme_uses_a_placeholder_host_in_its_join_held_example() {
    let readme = read("README.md");
    assert!(!readme.contains("io/*"), "a real-looking workstation name");
    assert!(readme.contains("--join-held 'remote-a/*'"));
}

#[test]
fn stage_one_notes_the_first_run_directory_and_what_it_writes() {
    let s1 = stage(1);
    has_all(
        "Stage 1",
        &s1,
        &["mkdir -p ~/.holler/herdr/setup"],
    );
    assert!(
        skill().contains(
            "read-only except the scratch directory and the config file you write from the user's answers"
        ),
        "the read-only sentence names the config file"
    );
}
