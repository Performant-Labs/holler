# Decisions — #676 pane and profile verbs exit 3 on a refusal, 1 on a failure

## A (Phase 3, up-front plan review) — 2026-10-09T10:30:57-06:00
- **Decided:** PASS on docs/handoffs/676-brief.md at 31fec64, with 0 blocks and 5 warns (see handoff-A.md). The plan extends `holler-pane/src/error.rs` (the one `PaneCode` table, `PaneCode::parse`, `is_valid_code`) and does not build a second code table in the CLI. `output.rs::exit_code` is the only exit-code decision point, and every pane/profile path reaches it. Dependency direction holds: no `lib.rs`, manifest, `holler-proto` or golden-file change. I accepted `ErrorClass::exit_code` in the types crate on the strength of decision 4 (#638's envelope helper pairs `ok: false` with 1, 2 or 3 and cannot depend on `holler-cli`) and the `is_valid_code` precedent.
- **Assumed:** #638 calls `class_of` (as the issue says) and needs the class-to-number mapping, not only `ok` versus exit 0. When #649 fills `Wiring::connect`, it may return a refusal-class code (ADR-0021 §10 reads the Herdr version on connect). This is the basis for W-2.
- **Hedged:**
  - W-1 is a warn, not a block. Under the open-code rule every open code exits 3. ADR-0021 I3's post-act mismatch ("exits 1 with a code") has no closed failure code, so #644/#645 must choose between `unavailable`, a closed-set amendment, and exit 3. The gap already existed in the frozen contract (`RefusalCode`/`Refused` are named as refusals), and this story only makes it explicit. The fix is one ADR sentence and one Risks bullet for the operator.
  - W-2 (the `main.rs:253-255` doc says "exit 1") is residue that AC 3's grep misses. It is accurate today, because `connect` cannot fail yet, so it is a warn.
  - W-5 (legacy "refusal" wording at exit 1) is left to #646, because the legacy CLI has no dominant pattern for the word.
- **Evidence:**
  - Files read: `error.rs` (all), `output.rs` (all), `holler-pane/src/lib.rs`, `main.rs:100-125`, `main.rs:220-290`, `pane/wiring.rs:1-80`, `prompt_target.rs:10-95`, `output_api.rs` (all), `error_test.rs` (all), `spec_flags.rs:1-12`, the doc lines in `pane/mod.rs`, `profile/mod.rs` and `args.rs`, ADR-0021 `:160-511`, ADR-0003 (all), the CHANGELOG `[Unreleased]` section, `docs_cli_test.rs` (matcher at `:121`) and `lint.sh:43-52`.
  - Issues read: #676, #633, #638, #644, #645 and #646 via `gh issue view`.
  - Greps: exit-code constants across `holler-cli` (no shared vocabulary), "refused or failed" and exit-1 wording across crates and docs, and `ErrorClass`/`class_of` collisions (none).
  - Sizes: `wc -l` on every touched file.

## T (Phase 4, author / RED) — 2026-10-09
- **Decided:** Authored `crates/holler-pane/tests/error_class_test.rs` (4 tests) and extended `output_api.rs` (3 new tests, 3 updated, header fixed). The exit-code numbers are pinned twice: through `class_of` over all `ALL_CODES` (catches a code added without a class) and through literal spot checks that do not call `class_of` (so a wrong table and a matching wrong test cannot both pass). I rustfmt-formatted `output_api.rs` (it was clean at HEAD).
- **Assumed:** A compile-time RED on the new `class_of`/`ErrorClass` API is unavoidable and accepted by the brief's test plan; the assertion-level RED is shown with a throwaway skeleton in `error.rs`, reverted.
- **Hedged:** `a_malformed_code_is_a_failure` passes against the all-Failure skeleton, so it is a regression guard, not a RED driver; it fails only if F routes malformed codes into the open-code (Refusal) arm. AC 3-5 (text residue, ADRs, CHANGELOG) have no new test; they are covered by the guard commands at Phase 6.
- **Evidence:** error E0432 from both targets as authored; skeleton run: holler-pane 3 failed / 1 passed, holler-cli `output_api` 6 failed / 15 passed (assertion output in handoff-T-red.md); `git status` shows only the two test files changed after the revert.
