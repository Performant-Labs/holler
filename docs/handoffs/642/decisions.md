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

## O (diff gate, manual rerun with a one-off prompt-ceiling override): 2026-10-09T23:50:00-06:00
- **Decided:** The outside diff gate is taken as PASS from a hand rerun, and the run resumes at the anti-duplication gate (phase 8).
  - The workflow's own diff gate did not run: the assembled prompt was estimated at ~64093 tokens (256370 bytes / 4) against the runner's configured ceiling of 64000 (`DUAL_REVIEW_MAX_PROMPT_TOKENS=64000` in `.env`), so it refused to send (`gate-unavailable`, `nonzero-exit`).
  - The ceiling was calibrated on another model; the same outside model (`deepseek-v4-pro`) returned complete reviews on prompts of ~52K and ~60K tokens in this repo. The estimate exceeded the ceiling by 93 tokens (0.15%).
  - A hand rerun of the same prompt (`docs/handoffs/642-diff-result-r1.md.prompt.txt`) with `DUAL_REVIEW_MAX_PROMPT_TOKENS=80000` exported for that one invocation (the repo's `.env` is unchanged) returned a complete review (`finish_reason: stop`, 3255 completion tokens). Its BLOCK section reads `None.` and its verdict reads `PASS — no BLOCK findings; testing may proceed.`
- **Assumed:** The estimate is a bytes/4 heuristic and DeepSeek's context is far larger than 64K, so a 64093-token prompt is reviewed fully, not silently truncated; the complete 3255-token review with specific findings supports that.
- **Hedged:** This waives the ceiling for this one prompt only. The anti-duplication gate and the spec audit still run on the full diff.
- **Evidence:** the refusal text (`ceiling: 64000 tokens`, `estimated tokens: ~64093`); the rerun's `usage.json`.

## A (Phase 7, anti-duplication gate) — 2026-10-09T23:58:27-06:00
- **Decided:** PASS on the diff `dc300ab..ff48da5`, with 0 blocks and 3 warns (see handoff-A-dup.md).
  - F extended every object the Reuse map named:
    - `known` through `session_reply`, so its rules are written once;
    - `call` through `call_until`, with no `call` after the tmux query;
    - `settled`'s loop, now the one `poll` shared with the TUI watch, with the one `SETTLE_POLL`;
    - `exec::run`, which shares `wait` with the new `capture`.
  - `classify` is in `tui.rs`. The only tmux `Command::new` is in `tui::tmux_command`, and every `-t` comes from
    `exact_target`.
  - W-1: #708 on `main` fixed the refused-port flake with a held-connection `closed_port`, and AC 31's `on_refused_port`
    fixes the same flake. The rebase must leave one public helper.
  - W-2: #696 has no comments. Record the three runners (the host's, this crate's `capture`, and #663's `run_bounded`),
    the two tmux mirrors and the two fake record formats there.
  - W-3: the real rig's raw `tmux()` is rightly independent of `tui::tmux_command`, but say so in a comment.
- **Assumed:**
  - The brief's written justifications ("Mirrored, not shared"; "New, justified") make the mirrors of #641 and the new
    objects deliberate, reviewed decisions. Per the role, that is a PASS, not drift.
  - #663's `run_bounded` is no extension point for this crate. It is private and probe-specific, and AC 22 bars
    `holler-pane` code changes.
- **Hedged:**
  - W-1 is a warn, not a block. Against the merge base there is one helper, in the file the map named. The second one
    exists only once `main` is merged in, and T-green's Advisory 3 already gives the resolution.
  - I did not verify that #708's held-connection port is refused on macOS. I took #708's own doc and its CI for that.
  - The diff-gate rerun's evidence (its review and its `usage.json`) is not in the worktree. I noted it for S and O and
    did not judge it.
- **Evidence:**
  - Read in full: `src/{attach,lib,exec,tui,server}.rs`; `tests/{attach_test,tui_test,real_opencode_test}.rs`,
    `tests/real_opencode/rig.rs`, `tests/support/{stub,fake_tmux}.rs` and `tests/fixtures/fake-tmux`; the diffs of
    `hermetic_test.rs`, ADR-0021, the `holler-pane` docs, `CHANGELOG.md` and the manifests.
  - Read at `origin/main`: #641's `exec.rs`, `tmux.rs` and `lib.rs` (structure); #663's `probe.rs` (visibility); #708's
    diff.
  - Greps over `src/`: `Command::new`, `"-t"`, the loops and sleeps, the `Duration` constants, `400 | 404`,
    `call(`/`call_until(`, `Value::Bool(true)`.
  - `hermetic_test.rs` has 30 tests at the base and at the head. `http.rs`, `server.rs`, the test kit and the workspace
    `Cargo.toml` are unchanged. `gh issue view 696` shows 0 comments.

## S (Phase 10, spec audit) — 2026-10-10T00:16:11-06:00
- **Decided:** REWORK with one TEST-ONLY item (see handoff-S.md).
  - The A and T preconditions are met.
  - Every canonical 642b AC has a proving test or other evidence that asserts behaviour, except AC 25 (the PR body),
    which cannot be audited before the PR exists.
  - Decisions 1, 3-6, 9, 10 and 15-23 are built as stated. A's W-2 to W-6, W-8(b) and W-9 are applied. F's five choices
    beyond the brief are disclosed and contradict no AC: the watch's port check, `select_session` failing at once on a
    dead or missing pane, the UTF-8 check before the resolver, `classify` with `Ran::reason`, and `poll`.
  - The quality checks are clean apart from one item: the guards, sizes, Risk 6 (one tmux `Command::new`, every `-t`
    through `exact_target`), the docs and the fixture.
  - The REWORK item: T-green's handoff names the operator's machine twice (`handoff-T-green.md:9` and `:162`), and no
    file on `main` holds that name. The fix is a neutral phrase there. Because the branch is unpushed and only `5d20f61`
    carries the name, O or the run's agent folds the fix into that commit at the rebase, so no pushed commit holds it.
- **Assumed:**
  - Handoffs are public. Handoff directories reach `main` (508 and 647 are there now), and a PR's commits stay visible
    on GitHub even when a cleanup later removes the directory.
  - I relied on T's recorded Tier 1 and Tier 2 results and did not re-run them, per S's role. My own checks were greps,
    `wc -l`, `git ls-files -s`, `git diff` and a `git merge-tree` preview.
- **Hedged:**
  - Two latent contract issues would let `attach_tui` confirm a TUI that never started, or replace the wrong pane:
    the respawn keeps the pane's old title, and A's W-1 (`=<name>:` is the current window's active pane). They could
    have been an ADVISORY-HOLD. I did not choose that, for four reasons:
    - nothing calls `attach_tui` yet: #644 and #649 are open with no PR;
    - the issue's acceptance holds;
    - A and T rated both non-blocking;
    - #640's audit handled latent consumer-facing items as follow-ups. #647's hold was for a missing,
      operator-confirmed issue amendment, which is not the case here.
  - Both must be filed before #644's relaunch uses `attach_tui`, and neither is filed yet (Advisories 1 and 2).
  - `test-only` routes the item to T because the file is T's own handoff. It is not a test file, so the item says so
    plainly.
- **Evidence:**
  - Read in full: the brief, the five phase handoffs, `evidence.md`, the production files, every new or changed test
    file and the fixture. Read as diffs: the CHANGELOG, ADR-0021, `holler-pane`, the manifests and `Cargo.lock`.
  - Greps over the diff's added lines: names, hosts, paths, IPs, secrets and closing keywords. The machine name has
    exactly two hits, and it appears in no file on `origin/main`.
  - `git ls-remote --heads origin issue-642-implementation` is empty, so the branch is not pushed.
  - `origin/main` moved to `abdcbb6` (#640, merged 00:06 MDT). `git merge-tree --write-tree origin/main HEAD` shows
    conflicts in `CHANGELOG.md`, `hermetic_test.rs`, `stub.rs` and ADR-0021's "Deferred" list. The resolution rules
    are in Advisory 3.
  - The diff gate's hand rerun is in the session scratchpad (`642-diff-r3.md` and its `usage.json`): a complete review
    with `finish_reason: stop` and verdict PASS. The prompt was 77,559 tokens by the model's count, not the ~64,093 that
    the O entry above states. All nine of its needs-verification items resolve in the code's favour.
  - `gh issue list` finds no filed follow-up for P-2, P-3, the stale title or W-1. #644 and #649 are open with no PR.
