#![allow(dead_code)] // #155 — shared by cli_surface_test and docs_cli_test; not every fn is used by both
//! Readers for the CLI-surface fixtures (issue #155 §5/§6).
//!
//! Format of `tests/fixtures/cli-surface*.txt`: one invocation per line,
//! `<leaf verb path> | <arguments>`; `#` starts a comment (whole line or
//! trailing); blank lines are skipped.

use std::path::PathBuf;

/// One fixture invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceLine {
    /// The leaf verb path, e.g. `hub token mint`.
    pub leaf: String,
    /// Everything after the `|`, verbatim (may be empty).
    pub args: String,
}

/// Read a fixture file from `tests/fixtures/`.
pub fn load_surface(name: &str) -> Vec<SurfaceLine> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", name].iter().collect();
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .map(strip_comment)
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (leaf, args) = l.split_once('|').unwrap_or_else(|| panic!("fixture line has no `|`: {l:?}"));
            SurfaceLine { leaf: leaf.trim().to_string(), args: args.trim().to_string() }
        })
        .collect()
}

/// The leaf paths listed in `cli-surface.pending.txt` — verbs ADR 0003
/// specifies that no story has implemented yet.
pub fn pending_leaves() -> Vec<String> {
    load_surface("cli-surface.pending.txt").into_iter().map(|l| l.leaf).collect()
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

/// Split an argument string on whitespace, keeping double-quoted groups as
/// one token (quotes removed). Enough for fixture lines; not a shell.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    for c in s.chars() {
        match (c, in_q) {
            ('"', _) => in_q = !in_q,
            (c, false) if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            (c, _) => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
