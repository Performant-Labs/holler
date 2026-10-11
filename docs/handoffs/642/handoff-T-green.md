# Handoff-T-green: #642 part 3 - The OpenCode adapter applies and verifies the pane's agent, and sends prompts

**Date:** 2026-10-11 (11:23 MDT)
**Branch:** `issue-642-implementation` (tip `437fbc3`, base `origin/main` `7284001`)
**Issue:** #642 (part 3)
**Handoff-F reviewed:** `docs/handoffs/642/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/642/handoff-T-red.md`

F changed no test file. `git diff 3d9ef9a 437fbc3` touches only `src/`, `holler-pane/src/ports.rs` (a doc comment), ADR-0021, `CHANGELOG.md` and the handoff files. F listed no "tests that look wrong", and I found none, so no test was repaired in this phase.

## GREEN confirmation

`cargo test -p holler-adapter-opencode` (hermetic):

| Test target | RED (Phase 4) | GREEN (now) |
|---|---|---|
| `agent_test` | compile RED at `AGENT_UNUSABLE`; with a throwaway constant, 1 passed and 10 failed | `ok. 11 passed; 0 failed` |
| `prompt_test` | 0 passed, 12 failed | `ok. 12 passed; 0 failed` |
| `hermetic_test` | 30 passed, 3 failed | `ok. 33 passed; 0 failed` |
| `attach_test` | 23 passed | `ok. 23 passed` |
| `tui_test` | 13 passed | `ok. 13 passed` |

Opt-in real OpenCode 1.19.0 and tmux 3.7c, run with `HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1`:

- **Second full run: `ok. 12 passed; 0 failed`** in 135.2 s. That includes:
  - `ac12_the_adapter_passes_the_harness_conformance_suite` (acceptance 10);
  - `ac19_serve_command_starts_nothing_and_applies_an_agent`;
  - `agent_default_reaches_the_first_message`;
  - `verify_serving_refuses_a_subagent_a_hidden_agent_and_an_unknown_key`;
  - `send_prompt_queues_a_turn_and_session_activity_follows_it`.
- **First full run: 11 passed, 1 failed.** `send_prompt_queues_…` panicked at `real_opencode/rig.rs:442`, which is the rig's `launch_on` (spawning the server, or its 30 s boot health wait). It did not fail on an assertion about the adapter. That test then passed 3 of 3 times alone (about 1.4 s each), and the whole suite passed on the next run. See the advisory notes.

### Spot-check: tests pin behaviour, not implementation

I mutated `src/` once per behaviour and ran the relevant targets. Every mutation turned at least one test red. `src/` was restored after each, and `git status` shows no change under `crates/`.

| # | Mutation of F's code | Caught by |
|---|---|---|
| M1 | `serve_command` uses `prefix()`, dropping the agent's pair | all five `ac1_*`, and `hermetic_test`'s `ac8_a_spec_with_an_agent_is_applied_…` |
| M2 | `verify_serving` skips the `/config` `default_agent` check | `ac5_a_server_that_does_not_run_the_agent_is_refused_agent_unusable` |
| M3 | `usable` ignores `hidden: true` | `ac5_…` |
| M4 | `usable` accepts a `subagent` | `ac5_…` |
| M5 | `send_prompt` uses `Call::unexpected`, which quotes the body | `ac7_a_post_not_answered_204_is_unavailable_and_never_carries_the_prompt` |
| M6 | `running` counts only `busy`, not `retry` or a missing `type` | `ac8_the_status_entry_decides_busy_or_idle`, and `hermetic_test`'s `ac5_abort_of_a_session_that_stays_in_retry_times_out` and `ac5_abort_settles_once_a_retrying_session_reads_idle` |
| M7 | `holds_a_question` never consults the child sessions | `ac8_a_held_question_or_permission_of_the_session_or_its_child_is_holding_question` |
| M8 | `child_ids` counts any session with a parent, so grandchildren and other sessions' children count | `ac8_an_item_outside_the_session_and_its_children_does_not_count` |
| M9 | no message-list fallback; a status that is not an object reads `Idle` | `ac8_the_message_list_decides_when_the_status_is_not_an_object`, `ac8_with_no_usable_status_or_message_list_it_is_unavailable` |
| M10 | `/permission` is not read | `ac8_a_held_question_or_permission_…` |
| M11 | `send_prompt` skips the existence check | three `ac7_*` (`…posts_the_text_part…`, `…unknown_session…`, `…another_answer_to_the_existence_check…`) |

The crate denies dead code. So M5 and M10 kept the unused item referenced (`let _ = …`), so that they built and the tests, not the compiler, did the catching.

## Tier 1 results

All commands ran from the worktree.

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Pipeline verify command | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0. Across 150 `test result` lines: 2,142 passed, 0 failed, 28 ignored. This matches F's numbers exactly. | PASS |
| Clippy, as CI runs it | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. It warns only on files already over 600 lines, including `lib.rs` at 741 (it warned at 603 before). No file is at 900. | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Unused dependencies | `cargo machete` | none | none | PASS |
| Format, changed crates | `cargo fmt -p holler-adapter-opencode -p holler-pane -- --check` | clean | clean | PASS |
| Real OpenCode, opt-in | see GREEN above | 12 passed | 12 passed on the second full run; one launch flake on the first | PASS (advisory) |

There is no server or API smoke for this story: there is no CLI, hub or wire change, and nothing wires the adapter in yet (#649). The real-OpenCode suite is the end-to-end check.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of each acceptance criterion | The table below | PASS |
| Test quality (`test-quality.md` §7) | **Each test names a behaviour:** the `acN_` prefixes, plus the four named real tests. **Each fails in isolation for the right reason:** the RED handoff and M1 to M11. **Cheapest tier:** the stub server for HTTP behaviour, and real OpenCode only for what a stub cannot prove (acceptance 11). **No duplicates:** acceptance 2's byte-for-byte pin stays #802's single test, and `agent_test` only adds the with-agent shapes. **Proportionate:** 23 new hermetic tests across about 8 behaviours. I found nothing to delete or merge. | PASS |
| Type safety | Clippy `-D warnings` is clean. The new public item is one typed `RefusalCode` constant; everything else is `pub(crate)`. | PASS |
| Error handling | These paths each have a test: refused, frozen (timeout with the method's `op`), HTML 200, a non-204 post, a missing `mode`, an unreadable list, and a status that is not an object. | PASS |
| Secrets and echoes | `ac7_a_post_not_answered_204_…` asserts the prompt is absent from both `Display` and `Debug`, against a server that echoes the posted body. `ac5_…` asserts that no reply-body marker appears in a refusal. The `env` pair carries only `{"default_agent":KEY}`. The new `prefix_with` doc says never to put a secret there. | PASS |
| Data integrity and migration | No store or schema is touched. | N/A |
| API and protocol contract | `HarnessPort` is unchanged. There is no protocol, golden file or `v2.md` change. | PASS |
| Evidence appendix | `evidence.md` (F's nine facts) covers every unchanged-code fact the tests rely on: `known`, `json_of` reading only a 200, `send_by`'s mapping, the `Garbled` texts, the engine's `verify_serving` call and rollback, the `AgentKey` alphabet, and `HarnessOp::VerifyServing`. I checked four of its `file:line` excerpts against source (`lib.rs:488`, `lib.rs:574`, `holler-pane-testkit/src/harness.rs:47`, `argv.rs:138`); they match. I appended nothing. | PASS |
| Size and allow rules | `lint.sh` passes. Every new `#[allow]` in the tests carries `// #642`. | PASS |
| Browser or visual | None in this repo. | N/A |

## Acceptance criteria status

| Acc | Status | Backing test(s) |
|---|---|---|
| 1. The agent is on the server argv as `OPENCODE_CONFIG_CONTENT={"default_agent":KEY}`, after the prefix | PASS | the five `ac1_*` in `agent_test` |
| 2. Without an agent, the argv is unchanged | PASS | `ac2_ac3_without_an_agent_…`, and #802's `ac8_serve_command_is_the_server_argv_behind_the_env_prefix` |
| 3. Without an agent, `verify_serving` is `Ok` and sends nothing | PASS | `ac2_ac3_without_an_agent_…` |
| 4. A server running the agent verifies (`/config`, then `/agent`) | PASS | `ac4_a_server_that_runs_the_agent_verifies` |
| 5. A server not running it is `agent-unusable`, quoting no body | PASS | `ac5_a_server_that_does_not_run_the_agent_is_refused_agent_unusable` |
| 6. Non-JSON, a missing `mode`, refused or frozen: `unavailable` or `timeout` | PASS | the three `ac6_*` |
| 7. `send_prompt` | PASS | the four `ac7_*`, and `ac7_ac8_a_frozen_server_…` |
| 8. `session_activity` | PASS | the seven `ac8_*` in `prompt_test` |
| 9. No `agent-unsupported` any more; `abort` treats `retry` as running | PASS | `hermetic_test`'s `ac8_a_spec_with_an_agent_is_applied_…` and both `ac5_abort_…retry…` |
| 10. The conformance suite passes, case 16 on the "argv carries the key" branch | PASS | `ac12_the_adapter_passes_the_harness_conformance_suite` (opt-in, real) |
| 11. Against real OpenCode: the agent reaches the first message; subagent, hidden and unknown are refused; a prompt and its activity | PASS | `agent_default_reaches_the_first_message`, `verify_serving_refuses_…`, `send_prompt_queues_…` (opt-in, real) |
| 12. Doc corrections (crate doc, `tui_argv`, `ports.rs`, ADR-0021) | No test, by design (the RED handoff) | S audits these. I read the doc diffs in `lib.rs`, `tui.rs` and `ports.rs`, and they match the new behaviour. |

## Blocking issues

None.

## Advisory notes

- **The opt-in real rig can flake when the machine is loaded.** The first full run lost `send_prompt_queues_…` at the rig's `launch_on` (`rig.rs:442`). The load average was about 9 to 20 at the time, with the parallel #644 and #649 pipeline runs building. The panic text was not captured. The test passed 3 of 3 times alone, and the full suite passed on the next run.
  - It is not caused by this diff: `launch_on` and its 30 s `BOOT` bound are unchanged by this story, and only `serve_with`'s wrapper is new.
  - It is not a CI risk: the suite is `#[ignore]` and opt-in.
  - If it recurs, the follow-up is to make the rig's launch panic name the cause, then decide whether `BOOT` needs headroom.
- **`lib.rs` is 741 lines,** which is still under the 900 gate. The next adapter story should plan a split before it adds to the crate root.
- **The `cargo fmt --all` drift** in `holler-body/src/acp_driver/answerable.rs` predates this branch, as F noted. CI runs no fmt step.
