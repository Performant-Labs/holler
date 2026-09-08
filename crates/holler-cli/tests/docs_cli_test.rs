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

fn markdown_files() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut out = vec![root.join("README.md")];
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "md") {
                out.push(p);
            }
        }
    }
    walk(&root.join("docs"), &mut out);
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
