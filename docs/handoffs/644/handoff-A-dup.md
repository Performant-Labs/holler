# Handoff-A-dup: #644 part 2 - the OpenCode agent through `launch` and `relaunch`  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** `issue-644-implementation`
**Diff base:** `7284001` (`origin/main`, #802 merged; the merge base)   **Diff head:** `3026df7` (T-green)
**Reuse map:** the brief's Evidence (`docs/handoffs/644-brief.md`, E1 to E37) and `handoff-A.md` (Phase 3, PASS, warn 5's test plan)
**Verdict:** PASS

## Summary

PASS: 0 `block` and 2 `warn` findings. F extended every object the map named and built no parallel path.

- **`Report`.** Both verbs print this one result type through `emit_outcome`. It gains `agent`, and `Report::new` takes it from the stored record (`launched.pane`), as it takes `spec_only`. The text clause goes through `list::text_value`, the line's one rule for stored strings.
- **`SpecField` and `FieldValue`.** This is the one comparison table: `profile show` iterates it and `diff_spec` walks it. It gains the 16th field. `Key(Some)` shares `Text`'s `write_escaped` arm, so there is no second escaper, and `differs` gains no arm.
- **The refusal.** An undefined key is still refused by the harness port's `verify_serving`, and the engine's own rollback undoes the run. `launch_request` and `relaunch_request` are unchanged: the CLI reads no file and adds no check.
- **Tests.** The tests extend the existing tests that Phase 3 warn 5 named. `launch/agent.rs` holds only new behavior. It reuses the rig's `pub(crate)` helpers, `crate::get::help` and `ServeSpec::with_agent`, so it does not copy `serve.rs`'s private `with_agent`. The diff does not touch `launch/rig.rs`, the test kit or `tx_launch`.

The two warns are not drift. One is small test constants and builders copied instead of shared, which the Blast radius made unavoidable. The other is a brief that was never amended with Phase 3's warns, although F and T applied them.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/pane_verbs/launch/agent.rs:24,27`; `crates/holler-pane/tests/profile_diff_test.rs:59` | Small test constants and builders are copied rather than shared. `STEP_PHRASE` (`"to reconcile, run"`) is now private in three sibling modules (`launch/serve.rs:32`, `launch/profiles.rs:17`, `launch/agent.rs:27`), and `AGENT` (`"feature-implementor"`) in two (`serve.rs:35`, `agent.rs:24`). `launch/rig.rs` already holds the matching `STEP` (`:90`) and `occurrences` (`:150`). The one-line `AgentKey` builder is now private in three `holler-pane` test files under three names: `ports_test.rs:563` `agent_key`, `profile_snapshot_test.rs:28` `agent`, `profile_diff_test.rs:59` `key`. Yet `tests/common/mod.rs` is the shared seam for typed builders (`pane_name`, `profile_name`). Each copy follows the pattern its module already used. The brief barred edits to `launch/rig.rs`, and `common/mod.rs` is outside the Blast radius, so this is not a block. | Follow-up, size XS: move `STEP_PHRASE` next to `STEP` in `launch/rig.rs`, add one `agent_key()` builder to `holler-pane/tests/common/mod.rs`, and fold the copies into them. |
| 2 | warn | `docs/adr/ADR-0003.md:93`; `docs/handoffs/644-brief.md` (Blast radius; Scope 3 line 20; Acceptance 4 line 41) | The brief is still the version from before Phase 3. Its Blast radius does not list ADR-0003, which F edited, and Scope 3 and Acceptance 4 still say "code and message unchanged". F and T applied Phase 3 warns 1 to 5 from `handoff-A.md` directly, so the code, the tests and the ADR text are right and the brief is stale. **The ADR-0003 edit is not a drive-by** (`parallel-agent-coexistence.md:102`): Phase 3 warn 1 asked for exactly this edit. `docs_rows.rs:18` assigns `pane launch` and `pane relaunch` to #644. The edit is one token, and #664's ADR-0003 hunk is at line 70, so the two do not overlap. | Before S, O folds Phase 3 warns 1 to 5 into the brief. In particular: add `docs/adr/ADR-0003.md` (line 93) to the Blast radius, and change "code and message unchanged" to "the code unchanged; the harness's message followed once by the reconcile step". Otherwise S audits a correct diff against stale text. |

### What was checked and cleared

**Production code: extend, not parallel.** `launch.rs` changes only the imports, the two doc comments, `Report` and its `text`, and `relaunch.rs` and `args.rs` change only doc and help text. No other CLI view prints the agent (`git grep opencode_agent crates/holler-cli/src`), so there is no second format to match. In `holler-pane`, the only users of `SpecField` and `FieldValue` outside `profile_diff.rs` are `profile/show.rs:138` (iterates `ALL`) and `:197` (builds `Text`). No other 15-field list goes stale: the two remaining `; 15]` arrays are conformance-case tables.

**Deliberate duplication, decided by the operator.**

- `data.agent` repeats `data.pane.opencode_agent`. This is operator decision 5. Both values come from the one stored record, so they cannot disagree.
- `Key(None)` prints `(default)`, while `Argv(None)` prints `none` and the CLI's empty value is `-`. This is decision 6 (ADR-0021 item 20), and the `Display` doc gives the reason: `none` and `-` are both valid keys, and `(default)` cannot be one.

**Tests: extended, not copied.**

- Four tests were extended in place:
  - `launch_with_agent_stores_the_key_in_the_record`;
  - both of #700's relaunch agent tests;
  - `launch_with_profile_adds_the_spec_and_bumps_once`;
  - `spec_field_values_serialize_like_the_spec`.
- **The refusal tables.** There is one table per verb, with all the rollback assertions in one helper (`assert_refused`). Only the launch row with the stand-in code overlaps #802's `a_verify_refusal_rolls_back_and_stops_the_server`, and Phase 3 accepted that as one row of a table.
- **The `--spec-only` test.** `spec_only_reports_no_agent` has the same setup as `spec_only_changes_the_profile_and_nothing_live` (`profiles.rs`), which runs only in text format. Every assertion the new test adds is new: `data.agent` is `null` while `--agent` was given.
- **The golden test.** It pins the whole result line. `launch_records_what_the_fakes_show` checks only parts of the line, and the brief asks for the golden line as a new requirement.
- **The `agent-mismatch` hook.** It inlines `kill`, `serve_command` and `start_server` because a hook receives `&Fakes` and not `&Rig`. `Rig::serve` therefore cannot be called from it, and it would start a server with no agent anyway.

**The runs going in parallel.**

- #664's `profile apply` builds its differences from `profile show`'s `PaneDiff` and has no agent comparison of its own. Its `a_dry_run_and_profile_show_agree` holds with the 16th field and without it.
- #664 changes `tests/profile_verbs/rig.rs` and `profile/show.rs` only by adding to them, and no signature of a helper the new `show.rs` test uses changes.
- #642 and #649 touch none of this diff's source or test files.

**The stack rules.**

- No `holler-proto` or golden-file change.
- The JSON key is additive, so `schema_version` stays 1 (ADR-0021's rule).
- The ADRs change in the same change as the code.
- No new persistence or concurrency.
- The agent key is a name, not a secret, and no personal infrastructure name appears in the added lines.
- File sizes: the production files are at most 457 lines, and the largest test file touched is `tests/pane_verbs/launch.rs` at 748 (under 800). `launch/rig.rs` (814) is not edited.
- Phase 7 candidates: no token-store operation, `Lockout`, `Roster`, `log` helper or harness helper (`Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`) is touched or copied.

## Notes for F

None. The verdict is PASS.

## Notes for O (non-blocking)

- **Warn 2, before S.** Fold Phase 3 warns 1 to 5 into the brief so that S audits against what was decided:
  - the ADR-0003 line in the Blast radius;
  - the `-` quoting;
  - "message followed once by the reconcile step";
  - a refused relaunch leaves the pane down with its record unchanged;
  - the table form of Acceptance 4 and 5.
- **Warn 1.** Open an XS follow-up issue, or fold it into the next story that edits `launch/rig.rs` or `holler-pane/tests/common/mod.rs`.
- **Unchanged from Phase 3.** The dead `(pid P)` clause in `Report::text` is still O's call (F kept it, as the brief decided). Phase 3 warn 6 (record decision 4 on #664's issue and decision 1 on #642's) is still O's or the MO's work.

## Patterns referenced

1. `crates/holler-cli/src/pane/launch.rs:345-457` and `crates/holler-cli/src/pane/list.rs:35`, `:280-295`: `Report`, `emit_outcome` and `failure_body`, and the one text rule (`text_value`, `NO_VALUE`).
2. `crates/holler-pane/src/profile_diff.rs:31-236` and `crates/holler-cli/src/profile/show.rs:134-142`: the one field table, `FieldValue`'s untagged `Serialize` and its `Display`, and how `profile show` consumes them.
3. `crates/holler-cli/tests/pane_verbs/launch/rig.rs:86-152`, `:276-290`, `:580-621` and `launch/serve.rs:31-48`, `:414-455`: the rig's shared helpers and hooks, and #802's verify-refusal test that the refusal table extends.
4. `crates/holler-pane/tests/common/mod.rs:87-105`, `ports_test.rs:563`, `profile_snapshot_test.rs:28`: the shared typed-builder seam and the existing private `AgentKey` builders.
5. `.aftersight/releases/claude/workflow/parallel-agent-coexistence.md:102`, `:176-178` and `crates/holler-cli/tests/pane_verbs/process/docs_rows.rs:18`: planned versus drive-by edits, and which story owns the ADR-0003 bullet.
