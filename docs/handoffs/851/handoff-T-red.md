# Handoff-T-red: #851 - `holler --version` prints the commit the binary was built from

**Date:** 2026-10-11
**Branch:** issue-851-implementation
**Brief / wireframe reviewed:** docs/handoffs/851-brief.md, docs/handoffs/851/handoff-A.md (wireframe: N/A, no UI surface)

## A precondition
Confirmed: A returned PASS on the plan, with 8 warns (handoff-A.md). These tests follow findings 1, 5 and 6.

## Contract F implements against (chosen at RED)

- **Format:** `holler <CARGO_PKG_VERSION> (<commit>)`, for example `holler 0.4.0 (bda1069)`. On a dirty tree it is
  `holler 0.4.0 (bda1069-dirty)`, git's own `--dirty` form inside the parentheses (finding 6). The first two
  tokens are unchanged. Read this way, the brief's "ends `-dirty`" means the commit token ends `-dirty`.
- **Dirty:** a tree is dirty only when tracked files have changed, staged or unstaged. Untracked files do not count
  (finding 6, as `git describe --dirty` counts it).
- **Precedence:** a git checkout's `HEAD` comes first, then a non-empty `HOLLER_GIT_SHA`, then `unknown`. The
  `-dirty` suffix applies only to a checkout's hash.
- **Shared resolver file:** `crates/holler-cli/build_version.rs`. `build.rs` includes it, and so does
  `tests/build_version_test.rs`, through `#[path = "../build_version.rs"]`. It must export:
  - `pub fn commit_token(git_hash: Option<&str>, dirty: bool, holler_git_sha: Option<&str>) -> String`
  - `pub fn git_state(dir: &std::path::Path) -> Option<(String, bool)>`, which returns the 7-character
    `git rev-parse --short=7 HEAD` and the dirty flag, or `None` outside a git checkout. A linked worktree, where
    `.git` is a file, is still a checkout.
- **Same string everywhere:** each `version` field a verb prints equals the text `holler --version` prints after
  `holler `, for example `0.4.0 (bda1069)`. This covers `body query status`, `body caps`'s `status.version`,
  `hub query status`, and the live hub's `hub status` in both its `--json` form and its human first line
  (`hub 0.4.0 (bda1069) (protocol 3)`). The key is unchanged and no field is added.

## Tests authored

| Test | File | Criterion / behavior pinned | Tier, and why |
|---|---|---|---|
| `a_checkout_uses_the_head_hash` | `tests/build_version_test.rs` (new `[[test]]`) | AC 1, git case: the checkout's hash, which wins over `HOLLER_GIT_SHA` | unit (pure function); cheapest, and needs no fixture crate |
| `a_dirty_checkout_appends_dirty` | same | AC 2: `-dirty` suffix | unit |
| `no_checkout_reads_holler_git_sha` | same | AC 1, `HOLLER_GIT_SHA=abc1234` with no `.git` gives `abc1234` | unit |
| `neither_is_unknown` | same | AC 1, neither gives `unknown`; an empty `HOLLER_GIT_SHA` counts as unset | unit |
| `git_state_of_a_clean_checkout_is_the_short_head_hash` | same | AC 1: the 7-character hash of `HEAD`; untracked files are not dirty | unit on a throwaway temp repo; the only way to pin what the build script asks git without dirtying the real checkout |
| `git_state_of_a_changed_tracked_file_is_dirty` | same | AC 2: an unstaged or staged tracked edit is dirty | unit, temp repo |
| `git_state_of_a_linked_worktree_is_its_own_head` | same | finding 4: in a worktree, `.git` is a file and the build still gets `HEAD` | unit, temp repo plus `git worktree add` |
| `git_state_outside_a_checkout_is_none` | same | the boundary that leads to `HOLLER_GIT_SHA` or `unknown` | unit, temp dir. A guard, not a RED: it passes against a `None` stub by design and fails only if `git_state` reports a checkout where none exists |
| `version_flag_prints_version_and_commit` (replaces `version_flag_prints_crate_version`) | `tests/cli_invocation_test.rs` | Work 1 and AC 1 on the real binary: `--version` and `-V` print `holler <ver> (<HEAD short hash>[-dirty])` | process; the binary's output is the behavior. `-dirty` is allowed either way, because a worktree is often dirty partway through a phase (finding 5) |
| `status_docs_carry_version` (extended) | `tests/query_test.rs` | Work 3: every `version` field a verb prints equals the `--version` string | process with a real hub; the producers live in the hub and body processes |
| `top_level_help_names_pane_and_profile` (relaxed) | `tests/pane_verbs/process/stub.rs` | Work 1's invariant: the first two tokens stay `holler <ver>` | process. It is not a RED test. Its old exact match on `holler <ver>\n` would break by design, and the exact format is already pinned in `cli_invocation_test`, so it now checks only the tokens |

Manifest: one new `[[test]]` entry, `build_version_test`, in `crates/holler-cli/Cargo.toml`, because
`autotests = false`.

## RED confirmation

Run: `cargo test -p holler-cli --test cli_invocation_test --test query_test --test pane_cli_process --test build_version_test`, at HEAD `3a9151e`, git 2.53.0.

- `version_flag_prints_version_and_commit`, FAILED, on an assertion about the missing behavior:
  ```
  panicked at crates/holler-cli/tests/cli_invocation_test.rs:54:32:
  --version: want `holler 0.4.0 (<commit>)`, got "holler 0.4.0\n"
  ```
- `status_docs_carry_version`, FAILED, on an assertion about the missing behavior:
  ```
  panicked at crates/holler-cli/tests/query_test.rs:87:5:
  --version: "holler 0.4.0\n"
  ```
  If F changed only the clap line, the `version` field assertions would fail next, because the fields would still
  be `0.4.0`.
- `top_level_help_names_pane_and_profile` passes, as intended (see the table). The other 13 tests in
  `cli_invocation_test`, the other 28 in `query_test` and all 36 in `pane_cli_process` pass.
- `build_version_test` fails to compile, and the failure names exactly the missing contract file:
  ```
  error: couldn't read `crates/holler-cli/tests/../build_version.rs`: No such file or directory (os error 2)
    --> crates/holler-cli/tests/build_version_test.rs:22:1
  ```
  A pure function that does not exist yet can only be RED at compile time. The tests are in their own target, so
  this compile error hides no other test. To show that the tests themselves are sound, I ran them against two
  throwaway versions of `build_version.rs` and deleted each one straight after; neither is in the tree:
  - **A wrong stub** (`String::new()` and `None`): 7 of 8 tests failed on their assertions, at lines 36, 49, 57,
    65, 117, 135 and 157. All fixture git calls succeeded, so the failures come after setup. The guard
    `git_state_outside_a_checkout_is_none` passed, as designed.
  - **A minimal correct reference** (`git rev-parse --short=7 HEAD` and `git status --porcelain
    --untracked-files=no`): 8 of 8 passed, so the contract can be met, including the worktree case.

Also: `cargo clippy -p holler-cli --test cli_invocation_test --test query_test --test pane_cli_process -- -D warnings`
exits 0, and `bash scripts/lint.sh` exits 0. The new file has 176 lines; `cli_invocation_test.rs` has 423 and
`query_test.rs` has 585. The new `#![allow]` carries `// #851`.

## Ready for F
RED is valid, and F may implement against these tests. F must:
- create `crates/holler-cli/build_version.rs` with the two signatures above;
- add `crates/holler-cli/build.rs`, which uses the file and emits the version string;
- add the seam that A's finding 1 describes, so that the hub and body producers print the same string.

F writes no tests. If a test looks wrong, F routes it back to T.
