# Handoff-T-green: Phase 7 - #669 hub plumbing for `pane/*` and `profile/*`

**Date:** 2026-10-09
**Branch:** issue-669-implementation
**Issue:** #669
**Handoff-F reviewed:** docs/handoffs/669/handoff-F.md
**Handoff-T-red:** docs/handoffs/669/handoff-T-red.md

## GREEN confirmation

`cargo test -p holler-hub --test pane_dispatch_test` -> `10 passed; 0 failed` (3 consecutive runs, identical).
Full suite (CI's command): `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` -> exit 0, 937 passed, 0 result blocks with failures. Matches F's reported 937.

Spot-check that the tests pin behavior: temporarily guarding the new `is_pane_method || is_profile_method` arm in `control_server.rs` with `false &&` made 3 tests fail
(`every_pane_method_is_forwarded...`, `every_profile_method_is_forwarded...`, `two_connections_share_the_one_pair_of_state_handles`). File restored byte-for-byte from a backup; `git diff` on it is empty.

## Test repair (F's "Tests that look wrong" item 1)

`crates/holler-hub/tests/pane_dispatch_test.rs` lines 1-6: the multi-line `#![allow(...)] // #669` failed lint check 1 (link not on the `#![allow(` line). Replaced by two single-line attributes, each with `// #669`:
`#![allow(clippy::unwrap_used, clippy::expect_used)] // #669` and `#![allow(clippy::panic, clippy::unreachable)] // #669`. `rustfmt --check --edition 2021` is clean; same four lints, no behavior change. Item 2 (control/status reads `listening.json`, not hermetic for one read) is correct as written: read-only, shape-only assertion.

## Tier 1 results

| Command | Result |
|---|---|
| `bash scripts/lint.sh` | exit 0 (was exit 1 before the test fix), only pre-existing size warnings |
| `bash scripts/changelog-check.sh` | ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | PASS, 937 passed |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3 passed) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3 passed) |
| `cargo machete` | no unused dependencies |

## Tier 2 results

- Coverage per AC: PASS (see below).
- Test quality: PASS. 10 tests, one behavior each, at the cheapest tier that reaches `handle_control_conn` (a `holler-hub` test, no process); no duplication seen. `testkit_links` is a deliberate dev-dependency link check, small.
- Error handling: PASS. Unknown `pane/*` / `profile/*` names still answer `-32601`; stubs answer `not-implemented` as a result.
- Protocol-visible change: none (no golden, no `docs/protocol/v2.md` change needed; `CATALOG` unchanged). `docs_cli_test` passes.
- File sizes: `control_server.rs` 837, `serve.rs` 838, under the 900 fail line.
- Allow links: PASS (lint).
- Secrets in logs: no logging or token handling added.
- Evidence appendix: spot-verified `evidence.md` entries against source (`encode_response` / `encode_error` at control_server.rs 705/710, `PaneReply::failure` at reply.rs 71, error.rs 95 and 414, `is_pane_method` exists); excerpts are verbatim, no additions needed. Note control_server.rs line numbers in the entries are post-edit and match.
- No browser/UI surface: U is N/A.

## Acceptance criteria status

The brief's criteria are backed by the 10 tests in `pane_dispatch_test.rs`: forwarding of every `pane/*` and `profile/*` method (PASS), `not-implemented` as a JSON-RPC result carrying a `PaneReply` (PASS), unknown names and existing `control/*` unchanged (PASS), shared state handles across connections and non-`Clone` state types (PASS), `profile/rename` routed to its stub and `check_membership` stub (PASS), AC 7 lint/clippy/machete gates (PASS after the test-line fix).

## Blocking issues

None.

## Advisory notes

- `serve.rs` wiring (`build_shared_state` -> `accept_loop`) has no automated test (private; A finding 3). F's live-hub check covers it; the holler-cli suites also run through the changed `serve_forever`.
- Open for O, unchanged from F: A finding 2 (`check_membership` cannot see the stored record) and finding 7 (`pane_wiring.rs` placeholder); `rename.rs` stub deviates from the issue's word "empty" (reasoned in F's handoff).
- CI's separately retried load-roster test and `--ignored` interop step were not run (the latter matches 0 tests per F).
