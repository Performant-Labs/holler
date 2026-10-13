# Handoff-F: Phase 5 - #878 pane compact (Part A)

**Date:** 2026-10-12
**Branch:** issue-878-implementation
**Issue:** #878

## What was done
- `crates/holler-pane/src/ports/compact.rs` (new): `CompactOutcome`, `CompactModel`, the open codes `COMPACT_UNSUPPORTED` and `COMPACT_MODEL_UNPINNED`, and the default's error.
- `crates/holler-pane/src/ports.rs`: `HarnessPort::compact(at, session, model: Option<&CompactModel>)`, default-armed (`compact-unsupported`); re-exports; amendment note in the module docs.
- `crates/holler-pane/src/ports/router.rs`: `HarnessRouter` forwards `compact` by endpoint; routing table updated.
- `crates/holler-pane/src/tx_compact.rs` (new): the engine `compact(ports, &Pane)`: endpoint, `session-not-found` for no session of record, `check_health`, `check_idle`, the act with `None`, the shrink rule (`unavailable` naming both counts).
- `crates/holler-pane/src/tx_switch.rs` / `tx_switch/plan.rs`: `check_health` and `check_idle` are `pub(crate)` and re-exported from `tx_switch` (visibility only; rustfmt re-wrapped the two signatures).
- `crates/holler-pane/src/lib.rs`: `pub mod tx_compact;` and module docs.
- `crates/holler-pane-testkit/src/harness.rs`: `HarnessOp::Compact`, `CompactRun`, `seed_context`, `set_compaction_pin`, `set_activity`, `compact_runs`; `session_activity` answers the seed (not logged); `compact` in T's check order.
- `crates/holler-pane-testkit/src/harnesses.rs`: `FakeHarnesses` forwards `compact`.
- `crates/holler-adapter-opencode/src/compact.rs` (new): the adapter's `compact` (known, activity re-check, model from the argument or `GET /config` pin split at the first `/`, then on a compaction `Call`: tokens before, one `POST /session/:id/summarize`, tokens after); `SUMMARIZE` and `OP_COMPACT` live here.
- `crates/holler-adapter-opencode/src/lib.rs`: `Timeouts::compact` (default 120 s), `mod compact;`, the trait method, one doc sentence (793 lines, under 800).
- `crates/holler-cli/src/pane/compact.rs` (new): `PaneCompact` (`PANE`, `--profile`), `resolve_pane_record` then the engine; `PaneNotFound` re-asked as `unmanaged::missing`; `output::emit` with a local `Compacted`.
- `crates/holler-cli/src/pane/mod.rs`: `pub mod compact`, `PaneCmd::Compact`, the `acting_pane` and `run` arms.
- `crates/holler-cli/src/pane/wiring/bind.rs`: `Wired` forwards `compact` (doc list updated). `crates/holler-cli/src/pane/wiring.rs`: `Unwired::compact` answers `not-implemented`.
- Docs: `docs/adr/ADR-0021.md` (section 2 bound, ports row, verb list, "Compaction (#878)" paragraph, failure-modes row, Deferred entry, decision 50), `docs/adr/ADR-0003.md` (the `pane compact` row, its own block), `docs/pane-control.md` (verb list, acting-verb list, `### holler pane compact PANE`, open codes, section 12), `CHANGELOG.md` (`[Unreleased]` / Enhancements).

## Design decisions
- **Fake keys context and activity by data directory and session** (as `turns` are), the pin by port (a server's config). The seeded pair is answered and left as it was (T's contract: "answers the seeded pair").
- **Adapter's compaction `Call` is opened after the model is resolved**, `Call { request: compact, ..call_until(port, OP_COMPACT, deadline_after(compact)) }` (A warn 2), so a hang on any read before it ends at `call`.
- **Summarize success is status `200` alone**, body not inspected (T's contract, "success is 200 only"); any other status is `unexpected_status`.
- **Token count:** sum of `input` + `output` (required) + `reasoning`, `cache.read`, `cache.write` (optional, `null` treated as absent) of the last assistant message; overflow or a non-count is `unavailable` naming the route and status. The last assistant message is used even when an earlier one is readable (never a guess).
- **`GET /config` that is not a JSON object** is `unavailable` (route/status only), while an object with no usable pin is `compact-model-unpinned`; neither message quotes the config. The unpinned message does not echo the pin value (it may be anything).
- **Verb's JSON `session`** is the raw session of record (as `rig.live(P).session`); text mode does not print it.

## Reuse / extend-vs-new
Extended, per the brief's Reuse map: `check_health`/`check_idle` (called through a `pub(crate)` re-export), `activity_refusal` (the fake and the adapter), `resolve_pane_record` (CLI only), `unmanaged::missing`, `HarnessInfo::endpoint`, `known`, `Call::send`/`unexpected_status`, `prompt::session_activity`, `prompt::messages`, `output::emit` (the `close.rs` pattern, A warn 1; `switch.rs` untouched). New objects are the ones the brief names: `ports/compact.rs`, `tx_compact.rs`, the adapter's `compact.rs`, the verb file.

## Architecture notes for A
- `HarnessPort` gains one default-armed method; every wrapper (router, `FakeHarnesses`, `Wired`, `Unwired`) forwards or overrides it explicitly.
- `holler-pane-testkit` now imports `holler_pane::tx_switch::activity_refusal` (already public); the OpenCode adapter imports it too. No new crate dependency.
- `tx_switch::plan` helpers widened to `pub(crate)`; `plan` stays private.
- No protocol, hub, wire, registry or log-event change; `schema_version` unchanged.

## Deviations from spec / wireframe
None. (ADR-0003 got a row: T's handoff asked for it; it is not in the brief's file list.)

## Tier 1 self-check (incl. tests now GREEN)
- `cargo test -p holler-pane-testkit`: all binaries ok (incl. `fake_harness_compact_test` 10/10, `fake_harnesses_test`, `harness_conformance_test`, `claude_conformance_test`).
- `cargo test -p holler-adapter-opencode --test compact_test --test hermetic_test`: 17/17, 33/33.
- `cargo test -p holler-cli`: `pane_verbs` 424/424, `pane_claude` 50/50, `pane_integration` 77/77, `pane_cli_process` 37/37, `cli_surface_test` 3/3, `docs_cli_test` 3/3.
- `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`: 187 binaries, 2925 passed, 1 failed: `holler-body` `rebind_test::the_re_attach_after_an_error_follows_the_new_id` ("the turn ends: Elapsed(())"), untouched code, passed 3/3 when re-run alone. An earlier fail-fast run also hit `holler-cli` `interrupt_test::interrupt_idle_session_is_ok_noop` (a body "reconnecting" during warm-up), which passed 11/11 alone. Both are load-timing flakes outside this diff; T should re-run them in Phase 6.
- `cargo clippy -p holler-pane -p holler-pane-testkit -p holler-adapter-opencode -p holler-adapter-claude --all-targets -- -D warnings`: clean. `cargo clippy -p holler-cli --lib --bins --test pane_verbs --test pane_claude --test pane_integration --test pane_cli_process -- -D warnings`: clean. (`--all-targets` on `holler-cli` still stops at the pre-existing `token_load_test.rs:60`.)
- `bash scripts/lint.sh`: exit 0 (warns only: `harness.rs` 830, `opencode/src/lib.rs` 793, both under 900). `bash scripts/changelog-check.sh`: ok. `rustfmt --check` on the touched files: clean.

## Evidence appendix
`docs/handoffs/878/evidence.md`

## Tests that look wrong (for T)
None.

## Known issues
- The OpenCode summarize request, its `200`, the `agent.compaction.model` in `GET /config` and the token source are from the 1.19.0 OpenAPI document and the restart script, never run (Part B.1 confirms).
- Two load-timing flakes in untouched tests (above).

## Files changed
- crates/holler-pane/src/ports.rs
- crates/holler-pane/src/ports/compact.rs
- crates/holler-pane/src/ports/router.rs
- crates/holler-pane/src/tx_compact.rs
- crates/holler-pane/src/tx_switch.rs
- crates/holler-pane/src/tx_switch/plan.rs
- crates/holler-pane/src/lib.rs
- crates/holler-pane-testkit/src/harness.rs
- crates/holler-pane-testkit/src/harnesses.rs
- crates/holler-adapter-opencode/src/compact.rs
- crates/holler-adapter-opencode/src/lib.rs
- crates/holler-cli/src/pane/compact.rs
- crates/holler-cli/src/pane/mod.rs
- crates/holler-cli/src/pane/wiring.rs
- crates/holler-cli/src/pane/wiring/bind.rs
- docs/adr/ADR-0021.md
- docs/adr/ADR-0003.md
- docs/pane-control.md
- CHANGELOG.md
