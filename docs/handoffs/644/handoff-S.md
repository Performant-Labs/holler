# Handoff-S: Phase 10 - #644 `holler pane launch` and `relaunch` (spec audit)

**Date:** 2026-10-10 01:05 MDT
**Branch:** issue-644-implementation (worktree `.claude/worktrees/0644-launch-relaunch`, head `e945791`; merge base with
`origin/main` is `cec1f82`; `origin/main` has since moved to `bd5e825`, #642 part 2)
**Issue:** #644 (epic #633), read live with `gh issue view 644` and its edit history (GraphQL `userContentEdits`)
**Brief:** `docs/handoffs/644-brief.md`, as amended through `95f07ed` / `40f486e`
**Handoffs reviewed:** `handoff-A.md` (plan re-review), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`,
`handoff-A-dup.md`, `decisions.md`, `evidence.md`, and the outside diff review `644-diff-result-r2.md` (PASS)

**Verdict:** ADVISORY-HOLD. F built what the brief asks for, and the build is sound. The brief is incomplete: the issue
gained an operator-confirmed "agent" amendment, and a dependency on #700 (still OPEN), five hours before the brief's last
amendments, and the brief never mentions it. One doc fix is also required before merge (see Verdict).

## A precondition

Met. `handoff-A.md` (the re-review of the amended brief) returned **PASS**: 0 blocks, 8 warns. `handoff-A-dup.md` returned
**PASS** on the diff: 0 blocks, 6 warns.

## T precondition

Met. `handoff-T-green.md` says "Blocking issues: None".

- **RED** (`handoff-T-red.md`): 3 passed and 54 failed. All 54 failed on an assertion about the missing behaviour, and
  none on a compile or setup error. The 3 that passed were the two kept #670 flag tests and the rig self-check.
- **GREEN** (`handoff-T-green.md`, at `5cf6d94`):
  - 57 of 57 launch/relaunch tests pass, in 20 of 20 repeated runs;
  - `pane_verbs` passes 187;
  - the workspace passes 1676, with 0 failed and 14 ignored.
- **Mutation spot-check:** 6 mutations, and each one failed its pinning tests.
- **Since T-green:** two commits only.
  - `c844f06` merges `origin/main` (#640, #713, #715). Its only conflict resolutions are in `docs/adr/ADR-0021.md`.
  - `e945791` is A-dup.

  This branch's production files are unchanged since `5cf6d94`. In its test paths only main's own `output_api.rs` and
  `process/stub.rs` changed. T has not re-run the merged tree, so CI must pass on it (Verdict, required change 2).

## Acceptance criteria

Tests are in `crates/holler-cli/tests/pane_verbs/` unless a row says otherwise. "S" marks a read-only check I ran myself.

**The issue's own criteria** (the source of truth):

| Issue criterion | Proving test or evidence | Status |
|---|---|---|
| Happy path | `launch.rs::launch_records_what_the_fakes_show` (the whole record as one `Pane` comparison, exact call logs) | Met |
| A wedged server aborts before any order is possible | `launch_onto_a_frozen_server_times_out_before_any_session`, `launch_aborts_when_the_server_never_gets_healthy` (no `CreateSession`, no `AttachTui`) | Met |
| A vanished Herdr pane | `launch_fails_when_its_herdr_pane_vanishes`; `relaunch.rs::relaunch_recreates_a_vanished_pane_at_its_cell` | Met |
| A stale generation | `relaunch_fails_on_a_stale_generation`, `launch_record_conflict_fails_loudly` | Met |
| A crash between steps (the next `doctor` finds and reports it) | `a_crash_mid_launch_leaves_no_record`: no record, no `CasPut`, and the Herdr pane and tmux session are left. The merged doctor's whole-fleet pass reports a Herdr pane no record names: `doctor.rs::unregistered_herdr_pane_is_reported` (#647) | Met in part, by those two tests together. A leftover server or tmux session is reported by no doctor pass. ADR-0021 section 8's new note says so (advisory 3). |
| After any outcome the registry equals what the fakes observe | `launch/rig.rs::Rig::assert_matches`, at the end of each verb-run case except AC 6, 7 and 18b (the brief's exceptions) | Met (advisory 5: AC 17 and AC 24 do not call it) |
| The record carries model, effort, env names and ceilings | `launch_records_model_effort_env_and_ceilings` | Met |
| `relaunch` without `--grid` keeps the position | `relaunch_without_grid_keeps_the_position` | Met |
| `--spec-only` changes P and nothing live | `profiles.rs::spec_only_changes_the_profile_and_nothing_live` (pane-store span exactly `[Get]`), `relaunch_spec_only_changes_the_profile_and_nothing_live` | Met |
| A profile conflict after the act fails with `profile-conflict` and prints the reconcile step | `a_profile_conflict_after_the_act_fails_loudly`, `the_reconcile_step_quotes_the_profile`, and `a_step_the_real_scope_printed_is_not_repeated` (over #663's real `StoreScope`) | Met |
| A failing probe refuses with `probe-failed` before any step; a passing one is recorded | `guards.rs::a_failing_probe_refuses_before_any_step`, `an_unscripted_probe_refuses`, `a_passing_probe_is_recorded` | Met |
| A JSON command string (`--command-json '"opencode serve"'`) is refused with `command-not-argv`; the fake host records an argv, never a shell line | `a_command_string_is_command_not_argv`, `the_command_reaches_the_host_as_argv` (`["prog", "a b", "$(id);x"]` verbatim) | Met |
| `launch --profile P` adds the spec and bumps P once; `relaunch --profile P --model X` updates it; a failed launch leaves P unchanged; a missing P refuses; no profile changes without `--profile`; JSON passes the envelope helper; exit codes are equal across formats | `launch_with_profile_adds_the_spec_and_bumps_once` (change log `[1, 2]`); `relaunch_with_profile_and_model_updates_the_spec`; `a_failed_act_restores_the_profile_specs` (specs equal, generation +2) and the 16d half of `a_failing_probe_refuses_before_any_step` (generation equal); `a_missing_profile_is_refused`; AC 1's test (no profile call); `check_envelope` in every JSON case; `exit_codes_equal_across_formats` | Met, with "unchanged" read as the brief's C-3 reads it (specs equal after a failed act, generation equal after a refusal) |
| `--grid c1r2`, `r2c1` and `2,1` reach Herdr as row 2, col 1; an ambiguous value is refused before any step; output prints `r2c1` | `every_grid_form_reaches_herdr_as_row_2_col_1` (text and JSON), `an_ambiguous_grid_is_refused_before_any_step` | Met |
| **Scope, amended 2026-10-09 17:38-17:40 MDT, "agent", operator-confirmed (epic decision 8):** `launch`/`relaunch` use `--agent KEY`, refuse a key the pane's project does not define ("the brief names the code"), record `Pane.opencode_agent` in the same CAS write, and start the server with it; the issue now depends on #700 | **None.** The brief, the ACs and the code do not have it. `Pane.opencode_agent`, `AgentKey` and `--agent` do not exist: S ran `grep -rn 'opencode_agent\|AgentKey' crates` and found no match | **Not met** (a brief defect; see Verdict) |
| Scope: "with an operation id for long work" | Decision 1, recorded as **PROPOSED (#644)** in ADR-0021 section 12 (line 546); the "Deferred" bullet (line 613) stays | Deferred on purpose, pending the operator's answer (advisory 2) |
| Scope: "observe and confirm SHOWN equals DRIVEN" | O1 checks SHOWN against `session_of_record` through `reconcile::shown_differs` (C-14, decision 22). `driven` is never inferred: AC 1 and AC 20 | Met, as the brief resolves it |

**The brief's ACs 1-31:**

| AC | Proving test or evidence | Status |
|---|---|---|
| 1 | `launch_records_what_the_fakes_show` | Met |
| 2 | `Rig::assert_matches` (both branches: record and no record) | Met |
| 3 | `launch_records_model_effort_env_and_ceilings` | Met |
| 4a, 4b | `launch_onto_a_frozen_server_times_out_before_any_session`, `launch_aborts_when_the_server_never_gets_healthy` | Met |
| 5a, 5b | `launch_fails_when_its_herdr_pane_vanishes` (the message names the id), `relaunch_recreates_a_vanished_pane_at_its_cell` (no `Close`) | Met |
| 6a, 6b | `relaunch_fails_on_a_stale_generation`, `launch_record_conflict_fails_loudly` (the message ends with `; to reconcile, run holler pane doctor`; live state left) | Met |
| 7 | `a_crash_mid_launch_leaves_no_record` (`catch_unwind`, no panic hook touched) | Met |
| 8 | `a_failed_attach_rolls_back` (the step once, `Close`, tmux session stays with `ps` empty, server `Killed`) | Met |
| 9a-9d | `launch_of_a_recorded_name_is_pane_exists`, `launch_refuses_a_cell_another_record_holds` (also with the stored grid stale), `launch_ignores_a_stale_record_at_a_free_cell`, `launch_never_adopts_an_unrecorded_pane`, `launch_never_adopts_a_running_server` | Met |
| 10a-10c | the three probe tests (`ProbeCall` with `PROBE_TIMEOUT`) | Met |
| 11a, 11b | `a_command_string_is_command_not_argv`, `the_command_reaches_the_host_as_argv` | Met |
| 12 | `an_env_value_is_refused_and_not_echoed` (both formats, both streams) | Met |
| 13a-13c | the three grid tests | Met |
| 14 | `an_unsupported_herdr_is_refused` (Herdr log `[Version]`) | Met |
| 15 | `launch_names_every_missing_flag`, `bad_policies_and_ceilings_are_usage`, `a_live_launch_needs_a_herdr_session`, `port_policy_round_trips_with_the_snapshot` | Met |
| 16a-16k | `profiles.rs` (16a, c, e, g, h, j, k); `relaunch.rs` (16b, the relaunch half of 16g, 16i); 16d is in `a_failing_probe_refuses_before_any_step`, and 16f is in AC 1's test | Met |
| 17 | `exit_codes_equal_across_formats` (9 cases; text stderr is exactly `error: ` + the JSON message + `\n`) | Met |
| 18, 18b | `relaunch_without_grid_keeps_the_position`, `a_failed_relaunch_never_closes_the_records_pane` | Met |
| 19, 19b, 19c | `relaunch_with_grid_moves_the_pane`, `relaunch_refuses_a_move_without_grid_and_a_new_directory`, `relaunch_engine_enforces_its_rules`, `launch_engine_refuses_a_spec_for_another_pane_and_spec_only_without_a_profile`, `relaunch_records_the_move_before_closing_the_old_pane` | Met |
| 20 | `relaunch_keeps_the_session_of_record`, `relaunch_replaces_a_deleted_session`, `relaunch_keeps_the_stored_driven` | Met |
| 21, 22, 23 | `relaunch_leaves_other_panes_alone`, `relaunch_fails_when_the_old_server_survives`, `relaunch_of_a_missing_pane_is_refused` | Met |
| 24 | `the_budget_bounds_a_slow_launch` (`Timeout { op: "pane.launch" }`, `acted: true`; harness log `[Health, Serve]`; Herdr log `[Version, Snapshot, EnsurePane, Close]`) | Met (advisory 5) |
| 25 | S: both greps over the three production files print nothing | Met |
| 26 | T-green: `cli_surface_test`, `docs_cli_test` and `pane_cli_process` pass. S: the stub grep prints nothing, and `grep -c '^    // #644$'` prints `1` | Met |
| 27 | S: the four greps print lines 546, 345, 405 and 613. The ADR diff touches only the end of section 8, two cells of section 9, section 12 and the mismatch bullet; sections 3 and 11 and the #647/#662 bullets are untouched | Met (the merge of #640 left a contradiction: Verdict, required change 1) |
| 28 | S: the entry sits under `## [Unreleased]` / `### Enhancements` and links #644; `changelog-check: ok` | Met |
| 29 | S: over the merge base (`origin/main...HEAD`) there is no `+...unsafe` and no manifest or lock change. The two-dot form now lists main's own changes (F's Known issue 6) | Met |
| 30 | T-green: rustfmt on the 8 files, clippy `-D warnings` (which denies `too_many_lines` at 100 and `cognitive_complexity` at 15), the three test runs and `lint.sh`. S: sizes are 786, 437 and 88 lines (production) and at most 711 (tests); no `unwrap`, `expect`, `panic!` or `#[allow]` in the production files | Met |
| 31 | S: I grepped the added lines (code, tests, ADRs, CHANGELOG and handoffs) for hosts, tailnet and account names, IPs, home paths and keys, and found nothing. `gitleaks git --log-opts=origin/main..HEAD` found no leaks in 14 commits. The PR text does not exist yet | Met for the diff |

The tests assert behaviour: records, call logs, envelopes and exit codes. RED shows that each one fails without the
feature, and T-green's six mutations show that the key rules are pinned.

## Spec compliance

I checked brief decisions 1-26 against `tx_launch.rs`, `launch.rs` and `relaunch.rs`. Each is built as stated. The
changes are documented in `handoff-F.md` and are within the brief's latitude:

- **The engine (decisions 2, 11, 12, 16, 17, 22, 23).**
  - The engine works only through the ports. Its only other call is `Instant` for the budget.
  - Step order:
    - launch: 0, then `spec_only`, then 1-6, then the act inside `edit_spec`;
    - relaunch: E0's spec check, then `spec_only`, then E0's cwd and cell rules, then 3-6.
  - The budget is checked before every live step, R included.
  - A rollback stops the session's processes and closes only a pane the plan's snapshot did not list. A record conflict
    is not rolled back.
  - B10 runs after R. Its error is captured, and the run exits 1 with `acted: true`.
  - `driven`: launch writes `None`, and relaunch keeps the stored value.
- **The verbs (decisions 4, 10, 15).**
  - `--herdr-session` is required unless `--spec-only`.
  - `effective_spec` gives one `usage` line that names every missing or bad flag. `--model` is split at its first `/`.
  - `stored_profile` passes on the stored name, so the reconcile step's exact-substring test holds for `--profile Demo` too.
  - `failure_body` appends `profile_scope::reconcile_step` only when `acted` is true and the message lacks the step.
- **The ADR and codes (decisions 1, 5, 14, 19, 20).**
  - `port_of_policy` accepts `fixed:<port>` only, in canonical decimal.
  - The three open codes are declared in `tx_launch.rs`.
  - The mismatch code is `unavailable`.
  - The ADR-0021 edits are as decision 20 lists them. The `unavailable` reason cell is also amended (A warn 7d).
- **F's documented changes:**
  - the port policy is parsed after step 2 or E0, before the probe;
  - step 5 reads `list()` only to word a refusal;
  - B8 calls `list_sessions` only when the record has a session of record;
  - the rollback note names each failed call;
  - E0 says "the spec gives".
- **T's documented changes:** `Box<SpecFlags>`, which `clippy::large_enum_variant` and the frozen `cli.rs` require; the
  positional field named `pane` (A warn 8); and relaunch cases that start from `seed_live`, which a self-check pins.

**Against the issue:** the one gap is the "agent" amendment (Verdict). The operation id is deferred on purpose, by a
reviewed and ADR-recorded decision that awaits the operator.

## Quality audit

- **Correctness and failure handling.**
  - I traced every path: steps 0-6, A1-O2, R, B1-B10, the rollback, both scopes, and the budget. I found no defect.
  - A store failure is returned before anything live happens.
  - A `list()` failure while wording `grid-occupied` still refuses the run (fail closed).
  - No write is lost:
    - the record is written by a CAS at generation 0 or the read generation;
    - P is written first by a CAS;
    - P's restore is a CAS, and a conflict there is `profile-conflict`.
  - Decision 24's lost-update window on P is an accepted risk with a follow-up.
- **Build guards.**
  - No `unwrap`, `expect`, `panic!` or `#[allow]` in the production files, and none added anywhere in `crates/`.
  - No file reaches 900 lines. `tx_launch.rs` is at 786, under the brief's size fallback (A-dup warn 6).
  - No dead code: `dead_code` is denied and clippy is clean.
- **Protocol.** None changed: no wire field and no golden file. The section 9 table gains the three open codes. The new
  `op` value does contradict a cell that #640 merged (Verdict, required change 1).
- **Tests.**
  - In-process over the fakes, with no cross-process behaviour before #649.
  - No sleep is used to synchronize. AC 24 uses a fake's delay and asserts an invariant with a generous bound.
  - The RED-first evidence is in T-red.
- **Documentation.**
  - The CHANGELOG entry is accurate, including that the real binary answers `not-implemented`: S checked that `Unwired`
    answers `NotImplemented` (`pane/wiring.rs`).
  - ADR 0003 rows 48-49 are updated, and so is ADR-0021.
  - No README or `docs/` page lists the pane verbs; the same is true for #647 and #646a.
- **Public-repository privacy.** Clean (AC 31). The test data is neutral: `demo-c1r1`/`demo-c2r1`, `scratch`, `main`,
  `/srv/demo`, ports 48100-48102, `p1/m1`, and the fake secret `s3cr3t644`.
- **Commit and PR hygiene.**
  - The subjects are Conventional, with the pipeline's `chore(#644): <phase>` and `docs(handoffs)` forms.
  - One merge commit, `87e90a3`, has no `Co-Authored-By`. Like `main`'s recent squash commits, none carries a session link.
  - The PR does not exist yet. Its body needs the AI disclosure (`CONTRIBUTING.md`). The script writes `Closes #644.`,
    which must not stand while the hold is open.

## Scope check

- **Delivered:** the brief's whole scope.
- **Outside the issue's blast radius:** the verb story's own surfaces (epic ruling 2). These are the fixture lines, the ADR
  0003 rows, the `stub.rs` entries, ADR-0021, the CHANGELOG and the `tests/pane_verbs/launch/{rig,guards,profiles}.rs`
  submodules. No frozen file and no manifest is touched (S checked the brief's "Not touched" list with `git diff`).
- **Over-delivery:** none. The `unavailable` cell edit and the shared `stored_profile`/`spec_of` helpers are what A asked
  for, or are needed to share code between the two verbs.
- **Under-delivery against the issue:** the `--agent` amendment (a brief defect), and the operation id (PROPOSED, pending
  the operator).

## Verdict

**ADVISORY-HOLD.**

**The defect.** The brief leaves out the operator-confirmed "agent" amendment to #644 and the issue's new dependency on
#700. Times are in MDT on 2026-10-09 unless noted:

| Time | Event |
|---|---|
| 16:44 | The brief is written (`fc65543`). |
| 17:21 | The brief is amended after the plan review (`7195993`). |
| 17:38 | The agent paragraph is added to #644's Scope (issue edit). |
| 17:39 | #700 is created. It is still OPEN, with no PR. |
| 17:40 | #644's "Depends on" gains #700. |
| 22:44, 22:59, 23:06 | The brief is amended three more times (`d92bf0a`, `2c35ae5`, `95f07ed`). None of these adds the agent part, and no outside review round saw it, because the reviewer sees only the brief. |
| 23:45 | T-red. |
| 2026-10-10 00:15 | F. F flags the gap as Known issue 4. |

**The rules it breaks:**

- CLAUDE.md: "The GitHub issue is the source of truth".
- Epic #633: the `opencode_agent` field, its `AgentKey` guard and the `--agent` flag are "#700 only; it merges before
  #642, #644 and #647 start". The epic's wave-3 table also says "#642, #644 and #647 also need #700".

**Why this is a hold and not REWORK.** F cannot build the agent part from this brief:

1. The field, the guard and the flag do not exist. They are #700's alone, in files that are frozen for this story
   (`holler-pane/**`, `cli.rs`).
2. "Refuse a key the pane's project does not define" needs some way to read a project's agents, and no frozen port has
   one. That is a contract decision.
3. The brief names no refusal code and has no AC for it.

**Why this is not a PASS.** The script writes the PR body as `Closes #644.` (`coding-pipeline.workflow.mjs:4097`), and
the run's agent merges its own PR. A PASS would therefore close #644 with an operator-confirmed requirement silently
dropped. #647 had the same defect, and that hold was settled by merging "#647, part 1" with #647 kept open (`e612878`).

**Proposed fix (O or the MO picks one):**

- **(a) Split in place.** This follows the #647, #646 and #642 multi-part precedent.
  - Land this branch as #644 part 1. The PR body says "Part of #644", not "Closes #644", and #644 stays open for the
    agent part, to be done after #700 merges.
  - Journal the split, and add it to the brief's Out of scope and Follow-ups.
  - Add one ADR-0021 "Deferred to named stories" bullet, as #642 did for applying the agent: for example, "Using
    `--agent` on launch and relaunch, refusing a key the pane's project does not define, and recording
    `Pane.opencode_agent` in the record write: #644's last part, after #700."
  - No code change is needed. Because this changes the scope of an operator-confirmed amendment, it may need the
    operator's OK.
- **(b) Hold #644 until #700 merges.**
  - Amend the brief: the flag on both verbs, the refusal code and where a project's agents are read from (probably an
    amend-first port decision), the field in the record write, and ACs for each.
  - Then re-run T-red, F, T-green, A-dup and S.

**Required before merge on either path** (doc-only; F on a rework, or the MO):

1. **ADR-0021 now contradicts itself on a `timeout`'s `op`.**
   - The section 9 reason cell (line 462) came in with #640 at `c844f06`. It says `op` "names the port method that ran
     out, as `<port>.<method>` ... in every implementation and fake".
   - This change's section 12 paragraph (line 551) and the binding `OP_LAUNCH`/`OP_RELAUNCH` (`tx_launch.rs:64-66`,
     pinned by AC 24) use `pane.launch` and `pane.relaunch`.
   - Fix: add the exception to the line 462 cell, for example "...; a verb's own run budget names the verb instead:
     `pane.launch`, `pane.relaunch` (#644)". This is A-dup warn 1.
   - The same claim in `PaneError::Timeout`'s doc (`crates/holler-pane/src/error.rs:457`, frozen here) is a one-line
     follow-up for that file's owner.
2. **Merge `origin/main`, now `bd5e825`, and run CI on the merged tree.**
   - A trial `git merge-tree` shows one conflict, in ADR-0021's "Deferred to named stories": keep this branch's decided
     mismatch bullet and main's three bullets. `CHANGELOG.md` merges cleanly.
   - T-green's runs were at `5cf6d94`, before `c844f06`, so the tree that merges must be green in CI.
3. **The PR body:** the AI disclosure (`CONTRIBUTING.md`), and "Part of #644" under (a).

## Advisory notes (non-blocking)

1. **ADR-0021 line 505** still marks the recording of `host.herdr_api_version` as "**PROPOSED**: #644's launch and
   relaunch record it". This change records it (`tx_launch.rs:332`, pinned by AC 1). Mark it decided, with the
   operator's OK, since the ADR's PROPOSED marks wait for the operator.
2. **Decision 1 (no operation id)** is still PROPOSED. Ask the operator in the same message as the hold question.
3. **AC 7 and the issue's "the next doctor finds and reports it."**
   - Doctor has merged since the brief's C-10 called it a stub. AC 7's test could run `pane doctor` after the crash and
     assert an `unregistered-herdr-pane` finding for the left pane, which would make the composition a direct proof.
   - A leftover server or tmux session is seen by no doctor pass, and a crashed launch's port stays in use until #695.
     This is A-plan warn 3, a follow-up for #647, #663 and #695.
4. **The rollback-failure note is untested** (T-green advisory 3). There is also a quiet edge in `with_note`
   (`tx_launch.rs:775-786`): an error variant with no payload (`generation-conflict`, `not-implemented`,
   `command-not-argv`, ...) keeps its code and drops the note. There is no way to add text to those variants. Record the
   edge in the brief, and add one cheap test: `fail_next(AttachTui, ..)` plus `fail_next(Close, Unavailable)`, asserting
   `; rollback failed: herdr.close (unavailable)`.
5. **Two gaps in the AC 24 and AC 17 tests.**
   - AC 24's test does not assert that the rollback stopped the server (`StopOwned` in the host log, or the server not
     `Running`).
   - Neither the AC 17 test nor the AC 24 test calls `assert_matches`.
   - AC 24's margins are about 100 ms each way (T-green advisory 2). If CI flakes, widen the delay and the budget
     together.
6. **The "never adopt" rule has a window.** A Herdr pane placed at the target cell between step 5's snapshot and A1 is
   answered by `ensure_pane` (an occupied cell answers its pane). It would then be recorded, and on a rollback closed.
   Closing the window needs a create-only `ensure_pane`, a contract item for #640 and #649.
7. **No check across records for the port.** Step 6 refuses only a port that a live server answers on. A record whose
   server is down still names its port, so a second launch can claim it, and the first pane's relaunch then fails at
   B2. A follow-up with #664 and #647 could refuse a port that another record names.
8. **Follow-ups to file:**
   - A-dup warns 2-5 (consolidation: `with_note`/`with_context`, `spec_of`/`stored_profile`, the rig pieces, the Herdr
     join key);
   - every follow-up in the brief except "the step names the pane";
   - A-plan warns 3, 5 and 6 (`handoff-F.md`, Known issues and follow-ups).
9. **The gate override's authority is not journalled.** The diff gate's prompt-ceiling override (the `decisions.md`
   entry timed 01:05 MDT) cites no operator OK, while the brief-gate override does ("Override and resume"). Journal
   who authorized it.
10. **Forward-compat, good news.** Main's #642 part 2 makes `attach_tui` return only once the TUI's title shows the
    session (`crates/holler-adapter-opencode/src/attach.rs`, "trusted only once seen"). So O1, right after A7 or B9,
    should see `Some(sid)` on the real adapter, which resolves brief Risk 10 favourably.
