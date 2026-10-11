# holler — Claude Code instructions

Review battery: `$WORKFLOW_ROOT/workflow/review-battery-setup.md`.
Runbook: `docs/reviews/review-battery.md`.
Skills: `$WORKFLOW_ROOT/workflow/skills/` via home-dir stubs.
Overlays: `docs/reviews/overlays/`.

## Commits

Commit automatically once a change is complete and verified, without asking first — this
overrides Claude Code's default "only commit when explicitly asked" for this repo.

## Pipeline

Implementation work uses the Performant Labs **coding pipeline (test-first)**, run from the
Aftersight `claude-pipeline` release pinned in `.aftersight/pipeline-pin.json` and linked at
`.aftersight/releases/claude/` (gitignored; resolve it in a fresh clone with Aftersight's
`scripts/pipeline-resolve.mjs --pipeline claude --ref <tag in the pin>`). It is defined in
`.aftersight/releases/claude/workflow/workflow-coding-pipeline.md` (read it first; shared primitives are
in `.aftersight/releases/claude/workflow/pipeline-conventions.md`). Phase order:

```
O -> D(UI only) -> A -> T(author/RED) -> F -> T(verify/GREEN) -> A(anti-duplication) -> U(UI only) -> S
```

Holler has **no UI**, so D and U never run (`uiSurface: false`). T writes the tests and confirms
RED before F writes any code, then confirms GREEN after. A reviews the brief up front and the diff
for duplication after. S audits the result against the issue's acceptance criteria.

**The GitHub issue is the source of truth** (there is no `docs/planning/SPEC.md` or
`BUILD_PLAN.md`); ADRs in `docs/adr/` and `docs/protocol/v2.md` are the standing spec. O writes the
brief at `docs/handoffs/<issue>-brief.md` from the issue, using the Playbook's `brief-template.md`
(`$WORKFLOW_ROOT/workflow/brief-template.md`; the release does not carry it).

### Review rigor

Declared per story in the brief: `direct` (one agent, self-check only; one-line or cosmetic
changes), `in-session` (the pipeline's own agents review, no outside model), `second-opinion`
(+ one outside model at the brief and diff gates; the pipeline's default minimum for non-trivial
work), `panel` (+ a cross-vendor pair). The outside review model for this repo is **`deepseek-v4-pro`**
on DeepSeek's hosted chat endpoint (also the cross-vendor arm at `panel`). It is set per clone with
`bash .aftersight/releases/claude/workflow/review-models.sh --tier outside-review=deepseek-v4-pro`, which writes a
managed block into the gitignored `.env`; also set `DUAL_REVIEW=1` there, or the runner exits 0 having
done nothing. `DEEPSEEK_API_KEY` (the variable the script names in `DUAL_REVIEW_API_KEY_VAR`) must be
the DeepSeek key, comes from the operator's secret store, and is never printed. A worktree needs a copy of the
clone's `.env` (it is gitignored, so `git worktree add` does not carry it).
The release's pre-flight honors `DUAL_REVIEW_API_KEY_VAR` (Aftersight #568, in `claude-pipeline-v0.1.7`), so
`OPENAI_API_KEY` no longer needs a copy of the key.

**Secrets-in-files rule (general, not just this key):** put a key or other sensitive value in a
file, hidden or not, only after verifying that file is git-ignored (`git check-ignore -v <path>`
confirms it) — `.env` already is, so writing `DEEPSEEK_API_KEY` there for a run is fine once
verified. Writing a secret to a file that is *not* git-ignored, or overriding this check at all,
needs the operator's express consent first, every time.
Before journalling any run above `in-session`, prove the gate can actually run with one real
completion against the configured endpoint; a model that is listed is not a model that works.

### Models and roles (resolved, never hardcoded here)

- The per-phase table comes from `bash .aftersight/releases/claude/workflow/review-models.sh resolve --project <worktree> --json`
  (a repo-local `.env`, gitignored, is applied on top of the release's table). Pass the output's **`phases` array,
  as it is**, as the workflow's required `stages` argument: twelve entries, each with its own model and effort.
  The caller builds no `models` or `effort` map from it. `models` and `effort` are optional per-run overrides, by
  phase key (`F`, `T_RED`, ...) or role letter, and are never `null`.
- **Holler's stage-model table** (a fresh clone or a new account has no `.env`, so set it once; it rewrites only the
  managed block of `.env`): every phase on Opus, with efforts O `max`, D `high`, A `max`, T `high`, F `max`, U `medium`,
  S `max`. Until the release ships this as its default, run:

  ```bash
  bash .aftersight/releases/claude/workflow/review-models.sh --tier reasoning=opus --tier throughput=opus \
    --effort O=max --effort D=high --effort A=max --effort T=high --effort F=max --effort U=medium --effort S=max
  ```

  Phase F runs on `claude-opus-5-5` by explicit id (`models: { F: 'claude-opus-5-5' }` in the call below), at the
  table's `max`.
- `.claude/agents/*.md` are **generated and gitignored**: they are copies of the release's role templates, so they
  are regenerated in each clone and worktree and never committed. Never hand-edit
  them. Project-specific role content lives in
  `docs/agent-overlays/<role>.md` (not `docs/reviews/overlays/`, which belongs to the review battery); after changing one, run
  `node .aftersight/releases/claude/workflow/sync-role-docs.mjs --project .` and run it again in any worktree that needs them.
  Re-running the sync must produce no further change. Roles with no overlay pass through from the release.
- The `roles` argument is the **content** of those six files.

### Running the canonical Workflow script

Every run uses a fresh git worktree, with no exception for `rigor: 'direct'`. Use a **fresh clone**
of this repo as the primary checkout (never a checkout whose history predates the 2026-09-23
history rewrite), and keep the repo-local git identity set to the GitHub no-reply address.

```bash
git fetch origin && git worktree list            # prune worktrees whose branch is merged or deleted first
git worktree add .claude/worktrees/NNNN-slug -b issue-NNN-implementation origin/main
cp .env .claude/worktrees/NNNN-slug/.env      # gitignored, so the worktree does not get it otherwise
# the resolved release is gitignored too: link the primary checkout's .aftersight/releases into the worktree
mkdir -p .claude/worktrees/NNNN-slug/.aftersight
ln -s "$(pwd)/.aftersight/releases" .claude/worktrees/NNNN-slug/.aftersight/releases
grep -qx '.aftersight/releases' "$(git rev-parse --git-path info/exclude)" || echo '.aftersight/releases' >> "$(git rev-parse --git-path info/exclude)"
```

The branch must be named exactly `issue-<N>-implementation` (the script pushes and opens the PR from
it). `NNNN` is the zero-padded issue number. There is no `node_modules` or port to allocate for a Rust
worktree; to skip a cold build, seed the worktree's own `target/` from a warm one with
`scripts/seed-target-dir.sh <warm-target-dir>` (never share one `CARGO_TARGET_DIR`).

```js
Workflow({
  scriptPath: '.aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs',
  args: { argsVersion: 2, repoPath: '<absolute worktree path>', issueNumber: N,
          briefPath: 'docs/handoffs/N-brief.md', uiSurface: false, rigor: 'in-session',
          stages: [ /* the `phases` array of review-models.sh resolve --project <worktree> --json, as it is */ ],
          models: { F: 'claude-opus-5-5' },
          roles: { /* content of .claude/agents/*.md */ } },
})
```

`verifyCommand` is left out: phase F's test command comes from `test.unit.command` in
`.aftersight/pipeline.config.json` (the CI workspace test with the one skipped timing test). The pre-flight table must
show `pipeline-version` and `pipeline-contract` passing and a `verify-command` row naming that cargo command.

After the script opens the PR, the run's own agent verifies CI and the run's gates, then merges the
PR itself (squash, branch deleted) and reports the merge — merging is agent work, not a human gate
and not MO work. It stops on a named reason
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

