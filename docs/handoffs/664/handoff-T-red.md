# Handoff-T-red: Phase 4 - `holler profile apply` (#664), fresh run after A round 4

**Date:** 2026-10-11 (11:15 MDT)
**Branch:** `issue-664-implementation`, head `3e09a1f` (A round 4: PASS), based on `origin/main` `7284001` (#802)
**Brief / wireframe reviewed:** `docs/handoffs/664-brief.md` as of `af230b6`. I re-read §1 to §13, the decisions on O1 to O6, and C18 to C25 in full. Also `docs/handoffs/664/handoff-A.md` (round 4) and `decisions.md`. Wireframe: N/A (`uiSurface: false`).
**First run's RED:** `e773945`. It authored the 38 tests of C14's entries 1 to 33, C6 and C8. Read it with `git show e773945:docs/handoffs/664/handoff-T-red.md`. Those tests are on the branch, and F's first-run code satisfies them. This handoff covers only what the amended brief adds.

## A precondition

Confirmed: A returned **PASS** on the plan in round 4 (0 blocks, 4 warns).

- **Warn 2 (who owns the agent-difference test).** I took A's in-story fix. Entry 11's fixture gains `pane-g`, which runs another agent than its spec names (below).
- **Warn 4 (C25's reconcile step is missing from the ADR sentence and `Step.acted`'s doc).** This is a doc change for F, and S checks it. No test pins doc wording.
- **Warns 1 and 3.** These are brief and Blast-radius edits for O. Neither is T's.

**The baseline.** Before I changed anything, the branch passed `cargo test -p holler-cli --test profile_verbs`: 81 passed, 0 failed. So the merge of #802 and the restore in `3c57c10` left the suite green, which A had left for T to confirm.

## What this round adds, by brief item

Everything is in the existing `profile_verbs` target, in files already on A's 27-path list. There is no new file and no rig change. Tests marked **RED** need new behaviour from F. The rest are **pins**: tests (or assertions in existing tests) of decisions C21, C22 and C25 that F's first-run code already meets. No test checked them before, so a regression would have gone unseen.

| # | Test (file) | Pins | Tier | Kind |
|---|---|---|---|---|
| 1 | `a_refusal_after_the_act_began_names_the_reconcile_step` (`apply/rows.rs`) | **C25.** `fail_next(HarnessOp::VerifyServing, Refused{agent-mismatch})`, run once on a `create` and once on a row f relaunch. In the engine, the step is `refused` with code `agent-mismatch` and the right action, and the run goes on: the next pane is `adopted`. At the verb, both cases exit 0 in both formats, and the step's JSON `detail` and its text line contain `reconcile_step(Some(P))`. Row a's refusal, which ran nothing, must not carry it. | engine + verb (JSON and text) | **RED** |
| 2 | `a_failed_step_that_had_acted_names_the_reconcile_step` (`apply/failures.rs`) | **C25** (ADR-0021 section 8, step 6). `fail_next(HarnessOp::CreateSession, unavailable)` fails a `create` after its act. Through `assert_stopped`: exit 1, `(create)`, and 1 skipped. The O6 message contains the reconcile step. | verb (both formats) | pin |
| 3 | `a_panes_health_and_shown_session_are_observed_at_once` (`apply/observe.rs`) | **C21** (§9, I5): one thread per call. Every harness call is delayed 400 ms. When `health` returns and when `shown_session` returns, each must find the other already in the fake's call log. A chain of the two, in either order, fails this. It is an invariant of the observation's shape, not a duration check. | engine (dry run) | pin |
| 4 | `one_run_takes_every_row_in_grid_order` (`apply/rows.rs`), extended | **C22.** In JSON, the actions are `refuse`, `create`, `adopt`, `report`, `refuse`, `relaunch` for rows a to f. A step object's keys are exactly `{pane, action, outcome, code, diff, detail}`, and pane-c's `code`, `diff` and `detail` are `null`. | verb JSON | pin |
| 5 | `a_dry_run_calls_only_its_reads_and_plans_every_row` (`apply/dry_run.rs`), tightened | **C22.** A dry-run step that would act (rows b, c and f) is `planned`. Before, the test only checked that it was none of the effect outcomes. | engine | pin |
| 6 | `a_per_pane_usage_from_the_relaunch_rules_is_refused_and_the_run_goes_on` (`apply/rows.rs`), tightened | **C22.** The per-pane `usage` code is `spec-not-launchable`. Before, the test only checked that the code was not empty. | engine | pin |
| 7 | `a_failed_health_call_stops_the_run`, `a_dry_run_with_a_failed_health_call_prints_only_the_failure`, `an_unreadable_snapshot_fails_the_first_step_that_needs_it` (`apply/failures.rs`), tightened | **C22.** A step whose observation failed has action `observe`, in the engine and in the O6 message's `(observe)`. Before, these tests passed `action: None`. | engine + verb | pin |
| 8 | `a_dry_run_and_profile_show_agree` (`apply/dry_run.rs`), extended | **C18 / A round 4 warn 2.** New pane `pane-g` is healthy and recorded, and its spec's `opencode_agent` is `review` while its record has none. The loop still compares each `show` row with its plan step, so the case passes before #644 part 2 (both verbs say `matches`, the plan says adopt) and after it (both say `differs`, with the same diff). It never flips, and with #644's own `show` test it covers the agent case by composition. The counts go from 3 to 4. | verb (both verbs) | pin |
| 9 | `there_is_no_budget_over_the_whole_run` (`apply/bound.rs`), re-derived | **C25 timing, entry 27.** With 100 ms per harness call and a **1200 ms** budget (was 900), one create takes about 800 ms: eight calls, which the measured run confirms. That is about 400 ms of room, and two creates (about 1600 ms) do not fit. The doc now says eight calls. | engine | re-derived |
| 10 | `a_step_that_outlasts_its_budget_fails_and_stops_the_run` (`apply/bound.rs`), re-checked | **C25 timing, entry 26.** The numbers stay at an 800 ms budget and 500 ms per call. Under #802 the plan's `serve_command` and port check take 1000 ms, which is past the budget, so the launch times out before its act. The test measured about 1.0 s against its 3 s bound. Only the doc changed. | engine | re-checked |

**Not pinned, on purpose.** These are left to S:
- the "Apply as built" ADR sentence (C22, plus C25 per A's warn 4);
- `Step.acted`'s doc;
- `observe.rs`'s module doc dropping "the shape of `observe_all`" (C21);
- `run.rs` citing `#818` (C23).

Each is a doc wording that S checks against the brief. A test that greps for the words would test the text, not the behaviour.

## RED confirmation

The `src/` tree is exactly F's first-run code; `git diff --stat -- crates/*/src` shows no change from this phase. The command:

`cargo test -p holler-cli --test profile_verbs`: **83 passed, 1 failed.**

```
---- apply::rows::a_refusal_after_the_act_began_names_the_reconcile_step stdout ----
panicked at crates/holler-cli/tests/profile_verbs/apply/rows.rs:572:9:
Create: the detail names "to reconcile, run holler pane doctor --profile 'demo' and then holler profile show 'demo'":
{"pane":"pane-x","action":"create","outcome":"refused","code":"agent-mismatch","diff":null,"detail":"the server on port 48101 does not run the agent the spec asks for"}
```

That is the right reason. The engine half of the test passes: the step is `refused` with `agent-mismatch`, the run goes on, and the verb exits 0 with a valid envelope. It fails only on the C25 assertion, because today the verb adds the reconcile step to a `failed` step alone (`profile/apply.rs:98-101`). The same RED repeated in 10 of 10 runs of `apply::`, with no other failure, so nothing flaked.

**Satisfiable, and every part has teeth.** All of these were throwaway edits to `src/`, restored from copies afterwards. `git status` shows only the five test files changed.

| Edit to `src/` | Result |
|---|---|
| Prototype: the verb adds `reconcile_step` to the `detail` of every `acted` step on the exit-0 path, behind the same "already contains" guard (about 12 lines) | **84 passed, 0 failed** |
| Mutant M1: the reconcile step on acted `create` steps only | test 1 fails (its relaunch half) |
| Mutant M2: the reconcile step on every step that has a code | test 1 fails (row a must not carry it) |
| Mutant M3: no reconcile step on a `failed` acted step | tests 1 and 2 fail |
| Mutant M4: `shown_session` starts 500 ms after `health` (a chained observation) | test 3 fails: `[(Health, false), (ShownSession, true)]` |

**Tier 1 on the test files:**
- `cargo clippy -p holler-cli --tests -- -D warnings` is clean.
- `rustfmt --edition 2021 --check` passes on the story's test files. Each file was formatted on its own; I never ran `cargo fmt` at the root (C24).
- `bash scripts/lint.sh` exits 0. `rows.rs` is 594 lines, under the 600-line warning, and `rig.rs` is unchanged at 759.
- The configured unit command, with `--no-fail-fast` so every binary runs, is `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`. Exactly one target fails: `-p holler-cli --test profile_verbs`, and only because of the RED test. Every other test binary in the workspace passes, so the merge of #802 and the restore in `3c57c10` broke nothing.

## Notes for F

- **C25, the RED.** `Step.acted` is already set for `refused` steps in the engine (`tx_apply.rs`, `halted`). The missing part is in the verb (`profile/apply.rs`): on the exit-0 path, add `reconcile_step(Some(P))` to the `detail` of each acted step, with `failure_body`'s "already contains" guard. The text form prints `detail` through `text()`, which escapes only control characters, so the step prints unchanged there too.
- **The doc work A's warn 4 and C21 to C23 name has no test.** That is the ADR sentence, `Step.acted`'s doc, `observe.rs`'s module doc, and `#818` in `run.rs`. S audits it.

## Notes for my Phase 6 (evidence appendix)

Tests 1 to 3 rely on three facts in unchanged code. The diff-gate reviewer will need each in `evidence.md`:

| Fact | Source |
|---|---|
| `verify_serving` runs inside `server::start`, after `host.run`, so a refusal there is acted | `tx_launch/server.rs:87-107` |
| A launch calls `create_session` after the server is up | `tx_launch.rs` |
| The fault switch logs a call before it sleeps the delay | `holler-pane-testkit/src/fault.rs`, `enter` |

## Ready for F

Confirmed: the RED is valid. One new test fails on the C25 assertion, the behaviour F must add, and on nothing else. The other additions pin C21, C22 and C25 decisions that the first run's code already meets, and four mutants show they have teeth. A throwaway prototype makes the whole target green.

The five changed test files are staged by explicit path. Nothing is committed.

T-red complete, RED is valid. F may implement against the authored tests.
