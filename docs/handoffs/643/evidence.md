# Evidence — #643 the read verbs

Facts in code the diff does not show, quoted verbatim with `file:line` (in this repo unless marked). The gate reads
only the first 12,000 bytes, so round 2 is first, and facts the brief already quotes are pointers.

## F (Phase 5, implement, round 2) — 2026-10-09

### Deviations from the brief (A's W-9)

Each is deliberate, in the file and layer the brief chose, and pinned by a test in the diff.

1. **Decision 2 lists four shared `pub` items; there are eleven.** Also `COLUMNS`, `NO_VALUE` (`list.rs:26-31`),
   `profile_name` (`:134-136`), `health_word`, `hold_word` (`:234-249`), `optional_text` (`:275-277`) and `json_text`
   (`:284-299`), for `get.rs:18-21` and `watch.rs:17`, one copy each (the frozen `pane/mod.rs` admits no module).
2. **Decision 6's `serde_json::to_string` is `json_text`:** the same JSON value, with each character
   `acts_on_terminal` flags as a `\u` escape, since `serde_json` escapes only `\x00`-`\x1F`, `"` and `\` (serde_json
   1.0.151, `src/ser.rs:2142`: `const UU: u8 = b'u'; // \x00...\x1F except the ones above`).
3. **Decision 11's trigger set is wider.** `text_value` also quotes a stored `-`, so that it never reads as the empty
   value (`list.rs:50-52`, `get.rs:37-39`), and every character `acts_on_terminal` (`list.rs:305-307`) flags.
4. **So the Risks bullet's "bidi ... passed through" is false in text mode.** JSON mode still writes them raw (#660).

### The round-1 gate's findings, settled from source

- **Fact (B-1):** a `what` that embeds a profile name in a sentence quotes it with `{:?}`, as `get.rs:105` does. The
  fake scope's `pane-not-in-profile` is that text byte for byte, and the hub's `pane-in-other-profile` and slug clash
  (`crates/holler-hub/src/profile/store.rs:397-398`) quote the same way. A profile name may hold spaces.
  **Source:** `crates/holler-pane-testkit/src/profile_scope.rs:122-123`, `crates/holler-hub/src/panes/store.rs:348-350`,
  `crates/holler-pane/src/profile.rs:30`
  **Verbatim excerpt:**
  > ```
  >             _ => Err(PaneError::PaneNotInProfile {
  >                 what: format!("{name} is not in profile {:?}", profile.as_str()),
  > ```
  > ```
  >             Err(PaneError::PaneInOtherProfile {
  >                 what: format!(
  >                     "{} is in profile {:?}, not {:?}",
  > ```
  > ```
  > /// The display name of a profile: spaces allowed, e.g. `Some Profile`.
  > ```

- **Fact (B-2, W-3):** no pane or profile name can be `-`, so a `-` in a name cell is always the empty value. A pane
  name starts and ends with `[0-9a-z]` (the brief quotes `vocab.rs:209-223`), and a profile name needs an ASCII letter
  or digit. Other stored strings (a session id, a cwd) can be `-`, and they print as `"-"`.
  **Source:** `crates/holler-pane/src/profile.rs:50-51`
  **Verbatim excerpt:**
  > ```
  >         } else if slugify(name).is_empty() {
  >             Some("a profile name needs at least one ASCII letter or digit")
  > ```

- **Fact (B-3, NV-6, W-1):** beyond printable ASCII and the backslash forms of `\0`, `\t`, `\n` and `\r`,
  `escape_debug` (`:546`, `ESCAPE_ALL`) writes `\u{..}` for exactly: control, private use, whitespace, grapheme
  extender (combining marks, VS16), default-ignorable (ZWSP, ZWJ, soft hyphen, BOM), format control (bidi, LRM, ALM)
  and unassigned characters. `é` and CJK print as themselves. So
  `acts_on_terminal` is that explicit list plus `is_control`, and `{:?}` of a `str` (what `text_value` prints) escapes
  with the same function and flag. A rustc 1.98.1 probe of 39 such and plain characters: they agree on all 39.
  **Source:** library source, rust-lang/rust tag `1.98.1` (this repo's toolchain):
  `library/core/src/char/methods.rs:489-503`, `library/core/src/fmt/mod.rs:2943-2944`
  **Verbatim excerpt:**
  > ```
  >             // ASCII fast path
  >             '\x20'..='\x7E' => EscapeDebug::printable(self),
  >
  >             _ if self.is_control()
  >                 || self.is_private_use()
  >                 || self.is_whitespace()
  >                 || args.escape_grapheme_extender && self.is_grapheme_extender()
  >                 || self.is_default_ignorable()
  >                 || self.is_format_control()
  >                 || !self.is_assigned() =>
  >             {
  >                 EscapeDebug::unicode(self)
  >             }
  >
  >             _ => EscapeDebug::printable(self),
  > ```
  > ```
  >                 let esc = c.escape_debug_ext(EscapeDebugExtArgs {
  >                     escape_grapheme_extender: true,
  > ```

- **Fact (NV-1, W-2):** `json_text` gets an `Argv`, a `Vec<String>` or a `ProfileSpec` (`get.rs:188-192`, `:267`),
  none holding a map (the brief lists `ProfileSpec`'s fields). So `serde_json::to_string` cannot fail there, and
  `list.rs:287`'s `unwrap_or_default()` is unreachable.
  **Source:** `crates/holler-pane/src/argv.rs:25-27`
  **Verbatim excerpt:**
  > ```
  > #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
  > #[serde(transparent)]
  > pub struct Argv(Vec<String>);
  > ```

- **Fact (NV-5):** both `Watch` iterators return `None` only after they have yielded an error, and `emit_stream` stops
  at an error item. So the `?` at `watch.rs:161` never ends a stream that has not failed.
  **Source:** `crates/holler-pane-testkit/src/feed.rs:278-282`, `crates/holler-hub/src/panes/store.rs:296-297`, `:311`
  **Verbatim excerpt:**
  > ```
  >         let feed = self.feed.take()?;
  >         let item = self.step(&feed);
  >         if item.is_ok() {
  >             self.feed = Some(feed);
  >         }
  > ```
  > ```
  >     /// `None` once an error has ended the stream.
  >     store: Option<Arc<Store>>,
  > ```
  > ```
  >         let store = self.store.as_ref()?;
  > ```

## F (Phase 5, implement) — 2026-10-09

Pointers to facts the brief quotes: `GridPos`'s key order (`crates/holler-pane/src/grid.rs:129-138`); JSON mode
serializes the data itself, `emit` adds a missing newline, `emit_stream` stops at the first non-zero exit
(`crates/holler-cli/src/output.rs:201-243`); idle is `Ok(None)`, any error ends a watch
(`crates/holler-pane/src/ports.rs:36-52`); no `pane` is a delete (`crates/holler-pane/src/pane.rs:256-268`); `resolve`
(`crates/holler-pane/src/profile.rs:380-389`); exit classes (`crates/holler-pane/src/error.rs:266-301`); UTC
(`crates/holler-cli/src/time_fmt.rs:7-11`); `Health`'s serde form (`crates/holler-pane/src/pane.rs:131-150`).

- **Fact:** profiles are the same profile when their slugs are equal, which is how `watch --profile` compares a
  record's `profile` with the scoped one.
  **Source:** `crates/holler-pane/src/profile.rs:70-71`
  **Verbatim excerpt:**
  > ```
  >     /// Two display names with the same slug are the same profile as far as
  >     /// uniqueness goes.
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

- Facts whose excerpts the brief quotes (pointers since round 2, for the 12,000-byte cap): a watch from `Cursor(0)`
  yields one put per live record and resumes from the head, while one from any other cursor yields every change after
  it (AC 12 vs AC 13, and why the leave test uses `--since 1`; `crates/holler-pane-testkit/src/feed.rs:22-26`). The
  fake's `list` is already sorted by name, so no fake-backed test can tell whether `list` sorts by itself
  (`crates/holler-pane-testkit/src/pane_store.rs:54`). `check_ndjson` refuses an empty stream, so AC 16 checks the
  empty `watch` output directly (`crates/holler-pane-testkit/src/envelope.rs:243-249`).

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
