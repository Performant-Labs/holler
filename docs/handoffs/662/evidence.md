# Evidence: #662a (F, Phase 5)

Facts in unchanged code that the diff, or T's tests over it, rely on. Every excerpt is copied from the tree at `3bdd129`
(the run's base; none of these files changes in this run).

- **Fact:** `emit` ends text output with one newline when the rendered text lacks it, so `list` and `show` render their
  lines joined by `\n` with no trailing newline.
  **Source:** `crates/holler-cli/src/output.rs:318-324`
  **Verbatim excerpt:**
  > ```rust
  > fn write_line(to: &mut dyn Write, line: &str) -> io::Result<()> {
  >     to.write_all(line.as_bytes())?;
  >     if !line.ends_with('\n') {
  >         to.write_all(b"\n")?;
  >     }
  >     to.flush()
  > }
  > ```

- **Fact:** in text mode a failed result writes `error: <message>` to `err` (nothing to `out`), and its exit code comes
  from the class of its code.
  **Source:** `crates/holler-cli/src/output.rs:281-284`
  **Verbatim excerpt:**
  > ```rust
  >         Err(error) => {
  >             let written = write_line(sink.err, &format!("error: {}", error.message));
  >             settle(written, exit_code(&error))
  >         }
  > ```

- **Fact:** `class_of` makes `usage` a usage error, `profile-not-found` a refusal, and `timeout`, `unavailable` and
  `not-implemented` failures, so `show` exits 2 for a blank NAME, 3 for a missing profile and 1 for a store failure, with
  no table in the verb.
  **Source:** `crates/holler-pane/src/error.rs:274-301`
  **Verbatim excerpt:**
  > ```rust
  >     match closed {
  >         PaneCode::Usage => ErrorClass::Usage,
  >         // Understood and declined: a guard, a policy or a gate said no, the name is
  >         // taken, or the request named something that does not exist.
  >         PaneCode::GridAmbiguous
  >         | PaneCode::GridOutOfRange
  >         | PaneCode::CommandNotArgv
  >         | PaneCode::EnvNameInvalid
  >         | PaneCode::ProfileSecretRefused
  >         | PaneCode::ProfileExists
  >         | PaneCode::ProfileHasLivePanes
  >         | PaneCode::PaneNotInProfile
  >         | PaneCode::PaneInOtherProfile
  >         | PaneCode::ProbeFailed
  >         | PaneCode::HerdrVersionUnsupported
  >         | PaneCode::ProfileNotFound
  >         | PaneCode::PaneNotFound
  >         | PaneCode::SessionNotFound => ErrorClass::Refusal,
  >         // Went wrong while doing the work: a race between writers, a bound that ran
  >         // out, something unreachable or unreadable, live state that disagrees with
  >         // its spec, or work the verb cannot do yet.
  >         PaneCode::GenerationConflict
  >         | PaneCode::ProfileConflict
  >         | PaneCode::Timeout
  >         | PaneCode::Unavailable
  >         | PaneCode::StoreCorrupt
  >         | PaneCode::NotImplemented
  >         | PaneCode::ProfileDrift => ErrorClass::Failure,
  > ```

- **Fact:** the exit codes of the three classes are 2 (usage), 3 (refusal) and 1 (failure).
  **Source:** `crates/holler-pane/src/error.rs:239-245`
  **Verbatim excerpt:**
  > ```rust
  >     pub const fn exit_code(self) -> i32 {
  >         match self {
  >             ErrorClass::Usage => 2,
  >             ErrorClass::Refusal => 3,
  >             ErrorClass::Failure => 1,
  >         }
  >     }
  > ```

- **Fact:** `ProfileName::parse` trims the text and refuses an empty name or one with a control character as `usage`, so
  `show "   "` is `usage` from the verb, and a profile name (header line, `list` row) holds no control character and prints
  unescaped.
  **Source:** `crates/holler-pane/src/profile.rs:42-61`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn parse(text: &str) -> Result<Self, PaneError> {
  >         let name = text.trim();
  >         let refusal = if name.is_empty() {
  >             Some("a profile name must not be empty")
  >         } else if name.chars().count() > MAX_NAME_CHARS {
  >             Some("a profile name is at most 64 characters")
  >         } else if name.chars().any(char::is_control) {
  >             Some("a profile name must not contain control characters")
  >         } else if slugify(name).is_empty() {
  >             Some("a profile name needs at least one ASCII letter or digit")
  >         } else {
  >             None
  >         };
  >         match refusal {
  >             Some(rule) => Err(PaneError::Usage {
  >                 message: format!("invalid profile name {}: {rule}", excerpt(text)),
  >             }),
  >             None => Ok(Self(name.to_owned())),
  >         }
  >     }
  > ```

- **Fact:** a profile read back has the slug of its name (a disagreeing record does not load), so `list` sorts by and
  prints the stored `slug` field, which is the profile's identity, and it is ASCII letters, digits and `-`.
  **Source:** `crates/holler-pane/src/profile.rs:238-247`
  **Verbatim excerpt:**
  > ```rust
  >     fn try_from(raw: RawProfile) -> Result<Self, String> {
  >         let expected = raw.name.slug();
  >         if raw.slug != expected {
  >             return Err(format!(
  >                 "profile slug {} does not match its name {} (expected {})",
  >                 excerpt(&raw.slug),
  >                 excerpt(raw.name.as_str()),
  >                 excerpt(&expected)
  >             ));
  >         }
  > ```

- **Fact:** `GridPos` prints as `r<row>c<col>`, which `FieldValue::Grid`'s `Display` reuses.
  **Source:** `crates/holler-pane/src/grid.rs:68-73`
  **Verbatim excerpt:**
  > ```rust
  > impl fmt::Display for GridPos {
  >     /// The `rRcC` form, row first: `r2c1`.
  >     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
  >         write!(f, "r{}c{}", self.row, self.col)
  >     }
  > }
  > ```

- **Fact:** `GridPos` serializes as `{"row","col","pos"}` in that order, through `serialize_struct`, so the untagged
  `FieldValue::Grid` gives a row-first object whatever `serde_json`'s map feature is.
  **Source:** `crates/holler-pane/src/grid.rs:129-138`
  **Verbatim excerpt:**
  > ```rust
  > impl Serialize for GridPos {
  >     /// `{"row":R,"col":C,"pos":"rRcC"}`, in that key order.
  >     fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
  >         let mut out = serializer.serialize_struct("GridPos", 3)?;
  >         out.serialize_field("row", &self.row)?;
  >         out.serialize_field("col", &self.col)?;
  >         out.serialize_field("pos", &self.to_string())?;
  >         out.end()
  >     }
  > }
  > ```

- **Fact:** `ProbeResult` serializes in snake case, so `show`'s JSON `probe` is `"ok"`, `{"failed":{"missing":[...]}}` or
  `{"error":"..."}`, taken as is from `Pane.probe.last`.
  **Source:** `crates/holler-pane/src/probe.rs:15-26`
  **Verbatim excerpt:**
  > ```rust
  > /// What one run of a health probe found.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(rename_all = "snake_case", deny_unknown_fields)]
  > pub enum ProbeResult {
  >     /// The probe ran and every expected string was in its output.
  >     Ok,
  >     /// The probe ran and these expected strings were not in its output.
  >     Failed { missing: Vec<String> },
  >     /// The probe could not be run to a verdict (the program is missing, it timed
  >     /// out, ...); the reason is plain text.
  >     Error(String),
  > }
  > ```

- **Fact:** `HarnessKind` serializes as a snake-case unit-variant name, so `SpecField::HarnessKind`'s value is the JSON
  string `"opencode"` taken from serde (`serde_text`), with no literal in `profile_diff.rs`.
  **Source:** `crates/holler-pane/src/pane.rs:125-129`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
  > #[serde(rename_all = "snake_case")]
  > pub enum HarnessKind {
  >     Opencode,
  > }
  > ```

- **Fact:** `PaneRole` serializes the same way (`"agent"`, `"orchestrator"`), so `SpecField::Role`'s value comes from
  serde too.
  **Source:** `crates/holler-pane/src/pane.rs:153-158`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
  > #[serde(rename_all = "snake_case")]
  > pub enum PaneRole {
  >     Agent,
  >     Orchestrator,
  > }
  > ```

- **Fact:** a spec's `command` and `check` are left out of its JSON when absent, so AC 2e compares an absent argv (a
  `FieldValue::Argv(None)`, which serializes as `null`) with the missing path read as `null`.
  **Source:** `crates/holler-pane/src/profile.rs:191-196`
  **Verbatim excerpt:**
  > ```rust
  >     /// The launch command: an argv array, never a shell string.
  >     #[serde(default, skip_serializing_if = "Option::is_none")]
  >     pub command: Option<Argv>,
  >     /// The health probe argv, e.g. `["curl","-s","http://127.0.0.1:8095/v1/models"]`.
  >     #[serde(default, skip_serializing_if = "Option::is_none")]
  >     pub check: Option<Argv>,
  > ```

- **Fact:** `FieldValue::Text`'s escape is the rule of Holler's text log lines: each control character through
  `char::escape_default`, every other character as is.
  **Source:** `crates/holler-proto/src/log.rs:483-497`
  **Verbatim excerpt:**
  > ```rust
  > fn escape_field_value(v: &str) -> String {
  >     if v.chars().any(char::is_control) {
  >         v.chars()
  >             .flat_map(|c| {
  >                 if c.is_control() {
  >                     c.escape_default().collect::<Vec<char>>()
  >                 } else {
  >                     vec![c]
  >                 }
  >             })
  >             .collect()
  >     } else {
  >         v.to_owned()
  >     }
  > }
  > ```

- **Fact:** the binary's ports are still `Unwired` (until #649), so the real `holler profile list` and `show` answer
  `not-implemented`, as the CHANGELOG entry says.
  **Source:** `crates/holler-cli/src/pane/wiring.rs:31-32`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn connect() -> Result<Self, PaneError> {
  >         Ok(Self { ports: Unwired })
  > ```

- **Fact:** `Unwired`'s profile store answers `not-implemented` for `get` and `list`, the first store call of `show` and
  of `list`.
  **Source:** `crates/holler-cli/src/pane/wiring.rs:76-82`
  **Verbatim excerpt:**
  > ```rust
  >     fn get(&self, _name: &ProfileName) -> Result<Option<Profile>, PaneError> {
  >         Err(PaneError::NotImplemented)
  >     }
  >
  >     fn list(&self) -> Result<Vec<Profile>, PaneError> {
  >         Err(PaneError::NotImplemented)
  >     }
  > ```

## Added by T (Phase 7, GREEN)

Copied by T from the tree (none of these files changes in this run; `git diff 3bdd129 HEAD` over them is empty).

- **Fact:** the test kit's `FakeProfileStore::list` answers in slug order: the feed keeps live records in a `BTreeMap` by
  key, and a profile's key is its slug. So `list_reports_name_slug_panes_live_generation_in_both_formats` pins the output
  order, but not the verb's own `sort_by` (a test comment in `profile_verbs/list.rs` says so).
  **Source:** `crates/holler-pane-testkit/src/feed.rs:64-66`, `:75-78`; `crates/holler-pane-testkit/src/profile_store.rs:310-315`
  **Verbatim excerpt:**
  > ```rust
  >     /// The last change of each live record, by key. It is always a put, because a
  >     /// delete removes the record's entry.
  >     live: BTreeMap<E::Key, E>,
  > ```
  > ```rust
  >     /// Every live record, in key order.
  >     pub(crate) fn records(&self) -> impl Iterator<Item = &E::Record> {
  >         self.live.values().filter_map(|change| change.record())
  >     }
  > ```
  > ```rust
  >     type Key = String;
  >     type Record = Profile;
  >
  >     fn key(&self) -> String {
  >         self.name.slug()
  >     }
  > ```

- **Fact:** the hub's profile registry also lists in slug order, while the port pins none; the verb's sort is defensive.
  **Source:** `crates/holler-hub/src/profile/store.rs:70-71`, `:168-169`
  **Verbatim excerpt:**
  > ```rust
  >     /// The entry filed under each slug: the record or its tombstone, and the change log.
  >     entries: BTreeMap<String, ProfileEntry>,
  > ```
  > ```rust
  >     /// Every live profile, in slug order (the port pins no order).
  >     pub(crate) fn list(&self) -> Result<Vec<Profile>, PaneError> {
  > ```

- **Fact:** a fake whose standing fault is `Fault::Wedged` answers every call with `PaneError::Timeout`, which is why
  `show_passes_a_store_failure_through` and `list_passes_a_store_failure_through` expect `timeout` (exit 1).
  **Source:** `crates/holler-pane-testkit/src/fault.rs:119-125`
  **Verbatim excerpt:**
  > ```rust
  >     fn take_fault(&mut self, op: Op) -> Result<(), PaneError> {
  >         match &self.standing {
  >             Some(Fault::Wedged) => {
  >                 return Err(PaneError::Timeout {
  >                     op: op.as_str().to_owned(),
  >                 })
  >             }
  > ```

- **Fact:** `ProbeResult::Error` holds its reason as a plain `String`, so the new `error ("...")` case in
  `show_reports_the_last_probe_result_without_running_one` serializes as `{"error":"..."}` (snake case; see F's
  `ProbeResult` entry above for the attribute).
  **Source:** `crates/holler-pane/src/probe.rs:23-25`
  **Verbatim excerpt:**
  > ```rust
  >     /// The probe could not be run to a verdict (the program is missing, it timed
  >     /// out, ...); the reason is plain text.
  >     Error(String),
  > ```
