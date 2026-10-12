//! The commit `holler --version` names (issue #851), worked out at build time.
//!
//! `build.rs` includes this file, and so does `tests/build_version_test.rs`, both through
//! `#[path]`: a build script's own `#[cfg(test)]` code never runs under `cargo test`, so the
//! logic the build runs is pinned by that test instead. Standard library only, because the
//! build script has no dependencies.

use std::path::Path;
use std::process::Command;

/// The commit token that follows the version, as in `holler 0.4.0 (bda1069)`.
///
/// A git checkout's own hash comes first, with `-dirty` when tracked files had changed
/// (git's own `--dirty` form). Without a checkout, the first word of `HOLLER_GIT_SHA`, the
/// build-time variable for a build from a source archive; an empty value counts as unset.
/// With neither, `unknown`. The token is one word in every case.
pub fn commit_token(git_hash: Option<&str>, dirty: bool, holler_git_sha: Option<&str>) -> String {
    let sha = holler_git_sha.and_then(|sha| sha.split_whitespace().next());
    match (git_hash, sha) {
        (Some(hash), _) if dirty => format!("{hash}-dirty"),
        (Some(hash), _) => hash.to_owned(),
        (None, Some(sha)) => sha.to_owned(),
        (None, None) => "unknown".to_owned(),
    }
}

/// What git says about the checkout `dir` is in: the 7-character hash of `HEAD`
/// (`git rev-parse --short=7 HEAD`) and whether tracked files differ from it, staged or
/// not. Untracked files do not count, as with `git describe --dirty`, so scratch files
/// never mark a build dirty. When git names `HEAD` but cannot tell whether the tree
/// changed, the tree counts as changed.
///
/// `None` when no git checkout tracks a file in `dir` (no checkout at all, or a source
/// archive unpacked inside some other repository, whose hash is not this source's), when
/// `HEAD` has no commit yet, or when git cannot run. A linked worktree, whose `.git` is a
/// file, is a checkout like any other: git resolves it.
pub fn git_state(dir: &Path) -> Option<(String, bool)> {
    git(dir, &["ls-files", "--error-unmatch", "--", "."])?;
    let hash = git(dir, &["rev-parse", "--short=7", "HEAD"])?;
    let status = git(dir, &["status", "--porcelain", "--untracked-files=no"]);
    Some((hash, status.is_none_or(|changes| !changes.is_empty())))
}

/// Runs `git ARGS` in `dir` and returns its standard output, trimmed, or `None` when git
/// cannot start or exits non-zero; its standard error is never shown. With
/// `GIT_OPTIONAL_LOCKS=0`, `git status` does not write the index back, so a build never
/// holds the index lock against a git command run beside it, and never changes a file
/// `build.rs` asks Cargo to watch.
pub fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
