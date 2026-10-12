
## Phase 3 - A, up-front plan review (2026-10-12)
- **Verdict:** BLOCK on the brief as written; the design (one `LinkedHost<H, L>` in `verb_harness`, named propagate/swallow modes) is sound.
- **Why:** AC 1's grep matches `LostChain` and misses a generic impl; AC 5 names `sessions_for` (another crate's helper) and a `scenario.rs` no-hub assertion that do not exist; AC 2 omits `profile_apply_scenario`, which path-includes `scratch/mod.rs` and `out.rs`.
- **Read:** the brief; the four `LinkedHost`s; `LostChain`; `pane_integration/{main,observe,bind,wired,no_hub,fake_hub,scenario,scratch/mod}.rs`; `profile_apply_scenario/main.rs`; `holler-pane-testkit` `host.rs`/`harness.rs`/`harnesses.rs`; `scripts/lint.sh`.
- **Output:** `docs/handoffs/846/handoff-A.md`.

## Phase 3 - A, up-front plan review, second pass (2026-10-12)
- **Verdict:** PASS on the amended brief (`e1eae6a`); the four first-pass blocks are resolved and each fix was re-checked against the tree at `6eff9cc`.
- **Why:** AC 1 greps now match exactly the four `LinkedHost`s (and not `LostChain`); W-3b is dropped; `profile_apply_scenario` is in the AC 2 command with `scratch/`, `out.rs` and `scenario.rs` untouched; the no-hub text is scoped to `no_hub.rs` and `fake_hub.rs` with both assertion strengths kept.
- **Warnings (non-blocking):** say "five" targets, not "four", in AC 2 and the Test plan; update `verb_harness/mod.rs`'s "included by" doc line; keep the `pane_integration` helper dependency one-way; keep `FakeHarnessLink` to named modes; keep `kill_served` defined once and called twice.
- **Output:** `docs/handoffs/846/handoff-A.md` (replaces the BLOCK from `b59e745`; history in git).

## Phase 4 - T, characterisation baseline (2026-10-12)
- **Decided:** No new test file; the RED is the AC 1 greps (4 + 4 duplicates, 2 `opencode-ran` writers) plus a recorded green baseline of the five targets (43 / 71 / 347 / 5 / 118 passed; 1 / 4 / 0 / 3 / 0 ignored) and `wc -l` of the rig files (852 / 776 / 693 / 482 / 321).
- **Assumed:** The pipeline stays on `claude-pipeline-v0.1.7` for this in-flight story; the operator said 0.1.8 is for new stories only and not to restart work.
- **Hedged:** A refactor has no failing behaviour assertion; the mutation check in Phase 6 is what shows the propagate/swallow modes are pinned.
- **Evidence:** `docs/handoffs/846/handoff-T-red.md`; baseline run on `90d62fd`, exit 0.

## Phase 5 - F, the fold (2026-10-12)
- **Decided:** One `LinkedHost<H, L>` in `tests/verb_harness/linked_host.rs` over a `HarnessLink`, with the modes as enums `OnStart { Propagate, Swallow }` and `OnStop { KillServed(&'static [u16]), KillNothing }`; the kill loop exists once as `OnStop::end`, called by `FakeHarnessLink` and the launch rig's private `HookedLink`. The #649 near-copies go to `pane_integration/traps.rs` (OpenCode trap, `NO_HUB`, `OPENCODE_MARK`, `names_the_missing_hub`) and `pane_integration/wrappers.rs` (`CallLog`, `herdr_suite_over`, `unbuildable_lazy_herdr`).
- **Assumed:** No `CHANGELOG.md` line for a test-only refactor, per #800 and #735; no D2 follow-up issue exists yet, so the module doc names it without a number.
- **Hedged:** Only the new `traps.rs` was rustfmt-ed; the edited `pane_integration` files were not rustfmt-clean at the base and were left as they were outside the changed hunks.
- **Evidence:** `docs/handoffs/846/handoff-F.md`; the five AC 2 targets reproduce T's baseline (43/71/347/5/118 passed; 1/4/0/3/0 ignored); the workspace test exits 0; AC 1 greps print one line each; `lint.sh` exit 0.

## Phase 7 - T, verify GREEN (2026-10-12)
- **Decided:** PASS. The five targets reproduce the baseline exactly (43 / 71 / 347 / 5 / 118 passed; 1 / 4 / 0 / 3 / 0 ignored); AC 1, 3, 5, 6 hold by grep, diff and `wc -l`; no test was repaired (F flagged none).
- **Assumed:** The workspace clippy failure at `token_load_test.rs:60` is pre-existing and unrelated (an untouched file, F reported the same); `cargo machete` is CI's, as it is not installed here and no dependency changed.
- **Hedged:** Mutating `OnStart::Propagate` to swallow left every target green, so that mode is unpinned (as at the base). Per the brief no behaviour test was added; it is an advisory note for O. The two kill-side mutations do fail tests.
- **Evidence:** `docs/handoffs/846/handoff-T-green.md`; mutation runs on `linked_host.rs`, reverted (`git status` clean).

## A (Phase 7, anti-duplication)
- verdict: PASS. One LinkedHost, one kill loop (OnStop::end), W-3/W-6/W-7 each folded once. Two warns: stale testkit doc, D2 issue number not filed.
