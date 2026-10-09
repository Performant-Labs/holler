# Handoff-S: #640 part 1 of 3, the pure Herdr protocol and grid core  (spec audit, round 2 after the test-only REWORK)

**Date:** 2026-10-09
**Branch:** issue-640-implementation (head a4f193c; base origin/main 9d61c9f)
**Issue:** #640, part 1 of 3 (epic #633). The PR says `Part of #640`.
**Round:** 2. This replaces round 1. Round 1 returned REWORK (test-only) at a5e0fef and stays in git history there.
**Handoffs reviewed:**
- The brief: `docs/handoffs/640-brief.md`.
- `docs/handoffs/640/`: `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` (with its rework
  section), `handoff-A-dup.md` (round 2), `evidence.md` and `decisions.md`.
- GitHub issue #640: the body (no comments).
- The full diff, `git diff origin/main...HEAD`, and all four test files.

## A precondition

Met.

- `handoff-A.md` is PASS, with 14 warns and no block.
- `handoff-A-dup.md` (round 2, at df48467) is PASS. It carries round 1's three warns and adds no block.

## T precondition

Met.

- **RED.** `handoff-T-red.md` confirms RED at 09ada91: layout 1/10, planner 1/19, protocol 3/33.
  - Every failure is an assertion failure.
  - The five tests that pass pin constants or `GridMap::default()`.
  - The stubs hold no logic (Decision 12).
- **GREEN.** `handoff-T-green.md` reports 62/62 and "Blocking issues: None".
- **After the rework,** T re-ran everything and it is green:
  - `protocol_test` passes 33/33.
  - `cargo test --workspace` has no failure.
  - clippy `-D warnings` is clean, and `lint.sh` shows only the existing size warnings.

## Round 1's REWORK: resolved

Round 1 found that `protocol_test.rs:398` asserted `contains("99")`. That assertion could not fail once
`contains("99.0.0-fake")` passed.

- **The fix.** The assertion is now `assert!(message.contains("protocol 99"), "{message}")` at
  `protocol_test.rs:399`, with a comment that says why. S checked the following:
- **The delta is that line and nothing else.**
  - `git diff a5e0fef HEAD -- crates/` is +2/-1 in `protocol_test.rs`.
  - The production code is still F's 78f5274, byte for byte. `git diff --stat 78f5274 HEAD` over `src/`,
    `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md` is empty.
  - `git diff --stat 09ada91 HEAD -- tests` lists only this file.
- **The assertion now bites.** "protocol 99" can come only from `check_supported`'s protocol clause
  (`protocol.rs:323`, `:328`):
  - the version is quoted and follows the word "version" (`version "99.0.0-fake"`);
  - `SUPPORTED_VERSIONS` reads "protocol 22".

  So a message that drops the protocol fails it, and so does one that names the wrong protocol. T's mutant record
  matches: `Some(_) => "an unsupported protocol"` gives 32 passed and 1 failed, at line 399.
- **Format.** `rustfmt --check --edition 2021` on the changed file exits 0.

## Acceptance criteria

| # | Criterion (brief) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Spike 2x4: `w1:p7` at r2c3, `cells()` all eight row first, `"r2c3"` | `layout_test.rs` `ac1_the_spike_2x4_tree_puts_the_marker_at_row_2_column_3`. The fixture (`tests/common/mod.rs` `spike_2x4`) has the build order and ids of `herdr-grid.sh:55-57`. | PASS |
| 2 | Transposition and off-by-one guard; T records two mutants | `ac2_row_and_column_are_not_swapped_and_count_from_one` and T-green's mutant table (both mutants caught). S re-derived the failures (advisory 3). | PASS |
| 3 | Nested chains flatten in order, in both directions | `ac3_a_down_chain_nested_on_either_side_flattens_in_order`, `ac3_a_right_chain_nested_on_either_side_flattens_in_order` | PASS |
| 4 | Unaligned rows are ranked by the tree; ratios never move a position | `ac4_unaligned_rows_are_ranked_by_the_tree_and_ratios_change_nothing` (0.5/0.3 and 0.01/0.99) | PASS |
| 5 | A nested slot is unplaced; later columns keep their slot numbers; row 2 is unchanged | `ac5_a_nested_slot_is_unplaced_and_later_columns_keep_their_slot_numbers` | PASS |
| 6 | Columns first places nothing; the default map is empty, with `rows() == 0` | `ac6_a_columns_first_tree_places_nothing_but_is_not_empty`, `ac6_the_default_map_is_an_empty_workspace` | PASS |
| 7 | Empty to 2x4 is the exact recipe; the input order does not matter | `plan_splits_test.rs` `ac7_an_empty_workspace_to_2x4_is_the_spikes_verified_recipe` (also with reversed input); `ac7_three_rows_split_the_row_above_with_a_third_then_a_half` | PASS |
| 8 | One more cell is one split; rows come before columns | `ac8_one_more_cell_is_one_split`, `ac8_rows_are_planned_before_columns` | PASS |
| 9 | Idempotent, also with unplaced panes; duplicates ignored | `ac9_a_target_whose_cells_all_exist_is_no_steps`, `ac9_existing_cells_are_answered_even_with_an_unplaced_pane`, `ac9_a_duplicate_cell_in_the_target_is_planned_once`. The last fails without `dedup`: the second `r1c2` becomes a gap. | PASS |
| 10 | Range checked first, `grid-out-of-range` naming the cell and "2 rows by 1 column" | `ac10_a_cell_outside_the_extent_is_grid_out_of_range_naming_cell_and_extent` (r1c2, r3c1, r0c1, r1c0), `ac10_the_range_is_checked_before_reachability` | PASS |
| 11 | `grid-unreachable` (a) to (d): a refusal, names the cell, one line, no partial plan | `ac11a_…`, `ac11b_a_gap_…`, `ac11b_no_partial_plan_…`, `ac11c_a_new_row_below_a_row_of_two_is_unreachable` ("one pane"), `ac11c_…_of_one_is_reachable`, `ac11d_no_new_cell_while_any_pane_is_unplaced`, `ac11d_a_workspace_of_only_unplaced_panes_is_never_created_again`. Each refusal goes through `unreachable_naming`, which checks the code, `class_of == Refusal`, the cell and that there is no newline. | PASS |
| 12 | The split-only cases that hold for real Herdr; the fake's `r2c2` step deliberately not reproduced | `ac12_an_empty_workspace_accepts_r1c1_alone`, `ac12_the_fakes_sequence_stops_where_real_herdr_nests`, and the AC 9, 10 and 11a tests | PASS (see Spec compliance) |
| 13 | Shared vocabulary with the test kit | `ac13_the_refusal_code_is_the_test_kits`, `ac13_the_supported_versions_text_is_the_test_kits` | PASS |
| 14 | One JSON line per request; schema params; keys sent verbatim | `protocol_test.rs` `ac14_every_request_is_one_json_line_with_its_id_and_method`, `ac14_params_match_the_schema` (all nine params exactly), `ac14_a_right_split_says_right`, `ac14_text_with_newlines_and_quotes_stays_on_one_line`, `ac14_keys_go_out_verbatim_with_no_case_folding` | PASS |
| 15 | The method allow-list | `ac15_every_method_is_on_the_allow_list_and_nothing_else_is` (a match with no wildcard arm, plus set equality), `ac15_the_allow_list_has_no_destructive_or_foreign_method` | PASS |
| 16 | `decode_reply`: the mapping; a garbled reply is `unavailable`; one line; no typed text | `ac16_a_result_reply_gives_the_result_object`, `ac16_pane_not_found_names_the_pane_of_the_request`, `ac16_pane_not_found_for_a_request_without_a_pane_is_unavailable`, `ac16_any_other_herdr_code_is_unavailable_naming_the_method_and_the_code`, `ac16_a_garbled_or_mismatched_reply_is_unavailable`, `ac16_no_message_has_a_newline_and_none_echoes_typed_text`, `ac16_a_request_never_prints_its_typed_text` | PASS (advisory 4) |
| 17 | Pong parsed; 22 accepted; 99 refused naming the version, the protocol 99 and `SUPPORTED_VERSIONS` on one line; a missing protocol is unknown and refused; a non-pong is `unavailable` | `ac17_the_spikes_pong_is_protocol_22_and_supported` (the fixture matches spike section 13, lines 415-416, verbatim), `ac17_another_protocol_is_refused_naming_the_version_the_protocol_and_the_supported` (now `contains("protocol 99")`), `ac17_a_pong_without_a_protocol_is_unknown_and_refused`, `ac17_a_result_that_is_not_a_pong_is_unavailable` | **PASS** (round 1: PARTIAL; fixed) |
| 18 | Snapshot by label; the lowest-numbered tab; an absent label is `None`; a duplicate label is `unavailable` naming the label and both ids; panes listed; extra fields ignored | `ac18_a_snapshot_gives_workspaces_by_label_with_the_lowest_numbered_tab`, `ac18_a_snapshot_lists_each_pane_with_its_workspace_and_tab`, `ac18_two_workspaces_with_one_label_are_unavailable_naming_both`, `ac18_a_garbled_snapshot_is_unavailable` | PASS |
| 19 | The export gives the AC 1 tree; a null or missing `pane_id`, `"left"` or another type is `unavailable` | `ac19_the_2x4_export_is_the_2x4_tree`, `ac19_a_lone_pane_export_is_a_pane`, `ac19_a_split_keeps_its_direction_and_ratio`, `ac19_a_pane_without_an_id_an_unknown_direction_or_another_type_is_unavailable` | PASS |
| 20 | `workspace_created`, `pane_info`, the last lines of a read, `expect_ok` | `ac20_a_created_workspace_gives_its_ref_and_its_root_pane`, `ac20_pane_info_gives_the_pane_id`, `ac20_read_gives_the_last_lines_joined_without_a_trailing_newline`, `ac20_expect_ok_accepts_only_ok`, `a_result_of_the_wrong_type_is_unavailable_for_every_parser` | PASS |
| 21 | The gates | From T: `cargo test -p` 62/62; `cargo test --workspace` green before and after the rework; clippy `-D warnings` exit 0; machete clean; `lint.sh` exit 0. From S (read-only): `rustfmt --check --edition 2021` on all eight new `.rs` files exits 0; there is no `unsafe`; the largest touched file is `protocol_test.rs` at 679 lines. | PASS |
| 22 | The CHANGELOG entry | `CHANGELOG.md:131`, under `## [Unreleased]` / `### Enhancements`, linking #633 and #640. Its text is accurate, and `changelog-check` reports ok (T). | PASS (but see advisory 1) |

## Spec compliance

- **The pinned API matches.** S diffed the public and derive lines of the T-red stubs (09ada91) against HEAD.
  - The stubs' `_`-prefixed parameter names lost their prefix.
  - The private types `Slot`, `Method` and `Object` gained derives.
  - `Request`'s derived `Debug` became the hand-written one that A's W-2 asked for.
  - `ALLOWED_METHODS` is built in a `const` block from the private `Method` table (A's W-3). It is still
    `[&str; 9]`, with the brief's values in the brief's order.

  Every item and signature under "The API this part creates" is present.
- **Decisions 1 to 12 are implemented as stated.** The production code is unchanged since round 1 checked each one.
  S re-checked the decisions that matter most:
  - **D3:** `grid_of` reads only the tree.
  - **D5:** each `Step::Split.creates` is a target cell, and a cell that needs a pane nobody asked for is refused
    through `gap`.
  - **D6:** a new row is allowed only below a row of one pane (`plan.rs:180-191`).
  - **D7:** cells that exist are answered before an unplaced pane is refused (`plan.rs:83-97`).
  - **D8:** the integer `protocol` decides support, and a missing one is "an unknown protocol".
  - **D9:** keys go out verbatim.
  - **D10:** `pane_not_found` names the request's own pane, and anything else is `unavailable`. Herdr's
    `message` is never quoted.
  - **D11:** a workspace is found by its label, and its grid tab is `min_by_key(number)`.
- **A documented deviation, accepted (as in round 1).** `lib.rs` has no flat re-exports, though the brief asks for
  them.
  - A recommended this before T started (W-1), because `Direction` and `SessionState` collide with
    `holler_proto`'s root names.
  - It is recorded in T-red, F, `decisions.md` and `lib.rs`'s own docs.
  - Nothing pinned is renamed or dropped.
- **The issue and the brief disagree on one acceptance line. That does not hold part 1.**
  - **The disagreement.** Issue #640 says "passes #638's split-only-mode case". AC 12 and Decision 6 deliberately
    refuse part of it.
  - **Where it diverges.** S traced `fake_herdr_test.rs:191-218`, which makes six `ensure_pane` calls, through
    `plan_splits`. The adapter agrees with the fake on the first four:
    - `r2c2` in an empty workspace is refused (11a);
    - `r1c1` is accepted;
    - `r2c2` before `r1c2` is refused (a gap naming `r2c1`);
    - `r1c2` is accepted, as a right split of `r1c1`.

    It diverges on the last two. After `r1c2` exists, the fake accepts `r2c2` and then `r2c1`. The adapter refuses
    both under 11c, because real Herdr nests a `down` split made in a row of two (spike section 7, VERIFIED, lines
    236-239).
  - **Why it does not hold part 1.**
    - The brief states the deviation and the evidence for it, and F implemented it faithfully.
    - The issue's own Scope ("fails loudly if the position cannot be reached") supports the refusal.
    - This PR is `Part of #640` and closes none of the issue's acceptance.
  - **The issue line still needs amending (advisory 2).**
- **Forward-compat holds.** The conformance suite that part 2 must pass (`conformance/herdr.rs`) uses a 2x1
  workspace.
  - It places only `r1c1` and `r2c1`.
  - It expects `r1c2` and `r3c1` to be `grid-out-of-range`.
  - It re-makes `r2c1` after a close.

  Every one of those is reachable under the stricter rule.

## Quality audit

- **Correctness and failure handling.**
  - **The planner.** S walked `plan_splits` through the cases below. Each gave the spike's recipe or the right
    refusal:
    - the 2x4 recipe (also with reversed input) and the 3x1 ratios (1/3, then 1/2);
    - AC 8's two cases, and the fake's six-step sequence above;
    - `[2,1]` plus `r1c3` gives a right split of `r1c2`;
    - `[1]` plus `{r2c1, r3c1, r2c2}` in 3x2 gives down 1/3, then down 1/2, then right 1/2, because rows come
      first and so the row above is still a row of one;
    - `[1,2]` plus `r3c1` gives 11c;
    - asking for AC 5's nested slot `r1c2` gives 11d;
    - an empty or zero-sized extent.
  - **Steps only append**, and a refusal returns no plan at all (`collect::<Result<Vec<_>, _>>`).
  - **The parsers fail closed.** A missing or mistyped field is `unavailable`. `decode_reply` requires the
    request's id and exactly one of a result object or an error.
  - **The pure core** has no I/O and no shared state.
- **AC 2's mutants, re-derived by S** (by reasoning, not by running them):
  - **Transposition.** The spike tree's root is a `down` split, so a swapped walk sees one row with two split
    slots. All eight panes become unplaced, so `position_of` in AC 1 and AC 2 returns `None`.
  - **Counting from 0.** This puts `w1:p7` at r1c2 and `w1:p2` at r1c0.

  Both fail AC 1 and AC 2.
- **Build guards.**
  - A grep of `src/` finds no `unwrap(`, `expect(`, `panic!`, `unsafe`, `todo!`, `unimplemented!`, `dbg!`,
    `println!` or `#[allow]`. The only matches for `unwrap` are three total `unwrap_or` calls.
  - The four `#![allow]` lines in the tests each carry `// #640`.
  - File sizes: `src/` 39, 222, 268 and 595 lines; the tests 86, 207, 352 and 679. All are under 900.
  - There is no dead code (clippy `-D warnings`, from T).
  - The dependencies follow the house style. `holler-pane` and the test kit are path dependencies, and the test kit
    is a dev-dependency only. `serde_json` comes from the workspace. Each has a comment.
- **Protocol changes.** Holler's own wire is unchanged, so no golden file and no `docs/protocol/v2.md` change. The one
  new open code, `grid-unreachable`, goes through `RefusalCode::from_static`, and the tests check that `class_of`
  gives `Refusal`. Its ADR-0021 section 9 row is part 3's, by the brief.
- **Tests.**
  - They are pure. A grep finds no sleep, socket, `Command`, thread, environment variable or `tempfile`.
  - RED-first evidence is in T-red.
  - Since the rework, no assertion is implied by another one that S could find.
- **Documentation.**
  - The CHANGELOG entry is present and accurate.
  - The module docs state the conversion rule, the split model, the ratio rule, the order of the checks and the
    grid-tab rule with the way it fails.
  - There is no new CLI surface, log event or protocol field, so no README or `docs/` change is due.
- **Public-repository privacy.** S grepped all 4,355 added lines (code, tests, fixtures and handoffs) for personal
  names, hostnames, tailnet names, IPs, email addresses, home paths and secret patterns. The only hits are:
  - the public trailer address `noreply@anthropic.com`;
  - the synthetic test constant `SECRET = "hunter2-secret-text"`;
  - the generic `/tmp/x` fixtures;
  - the brief's own rule text.
- **Commit and PR hygiene.**
  - Every branch commit has a Conventional Commit subject (`chore(#640): …`, `docs(handoffs): …`).
  - Every branch commit carries `Co-Authored-By: Claude <noreply@anthropic.com>` but no session link.
    `CONTRIBUTING.md:19-21` describes a trailer and a session link, but the recent squash merges on `main`
    (cd635c0, c76bbed) carry no session link either. The squash commit replaces these commits anyway
    (advisory 5).
  - No PR is open yet (`gh pr list --head issue-640-implementation`: none).
- **Merge readiness: new since round 1.** `origin/main` moved after round 1.
  - cd635c0 (#698, for #661) and c76bbed (#697, for #688) each append an `[Unreleased]` entry at the same place as
    this branch: after the #681 entry and before `## [0.4.0]`.
  - `git merge-tree 9d61c9f HEAD origin/main` reports a textual conflict in `CHANGELOG.md` only. Every other file
    merges cleanly.
  - This is not a defect in F's work. Advisory 1 says how to resolve it before the PR can merge.

## Scope check

F delivered the brief's scope exactly.

- **Production:** `Cargo.toml`, `lib.rs`, `layout.rs`, `plan.rs`, `protocol.rs`, `CHANGELOG.md` and the cargo
  update to `Cargo.lock`.
- **Tests (T's):** the three test files, plus `tests/common/mod.rs` for the shared tree builders.
- **Untouched:** `holler-pane`, the test kit, `holler-cli`, the ADRs, the protocol docs and the golden files.
- **No over-delivery:** there is no I/O, no transport and no `HerdrPort` implementation.
- **No under-delivery:** every pinned item is present.
- **The rework stayed in scope:** it is one assertion in one test file, with no `src/` change.
- **Size:** 1,124 production lines against the brief's estimate of about 800. Most of the difference is doc
  comments, it adds no scope, and every file is under the gate.

## Verdict

**PASS.**

- All 22 acceptance criteria are met, each with a test that would fail if the behaviour were wrong.
- Round 1's one finding (AC 17) is fixed, and the production code is unchanged.
- The implementation follows the brief, and its one deviation (no flat re-exports) is documented and accepted.
- Quality is acceptable.

Ready for O. Before the PR can merge, though, the branch must be rebased onto `origin/main` (advisory 1).

## Advisory notes (non-blocking)

1. **Rebase before merging. The branch conflicts with `origin/main` in `CHANGELOG.md`.**
   - **Who.** This is for whoever pushes and merges the PR.
   - **The fix.** Rebase `issue-640-implementation` onto `origin/main`. Resolve `CHANGELOG.md` by keeping all three
     `[Unreleased]` / `### Enhancements` entries: main's #661 and #688 entries, then this #640 entry, before
     `## [0.4.0]`.
   - **Then check.** Run `bash scripts/changelog-check.sh`, and let CI run on the rebased head.
   - **Why it matters.** Until this is done, GitHub cannot build the PR's merge ref, so its required checks cannot go
     green.
   - **What else changes.** Nothing else in the rebase touches this crate, `holler-pane` or the test kit's Herdr
     files (A-dup round 2).
2. **Issue #640 still contradicts the brief in two places, and none of A's follow-ups is filed** (`gh issue list`,
   2026-10-09).
   - **The split-only line (A's W-7).** The acceptance line still reads "passes #638's split-only-mode case". AC 12
     and Decision 6 refuse that case's `r2c2` and `r2c1` steps.
   - **The `host.herdr_api_version` line (A's W-11).** The Scope line says the adapter records the version. Decision
     8 says the adapter does not write the pane store, and that #644 and #647 record and show it.
   - **Unfiled:**
     - the #638 test-kit follow-up that tightens split-only mode to AC 11c (W-7);
     - the `HerdrSnapshot` amendment for unplaced panes (W-9), which A wants landed before part 2.
   - **When.** O or the MO amends both issue lines with the issue's own "(amended <date>, <topic>)" marker, and files
     the follow-ups, before the part 2 brief is written. Otherwise part 3's S audits against a source of truth that
     contradicts itself.
3. **AC 2's mutant record does not name the tests.** T-green says "layout 8 fail" for each mutant but names no test.
   S's re-derivation (in the Quality audit) shows that both mutants fail AC 1 and AC 2.
4. **`ac16_pane_not_found_names_the_pane_of_the_request` cannot show where the id comes from.** This is round 1's
   advisory 2, which T left as it is.
   - **The problem.** Herdr's message in the fixture names the same id as the request, `w1:p5`.
   - **Why it is not blocking.** The code takes the id from the request (`protocol.rs:285-287`).
   - **The fix, when part 2 touches the file.** Change that fixture's message to another id, such as
     `"pane w9:p9 not found"`.
5. **The PR, which the script opens after S.**
   - **The title.** It becomes the squash commit's subject, so make it a Conventional Commit, for example
     `feat(adapter-herdr): the pure Herdr protocol and grid core (#640 part 1)`.
   - **The body.** It says `Part of #640`, not `Closes #640`, and carries the AI disclosure that
     `CONTRIBUTING.md:25-29` requires. If the script's body lacks it, add it with `gh pr edit`.
   - **The trailer.** The squash commit should keep the `Co-Authored-By:` trailer.
6. **Carried forward to part 2.** These are already recorded and are not part 1 defects:
   - **A-dup W-1:** the `excerpt` copy. Its 64-character limit is not pinned by any test.
   - **A-dup W-2:** `Widths` repeats `layout::index` and `layout::saturate`. Also, `layout.rs:21-23` says "the order
     in one, `cell_at`", but the direction-to-axis order is also applied in `grid_of`'s two `chain` calls. That is
     where T's transposition mutant lives.
   - **A-dup W-3:** a `Direction::ALL` table.
   - **handoff-A W-10, W-12 and W-13:** inputs to the part 2 brief.
