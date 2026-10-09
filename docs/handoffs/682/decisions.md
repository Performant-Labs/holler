# Decisions — #682 part 1: `FakeProfileStore` and the `ProfileStore` conformance suite

## A (Phase 3, up-front plan review) — 2026-10-09T12:46:10-06:00
- **Decided:** PASS on docs/handoffs/682-brief.md at f3e8781, with six warns and no block (see handoff-A.md). The plan
  extends slice a's shared fault switch, its generic feed (`Change`, `Log`, `Feed`), the case runner and the watch helpers
  instead of copying them. It moves `Writer` and `lock` into `feed.rs` so both fakes share one of each. No manifest,
  `lib.rs` or `conformance/mod.rs` changes, and the test kit still depends on `holler-pane` only. The new pieces (the
  per-slug change log, the clock, the name rule and `rename`'s refusal) have nothing existing to extend.
- **Assumed:** The brief's "Evidence", "Reuse refactors" and "Extend vs new" sections are the Reuse map, because this run
  has no separate survey.md. Issue #682's rule that no slice edits `conformance/mod.rs` is binding, which is what makes the
  sibling import in W-2 acceptable.
- **Hedged:** W-1: the generation a `Deleted` log entry carries is ambiguous between ADR-0021:268-269 and
  `holler-pane/src/profile.rs:298` (0 or g + 1). The brief chose g + 1 and documented why. I made it a warn, not a block,
  because the ADR does not decide it. O settles it before T writes cases 11 and 14 and AC4. W-6: the script's PR body
  `Closes #682.` would close the issue before part 2 is built, and the "Needs operator" section the brief points to is
  missing.
- **Evidence:** Read the brief in full and issues #682 and #661. Read every slice-a file in `crates/holler-pane-testkit/`
  (src and tests); `holler-pane/src/profile.rs`, `generation.rs`, `argv.rs:90-110` and `error.rs:400-470`; ADR-0021 §1,
  §2, §5, §7, §8 and "Decisions taken"; and `holler-hub/src/panes/mod.rs`, `panes/feed.rs` and `profile/mod.rs`. Checked
  the workspace's `foo.rs` + `foo/` layout precedent, `wc -l` on every touched file, that no open PR touches the test kit,
  and the PR-body code in `$WORKFLOW_ROOT/workflow/coding-pipeline.workflow.mjs` (lines 4097 and 4822).
