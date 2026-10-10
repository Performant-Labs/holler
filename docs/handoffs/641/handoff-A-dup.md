# Handoff-A-dup: Phase 7 - #641 host adapter (tmux sessions, process control, the launcher primitive)  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-641-implementation
**Diff base:** 3bdd129 (the merge base with `origin/main`)   **Diff head:** 23e358a
**Reuse map:** docs/handoffs/641-brief.md §Files "Reuse map"
**Verdict:** PASS

## Summary

PASS. F extended every object the reuse map named and built no parallel path.

The brief justified one new object in writing, `exec.rs`, and its consolidation issue is now on record (#696). The other new objects have no counterpart anywhere in the tree, on this branch or on current `main`: `Call`, `Pid`, `Window`, `Refusal`, the escape functions and the kill seam. F's two deviations stay inside `classify` and the crate docs, and they add no drift.

There is one warn, and it is test-only: `real_tmux_test.rs` has its own copies of three helpers that `tests/common/mod.rs` already defines.

## Reuse map, row by row

| Map row | Called for | What the diff does | Result |
|---|---|---|---|
| `holler_pane::HostPort` | implement as is | `impl HostPort for TmuxHost` (`src/lib.rs:309-392`) keeps the frozen signatures; `holler-pane` is untouched | extended |
| `run_host_conformance` | reuse, no copy of its cases | `run_host_conformance(private)` (`tests/real_tmux_test.rs:211`). The fake-tmux tests pin argument vectors and error mappings, a different tier, and copy no case | reused |
| `HostOp::as_str` names | reuse the strings, with a test pinning them equal | `OP_*` literals (`src/lib.rs:134-137`); `every_method_is_bounded_and_reaps_its_child` asserts `op == HostOp::as_str` (`tests/fake_tmux_test.rs:95`); the test kit stays a dev-dependency | reused |
| `PaneError`, `PaneName`, `Argv` | reuse, no new variant | Only `Usage`, `Timeout`, `PaneNotFound` and `Unavailable` are used. `PaneNotFound { what: name }` and the empty-argv message match `FakeHost` exactly (`holler-pane-testkit/src/host.rs:211-227`) | reused |
| the crate skeleton | fill | Filled behind T's stub surface. `Cargo.toml` depends on `holler-pane` only (ADR-0021 §5) | extended |
| `wiring.rs` (#649) | not touched | Not touched. The diff covers only `crates/holler-adapter-host/**`, `CHANGELOG.md`, `Cargo.lock` and this run's handoffs | n/a |
| `run_probe` | not reused; `exec.rs` is new and justified, with W-7's follow-up | `exec.rs` is private. `run_probe` is still #663's stub on current `main`. Follow-up #696 is filed and names `exec.rs` and the fake-`kill` seam | new, justified |
| an existing launcher | none in the repo | none added anywhere | n/a |

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-21 | warn | `crates/holler-adapter-host/tests/real_tmux_test.rs:123-129, 135-142` | `name`, `argv` and `alive` repeat `tests/common/mod.rs:197-204, 224-230`, and the two `alive` bodies are byte-identical. The file declares no `mod common;`. In the neighbor `holler-adapter-herdr`, every test file imports its `tests/common` and none redefines a helper. `alive` also carries a portability rule: signal 0 through the `kill` binary with `LC_ALL=C`, never `/proc`, for the macOS CI leg (W-19). Only the common copy's doc comment states it, so a fix to one copy can miss the other. This is test-only, in T's file, and involves no reuse-map object, so it is not a Phase 7 block. | In `real_tmux_test.rs`, add `mod common;` and `use common::{alive, argv, name};`, then delete the three local copies. `common/mod.rs` already opens with `#![allow(dead_code)] // #641`. The W-20 guard counts `TmuxHost::new` file by file, so both its counts stay at 1. If the separation is deliberate (so that AC 9 can audit the real-tmux file on its own), say so in that file's header instead. |

Production code has no duplication, and the extension is clean.

## Checked, no finding

- **No parallel path in production code.** Checked against this branch and against current `main` (e612878):
  - The only production code that spawns processes is `holler-load-test`, a harness binary with unbounded `output()` and `spawn()` calls.
  - The only `kill` calls are test-only `libc::kill` in `holler-cli/tests`.
  - `holler-pane` exports no bound constant, deadline helper, pid type or `PaneError` constructor. I5's 10 s appears in rustdoc only.
  - `Argv` exposes no program accessor and no assignment check. The `=` refusals in `argv.rs` concern env-var names and profile JSON.
  - No crate outside the adapter uses tmux vocabulary.

  So none of `Call`, `deadline_after`, `Pid`, `Window`, `Refusal`, `escape`, `escape_cwd`, `check_argv` and `kill_group` has a counterpart. The sibling adapters have no deadline pattern to match: `holler-adapter-herdr` is still its pure core, and `holler-adapter-opencode` is a skeleton.
- **The overlay's candidates.**
  - The adapter does not log, so `log(Severity, ...)` does not apply. `token.rs`, `Lockout` and `Roster` are untouched.
  - The only `wait_for` is in `holler-cli/tests/support/mod.rs`. That module is private to its crate, so this adapter cannot reach it, and ADR-0021 §5 forbids depending on `holler-cli` anyway. `wait_until` is the crate's own.
  - No other crate has a fake-binary writer like `common::write_script`.
- **F's deviations.**
  - `no such window` is one more string in the single `classify` (`src/tmux.rs:304-319`). F's probe evidences it, and T-green's case pins it.
  - Decision 15's qualified prose went into the crate docs.

  Neither adds a code path.
- **Layering inside the crate.** `lib.rs` orchestrates the port. `tmux.rs` is pure except for one `stat` in `is_session_dir`. `exec.rs` does all the process I/O. `exec.rs` takes `tmux::Pid` so that only a range-checked pid reaches `kill` (W-15); when #696 moves the runner, it should move that type with `kill_group`.
- **Patterns.**
  - `with_*(mut self) -> Self` with `#[must_use]`, as in `holler-hub/src/live.rs:453`.
  - Crate docs in the style of `holler-adapter-herdr`.
  - `#![allow(...)] // #641` on the test files, and `tests/common/mod.rs` opening with `#![allow(dead_code)] // #641`, as herdr's does with `// #640`.
- **The consumer that landed after the base.** On `main`, #647's reconcile engine reads `HostPort::ps` (`holler-pane/src/reconcile/observe.rs:133-146`): `PaneNotFound` becomes `tmux-session-missing`, and any other error becomes `observe-failed`.
  - The adapter's Decision 8 mapping fits this: a tmux server that is not running reads as a missing session, not as a failed check.
  - #647's `HOST_PS` is a private label for a message and is never matched against the adapter's `Timeout.op`. Each crate spells the op names as local literals, which is the dominant pattern.
- **Size and hygiene.**
  - The largest file, `tests/fake_tmux_test.rs`, is 753 lines, under the ~800 flag; `src/lib.rs` is 476. There is no `unsafe`.
  - The host name from the issue title appears in no diff line and no commit message. Every commit uses the GitHub no-reply identity.
  - Nothing outside the blast radius changed.

## Notes (not findings)

- **`CHANGELOG.md` conflicts with current `main`.** `git merge-tree HEAD origin/main` reports a content conflict. #647 (e612878, merged after the base) added its `[Unreleased]` entry at the same insertion point, after the #688 entry. The fix is to keep both entries, and the run must resolve it before it pushes. No code is involved.
- **#696 also covers #642.** It names #642's planned private tmux and `kill` calls. #642's own A-dup gate should check those against this crate's `exec.rs`, `kill_group` and `tmux.rs`. ADR-0021 §5 bars a dependency from one adapter on another, so any shared code goes through `holler-pane`, as #696 says.

## Notes for F

None (the verdict is PASS).

## Patterns referenced

- `docs/handoffs/641-brief.md`: §Files "Reuse map", and Decisions 1-15.
- `crates/holler-pane/src/{ports.rs,argv.rs,error.rs,probe.rs}` and `crates/holler-pane-testkit/src/{host.rs,conformance/host.rs}`.
- `crates/holler-adapter-herdr/{Cargo.toml,src/lib.rs,tests/common/mod.rs,tests/*_test.rs}`.
- `crates/holler-pane/src/reconcile/observe.rs` on `origin/main` (e612878).
- Issue #696, the follow-up for one shared bounded runner.
