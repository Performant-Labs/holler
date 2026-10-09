# Handoff-T-red: #640 part 1 of 3 - the pure Herdr protocol and grid core

**Date:** 2026-10-09
**Branch:** issue-640-implementation
**Brief / wireframe reviewed:** docs/handoffs/640-brief.md (no wireframe: no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (Phase 3), with 14 warns and no block (docs/handoffs/640/handoff-A.md).
A's W-1 to W-6 were settled in the tests as below.

- **W-1.** The tests import by module path (`layout::Direction`, `protocol::SessionState`). The stub `lib.rs` has no flat re-exports.
- **W-2.** `Request` has no hand-written `Debug` yet. The stub derives it, and `ac16_a_request_never_prints_its_typed_text` fails on the derived output.
- **W-3.** The AC 15 tests match every `Request` variant with no wildcard arm. They also assert that the methods the variants use equal `ALLOWED_METHODS` as a set, and that every `SUPPORTED_PROTOCOLS` entry is named in `SUPPORTED_VERSIONS`.
- **W-4.** `ac6_a_columns_first_tree_places_nothing_but_is_not_empty` pins that `is_empty()` is false when any pane exists, placed or not. `ac11d_a_workspace_of_only_unplaced_panes_is_never_created_again` pins that r1c1 on such a map is `grid-unreachable`, never `CreateRoot`.
- **W-5.** `ac16_pane_not_found_for_a_request_without_a_pane_is_unavailable` pins that `Ping`, `SessionSnapshot` and `WorkspaceCreate` give `Unavailable` for `pane_not_found`. `Split` names its target.
- **W-6.** Each test header ends `// #640`. `serde` is not declared: F adds it only if a derive uses it.

## Files

- **Stubs, which hold only the pinned signatures and constants (Decision 12):**
  - `crates/holler-adapter-herdr/src/{lib,layout,plan,protocol}.rs`
  - `crates/holler-adapter-herdr/Cargo.toml`: `holler-pane` and `serde_json` as dependencies, `holler-pane-testkit` as a dev-dependency.
  - `Cargo.lock`
  - Each stub body carries `// stub (#640 part 1): F fills`. There is no logic in any of them.
- **Tests:** `crates/holler-adapter-herdr/tests/{layout_test,plan_splits_test,protocol_test}.rs`, plus `tests/common/mod.rs` (tree builders: `pane`, `split`, `chain`, the spike's 2x4 tree). There is no `[[test]]` entry to add: this crate does not set `autotests = false`.

F must keep every pinned item and replace the stub bodies. F should delete the `_stub` field of `GridMap` and the stub comments. F may add `serde` if a derive needs it.

## Tests authored

Everything is a unit-level test with no I/O, process or sleep, which is the cheapest sufficient tier (the brief's Test plan).

### `layout_test.rs`

| Test | AC | Pins |
|---|---|---|
| `ac1_the_spike_2x4_tree_puts_the_marker_at_row_2_column_3` | 1 | `w1:p7` at r2c3, and `to_string()` gives `r2c3`; `cells()` lists all eight row first; `rows`/`cols_in` are 2/4/4/0; `unplaced` is empty |
| `ac2_row_and_column_are_not_swapped_and_count_from_one` | 2 | `p2` is r2c1 and `p3` is r1c2 (transposition guard); `at(r3c1)`, `at(r1c5)`, `at(r0c1)` and `at(r1c0)` are `None`; no placed cell has 0 |
| `ac3_a_down_chain_nested_on_either_side_flattens_in_order` | 3 | `down(A, down(B, C))` and `down(down(A, B), C)` both give A r1c1, B r2c1, C r3c1 |
| `ac3_a_right_chain_nested_on_either_side_flattens_in_order` | 3 | the same for a `right` chain within a row |
| `ac4_unaligned_rows_are_ranked_by_the_tree_and_ratios_change_nothing` | 4 | rows split 0.5/0.3 and 0.01/0.99 give identical cells |
| `ac5_a_nested_slot_is_unplaced_and_later_columns_keep_their_slot_numbers` | 5 | `unplaced == [p3, p9]`; `at(r1c2)` is `None`; `p4` stays at r1c3; row 2 is unchanged |
| `ac6_a_columns_first_tree_places_nothing_but_is_not_empty` | 6 + W-4 | no placed cell, four unplaced in tree order, `!is_empty()` |
| `ac6_the_default_map_is_an_empty_workspace` | 6 | `GridMap::default()` is empty, `rows() == 0` |
| `a_lone_pane_is_row_1_column_1` | boundary | a root that is a pane is r1c1 |
| `a_chain_helper_tree_agrees_with_the_hand_built_one` | helper guard | the `chain` builder (used by the planner tests) makes the shape the walk reads |

### `plan_splits_test.rs`

Ratios use a 1e-9 tolerance.

| Test | AC | Pins |
|---|---|---|
| `ac7_an_empty_workspace_to_2x4_is_the_spikes_verified_recipe` | 7 | exactly the 8 steps of the spike's recipe, and the same plan for a reversed `cells` input |
| `ac7_three_rows_split_the_row_above_with_a_third_then_a_half` | 7 | row ratio `1/(rows - r + 2)`, each split from the row above's first pane |
| `ac8_one_more_cell_is_one_split` | 8 | `[Split{r1c1,Right,1/2,r1c2}]` |
| `ac8_rows_are_planned_before_columns` | 8 | `[Down to r2c1, Right to r1c2]` |
| `ac9_a_target_whose_cells_all_exist_is_no_steps` | 9 | `Ok(vec![])` |
| `ac9_existing_cells_are_answered_even_with_an_unplaced_pane` | 9 | `Ok(vec![])` for existing cells of a non-grid map |
| `ac9_a_duplicate_cell_in_the_target_is_planned_once` | 9 | duplicates ignored |
| `ac10_a_cell_outside_the_extent_is_grid_out_of_range_naming_cell_and_extent` | 10 | r1c2, r3c1, r0c1 and r1c0 in 2x1 give `GridOutOfRange`; `what` names the cell and "2 rows by 1 column" |
| `ac10_the_range_is_checked_before_reachability` | 10 | out of range and unreachable gives out of range |
| `ac11a_an_empty_workspace_starts_at_r1c1` | 11a | r1c2 and r2c1 on an empty map give `grid-unreachable` |
| `ac12_an_empty_workspace_accepts_r1c1_alone` | 12 | `[CreateRoot]` |
| `ac11b_a_gap_in_a_row_or_a_column_is_unreachable` | 11b | r1c3 without r1c2, r3c1 without r2c1 |
| `ac11b_no_partial_plan_when_one_cell_of_the_target_is_unreachable` | 11b, 5 | one reachable cell plus one unreachable cell gives the refusal only |
| `ac11c_a_new_row_below_a_row_of_two_is_unreachable` | 11c | r2c1 and r2c2 below `r1c1 r1c2` give `grid-unreachable`, with "one pane" in the message |
| `ac11c_a_new_row_below_a_row_of_one_is_reachable` | 11c | the positive control: a `Down` split |
| `ac11d_no_new_cell_while_any_pane_is_unplaced` | 11d, 9 | a new r1c4 beside a nested slot is refused; r1c1 is still answered |
| `ac11d_a_workspace_of_only_unplaced_panes_is_never_created_again` | 11d, W-4 | r1c1 on a columns-first map is `grid-unreachable` |
| `ac12_the_fakes_sequence_stops_where_real_herdr_nests` | 12 | r1c2 is allowed, and r2c2 under `r1c1 r1c2` is refused |
| `ac13_the_refusal_code_is_the_test_kits` | 13 | `GRID_UNREACHABLE` equals the test kit's |

Every refusal is checked as `PaneError::Refused` with `code == GRID_UNREACHABLE`, `class_of == Refusal`, a message that names the cell and has no newline.

### `protocol_test.rs`

| Test | AC | Pins |
|---|---|---|
| `ac14_every_request_is_one_json_line_with_its_id_and_method` | 14 | one JSON object and exactly one trailing `\n`; `id` is `holler:<method>`; `params` is an object |
| `ac14_params_match_the_schema` | 14 | the exact `params` of all nine requests (`focus: false`, `source: "recent"`, `format: "text"`, ...) |
| `ac14_a_right_split_says_right` | 14 | the direction name |
| `ac14_text_with_newlines_and_quotes_stays_on_one_line` | 14 | the JSON escapes the text |
| `ac14_keys_go_out_verbatim_with_no_case_folding` | 14, Decision 9 | `enter`, `ctrl+c`, `Enter` and `C-c` go out as given |
| `ac15_every_method_is_on_the_allow_list_and_nothing_else_is` | 15, W-3 | no-wildcard variant match; the methods the variants use equal `ALLOWED_METHODS` as a set |
| `ac15_the_allow_list_has_no_destructive_or_foreign_method` | 15 | no `server.stop`, `server.live_handoff`, `layout.apply`, `pane.move`, `pane.swap`, `plugin.*` or `integration.*` |
| `ac15_every_supported_protocol_is_named_in_the_supported_versions_text` | W-3 | the two constants agree |
| `ac13_the_supported_versions_text_is_the_test_kits` | 13 | `SUPPORTED_VERSIONS` equals the test kit's |
| `ac16_a_result_reply_gives_the_result_object` | 16 | `Ok(result)` |
| `ac16_pane_not_found_names_the_pane_of_the_request` | 16 | `PaneNotFound{"w1:p5"}` for `Close`, `Read`, `SendText`, `SendKeys` and `Split` |
| `ac16_pane_not_found_for_a_request_without_a_pane_is_unavailable` | 16, W-5 | `Unavailable` naming the method |
| `ac16_any_other_herdr_code_is_unavailable_naming_the_method_and_the_code` | 16 | `invalid_request` and an unknown code |
| `ac16_a_garbled_or_mismatched_reply_is_unavailable` | 16 | not JSON, an empty line, another request's id, neither `result` nor `error`, a non-object result, an array |
| `ac16_no_message_has_a_newline_and_none_echoes_typed_text` | 16 | a newline in the Herdr message or code, a secret echoed by Herdr, and a truncated line holding the secret: none reaches `what`, `Debug` or `Display` |
| `ac16_a_request_never_prints_its_typed_text` | 16, W-2 | `{:?}` and `{:#?}` of `SendText` |
| `ac17_the_spikes_pong_is_protocol_22_and_supported` | 17 | the verbatim pong gives protocol 22 and is accepted |
| `ac17_another_protocol_is_refused_...` | 17 | `HerdrVersionUnsupported` names the version, `99` and `SUPPORTED_VERSIONS` on one line |
| `ac17_a_pong_without_a_protocol_is_unknown_and_refused` | 17 | `protocol: None`; refused, "unknown" named |
| `ac17_a_result_that_is_not_a_pong_is_unavailable` | 17 | the type check |
| `ac18_a_snapshot_gives_workspaces_by_label_with_the_lowest_numbered_tab` | 18 | `grid_tab` is the lowest `number` (tabs listed 2 then 1); an absent label gives `Ok(None)` |
| `ac18_a_snapshot_lists_each_pane_with_its_workspace_and_tab` | 18 | the panes, unknown fields ignored |
| `ac18_two_workspaces_with_one_label_are_unavailable_naming_both` | 18 | the `parse_snapshot` call succeeds and `workspace()` fails, naming the label and both ids |
| `ac18_a_garbled_snapshot_is_unavailable` | Decision 10 | wrong type, missing `snapshot`, `workspaces` not a list, missing `panes` |
| `ac19_the_2x4_export_is_the_2x4_tree` | 19 | the export, with `cwd`, `label` and `command: null`, parses to the spike tree |
| `ac19_a_lone_pane_export_is_a_pane` | 19 | a root that is a pane |
| `ac19_a_split_keeps_its_direction_and_ratio` | 19 | direction and ratio |
| `ac19_a_pane_without_an_id_an_unknown_direction_or_another_type_is_unavailable` | 19 | `pane_id` null, `pane_id` missing, direction `"left"`, wrong `type`, missing `layout` |
| `ac20_a_created_workspace_gives_its_ref_and_its_root_pane` | 20 | `grid_tab` is `tab.tab_id`, plus `root_pane.pane_id` |
| `ac20_pane_info_gives_the_pane_id` | 20 | `pane.pane_id` |
| `ac20_read_gives_the_last_lines_joined_without_a_trailing_newline` | 20 | last N lines, `max_lines` 0 gives `""`, N beyond the text gives all of it, blank lines kept |
| `ac20_expect_ok_accepts_only_ok` | 20 | only `type: "ok"` |
| `a_result_of_the_wrong_type_is_unavailable_for_every_parser` | Decision 10 | wrong `type` for the three remaining parsers |

## RED confirmation

Run: `cargo test -p holler-adapter-herdr --no-fail-fast`. The crate compiles, with no warning. The totals:

- `layout_test`: 1 passed, 9 failed. The one pass is `ac6_the_default_map_is_an_empty_workspace`, which pins `GridMap::default()`. The stub already provides it.
- `plan_splits_test`: 1 passed, 18 failed. The one pass is `ac13_the_refusal_code_is_the_test_kits`, which is expected: it pins constants (brief, Test plan).
- `protocol_test`: 3 passed, 30 failed. The passes pin constants only: `ac13_the_supported_versions_text_is_the_test_kits`, `ac15_every_supported_protocol_is_named_in_the_supported_versions_text` and `ac15_the_allow_list_has_no_destructive_or_foreign_method`.

Each failure is on an assertion about the missing behavior, never on an import, a setup step or a missing `[[test]]`. The failures group as below:

| Group | Failing output (excerpt) |
|---|---|
| `grid_of`/`GridMap` tests | `assertion left == right failed  left: []  right: [(GridPos { row: 1, col: 1 }, PaneId("A")), ...]`; `ac5`: `left: []  right: [PaneId("w1:p3"), PaneId("w1:p9")]` |
| `plan_splits` plans | `a plan: NotImplemented`: the stub returns `Err(NotImplemented)` where a plan is expected |
| `plan_splits` refusals | `expected grid-unreachable, got Err(NotImplemented)`; `ac10`: `r1c2 is outside 2 rows by 1 column` |
| `to_line` and `id` | `assertion failed: line.ends_with('\n')` on `""` |
| `method` | `ac15`: `left: ""  right: "ping"` |
| `decode_reply` | `left: Err(NotImplemented)  right: Ok(Object {"type": String("ok")})`; `right: Err(PaneNotFound { what: "w1:p5" })`; `expected unavailable, got NotImplemented` |
| W-2 | `assertion failed: !format!("{request:?}").contains(SECRET)`: the derived `Debug` prints the text |
| parsers | `called Result::unwrap() on an Err value: NotImplemented`; `left: Err(NotImplemented)  right: Ok(PaneId("w1:pA"))` |

Where the stub returns `NotImplemented` and the test expects a value, the failing assertion is the one the feature will satisfy (Decision 12).

Gates on the RED tree:

- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo machete` passes.
- `bash scripts/lint.sh` and `bash scripts/changelog-check.sh` exit 0. `protocol_test.rs` is 678 lines, under 900.
- Every touched `.rs` file is `rustfmt --edition 2021` clean.

## Notes for F and later

- **Message wording pinned by the tests.** The tests ask for these substrings and no more:
  - "2 rows by 1 column" for `grid-out-of-range`.
  - "one pane" (case-insensitive) for 11c, on both r2c1 and r2c2 below a two-column row. If F finds that the gap check should win for r2c2, F stops and reports it. F does not weaken the test.
  - "unknown" for a pong without a protocol.
  - Each refusal names its cell and its version.
- **`plan_splits` depends on `grid_of` in the tests.** Existing maps come from `grid_of` through `tests/common`, since `GridMap` has no other public constructor. The `plan_splits` tests are therefore meaningful only once `grid_of` is implemented.
- **Ratios.** For a target with fewer columns than the extent, the brief's `1/(cols - c + 2)` could read `cols` as the extent's or the row's. The tests pin only cases where the two agree. F should use the extent's, as in the spike's recipe, and document it.
- **The mutants of AC 2.** They are for T in GREEN. T does not run them now, because `grid_of` has no body.
- **No evidence appendix yet.** `evidence.md` is a GREEN duty. The tests rely on nothing outside the diff except the testkit constants, which AC 13 compares directly.

## Ready for F

Confirmed. RED is valid: every new test except the five constant and default-value pins fails on an assertion about the missing behavior, and none fails for an import, setup or `[[test]]` reason. F may implement against these tests.
