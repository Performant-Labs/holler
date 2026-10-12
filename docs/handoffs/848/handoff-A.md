# Handoff-A: #848 rename-scan-skip-handoffs  (up-front plan review)

**Date:** 2026-10-12
**Branch:** issue-848-implementation
**Brief reviewed:** docs/handoffs/848-brief.md   **Reuse map:** the brief's "Reuse check" and "Reuse map" sections (no separate survey.md)   **Wireframe:** N/A (no UI)
**Verdict:** PASS

## Summary
The plan is a test-only change to one file: split `scan_hits` into `tracked_files` and `scan_files`, and skip `docs/handoffs/` by component-wise `starts_with`. That mirrors `docs_cli_test::is_cli_doc` (#462). No production code, wire, ADR, persistence, concurrency, secret or size concern applies. The decision to mirror the predicate rather than share it is justified in writing and holds up against the code. Verdict PASS with two warns.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Mirrored predicate, no shared helper | pattern consistency | The rule now exists in two test binaries (`docs_cli_test.rs:53`, the new `is_pipeline_scratch`). I verified the only other `docs/handoffs` references under `crates/` are comments, and the only other `git ls-files` use is `preflight.rs:610` (an index-mode query, unrelated). So there is no third consumer, and the brief's "reconsider at a third binary" trigger is correct. The duplication is deliberate and reasoned, so not a block. | Keep the comment naming `docs_cli_test::is_cli_doc` / #462, and mirror its three excluded and the `docs/research/handoffs-notes.md` included cases in the new test, as the brief says. |
| 2 | warn | AC 4 "Diff touches only `skilltext_w5_rename.rs` (plus this brief)" | naming and file structure | The run also commits `docs/handoffs/848/` (handoffs, `decisions.md`), which the pipeline requires to be tracked. The literal wording of AC 4 would fail a strict diff check by S. These files are pipeline scratch and are removed before the PR is pushed (#462 rationale), so the diff the PR carries is the single test file. | S should read AC 4 as "outside `docs/handoffs/`". Optionally O can reword it to say so. |

Dimensions checked, no finding:
- **Seam shape.** `tracked_files(root) -> Vec<String>` plus `scan_files(root, rels, needle)` follows the file's existing helpers (`root()`, `read()`). `scan_hits(needle)` stays as the thin composition, so the call site in `no_old_command_or_directory_outside_the_rename_note` is unchanged. The refactor is behaviour-preserving, and RED-first ordering (seam added, skip absent) makes the failing assertion meaningful.
- **Component semantics.** `Path::new(rel).starts_with("docs/handoffs")` matches `docs_cli_test.rs:53-55`, so `docs/handoffs-notes/x` and `docs/research/handoffs-notes.md` stay scanned. Git paths are `/`-separated and the file is `#![cfg(unix)]`, so there is no portability concern.
- **Self-skip.** Keeping `rel.ends_with("skilltext_w5_rename.rs")` inside `scan_files` is harmless for the temp-tree test. Its fixture files are named differently and the needle is built with `format!`, so it does not trip the real scan.
- **Test infrastructure.** `tempfile` is a holler-cli dev-dependency (`Cargo.toml:471`) and is already used in this file. The wizard binary (`wizard_scripts`, `Cargo.toml:559`) is manifest-registered, so no manifest edit is needed. No new module, no `tests/support/` edit, so there is no clash with #846.
- **Size.** The file is 180 lines, far below the 800-line watch mark.
- **Public repo and secrets.** Fixture contents are only the old skill path text. No personal infrastructure names or secrets are involved.
- **Risk acceptance.** Hiding a stale reference under `docs/handoffs/` is accepted, and the directory never reaches `main`. This matches #462.

## Notes for O
None required (PASS). Optional: reword AC 4 as in finding 2.

## Patterns referenced
- `crates/holler-cli/tests/docs_cli_test.rs:35-55` and `:216-233` (`is_cli_doc` and its `doc_filter_skips_pipeline_handoffs_only` test)
- `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs:37-79` (the scan being changed) and `:140-179` (existing `tempfile` use)
- `crates/holler-cli/Cargo.toml:471` and `:557-560` (tempfile dev-dep, `wizard_scripts` target)
- `crates/holler-cli/tests/wizard/main.rs` (module list)
