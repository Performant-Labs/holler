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
