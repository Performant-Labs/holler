#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #850
//! The text #850 makes true or false (Acceptance 8 and 10). A Claude Code pane has a body once
//! #850 merges, so:
//!
//! - the contract is amended first (ADR 0014 points 7 to 11, ADR-0021 section 5's crate rule and
//!   section 15 items 9 and 19, the support rule of `docs/protocol/v2.md`, the README recipe and
//!   the CHANGELOG): `the_contract_is_amended`;
//! - none of the present-tense statements that a Claude Code pane has no body, or that "#850" will
//!   give it one, is left in the ADR, the guide, the CHANGELOG, the module docs, the comments of
//!   the `pane_claude` tests or the wizard's text: `the_sentences_a_claude_body_replaced_are_gone`
//!   (A6's stale list) and `the_wizard_no_longer_says_a_claude_pane_has_no_body_row`.
//!
//! Each file is read as `docs_pane_control_test.rs` (#839) reads one: each line's comment or quote
//! marker dropped, then every run of whitespace one space, so a sentence wrapped across lines is
//! found. A missing file reads as empty, so each test still reports its own list. Every test
//! collects its failures and asserts once, so one run lists every item.
//!
//! The wizard's six places are `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md`.
//! The operator decided (N1) that #850 rewrites their text, and only their text: the test is RED
//! while any of them still says a Claude Code pane has no body row or no roster row "until #850"
//! or "yet (#850)". It does not pin the replacement's words.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// A workspace file's text, or the empty string when it does not exist.
fn read(rel: &str) -> String {
    std::fs::read_to_string(workspace_root().join(rel)).unwrap_or_default()
}

/// `text` as one line of words (see the module docs).
fn normalised(text: &str) -> String {
    let lines = text.lines().map(|line| {
        let line = line.trim_start();
        ["//!", "///", "//", ">"].iter().fold(line, |l, marker| l.strip_prefix(marker).unwrap_or(l))
    });
    lines.collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_none(what: &str, found: Vec<String>) {
    assert!(found.is_empty(), "{what}: {} item(s)\n{}", found.len(), found.join("\n"));
}

// --- the contract is amended first (A1 to A6) ---------------------------------------------------

/// Anchors of the amendment: headings and rule names, not whole sentences (F folds the plan
/// review's two wording warnings into the text it copies).
#[test]
fn the_contract_is_amended() {
    let adr14 = normalised(&read("docs/adr/ADR-0014.md"));
    let adr21 = normalised(&read("docs/adr/ADR-0021.md"));
    let wire = normalised(&read("docs/protocol/v2.md"));
    let readme = normalised(&read("README.md"));
    let guide = normalised(&read("docs/pane-control.md"));

    let wanted: [(&str, &str, &str); 17] = [
        ("docs/adr/ADR-0014.md", &adr14, "**Amended:** [#850]"),
        ("docs/adr/ADR-0014.md", &adr14, "### Amendment (#850): attaching to a Claude Code process"),
        ("docs/adr/ADR-0014.md", &adr14, "7. **One row shape, chosen by the harness id and the mode.**"),
        ("docs/adr/ADR-0014.md", &adr14, "8. **Delivery and observation have one implementation"),
        ("docs/adr/ADR-0014.md", &adr14, "9. **A turn.**"),
        ("docs/adr/ADR-0014.md", &adr14, "10. **Cancel, replace and answer.**"),
        ("docs/adr/ADR-0014.md", &adr14, "11. **`session/rebind`.**"),
        ("docs/adr/ADR-0021.md", &adr21, "`holler-body` depends on `holler-pane` (#850)"),
        ("docs/adr/ADR-0021.md", &adr21, "19. **The body of a Claude Code pane (#850).**"),
        ("docs/adr/ADR-0021.md", &adr21, "**The drive rule.**"),
        ("docs/adr/ADR-0021.md", &adr21, "**As built (#850):**"),
        ("docs/adr/ADR-0021.md", &adr21, "A Claude Code pane's body drives the `HarnessPort` that `holler-cli` hands it"),
        ("docs/protocol/v2.md", &wire, "claude not found on PATH"),
        ("docs/protocol/v2.md", &wire, "attaches to a Claude Code process"),
        ("docs/protocol/v2.md", &wire, "for a Claude Code row, its session id, a UUID"),
        ("README.md", &readme, "### A Claude Code pane"),
        ("README.md", &readme, "mode = \"attach\" session_id ="),
    ];
    let mut missing: Vec<String> = wanted
        .iter()
        .filter(|(_, text, anchor)| !text.contains(anchor))
        .map(|(file, _, anchor)| format!("{file}: missing {anchor:?}"))
        .collect();
    if !guide.contains("through its body") {
        missing.push("docs/pane-control.md: must say `say --pane` and `reset --first` reach a Claude Code pane through its body".to_owned());
    }
    // The CHANGELOG has a bullet of its own naming #850 under `## [Unreleased]`: before the story
    // two bullets name it (#831's, which says `say --pane` does not reach a Claude Code pane yet,
    // and #839's, which is history and stays); after it #831's bullet says it reaches the pane
    // through its body (#850), and the new bullet is the third.
    let raw = read("CHANGELOG.md");
    // `## [Unreleased]` plus the newest released section: the release flow (docs/releasing.md)
    // moves the entries under the new `## [x.y.z]` heading and leaves `[Unreleased]` empty.
    let sections: Vec<&str> = raw.split("\n## [").collect();
    let unreleased = sections
        .iter()
        .position(|s| s.starts_with("Unreleased"))
        .map(|at| sections[at..sections.len().min(at + 2)].join("\n"))
        .unwrap_or_default();
    let naming = unreleased.split("\n- ").filter(|bullet| bullet.contains("#850")).count();
    if naming < 3 {
        missing.push(format!(
            "CHANGELOG.md: `## [Unreleased]` needs a bullet of its own naming #850 (found {naming} bullet(s) that name it, two before the story)"
        ));
    }
    assert_none("the amendment of the contract", missing);
}

// --- A6's stale list ---------------------------------------------------------------------------

/// A6's table: each file, and the phrases it must no longer hold. Every phrase is a present-tense
/// statement that #850 makes false (or a sentence A3 and A4 replace).
fn stale() -> Vec<(&'static str, Vec<&'static str>)> {
    let adr = vec![
        "it has no body, item 18",
        "until a body drives its session",
        "until a body drives it (#850)",
        "no body drives a Claude Code pane until #850",
        "until a body can drive it (#850)",
        "Only the HTTP attach driver",
        "The body driver for `say` (#850)",
        "A body for a Claude Code pane, so that",
        "`interrupt-text-unsupported` in this story.",
        "a body for another harness implements it in its own driver",
        "Only an OpenCode attach body exists today",
    ];
    let guide = vec![
        "does not reach a Claude Code pane yet",
        "until a body drives it (#850)",
        "until a body drives its session",
        "reach a Claude Code pane (#850)",
    ];
    let say_interrupt = vec!["no body exists to run the text after it until #850", "has no body for the hub to reach"];
    let switch = vec!["no body drives such a pane until #850", "No body drives a single process until #850"];
    let reconcile = vec!["which no body drives until #850", "has no body until #850", "observes no DRIVEN"];
    let tests = vec![
        "until a body drives a Claude Code pane (#850)",
        "no body drives a Claude Code pane until #850",
        "naming #839 and #850",
        "needs #839 and #850",
        "no body at all until #850",
    ];
    vec![
        ("docs/adr/ADR-0021.md", adr),
        // Plan review W-6: no step of Holler writes a Claude row "from the registry" (D6); the
        // operator writes it from the pane's record. A guard on the amendment's wording; it passes
        // before the story.
        ("docs/adr/ADR-0014.md", vec!["written afresh from the registry"]),
        ("docs/pane-control.md", guide.clone()),
        ("CHANGELOG.md", guide),
        ("crates/holler-cli/src/say_cmd.rs", say_interrupt.clone()),
        ("crates/holler-cli/src/interrupt_cmd.rs", say_interrupt),
        ("crates/holler-pane/src/tx_switch.rs", switch.clone()),
        ("crates/holler-pane/src/tx_switch/plan.rs", switch),
        ("crates/holler-pane/src/reconcile.rs", reconcile.clone()),
        ("crates/holler-pane/src/reconcile/observe.rs", reconcile),
        ("crates/holler-body/src/session_manager.rs", vec!["Only the HTTP attach driver"]),
        ("crates/holler-cli/tests/pane_claude/driven.rs", tests.clone()),
        ("crates/holler-cli/tests/pane_claude/lifecycle.rs", tests.clone()),
        ("crates/holler-cli/tests/pane_claude/rig.rs", tests.clone()),
        ("crates/holler-cli/tests/pane_claude/main.rs", tests.clone()),
        ("crates/holler-cli/tests/pane_claude/mixed.rs", tests),
    ]
}

#[test]
fn the_sentences_a_claude_body_replaced_are_gone() {
    let mut found = Vec::new();
    for (file, phrases) in stale() {
        let text = normalised(&read(file));
        assert!(!text.is_empty(), "{file} is read");
        found.extend(phrases.iter().filter(|p| text.contains(*p)).map(|p| format!("{file}: {p}")));
    }
    assert_none("sentences that #850 makes false", found);
}

/// `mixed.rs` loses the ignored `say` step (it is `say_pane_scenario_test` now), and `main.rs` stops
/// naming it: no `#[ignore]` is left in the `pane_claude` target for #850.
#[test]
fn the_ignored_say_step_of_the_scenario_is_gone() {
    let mut found = Vec::new();
    for file in ["mixed.rs", "main.rs", "rig.rs", "driven.rs", "lifecycle.rs"] {
        let rel = format!("crates/holler-cli/tests/pane_claude/{file}");
        let text = read(&rel);
        if text.contains("#[ignore") {
            found.push(format!("{rel}: an ignored test is left"));
        }
        if text.contains("say_delivers_a_message_to_each_pane_of_the_scenario") {
            found.push(format!("{rel}: still names the ignored say step"));
        }
    }
    assert_none("the ignored say step", found);
}

// --- the wizard's six places (N1, Acceptance 10) -------------------------------------------------

/// The 160 characters before each mention of #850 in `text` (a `#850` or a link to issue 850).
fn before_each_mention(text: &str) -> Vec<String> {
    let mut contexts = Vec::new();
    for needle in ["#850", "issues/850"] {
        for (at, _) in text.match_indices(needle) {
            let mut start = at.saturating_sub(160);
            while !text.is_char_boundary(start) {
                start += 1;
            }
            contexts.push(text[start..at + needle.len()].to_owned());
        }
    }
    contexts
}

/// A context that says a Claude Code pane has no body or roster row (the six old places are
/// "no body row and no roster row until #850", "no server and no body row yet (#850)", "has no
/// body row and no roster row yet (#850)", "has no body row yet** (#850)", "(no body row until
/// #850)" and "so it has no roster row either" before a link to the issue).
fn says_no_row(context: &str) -> bool {
    let lower = context.to_lowercase();
    ["no body row", "no roster row", "has no body", "no body yet", "has no row"].iter().any(|p| lower.contains(p))
}

#[test]
fn the_wizard_no_longer_says_a_claude_pane_has_no_body_row() {
    let mut found = Vec::new();
    for file in ["agent-skills/herdr-setup/SKILL.md", "docs/setup-wizard.md"] {
        let text = normalised(&read(file));
        assert!(!text.is_empty(), "{file} is read");
        for context in before_each_mention(&text) {
            if says_no_row(&context) {
                found.push(format!("{file}: ...{}", context.trim_start()));
            }
        }
        for phrase in ["no body row yet", "no body row and no roster row until", "no roster row either"] {
            if text.contains(phrase) {
                found.push(format!("{file}: {phrase}"));
            }
        }
    }
    assert_none("the wizard's text says a Claude Code pane has no body row", found);
}

/// The wizard's own text must keep saying what #803 built and #850 leaves alone: Stage 8 derives
/// attach rows for OpenCode panes only. (A guard on the rewrite, not on #850's words: it passes
/// before the story and must pass after.)
#[test]
fn the_wizard_still_derives_opencode_rows_only() {
    let skill = normalised(&read("agent-skills/herdr-setup/SKILL.md"));
    assert!(skill.contains("Stage 8"), "Stage 8 is still in the skill");
    assert!(skill.contains("OpenCode"), "and the skill still says OpenCode panes get rows");
}
