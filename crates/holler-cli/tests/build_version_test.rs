#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #851
//! Issue #851: `holler --version` names the commit the binary was built from.
//!
//! `crates/holler-cli/build_version.rs` is the one file the build script
//! (`crates/holler-cli/build.rs`) and this test share through `#[path]`. Code
//! under `#[cfg(test)]` in a build script never runs under `cargo test`, so
//! the build-time logic is pinned here instead, without building a fixture
//! crate:
//!
//! - `commit_token(git_hash, dirty, holler_git_sha)` picks the commit token
//!   that follows the version: the checkout's hash (with `-dirty` when tracked
//!   files changed), else `HOLLER_GIT_SHA`, else `unknown`.
//! - `git_state(dir)` asks git for the 7-character hash of `HEAD` and whether
//!   tracked files changed, or `None` when `dir` is not in a git checkout.
//!   A linked worktree (where `.git` is a file) is still a checkout.
//!
//! The `--version` line itself, and the `version` fields that carry the same
//! string, are pinned against the real binary in `cli_invocation_test.rs` and
//! `query_test.rs`.

#[path = "../build_version.rs"]
mod build_version;

use std::path::Path;
use std::process::Command;

use build_version::{commit_token, git_state};

// --- commit_token: the pure resolver ------------------------------------------

/// AC: a build from a git checkout prints the hash of `HEAD`. When a checkout
/// is present it wins over `HOLLER_GIT_SHA`, which is only for builds without
/// `.git` (brief, Work 2).
#[test]
fn a_checkout_uses_the_head_hash() {
    assert_eq!(commit_token(Some("bda1069"), false, None), "bda1069");
    assert_eq!(
        commit_token(Some("bda1069"), false, Some("abc1234")),
        "bda1069",
        "a git checkout's own HEAD wins over HOLLER_GIT_SHA"
    );
}

/// AC: a dirty tree ends `-dirty`. The format is git's own `--dirty` form,
/// inside the parentheses: `holler 0.4.0 (bda1069-dirty)` (handoff-A
/// finding 6, chosen at RED).
#[test]
fn a_dirty_checkout_appends_dirty() {
    assert_eq!(commit_token(Some("bda1069"), true, None), "bda1069-dirty");
    assert_eq!(commit_token(Some("bda1069"), true, Some("abc1234")), "bda1069-dirty");
}

/// AC: a build with `HOLLER_GIT_SHA=abc1234` and no `.git` prints `abc1234`.
/// With no checkout there is no tree to be dirty, so the flag is ignored.
#[test]
fn no_checkout_reads_holler_git_sha() {
    assert_eq!(commit_token(None, false, Some("abc1234")), "abc1234");
    assert_eq!(commit_token(None, true, Some("abc1234")), "abc1234");
}

/// AC: a build with neither a checkout nor `HOLLER_GIT_SHA` prints `unknown`.
/// An empty `HOLLER_GIT_SHA` (set but blank) counts as unset, never `()`.
#[test]
fn neither_is_unknown() {
    assert_eq!(commit_token(None, false, None), "unknown");
    assert_eq!(commit_token(None, false, Some("")), "unknown");
}

// --- git_state: what the build script asks git --------------------------------

/// Runs git in `dir` with an identity and no hooks or signing, so the
/// operator's global git config cannot change or block the fixture.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=holler-test",
            "-c",
            "user.email=holler-test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A fresh repository with one committed file, `tracked.txt`.
fn repo_with_one_commit() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    git(dir.path(), &["init", "-q"]);
    std::fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
    git(dir.path(), &["add", "tracked.txt"]);
    git(dir.path(), &["commit", "-q", "--no-verify", "-m", "one"]);
    dir
}

/// A clean checkout gives the 7-character `git rev-parse --short=7 HEAD`, not
/// dirty. Untracked files do not make a tree dirty (handoff-A finding 6, as
/// `git describe --dirty` counts it), so pipeline scratch files never mark a
/// build dirty.
#[test]
fn git_state_of_a_clean_checkout_is_the_short_head_hash() {
    let repo = repo_with_one_commit();
    let head = git(repo.path(), &["rev-parse", "--short=7", "HEAD"]);
    assert_eq!(head.len(), 7, "fixture HEAD: {head:?}");
    assert_eq!(git_state(repo.path()), Some((head.clone(), false)));

    std::fs::write(repo.path().join("scratch.txt"), "untracked\n").unwrap();
    assert_eq!(
        git_state(repo.path()),
        Some((head, false)),
        "an untracked file does not make the tree dirty"
    );
}

/// AC: a dirty tree. An unstaged edit to a tracked file is dirty, and so is a
/// staged one.
#[test]
fn git_state_of_a_changed_tracked_file_is_dirty() {
    let repo = repo_with_one_commit();
    let head = git(repo.path(), &["rev-parse", "--short=7", "HEAD"]);

    std::fs::write(repo.path().join("tracked.txt"), "two\n").unwrap();
    assert_eq!(git_state(repo.path()), Some((head.clone(), true)), "unstaged edit");

    git(repo.path(), &["add", "tracked.txt"]);
    assert_eq!(git_state(repo.path()), Some((head, true)), "staged edit");
}

/// Every pipeline build runs in a linked worktree, where `.git` is a file and
/// the branch ref lives in the main checkout (handoff-A finding 4). That is
/// still a checkout: the hash is the worktree's own `HEAD`, never `None`.
#[test]
fn git_state_of_a_linked_worktree_is_its_own_head() {
    let repo = repo_with_one_commit();
    std::fs::write(repo.path().join("tracked.txt"), "two\n").unwrap();
    git(repo.path(), &["commit", "-q", "--no-verify", "-am", "two"]);
    let first = git(repo.path(), &["rev-parse", "--short=7", "HEAD~1"]);

    let parent = tempfile::tempdir().expect("temp dir");
    let wt = parent.path().join("wt");
    let wt_arg = wt.to_str().expect("utf-8 temp path");
    git(repo.path(), &["worktree", "add", "-q", "--detach", wt_arg, "HEAD~1"]);
    assert!(wt.join(".git").is_file(), "fixture: a linked worktree's .git is a file");

    assert_eq!(git_state(&wt), Some((first, false)));
}

/// AC precondition for `HOLLER_GIT_SHA` and `unknown`: a directory outside any
/// git checkout (a source archive) has no git state.
#[test]
fn git_state_outside_a_checkout_is_none() {
    let dir = tempfile::tempdir().expect("temp dir");
    let probe = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(dir.path())
        .output()
        .expect("run git");
    assert!(
        !probe.status.success(),
        "fixture: the temp dir {} must not sit inside a git checkout",
        dir.path().display()
    );
    assert_eq!(git_state(dir.path()), None);
}
