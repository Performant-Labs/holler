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

## A, up-front plan review, round two: 2026-10-12

- **Decided:** PASS, 0 `block`, 5 `warn` (`docs/handoffs/878/handoff-A.md`, replacing round one's file).
  - **Round one's block is closed:** the CLI resolves the record, the engine `tx_compact::compact(ports, &Pane)` owns health, idle, the act and the shrink rule; all eight round-one warns are taken.
  - **New warns:** `switch::emit_outcome` is switch-typed and cannot be reused (use `output::emit` as `close.rs` does); `call_until` has no request parameter (struct-update `Call` in `compact.rs`); the adapter's second activity read is deliberate; `HarnessOp` lists may need the new variant.
- **Assumed:** the brief's remaining line citations, which I spot-checked (`resolve_pane_record` 384, `check_idle`/`check_health`, `mod plan;` private at 86, `Call`/`call_until`/`Timeouts::default()` at `wiring.rs:229`, `PaneCmd`). No build or test run, per the role.
- **Evidence:** read the amended brief in full; `say_cmd.rs`, `interrupt_cmd.rs`, `tx_switch.rs`, `plan.rs`, `switch.rs`, `park.rs`, `pane/mod.rs`, `args.rs`, the OpenCode `lib.rs` and `prompt.rs`, `ports.rs` (`Ports`), file sizes.
