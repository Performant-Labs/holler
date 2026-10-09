# Evidence appendix: #637 slice a (facts in unchanged code that the diff relies on)

Appended by F (Phase 6). The diff gate's reviewer sees the diff and bounded excerpts, not the repo; each entry quotes unchanged source that the change depends on.

- **Fact:** `SessionName` already derives `Clone, PartialEq, Eq, Hash, PartialOrd, Ord`, so `PaneName(SessionName)` derives the same set without any edit to `vocab.rs` (it has no `Serialize`/`Deserialize`, which is why `PaneName` implements serde by hand through `parse`/`as_str`).
  **Source:** `crates/holler-proto/src/vocab.rs:73-74`
  **Verbatim excerpt:**
  > #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
  > pub struct SessionName(String);

- **Fact:** `SessionName::parse` is the ADR 0005 name grammar (non-empty, no `/`, then the per-segment check); `PaneName::parse` delegates to it, so the pane-name grammar is reused, not copied.
  **Source:** `crates/holler-proto/src/vocab.rs:82-91`
  **Verbatim excerpt:**
  > pub fn parse(s: &str) -> Result<Self, NameError> {
  >     if s.is_empty() {
  >         return Err(NameError::Empty);
  >     }
  >     if s.contains('/') {
  >         return Err(NameError::Slash);
  >     }
  >     check_segment(s.as_bytes())?;
  >     Ok(Self(s.to_owned()))
  > }

- **Fact:** the v2 wire catalog is closed at 22 rows and `find` is a lookup in it only, so `PANE_METHODS`/`PROFILE_METHODS` (added beside it, not in it) are `None` from `find` and a body connection still gets `method_not_found` for them.
  **Source:** `crates/holler-proto/src/methods.rs:57-59` and `crates/holler-proto/src/methods.rs:99-101`
  **Verbatim excerpt:**
  > /// The complete, closed v2 method catalog (22 rows).
  > #[rustfmt::skip]
  > pub const CATALOG: &[Method] = &[
  >
  > pub fn find(name: &str) -> Option<&'static Method> {
  >     CATALOG.iter().find(|m| m.name == name)
  > }

- **Fact:** the workspace lint set makes `unwrap`/`expect`/`panic`/`unreachable`, over-long or over-complex functions and unused code fatal, and `large_enum_variant` a warning that CI turns into an error; this is why helpers return `Result`, every item is `pub` or used, and `SpecEdit::Set` holds a `Box<ProfileSpec>`.
  **Source:** `Cargo.toml:19-30`, and `.github/workflows/ci.yml:280`
  **Verbatim excerpt:**
  > [workspace.lints.clippy]
  > unwrap_used = "deny"
  > expect_used = "deny"
  > panic = "deny"
  > unreachable = "deny"
  > cognitive_complexity = "deny"
  > too_many_lines = "deny"
  > struct_excessive_bools = "deny"
  > large_enum_variant = "warn"
  >
  > [workspace.lints.rust]
  > dead_code = "deny"
  >
  > if cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee "$log"; then

- **Fact:** the `derive` feature of `serde` is enabled once, at the workspace level, so the new crate declares `serde = { workspace = true }` with no feature list of its own (the `# for` consumer marker is on the workspace line, which this story does not edit).
  **Source:** `Cargo.toml:44`
  **Verbatim excerpt:**
  > serde = { version = "1", features = ["derive"] } # for holler-proto, holler-body
