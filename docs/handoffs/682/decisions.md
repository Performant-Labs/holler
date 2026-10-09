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

## T-red (Phase 4, author the RED) — 2026-10-09T13:20:00-06:00
- **Decided:** Authored `tests/profile_store_conformance_test.rs` (AC1, AC2: suite pass, 23 case ids, unbroken wrapper, six
  mutants) and `tests/fake_profile_store_test.rs` (AC3, AC4, plus one fixture-agreement test for A's W-5). RED is valid:
  both targets fail to build on the six missing public items only. Dropped a copy of the pane suite's guard-lifetime test
  because the runner is shared with the pane suite, which already pins it.
- **Assumed:** The brief's g + 1 for a `Deleted` entry's generation stands, since O did not re-settle A's W-1 before this
  phase. The `what` of `profile-not-found` is the display name. Seeding one name twice is `generation-conflict`.
- **Hedged:** A compile error is a weaker RED than a failing assertion, but it is the shape the brief's test plan names
  for a story whose whole API is new. To make sure a typo of mine would not surface only in F's phase, compiled both files
  against a throwaway stub and reverted it; nothing of it is staged. `fake_profile_store_test.rs` is 593 lines, 7 under
  the lint warn threshold.
- **Evidence:** `cargo test -p holler-pane-testkit --no-run` (E0432 only); stubbed build clean; slice-a tests 22 + 10 pass;
  `rustfmt --edition 2021 --check tests/*.rs` clean.

## F (Phase 6, implement) — 2026-10-09T13:15:47-06:00
- **Decided:**
  - Implemented part 1 against T's RED, in the brief's order: first the reuse refactors, with slice a green after
    them (22 + 10), then the fixtures, the fake, the suite and the CHANGELOG.
  - `Writer` and `lock` live in `feed.rs`, shared by both fakes.
  - Five pane-suite helpers are `pub(super)`, and four of them are generic over `Change`.
  - `sample_pane` and `sample_spec` share their model, context, grid, cwd and workspace (A's W-5).
  - The fake's per-slug log field is `change_logs`, with the documented lock order "feed first, never the reverse"
    (A's W-4). The event is published before the log entry, so a refused or unpublishable write logs nothing.
  - W-1 stays at the brief's g + 1. Both log rules and their reasons are in the suite's module doc (A's W-1 fix).
  - Applied the brief's Risks mitigation: cases 15 to 18 moved to `conformance/profile_store/log.rs`, because the
    one-file suite reached 640 lines. The files are now 521, 129 and 144.
- **Assumed:** The Risks section's named `profile_store/log.rs` overrides AC8's literal file list, because the brief
  prescribes that exact file for this exact condition. The changed failure-detail text of the generic `expect_change`
  (`PaneName("x")` where it was `x`) is behaviour-neutral, because no test pins detail text.
- **Hedged:**
  - AC10's clippy is red because of T's test `fake_profile_store_test.rs:559` (cognitive complexity 16/15). It is
    test-only. F did not edit it and flagged it for T with fix options; CI runs that exact clippy command
    (`ci.yml:280`).
  - The `log` module's name fits cases 15 and 16. Cases 17 and 18 went with them because the brief says
    "15 to 18".
  - `archChanged` is reported true: new module boundaries (`watch.rs`, `log.rs`) and new public items, all planned
    except `log.rs`.
- **Evidence:**
  - Tests: `cargo test -p holler-pane-testkit` (22 + 22 + 10 + 9 pass) and `cargo test --workspace` (exit 0,
    1140 passed, 0 failed).
  - Clippy: clean everywhere except T's one function (`--keep-going` shows a single error, and the run with
    `--exclude holler-pane-testkit` is clean).
  - Other guards: rustfmt clean, `cargo doc -D warnings` clean, `cargo machete` clean, `lint.sh` exit 0,
    `test-hooks.sh` exit 0, `changelog-check` ok.
  - AC5 to AC8: the `git diff --quiet` and `cargo tree` checks and AC7's greps, as listed in handoff-F.md.
  - `docs/handoffs/682/evidence.md` (11 verbatim entries).

## T (Phase 7, GREEN + Tier 2)
- **Decided:** repaired the one flagged test by splitting it in two (same assertions) rather than adding an `#[allow]`; a
  split keeps the lint guard useful and the file under the 600-line warn.
- **Assumed:** the `holler-cli` `body_run_test` failure is a pre-existing port flake, because the diff touches no
  `holler-cli` file and the test passed alone and in a full run after failing in earlier runs.
- **Hedged:** ran the testkit suite 15 times and ran one manual mutation of the generation check to confirm the tests
  fail when behavior is removed.
- **Evidence:** `cargo test --workspace --no-fail-fast` exit 0, 1141 passed, 0 failed; clippy `-D warnings` clean.
