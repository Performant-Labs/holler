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

## T (Phase 4, author / RED) — 2026-10-09T18:10-06:00
- **Decided:** RED is valid (handoff-T-red.md).
  - **28 new tests:**
    - 5 in `crates/holler-pane/tests/profile_snapshot_test.rs`;
    - 9 in `crates/holler-pane/tests/profile_diff_test.rs`;
    - 3 in `profile_verbs/list.rs`;
    - 9 in `profile_verbs/show.rs`;
    - the shared `profile_verbs/rig.rs` (`#[path]` from `list.rs`).
  - **Results at RED:** 25 fail on behaviour assertions. 2c passes vacuously, as the brief expects.
  - **Signature stubs landed by T**, per the brief's Test plan ("the allowed approach"): `profile_snapshot.rs`,
    `profile_diff.rs`, and `ProfileShow { name }` with `run` still `not_implemented`.
  - **Surface changes:** removed the list/show STUBS entries (keeping `// #662`), and moved the `profile show` fixture lines
    to take NAME.
  - **`SpecField` serializes through `as_str()`** in the stub (A warn 1, single source), not through 15 renames. The tests
    are neutral between the two: they pin the 15 paths against a hand-written list and check that serde equals `as_str`.
  - **The rig's no-adapter check keeps the AC's name, `assert_no_adapter_call()`**, not #643's `assert_nothing_observed`
    (A warn 2b). AC 3 names it literally and S audits against the AC. The later fold of the rigs is a one-line rename.
  - **Added beyond the ACs:**
    - `snapshot_ignores_the_fields_a_spec_does_not_hold` (the "Not copied" list);
    - `fixed_port_policy_is_the_prefix_and_the_port` (#644's pin);
    - `field_values_print_for_a_person` (the rest of the `Display` table);
    - show's bad-name `usage`;
    - show's profile-store failure passing through.
  - A warn 5 is pinned in 5e: a spec pane with ESC prints escaped.
- **Assumed:**
  - **AC 2c's "fully populated pane of 1a"** is covered by `common::pane()`, which is fully populated, plus a sparse
    orchestrator pane (the `None` and empty side). The 1a pane itself is not repeated, which avoids a second copy of its
    builder in a second file.
  - **The text forms** are exactly the brief's "What each verb prints": two-space indents, spec blocks before pane rows, and
    no probe line under a `Missing` row.
  - **The `logging_test` failures** (4 cases, `Unexpected success` on `holler roster`) are environmental: a hub is
    reachable on this machine. This diff does not touch roster.
- **Hedged:**
  - Before restoring the stubs, I validated satisfiability with a throwaway reference implementation of the pure modules
    and of list/show, written only from the brief. All 32 tests passed, and so did `cli_surface_test` and
    `pane_cli_process`. The throwaway code was reverted and is not staged. F writes its own.
  - `docs_cli_test` fails at RED on ADR-0003:68 (`holler profile show` without NAME). That is F's ADR row (AC 9), not a
    test defect.
- **Evidence:**
  - **Runs:**
    - `cargo test -p holler-pane --test profile_snapshot_test --test profile_diff_test --no-fail-fast` gives 1/9 and 0/5;
    - `cargo test -p holler-cli --test profile_verbs` gives 6/18, where the 6 passing are the other verbs' stub cases;
    - `cli_surface_test` 3/3, `pane_cli_process` 34/34, and `docs_cli_test` 2/3 (the ADR row).
  - **Lints:** `cargo clippy -p holler-pane -p holler-cli --all-targets -- -D warnings` exits 0, `bash scripts/lint.sh`
    exits 0, and `rustfmt --check --edition 2021` passes on every touched file.
  - **Read:**
    - the brief in full and handoff-A;
    - holler-pane `tests/common/mod.rs`;
    - the test kit's `lib`, `fixture`, `fault`, `envelope`, `pane_store`, `profile_store` and the adapter constructors;
    - `verb_harness/{mod,parse}.rs`, `pane_verbs/process/{stub,docs_rows}.rs` and `docs_cli_test.rs`;
    - #643's rig at `837718b`.
