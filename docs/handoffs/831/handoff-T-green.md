# Handoff-T-green: Phase 7 - #831 A Claude Code harness for panes

**Date:** 2026-10-11
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-claude-harness`, head `881ebe1`, which is F's
phase commit and concludes the merge of `origin/main` at `d6ae539`)
**Issue:** #831 (the PR is "Part of #831")
**Handoff-F reviewed:** `docs/handoffs/831/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/831/handoff-T-red.md`

**VERDICT: GREEN, with no blocking issue.** I repaired the two tests F flagged, as T's work, and changed no production
code. The full CI battery passes. Six behaviour-removal mutants were each killed by the test that pins the behaviour.

## Test repairs (T's, test-only)

F listed both of these under "Tests that look wrong (for T)":

1. **`crates/holler-pane/tests/findings_test.rs`.**
   - `CODES` is now `[&str; 15]` and gains `"pane-waiting"` and `"claude-login-missing"`, in that order, after
     `"unmanaged-stalled"` (contract item 13 appends them).
   - The module doc says "fifteen kinds" and records who appended what (#832, #831).
   - This test came from `main` (#832), so it was not part of T's RED. It failed only because the contract appends
     two kinds.
2. **`crates/holler-cli/tests/pane_claude/mixed.rs`, `say_delivers_a_message_to_each_pane_of_the_scenario`.** This is
   the ignored test, blocked by #839 and #850.
   - The unconditional `panic!` inside the loop tripped `clippy::never_loop`, and the CI Clippy step runs it with
     `-D warnings`.
   - The loop now collects both panes' resolved addresses, still asserting that each one equals the session of record.
     One `panic!` after the loop names both addresses.
   - The test still fails if it is run with `--ignored`, as intended until #839 and #850 land. No `#[allow]` was added.

Both files are rustfmt-clean.

## GREEN confirmation

Command: `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the
pipeline's `test.unit.command`, and CI's step).

- **Total: 2488 passed, 0 failed, 34 ignored**, across 164 test binaries. F's run had 2487 passed and 1 failed. The
  difference is `findings_test`, which is now green.
- Per target:
  - `pane_claude`: 42 passed, 1 ignored (the `say` step);
  - `claude_kind_test`: 15;
  - `fake_claude_test`: 13;
  - `claude_conformance_test`: 4, in 42 s (the mutation case);
  - `fake_harnesses_test`: 5;
  - `holler-adapter-claude` `hermetic_test`: 15, and `real_claude_test`: 1 ignored (opt-in);
  - `findings_test`: 1;
  - `cli_surface_test`: 3;
  - `pane_integration`: 67 passed, 4 ignored (opt-in real), in 119 s.

### Spot-check: the tests pin behaviour, not implementation

This is the check that `handoff-T-red.md` promised: each pinned behaviour was removed from production code, and the
pinning test was run against the result. Each mutant was applied, run and reverted by a script, and `git status`
afterwards shows only T's three files.

| Pin | Mutant (production code) | Pinning test | Result |
|---|---|---|---|
| W1 | `tx_launch.rs:448`: `ProcessSession::Resume` maps to `SessionChoice::Keep` | `lifecycle::switch_to_an_unknown_session_is_session_not_found_and_touches_nothing` | **killed**: the switch succeeded where a refusal was expected (`rig.rs:456`, "the run failed" finds no error) |
| W2 | `reconcile/observe/process.rs`: the idle gate refuses a pane only when it waits on a permission | `doctor::fix_does_not_relaunch_a_busy_claude_pane` | **killed**: `fix` was `"fixed"`, expected `"not-fixable"` (`doctor.rs:124`) |
| W4 | `tx_switch.rs`: `prompt(ports, &record, …)` sends to the old record, not the relaunched one | `lifecycle::reset_first_reaches_the_new_process` | **killed** (`lifecycle.rs:217`) |
| W11 | `tx_launch/process.rs`: `refuse_in_use` always returns `Ok` | `launch::a_launch_into_a_session_that_already_runs_claude_is_process_in_use` | **killed** (`launch.rs:229`) |
| frame | `inbox.rs`: the frame's `content` key is renamed to `text` | `hermetic_test::a_prompt_reaches_the_socket_the_launch_hook_reported_as_one_framed_line` | **killed** (`hermetic_test.rs:566`) |
| W9 | `ports.rs`: the default `login_status` returns `Ok(LoginStatus::Ok)` | `claude_kind_test::every_process_method_defaults_to_process_unsupported_login_status_included` | **killed** (`claude_kind_test.rs:233`) |

Notes on the first attempts:

- My first W1 mutant (at `tx_switch.rs:264`) and my first W2 mutant did not compile, so their "kills" proved nothing.
  I replaced both with the compiling mutants above and re-ran them.
- The 12 conformance cases each carry their own mutant inside `claude_conformance_test` (the 42 s case), and all of
  those are killed as part of the GREEN run.

## Tier 1 results

Reproduces `ci.yml` and `scripts/lint.sh`. Run after the two test repairs, finishing 2026-10-11 about 21:00 MDT.

| Check | Command | Expected | Actual | |
|---|---|---|---|---|
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 (F's `never_loop` error is gone) | PASS |
| Workspace tests | as above | 0 failed | 2488 passed, 0 failed | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0; size warnings only, for files that are not new (`fake_tmux_test.rs` 820, herdr `protocol*.rs`) | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused deps | `cargo machete` | clean | no unused dependencies | PASS |
| Docs parse | `cargo test -p holler-cli --test docs_cli_test` | pass | pass | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | pass | PASS |
| Proto | `cargo test -p holler-proto` | pass | pass | PASS |
| Hooks | `bash scripts/test-hooks.sh` | pass | pass | PASS |
| Pane restart | `bash scripts/tests/pane-restart-test.sh` | pass | pass | PASS |

**Cross-check against F's handoff.** My results match F's, except for the two items F attributed to T's tests
(`findings_test` and `mixed.rs:242`). Both are now fixed.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per criterion | The acceptance criteria table below | PASS, with the exceptions recorded there (item 17, the `say` step, the roster and watchdog) |
| Test quality | Behaviour-named tests. Mutants were killed for the right reason. The `pane_claude` tiers sit at verb-over-fakes, the adapter tier at the seams with no binary, and the kind tier is unit. No duplicate of an existing test was found: the forwarding and `Unwired` tests extend the existing recorders rather than adding new ones | PASS |
| Migration moved no assertion | Compared the assertion and `panic!` counts in each migrated test file between `main` (`d6ae539`) and HEAD. All are equal: `fake_harness_test` 130/130, `real_opencode_test` 60/60, `ports_test` 61/61, `switch.rs` 77/77, `doctor/surface.rs` 58/58, opencode `hermetic_test` 63/63, `attach_test` 56/56, `launch/serve.rs` 107/107. The removed-line diffs are signature-only (`P0` becomes `&at(P0)`, `port` becomes `Some(PORT)`) | PASS |
| One router (W5) | `FakeHarnesses` is `pub struct FakeHarnesses(HarnessRouter<FakeHarness, FakeClaude>)` (`harnesses.rs:22`). The workspace has no other router type | PASS |
| Type safety | The workspace builds clean under clippy `-D warnings` | PASS |
| Error paths | Plan refusals (`port-policy-unsupported`, `agent-unsupported`, `claude-login-env-refused` ×7, `process-in-use`, `harness-change-refused`, `session-not-found`, `session-busy`), `abort-unsupported`, `inbound-held` and `session-mismatch` are each pinned, each asserting that nothing was touched | PASS |
| Data integrity | `HarnessInfo` loads only a record that fits its kind, and an `opencode` record keeps the same bytes (`claude_kind_test`) | PASS |
| API contract | No wire change: `docs/protocol/v2.md` is untouched and no golden file moved (F's handoff, and the diff confirms it). `docs_cli_test` passes | PASS |
| Security | The login answer is never quoted (`an_unreadable_login_answer_quotes_none_of_it`). The 7 login variables are unset and never given a value. The socket directory is `0700` and its files `0600`. A planted `sessions/<pid>.json` is never read. `claude-login-missing` does not contain "auth status" | PASS |
| `#[allow]` links | Every `allow` in the changed crate files carries a `// #NNN` link | PASS |
| File sizes | Every changed file is under 900 lines. `pane_verbs/switch.rs` is at 893 (advisory, below) | PASS |
| Migrations | No schema migration | N/A |
| Browser / Playwright | Not applicable to this repo | N/A |

**Evidence appendix.** I appended one T entry to `evidence.md`: `ErrorClass::exit_code` (usage 2, refusal 3,
failure 1, from unchanged `error.rs:239-245`), which the `pane_claude` exit-code assertions rely on. F's six entries
cover the rest.

## Acceptance criteria status

Brief "Acceptance", and item 18's reading of it:

| Criterion | Status | Backing test |
|---|---|---|
| One OpenCode pane and one Claude pane launched from one profile | PASS | `pane_claude/mixed.rs` (profile apply of a mixed profile; a second apply writes nothing) |
| Both in `pane list` with SHOWN and DRIVEN | PASS | `mixed.rs` acceptance scenario (`shown`, `driven`, `sync` all `ok`) |
| A message delivered to each through `holler say` | **BLOCKED, by design** (#839, #850) | `say_delivers_a_message_to_each_pane_of_the_scenario`, `#[ignore]` naming both. Delivery is proven at `send_prompt` (scenario, conformance case 9, the adapter's frame test) and at `reset --first` |
| Interrupt one (a relaunch on the session of record, the session unchanged, no keypress) | PASS | `lifecycle::interrupt_relaunches_a_busy_pane_on_its_session_of_record`, and the scenario |
| Park one | PASS | the scenario, and `lifecycle` park |
| `doctor` clean | PASS | the scenario, `doctor.rs` (clean mixed grid, W3) |
| The subscription login is never read or printed | PASS | the adapter hermetic login tests, `claude-login-env-refused` ×7, the doctor login finding text |
| The roster and the watchdog see the Claude pane | **S to judge** | Neither reads the registry on `main` (`handoff-T-red.md` "Not authored") |
| Item 17: adopting existing Claude panes | **Not built** | Waits for #835 on `main`. The PR is "Part of #831" |

## Blocking issues

None.

## Advisory notes

- **`crates/holler-cli/tests/pane_verbs/switch.rs` is 893 lines.** `lint.sh` fails at 900. The next story to touch it
  should split it.
- **The opt-in real test was not run** (`HOLLER_TEST_CLAUDE=1 … real_claude_test -- --ignored`). It needs the
  operator's real login and a model turn. These remain unverified against real Claude Code:
  - the hook's `$PPID` key. If it is wrong, delivery fails closed (`unavailable`);
  - the inbox frame;
  - the end-to-end tmux launch.

  S or the operator should decide whether this run is a merge condition.
- **`harness-dialog-open` has no fake state.** It is pinned only by the adapter's mapping (carried over from T-red).
- **`reset_first_reaches_the_new_process` depends on #828 not having landed.** If #828 lands first, the test routes
  back to T.
- **#835 coordination.** F copied #835's hunks verbatim (`claude_code_rule_test`, `foreground_test`, and the host suite
  case 10). Whichever of the two lands second merges identical hunks.
