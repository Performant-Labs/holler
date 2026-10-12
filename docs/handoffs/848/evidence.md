- **Fact:** the predicate mirrored by `is_pipeline_scratch` excludes `docs/handoffs` by whole path components, so a name that merely contains `handoffs` is still scanned.
  **Source:** `crates/holler-cli/tests/docs_cli_test.rs:51-55`
  **Verbatim excerpt:**
  > /// `Path::starts_with` compares whole components, so a page whose name merely
  > /// contains `handoffs` (e.g. `docs/research/handoffs-notes.md`) is scanned.
  > fn is_cli_doc(rel: &Path) -> bool {
  >     !rel.starts_with("docs/handoffs")
  > }
