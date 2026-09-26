# holler — Claude Code instructions

Review battery: `$WORKFLOW_ROOT/workflow/review-battery-setup.md`.
Runbook: `docs/reviews/review-battery.md`.
Skills: `$WORKFLOW_ROOT/workflow/skills/` via home-dir stubs.
Overlays: `docs/reviews/overlays/`.

## Commits

Commit automatically once a change is complete and verified, without asking first — this
overrides Claude Code's default "only commit when explicitly asked" for this repo.

## Pipeline

Implementation work uses the Performant Labs **coding pipeline (test-first)**, defined in
`$WORKFLOW_ROOT/workflow/workflow-coding-pipeline.md` (read it first; shared primitives are
in `pipeline-conventions.md`). Phase order:

```
O -> D(UI only) -> A -> T(author/RED) -> F -> T(verify/GREEN) -> A(anti-duplication) -> U(UI only) -> S
```

Holler has **no UI**, so D and U never run (`uiSurface: false`). T writes the tests and confirms
RED before F writes any code, then confirms GREEN after. A reviews the brief up front and the diff
for duplication after. S audits the result against the issue's acceptance criteria.

**The GitHub issue is the source of truth** (there is no `docs/planning/SPEC.md` or
`BUILD_PLAN.md`); ADRs in `docs/adr/` and `docs/protocol/v2.md` are the standing spec. O writes the
brief at `docs/handoffs/<issue>-brief.md` from the issue, using the playbook's `brief-template.md`.

### Review rigor

Declared per story in the brief: `direct` (one agent, self-check only; one-line or cosmetic
changes), `in-session` (the pipeline's own agents review, no outside model), `second-opinion`
(+ one outside model at the brief and diff gates; the playbook's default minimum for non-trivial
work), `panel` (+ a cross-vendor pair). The outside review model for this repo is **`glm-5.3-flash`**
on Z.ai's hosted chat endpoint (also the cross-vendor arm at `panel`). It is set per clone with
`bash $WORKFLOW_ROOT/workflow/review-models.sh --tier outside-review=glm-5.3-flash`, which writes a
managed block into the gitignored `.env`; also set `DUAL_REVIEW=1` there, or the runner exits 0 having
done nothing. `OPENAI_API_KEY` must be the Z.ai key, comes from the operator's secret store, and is
passed in the environment of the run: never write it to `.env` or the repo, and never print it. A
worktree needs a copy of the clone's `.env` (it is gitignored, so `git worktree add` does not carry it).
Before journalling any run above `in-session`, prove the gate can actually run with one real
completion against the configured endpoint; a model that is listed is not a model that works.

### Models and roles (resolved, never hardcoded here)

- Models per phase come from `bash $WORKFLOW_ROOT/workflow/review-models.sh resolve --project . --json`
  (a repo-local `.env`, gitignored, can override tiers). Pass its output as the workflow's `models` argument.
- `.claude/agents/*.md` are **generated and gitignored**: they are copies of the playbook's role templates
  (which are private), so they are regenerated in each clone and worktree and never committed. Never hand-edit
  them. Project-specific role content lives in
  `docs/agent-overlays/<role>.md` (not `docs/reviews/overlays/`, which belongs to the review battery); after changing one, run
  `node $WORKFLOW_ROOT/workflow/sync-role-docs.mjs --project .` and run it again in any worktree that needs them.
  Re-running the sync must produce no further change. Roles with no overlay pass through from the playbook.
- The `roles` argument is the **content** of those six files.

### Running the canonical Workflow script

Every run uses a fresh git worktree, with no exception for `rigor: 'direct'`. Use a **fresh clone**
of this repo as the primary checkout (never a checkout whose history predates the 2026-09-23
history rewrite), and keep the repo-local git identity set to the GitHub no-reply address.

```bash
git fetch origin && git worktree list            # prune worktrees whose branch is merged or deleted first
git worktree add .claude/worktrees/NNNN-slug -b issue-NNN-implementation origin/main
cp .env .claude/worktrees/NNNN-slug/.env      # gitignored, so the worktree does not get it otherwise
```

The branch must be named exactly `issue-<N>-implementation` (the script pushes and opens the PR from
it). `NNNN` is the zero-padded issue number. There is no `node_modules` or port to allocate for a Rust
worktree; to skip a cold build, seed the worktree's own `target/` from a warm one with
`scripts/seed-target-dir.sh <warm-target-dir>` (never share one `CARGO_TARGET_DIR`).

```js
Workflow({
  scriptPath: '$WORKFLOW_ROOT/workflow/coding-pipeline.workflow.mjs',
  args: { argsVersion: 1, repoPath: '<absolute worktree path>', issueNumber: N,
          briefPath: 'docs/handoffs/N-brief.md', uiSurface: false, rigor: 'in-session',
          models: { /* review-models.sh resolve */ }, roles: { /* content of .claude/agents/*.md */ } },
})
```

The script never merges; a human always makes the final merge decision. It stops on a named reason
(`escalate`, `gate-unavailable`, `preflight-failed`, ...) rather than working around it. A stopped run
is resumed from the worktree and its git history (`resumeFromRunId` works only within the same session).
The script's own commit messages and PR body do not carry this repo's AI disclosure: after it opens the
PR, check the PR body against `CONTRIBUTING.md` and add the disclosure with `gh pr edit`.

Manually spawning O and the role agents one at a time remains a valid fallback for an ad hoc single-phase
re-run; investigate a `Workflow` tool error before falling back, never switch silently.

## Development

Run `bash scripts/setup-hooks.sh` once in every fresh clone. `core.hooksPath` is local git config and is
not tracked, so a clone starts without the hooks: a gitleaks scan on commit (fails closed when gitleaks is
not installed; `HOLLER_SKIP_GITLEAKS=1` is the explicit opt-out), a Conventional Commit subject check, and the
`Co-Authored-By` trailer for agent-made commits. The workflow's pre-flight fails without them. The hooks have
tests: `bash scripts/test-hooks.sh` (also run in CI).

