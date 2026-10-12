# Handoff-T-red: #848 rename-scan-skip-handoffs

**Date:** 2026-10-12
**Branch:** issue-848-implementation
**Brief / wireframe reviewed:** docs/handoffs/848-brief.md (no wireframe, no UI)

## A precondition
Confirmed: A returned PASS on the plan (docs/handoffs/848/handoff-A.md).

## Tests authored
File: `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs` (already declared via `wizard_scripts`; no manifest edit).

- Seam (behaviour-preserving refactor, no skip yet): `tracked_files(root) -> Vec<String>` runs `git ls-files -z`; `scan_files(root, rels, needle) -> Vec<String>` reads and matches (keeps the `ends_with("skilltext_w5_rename.rs")` self-skip); `scan_hits(needle)` is now `scan_files(&root(), &tracked_files(&root()), needle)`.
- `rename_scan_skips_pipeline_handoffs_only` (AC 1). Unit tier, temp tree via `tempfile`, explicit file list, no `git init`. Files `docs/handoffs/848-brief.md`, `docs/handoffs/848/handoff-A.md`, `docs/research/handoffs-notes.md` each quote `agent-skills/<old>` (built with `format!` so this file does not trip its own scan). Asserts exactly one hit, and that it is the `docs/research/` file. This pins both the skip (any depth) and the component-wise boundary (`handoffs-notes.md` still scanned).
- `no_old_command_or_directory_outside_the_rename_note` (AC 2) is unchanged in meaning; it is the real-repo regression.

## RED confirmation
Run: `cargo test -p holler-cli --test wizard_scripts skilltext_w5_rename::`

```
test skilltext_w5_rename::rename_scan_skips_pipeline_handoffs_only ... FAILED
assertion `left == right` failed: only the non-handoffs file is a hit, got:
["docs/handoffs/848-brief.md:1: quotes agent-skills/setup-wizard/SKILL.md",
 "docs/handoffs/848/handoff-A.md:1: ...", "docs/research/handoffs-notes.md:1: ..."]
  left: 3  right: 1
```
Right reason: the assertion about the missing skip fails; it compiles and the target is found.

Also failing, and expected: `no_old_command_or_directory_outside_the_rename_note`, because this worktree's tracked `docs/handoffs/848-brief.md:45` quotes `agent-skills/setup-wizard`. This is exactly the bug in the issue, observed live, and it goes GREEN once F adds the skip. The other 6 tests in the module pass. `cargo clippy -p holler-cli --test wizard_scripts -- -D warnings` clean; `scripts/lint.sh` shows no failure (file is 214 lines).

## Ready for F
Confirmed RED is valid; F may implement `is_pipeline_scratch(rel)` (`Path::new(rel).starts_with("docs/handoffs")`, with a comment citing `docs_cli_test::is_cli_doc` / #462 / #848) and apply it in `scan_files` before reading.
