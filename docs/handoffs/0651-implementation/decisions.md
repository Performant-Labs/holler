# Decision journal: #651 — the Herdr display plugin (display only)

Run `0651-implementation` · branch `issue-0651-implementation` · rigor in-session · uiSurface
false (D and U are N/A, recorded). Append-only; every phase adds its entry.

## 2026-10-10 · Phase 0 — pre-flight and survey (O)

- Pre-flight PASS (15 checks, 0 failing) at the primary checkout; the plugin provisioned the run
  worktree `.claude/worktrees/0651-implementation` (branch `issue-0651-implementation`, port
  9051) from `origin/main` at `3d95aec`. The worktree's `target/` is linked to the warm primary
  `target/` (config `linkDirs`); the clone's gitignored `.env` was copied in per CLAUDE.md (its
  contents never printed — in-session rigor needs no outside-model key).
- Survey written (`survey.md`). Dependencies confirmed closed and merged: #636 (spike,
  `docs/research/herdr-api-spike.md` §12 is the plugin contract: two `report_metadata` methods +
  read-only only, one `source` id, `ttl_ms`, re-report on `pane.created`/`layout.updated`,
  16-token/32-kept SCHEMA limits, Herdr 0.9.1 protocol 22) and #639 (`pane/*` control-socket
  handlers; `pane/list` data is every `Pane`, sorted by name).
- Key architectural fact: the CLI pane verbs' live wiring is #649 (open) — `Wiring::connect` still
  hands out `Unwired`. The plugin therefore reads through the merged one-shot client
  `holler_hub::control::run` directly, not through `holler pane list`.
- **Reuse map verdict:** one new crate (`plugins/herdr-holler`) that imports every shared fact
  (Pane types, `shown_differs`, the control client) and duplicates nothing; manifest and report
  shapes copied from the spike probe.
- **Declared tension for A:** root `Cargo.toml` `members = ["crates/*"]` needs `"plugins/*"`
  added (one line) or the acceptance tests never run under `cargo test --workspace`. Recommended
  and declared in the brief as the single blast-radius exception.
- Brief written (`brief.md`) declaring rigor **in-session**; ACs mapped to tests, RED policy,
  boundaries, and the workspace-membership exception.
- Git conventions for this run (operator, 2026-10-10): first real commit immediately after T's
  tests are staged (never held for the merge gate); the operator proxy may place an empty opener
  commit on the fresh branch — journal it as attributed, never remove or rewrite it; rebase the
  branch on `origin/main` before opening the PR. Handoff diffs and commit messages are grepped for
  private-path/agent/fleet identifiers before the PR (public-repo placeholder rule).

## 2026-10-10 · Phase A — up-front plan review (O journals; A = architecture-reviewer)

- Dispatched via the task tool, recorded with `stage run` (STAGE OK, attempt 1, verdict PASS).
- **VERDICT: PASS, 0 blockers, 4 warns** (folded into the brief as binding for T/F): the plugin's
  own minimal Herdr client is forced by the adapter's closed `Request` enum but must mirror its
  transport discipline; the SYNC token gate must doc-cite `SessionSync::of`; the mutating-name
  test is an **allowlist**, not a denylist; no `ttl_ms` on workspace reports (spike §4).
- A **approved the declared blast-radius exception** (root `Cargo.toml` members + `"plugins/*"`,
  one line) as required — without it the acceptance tests run nowhere.
- A verified the read seam (`pane/list` on the control socket via `holler_hub::control::run`,
  precedent `transport.rs:35`) and the unknown-never-stale three-legged design.
- Phase 2 (design) and Phase 8 (ui-walkthrough): N/A, no UI surface — recorded by the plugin's
  advance, never silent.

## 2026-10-10 · Phase 4 — T-red (T)

- **Decided** (the contract the RED suite pins, all in `plugins/herdr-holler/tests/`): the
  token vocabulary is exactly `pos`/`project`/`shown`/`driven`/`sync`/`hold` per pane
  (absent shown/driven → `-`; sync `ok`/`mismatch`/`-` mirroring `SessionSync::of`, derived
  only through `shown_differs`; hold `none`/`parked`/`drained`) and `profile` per
  workspace; one source id `holler`; `ttl_ms` on pane reports only (A-warn 4). The hub seam
  is the ambient `HOLLER_STATE_DIR` (exactly what `holler_hub::control::run` resolves) and
  the Herdr seam is `HERDR_SOCKET_PATH`; `Endpoints::from_env()` resolves both and the
  tests inject through those env vars under one mutex.
- **Decided:** the degraded ("unknown, never stale") contract re-sends every token as
  `unknown` plus `state_labels: ["unknown"]` rather than dropping tokens — Herdr's
  per-source merge semantics are unverified, so dropping might leave stale values live.
- **Assumed:** the wire shape of `tokens` (a JSON object name→value) and of the plugin's
  Herdr request envelope (`{id, method, params}`, success `{id, result: {type: "ok"}}`);
  the spike verified the CLI forms but not these socket details. The manifest's
  `contexts = ["pane"]` vocabulary is likewise unverified, so the tests assert only
  command shapes, not contexts.
- **Hedged:** the action pane operand. The manifest actions are bare `["holler","pane",
  <verb>]` argv and the tests pin that shape plus `resolve_action_pane` (HERDR_PANE_ID →
  the record with that `herdr.pane_id` → pane name); how the resolved name reaches the
  verb at run time is F's wiring to land.
- **Evidence:** `cargo test -p herdr-holler --no-fail-fast` → 10 tests, **5 FAILED for the
  right reasons** (stub `Err`/`None`, no `shown_differs` in src yet), 5 green on contact
  (self-authored static artifacts: the clean skeleton + manifest; the stub-`None`
  boundary), recorded in `handoff-T-red.md`. Full `cargo test --workspace --no-fail-fast
  -- --skip roster_stays_accurate_under_concurrent_body_load`: only the three herdr-holler
  test targets fail; every other crate green. `cargo fmt -p herdr-holler` clean,
  `cargo clippy -p herdr-holler --all-targets` clean, no new `unsafe`. Root `Cargo.toml`
  members line + generated `Cargo.lock` package entry staged with the suite; not committed
  (the orchestrator commits).

## 2026-10-10 · Phase T-red — the suite, authored and RED (O journals; T = tester)

- Dispatched via the task tool; recorded with `stage run` (STAGE OK, attempt 1). The plugin's own
  t-red crossing then ran the unit command and rendered the verdict: **RED, exit 101** (the first
  advance attempt was refused because no plugin-owned RED was on record — the fail-closed contract
  working as designed; the second advance to implement passed).
- Suite: 10 tests in the new crate — **5 failing for the right reasons** (refresh loop
  not-implemented ×2, endpoints unresolved ×1, action resolver stub ×1, `shown_differs` doc-cite
  absent ×1), **5 green on contact and recorded as standing guards** (allowlist scan — the
  skeleton is clean by construction; manifest validity; event-hook wiring; unknown-pane `None`
  boundary). Rest of the workspace green.
- T landed the compile-minimal skeleton (`plugins/herdr-holler`: Cargo.toml per the house layout,
  `src/lib.rs` stubs, `herdr-plugin.toml`) and the A-approved one-line members exception with its
  comment; fixture pane names chosen to keep agent-pane-name shapes out of the staged diff
  (public-repo scrub rule).
- **First real commit landed immediately after T staged** (operator git convention 2026-10-10):
  `test(herdr-plugin): #651 the display-only plugin suite, RED (5 failing), plus the run brief` —
  run docs (survey, brief, handoff-A, decisions) committed with it by explicit path. Hooks passed
  (gitleaks, Conventional subject). No opener commit from the operator proxy observed on the
  branch at commit time.
- API surface fixed by T's stubs for F: `Endpoints{herdr_socket, hub_control_socket}` + `from_env`,
  `Reporter::new/refresh -> Refreshed{pane_reports, workspace_reports, unknown}`,
  `PlugError{Env,Herdr}`, `resolve_action_pane(&[Pane], &str) -> Option<PaneName>`; hub read via
  `holler_hub::control::run` (`pane/list` → `PaneReply` → `Vec<Pane>`), reports one JSON line
  in/out over `HERDR_SOCKET_PATH`; token vocabulary pos/project/shown/driven/sync/hold, workspace
  `profile`, source `holler`, `ttl_ms` pane-only; degraded path re-sends tokens as `unknown` +
  `state_labels:["unknown"]`.
- T flagged: merge semantics for degraded re-report unverified (dropping tokens might leave stale
  values, so unknown is RE-SENT rather than cleared); `holler-hub` dep unconsumed until F wires
  `control::run`.
- **F started by `stage run --phase implement`** (claude-cli executor, claude-opus-5-5, effort
  xhigh) in the background; duration and cost captured for the run report.

## 2026-10-10 · Implement attempt 1 — STAGE FAILED (permission), operator decision (O)

- F (claude-cli, claude-opus-5-5 xhigh) ran 463.8 s, 33 turns, usage in 44 / out 49033 / reasoning
  35698, cache-read 1932124 / cache-write 139512, cost n/a — and wrote **no production code**: the
  stage's write allowlist follows `.aftersight/pipeline.config.json` `paths.production`, which
  covers only `crates/*`, so every write under `plugins/herdr-holler/**` was permission-denied.
  F refused to bypass the boundary via shell writes and stopped with a complete implementation
  plan in `handoff-F.md` (module layout, TTL decision 120 s, refusal-vs-fault semantics, and three
  findings for T/operator — see below).
- **Operator decision (asked and answered):** extend the globs on the run branch —
  `paths.production` gains `plugins/*/src/**`, `plugins/*/Cargo.toml`,
  `plugins/*/herdr-plugin.toml`; `paths.test` gains `plugins/*/tests/**`. Same class as the
  A-approved Cargo.toml members exception; carried by this run's PR. The shared-with-#674 surface
  (`test.unit.command`) is untouched.
- F findings carried forward: (1) the pinned action argv `["holler","pane",<verb>]` cannot carry
  the pane operand (CLI-side wiring is #649 territory) — follow-up issue, actions stay as pinned;
  (2) the SYNC suite covers three of A-warn 2's four cases (no `at>0` with no session of record);
  (3) unknown-never-stale holds in-process; across invocations the TTL (120 s) is the backstop —
  a watch loop or persisted last-shown is a follow-up, not this contract.
- Implement attempt 2 started after the config edit landed.

## 2026-10-10 · Implement attempt 2 — STAGE FAILED again; corrected root cause (O)

- Attempt 2: 433.2 s, 48 turns, usage in 78 / out 42299 / reasoning 24668, cache-read 3731115 /
  cache-write 146508, cost n/a. Same refusal, no code written — and F's attempt-2 handoff
  **corrected attempt 1's diagnosis**: the write boundary is NOT `paths.production` but the
  ordered `permission.edit` map in the installed role doc `<primary>/.aftersight/agents/
  feature-implementor.md` (read by the runner's claude-cli executor; `boundary.ts` →
  `toClaudeCliArgs`). Attempt 1's config commit `0d2a03d` was necessary (the shim derives
  role maps from `paths.*`) but not sufficient: the installed doc predates it.
- Fix (operator-approved unblock, following the machine's established pattern — run
  0700-implementation installed Holler's `crates/*` entries the same way, with a dated comment):
  additive edit of the shared role doc — allows `plugins/*/src/**`, `plugins/*/Cargo.toml`,
  `plugins/*/herdr-plugin.toml` (bare + worktree forms) before the ADR/handoffs allows, and
  `plugins/*/tests/**` denies in the trailing test-deny group, which stays LAST. Nothing
  existing removed or reordered; siblings' boundaries only gain paths outside their blast radii.
  The primary's `.aftersight/pipeline.config.json` also carries the same paths edit locally
  (uncommitted; the run's PR lands the tracked change, after which the primary matches main).
  `.aftersight/agents/*.md` is gitignored per-machine state — the shim writes opencode.json and
  prompts/ only, so the role doc edit cannot drift a generated artifact the shim would overwrite.

## 2026-10-10 · Operator order — primary pipeline-config edit lock (O journals)

- While any other pane has a stage running, this run does not edit `.aftersight/pipeline.config.json`,
  `opencode.json` or `.aftersight/agents/*` in the primary again (the attempt-2→3 role-doc widen
  happened with sibling runs in flight and changed their stage boundaries too). Any further such
  change stops and goes to MO first. Recorded in the session memory; binding from now on.

## 2026-10-10 · Implement attempt 3 — STAGE OK (O journals; F = feature-implementor on claude-cli)

- After the role-doc fix, F implemented: 790.9 s (13m11s), 66 turns, usage in 114 / out 60953 /
  reasoning 27596, cache-read 6518282 / cache-write 163206, cost n/a; 1 artifact (handoff-F.md),
  verdict none (the t-green crossing renders GREEN, not F).
- Files: `plugins/herdr-holler/{Cargo.toml, src/lib.rs, src/herdr.rs, src/report.rs, src/main.rs,
  herdr-plugin.toml}` per the attempt-2 plan (minimal Herdr client mirroring the adapter's
  transport discipline; report params with pane-only ttl_ms; `Endpoints::from_env`; refresh +
  degraded path; `resolve_action_pane`; `herdr-holler refresh` bin). Uncommitted at stage end —
  O commits immediately per the git convention.

## 2026-10-10 · Phase T-green — GREEN-CONFIRMED + Tier 2 (O journals; T = tester)

- Dispatched via the task tool; recorded with `stage run` (STAGE OK, attempt 1).
- The plugin's t-green crossing rendered **GREEN (exit 0)** on attempt 3 of 5. Attempts 1–2 were
  RED from **environmental contention**, not code: the captured tail showed `holler-cli`'s
  `body_confirm_test` dying with no panic line (abnormal kill signature) while sibling panes were
  mid-suite in the shared warm `target/`; the same command passed in this session twice, the test
  passes 4/4 in isolation, and by attempt 3 the sibling runs had closed and the crossing went
  green. herdr-holler itself was green in every run. Journaled as the run's second odd thing.
- T's evidence: narrow 10/10; full **147 targets, 1762 passed, 0 failed, 25 ignored, 1 filtered**
  (the configured skip). Tier 2: honest-not-vacuous GREEN (unknown-never-stale really removes the
  hub socket after a success; the allowlist scan really walks src + manifest; stubs→implementation
  is a natural A/B), F's diff confined to plugins/herdr-holler/** (+Cargo.lock, +the pre-F
  operator-approved chore), tests byte-identical to T-red, fmt/clippy -D warnings/no-unsafe/
  no-#[allow]/machete/binary smoke all clean, all four A-warns verified in code as written.
- F's three findings ruled acceptable-as-contracted (bare action argv = #649 follow-up; SYNC
  3-of-4 cases = suite-widening follow-up; in-process unknown + TTL backstop = the one-shot
  binary reality) — recorded as follow-ups, none a wrong test. No test repairs.
