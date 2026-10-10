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

## F (Phase 5, implement; the Workflow script's phase 6) — 2026-10-09T18:27-06:00
- **Decided:** implementation done; T's tests are GREEN (handoff-F.md).
  - **Code.** The bodies of `profile_snapshot.rs` and `profile_diff.rs` (the brief's API, no extra pub item) and the two
    verbs `profile/list.rs` and `profile/show.rs`. No test file is edited.
  - **Docs.** The ADR-0003 `show` row (`holler profile show NAME`, `#662` in column 67), ADR-0021 Decision 14 (i) and (iii),
    and one CHANGELOG entry.
  - **Comparison.** `diff_spec` goes through `spec_from_pane(live)`. `env` and `expect` compare as sets, tied to those two
    fields; everything else by `FieldValue` equality. Values keep stored order.
  - **Escaping.** `Text` escapes control characters through `char::escape_default`, as `holler_proto::log` does. Argv and
    lists print as `serde_json`'s compact array, with DEL and C1 controls also escaped as JSON `\u` (stricter than the
    brief's literal form, same JSON value).
  - **Text-mode refinements the brief leaves open.** A count of 1 is singular (`count` in `list.rs`, `pub(crate)`, shared
    with `show`), and a `failed` probe with an empty `missing` list prints `failed`.
  - **`show`'s JSON row** flattens `PaneDiff` and adds `probe` (`Pane.probe.last`, read, never run). The header's live count
    rides in the view with `#[serde(skip)]`.
  - **`archChanged`:** true. Two empty modules gained a public API, and `profile_diff` now depends on `profile_snapshot`.
- **Assumed:**
  - The Workflow script commits this phase's work. F stages by explicit path and does not commit.
  - #644's pre-flight grep is exactly the three patterns in 644-brief lines 2146-2147; the three lines match them
    byte for byte.
  - `serde_json` keeps the C0-only escaping it has now. If a later version escaped DEL and C1 itself, the extra pass would
    find nothing to do, so the output does not depend on it.
- **Hedged:**
  - **`list`'s `data` is a bare array** (brief, AC 4a and 4c). #643's `pane list` uses an object, so that a top-level field
    can be added later. Left as the brief fixes it, and flagged to A for the read-verb forms settlement (brief Follow-up).
  - **The four extra cases offered to T** (DEL/C1 escaping, a count of 1, the `error (...)` probe form, `list` store
    failures) are optional and not defects.
- **Evidence:**
  - **Runs (all in the worktree):**
    - `cargo test -p holler-pane --test profile_snapshot_test --test profile_diff_test` gives 5/5 and 9/9, and
      `cargo test -p holler-pane` passes every target;
    - `cargo test -p holler-cli --test profile_verbs` gives 18/18; `cli_surface_test` 3/3, `docs_cli_test` 3/3,
      `pane_cli_process` 34/34;
    - `cargo clippy --workspace --all-targets -- -D warnings` is clean, `bash scripts/lint.sh` exits 0,
      `bash scripts/changelog-check.sh` is ok, `rustfmt --check --edition 2021` passes on all 10 touched `.rs` files, and
      `cargo machete` finds nothing;
    - the AC 2e, 3, 10 and 13 greps print nothing, or the expected lines;
    - `cargo test --workspace --no-fail-fast` gives 1404 passed and 4 failed. The 4 are all in `logging_test`, and pass
      11/11 with `HOLLER_STATE_DIR` pointed at an empty directory: environmental.
  - **Read:**
    - the brief in full, handoff-A, handoff-T-red, and T's tests and rig;
    - holler-pane `src/{lib,profile,pane,argv,grid,probe,error}.rs` and `tests/common/mod.rs`;
    - holler-cli `src/{output,profile/mod,pane/args,pane/wiring}.rs`, `tests/verb_harness/mod.rs`,
      `tests/docs_cli_test.rs` and `tests/pane_verbs/process/docs_rows.rs`;
    - the test kit's `fixture.rs`, `fault.rs` and `pane_store.rs`, and `holler-proto/src/log.rs`
      (`escape_field_value`);
    - ADR-0003 and ADR-0021 sections 3 and 9, and "Deferred";
    - #643's `pane/list.rs` (for its shared-view pattern) and 644-brief's pins.
  - **Evidence file:** `evidence.md` has 15 facts, each excerpt checked by script against its source lines.

## T (Phase 7, verify / GREEN) — 2026-10-09T18:36-06:00

- **Decided:**
  - GREEN is valid: F's commit touches no test, and all 14 pure tests and 18 verb tests pass as authored.
  - Added four test cases F offered (DEL/C1 in argv and lists, the `error ("...")` probe form with an ESC, a count of 1,
    a `list` store failure), by extending the tests that own each behaviour plus one new `list` test. Each was
    mutation-checked against F's code.
  - Not pinning `list`'s own sort: every existing store already lists in slug order, and a nine-method wrapper store
    to observe it is out of proportion. Recorded as advisory, and the test comment was corrected.
- **Assumed:** the 4 `logging_test` failures are environmental (a reachable hub); they pass 11/11 with an empty
  `HOLLER_STATE_DIR`, they failed the same way at RED, and the diff does not touch `roster`.
- **Hedged:** CI is the clean re-check for `logging_test`.
- **Evidence:**
  - `cargo test --workspace --no-fail-fast`: 1404 passed, 4 failed (logging_test only), identical to F's report;
    clippy, lint.sh, changelog-check, machete, rustfmt, docs_cli_test, wire_selftest all clean.
  - Mutations (each reverted): ordered env/expect, raw-name membership, unescaped Text, unread probe.last, bare
    `fixed` policy, raw C1 in JSON arrays, always-plural count, unquoted probe error: each caught. Removing the
    `list` sort: survives (advisory 1).
  - Four facts appended to `evidence.md` ("Added by T"), copied from source by T.

## A (Phase 7, anti-duplication gate) — 2026-10-09T18:44-06:00

- **Decided:** PASS, with five warns and no block (handoff-A-dup.md; diff `3bdd129..80b59c3`).
  - F reused every object the Reuse map named, and each "new here" item is one copy: `spec_from_pane`, `is_member`,
    `diff_spec`/`diff_profile`. No inline membership filter, no second slug rule, no output code beyond the `text`
    closures, no new code. The public API is exactly the brief's, and no frozen file changed.
  - **Five warns:**
    - `count` is the third private pluralizer (adapter `plan.rs:262`, kit `herdr.rs:498`), and no seam is reachable;
    - `SpecField::ALL` has no completeness test against `ProfileSpec`, and `ModelSpec`/`ContextCeilings` are copied whole;
    - #647's in-flight `findings::embedded` is a near-copy of `write_escaped`, to settle under the brief's text-forms
      Follow-up;
    - merging with #647 conflicts in ADR-0021 (delete both bullets) and CHANGELOG (keep both entries), both mechanical;
    - three cross-story items from round 2 are still open: #643's `names_profile`, the kit's `belongs`, and #644's stale
      `COMPARED` quote.
- **Assumed:**
  - The in-flight branches, as read from their worktrees at about 18:40 MDT, are what those runs will merge: #643
    `3140f35` (T-red), #644 `7195993` (brief), #647 `00f7ff2` (PR #701 open, A-dup PASS), #663 `8fe683b` (T-red).
  - A merge that `git merge-tree` reports as clean is clean at rebase too, given those heads.
- **Hedged:**
  - Finding 3 is a warn, not a block. Neither escape is on `main`, the brief records the settlement in writing, and either
    fold would edit the other story's file.
  - Finding 2 is a warn: the record types are frozen under the epic's amend-first rule, so a new field is rare and
    deliberate. The fix is one test.
- **Evidence:**
  - **Read in full:** the four production files, `rig.rs`, the brief's API, Behaviour, Reuse map and Follow-ups,
    handoff-A, handoff-F, handoff-T-green, and the ADR, CHANGELOG, fixture and `stub.rs` diffs.
  - **Greps:** slug comparisons, membership helpers, escape helpers, pluralizers, `Display` impls,
    `Value::String` text paths, `Refused` and new codes, new `pub` items, frozen-file and manifest diffs, and the
    sibling branches' added lines.
  - **Runs:**
    - `git merge-tree --write-tree HEAD issue-647-implementation` conflicts in 2 files;
    - a scratch Rust program over all 65 control characters: `escape_default` and `escape_debug` differ on NUL only;
    - `cargo clippy -p holler-pane -p holler-cli --all-targets -- -D warnings` is clean; `bash scripts/lint.sh` exits 0;
    - #644's pre-flight grep matches 3 lines.
