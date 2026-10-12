# Handoff-A-dup: Phase 8 - #648 the roster shows pane, SHOWN and DRIVEN, and a loud line when they differ  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** issue-648-implementation (worktree `.claude/worktrees/0648b-roster-shown-driven`)
**Diff base:** `bda1069` (the branch's merge base with `origin/main`)   **Diff head:** `23db217` (production code as of F's `f7f2bcf`; later commits touch only handoffs)
**Reuse map:** no separate `survey.md`. The map is the brief (`docs/handoffs/648-brief.md`: R5, "Decisions on the review's blocking points", the Blast radius) together with the binding warns W-1 to W-8 in `docs/handoffs/648/handoff-A.md`.
**Verdict:** PASS

## Summary

**PASS.** F extended `roster_cmd.rs`, the object the brief names. Each piece the map pointed to is reused, not rebuilt:
- **From `pane::list`:** `PaneRow::from`, `PaneRow::cells`, `COLUMNS`, `NO_VALUE`, `profile_name` and `text_value`.
- **From `output`:** `emit`, `emit_error`, `ErrorBody`, `ErrorCode`, `Sink` and `FormatChoice`.
- **The registry reads:** `PaneStore::list` and `ProfileScope::resolve`.
- **The wiring:** `Wiring::connect` and `Wiring::ports`, still built only in `main.rs`.

No new production file was added, and no file outside `holler-cli`, the docs and CHANGELOG changed.

**Three objects look like parallel paths but are reviewed decisions.** A asked for each in writing at Phase 4:
- the pair-rule predicate (W-2, option 1, pinned by a test);
- the roster's own fixed-width `table_line` (W-7: do not mix in `pane list`'s computed widths);
- `run_roster` beside `run_with_stdio` (W-1: `connect` only after the hub answered, never with `--server`).

The findings below are follow-ups and cosmetic folds. None is a block.

**Stack checks, all clean:**
- Untouched: `holler-hub`, `holler-pane`, `holler-proto` and the test kit.
- Golden files: no existing one changed; four new ones are under `tests/fixtures/roster_panes/`.
- Unchanged: `cli-surface.txt` and `docs/protocol/v2.md`. The six keys are CLI output, not wire fields.
- ADR 0003 and ADR-0021 are updated in the same change.
- Nothing persists.
- No secret or personal infrastructure name appears.
- Every touched `.rs` file is under 800 lines (`roster_cmd.rs` 578, `cli.rs` 784), and `scripts/lint.sh` exits 0.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn (for O) | `crates/holler-cli/src/roster_cmd.rs:335-343` | **The pair rule's third copy is approved, but the older two do not name it.** `shown_driven_mismatch` is W-2's option 1: named, its doc names both twins, and `registry::the_mismatch_rule_is_say_pane_targets_rule` pins it to `say_cmd::resolve_pane_target`. The two older copies still say "keep the two equal" (`say_cmd.rs:300`, `circuit/dispatch.rs:86`) and do not name the roster's. #839 edits both. | Add to #839's brief (or W-8's follow-up): when the rule changes, change all three, and reword the twins' "keep the two equal" to name `roster_cmd::shown_driven_mismatch`. Or fold the two CLI copies into one `pub fn` in `pane::list` then (W-2's alternative). |
| 2 | warn (for O) | `crates/holler-cli/src/pane/list.rs:239-240`; `crates/holler-pane/src/reconcile.rs:181-182` | **Phase 4's W-8, still open: two docs outside the blast radius are now untrue.** `PaneRow.driven` says "printed, never compared", and the roster now compares it. `shown_differs` lists "the roster" among its readers, but the roster uses the pair rule, as the brief decided. `list.rs:284`'s "the one SHOWN/DRIVEN rule" is the same ambiguity, and it predates #648. | O files the follow-up W-8 named and lists these three lines. No change in this run: the files are outside the blast radius, and `holler-pane` is frozen. |
| 3 | warn | `crates/holler-cli/src/roster_cmd.rs:221-226` (its caller at `:125`) | **`usage()` copies `output.rs`'s private `usage_body`.** The copy is three lines. Its one caller, `fail(sink, mode, usage(PROFILE_WITH_SERVER))`, writes the same bytes as the public `output::emit_usage_error(sink, Format::Text, PROFILE_WITH_SERVER)`. | Optional, at the next edit of the file: call `emit_usage_error` there and drop `usage()`. Not worth a rework round. |
| 4 | warn | `crates/holler-cli/tests/roster_panes/hubless.rs:18-28` | **The state-dir lock is a second copy.** `STATE_DIR_LOCK` and `with_empty_state_dir` copy `pane_integration/main.rs:52-67`, the pattern W-6 asked T to reuse. The scratch dir is `tempfile::tempdir()`, where W-6 named `support::StateDir`. That is the tempfile crate, not a copy of `StateDir`. Each test binary needs its own static lock anyway, so two copies are harmless. | If a third in-process target makes hub calls, move `state_dir_lock` and `with_state_dir` into `tests/support/`, or into a `#[path]` module as `verb_harness` is, and include it from all three. |
| 5 | warn | `crates/holler-cli/tests/docs_pane_control_test.rs:300-308` | **The new docs test copies a matcher it does not need.** `section_4_documents_the_roster_columns` copies the whole-word matcher of `section_4_names_every_column` (`:234-242`) verbatim. That check is already covered by the older test: every `PANE_COLUMNS` entry is in `COLUMNS`, which `registry::the_cells_are_pane_lists_cells_for_the_same_record` enforces. R9 asked for the test, so it stays. | Optional: add a `names_whole_word(text, word)` helper beside `section` and `ticked`, and use it in both tests. |
| 6 | info | `crates/holler-cli/tests/roster_panes_test.rs:116-162`; `roster_panes/registry.rs` (the four call-log asserts) | **`Rig` follows the per-target pattern.** It is a near-copy of `pane_verbs/list.rs:39-94`: the same seven fakes, the same `ports()`, and the four asserts of `assert_nothing_observed` written inline. It is the eighth such rig, after `pane_verbs/{list,close,park,launch,doctor}` and `profile_verbs/{rig,apply/rig}`. A rig per test target is the dominant pattern, so this is not drift for this story. | Out of scope. One test-kit builder for "ports over the fakes" would fold all eight, if O wants a test-kit story. |

**Checked, and not duplication:**

- **`table_line` and `WIDTHS`** (`roster_cmd.rs:82-85`, `:423-436`): a second table formatter beside `pane::list::table_text`, as W-7 asked. That `WIDTHS`' length is not tied to the column arrays was already raised at the diff gate (its W-1 and W-2), so it is not repeated here.
- **`pane_label`** (`:323-333`): the same as `holler_proto::RoutableName::label()`, except that a row with no label keeps its whole name. W-10 allowed a documented split. It is the one place #839 changes.
- **The legacy `--json` writer** (`:384-390`): it bypasses `output::emit` because today's bytes have no trailing newline and `write_line` always adds one. This is the roster's existing path, kept, not a new one.
- **`REMOTE_POLICY_REFUSED`** (`:68-72`): each `*_cmd.rs` already maps `RemotePolicyRefused` to exit 3 (`say_cmd.rs:181`, `answer_cmd.rs:61`, `interrupt_cmd.rs:64`, `wait_cmd.rs:86`, `hub_cmd.rs:66`). The roster's code is declared like `say_cmd.rs`'s open codes and recorded in ADR-0021 section 9.
- **`Registry`, `Mode`, `Registered`, `Joined`, `PaneKeys` and `RosterData`:** no existing object does their job. `Registry` and `Mode` are the shapes of W-1 and W-4. `PaneKeys` takes every value from the `PaneRow` it is given, never from the `Pane` again (W-5).
- **No rework round ran,** so there is no drift from rework to re-check.

## Notes for F

None. This is a PASS.

## Patterns referenced

- `crates/holler-cli/src/pane/list.rs`: `PaneRow`, `cells`, `COLUMNS`, `table_text`, `profile_name`, `text_value`.
- `crates/holler-cli/src/output.rs`: `emit`, `emit_error`, `emit_usage_error`, `usage_body`.
- `crates/holler-cli/src/say_cmd.rs:288-372` and `crates/holler-hub/src/circuit/dispatch.rs:77-103`: the pair rule and its twin.
- `crates/holler-cli/tests/pane_integration/main.rs:50-67`, `crates/holler-cli/tests/pane_verbs/list.rs:36-95` and `crates/holler-cli/tests/verb_harness/mod.rs`: the test harness patterns.
- `docs/handoffs/648/handoff-A.md` (W-1 to W-12) and `docs/handoffs/648-diff-result-r1.md` (the diff gate's PASS).
