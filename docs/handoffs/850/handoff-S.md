# Handoff-S: Phase 8 - #850: a body driver for a Claude Code pane  (spec audit)

**Date:** 2026-10-12
**Branch:** issue-850-implementation (`main` at `ef881fc`; head `3f4663c`)
**Issue:** #850
**Handoffs reviewed:** `docs/handoffs/850-brief.md`, `850/handoff-A.md`, `850/handoff-T-red.md`, `850/handoff-F.md`, `850/handoff-T-green.md`, `850/handoff-A-dup.md`, `850/decisions.md`, `850/evidence.md`; the diff `git diff origin/main...HEAD` and the tests it adds.

## A precondition

PASS. `handoff-A.md` (plan review) is `PASS` with six warns, all folded by F (W-1 as a written PATH-only choice, W-2 as a test, W-3/W-4/W-5/W-6 as ADR text; W-5 also as a test). `handoff-A-dup.md` (anti-duplication) is `PASS`.

## T precondition

PASS. `handoff-T-red.md` is `PASS` with RED shown (two declared compile-RED targets for two new public items, the rest RED on an assertion). `handoff-T-green.md` is `PASS` with zero blocking issues: the CI test command exit 0 (2934 passed, 0 failed), lint, changelog check, `docs_cli_test` and `wire_selftest` pass, the timing tests repeated 3x clean, and a mutation (`START_WAIT` 5 s to 1 s) fails `a_turn_too_short_to_see_ends_after_the_start_wait`. The one clippy error (`token_load_test.rs:60`) is in a file this branch does not touch.

## Acceptance criteria

| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| 1 | The row rule (parse, refusals naming the key, UUID form, ACP bridge and OpenCode rows unchanged, one comparison of the id with `claude`) | `holler-body/tests/claude_row_test.rs`: `a_claude_attach_row_with_a_uuid_session_id_parses`, `..._with_an_endpoint_is_refused_naming_endpoint`, `..._with_interrupt_http_is_refused_naming_interrupt`, `..._without_a_session_id_is_refused_naming_session_id`, `a_session_id_that_is_not_a_lowercase_uuid_is_refused`, `a_claude_uuid_never_parses_as_a_routable_name`, `a_claude_spawn_row_is_still_the_acp_bridge`, `an_opencode_attach_row_is_unchanged`, `only_one_function_compares_the_harness_id_with_claude`. Source: `config/claude_row.rs`, `config.rs::attaches_to_claude_code` | MET |
| 2 | The driver over `FakeClaude` (fail-closed attach, one frame and no text, end_turn, short turn, relaunch between and during a turn, vanished process, refused delivery, permission wait, answer, cancel, no `login_status`, bounded stall) | `holler-body/tests/claude_attach_driver_test.rs` (13 tests, each named by the brief). `replace_is_refused_in_flight_and_plain_when_idle` is not a driver-level test, correctly: replace is the session task's cancel-then-prompt, and `claude_session_test::cancel_and_replace_in_flight_are_refused_and_replace_when_idle_is_a_prompt` pins it. Source: `claude_attach_driver.rs`, `claude_attach_driver/turn.rs` | MET |
| 3 | The session manager (advertised shape, omitted until attached, queue, re-attach after `error`, shutdown touches nothing, two rows one port, no port refuses, rebind held/unheld/busy/past deadline) | `holler-body/tests/claude_session_test.rs` (12 tests, including `rebind_to_a_held_session_swaps_...`, `rebind_of_a_row_mid_turn_is_refused_session_busy`, `a_rebind_lookup_past_the_deadline_is_refused_endpoint_did_not_answer`) | MET |
| 4 | Hub path for a typed address: one inbox frame, hold refuses (exit 4), in-flight interrupt refused, roster row, stale row off the roster, support PATH lookup, no login read | `holler-cli/tests/claude_attach_mode_test.rs` (7 tests; real hub, real `holler body run`, fake `claude` program and listener; `[[test]]` entry in `Cargo.toml`). `no_login_is_read_or_printed` asserts only `agents --json` was run, no credential variable reached the program, and a sentinel reached no output or log | MET |
| 5 | #831's `say` step as a default-run test for both harnesses, with holds, offline refusal (`pane-not-driven`), parked gate; the ignored step deleted | `holler-cli/tests/say_pane_scenario_test.rs` (4 tests); `docs_claude_body_test::the_ignored_say_step_of_the_scenario_is_gone`; `pane_claude/mixed.rs` no longer holds the ignored test | MET |
| 6 | The drive rule (call order with a second `cas_put`, reset `--first` through the hub, no-body case, failed rebind reported with the reconcile step, interrupt moves no body, doctor records DRIVEN and reports a body behind, lost second write healed, `driver_of` for a single process; plan-review W-5) | `holler-cli/tests/pane_claude/driven.rs` (all eight named tests plus `a_body_that_drives_an_earlier_session_is_reported_behind_...` and `a_say_between_the_two_writes_is_refused_and_not_delivered`); `holler-pane/tests/driven_single_process_test.rs` (6). Source: `tx_switch.rs::relaunched_and_driven`, `tx_switch/plan.rs`, `reconcile/*` | MET |
| 7 | No hub and no wire change | `git diff --stat origin/main...HEAD` lists no file under `crates/holler-hub/src`, `crates/holler-proto/src` or `crates/holler-proto/tests/golden`; no new `Code`, field or method | MET |
| 8 | Docs: stale phrases gone; `docs_pane_control_test` passes | `holler-cli/tests/docs_claude_body_test.rs` (`the_contract_is_amended`, `the_sentences_a_claude_body_replaced_are_gone`, ...); T-green reports `docs_pane_control_test` and `wizard_scripts` passing | MET |
| 9 | Guards | T-green: CI test command, `scripts/lint.sh`, `scripts/changelog-check.sh` pass; clippy error is pre-existing and in an untouched file; `cargo machete` not installed (neither T nor F could run it; `holler-pane` is used by shipped body code) | MET, with the machete caveat |
| 10 | Wizard text (six places) | `docs_claude_body_test::the_wizard_no_longer_says_a_claude_pane_has_no_body_row`, `the_wizard_still_derives_opencode_rows_only`; diff touches `agent-skills/herdr-setup/SKILL.md` and `docs/setup-wizard.md` as text only | MET |
| W-2 | Exactly one `.send_prompt(` in `holler-body` | `hold_single_path_test::the_body_calls_send_prompt_from_exactly_one_place` | MET |

## Spec compliance

Decisions D1 to D12 are implemented as stated:

- **D1** reuse of the one `HarnessPort`: the driver calls only `locate_process`, `send_prompt`, `session_activity`; `body_cmd` builds the harness with the wiring's `claude_harness` only when a Claude row exists and passes it through `connection::run` and `SessionManager::start_with`. No body-to-adapter edge; `holler-body -> holler-pane` is recorded in ADR-0021 section 5.
- **D2** one row shape, one comparison (`attaches_to_claude_code`), source-scan pinned.
- **D3** `spawn_blocking` under `OP_BOUND` (10 s), poll 500 ms, start wait 5 s; cost and abandoned-thread behaviour stated (W-3).
- **D4** cancel in flight refused with `DriverError::Cancel` naming `holler interrupt --pane NAME`; answer always refused; `interrupt --pane` unchanged (text form of `interrupt_cmd.rs` and `say_cmd.rs` only gets doc updates).
- **D5** the drive rule extended to a single process through `find_driver`, `drive` and a second compare-and-swap; reconcile records DRIVEN for a single process.
- **D6** no new verb; **D7** the hub is unchanged and sees a Claude row as any row; **D8** the roster join untouched; **D9** the support probe is a `PATH` lookup (`claude_code_support`), PATH-only by an explicit, documented choice (plan-review W-1); **D10** the scenario is default-run; **D11** no wire change; **D12** build guards below.
- **N1** the six wizard places are rewritten as text only.

Declared deviations, all disclosed by F or T and judged acceptable:

1. **Contract-first ordering.** The brief and the operator's amend-first decision say the ADR and docs amendment is "one commit, before any test". In the run the docs landed in F's commit, after T's RED commit (`b20dff0`), because Acceptance 8 and 10 are docs tests that must be RED first. F says so in `handoff-F.md` ("The contract was not a separate commit before the tests"). The text is the brief's own, so the content matches. Advisory note 1.
2. **Two existing tests edited**, although the brief said `config_test.rs` stays green unedited: `config_test::presence_doc_lists_every_session_idle` and `pane_verbs/import/join.rs::a_pane_that_is_not_opencode_is_skipped_and_the_others_import` each used an incidental `claude` attach row with an `endpoint`, which the row rule the brief decided now refuses. The only change is the harness id (`codex`); intent unchanged. The brief's "unedited" was an oversight, not a defect that blocks.
3. ADR 0014 point 9 reads "a permission prompt or a question" instead of "or a dialog", because the adapter reports a dialog as the refusal `harness-dialog-open`, not as an activity (a failed read, `error`). Correct against the adapter.
4. `Inner::with_claude` instead of a fifth `Inner::new` argument; keeps `rebind_tests.rs` as it was. Acceptable.

## Quality audit

- **Correctness and failure handling.** Attach fails closed (`DriverError::NotFound`) on no holder, a failed lookup or a lookup past the bound. A failed or timed-out port call ends the turn `error` and the message is never re-sent. A relaunch is `cancelled`, a cleared or moved session `error`. A refused delivery is one chunk with the adapter's own text, then `error`. `shutdown` ends only the driver's own turn task and touches no process. Crash isolation (drop the driver after an `error` turn, re-attach at the next prompt) is the session task's, unchanged and tested (`after_a_turn_that_ended_error_the_next_prompt_re_attaches`). No lost-write concern: nothing new persists; the second DRIVEN write is the existing compare-and-swap and a lost one is healed by doctor (tested).
- **Build guards.** No added `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` or `#[allow]` in `crates/*/src` (grep over the diff's added lines: none). No touched `.rs` file is at 900 lines: largest are `connection.rs` 875 and `task.rs` 802; new source files are 59, 158 and 222 lines. No dead code found; the one-use `record` wrapper in reconcile was removed.
- **Protocol.** No wire change; `docs/protocol/v2.md` section 5.3 (support rule), the presence line and the attach and rebind paragraphs are updated; no golden file changed; error-code table untouched (17 codes).
- **Tests.** Cross-process behavior uses the real hub and `holler body run` (`claude_attach_mode_test`, `say_pane_scenario_test`); the driver and session tests use `FakeClaude`; readiness uses `wait_for`; clock waits are only where the rule is the clock (`START_WAIT`, poll), mutation-checked by T. RED-first evidence is in `handoff-T-red.md`.
- **Documentation.** `CHANGELOG.md` has an `## [Unreleased]` bullet linking #850 (and the #831 bullet's stale sentence is corrected); README gains the "A Claude Code pane" recipe; `docs/pane-control.md`, ADR 0014, ADR-0021 and the wizard text are updated. No new log event, so the README "Debug output" table is correctly unchanged.
- **Public-repository privacy.** Grep of the diff's added lines for tailnet or personal host names, account names, home paths, IP addresses (other than loopback), key patterns: no hit outside the brief's own rule sentence. Examples use placeholders (a UUID, pane name `c1r1`). The fake program prints no credential and a test asserts it.
- **Commit hygiene.** Subjects are Conventional Commits; every commit carries a `Co-Authored-By` trailer. The trailers carry no session link and the pipeline's `chore(#850): ... verdict` subjects are the script's own. The PR does not exist yet, so the AI disclosure check against `CONTRIBUTING.md` is for O after the script opens it (as the repository's CLAUDE.md says). Advisory note 2.

## Scope check

F delivered the brief's scope: `holler-body` (row, driver, session manager injection, support probe), `holler-cli` (`body_cmd`, wiring visibility, doc comments), `holler-pane` (plan, switch, reconcile), the docs and the six wizard text places. No hub, proto, adapter or testkit change. No over-delivery found. The small `check_http_attach` extraction in `config.rs` is necessary to keep `parse` under clippy's line limit and moves the OpenCode rule unchanged. Under-delivery: none against the ten criteria.

## Verdict

PASS

## Advisory notes

1. **Amend-first ordering.** The contract landed with F, not before T's tests. The text matches the brief, and the pipeline's docs-test-RED requirement forced the order, but the operator's "amend-first, before any test" is not literally met in the git history. O should say so in the PR body or accept it.
2. **PR hygiene is O's.** After the script opens the PR, add the AI disclosure `CONTRIBUTING.md` asks for with `gh pr edit`. The commit trailers have no session link.
3. **Operator-facing behavior changes to be named in the PR:** (a) `pane import` of a legacy `sessions.toml` with a `claude` attach row that names an `endpoint` now blocks as an invalid file instead of being skipped (T-red finding 2); (b) the support probe is PATH-only, so a body whose `claude` is named by `HOLLER_CLAUDE_BIN` alone answers `ok:false` to `query/support` while it delivers (documented in v2.md 5.3 and the README).
4. **Known follow-up (F's known issue):** wizard Stage 8 still says the body config holds OpenCode rows "and nothing else", and a rerun rewrites that file, so a hand-written Claude row is not wizard-managed. A later story, or O's decision.
5. **Test fragility:** `a_relaunch_during_a_turn_ends_it_cancelled` has a ~1e-4 per-run window because the fake's relaunch is two calls; an atomic replace in the fake would remove it.
6. **Toolchain:** `cargo clippy --all-targets` fails on pre-existing `token_load_test.rs:60`; `cargo machete` was not run by anyone. CI on its own toolchain is the authority.
