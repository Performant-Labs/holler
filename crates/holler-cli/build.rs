//! Names the commit this binary is built from (issue #851).
//!
//! The script sets `HOLLER_BUILD_COMMIT` for the crate's code. `holler_cli::VERSION`
//! (`src/lib.rs`) is the workspace version followed by that token in parentheses, for
//! example `0.4.0 (bda1069)`: `holler --version` prints it, and `main` installs it for every
//! `version` field the hub and the body print. [`build_version`] picks the token: the
//! checkout's `HEAD`, with `-dirty` when tracked files had changed, else `HOLLER_GIT_SHA`,
//! else `unknown`. Nothing here can fail the build.
//!
//! Cargo runs the script again when `HOLLER_GIT_SHA` changes or a path it prints changes
//! (`docs/releasing.md`, "The commit in the version"):
//!
//! - the git files that move with the commit and the index ([`git_files`]), so a commit, a
//!   checkout, a pull or a staged change is seen at the next build;
//! - the sources compiled into the binary ([`source_paths`]), so a build from an edited
//!   tree says `-dirty` without waiting for the index to change.
//!
//! Once a script prints such paths, Cargo stops re-running it for every change in the
//! package. Editing a tracked file outside them (a doc, a test) re-runs nothing, and the
//! binary, which that file is not part of, keeps naming the tree it was built from.

#[path = "build_version.rs"]
mod build_version;

use std::path::{Path, PathBuf};

fn main() {
    let dir =
        std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| PathBuf::from("."), PathBuf::from);
    let state = build_version::git_state(&dir);
    let holler_git_sha = std::env::var("HOLLER_GIT_SHA").ok();
    let token = build_version::commit_token(
        state.as_ref().map(|(hash, _)| hash.as_str()),
        state.as_ref().is_some_and(|(_, dirty)| *dirty),
        holler_git_sha.as_deref(),
    );
    println!("cargo:rustc-env=HOLLER_BUILD_COMMIT={token}");
    println!("cargo:rerun-if-env-changed=HOLLER_GIT_SHA");
    let mut watched = source_paths(&dir);
    if state.is_some() {
        watched.extend(git_files(&dir));
    }
    for path in watched {
        // Cargo re-runs a script at every build while a path it watches does not exist.
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

/// The git files that change with the commit or the index, as git names them for the
/// checkout `dir` is in: `HEAD` and `logs/HEAD` (every move of `HEAD`), the branch `HEAD`
/// names, `packed-refs` and `reftable` (where a commit lands), and `index` (a stage, a
/// commit, a checkout). In a linked worktree they are its own `HEAD`, log and index, and the
/// main checkout's refs.
fn git_files(dir: &Path) -> Vec<PathBuf> {
    let branch = build_version::git(dir, &["symbolic-ref", "-q", "HEAD"]);
    let names = ["HEAD", "logs/HEAD", "packed-refs", "reftable", "index"]
        .into_iter()
        .chain(branch.as_deref());
    let mut args = vec!["rev-parse"];
    for name in names {
        args.extend(["--git-path", name]);
    }
    let Some(paths) = build_version::git(dir, &args) else {
        return Vec::new();
    };
    paths.lines().map(|path| dir.join(path)).collect()
}

/// The sources compiled into the binary: the `src/` directory and `Cargo.toml` of every
/// crate under `crates/`, the directory this crate is in (the workspace's other members,
/// under `plugins/`, are not part of the binary), and the workspace's `Cargo.toml` and
/// `Cargo.lock`.
fn source_paths(dir: &Path) -> Vec<PathBuf> {
    let Some(crates) = dir.parent() else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(crates)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|member| [member.path().join("src"), member.path().join("Cargo.toml")])
        .collect();
    if let Some(root) = crates.parent() {
        paths.extend([root.join("Cargo.toml"), root.join("Cargo.lock")]);
    }
    paths.sort();
    paths
}
