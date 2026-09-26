#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable, dead_code)] // #149
//! Docs doc-test (issue #155 §6): every `holler …` command the documentation
//! shows must parse against the real clap tree.
//!
//! Two sources, across `README.md` and `docs/**/*.md`:
//! 1. lines inside fenced code blocks (`bash`/`sh`/`shell`/`console`/`text`/
//!    untagged) that start with `holler ` (an optional `$ ` prompt is
//!    stripped),
//! 2. inline code spans `` `holler …` `` anywhere in the prose (a few
//!    `v2.md` §10.1 wire-mapping rows, e.g. `hub query <target> caps`).
//!    The bulk of the surface is the `v2.md` §10 code block (the ADR 0003
//!    table reproduced verbatim, #148), which arrives via source (1).
//!
//! Placeholders are normalised (`<x>` → `x`, `[optional]` dropped, `a|b` →
//! `a`, `…` dropped, a trailing annotation after two spaces / ` (` / ` — `
//! cut). A command whose leaf verb is in `cli-surface.pending.txt` is
//! tolerated (specified, not yet implemented) and reported, never silently
//! skipped.

mod fixture;

use std::path::{Path, PathBuf};

use clap::error::ErrorKind;
use clap::Parser;
use fixture::{pending_leaves, split_args};
use holler_cli::Cli;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// Whether a path relative to the workspace root (a markdown file such as
/// `docs/testing.md`, or a directory the walk would enter) holds CLI
/// documentation the doc-test must scan (#462).
///
/// Skipped: `docs/handoffs/`, the coding pipeline's per-story scratch
/// (briefs, handoffs, decision journals, gate output). It exists only in a
/// pipeline worktree while a story is in flight and is deleted before the PR
/// is pushed, so CI never sees it, but `cargo test --workspace` in that
/// worktree does, and a handoff that quotes a `holler …` fragment (on #420's
/// branch, a Rust format string) failed the whole run on text that is not
/// CLI documentation.
///
/// Scanned, deliberately: every other directory under `docs/`. They hold
/// tracked, hand-written documents. That includes `docs/reviews/`, which is
/// the review battery's runbook, prompts, overlays and pre-flight script, not
/// its output: the battery files its findings as GitHub issues and writes
/// nothing under `docs/`.
///
/// `Path::starts_with` compares whole components, so a page whose name merely
/// contains `handoffs` (e.g. `docs/research/handoffs-notes.md`) is scanned.
fn is_cli_doc(rel: &Path) -> bool {
    !rel.starts_with("docs/handoffs")
}

fn markdown_files() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut out = vec![root.join("README.md")];
    fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            // Filters directories too, so the walk never enters `docs/handoffs/`.
            if !is_cli_doc(p.strip_prefix(root).unwrap()) {
                continue;
            }
            if p.is_dir() {
                walk(root, &p, out);
            } else if p.extension().is_some_and(|x| x == "md") {
                out.push(p);
            }
        }
    }
    walk(&root, &root.join("docs"), &mut out);
    out.sort();
    out
}

/// A documented command and where it came from.
struct Doc {
    file: String,
    line_no: usize,
    raw: String,
}

/// Extract candidate `holler …` commands from one markdown file.
fn extract(path: &Path) -> Vec<Doc> {
    let text = std::fs::read_to_string(path).unwrap();
    let file = path.strip_prefix(workspace_root()).unwrap().display().to_string();
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence_ok = false;
    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        if let Some(rest) = line.trim_start().strip_prefix("```") {
            if in_fence {
                in_fence = false;
            } else {
                in_fence = true;
                let lang = rest.trim();
                fence_ok = matches!(lang, "" | "bash" | "sh" | "shell" | "console" | "text");
            }
            continue;
        }
        if in_fence {
            if fence_ok {
                let t = line.trim_start();
                let t = t.strip_prefix("$ ").unwrap_or(t);
                if t.starts_with("holler ") || t == "holler" {
                    out.push(Doc { file: file.clone(), line_no, raw: t.to_string() });
                }
            }
            continue;
        }
        // Inline code spans in prose / tables.
        let mut rest = line;
        while let Some(start) = rest.find('`') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('`') else { break };
            let span = &after[..end];
            if span.starts_with("holler ") {
                out.push(Doc { file: file.clone(), line_no, raw: span.to_string() });
            }
            rest = &after[end + 1..];
        }
    }
    out
}

/// Turn a documented form into an argv: strip prompt/annotation, resolve
/// placeholders, drop optional groups, pick the first of `a|b` alternatives.
fn normalise(raw: &str) -> Vec<String> {
    let mut s = raw.to_string();
    // Cut a trailing annotation: two+ spaces, " (", or " — ".
    for sep in ["  ", " (", " — ", " -- "] {
        if let Some(i) = s.find(sep) {
            s.truncate(i);
        }
    }
    // Drop optional groups [ ... ], innermost first so nesting such as
    // `[--advertise HOST[:PORT]]` collapses cleanly.
    while let Some(close) = s.find(']') {
        let Some(open) = s[..close].rfind('[') else { break };
        s.replace_range(open..=close, "");
    }
    // <placeholder> -> placeholder
    s = s.replace(['<', '>'], "");
    // ellipses
    s = s.replace('…', "").replace("...", "");
    // a|b alternatives -> a  (on whole tokens)
    let toks: Vec<String> = split_args(&s)
        .into_iter()
        .map(|t| t.split('|').next().unwrap_or("").to_string())
        .filter(|t| !t.is_empty())
        .collect();
    toks
}

/// The leaf verb path of an argv (leading non-flag tokens that clap knows).
fn leaf_of(argv: &[String]) -> String {
    argv.iter()
        .skip(1)
        .take_while(|t| !t.starts_with('-') && t.chars().all(|c| c.is_ascii_lowercase()))
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn every_documented_holler_command_parses() {
    let pending = pending_leaves();
    let mut failures = Vec::new();
    let mut tolerated = Vec::new();
    let mut checked = 0usize;
    for path in markdown_files() {
        for doc in extract(&path) {
            let argv = normalise(&doc.raw);
            if argv.first().map(String::as_str) != Some("holler") {
                continue;
            }
            let leaf = leaf_of(&argv);
            if pending.iter().any(|p| leaf.starts_with(p.as_str())) {
                tolerated.push(format!("{}:{}: {}", doc.file, doc.line_no, doc.raw));
                continue;
            }
            checked += 1;
            match Cli::try_parse_from(&argv) {
                Ok(_) => {}
                // `holler --help` / `--version`, and a bare namespace such as
                // `holler hub` or `holler body …` used in prose to name the
                // role: a prefix of the real tree is fine, an unknown or
                // malformed verb is not.
                Err(e) if matches!(
                    e.kind(),
                    ErrorKind::DisplayHelp
                        | ErrorKind::DisplayVersion
                        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                        | ErrorKind::MissingSubcommand
                ) => {}
                Err(e) => failures.push(format!("{}:{}: `{}` → {:?}: {}", doc.file, doc.line_no, doc.raw, argv, e.kind())),
            }
        }
    }
    eprintln!("docs doc-test: {checked} commands parsed; {} tolerated as pending:\n{}", tolerated.len(), tolerated.join("\n"));
    assert!(checked > 0, "no `holler …` commands found in README.md or docs/**/*.md");
    assert!(
        failures.is_empty(),
        "{} documented commands do not parse against the real CLI:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// #462: the coding pipeline's per-story scratch under `docs/handoffs/`
/// (briefs, handoffs, decision journals, at any depth) is not CLI
/// documentation and is excluded; every other docs page, including one whose
/// name merely contains `handoffs`, stays scanned.
#[test]
fn doc_filter_skips_pipeline_handoffs_only() {
    for excluded in ["docs/handoffs/462-brief.md", "docs/handoffs/462/handoff-A.md", "docs/handoffs/decisions.md"] {
        assert!(!is_cli_doc(Path::new(excluded)), "{excluded} is pipeline scratch and must not be scanned");
    }
    for included in [
        "README.md",
        "docs/testing.md",
        "docs/protocol/v2.md",
        "docs/adr/ADR-0004.md",
        "docs/reviews/review-battery.md",
        "docs/research/handoffs-notes.md",
    ] {
        assert!(is_cli_doc(Path::new(included)), "{included} is documentation and must be scanned");
    }
}

/// #462: the walk itself applies the filter, so a pipeline worktree with
/// files under `docs/handoffs/` never feeds them to the doc-test, while the
/// real docs pages are still returned.
#[test]
fn markdown_files_excludes_pipeline_handoffs() {
    let root = workspace_root();
    let rel: Vec<PathBuf> = markdown_files().iter().map(|p| p.strip_prefix(&root).unwrap().to_path_buf()).collect();
    let leaked: Vec<_> = rel.iter().filter(|p| p.starts_with("docs/handoffs")).collect();
    assert!(leaked.is_empty(), "markdown_files() returned pipeline scratch: {leaked:?}");
    for must in ["README.md", "docs/testing.md", "docs/protocol/v2.md"] {
        assert!(rel.iter().any(|p| p == Path::new(must)), "markdown_files() lost {must}");
    }
}
