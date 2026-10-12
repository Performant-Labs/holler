# Decision journal: #850 (a body driver for a Claude Code pane)

Run record for the build run. The brief is `docs/handoffs/850-brief.md`.

## A (Phase 3, up-front plan review) — 2026-10-12
- **Decided:**
  - PASS, with 0 blocks and 6 warns (`handoff-A.md`).
  - Kept the preparer's design: a `ClaudeAttachDriver` over the `HarnessPort` that `holler-cli` injects, the `holler-body -> holler-pane` edge (A1), the drive rule extended to a single process, and no hub, wire or golden change.
  - Asked for: a stated choice on whether the support probe honours `HOLLER_CLAUDE_BIN` (W-1); a source guard for the one body-side `send_prompt` caller (W-2); the poll's cost and the abandoned-thread behaviour in D3 (W-3); a stale-row note (W-4); the refusal window of the two drive writes (W-5); the ADR 0014 "from the registry" wording (W-6).
- **Assumed:**
  - The brief's E-excerpts match `main` at `ef881fc` (spot-checked: the driver enum, the task's `rebind`, `driver_of`, the adapter, `claude_bin`, the hold guard).
  - `claude agents --json` costs about 90 ms a run, as the adapter's doc says.
- **Hedged:**
  - W-1: which of the two probe choices is right is the operator's; the hub uses `harnesses_confirmed` only for status and caps, so the harm is a misleading report, not a refusal.
  - W-3: the cost per tick is counted from the adapter's code, not measured.
- **Evidence:**
  - Read in full: `docs/handoffs/850-brief.md` lines 1-365, the driver enum, ADR 0014, ADR 0021 section 5, `tx_switch.rs` (switch, drive, relaunched), `tx_switch/plan.rs`, `driven.rs` (`driver_of`), `hold_single_path_test.rs`.
  - Read in part: `holler-adapter-claude/src/lib.rs`, `holler-pane/src/ports.rs`, `holler-body/src/{config,query}.rs`, `session_manager/task.rs`, `holler-cli/src/pane/wiring/env.rs`, `holler-hub` for harness uses.
  - Commands: `grep -rn '"claude"' crates/*/src`; `cat crates/*/Cargo.toml` for the crate graph; `wc -l` on the touched files.

## T (Phase 4, author / RED) — 2026-10-12
- **Decided:**
  - RED is valid, with one declared caveat (`handoff-T-red.md`): the driver and session-manager tests (Acceptance 2 and 3) are compile-RED, because they name two new public items (`claude_attach_driver`, `SessionManager::start_with`) that no existing item can stand in for. Every other target is RED on an assertion.
  - Fixed the shapes the brief left open, so F builds to them: `ClaudeAttachDriver::attach(&SessionConfig, Arc<dyn HarnessPort>)` with `DriverError::NotFound` for a row nobody holds, `DriverError::Cancel` naming `holler interrupt --pane NAME`, `SessionManager::start_with(&registry, Option<port>)`.
  - Test layering: the cheapest sufficient tier for each rule (a pure parse and a source scan; the driver over `FakeClaude`; the session task and `session/rebind` at the wire; the in-process verbs over the fakes for the drive rule; a real hub and body only for what needs `body_cmd` and the hub's holds and gate). The scenario file replaces the ignored `say` step, which was deleted from `mixed.rs`.
  - Added the two plan-review guards that were asked for: W-2 (exactly one `.send_prompt(` in `holler-body`) and W-5 (a prompt between the two drive writes is refused, never delivered).
  - Edited two existing tests the brief said stay green: `config_test::presence_doc_lists_every_session_idle` and `pane_verbs::import::join::a_pane_that_is_not_opencode_is_skipped_and_the_others_import` both used an incidental `claude` attach row with an `endpoint`, which the row rule now refuses. Their harness id is `codex`; their intent is unchanged.
- **Assumed:**
  - `ClaudeAttachDriver` ends a turn `error` (after one chunk) on a refused delivery and never re-sends; the bound is 10 s per port call in `spawn_blocking`. A throwaway sketch in a scratch copy outside the repository satisfied every new test, repeatably, and was discarded.
  - W-1 is open: the support tests set `PATH` and unset `HOLLER_CLAUDE_BIN`, so they hold under either choice.
  - A mid-replacement poll on the fake (a relaunch is two calls) is a microsecond window against a 500 ms poll; accepted, with the fix (an atomic replace in the fake) named.
- **Hedged:**
  - `pane import` of a legacy `claude` attach row with an `endpoint` now blocks as an invalid file instead of being skipped; whether that is wanted is O's call (finding 2 of the handoff).
  - The wizard test pins the removal of the old sentences, not the words of the replacement; the CHANGELOG test counts bullets that name #850 (three after the story).
- **Evidence:**
  - `handoff-T-red.md` for each command and result; `cargo test --workspace --no-run --no-fail-fast` shows exactly the two compile-RED targets.
  - The scratch sketch: `claude_row_test` 9/9, `claude_attach_driver_test` 13/13, `claude_session_test` 12/12 (5 repeats), `claude_attach_mode_test` 7/7 and `say_pane_scenario_test` 4/4 (3 repeats), `pane_claude` 53/53, `pane_verbs` 407/407; it broke the two existing tests above and `rebind_tests.rs`'s `Inner::new` call sites.
