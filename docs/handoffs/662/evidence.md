# Evidence: #662b profile write verbs (`holler profile create`, `holler profile delete`)

Facts in **unchanged** code that F's diff (`crates/holler-cli/src/profile/{create,delete}.rs`) or T's tests rely on.
Line numbers are the worktree's, branch `issue-662-implementation` (merge base `ce12cdb`).

## F (Phase 5)

- **Fact:** `profile_from_panes` builds a new record with the name's slug, generation 0, both stamps 0, and one
  `spec_from_pane` per pane, in the order given; every `create` form builds its record with it (`&[]` for an empty
  profile and for `--from`).
  **Source:** `crates/holler-pane/src/profile_snapshot.rs:72-81`
  **Verbatim excerpt:**
  > ```rust
  > pub fn profile_from_panes(name: &ProfileName, panes: &[Pane]) -> Profile {
  >     Profile {
  >         name: name.clone(),
  >         slug: name.slug(),
  >         generation: 0,
  >         panes: panes.iter().map(spec_from_pane).collect(),
  >         created: 0,
  >         updated: 0,
  >     }
  > }
  > ```

- **Fact:** membership is decided by slug on both sides, so a pane whose `profile` is `SOME-PROFILE` is a member of
  `Some Profile`. `create`'s plan check (a pane is in another profile when it has a profile and is not a member),
  `create`'s undo re-read and `delete`'s member list all use this one function.
  **Source:** `crates/holler-pane/src/profile_diff.rs:261-265`
  **Verbatim excerpt:**
  > ```rust
  > pub fn is_member(pane: &Pane, profile: &ProfileName) -> bool {
  >     pane.profile
  >         .as_ref()
  >         .is_some_and(|own| own.slug() == profile.slug())
  > }
  > ```

- **Fact:** `count` writes the noun in the plural unless `n` is 1 (`0 specs`, `1 live pane`, `2 live panes`).
  **Source:** `crates/holler-cli/src/profile/list.rs:93-99`
  **Verbatim excerpt:**
  > ```rust
  > pub(crate) fn count(n: usize, noun: &str) -> String {
  >     if n == 1 {
  >         format!("1 {noun}")
  >     } else {
  >         format!("{n} {noun}s")
  >     }
  > }
  > ```

- **Fact:** `Ports` is `Copy`, so each verb's `run` copies `ctx.ports` once and uses it for both the plan and the
  writes.
  **Source:** `crates/holler-pane/src/ports.rs:226-227`
  **Verbatim excerpt:**
  > ```rust
  > #[derive(Clone, Copy)]
  > pub struct Ports<'a> {
  > ```

- **Fact:** an `ErrorBody` can be built with one error's code and the verb's own message: both fields are public,
  and `ErrorCode::from(&PaneError)` is infallible. The verbs do this where a message keeps the store's code (a failed
  join whose undo completed; a failed detach).
  **Source:** `crates/holler-cli/src/output.rs:118-132`
  **Verbatim excerpt:**
  > ```rust
  > impl From<&PaneError> for ErrorCode {
  >     /// A `PaneError` always has a valid code, so this cannot fail.
  >     fn from(error: &PaneError) -> Self {
  >         Self(error.code().to_owned())
  >     }
  > }
  >
  > /// The `error` member of a failed envelope.
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
  > pub struct ErrorBody {
  >     /// The stable code a script matches on.
  >     pub code: ErrorCode,
  >     /// One line for a person. In JSON mode [`emit`] puts it on one line.
  >     pub message: String,
  > }
  > ```

- **Fact:** `emit` takes the exit code from the class of the error's code, in both formats; JSON mode also puts the
  message on one line.
  **Source:** `crates/holler-cli/src/output.rs:288-300` and `crates/holler-cli/src/output.rs:336-340`
  **Verbatim excerpt:**
  > ```rust
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
  > ```
  > ```rust
  > /// The exit code of an error: the exit code of its code's class, decided by
  > /// `holler_pane::error::class_of` (1 runtime failure, 2 usage, 3 refusal).
  > fn exit_code(error: &ErrorBody) -> i32 {
  >     class_of(error.code.as_str()).exit_code()
  > }
  > ```

- **Fact:** `profile-exists`, `profile-has-live-panes`, `pane-in-other-profile` and `profile-not-found` are refusals
  (exit 3); `generation-conflict`, `profile-conflict`, `timeout` and `unavailable` are failures (exit 1). A join that
  fails with `pane-in-other-profile` and is undone therefore exits 3, and one that fails with `generation-conflict`
  exits 1.
  **Source:** `crates/holler-pane/src/error.rs:239-245` and `crates/holler-pane/src/error.rs:274-302`
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
  >     }
  > ```

- **Fact:** the messages the verbs build start with these `Display` texts (the `what` the verb supplies follows the
  prefix; `<E>` in a message is the whole text of the store's error).
  **Source:** `crates/holler-pane/src/error.rs:651-666`
  **Verbatim excerpt:**
  > ```rust
  >             PaneError::Conflict => f.write_str(
  >                 "the record changed since it was read (generation conflict); read it again and retry",
  >             ),
  >             PaneError::ProbeFailed { message } => write!(f, "health probe failed: {message}"),
  >             PaneError::ProfileConflict { what } => write!(f, "profile conflict: {what}"),
  >             PaneError::ProfileNotFound { what } => write!(f, "profile not found: {what}"),
  >             PaneError::ProfileExists { what } => write!(f, "profile already exists: {what}"),
  >             PaneError::ProfileHasLivePanes { what } => {
  >                 write!(f, "profile has live panes: {what}")
  >             }
  >             PaneError::PaneNotInProfile { what } => {
  >                 write!(f, "pane is not in the profile: {what}")
  >             }
  >             PaneError::PaneInOtherProfile { what } => {
  >                 write!(f, "pane belongs to another profile: {what}")
  >             }
  > ```

- **Fact:** `ProfileName::parse` trims its input and refuses an empty name as `usage`, so a blank NAME or a blank
  `--from` is exit 2 from the verb, not from clap.
  **Source:** `crates/holler-pane/src/profile.rs:42-46` and `crates/holler-pane/src/profile.rs:55-59`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn parse(text: &str) -> Result<Self, PaneError> {
  >         let name = text.trim();
  >         let refusal = if name.is_empty() {
  >             Some("a profile name must not be empty")
  >         } else if name.chars().count() > MAX_NAME_CHARS {
  > ```
  > ```rust
  >         match refusal {
  >             Some(rule) => Err(PaneError::Usage {
  >                 message: format!("invalid profile name {}: {rule}", excerpt(text)),
  >             }),
  >             None => Ok(Self(name.to_owned())),
  > ```

- **Fact:** a pane name is lowercase ASCII letters, digits and `-`, with a letter or digit first and last (the
  `SessionName` grammar `PaneName` reuses), so `pane_list` prints pane names unquoted in messages and in suggested
  commands.
  **Source:** `crates/holler-proto/src/vocab.rs:209-223`
  **Verbatim excerpt:**
  > ```rust
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
  > ```

- **Fact:** on the real store, a create at generation 0 of a name already stored **under the same display name**
  passes the name rule and is `generation-conflict` (`next_generation(1, 0)`). That is the create race that
  `insert_profile` reports as `profile-exists`. The name rule runs first, so another spelling of a stored slug is the
  store's own `profile-exists`, which passes through.
  **Source:** `crates/holler-hub/src/profile/store.rs:202-206` and `crates/holler-pane/src/generation.rs:25-28`
  **Verbatim excerpt:**
  > ```rust
  >         let slug = profile.name.slug();
  >         let previous = table.entries.get(&slug);
  >         let stored = previous.and_then(ProfileEntry::profile);
  >         check_name(stored, &profile.name, &slug)?;
  >         let generation = next_generation(stored.map_or(0, |stored| stored.generation), expected)?;
  > ```
  > ```rust
  > pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
  >     if current != expected {
  >         return Err(PaneError::Conflict);
  >     }
  > ```

- **Fact:** the store's name rule: a write whose slug is filed under another display name is `profile-exists`,
  whatever the generation.
  **Source:** `crates/holler-hub/src/profile/store.rs:394-404`
  **Verbatim excerpt:**
  > ```rust
  > fn check_name(stored: Option<&Profile>, name: &ProfileName, slug: &str) -> Result<(), PaneError> {
  >     match stored {
  >         Some(stored) if stored.name != *name => Err(PaneError::ProfileExists {
  >             what: format!(
  >                 "{:?} has the slug {slug:?} of the stored profile {:?}",
  >                 name.as_str(),
  >                 stored.name.as_str()
  >             ),
  >         }),
  >         _ => Ok(()),
  >     }
  > ```

- **Fact:** the hub refuses a `pane/cas_put` that names a profile that does not exist (`profile-not-found`), before
  the compare-and-swap. That is why `--from-current` writes the profile before any pane joins it.
  **Source:** `crates/holler-hub/src/panes/handlers.rs:110-113` and `crates/holler-hub/src/profile/mod.rs:206-216`
  **Verbatim excerpt:**
  > ```rust
  >     run(cid, obj, move |params: PaneCasPutParams| {
  >         check_membership(&params.pane, &profiles)?;
  >         store.cas_put(&params.pane, params.expected_generation)
  >     })
  > ```
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

- **Fact:** the pane registry refuses only a move between two profiles with different slugs. Leaving (`profile`
  none, which `detach` writes for `delete --keep-panes` and for `create`'s undo), joining from none, and keeping
  another spelling of the same profile (a pane that already names NAME's slug) are allowed.
  **Source:** `crates/holler-hub/src/panes/store.rs:344-357`
  **Verbatim excerpt:**
  > ```rust
  > fn refuse_profile_move(stored: Option<&Pane>, next: &Pane) -> Result<(), PaneError> {
  >     let current = stored.and_then(|pane| pane.profile.as_ref());
  >     match (current, next.profile.as_ref()) {
  >         (Some(current), Some(other)) if current.slug() != other.slug() => {
  >             Err(PaneError::PaneInOtherProfile {
  >                 what: format!(
  >                     "{} is in profile {:?}, not {:?}",
  >                     next.name,
  >                     current.as_str(),
  >                     other.as_str()
  >                 ),
  >             })
  >         }
  >         _ => Ok(()),
  > ```

- **Fact:** the real store's profile delete answers `profile-not-found` for a missing profile before it looks at the
  generation, and `generation-conflict` for a stale one. `delete` maps the latter to `profile-conflict` only when a
  member was already detached.
  **Source:** `crates/holler-hub/src/profile/store.rs:241-248`
  **Verbatim excerpt:**
  > ```rust
  >         let slug = name.slug();
  >         let previous = table.entries.get(&slug);
  >         let Some(stored) = previous.and_then(ProfileEntry::profile) else {
  >             return Err(PaneError::ProfileNotFound {
  >                 what: name.to_string(),
  >             });
  >         };
  >         let generation = next_generation(stored.generation, expected)?;
  > ```

- **Fact:** both the real store and the test kit's fake find a profile by the slug of the name asked for. So
  `create Beta --from ALPHA` finds `Alpha`, and `create some-profile` finds the stored `Some Profile` (whose stored
  name the `profile-exists` message then uses).
  **Source:** `crates/holler-hub/src/profile/store.rs:164-166` and `crates/holler-pane-testkit/src/profile_store.rs:250-253`
  **Verbatim excerpt:**
  > ```rust
  >     pub(crate) fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
  >         self.read(|table| table.record(name).cloned())
  >     }
  > ```
  > ```rust
  >     fn get(&self, name: &ProfileName) -> Result<Option<Profile>, PaneError> {
  >         self.faults.enter(ProfileStoreOp::Get)?;
  >         Ok(self.feed.read(|log| log.get(&name.slug()).cloned()))
  >     }
  > ```
