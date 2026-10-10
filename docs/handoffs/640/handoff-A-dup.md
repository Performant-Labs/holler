# Handoff-A-dup: Phase 7 - #640 part 2 of 3: the socket transport, `HerdrAdapter` and a simulated Herdr  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-640-implementation
**Diff base:** `3bdd129` (`origin/main`, the merge base)   **Diff head:** `0fbe0e5`
**Reuse map:** `docs/handoffs/640-brief.md`, section "Reuse map (extend, do not duplicate)"
**Verdict:** PASS

## Summary

PASS, with no block findings and four warns. F built on every object the Reuse map named. Every request is a
part-1 `protocol::Request`, and every reply goes through `decode_reply` and the part-1 parsers (`parse_pong`,
`check_supported`, `parse_snapshot` with `SessionState::workspace`, `parse_layout_export`,
`parse_workspace_created`, `parse_pane_info`, `parse_read`, `expect_ok`). `adapter.rs` and `transport.rs` contain no
`json!`. The one tree walk is `layout::grid_of` with `GridMap::{at, position_of, cells}`. Placement comes from
`plan::plan_splits` with a one-cell `Target`, and the adapter does not check range or reachability a second time.
`PaneError` is the only error type. The two new modules and the `Transport` seam are the objects the brief itself
specifies (Files table, the pinned API, Decisions 3, 14 and 18), so they are planned new objects, not a parallel path.
The socket client follows `holler_hub::control::send_over` as a pattern only, with no dependency on `holler-hub`
(ADR-0021 §5), and closes that client's three gaps.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `src/adapter.rs:231-240`, `:372-374`, `:386-390` | This is the A finding 2 that Phase 3 predicted. The new messages quote Herdr-sent text (the new pane id, a workspace label from the snapshot, the split target's id) with `{:?}`. They do not go through `protocol::excerpt` (`protocol.rs:580`, private, cut to 64 characters), because the brief freezes part 1. The result is the right one for duplication: there is no third copy of `excerpt`. But the crate's message rule (`protocol.rs:20-21`: "quote what Herdr sent (cut to 64 characters)") holds only in part: every message stays on one line, but its length is not bounded. F lists this as known issue 3. | A follow-up, or part 3 if its brief allows it: make `protocol::excerpt` `pub(crate)` (a visibility-only edit) and route these three quotes through it. Nothing needs to change in this part. |
| 2 | warn | `src/transport.rs:250-255` (`op` = `herdr.<wire method>`), passed through unchanged by every port method in `src/adapter.rs` | This is A finding 1, which was not adopted, and F followed Decision 3 as written. `FakeHerdr`/`PortOp` name a timeout by the port method (`herdr.ensure_pane`, `holler-pane-testkit/src/fault.rs:20-21`), and this adapter names it by the wire method (`herdr.layout.export`, `herdr.pane.split`, ...). This is a vocabulary divergence between the fake and the real adapter, not duplicated code. No ADR rule settles it. | Carry it, together with Decision 20, into part 3's ADR-0021 §9/§10 rows and the operator's list (F known issue 2, T-green advisory 1). |
| 3 | warn | `docs/adr/ADR-0021.md:426` (not touched by this diff) | ADR currency. This part settles three things that the ADR leaves to #640 or does not state yet: a workspace's extent comes from configuration, per label (`HerdrConfig.workspaces`, `adapter.rs:51`); the version gate runs at `connect` and in `version()` only (`adapter.rs:95-100`, `:346-355`); and the `op` rule in finding 2. The ADR is not updated in this PR. The deferral is in writing (the brief's Scope table and Out of scope, A finding 7), #640 is one issue whose part 3 writes these rows, and this PR's blast radius does not cover `docs/adr/`. So this is not a silent contradiction. | Part 3's brief names all three items when it writes the §9/§10 rows. |
| 4 | warn | `src/transport.rs:88-286`, `src/adapter.rs:41` | This is the first bounded-exchange code in an adapter (a worker thread per exchange, socket timeouts recomputed from one deadline, `recv_timeout`) and the first code constant for I5's 10 s (`DEFAULT_TIMEOUT`). The workspace has no shared helper it should have used: `holler-pane` defines no I5 constant, and the only other `recv_timeout` sites are in `holler-load-test`. So the code is correct where it is. #642 (OpenCode) will need the same code, and ADR-0021 §5 bars it from importing this crate. | No change now. #642's brief should look here first, and should raise a shared home (an I5 constant in `holler-pane`, and a bounded-exchange helper if the two shapes match) instead of a second copy (A finding 6). |

No duplication; extension is clean. Checked and not a finding:

- **Deliberate test-code duplication** in the wire fake (`tests/wire_herdr/mod.rs:632-650`: `last_lines` and
  `base36`; its own split tree, `:78-88` and `:500-579`; its own `Direction` to string match, `:137-140`, in place of
  `Direction::as_str`). The Reuse map justifies this in writing as the oracle's independence from `src/`, and AC 35
  enforces it. It is a reviewed decision, so it passes.
- **A finding 5 applied.** One `Tap` interceptor (`tests/wire_herdr/mod.rs:212-251`) serves AC 16, 17, 28, 29 and
  T's version-form test. There are no per-test `Transport` near-copies.
- **Test servers.** `transport_test.rs`'s `serve_one`/`hold_every_connection` (`:81-108`) and `wire_herdr/serve.rs`
  do different jobs: the first are misbehaving servers written inside each test (AC 1-12a), and the second serves a
  well-behaved fake. Neither copies the hub harness (`Hub`, `Body`, `StateDir`, ...), which none of these tests
  needs.
- **Private helpers.** In `adapter.rs`, `deadline_after` uses the workspace's `checked_add` idiom
  (`holler-hub/src/panes/store.rs:268`, `holler-pane-testkit/src/feed.rs:201`), and the panic it guards against is
  now pinned by an AC 27 case. `usage` is a two-line constructor. `Exchange`'s message builders are module-private,
  as part 1's `protocol::unavailable` is. The `version()` form check is Decision 8's rule: T showed that `parse_pong`
  accepts the strings it refuses, so the check does not duplicate part 1.
- **The rest of the stack.**
  - File size: every file is under 900 lines. The largest are `src/adapter.rs` (445), `tests/wire_herdr/mod.rs`
    (650), `tests/adapter_test.rs` (645) and `CHANGELOG.md` (638). `scripts/lint.sh` exits 0.
  - No protocol, golden file, `holler-proto` file, `docs/protocol/v2.md` or ADR file is touched.
  - Every `#[allow]` in the crate carries `// #640`, and `src/` has none.
  - The diff outside `docs/handoffs/` contains no personal host, user, path or IP. The test paths are tempdirs,
    `/unused/h.sock` and the fake's `/scratch` cwd.
  - Layering: `adapter` uses `transport`, `protocol`, `layout` and `plan`, and `transport` uses only
    `protocol::Request`. There is no new normal dependency, and the only new manifest line is the dev-dependency
    `tempfile`.

## Notes for F

None. The verdict is PASS. Findings 1-4 are follow-ups for part 3's brief, #642's brief, and the operator.
