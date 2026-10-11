# Handoff-T-red: #642 part 3 - The OpenCode adapter applies and verifies the pane's agent, and sends prompts

**Date:** 2026-10-11 (10:50 MDT)
**Branch:** `issue-642-implementation` (tip `3129ef8`, base `origin/main` `7284001`)
**Brief / wireframe reviewed:** `docs/handoffs/642-brief.md` (re-cut at `6752b70`), `docs/handoffs/642/handoff-A.md`. Wireframe: N/A (no UI surface).

## A precondition

Confirmed: A returned **PASS** on the plan (round 2, `3129ef8`), with 0 blocks and 5 warns. These tests act on warns 1 to 4. Warn 5 is docs work for F.

- **W-1.** No hermetic conformance run is built. Acceptance 10 is the opt-in `ac12_the_adapter_passes_the_harness_conformance_suite`. The hermetic stand-in for case 16 is acceptance 1's whole-argv pin.
- **W-2.** A `/question` or `/permission` answer that is not a 200 JSON array means nothing is held. A test pins this: `ac8_a_list_that_cannot_be_read_holds_nothing`.
- **W-3.** Compile RED is allowed at the one new name, following #808's precedent. Behaviour RED is then proven with a throwaway constant (below). Only `agent_test.rs` and `real_opencode_test.rs` name `AGENT_UNUSABLE`.
- **W-4.** `ID`, `known_session`, `harness()` and `agent()` moved into `tests/support/fixtures.rs` along with D12's list.

## Tests authored

All of them are in `crates/holler-adapter-opencode/tests/`. The crate does not set `autotests = false`, so the new files need no `[[test]]` entry.

### `support/fixtures.rs` (new, D12)

These fixtures moved out of `hermetic_test.rs` and were not copied:
- `ID`, `SLACK` and `BIN`;
- `quick`, `config`, `harness`, `serving`, `isolated`, `pane_name`, `agent` and `strings`;
- `known_session`, `unavailable`, `timeout_op`, `assert_one_line_naming` and `assert_within`.

Each item carries `#[allow(dead_code)] // #642`.

`stub.rs` changed only in the comment on `json_once`'s allow.

### `agent_test.rs` (new, hermetic unit/stub tier)

| Test | Pins |
|---|---|
| `ac1_an_agent_is_the_servers_default_agent_on_its_argv_under_inherit` | Acc 1 / D2. The exact argv `env -u OPENCODE_DISABLE_TERMINAL_TITLE OPENCODE_CONFIG_CONTENT={"default_agent":"plan"} BIN serve --port P --hostname 127.0.0.1 --pure`. |
| `ac1_under_isolated_the_pair_follows_the_configured_pairs` | Acc 1. The exact argv under `Isolated`, with the pair after the configured pairs. |
| `ac1_the_adapters_assignment_follows_a_configured_one_of_the_same_name` | Acc 1 / D2. A configured `OPENCODE_CONFIG_CONTENT=x` comes before the adapter's pair, which comes before the binary. |
| `ac1_each_key_is_carried_in_exactly_one_element` | Acc 1. For `plan`, `orchestrator` and `a-b_c`: exactly one element carries the key, that element is the pair, and its JSON is exactly `{"default_agent":KEY}`. |
| `ac1_with_an_agent_the_server_prefix_is_the_tuis_plus_one_pair` | Acc 1 / D2, under both envs. The argv is `tui_argv`'s prefix, then the one pair, then the binary. Removing the pair gives the no-agent argv. |
| `ac2_ac3_without_an_agent_nothing_is_applied_and_nothing_is_sent` | Acc 2 and 3. No pair is on the argv, `verify_serving(None)` is `Ok`, and the stub saw no request. The byte-for-byte pin stays #802's unchanged `ac8_serve_command_is_the_server_argv_behind_the_env_prefix`. |
| `ac4_a_server_that_runs_the_agent_verifies` | Acc 4 / D4. `Ok` with exactly `GET /config` then `GET /agent`, for three cases: E21's list (`hidden: null`), an element with no `hidden` key, and `mode: "all"`. |
| `ac5_a_server_that_does_not_run_the_agent_is_refused_agent_unusable` | Acc 5 / D4, D5. Code `AGENT_UNUSABLE.as_str()`, a one-line message with the reason, and no reply body (a marker placed in every reply must be absent). The cases: default `build`, no `default_agent`, absent, subagent, hidden. |
| `ac6_an_html_200_for_config_or_agent_is_unavailable` | Acc 6. An HTML 200 on `/config` or `/agent` is `unavailable`, in one line naming the route, `200` and the port. |
| `ac6_an_agent_without_a_string_mode_is_unavailable` | Acc 6 / D4(b). No `mode`, `null` or `7` is `unavailable`. |
| `ac6_a_refused_or_frozen_server_is_unavailable_or_timeout` | Acc 6 / D8. A refused port is `unavailable`. A frozen server is `timeout` with op `HarnessOp::VerifyServing.as_str()`, within `call + SLACK`. |

### `prompt_test.rs` (new, stub tier)

| Test | Pins |
|---|---|
| `ac7_send_prompt_posts_the_text_part_to_prompt_async_and_a_204_is_ok` | Acc 7 / D6. Exactly `GET /session/:id` then `POST /session/:id/prompt_async`. The body equals `{"parts":[{"type":"text","text":…}]}`, with no `agent`. A `204` is `Ok`. |
| `ac7_an_unknown_session_is_session_not_found_and_nothing_is_posted` | Acc 7. A 404 or a 400 is `session-not-found`, and no post is sent. |
| `ac7_another_answer_to_the_existence_check_is_unavailable_and_nothing_is_posted` | Acc 7 / E29. A 500, or a 200 for another id, is `unavailable` with no post. |
| `ac7_a_post_not_answered_204_is_unavailable_and_never_carries_the_prompt` | Acc 7 / D6. The post is answered with an HTML 200, a 500, or a JSON 200. **Each reply echoes the posted body**, with the prompt within its first 60 bytes. The result must be `unavailable` in one line naming `/prompt_async`, the status and the port. Neither `Display` nor `Debug` may contain the prompt. Today's `Call::unexpected` would quote it and fail this test. |
| `ac7_ac8_a_frozen_server_is_timeout_with_each_methods_op` | D8. The literal ops `harness.send_prompt` and `harness.session_activity`, within `call + SLACK`. The prompt is absent from the timeout. |
| `ac8_a_held_question_or_permission_of_the_session_or_its_child_is_holding_question` | Acc 8 / D7(a). A question or a permission of this session or of a direct child gives `HoldingQuestion`. The status is `busy` in these cases, which also pins that the lists are checked first. |
| `ac8_an_item_outside_the_session_and_its_children_does_not_count` | Acc 8. An item of another session, of a child of another session, or of a grandchild does not count: the status decides, `Busy`. |
| `ac8_with_no_session_list_only_an_exact_match_counts` | Acc 8. When `GET /session` is HTML, an exact match is `HoldingQuestion` and an unlisted child gives `Busy`. |
| `ac8_a_list_that_cannot_be_read_holds_nothing` | W-2. An HTML 200 or a 500 on `/question` or `/permission` holds nothing, and the status decides (`Busy`). |
| `ac8_the_status_entry_decides_busy_or_idle` | Acc 8 / D7(b). `busy`, `retry` and an entry with no `type` give `Busy`. `{}`, an `idle` entry, and another session's `busy` entry give `Idle`. |
| `ac8_the_message_list_decides_when_the_status_is_not_an_object` | Acc 8 / D7(c). When `/session/status` is HTML, a 500 with an object body, or a 200 `[]`, the message list decides: an unfinished last assistant message gives `Busy`, a completed one gives `Idle`. |
| `ac8_with_no_usable_status_or_message_list_it_is_unavailable` | Acc 8 / D7(c). When neither is usable, `unavailable`. |

### `hermetic_test.rs` (changed in place; 813 to 746 lines)

| Test | Pins |
|---|---|
| `ac8_a_spec_with_an_agent_is_applied_and_checked_on_the_server` (replaces `ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies`) | Acc 9. Neither method answers `agent-unsupported` for an agent any more. The argv carries the key, and `verify_serving` asks `GET /config` first. It does not name `AGENT_UNUSABLE` (W-3). |
| `ac5_abort_of_a_session_that_stays_in_retry_times_out` (new) | Acc 9 / D7(b). An abort whose status stays `retry` is not settled, so the call is `timeout` with `harness.abort`. |
| `ac5_abort_settles_once_a_retrying_session_reads_idle` (new) | Acc 9 / D7(b). The status reads `retry` once, then `idle`. The answer is `Ok` only after a second status poll. |

### `real_opencode_test.rs` (opt-in `#[ignore]`) and `real_opencode/rig.rs`

| Test | Pins |
|---|---|
| `ac19_serve_command_starts_nothing_and_applies_an_agent` (changed in place, was `…_refuses_an_agent`) | The argv carries the key and still starts nothing. |
| `agent_default_reaches_the_first_message` | Acc 11. The `plan` server passes `verify_serving`. The no-agent server is refused `agent-unusable` ("the server's default agent is not plan"). A raw `prompt_async` gives a first assistant message with `info.agent` and `info.mode` equal to `plan`, or, on the second server, its own default. That default is read from the raw `/agent` and must not be `plan`. |
| `verify_serving_refuses_a_subagent_a_hidden_agent_and_an_unknown_key` | Acc 11. `explore`, `title` and an unknown key, each on a real server started with it, are refused `agent-unusable` with D4's reason. |
| `send_prompt_queues_a_turn_and_session_activity_follows_it` | Acc 11 / D6, D7. `send_prompt` is `Ok`. While the turn is in `/session/status`, the adapter reads `Busy`. After a raw abort, and once the raw message list shows the assistant message's `time.completed`, it reads `Idle` within `Timeouts.settle`. |

Rig changes (E36):
- `Recording` forwards `send_prompt` and `session_activity`.
- `Rig::serve_with(i, &ServeSpec)` runs `serve`'s one port-retry loop, and `serve(i)` calls it with the default spec.
- The headers of `rig.rs` and `real_opencode_test.rs` no longer say "no prompt is ever sent".

**Departure from the brief's acceptance 11 (a fact found at RED; it needs no amendment).** D11 and acceptance 11 assume the dead-end provider ends a turn with an `error` that the test tolerates. I checked this against OpenCode 1.19.0 with the rig's exact provider config. A refused connection does **not** end the turn:
- The session's status reads `{"type":"retry","attempt":N,…}`, with attempts 1 to 5 seen over 40 s and the backoff growing.
- The assistant message (`agent: plan`, `mode: plan`) exists, but has no `time.completed`.
- A raw `POST /session/:id/abort` answers `true`, `/session/status` becomes `{}`, and the message gets `time.completed`.

So the third test ends the turn with a **raw** abort, as the brief requires (the turn's end is read from the raw server, not from the adapter). It also checks live that a session in `retry` reads `Busy` (D7(b)). The other two tests abort their turns after reading the first message, so no server goes on retrying. The probe also confirmed two things:
- With no `default_agent`, `/config` has no such key, and the turn runs as `build`.
- `prompt_async` answers `204`.

## RED confirmation

Every command ran from the worktree on the unchanged base `src/`.

**Runtime RED.** These compile today and fail on the assertion the feature satisfies.

- `cargo test -p holler-adapter-opencode --test prompt_test`: **0 passed, 12 failed.**
  - The `send_prompt` cases fail with `left: Err(Refused { code: RefusalCode("prompt-unsupported"), … }) right: Ok(())`, or `right: Err(SessionNotFound …)`. The `unavailable` and `timeout` ones fail with `expected unavailable/timeout, got Err(Refused … prompt-unsupported …)`.
  - The `session_activity` cases fail with `left: Ok(Idle) right: Ok(HoldingQuestion)` or `right: Ok(Busy)`. The default answers `Idle`, and every case was built to need a non-`Idle` answer. The `unavailable` case fails with `expected unavailable, got Ok(Idle)`.
- `cargo test -p holler-adapter-opencode --test hermetic_test`: **30 passed, 3 failed.**
  - `ac8_a_spec_with_an_agent_is_applied_and_checked_on_the_server` fails at `!is_agent_unsupported(&built)` (`Err(Refused agent-unsupported)`).
  - `ac5_abort_of_a_session_that_stays_in_retry_times_out` fails with `a session that stays in retry: expected timeout, got Ok(())`.
  - `ac5_abort_settles_once_a_retrying_session_reads_idle` fails with `not settled at retry: ["GET /session/… ", "POST …/abort …", "GET /session/status …"]`. Only one status poll was made.

**Compile RED at the one new name** (W-3, #808's precedent). These are the only errors, so there is no typo, import or setup error:

- `--test agent_test`: `error[E0432]: unresolved import holler_adapter_opencode::AGENT_UNUSABLE`, at `agent_test.rs:18`.
- `--test real_opencode_test`: the same `E0432`, at `real_opencode_test.rs:23`.

**Behaviour RED, proven before F.** I appended a throwaway `pub const AGENT_UNUSABLE` to `src/lib.rs`. It was never staged, and `src/lib.rs` was restored byte for byte afterwards; `git status` shows no change under `src/`. With it in place:

- `--test agent_test`: **1 passed, 10 failed.**
  - Each `ac1_*` fails with `called Result::unwrap() on an Err value: Refused { code: RefusalCode("agent-unsupported"), message: "the OpenCode adapter does not apply an agent yet (#642)" }`.
  - `ac4` fails with `left: Err(Refused agent-unsupported) right: Ok(())`.
  - `ac5` fails with `left: "agent-unsupported" right: "agent-unusable"`.
  - Each `ac6_*` fails with `expected unavailable/timeout, got Err(Refused agent-unsupported …)`.
  - The one pass is `ac2_ac3_without_an_agent_nothing_is_applied_and_nothing_is_sent`, the pin that acceptance 2 and 3 keep the previous behaviour. It is green before and after, by design.
- `HOLLER_TEST_OPENCODE=1 cargo test --test real_opencode_test -- --ignored --test-threads=1` (the four changed or new tests, against real OpenCode 1.19.0 and tmux): **0 passed, 4 failed.**
  - `ac19_…` fails with `unwrap() on Err(Refused agent-unsupported)`.
  - `agent_default_…` and `verify_serving_refuses_…` fail with `serve_command(demo-c1r1, P): Refused agent-unsupported` (in `serve_with`).
  - `send_prompt_queues_…` fails with `left: Err(Refused prompt-unsupported) right: Ok(())`.
- `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings`: clean.

**Unchanged and still green:**
- `attach_test`: 23 passed.
- `tui_test`: 13 passed.
- `real_opencode_test`'s `ac13` and `ac17`: run opted in after the rig change, both pass, so `serve` works through `serve_with`.

**Other checks:**
- `cargo fmt --check` passes.
- `bash scripts/lint.sh` exits 0. Every file is under 900 lines (the largest test file, `hermetic_test.rs`, is 746), and every new `allow` carries `// #642`.

## Ready for F

**RED is valid.** F may implement against these tests. Notes for F:

- **The constant.** Add `AGENT_UNUSABLE` in `src/server.rs` and re-export it at the crate root (D5). The two compile-RED crates then reach the runtime failures shown above.
- **What makes `ac7_a_post_not_answered_204_…` pass.** It needs the body-free sibling of `Call::unexpected` (D6). A message that names `/prompt_async`, the status and the port, and quotes nothing, passes.
- **`/question` and `/permission`.** A refused connection or a timeout stays an error. Any other non-array answer holds nothing (W-2).
- **The rig.** Its forwarding and `serve_with` are in place, so no test-support change is needed for acceptance 11.
- **No test asserts the doc edits** of acceptance 12 or W-5. S audits those.
