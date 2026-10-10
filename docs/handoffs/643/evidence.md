# Evidence — #643 the read verbs

Source facts in **unchanged** code that the F diff (`crates/holler-cli/src/pane/{list,get,watch}.rs`) relies on.

## F (Phase 5, implement) — 2026-10-09

- **Fact:** a verb's `data` is serialized as the struct itself inside the envelope (never through a `serde_json::Value`),
  so the JSON keys of `PaneRow`, `PaneChange` and `PaneDetail` come out in field order, and a nested `GridPos` keeps
  its own key order (AC 2, AC 5's raw `"pos":{"row":2,"col":1,"pos":"r2c1"}`).
  **Source:** `crates/holler-cli/src/output.rs:304-306`
  **Verbatim excerpt:**
  > ```
  > fn write_envelope<T: Serialize>(sink: &mut Sink<'_>, envelope: &Envelope<T>) -> io::Result<()> {
  >     match serde_json::to_string(envelope) {
  >         Ok(line) => write_line(sink.out, &line),
  > ```

- **Fact:** `GridPos` serializes as `row`, `col`, `pos`, in that order, with `pos` its `rRcC` display.
  **Source:** `crates/holler-pane/src/grid.rs:129-138`
  **Verbatim excerpt:**
  > ```
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

- **Fact:** `emit_stream` prints item by item and returns at the first item whose exit code is not 0 (an error item,
  or a failed write), so `watch` yields one `Err` and the stream ends there (AC 15).
  **Source:** `crates/holler-cli/src/output.rs:231-237`
  **Verbatim excerpt:**
  > ```
  >     for item in items {
  >         let code = emit(sink, format, item, &text);
  >         if code != 0 {
  >             return code;
  >         }
  >     }
  >     0
  > ```

- **Fact:** a rendered text is written with a newline added when it lacks one, and flushed, so `watch`'s per-line
  render returns no newline and `list`/`get` may end theirs with one.
  **Source:** `crates/holler-cli/src/output.rs:318-324`
  **Verbatim excerpt:**
  > ```
  > fn write_line(to: &mut dyn Write, line: &str) -> io::Result<()> {
  >     to.write_all(line.as_bytes())?;
  >     if !line.ends_with('\n') {
  >         to.write_all(b"\n")?;
  >     }
  >     to.flush()
  > }
  > ```

- **Fact:** a watch's `next()` yields `Ok(None)` for idle (the stream stays usable), and any error ends the stream;
  `next()` blocks for at most I5's bound, so `watch` without `--until-idle` waits on idle without busy-looping on a
  real store.
  **Source:** `crates/holler-pane/src/ports.rs:43-52`
  **Verbatim excerpt:**
  > ```
  > /// - `next()` blocks for at most I5's bound and yields one of three things:
  > ///   - `Ok(Some(change))`: the next change;
  > ///   - `Ok(None)` (the item, not the end of the iterator): **idle**, nothing happened
  > ///     within the bound. This is an ordinary outcome, as in the hub's `control/wait`,
  > ///     and the stream stays usable. A hub long-poll that sees it answers
  > ///     `{events: [], cursor}`;
  > ///   - `Err(..)`: a failure. `Err(PaneError::Timeout)` means the store did not
  > ///     answer within the bound (a wedged store), never "idle". Any error ends the
  > ///     stream (call `watch` again).
  > pub type Watch<T> = Box<dyn Iterator<Item = Result<Option<T>, PaneError>> + Send>;
  > ```

- **Fact:** a `PaneEvent` with no record is a delete, which is how `watch` tells `"put"` from `"delete"`.
  **Source:** `crates/holler-pane/src/pane.rs:265-267`
  **Verbatim excerpt:**
  > ```
  >     /// The record after the change; `None` when the record was deleted.
  >     #[serde(default)]
  >     pub pane: Option<Box<Pane>>,
  > ```

- **Fact:** `ProfileScope::resolve` is the membership check the three verbs delegate to: every pane of P with no name,
  the named pane only if it belongs to P (`pane-not-in-profile`), `profile-not-found` for a missing P.
  **Source:** `crates/holler-pane/src/profile.rs:380-389`
  **Verbatim excerpt:**
  > ```
  > pub trait ProfileScope: Send + Sync {
  >     /// The profile and the panes of it a verb acts on. With no `pane`, every pane
  >     /// of the profile; with a named pane, just that one, which must belong to the
  >     /// profile (`pane-not-in-profile` otherwise). A missing profile is
  >     /// `profile-not-found`.
  >     fn resolve(
  >         &self,
  >         profile: &ProfileName,
  >         pane: Option<&PaneName>,
  >     ) -> Result<ResolvedScope, PaneError>;
  > ```

- **Fact:** the two codes the verbs raise themselves (`pane-not-found` in `get`, `pane-not-in-profile` for a scope
  that returns no such pane) are refusals, so they exit 3 in both formats.
  **Source:** `crates/holler-pane/src/error.rs:285-291` and `crates/holler-pane/src/error.rs:241-242`
  **Verbatim excerpt:**
  > ```
  >         | PaneCode::PaneNotInProfile
  >         | PaneCode::PaneInOtherProfile
  >         | PaneCode::ProbeFailed
  >         | PaneCode::HerdrVersionUnsupported
  >         | PaneCode::ProfileNotFound
  >         | PaneCode::PaneNotFound
  >         | PaneCode::SessionNotFound => ErrorClass::Refusal,
  > ```
  > ```
  >             ErrorClass::Usage => 2,
  >             ErrorClass::Refusal => 3,
  > ```

- **Fact:** `format_epoch` formats epoch seconds in UTC, so `observed_at` labels its text ` UTC`.
  **Source:** `crates/holler-cli/src/time_fmt.rs:7-11`
  **Verbatim excerpt:**
  > ```
  > /// Format a unix epoch second as a UTC `YYYY-MM-DD HH:MM:SS` string
  > /// (the `hub token list`/`mint`/`ping` EXPIRES column). No chrono
  > /// dependency: a manual civil calendar conversion (Howard Hinnant's
  > /// algorithm) is enough for epoch seconds.
  > pub fn format_epoch(secs: u64) -> String {
  > ```

- **Fact:** profiles are the same profile when their slugs are equal, which is how `watch --profile` compares a
  record's `profile` with the scoped one.
  **Source:** `crates/holler-pane/src/profile.rs:68-74`
  **Verbatim excerpt:**
  > ```
  >     /// The unique id derived from the name: ASCII letters and digits in lower case,
  >     /// each run of anything else becoming one `-`, with no leading or trailing `-`.
  >     /// Two display names with the same slug are the same profile as far as
  >     /// uniqueness goes.
  >     pub fn slug(&self) -> String {
  >         slugify(&self.0)
  >     }
  > ```

- **Fact:** `Health` serializes snake_case: `"healthy"`, `"unknown"`, `{"unhealthy": REASON}` (the `health` of a
  `PaneRow`, AC 4).
  **Source:** `crates/holler-pane/src/pane.rs:131-139`
  **Verbatim excerpt:**
  > ```
  > /// What the harness server last reported about its health.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(rename_all = "snake_case")]
  > pub enum Health {
  >     Healthy,
  >     /// Unhealthy, with the reason.
  >     Unhealthy(String),
  >     Unknown,
  > }
  > ```

## T (Phase 7, verify / GREEN) — 2026-10-09

Facts in unchanged test-kit code that the tests rely on, excerpts copied from source by T.

- **Fact:** a sample pane works in `/srv/demo` (the PROJECT cell of AC 1 and AC 18's clean comparison pane).
  **Source:** `crates/holler-pane-testkit/src/fixture.rs:29`, `:51-56`
  **Verbatim excerpt:**
  > ```
  > const SAMPLE_CWD: &str = "/srv/demo";
  > ```
  > ```
  >         host: HostInfo {
  >             name: "localhost".to_owned(),
  >             tmux: name.to_string(),
  >             cwd: SAMPLE_CWD.to_owned(),
  >             herdr_api_version: None,
  >         },
  > ```

- **Fact:** a sample pane's context ceilings are 100000 / 150000 (AC 8).
  **Source:** `crates/holler-pane-testkit/src/fixture.rs:142-148`
  **Verbatim excerpt:**
  > ```
  > /// The context ceilings of every sample pane and spec.
  > fn sample_context() -> ContextCeilings {
  >     ContextCeilings {
  >         soft: 100_000,
  >         hard: 150_000,
  >     }
  > }
  > ```

- **Fact:** seeding stores panes in the order given, and every change takes the next cursor (head + 1), so seeded
  panes get cursors 1, 2, 3, ... in seed order (AC 12-15 and `watch_profile_prints_a_pane_leaving_the_profile_once`).
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:90-99`, `crates/holler-pane-testkit/src/feed.rs:87`
  **Verbatim excerpt:**
  > ```
  >     /// A store holding `panes`, each created at expected generation 0 and so stored
  >     /// at 1, in order. Seeding bypasses the faults and the call log. Two seeds with one
  >     /// name are `generation-conflict`, as a second create would be.
  > ```
  > ```
  >         let cursor = Cursor(self.head.0.checked_add(1).ok_or_else(overflowed)?);
  > ```

- **Fact:** a watch from `Cursor(0)` yields one put per live record and resumes from the head, while one from any other
  cursor yields every change after it (AC 12 vs AC 13, and why the leave test uses `--since 1`).
  **Source:** `crates/holler-pane-testkit/src/feed.rs:22-26`
  **Verbatim excerpt:**
  > ```
  > //! 2. From `Cursor(0)`: one put for each live record, carrying the cursor of that
  > //!    record's last change, in cursor order. The stream then resumes from the head as
  > //!    of that snapshot, as the hub's does, so a record deleted before the snapshot
  > //!    leaves no event at all.
  > //! 3. From any other cursor: every change after it, in order.
  > ```

- **Fact:** the fake's `list` is already sorted by name, so no fake-backed test can tell whether `list` sorts by itself
  (Decision 4's "the verb sorts").
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:54`
  **Verbatim excerpt:**
  > ```
  > /// - `list` is sorted by name.
  > ```

- **Fact:** `check_ndjson` refuses an empty stream, so AC 16 checks the empty `watch` output directly.
  **Source:** `crates/holler-pane-testkit/src/envelope.rs:243-249`
  **Verbatim excerpt:**
  > ```
  >     let lines: Vec<&str> = match stdout.strip_suffix('\n').unwrap_or(stdout) {
  >         "" => Vec::new(),
  >         body => body.split('\n').collect(),
  >     };
  >     let Some((last, before)) = lines.split_last() else {
  >         return Err(EnvelopeFault::EmptyStream);
  >     };
  > ```

## T (Phase 4, author / RED, round 2) — 2026-10-09

Facts in unchanged test-kit code that the round-2 tests and the W-4 triage rely on, excerpts copied from source by T.

- **Fact:** every port method of a fake records its op in the call log first, before any delay or fault. So an empty
  set of write ops in `faults().calls()` means no write was attempted (`read_verbs_call_no_adapter_or_probe`, the
  profile-store half added for the gate's W-5).
  **Source:** `crates/holler-pane-testkit/src/fault.rs:90-98`
  **Verbatim excerpt:**
  > ```
  >     /// What a fake calls first in every port method. It records the call, sleeps for
  >     /// the delay (without holding the lock, so other calls proceed), and then answers
  >     /// the standing fault if there is one, or else the oldest error queued for `op`.
  >     pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
  >         let delay = {
  >             let mut state = self.lock();
  >             state.calls.push(op);
  > ```

- **Fact:** the profile store's write ops are `CasPut`, `Delete` and `Rename`, and its `get` records `Get` (why the
  profile-store half is not vacuous: `get demo-c1r1` without `--profile` reads the pane's profile).
  **Source:** `crates/holler-pane-testkit/src/profile_store.rs:27-37`, `:250-251`
  **Verbatim excerpt:**
  > ```
  > pub enum ProfileStoreOp {
  >     Get,
  >     List,
  >     CasPut,
  >     Delete,
  >     Watch,
  >     /// One `next()` of an open watch.
  >     WatchNext,
  >     Log,
  >     Rename,
  > }
  > ```
  > ```
  >     fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
  >         self.faults.enter(ProfileStoreOp::Get)?;
  > ```

- **Fact:** one `next()` of the fake's watch records `WatchNext` before it polls the feed, and the poll then waits
  for a write. So AC 14's thread can see `WatchNext` and write either just before the poll takes the lock or during
  the wait. Either way that `next()` returns the write, once, which is the invariant the test asserts (the gate's W-4).
  **Source:** `crates/holler-pane-testkit/src/feed.rs:263-269`
  **Verbatim excerpt:**
  > ```
  >     fn step(&mut self, feed: &Feed<E>) -> Result<Option<E>, PaneError> {
  >         self.faults.enter(self.next_op)?;
  >         if self.owed.is_empty() {
  >             let (owed, resume) = feed.poll(self.since);
  >             self.owed = owed.into();
  >             self.since = resume;
  >         }
  > ```
