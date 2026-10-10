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

## T (Phase 4, author tests / RED) — 2026-10-09T23:04:25-06:00
- **Decided:** RED is valid; F may start (see handoff-T-red.md).
  - Kept and reviewed the staged suite of the interrupted first T run: `tui_test.rs` (13 tests), `attach_test.rs`,
    `support/fake_tmux.rs`, the committed `fixtures/fake-tmux` (100755), `real_opencode_test.rs` and `real_opencode/rig.rs`
    (9 opt-in tests), AC 31's `on_refused_port` in `stub.rs` used by all seven of `hermetic_test.rs`'s refused-port
    tests, and the minimum public stubs in `src/tui.rs`.
  - Added `ac30c_a_tui_that_exits_because_its_session_went_away_is_session_not_found` and the helper `Stub::json_once`.
    The Behaviour section's death branch (re-`GET`; 404 means `session-not-found`) had no test.
  - RED: `tui_test` 0/13 pass; `attach_test` 1/22 (only AC 33's fixture check); `hermetic_test` 30/30. Opted-in real run
    (OpenCode 1.18.35, tmux 3.7c): 3/9. AC 12 fails on exactly the 8 TUI cases, and AC 13/14/15/18/19a fail on
    `NotImplemented`.
- **Assumed:**
  - The brief at 6c3809c is canonical. A's W-1 (`=<session>:^`) and W-8(a) (`escape`/`escape_cwd`) were not applied by O,
    so the tests pin `=<session>:` and `escape_arg`/`escape_dir`. An amendment changes only those strings and names.
  - AC 16, 17 and 19 pass before F because they exercise 642a's merged server side. They are regression pins, not RED.
- **Hedged:**
  - `ac30e` (`timeouts.call = 0`) is deterministic only if each tmux spawn checks the time left first. That is noted for F
    in the handoff, not worked around in the test.
  - I did not run concurrent repeats of the hermetic files now. The timing tests cannot reach their bounds before F's code
    exists, so that repeat belongs to T-green.
- **Evidence:** the RED outputs in handoff-T-red.md. Clippy `-D warnings` on the crate's targets is clean, rustfmt
  `--check` passes, `scripts/lint.sh` exits 0, and the AC 24 grep is empty. Leak check after the real run: no scratch dir,
  no `opencode` on 48100-48199, no private tmux server.

## F (Phase 5, implement against RED) — 2026-10-09T23:32:36-06:00
- **Decided:** GREEN (see handoff-F.md).
  - `tui.rs` holds the real builders and parsers under T's pinned names, plus `classify` (W-4).
  - The new private `attach.rs` holds the three bodies and one shared watch. `exec.rs` gains `capture` and shares one
    deadline loop with `kill`'s runner.
  - `lib.rs` gains `call_until` (W-2), `session_reply`, which `known` calls (W-5), and `poll`, shared by `settled` and the
    watch (W-6).
  - The docs follow AC 23 and 26, with W-3's fact-2 correction and a "Deferred" item for #695.
  - Two choices beyond the brief:
    - The watch confirms only a live attach to the requested port whose title shows the id.
    - `select_session` fails at once with "no TUI in pane P" when its watch finds a dead or missing pane.
- **Assumed:**
  - The brief at 6c3809c is canonical: `=<session>:` and `escape_arg`/`escape_dir`, because T's tests pin them. W-1 and
    W-8(a) stay follow-ups.
  - F does not rebase. AC 22 was read against the merge base `dc300ab`.
- **Hedged:**
  - The stale title after `respawn-pane` (probed on tmux 3.7c) means a re-attach of the session a pane already showed can
    be confirmed before the new TUI runs. The pinned call sequence of AC 30(a) leaves no room for a fix in F. It is
    reported with a proposed fix (chain `select-pane -T ''` into the respawn invocation) as Known issues 1.
  - One real-rig `serve` failure on a fresh port, in one of three full runs (case 15, "exited before it answered (exit
    status: 1)"), is attributed, as an unverified hypothesis, to a non-listening socket that holds a port in the ephemeral
    range. It is not the TUI code: it fails before any TUI call, and four further conformance runs passed.
- **Evidence:**
  - Hermetic: `tui_test` 13/13, `attach_test` 22/22, `hermetic_test` 30/30. Three concurrent runs were all green.
  - Real, against OpenCode 1.18.35 and tmux 3.7c: 9/9 twice, the second after the `poll` refactor, and AC 12 alone 2/2.
    Nothing leaked.
  - Workspace clippy `-D warnings` is clean, as are rustdoc `-D warnings` on the two crates, rustfmt on the changed files,
    `scripts/lint.sh`, `cargo machete` and the changelog check.
  - Probes, each on a private `-L` tmux server killed afterwards: a TAB in a start-command argument prints as `\t`, and a
    pane keeps its title across `respawn-pane -k`.

## T (Phase 6, verify GREEN + Tier 2) — 2026-10-09T23:45:08-06:00
- **Decided:**
  - GREEN, with no blocking issue (see handoff-T-green.md).
  - I added one test, `ac30_a_tui_of_another_server_showing_the_session_does_not_confirm_the_attach`. Without it, a
    mutation that removes F's port check from the watch passed the whole suite.
  - The real rig's `free_port()` now also requires the port to be bindable. This is F's item 2, the ephemeral-range
    flake.
  - No production code was touched.
- **Assumed:**
  - The stale title after `respawn-pane` (reproduced on tmux 3.7c) is a contract change for O and S, not a defect in
    F's code against this brief. The brief pins the respawn sequence (AC 11a, 30(a)), and nothing calls `attach_tui`
    until #644 and #649. So it is recorded as an advisory that must land before #644, not as a BLOCK.
  - The `body_run_test` failure on the fixed port 41918 is contention with other worktrees' suites, not this diff. It
    passed 10/10 twice once the port was free, and the full `--no-fail-fast` run passed. holler-cli reaches this diff
    only through `holler-pane`'s doc comments.
- **Hedged:**
  - The bindable check is hardening. Two clean real runs do not prove the 1-in-3 flake gone.
  - AC 22 was checked against the merge base `dc300ab`. It needs a re-check after the rebase onto `d9eabbb` (#708
    conflicts in `stub.rs` and `hermetic_test.rs`).
- **Evidence:**
  - Hermetic: 13 + 23 + 30, eight times in sequence and three times concurrently, all green.
  - Real, against OpenCode 1.18.35 and tmux 3.7c: 9/9 twice (55.0 s, 49.9 s). Nothing leaked.
  - Workspace: 1540 passed, 0 failed, exit 0.
  - Clippy, rustdoc, rustfmt, `lint.sh`, `machete` and the changelog check are clean. The AC 24 grep is empty.
  - Mutation table: 7 of 8 mutations were caught before the new test, and 8 of 8 after.
  - tmux probe on a private `-L` server: the title survives `respawn-pane -k`, and a chained `select-pane -T ''`
    clears it.
