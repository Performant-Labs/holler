# Handoff-F: Phase 5 - #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort`

**Date:** 2026-10-09
**Branch:** issue-642-implementation at 6bcaa23, with this phase's changes uncommitted (base `dc300ab`; not rebased:
`origin/main` is at `d9eabbb`, see Known issues 4)
**Issue:** #642, part 2 of 3 (epic #633)

| Field | Value |
|-------|-------|
| GitHub issue | #642 (642b only; 642c applies the agent after #700) |
| Working branch | `issue-642-implementation` |
| Build plan phase | epic #633, wave 3 |
| Input documents read | the brief (all 1,295 lines), `handoff-A.md`, `handoff-T-red.md`, `decisions.md`; the crate as merged; #641's `exec.rs`, `tmux.rs`, `lib.rs` at `origin/main`; the test kit's `harness.rs` and `conformance/harness.rs`; every test file of the crate; ADR-0021 §2 and "Deferred" |
| Acceptance criteria count | 642b's: AC 6 and 11d (642b clauses), 9, 10, 11a-11c, 11f, 12-19a, 20-26, 27-33 |
| Handoff document path | `docs/handoffs/642/handoff-F.md` |

A Workflow run has no human to confirm the table with, so it is recorded here and the work went ahead (memory "Decide,
don't ask"). There was no scope split: the brief is already the 642a/642b/642c split, sized by O and passed by A.

## What was done

- `crates/holler-adapter-opencode/src/tui.rs`: T's RED stubs are replaced by the real pure builders and parsers of the
  brief's API, under the same names and signatures (`tmux_command`, `exact_target`, `escape_arg`, `escape_dir`,
  `tui_argv`, `respawn_args`, `remain_on_exit_args`, `query_args`, `QUERY_FORMAT`, `Query`, `parse_query`, `TitleShows`,
  `parse_title`, `attach_port`). The tmux stderr rule (`classify` and its two-way `Refusal`, `pub(crate)`) moved here per
  A's W-4. The module docs now say the builders are public for tests and are not an interface (W-8(b)).
- `crates/holler-adapter-opencode/src/attach.rs` (**new, private**): the three method bodies, a `Tui` helper (the
  resolved tmux session, the method's `op` and deadline, the one tmux runner, the query, a pane-changing call) and
  `watch`, the one poll that both `attach_tui` and `select_session` use.
- `crates/holler-adapter-opencode/src/exec.rs`: gains `capture` (stdout and stderr drained on two threads, 64 KiB kept of
  each, nothing started once the deadline has passed) and `Ran::reason`. `run` (the `kill` helper's runner) now shares
  one deadline loop, `wait`, with `capture`. `kill_group` is unchanged. The header now describes the module and no longer
  points at the brief.
- `crates/holler-adapter-opencode/src/lib.rs`: `mod attach`, the three `op` constants, the `SELECT` route,
  `OpenCodeHarness::call_until` (which `call` delegates to; W-2), `session_reply` (which `known` calls; W-5), and `poll`,
  one poll loop shared by `abort`'s `settled` and the TUI watch (W-6). The three methods delegate in one line each. The
  crate docs say part 2 is built and 642c is next. They also cite the ADR note's facts instead of the brief's decision
  numbers, and they document decision 15(b), W-3's attribution and W-9's environment and binary rules.
- `CHANGELOG.md`: the part-1 sentence AC 23 names is removed, and the part-2 Enhancements entry is added.
- `docs/adr/ADR-0021.md`: the §2 paragraph's first sentence is rewritten, the note "`HarnessPort` as built (#642)" is
  added with its seven facts (fact 2 as A's W-3 corrected it), and the "Deferred" item becomes three items (AC 26 and W-3).
- `crates/holler-pane/src/ports.rs`, `crates/holler-pane/src/lib.rs`: doc comments only (AC 26).

## Design decisions

1. **The watch confirms by port as well as by title.** `Seen::Shown` needs a live pane whose start command is an attach
   to *the requested port* and whose title shows the id. The brief asks only for the title, "as `shown_session` reads
   it". The port is in the same query and costs nothing. Two servers share one data directory, so a TUI attached to
   another port can show the same id. That should not count as this attach or this switch (I3).
2. **A dead or missing pane during `select_session`'s watch fails at once** with the pre-check's own answer
   (`unavailable`, "no TUI in pane P"), not after waiting out `settle`. The brief defines the death branch for
   `attach_tui` only. For `select_session` it says to poll until the title shows the id, so the only alternative was a
   timeout that is already certain. Both answers are failures (exit 1).
3. **The TUI argv is built before the resolver.** So an `opencode_bin` that is not valid UTF-8 is `unavailable` before
   anything is resolved or touched. The brief places that check nowhere.
4. **`classify` returns no text, and `Ran::reason` does** (the first non-empty stderr line, or the exit status when there
   is none). This mirrors #641, whose `Refusal` carries no text and whose `Ran::reason` gives it. With stderr empty, the
   brief's `Other(first line)` would have produced a message ending in nothing.
5. **The death branch's re-`GET` is `known` itself.** `session-not-found` passes through. Any other outcome (the session
   is still there, the server is refused, the deadline has passed) reports the exit, which is the fact that was observed.
6. **`poll` replaces `settled`'s loop instead of copying it.** The order (step, then check, then sleep) and the bound
   (`settle` or the method's deadline, whichever ends first) are unchanged, and `settled` keeps its signature. All of
   642a's abort tests pass, and so does AC 17 against real OpenCode.
7. **Messages.** They are "no tmux session for pane P", "no TUI in pane P", "the TUI in pane P exited with status N" (or
   "... exited, with no exit status"), "tmux failed for pane P: <tmux's first stderr line>" and "cannot run <tmux_bin>:
   <error>". The pane id goes through `one_line`. No message holds an argument, a directory, an env value, the format, a
   target or a raw title (Risk 3).
8. **Bounded capture.** 64 KiB of each output is kept and the rest is read and dropped, so a chatty child never blocks
   on a full pipe. The drain threads come from `thread::Builder`, so a failed spawn is `unavailable`, not a panic.

## Reuse / extend-vs-new

- **Extended, per the brief's Reuse map and A's warns:**
  - `known`, through the sibling `session_reply`, which it calls (W-5).
  - `OpenCodeHarness::call`, through `call_until` (W-2).
  - Reused as is: `Call::{send, send_by, unexpected}`, `session_path`, `json_of`, `string_at`, `one_line` and
    `deadline_after`.
  - `SETTLE_POLL`, its doc widened. `settled`'s loop, now the shared `poll` (W-6).
  - `exec::run`, now sharing `wait` with its capturing sibling.
- **New, as the brief justified:** `src/attach.rs` (Decision 17; 642a A's N-2). `lib.rs` is 555 lines with it, and would
  be near 800 without it.
- **Mirrored, not shared:** `escape_arg`/`escape_dir`/`classify` follow #641's `escape`/`escape_cwd`/`classify` by rule.
  The names stay the brief's, because T's tests import them (A's W-8(a) was not applied before T-red). One shared home is
  #696.

## Architecture notes for A

- One new private module (`attach`) behind one-line delegations, the same layering as `server`.
- New public items in `pub mod tui` (the brief's API, pure, documented as not an interface), plus `pub(crate)`
  `classify`/`Refusal`.
- New `pub(crate)` `exec::capture` and `exec::Ran`.
- No new dependency, no change to `HarnessPort` or `PaneError`, no new code.
- `holler-pane`: `//!`/`///` lines only. No change to `http.rs`, `server.rs`, the test kit or the manifests.
- Every tmux process comes from `tui::tmux_command` (one call site: `Tui::run`), and every `-t` from `exact_target`
  (Risk 6).
- `archChanged: true` (a new module boundary and new public items).

## Deviations from spec / wireframe

- Design decisions 1 and 2 above (the watch's port check; `select_session` failing at once on a dead or missing pane).
- `poll` refactors `settled` (decision 6). The brief does not name it; A's W-6 asked for `settled`'s rule to be reused.
- In the ADR note, the facts carry no "(decision N)" labels: they point at the 642a brief, which is not on `main`. The
  crate docs cite the note's fact numbers instead (AC 26's repointing).
- A's W-3 is applied: fact 2 now attributes the stop to #695, and "Stopping the harness server that `serve` started:
  #695" is a third "Deferred" item.
- Not applied, because T's tests pin the brief as written: W-1 (`=<session>:^`) and W-8(a) (`escape`/`escape_cwd`).

## Tier 1 self-check (incl. tests now GREEN)

All builds ran with `CARGO_BUILD_JOBS=4`, on Linux, rustc 1.98.1.

```
cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings    clean
cargo clippy --workspace --all-targets -- -D warnings                  clean
cargo test -p holler-adapter-opencode
  tui_test       13 passed; 0 failed          (RED was 0/13)
  attach_test    22 passed; 0 failed          (RED was 1/22)
  hermetic_test  30 passed; 0 failed          (642a's, unchanged)
  real_opencode_test  0 passed; 9 ignored     (not opted in)
3 concurrent runs of tui_test + attach_test + hermetic_test: all green, every time
cargo test -p holler-pane                                               all green (doc-only change)
RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-opencode -p holler-pane --no-deps   clean
rustfmt --check --edition 2021 crates/holler-adapter-opencode/src/*.rs crates/holler-pane/src/{ports,lib}.rs   ok
bash scripts/lint.sh                                                     exit 0 (warns only; nothing of mine >= 600)
cargo machete                                                            no unused dependencies
bash scripts/changelog-check.sh                                          ok
grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode          nothing
```

Opted in, against OpenCode 1.18.35 (`/usr/local/bin/opencode`) and tmux 3.7c:
`HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1`

| Run | Result |
|---|---|
| 1 | 8 passed, 1 failed (AC 12: 14 of 15 cases; see Known issues 3) |
| AC 12 alone, twice | passed both times: all 15 conformance cases |
| 2 | 9 passed in 61.8 s |
| 3 (after the `poll` refactor) | 9 passed in 57.9 s |

After each run nothing was left: no `/tmp/hlr642r-*`, no `opencode` beyond the live fleet's own two servers (left
untouched), and no private tmux server. `cargo fmt --all -- --check` reports diffs only in `holler-body` files that this
run does not touch, so they predate it.

## Evidence appendix

`docs/handoffs/642/evidence.md`: seven entries (the HTTP client's zero-budget timeout and refusal, `Call::send_by`'s
mapping, `Call::unexpected`, `json_of`, `one_line`, and the pane-name grammar).

## Tests that look wrong (for T)

No test is wrong against the brief. Three things for T:

1. **AC 30(a) pins the exact attach sequence** (`set-option`, `respawn-pane`, then only queries), and AC 11a pins
   `respawn_args`' vector. Both leave no room for the title reset that Known issues 1 needs. The fix changes those pins
   together with the brief.
2. **The real rig's ports** (Known issues 3): `free_port()` and `refused()` see only a listener. In the conformance path
   there is no retry for a `serve` that fails on a port something else holds.
3. **Rebase conflicts in T's files** (Known issues 4): `tests/support/stub.rs` and `tests/hermetic_test.rs` conflict with
   #708 on `main`.

## Known issues

1. **A respawn keeps the pane's old title** (probed on tmux 3.7c, on a private `-L` server killed afterwards): after
   `select-pane -T 'OC | ses_X'` and then `respawn-pane -k`, `#{pane_title}` still reads `OC | ses_X`. The pane's start
   command changes at the respawn. So when `attach_tui` re-attaches a pane to the session its old TUI showed (#644's
   relaunch, or a relaunch onto a new port), the first poll sees a live attach to the requested port with a matching title
   and answers `Ok` before the new TUI has started. I3 then holds only if the new TUI comes up. A fresh pane is not
   affected: its title is the host name, which the brief's Risk 1 assumed every respawned pane shows. Fix (a contract
   change for O and T): chain a title reset into the respawn's own tmux invocation, `respawn-pane ... -- <argv> \;
   select-pane -t =<name>: -T ''`. respawn gives the pane a new pty, so after it no output of the old process can reach the
   pane, and any title seen afterwards was set by the new TUI. This must land before #644's relaunch relies on
   `attach_tui`.
2. **A's W-1 stands:** `=<name>:` is the session's current window. A window or split that a person opened, if current,
   is what `attach_tui` would respawn. The crate docs, the `tui` docs and the ADR's fact 1 now say which pane is the TUI's.
   The fix (`=<name>:^`, or a start-command guard) changes T's pins, and A says it must land before #644's relaunch runs
   against real tmux.
3. **One real-rig flake in three full runs, not reproduced in four more conformance runs.** In case 15
   (`select-reaches-only-its-pane`), the first `serve` failed: "the harness server for demo-c1r1 on port 48110 exited
   before it answered (exit status: 1)". The port was fresh, and no TUI call had been made in that case. My hypothesis is
   unverified, because the server's stderr is null. 48100-48199 lies inside Linux's ephemeral range (E10), so a socket
   that is not listening (a client end, or `TIME_WAIT`) can hold a port that `refused()` reports free, and `opencode serve`
   then cannot bind it. This is 642a's `serve` path and T's rig, not the TUI code.
4. **The branch is not rebased.** `origin/main` (`d9eabbb`) has, since `dc300ab`:
   - #708, which rewrote `tests/support/stub.rs`'s `closed_port` (a held connection) and one line of
     `tests/hermetic_test.rs`. That conflicts with T's AC 31 rewrite of the same helper.
   - ADR-0021 edits by #663 and #646, none on the lines this run edits.
   - New CHANGELOG entries at the end of `[Unreleased]` Enhancements, where A's W-7(c) predicted the conflict with the
     part-2 entry.

   AC 22 above was checked against the merge base.
5. Unchanged and still the brief's follow-ups: the `FakeHarness` parity issue (P-3), the macOS socket-timeout tolerance
   of `http.rs` (P-2), the comment fixes in `http.rs` and `server.rs` (642a's A-dup D-1, which are outside this run's
   files), and the #696 notes (W-7(b), D-2).

## Files changed

Production and docs (F):

- `crates/holler-adapter-opencode/src/attach.rs` (new)
- `crates/holler-adapter-opencode/src/exec.rs`
- `crates/holler-adapter-opencode/src/lib.rs`
- `crates/holler-adapter-opencode/src/tui.rs`
- `CHANGELOG.md`
- `docs/adr/ADR-0021.md`
- `crates/holler-pane/src/ports.rs` (doc comments only)
- `crates/holler-pane/src/lib.rs` (doc comments only)

Handoff files: `docs/handoffs/642/handoff-F.md`, `docs/handoffs/642/evidence.md`, `docs/handoffs/642/decisions.md`
(appended).
