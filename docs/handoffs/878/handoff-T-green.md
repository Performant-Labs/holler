# Handoff-T-green: #878 pane compact (Part A), phase 7 (verify GREEN + Tier 2)

**Date:** 2026-10-12
**Branch:** issue-878-implementation
**Issue:** #878
**Handoff-F reviewed:** docs/handoffs/878/handoff-F.md
**Handoff-T-red:** docs/handoffs/878/handoff-T-red.md

## GREEN confirmation
The authored suite passes on F's code; no test was changed in this phase (F flagged none as wrong).
- `cargo test -p holler-pane -p holler-pane-testkit -p holler-adapter-opencode -p holler-adapter-claude --no-fail-fast`: every binary ok, 0 failed (`fake_harness_compact_test` 10/10, `fake_harnesses_test` 8/8, `harness_conformance_test` 20/20, `claude_conformance_test` 4/4, opencode `compact_test` 17/17, `hermetic_test` 33/33 and claude 18/18).
- `cargo test -p holler-cli --test ...`: `pane_verbs` 424/424, `pane_claude` 50/50, `pane_integration` 77/77, `pane_cli_process` 37/37, `cli_surface_test` 3/3, `docs_cli_test` 3/3, `wire_selftest` 3/3.
- Spot-check that the tests pin behavior (production edited, then restored with `git checkout`; tree clean after):
  - shrink rule weakened in `tx_compact.rs`: 2 tests fail (`a_context_that_did_not_shrink_is_unavailable_naming_both_counts`, `compact_that_does_not_shrink_exits_1_naming_both_counts`).
  - `split_once` replaced by `rsplit_once` in the adapter: `the_pin_is_split_at_the_first_slash` fails.

## Tier 1 results
| Command | Result |
|---|---|
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | one failure: `holler-body` `rebind_test::a_rebind_re_points_the_session_and_keeps_its_name` (load timing, untouched code); `rebind_test` 8/8 three times alone. FAIL under load, PASS alone: flake, not this diff |
| `cargo test -p holler-cli --test interrupt_test` (F's earlier flake) | 11/11, PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | the only error is `token_load_test.rs:60` `unneeded cloning of iterator items`; the file is untouched by this branch (not in `git diff origin/main`). With `--keep-going` nothing else errors. PASS for this diff |
| `bash scripts/lint.sh` | exit 0 (warns only; `harness.rs` 830, under 900) PASS |
| `bash scripts/changelog-check.sh` | ok PASS |
| `cargo machete` | not installed on this machine (`no such command`); not run. Note for CI, which is the source of truth |

## Tier 2 results
- Coverage: every acceptance criterion has a test (below). PASS.
- Test quality: tests name behaviors, readiness uses the stub's hold/hang and `wait_for`, no fixed-sleep readiness, bounds asserted as "ended near the bound" with margins. The adapter `compact_test` is 17/17 across the runs here (2.6 s). PASS.
- Error paths: no-shrink, busy/question/permission, unpinned, unhealthy, unknown session, unmanaged pane, bad status, garbled bodies, timeouts: all tested. PASS.
- Secrets: the leak test checks no reply body (config, messages, summarize) reaches an error. PASS.
- Protocol: no wire, hub, registry or log-event change, so no golden drift; `docs_cli_test` and `cli_surface_test` green, ADR-0003 row present. PASS.
- File sizes: all under 900 lines; no new `#[allow]` found in the diff. PASS.
- Playwright/UI: none in this repo.

## Acceptance criteria status
- AC1 PASS: `compact_prints_before_and_after_on_the_harness_pinned_model`.
- AC2 PASS: `compact_refuses_a_busy_session_and_sends_nothing`.
- AC3 PASS: `compact_without_a_pinned_model_is_refused_and_sends_nothing`; `an_idle_claude_pane_is_compact_unsupported`.
- AC4 PASS: `compact_that_does_not_shrink_exits_1_naming_both_counts` and the engine test.
- AC5 PASS: `switch::both` in every verb case; `assert_inert` in `pane_claude/compact.rs`.
- AC6 PASS: conformance cases 20 and 14, plus mutants.
- AC7 PASS: adapter `compact_test`.
- AC8 PASS: router, `FakeHarnesses`, `Wired`, `Unwired` forwarding tests.
- AC9 PASS for what T runs (lint, changelog, clippy on touched files, workspace tests); `cargo machete` is for CI.

## Blocking issues
None.

## Advisory notes
- Two load-timing flakes in untouched code (`holler-body` rebind tests, `holler-cli` `interrupt_test`); both pass alone.
- `pane_integration` takes about 150 s (`fake_hub` test noted "running over 60 seconds"); pre-existing.
- The OpenCode summarize request, its 200, the `agent.compaction.model` config and the token source remain unverified against a real server (Part B.1).
