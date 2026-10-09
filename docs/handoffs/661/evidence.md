# Evidence: #661 the hub profile registry

Facts in **unchanged** code that the change (F) or its tests rely on. Line numbers are as of the F diff.

- **Fact:** a missing registry file is an empty registry, an unreadable one is a problem, and the version is checked before the whole document is parsed, then `check` runs over the entries.
  **Source:** `crates/holler-hub/src/panes/persist.rs:128-146`
  **Verbatim excerpt:**
  > pub(crate) fn load_doc<E>(path: &Path) -> Result<Option<Doc<E>>, Problem>
  > where
  >     E: RegistryEntry + DeserializeOwned,
  > {
  >     let bytes = match std::fs::read(path) {
  >         Ok(bytes) => bytes,
  >         Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
  >         Err(err) => return Err(Problem(format!("cannot be read: {}", err.kind()))),
  >     };
  >     let Versioned { version } = serde_json::from_slice(&bytes).map_err(|e| Problem::parse(&e))?;
  >     if version != VERSION {
  >         return Err(Problem(format!(
  >             "version {version} is not one this hub reads (it reads version {VERSION})"
  >         )));
  >     }
  >     let doc: Doc<E> = serde_json::from_slice(&bytes).map_err(|e| Problem::parse(&e))?;
  >     check(&doc)?;
  >     Ok(Some(doc))
  > }

- **Fact:** a document that does not parse (including a serde data error raised by `ProfileEntry`'s `try_from` or by `EnvVarName`'s decode) is reported by category, line and column only; serde's message, which could quote the file or a secret value, is never used.
  **Source:** `crates/holler-hub/src/panes/persist.rs:98-112`
  **Verbatim excerpt:**
  > /// A document that does not parse, described by the error's category and position
  > /// only. serde's own message is never used: it can quote the file.
  > fn parse(err: &serde_json::Error) -> Self {
  >     let kind = match err.classify() {
  >         Category::Syntax => "not valid JSON",
  >         Category::Eof => "JSON that ends early",
  >         Category::Data => "a value this hub cannot read",
  >         Category::Io => "a read error",
  >     };

- **Fact:** `persist::check` refuses two entries with one `name()` (for profiles, one slug) and two entries with one cursor, and `check_entry` refuses a cursor outside `1..=head`, a record whose `record()` name differs from the entry's `name()` (for profiles: the record's slug versus the entry's slug), and a record at generation 0.
  **Source:** `crates/holler-hub/src/panes/persist.rs:160-196`
  **Verbatim excerpt:**
  > fn check<E: RegistryEntry>(doc: &Doc<E>) -> Result<(), Problem> {
  >     let mut names = BTreeSet::new();
  >     let mut cursors = BTreeSet::new();
  >     for entry in &doc.entries {
  >         check_entry(entry, doc.cursor)?;
  >         if !names.insert(entry.name()) {
  >             return Err(Problem(format!("{} has more than one entry", entry.name())));
  >         }
  >         if !cursors.insert(entry.cursor()) {
  > ...
  >     match entry.record() {
  >         Some((record, _)) if record != name => Err(Problem(format!(
  >             "the entry {name} holds the record of {record}"
  >         ))),
  >         Some((_, 0)) => Err(Problem(format!("the record {name} is at generation 0"))),
  >         _ => Ok(()),
  >     }

- **Fact:** the feed rules read an entry only through `cursor()` and `record().is_some()`: from 0, one put per live entry by cursor; at or after the ring's floor, the ring; otherwise one event per entry changed since `since`, tombstones included. The profile store passes the entries' events (`entries.values().map(|entry| &entry.event)`), so the logs never reach the feed.
  **Source:** `crates/holler-hub/src/panes/feed.rs:103-120`
  **Verbatim excerpt:**
  > pub(crate) fn select<'a, E>(
  >     since: Cursor,
  >     head: Cursor,
  >     ring: &Ring<E>,
  >     entries: impl Iterator<Item = &'a E>,
  > ) -> Result<Vec<E>, PaneError>
  > where
  >     E: RegistryEntry + 'a,
  > {
  >     check_since(since, head)?;
  >     Ok(if since == Cursor(0) {
  >         by_cursor(entries.filter(|entry| entry.record().is_some()))
  >     } else if since >= ring.floor {
  >         ring.after(since)
  >     } else {
  >         by_cursor(entries.filter(|entry| entry.cursor() > since))
  >     })
  > }

- **Fact:** a request with no `params` member, or `params: null`, is decoded from `{}`, so `profile/list` and `profile/watch` work without params (AC 26, 42).
  **Source:** `crates/holler-hub/src/panes/handlers.rs:63-68`
  **Verbatim excerpt:**
  > fn params_of(obj: &Value) -> Value {
  >     match obj.get("params") {
  >         None | Some(Value::Null) => Value::Object(Map::new()),
  >         Some(params) => params.clone(),
  >     }
  > }

- **Fact:** `pane/cas_put` runs the membership hook on the blocking pool before the pane store's compare-and-swap takes the pane lock, so the hook's read of the profile registry is never nested inside the pane lock (D8, ADR-0021 §7).
  **Source:** `crates/holler-hub/src/panes/handlers.rs:105-116`
  **Verbatim excerpt:**
  > pub(crate) async fn cas_put(
  >     cid: &CorrelationId,
  >     obj: &Value,
  >     store: Arc<Store>,
  >     profiles: Arc<ProfileState>,
  > ) -> String {
  >     run(cid, obj, move |params: PaneCasPutParams| {
  >         check_membership(&params.pane, &profiles)?;
  >         store.cas_put(&params.pane, params.expected_generation)
  >     })
  >     .await
  > }

- **Fact:** `profile/cas_put`'s params refuse an unknown member, so a request that carries a `log` beside the profile is `usage` (AC 9, 30); the `Profile` inside refuses one too (its `RawProfile` is `deny_unknown_fields`).
  **Source:** `crates/holler-pane/src/reply.rs:168-174`
  **Verbatim excerpt:**
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct ProfileCasPutParams {
  >     pub profile: Profile,
  >     pub expected_generation: u64,
  >     pub actor: Actor,
  > }

- **Fact:** a stored or submitted `Profile` whose slug differs from its name's slug does not decode, so every record in a loaded file is filed under its name's slug (the file check on the entry's slug then needs only `persist`'s record-name comparison).
  **Source:** `crates/holler-pane/src/profile.rs:235-247`
  **Verbatim excerpt:**
  > impl TryFrom<RawProfile> for Profile {
  >     type Error = String;
  >
  >     fn try_from(raw: RawProfile) -> Result<Self, String> {
  >         let expected = raw.name.slug();
  >         if raw.slug != expected {
  >             return Err(format!(
  >                 "profile slug {} does not match its name {} (expected {})",

- **Fact:** an env entry with a `=` is refused by `EnvVarName` itself (`profile-secret-refused`), in a request and in the file alike; the hub adds no scan of its own (AC 15, 28).
  **Source:** `crates/holler-pane/src/argv.rs:97-100`
  **Verbatim excerpt:**
  > pub fn parse(text: &str) -> Result<Self, PaneError> {
  >     if text.contains('=') {
  >         return Err(PaneError::ProfileSecretRefused);
  >     }

- **Fact:** a guard's coded serde error maps back to its own code in `decode_params`, and the guard's payload text is dropped for a variant without one (`ProfileSecretRefused`, `EnvNameInvalid`), so the reply carries no value.
  **Source:** `crates/holler-pane/src/error.rs:627-633`
  **Verbatim excerpt:**
  > pub(crate) fn from_decode(err: &serde_json::Error) -> PaneError {
  >     let text = err.to_string();
  >     match PaneCode::split_prefix(&text) {
  >         Some((code, rest)) => PaneError::from_closed(code, rest.to_owned(), None),
  >         None => PaneError::Usage { message: text },
  >     }
  > }

- **Fact:** the compare-and-swap rule both stores call: a write is applied only at the current generation and moves it up by one; anything else is `generation-conflict`.
  **Source:** `crates/holler-pane/src/generation.rs:25-33`
  **Verbatim excerpt:**
  > pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
  >     if current != expected {
  >         return Err(PaneError::Conflict);
  >     }
  >     current
  >         .checked_add(1)
  >         .ok_or_else(|| PaneError::StoreCorrupt {
  >             what: "a generation counter overflowed".to_owned(),
  >         })
