# Handoff-A-dup: #846 linkedhost-fakes  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-846-implementation
**Diff base:** 6eff9cc   **Diff head:** cfdd734
**Reuse map:** docs/handoffs/846-brief.md ("Files" / "Reuse map")
**Verdict:** PASS

## Summary
F extended `tests/verb_harness/` (the brief's home, already path-included by all four targets) with one generic `LinkedHost<H, L>`
and removed all four per-rig copies. The propagate/swallow and kill/no-kill differences are named enums (`OnStart`, `OnStop`). The
W-3, W-6 and W-7 near-copies are one each. No parallel path was built; no production code or `Cargo.toml` changed.

## Verification
- `grep -rnE "struct LinkedHost|HostPort for LinkedHost" crates/*/tests`: one hit each, `verb_harness/linked_host.rs:46/51`. The only other
  `HostPort` impl in `holler-cli/tests` is `LostChain` (`import/harness.rs:307`), named out of scope by the brief.
- Kill loop: defined once, `OnStop::end`, called from `FakeHarnessLink::stopped` and launch's `HookedLink::stopped` (A's Phase-3 warn 4 check).
  `launch/rig.rs:546` (`for port in RIG_PORTS`) is an assertion that no server runs, not a copy.
- `HookedLink` in the launch rig is justified in the brief: its `started` fires the rig-local server-start hook with the started port, which
  the shared links cannot. It reuses `OnStop` for the stop side.
- `opencode-ran`: one hit, `pane_integration/traps.rs`. `no live holler hub reachable` in the `pane_integration` target is built from `traps::NO_HUB`.
  Hits in other test targets (`hub_serve_test`, `query_test`, `roster_panes`, ...) are separate targets outside the brief's scope.
- `CallLog`, `herdr_suite_over`, `unbuildable_lazy_herdr` live in `pane_integration/wrappers.rs`, neutral to `bind` and `observe` (A's Phase-3 warn 3).
  The two remaining `LazyHerdr::new` uses in `bind.rs:123` and `observe.rs` count builds of a working fake, a different shape.
- Reuse of the test kit: `FakeHost`, `FakeHarness`, `FakeHarnesses` are used as they are; no new testkit type. The Hub, Body, `mint_token`, `join`,
  `wait_for`, `StateDir` helpers, token store, `Lockout`, `Roster` and `log(Severity, ...)` candidates are not touched.
- Diff touches only `crates/holler-cli/tests/` and `docs/handoffs/`.

## Findings
| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane-testkit/src/harnesses.rs:9-12` | Still says the pre-#831 rigs "keep their own linking"; now stale. Known to F (handoff-F, Known issues). | Fix with the D2 follow-up that folds `LinkedHost` into the testkit. |
| 2 | warn | `verb_harness/linked_host.rs` module doc | Cites no follow-up issue number for D2; none is filed. | O files the issue and adds the number. |

No duplication; extension is clean.

## Notes for F
None (PASS).
