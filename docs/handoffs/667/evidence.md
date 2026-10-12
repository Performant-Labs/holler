# Evidence: #667 (source facts the diff relies on but does not show)

## F (Phase 6), 2026-10-11

- **Fact:** CI's CHANGELOG guard requires only that an `## [Unreleased]` heading exists. F's entry goes under that heading, which stays where it was.
  **Source:** `scripts/changelog-check.sh:15-18`
  **Verbatim excerpt:**
  > if ! grep -qE '^## \[Unreleased\]' "$file"; then
  >   echo "changelog-check: $file has no '## [Unreleased]' heading"
  >   exit 1
  > fi

- **Fact:** The guide's docs test needs one line in `[Unreleased]` that links `/issues/652` and names `pane-control.md`. F's new entry goes above the #652 entry, in the same `[Unreleased]` section, so that line stays inside the text the test reads.
  **Source:** `crates/holler-cli/tests/docs_pane_control_test.rs:478-487`
  **Verbatim excerpt:**
  > /// Acceptance 16: an `## [Unreleased]` CHANGELOG line links #652 and names the guide.
  > #[test]
  > fn changelog_unreleased_names_the_guide() {
  >     let changelog = read("CHANGELOG.md");
  >     let unreleased = from_heading(&changelog, "## [Unreleased]");
  >     let unreleased = unreleased.split("\n## [").next().unwrap_or_default();
  >     assert!(
  >         unreleased.lines().any(|l| l.contains("/issues/652") && l.contains("pane-control.md")),
  >         "CHANGELOG.md's [Unreleased] has no line linking /issues/652 and naming pane-control.md"
  >     );

## T (T-green), 2026-10-11

These are the unchanged production facts that the opted-in scenario asserts literally. Each excerpt was copied from source at `e22ce66`.

- **Fact:** Step 4's dry-run detail for `B1` is the unhealthy reason `judge` gives when the harness server does not answer. `scenario.rs:401` asserts that it contains this text.
  **Source:** `crates/holler-pane/src/tx_apply/observe.rs:121-126`
  **Verbatim excerpt:**
  > if !answers {
  >     let port = record.harness.port;
  >     return Ok(Judged::Unhealthy(format!(
  >         "its harness server on port {port} did not answer"
  >     )));
  > }

- **Fact:** `profile show` in text prints a `Missing` row as `missing (no live pane)`. Step 1's control counts that string twice for `Q` (`scenario.rs:174`).
  **Source:** `crates/holler-cli/src/profile/show.rs:175-181`
  **Verbatim excerpt:**
  > fn status_text(status: PaneStatus) -> &'static str {
  >     match status {
  >         PaneStatus::Matches => "matches",
  >         PaneStatus::Differs => "differs",
  >         PaneStatus::Missing => "missing (no live pane)",
  >         PaneStatus::Extra => "extra (no spec)",
  >     }

- **Fact:** The text header of a dry run is `profile NAME: N steps, planned (dry run; nothing changed)`. `assert_text_plan` (`scenario.rs:264-267`) asserts the first line equals it.
  **Source:** `crates/holler-cli/src/profile/apply.rs:136-146`
  **Verbatim excerpt:**
  > fn render(applied: &Applied) -> String {
  >     let done = if applied.dry_run {
  >         "planned (dry run; nothing changed)"
  >     } else {
  >         "applied"
  >     };
  >     let mut lines = vec![format!(
  >         "profile {}: {}, {done}",
  >         applied.profile,
  >         count(applied.steps.len(), "step")

- **Fact:** Steps are in `(workspace, row, col)` order. The scenario asserts `[A1, A2, B1, X]` (`hlr649` r1, r2, then `hlr649b` r1, r2) and test B asserts `[W, Z, Y]` (`hlr649g` r1, then `hlr649h` r1, r2).
  **Source:** `crates/holler-pane/src/tx_apply.rs:343-351`
  **Verbatim excerpt:**
  > chosen.sort_by(|(a, _), (b, _)| cell_key(a).cmp(&cell_key(b)));
  > chosen.into_iter().map(|(_, planned)| planned).collect()
  > }
  >
  > /// A spec's place in the run: its workspace, then its row and column.
  > fn cell_key(spec: &ProfileSpec) -> (&str, u16, u16) {
  >     let grid = spec.herdr.grid;
  >     (spec.herdr.workspace.as_str(), grid.row, grid.col)

- **Fact:** A plan with a create and no `--herdr-session` is `usage`. A plan with no create needs no flag. So step 2's dry run without the flag exits 2 `usage`, and step 4's live apply, which has no create, runs without it.
  **Source:** `crates/holler-pane/src/tx_apply.rs:272-283`
  **Verbatim excerpt:**
  > .filter(|spec| !records.iter().any(|r| r.name.as_str() == spec.pane))
  > .map(|spec| quoted(&spec.pane))
  > .collect();
  > if creates.is_empty() {
  >     return Ok(());
  > }
  > Err(PaneError::Usage {
  >     message: format!(
  >         "--herdr-session is required: apply creates the spec panes that have no record ({}), \
  >          and the flag names the Herdr session they go in",

- **Fact:** A take-over is two compare-and-swap writes, the leave and then the join. Step 5 asserts that `A1`'s generation goes up by exactly 2 (`scenario.rs:485-489`).
  **Source:** `crates/holler-pane/src/tx_apply/run.rs:219-225` and `:230-239`
  **Verbatim excerpt:**
  > fn leave(pane_store: &dyn PaneStore, record: &Pane) -> Result<Pane, PaneError> {
  >     let left = Pane {
  >         profile: None,
  >         ..record.clone()
  >     };
  >     pane_store.cas_put(&left, record.generation)
  
  > fn join(
  >     pane_store: &dyn PaneStore,
  >     record: &Pane,
  >     profile: &ProfileName,
  > ) -> Result<Pane, PaneError> {
  >     let member = Pane {
  >         profile: Some(profile.clone()),
  >         ..record.clone()
  >     };
  >     pane_store.cas_put(&member, record.generation)
