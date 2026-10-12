# Decision journal: #878, pane compact

Run handoff directory `docs/handoffs/878` · branch `issue-878-implementation` · rigor in-session · uiSurface false (D and U
are N/A). Append-only; every phase adds its entry.

## A, up-front plan review: 2026-10-12

- **Decided:** BLOCK, with 1 `block` and 8 `warn` findings (`docs/handoffs/878/handoff-A.md`).
  - **The block:** the engine is placed in `holler-pane` but told to reuse `resolve_pane_record` and the pane-state messages, which live in `holler-cli` (`say_cmd.rs`). The plan can only duplicate them or invert the dependency, and it gives T no engine signature.
  - **The fix:** the CLI resolves the record (the `interrupt_pane` precedent), then calls `tx_compact::compact(ports, &Pane)`.
  - **The warns:** private `mod plan` hides the `pub(crate)` checks; six test files build `Timeouts` by literal; `GET /config` may hold a credential and must be reported with `unexpected_status`; the 120 s bound also covers the cheap reads; `lib.rs` is near 800 lines; a model id may contain `/`; check order hides `compact-unsupported`; ADR section 2 wording.
- **Assumed:** the brief's file and line citations, which I spot-checked against the worktree at `7812ca2` (all matched). I ran no build or test, per the role.
- **Evidence:** read the brief in full; `ports.rs`, `ports/router.rs`, `tx_switch.rs`, `tx_switch/plan.rs`, `say_cmd.rs`, `interrupt_cmd.rs`, `gate.rs`, the OpenCode adapter's `lib.rs`, `prompt.rs`, `server.rs`, `pane/mod.rs`; `grep` for every `impl HarnessPort for` and every `Timeouts {` literal.
