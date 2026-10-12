# Handoff-S: Phase 10 - #647 the reconcile engine and `holler pane doctor` (spec audit)

**Date:** 2026-10-09
**Branch:** issue-647-implementation (worktree `.claude/worktrees/0647-reconcile-doctor`, head `9806f72`, on `origin/main` `3bdd129`; `git ls-remote` shows `main` unmoved)
**Issue:** #647 (epic #633). Read live with `gh issue view 647` and its edit history (GraphQL `userContentEdits`).
**Brief:** `docs/handoffs/647-brief.md`, as amended in `a98258a`
**Handoffs reviewed:**
- `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` and `handoff-A-dup.md`;
- `decisions.md` and `evidence.md`;
- the outside diff review `647-diff-result-r1.md` (PASS) and the round-2 brief review `647-brief-result-r2.md` (PASS).

**Verdict:** ADVISORY-HOLD. F built what the brief asked for. The brief is out of date: the operator confirmed an amendment to the issue during the run, and the brief does not have it.

## A precondition

Met. `handoff-A.md` returned PASS on the plan (0 blocks, 7 warns), and `handoff-A-dup.md` returned PASS on the diff
(0 blocks, 5 warns).

## T precondition

Met. `handoff-T-green.md` reports "Blocking issues: None".

- **RED:** T-red ran at `8d1b199`: 0 passed and 31 failed. Every failure was an assertion, not a compile error.
- **GREEN:** T-green ran at `baf176f`. The results:
  - the doctor tests: 32 of 32 pass;
  - `pane_verbs`: 95 of 95 pass;
  - the workspace in CI's form: 1414 passed, 0 failed.
- **Mutation check:** T made 7 mutations. 6 were caught. T closed the 7th gap with
  `acknowledged_select_that_switches_nothing_is_not_fixed`.
- **Since T-green:** nothing but docs changed (`git diff --stat baf176f..HEAD` lists only `decisions.md` and
  `handoff-A-dup.md`).

## Acceptance criteria

Tests are in `crates/holler-cli/tests/pane_verbs/` unless a row says otherwise. "S" means I re-ran the read-only check
myself.

| Criterion | Proving test or evidence | Status |
|---|---|---|
| Issue: each 2026-10-07 incident is a test | the AC 1-4 tests below | Met |
| Issue: `--fix` never changes the session of record, and a second run reports nothing new | AC 7, AC 8 | Met |
| Issue: `--profile P` reports only P's panes, its JSON passes the envelope helper, and exit codes match across formats | AC 14, AC 16 | Met |
| Issue: a finding at row 2, col 1 prints `r2c1` | AC 18 | Met |
| Issue: each host's `herdr_api_version` is shown, and `herdr-version-unsupported` is reported | AC 20 | Met |
| Issue: decide the hub timer in the brief and raise it on #634 | ADR-0021 edit (AC 28) | Met; the #634 comment is a post-merge step for the MO |
| **Issue, amended 2026-10-09 17:40 MDT ("agent", operator-confirmed):** `doctor` raises `agent-cannot-dispatch` when a pane's recent hub-delivered turns ran as an agent that cannot dispatch while its project ships an orchestrator; it names the cure (`relaunch --agent KEY`); a deliberate opt-out raises nothing; `--fix` never changes a pane's agent; tested with the fakes and a transcript fixture. The issue now also depends on #700. | **None.** It is not in the brief and not in the code: `FindingKind::ALL` has 12 kinds, and AC 25 pins exactly 12. | **Not met** (a brief defect; see Verdict) |
| AC 1 | `doctor.rs::incident_tui_on_new_empty_session_while_hub_drives_the_old_one`; exit 0 via `doctor/surface.rs::doctor_envelope_and_exit_codes_match_across_formats` | Met |
| AC 2 | `doctor.rs::incident_three_day_wedged_server` | Met |
| AC 3 | `doctor.rs::incident_bare_unregistered_harness_in_the_orchestrators_pane`, `doctor.rs::unregistered_herdr_pane_is_reported` | Met |
| AC 4 | `doctor.rs::incident_registered_pane_whose_process_died`, `tmux_session_gone_is_reported`, `herdr_pane_gone_is_reported` | Met |
| AC 5 | `doctor.rs::incident_stray_ping_session` | Met |
| AC 6 | `doctor.rs::incident_deleted_session_under_a_tui` | Met |
| AC 7 | `doctor.rs::fix_selects_the_session_of_record_and_never_changes_it`; Decision 5's post-act check: `acknowledged_select_that_switches_nothing_is_not_fixed` | Met |
| AC 8 | `doctor.rs::second_fix_run_reports_nothing_new` | Met |
| AC 9 | `doctor.rs::fix_never_guesses_a_session` (both halves) | Met |
| AC 10 | `doctor.rs::fix_skips_the_orchestrator_unless_named` | Met |
| AC 11 | `doctor.rs::fix_failure_is_reported` (verb, JSON, exit 0) | Met |
| AC 12 | `doctor/read_only.rs::doctor_without_fix_makes_only_read_calls` (checks every incident kind is present, so the call-log checks are not vacuous) | Met |
| AC 13 | `doctor/read_only.rs::unchanged_observation_writes_nothing` | Met |
| AC 14 | `doctor/surface.rs::doctor_profile_reports_only_its_panes`, `another_profiles_session_of_record_is_not_a_stray` (with a precondition) | Met |
| AC 15 | `doctor/surface.rs::doctor_named_pane_scopes` (scope, plus 4 refusals in both formats) | Met |
| AC 16 | `doctor/surface.rs::doctor_envelope_and_exit_codes_match_across_formats`, plus the AC 15 refusals through `both_formats` | Met |
| AC 17 | `doctor/surface.rs::doctor_store_failure_is_an_error` | Met |
| AC 18 | `doctor/surface.rs::finding_prints_rowcol` | Met |
| AC 19 | `doctor/surface.rs::json_data_shape_is_pinned` | Met |
| AC 20 | `doctor/surface.rs::herdr_version_shown_and_unsupported_reported` | Met |
| AC 21 | `doctor/surface.rs::observe_failure_is_a_finding_not_an_abort` | Met |
| AC 22 | `doctor/surface.rs::doctor_output_holds_no_secret` (4 world states, with and without `--fix`, both formats, both streams) | Met |
| AC 23 | `doctor/surface.rs::text_output_escapes_control_characters` | Met |
| AC 24 | `doctor/surface.rs::observation_runs_concurrently` (an invariant, 20/20 in T-green) | Met |
| AC 25 | `crates/holler-pane/tests/findings_test.rs::finding_kind_codes_are_stable` | Met for the brief; it pins 12, which the issue now contradicts |
| AC 26 | `doctor/surface.rs::doctor_remedies_parse` (through `verb_harness::parse::try_parse`) | Met |
| AC 27 | S: the ADR 0003 grep prints `1`; the `# #647` group is Decision 11's five lines; `("pane", "doctor", 647)` is gone and `// #647` stays. T-green: the `cli_surface_test`, `docs_cli_test` and `pane_cli_process` targets pass. | Met |
| AC 28 | S: `Deferred to #647` count is `0`; `Decided (#647)` is at line 472; ADR-0021 is +8/-3, exactly Decision 1's three edits | Met |
| AC 29 | S: the entry is under `## [Unreleased]` / `### Enhancements` and links #647 (line 180). T-green: `changelog-check: ok`. | Met |
| AC 30 | T-green: clippy, workspace tests and lint all pass. S: the largest touched file is 586 lines, and the only new `#[allow]` carries `// #647`. | Met |
| AC 31 | T-green: `rustfmt --check` is clean on the touched files. The brief's `cargo fmt` grep matches every file here, because the worktree path contains `reconcile` and `doctor`; it was run with relative paths instead. | Met (the command needs fixing: advisory 5) |
| AC 32 | S: the `unsafe` grep is empty and the manifest and lock diff is 0 lines. T-green: machete is clean. | Met |
| AC 33 | Outside the Files list: `crates/holler-pane/tests/findings_test.rs` (A's W-6(a)), plus `doctor/read_only.rs` and `doctor/surface.rs` by name (AC 31 already allows submodules) | Met in substance; the Files list is out of date (advisory 5) |

Two more tests are not tied to a numbered AC:

- `doctor.rs::healthy_fleet_reports_nothing_and_records_the_observation` (the baseline: no false positives);
- `doctor/read_only.rs::record_write_conflict_is_reported_not_retried` (Decision 6 and R-6).

The tests assert behaviour: findings, records, call logs, envelopes and exit codes. They do not assert internals.
T's mutations show that removing a behaviour makes a test fail.

## Spec compliance

I checked each brief decision against `findings.rs`, `reconcile.rs`, `reconcile/observe.rs` and `pane/doctor.rs`.
Every decision is built as stated, or the change from it is documented in `handoff-F.md`:

- **D1, no hub timer:** done. ADR-0021 edits (a), (b) and (c) are in. Edit (c) uses A's more accurate wording ("which I2
  makes the session the hub drives"), which is documented. The #634 comment is for after merge.
- **D2, once per run:**
  - Each finding appears once per run. The sort key is D2's, with the message added to break the last ties.
  - **Documented change:** F dedupes on the whole finding, not D2's `(kind, pane, session, herdr_pane)` key. D2's key
    would merge the two findings that AC 21 requires to stay separate (both `observe-failed` with no pane). It would also
    merge a pane's separate `observe-failed` findings, one per failed call. The brief contradicts itself here, and F
    settled it the way AC 21 requires (advisory 2).
- **D3, the 12 kinds:** each "observed when" rule, fix state and remedy matches the table. Specifically:
  - `tui-foreign-session` takes precedence over the mismatch;
  - only a healthy server's list is trusted for `session-of-record-missing`, strays and fixability;
  - strays are judged against every record;
  - `unregistered-herdr-pane` is reported in whole-fleet runs only.
- **D4, the observation order:** done. Each pane is checked with `ps`, `health`, `list_sessions` and `shown_session`. One
  Herdr thread calls `version`, then `snapshot`. `pane_store.list()` runs in every mode.
- **D5, `--fix`:** done.
  - The fix runs only when the server is healthy and lists S, and the orchestrator only when the run names it. Parked and
    drained panes are not treated differently.
  - It runs plan, act, observe, record, and calls no writer (AC 7).
  - One refinement: `fixed` also needs the select call to succeed. It is under F's design decisions, not under F's
    deviations (advisory 1).
- **D6, the record write:**
  - The record is written only when a value changed. A conflict is one `observe-failed`, with one `CasPut` and no retry.
    `driven` and every other field stay as stored.
  - **Documented change:** a record's first observation (`at == 0`) is written even when the values equal the defaults,
    so `at > 0` always means "observed" (A's W-1 edge case).
- **D7, exit codes:** done.
- **D8, security:** done (see the quality audit).
- **D9, concurrency:** done.
  - It uses `thread::Builder::spawn_scoped`, and every handle is joined.
  - A thread that cannot start, or that panics, becomes an `observe-failed` finding.
- **D10, the public API:** done, with documented changes that A endorsed:
  - `Serialize` for `FindingKind` and `FixState` is hand-written from `code()`/`as_str()`. The JSON is the same, and each
    code now has one source.
  - `ObservedHealth` is `ServerWedged`/`ServerDown`, spelled `server-wedged`/`server-down` in JSON, not
    `wedged`/`down` (W-5(b)).
  - The clock is `now_millis()`, not a hand-written `SystemTime` read (W-3).
  - Additive `pub` items, each asked for by A: `shown_differs`, `doctor_command`, `FindingKind::remedy`, `quoted`,
    `FixState::as_str`, and `From<&PaneError> for FixError`.
- **D11, the CLI:** the `Args`, the text line formats, the summary line and the five fixture lines are as stated.
- **D12, where tests live:** the tests are in `pane_verbs/doctor.rs` and its submodules. AC 25 moved to
  `holler-pane/tests/findings_test.rs` (W-6(a)), which needs no manifest change: `serde_json` is already a normal
  dependency of `holler-pane`.
- **D13, hosts and Herdr version:** hosts come from the records in scope, and the version from the live call. Neither
  field is written.

**Against the issue:** the one gap is the "agent" amendment (Verdict). Every other issue paragraph is covered.

## Quality audit

- **Correctness and failure handling.**
  - A failed read is an `observe-failed` finding, and any rule that needs that value is skipped, so nothing is guessed.
  - A failure of the store or the scope exits 1, with `data` null.
  - A conflict on the CAS write at the generation the pass read is reported, and the other writer's record wins, so no
    write is lost.
  - Nothing is written when nothing was observed.
  - I traced every rule for wedged, down, unknown, unread-list, unseen-screen and no-record states, and found no defect.
- **Build guards.**
  - The four production files have no `unwrap`, `expect`, `panic!`, `todo!`, `unreachable!` and no direct printing
    (S grep).
  - There is one new `#[allow]`, in `findings_test.rs`, and it carries `// #647`.
  - File sizes: the code files are 113 to 444 lines, and the tests up to 586.
  - Clippy `-D warnings` is clean, and the workspace denies `dead_code`.
- **Protocol.** None: no wire method, field, golden file or `docs/protocol/v2.md` change. The doctor JSON is a new part
  of the CLI's versioned `--json` surface. AC 19 pins its shape.
- **Tests.**
  - The verb is tested in-process over the test kit's fakes. This story has no cross-process behaviour; wiring is #649.
  - No sleep is used to synchronise. AC 24 waits on a condvar with a bound and asserts an invariant, not a duration.
  - The RED-first evidence is in T-red.
- **Documentation.**
  - The CHANGELOG entry is accurate, including that the verb answers `not-implemented` until #649 wires the ports (I
    checked `pane/wiring.rs`).
  - The ADR 0003 row and the ADR-0021 edits are in.
  - The module docs give the meaning of what reconcile writes.
  - No README or `docs/` page lists the pane verbs yet; the operator guide is #652's.
- **Public-repository privacy.**
  - I grepped the added lines for personal names, hosts, IPs and private domains: no hit.
  - `gitleaks git --log-opts=origin/main..HEAD` found no leaks.
  - The test data is neutral: `demo-*`, `scratch`, `localhost`, `/srv/demo`, `bare-opencode`, and ports 48100-48150.
- **Commit and PR hygiene.**
  - The commits are the pipeline's own `chore(#647): <phase> -- ...` subjects with a `Co-Authored-By` trailer. Like
    `main`'s recent squash commits, they have no session link.
  - There is no PR yet. When it opens, add the AI disclosure to the body (`CONTRIBUTING.md`; CLAUDE.md's post-PR
    `gh pr edit`). Use `Closes #647` only after the hold below is settled.

## Scope check

- **Delivered:** the brief's whole scope.
- **Over-delivery:** none beyond what A asked for: the `pub` helpers above, the brief's named fallback files
  (`reconcile/observe.rs`, `doctor/rig.rs`) and the test submodules.
- **Outside the issue's Blast radius:** `crates/holler-pane/tests/findings_test.rs`. It is a new file that no other story
  owns, but #665 (and any story that adds the agent kind) must now edit its pinned list.
- **Under-delivery against the issue:** `agent-cannot-dispatch`. F could not have known about it, because it reached the
  issue after the brief and after T-red.

## Verdict

**ADVISORY-HOLD.**

**The defect.** The brief predates the operator-confirmed "agent" amendment to #647. Timeline, in local time:

| Time (MDT) | Event |
|---|---|
| 16:39 | brief written |
| 17:06 | brief amended |
| 17:35 | T-red |
| 17:38 | the amendment is added to the issue as PROPOSED |
| 17:39 | epic #633 gains decision 8; #700 is created |
| 17:40 | the operator confirms the amendment |
| 18:03 | F |

The confirmed amendment requires `doctor` to raise a new finding, `agent-cannot-dispatch`, and makes #647 depend on
#700, which is OPEN. The brief, all 33 ACs and the code know 12 kinds, and AC 25 pins exactly 12. No test covers the
requirement, and no phase noticed it.

**The rules it breaks:**

- CLAUDE.md: "The GitHub issue is the source of truth".
- Epic #633, amended 2026-10-09 ("agent"): the `opencode_agent` field, its guard and the flag are "#700 only; it merges
  before #642, #644 and #647 start".
- The epic's wave-3 table: "#642, #644 and #647 also need #700".

**Why this is a hold, not REWORK.** F cannot build this finding from this brief:

1. `Pane.opencode_agent` does not exist on `main`, and in `holler-pane/**` that field is #700's alone.
2. No frozen port can observe what the finding needs: which agent recent hub-delivered turns ran as, whether `task` is
   denied, or whether the pane's project ships an orchestrator. `HarnessPort` (`ports.rs:176-200`) has only
   serve/health/sessions/abort/attach/select/shown. So the finding needs an amend-first decision about the contract, and
   a transcript fixture in the test kit. Both are outside this story's blast radius.
3. The brief has no detection rule, remedy string or AC for it. The cure `relaunch --agent KEY` also needs #644's flag.

**Proposed fix: O or the MO picks one.**

- **(a) Split. This is the cheapest option, and it follows the #665 `profile-drift` precedent.**
  - Move the agent paragraph and the #700 dependency out of #647 into a new story. That story adds one `FindingKind`
    (`agent-cannot-dispatch`) after #647 and #700 merge. It owns the decision on where the observation comes from, and it
    edits `findings_test.rs`'s pinned list.
  - Amend #647's text to point at the new story. Add it to the brief's Out of scope and Forward-compat, journal it, and
    clear this hold. This branch needs no code change.
  - The operator confirmed the amendment, so moving it to another story changes the scope of an operator decision. It
    may need the operator's OK.
- **(b) Hold #647 until #700 merges.**
  - Amend the brief: a 13th kind, its observation source (through an amend-first change to a port), its remedy, and its
    ACs. AC 25's pinned list grows to 13.
  - Then re-run T-red, F, T-green, A-dup and S.

## Advisory notes (non-blocking; fold them into the same brief amendment)

1. **A select call that errors but switches anyway.** In `select()` (`observe.rs:318-340`), a failed `select_session`
   whose follow-up `shown_session` shows S is reported `failed`, with remedy `holler pane relaunch <pane>`. The same run
   records `last_observed.shown` = S, and `panes[].shown` is S too. D5's literal rule ("equal to S is `fixed`") would say
   `fixed`. No test covers this case. It could tell an operator to relaunch a pane that is already fine.
   - Fix: record the rule in the brief. Either the observation decides (`fixed`), or keep `failed` with remedy
     `holler pane doctor <pane>`, not relaunch.
2. **D2 contradicts AC 21.** Amend D2 to the rule F implemented: drop exact duplicates, and merge strays by session.
3. **D6's first-observation write.** Record F's rule in the brief. The CHANGELOG's "writing a record only when that
   changed" leaves out this one-time first write.
4. **W-1, still open.** ADR-0021 §1's `harness` and `last_observed` rows still name neither the writer nor the meaning of
   what reconcile writes. AC 28 forbade the edit. The meanings are only in `reconcile.rs`'s module doc (lines 26-40),
   yet #645, #646 and #648 gate on these fields.
5. **Brief bookkeeping.**
   - AC 31's grep needs relative paths.
   - Add `findings_test.rs` to Files, AC 33 and C-3.
   - Tell #665 that it edits `findings_test.rs` too.
6. **C-8 can be closed by ordering.** `doctor` reaches real ports only through #649, and the epic's table makes #649
   depend on #639 through #647, so on #644 and #645. So no operator can run a relaunch or reset remedy before those verbs
   exist. R-4 still stands: #644's `relaunch` positional must match `holler pane relaunch <pane>`.
7. **A-dup's D-1 to D-5 are still O's decisions:**
   - D-1: make the rig `pub(crate) mod rig;`;
   - D-2: pick one terminal-quoting rule with #643;
   - D-3: #663 and #644 build the reconcile step with `findings::doctor_command`;
   - D-4: agree the post-act mismatch code with #644 and #645;
   - D-5: an audit item for the sanitizers.
8. **After merge:** one comment on #634 that links the ADR-0021 change (D1, C-7). This is an outward action for the MO.
