# Handoff-A-dup: Phase 7 - Anti-duplication gate (#646 parts 2 and 3)

**Date:** 2026-10-10
**Branch:** `issue-0646-implementation` (`<run-worktree>`)
**Diff base:** 3d95aec   **Diff head:** ff32f8d (cbec3bd red suite + ff32f8d implementation)
**Reuse map:** `docs/handoffs/0646-implementation/survey.md` (§ Reuse & Analogous-Feature map)
**Brief:** v2 (amended; A's 11 bindings are the contract)
**Verdict:** PASS

## Summary

F extended the objects the reuse map named and built nothing parallel. `close` keeps its
engine in `close.rs` and delegates the profile transaction to `ProfileScope::edit_spec`
verbatim; the routing engine lives in `say_cmd.rs` and reuses the frozen
`prompt_target` resolve machinery instead of re-implementing it; the hub gate extends
the `with_holds`/`HoldGate`/`send_prompt` precedent via a minimal `with_panes` mirror;
the routed `--queue` wait replaces only `ControlCall.timeout` while `timeout_ms` keeps
`--timeout`. The two A-accepted one-line touches (`circuit.rs`, `serve.rs`) landed
exactly as bound; every other changed file is inside the brief's blast radius or is the
forced test-side sibling rewrite ruling 9 makes unavoidable. No block findings.

## Drift check (against the reviewed plan and A's 11 bindings)

| Binding | What was checked | Result |
|---|---|---|
| 1 (radius) | `git diff 3d95aec..HEAD --name-status`: every changed file accounted for. `circuit.rs` is exactly the one `HoldGate` construction line (`panes: self.registry.panes().cloned()`); `serve.rs` is the `PaneDeps::load`-above-registry reorder plus `.with_panes(Arc::clone(..))` plus one doc-comment line; `HoldGate` drops `Copy` keeps `Clone`; the three pre-existing `hold_tests` constructions gain `panes: None`. Frozen files untouched: `prompt_target.rs`, `cli.rs`, `holler-pane/**`, `holler-proto/**` show zero diff. | PASS |
| 2 (wire carrier) | `pane_gate` answers `Code::SessionHeld` with `with_hold_kind("pane")`, `data.reason` = the pane kebab code, no `data.since`; message names pane and code. `say_cmd::pane_hold_refusal` (`say_cmd.rs:460`) runs before the generic `is_held` arm in both `say` (`control_refusal`) and `interrupt`; exit 3, remedy line, never `HELD_EXIT_CODE` 4; `answer` has no pane arm. JSON is the pinned 4-key object — `hold_cmd::held_refusal` cannot produce it (its shape always carries `since` and the wire's reason), so the local `PaneHeld` serializer is justified, not a parallel renderer. | PASS |
| 3 (exit codes) | `routing_stop` uses `class_of(code).exit_code()` — refusal 3, usage 2, failure 1; open codes are the three `RefusalCode::from_static` consts in `say_cmd.rs`; session-side codes (busy 1, ambiguous 2, held 4, invalid-grant 5) unchanged in `control_refusal`. ADR-0003's say row amended with the exit-3 extension. | PASS |
| 4 (spec pre-check) | `close.rs::plan` pre-checks `spec_of(profile, &name)` and refuses `pane-not-in-profile` in both `--spec-only` and live `--profile` paths, before any write; `edit_spec`'s trait untouched. | PASS |
| 5 (CLI parked check) | The CLI predicate includes `Hold::Parked` (`pane-parked`, const, exit 3) — the record was read; no hub round trip. | PASS |
| 6 (gate matching, twin) | Hub `pane_gate` (`dispatch.rs`): one synchronous `PaneState::list()`, no `await`, bare-session match against `session_of_record`, fail-closed (any bad pane refuses), first bad pane in name order, checked after `holds.admit`. Both twins (`say_cmd.rs:344`, `dispatch.rs:87`) hold exactly the pinned predicate set parked → unhealthy → SHOWN≠DRIVEN (both present, differing) — neither grew — and each doc comment cross-references the other. | PASS |
| 7 (reconcile discipline) | `failure_body` appends `reconcile_step` only when a live step was reached (`reached` is `Some` from `StopOwned` on), guarded by a `contains` check so `edit_spec`'s own carrier is not doubled; plan refusals carry none. | PASS |
| 8 (`--spec-only` no record) | `remove_spec` runs `edit_spec` with an empty act; no record read required. | PASS |
| 9 (stub.rs) | The `("pane", "close", 646)` row and the `PANE_FORM_REFUSAL` const are deleted, the `// #646` comment line kept. | PASS |
| 10 (queue) | `returns_on_acceptance` = routed `--queue` + local control socket only; `call.timeout = QUEUE_ACCEPT_WAIT` (2 s) replaces **only** the client wait — `ControlCall::say_with` puts `--timeout` into `timeout_ms` (control.rs:83-95), so the hub keeps waiting for the turn; `queue_outcome` is a pure function mapping a post-acceptance `Io` deadline to `queued <session>` exit 0; bare `say --queue SESSION` and remote `--server` keep the reply wait. | PASS |

## Duplication check (against the reuse map)

| Map row | What F did | Result |
|---|---|---|
| close engine in `close.rs`, no `holler-pane` additions | Engine is private to `close.rs`; mirrors how `park.rs` grew its own engine; no `tx_close`, no frozen-file edits. | Extended in place |
| `ProfileScope::edit_spec` reused verbatim | Both `--profile` and `--spec-only` go through `ports.scope.edit_spec(.., SpecEdit::Remove, act)`; no hand-rolled CasPut/restore logic anywhere in `close.rs`. | Reused |
| ports reused (`stop_owned`, `HerdrPort::close`, `PaneStore::delete`) | All called through `Ports`; I3 order stop → close → delete inside the act; no `ensure_pane`, no keystroke. | Reused |
| routing engine in `say_cmd.rs`, called by interrupt/answer | `route_target` (`say_cmd.rs:236`) + `resolve_pane_target` (`:301`); `interrupt_cmd.rs`/`answer_cmd.rs` call it. All tail parsing stays in the frozen `prompt_target` (`Say::resolve` etc.); only frozen `route()`'s not-implemented arm is replaced — forced by the #670 freeze, exactly as the map prescribed. `route()` stays `pub` per the brief. | Extended, not duplicated |
| hub gate extends `with_holds`/`HoldGate`/`send_prompt` | `Registry::with_panes`/`panes()` mirror `with_holds`/`holds()` (`live.rs`); gate at `send_prompt` after `holds.admit`; no new registry or handle machinery beyond the mirror; no `talk::say` fast-path copy (correctly absent). | Extended exactly this pattern |
| tests copy the park rig pattern | `close/rig.rs` follows the launch/park rig shape over the kit fakes with a call-log span; `target_flags.rs` reuses `crate::list::Rig` rather than building another; gate tests mirror in-file `hold_tests` (via O-applied `gate-tests-dispatch.md`); `check_envelope`/`run_both` used throughout. | Copied the pattern, no parallel rig |

## Findings

| # | Severity | File:line | Finding |
|---|---|---|---|
| 1 | advisory | `crates/holler-cli/tests/pane_verbs/process/legacy_verbs.rs:1` | This file is not in the brief's named test radius but had to change twice over: ruling 9 deletes `PANE_FORM_REFUSAL`, which it imported, and the story replaces the very refusal its cases asserted. The rewrite is test-side, minimal given those two forces, and keeps the file's process-level role (no stub, usage wins, session forms untouched). Accepted under the forced-sibling precedent; recorded here so the radius reading is visible, not silent. |
| 2 | advisory | `crates/holler-hub/src/circuit/dispatch.rs:48` | `SendPromptError` gained `#[derive(Debug)]` — not named in a binding, but required by the gate tests' failure prints and confined to a radius file. Test-enabling, no behavior change. |
| 3 | advisory | (observation, not a diff finding) | `prompt_target::route` is now unused by production (all three verbs moved off it) and `cli.rs`'s `--pane` help strings still say "refused until #646" — both are the frozen-to-#670 staleness F flagged for O, correctly left unedited this run. Follow-up story material alongside them; the ADR-0021 "as built" text carries the truth meanwhile. |
| 4 | advisory | (worktree state, not a diff finding) | `docs/handoffs/0646-implementation/decisions.md` has uncommitted working-tree edits (the T-green journal entry). O should commit it before the merge phase, which refuses a dirty worktree. |

No `BLOCK` findings. No dead code, commented-out blocks, debug prints (`dbg!`/`println!`/`eprintln!`/`TODO` greps over the added lines are clean), or leftover #646 stubs: the only new occurrence of the old stub string is `legacy_verbs.rs`'s `OLD_STUB` sentinel, which exists to assert the line is gone.

## Notes for F

None — no rework required.

## Patterns referenced

- `crates/holler-cli/src/pane/park.rs` — the one-verb-one-file engine precedent (part 1, #711)
- `crates/holler-cli/src/prompt_target.rs` — the frozen #670 resolve machinery the routing reuses
- `crates/holler-hub/src/live.rs` (`with_holds`/`holds()`) — the builder mirror `with_panes` follows
- `crates/holler-hub/src/circuit/dispatch.rs` `hold_tests` — the gate-test pattern the pane gate tests mirror
- `crates/holler-hub/src/control.rs:83` (`say_with`) — the `timeout_ms` vs `call.timeout` split binding 10 rests on

**Verdict:** PASS
