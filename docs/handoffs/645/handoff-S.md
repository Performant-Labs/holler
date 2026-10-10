# Handoff-S: Phase 8 - #645a `pane switch` and `pane reset` (spec audit)

**Date:** 2026-10-10 00:11 MDT
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`), head `08d96df` (A-dup round 2)
**Issue:** #645, part 1 of 2 (645a), epic #633. **Brief:** `docs/handoffs/645-brief.md`, as amended in `8cf4f00`
**Handoffs reviewed:** `handoff-A.md` (round 2), `handoff-T-red.md`, `handoff-F.md` (round 3), `handoff-T-green.md`
(round 3), `handoff-A-dup.md` (round 2), `decisions.md` and `evidence.md`. Also the outside gates' result files, which are
gitignored: brief r1 PASS (20:03 MDT, on the amended brief), diff r1 PASS (23:03), r2 BLOCK B-1 (23:37), r3 PASS (23:52).
**Diff audited:** `git diff origin/main...HEAD`, merge base `d9eabbb`. Since A-dup's round 2, `origin/main` has moved to
`abdcbb6`: #713 (#660) and #714 (#640 part 3) merged at 00:05 and 00:06 MDT on 2026-10-10.

## A precondition

Met. `handoff-A.md` round 2 is **PASS** (the plan), and `handoff-A-dup.md` round 2 is **PASS** (the diff, on `940e338`).

## T precondition

Met. `handoff-T-green.md` round 3 lists no blocking issues. RED was confirmed in `handoff-T-red.md`: 94 passed and 19
failed, each failing on its assertion, with no compile error, setup panic or timeout. GREEN gave `pane_verbs` 154/154, and
the workspace in CI's form 1642 passed, 0 failed, 14 ignored.

## Acceptance criteria

**The brief's ACs.** All test paths are under `crates/holler-cli/tests/pane_verbs/`. Every case runs through `both_with`
(`switch.rs:121-150`). It checks, on every run of every test, that text and JSON exit with the same code, that the JSON is
one valid envelope with nothing on `err`, that text writes one line to the right stream, and that there is no Herdr or
host call. The rig's records start at `Health::Unknown` and `last_observed {shown: None, driven: None, at: 0}`
(`holler-pane-testkit/src/fixture.rs:41-80`), so each field AC 1 asserts is a real write.

| AC | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | TUI and record move together, exactly four fields, generation +1 | `switch.rs` `switch_moves_the_tui_and_the_record_together` (`assert_recorded` compares the whole record; `driven` pre-set to `ses_driven` and kept; exact text line; JSON `verb`, `previous`, `session_of_record`, and `data.pane` equal to the stored record) | MET |
| 2 | No keystroke; exact call sequences | The I4 check in `both_with` (`switch.rs:141-148`) on every run; AC 1's `[Health, ListSessions, SelectSession, ShownSession]` and `[Get, List, CasPut]` (`switch.rs:277-286`); AC 14's `[Health, CreateSession, SelectSession, ShownSession]` and `[Get, CasPut]` (`reset.rs:71-79`). There is no test named `switch_and_reset_type_nothing`, but these assertions cover more than it would | MET |
| 3 | Deleted session: `session-not-found`, nothing changes | `switch_to_a_deleted_session_changes_nothing` (T's mutation: skipping P4 fails it) | MET |
| 4 | Another pane's session: `session-of-other-pane` | `switch_to_another_panes_session_is_refused` (shared data directory; names `demo-c2r1`; both panes unchanged; skipping P5 fails it) | MET |
| 5 | Unhealthy server: 3 `server-unhealthy`; health timeout: 1 `timeout` | `switch_refuses_an_unhealthy_server` (kill and freeze stop at `[Health]`; message has `run holler pane relaunch demo-c1r1`) | MET |
| 6 | Orchestrator's pane needs `--as-operator` | `switch_refuses_the_orchestrators_pane_unless_as_operator` | MET |
| 7 | Mismatch after select records nothing (I3) | `switch_mismatch_after_select_records_nothing` (`its home screen`, ends with the reconcile step; ignoring O1 fails it) | MET |
| 8 | Select failure names the reconcile step | `switch_select_failure_names_the_reconcile_step` | MET |
| 9 | Record conflict after the act | `switch_record_conflict_after_the_act` (`WriterInSelect`: the other writer's record stands and the TUI shows S2, so the verb neither retries nor compensates) | MET |
| 10 | `--profile` scoping | `switch_in_a_profile` | MET |
| 11 | Unknown pane | `switch_unknown_pane` | MET |
| 12 | Usage before any port; no raw ESC | `switch_usage` (every `calls.*` empty, `probes == 0`) | MET |
| 13 | Switch to the current session | `switch_to_the_current_session_is_idempotent` | MET |
| 14 | Reset creates, shows and records a fresh session | `reset.rs` `reset_creates_a_fresh_session_and_switches_to_it` | MET |
| 15 | Reset is doctor's remedy, (a) and (b) | `reset_is_doctors_remedy_for_no_session_of_record`; `reset_is_doctors_remedy_for_a_deleted_session_of_record` (doctor's own remedy string, `holler pane reset demo-c1r1`, parses and is the argv that runs) | MET |
| 16 | Old session stays as the one stray | `reset_leaves_the_old_session_as_a_stray` | MET |
| 17 | Reset refusals create nothing | `reset_refusals_create_nothing` (four refusals; sessions listed through Q's server) | MET |
| 18 | Failure after create names the unrecorded session | `reset_failure_after_create_names_the_unrecorded_session` | MET |
| 19 | Reset mismatch records nothing | `reset_mismatch_records_nothing` | MET |
| (A1) | A failed create: no step, no created note | `reset_create_failure_changes_nothing` (T, round 2) | MET |
| 20 | `stub.rs` entries removed, `// #645` kept | the diff; `pane_cli_process` 34/34 (T-green) | MET |
| 21 | `cli-surface.txt` `# #645` block exactly | the diff matches the brief line for line; `cli_surface_test` 3/3, `docs_cli_test` 3/3 | MET |
| 22 | ADR 0003 rows 51-52 | the diff: exact text, with `#645` at column 67 like rows 44-45, 48-49, 55-56 and 61 | MET |
| 23 | Help names the arguments, no `--first` | `switch.rs` `help_names_the_arguments` (it fails on `origin/main`, per T-red) | MET |
| 24 | ADR-0021 holds exactly Decision 17's edits | On the merge base `d9eabbb`: four hunks; the paragraph is at `:345`, between `### 8.` (`:285`) and `### 9.` (`:372`); one `unavailable` decision in section 8 (`:352`); `#645` 9 times against 6. **Against `origin/main` now (`abdcbb6`), the AC's own command `git diff origin/main -- docs/adr/ADR-0021.md` shows 8 hunks, and the branch does not merge.** | **NOT MET on the tree that will land** (REWORK 1) |
| 25 | clippy, rustfmt, lint, workspace tests, no new dependency | T-green round 3, on the `d9eabbb` base: all clean. The three-dot `Cargo.toml` diff is empty. **Against `abdcbb6`, `git diff origin/main -- '*Cargo.toml'` is 14 lines** (#714's `holler-adapter-herdr/Cargo.toml`, shown reversed), and no workspace run has covered #713's and #714's tests | **NOT MET on the tree that will land** (REWORK 1) |
| 26 | CHANGELOG entry | under `## [Unreleased]` / `### Enhancements`, `Part of [#645]`; `changelog-check: ok` (T). It merges cleanly with #714's entry (checked in a merge-tree) | MET |

**The issue's criteria** (the source of truth):

| Issue criterion | Evidence | Status |
|---|---|---|
| No call types into a TUI, and the test fails if one does | `both_with`'s I4 check on every run. The verbs run on the rig's logged `FakeHerdr` and `FakeHost` (`doctor/rig.rs:160-169, 250-259`) | MET |
| Refusals are named, with stable codes | every refusal test asserts its code, in JSON and by exit code | MET for 645a's refusals |
| A switch to a deleted session fails and changes nothing | AC 3 | MET |
| The first message lands in the new session and nowhere else | no port sends a prompt (brief C-2, B-2, F-1) | **Deferred to 645b** by Decision 1, which the operator's request ("#645a") acknowledges. The switch form of the wrong-session incident is AC 4 |
| A pane outside P is refused and nothing changes | AC 10, AC 17 | MET |
| Every refusal and success passes the envelope helper; exit codes equal across formats | `both_with`, `switch.rs:128-135` | MET |
| Scope: unhealthy server and orchestrator's pane refused; not idle or a held question | AC 5, 6, 17 | MET; not idle and held question are 645b |
| `--format=text\|json` through `output::emit()` | `emit_outcome` calls `emit` (`pane/switch.rs:143`). `execute`'s usage path calls `emit_error`, which is `emit` (`output.rs:241-242`) | MET |
| Only #638's fakes and envelope helper; no new dependency | the doctor rig over the test kit; `check_envelope`; the three-dot `Cargo.toml` diff is empty | MET |

## Spec compliance

- **The public API** is the brief's, item for item: the three `RefusalCode`s and `SESSION_ID_MAX`, `parse_session_id`,
  `Target`, `SwitchRequest`, `Switched`, `SwitchFailure` with `From<PaneError>` and `message`, and `switch`
  (`tx_switch.rs:50-178`); `PaneSwitch`, `run`, `Verb` and `emit_outcome` (`pane/switch.rs:31-66, 123-128`); and
  `PaneReset` and `run` (`pane/reset.rs:20-43`). The names, fields, derives and doc strings all match.
- **The Behaviour table, P0 to R**, runs in the brief's order: `plan` (`:182-195`), then `read`, `refuse_orchestrator`,
  `check_health`, `check_listed` and `check_unclaimed` (`:200-283`). Then `switch`'s act (`:154-178`),
  `select_and_observe` (`:287-302`) and `recorded` (`:317-324`). The messages of P2, P3, P5 and O1 and of
  `SwitchFailure::message` are word for word the brief's. `screen_text` is the brief's private copy, word for word
  (`:307-312`).
- **Decisions 1-19 are implemented as stated:**
  - one engine;
  - refusals before any write;
  - `--as-operator` as an intent gate;
  - health observed live, with `Err` passed through;
  - P4, and P5 with a stray allowed;
  - no park gate;
  - `unavailable` for a mismatch;
  - no compensation;
  - a clone-and-set of four fields in one compare-and-swap, never retried, with `driven` kept;
  - no Herdr or host call;
  - at most seven port calls;
  - the session-id grammar;
  - the output shape;
  - the rig declared once (`doctor.rs:10` is the only `doctor.rs` change);
  - the ADR-0021 edits;
  - the previous session left as a stray.
- **Deviations.** Each is documented in `handoff-F.md` or `handoff-A.md`, and each is acceptable:
  - P1 takes the resolved pane with `find` on its name, not `into_iter().next()` (`tx_switch.rs:206-207`). Under
    `ProfileScope`'s contract it is the same pane: the real `StoreScope` and the fake both answer exactly the pane `n`
    (`evidence.md`, F round 3). It does no indexing, which was the point of A's round-1 warn 7.
  - `reset.rs` reuses `switch::{execute, Verb}`, not `{emit_outcome, Verb}`. `execute` calls `emit_outcome`, so nothing is
    copied.
  - The "Deferred" bullet takes A's round-2 form, "#645 for switch and reset; #644 to follow for launch and relaunch".
    #644's paragraph is still not on `main` (checked at `abdcbb6`), so this form is more accurate than Decision 17(c)'s
    literal text.
  - The #645 paragraph has four sentences that Decision 17(b) does not list: when the reconcile step is printed, and why
    it names the pane with `--fix`, unlike #663's step 6. They are A-dup round 1's fix, and they sit inside AC 24's "end
    of section 8".
- **No ADVISORY-HOLD.** The brief is not defective. The split (C-2, Decision 1) is reasoned and recorded, and the
  operator's request for this run names the story "#645a".

## Quality audit

- **Correctness and failure handling.**
  - Every refusal comes before any write or live change, by construction.
  - The record write is one compare-and-swap at the plan's generation, never retried.
  - Nothing is compensated after the act: AC 9 shows the TUI stays on S2 and the other writer's record stands.
  - A mismatch fails closed as `unavailable` and records nothing.
  - A session that a reset created and did not record is named in the message.
  - A port's `Err` is passed on unchanged.
  - Accepted risk: two concurrent switches to one session can both pass P5 (R-5). The ADR states this, and F-3 is its
    follow-up.
- **Build guards.**
  - No `unwrap`, `expect`, `panic!`, `todo!` or indexing in the three production files (grep).
  - No `#[allow]` in any touched file.
  - Every touched file is under 900 lines: `tx_switch.rs` 324, `pane/switch.rs` 160, `pane/reset.rs` 43, test `switch.rs`
    613, test `reset.rs` 288, `ADR-0021.md` 633, `CHANGELOG.md` 769.
  - No dead code: clippy with `-D warnings` is clean (T), and `execute` and `emit_outcome` both have callers.
- **Protocol.** No wire, golden or `docs/protocol/v2.md` change, and none is needed: there is no hub method and no field.
  The three codes are open `RefusalCode`s declared with `from_static` in the verb's own file, as ADR-0021 section 9 asks,
  and the section 9 row now lists them. The closed-code table is untouched.
- **Tests.**
  - In-process over the test kit's fakes, the tier the issue names.
  - No sleeps (grep). The clock is asserted as a window.
  - AC 9's concurrent writer is injected deterministically inside `select_session`.
  - RED-first evidence is in T-red. T-green's mutations across rounds 1-3 (P4, P5, O1, the record fields, the reconcile
    step, the created note, `acted`, and the remedy's `Some` arm) were each caught by the right test.
- **Security.**
  - `SESSION` is typed to `[A-Za-z0-9_-]{1,64}` before any port is called (AC 12).
  - Harness ids are quoted in every message and every text line.
  - The reconcile step is built from a validated `PaneName`.
  - JSON `data.pane` is the stored record. Its `env` holds variable names only (`holler-pane/src/pane.rs:246-248`), and
    `pane get` prints the same record (`pane/get.rs:66-69`).
- **Documentation.**
  - The CHANGELOG entry, the ADR 0003 rows and the ADR-0021 edits.
  - The merged verb stories #643 (`efd9a00`) and #646 (`13edbb4`) changed the same set of docs, and no README page.
  - Code comments cite ADR-0021 sections and issues. The test-module docs cite "brief ACs", as the merged
    `doctor/rig.rs` and `park.rs` do.
- **Public-repository privacy.**
  - Scanned: every added line of the diff, the handoffs included.
  - Looked for: host, tailnet and machine names, private IPs, absolute home paths, email addresses and key-like strings.
  - Found: one hit, the brief's own rule statement (`645-brief.md:21`).
  - The tests use only the neutral names `demo-c1r1`, `demo-c2r1`, `demo` and `scratch`, and ports from 48100.
- **Commit hygiene.**
  - 12 non-merge commits, all with Conventional subjects (`chore(#645): ...`, `docs(handoffs): ...`).
  - Every commit has a `Co-Authored-By` trailer, and the author and committer are the GitHub no-reply address.
  - None carries a session link. The merged squash commits `d9eabbb` and `13edbb4` have the same form, so this is the
    script's practice, not this story's defect.
  - No PR exists yet, so the AI disclosure cannot be checked (advisory 3).

## Scope check

The scope matches the brief's Files list exactly: no frozen file, no #647 file but `doctor.rs:10`, no test-kit file and no
`Cargo.toml`. `execute` and `request` are small private structure. T's `failed_before_the_act` and
`reset_create_failure_changes_nothing` pin the ADR's no-step-before-the-act sentence and the brief's A1 row, so they are
inside the story. Nothing is under-delivered against the brief. Against the issue, the not-idle and held-question
refusals, `--first` and "the first message lands in the new session" are 645b by Decision 1. This PR is "Part of #645"
and must not close it.

## Verdict

**REWORK** (production: F acts, then T re-verifies GREEN). There is one required change.

1. **Merge `origin/main` (`abdcbb6`) and resolve the conflict in `docs/adr/ADR-0021.md` by keeping both sides.**
   `git merge-tree --write-tree HEAD origin/main` exits 1, and that file is the only conflict. `CHANGELOG.md` and
   `tests/pane_verbs/process/stub.rs` merge cleanly: the #645 entry stays under `[Unreleased]` / `### Enhancements`, and
   `stub.rs` keeps `// #645` with no entries. Both hunks are adjacent-line edits from #714 (#640 part 3), the conflict
   that R-1 and A-dup's note anticipated:
   - **Section 9 table** (`ADR-0021.md:403-404` on the branch). Take `origin/main`'s `pane launch`, `pane relaunch` row,
     which now ends `; open (#640) for a cell that no single split reaches, `grid-unreachable``, and this branch's
     `pane switch`, `pane reset` row.
   - **"Deferred to named stories"** (`ADR-0021.md:603-608` on the branch). Keep this branch's mismatch-code bullet and
     its **PROPOSED (#645, pending the operator)** bullet. Follow them with `origin/main`'s rewritten bullet,
     "`HarnessPort` in its final form: #635, then #642. `HerdrPort`: #640 implements it as merged; ...", in place of the
     old "`HerdrPort` and `HarnessPort` in their final form" bullet.
   - Taking either side whole is wrong. "Ours" reverts #714's row and bullet, and "theirs" drops this story's row and two
     bullets. Either way AC 24 fails.

   After the merge, check:
   - AC 24's `git diff origin/main -- docs/adr/ADR-0021.md` again shows exactly this story's four hunks, with the #645
     paragraph between `### 8.` and `### 9.`.
   - AC 25's `git diff origin/main -- '*Cargo.toml'` is empty again.
   - T re-runs GREEN with `CARGO_BUILD_JOBS=4`: `pane_verbs`, `pane_cli_process`, `cli_surface_test`, `docs_cli_test`, the
     workspace in CI's form, clippy, `lint.sh` and `changelog-check.sh`. The merged tree adds #713's `output_api.rs` and
     `stub.rs` tests and #714's adapter tests.

   No code or test change is expected:
   - #714's edits to `holler-pane` (`error.rs`, `pane.rs`, `ports.rs`, `reconcile.rs`) are doc comments only.
   - #660's new stub test picks the first remaining pane stub (`launch`, #644).
   - Sections 8 and 11 merge cleanly, and their #645 text stays in place.

**Why REWORK and not PASS with a note.** The Workflow script pushes and opens the PR without merging `main`
(`coding-pipeline.workflow.mjs:3336-3347`), so a PASS now would open a conflicting PR that CI cannot test. Resolving the
conflict after S would edit the standing spec outside every gate. The brief assigns this resolution to F: R-1 says
whichever story merges second resolves it, editing by anchor text. F's round-2 merge, after A-dup's round-1 block, is the
precedent in this run. Everything else in this audit passes, so after the merge only AC 24 and AC 25 need re-checking.

## Advisory notes (non-blocking)

1. **The issue and the PR.** The PR must say "Part of #645", not "Closes #645". The brief's P3 is still not done: #645's
   body does not mention the split, and the issue has no comments. P1 and P2 (645b's design, and landing it before #649's
   first-message step and before #654) are still PROPOSED.
2. **Follow-ups to file.** None is filed yet; A-dup searched at 23:55 MDT. Once the `screen_text` fold is filed, the
   comment at `tx_switch.rs:304-306` should name its issue instead of "a follow-up of #645".
   - F-1: the stray session that every reset leaves.
   - F-2: the fourth both-format runner, with `WriterInSelect`.
   - F-3: one session per pane, enforced in the registry's compare-and-swap. A's optional ordering is "before #654", since
     R-5's race goes live once #649 wires the ports.
   - F-4, widened: the `to reconcile, run ` lead-in and `screen_text`, moved into `findings`, with `reconcile_step`'s doc
     corrected.
   - A-dup warn 4: the four copies of the scoped read.
3. **Hygiene at merge.**
   - A Conventional squash subject, e.g. `feat(cli): holler pane switch and reset (#645 part 1 of 2)`.
   - The `Co-Authored-By` trailer with a session link (`CONTRIBUTING.md:19-21`).
   - The AI disclosure in the PR body, added with `gh pr edit` after the script opens the PR (`CLAUDE.md`).
4. **The journal.**
   - The outside diff gate's r3 (PASS, 23:52 MDT, on F's round-3 tree) is cited only in A-dup round 2's evidence. It has
     no round entry of its own, which `pipeline-conventions.md` section 1 asks for.
   - The r3 result file calls itself "Implementation Review (Round 1)", and its usage file says `"round": "1"`.
5. **ADR-0021 section 12.** Its "prints the reconcile step" is broader than the #645 paragraph's no-step-before-the-act
   rule (A-dup warn 1). This is for the next section-12 edit (#644).
6. **A failed create.** A `timeout` from reset's `create_session` may leave a session that the message cannot name. Doctor
   reports it as a stray. This is a 645b question, from F's observation.
7. **`--profile` typing.** `pane/switch.rs:97-101` can call #643's `list::profile_name` when 645b edits this file (A-dup
   warn 2).
