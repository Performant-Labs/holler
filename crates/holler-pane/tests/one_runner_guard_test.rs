#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #696
//! Source guard (#696): exactly one bounded subprocess runner exists, in
//! `holler-pane/src/bounded.rs`; the probe and the three adapters call it.
//!
//! The limit, stated plainly: this proves "only one runner", not "the same behaviour". The
//! behaviour is pinned by `bounded_runner_test.rs` and by the unchanged suites of the four
//! crates. The words searched for are built with `concat!` so this file does not match its
//! own search, and comment text is stripped before searching (the idiom of
//! `holler-adapter-host/tests/fake_tmux_test.rs`, `no_broad_kill_in_source`).

use std::fs;
use std::path::{Path, PathBuf};

/// The one file allowed to poll a child.
const RUNNER: &str = "crates/holler-pane/src/bounded.rs";

/// The crates whose sources may not run a child under a deadline by themselves.
const CONSUMERS: [&str; 4] = [
    "holler-pane",
    "holler-adapter-host",
    "holler-adapter-opencode",
    "holler-adapter-claude",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `(path relative to the repo, code with each line's comment removed)` for every source
/// file of the consumer crates.
fn sources() -> Vec<(String, String)> {
    let repo = repo();
    let mut found = Vec::new();
    for name in CONSUMERS {
        let mut files = Vec::new();
        collect_rs(&repo.join("crates").join(name).join("src"), &mut files);
        assert!(!files.is_empty(), "no sources found for {name}");
        for file in files {
            let text = fs::read_to_string(&file).unwrap();
            let code: Vec<&str> = text
                .lines()
                .map(|line| line.split("//").next().unwrap_or_default())
                .collect();
            let rel = file
                .strip_prefix(&repo)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            found.push((rel, code.join("\n")));
        }
    }
    found
}

/// Every file (other than the runner) whose code contains `word`.
fn outside_the_runner(word: &str) -> Vec<String> {
    sources()
        .into_iter()
        .filter(|(path, code)| path != RUNNER && code.contains(word))
        .map(|(path, _)| path)
        .collect()
}

#[test]
fn only_the_shared_runner_polls_a_child() {
    let poll = concat!("try_", "wait(");
    let hits = outside_the_runner(poll);
    assert!(
        hits.is_empty(),
        "{poll} outside {RUNNER}: a private runner copy remains in {hits:#?}"
    );
    let runner = sources()
        .into_iter()
        .any(|(path, code)| path == RUNNER && code.contains(poll));
    assert!(
        runner,
        "{RUNNER} does not exist or does not poll: the runner is not there"
    );
}

#[test]
fn only_the_shared_runner_waits_on_a_reader_with_a_timeout() {
    let word = concat!("recv_", "timeout(");
    let hits = outside_the_runner(word);
    assert!(hits.is_empty(), "{word} outside {RUNNER}: {hits:#?}");
}

#[test]
fn no_adapter_keeps_its_own_drain_collect_wait_or_reap() {
    let mut hits = Vec::new();
    for name in [
        "holler-adapter-host",
        "holler-adapter-opencode",
        "holler-adapter-claude",
    ] {
        let exec = repo().join("crates").join(name).join("src/exec.rs");
        let Ok(text) = fs::read_to_string(&exec) else {
            continue; // deleted: the best outcome
        };
        for word in [
            concat!("fn ", "wait"),
            concat!("fn ", "drain"),
            concat!("fn ", "collect"),
            concat!("fn ", "reap"),
            concat!("thread::", "Builder"),
            concat!("read_", "to_end"),
            concat!("child.", "kill"),
        ] {
            let code: String = text
                .lines()
                .map(|line| line.split("//").next().unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\n");
            if code.contains(word) {
                hits.push(format!("{}: {word}", exec.display()));
            }
        }
    }
    assert!(hits.is_empty(), "a private runner copy remains: {hits:#?}");
}

#[test]
fn the_opencode_and_claude_adapters_have_no_exec_module_left() {
    for name in ["holler-adapter-opencode", "holler-adapter-claude"] {
        let crate_dir = repo().join("crates").join(name).join("src");
        assert!(
            !crate_dir.join("exec.rs").exists(),
            "{name}/src/exec.rs still exists: its runner moved to {RUNNER}"
        );
        let lib = fs::read_to_string(crate_dir.join("lib.rs")).unwrap();
        let declares = lib
            .lines()
            .map(|line| line.split("//").next().unwrap_or_default().trim())
            .any(|line| line.ends_with(concat!("mod ", "exec;")));
        assert!(
            !declares,
            "{name}/src/lib.rs still declares its exec module"
        );
    }
}
