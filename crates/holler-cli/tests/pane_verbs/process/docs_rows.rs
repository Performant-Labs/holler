//! ADR 0003 and the CLI-surface fixture carry the new surface (story #670, AC 7).
//!
//! ADR 0003 "fixes the complete CLI surface": later stories implement exactly it. Each
//! verb has one row in its bare form, grouped by the story that owns it, so a verb story
//! edits only its own row and its own fixture line. (`docs_cli_test` parses every
//! `holler ...` row against the clap tree, and `cli_surface_test` compares the fixture's
//! leaf set with clap's; these tests pin the layout those two cannot see.)

use std::path::{Path, PathBuf};

use crate::{assert_no_failures, PANE_VERBS, PROFILE_VERBS};

fn workspace_file(rel: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The ADR's lines that start with `holler <words>` as a whole row (the verb word is
/// followed by whitespace or the end of the line, so `pane list` does not match `pane listen`).
fn adr_rows(adr: &str, words: &str) -> Vec<String> {
    let prefix = format!("holler {words}");
    adr.lines()
        .map(str::trim_start)
        .filter(|l| {
            l.strip_prefix(&prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        })
        .map(str::to_string)
        .collect()
}

#[test]
fn adr_0003_has_one_row_per_pane_and_profile_verb() {
    let adr = workspace_file("docs/adr/ADR-0003.md");
    let mut failures = Vec::new();
    for (namespace, verbs) in [("pane", PANE_VERBS), ("profile", PROFILE_VERBS)] {
        for &(verb, _) in verbs {
            let rows = adr_rows(&adr, &format!("{namespace} {verb}"));
            if rows.len() != 1 {
                failures.push(format!("ADR-0003 must have exactly one `holler {namespace} {verb}` row, found {}: {rows:?}", rows.len()));
            }
        }
    }
    assert_no_failures(failures);
}

/// Rows are grouped by owning story: rows of two different stories never sit on adjacent
/// lines (a rebase conflict is a conflict on adjacent lines), so each group is set off by a
/// separator, a blank line in ADR 0003's text block.
#[test]
fn adr_0003_rows_of_different_stories_are_never_adjacent() {
    let adr = workspace_file("docs/adr/ADR-0003.md");
    let lines: Vec<&str> = adr.lines().collect();
    let groups: [&[&str]; 9] = [
        &["pane list", "pane get", "pane watch"],    // #643
        &["pane launch", "pane relaunch"],           // #644
        &["pane switch", "pane reset"],              // #645
        &["pane park", "pane unpark", "pane close"], // #646
        &["pane doctor"],                            // #647
        &["pane import"],                            // #650
        &[
            "profile create",
            "profile delete",
            "profile list",
            "profile show",
        ], // #662
        &["profile apply"],                          // #664
        &["profile rename", "profile export", "profile import"], // #665
    ];
    let group_of = |line: &str| {
        groups.iter().position(|g| {
            g.iter().any(|words| {
                line.trim_start()
                    .strip_prefix(&format!("holler {words}"))
                    .is_some_and(|r| r.is_empty() || r.starts_with(char::is_whitespace))
            })
        })
    };
    let mut failures = Vec::new();
    for pair in lines.windows(2) {
        if let (Some(a), Some(b)) = (group_of(pair[0]), group_of(pair[1])) {
            if a != b {
                failures.push(format!(
                    "rows of two stories are adjacent:\n  {}\n  {}",
                    pair[0], pair[1]
                ));
            }
        }
    }
    let rows = lines.iter().filter(|l| group_of(l).is_some()).count();
    if rows < 20 {
        failures.push(format!(
            "only {rows} of the 20 pane and profile rows are in ADR-0003"
        ));
    }
    assert_no_failures(failures);
}

#[test]
fn adr_0003_mentions_the_new_flags_and_the_amended_sentences() {
    let adr = workspace_file("docs/adr/ADR-0003.md");
    let mut failures = Vec::new();
    for needle in ["--format", "--pane", "--profile", "#633", "ADR-0021"] {
        if !adr.contains(needle) {
            failures.push(format!("ADR-0003 should mention {needle}"));
        }
    }
    // The old sentence said say/interrupt/roster are the only top-level verbs.
    if adr.contains("`say`/`interrupt`/`roster` are the only top-level verbs") {
        failures.push(
            "the \"only top-level verbs\" sentence must be amended for pane and profile"
                .to_string(),
        );
    }
    let global_flags = adr
        .lines()
        .find(|l| l.contains("global flags on every subcommand"))
        .unwrap_or_default();
    if !global_flags.contains("--format") {
        failures.push(format!(
            "the global-flags line should list --format: {global_flags:?}"
        ));
    }
    // The rows list only the shared flags; each owning story adds its own.
    if !adr.to_lowercase().contains("owning stor") {
        failures.push(
            "ADR-0003 should say each owning story adds its own positionals and flags".to_string(),
        );
    }
    assert_no_failures(failures);
}

// --- the CLI-surface fixture ---------------------------------------------------

fn fixture() -> String {
    workspace_file("crates/holler-cli/tests/fixtures/cli-surface.txt")
}

#[test]
fn the_fixture_covers_every_new_leaf_and_every_shared_flag() {
    let fixture = fixture();
    let mut failures = Vec::new();
    for (namespace, verbs) in [("pane", PANE_VERBS), ("profile", PROFILE_VERBS)] {
        for &(verb, _) in verbs {
            let leaf = format!("{namespace} {verb} |");
            if !fixture.lines().any(|l| l.trim_start().starts_with(&leaf)) {
                failures.push(format!("cli-surface.txt has no `{leaf}` line"));
            }
        }
    }
    for flag in [
        "--format",
        "--pane",
        "--profile",
        "--spec-only",
        "--take-over",
        "--project",
        "--workspace",
        "--grid",
        "--model",
        "--effort",
        "--role",
        "--env",
        "--ctx-soft",
        "--ctx-hard",
        "--port-policy",
        "--command-arg",
        "--command-json",
        "--check-arg",
        "--check-json",
        "--expect",
    ] {
        if !fixture
            .lines()
            .any(|l| l.split('#').next().is_some_and(|code| code.contains(flag)))
        {
            failures.push(format!("cli-surface.txt never uses {flag}"));
        }
    }
    // The regression lines for the forms the `--pane` redesign could break.
    for line in ["say | io/alpha --parts-file f", "interrupt | io/alpha"] {
        if !fixture.lines().any(|l| l.trim() == line) {
            failures.push(format!(
                "cli-surface.txt lacks the regression line `{line}`"
            ));
        }
    }
    assert_no_failures(failures);
}

/// The new fixture lines are grouped by owning story under a `# #NNN` comment line, so
/// two verb stories never edit adjacent lines.
#[test]
fn the_fixture_groups_new_lines_by_owning_story() {
    let fixture = fixture();
    let lines: Vec<&str> = fixture.lines().collect();
    let mut failures = Vec::new();
    for story in [643, 644, 645, 646, 647, 648, 650, 662, 664, 665] {
        let header = format!("# #{story}");
        if !lines.iter().any(|l| l.trim() == header) {
            failures.push(format!("cli-surface.txt has no `{header}` comment line"));
        }
    }
    // Each pane/profile leaf line sits under the header of its owning story.
    for (namespace, verbs) in [("pane", PANE_VERBS), ("profile", PROFILE_VERBS)] {
        for &(verb, story) in verbs {
            let header = format!("# #{story}");
            let leaf = format!("{namespace} {verb} |");
            let Some(at) = lines.iter().position(|l| l.trim_start().starts_with(&leaf)) else {
                continue;
            };
            let owner = lines[..at]
                .iter()
                .rev()
                .find(|l| l.trim_start().starts_with("# #"))
                .map(|l| l.trim());
            if owner != Some(header.as_str()) {
                failures.push(format!(
                    "`{leaf}` should sit under `{header}`, found under {owner:?}"
                ));
            }
        }
    }
    assert_no_failures(failures);
}
