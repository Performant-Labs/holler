# Evidence — #643 the read verbs

Facts in code the diff does not show, quoted verbatim with `file:line` (in this repo unless marked). The gate reads
only the first 12,000 bytes, so the newest round is first, and facts the brief already quotes are pointers.

## F (Phase 5, implement, amendment 1) — 2026-10-09

The brief quotes `shown_differs` and its reader's proviso (`reconcile.rs:171-181`), reconcile's writes, never
`driven` (`observe.rs:361-375`), and a sample pane with no session of record, so AC 1 and AC 2 stay `unobserved`
(`fixture.rs:34-41`). Its verb-file cites are at 05e7337: this round moves `list.rs` lines down 4 to 19 (19 from
`SessionSync::text` on), and `get.rs` and `watch.rs` lines down 2.

- **Fact (W-13; departs from the letter of Decision 3's `at == 0`):** `at` is a plain `i64`, so a record can hold a
  negative one. `SessionSync::of` guards with `at <= 0`, the doc's "provided `at > 0`" and where `observed_at`
  prints `never`.
  **Source:** `crates/holler-pane/src/pane.rs:188-189`
  **Verbatim excerpt:**
  > ```
  >     /// When it was observed (milliseconds since the Unix epoch).
  >     pub at: i64,
  > ```

- **Fact (W-12; departs from Decision 7's wording, "slug compared, as `ProfileName::slug`"):** `watch --profile`
  tests membership with `profile_diff::is_member`, public since #662 part 1 and called by `profile list` and
  `profile show` (`crates/holler-cli/src/profile/list.rs:59`, `crates/holler-cli/src/profile/show.rs:89`). It is
  the same slug comparison, so nothing printed changes.
  **Source:** `crates/holler-pane/src/profile_diff.rs:258-265`
  **Verbatim excerpt:**
  > ```
  > /// This is what "a live pane of a profile" means (ADR-0021 section 3): `profile list`'s
  > /// live count, `profile show`'s comparison and `profile delete`'s refusal
  > /// `profile-has-live-panes` all use it.
  > pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
  >     pane.profile
  >         .as_ref()
  >         .is_some_and(|own| own.slug() == profile.slug())
  > }
  > ```

- **Fact (W-16; a decided non-fold, as for `findings::quoted`):** #662 part 1 prints a stored string a third way,
  `FieldValue`'s `Display`, which `profile show` uses (`crates/holler-cli/src/profile/show.rs:196-198`). It escapes
  only `is_control` characters and never quotes, so a space, an `=` or a bidi override prints as itself. `text_value`
  cannot fold onto it: AC 18 and the `key=value` watch line need values quoted, and its bidi escaping would be lost.
  `ProbeResult` also prints two ways: `failed missing=["ok"]` here, `failed (missing "ok")` in `profile show`
  (`crates/holler-cli/src/profile/show.rs:181-191`). Follow-up: D-2, widened to the three rules.
  **Source:** `crates/holler-pane/src/profile_diff.rs:197-199`
  **Verbatim excerpt:**
  > ```
  >     for c in text.chars() {
  >         if c.is_control() {
  >             write!(f, "{}", c.escape_default())?;
  > ```

## F (Phase 5, implement, round 2) — 2026-10-09

A's W-9 deviations are now in the brief (Decisions 2, 6 and 11 and Risks, "As built"). The source for Decision 6's
claim: `serde_json` escapes only `\x00`-`\x1F`, `"` and `\` (serde_json 1.0.151, `src/ser.rs:2142`:
`const UU: u8 = b'u'; // \x00...\x1F except the ones above`).

### The round-1 gate's findings, settled from source

- **Fact (B-1):** a `what` that embeds a profile name in a sentence quotes it with `{:?}`, as `get.rs:107` does. The
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
  `escape_debug` writes `\u{..}` for exactly the classes below (bidi is a format control; ZWSP, ZWJ and the BOM are
  default-ignorable). `{:?}` of a `str` (what `text_value` prints) sets the grapheme-extender flag, and
  `acts_on_terminal` (`list.rs:324-326`) is that test plus `is_control`: a rustc 1.98.1 probe of 39 characters agrees
  on all, and `é` and CJK print as themselves.
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

- **Fact (NV-1, W-2):** `json_text` gets an `Argv`, a `Vec<String>` or a `ProfileSpec` (`get.rs:190-194`, `:269`),
  none holding a map (the brief lists `ProfileSpec`'s fields). So `serde_json::to_string` cannot fail there, and
  `list.rs:306`'s `unwrap_or_default()` is unreachable.
  **Source:** `crates/holler-pane/src/argv.rs:25-27`
  **Verbatim excerpt:**
  > ```
  > #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
  > #[serde(transparent)]
  > pub struct Argv(Vec<String>);
  > ```

- **Fact (NV-5):** both `Watch` iterators return `None` only after they have yielded an error, and `emit_stream` stops
  at an error item. So the `?` at `watch.rs:163` never ends a stream that has not failed.
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

Round 1's entries were pointers to spans the brief quotes, and a slug fact that W-12's `is_member` (above) replaces.

## T (Phase 7, verify / GREEN) — 2026-10-09

Facts in unchanged test-kit code that the tests rely on, excerpts copied from source by T.

- **Fact:** a sample pane works in `/srv/demo` (the PROJECT cell of AC 1 and AC 18's clean comparison pane).
  **Source:** `crates/holler-pane-testkit/src/fixture.rs:29`, `:54`
  **Verbatim excerpt:**
  > ```
  > const SAMPLE_CWD: &str = "/srv/demo";
  > ```
  > ```
  >             cwd: SAMPLE_CWD.to_owned(),
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

- Pointers (the brief quotes them): the feed's rules from `Cursor(0)` and from a later cursor (AC 12 vs AC 13, and why
  the leave test uses `--since 1`; the test kit's `feed.rs:22-26`), the fake's `list` already sorted by name, so no
  fake-backed test can tell whether `list` sorts (`pane_store.rs:54`), and `check_ndjson` refusing an empty stream, so
  AC 16 checks the empty output directly (`envelope.rs:243-249`).

## T (Phase 4, author / RED, round 2) — 2026-10-09

Facts in unchanged test-kit code that the round-2 tests and the W-4 triage rely on, excerpts copied from source by T.

- **Fact:** every port method of a fake records its op in the call log first, before any delay or fault. So an empty
  set of write ops in `faults().calls()` means no write was attempted (`read_verbs_call_no_adapter_or_probe`, the
  profile-store half added for the gate's W-5).
  **Source:** `crates/holler-pane-testkit/src/fault.rs:90-92`
  **Verbatim excerpt:**
  > ```
  >     /// What a fake calls first in every port method. It records the call, sleeps for
  >     /// the delay (without holding the lock, so other calls proceed), and then answers
  >     /// the standing fault if there is one, or else the oldest error queued for `op`.
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
