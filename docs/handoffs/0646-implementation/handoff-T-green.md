# Handoff-T-green: Phase 6 — the suite is GREEN after four test-side repairs

**Date:** 2026-10-10
**Branch:** `issue-0646-implementation` (`<run-worktree>`)
**Issue:** #646 (parts 2 and 3)
**Handoff-F reviewed:** `handoff-F.md` (rework 2)
**Handoff-T-red:** `handoff-T-red.md`

## What T changed and why (all in `crates/*/tests/**`; F's patch verified, applied by hand)

F was right on all four defects — each was a fault in T's red suite, not in the
implementation. T applied the equivalent of F's patch hunk-by-hunk (T read every hunk
and owns the result):

1. **Defect 3 — the contract ruling (F's one signature change): ACCEPTED.**
   `pane_hold_refusal(session: &str, e: &WireError, json: bool)` stands, and the four
   call sites (`target_flags.rs`, was 575/592/614/618) now pass `"ses-demo-c1r1"`.
   Reasoning as contract owner: T's own T-red decision 3 pins the pane-arm JSON as
   `{"error":"session_held","session":"<s>","reason":"<code>","hold_kind":"pane"}` — the
   session is IN the object T demanded. `WireError`'s `ErrorData` carries no session
   (only `hold_kind`, `reason`, `since`), so no 2-argument renderer can ever produce the
   pinned JSON; rejecting F's shape would force T to weaken the pinned contract (drop
   `session`) instead. The 3-arg shape also mirrors `hold_cmd::held_refusal(session, e,
   json)` — the held form binding 2 explicitly says to mirror. The JSON assertion itself
   is unchanged (key order and all four members still asserted).
2. **Defect 1 — `target_flags.rs:471`:** `assert_eq!(error.code(), *code, …)` — `code`
   is a `&&str` (bound by iterating a `&[(…)]`), `code()` returns `&str`.
3. **Defect 2 — `target_flags.rs:493/502/511`:** `.unwrap()` → `.unwrap_err()` — all
   three cases expect errors (`usage`, `pane-not-in-profile`, `session-not-found`); the
   `.unwrap()` form both failed to compile (`String` has no `code()`) and contradicted
   each case's own code assertion one line below.
4. **Defect 4 — `close.rs:232` (`close_spec_only_removes_the_spec_and_touches_nothing_live`):**
   dropped `assert_untouched(&run.calls)` (zero writes anywhere), replaced with an
   explicit **no-pane-store-write** assert; dropped the now-unused `assert_untouched`
   import (the `Both::assert_untouched` method used elsewhere needs no import). The old
   assert contradicted the test's own envelope pin two lines above (`"generation": 2` —
   the spec removal IS one profile `CasPut`; the profile log is `[Get, Get, CasPut]`, and
   `ProfileStoreOp` has no other write op), so no production code could ever satisfy
   both. The "nothing live" pin is intact and now unambiguous: exact-empty `herdr`,
   `host`, `harness` and `probes` asserts (unchanged, lines above), the record-stays
   assert, plus the new zero-pane-store-write assert. A comment states the one profile
   `CasPut` is the removal itself.
5. **Clippy in T's file:** the two `option_map_unit_fn` lints in
   `every_other_held_error_falls_through_to_the_generic_arm` fixed per F's rewrite
   (`Option::map` for side effects → `if let Some(data) = …`). CI's
   `--all-targets -D warnings` gate fails on these otherwise.
6. **Fixture:** the four `# #646` lines F drafted added to
   `tests/fixtures/cli-surface.txt`, grouped verb-wise (`pane close | demo-c1r1`,
   `pane close | demo-c1r1 --profile demo --format=json`,
   `pane close | demo-c1r1 --profile demo --spec-only`,
   `say | --pane demo-c1r1 --profile demo hello --queue`). `cli_surface_test` 3/3.

## GREEN confirmation

The plugin's command, run by T in the worktree (private seeded target dir):

```
cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
→ exit 0 · 140 test binaries · 1779 passed · 0 failed
```

Per-binary citations: `pane_verbs` **234 passed** (the 21 T-red routing/queue/arm/close
process cases included), `pane_cli_process` **36 passed**, `holler-hub --lib`
**83 passed** (the 6 gate tests + 2 pre-existing hold tests), `cli_surface_test`
**3 passed**. Every number matches F's scratch-copy self-check exactly.

**Pins behavior, not implementation:** the RED→GREEN delta is F's implementation plus
T's four repairs above, and each repair is either compile-shape (1, 2, 3) or the
correction of a self-contradiction (4) — no assertion was weakened except removing the
impossible half of a contradictory pair, and the JSON/key-order/wire-shape assertions
are untouched. Mutation testing is a non-goal this run; the process-level cases run the
real binary path.

## Tier 1 results

| Check | Command | Expected | Actual | |
|---|---|---|---|---|
| Build/compile | `cargo test -p holler-cli --test pane_verbs --no-run` | compiles | compiles clean | PASS |
| Full suite | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0, 1779/0/0 | PASS |
| Clippy (workspace) | `cargo clippy --workspace --all-targets` | only the `unreachable!` error below | exactly 1 error: `clippy::unreachable` at `dispatch.rs:650` (deny-by-default) | PASS\* |
| Clippy (T's crate) | `cargo clippy -p holler-cli --all-targets` | clean | exit 0, clean | PASS |
| Repo lint script | `scripts/lint.sh` | exit 0 | exit 0 (file-size advisories only; every touched file under 900) | PASS |
| rustfmt | `rustfmt --check --edition 2021` on `target_flags.rs`, `close.rs` | clean | clean (fixture/handoffs are not Rust) | PASS |

\* the single workspace clippy error is the one item left for O below; it is in `src/**`
(T's applied gate block), outside T's write scope. With O's one-line fix applied — the
same tree F verified in a scratch copy — the identical command exits 0 under
`-D warnings`.

## Tier 2 results

- **Coverage per AC**: every brief AC is backed by a passing T-authored test (table
  below). No AC is test-free.
- **Test quality**: the four defects were quality faults in T's own suite and are now
  repaired at the cheapest sufficient tier (unit for pure functions/engine, in-process
  harness for verb behavior, process binary for the CLI surface, hub unit for the
  gate); no test duplicates another's pin; the suite is unchanged in size (no additions
  needed at green).
- **Error paths**: refusal codes exercised table-driven across all seven codes; usage
  (exit 2) vs refusal (exit 3) vs failure (exit 1) all asserted; the reconcile-step
  discipline (present only after the first live call, never doubled) pinned both ways.
- **Data integrity**: generation semantics pinned (bump-once on success, move-by-two +
  restore on failure, no silent bump on the pre-check refusal); CAS conflict paths
  tested (`a_record_conflict_after_the_act_fails_loudly`); store failure tested
  (`a_store_failure_fails_the_run`).
- **API/wire contract**: the gate's wire shape asserted field-by-field (`-32011`,
  `hold_kind: "pane"`, `data.reason` = the code, no `data.since`, one line, nothing
  sent); JSON envelope parity held by `run_both`/`check_envelope` on every close case.
- **Security/non-goals**: no live fleet, pane, Herdr session or credential touched —
  every run is over `holler-pane-testkit` fakes with synthetic `demo-*`/`ses-demo-*`
  names; no new `unsafe`, no new dependency, no migrations (n/a).
- **Playwright/e2e**: none in this repo surface (uiSurface false); nothing to skip.

## Acceptance criteria status

| AC | Backing test(s) | |
|---|---|---|
| Park/unpark round-trip (part 1, #711) | presence-only for S (not re-tested, per brief) | n/a |
| `close` leaves no owned process / closes Herdr / deletes record | `close_stops_the_processes_closes_the_herdr_pane_and_deletes_the_record` (exact `[StopOwned]`/`[Close]`/`[Delete]` call sets, `assert_gone`) | PASS |
| `say --pane` SHOWN≠DRIVEN refuses exit 3 | `the_engine_refuses_every_routed_case_with_exit_3`; hub `unhealthy_and_shown_driven_mismatched_panes_refuse_the_same_way` | PASS |
| unhealthy / parked refusals, CLI-side and hub gate | same table (CLI); hub `a_prompt_to_a_parked_pane_s_session_of_record_is_refused_in_every_variant` + the unhealthy/mismatch test | PASS |
| routed `say --queue` returns without waiting | `a_deadline_expiry_after_acceptance_is_queued_exit_0`, `a_reply_inside_the_wait_prints_as_the_bare_form_does` (pure decision fn) | PASS |
| close `--profile`/`--spec-only`/no-profile generation semantics + `pane-not-in-profile` pre-check | `close_with_a_profile_removes_…`, `a_failed_live_close_restores_the_profile_specs`, `close_spec_only_removes_the_spec_and_touches_nothing_live`, `close_spec_only_removes_a_detached_spec_…`, `close_refuses_a_spec_less_pane_before_any_profile_write`, `close_without_a_profile_never_touches_a_profile` | PASS |
| `--pane X` outside P refuses `pane-not-in-profile` | engine table row + `close_refuses_a_spec_less_pane_…` | PASS |
| JSON envelope helper, exits equal across formats | `run_both` on every close case; `close_json_is_one_envelope_and_the_exits_agree` | PASS |
| hub gate at `send_prompt` (healthy goes through; wire shape) | `a_healthy_matching_pane_and_an_unmatched_session_go_through`, `assert_pane_refusal` shape, `any_bad_pane_of_the_same_session_refuses_fail_closed`, `an_operator_hold_still_refuses_first_with_its_own_kind` | PASS |
| ADR rows, cli-surface lines, CHANGELOG, stub.rs per ruling 9 | ADRs landed by F; fixture lines added by T (this handoff); CHANGELOG + `cli.rs`/`prompt_target.rs` staleness left for O (below) | PASS w/ O items |
| fmt/clippy/no unsafe/fakes-only | Tier 1 table; clippy has the one O item | PASS w/ O item |

## Blocking issues

None that block U/S. **U is N/A (uiSurface false) — ready for S.**

Two items are left for O because T's write scope denies both files:

1. **`crates/holler-hub/src/circuit/dispatch.rs` — one line.** T's gate block's
   `let … else { unreachable!() }` (line 650) trips the workspace-denied
   `clippy::unreachable` under `--all-targets`. Apply F's fix — add the lint to the
   module's existing allow (exact edit):

   ```diff
    #[cfg(test)]
   -#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #442
   +#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #442, #646
    mod hold_tests {
   ```

   (current file: lines 456-458; the `// #442` comment keeps `scripts/lint.sh` check 1
   passing — verified, the script exits 0 today). Alternative, equally one line: rewrite
   650's else arm to `panic!("refused, checked above")` (already allowed by the module
   allow). Until one of these lands, CI's clippy gate fails on exactly this one error;
   the test suite is unaffected. F verified the allow form exits 0 under
   `-D warnings` in the scratch copy.
2. **`CHANGELOG.md`** — F's drafted entry (handoff-F.md, "For O") was denied by scope
   for both F and T; O applies it as drafted.

## Advisory notes (non-blocking)

- The `--queue` fixture line pins the *routed* queue form; the bare
  `say --queue SESSION` line was already present (line 52) — both contracts covered.
- F's flagged staleness (`cli.rs` `--pane` help strings, `prompt_target.rs` module doc)
   is in files frozen to earlier stories; correctly left. Follow-up story material, and
   S should hold the ADR text as the truth.
- F's seven implementation decisions (handoff-F.md, "Decisions S should audit") all
   stay within T's pinned contract or in the deliberately-unpinned space T recorded at
   red (call-set pins, already-gone Herdr pane, parked close). Nothing new needs a T
   ruling.

**Verdict:** PASS
