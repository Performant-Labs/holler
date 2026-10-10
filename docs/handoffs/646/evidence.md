# Evidence: #646 part 1 of 3 (646a), `holler pane park` and `holler pane unpark`

Source facts in **unchanged** code that the diff (`crates/holler-cli/src/pane/park.rs`, `unpark.rs`) and T's tests rely
on. Line numbers are those of the branch after F's change (ADR-0021 lines after line 130 moved down by the 10 inserted
lines).

## F (Phase 5, implement)

- **Fact:** `ErrorBody::from(&PaneError)` carries the error's own code and its `Display` text as the message, so a usage,
  scope or store error that is "answered as it is" prints the error's own words.
  **Source:** `crates/holler-cli/src/output.rs:134-141`
  **Verbatim excerpt:**
  > impl From<&PaneError> for ErrorBody {
  >     fn from(error: &PaneError) -> Self {
  >         Self {
  >             code: ErrorCode::from(error),
  >             message: error.to_string(),
  >         }
  >     }
  > }

- **Fact:** `ErrorCode::from(&PaneError)` is that error's code, so the composed failed-write message keeps the failing
  write's code (`generation-conflict`, `unavailable`, ...).
  **Source:** `crates/holler-cli/src/output.rs:118-123`
  **Verbatim excerpt:**
  > impl From<&PaneError> for ErrorCode {
  >     /// A `PaneError` always has a valid code, so this cannot fail.
  >     fn from(error: &PaneError) -> Self {
  >         Self(error.code().to_owned())
  >     }
  > }

- **Fact:** in text mode an error prints as `error: <message>` on `err`, with the exit code of the error's class.
  **Source:** `crates/holler-cli/src/output.rs:281-284`
  **Verbatim excerpt:**
  > Err(error) => {
  >     let written = write_line(sink.err, &format!("error: {}", error.message));
  >     settle(written, exit_code(&error))
  > }

- **Fact:** in JSON mode a result is one envelope on `out` (a failure's message put on one line), and nothing goes to
  `err`.
  **Source:** `crates/holler-cli/src/output.rs:288-300`
  **Verbatim excerpt:**
  > fn emit_json<T: Serialize>(sink: &mut Sink<'_>, result: Result<T, ErrorBody>) -> i32 {
  >     match result {
  >         Ok(data) => settle(write_envelope(sink, &Envelope::success(data)), 0),
  >         Err(error) => {
  >             let code = exit_code(&error);
  >             let error = ErrorBody {
  >                 message: one_line(&error.message),
  >                 ..error
  >             };
  >             settle(write_envelope(sink, &Envelope::<()>::failure(error)), code)
  >         }
  >     }
  > }

- **Fact:** the exit code is decided by `holler_pane::error::class_of` from the code alone, so it is the same in both
  formats.
  **Source:** `crates/holler-cli/src/output.rs:336-340`
  **Verbatim excerpt:**
  > /// The exit code of an error: the exit code of its code's class, decided by
  > /// `holler_pane::error::class_of` (1 runtime failure, 2 usage, 3 refusal).
  > fn exit_code(error: &ErrorBody) -> i32 {
  >     class_of(error.code.as_str()).exit_code()
  > }

- **Fact:** `usage` is its own class (exit 2); `pane-not-in-profile`, `profile-not-found` and `pane-not-found` are
  refusals (exit 3); `generation-conflict`, `timeout`, `unavailable`, `store-corrupt` and `not-implemented` are failures
  (exit 1).
  **Source:** `crates/holler-pane/src/error.rs:274-301`
  **Verbatim excerpt:**
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
  >     }

- **Fact:** a `PaneError::Usage` displays as its message alone, so the guard messages print exactly as built.
  **Source:** `crates/holler-pane/src/error.rs:640`
  **Verbatim excerpt:**
  >             PaneError::Usage { message } => f.write_str(message),

- **Fact:** the text of `generation-conflict`, which the failed-write message embeds after `<pane>: `.
  **Source:** `crates/holler-pane/src/error.rs:651-653`
  **Verbatim excerpt:**
  >             PaneError::Conflict => f.write_str(
  >                 "the record changed since it was read (generation conflict); read it again and retry",
  >             ),

- **Fact:** the text of `pane-not-found`, so a pane named alone with no record prints `pane not found: <name>`.
  **Source:** `crates/holler-pane/src/error.rs:674`
  **Verbatim excerpt:**
  >             PaneError::PaneNotFound { what } => write!(f, "pane not found: {what}"),

- **Fact:** the text of `unavailable` (AC 8b's standing pane-store fault answers the `get` before any write, so it is
  answered as it is).
  **Source:** `crates/holler-pane/src/error.rs:677`
  **Verbatim excerpt:**
  >             PaneError::Unavailable { what } => write!(f, "unavailable: {what}"),

- **Fact:** `findings::quoted`, which the text output uses for every stored reason, release condition and the profile
  name, is `error::excerpt`.
  **Source:** `crates/holler-pane/src/findings.rs:328-334`
  **Verbatim excerpt:**
  > /// `text` as one value in a message or a line of text output: `{:?}`-quoted, so every
  > /// control character and quote in it is escaped, and cut to 64 characters
  > /// (`error::excerpt`). The quoting of every untrusted value doctor prints, its text output
  > /// included.
  > pub fn quoted(text: &str) -> String {
  >     excerpt(text)
  > }

- **Fact:** `excerpt` `{:?}`-quotes the text and cuts it to 64 characters, so a stored reason of 65 to 200 characters
  prints cut in text mode (JSON carries all of it), and `"disk full"` prints as `"disk full"`.
  **Source:** `crates/holler-pane/src/error.rs:686-695`
  **Verbatim excerpt:**
  > /// `text` quoted for an error message, cut to 64 characters so an oversized input
  > /// cannot produce an oversized message.
  > pub(crate) fn excerpt(text: &str) -> String {
  >     const LIMIT: usize = 64;
  >     if text.chars().count() <= LIMIT {
  >         return format!("{text:?}");
  >     }
  >     let head: String = text.chars().take(LIMIT).collect();
  >     format!("{head:?}...")
  > }

- **Fact:** `Hold`'s serde form is snake_case, so the report's `hold` serializes as `"none"`,
  `{"parked": {"reason", "release_when", "since"}}` or `"drained"`; `since` is milliseconds since the Unix epoch.
  **Source:** `crates/holler-pane/src/pane.rs:165-177`
  **Verbatim excerpt:**
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(rename_all = "snake_case", deny_unknown_fields)]
  > pub enum Hold {
  >     None,
  >     Parked {
  >         reason: String,
  >         /// When the park ends, in the verb's own words (a time or a condition).
  >         release_when: String,
  >         /// When the pane was parked (milliseconds since the Unix epoch).
  >         since: i64,
  >     },
  >     Drained,
  > }

- **Fact:** `PaneStore::cas_put` returns the stored record with its bumped generation, which the report's
  `generation` and `hold` for a written pane are taken from.
  **Source:** `crates/holler-pane/src/ports.rs:69-71`
  **Verbatim excerpt:**
  >     /// Store `pane` if the stored one is still at `expected_generation` (0 for a
  >     /// new pane); returns the stored record with its bumped generation.
  >     fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;

- **Fact:** `ProfileScope::resolve` owns the membership rule: every pane of the profile with no pane name, or just the
  named one, which must belong (`pane-not-in-profile`); a missing profile is `profile-not-found`.
  **Source:** `crates/holler-pane/src/profile.rs:380-389`
  **Verbatim excerpt:**
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

- **Fact:** the fake scope the tests run over reads the profile first, then lists the members or gets the one named
  member, through the pane store it was built over (the test's wrapper in AC 8c).
  **Source:** `crates/holler-pane-testkit/src/profile_scope.rs:191-205`
  **Verbatim excerpt:**
  >     fn resolve(
  >         &self,
  >         profile: &ProfileName,
  >         pane: Option<&PaneName>,
  >     ) -> Result<ResolvedScope, PaneError> {
  >         let stored = self.stored(profile)?;
  >         let panes = match pane {
  >             None => self.members(&stored.name)?,
  >             Some(name) => vec![self.member(&stored.name, name)?],
  >         };
  >         Ok(ResolvedScope {
  >             profile: stored,
  >             panes,
  >         })
  >     }

- **Fact:** the fake scope already returns the members in name order, so the verb's own sort changes nothing over the
  fake; it is kept so the order does not depend on a scope implementation.
  **Source:** `crates/holler-pane-testkit/src/profile_scope.rs:105-115`
  **Verbatim excerpt:**
  >     /// Every pane whose record names `profile`, in name order.
  >     fn members(&self, profile: &ProfileName) -> Result<Vec<Pane>, PaneError> {
  >         let mut members: Vec<Pane> = self
  >             .panes
  >             .list()?
  >             .into_iter()
  >             .filter(|pane| belongs(pane, profile))
  >             .collect();
  >         members.sort_by(|x, y| x.name.cmp(&y.name));
  >         Ok(members)
  >     }

- **Fact:** the fake pane store seeds each record at generation 1, which is why a parked-by-this-run pane is at
  generation 2 and an unchanged one at 1.
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:90-99`
  **Verbatim excerpt:**
  >     /// A store holding `panes`, each created at expected generation 0 and so stored
  >     /// at 1, in order. Seeding bypasses the faults and the call log. Two seeds with one
  >     /// name are `generation-conflict`, as a second create would be.
  >     pub fn seeded(panes: impl IntoIterator<Item = Pane>) -> Result<Self, PaneError> {
  >         let store = Self::new();
  >         for pane in panes {
  >             store.put(&pane, Writer::Port(0))?;
  >         }
  >         Ok(store)
  >     }

- **Fact:** the fake's one write path stores the given record with only its generation replaced, and checks
  membership on a port write; park never changes `profile`, so that check cannot refuse it.
  **Source:** `crates/holler-pane-testkit/src/pane_store.rs:127-148`
  **Verbatim excerpt:**
  >     /// The one write path of a put (`cas_put`, `seeded`, `concurrent_put`): the
  >     /// stored record is `pane` with only its generation replaced.
  >     fn put(&self, pane: &Pane, writer: Writer) -> Result<Pane, PaneError> {
  >         self.feed.write(|log| {
  >             let stored = log.get(&pane.name);
  >             let current = stored.map_or(0, |stored| stored.generation);
  >             let generation = next_generation(current, writer.expected(current))?;
  >             if let Writer::Port(_) = writer {
  >                 check_membership(stored, pane)?;
  >             }
  >             let next = Pane {
  >                 generation,
  >                 ..pane.clone()
  >             };
  >             log.append(|cursor| PaneEvent {
  >                 cursor,
  >                 name: next.name.clone(),
  >                 pane: Some(Box::new(next.clone())),
  >             })?;
  >             Ok(next)
  >         })
  >     }

- **Fact:** `now_millis` is the Unix epoch in whole milliseconds (0 on a clock error); park takes it once per run.
  **Source:** `crates/holler-proto/src/clock.rs:33-40`
  **Verbatim excerpt:**
  > /// The current unix epoch in whole milliseconds. `0` on a clock error, same
  > /// discipline as [`now_secs`].
  > pub fn now_millis() -> i64 {
  >     std::time::SystemTime::now()
  >         .duration_since(std::time::UNIX_EPOCH)
  >         .map(|d| d.as_millis() as i64)
  >         .unwrap_or(0)
  > }

- **Fact:** the prompt hold's reason guard has the same 200-character cap and the opposite policy (cut, not refused);
  `MAX_TEXT_CHARS` in `park.rs` cites it rather than importing it (A warn 1).
  **Source:** `crates/holler-hub/src/holds.rs:66-69`
  **Verbatim excerpt:**
  > /// The longest reason kept, in characters. Longer text is cut (not refused):
  > /// the reason is a note for the next reader, and it is echoed in an error
  > /// message and on the roster.
  > pub const MAX_REASON_CHARS: usize = 200;

- **Fact:** ADR-0021 keeps the park hold apart from the prompt hold (the reason the cap constant is not shared).
  **Source:** `docs/adr/ADR-0021.md:44`
  **Verbatim excerpt:**
  > | `hold` | `Hold` | `"none"`, `{"parked": {reason, release_when, since}}` or `"drained"`. This is the park state of a pane, **not** the prompt hold of the hold and release verbs (`holler_proto::SessionHold`). |

- **Fact:** `in_scope` in `park.rs` copies the arms of reconcile's private `resolve` (A warn 4): a profile goes to
  `ProfileScope::resolve`, a pane named alone is one `get` with `pane-not-found` built the same way.
  **Source:** `crates/holler-pane/src/reconcile.rs:219-243`
  **Verbatim excerpt:**
  > /// The panes in scope, and every record (the stray rule judges against all of them). The
  > /// membership refusals are `ProfileScope::resolve`'s, and a named pane with no record is
  > /// `pane-not-found`.
  > fn resolve(
  >     ports: Ports<'_>,
  >     request: &ReconcileRequest<'_>,
  > ) -> Result<(Vec<Pane>, Vec<Pane>), PaneError> {
  >     let scope = match (request.profile, request.pane) {
  >         (Some(profile), pane) => ports.scope.resolve(profile, pane)?.panes,
  >         (None, Some(name)) => {
  >             let pane = ports
  >                 .pane_store
  >                 .get(name)?
  >                 .ok_or_else(|| PaneError::PaneNotFound {
  >                     what: name.to_string(),
  >                 })?;
  >             vec![pane]
  >         }
  >         (None, None) => {
  >             let records = ports.pane_store.list()?;
  >             return Ok((records.clone(), records));
  >         }
  >     };
  >     Ok((scope, ports.pane_store.list()?))
  > }

- **Fact:** a pane name is a session name, whose grammar allows only ASCII digits, lower-case letters and `-`, so the
  verb prints pane names bare (no quoting) in its lines and messages.
  **Source:** `crates/holler-proto/src/vocab.rs:209-223`
  **Verbatim excerpt:**
  >     if !is_word(seg[0]) || !is_word(seg[seg.len() - 1]) {
  >         return Err(NameError::BadCharacter);
  >     }
  >     for &c in seg {
  >         if !(is_word(c) || c == b'-') {
  >             return Err(NameError::BadCharacter);
  >         }
  >     }
  >     Ok(())
  > }
  >
  > #[inline]
  > fn is_word(b: u8) -> bool {
  >     matches!(b, b'0'..=b'9' | b'a'..=b'z')
  > }

- **Fact:** `PaneName::parse` is that grammar, refusing with `usage` in its own words (AC 7b compares against it).
  **Source:** `crates/holler-pane/src/pane.rs:35-42`
  **Verbatim excerpt:**
  >     /// Parse a pane name; the grammar is `SessionName::parse`'s.
  >     pub fn parse(text: &str) -> Result<Self, PaneError> {
  >         SessionName::parse(text)
  >             .map(Self)
  >             .map_err(|e| PaneError::Usage {
  >                 message: format!("invalid pane name {}: {e}", excerpt(text)),
  >             })
  >     }

- **Fact:** ADR-0021 section 8 asks for the reconcile step after a record-write conflict that follows a live change;
  park and unpark make no live change, so they print none (A warn 3; the ADR text is a follow-up, see the handoff).
  **Source:** `docs/adr/ADR-0021.md:284-286`
  **Verbatim excerpt:**
  > - A verb takes its expected generation when it plans, and writes the record with it after the act. If another writer got in
  >   between, the verb's record write fails with `generation-conflict` **after** the live change: the verb fails loudly, exits 1,
  >   writes nothing more, and prints the reconcile step (the pane doctor command line for that pane).

- **Fact:** ADR-0021 section 12 says a verb that times out prints the reconcile step; park and unpark do not (same
  reason as above).
  **Source:** `docs/adr/ADR-0021.md:471-472`
  **Verbatim excerpt:**
  > every step, and every port call is bounded by I5 (default 10 s) or ends in `timeout`. A verb that times out stops,
  > compensates as section 8 says, exits 1 with `timeout`, and prints the reconcile step. A crash between steps leaves state that

## T (Phase 7, verify GREEN)

- **Fact:** `ProfileScope::resolve` promises no order for the panes it answers (its doc names membership and the two
  refusals, nothing about order), so a scope wrapper that reverses them is a legal implementation. This is why the AC 4
  test's `ReversedScope` (`crates/holler-cli/tests/pane_verbs/park.rs`) pins the verb's own name-order sort, which the fake
  scope's own sort (entry above, `profile_scope.rs:105-115`) would otherwise hide. `edit_spec` is the trait's only other
  method, delegated unchanged.
  **Source:** `crates/holler-pane/src/profile.rs:380-404`
  **Verbatim excerpt:**
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
  ...
  >     fn edit_spec(
  >         &self,
  >         profile: Option<&ProfileName>,
  >         pane: &PaneName,
  >         edit: &SpecEdit,
  >         act: &mut dyn FnMut() -> Result<(), PaneError>,
  >     ) -> Result<Option<Profile>, PaneError>;
  > }
