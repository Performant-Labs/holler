
## Phase 3 - A, up-front plan review (2026-10-12)
- **Verdict:** BLOCK on the brief as written; the design (one `LinkedHost<H, L>` in `verb_harness`, named propagate/swallow modes) is sound.
- **Why:** AC 1's grep matches `LostChain` and misses a generic impl; AC 5 names `sessions_for` (another crate's helper) and a `scenario.rs` no-hub assertion that do not exist; AC 2 omits `profile_apply_scenario`, which path-includes `scratch/mod.rs` and `out.rs`.
- **Read:** the brief; the four `LinkedHost`s; `LostChain`; `pane_integration/{main,observe,bind,wired,no_hub,fake_hub,scenario,scratch/mod}.rs`; `profile_apply_scenario/main.rs`; `holler-pane-testkit` `host.rs`/`harness.rs`/`harnesses.rs`; `scripts/lint.sh`.
- **Output:** `docs/handoffs/846/handoff-A.md`.
