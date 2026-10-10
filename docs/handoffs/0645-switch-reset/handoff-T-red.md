# Handoff-T-red: Phase 4 - #645 part 2, `--first` on reset and the activity refusals

**Date:** 2026-10-10
**Branch:** `issue-0645-switch-reset` (at `3d95aec` = origin/main tip)
**Brief reviewed:** `docs/handoffs/0645-switch-reset/brief.md` · **Survey:** `survey.md` ·
**Decisions:** `decisions.md` (the carve-out) · **A:** `handoff-A.md` (PASS, N-1..N-5 carried)

**VERDICT: RED — compile-level and honest (A's N-4), on the whole `pane_verbs` binary.**

## A precondition

Confirmed: A returned PASS on the plan (W-1 applied by O before F; N-1..N-5 carried into this
suite — see the case list).

## Files touched (T boundary only)

- `crates/holler-cli/tests/pane_verbs/switch.rs` — the shared seam doubles + 2 cases + the
  `:648` pin flip (N-4).
- `crates/holler-cli/tests/pane_verbs/reset.rs` — the `reset_over_seam` helper + 5 cases.

No production file, no `Cargo.toml`, no testkit file, no `tx_switch.rs` (see "Engine-side
decision" below). Nothing outside the two boundary test files.

## Tests authored (9 items mapped to the brief's checkboxes)

| Case (file) | Brief AC | Behavior pinned | Tier |
|---|---|---|---|
| `reset_with_first_queues_the_message_to_the_new_session_only` (reset.rs) | AC 1, AC 2 | The incident: with `Q` and the old session present (shared data dir), `--first` queues exactly ONE prompt — to the NEW session on P's port, never the old session, never Q's session/port — **after the record write** (each queued prompt snapshots the pane store's `session_of_record` at prompt time: it must already name the prompted session), with part 1's exact fake-log op sequence `[Health, CreateSession, SelectSession, ShownSession]`; zero Herdr/host calls (I4) via the runner. | integration (verb rig) |
| `reset_without_first_queues_no_prompt` (reset.rs) | AC 1 | `--first` absent → **no `send_prompt` call** (the wrapper would have recorded one) and the run is part 1 exactly: same op sequence, `[Get, CasPut]` pane-store writes, record per `assert_recorded`. | integration |
| `reset_first_prompt_failure_after_the_record_is_honest` (reset.rs) | AC 3 (N-2) | A `send_prompt` fault (`Unavailable`) after a successful record: exit 1 `unavailable` both formats, the message contains the brief's own phrase **"first message did not land"**, appends **no** reconcile step and **no** "is not recorded" clause, the prompt WAS attempted, and the record/TUI stand exactly as a successful reset left them (`assert_recorded`). | integration |
| `reset_refuses_a_busy_or_questioning_conversation` (reset.rs) | AC 4 | Table-driven (the `reset_refusals_create_nothing` shape): `Activity::Busy` → exit 3 `session-busy`, `Activity::HoldingQuestion` → exit 3 `session-holds-question` (the stable kebab pair this contract names); plan-phase — no `CreateSession`, no `CasPut`, nothing changes, exactly ONE `session_activity` query on the pane's CURRENT session of record, message names the pane. | integration |
| `reset_skips_the_activity_gate_without_a_session_of_record` (reset.rs) | AC 4 (N-5) | No session of record + a wrapper answering **Busy for ANY query**: the run SUCCEEDS and the activity log is EMPTY — the skip is by `None`, not by the idle default (the non-vacuous proof A demanded). | integration |
| `switch_is_unaffected_by_the_activity_seam` (switch.rs) | AC 4 | A switch through the same busy-for-any wrapper succeeds, records its target, and neither queries activity nor queues a prompt (the gate is `Target::Fresh` only). | integration |
| `seam_defaults_are_permissive` (switch.rs) | AC 5 | Through the kit's `FakeHarness` (no seam override, as until #684): `session_activity` answers `Ok(Activity::Idle)`; `send_prompt` refuses with the stable code **`prompt-unsupported`**. No existing impl changes behavior. | integration (port-level) |
| `help_names_the_arguments` flip (switch.rs, was :648) | AC 1 surface (N-4) | `reset --help` now MUST name `--first` (part 1's negative pin flipped; the assertion is part of the RED). | unit (parse) |
| Runner inheritance | AC 6 | Every new case rides `both_with`/`one_case`: envelope-valid JSON (`check_envelope`), one text line, exit parity across formats, and the no-Herdr/no-host I4 assertion come free. | — |

Engine-side unit test beside `tx_switch.rs`: **deliberately not added** — see Decided below.

## RED confirmation (the contract F implements against)

Baseline before my edits: `cargo test -p holler-cli --test pane_verbs` at `3d95aec` →
**219 passed, 0 failed** (green). After my edits the same command **fails to compile** — the
wrapper doubles must override the two seam methods the carve-out has F add, so rustc itself
carries the RED (N-4's design; the whole binary, all 219 existing tests included, reads RED
with it). The complete error set — six errors, every one naming the not-yet-existing seam,
nothing else:

```text
error[E0432]: unresolved import `holler_pane::ports::Activity`
 --> crates/holler-cli/tests/pane_verbs/reset.rs:8:5
error[E0432]: unresolved import `holler_pane::ports::Activity`
 --> crates/holler-cli/tests/pane_verbs/switch.rs:16:5
error[E0407]: method `send_prompt` is not a member of trait `HarnessPort`
 --> crates/holler-cli/tests/pane_verbs/switch.rs:767:5
error[E0407]: method `session_activity` is not a member of trait `HarnessPort`
 --> crates/holler-cli/tests/pane_verbs/switch.rs:779:5
error[E0599]: no method named `session_activity` found for struct `FakeHarness` in the current scope
 --> crates/holler-cli/tests/pane_verbs/switch.rs:795:10
error[E0599]: no method named `send_prompt` found for struct `FakeHarness` in the current scope
 --> crates/holler-cli/tests/pane_verbs/switch.rs:800:10
error: could not compile `holler-cli` (test "pane_verbs") due to 6 previous errors
```

`cargo test -p holler-pane --lib` (the brief's other narrow command): **12 passed, green** —
that crate is untouched. Because the binary is RED at compile level by design, no case can be
"green on contact" — the RED policy's green-on-contact clause cannot fire; once the trait
exists, the `--first` cases additionally fault at clap/runtime until F wires the flag, engine
gate and prompt step.

### Environmental hazard for O and F (journaled in decisions.md)

This worktree's `target` is a symlink to `<primary>`'s `target`, shared with sibling sessions.
Mid-run, a sibling's build of an unmerged branch (where `ProfileSpec` has a field
`opencode_agent` this tree's does not — confirmed absent at `origin/main` == my HEAD) poisoned
the shared cache: my first RED run also showed `E0063 missing field opencode_agent` in
`launch/rig.rs`, a file this run never touched. The RED above was captured with a run-private
`CARGO_TARGET_DIR` under `<cache>/tmp/opencode/0645-target`, which reproduces cleanly.
**F and the stage runs must use the same private target (or force a rebuild of
`holler-pane` from this tree) or they will see phantom errors.**

## What F must create for these cases to compile and pass

All of it inside the brief's F-boundary; signatures and names below ARE the test contract:

1. `crates/holler-pane/src/ports.rs` (the carve-out's whole extent, N-1 — the two methods
   AND their minimal signature types):
   - `pub enum Activity { Idle, Busy, HoldingQuestion }` — closed, minimal derives
     (`Debug, Clone, Copy, PartialEq, Eq`), no Serde. **These variant names are pinned** (the
     wrappers construct them).
   - `fn send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError>`,
     default → `Err(PaneError::Refused { code: PROMPT_UNSUPPORTED, .. })` with
     `pub const PROMPT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("prompt-unsupported")`
     (const name is F's; the code string is pinned).
   - `fn session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError>`,
     default → `Ok(Activity::Idle)`.
   - Tests import `holler_pane::ports::Activity` — there is **no root re-export**
     (`lib.rs` is outside every boundary; do not assume one).
2. `crates/holler-pane/src/tx_switch.rs`:
   - Refusal codes `session-busy` and `session-holds-question` (consts, ruling 3) for
     `Activity::Busy` / `Activity::HoldingQuestion` on the pane's current session of record.
   - The plan-phase activity check after `check_health`/`refuse_orchestrator`, gated
     `Target::Fresh` **and** `session_of_record.is_some()` (verb-scoped, not `--first`-scoped —
     the busy-refusal cases run WITHOUT `--first`).
   - `SwitchRequest.first: Option<String>` (the survey's name).
   - After the record write (`cas_put`), `first`'s text via `harness.send_prompt(port, new)`.
   - **N-2's third `SwitchFailure` decoration** for a prompt failure after a successful
     record: the message states the first message did not land (the brief's phrase
     "first message did not land" is asserted), appends neither the reconcile step nor the
     "was created and is not recorded" clause; the error stays the port's own; exit per
     `class_of`.
3. `crates/holler-cli/src/pane/reset.rs`: `--first TEXT` in `PaneReset` (ruling 2), passed
   through to `execute`.
4. `crates/holler-cli/src/pane/switch.rs`: the pass-through (`Some(text)` for reset, `None`
   for switch).
5. Docs rows (W-1, F's): ADR-0003, ADR-0021's three spots, `cli-surface.txt` — the ADR
   amendment should record the two activity codes and the `prompt-unsupported` departure
   (N-3).

## Tier-2 self-checks (T's own suite quality)

- Every case names one behavior, sits at the cheapest sufficient tier (the existing verb rig
  — no e2e/process tier needed), and none duplicates another: the two ordering pins are
  independent (the prompt-log snapshot vs the fake-log op sequence), and the N-5 case is the
  only non-vacuous no-session proof.
- Wrappers stay `WriterInSelect`-shaped (~40 lines, delegate the eight, override only the
  seam, record-only + a test-set answer table; no session/TUI state of their own — the a-dup
  constraint is documented on the type).
- Assertions are on behavior (codes, records, logs, exits), never on F's internals; message
  pins are the brief's own phrases only.
- `rustfmt --edition 2021 --check` clean on both files (existing content untouched by the
  format pass). Clippy on the touched target is deferred to T-green by design — it cannot run
  against a binary that is RED at compile level.
- No new `unsafe`; no mutation testing; one cargo at a time.

## Boundary compliance statement

Edited exactly `crates/holler-cli/tests/pane_verbs/reset.rs` and `switch.rs`. Not touched:
`ports.rs`, `tx_switch.rs` (see below), `src/pane/reset.rs`/`switch.rs`, the testkit,
adapters, `doctor/**`, `launch/**`, `wiring.rs`, `cli.rs`/`main.rs`, every `Cargo.toml`, ADR
and fixture rows, `main.rs` of the test binary. Nothing committed — staged the two test files
by explicit path only; O commits.

## Engine-side decision (tx_switch.rs unit test: not natural, evidence)

The brief allows engine unit tests beside `tx_switch.rs` "where the fakes allow". They do not:
`holler-pane` has **no dev-dependency on `holler-pane-testkit`** (its `Cargo.toml` lists only
`serde`, `serde_json`, `holler-proto`), so using the kit's fakes would need a manifest edit —
`Never` territory. The alternative, hand-rolling a `MemPaneStore`-plus-harness double set
inside the engine, would duplicate the rig at worse fidelity — the second-`FakeHarness` shape
N-4/a-dup forbid — and part 1 shipped zero unit tests there (all engine behavior is pinned
through the verb rig). The prompt-after-record ordering is therefore pinned through the
wrapper's pane-store snapshot (`then_of_record`), which fails if F queues before `cas_put`.

## Ready for F

RED is valid (compile-level, by design). F may implement against these tests. Sequencing
note: adding the two trait methods alone will make the binary compile — from that point the
remaining RED is runtime (`--first` rejected by clap, busy panes resetting unrefused, no
prompt step), which is exactly F's ladder.
