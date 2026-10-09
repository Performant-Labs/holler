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

## A (up-front plan review, round 2, fresh run on the amended brief `31062ce`) — 2026-10-09T17:56-06:00
- **Decided:** PASS, with five warns and no block (handoff-A.md, which replaces round 1's; that one is in git at `ca42de7`).
  - **Round 1's block is fixed.** AC 1 and 2 now live in `crates/holler-pane/tests/profile_{snapshot,diff}_test.rs` over
    `tests/common`, as do every place that names them (Decision 11, the Files list, AC 11, 12 and 14, the Size check and the
    RED note). Round 1's six warns are addressed.
  - **Decision 3 (`port_policy` now compared)** is consistent with #644's canonical `port_of_policy`, and ADR-0021 is edited
    in the same change.
  - **Five warns:**
    - `SpecField` writes each path twice (serde renames plus `as_str`), against the crate's single-source rule;
    - the Forward-compat and Follow-up gaps: `is_member` covers only one merge order, the rig list leaves out #647's rig,
      and #644's I-2 quotes the superseded Decision 3;
    - Decision 4 does not say why the verbs bypass `ProfileScope::resolve(P, None)`;
    - a git conflict with #647 on ADR-0021's adjacent "Deferred" lines 524 and 525;
    - `ProfileSpec.pane` (unchecked text) is left out of `show`'s escaping rule.
- **Assumed:**
  - The sibling plans, as read from their worktrees, are what those runs implement: #643 `837718b` (T-red PASS), #644
    `7195993`, #647 `8d1b199` (T-red PASS), #663 `89b611f` (brief gate overridden by the operator). None has merged;
    `origin/main` is `3bdd129`.
  - Git reports deletions on adjacent lines in two branches as a conflict; `stub.rs:12-16` documents this for its own list.
- **Hedged:**
  - Finding 1 is a warn, not a block: AC 2e's test pins `as_str` equal to the serde name, so the two copies cannot drift
    silently, and the fix changes neither the JSON nor the API.
  - Finding 3 is a warn: the bypass is justified by the issue's own port list and by `list`'s one-pass count; only the
    rationale is missing.
  - Findings 2 and 4 depend on a merge order that is the MO's call.
- **Evidence:**
  - **This brief:** the whole of `docs/handoffs/662-brief.md` at `31062ce` (2,037 lines), plus the `31062ce` diff.
  - **ADR-0021:** sections 3, 5 and 8, the failure-mode table and the "Deferred" list.
  - **holler-pane:** `src/{error,pane,profile,argv,lib}.rs`, `tests/common/mod.rs`, every `tests/*_test.rs` header and the
    `Cargo.toml`.
  - **holler-cli:**
    - `src/pane/{args,mod}.rs` and `src/output.rs`;
    - `tests/verb_harness/mod.rs` and `tests/profile_verbs/*.rs`;
    - `tests/pane_verbs/process/{stub,flags,main}.rs`;
    - the manifest's test targets.
  - **The test kit:** `profile_scope.rs` (`belongs`).
  - **holler-proto:** `src/{log,error}.rs`.
  - **Scripts:** `scripts/lint.sh`.
  - **Siblings:** 643-brief and `pane_verbs/list.rs`; 644-brief (I-1, I-2, decisions 5, 10, 20 and 25, the pre-flight grep);
    647-brief and `pane_verbs/doctor/rig.rs`; 663-brief (its Reuse map and Decision 2).
  - **Greps:** slug comparisons, escape helpers, per-variant serde renames, `port_policy`, spec-pane validation, inline
    tests, and the sibling briefs for this story's API names.
