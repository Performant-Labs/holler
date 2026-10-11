# Handoff-T-red: Phase 5 (Workflow script; the role doc's Phase 4), round 2 - #802 the health wait checks the run's deadline before every ask

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, head `d634671`)
**Brief / wireframe reviewed:** `docs/handoffs/802-brief.md` as amended in `0dc4e8b`, `docs/handoffs/802/handoff-A.md` (round 2, W-4 restated and binding, W-14 to W-16), `docs/handoffs/802/handoff-T-green.md` (B-1). No wireframe (no UI).

Round 1 of this handoff (the suite for AC 1-12, RED on `0421ec1`) is at
`git show 26148ba:docs/handoffs/802/handoff-T-red.md`. Those tests are unchanged and GREEN on F's
code. This round adds only what the amended Scope 2 and A's restated W-4 ask for.

**VERDICT: RED is valid.** Three tests, all failing on an assertion about the missing behaviour
(an extra `health` call), none on a compile, setup or timing error.

## A precondition

Confirmed: A returned PASS on the amended brief (round 2, `d634671`), no blocks. W-4 is restated
and binding: the one shared `wait_healthy` checks the deadline before every `health` call,
including the first after `run`, and the rule lives there, not in a second `budget.check()` in
`server::start` (Phase 8 rejects that).

## Tests authored

All in `crates/holler-cli/tests/pane_verbs/launch/serve.rs` (541 lines). No `src/`, no
`Cargo.toml`. `pane_verbs` is already a declared `[[test]]`.

| Test | Pins | Tier | Why this tier |
|---|---|---|---|
| `no_health_poll_starts_once_the_budget_has_run_out` (added by T-green, `dab3cd3`; kept as is) | AC 6 + Scope 2 at the engine: a server `run` that returns past the budget is never asked; the run is `timeout` (`pane.launch`), `acted: true`, rolled back, harness calls exactly `[ServeCommand, Health, Health]` | integration (engine over the rig's fakes) | ADR-0021 section 12's bound is a property of the run, so one test must observe the run |
| `the_health_wait_asks_nothing_once_its_deadline_has_passed` (new) | W-4 "the first ask": `wait_healthy` with a deadline already passed answers `Ok(false)` and makes **no** `health` call | direct call of the shared `pub fn` with a bare `FakeHarness` | Cheapest tier that pins the rule *in the shared function*. The engine test above would also pass with a second `budget.check()` in `start`, which W-4 rules out; this one does not. It also covers the wait's two other callers (conformance suite, real rig). |
| `a_deadline_during_the_health_waits_sleep_ends_it_without_another_ask` (new) | W-4 "the last ask" / Scope 2 "each `health` call ... with the budget checked before it": a deadline that comes during the sleep ends the wait with no further ask | same as above | Same. Deterministic by construction: the poll (60 s) is far longer than the window (200 ms), so the wait's single sleep always ends at or past the deadline. The outcome is a call count fixed by ordering, not a duration. |

Not added, on purpose:
- **W-14** (`Budget::check` vs `Budget::deadline` for a budget the clock cannot hold): A lets F
  choose between two code fixes or a doc-only statement. A test would pick for F, and no caller
  passes such a budget. Left to F and S.
- **W-16** (rig imports `BOOT_POLL`) and the CHANGELOG clause (AC 12 advisory): structural and
  doc changes, checked by A at Phase 8 and by S, not by a behaviour test.

## RED confirmation

```text
$ cargo test -p holler-cli --test pane_verbs -- launch::serve

---- launch::serve::the_health_wait_asks_nothing_once_its_deadline_has_passed stdout ----
assertion `left == right` failed: no health call once the deadline has passed
  left: [Health]
 right: []

---- launch::serve::a_deadline_during_the_health_waits_sleep_ends_it_without_another_ask stdout ----
assertion `left == right` failed: one ask before the deadline, none once it has passed
  left: [Health, Health]
 right: [Health]

---- launch::serve::no_health_poll_starts_once_the_budget_has_run_out stdout ----
assertion `left == right` failed: no health call after the run: the budget had run out
  left: [ServeCommand, Health, Health, Health]
 right: [ServeCommand, Health, Health]

test result: FAILED. 12 passed; 3 failed; 0 ignored; 0 measured; 246 filtered out; finished in 1.20s
```

- The two new tests were run 5 more times: RED every time (`0 passed; 2 failed`, 0.20 s).
- **Satisfiable:** with a trial deadline check at the top of `wait_healthy`'s loop (applied,
  then reverted with `git checkout -- crates/holler-pane/src/tx_launch/server.rs`), `pane_verbs`
  was 261/261 and every `holler-pane-testkit` target passed (conformance included). F owns the
  final form.
- `rustfmt --edition 2021 --check` on `serve.rs`: clean. `cargo clippy -p holler-cli --tests -- -D warnings`: clean.

## Ready for F

RED is valid. F may implement W-4 against these three tests: the deadline check inside the shared
`wait_healthy`, before every `health` call, and the four docs W-4 names (`wait_healthy`'s doc,
`Budget::deadline`'s doc, step 3 of the module doc, ADR-0021's health-wait paragraph). W-14, W-16
and the CHANGELOG clause go in the same pass, per A.
