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
