# Survey — issue #646 (parts 2 and 3): `pane close`, and `say`/`interrupt`/`answer` routed by pane

Run: `0646-implementation` · branch `issue-0646-implementation` · rigor: in-session · uiSurface: false.
Surveyed from `origin/main` at 3d95aec in `<run-worktree>`.

## What already merged (scope split)

#646 is in flight as three parts. **Part 1 (`pane park` / `pane unpark`) merged as #711**
(13edbb4): `park.rs` holds the full hold-change engine (`HoldTarget`, `HoldChange`,
`run_hold_change`), `unpark.rs` calls it, with ADR-0003/0021 rows, `cli-surface.txt` lines and
`tests/pane_verbs/park*/unpark.rs`. Its module doc says: *"Nothing in Holler reads the hold yet.
Whether a parked pane refuses a prompt is decided at `send_prompt` by a later part of #646."*

**This run owns the rest: part 2 (`pane close`) and part 3 (routed prompt verbs + the
`send_prompt` pane-state gate).** Still stubs on main: `close.rs` (#670's refusal), and the
`--pane`/`--profile` routing of `say`/`interrupt`/`answer` (`prompt_target.rs::route` refuses with
`not implemented (story #646)`, exit 1, before any hub contact).

## The contract this story builds against (already on main)

- `holler-pane` (frozen by #637): `Pane` with `session_of_record: Option<String>`,
  `harness.health: Health` (`Healthy|Unhealthy(reason)|Unknown`), `hold: Hold`
  (`None|Parked{reason,release_when,since}|Drained`), `last_observed {shown, driven, at}`;
  `PaneStore::delete(name, expected_generation)` (ruling 9, built by #639); `ProfileScope`
  (`resolve`, `edit_spec` with the I8 transaction and ADR-0021 decision-1 restore-on-failed-act);
  open refusal codes via `PaneError::Refused` + `RefusalCode::from_static` (ruling 3);
  `class_of` → exit 2/3/1.
- Hub: `panes::PaneState` (the hub's pane store, `Arc`, in-memory table behind a std mutex, read
  paths cheap and synchronous); `Registry::with_holds` (live.rs) — the precedent the issue names
  for carrying a pane-state handle; `HoldGate` + `send_prompt` (circuit/dispatch.rs) — the hub's
  one prompt enforcement point, with in-file `hold_tests` as the gate-test pattern.
- CLI: `say_cmd.rs`/`interrupt_cmd.rs`/`answer_cmd.rs` all resolve their tail through
  `prompt_target::route(...)` at the same seam (line 39 in interrupt/answer); the say flow is
  `transport::call(ControlCall::say_with(...))` → control server → `talk::say` →
  `LiveCommand::Say` → `HoldGate` → `send_prompt`. `talk::say` also has a fast-path
  `holds.would_refuse` check (precedent: fast path advisory, `send_prompt` authoritative).
  `main.rs::run_with_stdio` builds `VerbCtx` from `pane::wiring::Wiring::connect()` — today
  `Unwired` (#649 builds the real wiring), so every pane verb's runtime port set answers
  `not-implemented` until #649; tests inject `Ports` from the test kit.
- Test kit (`holler-pane-testkit`): pane-store / profile-store / profile-scope / herdr / host
  fakes with call logs, `envelope::check_envelope`, and part 1's `park/rig.rs` rig pattern
  (`run_verb_with`, `assert_failure`, seeded records) to copy for `close`.
- Wire side: `holler_proto::Code` is a **closed 17-variant table**; pane-domain refusals over the
  control socket travel as envelope data (`pane/*` replies), not JSON-RPC errors — but
  `send_prompt` refusals are `WireError`s on the say path. Relevant constraint for the gate
  (open point 2 in the brief).

## Reuse & Analogous-Feature map

| Need | Closest existing thing | Recommendation |
|---|---|---|
| `close` verb structure, output, failure+reconcile message | `pane/switch.rs` + `holler_pane::tx_switch` (`SwitchFailure::message` ends with the reconcile step) — but `tx_*` engines are frozen to their stories; **extend `close.rs` itself** (one verb one file, ruling 2) with a private engine, mirroring how `park.rs` grew its own engine rather than a `tx_park` | **Extend/new-in-place**: engine lives in `close.rs`; no `holler-pane` additions |
| Profile-scoped transaction for `close --profile` | `ProfileScope::edit_spec(profile, pane, SpecEdit::Remove, act)` (#663, real impl in `pane/profile_scope.rs`; `FakeProfileScope` in the kit) | **Reuse** `edit_spec` verbatim; `act` = the live close |
| Stop owned processes + close Herdr pane | `HostPort::stop_owned`, `HerdrPort::close` (both built, fakes log calls — park's rig proves the "nothing live" direction; close proves the "everything stopped" direction) | **Reuse** the ports; order per I3 plan→act→observe→record |
| Record removal | `PaneStore::delete(name, generation)` (ruling 9; #646's close is its declared consumer) | **Reuse** |
| Routing engine shared by three cmd files | `park.rs`/`unpark.rs` share one engine (`super::park`) because `pane/mod.rs` is frozen; the three prompt-cmd files are not under a frozen mod (they are top-level modules in `cli.rs`'s tree) but `prompt_target.rs` is #670-frozen | **Extend**: put the pane→session routing engine in `say_cmd.rs`, called by `interrupt_cmd.rs`/`answer_cmd.rs` (same precedent, same story) |
| Hub-side gate | `Registry::with_holds` + `HoldGate` + in-`send_prompt` check + `hold_tests` in dispatch.rs | **Extend exactly this pattern** with a pane-state handle |
| CLI-side record read for routing | `Wiring::connect().ports().pane_store.get(...)` — the same ports every pane verb uses; `Unwired` at runtime until #649 (fails loudly, accepted) | **Reuse**; tests inject kit fakes |
| Tests | part 1's `pane_verbs/park.rs` + `park/rig.rs` + `park/failures.rs`; `process/stub.rs` row removal; `target_flags.rs` refusal cases (must be updated: the refusal they assert is the one this story replaces) | **Copy the rig pattern** into `pane_verbs/close.rs` (+ `close/` if needed) |

## Blast-radius reading (recorded for A)

The issue adds `holler-hub/src/live.rs` and `circuit/dispatch.rs` "only for the pane-state gate at
`send_prompt`". Two mechanical one-line touches sit outside those filenames but inside that gate:
`circuit.rs`'s single `HoldGate { ... }` construction (the gate cannot be checked unless it is
handed to `send_prompt`) and `serve.rs`'s `Registry::new().with_holds(..)` chain gaining
`.with_panes(..)` (the handle cannot reach the Registry otherwise). The brief records this as the
proposed reading, with the issue's fallback (a CLI-side exception recorded in the brief) if A
refuses it. Doc edits follow part 1's precedent: ADR-0003 rows, `cli-surface.txt` lines,
ADR-0021 "as built" sections + code-table row, CHANGELOG.

## Facts to carry into the brief

- ADR-0021 code table row for this story: `say/interrupt/answer --pane` → closed codes
  `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; **open (#646)** for an unhealthy
  pane and for SHOWN differing from DRIVEN. Parked-pane prompt refusal is part 3's gate decision.
- ADR-0021 §"Park and unpark as built": park/unpark never touch adapters; the prompt gate is
  explicitly left to this part ("the prompt gate of #646").
- I8 write order (ADR-0021 decision 1): profile CAS first, then act, then record; a failed act
  restores specs by a second write, so "a failed close leaves P unchanged" means the specs — the
  generation may move by two. `edit_spec` already implements this.
- `--spec-only` requires `--profile` (clap `requires` on `SpecOnly`); without `--profile` close
  touches no profile.
- `say --queue`: the hub forwards a queued `session/prompt` immediately (PendingSay doc) — the
  acceptance is that the routed `--queue` form returns without waiting for the turn's reply.
- Exit codes: pane verbs 0/1/2/3 via `class_of`; the prompt verbs keep ADR-0003's own say
  contract (their `--json` is the result document, not the envelope). Open point 3 in the brief.
