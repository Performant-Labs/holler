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
