# Handoff-S: Phase 10 - #644 `holler pane launch` and `relaunch` (spec audit, re-run after the split)

**Date:** 2026-10-10 01:26 MDT
**Branch:** issue-644-implementation (worktree `.claude/worktrees/0644-launch-relaunch`, head `c76aeaf`). After a fetch at
this audit, the merge base with `origin/main` is `bd5e825`, which is `origin/main`'s tip.
**Issue:** #644 (epic #633), read live with `gh issue view 644`. It is OPEN and was last edited 2026-10-09 17:40 MDT (the
agent amendment). #700 is OPEN with no PR.
**Brief:** `docs/handoffs/644-brief.md`, including the scope-split note of 2026-10-10 (line 10, added in `c76aeaf`)
**Handoffs reviewed:** `handoff-A.md` (PASS), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`
(PASS), `evidence.md`, `decisions.md` (through O's split entry), the outside diff review `644-diff-result-r2.md` (PASS), and
the previous S handoff (ADVISORY-HOLD, `90ba971`). This handoff replaces that one; the hold stays in git history.

**Verdict: PASS.** The hold's defect is resolved: #644 is split along #700, so this run is `Part of #644`, and the
agent part is #644's last part, after #700. The hold's two required doc fixes are done. Two conditions remain for the
merging agent, and neither is F's work: the PR body (`Part of #644.` instead of the script's `Closes #644.`, plus the AI
disclosure), and green CI on the merged tree. See the Verdict section.

## A precondition

Met. `handoff-A.md` (the re-review of the amended brief) is **PASS**, with 0 blocks and 8 warns. `handoff-A-dup.md` is
**PASS** on the diff, with 0 blocks and 6 warns.

## T precondition

Met. `handoff-T-green.md` says "Blocking issues: None".

- **RED** (`handoff-T-red.md`): 3 passed and 54 failed. Every failure is an assertion about the missing behaviour; none
  is a compile or setup error. The 3 that passed are the two kept #670 flag tests and the rig self-check.
- **GREEN** (`handoff-T-green.md`, at `5cf6d94`): 57 of 57 launch/relaunch tests pass, and 20 of 20 repeated runs. The
  `pane_verbs` target passes 187. The workspace passes 1676, with 0 failed and 14 ignored. Six mutations each fail their
  pinning tests.
- **Since T-green:** the production files are byte-identical to `5cf6d94`. Two merges of `origin/main` followed:
  - `c844f06` brought #640, #713 and #715;
  - `c76aeaf` brought #642 part 2, O's split note, and the two ADR-0021 edits.

  I checked what those merges change under this branch's crates (read-only):
  - `holler-pane`'s `error.rs`, `lib.rs`, `pane.rs`, `ports.rs` and `reconcile.rs` changed only in comment and blank
    lines;
  - main's two changed `holler-cli` test files (`output_api.rs`, `process/stub.rs`) do not run `launch` or `relaunch`.
    The stub-table test now picks `pane switch`, as F predicted.
  - `docs_cli_test` skips `docs/handoffs/`, and O's ADR edits add no inline `holler ...` span.

  The merged tree is therefore unlikely to break. Nobody has run it yet, so CI must (Verdict, condition 2).

## Acceptance criteria

Tests are in `crates/holler-cli/tests/pane_verbs/` unless a row says otherwise. "S" marks a read-only check I ran.

**The issue's Acceptance list** (the source of truth):

| Issue criterion | Proving test or evidence | Status |
|---|---|---|
| Happy path | `launch.rs::launch_records_what_the_fakes_show`: the whole record compared as one `Pane`, one session (no ping session), and the exact harness and Herdr call logs | Met |
| A wedged server aborts before any order is possible | `launch_onto_a_frozen_server_times_out_before_any_session` (`timeout`), `launch_aborts_when_the_server_never_gets_healthy` (`unavailable`). Neither log holds a `CreateSession` or an `AttachTui` | Met |
| A vanished Herdr pane | `launch_fails_when_its_herdr_pane_vanishes` (O2; the message names the id); `relaunch.rs::relaunch_recreates_a_vanished_pane_at_its_cell` | Met |
| A stale generation | `relaunch_fails_on_a_stale_generation`, `launch_record_conflict_fails_loudly` | Met |
| A crash between steps (the next `doctor` finds and reports it) | `a_crash_mid_launch_leaves_no_record` shows no record, no `CasPut`, and the Herdr pane and tmux session left. The merged doctor's whole-fleet pass reports such a pane (`doctor.rs::unregistered_herdr_pane_is_reported`, #647) | Met for this story's half, which the brief scopes (C-10). The doctor gap is advisory 6 |
| After any outcome the registry equals what the fakes observe | `launch/rig.rs::Rig::assert_matches`, which checks both the record and the no-record case, at the end of every verb-run case except AC 6, 7 and 18b (the brief's exceptions) | Met (advisory 5) |
| The record carries model, effort, env names and ceilings | `launch_records_model_effort_env_and_ceilings` | Met |
| `relaunch` without `--grid` keeps the position | `relaunch_without_grid_keeps_the_position` | Met |
| `--spec-only` changes P and nothing live | `profiles.rs::spec_only_changes_the_profile_and_nothing_live` (pane-store span exactly `[Get]`), `relaunch_spec_only_changes_the_profile_and_nothing_live` | Met |
| A profile conflict after the act fails with `profile-conflict` and prints the reconcile step | `a_profile_conflict_after_the_act_fails_loudly`, `the_reconcile_step_quotes_the_profile`, and `a_step_the_real_scope_printed_is_not_repeated`, which runs over #663's real `StoreScope` | Met |
| A failing probe refuses with `probe-failed` before any step; a passing one is recorded | `guards.rs::a_failing_probe_refuses_before_any_step` (no Herdr, host or harness call), `an_unscripted_probe_refuses`, `a_passing_probe_is_recorded` (`probe.last: Some(Ok)`, `PROBE_TIMEOUT`) | Met |
| `--command-json '"opencode serve"'` is refused with `command-not-argv`; the fake host records argv, never a shell line | `a_command_string_is_command_not_argv`; `the_command_reaches_the_host_as_argv` (`["prog", "a b", "$(id);x"]` verbatim) | Met |
| `launch --profile P` bumps P once; `relaunch --profile P --model X` updates it; a failed launch leaves P unchanged; a missing P refuses; no profile change without `--profile`; JSON passes the envelope helper; exit codes are equal across formats | `launch_with_profile_adds_the_spec_and_bumps_once` (log `[1, 2]`); `relaunch_with_profile_and_model_updates_the_spec`; `a_failed_act_restores_the_profile_specs`, and the 16d half of `a_failing_probe_refuses_before_any_step`; `a_missing_profile_is_refused`; AC 1's test (no profile call); `check_envelope` in every JSON case; `exit_codes_equal_across_formats` | Met. "Unchanged" is read as the brief's C-3 and ADR-0021 "Decisions taken" item 1 read it: after a failed act the specs are equal; after a refusal the generation is equal too |
| `--grid c1r2`, `r2c1` and `2,1` reach Herdr as row 2, col 1; an ambiguous value is refused before any step; output prints `r2c1` | `every_grid_form_reaches_herdr_as_row_2_col_1` (text and JSON), `an_ambiguous_grid_is_refused_before_any_step` | Met |

**The issue's Scope beyond its Acceptance list:**

| Scope item | Status |
|---|---|
| The agent amendment (2026-10-09, operator-confirmed): `--agent KEY`, refusing an undefined key, `Pane.opencode_agent` in the record write | **Deferred by the split** to #644's last part, after #700. It is recorded in the brief (line 10), in ADR-0021 "Deferred to named stories" (line 644), and in `decisions.md`; #644 stays open. S checked that #700 is still OPEN and that `Pane.opencode_agent`, `AgentKey` and `--agent` do not exist on `main`, so nothing could have been built against them. The same split was made for #642 and #647 along the same dependency |
| "with an operation id for long work" | Deferred: decision 1 is **PROPOSED** in ADR-0021 section 12 (line 566), and the "Deferred" bullet (line 633) stays. ADR-0021 itself requires a contract amendment first. Awaits the operator (advisory 4) |
| "observe and confirm SHOWN equals DRIVEN" | Met as the brief resolves it (C-14, decision 22): O1 checks SHOWN against `session_of_record` through `reconcile::shown_differs`, and `driven` is never inferred (AC 1, AC 20). The order is invariant I3's ("plan, act, observe, record", ADR-0021 line 203): it observes before the record write, which the issue's looser wording does not override |

**The brief's ACs 1-31:**

| AC | Proving test or evidence | Status |
|---|---|---|
| 1, 2, 3 | `launch_records_what_the_fakes_show`; `Rig::assert_matches`; `launch_records_model_effort_env_and_ceilings` | Met |
| 4a, 4b, 5a, 5b | the two frozen-server tests; `launch_fails_when_its_herdr_pane_vanishes`, `relaunch_recreates_a_vanished_pane_at_its_cell` (no `Close`) | Met |
| 6a, 6b | `relaunch_fails_on_a_stale_generation`, `launch_record_conflict_fails_loudly`. The message ends with `; to reconcile, run holler pane doctor`, the other writer's record stands, and the live state is left | Met |
| 7 | `a_crash_mid_launch_leaves_no_record` (`catch_unwind`; no panic hook touched) | Met |
| 8 | `a_failed_attach_rolls_back`: the step once, `Close`, the snapshot empty, the tmux session left with `ps` empty, and the server `Killed` | Met |
| 9a-9d | `launch_of_a_recorded_name_is_pane_exists`, `launch_refuses_a_cell_another_record_holds` (also with a stale stored grid), `launch_ignores_a_stale_record_at_a_free_cell`, `launch_never_adopts_an_unrecorded_pane`, `launch_never_adopts_a_running_server` | Met |
| 10a-10c, 11a, 11b, 12 | the three probe tests; the two argv tests; `an_env_value_is_refused_and_not_echoed` (both formats, both streams) | Met |
| 13a-13c, 14 | the three grid tests (13c from `ensure_pane`); `an_unsupported_herdr_is_refused` (Herdr log `[Version]`) | Met |
| 15 | `launch_names_every_missing_flag`, `bad_policies_and_ceilings_are_usage`, `a_live_launch_needs_a_herdr_session`, `port_policy_round_trips_with_the_snapshot` | Met |
| 16a-16k | `profiles.rs` (16a, c, e, g, h, j, k); `relaunch.rs` (16b, relaunch half of 16g, 16i); 16d inside 10a's test, 16f inside AC 1's | Met |
| 17 | `exit_codes_equal_across_formats`: nine cases, and text stderr is exactly `error: ` + the JSON message + `\n` | Met |
| 18, 18b, 19, 19b, 19c | `relaunch_without_grid_keeps_the_position`, `a_failed_relaunch_never_closes_the_records_pane`, `relaunch_with_grid_moves_the_pane`, `relaunch_refuses_a_move_without_grid_and_a_new_directory`, `relaunch_engine_enforces_its_rules`, `launch_engine_refuses_a_spec_for_another_pane_and_spec_only_without_a_profile`, `relaunch_records_the_move_before_closing_the_old_pane` | Met |
| 20-23 | `relaunch_keeps_the_session_of_record`, `relaunch_replaces_a_deleted_session`, `relaunch_keeps_the_stored_driven`, `relaunch_leaves_other_panes_alone`, `relaunch_fails_when_the_old_server_survives`, `relaunch_of_a_missing_pane_is_refused` | Met |
| 24 | `the_budget_bounds_a_slow_launch`: `Timeout { op: "pane.launch" }` with `acted: true`, harness log `[Health, Serve]`, Herdr log `[Version, Snapshot, EnsurePane, Close]`, the snapshot empty | Met (advisory 5) |
| 25 | S: both greps over the three production files print nothing | Met |
| 26 | T-green: the surface, docs and process tests pass. S: the stub grep prints nothing, and `grep -c '^    // #644$'` prints `1` | Met |
| 27 | S: the four greps print lines 566, 365, 425 and 633. The three-dot diff's hunks are at the end of section 8, the launch row and the `timeout`/`unavailable` cells of section 9, section 12's paragraph, and the "Deferred" list, where it changes only the mismatch bullet and adds the agent bullet. It touches nothing in sections 3 or 11, and no other bullet | Met |
| 28 | S: the entry sits under `## [Unreleased]` / `### Enhancements` and links #644; `changelog-check: ok` | Met (advisory 3) |
| 29 | S: no `+... unsafe` and no manifest or lock change. The two-dot and three-dot forms now agree, since the merge base is `origin/main` | Met |
| 30 | T-green: rustfmt, clippy `-D warnings`, the three test runs and `lint.sh`. S: sizes are 786, 437 and 88 lines (production) and at most 711 (tests); no `unwrap`, `expect`, `panic!`, `todo!` or `#[allow]` in the production files; no `#[allow]` added anywhere in `crates/` | Met |
| 31 | S: a scan of all 7,809 added lines found no machine or tailnet names (I used the playbook's banned list), no home paths, no account names or emails, no IP other than loopback, no keys and no private domains. `qwen38` is already used 30 times on `main`. `gitleaks git --log-opts=origin/main..HEAD` scanned 15 commits and found no leaks. The PR text does not exist yet | Met for the diff |

The tests assert behaviour: records, call logs over the run's own span, envelopes, exit codes and messages. RED shows
that each one fails without the feature, and T-green's mutations show that the key rules are pinned.

## Spec compliance

I traced `tx_launch.rs`, `launch.rs` and `relaunch.rs` against the brief's binding API and decisions 1-26.

- **The step order is as the brief gives it.**
  - Launch: 0, `spec_only`, 1, 2, then the policy, 3-6, then `edit_spec` with the act.
  - Relaunch: E0's spec check, `spec_only`, E0's cwd and cell rules, then 3-6, the act (B1-B9, O1, O2), R and B10.
- **The budget.** It is an `Instant` taken on entry and checked before every live step, R included. A timeout before R
  rolls back. B10 runs after R and outside the budget.
- **What gets recorded.** The record is the brief's field for field: `HOST_NAME`, `herdr_api_version` from step 4,
  `driven: None` on launch, and on relaunch the stored `hold`, DRIVEN and profile.
- **Rollback.** It stops processes only after A2 ran, and closes only a pane the step 5 snapshot did not list. A record
  conflict is not rolled back.
- **The supporting functions behave as the engine needs.** I checked them in the source:
  - `shown_differs(Some(sid), None)` is true, so O1 refuses a TUI that shows no session;
  - `HerdrPane` holds exactly session, workspace, id and grid, so O2's `contains` means "listed at its cell";
  - `HarnessKind` has one variant, so `spec.harness.kind` is the brief's `Opencode`.
- **The verbs.**
  - Each runs the CLI steps in the brief's order. `effective_spec` gives one `usage` line.
  - The stored profile name is passed on to the engine and to `emit_outcome`.
  - The reconcile step is appended only when `acted` is true and the message lacks that exact step.
  - The `data` and text shapes are the brief's.
- **The documented changes are within the brief's latitude** (`handoff-F.md`, "Deviations"):
  - the policy is parsed before the probe;
  - `list()` runs only to word `grid-occupied`;
  - B8 lists sessions only when the record has a session of record;
  - the rollback note names the call;
  - E0 says "the spec gives";
  - T's `Box<SpecFlags>` and the field name `pane`.
- **The split (new since the hold).**
  - The brief's dated note (line 10) says this run is part 1 of 2, ACs 1-31 are unchanged, and the PR says
    `Part of #644`.
  - ADR-0021's agent bullet names #644's last part after #700.
  - This changes no requirement the operator confirmed; it only orders it after its dependency, as #642 and #647 did.
- **The hold's required doc fixes are done.**
  - ADR-0021 line 482's `timeout` cell now states the exception (`launch` and `relaunch` name `pane.launch` and
    `pane.relaunch`). It no longer contradicts section 12 or `OP_LAUNCH`/`OP_RELAUNCH`.
  - `origin/main` is merged. The one "Deferred" conflict kept both sides.

## Quality audit

- **Correctness and failure handling.**
  - Every refusal before the act writes nothing.
  - A store read failure is returned before anything live happens.
  - A failed `list()` while wording `grid-occupied` still refuses the run (fail closed).
  - B1 and B2 failures leave the record as it was.
  - B10's failure is captured after R, so P keeps its edit and the run still exits 1.
  - No write is lost:
    - the record is written by a CAS at generation 0 or the read generation;
    - P's edit and its restore go through `edit_spec`'s CAS, and a conflict there is `profile-conflict`.
  - Decision 24's window on P is an accepted, documented risk.
- **Build guards.**
  - There is no `unwrap`, `expect`, `panic!` or `#[allow]` in production code.
  - No touched file reaches 900 lines: `tx_launch.rs` is at 786, under the brief's size fallback.
  - `dead_code` is denied and clippy is clean.
- **Protocol.** No wire field, golden file or `v2.md` change. Section 9 gains the three open codes, declared once in
  `tx_launch.rs`.
- **Tests.**
  - All run in-process over the fakes; there is no cross-process behaviour before #649.
  - No sleep is used for synchronization. AC 24's delay is a fake's fault switch, asserted as an invariant with a 3 s
    bound.
  - RED-first is evidenced in T-red.
- **Documentation.**
  - The CHANGELOG entry is accurate, including that the real binary answers `not-implemented` until #649.
  - ADR-0003 rows 48-49 and ADR-0021 are updated.
  - No README or `docs/` page lists the pane verbs, as is also true for #646a and #647.
- **Public-repository privacy.** Clean (AC 31).
  - The test data is neutral: `demo-c1r1`/`demo-c2r1`, `demo`, `scratch`, `main`, `/srv/demo`, ports 48100-48102,
    `p1/m1`, and the fake secret `s3cr3t644`.
  - `644-diff-result-r2.md` and its `.usage.json` are tracked although `.gitignore` line 25 matches them; they were
    force-added as gate evidence. Neither holds private data.
- **Commit and PR hygiene.**
  - The subjects are Conventional, apart from git's default subject on the two merge commits.
  - Every commit has a `Co-Authored-By` trailer except merge `87e90a3`. None carries a session link, which is also true
    of `main`'s recent history.
  - No branch commit holds a closing keyword (S grepped). The repo's squash message default is `COMMIT_MESSAGES`, so the
    PR body is the only path that could close #644 (Verdict, condition 1).

## Scope check

- **Delivered:** the brief's whole scope (ACs 1-31). The issue's Acceptance list is met.
- **Deferred, and recorded on purpose:**
  - the agent amendment (the split, recorded three times);
  - the operation id (PROPOSED in the ADR).
- **No frozen file and no manifest is touched.** S checked the brief's "Not touched" list with the three-dot diff.
  - The surface edits are the ones the epic assigns to the verb story: the fixture lines, ADR-0003 rows 48-49, the
    `stub.rs` deletion, ADR-0021, the CHANGELOG, and the `tests/pane_verbs/launch/{rig,guards,profiles}.rs` submodules.
- **Over-delivery:** none. The `unavailable` reason cell, the `timeout` exception, and the `stored_profile`/`spec_of`
  helpers are what A or the hold asked for, or what sharing code between the verbs needs.

## Verdict

**PASS.** Ready for O. Two conditions hold for the PR and the merge. Both are for the run's merging agent, not for F:

1. **The PR body.** The script writes `Closes #644.` (`coding-pipeline.workflow.mjs:4822`). Before merge, replace it with
   `Part of #644.`, so that no closing keyword appears anywhere in the body. Add the AI disclosure as well
   (`CONTRIBUTING.md`, "Disclosure is required"). If the body kept `Closes #644`, the merge would close #644 with the
   operator-confirmed agent part not done, which is exactly what the split prevents.
2. **CI on the merged tree.** T-green ran at `5cf6d94`, before both merges. Both required checks, `test (ubuntu-latest)`
   and `test (macos-latest)`, must be green on the pushed head. If AC 24 flakes on macOS, widen the delay and the budget
   together (advisory 5); do not loosen the assertion.

## Advisory notes (non-blocking)

1. **#700 will have to edit this story's files (new).** This part merges before #700, the reverse of the epic's order,
   and #700 adds `Pane.opencode_agent` and `ProfileSpec.opencode_agent`. This branch adds the first full `ProfileSpec`
   literals (no `..` rest) in `holler-cli`:
   - `crates/holler-cli/src/pane/launch.rs:198` (`effective_spec`);
   - `crates/holler-cli/tests/pane_verbs/launch/rig.rs:555` (`launch_spec`).

   It also adds a full `Pane` literal at `crates/holler-pane/src/tx_launch.rs:324` (`Plan::record`). Every `Pane` and
   `ProfileSpec` built in `holler-cli` on `main` uses `..sample_pane(..)` or mutates a fixture. #700 will therefore not
   compile until it touches those three places. Two are outside its declared blast radius, and its acceptance says
   "every existing test passes unedited". The MO should note this on #700 before it starts: either add
   `opencode_agent: None` in those three places, or leave the field's wiring to #644's last part.
2. **The brief's split note sits only in its header** (line 10). "Out of scope" (line 2577) and "Follow-ups" (line 2586)
   do not name the agent part. The durable record is the ADR bullet plus #644 staying open, so this is cosmetic.
3. **The CHANGELOG entry does not say it is part 1.** It does not mention that `--agent` follows after #700. #642
   part 2's and #646's entries mark their parts; #647's part 1 does not. One optional clause would fix it.
4. **Two items await the operator; ask both in one message.**
   - ADR-0021 line 525 still marks the recording of `host.herdr_api_version` "**PROPOSED**: #644's launch and relaunch
     record it". This change records it (`tx_launch.rs:332`, pinned by AC 1).
   - Decision 1 (no operation id) is PROPOSED at line 566.
5. **Test gaps carried from T-green and the hold.**
   - The rollback-failure note is untested.
   - `with_note` (`tx_launch.rs:775`) drops the note for a variant with no payload (`generation-conflict`,
     `not-implemented`, ...). It appends the note inside `Timeout.op`, which renders `timed out: pane.launch; rollback
     failed: ...` but leaves an in-process caller (#664) a non-canonical `op`.
   - AC 24's test does not assert that the rollback stopped the server.
   - AC 17's and AC 24's tests do not call `assert_matches`. AC 17's cases are covered by their own tests; AC 24's is
     not.
   - AC 24 has about 100 ms of margin each way.

   One cheap test would cover the first two points: `fail_next(AttachTui, ..)` plus `fail_next(Close, Unavailable)`,
   asserting `; rollback failed: herdr.close (unavailable)`.
6. **The issue's "the next doctor finds and reports it" is only partly true of the merged doctor.** Only a whole-fleet
   pass reports the leftover Herdr pane. No pass sees a leftover server or tmux session. A crashed launch's port stays in
   use until #695. ADR-0021 section 8's note says so. This is a follow-up for #647, #663 and #695 (A-plan warn 3). AC 7
   could also run `pane doctor` after the crash to make the composition a direct proof.
7. **Still open from the hold** (its advisories 6-8):
   - the never-adopt window between step 5's snapshot and A1, since `ensure_pane` answers an occupied cell's pane (#640
     or #649: a create-only `ensure_pane`);
   - no cross-record port check, so a record whose server is down still names its port (#664 or #647);
   - the follow-ups to file: A-dup warns 2-5, the brief's follow-ups except "the step names the pane", and A-plan warns
     3, 5 and 6.
8. **A journal stamp is wrong.** O's split entry in `decisions.md` is stamped 2026-10-10 01:50 MDT, but its commit
   `c76aeaf` is 01:07 MDT and this audit ran at 01:26 MDT. The Chain Summary should use the commit time.
