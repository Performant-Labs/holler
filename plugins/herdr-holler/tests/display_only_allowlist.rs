#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)] // #651
//! Acceptance: **no call to the Herdr pane-changing API** — as the A gate ruled
//! (brief, A-warn 3), an **allowlist**, stronger than a denylist: the crate's own
//! sources and its manifest may name ONLY `pane.report_metadata`,
//! `workspace.report_metadata` and the read-only set; any other Herdr method
//! name — known mutator or unknown — fails the test.
//!
//! A second tripwire enforces the one SHOWN/DRIVEN rule (A-warn 2): the sync
//! token must be derived through `holler_pane::reconcile::shown_differs`, the
//! gate around it must doc-cite `SessionSync::of`
//! (`crates/holler-cli/src/pane/list.rs`) so the two cannot drift silently, and
//! no source may compare `shown` itself (a second comparison).

use std::fs;
use std::path::{Path, PathBuf};

/// The only Herdr method names the crate's own sources and manifest may carry
/// (the spike section 3 method list minus every mutator; the A gate's allowlist).
const ALLOWED_METHODS: &[&str] = &[
    "pane.report_metadata",
    "workspace.report_metadata",
    "ping",
    "session.snapshot",
    "pane.get",
    "pane.list",
    "pane.read",
    "events.subscribe",
];

/// Event-hook subscription names (spike section 10) share the dotted shape of
/// method names but are events Herdr delivers, not API calls. The manifest test
/// pins exactly these two hooks; no other dotted name gets this exemption.
const EVENT_HOOKS: &[&str] = &["pane.created", "layout.updated"];

/// Every Herdr namespace a method name may live under (spike section 3).
const NAMESPACES: &[&str] = &[
    "session",
    "workspace",
    "tab",
    "pane",
    "layout",
    "server",
    "integration",
    "plugin",
    "agent",
    "worktree",
    "notification",
    "events",
];

/// The crate root, resolveless and stable for the test binary.
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `.rs`/`.toml` file under `dir`, recursively.
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("rs") | Some("toml")
        ) {
            out.push(path);
        }
    }
}

/// The crate's own sources (`src/**`) plus the manifest: the scanned surface.
fn scanned_files() -> Vec<(PathBuf, String)> {
    let mut paths = Vec::new();
    collect_files(&crate_root().join("src"), &mut paths);
    paths.push(crate_root().join("herdr-plugin.toml"));
    paths
        .into_iter()
        .filter_map(|p| fs::read_to_string(&p).ok().map(|text| (p, text)))
        .collect()
}

/// Whether `at` (a byte index in `text`) starts a token character of a dotted
/// name: anything a Herdr method name is built of, or a dot.
fn is_name_byte(text: &str, at: usize) -> bool {
    text.as_bytes()
        .get(at)
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.'))
}

/// Every dotted `namespace.word` token in `text` under a Herdr namespace.
fn dotted_names(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for ns in NAMESPACES {
        let mut from = 0;
        while let Some(at) = text[from..].find(ns) {
            let start = from + at;
            from = start + ns.len();
            // A word boundary before, and a dot right after: `pane.list` yes,
            // `pane_id`, `xpane` and prose no.
            if start > 0 && is_name_byte(text, start - 1) {
                continue;
            }
            if text.as_bytes().get(from) != Some(&b'.') {
                continue;
            }
            let mut end = from + 1;
            while end < text.len()
                && (text.as_bytes()[end].is_ascii_lowercase()
                    || text.as_bytes()[end].is_ascii_digit()
                    || text.as_bytes()[end] == b'_')
            {
                end += 1;
            }
            if end > from + 1 {
                found.push(text[start..end].to_owned());
            }
        }
    }
    found
}

#[test]
fn sources_and_manifest_name_only_allowed_herdr_methods() {
    let mut offenders = Vec::new();
    for (path, text) in scanned_files() {
        for name in dotted_names(&text) {
            let allowed =
                ALLOWED_METHODS.contains(&name.as_str()) || EVENT_HOOKS.contains(&name.as_str());
            if !allowed {
                offenders.push(format!("{}: {}", path.display(), name));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "names outside the display-only allowlist (pane.report_metadata, \
         workspace.report_metadata and read-only methods only):\n{}",
        offenders.join("\n")
    );
}

/// Every `.rs` file under `src/`, with its text.
fn source_files() -> Vec<(PathBuf, String)> {
    let mut paths = Vec::new();
    collect_files(&crate_root().join("src"), &mut paths);
    paths
        .into_iter()
        .filter_map(|p| fs::read_to_string(&p).ok().map(|text| (p, text)))
        .collect()
}

/// Whether `text` compares `shown` itself (`shown ==`, `shown !=`, or the
/// comparison reversed): the one rule is `holler_pane::reconcile::shown_differs`,
/// and a raw comparison would be a second one.
fn has_raw_shown_comparison(text: &str) -> bool {
    // Byte windows only: slicing at an arbitrary index would panic inside a
    // multi-byte character, and every token matched here is ASCII.
    let bytes = text.as_bytes();
    for at in 0..bytes.len() {
        let (width, op_after) = if bytes[at..].starts_with(b"shown") {
            (b"shown".len(), true)
        } else if bytes[at..].starts_with(b"==") || bytes[at..].starts_with(b"!=") {
            (2, false)
        } else {
            continue;
        };
        let mut i = at + width;
        while bytes.get(i) == Some(&b' ') || bytes.get(i) == Some(&b'\t') {
            i += 1;
        }
        let hit = if op_after {
            bytes[i..].starts_with(b"==") || bytes[i..].starts_with(b"!=")
        } else {
            bytes[i..].starts_with(b"shown")
        };
        if hit {
            return true;
        }
    }
    false
}

#[test]
fn sync_comes_from_shown_differs_and_cites_the_cli_cells() {
    let sources = source_files();
    let callers: Vec<&str> = sources
        .iter()
        .filter(|(_, text)| text.contains("shown_differs"))
        .map(|(p, _)| p.to_str().unwrap_or("src"))
        .collect();
    assert!(
        !callers.is_empty(),
        "the sync token must be derived through holler_pane's shown_differs (the one \
         SHOWN/DRIVEN rule), somewhere under src/"
    );
    for (path, text) in &sources {
        if text.contains("shown_differs") {
            assert!(
                text.contains("SessionSync::of"),
                "{} gates shown_differs without doc-citing SessionSync::of \
                 (crates/holler-cli/src/pane/list.rs) — the cite is what keeps the \
                 plugin's sync word and the CLI's SYNC cell from drifting",
                path.display()
            );
        }
        assert!(
            !has_raw_shown_comparison(text),
            "{} compares shown itself; the one rule is shown_differs",
            path.display()
        );
    }
}
