# Handoff-A-dup: #642 part 3 - The OpenCode adapter applies and verifies the pane's agent, and sends prompts  (anti-duplication gate)

**Date:** 2026-10-11 (11:29 MDT)
**Branch:** `issue-642-implementation`
**Diff base:** `7284001` (`origin/main`, the merge-base)   **Diff head:** `3f898ca` (T-green)
**Reuse map:** the brief's D2, D5 to D8, D12 and D13 (`docs/handoffs/642-brief.md`), as narrowed by the Phase 3 review (`docs/handoffs/642/handoff-A.md`, warns 2 and 4). This run has no separate `survey.md`.
**Verdict:** PASS

## Summary

PASS, with no `block` and two `warn` findings. F extended every object the reuse map named and built no parallel path:

- **`ProcessEnv`:** `prefix` now delegates to the new `prefix_with`, and `tui_argv` still calls `prefix` (`tui.rs:175`).
- **`Call`:** `unexpected` and its new body-free sibling, `unexpected_status`, share one `answered` format.
- **`/session/status`:** `settled` and `session_activity` read the route only through `status_object` and `running`.
- **`parentID`:** `top_level_ids` and the new `child_ids` read it one way, through `ids_where`.
- **The rig:** `serve` now delegates to `serve_with`, and `Recording` forwards the two new methods.
- **Test fixtures:** all of D12's list, plus the four my plan review added, moved into `tests/support/fixtures.rs`. There is no copy of `config()`, `serving()`, `known_session` or `harness()` in any test file.

`prompt.rs` re-reads `holler-body`'s child-session scope and prompt body as plain JSON. That looks like a duplicate, but the brief justifies it in writing: D7(a), D6, and ADR-0021 section 5's rule that the adapter depends on `holler-pane` alone. So it is a reviewed decision, not drift.

The two warns are small:

1. One status-entry fixture is written in two test files.
2. `lib.rs` and `hermetic_test.rs` are each within 60 lines of the 800-line flag.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `tests/prompt_test.rs:376-378`, `tests/hermetic_test.rs:387-389` | The `retry` status entry `{"<ID>":{"type":"retry","attempt":1,"message":"x","next":1}}` is written twice: as `retrying()` in `hermetic_test.rs`, and inline in `prompt_test.rs`'s status test. `prompt_test.rs`'s `status_of(kind)` (`:248-250`) builds the same `{"<ID>":{"type":KIND}}` shape that `hermetic_test.rs` writes inline (`:375`, `busy`, from the base; `:420`, `idle`, new). Both files already include `support/fixtures.rs`, whose rule is "moved, never copied" (D12). This is test data, not one of the helpers D12's block condition names, so it does not block. | Move `retrying()` and `status_of(kind)` into `tests/support/fixtures.rs`, and use them from both files. It is optional and cheap, and not worth an issue of its own. |
| 2 | warn | `src/lib.rs` (741 lines), `tests/hermetic_test.rs` (746) | Both files are within 60 lines of the overlay's 800-line flag; the lint gate fails at 900. `lib.rs` grew by 138 lines, against D8's "about +60". Every item added is one the brief placed there, so this is not drift: six `Route`s at four lines each, `prefix_with` and `assignment`, `unexpected_status` and `answered`, `status_object` and `running`, `child_ids` and `ids_where`, three one-line delegations, and the corrected crate doc (acceptance 12). `hermetic_test.rs` shrank from 813, as D12 meant it to. | The next brief that adds to either file should name where the new code goes. For `lib.rs`, the natural split is the route table (`:406-468`) and `Call` with its message helpers (`:470-551`, `excerpt`, `one_line`). Every submodule already imports both from the crate root. |

No duplication of a reuse-map object; the extension is clean.

### Checked, no finding

**Source.**

- **`server.rs`.** `AGENT_UNUSABLE` and the private `unusable()` follow the Herdr adapter's `GRID_UNREACHABLE` and `refuse()` (`holler-adapter-herdr/src/plan.rs:39` and `:254-259`). The root re-export is D5's.
- **The submodule seam.** `server::verify_serving` and `prompt.rs` follow `attach.rs`. The crate root delegates in one line per method, and the submodule takes `&OpenCodeHarness` and imports the root's private helpers. The new `Route`s and `OP_*` constants joined the root's two tables.
- **Reused as they were:** `known` and `session_path` (send_prompt's existence check, the same shape as `abort`), `json_of` and `string_at`.
- **`/config` is not read through `status_object`.** The expression is the same, `json_of(..).filter(Value::is_object)`, but `status_object`'s name and doc bind it to `/session/status` (D7(b)'s "one reading of the route"). Reusing it for `/config` would blur that.
- **The "a 200 whose JSON is an array" reads.** There are three (`server.rs:101-102`, `prompt.rs:94-95` and `:121-122`), and each has its own failure rule: `unavailable`, holds nothing, `unavailable`. That is an idiom, not a duplicated object, and it is below this gate's bar.
- **`from_messages` and `busy_or_idle` are new and duplicate nothing.** No message-list reader existed in the crate, and `holler-body`'s is out of reach by ADR-0021 section 5.
- **Choke point.** `send_prompt` is pure transport, with no hold, limit or park (D13). ADR-0021 section 8 records that `pane reset --first` bypasses #646's gate.
- **D9.** The `ports.rs` change is comment-only.
- **Public surface and dependencies.** One new public item, `AGENT_UNUSABLE`. Neither `Cargo.toml` nor `Cargo.lock` changed.
- **Blast radius.** No file outside it changed.

**Tests.**

- **The two new test files** include `fixtures.rs` and `stub.rs`, and define only scenario-local helpers:
  - `agent_test.rs`: `spec`, `pair`, `config_naming`, `server` and `refused`;
  - `prompt_test.rs`: `Scene`, `Answer`, `activity`, `question`, `permission`, `family` and `messages`.
- **Includes.** Both include the stub the way `attach_test.rs:11-13` does.
- **`VAR` and `prompt_body()` deliberately restate wire values.** A test that imported the adapter's own constant could not catch a typo in it.
- **`refused()` / `refusal()` and `spec()` exist in both `agent_test.rs` (`:54`, `:29`) and `real_opencode_test.rs` (`:440`, `:387`).** This follows the base's own split: the real suite does not include `fixtures.rs`, which needs `stub.rs`. At `7284001` it already kept its own `unavailable` and `json_of` apart from `hermetic_test.rs`'s. No follow-up unless a third copy appears.
- **`observed()` is a new generic poll in `real_opencode_test.rs`.** No shared poll is reachable from this crate:
  - `wait_for` is `holler-cli`'s own test support;
  - the rig's `shown_within` is specific to `shown_session`;
  - the test kit has none.

**Docs.**

- **ADR-0021 is updated in the same change.** That covers section 2's decisions 4, 5 and 7 and facts 2 and 7, section 8, the "as built (#644)" paragraph, the three code-table rows and the two Deferred bullets.
- **No stale sentence outside the blast radius.** A repo-wide grep found none still saying that the OpenCode adapter refuses every agent. The one exception is #802's own CHANGELOG entry (`CHANGELOG.md:425`), a record that was true when written.

## Notes for F

None required: the verdict is PASS. Warn 1 is a five-minute fold if anyone touches the two test files again. Warn 2 is for the next brief that adds to `lib.rs` or `hermetic_test.rs`.

## Patterns referenced

1. `crates/holler-adapter-opencode/src/lib.rs` at `7284001` and `3f898ca`: `ProcessEnv::prefix`, the `Route` and `OP_*` tables, `Call`, `known`, `settled`, `top_level_ids`, `json_of`, `string_at`; and `src/attach.rs:1-40`, the submodule seam.
2. `crates/holler-adapter-herdr/src/plan.rs:39` and `:241-259`: an adapter-owned open refusal code and its private constructor.
3. `crates/holler-adapter-opencode/tests/hermetic_test.rs` at `7284001` (the fixtures D12 moved), `tests/attach_test.rs:9-13` (the stub include), and `tests/real_opencode/rig.rs:329-490` (`raw`, `serve`, `shown_within`).
4. `crates/holler-body/src/http_attach_driver/connection.rs:277-283`, `:310` and `:375`: the bare `prompt_async` route, `refresh_child_sessions` and `session_in_scope`, re-read as plain JSON per D6, D7(a) and ADR-0021 section 5.
5. `crates/holler-cli/tests/support/mod.rs:151` (`wait_for`, a test-support helper in another crate) and `crates/holler-pane-testkit/src/harness.rs` (no poll helper): why `observed()` duplicates nothing reachable.
