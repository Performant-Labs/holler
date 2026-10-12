# Evidence: #665, facts in unchanged code that the diff relies on

Line numbers are the branch head's; a script checks each excerpt against its cited lines. The diff gate shows only this file's first 12000 bytes, so a fact the brief already quotes is named by its E-number instead.

## Where each claim can be checked

- **New code is in the diff, whole.**
  - `Store::rename` and `renamed`, in `crates/holler-hub/src/profile/store.rs`: the profile lock, then `PaneState::locked`; the checks in the brief's order; the pane save, then the profile save, and `Locked::restore` when that fails; `apply` only after both saves.
  - `Store::locked` and `Locked`, in `crates/holler-hub/src/panes/store.rs`; `reconcile/drift.rs` (new); the fake's `move_record`.
- **The brief's E-numbers cite the base, `bda1069`.** In a file the diff changes, the SOURCE EXCERPTS show the branch's lines, not the ones an E-number meant, so read the diff there. Its quotes of code the diff leaves unchanged still hold:
  - E7: `entry.rs`'s load `check`, which a tombstone passes when its log ends with `Deleted`, and `ProfileEntry::next`, which appends to the previous log;
  - E8 (`stamp`), E13 (`forward`), E20 (`decode_params`), E23 (the env guard), E26 (`insert_profile`), E32 (`is_member`).
- **The restore's bytes are pinned** by `a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds` (`crates/holler-hub/tests/profile_rename_faults_test.rs`): the previous bytes, a new inode, memory unchanged.

## The rename transaction (hub)

- **Fact:** the one pane path that reads the profile registry is `pane/cas_put`'s membership hook. It runs, and releases the profile lock, before the pane write takes the pane lock. So the rename's order (the profile lock, then the pane lock) cannot deadlock against it.
  **Source:** `crates/holler-hub/src/panes/handlers.rs:111-114`
  **Verbatim excerpt:**
  >     run(cid, obj, move |params: PaneCasPutParams| {
  >         check_membership(&params.pane, &profiles)?;
  >         store.cas_put(&params.pane, params.expected_generation)
  >     })

- **Fact:** a registry save writes the whole document through `write_atomic` (a temporary file, then a rename). So every save, the restore included, replaces the file in one step at a new inode, which is how the panes-first test sees the order.
  **Source:** `crates/holler-hub/src/panes/persist.rs:150` and `crates/holler-hub/src/panes/persist.rs:156`
  **Verbatim excerpt:**
  > pub(crate) fn save_doc<E: Serialize>(path: &Path, doc: &Doc<E>) -> Result<(), Problem> {
  >     write_atomic(path, &bytes, FILE_MODE).map_err(|e| Problem::not_written(&e))

- **Fact:** the process tests' `LiveHub` is the real binary's `holler hub serve` on its own state dir, and the hub hands every `profile/*` method to `profile::dispatch` with both registries. So `process/profile_io.rs`'s round trip reaches the real `profile/rename` transaction.
  **Source:** `crates/holler-cli/tests/pane_verbs/process/main.rs:91`, `crates/holler-cli/tests/support/mod.rs:289` and `crates/holler-hub/src/pane_dispatch.rs:83-84`
  **Verbatim excerpt:**
  >         let hub = support::Hub::start(&state);
  >         cmd.args(["hub", "serve"])
  >     } else if is_profile_method(method) {
  >         profile::dispatch(method, cid, obj, &deps.profiles, &deps.panes).await

## The verbs (CLI)

- **Fact:** `write_atomic` writes a temporary file beside the destination at exactly `mode`, then renames it over the destination, replacing a file there, and removes it if the rename fails. It creates no directory: a missing parent is `NotFound` and nothing is written (an export test pins this).
  **Source:** `crates/holler-proto/src/atomic_file.rs:66-68` and `crates/holler-proto/src/atomic_file.rs:145-147`
  **Verbatim excerpt:**
  > pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
  >     let tmp = write_temp(path, bytes, mode, false)?;
  >     std::fs::rename(&tmp, path).inspect_err(|_| discard(&tmp))
  > fn fill(mut file: File, bytes: &[u8], mode: u32, durable: bool) -> io::Result<()> {
  >     set_mode(&file, mode)?;
  >     file.write_all(bytes)?;

- **Fact:** a decode failure keeps a guard's own code, and any other becomes `usage` carrying serde's text, which can quote the input: import replaces a spec's `usage` message for that reason. Of the codes a spec's decode answers, only the two grid codes keep text; `command-not-argv`, `env-name-invalid` and `profile-secret-refused` carry none, and an env entry with `=` is `profile-secret-refused`.
  **Source:** `crates/holler-pane/src/error.rs:632-634`, `crates/holler-pane/src/error.rs:573-576`, `crates/holler-pane/src/error.rs:585` and `crates/holler-pane/src/argv.rs:102-103`
  **Verbatim excerpt:**
  >         match PaneCode::split_prefix(&text) {
  >             Some((code, rest)) => PaneError::from_closed(code, rest.to_owned(), None),
  >             None => PaneError::Usage { message: text },
  >             PaneCode::GridAmbiguous => PaneError::GridAmbiguous { what: text },
  >             PaneCode::GridOutOfRange => PaneError::GridOutOfRange { what: text },
  >             PaneCode::CommandNotArgv => PaneError::CommandNotArgv,
  >             PaneCode::EnvNameInvalid => PaneError::EnvNameInvalid,
  >             PaneCode::ProfileSecretRefused => PaneError::ProfileSecretRefused,
  >         if text.contains('=') {
  >             return Err(PaneError::ProfileSecretRefused);

- **Fact:** both grid codes quote what they refuse: `GridPos::parse` an excerpt of a text position, the object form's check its `pos` label. Import reads a text position through `GridPos::parse`, so `spec_refused` replaces both messages with `herdr.grid` (`SpecField::Grid`'s path), the spec's number, and the hint or bounds, which repeat holler-pane's private `FORMS` and `BOUNDS`. A verb prints an error as its `Display`: a fixed prefix, then that text.
  **Source:** `crates/holler-pane/src/grid.rs:47`, `crates/holler-pane/src/grid.rs:52`, `crates/holler-pane/src/grid.rs:161-162`, `crates/holler-pane/src/grid.rs:24`, `crates/holler-pane/src/grid.rs:27`, `crates/holler-pane/src/profile_diff.rs:81`, `crates/holler-pane/src/error.rs:644` and `crates/holler-cli/src/output.rs:138`
  **Verbatim excerpt:**
  >             what: format!("{} ({FORMS})", excerpt(text)),
  >                 what: format!("{} ({BOUNDS})", excerpt(text)),
  >                         "pos {} disagrees with row {} and column {}",
  >                         excerpt(&label),
  > const FORMS: &str = "write r<row>c<col>, c<col>r<row> or <row>,<col>";
  > const BOUNDS: &str = "rows and columns run from 1 to 65535";
  >             SpecField::Grid => "herdr.grid",
  >             PaneError::GridAmbiguous { what } => write!(f, "ambiguous grid position: {what}"),
  >             message: error.to_string(),

- **Fact:** `ProfileName::parse` quotes a name it refuses, so a document's invalid `name` is quoted as `--name` is. The docs therefore promise only that no refusal quotes what a **spec** holds.
  **Source:** `crates/holler-pane/src/profile.rs:57`
  **Verbatim excerpt:**
  >                 message: format!("invalid profile name {}: {rule}", excerpt(text)),

- **Fact:** a text result gets a newline only when it lacks one, so export's document, which ends with one, is stdout byte for byte.
  **Source:** `crates/holler-cli/src/output.rs:320-321`
  **Verbatim excerpt:**
  >     if !line.ends_with('\n') {
  >         to.write_all(b"\n")?;

## The drift finding (reconcile)

- **Fact:** `Finding::about` takes the pane, its record's position, and the kind's remedy (none for `profile-drift`), as `not-fixable`; an `observe-failed` message names the call and the error's code (`profile_store.list failed (<code>): ...`).
  **Source:** `crates/holler-pane/src/findings.rs:274-276` and `crates/holler-pane/src/reconcile.rs:450-451`
  **Verbatim excerpt:**
  >             pane: Some(pane.name.clone()),
  >             grid: Some(pane.herdr.grid),
  >             remedy: kind.remedy(Some(&pane.name), FixState::NotFixable),
  >         "{op} failed ({}): {}",
  >         error.code(),

## Conformance cases 26 and 27 over the wire

- **Fact:** the CLI's `HubProfileStore`, the suite's third runner, makes a watch's first long-poll inside `watch()` and keeps that reply for the first `next()`; a reply with no events is one idle item. A watch opened at the head therefore yields `Ok(None)` first, and cases 26 and 27 read it before they rename, as case 19 does.
  **Source:** `crates/holler-cli/src/pane/wiring/store.rs:274`, `crates/holler-cli/src/pane/wiring/store.rs:285` and `crates/holler-cli/src/pane/wiring/store.rs:296`
  **Verbatim excerpt:**
  >     /// The watch of this `*/watch` method from `since`, its first request made now (D4).
  >         feed.batch = Some(feed.fetch()?);
  >     /// batch with no events stands for one idle item.

## T rework (Phase 7, after S REWORK items 3 and 4)

Facts from unchanged code that T's repaired tests rely on. Each excerpt was copied by T from source.

- **Fact:** the hub's profile registry stamps by the wall clock, in milliseconds since the Unix epoch. H1 (`the_record_moves_to_the_new_name_and_the_reply_is_the_stored_record`) waits on the same function until it has passed the create's stamp, so a rename that kept the old `updated` cannot equal its own stamp.
  **Source:** `crates/holler-proto/src/clock.rs:35-40`
  **Verbatim excerpt:**
  > ```rust
  > pub fn now_millis() -> i64 {
  >     std::time::SystemTime::now()
  >         .duration_since(std::time::UNIX_EPOCH)
  >         .map(|d| d.as_millis() as i64)
  >         .unwrap_or(0)
  > }
  > ```

- **Fact:** a hub write's stamp is `now`, or later if the entry already holds a later `at` or `updated`; it is the log entry's `at` and the record's `updated` alike. So "the record's `updated` equals the `at` of the new name's one entry" is the hub's own rule, which H1 and conformance case 23 assert.
  **Source:** `crates/holler-hub/src/profile/entry.rs:172-178`
  **Verbatim excerpt:**
  > ```rust
  > /// record's `updated`. The one stamp is the log entry's `at`, the record's `updated`, and a
  > /// create's `created`, so a log's `at` never decreases and a record's `updated` never goes
  > /// backwards, even when the clock steps back.
  > pub(crate) fn stamp(entry: Option<&ProfileEntry>, now: i64) -> i64 {
  >     entry.map_or(now, |entry| {
  >         let logged = entry.log.last().map_or(now, |last| last.at);
  >         let updated = entry.profile().map_or(now, |record| record.updated);
  > ```

- **Fact:** the fake's clock is 0 unless a test sets it. Conformance case 23's `updated` check therefore holds on the fake but cannot tell a kept `updated` from a new stamp there; H1 is the control for that.
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:91-93`
  **Verbatim excerpt:**
  > ```rust
  > /// - The clock ([`FakeProfileStore::set_now`], 0 by default): a create stamps
  > ///   `created` and `updated`, an update keeps `created` and stamps `updated`, and each
  > ///   log entry's `at` is the time of its write.
  > ```
