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
