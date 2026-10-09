# Handoff-S: #640 part 1 of 3, the pure Herdr protocol and grid core  (spec audit)

**Date:** 2026-10-09
**Branch:** issue-640-implementation (head d9e19e3; base origin/main 9d61c9f)
**Issue:** #640, part 1 of 3 (epic #633). The PR says `Part of #640`.
**Handoffs reviewed:** `docs/handoffs/640-brief.md`; `docs/handoffs/640/{handoff-A.md, handoff-T-red.md, handoff-F.md,
handoff-T-green.md, handoff-A-dup.md, evidence.md, decisions.md}`; GitHub issue #640 (body, no comments); the full diff
`git diff origin/main...HEAD` and every new test.

## A precondition

Met. `handoff-A.md` is PASS (14 warns, no block). `handoff-A-dup.md` is PASS (3 warns, no block).

## T precondition

Met.

- **RED.** `handoff-T-red.md` confirms RED: layout 1/10, planner 1/19 and protocol 3/33 pass.
  - The five passes pin constants or `GridMap::default()`.
  - Every failure is an assertion failure.
  - The stubs at 09ada91 hold no logic (Decision 12). Each body is `Err(NotImplemented)`, `""`, `String::new()` or
    the default.
- **GREEN.** `handoff-T-green.md` reports GREEN at 62/62 and "Blocking issues: None".

## Acceptance criteria

| # | Criterion (brief) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Spike 2x4: `w1:p7` at r2c3, `cells()` all eight row first, `"r2c3"` | `layout_test.rs` `ac1_the_spike_2x4_tree_puts_the_marker_at_row_2_column_3`. The fixture matches `herdr-grid.sh:55-57`: the same build order and the same ids. | PASS |
| 2 | Transposition and off-by-one guard; two mutants recorded in GREEN | `ac2_row_and_column_are_not_swapped_and_count_from_one`, plus T-green's mutant table (both caught, "layout 8 fail" each). S derived which eight fail. Transposition fails all but `ac6_the_default_map_is_an_empty_workspace` and `a_lone_pane_is_row_1_column_1`. Count-from-0 fails all but the two `ac6_*` tests. Both sets include AC 1 and AC 2. | PASS (advisory 3) |
| 3 | Nested chains flatten in order, both directions | `ac3_a_down_chain_nested_on_either_side_flattens_in_order`, `ac3_a_right_chain_nested_on_either_side_flattens_in_order` | PASS |
| 4 | Unaligned rows are ranked by the tree; ratios never move a position | `ac4_unaligned_rows_are_ranked_by_the_tree_and_ratios_change_nothing` | PASS |
| 5 | A nested slot is unplaced; later columns keep their slot numbers | `ac5_a_nested_slot_is_unplaced_and_later_columns_keep_their_slot_numbers` | PASS |
| 6 | Columns first places nothing; the default map is empty with `rows() == 0` | `ac6_a_columns_first_tree_places_nothing_but_is_not_empty` (also pins A's W-4); `ac6_the_default_map_is_an_empty_workspace`, which passes at RED by design because it pins `Default` | PASS |
| 7 | Empty to 2x4 is the exact recipe; the input order does not matter | `plan_splits_test.rs` `ac7_an_empty_workspace_to_2x4_is_the_spikes_verified_recipe` (also with reversed input); `ac7_three_rows_split_the_row_above_with_a_third_then_a_half` | PASS |
| 8 | One more cell is one split; rows come before columns | `ac8_one_more_cell_is_one_split`, `ac8_rows_are_planned_before_columns` | PASS |
| 9 | Idempotent, also with unplaced panes; duplicates ignored | `ac9_a_target_whose_cells_all_exist_is_no_steps`, `ac9_existing_cells_are_answered_even_with_an_unplaced_pane`, and `ac9_a_duplicate_cell_in_the_target_is_planned_once`. The last one fails without `dedup`, because the second `r1c2` becomes a gap. | PASS |
| 10 | Range checked first, `grid-out-of-range` naming the cell and the extent | `ac10_a_cell_outside_the_extent_is_grid_out_of_range_naming_cell_and_extent` (r1c2, r3c1, r0c1, r1c0), `ac10_the_range_is_checked_before_reachability` | PASS |
| 11 | `grid-unreachable` (a) to (d): a refusal class, names the cell, one line, no partial plan | `ac11a_an_empty_workspace_starts_at_r1c1`, `ac11b_a_gap_in_a_row_or_a_column_is_unreachable`, `ac11b_no_partial_plan_when_one_cell_of_the_target_is_unreachable`, `ac11c_a_new_row_below_a_row_of_two_is_unreachable` ("one pane"), `ac11c_a_new_row_below_a_row_of_one_is_reachable`, `ac11d_no_new_cell_while_any_pane_is_unplaced`, `ac11d_a_workspace_of_only_unplaced_panes_is_never_created_again`. Every refusal goes through `unreachable_naming`, which checks the code, `class_of == Refusal`, the named cell and that there is no newline. | PASS |
| 12 | The split-only cases that hold for real Herdr; the fake's `r2c2` step deliberately not reproduced | `ac12_an_empty_workspace_accepts_r1c1_alone`, `ac12_the_fakes_sequence_stops_where_real_herdr_nests`, and the AC 11a, 9 and 10 tests. Together these mirror the test kit's `split_only_mode_starts_an_empty_workspace_at_r1c1`, `..._returns_an_occupant_without_a_split` and `..._checks_the_range_first`. | PASS (see Spec compliance) |
| 13 | Shared vocabulary with the test kit | `ac13_the_refusal_code_is_the_test_kits`, `ac13_the_supported_versions_text_is_the_test_kits` | PASS |
| 14 | One JSON line per request; schema params; keys sent verbatim | `protocol_test.rs` `ac14_every_request_is_one_json_line_with_its_id_and_method`, `ac14_params_match_the_schema` (the exact params of all nine), `ac14_a_right_split_says_right`, `ac14_text_with_newlines_and_quotes_stays_on_one_line`, `ac14_keys_go_out_verbatim_with_no_case_folding` | PASS |
| 15 | The method allow-list | `ac15_every_method_is_on_the_allow_list_and_nothing_else_is` (a match with no wildcard arm, plus set equality), `ac15_the_allow_list_has_no_destructive_or_foreign_method` | PASS |
| 16 | `decode_reply` mapping; a garbled reply is `unavailable`; one line; no typed text | `ac16_a_result_reply_gives_the_result_object`, `ac16_pane_not_found_names_the_pane_of_the_request`, `ac16_pane_not_found_for_a_request_without_a_pane_is_unavailable`, `ac16_any_other_herdr_code_is_unavailable_naming_the_method_and_the_code`, `ac16_a_garbled_or_mismatched_reply_is_unavailable`, `ac16_no_message_has_a_newline_and_none_echoes_typed_text`, `ac16_a_request_never_prints_its_typed_text`. T-green's `Debug`-leak mutant confirms the last one fails when the code is wrong. | PASS (advisory 2) |
| 17 | Pong parsed; 22 accepted; 99 refused naming the version, **the protocol 99** and `SUPPORTED_VERSIONS` on one line; a missing protocol is unknown and refused; a non-pong result is `unavailable` | `ac17_the_spikes_pong_is_protocol_22_and_supported` (its fixture is the spike's section 13 pong, verbatim), `ac17_another_protocol_is_refused_naming_the_version_the_protocol_and_the_supported`, `ac17_a_pong_without_a_protocol_is_unknown_and_refused`, `ac17_a_result_that_is_not_a_pong_is_unavailable` | **PARTIAL.** No assertion can detect a message that leaves out "protocol 99" (REWORK 1). The code does name it (`protocol.rs:323`). |
| 18 | Snapshot: found by label, the lowest-numbered tab, an absent label is `None`, a duplicate label is `unavailable` naming the label and both ids, panes listed, extra fields ignored | `ac18_a_snapshot_gives_workspaces_by_label_with_the_lowest_numbered_tab`, `ac18_a_snapshot_lists_each_pane_with_its_workspace_and_tab`, `ac18_two_workspaces_with_one_label_are_unavailable_naming_both`, `ac18_a_garbled_snapshot_is_unavailable` | PASS |
| 19 | The layout export gives the AC 1 tree; a null or missing `pane_id`, `"left"` or the wrong type is `unavailable` | `ac19_the_2x4_export_is_the_2x4_tree`, `ac19_a_lone_pane_export_is_a_pane`, `ac19_a_split_keeps_its_direction_and_ratio`, `ac19_a_pane_without_an_id_an_unknown_direction_or_another_type_is_unavailable` | PASS |
| 20 | `workspace_created`, `pane_info`, the last lines of a read, `expect_ok` | `ac20_a_created_workspace_gives_its_ref_and_its_root_pane`, `ac20_pane_info_gives_the_pane_id`, `ac20_read_gives_the_last_lines_joined_without_a_trailing_newline`, `ac20_expect_ok_accepts_only_ok`, `a_result_of_the_wrong_type_is_unavailable_for_every_parser` | PASS |
| 21 | The gates | T-green: `cargo test -p` 62/62; `cargo test --workspace` 1317 passed, 0 failed; clippy `-D warnings` exit 0; machete clean; `lint.sh` exit 0. rustfmt: F's self-check reports it clean. T-green's table has no rustfmt row, and neither `lint.sh` nor CI runs rustfmt, so S ran the read-only `rustfmt --check --edition 2021` on all eight new `.rs` files: exit 0. There is no `unsafe`. The largest touched file is `protocol_test.rs` at 678 lines. | PASS |
| 22 | The CHANGELOG entry | `CHANGELOG.md`, `## [Unreleased]` / `### Enhancements`, the last entry before `## [0.4.0]`. It links #633 and #640, and `changelog-check` reports ok. | PASS |

## Spec compliance

- **Pinned API: matches.** S diffed the public and derive lines of the T-red stubs (09ada91) against HEAD. Only private
  items were added: `Slot`, `Widths`, `Method`, `Object` and `TabRef`. Two pinned items changed form but not meaning:
  - `Request`'s derived `Debug` became a hand-written one, as A's W-2 asked.
  - `ALLOWED_METHODS` is now built in a `const` block from the private `Method` table (A's W-3). It is still
    `[&str; 9]`, with the pinned values.

  Every signature in "The API this part creates" is present and unchanged.
- **Decisions 1 to 12 are implemented as stated.** The points worth recording:
  - **D3.** `grid_of` reads only the tree, and no rect is parsed.
  - **D5.** Every `Step::Split.creates` is a target cell. A cell that needs a pane nobody asked for is refused
    through `gap`.
  - **D6.** A new row needs a row of one pane above it (`plan.rs:181`).
  - **D7.** The cells that exist are answered before the refusal of an unplaced pane (`plan.rs:83-97`).
  - **D8.** The gate reads the integer `protocol`. A missing protocol becomes `None`, which is refused as "an unknown
    protocol".
  - **D9.** `Key::as_str` goes out verbatim.
  - **D10.** `pane_not_found` about a request's pane gives that pane's id (`protocol.rs:285-287`). Anything else is
    `unavailable`, and Herdr's `message` is never quoted.
  - **D11.** `SessionState::workspace` and `workspace_ref` (`min_by_key(number)`).
  - **D12.** Checked against the stubs.
- **A documented deviation, accepted.** `lib.rs` has no flat re-exports, though the brief asks for "re-exports of the
  names below". The deviation is not silent:
  - A recommended it before T started (W-1), because `Direction` and `SessionState` collide with `holler_proto`'s root
    names.
  - It is recorded in `handoff-T-red.md`, in `handoff-F.md` "Deviations", in `decisions.md`, and in `lib.rs`'s own
    docs.
  - Nothing pinned is renamed or dropped. The module paths are the public paths.
- **The issue and the brief disagree on one line (not a hold).**
  - **The disagreement.** Issue #640's acceptance still says "passes #638's split-only-mode case". AC 12 and
    Decision 6 deliberately refuse that sequence's `r2c2` step, because real Herdr nests a `down` split of `r1c2`
    (spike section 7, VERIFIED).
  - **Why it does not hold part 1.**
    - The brief is internally consistent and states its reason, and F implemented it faithfully.
    - The issue's own Scope line ("fails loudly if the position cannot be reached") supports the refusal.
    - This PR is `Part of #640` and closes none of the issue's acceptance.
  - **What must still happen** before part 3's PR closes the issue: the issue line must be amended (advisory 1).
- **Forward-compat check.** The conformance suite that part 2 must pass places only r1c1 and r2c1 in a 2x1 workspace,
  and expects r1c2 and r3c1 to be out of range. That is all reachable under the stricter rule, so part 2 is not boxed
  in.

## Quality audit

- **Correctness and failure handling.**
  - S walked `plan_splits` through these cases, and each gave the spike's recipe or the right refusal:
    - the 2x4 recipe and AC 8's cases;
    - multi-row extents;
    - a new row below a wider row;
    - a gap at the end of a row;
    - a duplicate target;
    - an empty or zero-sized extent;
    - `CreateRoot` with later steps.
  - A step only appends: a `right` split from the last pane of a row, or a `down` split under a row of one pane.
  - A refusal returns no plan at all (`collect::<Result<Vec<_>, _>>`).
  - The parsers fail closed. A missing or mistyped field is `unavailable`. `decode_reply` requires the request's id and
    exactly one of a result object or an error.
  - There is no I/O and no shared state.
- **Build guards.**
  - `src/` has no `unwrap`, `expect`, `panic!`, `unsafe`, `todo!` or `#[allow]` (grep).
  - The four `#![allow]` lines in the tests carry `// #640`.
  - The largest files are `protocol.rs` at 595 lines and `protocol_test.rs` at 678, both under 900.
  - There is no dead code (clippy `-D warnings`).
  - The dependency declarations match the house style: path dependencies for `holler-pane` and the testkit, and
    `serde_json` from the workspace, each with a comment.
- **Protocol changes.** Holler's wire is unchanged, so no golden file or `docs/protocol/v2.md` change is needed. The one
  new open code, `grid-unreachable`, goes through `RefusalCode::from_static`, and `class_of` gives it `Refusal`. The
  brief defers its ADR-0021 section 9 row to part 3, and A agreed.
- **Tests.**
  - All of them are pure: no sleep, socket, process or thread (grep).
  - RED-first evidence is in `handoff-T-red.md`.
  - One assertion cannot fail on its own (REWORK 1).
- **Documentation.**
  - The CHANGELOG entry is present and accurate.
  - The module docs state the conversion rule, the split model, the ratio rule (the extent's size, as T-red asked),
    the order of the checks, and the grid-tab rule with the way it fails (A's W-13).
  - There is no new CLI surface, log event or protocol field, so no README or `docs/` change is due.
- **Public-repository privacy.** S grepped every added line (code, tests, fixtures and the handoff documents) for
  personal names, hostnames, tailnet names, IPs, email addresses, home paths and secrets. The only hits are the
  synthetic test constant `SECRET = "hunter2-secret-text"`, the generic `/tmp/x` fixtures, a relative worktree path,
  and the brief's own rule text.
- **Commit and PR hygiene.**
  - The branch commits use Conventional Commit subjects (`docs(handoffs): ...`, `chore(#640): ...`).
  - Each commit carries `Co-Authored-By: Claude <noreply@anthropic.com>`. That is the same form as recent commits on
    `main`, which carry no session link either.
  - The PR is not opened yet (advisory 4).

## Scope check

F delivered the brief's scope exactly.

- **Production files:** `Cargo.toml`, `lib.rs`, `layout.rs`, `plan.rs`, `protocol.rs`, `CHANGELOG.md` and the
  `Cargo.lock` update.
- **T's test files:** the three test files plus `tests/common/mod.rs`, which holds the shared tree builders.
- **Untouched:** `holler-pane`, the test kit, `holler-cli`, the ADRs and the protocol docs.
- **No over-delivery.** There is no I/O, no transport and no `HerdrPort` implementation.
- **No under-delivery.** Every pinned item is present.
- **Size.** The production code is 1,124 lines against the brief's estimate of about 800. The difference is mostly doc
  comments, it adds no scope, and every file stays under the gate.

## Verdict

**REWORK (test-only).**

1. **`crates/holler-adapter-herdr/tests/protocol_test.rs:398`**, in
   `ac17_another_protocol_is_refused_naming_the_version_the_protocol_and_the_supported`.
   - **The problem.** The assertion `assert!(message.contains("99"))` cannot fail once line 397's
     `assert!(message.contains("99.0.0-fake"))` passes, because the version string itself contains "99". AC 17 says the
     message "names the reported version, the protocol `99` and `SUPPORTED_VERSIONS`", and the protocol clause has no
     assertion that proves it.
   - **The surviving mutant.** A `check_supported` whose message drops the protocol still passes the whole suite, for
     example `Some(_) => "an unsupported protocol".to_owned()` at `protocol.rs:323`.
   - **The fix.** Check the protocol apart from the version. Either of these works:
     - `assert!(message.replace("99.0.0-fake", "").contains("99"), "{message}")`, which keeps the wording open;
     - `assert!(message.contains("protocol 99"), "{message}")`.
   - **Confirm and record.** Show the new assertion fails with that mutant (local, not committed), and record the
     result.

   **TEST-ONLY:** no `src/` change is required.

## Advisory notes (non-blocking)

1. **Issue #640 still contradicts AC 12 and Decision 6.** Its acceptance line reads "passes #638's split-only-mode
   case".
   - **A's W-7 asked for two things:** amend that line with the issue's own "(amended <date>, <topic>)" marker, and
     file the #638 test-kit follow-up that tightens split-only mode to AC 11c.
   - **Neither exists yet** (issue search, 2026-10-09).
   - **When.** Do both before part 3's PR closes #640. Otherwise part 3's S audits against a source of truth that
     contradicts itself.
   - **A's W-9 is also unfiled.** It is the `HerdrSnapshot` amendment for unplaced panes, which A wanted landed before
     part 2.
2. **`ac16_pane_not_found_names_the_pane_of_the_request` cannot show where the id comes from.** Its Herdr message,
   "pane w1:p5 not found", names the same id as the request. So the test cannot tell "the id from the request" apart
   from an id parsed out of Herdr's message.
   - The code uses the request's id (`protocol.rs:285-287`).
   - The W-5 test makes the parsed-id reading unlikely.
   - **Optional fix.** If T is in the file for REWORK 1, a different id in that fixture's message (for example
     `"pane w9:p9 not found"`) would pin the source too.
3. **AC 2 asks T to record that each mutant fails AC 1 or AC 2.** T-green records "layout 8 fail" for each mutant but
   does not name the tests. S's derivation in the AC table shows that both sets include AC 1 and AC 2. Naming the
   tests in the rework handoff would make the record self-evident.
4. **The PR, which the script opens after S.**
   - Its title is the squash commit's subject, so make it a Conventional Commit, for example
     `feat(adapter-herdr): the pure Herdr protocol and grid core (#640 part 1)`.
   - Its body says `Part of #640`, not `Closes #640`.
   - It carries the AI disclosure that `CONTRIBUTING.md` requires. If the script's body lacks it, add it with
     `gh pr edit`.
5. **Carried forward to part 2.** These are already recorded and are not part 1 defects:
   - **A-dup W-1:** the `excerpt` copy. Its 64-character limit is unpinned, and T-green's `EXCERPT_LIMIT` mutant
     survived.
   - **A-dup W-2:** `Widths` repeats `layout::index` and `layout::saturate`.
   - **A-dup W-3:** a `Direction::ALL` table.
   - **handoff-A W-10, W-12 and W-13:** inputs to the part 2 brief.
