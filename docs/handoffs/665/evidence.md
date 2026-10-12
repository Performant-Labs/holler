# Evidence: #665, facts in unchanged code that the diff relies on

Written by F (Phase 6 of the script, implement). Line numbers are the worktree's after F's change. T may append its own entries.

## The rename transaction (hub)

- **Fact:** the membership hook of `pane/cas_put` reads the profile registry (taking and releasing the profile lock) before the pane write takes the pane lock, so no pane write holds the pane lock while it waits for the profile lock. That is why the rename's order (the profile lock, then the pane lock) cannot deadlock against it.
  **Source:** `crates/holler-hub/src/panes/handlers.rs:111-114`
  **Verbatim excerpt:**
  > ```rust
  >     run(cid, obj, move |params: PaneCasPutParams| {
  >         check_membership(&params.pane, &profiles)?;
  >         store.cas_put(&params.pane, params.expected_generation)
  >     })
  > ```

- **Fact:** `check_membership` reads the profile registry through `get`, one short lock, and answers `profile-not-found` for a name whose profile does not exist. After a rename a pane write naming the old name is therefore refused.
  **Source:** `crates/holler-hub/src/profile/mod.rs:212-222`
  **Verbatim excerpt:**
  > ```rust
  > pub fn check_membership(pane: &Pane, profiles: &ProfileState) -> Result<(), PaneError> {
  >     let Some(name) = &pane.profile else {
  >         return Ok(());
  >     };
  >     match profiles.store.get(name)? {
  >         Some(_) => Ok(()),
  >         None => Err(PaneError::ProfileNotFound {
  >             what: name.to_string(),
  >         }),
  >     }
  > }
  > ```

- **Fact:** the one stamp of a write is never before the entry's last log time or its record's `updated`; the rename takes the larger of OLD's and NEW's previous entry's stamps, so `at` never decreases in either log.
  **Source:** `crates/holler-hub/src/profile/entry.rs:175-181`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn stamp(entry: Option<&ProfileEntry>, now: i64) -> i64 {
  >     entry.map_or(now, |entry| {
  >         let logged = entry.log.last().map_or(now, |last| last.at);
  >         let updated = entry.profile().map_or(now, |record| record.updated);
  >         now.max(logged).max(updated)
  >     })
  > }
  > ```

- **Fact:** an entry's log is the previous entry's log (a tombstone's included) with the new entry appended, so OLD's log goes on with `deleted` and a deleted profile's log under NEW's slug goes on with `renamed`; nothing is copied between slugs.
  **Source:** `crates/holler-hub/src/profile/entry.rs:114-123`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn next(
  >         previous: Option<&Self>,
  >         slug: String,
  >         event: ProfileEvent,
  >         logged: ProfileLogEntry,
  >     ) -> Self {
  >         let mut log = previous.map_or_else(Vec::new, |entry| entry.log.clone());
  >         log.push(logged);
  >         Self { slug, event, log }
  >     }
  > ```

- **Fact:** the load check requires a tombstone's log to end with `Deleted` and a live record's log to end at the record's generation; OLD's tombstone (log ending `deleted`) and NEW's entry (log ending `renamed` at the new generation) both pass it, which is what the restart test reloads.
  **Source:** `crates/holler-hub/src/profile/entry.rs:97-105`
  **Verbatim excerpt:**
  > ```rust
  >     let deleted = matches!(last.change, ProfileChange::Deleted);
  >     match event.profile.as_deref() {
  >         Some(record) if record.name != event.name => {
  >             Err("the entry's event and record name the profile differently")
  >         }
  >         Some(record) if deleted || last.generation != record.generation => {
  >             Err("the entry's change log does not end with a write at its record's generation")
  >         }
  >         None if !deleted => Err("the entry's change log does not end with its deletion"),
  > ```

- **Fact:** a registry document is saved through `write_atomic` (a temporary file, then a rename), so every save, the restore included, gives `panes.json` a new inode; the panes-first test observes the order by that inode.
  **Source:** `crates/holler-hub/src/panes/persist.rs:150-157`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn save_doc<E: Serialize>(path: &Path, doc: &Doc<E>) -> Result<(), Problem> {
  >     let bytes = serde_json::to_vec_pretty(doc)
  >         .map_err(|_| Problem("not written: the document could not be encoded".to_owned()))?;
  >     if let Some(dir) = path.parent() {
  >         std::fs::create_dir_all(dir).map_err(|e| Problem::not_written(&e))?;
  >     }
  >     write_atomic(path, &bytes, FILE_MODE).map_err(|e| Problem::not_written(&e))
  > }
  > ```

- **Fact:** membership is by slug on both sides; the transaction's member moves and the drift step use it.
  **Source:** `crates/holler-pane/src/profile_diff.rs:275-279`
  **Verbatim excerpt:**
  > ```rust
  > pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
  >     pane.profile
  >         .as_ref()
  >         .is_some_and(|own| own.slug() == profile.slug())
  > }
  > ```

- **Fact:** `next_generation(current, expected)` is `current + 1` when they match and `generation-conflict` otherwise; a member's move calls it with its own generation twice, which is `+ 1` with the overflow check.
  **Source:** `crates/holler-pane/src/generation.rs:25-34`
  **Verbatim excerpt:**
  > ```rust
  > pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
  >     if current != expected {
  >         return Err(PaneError::Conflict);
  >     }
  >     current
  >         .checked_add(1)
  >         .ok_or_else(|| PaneError::StoreCorrupt {
  >             what: "a generation counter overflowed".to_owned(),
  >         })
  > }
  > ```

## The verbs (CLI)

- **Fact:** `write_atomic` creates no directory; it opens a temporary file beside the destination with `create_new`, so a missing parent directory fails with the I/O error (`NotFound`) and nothing is written. Export classes that `usage`.
  **Source:** `crates/holler-proto/src/atomic_file.rs:66-68` and `:132-139`
  **Verbatim excerpt:**
  > ```rust
  > pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
  >     let tmp = write_temp(path, bytes, mode, false)?;
  >     std::fs::rename(&tmp, path).inspect_err(|_| discard(&tmp))
  > ```
  > ```rust
  > fn open_temp(tmp: &Path, mode: u32) -> io::Result<File> {
  >     let mut opts = OpenOptions::new();
  >     opts.write(true).create_new(true);
  >     #[cfg(unix)]
  >     std::os::unix::fs::OpenOptionsExt::mode(&mut opts, mode);
  >     #[cfg(not(unix))]
  >     let _ = mode; // no permission bits to set
  >     opts.open(tmp)
  > ```

- **Fact:** `decode_params` keeps a guard's own code and turns any other decode failure into `usage` carrying serde's own text, which can quote the input; import therefore replaces the text of a spec's `usage` and keeps every other code.
  **Source:** `crates/holler-pane/src/reply.rs:115-117` and `crates/holler-pane/src/error.rs:630-636`
  **Verbatim excerpt:**
  > ```rust
  > pub fn decode_params<T: DeserializeOwned>(params: Value) -> Result<T, PaneError> {
  >     serde_json::from_value(params).map_err(|e| PaneError::from_decode(&e))
  > }
  > ```
  > ```rust
  >     pub(crate) fn from_decode(err: &serde_json::Error) -> PaneError {
  >         let text = err.to_string();
  >         match PaneCode::split_prefix(&text) {
  >             Some((code, rest)) => PaneError::from_closed(code, rest.to_owned(), None),
  >             None => PaneError::Usage { message: text },
  >         }
  >     }
  > ```

- **Fact:** an env entry with `=` is `profile-secret-refused` and a blank one `env-name-invalid`, and a bare string with `=` in place of the list is `profile-secret-refused`; neither refusal carries the text. This is the only secret check import makes.
  **Source:** `crates/holler-pane/src/argv.rs:101-109` and `:182-192`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn parse(text: &str) -> Result<Self, PaneError> {
  >         if text.contains('=') {
  >             return Err(PaneError::ProfileSecretRefused);
  >         }
  >         if text.is_empty() || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
  >             return Err(PaneError::EnvNameInvalid);
  >         }
  >         Ok(Self(text.to_owned()))
  >     }
  > ```
  > ```rust
  >     match Value::deserialize(deserializer)? {
  >         Value::Array(items) => items
  >             .into_iter()
  >             .map(|item| match item {
  >                 Value::String(name) => EnvVarName::parse(&name),
  >                 _ => Err(PaneError::EnvNameInvalid),
  >             })
  >             .collect::<Result<Vec<_>, _>>()
  >             .map_err(refuse),
  >         // A bare string with a `=` is a `NAME=value` entry that lost its list.
  >         Value::String(text) if text.contains('=') => Err(refuse(PaneError::ProfileSecretRefused)),
  > ```

- **Fact:** a command or check that is not an array is `command-not-argv`, with no payload.
  **Source:** `crates/holler-pane/src/argv.rs:65-68`
  **Verbatim excerpt:**
  > ```rust
  >     fn from_value(value: Value) -> Result<Self, PaneError> {
  >         let Value::Array(items) = value else {
  >             return Err(PaneError::CommandNotArgv);
  >         };
  > ```

- **Fact:** `insert_profile` turns a create race (`generation-conflict` on a create) into `profile-exists`; import stores through it after its own `get` check.
  **Source:** `crates/holler-cli/src/profile/create.rs:208-219`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn insert_profile(
  >     store: &dyn ProfileStore,
  >     profile: &Profile,
  >     actor: &Actor,
  > ) -> Result<Profile, PaneError> {
  >     store
  >         .cas_put(profile, 0, actor)
  >         .map_err(|error| match error {
  >             PaneError::Conflict => taken(&profile.name),
  >             other => other,
  >         })
  > }
  > ```

- **Fact:** text output adds a newline only when the text lacks one, so export's text (the document, which ends with a newline) is the whole stdout byte for byte.
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

- **Fact:** `show::text` escapes a stored string's control characters; export prints its `--out` path through it.
  **Source:** `crates/holler-cli/src/profile/show.rs:201-203`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn text(stored: &str) -> String {
  >     FieldValue::Text(stored.to_owned()).to_string()
  > }
  > ```

## The drift finding (reconcile)

- **Fact:** `Finding::about` takes the pane and its record's position, the kind's remedy for that pane (none for `profile-drift`) and `not-fixable`.
  **Source:** `crates/holler-pane/src/findings.rs:272-279`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn about(kind: FindingKind, pane: &Pane, message: String) -> Self {
  >         Self {
  >             pane: Some(pane.name.clone()),
  >             grid: Some(pane.herdr.grid),
  >             remedy: kind.remedy(Some(&pane.name), FixState::NotFixable),
  >             ..Self::new(kind, message)
  >         }
  >     }
  > ```

- **Fact:** an `observe-failed` message names the call and the error's code, so a failed `profile_store.list` says `profile_store.list failed (<code>): ...`.
  **Source:** `crates/holler-pane/src/reconcile.rs:448-454`
  **Verbatim excerpt:**
  > ```rust
  > fn failed_message(op: &str, error: &PaneError) -> String {
  >     format!(
  >         "{op} failed ({}): {}",
  >         error.code(),
  >         embedded(&error.to_string())
  >     )
  > }
  > ```

## Why two of T's conformance cases fail over the wire only

- **Fact:** the CLI's `HubProfileStore` (the third runner of the suite, `pane_integration/clients.rs`) makes a watch's first long-poll inside `watch()` and keeps that reply for the first `next()`; a reply with no events stands for one idle item. A watch opened at the head therefore yields `Ok(None)` at its first `next()` for the window that passed inside `watch()`, even if a write happened after `watch()` returned. Cases 19 and 21 of the suite read that first idle item before they write; cases 26 and 27 (`rename-of-one-slug-is-one-put`, `rename-feed-is-deletion-then-put`) rename right after `open(...)` and fail with "got None".
  **Source:** `crates/holler-cli/src/pane/wiring/store.rs:274-287` and `:291-300`
  **Verbatim excerpt:**
  > ```rust
  >     /// The watch of this `*/watch` method from `since`, its first request made now (D4).
  >     fn watch<E>(self, since: Cursor) -> Result<Watch<E>, PaneError>
  >     where
  >         E: DeserializeOwned + Send + 'static,
  >     {
  >         let mut feed = Feed {
  >             method: self,
  >             since,
  >             batch: None,
  >             ended: false,
  >         };
  >         feed.batch = Some(feed.fetch()?);
  >         Ok(Box::new(feed))
  >     }
  > ```
  > ```rust
  > struct Feed<E> {
  >     method: Method,
  >     /// Where the next request resumes: the cursor of the last reply.
  >     since: Cursor,
  >     /// What is left of the last reply's batch, or `None` when the next item needs a request. A
  >     /// batch with no events stands for one idle item.
  >     batch: Option<VecDeque<E>>,
  >     /// An error ended the stream.
  >     ended: bool,
  > }
  > ```

- **Fact:** the suite's own case 19 reads the first idle item of a fresh watch before its first write, which is what makes it hold for that client.
  **Source:** `crates/holler-pane-testkit/src/conformance/profile_store/watch.rs:48-54`
  **Verbatim excerpt:**
  > ```rust
  >     let mut watch = open(store, Cursor(0))?;
  >     expect_eq(
  >         "the first next() on an empty store",
  >         next_item(&mut watch)?,
  >         None,
  >     )?;
  >     let created = put(store, &p, 0, &who)?;
  > ```

## F rework 1 (after S REWORK, items 1 and 2): the spec count and the import refusals

- **Fact:** `count` writes the noun in the singular for 1 and in the plural otherwise, and `create` and `show` print a profile's spec count through it; `import` and `export` now do too.
  **Source:** `crates/holler-cli/src/profile/list.rs:91-99`, `crates/holler-cli/src/profile/create.rs:339`, `crates/holler-cli/src/profile/show.rs:126`
  **Verbatim excerpt:**
  > ```rust
  > /// `n` and `noun`, with the noun in the plural unless `n` is 1: `1 pane`, `2 panes`,
  > /// `0 specs`.
  > pub(crate) fn count(n: usize, noun: &str) -> String {
  >     if n == 1 {
  >         format!("1 {noun}")
  >     } else {
  >         format!("{n} {noun}s")
  >     }
  > }
  > ```
  > ```rust
  >         count(profile.panes.len(), "spec"),
  > ```

- **Fact:** `GridPos::parse` refuses a text position with an excerpt of that text (up to 64 characters) in both grid codes. Import reads a file's text position through it (`read_position`), so without the replacement in `spec_refused` the file's text reached the message.
  **Source:** `crates/holler-pane/src/grid.rs:44-55`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn parse(text: &str) -> Result<GridPos, PaneError> {
  >         let lower = text.trim_ascii().to_ascii_lowercase();
  >         let (row, col) = split_cells(&lower).ok_or_else(|| PaneError::GridAmbiguous {
  >             what: format!("{} ({FORMS})", excerpt(text)),
  >         })?;
  >         match (coordinate(row), coordinate(col)) {
  >             (Some(row), Some(col)) => Ok(GridPos { row, col }),
  >             _ => Err(PaneError::GridOutOfRange {
  >                 what: format!("{} ({BOUNDS})", excerpt(text)),
  >             }),
  >         }
  >     }
  > ```

- **Fact:** a position in the record's object form is checked by `GridPos`'s deserializer, during the spec's decode. A `pos` label that disagrees is `grid-ambiguous` quoting the label, and a zero cell is `grid-out-of-range` naming both numbers.
  **Source:** `crates/holler-pane/src/grid.rs:157-168` and `:58-63`
  **Verbatim excerpt:**
  > ```rust
  >         if let Some(label) = raw.pos {
  >             if GridPos::parse(&label).ok() != Some(cell) {
  >                 let err = PaneError::GridAmbiguous {
  >                     what: format!(
  >                         "pos {} disagrees with row {} and column {}",
  >                         excerpt(&label),
  >                         raw.row,
  >                         raw.col
  >                     ),
  >                 };
  >                 return Err(de::Error::custom(err.coded_message()));
  >             }
  > ```
  > ```rust
  >     fn from_cells(row: u16, col: u16) -> Result<GridPos, PaneError> {
  >         if row == 0 || col == 0 {
  >             return Err(PaneError::GridOutOfRange {
  >                 what: format!("row {row}, column {col} ({BOUNDS})"),
  >             });
  >         }
  > ```

- **Fact:** `decode_params`'s `from_closed` keeps the coded message's text in the two grid variants, so the excerpt survives the decode. The other codes a spec's decode can answer (`command-not-argv`, `env-name-invalid`, `profile-secret-refused`) are unit variants and carry no text. This is why `spec_refused` replaces the two grid messages and leaves the rest alone.
  **Source:** `crates/holler-pane/src/error.rs:568-577` and `:585`
  **Verbatim excerpt:**
  > ```rust
  >     fn from_closed(code: PaneCode, message: String, detail: Option<String>) -> PaneError {
  >         let text = detail.unwrap_or(message);
  >         match code {
  >             PaneCode::NotImplemented => PaneError::NotImplemented,
  >             PaneCode::Usage => PaneError::Usage { message: text },
  >             PaneCode::GridAmbiguous => PaneError::GridAmbiguous { what: text },
  >             PaneCode::GridOutOfRange => PaneError::GridOutOfRange { what: text },
  >             PaneCode::CommandNotArgv => PaneError::CommandNotArgv,
  >             PaneCode::EnvNameInvalid => PaneError::EnvNameInvalid,
  >             PaneCode::GenerationConflict => PaneError::Conflict,
  > ```
  > ```rust
  >             PaneCode::ProfileSecretRefused => PaneError::ProfileSecretRefused,
  > ```

- **Fact:** the hint and the bounds that import's grid refusals carry are holler-pane's private constants, which import repeats as `GRID_FORMS` and `GRID_BOUNDS`.
  **Source:** `crates/holler-pane/src/grid.rs:24` and `:27`
  **Verbatim excerpt:**
  > ```rust
  > const FORMS: &str = "write r<row>c<col>, c<col>r<row> or <row>,<col>";
  > ```
  > ```rust
  > const BOUNDS: &str = "rows and columns run from 1 to 65535";
  > ```

- **Fact:** a verb's error message is the error's `Display`, which for the two grid codes is a fixed prefix and then the `what` import builds. So the message is `ambiguous grid position: herdr.grid of spec N (...)` or `grid position out of range: herdr.grid of spec N (...)`.
  **Source:** `crates/holler-cli/src/output.rs:134-141` and `crates/holler-pane/src/error.rs:644-647`
  **Verbatim excerpt:**
  > ```rust
  > impl From<&PaneError> for ErrorBody {
  >     fn from(error: &PaneError) -> Self {
  >         Self {
  >             code: ErrorCode::from(error),
  >             message: error.to_string(),
  >         }
  >     }
  > }
  > ```
  > ```rust
  >             PaneError::GridAmbiguous { what } => write!(f, "ambiguous grid position: {what}"),
  >             PaneError::GridOutOfRange { what } => {
  >                 write!(f, "grid position out of range: {what}")
  >             }
  > ```

- **Fact:** `SpecField::Grid`'s path is `herdr.grid`, the name `profile show` and the drift finding give that member. Import's grid refusals name the member by it.
  **Source:** `crates/holler-pane/src/profile_diff.rs:81`
  **Verbatim excerpt:**
  > ```rust
  >             SpecField::Grid => "herdr.grid",
  > ```

- **Fact:** `ProfileName::parse` quotes an invalid name. A document's name that is not a valid name is therefore quoted, as `--name` is, and a valid one is printed on success. The docs therefore scope the claim to what a **spec** holds, not to the whole file.
  **Source:** `crates/holler-pane/src/profile.rs:55-58`
  **Verbatim excerpt:**
  > ```rust
  >         match refusal {
  >             Some(rule) => Err(PaneError::Usage {
  >                 message: format!("invalid profile name {}: {rule}", excerpt(text)),
  >             }),
  > ```
