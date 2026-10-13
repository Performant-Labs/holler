# Decision journal: #878, pane compact

Run handoff directory `docs/handoffs/878` · branch `issue-878-implementation` · rigor in-session · uiSurface false (D and U
are N/A). Append-only; every phase adds its entry.

## A, up-front plan review: 2026-10-12

- **Decided:** BLOCK, with 1 `block` and 8 `warn` findings (`docs/handoffs/878/handoff-A.md`).
  - **The block:** the engine is placed in `holler-pane` but told to reuse `resolve_pane_record` and the pane-state messages, which live in `holler-cli` (`say_cmd.rs`). The plan can only duplicate them or invert the dependency, and it gives T no engine signature.
  - **The fix:** the CLI resolves the record (the `interrupt_pane` precedent), then calls `tx_compact::compact(ports, &Pane)`.
  - **The warns:** private `mod plan` hides the `pub(crate)` checks; six test files build `Timeouts` by literal; `GET /config` may hold a credential and must be reported with `unexpected_status`; the 120 s bound also covers the cheap reads; `lib.rs` is near 800 lines; a model id may contain `/`; check order hides `compact-unsupported`; ADR section 2 wording.
- **Assumed:** the brief's file and line citations, which I spot-checked against the worktree at `7812ca2` (all matched). I ran no build or test, per the role.
- **Evidence:** read the brief in full; `ports.rs`, `ports/router.rs`, `tx_switch.rs`, `tx_switch/plan.rs`, `say_cmd.rs`, `interrupt_cmd.rs`, `gate.rs`, the OpenCode adapter's `lib.rs`, `prompt.rs`, `server.rs`, `pane/mod.rs`; `grep` for every `impl HarnessPort for` and every `Timeouts {` literal.

## A, up-front plan review, round two: 2026-10-12

- **Decided:** PASS, 0 `block`, 5 `warn` (`docs/handoffs/878/handoff-A.md`, replacing round one's file).
  - **Round one's block is closed:** the CLI resolves the record, the engine `tx_compact::compact(ports, &Pane)` owns health, idle, the act and the shrink rule; all eight round-one warns are taken.
  - **New warns:** `switch::emit_outcome` is switch-typed and cannot be reused (use `output::emit` as `close.rs` does); `call_until` has no request parameter (struct-update `Call` in `compact.rs`); the adapter's second activity read is deliberate; `HarnessOp` lists may need the new variant.
- **Assumed:** the brief's remaining line citations, which I spot-checked (`resolve_pane_record` 384, `check_idle`/`check_health`, `mod plan;` private at 86, `Call`/`call_until`/`Timeouts::default()` at `wiring.rs:229`, `PaneCmd`). No build or test run, per the role.
- **Evidence:** read the amended brief in full; `say_cmd.rs`, `interrupt_cmd.rs`, `tx_switch.rs`, `plan.rs`, `switch.rs`, `park.rs`, `pane/mod.rs`, `args.rs`, the OpenCode `lib.rs` and `prompt.rs`, `ports.rs` (`Ports`), file sizes.

## T, phase 4, author the tests (RED): 2026-10-12

- **Decided:** the RED is valid (`docs/handoffs/878/handoff-T-red.md`, verdict PASS).
  - **Names F cannot guess, fixed by T:** the fake's `seed_context`, `set_compaction_pin`, `set_activity`, `compact_runs`, `CompactRun`, `HarnessOp::Compact`; `CompactOutcome`/`CompactModel` under `holler_pane::ports`; the check order of the fake (it is the order the OpenCode adapter follows). The fake's seeds and `session_activity` stay out of the call log, so no existing exact op-list assertion moves.
  - **Engine tests live in `holler-cli/tests/pane_verbs/compact/engine.rs`, not `holler-pane/tests/`:** `holler-pane` has no dev-dependency on the kit (the kit depends on it), and every other engine's tests are in `holler-cli`.
  - **The CLI-surface rows are part of the verb:** `PANE_VERBS`, `STORY_GROUPS` and `cli-surface.txt` now name `pane compact`; the ADR-0003 row is a docs edit F owns (the one test left failing on the oracle). `unmanaged.rs` now includes `compact`, which makes `unmanaged::missing` part of the verb's contract (not in the brief).
  - **Token source pinned as a stand-in:** the sum of `info.tokens` of the last assistant message; success `200` only. Part B.1 confirms or corrects; the tests' bodies are then replaced by recorded replies.
  - **Mutants, not just cases:** three harness mutants and one Claude mutant; the adapter and wrapper tests were each mutation-checked against an oracle.
- **Assumed:** OpenCode 1.19.0's `summarize` answers `200 true` and its assistant messages carry `info.tokens {input, output, reasoning, cache{read, write}}` (the shape in `fixtures/agents/messages.json` is a real 1.19.0 reply; `summarize`'s status is from the OpenAPI document only, never run). The Claude half stays `compact-unsupported`.
- **Hedged:** `HarnessOp` was grepped for exhaustive lists (A's warn 5): only `HarnessOp::as_str` matches on every variant, and `HARNESS_READS` in the doctor rig is an allow-list that needs no entry. The conformance cases accept `compact-model-unpinned` for a fresh unpinned server, so an opt-in real run with a pin and an empty session would fail case 20 (`unavailable`): not run here.
- **Evidence:** three runs recorded in the handoff (committed state, stub scaffold, oracle scaffold); scaffolds reverted and unstaged, with four emptied untracked files left for F to overwrite (a safety check blocked `rm`). Staged by explicit path: the 24 test and test-support files only. Pre-existing: `token_load_test.rs:60` fails `cargo clippy -D warnings`.

## F, phase 5, implement against the RED: 2026-10-12

- **Decided:** implemented Part A as the brief and T's contract say (`docs/handoffs/878/handoff-F.md`); every authored test is GREEN.
  - The adapter's compaction `Call` is a struct update of `call_until` with `request: timeouts.compact`, opened after the model is resolved (A warn 2); nothing before it gets the 120 s.
  - The verb prints through `output::emit` with its own `Compacted` (A warn 1); `switch.rs` is untouched.
  - The fake keys context and activity by data directory and session, the pin by port; a refused call changes nothing and the seeded pair is not consumed.
  - ADR-0003 gets the `pane compact` row in its own block (T's ask); ADR-0021 decision 50 is the next free number today.
- **Assumed:** OpenCode 1.19.0's `summarize` answers `200` on success and the context size is the sum of the last assistant message's `info.tokens` (T's stand-in, the OpenAPI document; never run).
- **Hedged:** two workspace failures in untouched code (`holler-body` `rebind_test`, `holler-cli` `interrupt_test`) passed when re-run alone; recorded as load-timing flakes, not fixed.
- **Evidence:** the runs in the handoff's Tier 1 section; `docs/handoffs/878/evidence.md`.

## T (phase 7, verify GREEN + Tier 2)
- **Decided:** PASS. The suite is GREEN on F's code; no test changed.
- **Assumed:** the `holler-body` `rebind_test` and `interrupt_test` failures under full-workspace load are flakes, since they pass 3/3 and 11/11 alone and the code is untouched by this branch.
- **Hedged:** `cargo machete` is not installed locally, so it was not run; CI covers it.
- **Evidence:** `docs/handoffs/878/handoff-T-green.md`; two mutations (shrink rule, `rsplit_once`) each failed the named tests and were restored.

## A (phase 8, anti-duplication)
- **Decided:** PASS. F extended `check_health`/`check_idle` (visibility only), `activity_refusal`, `known`, `Call::send`, `unexpected_status`, `session_activity`, `messages`, `resolve_pane_record`, `unmanaged::missing` and `output::emit`; the new files are the ones the brief names.
- **Assumed:** the three-line `Refused` wrapper around `activity_refusal` at three sites is wording that differs per site, not a parallel path.
- **Hedged:** warn that `holler-pane-testkit/src/harness.rs` grew from 692 to 830 lines (under the 900 lint limit); a split is follow-up work.
- **Evidence:** `docs/handoffs/878/handoff-A-dup.md`.
