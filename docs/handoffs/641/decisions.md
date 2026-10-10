# Decisions: #641 host adapter (tmux sessions, process control, the launcher primitive)

## A (Phase 3, up-front plan review) — 2026-10-09T14:52:00-06:00
- **Decided:** BLOCK on docs/handoffs/641-brief.md at 69e71b5, with 3 blocks and 9 warns (see handoff-A.md). The structure is sound and has nothing in the tree to extend:
  - the dependency direction follows ADR-0021 §5;
  - the frozen `HostPort` is implemented as is, and the conformance suite is reused;
  - no new error code, no `unsafe`, and ownership is recorded inside tmux (I6).

  The plan is still not safe to test or implement as written:
  - **B-1.** An argv element that ends in `;` splits the tmux command, so the elements after it run as tmux commands.
  - **B-2.** tmux format-expands the `-c` cwd, and `#(...)` in it runs a shell command.
  - **B-3.** `kill` has no seam, so fake-tmux tests can make the adapter signal real process groups on the pipeline host.

  Each fix is an addition: an escaping rule in `tmux.rs`, `with_kill_binary`, a test-safety rule, and the ACs in Notes for O.
- **Assumed:** the `-c` format expansion behaves in other supported tmux versions as it does in 3.7c, the only version probed. The `;` splitting does not rest on the probe alone: `man tmux`, section PARSING SYNTAX, says "a trailing semicolon is also interpreted as a command separator" and gives the escape `neww 'foo\;' bar`. The `remain-on-exit` and `exit-unattached` effects in W-8 come from tmux's documented options and were not probed.
- **Hedged:** I graded B-3 a block, not a warn. The ACs as written do not require a fake test to reach `kill`, but AC 6e invites one, and the consequence is a signal to a live process on the pipeline host. The seam also gives Decision 4 default-run coverage, which it otherwise lacks, because AC 3 and AC 4 are `#[ignore]` and CI never runs them. W-1 (two `usage` refusals only the adapter has) is a warn, not a block: nothing in the conformance suite or in a consumer story depends on those inputs today.
- **Evidence:**
  - Read the brief, issue #641, epic #633, #663, #644, #649, #667, #684, #642 and #640. Read `ADR-0021.md` in full.
  - Source read: `holler-pane/src/{ports,probe,argv,lib}.rs`, `error.rs:380-510`, `holler-pane-testkit/src/{host,lib}.rs`, `conformance/host.rs`, `tests/host_conformance_test.rs`, `holler-cli/src/pane/wiring.rs`, the precedent guard tests, `scripts/lint.sh`, `clippy.toml`, `.github/workflows/ci.yml`, `CHANGELOG.md` and the agent overlays.
  - `grep` checks: no production code spawns a process, nothing depends on `holler-adapter-host`, and no personal host name appears anywhere in the tree.
  - Three tmux 3.7c probes on private `-S` servers under `/tmp/hlr-a641-*`, each killed and deleted on exit. Commands and results are under "Probe evidence" in handoff-A.md.

## O (brief amendment after A's BLOCK) — 2026-10-09
- **Decided:** amended docs/handoffs/641-brief.md, applying A's Notes for O verbatim; no earlier decision reversed.
  - B-1, B-2: Decision 13 (one pure escape for every value the adapter did not author; `#` doubled in the cwd; tmux output validated), AC 6d and 6g extended, AC 11 (real tmux) added, a Risks line.
  - B-3: `with_kill_binary` (Decision 1), Decision 14 (no default-run test signals a pid it did not spawn), AC 6h.
  - W-1: three deliberate narrowings listed in Decision 12; the cwd is checked only when the session would be created (`has-session` on a bad cwd).
  - W-2: the `argv[0]`-contains-`=` refusal never echoes the element; `Unavailable.what` is never argv or cwd.
  - W-3: on a non-window-gone `set-option` failure, `run` KILLs the printed pid's group through the kill seam first (AC 6i).
  - W-4: AC 7's guard walks `CARGO_MANIFEST_DIR/src` at runtime, skipping `//` comments.
  - W-5: AC 8 adds `scripts/lint.sh`, `scripts/changelog-check.sh`, workspace clippy; RED is T-landed signature-only stubs answering `not-implemented`.
  - W-6: Decision 15 and two forward-compat rows (#646; #650/#654).
  - W-7: follow-up for #663 to expose its bounded runner from holler-pane (the orchestrator files it).
  - W-8: `pane_dead` 1 counts as gone; the tag invocation also sets the window's `remain-on-exit off`.
  - W-9: the personal host name in the issue title stays out of every public artifact.
- **Evidence:** O re-probed on a private `-S` server under `/tmp` (killed and deleted afterwards, no operator session touched): `-c "<dir>/q\;"` and `-c "<dir>/p##S"` give the exact session_path; `set-option ... @holler-pid N ; set-option ... remain-on-exit off` in one invocation sets both; with global `remain-on-exit on` a TERMed window lingers with `pane_dead` 1, while the window with its own `remain-on-exit off` is gone.
- **Size:** ~+255 lines (~1,455 in all), same six crate files plus two mechanical; still one run.

## A (Phase 3, up-front plan review, round 2) — 2026-10-09T17:15:00-06:00
- **Decided:** BLOCK on docs/handoffs/641-brief.md at 81b0ddd, with 2 new blocks and 4 warns (handoff-A.md, round 2). Every round-1 finding is applied as asked.
  - **B-4.** `list-panes -s -t =NAME` (Decisions 4 and 5; AC 6e accepts it) is not exact. tmux reads it as a window target and then falls back to a session prefix match. So `stop_owned` of a crashed pane signals a sibling whose name it prefixes (`hj-c1r1` and `hj-c1r10`). The fix is `=NAME:` for every target except `has-session`'s.
  - **B-5.** `new-window` without `-c` starts `run`'s program in the tmux client's cwd, that is the CLI's or the hub's, not in the session's `host.cwd`. The fix is `-c '#{session_path}'`, a constant format.
  - **Warns.**
    - W-10: one shared grace for all owned panes, not one per pid.
    - W-11: a dead tagged pane is never signalled.
    - W-12: `LC_ALL=C` on `kill` only. Never add a variable to a tmux subprocess, because it reaches every pane.
    - W-13: a `Timeout` of `new-window` can leave an untagged process (documentation only).
- **Assumed:** only tmux 3.7c was probed. The fixes (`=NAME:`, `-c '#{session_path}'`) are plain documented syntax. They do not depend on how each version falls back on a colon-less window target.
- **Hedged:** I graded B-5 a block, not a warn, although runtime behaviour belongs to F and T. The reason is that the brief fixes the exact `new-window` vector, AC 6d would pin the defect into a test, and no AC or suite case reads a process's cwd. B-4 is a block for the same reason: AC 6e explicitly accepts the faulty form. W-11 is a warn because Decision 3's word "live" already excludes dead panes; only the explicit rule and its AC are missing.
- **Evidence:**
  - Read the amended brief, the round-1 handoff, this journal, and the outside-model rounds r1 and r2.
  - Source read: `holler-pane/src/{ports,pane}.rs`, `holler-proto/src/vocab.rs`, `holler-pane-testkit/src/{host.rs,conformance/host.rs}` and `holler-adapter-herdr/{Cargo.toml,src/lib.rs}`.
  - Docs and other sources: ADR-0021 §1 to §3 and its import table, issues #641 and #644, the epic, `scripts/lint.sh`, `docs/testing.md` and the CI matrix.
  - Probes: three scripts on tmux 3.7c, each on a private relative-socket server in the session scratchpad, with `TMUX` and `TMUX_PANE` unset. Each killed its server and removed its directory, and a process check afterwards found no probe server left.

## A (Phase 3, up-front plan review, round 3) — 2026-10-09T17:35:41-06:00
- **Decided:** BLOCK on docs/handoffs/641-brief.md at 95e2260, with 1 new block and 4 warns (handoff-A.md, round 3). Every round-2 finding is applied as asked. This is the third Phase 3 BLOCK, so the escalation to the operator applies.
  - **B-6.** `run`'s `-c '#{session_path}'` works only while the session's directory is absolute and exists.
    - tmux keeps a relative `-c` verbatim and resolves it again against each later client's cwd.
    - tmux falls back to `$HOME` when the directory is missing, as with a removed worktree.

    In both cases `run` is `Ok`. The fix has two parts:
    - `ensure_session` creates only with an absolute existing directory;
    - `run` first reads `list-panes -t =NAME: -F '#{session_path}'` and refuses (`unavailable`) unless it is an absolute existing directory. The read replaces `run`'s `has-session`.
  - **Warns.**
    - W-14: a TERM-ignoring group member outlives its leader, so escalate by group, not by pane, or document it.
    - W-15: range-check pids read from tmux (`2..=i32::MAX`) before they reach `kill`.
    - W-16: AC 6e's "no bare name" contradicts `new-session -s`.
    - W-17: shared test helpers go in `tests/common/mod.rs`, as the neighbors do.
- **Assumed:** only tmux 3.7c and procps-ng 4.0.4 were probed. util-linux `kill` was not run here: its `kill -9 -1` behaviour is from its own man page, and the `kill(2)` meanings of `0` and `-1` are from the system call's documentation.
- **Hedged:**
  - I graded B-6 a block, not a warn, on the grounds round 2 used for B-5. The brief fixes the exact vector, the failure is silent (`Ok`, and a coding agent works in the wrong directory), no AC or suite case covers it, and the inputs are realistic: `--project` is a plain string that nothing canonicalizes, and worktrees are removed routinely.
  - Part (b) could in principle be documented instead. But no port call lets a consumer read or end a session, so documentation would leave #644 with no defense.
  - W-14 is a warn because it is the conservative failure (a process left running), not a signal to a stranger.
  - W-15 is a warn because a real tmux never prints such a pid.
- **Evidence:**
  - Read the brief as amended (the full diff 81b0ddd..95e2260), the round-2 handoff, this journal, and the outside-model results r1 and r2 (both PASS, deepseek-v4-pro).
  - Read issue #641 and ADR-0021 §1 to §5, §12 and §13.
  - Source read: `holler-pane/src/{ports,argv,probe}.rs`, `error.rs:395-480`, `pane.rs:105-111`; `holler-pane-testkit/src/host.rs`; `holler-cli/src/pane/args.rs`; `holler-adapter-herdr/{Cargo.toml,src/lib.rs,tests/common/mod.rs}`; the workspace `Cargo.toml` lints and edition; `clippy.toml`; `scripts/lint.sh`; the tester overlay.
  - `grep` checks: no bounded subprocess runner in any production crate; the existing `kill` calls are test-only `libc::kill`; remote `main` is unchanged at 3bdd129; no tracked file holds the issue title's host name; the outside-model prompt and result files are git-ignored.
  - Probes: three scripts (PR1 to PR7) on private relative-socket servers in the session scratchpad, with `TMUX` and `TMUX_PANE` unset. The only real signal was one TERM to the probe's own pane group; every other `kill` call used signal 0. Each script killed its server and removed its directory, and a process check afterwards found none left.

## A (Phase 3, up-front plan review, round 4) — 2026-10-09T18:12:00-06:00
- **Decided:** PASS on docs/handoffs/641-brief.md at 0f18b80, with 0 blocks and 3 warns (see handoff-A.md, round 4). Every round-3 finding is applied as asked (B-6 and W-14 to W-17). The amendment adds no drift and no parallel path.
  - **W-18.** The brief assumes the shell window always outlives a stop. Once the harness window is the session's last, the TERM ends the session, and the next poll answers "missing". Decision 8's `Ok` for `stop_owned` would then skip Decision 4's `kill -s 0` check, so a TERM-ignoring member survives. Decision 15 and the #644 row rest on the same assumption: relaunch should ensure the session again before `run`.
  - **W-19.** AC 6h's fakes need answers keyed to the arguments and to the kill record, not only "one for every call" or a numbered queue. The number of polls that fit in a grace depends on timing. AC 6a's pid check must not use `/proc`, because the default run is also CI's macOS leg.
  - **W-20.** AC 9's grep (no `TmuxSocket::Default` or `TmuxSocket::Name` under `tests/`) contradicts AC 6f, which must build those hosts in `tests/fake_tmux_test.rs`. The fix is to scope the grep to `tests/real_tmux_test.rs` and to build every fake host through one helper that always sets both fake binaries.
- **Assumed:**
  - Only tmux 3.7c and procps-ng 4.0.4 were probed.
  - That an interactive shell in a detached pane exits on an idle `TMOUT` comes from the bash and zsh documentation; it was not probed. The probe closed the shell window with `kill-window` instead, which yields the same session state.
- **Hedged:**
  - W-18 is a warn, not a block. It is W-14's class: the conservative failure, a process left running, with no stranger signalled. It also needs two uncommon conditions together: the shell window gone, and a member that ignores TERM and HUP.
  - W-20 is a warn for the same reason round 3 graded W-16 a warn: an AC that, read literally, forbids the test or vector another part of the brief requires. It is certain to be hit, though. So the handoff gives T a reading that keeps the grep's intent, to apply and journal.
  - I did not block a fourth time on spec precision. The plan's architecture has been stable since round 1, and none of the three warns reverses a decision.
- **Evidence:**
  - Read the brief in full (O's diff 2af088d..0f18b80), the round-3 handoff, this journal, and the outside-model results r1 (deepseek-v4-pro on the current brief: PASS, NV-3 and NV-8 matching W-19) and r2.
  - Read issues #641 and #644, the epic's rigor lines, and ADR-0021 §2, §5 and §9.
  - Source read: `holler-pane/src/{ports,argv,probe}.rs`, `error.rs:370-520`; `holler-pane-testkit/src/{host.rs,conformance/host.rs}`, `conformance/mod.rs`, `tests/host_conformance_test.rs`; `holler-adapter-host/{Cargo.toml,src/lib.rs}`; `holler-adapter-herdr/{Cargo.toml,src/lib.rs,tests/common/mod.rs}`; `holler-proto/src/vocab.rs`; the workspace `Cargo.toml`, `clippy.toml`, `scripts/lint.sh`, `scripts/changelog-check.sh`, `.github/workflows/ci.yml` and the tester overlay.
  - `grep` checks:
    - no bounded subprocess runner in any production crate, and `kill` only as test-only `libc::kill`;
    - the `with_*` builder precedent;
    - the `current_exe` re-exec precedent;
    - when AC 9's grep and AC 6f entered the brief: both in every version since 69e71b5;
    - no tracked file holds the issue title's host name, and the outside-model files are git-ignored.
  - Probes: two scripts (Q1 to Q6) on private relative-socket servers in the session scratchpad, with `TMUX` and `TMUX_PANE` unset, run through `/bin/sh`.
    - Signals: one TERM and one KILL, both to the probe's own pane group; every other `kill` call used signal 0.
    - Each script killed its server and removed its directory, and a process check afterwards found none left.

## T (Phase 4, author / RED) — 2026-10-09T18:30:07-06:00
- **Decided:** RED is valid. I authored 37 tests: 28 default-run tests in `tests/fake_tmux_test.rs`, with the fakes in `tests/common/mod.rs`, and 9 `#[ignore]` real-tmux tests in `tests/real_tmux_test.rs`. All 25 default-run behaviour tests and all 9 real-tmux tests fail on `NotImplemented`. Three tests pass at RED by design: two absence guards (AC 7, AC 9) and the AC 6f re-exec helper. Following the brief's Test plan, I landed a `src/lib.rs` stub with Decision 1's signatures and no logic, plus the `Cargo.toml` deps, so that no failure is a compile error.
  - **W-18 applied** (A round 4, Notes for O item 1). I added an AC 6h test, `a_session_that_ends_on_term_still_has_its_group_checked`, in which a poll answers `can't find session` after the TERM. The pinned rule: Decision 8's `Ok` applies to `stop_owned`'s first listing only. A poll that answers "missing" lists no pane, and each group is done only once `kill -s 0` reports `No such process`. Decision 15 and the #644 row are prose for the brief, not tests, so they are left to O and S.
  - **W-19 applied.** The fakes answer in a third mode, keyed on `"$*"` and on the kill record (`killed '<line>'`), so no poll count is ever assumed. AC 6a's reaped-pid check uses `kill -s 0` through the `kill` binary, never `/proc`.
  - **W-20 applied.** AC 9's grep is scoped to `tests/real_tmux_test.rs`. Every fake-side host is built by `common::host_on`, which always sets both fake binaries. A default-run guard pins that `TmuxHost::new` appears once on the fake side and once in the real file, and that the real file names neither `TmuxSocket::Default` nor `TmuxSocket::Name`.
  - Shared fake helpers went to `tests/common/mod.rs`, opening with `#![allow(dead_code)] // #641`. This is the brief's W-17 fallback, taken up front: one file would have been ~1,050 lines, over the 900-line gate.
- **Assumed:**
  - AC 6c's missing-session list includes `Connection refused` because Decision 8 names it, although AC 6c's own list omits it.
  - `ps` lists only live panes (`pane_dead` 0), per Decision 5's "every live pane". A default-run test pins it.
  - `Unavailable.what` holds the first stderr line verbatim. The tests check `contains`, not equality.
- **Hedged:**
  - AC 6e lets any subcommand other than `has-session`, `new-session` and `set-option` carry only `-t =demo-c1r1:`. It does not fail on extra calls, so F is not over-constrained beyond Decision 10.
  - AC 6h's first bullet pins that a KILL is present, not that there is exactly one. The W-14 and W-18 bullets do pin exactly one, as the brief words them.
  - The fake scripts are warmed up once, unrecorded, with a bounded retry on `ETXTBSY`. A script written while a parallel test forks can be briefly unexecutable, and the adapter would report that as `Unavailable`. This repo has a history of flaky tests, so I closed that window.
- **Evidence:**
  - `cargo test -p holler-adapter-host`: 3 passed, 25 failed, every failure `NotImplemented`.
  - `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored` on tmux 3.7c: 0 passed, 9 failed, all on `NotImplemented`. No `/tmp/hlr-tmux-*` dir was left.
  - Clean on the four new files: `cargo clippy -p holler-adapter-host --all-targets -- -D warnings`, `rustfmt --check --edition 2021`, `bash scripts/lint.sh` (exit 0) and `cargo machete`.
  - A throwaway test, deleted and never staged, drove both fake scripts by hand and confirmed the record, queue, `SAY_DIR`, kill-record keying and `LC_ALL` capture. The handoff is `docs/handoffs/641/handoff-T-red.md`.

## F (Phase 6, implement) — 2026-10-09T18:58:21-06:00
- **Decided:** I implemented `TmuxHost` against T's RED in three production files, as the brief's §Files plans them, with T's public surface unchanged. `src/lib.rs` has the crate docs, the types and `impl HostPort`. `src/exec.rs` is the bounded runner and `kill_group`. `src/tmux.rs` has the vectors, the escape, the strict parsers and `classify`. I also added the CHANGELOG entry. Every default-run test (28) and every real-tmux test (9) passes.
  - **A deviation found on real tmux.** tmux 3.7c answers the tag of a closed window (`set-option -w -t @<id>`) with `no such window: @<id>`. Only target lookups print the brief's `can't find window`. `classify` reads both as a closed window, so the tag is `Ok` (Decision 3's intent) and `run`, `ps` and `stop_owned` read it as "missing" (Decision 8). Without this, AC 11 and AC 13 failed, and every short-lived `run` would be `unavailable` and send a cleanup KILL to an exited pid's group.
  - **The crate docs state Decision 15 with W-18's qualification**: the session survives a stop "while the session's shell window exists", and a relaunch is `stop_owned`, then `ensure_session`, then `run`.
  - **A failed tag.** The cleanup `kill -s KILL -- -<pid>` is sent first. It gets `max(time left, 250 ms)` and is best effort. The error is then mapped as for any `run` call: missing-class is `pane-not-found`, any other stderr is `unavailable`, and a timeout stays `timeout`.
  - **Readers of tmux's output.** The stop listing is read leniently: a malformed line is never owned and never counted live. `ps` refuses a malformed listing. A window id is parsed to a `u32` and printed again.
  - **The `what` of an `unavailable`** is `tmux <subcommand>: <first stderr line>`, `kill -s <SIG>: <first stderr line>`, or `<binary>: <io error>`. It never holds an argv element or a directory.
- **Assumed:**
  - `no such window` is what `set-option` prints for a closed window in the other supported tmux versions too. Only 3.7c was probed.
  - The default-run fakes answer as T checked them. I relied on T's handoff for that, and the 28 tests pass against them.
  - util-linux and BSD `kill` print `No such process` for an empty group, as procps-ng 4.0.4 does. Only procps-ng was run here.
- **Hedged:**
  - The cleanup `kill`'s own failure is dropped and the tag's error is returned, because the brief says only "then returns the error". If that `kill` fails, an untagged process may remain. `tag`'s rustdoc says so.
  - `archChanged: true`. Two private modules are new, though exactly as A's PASSed plan names them, and the crate's public surface is T's stub, unchanged. A false positive costs one A pass.
  - On this machine 4 workspace tests in `holler-cli/tests/logging_test.rs` fail because a live hub answers `holler roster`. They pass with `HOLLER_STATE_DIR` pointed at an empty directory, and nothing in this diff reaches `holler-cli`. I report this as environmental, not as GREEN.
- **Evidence:**
  - `cargo test -p holler-adapter-host`: 28 passed. `-- --ignored` on tmux 3.7c: 9 passed on three runs, and once more after the last doc edit. The first run failed 2 tests on `tmux set-option: no such window: @1`, which led to the fix.
  - Flake check: 15 sequential runs and 24 runs as 8 parallel copies of `fake_tmux_test`, every one 28/28.
  - Gates: `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `rustfmt --check`, `bash scripts/lint.sh` (exit 0), `bash scripts/changelog-check.sh` and `cargo machete` are all clean. The AC 7, `unsafe` and AC 9 greps print nothing.
  - `cargo test --workspace --no-fail-fast` (skipping the CI-skipped load test): 1407 passed and 4 failed, the environmental `logging_test` cases above.
  - Probe: tmux 3.7c on a private `-S` server in a fresh `/tmp/hlr-f641-*` directory, with `-f /dev/null` and `TMUX` and `TMUX_PANE` unset. It sent no signal and was killed and removed on exit. `set-option -w -t @1 ...` on a closed window printed `no such window: @1`, and `list-panes -t @1` printed `can't find window: @1`. The transcript is in `handoff-F.md`.
  - After every real-tmux run there was no `/tmp/hlr-tmux-*` directory, no `demo-*` session on the default socket and no `hlr-tmux` process.
  - Source read: the brief in full, `handoff-A.md`, `handoff-T-red.md`, the three test files, `holler-pane` (`ports.rs`, `argv.rs`, `error.rs`, `pane.rs`, `probe.rs`), `holler-pane-testkit` (`host.rs`, `conformance/host.rs`), `holler-proto/src/vocab.rs` and `holler-adapter-herdr`'s crate docs and manifest. The handoff is `docs/handoffs/641/handoff-F.md`, with `docs/handoffs/641/evidence.md`.

## T (Phase 7, verify GREEN) — 2026-10-09T19:06:00-06:00
- **Decided:** GREEN, with no blocking issue. I closed F's flagged coverage gap in T's own test: `a_window_gone_before_its_tag_is_ok_and_signals_nothing` now also answers the tag with `no such window: @7`, the text tmux 3.7c prints, so CI pins F's Design decision 1 without real tmux. No production code was changed.
- **Assumed:** the 4 `holler-cli/tests/logging_test.rs` failures are environmental. A live hub on this machine answers `holler roster`, they pass with an empty `HOLLER_STATE_DIR`, and no `holler-cli` file changed on this branch.
- **Hedged:** AC 9's literal grep over all of `tests/` matches the AC 6f socket-flag test in `fake_tmux_test.rs`. I accepted the W-20 scoping journalled at RED rather than move the test, and noted it for S.
- **Evidence:**
  - `cargo test -p holler-adapter-host`: 28 passed. `-- --ignored` on tmux 3.7c: 9 passed, run twice. No `/tmp/hlr-tmux-*` and no `demo-*` session was left.
  - 7 mutants of F's source were each killed and then restored with `git checkout`: `no such window`, `env --`, `#` doubling, `LC_ALL=C`, the ownership filter, the `pane_dead` filter and the W-14 group probe.
  - Flake check: 24 parallel and 10 sequential runs, no failure.
  - Clean: lint.sh, changelog-check, clippy `-D warnings`, rustfmt, machete, wire_selftest, docs_cli_test and test-hooks. Workspace: 1407 passed and 4 failed (environmental), matching F. The handoff is `docs/handoffs/641/handoff-T-green.md`.
