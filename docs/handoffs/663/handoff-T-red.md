# Handoff-T-red: Phase 4 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (re-entry RED)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `e46b427` plus the staged RED change below)
**Brief / wireframe reviewed:** `docs/handoffs/663-brief.md` at `9762a97` (its Test plan, "Re-entry RED (this run)"); wireframe N/A (no UI surface)

This replaces the previous run's RED handoff (from scratch, at `8fe683b`; still in git history). The branch already holds
F's implementation, green on every AC of the previous plan (T-green at `93fb653`). This amendment changes only AC 5 (the
`Option` signature, the `doctor_command` equality, the greps), AC 10's size rule and AC 14's ADR wording.

## A precondition

Confirmed: A returned PASS on the amended plan (`docs/handoffs/663/handoff-A.md`, fourth pass, commit `e46b427`), with warns
W-16 to W-18. None changes what a test asserts. W-17's optional rustdoc sentence is F's.

## What T landed besides tests (the brief's Test plan)

A compile error is not RED, so T landed Decision 8's signature as a stub in `crates/holler-cli/src/pane/profile_scope.rs`:

- `pub fn reconcile_step(profile: Option<&ProfileName>) -> String`, body `String::new()`. It maps `profile` through
  `single_quoted` into `let _` only so `dead_code = "deny"` does not fail the build; F replaces the body and its rustdoc.
- `pub const RECONCILE_STEP_UNSCOPED` is deleted.
- The scope's two call sites (`restore`, `may_have_landed`) pass `Some(..)`.

Nothing else in production code changed. `probe.rs` is untouched.

## Tests authored

One test changed; every other test is unchanged from the previous run and stays as a regression guard.

| Test | Pins | Tier |
|---|---|---|
| `reconcile_step_single_quotes_the_profile_name` (amended) | AC 5 / Decision 8:<br>- `reconcile_step(Some(Demo Alpha))` equals `to reconcile, run holler pane doctor --profile 'Demo Alpha' and then holler profile show 'Demo Alpha'` exactly;<br>- `It's $(id) Demo` gives `--profile 'It'\''s $(id) Demo'`, with no `\n`;<br>- `reconcile_step(None)` equals `to reconcile, run holler pane doctor` exactly (#644's text);<br>- `reconcile_step(None)` equals `format!("to reconcile, run {}", holler_pane::findings::doctor_command(None, false))`, the regression guard that ties the unscoped form to #701's builder. | Unit, a pure function |

The `RECONCILE_STEP_UNSCOPED` assertion is gone with the const. AC 5's three greps and AC 14's ADR checks are source and
document checks, not tests. T-green runs them; their RED values are below.

## RED confirmation

`cargo test -p holler-cli --lib pane::profile_scope` gives `3 passed; 4 failed`. Each failure is a feature assertion on
the reconcile step:

- `reconcile_step_single_quotes_the_profile_name`: `assertion left == right failed`, `left: ""`,
  `right: "to reconcile, run holler pane doctor --profile 'Demo Alpha' and then holler profile show 'Demo Alpha'"`.
- `first_write_timeout_says_the_edit_may_have_landed`: `"holler pane doctor --profile 'Demo Alpha'" missing from "timed
  out: profile_store.cas_put; the write may have landed, so profile \"Demo Alpha\" may hold the edit of demo-c1r1, and
  nothing live was changed; "`.
- `restore_conflict_names_the_act_error_and_the_reconcile_step`: `"holler pane doctor --profile 'Demo Alpha'" missing from
  "profile conflict: \"Demo Alpha\" was changed by another writer during the live change to demo-c1r1, ... the other
  writer's version stays; "`.
- `restore_failure_keeps_its_code_and_names_the_unrestored_edit`: `"holler pane doctor --profile 'Demo Alpha'" missing
  from "timed out: profile_store.cas_put; profile \"Demo Alpha\" may still hold the edit of demo-c1r1, but the live change
  or its record failed (unavailable: act); "`.

All four messages end in `; ` followed by the stub's empty step, which is exactly the missing behaviour. Every other part of
those messages (the code, `Demo Alpha`, `demo-c1r1`, `unavailable: act`, `may hold the edit`) is already right.

Green, by design (regression guards):

- `store_scope_passes_the_profile_scope_conformance_suite` (AC 1). The suite checks codes and names, not the step's text.
- `set_of_a_spec_for_another_pane_is_usage_before_any_write` (AC 6) and `pane_store_fault_fails_a_remove_before_the_profile_write`
  (AC 7). Neither path carries the step.
- `cargo test -p holler-pane --lib probe::tests`: `12 passed` (AC 8a-8l; Decisions 13-15 and 19 now state what the code
  already does).
- `cargo test -p holler-pane --test ports_test run_probe_stub_never_reports_success`: `1 passed` (AC 8m).

The source and document checks on the RED state:

| Check | Expected after F | On the RED stub |
|---|---|---|
| AC 5, production lines with `holler pane doctor` | `0` | `0` (the stub already dropped the const and the literal) |
| AC 5, production lines with `doctor_command(None, false)` | at least `1` | `0`: **RED** |
| AC 5, `RECONCILE_STEP_UNSCOPED` in `profile_scope.rs` | `0` | `0` (T deleted it) |
| AC 14, `grep -cE 'RECONCILE_STEP_UNSCOPED\|takes one \(#647\)\|for now the' docs/adr/ADR-0021.md` | `0` | `3`: **RED** |

**Validity checks:**

- **No compile, setup or harness failure.** Both crates build, and every failure is a feature assertion.
- **Clean.** `cargo clippy -p holler-pane -p holler-cli --all-targets -- -D warnings` exits 0. `bash scripts/lint.sh` exits
  0, with no `warn:` for either file. `rustfmt --check --edition 2021` passes on both files. `profile_scope.rs` is 592 lines.
- **Nothing left behind.** No `hlr-probe-663-*` directory under `$TMPDIR`, and no `sleep 30` process.
- **The RED is for the right reason.** The four failures are exactly the ones the brief's Test plan predicts. The
  `doctor_command` equality is not vacuous: `doctor_command(None, false)` is `holler pane doctor` (`findings.rs:306-316`),
  so a correct `reconcile_step(None)` meets both of AC 5's unscoped equalities, and the `""` stub meets neither.

## Notes for F

- Build `reconcile_step` on `holler_pane::findings::doctor_command(None, false)` for both arms (Decision 8). The production
  code must spell `holler pane doctor` nowhere (AC 5's first grep, with comments excluded).
- Replace the stub's rustdoc and its `let _` line.
- Amend the ADR-0021 sentences a-e in place, and add f (AC 14). ADR line 323 still names the deleted const.
- `profile_scope.rs` is 592 lines. The 600-line `warn:` is accepted (AC 10) and must stay under 900.

## Ready for F

Confirmed: the RED is valid. F may implement against these tests.
