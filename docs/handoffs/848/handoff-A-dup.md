# Handoff-A-dup: #848 rename-scan-skip-handoffs  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-848-implementation
**Diff base:** 6eff9cc   **Diff head:** 4c23a4c
**Reuse map:** docs/handoffs/848-brief.md (Reuse check, Files)
**Verdict:** PASS

## Summary
F extended the existing scan in `skilltext_w5_rename.rs` (split into `tracked_files` and `scan_files`, `scan_hits` kept as the wrapper) and mirrored `docs_cli_test::is_cli_doc` as the brief decided in writing. No parallel scan, no new module, no `tests/support/` edits. Production code is untouched, and only the one named test file changed outside the handoff docs.

## Findings
| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs:52` | `is_pipeline_scratch` repeats the `starts_with("docs/handoffs")` rule of `docs_cli_test::is_cli_doc` (:53). The brief justified this in writing (separate test binary, one-line predicate, no cross-binary module), and it cites the origin, so this is a deliberate decision, not a block. Reconsider if a third binary needs it. | None now. |
| 2 | warn | `skilltext_w5_rename.rs:38` | `tracked_files` is a `git ls-files -z` helper that is a near-neighbour of the `git ls-files` use in `wizard/preflight.rs:610`. That one is `-s` on named files for a different purpose, so it is not a copy. | None. |

No test-harness helper (`Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`), token-store, `Lockout`/`Roster` or `log` near-copy appears in the diff. The new test reuses the file's existing `tempfile` dev-dependency and its existing `format!("{}-{}", ...)` self-scan-avoidance trick. The file is 222 lines, well below the 800-line threshold.

## Notes for F
None required.
