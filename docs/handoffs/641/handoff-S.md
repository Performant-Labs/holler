# Handoff-S: Phase 8 - #641 host adapter (tmux sessions, process control, the launcher primitive)  (spec audit)

**Date:** 2026-10-09, 19:25 MDT
**Branch:** issue-641-implementation (worktree `.claude/worktrees/0641-host-adapter`, at aada10e; merge base 3bdd129)
**Issue:** #641 (epic #633). The issue and `docs/handoffs/641-brief.md` (as amended after round 3) are the source of truth.
**Handoffs reviewed:** `handoff-A.md` (round 4), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`, `decisions.md`, `evidence.md`; the outside-review results (`641-brief-result-r1.md`, `-r2.md`, `641-diff-result-r1.md`, git-ignored).
**Read in full:** `src/{lib,exec,tmux}.rs`, `tests/{fake_tmux_test,real_tmux_test}.rs`, `tests/common/mod.rs`, and the `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md` diffs.

## A precondition

Met. `handoff-A.md` round 4 is **PASS** on the brief: 0 blocks and 3 warns, W-18 to W-20, which T applied and journalled at RED. `handoff-A-dup.md` is also **PASS** on the diff, with one test-only warn, W-21.

## T precondition

Met. `handoff-T-green.md` reports **no blocking issues**.

RED was confirmed in `handoff-T-red.md`:
- the 25 default-run behaviour tests and the 9 real-tmux tests failed on the stub's `NotImplemented`, not on a compile error;
- the failure lines are quoted verbatim;
- 3 tests passed at RED by design: two absence guards and the re-exec helper.

GREEN was confirmed after RED:
- 28/28 default-run tests and 9/9 real-tmux tests pass;
- a 7-mutant spot check was run, and every mutant was killed.

The two empty `t-red -- verdict: BLOCK` commits (5344681, e85eef0) change no file. They are workflow markers from earlier attempts.

**S run, required by the brief's Decision 11** ("run by T-green and S"):
- Command: `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored`, with `TMUX` and `TMUX_PANE` unset, at 19:20 MDT on tmux 3.7c.
- Result: **9 passed, 0 failed** in 1.31 s.
- Afterwards there was no `/tmp/hlr-tmux-*` directory and no test process. A count-only read of the default socket found 0 `demo-*` sessions.

I re-ran no Tier 1 gate. The rows below cite T's recorded output for those.

## Acceptance criteria

Test files are under `crates/holler-adapter-host/tests/`. Line numbers are the test's `fn` line.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Conformance suite on real tmux, all nine cases (missing session is `pane-not-found` for `ps`/`run` and `Ok` for `stop_owned`; empty argv is `usage`; missing session checked before empty argv) | `real_tmux_test.rs:207` `the_tmux_host_passes_the_host_conformance_suite`: `run_host_conformance(private)` returns `Ok(())`, and the suite is reused, not copied. Re-run by S. | PASS |
| 2 | Ensuring twice makes one session; `session_path` keeps the first cwd; a third call with a nonexistent cwd is `Ok` and changes nothing | `real_tmux_test.rs:218` `ensure_session_twice_makes_one_session` | PASS |
| 3 | Stop kills only the owned process. The shell pane, the session, the other session's pid and an outside `sleep` survive. | `real_tmux_test.rs:245` `stop_owned_kills_only_the_owned_process` (pid gone from `ps` and from the OS, checked with the `kill` binary) | PASS |
| 4 | Escalation to `KILL`, and a member that outlives its leader (W-14) | `real_tmux_test.rs:276` `stop_owned_escalates_to_kill` (TERM-ignoring group, then the `member.pid` case). The test fails if the KILL or the group probe is removed: the pid would survive, or `stop_owned` would time out. | PASS |
| 5 | Missing session is a typed error when no server was ever started | `real_tmux_test.rs:305` `missing_session_is_typed` (asserts first that the socket file does not exist) | PASS |
| 6a | Every method is bounded, `op` equals `HostOp::as_str`, and the hung child is reaped | `fake_tmux_test.rs:87` `every_method_is_bounded_and_reaps_its_child`. It asserts under 1.2 s. The reap check is `kill -s 0` through the binary. A zombie still answers signal 0, so the check would fail without the `wait`. | PASS |
| 6b | A missing binary is `Unavailable` from all four methods, `stop_owned` included | `fake_tmux_test.rs:114` | PASS |
| 6c | Error mapping. Missing strings give `PaneNotFound`/`Ok`. Others give `Unavailable` carrying the first stderr line, never the argv or cwd sentinel. | `fake_tmux_test.rs:125` (the 5 Decision 8 strings, no signal), `:145` (three other stderrs, four methods, sentinels absent) | PASS |
| 6d | Argv passed exactly in three spawns; `env --` always; trailing `;` escaped | `:173` (all three vectors compared in full, tag separate, so the K2 form is ruled out), `:236`, `:248` | PASS |
| 6e | Exact targets on every path; `new-session -s NAME` takes no `=` | `:261` `every_target_names_the_session_exactly`: covers both ensure paths, the read, `new-window`, `ps`, and the stop listing plus at least one poll | PASS |
| 6f | Socket and config flags come before the subcommand; `TMUX`/`TMUX_PANE` are removed and nothing is added | `:319`; `:366` with the re-executed child `:357` (`unset unset unset`, no `set_var`, no `unsafe`) | PASS |
| 6g | Bad input refused, values escaped | `:392` (missing absolute dir and `.`: only `has-session`, `Usage`/`Ok`), `:418` (`p##S`, `q\;`), `:438` (`argv[0]` with `=`, nothing echoed), `:466` (relative, gone and empty session dir), `:490` (session checked before argv) | PASS |
| 6h | Stop by ownership through the kill seam | `:516` (five panes; one TERM `-101`; 202/303/404/4294967295 never named; every `LC_ALL` is `C`; KILL after the grace), `:539` (W-8), `:550` (W-14), `:560` (W-18, A's added bullet), `:584` (a kill failure is `Unavailable`), `:597` (W-10: every TERM before any KILL, one KILL each, under grace + 1 s). Fakes answer by argument and by the kill record, never a timing-sized queue (W-19). | PASS |
| 6i | No untagged orphan; strict `new-window` parse; out-of-range pid in `ps` | `:632` (`can't find window` and `no such window`: `Ok`, no signal), `:647` (exactly `-s KILL -- -4242`), `:657` (seven bad answers: two calls, no signal), `:681` | PASS (one sub-case untested, Advisory 3) |
| 7 | No broad kill; signals only to owned groups, or the failed-tag pid, in `2..=i32::MAX` | `fake_tmux_test.rs:694` `no_broad_kill_in_source` (walks `src/` at runtime, skips `//`); the AC grep, re-run by S, prints nothing; `Pid` is range-checked by construction (`src/tmux.rs:174-189`) | PASS |
| 8 | Quality gates | T-green's Tier 1 table: lint, changelog-check, clippy `-D warnings`, rustfmt, machete and test-hooks are clean. S's read-only checks: no `unsafe`; no `unwrap`/`expect`/`panic` in `src/`; every `#![allow]` carries `// #641`; largest file 753 lines. | PASS. The workspace run has 4 environmental failures in `holler-cli/tests/logging_test.rs`, caused by a live hub on this host; they pass with an empty `HOLLER_STATE_DIR`, and CI decides (Advisory 2). |
| 9 | Real-tmux tests are isolated | `real_tmux_test.rs:106` `private()`: `/tmp/hlr-tmux-*`, `-S <dir>/s` asserted under 100 bytes, a config with `/bin/sh`, and a guard that runs `kill-server` and then removes the dir. `fake_tmux_test.rs:720` pins one `TmuxHost::new` per side and no `Default`/`Name` in the real file. All test session names are `demo-*`. | PASS as resolved by W-20. The literal grep over all of `tests/` prints `fake_tmux_test.rs:327,330`, the hosts AC 6f requires (Advisory 4). |
| 10 | CHANGELOG `[Unreleased]` / `Enhancements` entry linking #641, naming no host | `CHANGELOG.md` diff: one entry, links #633 and #641, states the ownership narrowing; no host name | PASS |
| 11 | Argv and cwd pass exactly on real tmux | `real_tmux_test.rs:329` `argv_and_cwd_pass_exactly`: six hostile elements arrive verbatim; the session is not renamed; `p#S` and `#(touch ...)` paths are exact; `pwd -P` is checked; no `ran` file | PASS |
| 12 | Targets are exact on real tmux | `real_tmux_test.rs:379` `targets_are_exact`: the prefix case, and a window name in the newest session | PASS |
| 13 | `run` works in the session's cwd | `real_tmux_test.rs:409` | PASS |
| 14 | `run` refuses a missing or relative directory | `real_tmux_test.rs:429` | PASS |

The issue's own acceptance bullets map onto these rows:
- the conformance suite is AC 1;
- the opt-in temporary-server bullets are ACs 2, 3 and 5;
- no `pkill`/`killall`/name match is AC 7;
- "a hung call is reported, not waited on" is AC 6a.

## Spec compliance

Every decision of the brief is implemented as stated.

- **Decision 1:** `TmuxSocket`, `TmuxHost::new` and the five `with_*` builders, with the stated defaults. Plain data, so `Send + Sync`. No I/O in the constructor (`src/lib.rs:146-220`).
- **Decision 2:** one `Call` deadline per method, and every subprocess gets the time left (`src/lib.rs:394-427`). Nothing is spawned after the deadline. A child at the deadline is killed and reaped (`src/exec.rs:61-142`). The tag cleanup gets `max(left, 250 ms)` (`src/lib.rs:258`).
- **Decision 3:** `run` makes three spawns in order: the read, `new-window`, then the tag. The refusal order is missing session, empty argv, `argv[0]` with `=`, then a bad directory, which gets the brief's exact `what` (`src/lib.rs:337-354`). The `new-window -P` answer is parsed strictly (`src/tmux.rs:216-220`). The tag is one invocation with the adapter's own `;` (`src/tmux.rs:102-119`). A failed tag sends the cleanup KILL first (`src/lib.rs:252-261`).
- **Decision 4:**
  - TERM goes to every owned group first, then one shared grace capped at the deadline, then KILL to every group not done, then the rest of the bound (`src/lib.rs:359-380`).
  - A group is done once its pane is gone and `kill -s 0` finds no member, and the probe is asked only after the pane is gone (`src/lib.rs:280-306`).
  - A missing poll listing still probes each group (W-18).
  - A pane is owned only if it is live, its tag equals its pid, and the pid is in range (`src/tmux.rs:249-265`).
  - `kill` runs with `LC_ALL=C`, and only `No such process` counts as empty (`src/exec.rs:202-225`).
- **Decisions 5 to 7:**
  - `ps` lists live panes and refuses a malformed listing (`src/tmux.rs:224-236`).
  - `ensure_session` checks absolute and existing on the unescaped value, takes `duplicate session` as `Ok`, and otherwise asks only `has-session` (`src/lib.rs:314-333`).
  - The `argv[0]` refusal never echoes the element (`src/lib.rs:453-464`).
- **Decisions 8 to 10:**
  - `classify` covers every Decision 8 string (`src/tmux.rs:304-319`).
  - `TMUX` and `TMUX_PANE` are removed and nothing is added to tmux (`src/lib.rs:224-238`).
  - `has-session` gets `=NAME`, everything else `=NAME:`, and `new-session` gets `-s NAME`. `display-message` is never used.
- **Decisions 11 to 15:**
  - There is no CI change.
  - The five narrowings and the Decision 15 facts are in the crate docs (`src/lib.rs:49-101`).
  - There is one escape function plus the cwd's `#` doubling (`src/tmux.rs:160-172`).
  - Every default-run host is built through `common::host_on` with both fake binaries.

**Declared deviations, both justified:**
1. `classify` also reads `no such window` as a closed window. tmux 3.7c prints that text for `set-option` on a gone window. F's probe shows it, AC 11 and AC 13 fail without it, and T-green pinned it in the default run. It keeps Decision 3's intent: a window that is already gone when the tag runs is `Ok`.
2. The crate docs state Decision 15 with W-18's qualification and give the relaunch order `stop_owned`, then `ensure_session`, then `run`. A showed the unqualified form false (Q6).

**One undeclared reading, documented and not silent:** F's Design decision 2 is about a tag that fails with a missing-class stderr. The adapter returns `pane-not-found` after the cleanup KILL, where AC 6i's words say `Unavailable` (Advisory 3).

## Quality audit

**Correctness and failure handling.**
- Every error path fails closed. A malformed or out-of-range pid is never signalled, and it makes `ps` unavailable.
- A missing binary, a spawn failure or a lost output is `unavailable`. A hung subprocess is `timeout` and reaped.
- A drain thread cannot stretch a call past its deadline (`recv_timeout`).
- There is no shared state, so there is no lost-write risk.

**Build guards.**
- No `unwrap`, `expect` or `panic` in `src/`, and no `unsafe` in the crate.
- Three `#![allow]`s, each with `// #641`.
- Sizes: `lib.rs` 476, `exec.rs` 225, `tmux.rs` 319, `common/mod.rs` 311, `fake_tmux_test.rs` 753, `real_tmux_test.rs` 452. All are under 900.
- clippy is clean per T, including `too_many_lines`, `cognitive_complexity` and `dead_code`.

**Protocol.** Not touched: no wire surface, error code, golden file or `docs/protocol/v2.md` change.

**Tests.**
- The real-tmux tier runs on private servers only.
- No fixed sleep is used for synchronization: the one sleep is a bounded `ETXTBSY` retry, and the waits are polls.
- RED-first evidence is in T-red.
- The tests assert behaviour, not implementation. Each test I traced fails if its behaviour is removed, and T's 7 mutants were all killed.

**Documentation.**
- The CHANGELOG entry is present and `changelog-check` is ok.
- There is no new log event, CLI surface or protocol field, so README and `docs/` need no change.
- The crate docs carry the brief's error table, ownership rule, escaping rule, narrowings and consumer facts.

**Public-repository privacy.**
- I grepped every added line, the handoff docs included, for:
  - the issue title's host name and other host names;
  - account names and home-directory paths;
  - tailnet names and IPv4 addresses;
  - key, token and password shapes.
- No hits. The commit messages have no hits either, and every commit uses the GitHub no-reply identity.
- `pfleet` appears once, and it is already public on `main` (ADR-0020, ADR-0021).

**Commit hygiene.**
- Every subject passes `.githooks/commit-msg`; the merge commit 9b84385 is exempt by the hook.
- Every non-merge commit carries a `Co-Authored-By` trailer.
- None has a session link. That matches every recent commit on `main` and the trailer `prepare-commit-msg` adds.
- No PR exists yet, so PR hygiene is the post-S step (Advisory 1).

## Scope check

The scope is exactly the brief's:
- `crates/holler-adapter-host/**` (three `src` files and three test files);
- the mechanical `Cargo.lock` and `CHANGELOG.md`;
- this run's handoffs.

`wiring.rs` (#649), `FakeHost`, ADR-0021 and CI are untouched.

The seventh file, `tests/common/mod.rs`, is the brief's W-17 fallback. T took it up front and journalled why: a single file would have been about 1,050 lines. Production code is about 1,020 lines against the brief's estimate of about 695. The difference is mostly crate docs; there is no extra feature and no unrelated refactor.

## Verdict

**PASS.** Every one of the 14 acceptance criteria is met and proven by a named test:
- ACs 1 to 5 and 11 to 14 were re-run by S on real tmux;
- AC 9 holds under A's sanctioned W-20 reading.

Every brief decision is implemented, and both deviations are declared and evidenced. The quality, privacy and scope checks are clean. Ready for O.

## Advisory notes (non-blocking)

1. **Before the push and merge.**
   - `git merge-tree HEAD origin/main` (main is now at ce12cdb) reports a content conflict in `CHANGELOG.md` only. When merging main, keep both `[Unreleased]` entries. Nothing the crate builds against (`holler-pane` ports, types and re-exports, or the test kit's host fake and suite) changed on main since 3bdd129.
   - The script opens the PR as `Implements #641` with the body `Closes #641.`. Retitle it to a Conventional Commit subject, because the squash subject lands on `main`, for example `feat(adapter-host): TmuxHost, the host adapter over a local tmux server (#641)`.
   - Keep the issue title's host name out of the PR title and body (W-9).
   - Add the AI disclosure that `CONTRIBUTING.md` requires with `gh pr edit`.
2. **CI decides two things.**
   - The 4 `logging_test` failures are environmental: the test helper inherits this host's hub state. Worth a follow-up issue; it is outside this story.
   - The macOS leg runs the fakes on `/bin/sh` and BSD `grep`. The `alive` helper also depends on BSD `kill` accepting `--`. Confirm both legs are green before merging.
3. **AC 6i versus Decision 8, one untested input.**
   - When the tag fails with a missing-class stderr (the server went away between `new-window` and the tag), `run` sends the cleanup KILL and then returns `pane-not-found` (`src/lib.rs:253-255` through `refused`). AC 6i's words say `Unavailable` for "any other stderr".
   - Decision 8 and the crate docs' table support F's mapping, and the AC's safety point (the KILL) holds either way.
   - Fix: reconcile the AC wording, or add one default-run case that pins the code chosen.
4. **AC 9's text is stale.** Its literal grep over `tests/` contradicts AC 6f (W-20). The resolved reading is journalled (T, RED) and pinned by `fake_tmux_test.rs:720`. Amend the brief's AC 9 for the record, or say so in the PR.
5. **Follow-ups not yet filed.**
   - A's round-4 Note 3 (W-18, B-6) asks for a comment on #644, and on #670 if its flag layer owns `--project`, saying:
     - `--project` must be absolute before it is recorded (a relative one is `usage` at session creation);
     - a relaunch is `stop_owned`, then `ensure_session`, then `run`.
   - As of 19:25 MDT neither issue has that comment.
   - #696 (W-7, one shared bounded runner) is filed and open.
6. **Journal and gates.**
   - `decisions.md` has no O entry for the round-2 and round-3 brief amendments (95e2260, 0f18b80). Decisions 12 and 15 say their content is recorded there; the crate docs carry it in full.
   - A's round-4 Note 4 about the outside model is moot. `CLAUDE.md` has named `deepseek-v4-pro` since ec0431f (09:14 MDT today), and every gate ran it with a real completion: brief r1 and r2 and diff r1, all PASS. The diff gate's NV-1 to NV-3 are covered by the brief's evidence and Risks.
   - The issue's `rigor: in-session` was raised to second-opinion by the operator (dd5e99b).
7. **Optional test-only hardening.**
   - `duplicate session` giving `Ok` (Decision 6) is pinned only by the opt-in AC 2 and conformance case 4.
   - `has-session` with a non-missing failure giving `unavailable` is not pinned at all.
   - One fake case each would put both in CI.
   - A-dup's W-21 still stands: `real_tmux_test.rs` duplicates `name`, `argv` and `alive` from `tests/common/mod.rs`.
