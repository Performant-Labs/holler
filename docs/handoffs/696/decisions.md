
## A (up-front plan review) — 2026-10-12
- **Decided:** PASS, six warns (payload for `ReaderFailed`; no flat re-export of generic names; both-streams-then-wait rule; read-error semantics per `over` mode; named homes for the OpenCode/Claude mapping; `&mut Command` and a stub using an existing `RunError` variant).
- **Assumed:** the operator's six decisions of 2026-10-12 are final and not re-reviewed; thread names are not pinned by any test (not checked).
- **Hedged:** the read-error semantics (finding 4) are unreachable in tests, so the choice is a judgement, not verified.
- **Evidence:** read probe.rs 1-330, the three exec.rs in full, ADR-0021 lines 76-92 and 385-410, lib.rs of holler-pane, `grep try_wait crates/*/src`, call sites of `exec::` in the three adapters.
