
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
