#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! The CLI surface as a fixture (issue #155 §5).
//!
//! `tests/fixtures/cli-surface.txt` is the normative, machine-readable form
//! of ADR 0003's CLI table. Three properties are pinned:
//!
//! 1. every line in it parses (`Cli::try_parse_from`),
//! 2. every line in `cli-surface.pending.txt` does **not** parse yet — so the
//!    story that adds a verb must move its line over, keeping both files
//!    honest in both directions,
//! 3. the set of leaf verbs in the fixture equals the set clap knows — a verb
//!    added to the tree without a fixture line (or vice versa) fails here.
//!
//! This is what would have caught the `--json`-on-the-wrong-struct bug
//! (#147): `hub token mint --label x --json` is a fixture line.

mod fixture;

use std::collections::BTreeSet;

use clap::{CommandFactory, Parser};
use fixture::{load_surface, split_args, SurfaceLine};
use holler_cli::Cli;

fn try_parse(line: &SurfaceLine) -> Result<(), String> {
    let mut argv: Vec<String> = vec!["holler".to_string()];
    argv.extend(line.leaf.split_whitespace().map(str::to_string));
    argv.extend(split_args(&line.args));
    Cli::try_parse_from(&argv).map(|_| ()).map_err(|e| format!("{argv:?}: {e}"))
}

/// Every leaf verb path clap knows (`hub token mint`, `roster`, …), built by
/// walking the derived command tree. `help` is clap's own, not ours.
fn clap_leaves() -> BTreeSet<String> {
    fn walk(cmd: &clap::Command, prefix: &str, out: &mut BTreeSet<String>) {
        let mut subs = cmd.get_subcommands().filter(|c| c.get_name() != "help").peekable();
        if subs.peek().is_none() {
            out.insert(prefix.trim().to_string());
            return;
        }
        for sub in subs {
            walk(sub, &format!("{prefix} {}", sub.get_name()), out);
        }
    }
    let mut out = BTreeSet::new();
    let cmd = Cli::command();
    for sub in cmd.get_subcommands().filter(|c| c.get_name() != "help") {
        walk(sub, sub.get_name(), &mut out);
    }
    out
}

#[test]
fn every_surface_line_parses() {
    let lines = load_surface("cli-surface.txt");
    assert!(!lines.is_empty(), "cli-surface.txt has no invocations");
    let failures: Vec<String> = lines.iter().filter_map(|l| try_parse(l).err()).collect();
    assert!(
        failures.is_empty(),
        "{} of {} surface lines failed to parse:\n{}",
        failures.len(),
        lines.len(),
        failures.join("\n")
    );
}

#[test]
fn every_pending_line_does_not_parse_yet() {
    let lines = load_surface("cli-surface.pending.txt");
    let parsed: Vec<String> = lines
        .iter()
        .filter(|l| try_parse(l).is_ok())
        .map(|l| format!("{} | {}", l.leaf, l.args))
        .collect();
    assert!(
        parsed.is_empty(),
        "these pending lines now parse — the owning story landed; move them to cli-surface.txt:\n{}",
        parsed.join("\n")
    );
}

#[test]
fn fixture_leaf_set_equals_clap_leaf_set() {
    let fixture: BTreeSet<String> = load_surface("cli-surface.txt")
        .into_iter()
        .map(|l| canonical_leaf(&l.leaf))
        .collect();
    let clap = clap_leaves();
    let missing_from_fixture: Vec<_> = clap.difference(&fixture).cloned().collect();
    let unknown_to_clap: Vec<_> = fixture.difference(&clap).cloned().collect();
    assert!(
        missing_from_fixture.is_empty() && unknown_to_clap.is_empty(),
        "leaf verbs out of sync.\n  in clap but not in cli-surface.txt: {missing_from_fixture:?}\n  in cli-surface.txt but unknown to clap: {unknown_to_clap:?}"
    );
}

/// Aliases (`rm`, `remove`) resolve to their canonical leaf for the set
/// comparison; clap reports the canonical name, the fixture exercises the
/// aliases as separate lines.
fn canonical_leaf(leaf: &str) -> String {
    match leaf {
        "hub token rm" | "hub token remove" => "hub token delete".to_string(),
        other => other.to_string(),
    }
}
