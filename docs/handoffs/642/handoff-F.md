# Handoff-F: Phase 6 - The OpenCode adapter applies and verifies the pane's agent, and sends prompts (#642 part 3)

**Date:** 2026-10-11 (11:08 MDT)
**Branch:** `issue-642-implementation` (tip `3d9ef9a`, base `origin/main` `7284001`)
**Issue:** #642 (part 3)

## What was done

- `crates/holler-adapter-opencode/src/server.rs`: `AGENT_UNUSABLE` (D5). `serve_command` adds the one `OPENCODE_CONFIG_CONTENT={"default_agent":KEY}` pair for a spec with an agent (D2). `verify_serving` asks `GET /config` and then `GET /agent` (D4). `refuse_an_agent` and the `AGENT_UNSUPPORTED` import are deleted (D8).
- `crates/holler-adapter-opencode/src/prompt.rs` (new, D8): `send_prompt` (D6) and `session_activity` (D7). The latter covers held questions and permission requests of the session or a direct child, then the status route, then the message-list fallback.
- `crates/holler-adapter-opencode/src/lib.rs`:
  - `mod prompt` and `pub use server::AGENT_UNUSABLE` (D5);
  - `OP_VERIFY_SERVING`, `OP_SEND_PROMPT` and `OP_SESSION_ACTIVITY`, with the table's comment naming the two that have no `HarnessOp` yet (D8);
  - six new `Route`s (D8);
  - `ProcessEnv::prefix_with`, with `prefix()` calling it with no pair (D2);
  - `Call::unexpected_status`, the body-free sibling of `Call::unexpected`, which share one `Call::answered` format (D6);
  - `status_object` and `running`, the one reading of `/session/status`, now used by `settled` too (D7(b));
  - `child_ids` beside `top_level_ids`, both on one `ids_where` reader of `parentID` (A's W-2);
  - the three trait methods wired;
  - the crate doc, `ProcessEnv`'s, `prefix`'s, `settled`'s and `SETTLE_POLL`'s docs corrected (acceptance 12, A's W-5).
- `crates/holler-adapter-opencode/src/tui.rs`: the `tui_argv` doc only (acceptance 12).
- `crates/holler-pane/src/ports.rs`: the `AGENT_UNSUPPORTED` doc only (D9, comment-only).
- `docs/adr/ADR-0021.md`:
  - section 2: decisions 4, 5 and 7, and facts 2 and 7 of "`HarnessPort` as built";
  - the "Launch and relaunch as built (#644)" sentence;
  - section 8, the prompt and activity seam, with D13's sentence and A's W-5 permission note;
  - the three code-table rows (the `pane launch`/`pane relaunch` row, `profile apply`, and the `Refused` class);
  - the two Deferred bullets.
- `CHANGELOG.md`: an `## [Unreleased]` Enhancements entry linking #642.
- `docs/handoffs/642/evidence.md` (new): the evidence appendix.

## Design decisions

- **`prefix_with` appends the extra pairs after both prefix forms** (`env -u X K=V`, `env -i A=1 K=V`). That is D2's placement, so `env`'s in-order assignment lets the adapter's value win. `prefix()` delegates with `&[]`, so `tui_argv` and the no-agent server argv are byte-for-byte as before (#802's pin stays green). The pair is built with `serde_json::json!(...).to_string()`, which is compact JSON.
- **The `verify_serving` order is D4's:** `/config` first, then `/agent`. One `Call` carries `OP_VERIFY_SERVING`, so both requests share the call's bound. Within `/agent`:
  - the agent is matched by `name`;
  - a missing string `mode` is `unavailable` (D4(b)), checked before the refusals;
  - then `subagent`, then `hidden == true`.

  This is OpenCode's own order (E33). An absent or `null` `hidden` passes.
- **Each refusal message is exactly D4's reason** (for example `the server's default agent is not plan`). The message names the key and quotes nothing from a reply. I chose not to add the actual default from `/config`, because D4 says the message "carries no body of the reply".
- **Non-JSON answers to `/config` and `/agent` use `Call::unexpected`,** which quotes at most 60 bytes, as D4 requires. Only `prompt_async` uses the body-free `Call::unexpected_status`. I refactored `unexpected` to format through the shared `answered` helper, so the two messages cannot drift. The text of `unexpected`'s messages is unchanged.
- **`session_activity` reads the two lists first, and asks `GET /session` only when an item belongs to another session.** An exact match answers at once. An empty pair of lists skips the request. So a quiet server costs three requests (`/question`, `/permission`, `/session/status`). Within each list, an item with no string `sessionID` is skipped. That is more lenient than `holler-body`'s all-or-nothing typed parse, which favours finding a held item.
- **`running` treats an entry that is not an object as running,** because its `type` is not `idle`. That covers `null` and a bare string, which OpenCode never sends. It errs toward `Busy` and "not settled", the safe side for `pane reset` and `abort`.
- **The message-list fallback reads `info.time.completed` as present when it is non-null,** rather than requiring a number. It finds the last assistant message with `rfind` (clippy's `double_ended_iterator_last` rules out `.last()` on this iterator). No assistant message at all reads `Idle`, per D7(c)'s "otherwise Idle".
- **`child_ids` and `top_level_ids` share one reader, `ids_where`,** so `parentID` is read one way (A's W-2: "they read `parentID` one way"). `top_level_ids` behaves exactly as before: a string `parentID` makes a session a child.

## Reuse / extend-vs-new

I extended the brief's reuse map and created nothing parallel:

- `ProcessEnv::prefix` became `prefix_with`, and `prefix` delegates to it (D2: one prefix builder).
- `server.rs` holds the agent code (D8).
- `Call`, `known`, `json_of`, `string_at` and `session_path` are reused as they are.
- `Call::unexpected` gained a sibling, `unexpected_status` (D6). They share a format.
- `settled` moved onto the shared `status_object` and `running` (D7(b)).
- `top_level_ids` gained a sibling, `child_ids`, through the shared `ids_where` (A's W-2).
- The existing `STATUS` and `LIST` routes are reused (D8).

The one new file, `src/prompt.rs`, is the brief's own decision (D8, which places it to keep `lib.rs` under the 900-line gate). It follows the `attach.rs` seam: the crate root delegates one line per method, and the submodule reaches the root's private helpers.

## Architecture notes for A

- **Layers:** `holler-adapter-opencode` only, plus one comment in `holler-pane/src/ports.rs`. There is no new dependency (`serde_json`'s `json!` was already used), and the adapter still depends on `holler-pane` alone (ADR-0021 section 5).
- **Public surface:** one new public item, the constant `holler_adapter_opencode::AGENT_UNUSABLE`, a `RefusalCode`. Everything else is `pub(crate)` or private.
- **Contract:** the `HarnessPort` contract is unchanged; the adapter now overrides the two default-armed methods. There is no protocol, hub, `holler-body` or CLI change.
- **Behaviour change in an existing method:** `abort` is stricter. A session whose status entry stays `retry`, or has no `type`, is not settled; before, only `busy` was. Acceptance 9 pins this.
- **Line counts:** `lib.rs` is 741 (it was 603; that is above the 600 warning and under the 900 gate), `server.rs` 156 and `prompt.rs` 145. `lib.rs` grew more than D8's "+60". About 45 of the lines are the corrected crate-doc bullets that acceptance 12 asks for.

## Deviations from spec / wireframe

None. Two readings of the brief are worth calling out:

1. **What a list answer that is not a `200` JSON array means.** D7(a) states no rule for `/question` and `/permission` answers like that. I read them as holding nothing, following A's W-2 and T's pinned test `ac8_a_list_that_cannot_be_read_holds_nothing`.
2. **The prompt seam's `unavailable` text.** It reads `POST /session/:id/prompt_async on port P answered <status>, not 204`.

## Tier 1 self-check (incl. tests now GREEN)

All commands ran from the worktree.

- `cargo build -p holler-adapter-opencode --tests`: compiles, with no warnings.
- Hermetic tests, run with `cargo test -p holler-adapter-opencode --test <t>`:
  - `agent_test`: `ok. 11 passed; 0 failed` (T's RED: compile error, then 1 passed and 10 failed);
  - `prompt_test`: `ok. 12 passed; 0 failed` (RED: 0 passed, 12 failed);
  - `hermetic_test`: `ok. 33 passed; 0 failed` (RED: 30 passed, 3 failed);
  - `attach_test`: `ok. 23 passed`;
  - `tui_test`: `ok. 13 passed`.
- Opt-in real OpenCode 1.19.0 and tmux 3.7c: `HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1` gave `ok. 12 passed; 0 failed` in 96.7 s. That run includes:
  - `ac12_the_adapter_passes_the_harness_conformance_suite` (acceptance 10, with case 16 now on the "argv carries the key" branch);
  - `ac19_serve_command_starts_nothing_and_applies_an_agent`;
  - `agent_default_reaches_the_first_message`;
  - `verify_serving_refuses_a_subagent_a_hidden_agent_and_an_unknown_key`;
  - `send_prompt_queues_a_turn_and_session_activity_follows_it`.

  The rig left no server, tmux server or scratch dir behind.
- `cargo clippy -p holler-adapter-opencode -p holler-pane --all-targets -- -D warnings`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`, as CI runs it: exit 0.
- `cargo fmt -p holler-adapter-opencode -p holler-pane -- --check`: clean. `cargo fmt --all -- --check` reports diffs only in `crates/holler-body/src/acp_driver/answerable.rs`. That file is untouched on this branch, so the diffs predate it, and CI runs no fmt step.
- `bash scripts/lint.sh`: exit 0. No file is at 900 lines; `lib.rs` warns at 741, as it already warned at 603.
- `bash scripts/changelog-check.sh`: passes, because the `## [Unreleased]` heading is kept.
- The pipeline's verify command, `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`, exited 0. Across 150 `test result` lines: 2,142 passed, 0 failed and 28 ignored (the opt-in real tests, which I ran opted in above).

## Evidence appendix

`docs/handoffs/642/evidence.md` has nine facts. They cover:

- `known` and `session_reply`;
- `json_of` reading only a 200;
- `Call::send_by`'s error mapping;
- the `Garbled` texts carrying no reply bytes;
- the engine's `verify_serving` call and rollback;
- `AgentKey`'s alphabet and the host's tmux escape;
- `HarnessOp::VerifyServing`;
- the brief's E33 and E32 cuts of OpenCode 1.19.0.

## Tests that look wrong (for T)

None.

## Known issues

- **I stopped an orphaned process left by T's probe.** T's RED-phase live probe (`probe.sh` in this session's scratchpad, `opencode serve --pure --port 48179`) was still running at 11:02 MDT: an orphan with its scratch dir already deleted, inside the rig's port range. Its script, port and start time (10:49 MDT) match `handoff-T-red.md`'s probe of ports 48177-48179. I stopped exactly that server's own process group (it ran under `setsid`) and its orphaned script, and nothing else. It held no state.
- **Not exercised here:** the JSON pair surviving the host adapter's real `tmux new-window` path. The real rig launches the server without tmux (ADR-0021 section 5), so that path was checked only by reading the code: tmux's escape changes only a trailing `;`, and an `AgentKey` cannot contain one (evidence.md). #649's wiring runs it for real.

## Files changed

- `crates/holler-adapter-opencode/src/lib.rs`
- `crates/holler-adapter-opencode/src/server.rs`
- `crates/holler-adapter-opencode/src/prompt.rs` (new)
- `crates/holler-adapter-opencode/src/tui.rs` (doc only)
- `crates/holler-pane/src/ports.rs` (doc comment only, D9)
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`
