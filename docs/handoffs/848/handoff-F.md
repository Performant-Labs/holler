# Handoff-F: #848 rename-scan-skip-handoffs

**Date:** 2026-10-12
**Branch:** issue-848-implementation
**Issue:** #848

## What was done
- `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs`: added `is_pipeline_scratch(rel)` (`Path::new(rel).starts_with("docs/handoffs")`), with a doc comment that cites `docs_cli_test::is_cli_doc` (#462) and #848. `scan_files` now skips those paths before it reads them, next to the existing self-skip.

This issue is test-only. Per the brief, the code under test is the scan helper in this test file, so the "production" side of the change is the skip inside `scan_files`. T's seam and the new test were left exactly as T authored them.

## Design decisions
- The skip sits in `scan_files`, not in `tracked_files`, so the pure seam that the new test drives has the behaviour. Filtering in `tracked_files` would leave the test RED.
- The predicate is a separate named function rather than an inline `starts_with`, as the brief specified. The name and the comment make the mirror of `is_cli_doc` findable.

## Reuse / extend-vs-new
The brief's Reuse map: mirror `docs_cli_test::is_cli_doc` (#462) with the same component-wise `starts_with("docs/handoffs")` and a comment naming it. The predicate is not shared, because it lives in a different test binary and sharing it would need a cross-binary module (A finding 1, accepted). Nothing in `tests/support/` was touched.

## Architecture notes for A
None. One private test helper was added to one test file. There are no new dependencies, no module changes and no production code.

## Deviations from spec / wireframe
None.

## Tier 1 self-check (incl. tests now GREEN)
`cargo test -p holler-cli --test wizard_scripts skilltext_w5_rename::`
```
test skilltext_w5_rename::directory_and_frontmatter_are_renamed ... ok
test skilltext_w5_rename::rename_scan_skips_pipeline_handoffs_only ... ok
test skilltext_w5_rename::the_wizard_prefix_names_that_stay_are_untouched ... ok
test skilltext_w5_rename::docs_say_the_command_is_herdr_setup ... ok
test skilltext_w5_rename::rename_note_is_in_readme_and_docs ... ok
test skilltext_w5_rename::stage_count_covers_stage_0_to_stage_11 ... ok
test skilltext_w5_rename::install_command_lands_lib_in_herdr_setup ... ok
test skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 278 filtered out
```
- `no_old_command_or_directory_outside_the_rename_note` was RED at T-red because this worktree's tracked `docs/handoffs/848-brief.md` quotes the old skill path. It is now GREEN with that file still tracked, which is live evidence for AC 3. T-verify still owns the manual scratch-file check.
- `cargo clippy -p holler-cli --test wizard_scripts -- -D warnings`: clean.
- `scripts/lint.sh`: no failures (size warnings only, on unrelated files). This file is 222 lines.
- `cargo fmt --check` reports no diff on the lines I added. The workspace has no rustfmt config or CI fmt gate, and it already differs from default rustfmt across many files, including older lines of this file. I did not reformat anything.
- `rg 'handoffs' crates/holler-cli/tests/wizard/skilltext_w5_rename.rs`: the skip is documented with `#462` and `#848` (lines 49-53). This satisfies AC 4.

## Evidence appendix
docs/handoffs/848/evidence.md (the mirrored predicate's origin).

## Tests that look wrong (for T)
None.

## Known issues
None. CHANGELOG is untouched: the brief limits the diff to the one test file, and `scripts/changelog-check.sh` only requires the `[Unreleased]` heading.

## Files changed
- `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs` (the skip inside the test's scan helper; this is a test-only issue, so the file has no production counterpart)
