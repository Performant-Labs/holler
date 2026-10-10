# Decision journal: #645 switch-reset

## A (up-front plan review) — 2026-10-09T19:51-06:00
- **Decided:** BLOCK, on one block finding (handoff-A.md):
  1. Decision 16 loads `tests/pane_verbs/doctor/rig.rs` a second time with `#[path]` from `switch.rs`, while
     `doctor.rs:10` already loads it as `mod rig;`. Clippy's `duplicate_mod` then fails AC 25 and CI
     (`cargo clippy --workspace --all-targets -- -D warnings`). This also departs from the crate's pattern of one
     declaration reached by crate path (#662's `crate::list::rig`). The brief rules out both editing `doctor.rs` and
     copying the rig, so O must choose between a named one-word visibility edit in `doctor.rs` (preferred) and a
     justified `#[allow(clippy::duplicate_mod)] // #645`.
  Six warns:
  - P5's one-pane-per-session rule is a read-check with a race against a concurrent switch, and the hub store does not
    enforce it.
  - P3's message spells `holler pane relaunch` outside the remedy table.
  - The as-built paragraph and the `unavailable` decision would land in section 11, while #644 records the same
    decision in section 8.
  - The reconcile-step lead-in is spelled in both `holler-pane` and #663's `holler-cli` function.
  - Forward-compat should name `SERVER_UNHEALTHY`, `ORCHESTRATOR_PANE` and `parse_session_id` for reuse (ADR row 345,
    646c).
  - P1 indexes `panes[0]`.
  The rest of the plan matches the codebase: one pure engine over `Ports`, refusals before any live change, live
  health, a clone-and-set single compare-and-swap, no Herdr or host call, reuse of `doctor_command`, `quoted` and
  `shown_differs`, and open codes declared with `from_static`. `unavailable` and the CLI pairing and output shape
  match #644. 645b's prompt goes through `send_prompt`.
- **Assumed:**
  - The sibling briefs on their worktrees are the plans those runs will implement: `issue-644-implementation` at
    `7195993`, `issue-646-implementation` at `e957a8d`, `issue-643-implementation` and `issue-663-implementation` at
    `137c00f`.
  - #647 part 2 (still open) will keep owning `tests/pane_verbs/doctor.rs`.
  - `origin/main` moved from `ce12cdb` to `dc300ab` (#640 part 2, #642 part 1). Among the files this story touches,
    that changed only `CHANGELOG.md`, so the brief's evidence still holds.
- **Hedged:**
  - Finding 1's choice between (a) and (b) is O's, because it trades the brief's no-#647-file rule against a lint
    suppression. I recommend (a) as the established pattern but did not decide it.
  - Finding 2 is a warn, not a block. The race is narrow (two concurrent switches to one session), no merged pattern is
    broken, and the fix is a store change outside 645a.
  - Findings 4 and 5 depend on the merge order of #644 and #663, so the fix is phrased as "the second to land cites the
    first".
- **Evidence:**
  - The whole brief (1,120 lines), and the outside brief review r1 with its usage file.
  - ADR-0021: sections 1-5, 7-9, 11, 12, "Deferred" and "Decisions taken". ADR 0003's pane rows.
  - `holler-pane/src/{lib,ports,error,findings,reconcile,generation,probe}.rs`, `reconcile/observe.rs`, and the `tx_*`
    stubs.
  - `holler-cli/src/pane/{doctor,mod,wiring}.rs` and `output.rs`.
  - `tests/pane_verbs/{main,doctor}.rs`, `doctor/rig.rs`, `doctor/surface.rs:456-514`, and `tests/profile_verbs/` (the
    rig pattern).
  - The test kit's harness and pane-store API.
  - `holler-hub/src/{holds.rs,panes/store.rs}` and `circuit/dispatch.rs`.
  - The OpenCode adapter's module doc (#642 part 1, now on `main`).
  - Issues #645 and #647. `Cargo.toml` lints, `ci.yml:280` and `scripts/lint.sh`.
  - The sibling briefs and handoffs: 644 brief and handoff-A, 646 brief, 643 brief, 663 brief.
  - Greps for open codes, `--as-operator`, session-id validators and `HarnessPort` wrappers.
  - A scratch-crate reproduction of `duplicate_mod` on clippy 0.1.98 / rustc 1.98.1, in the session scratchpad, outside
    the repo.

## A (up-front plan review, round 2) — 2026-10-09T20:14-06:00
- **Decided:** PASS on the brief as amended in `8cf4f00` (handoff-A.md, which replaces round 1's; round 1 stays in git at
  `2bbb237`).
  - Round 1's block is fixed by option (a). `doctor.rs:10` becomes `pub(crate) mod rig;`, and `switch.rs` and `reset.rs`
    reach the rig as `crate::doctor::rig`, the `profile_verbs` pattern on `main`. I verified it on a built copy of the tree.
  - All six round-1 warns are written into the brief: P1, P3 and O1 wording, R-5, F-3, F-4, Decision 17 in section 8,
    AC 22 and AC 24, and the Forward-compat rows. The amendments add no drift.
  - Two new warns, neither blocking:
    1. O1's private copy of `screen_text` has no follow-up to fold it. Widen F-4 to cover the screen wording.
    2. Decision 17(c)'s "Deferred" bullet credits #644 if #645 lands first. Write "#644 to follow" in that case.
- **Assumed:**
  - The sibling plans are the ones on their branches now: `issue-644-implementation` at `7195993`,
    `issue-646-implementation` at `b996a8f`, and `issue-663-implementation` at `e46b427` (its A gate passed). #647 part 2
    is not briefed in the repo, and still owns `tests/pane_verbs/doctor.rs` and `reconcile*.rs`.
  - `origin/main` moved from `dc300ab` to `e327569` during the review. That commit (#641) touches only
    `holler-adapter-host`, `Cargo.lock` and `CHANGELOG.md`, so the brief's evidence holds.
- **Hedged:**
  - Both new findings are warns. The `screen_text` copy is justified in writing, which is a PASS at Phase 7, and round 1
    accepted it. The bullet wording is a matter of accuracy in the standing spec, not a contradiction, because the rule
    stated in section 8 is general.
  - Giving F-3 a "before #654" ordering, as P2 does for 645b, is offered to O as optional, not raised as a finding.
  - That P0 to R in one function would exceed `cognitive_complexity` 15 is an estimate. It is passed to F as a note.
- **Evidence:**
  - The whole amended brief (1,253 lines), the deleted lines of the amendment diff, round 1's handoff, and the outside
    brief review r1.
  - A scratch copy of `HEAD` built outside the repo, in its own seeded `target/`, with the one-word edit and probe tests in
    both verb files:
    - `cargo clippy -p holler-cli --test pane_verbs -- -D warnings` was clean.
    - 36 tests passed (the probes and all of doctor's).
    - The probes confirmed that the rig's items can be reached from a sibling file, AC 2's call-log sequences, that P's
      server lists Q's session (AC 4), and the single `stray-session` after a reset (AC 16).
  - Source read for this round:
    - `tests/pane_verbs/{main,doctor}.rs`, `doctor/rig.rs`, and `tests/profile_verbs/{main,list,show}.rs`.
    - `findings.rs` (the remedy table, `doctor_command`, `quoted`), `reconcile.rs` (`shown_differs`, `resolve`) and
      `reconcile/observe.rs:286-391`.
    - `error.rs` (`class_of`, `excerpt`, the variants), `lib.rs`, `pane.rs` (`PaneName`, `PaneId`), and the empty
      `tx_apply.rs` and `tx_launch.rs` stubs.
    - `holler-cli/src/pane/doctor.rs`, `output.rs` (its public items), and `cli.rs` and `profile/apply.rs` (the existing
      bool flags).
    - `holler-hub/src/panes/store.rs:160-182, 330-352`, `Cargo.toml` lints and `clippy.toml`.
    - ADR-0021: section 8, rows 336-346, section 11, "Deferred" and "Decisions taken". ADR 0003, lines 48-62.
    - `cli-surface.txt`'s #645 and #647 blocks.
  - The sibling briefs, grepped for codes, failure types, the reconcile step and the orchestrator and unhealthy handling:
    644 (`TxFailure`, `LaunchRequest`, decision 20), 646, 643, and 663 (decision 8, follow-ups F1, F2 and F5).

## T (author / RED) — 2026-10-09T20:23-06:00
- **Decided:**
  - RED is valid (handoff-T-red.md).
  - 19 behavior tests cover ACs 1-19: 13 in `tests/pane_verbs/switch.rs` and 6 in `reset.rs`. Each runs in both formats
    on fresh doctor rigs, reached as `crate::doctor::rig`.
  - The shared runner `both` asserts on every run that the exit code is the same in both formats, that the envelope is
    valid, that each format writes one line, and that there is no Herdr or host call (I4). The same runner serves AC 2's
    call sequences.
  - All 19 fail on the verb's answer: `not-implemented` with exit 1, where the test expects 0, 2 or 3, or another code.
  - AC 23 (help) passes in RED, because the brief puts the real `Args` structs in RED. It fails on `origin/main`.
  - ACs 20-22 are edited in RED, and their existing suites pass.
  - The `doctor.rs:10` edit is visibility only.
- **Assumed:**
  - The brief's test plan overrides the role doc's "T writes no production code" for the RED skeleton. That plan was
    reviewed by A and passed. The skeleton is the `tx_switch.rs` API with `not-implemented` bodies, and the two `Args`
    structs.
  - The CLI `run` bodies still emit `not_implemented(645)` instead of calling the stub engine. Calling it would mean
    writing step 1's parsing and `emit_outcome`, which are F's behavior. `Verb` and `emit_outcome` are left out because
    `dead_code` is denied.
- **Hedged:**
  - AC 1 sets `last_observed.driven` to `ses_driven` before the run (by `rig.rewrite`), so that "driven kept" is
    observable. The brief's literal case has `driven: None`, which no implementation could fail.
  - AC 17's session list is read through Q's server, which shares the data directory. P's killed server cannot be listed.
  - AC 5 also pins the P3 message's `run holler pane relaunch demo-c1r1`. AC 6 pins `--as-operator` in the refusal. Both
    strings are from the brief's Behaviour table.
- **Evidence:**
  - The brief (all of it) and handoff-A round 2.
  - The test-side sources: `doctor/rig.rs`, `verb_harness/{mod,parse}.rs`, the test kit's `harness.rs`, `envelope.rs`,
    `fault.rs` and `pane_store.rs`, and `fixture::sample_pane`.
  - The `holler-pane` sources: `findings.rs` and the `error.rs` variants.
  - The process suites: `process/{stub,flags,docs_rows}.rs`.
  - The commands, with their results:
    - `cargo test -p holler-cli --test pane_verbs`: 94 passed, 19 failed, each on the line 207, 212 or 221 assertions.
    - `pane_cli_process`, `cli_surface_test` and `docs_cli_test`: all green.
    - `cargo clippy --workspace --all-targets -- -D warnings`: clean.
    - rustfmt `--check`: clean. `scripts/lint.sh`: exit 0.
    - The real binary takes each AC 12 argv and answers `not-implemented`, not a clap error.

## F (implement / GREEN) — 2026-10-09T20:40-06:00
- **Decided:**
  - `tx_switch.rs` filled as the brief's table: plan (P0-P5, every refusal before any write or live change), act
    (`create_session` for reset, then `select_session`), observe (`shown_session`, a mismatch is `unavailable`), record
    (one `cas_put` of the clone-and-set four fields at P1's generation). No Herdr or host call.
  - The two verbs share one after-clap path, `switch::execute` (type `PANE` and `--profile`, then the target; run the
    engine; print through `emit_outcome`), so `reset.rs` is the `Args` struct and a one-call `run`, and copies nothing.
  - P1 takes the resolved pane whose name is the request's (`find`), not blindly the first (`next`): the same pane under
    `ProfileScope::resolve`'s contract, and `pane-not-found` rather than the wrong pane if a scope ever broke it.
  - ADR-0021: #644's paragraph is not on `main`, so the section 8 paragraph states the `unavailable` decision, and the
    "Deferred" bullet takes A's round-2 form ("#645 for switch and reset; #644 to follow for launch and relaunch").
  - The CHANGELOG entry sits after #647's doctor entry, not at the end of the list, because `origin/main` (now
    `e327569`) appended #641's entry at the end; this placement merges without a conflict.
  - The private `screen_text` copy carries a comment naming its original and the fold as a #645 follow-up (A round 2,
    warn 1). Code comments cite ADR-0021 and issues, never the brief, since `docs/handoffs/` is removed before the push.
- **Assumed:**
  - `SwitchFailure::message` uses the error's text as is (no `embedded` pass): the brief says "`error`'s text", every
    other verb prints a `PaneError` the same way (`ErrorBody::from`), ADR-0021 section 9 makes every port message one
    line, and JSON mode flattens it anyway. Every value the engine itself puts in a message is quoted.
  - `switch::run` types `SESSION` before calling `execute`, which reports it only after `PANE` and `--profile`, so the
    brief's step-1 order holds (the parse is pure, so typing it early is not observable).
  - `emit_outcome` stays `pub(crate)` as the brief's API gives it, although only `execute` calls it now.
- **Hedged:**
  - `cargo test --workspace` stopped once on `join_held_test::racing_senders_get_exactly_one_prompt_through_a_grant`
    (round 13: a body `connection_lost` mid-turn) with the machine's load average at about 25 on 24 cores. That test
    drives a real hub, body and stub agent and touches nothing this story changes; the target passed 11/11 on a
    rerun, and the full suite was rerun with `--no-fail-fast` (handoff-F.md has the result).
  - `archChanged: false`: the module, its public API (T's RED stub) and the dependency edges (`tx_switch` on
    `findings` and `reconcile`, the CLI on `tx_switch`, `reset` on `switch`) are all the plan A passed.
- **Evidence:**
  - The brief (all of it), handoff-A round 2, handoff-T-red, and every RED test in `switch.rs` and `reset.rs`.
  - Source read: `holler-pane/src/{lib,error,pane,ports,profile,findings,reconcile}.rs` and `reconcile/observe.rs`;
    `holler-cli/src/{output.rs,pane/mod.rs,pane/args.rs,pane/doctor.rs,pane/wiring.rs}`; the test kit's `harness.rs`
    and `profile_scope.rs`; `holler-hub/src/panes/store.rs:163-182, 336-352`; ADR-0021 sections 8, 9, 11 and
    "Deferred"; #644's brief decisions 14 and 20 on its branch.
  - A throwaway probe crate in the session scratchpad (outside the repo) ran the engine on the fakes and printed every
    refusal and failure message; each matched the brief's wording.
  - The built binary: bad arguments exit 2 `usage` before any port; everything else exits 1 `not-implemented` from
    the `Unwired` ports, as the CHANGELOG entry says.
  - The commands and results are in handoff-F.md, "Tier 1 self-check"; the source facts are in evidence.md.

## T (verify / GREEN) — 2026-10-09T23:01-06:00
- **Decided:**
  - GREEN is valid: `pane_verbs` 113/113, the workspace suite passes (exit 0, 1523 passed), and every Tier 1 check is
    clean. F edited no test, and T repaired none.
  - The tests pin behaviour: 8 mutations of `tx_switch.rs` (P4, P5, O1, the two record fields, the reconcile step, the
    created-session note) were each caught by the test of their AC.
  - Added two evidence entries for test-kit facts the tests rely on: the shared data directory (AC 4) and
    `concurrent_put` (AC 9).
- **Assumed:**
  - `cargo fmt --all --check` failing on untouched files is not this story's problem: neither CI nor `lint.sh` runs it,
    and AC 25 names only the five files, which pass.
- **Hedged:**
  - `origin/main` is at `d9eabbb`. The merge is textually clean and the merged ADR-0021 keeps the #645 paragraph in
    section 8. T did not build the merged tree: no file main changed overlaps this story's code, and #663's helper is
    used by no verb yet.
- **Evidence:**
  - handoff-T-green.md (commands, outputs, the mutation table); `git merge-tree --write-tree HEAD origin/main` exit 0.

## A (anti-duplication gate) — 2026-10-09T23:14-06:00
- **Decided:** BLOCK, on one finding that is not a parallel path (handoff-A-dup.md).
  - F extended every object of the Reuse map: the stubs, doctor's select, observe and record, `doctor_command`,
    `quoted`, `shown_differs`, the remedy table, `scope.resolve`, `output::emit`, and the doctor rig declared once. The
    one copy, `screen_text`, is the one the brief justifies.
  - The block: #663, on `origin/main` (`d9eabbb`), rewrote ADR-0021 section 8's generations bullet. A verb's reconcile
    step after a post-act `generation-conflict` now names no pane. The #645 paragraph and AC 9's behaviour print the
    pane doctor line for the pane with `--fix`. The merge is textually clean, so the merged standing spec would hold
    both rules unflagged.
  - Fix: merge `origin/main`, then one sentence in the #645 paragraph (inside AC 24's limits) stating the pane-scoped
    form and why. No code or test change.
  - Three warns, all follow-ups for O to file:
    1. The reconcile-step lead-in and `screen_text` are spelled twice. F-4 is out of date, and the `screen_text` fold has
       no issue.
    2. `tx_switch::read` is the third private copy of the scoped-read arms (with reconcile's `resolve` and #646's
       `in_scope`).
    3. The test runner is the fourth both-format runner, with names unlike #662's and #646's `run_both`. F-2's trigger
       has fired.
- **Assumed:**
  - `origin/main` at `d9eabbb` (fetched during this review) is what the PR will merge into.
  - The branch merges `origin/main` before the PR, as T-green notes.
  - The brief's AC 24 still binds which ADR-0021 lines this change may touch, so the fix goes in the #645 paragraph, not
    in #663's bullet.
- **Hedged:**
  - Finding 1 is a block, not a warn, for three reasons. The contradiction is in the standing spec on the tree that will
    land. AC 9 pins the code's side of it. Nothing downstream (a clean merge, S on the branch's ACs, a docs CI that only
    parses commands) would catch it. The fix is one sentence.
  - Which form should win is O's ruling. I recommend the pane-scoped `--fix` form: step 6's stated reason (a pane with no
    record) does not apply to a pane the plan read, and `--fix` is the repair. Adopting step 6's form would change
    behaviour and drop the repair.
  - I did not build the merged tree. It merges cleanly, and no file `origin/main` changed overlaps this story's code.
- **Evidence:**
  - The brief (all of it), handoff-A round 2, handoff-T-red, handoff-F, handoff-T-green and this journal.
  - The full diff `dc300ab..583e9d4`. Every changed file read in full: `tx_switch.rs`, `pane/switch.rs`, `pane/reset.rs`,
    `tests/pane_verbs/{switch,reset}.rs`, and the `doctor.rs`, `stub.rs`, fixture, ADR 0003, ADR-0021 and CHANGELOG hunks.
  - The analogues: `reconcile.rs`, `reconcile/observe.rs`, `findings.rs`, `pane/doctor.rs`, `pane/args.rs`, `doctor/rig.rs`,
    `doctor/surface.rs`, `profile_verbs/rig.rs`, `verb_harness/`, `error.rs` (`excerpt`).
  - On `origin/main` (`d9eabbb`), checked against `dc300ab`: the ADR-0021 diff, `pane/profile_scope.rs`
    (`reconcile_step`, `StoreScope`), `pane/park.rs`, `pane/list.rs`, `tests/pane_verbs/{main,list,park}.rs`,
    `park/rig.rs` and `holler-pane/src/probe.rs`.
  - `git merge-tree --write-tree HEAD origin/main` (tree `5bec458`), with the merged ADR-0021 read at lines 285-366 and
    522.
  - Greps: production callers of `select_session` and `create_session` (only reconcile's repair and the engine);
    session-id validators (none; the OpenCode adapter percent-encodes ids, `lib.rs:422-424`); lookups by session of
    record (none); both-format runners in `tests/`; the hub for any switch path (none).

## F (rework, round 2: the anti-duplication BLOCK) — 2026-10-09T23:27-06:00
- **Decided:**
  - Merged `origin/main` (`d9eabbb`), following A-dup's note 1. It merged cleanly. I staged it with
    `git merge --no-commit --no-ff`, because F makes no commits. The Workflow script's phase commit (`git add -A`, then
    `git commit`) records it as the merge commit. gitleaks found no leaks on the staged merge.
  - The pane-scoped `--fix` reconcile step stays, as A recommended. The code and tests are unchanged. Step 6's reason for
    naming no pane is a pane that a failed launch leaves with no record. That does not apply here: a switch or reset reads
    the pane's record before it acts. The step carries no `--profile`, and a doctor run by name refuses only a pane with no
    record (`reconcile.rs:226-236`). `--fix` is doctor's own remedy for a fixable mismatch (`findings.rs:135-141`).
  - ADR-0021 gets four sentences, in the #645 paragraph only (AC 24). They say:
    - when the step is printed: every failure once `select_session` has been called, a record conflict or a `timeout`
      included, and none before;
    - that it names the pane even after a run with `--profile`;
    - why it differs from the two forms of step 6 that #663's generations rule gives;
    - that no `profile show` is needed, since neither verb edits P's specs.
    #663's bullet and step 6 are untouched.
  - The boundary is "once `select_session` has been called", not A's draft "once the act has begun". The paragraph
    defines reset's act as `create_session` and then `select_session`, and a failed `create_session` carries no step
    (`acted` is false). Two of my round-1 doc comments in `tx_switch.rs` had the same imprecision. I fixed them, comments
    only.
- **Assumed:**
  - AC 24's limits still decide where the fix may go. So the generations bullet, which is #663's text, stays as it is,
    and the exception is stated in the #645 paragraph.
  - Fixing two doc comments of my own does not break A's "no code or test change". A meant the behaviour:
    `SwitchFailure::message` and `RECONCILE_P` are unchanged.
- **Hedged:**
  - I wrote "a doctor run for that pane is not refused", not A's "never refused". A record deleted by another writer
    after the plan would still be `pane-not-found`. That is an edge of its own, outside the reason step 6 gives.
  - The "before then, no step" clause makes explicit what the brief's Decision 13 decided ("with the reconcile step when
    `acted`"). Section 12 says that a verb that times out "prints the reconcile step". The clause gives the reason (the
    TUI and the record have not moved), as step 2 of the I8 order does for its pre-act conflict.
  - Observation, not changed: when reset's `create_session` times out, the server may still hold a session, and the
    message cannot name it. Doctor reports such a session as a stray.
- **Evidence:**
  - handoff-A-dup.md; `origin/main`'s ADR-0021, section 8 (`:285-344`) and section 12; the merged ADR-0021 at
    `:285-372`; `profile_scope.rs` `reconcile_step` on `origin/main`; `reconcile.rs:219-243`; `findings.rs:111-143`;
    `tx_switch.rs` in full.
  - `git merge-tree --write-tree HEAD origin/main` exited 0. On the merged tree: `git diff origin/main --
    docs/adr/ADR-0021.md` has the four AC-24 hunks only. The paragraph is at line 345, between `### 8.` (285) and
    `### 9.` (372). `#645` occurs 9 times (6 on `origin/main`). The source files quoted in evidence.md are unchanged
    against `origin/main`.
  - The commands and results are in handoff-F.md, "Tier 1 self-check". The four new source facts are in evidence.md,
    under "Added by F (round 2)".

## T (Phase 7, GREEN, round 2, after F's anti-duplication rework) — 2026-10-09T23:36-06:00
- **Decided:**
  - GREEN on the merged tree (`37f2101`, with `origin/main` `d9eabbb`): the workspace in CI's form gives 1641 passed,
    0 failed, 14 ignored over 135 result lines, the same counts F reported. Clippy, lint, changelog-check, rustfmt and
    machete are clean.
  - Repaired a test gap the round-2 ADR sentence exposed. "A failure before `select_session` has no step" was pinned by
    nothing: forcing `SwitchFailure::message` to always append the step (`if true`) left all 20 switch and reset tests
    green. I added `failed_before_the_act` in `tests/pane_verbs/switch.rs`, which is `failed` plus "no message contains
    `to reconcile`", and used it at every refusal and pre-act failure site: 9 in `switch.rs` and AC 17's in `reset.rs`.
    I also added `reset_create_failure_changes_nothing` for the brief's A1 row (`acted: false`, `created: None`), which
    the new ADR sentence names and no test covered. The same mutation now fails 8 tests.
- **Assumed:**
  - This is test-only work for T. The production behaviour already matches the ADR sentence, so F has nothing to change.
  - `switch_usage` uses the helper too, although usage errors are reported before the engine runs and the mutation does
    not reach them. The assertion still holds there and costs nothing.
- **Hedged:**
  - The new reset test fails the create with `unavailable`, not `timeout`. F's observation, that a timed-out create may
    leave a session the message cannot name, is a 645b question and is not pinned here.
- **Evidence:**
  - Mutations on `tx_switch.rs:143`, each restored with `git checkout`: `if false` fails 5 tests (ACs 7, 8, 9, 18, 19),
    both before and after the repair. `if true` fails 0 tests before the repair and 8 after.
  - `cargo test -p holler-cli --test pane_verbs` gives 154 passed (153 + 1). The fact the new test relies on is in
    evidence.md, under "Added by T (Phase 7, GREEN, round 2)".

## F (rework, round 3: the outside diff gate's BLOCK) — 2026-10-09T23:44-06:00
- **Decided:**
  - The outside diff gate (`deepseek-v4-pro`) has run twice. Its result files are gitignored, so both rounds are
    journalled here. Round 1, on `583e9d4` at 23:03 MDT, was PASS with no block. Round 2, on `4443dd4` at 23:37 MDT, was
    BLOCK on B-1: `check_health`'s remedy fallback used `Option::unwrap_or_else`, and the brief's P3 says "with no
    `unwrap`/`expect`".
  - B-1 is fixed in code, not argued. `check_health` now picks the remedy by an explicit `match` (`Some(remedy) =>
    remedy, None => doctor_command(pane, false)`). The behaviour is the same, and the `None` arm is still never taken for
    a named pane (`findings.rs:127-131`).
  - NIT-1 is taken: the `SESSION_ID_MAX` doc now says that `quoted` counts characters and `parse_session_id` counts bytes,
    which agree on an ASCII id. NIT-2 is about a test file's module doc, so it is noted for T, with no change.
  - NV-1 is answered with evidence: two `evidence.md` entries quote the real `StoreScope::resolve` and `member` (#663),
    `PaneStore::get`'s contract, and the fake's `resolve` and `member`. Each answers `resolve(P, Some(n))` with exactly
    the pane `n`, or `pane-not-in-profile`, so P1's `find` by name and `next()` agree. `find` stays.
- **Assumed:**
  - The classifier routed B-1 to F as production work, which is right: the line is in `tx_switch.rs`.
  - A `match` meets the brief's "no `unwrap`/`expect`" in every reading, the gate's literal one included, so this finding
    will not come back.
- **Hedged:**
  - B-1's stated failure mode is wrong. `unwrap_or_else` cannot panic: it runs the closure on `None`, clippy's
    `unwrap_used` does not cover it, and round 1 passed the same line (its NV-3). I changed the code anyway, because the
    change is free and removes the ambiguity. Round 2 blocked a line that round 1 passed and that did not change between
    them, so the gate's next round may raise something new that neither round raised.
- **Evidence:**
  - `docs/handoffs/645-diff-result-r1.md` and `-r2.md` and their `.usage.json` files (gitignored); `tx_switch.rs` in full;
    `findings.rs:127-131, 319-334`; `error.rs:686-695`; `holler-cli/src/pane/profile_scope.rs:1-5, 92-100, 159-173`;
    `holler-pane-testkit/src/profile_scope.rs:117-126, 190-204`; `ports.rs:63-64`; the brief's P3 row (line 914).
  - The new evidence excerpts were checked against the source with `diff`. With `CARGO_BUILD_JOBS=4`: workspace clippy
    with `-D warnings` is clean, `pane_verbs` gives 154 passed, and the other three CLI targets and `holler-pane` pass.
    rustfmt, `lint.sh` and `changelog-check.sh` pass. The workspace in CI's form exits 0, with 1642 passed, 0 failed and
    14 ignored over 135 result lines (handoff-F.md, "Tier 1 self-check").

## T, Phase 7 (GREEN), round 3, 2026-10-09 23:53 MDT

- **Decided:**
  - GREEN on `62aae72`. F's `match` in `check_health` keeps the behaviour. Dropping its `Some` arm fails
    `switch_refuses_an_unhealthy_server`, and the `None` arm cannot be reached for a named pane, so it needs no test.
  - NIT-2 is taken as a test-file doc fix. The module doc of `tests/pane_verbs/reset.rs` now describes `both_with` as it
    is: a text run and a JSON run on separate rigs. No test logic changed.
- **Assumed:**
  - The gate's other r2 entries need nothing from T. NV-1 is settled by F's evidence, and I checked the
    `StoreScope::resolve` excerpt against the source.
- **Hedged:**
  - None.
- **Evidence:**
  - With `CARGO_BUILD_JOBS=4`: `pane_verbs` 154 passed; the workspace in CI's form exits 0, with 1642 passed, 0 failed and
    14 ignored over 135 result lines; clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete` and
    rustfmt on the five story files are clean. `origin/main` `d9eabbb` is an ancestor of HEAD.

## A (anti-duplication gate, round 2) — 2026-10-10T00:00-06:00
- **Decided:** PASS on `940e338` (handoff-A-dup.md, which replaces round 1's; round 1 stays in git at `da53aba`).
  - Round 1's block is fixed. After the merge, the #645 paragraph says that the step names the pane with `--fix` even
    with `--profile`, and why that differs from step 6. #663's bullet and step 6 are untouched, and the ADR diff against
    `origin/main` has the four AC-24 hunks only.
  - This cycle adds no parallel path:
    - F rewrote the reused remedy call as a `match` and changed four doc comments.
    - T's `failed_before_the_act` extends the story's own `failed`.
    - The new reset test uses the existing helpers.
  - Five warns, none needing a change in 645a:
    1. Section 12's "prints the reconcile step" is broader than the #645 paragraph's no-step-before-the-act rule. It is
       read as #646's park reads it ("after a live act"), and is for the next section-12 edit.
    2. #643's `list::profile_name`, which arrived with the merge, is spelled inline in `pane/switch.rs:97-101`, as doctor
       spells it.
    3. The lead-in and `screen_text` are carried from round 1, and `reconcile_step`'s doc now lags.
    4. The scoped-read arms are carried from round 1, now four copies with `get.rs`, which answers the
       unreachable arm differently.
    5. The fourth both-format runner (F-2) is carried from round 1.
- **Assumed:**
  - `origin/main` at `d9eabbb` (fetched 2026-10-09 23:53 MDT, unchanged) is what the PR merges into.
  - AC 24 still limits this story's ADR edits, so section 12 is left for a later story.
- **Hedged:**
  - Warn 1 is not a block, unlike round 1's finding. There the merged spec gave two specific rules for the same failure,
    and the #645 text did not acknowledge #663's. Here a general sentence meets a specific exception that gives its
    reason, and `main` already reads it that way (`pane/park.rs:11-12`).
  - Warn 2 is a warn because no `--profile` typing pattern dominates outside the read verbs (doctor inlines it, and park
    has its own), and the expression is one line with the same behaviour.
  - I did not build or test. T-green's run on this head (`CARGO_BUILD_JOBS=4`) is the runtime evidence, and A does not
    own runtime.
- **Evidence:**
  - handoff-A-dup round 1, handoff-F (rounds 1-3), handoff-T-green (round 3), this journal, the brief (Reuse map, the
    follow-ups, Decisions 13 and 17) and the outside diff gate's r3 result (PASS).
  - Diffs: `git diff 583e9d4 940e338` over the story's code and test files, and `git diff d9eabbb HEAD` over the ADRs,
    CHANGELOG, fixture, `stub.rs` and `doctor.rs`. Read in full: `tx_switch.rs`, `pane/switch.rs`, `pane/reset.rs` and
    `tests/pane_verbs/{switch,reset}.rs`.
  - On the merged tree: ADR-0021 sections 8, 11 and 12 and "Deferred"; `profile_scope.rs` (`reconcile_step`, `member`,
    `resolve`); `findings.rs` (the remedy table, `doctor_command`); `reconcile.rs` `resolve`; `reconcile/observe.rs`
    `select` and `screen_text`; `pane/{list,get,park,doctor}.rs`; `park/rig.rs`; `doctor/surface.rs` `HealthGate`; the
    test kit's fault and harness API; `probe.rs`.
  - Greps over `crates/`: `to reconcile`, `ProfileName::parse`, `PaneName::parse`, `from_static(`, the callers of
    `select_session`, `create_session` and `shown_session`, `impl HarnessPort for`, `concurrent_put` and the
    both-format runners. A scan of the diff for personal names.
  - `gh issue list` searches for the follow-ups: none filed. `git merge-tree --write-tree` against the two open PRs: #714
    conflicts in ADR-0021 only, and #713 merges cleanly.

## S (spec audit) — 2026-10-10T00:11-06:00
- **Decided:** REWORK, production, one item (handoff-S.md).
  - Every brief AC except 24 and 25, and every issue criterion in 645a's scope, has a proving test or evidence. The
    public API, the P0-R table, the messages and Decisions 1-19 are as the brief states. The documented deviations are
    acceptable: `find` for `next()`, `reset` reusing `execute`, A's round-2 form of the "Deferred" bullet, and A-dup
    round 1's four ADR sentences. Build guards, privacy and scope are clean.
  - The item: after A-dup's round 2, `origin/main` moved to `abdcbb6`, with #713 (#660) and #714 (#640 part 3) merged at
    00:05 and 00:06 MDT. The branch now conflicts in `docs/adr/ADR-0021.md`, in two adjacent-line hunks: the section 9
    `pane launch` row, and the "Deferred" `HerdrPort`/`HarnessPort` bullet. Against the current `origin/main`, AC 24's
    own command shows 8 hunks, and AC 25's `Cargo.toml` diff is 14 lines. F merges `origin/main` and keeps both sides.
    T then re-runs GREEN on the merged tree. No code or test change is expected.
- **Assumed:**
  - The Workflow script opens the PR without merging `main` (`coding-pipeline.workflow.mjs:3336-3347` pushes and creates;
    no merge step was found). So a PASS would give a PR that conflicts.
  - T-green's round-3 Tier 1 results hold for the `d9eabbb`-based tree. I did not re-run them.
- **Hedged:**
  - REWORK rather than PASS with a note. The resolution is mechanical, but taking either side whole silently breaks AC 24,
    and resolving it after S would edit the standing spec outside every gate. The brief's R-1 assigns this to F.
  - The issue's "first message lands in the new session" criterion has no test, by Decision 1. I treat it as out of
    645a's scope, not as REWORK, because the operator's request names "#645a".
  - Commits carry `Co-Authored-By` but no session link. That is the script's practice, and the merged squash commits
    `d9eabbb` and `13edbb4` match it. It is advisory for the squash at merge.
- **Evidence:**
  - Read: the brief (all of it), handoffs A, T-red, F, T-green and A-dup, `evidence.md`, this journal, and the outside
    gates' brief r1 and diff r1-r3 results (r3 PASS at 23:52 MDT).
  - Read in full: `tx_switch.rs`, `pane/switch.rs`, `pane/reset.rs`, and tests `pane_verbs/{switch,reset}.rs`. Also the
    hunks of `doctor.rs`, `stub.rs`, `cli-surface.txt`, ADR 0003, ADR-0021 and the CHANGELOG. Also `doctor/rig.rs`,
    `fixture.rs` `sample_pane`, `output.rs` `emit_error`, `pane.rs` `env`, `pane/get.rs`, and `CONTRIBUTING.md`.
  - Commands: `git fetch`; `git ls-remote origin main` (`abdcbb6`); `gh pr view 713 714` (both merged);
    `git merge-tree --write-tree HEAD origin/main` (exit 1, ADR-0021 only), with the merged ADR read at `:398-411` and
    `:605-628`; `git diff d9eabbb origin/main` over `holler-pane/src` (doc comments only) and over `stub.rs`; the ADR-0021
    heading, paragraph and count greps; `wc -l` on every touched file; greps for banned calls, `#[allow]` and sleeps; a
    privacy scan of every added line; the commit trailers and author emails.

## F (rework, round 4: S's REWORK, merge `origin/main`) — 2026-10-10T00:25-06:00
- **Decided:**
  - Merged `origin/main` (`abdcbb6`: #713 for #660, and #714 for #640 part 3) with `git merge --no-commit --no-ff`, as in
    round 2. The merge is staged with `MERGE_HEAD` set, and the Workflow script's phase commit records it with both
    parents. `CHANGELOG.md` and `process/stub.rs` merged cleanly.
  - The one conflict, in `docs/adr/ADR-0021.md`, is resolved by keeping both sides, as S's item 1 says:
    - Section 9: `origin/main`'s `pane launch` row (with `grid-unreachable`) and this branch's `pane switch` row.
    - "Deferred": this branch's mismatch-code bullet and PROPOSED bullet, then `origin/main`'s rewritten
      `HarnessPort`/`HerdrPort` bullet in place of the old one.
  - `git diff origin/main -- docs/adr/ADR-0021.md` again shows exactly the four AC-24 hunks. The paragraph is at `:345`,
    between `### 8.` (`:285`) and `### 9.` (`:372`), the `unavailable` decision is stated once in section 8 (`:352`), and
    `#645` occurs 9 times (6 on `origin/main`). `git diff origin/main -- '*Cargo.toml'` is empty again (AC 25).
  - `evidence.md`: #714's three doc-comment lines in `error.rs` moved three citations down by 3, and I corrected them.
    I also completed two excerpts (`quoted`, `emit_json`) that had stopped one line short of their cited ranges since
    round 1. Every excerpt now matches its lines on the merged tree.
  - I did not edit ADR-0021 `:465` ("#645's and #646's, planned"), although it will be out of date for #645's codes once
    645a lands. AC 24 allows no other ADR-0021 line. The base `d9eabbb` had the same status ("the open codes planned so
    far (#645, #646)"), Decision 17 left it, and every gate passed it. It is listed for O in handoff-F's "Known issues".
  - No code or test change. `archChanged: false`.
- **Assumed:**
  - The script's phase commit (`git add -A`, `git commit`) completes a staged merge, as it did for `37f2101`.
  - `origin/main` at `abdcbb6` (fetched again at 00:23 MDT) is what the PR merges into. The open PRs #715 and #673 touch
    no file of this story.
- **Hedged:**
  - #644 and #642 part b, which are being restarted alongside this run, may land ADR-0021 edits next to this story's lines
    before 645a merges. Then R-1's rule applies again: whichever lands second resolves the conflict by anchor text.
- **Evidence:**
  - handoff-S.md (item 1 and its checks); `git diff d9eabbb origin/main` and `git diff d9eabbb HEAD` over ADR-0021; the
    conflict hunks at `:403-409` and `:613-626` of the conflicted file; `git diff d9eabbb origin/main` over
    `holler-pane/src`, `process/stub.rs` and `docs/testing.md`; the merged `STUBS` table and the `[Unreleased]` section;
    `output_api.rs` (no verb iteration); `gh pr view 715 673 --json files`.
  - With `CARGO_BUILD_JOBS=4` on the merged tree: workspace clippy with `-D warnings` is clean. `pane_verbs` has 161
    passed (21 of them switch and reset), `pane_cli_process` 35, `cli_surface_test` 3 and `docs_cli_test` 3. rustfmt on
    the five story files, `lint.sh` and `changelog-check.sh` pass, and gitleaks finds nothing, staged or unstaged. The
    workspace in CI's form exits 0, with 1663 passed, 0 failed and 16 ignored over 137 result lines. That is round 3's
    1642 plus #713's 8 and #714's 13, and the 2 new ignored are #714's opt-in real-Herdr runs.
  - A scratchpad script compared all 24 `evidence.md` excerpts with their cited lines: 0 mismatches.

## T, Phase 7 (GREEN), round 4, after S's REWORK merge — 2026-10-10T00:34-06:00

- **Decided:**
  - GREEN holds on F's merged tree (`29fa0ff`, parents `ddb6fc3` and `abdcbb6`). No test needed repair, and F flagged
    none, so T changed no test.
  - S's NOT-MET rows are now met. AC 24: `git diff origin/main -- docs/adr/ADR-0021.md` has 4 hunks, with #714's row
    and bullet kept. AC 25: the Cargo diff is empty, and the workspace in CI's form passes with #713's and #714's tests.
  - PASS.
- **Assumed:** `origin/main` at `abdcbb6` (fetched 00:26 MDT) is the PR's base.
- **Hedged:** #644 or #642 part b may land an ADR-0021 edit first, and AC 24 would then need the same merge again (R-1).
- **Evidence:** run with `CARGO_BUILD_JOBS=4`. The workspace in CI's form exits 0, 1663 passed, 0 failed and 16 ignored
  over 137 lines. `pane_verbs` 161, `switch::`/`reset::` 21, `pane_cli_process` 35, `cli_surface_test` 3,
  `docs_cli_test` 3 and `wire_selftest` 3. Clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete` and
  rustfmt on the five story files are clean. P4 mutation: `switch_to_a_deleted_session_changes_nothing` fails, then is
  restored. I compared the moved `error.rs` evidence citations against source myself.

## Outside diff gate, r4 (deepseek-v4-pro), recorded by F — 2026-10-10T00:32-06:00
- **Decided (the gate's verdict):** BLOCK, "2 blocking finding(s)", on T-green round 4's tree (`48f2395`). B-1 stands. The
  reviewer wrote B-2 to B-4 under its BLOCK heading, then withdrew B-2, said B-3 was not a block, and found no
  contradiction in B-4. So which second finding the count includes is not clear. NV-1 and NV-3 ask for evidence. W-1
  repeats B-1 as a warn, and NIT-1 repeats B-3.
  - B-1: `select_and_observe` merged `select_session` with the observation, so `acted: true` was set for any failure of a
    function. Nothing in the code tied the flag to the `select_session` call that its doc names.
- **Assumed:** the script classified the BLOCK as production. F was spawned with no `fNote`, so it was not `mixed`.
- **Hedged:** this entry is F's record of the gate round. The script's gate writes none, as S's advisory 4 noted for r3
  (PASS, 23:52 MDT).
- **Evidence:** `docs/handoffs/645-diff-result-r4.md` and its `.usage.json` (round "1" by the script's #653 numbering; 56
  s; finish `stop`). Both are gitignored.

## F (rework, round 5: the outside diff gate's r4 BLOCK) — 2026-10-10T00:45-06:00
- **Decided:**
  - B-1 is fixed in code, not argued. `switch` now calls `ports.harness.select_session(..)` itself, right where the
    `acted` mapping starts, and maps that call's error with `acted`. The observation is its own function, `observe`
    (`shown_session` and the comparison), and runs only after `select_session` returned `Ok`. `select_and_observe` is
    gone, so no one function mixes the act with anything else. A step added before the call converts through
    `From<PaneError>` (`acted: false`). Behaviour is unchanged: the same calls in the same order, the same errors and the
    same messages. No spec text names the old helper: the brief leaves private helpers to F, and
    `grep -rn select_and_observe` over the brief, the ADRs and `crates/` now finds nothing. Only earlier handoffs name it.
  - W-1 is answered in the comment at that call and in the `acted` field doc. The field is set on `select_session`'s own
    failure too, because the engine cannot tell how far a failed call got. The planned real adapter fails after it has
    sent the request that moves the screen (spike `:237-239`, now in `evidence.md`). Printing the step when the TUI did
    not move is harmless, since doctor `--fix` finds nothing to repair. Leaving it out when the TUI did move would leave
    the operator without the repair.
  - B-3 and NIT-1: P1 keeps `find` by name, which S accepted. `read`'s doc now says why. It matches `pane get`
    (`get.rs:101-105`). It is the same pane as `next()` while `resolve` keeps its contract. A scope that broke the
    contract cannot make the run act on another pane.
  - B-2 (withdrawn by the reviewer): no code change. `reset::run`'s doc now says that it takes switch's path,
    `execute`, which prints through `emit_outcome`, so nothing is copied.
  - B-4: no change. `execute`'s doc already states the order of the usage checks, and no AC sets an order.
  - NV-1 and NV-3: three `evidence.md` entries.
    - A failed `select_session` may have moved the screen: `ports.rs:198-199` and spike `:237-239`.
    - OpenCode's id shape: spike `:149-151`, `hermetic_test.rs:25-26` and the fake's `harness.rs:122-124`.
    - The rig's call log covers every port: `rig.rs:249-272`, `:159-170`, `fault.rs:85-97`, and `herdr.rs:259-260`
      and `:270-271`.
  - No merge. Since `abdcbb6`, `origin/main` has moved only by #715 (`cec1f82`, `docs/handoffs/0660-output/decisions.md`),
    which conflicts with nothing here and changes no AC-24 or AC-25 input.
  - `archChanged: false`. One private function was split in two. The public API, the module boundaries and the
    dependency direction are unchanged.
- **Assumed:**
  - `origin/main` at `cec1f82` (fetched at 00:34 MDT) is what the PR merges into.
  - The next diff-gate round reviews `git diff origin/main...HEAD`, as r1-r4 did, so it sees the new docs beside the code.
- **Hedged:**
  - The next diff-gate round could raise B-2 or B-3 again, since the same reviewer prompt saw them twice. They are now
    answered in the code's own docs, where the reviewer sees them.
  - T may want a `fail_next(ShownSession, ..)` case, which would pin `acted` for an observation that errs after a good
    select. The gate's NV-2 noted that no test covers it. That is T's call. F writes no tests.
- **Evidence:**
  - Read: `645-diff-result-r4.md` and r1-r3, the brief (all of it), handoffs F (round 4), T-green (round 4), S and A-dup
    (round 2), and `evidence.md`. Also `tx_switch.rs`, `pane/{switch,reset,get}.rs`, `reconcile/observe.rs:316-348`, the
    tests `pane_verbs/{switch,reset}.rs`, `doctor/rig.rs`, the test kit's `fault.rs`, `harness.rs` and `herdr.rs`,
    `ports.rs`, the spike, and `holler-adapter-opencode` (`lib.rs:422-436`, `hermetic_test.rs:25-26`).
  - With `CARGO_BUILD_JOBS=4`:
    - rustfmt `--check` on the five story files: exit 0. Workspace clippy with `-D warnings`: exit 0, run before and
      after the last doc edit.
    - `pane_verbs` 161 passed (`switch::`/`reset::` 21), `pane_cli_process` 35, `cli_surface_test` 3, `docs_cli_test` 3
      and `holler-pane` 98.
    - The workspace in CI's form on the finished tree, 00:47-00:51 MDT: exit 0, 137 result lines, 1663 passed, 0
      failed, 16 ignored. That is round 4's count. An earlier run at 00:39-00:43, before a doc-only reword of the `acted`
      field, gave the same.
    - `lint.sh` exit 0, with size warnings only. `changelog-check: ok`. gitleaks found nothing. The `Cargo.toml` and
      `Cargo.lock` diff against `origin/main` is 0 lines, and the ADR-0021 diff has 4 hunks.
  - Self-check mutations, each restored: mapping the select's error with `?` instead of `acted` fails AC 8 and AC 18,
    and doing the same for `observe` fails AC 7 and AC 19.
  - `check_evidence.py`: 27 entries, 46 blocks, 0 mismatches.
