# Handoff-T-green: #848 rename-scan-skip-handoffs

**Date:** 2026-10-12
**Branch:** issue-848-implementation
**Issue:** #848
**Handoff-F reviewed:** docs/handoffs/848/handoff-F.md
**Handoff-T-red:** docs/handoffs/848/handoff-T-red.md

## GREEN confirmation
`cargo test -p holler-cli --test wizard_scripts skilltext_w5_rename::` -> 8 passed, 0 failed (was 6 passed, 2 failed at RED). Full `wizard_scripts` binary: 286 passed. `docs_cli_test`: 3 passed.

Spot-check that the tests pin behavior (production predicate temporarily mutated, then restored; `git status` clean afterwards):
- `is_pipeline_scratch` forced to `false`: `rename_scan_skips_pipeline_handoffs_only` FAILS and `no_old_command_or_directory_outside_the_rename_note` FAILS.
- `is_pipeline_scratch` widened to `rel.starts_with("docs/") && rel.contains("handoffs")`: `rename_scan_skips_pipeline_handoffs_only` FAILS (the `handoffs-notes.md` boundary is pinned).
- Removing the call from `scan_files` is a dead-code compile error under the environment's `-D dead-code`, so it cannot silently regress either.

## Tier 1 results
- `cargo test -p holler-cli --test wizard_scripts skilltext_w5_rename::`: PASS (8/8).
- `cargo clippy -p holler-cli --test wizard_scripts -- -D warnings`: PASS (clean).
- `cargo clippy --workspace --all-targets -- -D warnings`: FAIL, but only on `crates/holler-cli/tests/token_load_test.rs:60` (`redundant_iter_cloned`, clippy 1.93). That file is not in this diff (diff vs 6eff9cc touches one `.rs` file); pre-existing and unrelated. Noted, not blocking.
- `bash scripts/lint.sh`: PASS (exit 0, size warnings on unrelated files; the changed file is about 222 lines).
- `bash scripts/changelog-check.sh`: PASS.
- `cargo test --workspace --no-fail-fast`: 166 `test result: ok`, one failure: `remote_admin_test::remote_say_wait_and_hub_query_json_match_local_after_masking` under full-workspace load. It passes 21/21 on three isolated re-runs. This is the #847 flake the brief names; unrelated to this change.
- `cargo test -p holler-cli --test wire_selftest`: PASS (3/3).
- `cargo machete`: not installed here; no dependency change in the diff (tempfile was already a dev-dependency), so not applicable.

## Tier 2 results
- Test quality: one new test names a behavior, runs at the unit tier on a temp tree, covers nested and top-level handoffs plus the component boundary in one test (proportionate, no duplication). PASS.
- Assertion style: deterministic, no sleeps or timing. PASS.
- Secrets: no secret involved. PASS.
- `#[allow]` without issue link: none added. PASS.
- Protocol / goldens / docs commands: not touched. N/A.
- Evidence appendix: `evidence.md` lists the one unchanged-code fact (`is_cli_doc` component semantics, `docs_cli_test.rs:51-55`). I compared the excerpt to source: the comment and function lines match verbatim. PASS.
- No browser surface in this repo. N/A.

## Acceptance criteria status
1. PASS: `rename_scan_skips_pipeline_handoffs_only` (skips any-depth `docs/handoffs/`, still scans `docs/research/handoffs-notes.md`; mutation-checked above).
2. PASS: the module passes, `no_old_command_or_directory_outside_the_rename_note` unchanged in meaning, on the real repo.
3. PASS: this worktree tracks `docs/handoffs/848-brief.md`, which quotes `agent-skills/setup-wizard` (1 occurrence), and the real scan passes. Mutating the skip off makes the real scan fail on it, so the tracked brief is a live scratch file; no extra file was created, nothing to remove.
4. PASS: diff vs main touches only `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs` outside `docs/handoffs/`; the skip is documented with `#462` and `#848` (lines 49-53).

## Blocking issues
None.

## Advisory notes
- Workspace clippy fails on `token_load_test.rs:60` (`redundant_iter_cloned`) independent of this change; worth its own issue if CI's toolchain is not pinned.
- `remote_admin_test` flake reproduced once per full workspace run (twice in two runs), consistent with #847.
