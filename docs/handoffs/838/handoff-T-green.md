# Handoff-T-green: #838 - Stage 1 asks which Herdr session to build in

**Branch:** issue-838-implementation
**Handoff-F reviewed:** docs/handoffs/838/handoff-F.md
**Handoff-T-red:** docs/handoffs/838/handoff-T-red.md

## GREEN confirmation
`cargo test -p holler-cli --test wizard_scripts`: 285 passed, 0 failed (includes all 9 skilltext_838 tests).
Spot-check: with SKILL.md reverted to origin/main, skilltext_838 gives 7 failed, 2 passed (the 2 are absence/negative pins). File restored; tree clean.

## Tier 1
- scripts/lint.sh: exit 0 PASS
- scripts/changelog-check.sh: ok PASS
- docs_cli_test: 3 passed PASS
- wire_selftest: 3 passed PASS
- cargo machete: not installed locally (N/A)
- clippy: F reported a pre-existing token_load_test.rs:60 failure on the local rustc, file untouched; not re-run.
- Full workspace: F reported two load-sensitive flakes, passing alone, unrelated (no Rust changed).

## Tier 2
Change is text only (skill, docs, changelog); no protocol/golden impact. Test file 219 lines (<900). No secrets involved. PASS.

## Acceptance criteria
Each is backed by a skilltext_838 test (Stage 0 row, Stage 1 question, refusal, outside-a-pane rule, Verify, Stage 3 plan). PASS.

## Blocking issues
None.
