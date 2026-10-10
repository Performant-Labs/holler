# Handoff-T-green: Phase 6 - #642b the OpenCode adapter, part 2: the TUI side of `HarnessPort`

**Date:** 2026-10-09
**Branch:** issue-642-implementation at 48fcd05 (base `dc300ab`; not rebased: `origin/main` is at `d9eabbb`)
**Issue:** #642 (642b only)
**Handoff-F reviewed:** docs/handoffs/642/handoff-F.md
**Handoff-T-red:** docs/handoffs/642/handoff-T-red.md

Every build ran with `CARGO_BUILD_JOBS=4` on Linux (this machine), with OpenCode 1.18.35 (`/usr/local/bin/opencode`) and
tmux 3.7c.

## GREEN confirmation

`cargo test -p holler-adapter-opencode`:

| Target | RED | GREEN |
|---|---|---|
| `tui_test` | 0/13 | 13 passed |
| `attach_test` | 1/22 | 23 passed (22 authored at RED, plus 1 added in this phase, below) |
| `hermetic_test` (642a) | 30/30 | 30 passed |
| `real_opencode_test`, not opted in | 9 ignored | 9 ignored |

**Flake checks.** I ran the three hermetic files 8 times in sequence and 3 times concurrently (9 processes at once):
all green every time. The timing tests (`ac29d`, `ac30_..._times_out_within_settle`, `ac30e`) never went past their
`settle + 500 ms` bound.

**Real OpenCode, opted in:**
`HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1`

| Run | Result |
|---|---|
| 1 (F's rig) | 9 passed in 55.0 s, AC 12 with all 15 conformance cases |
| 2 (after the rig change below) | 9 passed in 49.9 s |

After each run nothing was left behind: no `/tmp/hlr642r-*`, no listener on 481xx, and no private tmux server. The only
`opencode` processes were the live fleet's (ports 470xx, started before this run), and I did not touch them.

**The tests check behaviour, not implementation.** I mutated F's code eight times, one change at a time, and restored
each with `git checkout`:

| Mutation | Caught by |
|---|---|
| M1 the watch ignores the TUI's port | **nothing at first.** Closed by the new test below |
| M2 no re-`GET` when the TUI dies | `ac30c_..._went_away_is_session_not_found` |
| M3 `exact_target` gives a bare name | `ac28a`, `ac29c`, `ac30a` |
| M4 `parse_query` ignores the session name | `ac27_a_reply_that_is_not_this_sessions_pane_is_no_pane` |
| M5 `select_session` skips the watch | `ac29d` |
| M6 no `remain-on-exit` | `ac30a`, `ac30b` |
| M7 `escape_dir` does not double `#` | `ac30a` |
| M1 again, with the new test | `ac30_a_tui_of_another_server_showing_the_session_does_not_confirm_the_attach` |

### Test changes in this phase (test files only; no production code touched)

1. **`attach_test.rs`: new test `ac30_a_tui_of_another_server_showing_the_session_does_not_confirm_the_attach`.** F's
   design decision 1 makes the watch confirm an attach only for a TUI attached to *the requested port* (I3). Two servers
   share one data directory, so another server's TUI can show the same id. No test covered this: M1 removed the check and
   the whole suite still passed. The new test runs the attach while the pane shows `OC | ses_A`, but attached to another
   port. It expects `timeout { op: "harness.attach_tui" }` within `settle + 500 ms`. It passes on F's code and fails under
   M1. The file is now 678 lines.
2. **`real_opencode/rig.rs`: `free_port()` also requires the port to be bindable** (new private `bindable`, a
   `TcpListener::bind` that is dropped at once). This answers F's "Tests that look wrong" item 2 and Known issues 3: the
   brief's range 48100-48199 lies inside Linux's ephemeral range (32768-60999 here). There, a socket that does not listen
   can hold a port that `refused()` reports as free. The check is stricter than the brief's "refused and not handed out"
   and stays inside the range. The flake F saw (1 in 3 full runs) did not come back in my 2 runs. That is too few runs to
   call it fixed, so this is hardening, not a proven cure.

F's item 1 (AC 30(a) and AC 11a pin the respawn sequence) is about the contract, not a wrong test. See Advisory 1. F's
item 3 (conflicts with #708 on rebase) is a merge-time task. See Advisory 3.

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0, no file at 900 | exit 0. Warnings only: `hermetic_test.rs` 798, `attach_test.rs` 678 | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean (again after my test edits: `-p holler-adapter-opencode --all-targets` clean) | PASS |
| `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's step) | all pass | `--no-fail-fast`: **1540 passed, 0 failed, 14 ignored, exit 0**. Includes `docs_cli_test` and `wire_selftest` | PASS |
| `cargo test -p holler-cli --test wire_selftest` (canary) | pass | passed inside the workspace run | PASS |
| `cargo machete` | clean | no unused dependencies | PASS |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-opencode -p holler-pane --no-deps` | clean | clean | PASS |
| `rustfmt --check --edition 2021` on the crate's `src/*.rs` and `tests/*.rs`, `real_opencode/rig.rs`, and holler-pane's `ports.rs` and `lib.rs` | ok | ok | PASS |
| Opt-in real run (AC 20) | 9 passed | 9 passed, twice | PASS |

**One discrepancy, not caused by this diff.** My first workspace run, which fails fast, stopped at
`holler-cli --test body_run_test`. The failing test was `fresh_hello_and_presence_on_every_reconnect`: "hub did not report
listening within 10s: Disconnected" at `body_run_test.rs:561`, the first `start_hub_at("127.0.0.1:41918")`. It failed 4
times in a row. A hub started by hand at the same moment logged `bind_failed ... 127.0.0.1:41918: Address already in use`,
and yet `ss` showed no listener there, only `TIME-WAIT`s. A few minutes later a hand-started hub bound the port, and the
test passed 10/10 twice. Then the full `--no-fail-fast` run passed. Two facts rule out this diff:
- holler-cli reaches the diff only through `holler-pane`, whose only changes are `//!`/`///` lines.
- The test binds a fixed port inside the ephemeral range, on a machine where other worktrees were running their own
  suites at the time.

So the cause is contention for that fixed port, not this diff. F's handoff reports `clippy --workspace` but not
`cargo test --workspace`, so there was no F result to compare it with.

**F's other reported commands, re-run:** the crate's tests, clippy (crate and workspace), rustdoc, rustfmt, `lint.sh`,
`machete`, the changelog check and the AC 24 grep. All match F's results.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per AC | AC-by-AC table below; mutation table above | PASS (the one gap, M1, closed in this phase) |
| Test quality | Each test names one behaviour and asserts one exact answer (`assert_eq!` or one named variant, never `is_err()` alone). Pure rules sit at unit tier (`tui_test`), and call order and resolver use at integration tier (`attach_test`). The real tier is opt-in. No test duplicates another. The new test is the only one with a TUI on a different port | PASS |
| Proportionality | 13 + 23 hermetic + 9 opt-in tests for 3 methods, 14 pure items and 2 runners. Nothing redundant to prune | PASS |
| Type safety | Rust. No `unsafe` in the crate's `src/` or `tests/`. Every `#[allow]` carries `// #642` (`lint.sh` checks this) | PASS |
| Error paths | Every refusal is pinned: `session-not-found` (404, 400, gone mid-attach), `pane-not-found` as is, `unavailable` for no TUI, missing tmux, failing tmux, an unbound port, a malformed reply or the web app, and `timeout` with each method's own `op` | PASS |
| Secrets and echoes (Risk 3) | `echoes_nothing_it_did_not_author` covers the binary, the directory, the placeholder, the env var and the format, for every `unavailable`. No test prints a raw `#{pane_title}` | PASS |
| Injection and prefix-match (Risk 6) | Grep: the only tmux `Command::new` is in `tui::tmux_command`, and all three `"-t"` in `src/` are followed by `exact_target`. AC 11a-11c and 30(a) pin this. M3 and M7 are caught | PASS |
| Timing (no blind sleeps) | Bounds are invariants (`< settle + SLACK`). Only the real rig's bounded polls sleep. 8 sequential and 3 concurrent repeats were green | PASS |
| Platform (AC 32, 33) | Fixture `100755` (`git ls-files -s`), `#!/bin/sh`. Hermetic files start only the fixture. No socket option beyond `connect_timeout` | PASS |
| File scope (AC 22) | `git diff --name-only dc300ab`: only the allowed paths. `Cargo.lock` adds only `tempfile` to `holler-adapter-opencode` (a dev-dependency). Every `holler-pane` line is `//!` or `///` | PASS (against the merge base; see Advisory 3) |
| Golden files and protocol | Not touched: this changes no wire format | N/A |
| Evidence appendix | I added one entry, `HarnessOp`'s strings, which the timeout assertions rely on. I checked F's seven entries against the source (`http.rs:101-108`, `lib.rs:540-545` and others): verbatim | PASS |
| Browser, Playwright, UI | None in this repo | N/A |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 6 (642b clause) | PASS | `ac6_attach_to_an_unbound_port_is_unavailable_before_the_resolver` |
| 9 | PASS | `ac9_parse_title_...` |
| 10 | PASS | `ac10_attach_port_...` |
| 11a | PASS | `ac11a_every_builder_targets_the_exact_session`, `ac11a_no_builder_passes_a_bare_session_name_...` |
| 11b | PASS | the three `ac11b_*` tests |
| 11c | PASS | `ac11c_tmux_command_names_the_socket_and_drops_tmux_and_tmux_pane` |
| 11d (642b clause) | PASS | `ac11d_attach_to_a_session_reply_that_is_not_that_session_is_unavailable` |
| 11f | PASS | `ac11f_the_tui_argv_under_inherit_and_isolated` |
| 12 | PASS | `ac12_the_adapter_passes_the_harness_conformance_suite`, all 15 cases, real |
| 13-19a | PASS | `ac13` ... `ac19a`, real, opted in, 9/9 twice |
| 20 | PASS | workspace 1540/0; real run 9/9 (OpenCode 1.18.35, tmux 3.7c) |
| 21 | PASS | clippy, rustfmt, no `unsafe`, `lint.sh`, `machete` (Tier 1) |
| 22 | PASS against `dc300ab` | file scope (Tier 2). Re-check after the rebase |
| 23 | PASS (read) | the `CHANGELOG.md` diff: the part-1 sentence is removed and the part-2 entry added. `changelog-check` ok. S audits the wording |
| 24 | PASS | grep finds nothing. Real-rig ports stay in 48100-48199. No raw title is printed |
| 25 | open, by design | the PR body comes at PR time (O and the script) |
| 26 | PASS (read) | the ADR-0021 and `holler-pane` doc diffs. S audits the wording |
| 27 | PASS | the three `ac27_*` tests |
| 28a-e | PASS | `ac28a` ... `ac28e` |
| 29a-e | PASS | `ac29a` ... `ac29e` |
| 30a-f | PASS | `ac30a` ... `ac30f`, plus `ac30c_..._went_away`, `ac30_attach_of_an_unknown_session_...`, `ac30_an_attach_the_title_never_confirms_...` and the new `ac30_a_tui_of_another_server_...` |
| 31 | PASS | `on_refused_port` in `stub.rs`. The real rig's `serve` retries on "is in use", and `free_port` now also checks the port is bindable |
| 32 | PASS | platform check (Tier 2) |
| 33 | PASS | `ac33_the_fixture_is_an_executable_posix_sh_script`, mode `100755` |

## Blocking issues

None. F's code meets every 642b acceptance criterion as the brief writes it, and nothing in it has to change for this
story.

## Advisory notes

1. **Must land before #644's relaunch uses `attach_tui`: a respawned pane keeps its old title (F's Known issues 1),
   reproduced here.** I ran it on a private `tmux -L` server on tmux 3.7c, killed afterwards:
   - `select-pane -T 'OC | ses_X'`, then `respawn-pane -k -- sleep 3601`: the title still reads `OC | ses_X`, while
     `#{pane_start_command}` already reads `sleep 3601`.
   - The same respawn with `\; select-pane -t =probe: -T ''` chained on: the title is empty.

   So `attach_tui(pane, P, S)` on a pane whose TUI already showed `S` on port `P` can answer `Ok` on its first poll,
   before the new TUI has started. That breaks I3 ("trusted only once seen") in exactly this re-attach case. A fresh pane
   is not affected: its title is the machine's host name, as the brief's Risk 1 assumed. Nothing calls the adapter
   yet (#649), so 642b has no user-visible failure. The fix changes the brief's contract (`respawn_args`' vector, AC 11a,
   and AC 30(a)'s call sequence), so it belongs to O and S, not to F or T alone. It should go in as an amendment or a
   follow-up issue that blocks #644. When it does, T updates those two pins and adds a test where the pane already shows
   the session.
2. **A's W-1 is still open** (`=<name>:` is the session's *current* window). It is recorded in the crate docs and ADR fact
   1, and also has to land before #644 runs against real tmux.
3. **Rebase.** The branch is 5 commits ahead of and 7 behind `origin/main` (`d9eabbb`). #708 rewrote `closed_port` in
   `tests/support/stub.rs` and one line of `tests/hermetic_test.rs`, which collides with T's AC 31 rewrite. Keep T's
   `on_refused_port` and fold in #708's held-connection idea. The CHANGELOG and ADR-0021 entries also need a rebase merge
   (Risk 8, W-7(c)). After the rebase, re-run AC 22 against `origin/main`, and the hermetic files.
4. **`body_run_test`'s fixed ports (41917, 41918) sit inside the ephemeral range** and failed here while other worktrees'
   suites were running. This is outside #642 and not a regression. Worth an issue if it recurs in CI.
5. The follow-ups the brief already names (P-2 macOS socket timeouts, P-3 `FakeHarness` parity, #696's shared runner) are
   still open and outside this phase.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.

## Test-only rework (S round, #678)

S's one REWORK item was the operator's machine name in this handoff (lines 9 and 162). Both are replaced with
neutral phrases ("on Linux (this machine)", "its title is the machine's host name"). No test and no production code
changed, so no suite re-run is needed for this round; the GREEN results above stand. `git grep -i` over the branch
tree finds no remaining occurrence. The name still lives in commit 5d20f61, so O or the run's agent must fold this edit
into it (`git commit --fixup=5d20f61`, then the rebase onto origin/main with `--autosquash`) before the first push.
