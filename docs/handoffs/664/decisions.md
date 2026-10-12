# Decision journal: #664 `holler profile apply`

## A (Phase 3, up-front plan review): 2026-10-11 07:00 MDT
- **Decided:** PASS, with 0 block and 9 warn findings (`docs/handoffs/664/handoff-A.md`). The layering and placement are right: the engine `tx_apply` is in `holler-pane` and works only through `Ports`, the verb is in `holler-cli`, and `tx_launch` is called unchanged. The twins are an operator-decided, justified duplication. The warns:
  1. Take `origin/main` (#808) before T.
  2. Name and follow `reconcile::observe_all`'s fan-out shape, and define a lost thread.
  3. Name `diff_profile` as the comparison.
  4. Carry the reconcile step in an acted failure, and quote and escape stored strings.
  5. C5 overstates what the re-read fences, and a row chosen up front can be stale by the time its step runs.
  6. Extend the profile rig additively, modelled on `launch/rig.rs`.
  7. Fill the blast-radius gaps (`profile_verbs/apply.rs`, §10's list, `tx_apply/` for overflow).
  8. Declare `orchestrator-not-relaunched` as a `RefusalCode` constant, list it in ADR-0021 section 9's apply row, and define "adopt" against the existing "never adopted" text.
  9. Name the step vocabulary (`action`, the dry-run `outcome`, the per-pane `usage` code, the step for an invalid spec name).
- **Assumed:** The brief's operator section (§1 to §13 and the decisions on O1 to O6) is final and outside A's remit, so findings that touch it (#5(b), #9) are framed as choices for the operator, not drift. No `survey.md` or Reuse map exists for this run, so the handoff's "Reuse expectations for Phase 7" stands in for one.
- **Hedged:** I did not block on the base having moved (#1). C9 already prescribes the label-free `(session, pane_id)` comparison that ADR-0021 now requires, and `parallel-agent-coexistence.md` expects a rebase. But the pipeline does not sync the branch with `main`: its diff gates use `origin/main...HEAD`. Unless someone merges `main` before T, the labelled-pane test #1 asks for cannot be written, and an equality-based check would pass every test on this base. My estimate that `tx_apply.rs` will pass 600 lines (#7(c)) is a judgment, not a measurement.
- **Evidence:** I read the whole brief (2657 lines). I read `reconcile.rs` and `reconcile/observe.rs`; `profile_diff.rs`; `profile/show.rs`, `profile/mod.rs` and `profile/apply.rs`; `pane/launch.rs` (`failure_body`), `pane/relaunch.rs` and `pane/profile_scope.rs` (`reconcile_step`); `output.rs` (`ErrorBody`); `error.rs` (open codes, `class_of`); `tx_switch.rs`; and `tx_launch.rs` (`run`, `stop_old`). On the test side: `tests/profile_verbs/{main,apply,rig}.rs`, `tests/pane_verbs/launch/rig.rs`, the `park` and `close` rig docs, the test kit's `host.rs`, `harness.rs` and `profile_scope.rs`, `cli-surface.txt`, ADR-0003:70, and ADR-0021 section 9 (515-531) and its uses of "adopt". I also ran `git log HEAD..origin/main` (#808 is `263a2d7`) and `git diff HEAD...origin/main` on `pane.rs`, `ports.rs`, `tx_launch.rs`, `holler-pane-testkit/src/herdr.rs` and `ADR-0021.md`, and grepped for `thread::scope`, `RefusalCode::from_static`, leave/join writes in `holler-pane`, and `--dry-run`.

## T (Phase 4, author / RED): 2026-10-11 07:20 MDT
- **Decided:**
  - **The merge (A warn 1).** I merged `origin/main` (`263a2d7`, #808) into the branch with `--no-commit`
    before writing tests. The driver's phase commit completes the merge, so T commits nothing.
  - **The tests.** 38 tests in `crates/holler-cli/tests/profile_verbs/apply.rs` and `apply/*.rs` cover every C14
    entry (1 to 33), C8, C6, and A warn 1's labelled pane. Each runs on the engine (`tx_apply::apply`, with steps
    read from its result, per O6) or on the in-process verb. `assert_stopped` checks O6's output at exit 1.
  - **The API.** The tests pin `apply(ports, &ApplyRequest, &TxOptions) -> Result<Applied, PaneError>` and the
    `Step`, `Action` and `Outcome` types. `Outcome`'s JSON strings come from §5. `Action`'s strings are F's
    (compared through serde).
  - **The rig.** It is additive (A warn 6): `apply/rig.rs` copies `launch/rig.rs` by name, and the read verbs' rig
    gains only `serve_server` (O3's one seeding helper).
- **Assumed:**
  - **No regression of A warn 1.** The pair comparison is checked by a labelled pane and by another-session cases.
  - **The pane-exists seam.** Its timing assumes C1's order: step 1's registry read comes before the observation,
    and the step's re-read is the first read after it.
  - **Dry-run outcomes for rows b, c and f.** They are unpinned beyond "no effect claimed", since the brief does not
    name them (A warn 9).
- **Hedged:**
  - **The prototype.** It proves the suite can be satisfied as written, but it is mine, not F's. F's design may meet a
    seam assumption differently (the pane-exists timing). Each such assumption is named in the handoff's RED notes.
  - **Timing.** Entries 26 and 27 rely on harness delays. Their margins are 3 s (26), and 300 ms on a 900 ms budget
    (27). They were stable over 15 runs here.
- **Evidence:**
  - **Compile RED.** 10 errors, all `E0432`/`E0425` on `holler_pane::tx_apply::{apply, Action, Applied, ApplyRequest,
    Outcome, Step}`.
  - **API-only stub.** 43 passed, 38 failed, all on assertions.
  - **Prototype.** 81 passed, over 5 runs of the target and 10 runs of `apply::`.
  - **Mutants.** 16, each failing at least one test (table in `handoff-T-red.md`).
  - **Lint.** `cargo clippy -p holler-cli --test profile_verbs -- -D warnings` is clean, and `bash scripts/lint.sh`
    exits 0 (`apply/rig.rs` warns at 742 lines).
  - **Source.** `src/` is identical to `origin/main` after every throwaway edit (`git diff origin/main --stat --
    'crates/*/src'` is empty).

## F (Phase 6, implement): 2026-10-11 07:55 MDT
- **Decided:**
  - **The engine.** `tx_apply.rs` holds the module doc and the bound, the API T pinned, the gate, the plan and the
    stop loop. `tx_apply/observe.rs` holds the fan-out and §2's judgment. `tx_apply/run.rs` holds one live step, the
    take-over move and the twins. `tx_launch.rs` is unchanged.
  - **The observation** runs one scoped thread per call (the snapshot, and each recorded pane's `health` and
    `shown_session`), so it takes one per-call bound, which is §9's 10 s, rather than two.
  - **Rows are chosen once, at planning.** A step acts on its re-read record and on P's current spec. Rows a and e
    re-read nothing.
  - **Classification goes through `class_of`.** `usage` for a step's spec becomes the open code `spec-not-launchable`.
    An observation error that is not failure class becomes `unavailable`, so a failed step always exits 1.
  - **The vocabulary.** Actions are `create`, `adopt`, `report`, `relaunch`, `refuse` and `observe`, and a dry run's
    acting step is `planned`. The step JSON adds `detail` (C12/O2's "the step's detail"), and absent keys print
    `null`.
  - **Reuse.** The verb reuses `show`'s `difference_line` and `text`, made `pub(crate)`, and `reconcile_step`.
  - **Docs.** ADR-0021 section 9's apply row and "Apply as built (#664)", which defines "adopt"; ADR 0003's row;
    `cli-surface.txt`; the CHANGELOG; and one inline span in `docs/research/herdr-api-spike.md`, without which
    `docs_cli_test` fails once `NAME` is required.
- **Assumed:**
  - **C3, literally.** A snapshot error fails every recorded pane that row a does not refuse.
  - **The row e test uses the record's role**, as §3 says ("record, not healthy, role orchestrator").
  - **A spec whose pane is not a pane name** is a planning refusal (action `refuse`). It never trips §4's gate.
- **Hedged:**
  - **The follow-up issues of §9 and §13 are not filed.** CONTRIBUTING.md makes filing an issue an assertion of
    human review. Both issues are drafted in `handoff-F.md` for the operator or the run's agent.
  - **`stub.rs`'s `STUBS` still lists `profile apply`.** That is a test file, so T must delete it. Until then the two
    `pane_cli_process` stub tests fail.
  - **Two warns stay open, as the brief leaves them** (Known issues): a row-f pane that recovers before its step is
    still relaunched (A warn 5(b)), and the stale-P window after a re-read remains (A warn 5(a)).
- **Evidence:**
  - **`profile_verbs`.** 81 of 81 pass (T's 38 apply tests and the 43 existing), over 5 runs.
  - **Lint and format.** Workspace clippy with `-D warnings` is clean, `scripts/lint.sh` exits 0, `changelog-check`
    is ok, and rustfmt is clean on every changed file.
  - **`cli_surface_test` and `docs_cli_test`** pass.
  - **`pane_cli_process`.** 34 of 36 pass; the 2 failures are the `STUBS` entry.
  - **The whole workspace.** See `handoff-F.md`'s Tier 1 output.

## T (Phase 7, verify / GREEN): 2026-10-11 08:45 MDT
- **Decided:** I took F's flag on `pane_verbs/process/stub.rs` as a test fix: I deleted the `("profile", "apply", 664)` `STUBS` entry and kept the `// #664` line, as the file's doc requires. That was my only test change. The verdict is PASS with no blocking issue. I added two evidence entries for unchanged-code facts my tests rely on: `herdr.version` is called only inside `tx_launch`'s observation, and the split-only fake refuses an unreachable cell.
- **Assumed:** the repo-wide `cargo fmt --check` drift is not this branch's concern. None of the files that drift is changed by the branch, and CI gates on `lint.sh`, not `cargo fmt`.
- **Hedged:** three cases stay untested: an invalid `PaneName` spec, a record that moves to another profile mid-run without `--take-over`, and duplicate specs. They are advisories for S, not blockers, because the brief does not pin them (T-red's "Not pinned").
- **Evidence:**
  - The whole workspace passes with `--no-fail-fast`: 2125 passed, 0 failed.
  - `profile_verbs` passed 81/81 on 10 of 10 runs, and `pane_cli_process` passed 36/36.
  - These are clean: clippy `-D warnings`, `lint.sh`, `changelog-check`, `cargo machete`, `docs_cli_test`, `cli_surface_test`, `wire_selftest`, `test-hooks.sh`, `pipeline-adoption-test.sh`, and rustfmt on the changed files.
  - Five mutants of F's code were all killed, each reverted with `git checkout`.
  - I compared the twins `leave` and `join` in `run.rs` with `profile::delete::detach` and `profile::create::join` by reading both.

## A (Phase 3 re-entry, up-front plan review, round 2): 2026-10-11 08:14 MDT
- **Decided:** BLOCK, with 1 block and 4 warns (`docs/handoffs/664/handoff-A.md`).
  - **The block.** The brief gives the up-front observation two shapes that cannot both hold:
    - §9 bounds it by one I5 per-call bound ("10 s plus N step bounds").
    - C1 step 3 and C4 put each pane's `health` and shown session on one thread, which makes a chain of two per-call bounds.
  - **How it surfaced.** F ran one thread per call to keep §9, and the outside diff gate blocked that on "one thread per pane". No code meets every sentence, so another T/F cycle would end in a block, so the brief must change.
  - **The amendment I recommend (option 1).** An O-level clarification that applies the brief's precedence rule (§9 wins) and keeps F's per-call shape, with ready-to-paste text in Notes for O. Option 2, the per-pane chain, needs the operator, because it changes §9's arithmetic.
  - **The warns:**
    1. `observe.rs`'s doc claims `observe_all`'s shape but has only its mechanism.
    2. Name the step vocabulary in the same amendment, because §5 calls its outcomes exhaustive without `planned`.
    3. File §9's and §13's follow-ups before the PR.
    4. Round 1's warn 5 stays open for the operator.
- **Assumed:**
  - **How the run got here.** It reached A through `nextPhase(7, BLOCK)`, with F's `archChanged` true: the diff gate ran at 07:59, after T-green at 07:58. The task's "before any code exists" is the script's fixed wording, so I reviewed the plan with F's code as evidence.
  - **What my BLOCK does.** On the unchanged brief, the script stops with `needs-amendment`, which is the route for a brief defect. A PASS would have sent T and F into a cycle the brief cannot settle.
- **Hedged:**
  - **The recommendation.** I judged the per-call fan-out sound and recommended keeping it, because §9 is the operator's text and a pane's two reads are independent. The per-pane chain is the house shape (`reconcile`), and choosing it is the operator's call.
  - **What I cannot predict.** I cannot say what the outside gate will make of an amended brief.
  - **My own miss.** My round-1 PASS missed this contradiction.
- **Evidence:**
  - **Reviews and run state.** I read `664-diff-result-r1.md` whole, and its usage file (model `deepseek-v4-pro`). I read the script's phase-4 entry, `nextPhase`, `decideArchReReview`, `buildArchStop` and `RESUMABLE_PHASES`.
  - **The brief.** I read §1 to §13, C1 to C20, O1 to O6 and the decisions on them. I grepped every "thread", "chain" and "slowest" in it. It is unchanged since `7bd8242` (`git diff --stat` is empty).
  - **The code.** I read F's `tx_apply.rs`, `tx_apply/observe.rs`, `tx_apply/run.rs` and `profile/apply.rs`, and the diffs of `show.rs`, ADR-0021, ADR-0003, `cli-surface.txt`, the CHANGELOG and `profile_verbs/rig.rs`. I read `reconcile.rs`'s `observe_all` and `reconcile/observe.rs`'s chain, ADR-0021 sections 2 and 12 on I5, `diff_profile`, `PaneName::parse`, and the profile name's text-form notes.
  - **The tests.** I read `apply/observe.rs`, `apply/bound.rs`'s timing tests (entries 26 and 27 time only unrecorded panes) and entry 19's hook. Neither thread shape breaks a test.
  - **Untouched shared code.** `tx_launch.rs`, `lib.rs` and the Cargo files are unchanged on the branch.

## A (Phase 3, up-front plan review, round 3, fresh run): 2026-10-11 10:40 MDT
- **Decided:** BLOCK: 1 block and 5 warns (`docs/handoffs/664/handoff-A.md`).
  - **The block.** The merge `780a578` (origin/main `7284001`, #802) also ran `cargo fmt` at the workspace root.
    - It rewrote 183 files outside the Blast radius, in holler-body, holler-cli, holler-hub, holler-load-test and holler-proto. The rewrite is in neither merge parent.
    - `scripts/lint.sh`, CI's first gate, now fails three checks: 15 files of 900 lines or more, 91 `#[allow]` lines without a link, and 1 `dead_code` allow without a link.
    - The diff every gate reads grows from 415 KB over 27 files to 1.86 MB over 210 files.
    - F and T may not undo it, so it must be undone before T. The fix is one `git checkout origin/main --` of those paths, plus a C24 clarification that scopes "`cargo fmt`".
  - **The design passes.** C21 settles round 2's block. #802 leaves the plan's architecture intact.
  - **The warns:**
    1. #802 made refusals possible during the act, and apply then drops the reconcile step on that `refused` step.
    2. #644 part 2 supersedes §13, and claims that #664 owns the test for an agent difference. Neither brief owns it.
    3. C23 has F cite a twins issue that no one has filed yet.
    4. C22 says ADR-0021 names the vocabulary, but it does not yet.
    5. The Evidence was cut at `92fbc6e`, before #802. Separately, entry 27's timing margin dropped from about 300 ms to about 100 ms (eight harness calls per create, not six).
- **Assumed:**
  - The operator's §1 to §13 and the decisions on O1 to O6 are final, so I framed the #802 effects as clarifications, not as changes.
  - I read the merge commit's message as the MO's own merge, not a pipeline phase.
  - My inference: the bare "`cargo fmt`" in the brief's Rules line set off the root reformat.
- **Hedged:**
  - I did not run cargo test, so whether the branch passes its tests after the merge is T's to confirm.
  - Whether the 1.86 MB diff would overflow the outside model's context is not measured. I state only the size.
  - I counted harness calls per create statically from `tx_launch.rs` and `server.rs`.
- **Evidence:**
  - **The merge.** `git diff --name-only` of `d56707b..HEAD` intersected with `origin/main..HEAD` gives 188 paths, 183 of them outside the story. Each of the 183 is byte-identical to `rustfmt --edition 2021` of its `origin/main` version (183/183). `git diff --name-only 263a2d7 d56707b` touches none of them.
  - **Lint.**
    - On HEAD, `bash scripts/lint.sh` exits 1.
    - On a `git archive` of `origin/main`, it exits 0.
    - On HEAD with the 183 files restored from `origin/main`, in a scratch copy, it exits 0.
    - A `wc -l` comparison shows 15 files go over 900 lines; `main` has none.
  - **Files read.**
    - The whole brief, with a focus on §1 to §13, C1 to C23 and O1 to O6. I spot-checked Evidence E1 to E20.
    - Round 2's `handoff-A.md`, and `decisions.md`.
    - F's `tx_apply.rs`, `tx_apply/observe.rs`, `tx_apply/run.rs`, `profile/apply.rs` and `apply/bound.rs`.
    - #802's diffs of `ports.rs`, `lib.rs`, `tx_launch.rs`, `tx_launch/server.rs`, the testkit's `harness.rs` and `host.rs`, and ADR-0021.
    - The merge's changes to `apply/rig.rs`, `apply/rows.rs` and `profile_verbs/rig.rs`.
    - `pane/launch.rs` (`failure_body`) and `pane/profile_scope.rs` (`reconcile_step`).
    - ADR-0021 :150-166, :392-432, :538-575, :648 and :655-664.
    - `644-brief.md` (Blast radius, items 2 to 4 and 7, and the Rules line) on the `issue-644-implementation` worktree, and `642-brief.md`'s Blast radius.
  - **GitHub.** `gh issue list` searches found no filed twins or §13 follow-up.

## A (Phase 3, up-front plan review, round 4, fresh run): 2026-10-11 10:58 MDT
- **Decided:** PASS: 0 blocks and 4 warns (`docs/handoffs/664/handoff-A.md`).
  - **Round 3's block is fixed.** `3c57c10` restored the 183 files, `lint.sh` exits 0, the diff against `origin/main` is the story's 27 paths, and C24 scopes formatting to the story's own files.
  - **The design holds.** C25 adds the reconcile step for an acted refusal in the verb, which is where `pane launch` adds it.
  - **The warns:**
    1. E40's apply-row excerpt is empty, and its cited line is stale (`530`; the row is at `620` on `7284001`).
    2. #644's brief was not amended to match C18's order clause. If #664 merges first, no one writes the agent-difference test. The fix: extend entry 11's fixture, which passes both before and after #644.
    3. The Blast radius still omits `tx_apply/{observe,run}.rs`, which C21 and C23 tell F to edit, and five other paths the branch changes.
    4. C25's reconcile step is missing from C22's ADR sentence and from `Step.acted`'s doc.
- **Assumed:**
  - The operator's §1 to §13 and the decisions on O1 to O6 are final.
  - The fresh run's T and F start from the first run's code, as round 3's Notes for O item 4 planned.
  - #664 is likelier than #644 part 2 to merge first, since #664's code exists and #644 part 2 has none.
- **Hedged:**
  - I did not run `cargo test`. Whether the branch is green after the merge and the restore is T's to confirm.
  - I counted the harness calls of a `create` from the code, not from a run.
  - E40's harm is small, because the ADR diff shows the row, so I rated it a warn.
- **Evidence:**
  - **The branch.**
    - `git ls-remote` shows `origin/main` is `7284001`.
    - `git diff --name-only origin/main...HEAD` lists 27 paths. Each of the 183 paths `3c57c10` restored equals `origin/main`.
    - `af230b6` changes only the brief.
    - `bash scripts/lint.sh` exits 0, and `rustfmt --check --edition 2021` passes on all 15 of the story's `.rs` files.
  - **The brief.**
    - I read §1 to §13, C1 to C25, O1 to O6 and the decisions, and the whole diff `d56707b..af230b6`.
    - A script checked every Evidence code line against its cited files at `7284001`. All of them matched, except E40's empty block.
    - C11's line numbers (198, 332, 376, 402, 516) match.
  - **The code.**
    - F's `tx_apply.rs`, `tx_apply/run.rs`, the head of `tx_apply/observe.rs` and `profile/apply.rs`.
    - `tx_launch.rs` (`observe`, `bring_up`, `session_of_record`, `observe_live`) and `tx_launch/server.rs` whole.
    - `pane/launch.rs` (`failure_body`), `pane/profile_scope.rs` (`reconcile_step`) and `error.rs` (`class_of`).
    - The test kit's `HarnessOp` and its fake `verify_serving`; the OpenCode adapter's `verify_serving`.
    - Entry 11's test (`apply/dry_run.rs:115-175`).
  - **The ADR.** ADR-0021 :548-576 and :640-665 on HEAD, and :620 on `origin/main`.
  - **Other stories.**
    - #818 via `gh issue view`.
    - `644-brief.md` at `93b2cd7`: items 2, 4 and 7, its Blast radius, and its Rules.
    - What each in-flight branch changes, and #652's references to `tx_apply`.
    - `gh pr list` shows no PR yet for #644 part 2.

## T (Phase 4, author / RED, fresh run after A round 4): 2026-10-11 11:15 MDT
- **Decided:**
  - **One RED test.** `a_refusal_after_the_act_began_names_the_reconcile_step` (`apply/rows.rs`) covers C25. It runs `fail_next(VerifyServing, Refused)` on a `create` and on a row f relaunch. Each step is `refused` with `agent-mismatch`, the run goes on and exits 0, and the step's `detail` names `reconcile_step(Some(P))` in JSON and in text. Row a's refusal does not.
  - **Pins.** These cover decisions the first run's code already meets but no test checked:
    - C25: a `failed` acted step's message names the reconcile step.
    - C21: a pane's `health` and `shown_session` are in flight together. This is an invariant based on the call log, not a duration.
    - C22: the six action strings, the exact step keys with `null` for an absent key, `planned`, `spec-not-launchable`, and `observe`.
    - A round 4 warn 2: entry 11 gains an agent-difference pane that holds before and after #644 part 2.
  - **Timing.** Entry 27 is re-derived to 100 ms per call and a 1200 ms budget. The run measured 1.60 s, which is 16 calls and confirms eight per create. Entry 26 keeps its numbers (about 1.0 s against a 3 s bound); only its doc changed.
  - **No new file and no rig change.** Every edit is in a path on A's 27-path list.
- **Assumed:**
  - F's first-run code is the base this cycle builds on (A round 4's assumption), so a test of an already-met decision is a pin, not an invalid RED.
  - The reconcile step goes on an acted step's `detail` at the verb, not in the engine's `Step.detail`, because `reconcile_step` belongs to `holler-cli` (C25). So the RED is asserted at the verb, and the engine half asserts only the outcome and code.
- **Hedged:**
  - The C21 test relies on a 400 ms delay being far longer than a thread's start-up. It passed 10 of 10 runs and failed a chained-observation mutant.
  - I left the doc-only items to S rather than grep for their wording: C21's `observe.rs` doc, C22 and C25's ADR sentence, `Step.acted`'s doc, and C23's `#818`.
- **Evidence:**
  - **Before my edits:** 81 of 81 passed.
  - **After:** 83 passed and 1 failed. The failure is on the C25 detail assertion, and it was the same in 10 of 10 runs of `apply::`.
  - **Throwaway prototype in the verb:** 84 of 84 passed.
  - **Mutants M1 to M4:** each failed at least one new test.
  - **Tier 1 on the test files:** clippy `-D warnings` is clean, `rustfmt --check` passes, and `lint.sh` exits 0.
  - **Files read:** the brief's C18 to C25, `handoff-A.md` round 4, `tx_apply.rs`, `tx_apply/observe.rs`, `profile/apply.rs`, `tx_launch/server.rs:85-107`, the test kit's `fault.rs` and `harness.rs`, and `pane/profile_scope.rs` (`reconcile_step`).

## F (Phase 6, implement, fresh run after A round 4): 2026-10-11 11:22 MDT
- **Decided:**
  - **C25, the RED.** The verb adds the reconcile step in one place: a `reconciled` pass that ends the detail of every `acted` step, `refused` or `failed`, with `reconcile_step(Some(P))`. It appends after `"; "`, and only when the detail lacks it, as `failure_body` does. `stopped` no longer appends it, since the failed step's detail already carries it. The engine only marks `acted`, because `reconcile_step` is `holler-cli`'s.
  - **The docs the amended brief gives F.**
    - `Step.acted` and the engine's "Outcomes" bullet cover `refused` steps (A round 4 warn 4).
    - `observe.rs` names `reconcile`'s mechanism at one thread per call, and drops "the shape of `observe_all`" (C21).
    - `run.rs` cites #818 (C23).
    - One ADR-0021 sentence names the step object, the six actions, the outcomes, `null` for absent keys, the step's own `detail` against "the envelope has no `detail`", and the reconcile step on an acted step (C22 and warn 4).
    - The CHANGELOG gains one clause.
    - The engine's doc says a `PaneName` is checked text. That answers the first diff gate's W on `other_profile`.
  - **Evidence.** `evidence.md` is re-cut to HEAD, since #802 moved `tx_launch.rs`. That includes T's two entries, with their wording kept and a note added. Four C25 facts are added.
- **Assumed:**
  - An acted step always has a detail, because `halted` sets it. The pass still falls back to the error's text, and then to the step alone.
  - T's three new pins (C21, C22, the acted `failed` step) pass on the first run's code, as T-red reported. They needed no change.
- **Hedged:**
  - I edited T's two evidence entries, line numbers and one excerpt only, so the diff gate's file:line attachment matches. T should re-verify them at Phase 7.
  - A round 4's warns 1 and 3 (E40, the 27-path list) are brief edits for O, and warn 2 is the MO's. I made none of them.
  - The first run's open points (a row chosen once, the stale-P window, duplicate specs) stay open, as C23 says.
- **Evidence:**
  - **The RED, then GREEN.** `profile_verbs` went from 83 passed and 1 failed (the C25 test) to 84 passed, over 7 runs.
  - **The whole workspace.** `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` exits 0: 148 binaries, 2157 passed, 0 failed.
  - **Other targets.** `holler-pane` passed 108, `docs_cli_test` 3, `cli_surface_test` 3 and `pane_cli_process` 36.
  - **Lint and format.** Clippy `-D warnings` over the workspace and all targets is clean. `lint.sh` exits 0, `changelog-check` is ok, `rustfmt --check` passes on the story's files (no `cargo fmt`, C24), and rustdoc `-D warnings` on `holler-pane` is clean.
  - **The excerpts.** A script found every line of all 15 evidence excerpts verbatim at its cited lines.
  - **The paths.** `git diff --name-only origin/main` still lists exactly A's 27 paths.
  - **Files read.** The brief's C1 to C25 and the decisions, `handoff-A.md` round 4, `handoff-T-red.md`, the RED test, `pane/launch.rs` (`failure_body`), `pane/profile_scope.rs` (`reconcile_step`), `tx_launch.rs` (`TxFailure`, `run`, `act_live`, `bring_up`, `observe`), `tx_launch/server.rs` (`start`), `reconcile.rs` (`observe_all`), `holler-proto`'s `SessionName` grammar, the OpenCode adapter's `verify_serving`, and `gh issue view 818`.

## T (Phase 7, verify / GREEN, fresh run after A round 4): 2026-10-11 11:28 MDT
- **Decided:**
  - **GREEN, with no test repair.** F listed no test as wrong, and I found none. No test file changed, and no production code was touched.
  - **The spot-check targets F's real C25 code.** I used five mutants of `reconciled` and `stopped`, not the RED prototype, and each was killed.
  - **One evidence entry.** It covers the fault switch's log-before-delay fact, the one F left for T, which the C21 test relies on. F's other 15 entries stand. I checked three C25 excerpts against source myself.
- **Assumed:**
  - The first run's five engine mutants still speak for the engine, because this run changed only docs there (`git show 0b26110` touches `tx_apply.rs`, `observe.rs` and `run.rs` in comments only).
  - `origin/main`'s move to #820 during this phase is the PR step's to merge, not T's. It overlaps only in `CHANGELOG.md`, and `git merge-tree` is clean.
- **Hedged:**
  - The "already contains" guard in `reconciled` has no test. It is unreachable from the engine today, and it mirrors `failure_body` as A-dup expectation 6 asks. I recorded it as advisory, not blocking.
  - The doc-only items (C21, C22, C23, C25's ADR sentence, `Step.acted`'s doc) are present in F's diff. Their wording is S's to audit.
- **Evidence:**
  - `profile_verbs`: 84 of 84 passed, 10 of 10 runs, under load.
  - Workspace verify command: exit 0. 148 binaries, 2157 passed, 0 failed and 25 ignored, identical to F's counts.
  - Clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test`, `cli_surface_test`, `wire_selftest`, `pane_cli_process`, `test-hooks.sh`, rustdoc `-D warnings`, and `rustfmt --check` on the story's files: all clean.
  - Mutants M1 to M5 were all killed, and the restored tree is GREEN.

## A (Phase 7, anti-duplication gate, fresh run after A round 4): 2026-10-11 11:36 MDT
- **Decided:** PASS, with 0 block and 2 warn findings (`docs/handoffs/664/handoff-A-dup.md`).
  - F extended the objects the plan named and built no parallel path. `tx_launch` is called unchanged. The plan is `diff_profile`'s rows, outcomes go through `class_of`, and the verb reuses `reconcile_step`, `list::count` and `show`'s helpers.
  - The twins match `detach` and one pane's `join` write line for line, and #818 is open and tracks them.
  - All nine of A round 4's reuse expectations are met.
  - The two warns:
    1. `joined`, three call-name constants and a third spelling of the `(session, pane_id)` join key are copied from `reconcile`/`tx_launch` and not tracked.
    2. `apply/rig.rs` re-creates about 176 verbatim lines of `launch/rig.rs`, and that copy has begun to drift (`pid`). No issue tracks the consolidation.
  - Both warns are follow-up work, not changes in this story.
- **Assumed:**
  - The rig copy counts as a reviewed decision, not drift. A round 1 warn 6 directed it, it is the house pattern (the `park` and `close` rig docs), and the two rigs are in different test binaries.
  - Copying `joined` is incidental to the separate fan-out that C21 justifies. Sharing it would mean editing `reconcile.rs`, which is outside the Blast radius.
- **Hedged:**
  - I did not run the test suite. T-green ran it on the same head (`8ed5f8c`), and nothing has changed since.
  - The 176-line overlap is a mechanical count: normalized, non-trivial lines of `apply/rig.rs` that occur verbatim in `launch/rig.rs`. It undercounts the lines that differ only in `self.x` against `self.fakes.x`.
- **Evidence:**
  - **The diff.** `git diff --name-only 7284001..8ed5f8c` lists exactly the 27 paths. `bash scripts/lint.sh` exits 0, and the story's only warning is `apply/rig.rs` at 759 lines.
  - **Shared code.** The diff of `tx_launch*`, `reconcile*`, `lib.rs`, `profile/mod.rs` and every Cargo file is empty.
  - **Searches.** grep finds no I/O, `unsafe` or `#[allow]` in the engine or the verb, no hub harness names, and no personal infrastructure names in the diff.
  - **Issue #818.** `gh issue view 818`: open, and its body carries C10's clause.
  - **Files read.** `tx_apply.rs`, `tx_apply/{observe,run}.rs` and `profile/apply.rs` in full, and the diffs of `show.rs`, `profile_verbs/rig.rs`, `process/stub.rs`, `cli-surface.txt`, ADR-0003, ADR-0021, the CHANGELOG and the spike doc. Against them:
    - `reconcile.rs` (`observe_all`, `joined`, `missing_herdr_panes`);
    - `tx_launch.rs` (`same_pane`, `TxFailure`, `Launched`);
    - `profile/delete.rs` (`detach`) and `profile/create.rs` (`join`);
    - `pane/launch.rs` and `pane/close.rs` (`failure_body`), and `pane/profile_scope.rs` (`reconcile_step`, `StoreScope::stored`);
    - `profile_diff.rs` (`diff_profile`, `is_member`) and `tx_switch.rs` (its open codes);
    - `apply/rig.rs` and `launch/rig.rs` in full, and the headers of the `park`, `close` and `doctor` rigs;
    - the brief's §1 to §13 and C1 to C25, and handoff-A rounds 1, 2 and 4, handoff-F and handoff-T-green.
