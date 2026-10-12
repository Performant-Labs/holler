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
