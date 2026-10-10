# Handoff-T-green: Phase 6 — verify GREEN + Tier 2 for #651 (Herdr display plugin)

**Date:** 2026-10-10
**Branch:** `issue-0651-implementation` (run worktree `<run-worktree>`, repo-relative
`.claude/worktrees/0651-implementation`)
**Issue:** #651
**Handoff-F reviewed:** `docs/handoffs/0651-implementation/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/0651-implementation/handoff-T-red.md`

All commands below ran in the run worktree, one cargo at a time. The worktree's `target/`
symlink points at `<primary>`'s shared cache — F's warned environment fault did not
recur: no cross-worktree fixture paths, no `opencode_agent` errors, no file-lock errors.

## GREEN confirmation

**Narrow** — `cargo test -p herdr-holler --no-fail-fast` → **exit 0**:

| target | result |
|---|---|
| `tests/action_resolution.rs` | 2 passed, 0 failed |
| `tests/display_only_allowlist.rs` | 2 passed, 0 failed |
| `tests/manifest.rs` | 3 passed, 0 failed |
| `tests/report_contract.rs` | 3 passed, 0 failed |
| lib unit / doc-tests | 0 tests (empty) |

**10 passed, 0 failed** — the 5 formerly-RED tests pass and the 5 standing guards still
hold. Identical to the implementer pane's self-check.

**Full** — `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`
→ **exit 0**: **147 test targets, 1762 passed, 0 failed, 25 ignored, 1 filtered out**
(the configured skip). Identical to the implementer pane's numbers; the rest of the
workspace stays green.

**The GREEN is honest, not vacuous** (verified by reading the implementation against each
test — no mutation testing, per the run rule):

- The git history is a natural pre/post pair: at `5b06238` (T-red, refusing stubs) the 5
  feature tests failed for the feature reasons (exact outputs in handoff-T-red); `f6141c3`
  (the implementer pane) changes only `plugins/herdr-holler/{Cargo.toml,src/**}` plus
  handoff docs — no test edit — and the 5 pass. Each test's assertions demand behavior the
  stubs could not do (resolve a pane id, derive sync through `shown_differs`, exchange
  reports over two real Unix sockets, resolve endpoints from env).
- `unknown_never_stale` does drive a **failed hub read after a successful one**: one
  `refresh()` over the live fake hub (asserted `Refreshed{1,1,false}`), then the hub
  socket file is removed (`fs::remove_file`), then a second `refresh()` asserted
  `Refreshed{1,0,true}` with exactly one degraded report, `state_labels` containing
  `unknown`, **every** token re-sent as the string `unknown` (replaying the first set
  would fail the per-token assert), `ttl_ms` present — and the implementation's degraded
  path (`src/lib.rs` `refresh`) supplies it from the remembered ids.
- The allowlist scan reads real bytes from disk: `scanned_files()` walks `src/**`
  recursively **and** pushes `herdr-plugin.toml` itself; any `namespace.word` token under
  the twelve Herdr namespaces that is not in `ALLOWED_METHODS ∪ EVENT_HOOKS` — unknown
  names included — fails the test. It is an allowlist as A-warn 3 requires.

**Test repairs made:** none. No test was wrong; none was widened (run rule: repairs only
for wrong tests, not narrow ones).

## Tier 1 results

| check | command | expected | actual | verdict |
|---|---|---|---|---|
| narrow suite | `cargo test -p herdr-holler --no-fail-fast` | 10/10 | 10 passed, 0 failed, exit 0 | PASS |
| full suite | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0; 1762/0/25/1 | PASS |
| fmt | `cargo fmt -p herdr-holler -- --check` | clean | clean, exit 0 | PASS |
| clippy | `cargo clippy -p herdr-holler --all-targets -- -D warnings` | clean | exit 0 | PASS |
| binary smoke (usage) | `./target/debug/herdr-holler` (no args) | usage, exit 2 | `usage: herdr-holler refresh`, exit 2 | PASS |
| binary smoke (unset env) | `herdr-holler refresh` with `HERDR_SOCKET_PATH` unset | exit 1, names the var | exit 1, names `HERDR_SOCKET_PATH` (name only, no value) | PASS |
| unused deps | `cargo machete plugins/herdr-holler` | none | none found | PASS |

(The Rust analogue of "server starts / API smoke": the two binary invocations above, both
inside the crate's own temp-dir discipline — no live hub, no Herdr session, no fleet.)

## Tier 2 results

- **Boundaries** — `git diff $(git merge-base HEAD origin/main) HEAD --stat` (= `3d95aec`,
  the brief's stated cut): the run touches `plugins/herdr-holler/**` (crate, manifest,
  src, tests), `docs/handoffs/0651-implementation/**`, `Cargo.toml` (+`"plugins/*"` to
  `members` — exactly the declared one-line exception, now with its comment), `Cargo.lock`
  (+12 lines, the declared generated side effect), and `.aftersight/pipeline.config.json`
  (see note below). **No `crates/**` edits.** The initial `git diff origin/main --stat`
  showed `crates/**` churn — that is `origin/main` having moved past the merge-base
  (#721/#722/#723), re-verified against the merge-base. **PASS**
- **Tests not edited by F** — `git diff 5b06238 HEAD -- plugins/herdr-holler/tests/
  plugins/herdr-holler/herdr-plugin.toml` is **empty**: the suite and the manifest are
  byte-identical to T-red's committed versions; the F commit's stat names only
  `Cargo.toml`, `src/**` and handoff docs. **PASS**
- **Operator config chore (recorded, not F's)** — `0d2a03d` ("chore(pipeline): #651 …
  operator-approved") widens `paths.test`/`paths.production` in the pipeline config to
  cover `plugins/*`, unblocking the implementer stage after attempts 1–2 were
  permission-denied. It predates F's commit, is operator-approved, and is journaled in
  `decisions.md`. Noted here so it is visible at review, not silent. **PASS (recorded)**
- **House rules** — fmt clean; clippy `-D warnings` clean; **no `unsafe`** anywhere in
  the run's src diff (grep over `git diff 3d95aec HEAD -- plugins/herdr-holler/src/`:
  no matches); **no `#[allow]` in `src/**`** (the `#![allow(clippy::unwrap_used, …)]`
  heads live only in `tests/**`, where they belong); all three run commits are
  Conventional (`test(herdr-plugin): …`, `chore(pipeline): …`, `feat(herdr-plugin): …`);
  secret scan of the run diff: env var **names** only, no values, no key/token material
  (the single grep hit is the brief's own rule text). **PASS**
- **A-warn 1 (minimal client mirrors the adapter's transport discipline)** — `src/herdr.rs`:
  one request per connection (each `send` builds a fresh `Exchange`, connects, writes one
  JSON line + `\n`, reads to the first newline, closes); one 2 s deadline bounding
  connect+write+read (worker thread, caller `recv_timeout(remaining())`, per-call
  `set_*_timeout(time left)`, the adapter's `bound`/`remaining` pattern from
  `crates/holler-adapter-herdr/src/transport.rs`); reply capped at 64 KiB (tighter than
  the adapter's 16 MiB — proportionate for a report ack); the macOS `InvalidInput`
  tolerance is the adapter's rule; `Method` is a closed two-variant enum, so no third
  Herdr request can be built. **PASS**
- **A-warn 2 (SYNC through the one rule, doc-cited)** — `src/report.rs` `sync_word`
  imports and calls `holler_pane::reconcile::shown_differs`; its doc comment cites
  `SessionSync::of` (`crates/holler-cli/src/pane/list.rs`) — verified against the real
  `SessionSync::of` (list.rs `of()`): identical case structure (`at <= 0 || no session of
  record` → `-`; `shown_differs` → `mismatch`; else `ok`), same `None`-is-home-screen
  reading. No source compares `shown` itself (the suite's scan enforces it). **PASS**
- **A-warn 3 (allowlist, not denylist)** — see GREEN confirmation above: the scan fails on
  any dotted name under a Herdr namespace that is not explicitly allowed, unknown names
  included; only `pane.report_metadata`, `workspace.report_metadata` and the read-only
  set pass, plus exactly the two wired event-hook names. **PASS**
- **A-warn 4 (no `ttl_ms` on workspace reports)** — `workspace_reports` builds
  `{workspace_id, source, tokens}` with no `ttl_ms`; both waves of both socket tests
  assert pane reports carry `ttl_ms > 0` and workspace reports carry none. **PASS**
- **Test coverage / quality (§7)** — one test per acceptance criterion (map below); every
  test names a behavior, fails in isolation for the feature reason (proven at T-red), sits
  at the integration tier — the cheapest tier that can see real Unix sockets, the
  manifest on disk, and process env — and none duplicates another (the two socket tests
  share the rig but pin different criteria). Assertions are on wire shapes and manifest
  values, not internals (the two source-scan guards are the brief's own demanded form).
  Suite is proportionate: 10 tests, none redundant, none to prune. Env-touching tests
  serialize on one mutex and restore prior values. **PASS**
- **Type safety / error handling / data integrity / security** — Rust + clippy `-D
  warnings`, no `unsafe`; error paths pinned: hub unreachable → `unknown` refresh, unknown
  `HERDR_PANE_ID` → `None`, malformed hub reply → typed degradation causes in
  `read_registry`; replies are validated (JSON object, id echo, `result.type == "ok"`)
  and size-capped; no error message carries request or reply bytes (refusals quote only a
  64-char excerpt of Herdr's own error code). No DB, no migrations (N/A). Inputs from env
  are the two socket paths only. **PASS** (advisories below)
- **Playwright / e2e** — N/A: no UI surface (D/U recorded N/A per the brief).

## Rulings on the implementer pane's three findings ("tests I think are wrong")

None of the three is a wrong test. All three are **acceptable-as-contracted for this
story**; none requires a repair now.

1. **Actions pinned to bare `["holler","pane",<verb>]` cannot carry the pane operand.**
   The brief's criterion asks exactly what the suite pins: every action is argv running a
   merged pane verb, and the target pane is resolved from the registry
   (`resolve_action_pane`, implemented and tested here) — the plugin mutates nothing
   itself. Carrying the operand to the CLI is `crates/holler-cli` wiring, explicitly
   #649's territory and outside this run's boundary. **Acceptable-as-contracted;
   follow-up lives in #649.**
2. **The SYNC suite covers 3 of A-warn 2's 4 cases** (no case with `at > 0` and no
   session of record). The code handles it — `sync_word` mirrors the `at <= 0 || record
   .is_none()` OR exactly — so nothing tested is false; the suite is *narrow*, not
   *wrong* (`demo-gamma` pins the `-` branch, `demo-alpha` `ok`, `demo-beta` `mismatch`).
   Per the run rule (repair only wrong tests), no change now. **Acceptable-as-contracted;
   recorded as a suite-widening follow-up** (add one pane with `at > 0`, no session of
   record, expecting `-`).
3. **`unknown_never_stale` holds in-process; TTL is the cross-invocation backstop.** The
   brief's criterion prescribes precisely the in-process sequence the test drives ("one
   successful refresh, then a failed read … second report set is `unknown`") and names
   `ttl_ms` as the backstop "so a plugin that stops reporting leaves expiring data" —
   which is the one-shot-binary reality (each hook a fresh process; 120 s pane TTL bounds
   staleness). A watch loop or persisted last-reported set is a design change beyond this
   story. **Acceptable-as-contracted; recorded as a follow-up.**

## Acceptance criteria status

| criterion | verdict | backed by |
|---|---|---|
| No call to the Herdr pane-changing API (allowlist) | PASS | `sources_and_manifest_name_only_allowed_herdr_methods` + closed `Method` enum |
| Unknown, never stale (+ `ttl_ms`) | PASS | `unknown_never_stale` (both waves, ttl rules asserted) |
| Per-pane display facts (six tokens, SCHEMA limits, workspace profile) | PASS | `per_pane_display_facts`, `endpoints_come_from_the_environment` |
| Every action a `holler pane` verb, registry-resolved | PASS | `every_action_is_a_holler_pane_verb`, `manifest_is_valid_and_every_command_is_argv`, `resolves_the_registry_pane_by_herdr_pane_id`, `an_unknown_herdr_pane_id_resolves_to_none` |
| Re-report triggers wired | PASS | `event_hooks_rerun_the_refresh_on_the_two_triggers` (+ binary `refresh` entry, smoked) |
| fmt / clippy / no new `unsafe` / full unit green | PASS | Tier 1 table above |

## Blocking issues

None.

## Advisory notes (non-blocking)

- The Herdr **refusal** and **fault** reply paths (`Failure::Refused`/`Failure::Fault` in
  `src/herdr.rs`) are implemented but only the `ok` path is suite-pinned (the fake always
  answers ok). A follow-up fake variation could pin them.
- The missing-env error path of `Endpoints::from_env` is covered by the binary smoke, not
  a committed test.
- Follow-ups 4–6 from handoff-F carry forward unchanged: workspace `profile` tokens carry
  no TTL by design (spike §4) so they persist while the hub is down; a pane leaving the
  registry keeps its last report until TTL; token values are sent verbatim (terminal-safe
  escaping unverified on Herdr's side).
- Per the run instruction, `handoff-T-green.md` is the only file this phase wrote; the
  `decisions.md` T-green entry is left for O to journal.

VERDICT: GREEN-CONFIRMED
