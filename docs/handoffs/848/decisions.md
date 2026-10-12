## A (up-front plan review) — 2026-10-12T17:16:23Z
- **Decided:** PASS. The seam split plus a mirrored `starts_with("docs/handoffs")` predicate is consistent with `docs_cli_test::is_cli_doc` (#462). Not sharing the helper is justified in writing, since there are only two test binaries and no third consumer. Two warns: the duplicated rule (keep the origin comment and mirrored cases), and AC 4's "touches only one file" must be read as outside `docs/handoffs/`.
- **Assumed:** `docs/handoffs/` is removed before the PR is pushed (per #462 and the brief), so the PR diff is the single test file. The brief's description of `tests/wizard/` as not including `tests/support/` is correct (not exhaustively verified beyond `main.rs`'s module list).
- **Hedged:** nothing material. No separate survey.md exists, so the brief's Reuse check was the Reuse map.
- **Evidence:** read the brief, `skilltext_w5_rename.rs` in full, `docs_cli_test.rs:35-70,210-240`, `Cargo.toml` tempfile and wizard_scripts entries, and `rg "ls-files|docs/handoffs" crates/`.

## T (author / RED) — 2026-10-12
- **Decided:** RED is valid. Added the `tracked_files`/`scan_files` seam with no skip, plus `rename_scan_skips_pipeline_handoffs_only`; it fails on the count assertion (3 hits vs 1). Used one test covering both nested and top-level handoffs paths plus the `handoffs-notes.md` boundary, rather than several tests, for proportionality.
- **Assumed:** The real-repo scan failing on this worktree's own `docs/handoffs/848-brief.md` is expected evidence of the bug, not a wrong-reason failure; F's skip resolves it.
- **Hedged:** The new test does not call the not-yet-existing `is_pipeline_scratch` directly, so it stays a behavioural test through `scan_files` and compiles before F.
- **Evidence:** `cargo test -p holler-cli --test wizard_scripts skilltext_w5_rename::` (6 passed, 2 failed); clippy and lint.sh clean.

## F (implement / GREEN) — 2026-10-12
- **Decided:** Added `is_pipeline_scratch(rel)` (component-wise `starts_with("docs/handoffs")`, with a comment citing `docs_cli_test::is_cli_doc` / #462 / #848) and applied it in `scan_files` next to the self-skip. The skip lives in the pure seam, not in `tracked_files`, so the temp-tree test exercises it.
- **Assumed:** No CHANGELOG entry, because the brief confines the diff to the one test file and the changelog check only requires the `[Unreleased]` heading.
- **Hedged:** Did not run `cargo fmt` on the file, because the workspace is not default-rustfmt formatted and has no fmt gate. The added lines produce no fmt diff.
- **Evidence:** `cargo test -p holler-cli --test wizard_scripts skilltext_w5_rename::` 8 passed (was 6 passed, 2 failed at RED); clippy `-D warnings` clean; `scripts/lint.sh` no failures.

## T (verify / GREEN) — 2026-10-12
- **Decided:** GREEN, no blocking Tier 2 issue. Module 8/8, wizard_scripts 286/286, docs_cli_test and wire_selftest pass. Two production-predicate mutations (always false; over-wide match) each fail the new test, so it pins behavior. AC 3 was checked through the tracked `docs/handoffs/848-brief.md`, which quotes the old path, instead of adding a scratch file.
- **Assumed:** The workspace clippy error in `token_load_test.rs` (untouched file) and the `remote_admin_test` failure under load are pre-existing/unrelated (#847), since the file passes 21/21 alone three times.
- **Hedged:** `cargo machete` is not installed, so the unused-dependency check was skipped; the diff adds no dependency.
- **Evidence:** `docs/handoffs/848/handoff-T-green.md`.
