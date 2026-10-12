# Handoff-S: #848 rename-scan-skip-handoffs

**Date:** 2026-10-12
**Branch:** issue-848-implementation
**Issue:** #848
**Handoffs reviewed:** handoff-A.md, handoff-T-red.md, handoff-F.md, handoff-T-green.md, handoff-A-dup.md, decisions.md, evidence.md; brief docs/handoffs/848-brief.md; diff `origin/main...HEAD`

## A precondition
Met. handoff-A.md and handoff-A-dup.md both show PASS.

## T precondition
Met. handoff-T-red.md records a valid RED (3 hits vs 1 expected); handoff-T-green.md records GREEN (module 8/8, wizard_scripts 286) with "Blocking issues: None" and two mutation checks.

## Acceptance criteria
| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Any-depth `docs/handoffs/` not a hit; `docs/research/handoffs-notes.md` still a hit | `skilltext_w5_rename::rename_scan_skips_pipeline_handoffs_only` (temp tree; asserts exactly 1 hit, and that it is the `docs/research/` file). Mutation-checked by T: predicate forced false and over-widened predicate each fail it. | Met |
| 2 | Module passes; real-repo scan unchanged in meaning | T-green: 8/8. `no_old_command_or_directory_outside_the_rename_note` body unchanged in the diff (only `scan_hits` internals refactored); it was RED at T-red and GREEN now. | Met |
| 3 | Worktree with handoff quoting the old path passes the real scan | The tracked `docs/handoffs/848-brief.md` quotes the old path; real scan RED before the skip, GREEN after. Live evidence, no scratch file needed. | Met |
| 4 | Diff touches only the one test file (plus brief); skip documented with #462/#848 | `git diff --stat`: one `.rs` file outside `docs/handoffs/`. Comment at `skilltext_w5_rename.rs:49-53` cites `docs_cli_test::is_cli_doc`, #462 and #848. A's note on reading "plus this brief" as "outside docs/handoffs/" accepted. | Met |

## Spec compliance
All "decisions already made" are implemented as stated: `tracked_files(root)` and `scan_files(root, rels, needle)` seam; `is_pipeline_scratch` as `Path::new(rel).starts_with("docs/handoffs")`; `scan_hits` is `scan_files(&root(), &tracked_files(&root()), needle)`; the `ends_with("skilltext_w5_rename.rs")` self-skip is kept; no shared helper; new test uses `tempfile::tempdir()` with explicit file list and the `format!("{}-{}", ...)` trick. No deviations.

## Quality audit
- Correctness: skip applied before the file read, inside the pure seam. The unreadable/non-UTF-8 `continue` is preserved. `unwrap` use is confined to a test file (permitted).
- Build guards: no production code; no `#[allow]` added (crates diff has none); file is 222 lines (< 900); no dead code (both new helpers are used).
- Protocol/goldens/CLI surface: untouched; no CHANGELOG or docs entry required for a test-only change (changelog-check passes per T).
- Tests: unit tier, deterministic, no sleeps, RED-first evidence present, mutation-checked.
- Public-repo privacy: grep of the code diff for tailnet names, IPs, personal names, secrets: no hits.
- Commit hygiene: commits have Conventional Commit subjects and the `Co-Authored-By` trailer (pipeline commits carry no session link in the trailer; they carry the standard trailer the hooks add). PR body AI disclosure per CONTRIBUTING.md is for O to add after the PR opens.

## Scope check
Exactly the brief's scope: one test file. No over- or under-delivery. `docs/handoffs/848/` files are pipeline scratch, removed before push per #462.

## Verdict
PASS

## Advisory notes
- `is_pipeline_scratch` duplicates `docs_cli_test::is_cli_doc`; deliberate and documented. Reconsider if a third test binary needs it.
- Pre-existing, unrelated: workspace clippy failure at `token_load_test.rs:60`, and the #847 `remote_admin_test` flake under full-workspace load.
- Remember to strip `docs/handoffs/` before pushing, and check the PR body for the AI disclosure.
