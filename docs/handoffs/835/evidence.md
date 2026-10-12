# Evidence: #835

Facts in code the diff does not change, which the change or the tests rely on. Line numbers are at the F worktree
(`issue-835-implementation`, uncommitted F changes on top of `36b0de8`).

## F (Phase 5/6 of the script's numbering), 2026-10-11

- **Fact:** A pane with any `problem` finding gets no record and the row's action `blocked`, so a new `problem` kind
  (`harness-unreadable`) blocks a pane with no further code.
  **Source:** `crates/holler-pane/src/import/assess.rs:163-174`
  **Verbatim excerpt:**
  > ```rust
  >     let blocked = findings.iter().any(|f| f.level() == Level::Problem);
  >     let seen = Seen { health, session };
  >     let record = herdr
  >         .filter(|_| !blocked)
  >         .and_then(|herdr| record(context, files, herdr, seen, role, command));
  >     let row = PaneRow {
  >         name: name.clone(),
  >         action: if record.is_some() {
  >             Action::Record
  >         } else {
  >             Action::Blocked
  >         },
  > ```

- **Fact:** A real run with any blocked pane is refused as `import-blocked` before any write.
  **Source:** `crates/holler-pane/src/import.rs:144-148` (`Plan::refusal`, `:159-167`, refuses when it is above 0) and
  `crates/holler-cli/src/pane/import/writes.rs:38-42`
  **Verbatim excerpt:**
  > ```rust
  >     pub fn blocked(&self) -> usize {
  >         let panes = self
  >             .panes
  >             .iter()
  >             .filter(|row| row.action == Action::Blocked)
  >             .count();
  > ```
  > ```rust
  > pub(super) fn write(plan: &Plan, ports: Ports<'_>) -> Result<Option<Written>, ErrorBody> {
  >     let body = |error: PaneError| ErrorBody::from(&error);
  >     if let Some(refusal) = plan.refusal() {
  >         return Err(body(refusal));
  >     }
  > ```

- **Fact:** The import skips a pane that already has a record before it observes anything, so every pane the chain
  observes has no record. This is why the registry rule of `holler_pane::unmanaged` cannot tell a Claude Code pane from
  an OpenCode one (ADR-0021 section 13, "Not the registry's rule"; A's W3).
  **Source:** `crates/holler-pane/src/import.rs:181-189`
  **Verbatim excerpt:**
  > ```rust
  >     for files in panes {
  >         if ports.pane_store.get(&files.name)?.is_some() {
  >             outcomes.push(Outcome::already_imported(&files.name));
  >         } else if files.sessions.not_opencode() {
  >             outcomes.push(Outcome::not_opencode(&files.name));
  >         } else {
  >             candidates.push(files);
  >         }
  >     }
  > ```

- **Fact:** The tmux adapter maps a refused call whose stderr says the session is missing to `pane-not-found` and any
  other refusal to `unavailable`. `foreground_commands` reuses this mapping unchanged, as `ps` does.
  **Source:** `crates/holler-adapter-host/src/lib.rs:463-471`
  **Verbatim excerpt:**
  > ```rust
  > fn refused(name: &PaneName, refusal: Refusal, what: String) -> PaneError {
  >     if refusal.is_missing() {
  >         PaneError::PaneNotFound {
  >             what: name.to_string(),
  >         }
  >     } else {
  >         PaneError::Unavailable { what }
  >     }
  > }
  > ```

- **Fact:** The CLI's one wording for a story that has not landed, which the `--adopt-claude-code` refusal reuses.
  **Source:** `crates/holler-cli/src/output.rs:253-255`
  **Verbatim excerpt:**
  > ```rust
  > pub fn not_implemented_message(story: u32) -> String {
  >     format!("not implemented (story #{story})")
  > }
  > ```

- **Fact:** A `usage` error exits 2 in both formats, so the switch's refusal (a `PaneError::Usage` from `Input::read`) is
  exit 2.
  **Source:** `crates/holler-pane/src/error.rs:239-245` and `:274-275`
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
  > ```

## T (Phase 7, GREEN), 2026-10-11

- **Fact:** `pane import`'s JSON `data.profile` is the serde form of `Profile`, and each spec's pane is under the key
  `pane` (no rename), which `pane_verbs/import/harness.rs`'s `spec_names()` reads.
  **Source:** `crates/holler-cli/src/pane/import.rs:128-137` and `crates/holler-pane/src/profile.rs:178-181`
  **Verbatim excerpt:**
  > ```rust
  > /// The data of `pane import`. Its JSON keys are its fields, in this order.
  > #[derive(Debug, Serialize)]
  > struct Imported {
  > ```
  > ```rust
  >     /// The profile a dry run would create, or the one a real run stored.
  >     profile: Option<Profile>,
  > ```
  > ```rust
  > #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
  > #[serde(deny_unknown_fields)]
  > pub struct ProfileSpec {
  >     pub pane: String,
  > ```

## F, cycle 2 (the guide and the records after the merge with `main`), 2026-10-11

Line numbers are at `526bbd3` (the branch with `origin/main` `bda1069` merged in), unchanged by this cycle's text edits.

- **Fact:** In the import, `observe-failed` is raised only for a failed call to the pane's harness server: an error from
  its health check, or from the session list of a server that passed it. The guide's `observe-failed` row and
  `ImportFinding::ObserveFailed`'s doc are narrowed to this.
  **Source:** `crates/holler-pane/src/import/observe.rs:177-187`
  **Verbatim excerpt:**
  > ```rust
  >     pub(super) fn finding(&self, pane: &PaneName, port: u16) -> Option<Finding> {
  >         let (kind, message) = match self {
  >             Server::Healthy(Ok(_)) => return None,
  >             Server::Healthy(Err(error)) => (
  >                 ImportFinding::ObserveFailed,
  >                 failed_message(HARNESS_LIST_SESSIONS, error),
  >             ),
  >             Server::Unknown(error) => (
  >                 ImportFinding::ObserveFailed,
  >                 failed_message(HARNESS_HEALTH, error),
  >             ),
  > ```

- **Fact:** Every `usage` refusal of `pane import` comes from `Input::read`, before the launcher table and the pane files
  are read, and `plan` (the first port call) runs only after it, so a dry run gets the same `usage`. The guide's
  exit-code paragraph lists these four causes (the `--adopt-claude-code` check, first in `Input::read`, is in the diff).
  **Source:** `crates/holler-cli/src/pane/import.rs:145-146` and `:182-207`
  **Verbatim excerpt:**
  > ```rust
  >     let input = Input::read(args).map_err(body)?;
  >     let planned = plan(ports, &input.request()).map_err(body)?;
  > ```
  > ```rust
  >         let herdr_session = args
  >             .herdr_session
  >             .clone()
  >             .filter(|session| !session.trim().is_empty())
  >             .ok_or_else(|| {
  >                 usage("--herdr-session is required: name the Herdr session the panes live in")
  >             })?;
  >         let root = Path::new(&args.from);
  >         if !root.is_dir() {
  >             return Err(usage(&format!(
  >                 "--from {:?} is not a directory: name the directory that holds one directory \
  >                  per pane",
  >                 args.from
  >             )));
  >         }
  >         let listing = sources::list(root)?;
  >         let orchestrators = orchestrators(&args.orchestrator, &listing, &args.from)?;
  >         let launcher_path = args
  >             .launcher
  >             .as_ref()
  >             .map_or_else(|| root.join(LAUNCHER_FILE), PathBuf::from);
  >         let table = LauncherTable::read(&launcher_path);
  >         let panes = listing
  >             .panes
  >             .iter()
  >             .map(|(name, dir)| sources::read(name, dir, &table))
  > ```

- **Fact:** A whole-fleet reconcile pass reports every snapshot pane that no record names as `unregistered-herdr-pane`,
  through `unmanaged::unregistered`. A pane the import leaves alone gets no record, so `pane doctor` reports it, as the
  guide's `harness-claude-code` row and the corrected docs of `unregistered` say.
  **Source:** `crates/holler-pane/src/reconcile.rs:362-372`
  **Verbatim excerpt:**
  > ```rust
  > /// `unregistered-herdr-pane` for each pane of the snapshot that no record names.
  > fn unregistered_herdr_panes(snapshot: &HerdrSnapshot, records: &[Pane]) -> Vec<Finding> {
  >     unmanaged::unregistered(snapshot, records)
  >         .map(|herdr| {
  >             let message = format!(
  >                 "Herdr pane {} (session {}, workspace {}) is in no pane record; no holler verb adopts a pane",
  >                 quoted(herdr.pane_id.as_str()),
  >                 quoted(&herdr.session),
  >                 quoted(&herdr.workspace)
  >             );
  >             Finding::new(FindingKind::UnregisteredHerdrPane, message).with_herdr_pane(herdr)
  > ```
