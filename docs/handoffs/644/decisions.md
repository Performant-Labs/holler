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
