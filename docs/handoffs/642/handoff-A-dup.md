# Handoff-A-dup: Phase 7 - #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort`  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-642-implementation (not rebased; `origin/main` is at `d9eabbb`)
**Diff base:** `dc300ab` (the merge base)   **Diff head:** `ff48da5` (code as of `5d20f61`, T-green)
**Reuse map:** docs/handoffs/642-brief.md, "Reuse map (extend the 642a objects; do not duplicate)" (lines 1121-1139), and the A-dup checks that handoff-A.md's W-2, W-4, W-5 and W-6 handed to this gate
**Verdict:** PASS

## Summary

PASS, with 0 blocks and 3 warns. F extended every object the Reuse map named and built no parallel path:

- `known` now calls a new sibling, `session_reply`, so its rules (400 or 404 is `session-not-found`, a mismatched id is
  `unavailable`) are written once. `attach_tui` reads `directory` from what `session_reply` returns.
- `call` delegates to the new `call_until`, and `select_session` builds its `Call` from the deadline it took at entry.
  No `call` comes after the tmux query.
- `settled`'s loop is now the crate's one `poll`, which `abort` and the TUI watch share, with the one `SETTLE_POLL`.
- `exec::run` shares its deadline loop (`wait`, `reap`) with the new capturing `capture`, and `kill_group` is unchanged.
- `classify` lives in `tui.rs`, so the runner knows nothing of tmux.

The three method bodies live in the private `attach.rs` behind one-line delegations, as `serve` delegates to `server.rs`.
Every new object is one the brief justified. None of the three warns is about F's production code. They concern the
rebase onto #708, the record #696 still lacks, and one comment in the real rig.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-1 | warn | `crates/holler-adapter-opencode/tests/support/stub.rs:173` (`closed_port`), `:203` (`on_refused_port`) | **After the rebase, two refused-port helpers would exist.** #708 (`3107487`, merged 20:34 MDT, after this brief was written at 20:05 MDT) fixed the same flake on `main` differently. It turned `closed_port()` into the client end of a held connection, which never had a listener and stays in use for the life of the process, and kept it `pub`. This branch keeps bind-then-drop as a private candidate and adds `on_refused_port`, a check-and-retry wrapper, as AC 31 requires. Against the merge base there is one helper, extended in the file the map named. Against `main` the two collide in `stub.rs` (body, docs, module header) and in `hermetic_test.rs`'s header line 3. A resolution that keeps both public leaves two entry points for one job, which breaks AC 31's "one helper". The rig's `refused()` (`real_opencode/rig.rs:99`) is exactly `!stub::accepts()`. It lives in another test binary, and the rig needs its own picker for the 48100-48199 range anyway. | At the rebase, as T-green's Advisory 3 says: keep `on_refused_port` as the only public entry point. Give the private `closed_port` #708's held-connection body and its explanation, so a candidate can no longer be handed to a later `bind(0)`, and the retry becomes a backstop. Keep one description of the mechanism in `stub.rs`'s module docs and in `hermetic_test.rs`'s header. Check afterwards that `grep -rn "closed_port" crates/holler-adapter-opencode/tests` finds only `stub.rs`'s own definition and its one call. Leave the rig's `refused()` as it is. |
| W-2 | warn | `crates/holler-adapter-opencode/src/exec.rs:90` (`capture`); `src/tui.rs:139-159, 286-307` | **#696 lacks the record of what it will merge, and there are now three runners.** `capture` is the third copy of the drain-collect-wait-reap runner. The brief justified the mirror (ADR-0021 §5; the map's "Mirrored, not shared"), and #663's `run_bounded` (`d9eabbb`, merged after this branch's base) is private and probe-specific, so there was nothing to extend, and AC 22 bars `holler-pane` code changes anyway. But #696 has no comments (checked 23:58 MDT). Phase 3's W-7(b) note on the two fake record formats, W-8(a)'s name mapping, and 642a A-dup's D-2 are still unrecorded. | O comments on #696 with the differences. **Runners:** the host's `exec::run` keeps all output, polls every 2 ms, answers `Failed::{Expired, Unavailable}` and does not one-line `Ran::reason`. Its `kill_group` takes a configurable `kill` path, sets `LC_ALL=C`, sends TERM, KILL or 0, and reads `No such process` as an empty group. This crate's `capture` keeps 64 KiB of each stream and drains the rest to a sink, polls every 5 ms, answers `PaneError` directly and one-lines `Ran::reason`. Its `kill_group` sends KILL only, uses `kill` from `PATH` and refuses a pgid of 1 or less. `holler-pane`'s `run_bounded` reads stdout only (more than 1 MiB is an error), discards stderr, runs the child in its own process group, kills the group on the deadline, never signals after the leader is reaped, and polls every 10 ms. **tmux plumbing:** #641's `escape`, `escape_cwd` and `window_target` are `escape_arg`, `escape_dir` and `exact_target` here. #641's `classify` has four outcomes (with `WindowGone` and `Duplicate`); this one has two, and also reads `can't find pane` and `no such pane` as missing. #641 builds its tmux `Command` inline in `run_tmux`; this crate builds it in `tui::tmux_command`. #641's fake writes one argument per line and then `-=END=-`; this fixture writes a U+001F after each argument. |
| W-3 | warn | `crates/holler-adapter-opencode/tests/real_opencode/rig.rs:138` (`tmux`), `:356` (`pane_format`) | **The rig's raw tmux helper repeats `tui::tmux_command`** (the program, `-S <sock>`, the arguments, and `TMUX` and `TMUX_PANE` removed), and `pane_format` writes `={session}:` itself instead of calling `tui::exact_target`. That is right: this helper runs the guard's `kill-server` and gives AC 14, 18 and 19a their raw readings, so it must not depend on the code under test. A regression in `tmux_command` that dropped `-S` would otherwise point the guard's `kill-server` at the operator's default tmux server. But nothing in the code says so, and #667 may lift the rig, so a later clean-up could fold it into `tmux_command`. | Add one comment line on `tmux()`: it is deliberately not `tui::tmux_command`, because the guard and the raw readings must stay independent of the adapter under test. Do it at the rebase or in 642c. No issue needed. |

No duplication in the production diff; the extension is clean. No architectural drift came in during rework: T-green
changed only test files (one test in `attach_test.rs`, `bindable` in the rig), and F's `poll` refactor is the one W-6 asked
for.

### Checked, no finding

- **The checks Phase 3 handed to this gate:**
  - W-2: `call_until` is the deadline-taking constructor, and `call` delegates to it (`lib.rs:222-236`).
  - W-4: `classify` and `Refusal` are in `tui.rs`, and the header of `exec.rs` says it knows nothing of tmux.
  - W-5: `session_reply` is the one sibling, and `known` is `session_reply(..).map(drop)` (`lib.rs:426-443`). The
    `POST /tui/select-session` 404 rule (`attach.rs:84-88`) belongs to another route, as the brief specifies.
  - W-6: one settle loop (`poll`, `lib.rs:462-478`) and one interval (`SETTLE_POLL`). The loops left in `exec.rs` (a
    child's exit), `server.rs` (the boot poll, unchanged under AC 22) and `http.rs` (reads) do other jobs.
- **Risk 6:** the only tmux `Command::new` is in `tui::tmux_command` (`tui.rs:122`), and `Tui::run` is its one caller.
  All three `"-t"` in `src/` are followed by `exact_target`.
- **Reading the pane, once:** `showing` is the one reading of a query, shared by `shown_session` and the watch. The
  death branch's re-`GET` is `known` itself. `select_session` repeats the step from a live pane to `attach_port` because
  it needs the port even when the title is not an id (the home screen). That is two lines, and the compiler flags both
  sites if `Query::Live` ever changes (Decision 4's fallback).
- **Mirrors of #641,** which an adapter cannot depend on (ADR-0021 §5). `tui.rs`'s escapes, target and `classify`
  follow #641's by rule, and its module docs name #641's names. `exec::capture` follows #641's `exec::run` line for line.
  The brief justified both in writing, so per the role's rule they pass. W-2 asks only for the record.
- **New objects, all justified in the brief:**
  - `attach.rs` (Decision 17). With it, `lib.rs` is 555 lines.
  - The committed fixture. #641's `Fake` builds `TmuxHost` and writes its scripts at test time, which AC 32 forbids.
  - `support/fake_tmux.rs`, the split the brief planned for when `attach_test.rs` nears 700 lines (it is 678).
  - The real rig (E11). The test kit has no recording wrapper, port picker or tmux helper it could have reused.
- **Test helpers:**
  - `stub.rs` is extended in place (`json_once`, `on_refused_port`), and `attach_test.rs` includes it through `#[path]`
    with the brief's `#[allow(dead_code)] // #642`.
  - Each test binary keeps its own small assertion helpers (`unavailable`, `quick`, `strings`, `SLACK`), as the Herdr
    adapter does with `unavailable_what` in three files. The two `quick()` carry different bounds.
  - The `FORMAT` literals in `tui_test.rs` and `attach_test.rs` are independent expected values for `QUERY_FORMAT`, not
    copies of it.
  - `json_once` repeats one `Canned` literal from `set`. That is a nit, not a parallel path.
- **Nothing else repeated:** no other crate reads an OpenCode TUI title or an attach command line. `holler-body`'s async
  `check_exists` is a client of another crate, which 642a already kept separate.
- **Scope and size:**
  - No new runtime dependency. `tempfile` is added as a dev-dependency, with its use named in the manifest.
  - No change to `http.rs`, `server.rs`, the test kit or the workspace manifest. `holler-pane` changes `//!` and `///`
    lines only.
  - `hermetic_test.rs` still has 30 tests (30 at the base) and is 798 lines, so 642c's tests go in a new file.
    `attach_test.rs` is 678 lines.

### Not re-flagged here (not duplication; already recorded for O and S)

- Phase 3's W-1: `=<name>:` is the session's current window.
- The stale title after `respawn-pane` (F's Known issues 1, T-green's Advisory 1).
- The rebase itself (T-green's Advisory 3), apart from W-1 above.

### Noticed, out of scope

`docs/handoffs/642-diff-result-r1.md` (gitignored) still holds the gate's refusal. The hand rerun's review and its
`usage.json`, which the O entry in `decisions.md` cites as evidence, are not in this worktree. S or O should make sure that
evidence exists before journalling the gate as run.

## Notes for F

None: the verdict is PASS.

## Notes for O

- W-1 applies at the rebase, which also has to settle the conflicts in `CHANGELOG.md` and ADR-0021 (T-green's
  Advisory 3). If the rebase changes `stub.rs` beyond W-1's fix, re-run the hermetic files.
- W-2 is a comment on #696, which also clears Phase 3's open W-7(b) and W-8(a) notes and 642a A-dup's D-2.
- W-3 is one comment line, at the rebase or in 642c.

## Patterns referenced

- `crates/holler-adapter-opencode/src/{lib,attach,exec,tui,server,http}.rs` at `ff48da5`, and `lib.rs`, `exec.rs` and
  `tui.rs` at `dc300ab`.
- `crates/holler-adapter-host/src/{exec,tmux,lib}.rs` at `origin/main` (`d9eabbb`).
- `crates/holler-pane/src/probe.rs` at `d9eabbb` (#663's `run_bounded`).
- `tests/support/stub.rs` at `3107487` (#708).
- `crates/holler-pane-testkit/src/` (no reusable rig helpers); the Herdr adapter's tests (per-file helpers).
