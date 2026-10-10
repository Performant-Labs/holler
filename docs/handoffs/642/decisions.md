# Decisions — #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort`

## A (Phase 3, up-front plan review) — 2026-10-09T20:26:01-06:00
- **Decided:** PASS on docs/handoffs/642-brief.md at 6c3809c (sha256 `c2b09a81b3191e37...`), with 0 blocks and 9 warns
  (see handoff-A.md).
  - The objects and layers are right:
    - a private `attach.rs` behind one-line delegations, as `serve` delegates to `server.rs`;
    - pure tmux builders and parsers in `tui.rs`;
    - HTTP through `Call` and `known`;
    - no new dependency, code or signature;
    - ADR-0021 edited in the same change.
  - W-1 (the most consequential): `=<session>:` is the session's current window. With a window or split a person opened
    current, `attach_tui` kills its program, and that program can be a #641-owned `command`. The fix is `=<session>:^`, or
    a start-command guard in `attach_tui`, decided before T-red.
  - W-2: `select_session` needs one deadline from entry, but `OpenCodeHarness::call` restarts the clock when called after
    the tmux query.
  - W-3: the ADR note's fact 2 credits #641 with stopping the harness server. #641 as merged does not; #695 is open.
  - W-4: `classify` belongs in `tui.rs`, not in the generic runner, matching #641's `tmux.rs` and ready for #696.
  - W-5: strike the "same rules inline" option for `attach_tui`'s existence check.
  - W-6: one poll loop for the two TUI polls, reusing `settled`'s shape and `SETTLE_POLL`.
  - W-7: #641 merged at `e327569` (20:06 MDT). The dependency row and the fixture's "no tmux double exists" are stale, a
    rebase is needed before the diff gate, and the two fake record formats go on #696.
  - W-8: the escape names do not match #641's as E8 claims, and `tui` needs `http`'s "not an interface" sentence.
  - W-9: under `Inherit` the TUI gets the tmux server's environment, and a bare `opencode_bin` can resolve to a different
    binary than `serve`'s.
- **Assumed:**
  - The merged code at `dc300ab` (this crate) and `e327569` (#641) is the pattern baseline.
  - #644's brief at `7195993`, in its own worktree, is that story's current plan: launch runs `ensure_session`, then an
    optional `run`, and later `attach_tui`. It is evidence of the plan, not a pattern.
  - tmux's target resolution is the same on CI's tmux (macOS leg) as on tmux 3.7c here. The hermetic tests never run tmux,
    so only T-green's real run depends on it.
- **Hedged:**
  - W-1 is a warn, not a block. It needs a person's window change in the pane's tmux session together with an
    `attach_tui`. `select_session`, `shown_session` and reconcile's repair all fail closed. The fix is contained in
    `exact_target`, or one query in `attach_tui`. It is the item most worth amending before T-red.
  - W-2 is a warn because the brief's General rule (a deadline at entry) already decides the conflict, and F can meet it
    without leaving the brief.
  - W-3 is a warn because the text is F's to write in this run, and the correction keeps the fact's meaning.
  - W-4, W-6 and W-8 concern mirrors of one sibling. There is no dominant pattern beyond #641, so per the role rule they
    are warns.
  - I did not verify that tmux escapes a TAB inside `#{pane_start_command}`. If it did not, Decision 23's
    "title last, kept whole" would mis-split; T-green's real run would show it. It is not an architecture item.
- **Evidence:**
  - Read in full: the brief; its two outside-review results (r1, r2);
    `crates/holler-adapter-opencode/src/{lib,exec,tui,server,http}.rs`; `tests/{hermetic_test.rs,support/stub.rs}`;
    `crates/holler-pane-testkit/src/{conformance/harness.rs,fault.rs}`.
  - Also read: `harness.rs:1-140` (`FakeHarness`, `HarnessOp`); `holler-pane/src/{probe,reconcile}.rs` and the doc lines
    of `ports.rs` and `lib.rs`; ADR-0021 §2, §4, §5 and "Deferred".
  - #641 at `e327569`: `crates/holler-adapter-host/src/{exec,tmux}.rs`, `lib.rs` (docs and the `run_tmux`/`Call` shape),
    `tests/common/mod.rs` and the heads of `real_tmux_test.rs` and `fake_tmux_test.rs`. Also
    `crates/holler-adapter-herdr/src/lib.rs`, `scripts/lint.sh`, the workspace lints and `CHANGELOG.md` (at both bases).
  - The 642a handoffs at `d6393b2` (A, A-dup) and the 642a brief at `d6393b2` (B-1).
  - Issues #642, #695, #696 (with its comments, of which there are none), #700, #649, #667 and #663. PR #706: merged
    2026-10-09 at 20:06:15 MDT.
  - `git diff f2ea297 e327569 -- crates/holler-adapter-host` is empty. `git ls-files` shows `tests/fixtures/` against
    `tests/fixture/`, and no `100755` file under `crates/`.
  - Probes on tmux 3.7c, each on a private `-L` server killed afterwards (never the default socket):
    - `=demo-c1r1:` follows `select-window`, and `:^` stays on window 0.
    - `set-option -p` and `respawn-pane -k` land on the current window's pane.
    - `respawn-pane -k -t '=demo-c1r1:^'` exits 0.
    - A commandless session's `#{pane_start_command}` is empty.
    - A respawned pane gets the server's environment, not the client's.
