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

Appended by F (Phase 6, rework pass after S's REWORK). Each entry quotes unchanged source that the rework relies on.

- **Fact:** a guard turns a refusal into the serde error text with `coded_message()` (`"<code>: <payload, or the Display text when the variant has none>"`), and the two env refusals are unit variants whose `Display` is one fixed sentence, so a refusal built from them cannot carry the input it refused. `deserialize_env_names` builds only those two variants, which is why no env refusal echoes its input.
  **Source:** `crates/holler-pane/src/error.rs:524-530`, `:555-557` and `:574-576`
  **Verbatim excerpt:**
  > pub(crate) fn coded_message(&self) -> String {
  >     let text = match self.detail() {
  >         Some(detail) => detail.to_owned(),
  >         None => self.to_string(),
  >     };
  >     format!("{}: {}", self.code(), text)
  > }
  >
  > PaneError::EnvNameInvalid => f.write_str(
  >     "an environment variable name must be non-empty and contain no whitespace",
  > ),
  >
  > PaneError::ProfileSecretRefused => f.write_str(
  >     "a profile holds environment variable names only, never a value (an entry with '=' was refused)",
  > ),

- **Fact:** `Argv` already reads a `serde_json::Value` first and refuses every non-array shape with this crate's own error, never serde's type error; `deserialize_env_names` follows the same pattern for `env`.
  **Source:** `crates/holler-pane/src/argv.rs:76-81`
  **Verbatim excerpt:**
  > impl<'de> Deserialize<'de> for Argv {
  >     fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
  >         let value = Value::deserialize(deserializer)?;
  >         Argv::from_value(value).map_err(|e| de::Error::custom(e.coded_message()))
  >     }
  > }

- **Fact:** serde's own type error quotes the rejected string, which is why the derived `Vec<EnvVarName>` reader could not stay: a bare `"env": "TOKEN=hunter2"` produced `invalid type: string "TOKEN=hunter2", expected a sequence`, and `decode_params` put that text in both `message` and `detail` of the reply. This is third-party source (the pinned `serde_core` 1.0.229 in the cargo registry, so the path is outside the repo); S reproduced the echo in a scratch crate before the fix (handoff-S.md, "Quality audit").
  **Source:** `serde_core-1.0.229/src/de/mod.rs:214` and `:410`
  **Verbatim excerpt:**
  > Error::custom(format_args!("invalid type: {}, expected {}", unexp, exp))
  >
  > Str(s) => write!(formatter, "string {:?}", s),

- **Fact:** the hub's own long-poll, `control/wait`, answers "nothing happened in this window" as an ordinary success, not as an error; the `Watch` idle signal (`Ok(None)`, operator decision D1=a) follows that precedent, and `Err(Timeout)` is left to mean only that the store did not answer.
  **Source:** `crates/holler-hub/src/control_server.rs:477-480`
  **Verbatim excerpt:**
  > /// The result is always a **success** envelope, `{matched, rows}`: a timeout
  > /// is `matched:false, rows:[]`, not a JSON-RPC error — "nothing happened in
  > /// this window" is an ordinary outcome of a wait, not a hub-side refusal, and
  > /// giving it its own wire error code would be one more code the CLI has to
