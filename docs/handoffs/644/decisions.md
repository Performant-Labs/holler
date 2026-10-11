# Decision journal: #644 part 2, the OpenCode agent through `launch` and `relaunch`

Run handoff directory `docs/handoffs/644` · branch `issue-644-implementation` · rigor second-opinion · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry. The brief gate's round 1 is in
`docs/handoffs/644-brief-result-r1.md` (PASS).

## A, up-front plan review: 2026-10-11T07:02-06:00 (MDT)

- **Decided:** BLOCK, with 1 `block` finding and 8 `warn` findings (`docs/handoffs/644/handoff-A.md`).
  - **The block.** The brief's own dependency ("Built after #802 and #642 part 3 have merged") is not met in this
    worktree. The base is `263a2d7`, which equals `origin/main`. #802 has no PR and has not been pushed; its code exists
    only on the local branch `issue-802-implementation` at `3fe111d`, which is based on `92fbc6e`. #642 part 3 has no
    commits. None of #802's seams is on this base (`ServeSpec`, `serve_command`, `verify_serving`, `AGENT_UNSUPPORTED`,
    `HarnessOp::VerifyServing`, `FakeHarness::serve_specs`, `AGENT_MISMATCH`, `tx_launch/server.rs`, `launch/serve.rs`).
    So Scope 3, Acceptance 4, Acceptance 2's `ServeSpec` half and Acceptance 5's `verify_serving` half cannot be built
    inside the Blast radius. F would have to rebuild #802 as a parallel path in files the brief puts out of scope.
  - **The warns:**
    1. Two acceptance clauses cannot pass as written. Acceptance 5's "P unchanged" contradicts the I8 compensation
       (specs equal, generation moved by two). Acceptance 2's "recorded for both" fails because #802's fake records
       only `serve_command`'s spec.
    2. Acceptance 8 names one ADR-0021 place; five go stale (section 8, the section 9 row, the deferred bullet for
       #644's last part, "Decisions taken", section 3).
    3. Help text: the `--agent` line (`args.rs:60-61`) contradicts operator decision 2, and both verbs' `--help`
       (`launch.rs:54-55`, `relaunch.rs:35`) lists the data keys without `agent`. Both are outside or only partly in
       the Blast radius.
    4. `FieldValue::Key(None)` prints `none`, which is also a valid `AgentKey`.
    5. The new module would hold near-copies of #700's record and keep/replace tests.
    6. Three clauses of Acceptance 7 and 8 find nothing on this base, and `profile apply` is still a stub.
    7. Coordination with #664 (its brief item 13) and with #700's journal note that #647 owns `SpecField`.
    8. On #802's branch `launch/rig.rs` is 793 lines, so new helpers go in `agent.rs`.
  - **On the merits** the plan extends the right objects: the one `SpecField`/`FieldValue` table, the shared `Report`,
    and the port's refusal with the engine's rollback.
- **Assumed:**
  - #802 lands roughly as it reads at `3fe111d`, so warn 1's (b) still holds after it merges. The fresh run must
    re-check every contract name against the merged code.
  - The automated script stops this run on the BLOCK (`needs-amendment`), and recovery is a fresh run, as the role doc
    says.
  - "Built after ... merged" is the operator's real intent, not a wording slip. The brief's "Depends on" section
    repeats it.
- **Hedged:**
  - Whether #642 part 3 must merge before the fresh run is the operator's call. The tests do not need it (the fake
    stands in), but the ADR and CHANGELOG text would then have to describe `agent-unusable` as a future code.
  - Path B (split the issue so part of it lands on today's `main`) is offered, not recommended. It conflicts with #802
    in ADR-0021 sections 8 and 9, the module list of `tests/pane_verbs/launch.rs`, and `CHANGELOG.md`, so it needs a
    designated merge order.
  - Warn 4 (`none`) is a product decision. I flagged it rather than prescribe a form.
  - NV-1 and NV-2 (relaunch rollback and the reconcile step after a `verify_serving` refusal) cannot be checked on this
    base. They carry to the fresh run.
- **Evidence:**
  - `git fetch origin`; `git rev-parse HEAD origin/main`; `git merge-base HEAD origin/main` (`263a2d7`).
  - `gh pr list --head issue-802-implementation --state all` (`[]`); `git ls-remote origin 'refs/heads/issue-802*'`
    (empty); `git worktree list`; `git log issue-802-implementation`.
  - `gh issue view 642/644/700/802`.
  - `grep -rn 'verify_serving\|ServeSpec\|AGENT_UNSUPPORTED' crates/` (0 files); the rig helper greps on base and on
    `3fe111d`; `git diff --stat 92fbc6e 3fe111d`; the overlap with `main` via `comm` (9 files).
  - Sizes from `wc -l` on base and `git show 3fe111d:<file> | wc -l`.
  - Files read: `crates/holler-cli/src/pane/{launch,relaunch,args,mod}.rs`, `crates/holler-pane/src/profile_diff.rs`,
    `crates/holler-cli/src/profile/{show,apply}.rs`, `crates/holler-cli/tests/pane_verbs/{launch.rs,launch/profiles.rs}`,
    `docs/adr/ADR-0021.md` (sections 2, 3, 8, 9, deferred and decided), `docs/adr/ADR-0003.md`,
    `3fe111d:crates/holler-pane-testkit/src/harness.rs`, #664's and #647's briefs on their branches, and
    `docs/handoffs/0700-implementation/decisions.md`.

## A, up-front plan review, fresh run on merged #802: 2026-10-11T10:40-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 6 `warn` findings (`docs/handoffs/644/handoff-A.md`, which supersedes
  round 1's BLOCK at `275052f`).
  - **Round 1's block is resolved.** The branch's merge base is `origin/main` `7284001` (#802 merged). Every seam the
    plan names exists on it. All 38 path-cited Evidence blocks match the tree line for line (checked by script), and E35's
    greps give the same output when re-run. Round 1's warns 2 to 9 are folded in correctly.
  - **On the merits** the plan extends the right objects: the shared `Report` (its key taken from the stored record),
    the one `SpecField`/`FieldValue` table, and the port's refusal with the engine's rollback. The CLI reads no file.
  - **The warns:**
    1. ADR-0021 section 3's spec-flag table and its `--from-current` list lack `opencode_agent`, and ADR-0003:93 lacks
       `--agent KEY` (all three from #700). Acceptance 8(e) should fix the first two, and ADR-0003 line 93 should join
       the Blast radius.
    2. The result line should print the key through `text_value`, per `Report::text`'s own doc rule: `-` is a valid
       `AgentKey` and is also the line's empty value. The `(pid P)` clause cannot print, since the engine writes
       `pid: None`.
    3. "Code and message unchanged" is wrong: `failure_body` appends the reconcile step once the act has begun.
    4. On a relaunch, a `verify_serving` refusal comes after B1, so the pane is left with no server and its record
       unchanged. Acceptance 8(a)'s ADR text should say so.
    5. Test copies: Acceptance 2's new-module clause repeats E24, Acceptance 4's launch case with a stand-in code repeats
       E25, and Acceptance 5 repeats two `profiles.rs` tests. The fix is one table in `agent.rs` plus extending the
       existing tests; no rig edit is needed.
    6. #649 and #664 also edit ADR-0021 and `CHANGELOG.md`, and #664 edits ADR-0003. #664's item 13 and #642's "Out of
       scope" still contradict decisions 4 and 1.
- **Assumed:**
  - The script commits this handoff and the journal entry as it did in round 1 (`275052f`). A writes no commit of its
    own.
  - The fresh run's outside brief-gate result for the revised brief is not on disk yet. The only result file is round 1's
    (`644-brief-result-r1.md`, 06:51 MDT, on the old brief). This review does not depend on it.
- **Hedged:**
  - Warn 2's `(pid P)` clause: removing it is within the Blast radius, but the brief chose to keep it. I left the choice
    to O.
  - Warn 4's check before B1 is offered as a possible follow-up, not recommended for this story. It needs a new
    read-only port method, and the old server's agent set can be stale, which is why operator decision 1 holds for a
    relaunch.
  - Warn 5 is test guidance, which is T's domain. I raised it because the brief's own rule says a near-copy is a
    Phase 7 rejection.
  - Round 1's outside-gate items NV-1 and NV-2 are now settled from the merged code. NV-1: the rollback does not
    restart the old server. NV-2: the reconcile step is appended once for a relaunch too.
- **Evidence:**
  - `git fetch origin`; `git rev-parse HEAD origin/main`; `git merge-base HEAD origin/main` (`7284001`).
  - A script compared each path-cited Evidence block of the brief with `sed`-equivalent slices of the tree (38 of 38
    match), and E35's greps were re-run.
  - Greps for the seams; for `SpecField`/`FieldValue` consumers; for `spec_only`, `server's default agent`, `harness
    port`, `NO_VALUE` and `preserve_order`; for `SpecFlags` and `; 15]`.
  - `wc -l` on every file in the Blast radius; `scripts/lint.sh`'s size gate.
  - Sibling runs: `git worktree list`; for #642 and #664, `git diff --name-only <merge-base> <branch>` and their
    briefs' Blast radius and D-items; `gh issue view 649` (Blast radius, D5, D17).
  - Files read: `crates/holler-cli/src/pane/{launch,relaunch,args,mod,list}.rs`,
    `crates/holler-cli/src/profile/show.rs`, `crates/holler-pane/src/{profile_diff,tx_launch}.rs`,
    `crates/holler-pane/src/tx_launch/server.rs`, `crates/holler-pane/src/error.rs` (`Refused`'s `Display`),
    `crates/holler-cli/src/output.rs` (`ErrorBody`), `crates/holler-pane-testkit/src/{envelope,fixture}.rs`,
    `crates/holler-cli/tests/pane_verbs/{launch.rs,relaunch.rs,launch/serve.rs,launch/rig.rs,launch/profiles.rs}`,
    `crates/holler-cli/tests/profile_verbs/show.rs`, `crates/holler-cli/tests/pane_verbs/process/docs_rows.rs`,
    `crates/holler-cli/tests/fixtures/cli-surface.txt`, `docs/adr/ADR-0021.md` (sections 2, 3, 8, 9, deferred and
    decided), `docs/adr/ADR-0003.md`, `docs/handoffs/644-brief-result-r1.md` (round 1's NV items), and
    `.aftersight/releases/claude/workflow/parallel-agent-coexistence.md`.

## T, author tests (RED): 2026-10-11T10:50-06:00 (MDT)

- **Decided:** RED is valid (`docs/handoffs/644/handoff-T-red.md`).
  - **New tests.** `crates/holler-cli/tests/pane_verbs/launch/agent.rs` holds six tests: the text line (and its golden form
    with no agent), the quoting of `-`, `data.agent` and its key order, `--spec-only`, the help text of both verbs, and
    one refusal table each for launch and relaunch. Elsewhere there are two new `profile_diff_test.rs` tests and one
    new `profile_verbs/show.rs` test.
  - **Extended tests.** `launch_with_agent_stores_the_key_in_the_record`, both of #700's relaunch agent tests,
    `launch_with_profile_adds_the_spec_and_bumps_once`, and `spec_field_values_serialize_like_the_spec` (16 `PATHS`).
  - **Result.** 13 tests fail on assertions about the missing behavior, and clippy is clean.
  - **Applied A's notes for T.** Existing tests are extended rather than copied (warn 5). The message is the
    harness's message plus the step, once (warn 3). The key prints through `text_value`, so `-` prints as `"-"` (warn
    2). A refused relaunch is asserted as `StopOwned` twice with the record unchanged (warn 4). Key order is read on the
    raw stdout.
- **Assumed:**
  - O folds A's warn 2 into the brief (the `-` quoting) as A recommended. If O declines, F must not meet it by a second
    escaper: drop `a_key_that_reads_as_the_empty_value_is_quoted` instead.
  - `agent-unusable` is a stand-in string for #642 part 3's code. The test needs no #642 code, and any open code would
    serve.
- **Hedged:**
  - The two refusal-table tests pass before F. Scope 3 makes that behavior #802's, so they are regression pins, not a
    RED for F. I kept them because Acceptance 4 and 5 ask for them, and they add the relaunch, `agent-mismatch` and
    `--profile` cells that #802 did not test.
  - The new field is reached by its path through `SpecField::ALL`, never by its variant name. This keeps the RED free of
    compile errors, and it is how `profile show` reaches the field.
- **Evidence:**
  - `cargo test -p holler-pane --test profile_diff_test` (3 failed of 11), `cargo test -p holler-cli --test pane_verbs`
    (9 failed of 275), `cargo test -p holler-cli --test profile_verbs` (1 failed of 45).
  - `cargo clippy -p holler-cli -p holler-pane --all-targets -- -D warnings` (clean, after the refusal asserts moved
    into one helper for `cognitive_complexity`).
  - `wc -l` on every test file touched. A grep found no other test that pins the full `profile show` field block or the
    exact `data` keys.
  - An accidental `cargo fmt --all` reformatted about 180 unrelated files. I reverted them with `git checkout`, and only
    my hunks remain.

## F, implement against the RED: 2026-10-11T11:00-06:00 (MDT)

- **Decided:** done, with all 13 of T's RED tests GREEN and no test file touched (`docs/handoffs/644/handoff-F.md`).
  - **Field table.** `SpecField::OpencodeAgent` (`opencode_agent`, in `ALL` after `Effort`, 16 fields). `FieldValue`
    gains `Key(Option<String>)`, which serializes as the key or `null`. `Key(Some)` shares `Text`'s `write_escaped`
    arm, and `Key(None)` prints `(default)`. `differs` gains no arm.
  - **Result.** `Report.agent: Option<AgentKey>` goes after `spec_only`, read from `launched.pane` (the CAS result).
    The text gains `, agent KEY` after the harness port, through `text_value`, so `-` prints `"-"` (A's warn 2).
  - **Help.** The `--agent` line now reads "a new spec has none (the server's default agent) and an existing spec keeps
    its own", and both verbs' doc comments list `agent`.
  - **Docs.** ADR-0021 gets the five places of Acceptance 8 (sections 3, 8 and 9, the deferred bullet, and decisions
    15 to 20). Section 3 also gets the `--agent KEY` table row and `opencode_agent` in the `--from-current` list (A's
    warn 1). ADR-0003's SPEC FLAGS bullet gets `--agent KEY`, and `CHANGELOG.md` an Unreleased entry.
  - **ADR wording.** The refusal text follows A's warns 3 and 4: the harness's message gets the reconcile step once,
    and a refused relaunch leaves the pane with no server and its record unchanged.
  - **`archChanged: true`.** The change is additive and is what A reviewed, but it changes public `holler-pane`
    enums (`SpecField` and `FieldValue` gain variants, and `ALL`'s type becomes `[SpecField; 16]`).
- **Assumed:**
  - Editing ADR-0003 line 93 is acceptable although the brief's Blast radius omits that file. A's PASS warn 1 asked
    for it to be added. It is recorded as a deviation, and reverting it is one line.
  - The driver's phase commit (`git add -A`) commits these handoff files. F stages only the production files, by
    path.
- **Hedged:**
  - Kept the unreachable `(pid P)` clause, as the brief decided, and left its removal to O or the operator.
  - A's warn 6 (recording decisions on #664's and #642's issues) is O or MO work and was not done here.
  - Merge with #664 (ADR-0021, ADR-0003 and CHANGELOG): no hunks touch today. Whichever run merges second merges
    `main`.
- **Evidence:**
  - Before F (RED, per T): `profile_diff_test` 8 passed, 3 failed; `pane_verbs` 266 passed, 9 failed;
    `profile_verbs` 44 passed, 1 failed.
  - After F: `cargo test -p holler-pane --test profile_diff_test` (11 passed). `cargo test -p holler-cli --test
    pane_verbs --test profile_verbs` (275 and 45 passed).
  - `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings). `bash scripts/lint.sh` (exit 0).
    `bash scripts/changelog-check.sh` (ok). `rustfmt --check` on the 4 source files (clean).
  - The pipeline's `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`: exit 0, with
    2127 passed, 0 failed and 25 ignored across 148 test binaries.
  - The rendered `holler pane launch --help` and `relaunch --help`.
  - `git grep` on the sibling branches #642, #647, #649, #650 and #664 found no exhaustive match on `SpecField` or
    `FieldValue`. `git diff` hunk positions were checked for #664's ADR and CHANGELOG edits.
  - `docs/handoffs/644/evidence.md`: 18 entries, each excerpt checked by script against its `file:line`.

## T, verify (GREEN) + Tier 2: 2026-10-11T11:09-06:00 (MDT)

- **Decided:** GREEN, no blocking issue (`docs/handoffs/644/handoff-T-green.md`). PASS.
  - All 13 RED tests pass; workspace test (the configured `test.unit.command`), clippy, lint, changelog-check, machete,
    `docs_cli_test` and `wire_selftest` all pass. F's reported numbers reproduce exactly.
  - No test was repaired: F flagged none and T found none.
  - Three removal mutations (no text clause; `Report.agent` always `None`; `Key(None)` printing `none`) each turned
    tests red, then were reverted with `git checkout`.
  - Appended two evidence entries (fake `verify_serving`'s `agent-mismatch` message, `fail_next`) for unchanged test-kit
    facts the refusal table relies on.
- **Assumed:** F's ADR-0003 one-token edit (Deviation 1) is O's call to keep; it does not affect any test.
- **Hedged:** none.
- **Evidence:** the Tier 1 log in this session's scratchpad; `handoff-T-green.md` tables.

## A, anti-duplication gate: 2026-10-11T11:15-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 2 `warn` findings (`docs/handoffs/644/handoff-A-dup.md`; diff `7284001..3026df7`).
  - **No parallel path.** F extended the one `Report` (its `agent` comes from the stored record, through `text_value`)
    and the one `SpecField`/`FieldValue` table (`Key(Some)` shares `Text`'s `write_escaped` arm, and `differs` gains
    no arm). The refusal is still `verify_serving`'s, with the engine's rollback. The request builders, the rig, the
    test kit and `tx_launch` are unchanged.
  - **Tests.** Four existing tests were extended. The new module holds only new behavior: one refusal table per verb,
    with the rollback assertions in one helper, and `serve.rs`'s private `with_agent` is not copied.
  - **Warn 1.** Small test constants and builders are copies: `STEP_PHRASE` x3 and `AGENT` x2 in the launch test
    modules, beside the rig's `STEP`, and a one-line `AgentKey` builder x3 with three names in the `holler-pane` tests,
    beside `tests/common/mod.rs`. Each follows its module's own pattern, and the Blast radius barred the shared files.
    An XS follow-up can hoist them.
  - **Warn 2.** The brief never received Phase 3's warns 1 to 5: its Blast radius leaves out ADR-0003, and Scope 3 and
    Acceptance 4 still say "message unchanged". O should fold them in before S. The ADR-0003:93 edit is not a drive-by:
    Phase 3 warn 1 asked for it, `docs_rows.rs:18` assigns the two verbs to #644, and #664's hunk is at line 70.
- **Assumed:** the driver's phase commit commits this handoff and journal entry. A writes no commit of its own.
- **Hedged:**
  - `data.agent` repeats `data.pane.opencode_agent`, and `(default)` is a third spelling of an absent value beside
    `none` and `-`. Both were cleared as operator decisions 5 and 6: one source, and both reasons are documented. I
    did not count them as duplication.
  - The outside diff gate's NIT-2 (`MISMATCH_MESSAGE` hard-codes port 48100) is a test oracle that copies the fake's
    text. That is T's domain, and I did not raise it here.
- **Evidence:**
  - `git merge-base HEAD origin/main` (`7284001`); `git diff --stat` and full diffs of every changed file.
  - Greps over the tree:
    - `STEP_PHRASE`, `split_whitespace`, `harness port`, `Format::Text`, `get::help`;
    - `SpecField::`/`FieldValue::` outside `profile_diff.rs`;
    - `opencode_agent` in `holler-cli/src`, `; 15]`, `"model.effort"`;
    - `AgentKey::parse` and `fn key|agent` across `holler-pane/tests` and the test kit;
    - the `common/mod.rs` builders.
  - Sibling branches:
    - `git diff 7284001 issue-{664,642,649}-implementation` over this diff's source files (none touched);
    - ADR-0003 hunks (#664: line 70 only);
    - new `opencode_agent`/`SpecField` lines (#664 composes through `profile show`'s `PaneDiff`);
    - #664's `profile_verbs/rig.rs` change (additive only).
  - A scan of the added lines for personal infrastructure names: none.
  - `wc -l` on every touched file.
  - Files read in full:
    - `launch/agent.rs`, `launch/serve.rs`, `launch/rig.rs`;
    - `profile_diff.rs` and `launch.rs`'s `Report` region;
    - `handoff-A.md`, `handoff-F.md`, `handoff-T-red.md`, `handoff-T-green.md`, `644-diff-result-r1.md`;
    - the brief.
