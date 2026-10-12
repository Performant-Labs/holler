# Brief: #848 rename-scan-skip-handoffs

Repo: Performant-Labs/holler. Issue: #848. Rigor: in-session. UI surface: no. Kind: test-only.

Rigor note: `direct` would fit (one test file, about ten lines of logic, no production code). The operator's rule is `in-session`, so that is what this declares; the pipeline's own agents review, no outside model.

## Problem
The wizard's rename scan, `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note` (#820), walks every file `git ls-files` lists and fails on the old skill path. In a pipeline worktree that includes `docs/handoffs/` (briefs, journals), which quote history verbatim; #649's brief and journal had to be edited to pass. `docs_cli_test` already skips `docs/handoffs/` (#462). The rename scan should too.

## Evidence
`crates/holler-cli/tests/wizard/skilltext_w5_rename.rs:37-61` (the scan; root is hard-wired and there is no per-path filter):
```
fn scan_hits(needle: &str) -> Vec<String> {
    let listing = Command::new("git").args(["ls-files", "-z"]).current_dir(root()).output().unwrap();
    assert!(listing.status.success());
    let mut hits = Vec::new();
    for rel in String::from_utf8(listing.stdout).unwrap().split('\0') {
        if rel.is_empty() || rel.ends_with("skilltext_w5_rename.rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root().join(rel)) else { continue; };
        for (i, line) in text.lines().enumerate() {
            if line.contains(needle) { hits.push(format!("{rel}:{}: {line}", i + 1)); }
        }
    }
    hits
}
```
`skilltext_w5_rename.rs:11-13`: `fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") }`.
`skilltext_w5_rename.rs:63-79`: the test calls `scan_hits(&needle)` for `agent-skills/<old>` and `/<old>`, drops `docs/<old>` and `../<old>` occurrences, and allows lines containing the rename-note marker.

The existing skip, `crates/holler-cli/tests/docs_cli_test.rs:53-55`:
```
fn is_cli_doc(rel: &Path) -> bool {
    !rel.starts_with("docs/handoffs")
}
```
`Path::starts_with` compares whole components, so `docs/research/handoffs-notes.md` is still scanned (its test: `docs_cli_test.rs:219-233`).

Reuse check: `is_cli_doc` is private to the `docs_cli_test` binary. The wizard tests are a different test binary (`tests/wizard/main.rs`, run as `--test wizard_scripts`) and do not include `tests/support/` or `tests/fixture/`. Sharing it would need a new cross-binary module, or edits to `tests/support/`, for a one-line `starts_with`. Decision below: do not share; mirror the predicate, cite the origin.

## Acceptance criteria
1. A file under `docs/handoffs/` (any depth) containing the old skill path does not make the scan report a hit; a file outside it (including `docs/research/handoffs-notes.md`) still does. Test: `skilltext_w5_rename::rename_scan_skips_pipeline_handoffs_only`, using a temp tree.
2. `cargo test -p holler-cli --test wizard_scripts skilltext_w5_rename::` passes, including `no_old_command_or_directory_outside_the_rename_note` unchanged in meaning on the real repo.
3. A worktree with a `docs/handoffs/<n>-brief.md` that quotes `agent-skills/setup-wizard` passes the real scan (manual check by T-verify, then remove the scratch file).
4. Diff touches only `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs` (plus this brief). `rg 'handoffs' crates/holler-cli/tests/wizard/skilltext_w5_rename.rs` shows the skip documented with a `#462`/`#848` pointer.

## Files
Production: none. Tests: `crates/holler-cli/tests/wizard/skilltext_w5_rename.rs` only.
Reuse map: the skip rule is `docs_cli_test::is_cli_doc` (#462), mirrored not copied-with-drift: same `starts_with("docs/handoffs")` component semantics, with a comment naming it. Nothing in `tests/support/` is touched.

## Decisions already made (MO)
- Seam: split the scan into a pure part and the git part. `tracked_files(root) -> Vec<String>` runs `git ls-files -z`; `scan_files(root: &Path, rels: &[String], needle: &str) -> Vec<String>` reads and matches, applying `is_pipeline_scratch(rel)` (`Path::new(rel).starts_with("docs/handoffs")`) before reading. `scan_hits(needle)` becomes `scan_files(&root(), &tracked_files(&root()), needle)`. This lets the new test use a `tempfile::tempdir()` and an explicit file list, with no `git init` (tempfile is already a holler-cli dev-dependency and this file already uses it).
- Keep the existing `rel.ends_with("skilltext_w5_rename.rs")` self-skip as is.
- No shared helper (see Reuse check). Reconsider only if a third test binary needs the same rule.
- New test: temp tree with `docs/handoffs/848-brief.md`, `docs/handoffs/848/handoff-A.md`, `docs/research/handoffs-notes.md`, each containing `agent-skills/<old>` (built with the `format!("{}-{}", ...)` trick so this file does not trip its own scan); assert hits name only the `docs/research/` file.

## Out of scope
Making `docs_cli_test::is_cli_doc` shared; the other scans over tracked files; changing the rename-note allowance; the wizard text itself.

## Test plan
T (RED): add the seam signatures (`tracked_files`, `scan_files` with no skip yet, so the refactor is behaviour-preserving) and `rename_scan_skips_pipeline_handoffs_only`. It fails on the assertion today: hits include both `docs/handoffs/` files. Confirm the other tests in the module still pass. F (GREEN): add `is_pipeline_scratch` and the skip inside `scan_files`. T (verify): module passes, then criterion 3 by hand.

## Risks
- Skipping by path hides a real stale reference placed under `docs/handoffs/`; accepted, that directory is deleted before the PR is pushed (#462 rationale) and never reaches `main`.
- The mirrored predicate can drift from `is_cli_doc`; mitigated by the comment and the identical component-wise test cases.

## Clashes (siblings)
- #846 (shared LinkedHost support module): edits `tests/support/` and the pane rigs; this brief touches neither. No file overlap.
- #844, #845 (OpenCode adapter / pane switch bugs): production pane code and pane integration tests; no overlap.
- #847 (remote_admin flake): `remote_admin_test.rs`; no overlap.
- #696 (shared subprocess runner): `holler-pane` adapter sources; no overlap.
Only shared surface is the workspace test run, which #847's flake can fail independently; re-run it alone before blaming this change.
