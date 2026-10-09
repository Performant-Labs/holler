# Decision journal: #662 profile-verbs (run 662a)

## A (up-front plan review) — 2026-10-09T17:19-06:00
- **Decided:** BLOCK, on one block finding (handoff-A.md).
  - **The block.** The tests of `profile_snapshot` and `profile_diff` (AC 1 and AC 2) belong in `crates/holler-pane/tests/`,
    over `tests/common`, not in `tests/profile_verbs/show.rs`.
    - That is the uniform pattern: nine test files, plus docs/testing.md:45.
    - Decision 11's premise ("holler-pane cannot take the test kit") does not apply to pure functions over records, which
      need no fake.
  - **The production plan passes.** Placement per ADR-0021 section 5, layering, the reuse of output, `class_of`, the closed
    codes, the names and the serde forms, the contract shape and the ADR-0021 edits all fit.
  - **Six warns:**
    - `is_member` takes `&ProfileName`, not a bare slug;
    - Forward-compat names #644, a hard consumer since `7195993`, plus #663 and #643;
    - `harness.kind` and `role` text comes from serde, not literals;
    - a follow-up to compare `port_policy`, and drop the non-existent `auto`;
    - the rig goes in its own file through `#[path]`;
    - the text forms are settled with #643's W-2.
  - **Told O:** do not change the three `profile_snapshot` signatures, which #644 now pins verbatim.
- **Assumed:**
  - The sibling plans, as read from their worktrees, are what those runs will implement: #643 `fafd138` (A PASS), #644
    `7195993` (amended after A BLOCK), #647 `a98258a`, #663 `ec3a214`. None has merged code.
  - T's file budget for 662a has room for one or two holler-pane test files; one combined file is offered if not.
- **Hedged:**
  - The block concerns test structure, not production code. I raised it as a block, not a warn, for three reasons:
    - Decision 11 and the Files list pin the placement, and in the automated path a warn has no lever over them.
    - The A-dup gate checks F's code, not where tests live.
    - The fix is cheapest before T writes RED.
  - Warns 3, 6 and 7 depend on sibling runs whose merge order is the MO's call. Their line numbers moved while I read them:
    #663 and #644 were amended during this review.
- **Evidence:**
  - **This brief:** the whole of `docs/handoffs/662-brief.md` (1,994 lines) and the outside model's round 1
    (`662-brief-result-r1.md`, PASS).
  - **ADR-0021:** read in full.
  - **holler-pane:** `src/{lib,profile,pane,argv,grid,probe,ports,error}.rs`, `tests/common/mod.rs`, every
    `tests/*_test.rs` header and the `Cargo.toml`.
  - **holler-cli:**
    - `src/pane/{args,profile_scope}.rs`, `src/profile/{mod,list,show}.rs` and `src/output.rs`;
    - `tests/verb_harness/mod.rs`, `tests/{pane_verbs,profile_verbs}/main.rs`, `tests/pane_verbs/process/stub.rs`;
    - the manifest's test targets.
  - **The test kit:** `lib.rs`, `profile_scope.rs` (`belongs`), `pane_store.rs` and `profile_store.rs`.
  - **The hub:** `holler-hub/src/profile/entry.rs` (`summary`, which is not analogous).
  - **holler-proto:** `src/log.rs` (`escape_field_value`).
  - **Project docs and scripts:** `docs/testing.md` and `scripts/lint.sh`.
  - **Greps:** slug comparisons, `port_policy`, `auto`, the escape helpers, and the sibling briefs for `is_member`,
    `spec_from_pane`, the rigs and `fixed:`.
  - **Siblings:** #644's and #643's `handoff-A.md`.
