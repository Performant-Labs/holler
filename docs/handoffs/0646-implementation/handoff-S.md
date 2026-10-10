# Handoff-S: Phase 8 — Spec audit (issue #646, parts 2 and 3)

**Date:** 2026-10-10
**Branch:** `issue-0646-implementation` (`<run-worktree>`)
**Issue:** #646 (parts 2 and 3; part 1 park/unpark merged as #711)
**Handoff-T reviewed:** `docs/handoffs/0646-implementation/handoff-T-green.md`
**Handoff-A reviewed:** `docs/handoffs/0646-implementation/handoff-A.md` (PASS, attempts 2–3) and `handoff-A-dup.md` (PASS)
**Handoff-F reviewed:** `docs/handoffs/0646-implementation/handoff-F.md` (rework 2)
**Diff audited:** `git diff 3d95aec..HEAD` (cbec3bd red suite + ff32f8d implementation), read at source level
**Decision journal:** `docs/handoffs/0646-implementation/decisions.md`

## A precondition

Confirmed. A's up-front review returned PASS (attempt 2 recorded; attempt 3 was the verdict-line
format fix the journal describes) and the a-dup gate returned PASS with four advisory findings,
no blocks. Both handoffs reviewed.

## T precondition

Confirmed. handoff-T-green reports zero blocking issues: the plugin's suite run GREEN
(`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` → exit 0,
140 binaries, 1779 passed / 0 failed), clippy clean workspace-wide after O's one allow-line,
rustfmt clean, `scripts/lint.sh` exit 0. The two T-green "O items" (the `clippy::unreachable`
allow line and the CHANGELOG entries) are both present in the committed diff — O applied and
committed them in ff32f8d, so nothing is left open from T's table.

## Visual-diff-tool precondition

N/A — uiSurface false (no UI surface; D and U declared N/A at pre-flight).

## Spec compliance — every brief AC checkbox

| AC (brief §Acceptance criteria) | Evidence (spot-checked at source by S, not just T's table) | Status |
|---|---|---|
| Park/unpark round-trip (part 1, #711) — presence only | `crates/holler-cli/src/pane/park.rs` and `unpark.rs` exist; not re-tested per brief | PASS |
| `close` leaves no owned process; closes Herdr pane; record gone | `close_stops_the_processes_closes_the_herdr_pane_and_deletes_the_record` (close.rs:69): host log exactly `[StopOwned]`, herdr `[Close]`, pane writes exactly `[Delete]` (no `ensure_pane`, no keystroke), `assert_gone`. Source: `close_live` runs stop → close → delete in order, each gated on the prior | PASS |
| `say --pane` SHOWN≠DRIVEN refuses exit 3 `pane-shown-driven-mismatch` | Engine table test `the_engine_refuses_every_routed_case_with_exit_3` (all 7 codes, class Refusal ⇒ exit 3, code in message); hub `unhealthy_and_shown_driven_mismatched_panes_refuse_the_same_way`. Source: `pane_state_refusal` requires both sides present and differing | PASS |
| Unhealthy / parked refusals, CLI-side AND hub gate | CLI: same table (rows 5–6, reason named for unhealthy). Hub: `a_prompt_to_a_parked_pane_s_session_of_record_is_refused_in_every_variant` (queue × replace matrix, wire shape asserted) + the unhealthy row. Both twins hold the identical predicate order parked → unhealthy → mismatch, each doc comment cross-references the other (binding 6) | PASS |
| Routed `say --queue` returns without waiting | Pure-decision tests `a_deadline_expiry_after_acceptance_is_queued_exit_0` (`queued ses-demo-c1r1`, stdout, exit 0; JSON `{"queued":true,"session":...}`) and `a_reply_inside_the_wait_prints_as_the_bare_form_does`. Source: `returns_on_acceptance` (routed + `--queue` + local socket only), `QUEUE_ACCEPT_WAIT` = 2 s replaces only `call.timeout` (`say_with` still puts `--timeout` in `timeout_ms`), `wait_ended` maps the post-acceptance Io deadline to `Deadline` | PASS |
| `close --profile P` bumps generation once; failed close leaves P unchanged; `--spec-only` changes P and nothing live; without `--profile` no profile changes; spec-less pane refuses | `close_with_a_profile_removes_the_spec_and_bumps_the_generation_once` (generation 2, exactly one `CasPut`); `a_failed_live_close_restores_the_profile_specs` (specs restored, generation 3 = edit + reversal, record stays, reconcile step in message); `close_spec_only_removes_the_spec_and_touches_nothing_live` (exact-empty herdr/host/harness/probes, record stays, no pane-store write); `close_without_a_profile_never_touches_a_profile` (zero profile-store calls); `close_refuses_a_spec_less_pane_before_any_profile_write` (exit 3, `assert_untouched`, generation stays 1 — the pre-check-before-any-write pin, both `--spec-only` and live paths) | PASS |
| `say --profile P --pane X`, X outside P, refuses `pane-not-in-profile` | Engine table row 3; `--profile` scope via `ProfileScope::resolve` in `routed_record` (the one helper, as park) | PASS |
| JSON passes the envelope helper; exits equal across formats | `run_both` (rig, close/rig.rs:232) runs text + JSON per case, `check_envelope` on the JSON side, exit equality; `close_json_is_one_envelope_and_the_exits_agree` pins `{"pane": ...}` data | PASS |
| Hub gate at `send_prompt`; healthy matching pane goes through; wire shape | Six gate tests inside `dispatch.rs`'s `mod hold_tests` (O applied T's authored block verbatim — authorship T's, recorded in the journal): parked-every-variant, unhealthy+mismatch, healthy-and-unmatched go through, fail-closed across same-named panes (first bad in name order, both sort orders), operator-hold-first with its own kind, wiring pin. Wire shape asserted field-by-field: `-32011`, `hold_kind: "pane"`, `data.reason` = the code, no `data.since`, one line, nothing sent | PASS |
| ADR-0003 rows (say-row exit-3 extension), ADR-0021 as-built + code-table, cli-surface lines, CHANGELOG, stub.rs ruling 9 | ADR-0003: close row gains `PANE`, say row gains "exit 3 also for a pane-routed refusal", the say/interrupt/answer paragraph rewritten to the shipped behavior. ADR-0021: three "as built (#646)" sections (Close, Prompting a pane, the pane-state gate), §9 rows for close (+`pane-not-in-profile`) and the prompt verbs (three open codes, exit 3, hub gate), class-of row names the three codes refusals, park-section pointer amended. cli-surface.txt: 4 `# #646` lines (3 close forms + routed queue). CHANGELOG: part-2 and part-3 entries after the park entry, matching what shipped. stub.rs: the `("pane","close",646)` row and `PANE_FORM_REFUSAL` deleted, the `// #646` comment kept | PASS |
| fmt / clippy / no new unsafe / no new dep / fakes-only | T's Tier 1 table (rustfmt clean; clippy exit 0 after O's applied allow line); S's own diff greps: 0 `unsafe` in code, 0 `dbg!`/`TODO`/`println!` additions, 0 Cargo.toml changes; tests import only `holler_pane_testkit` (fakes, kit rig, envelope helper) and synthetic `demo-*`/`ses-demo-*` names | PASS |

### The load-bearing spot-checks O asked for (all confirmed at source)

1. **Gate fail-closed order**: `send_prompt` runs `gate.holds.admit(...)?` first, then
   `pane_gate(...)?` — an operator/default hold refuses first with its own kind (pinned by
   `an_operator_hold_still_refuses_first_with_its_own_kind`); the pane gate itself is one
   synchronous `list()` (no `await`), matches the bare session name, and any bad pane refuses
   (first in name order).
2. **Exit 3 + stable codes in messages**: `routing_stop` maps through `class_of(code).exit_code()`
   and `coded()` puts the code in the message exactly once; all seven codes exercised
   table-driven; the gate's refusal rendered CLI-side is also exit 3
   (`a_pane_hold_renders_one_line_exit_3_with_the_right_remedy`, JSON pinned as the 4-key object,
   never the held exit 4 or the release tail).
3. **`--queue`'s `queued <session>` exit 0**: see AC row above; the bare `say --queue SESSION`
   form is untouched (`returns_on_acceptance` requires `--pane` and no `--server`).
4. **Close transaction rollback**: `edit_spec(P, pane, Remove, act)` wraps the live act; the
   failed-close test pins specs restored, generation moved by two, record kept, reconcile step.
5. **`pane-not-in-profile` before any write**: `plan()` checks `spec_of` after reading P and
   before `close_live`/`remove_spec` open any write; the test pins `assert_untouched` + no
   generation bump.
6. **`--spec-only` closes nothing**: empty act (`nothing_live`) inside `edit_spec`; test pins
   exact-empty adapter logs and record retention.

## Deviations audit (record, not re-litigated)

| Deviation | Recorded where | S's audit of the record |
|---|---|---|
| `pane_hold_refusal(session, e, json)` 3-arg shape (F's change to T's pinned signature) | handoff-F "Decision kept"; handoff-T-green item 1 (T ACCEPTED as contract owner); decisions.md T-green entry | Record complete and sound: T's own pinned JSON needs `session`, which only the caller has (`ErrorData` carries none), and the shape mirrors `hold_cmd::held_refusal` per binding 2. JSON assertion unchanged in the tests — verified at source |
| `SendPromptError` `#[derive(Debug)]` | handoff-A-dup finding 2 (advisory) | Test-enabling only (failure prints in the gate tests); confined to a radius file; no behavior change. Verified in the diff |
| `legacy_verbs.rs` rewrite | handoff-A-dup finding 1; decisions.md T-red entry ("Assumed") | Forced twice over by ruling 9 (import deleted) and the story replacing the refusal the file asserted. The rewrite is minimal and keeps the file's process-level role (no stub, usage wins, session forms untouched) — verified by reading the file |
| `cli.rs` / `prompt_target.rs` staleness (help strings "until #646", `route` now unused) | handoff-F "For O"; handoff-A-dup finding 3; decisions.md F entry | Correctly left: both files frozen to #637/#670; S verified zero diff on them in this run. ADR-0021's as-built text carries the truth meanwhile. Follow-up story material |
| "not implemented until #649" for routed forms under unwired ports | CHANGELOG part 3; `legacy_verbs.rs` module doc; the story's own AC text | Consistent: `Wiring::connect` is #649's unwired port set (wiring.rs:9-11); the process tests pin exit 1, an error line, and the old #646 stub line absent — the routed forms run the engine and fail as pane-path failures, exactly as the issue's split amendment describes |
| F's seven implementation decisions (handoff-F "Decisions S should audit") | handoff-F; handoff-T-green advisory ("nothing new needs a T ruling") | Reviewed: 1 (unreadable registry gates nothing) and 4 (remote `--queue` keeps waiting) are recorded in ADR-0021's as-built text; 2 and 6 follow T's own pins (gate after admit; no snapshot observe); 3 and 5 sit in deliberately-unpinned space T recorded at red; 7 (`answer --pane` checks CLI-side though `session/answer` is ungated) is what the issue's routing AC requires. All stay within contract or recorded unpinned space |

## Quality audit

| Area | Result | Notes |
|---|---|---|
| API consistency | PASS | `close` follows park's verb pattern (own clap `Args`, optional positional + verb-level usage, `output::emit`); routing reuses `ProfileScope::resolve` and the frozen `prompt_target` tail parsing |
| Error handling | PASS | Seven refusal codes table-driven; usage 2 / refusal 3 / failure 1 all asserted; reconcile-step discipline pinned both ways (present after first live call, absent on plan refusals, never doubled) |
| UI/UX match to spec | N/A | No UI surface |
| Accessibility | N/A | No UI surface |
| Architecture gate | PASS | A up-front + a-dup both PASS; S found no quality finding that conflicts with A's verdict |
| Code organization | PASS | One verb one file (close engine private to close.rs); routing engine in say_cmd.rs called by interrupt/answer; hub gate mirrors the with_holds precedent; twin predicates cross-referenced |
| Security | PASS | No credentials touched; synthetic names only; no new deps; no unsafe |
| Performance | PASS | Gate is one sync `list()` per prompt with no `await` (totally ordered against pane writes, as `holds.admit`); CLI routing is record reads only, no live probe |
| Visual regression | N/A | No UI surface |
| Naming consistency | PASS | Codes, consts, and wording match ADR-0021 §9 and T's pinned contract; close wording set exactly as decided |
| Test quality (rubric §7) | PASS | 33 new/rewritten tests across four tiers (pure-decision unit, engine unit, in-process verb harness, process binary, hub unit), each naming one behavior and asserting behavior (call logs, output, exits), not implementation; proportionate to a feature story; no assertion-free, tautological, or coverage-padding tests found; the four T-green repairs were compile-shape or self-contradiction fixes with no weakened pin (JSON/key-order/wire-shape assertions verified untouched); no "delete or merge" findings |

## Scope check

Every changed file is inside the brief's blast radius, an AC-named doc, a run artifact, or the
ruling-9-forced test sibling — verified via `git diff 3d95aec..HEAD --name-status`:

- Code (7): `close.rs`, `say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs`, `live.rs`,
  `circuit/dispatch.rs`, `serve.rs` + the one-line `circuit.rs` touch — all named; the two
  binding-1 touches landed exactly as bound (one `HoldGate` construction line; `PaneDeps::load`
  reorder + `.with_panes` + one doc-comment line).
- Tests (5): `close.rs` + `close/rig.rs`, `target_flags.rs`, `process/stub.rs` (ruling 9),
  `process/legacy_verbs.rs` (forced sibling, recorded), plus 4 fixture lines.
- Docs (4): ADR-0003, ADR-0021, CHANGELOG, cli-surface fixture — all ACs.
- Run artifacts: the run's own handoffs and journal.

No golden rewrites, no unrelated refactors, no drive-by fixes. Frozen files
(`prompt_target.rs`, `cli.rs`, `holler-pane/**`, `holler-proto/**`) show zero diff. No
under-delivery: every brief AC row above is backed. The story's scope (parts 2 and 3) is exactly
what shipped; part 1 verified present from #711.

## Findings

| # | Severity | Finding |
|---|---|---|
| 1 | advisory (O reminder, not a finding) | The working tree carries uncommitted run artifacts (`decisions.md` T-green entry modified; `handoff-A-dup.md` untracked) — expected per the run's own rules and O's to commit, but the merge phase refuses a dirty worktree, so commit them before advancing. |
| 2 | advisory | Follow-up story material, already recorded by A-dup finding 3 and F's "For O": `cli.rs` `--pane` help strings still say "refused until #646", `prompt_target.rs`'s module doc is stale, and `prompt_target::route` is now unused by production (kept `pub` per the brief). Bundle with #670. |
| 3 | advisory | F decision 3 (a bare `interrupt SESSION TEXT` to a gated session applies its cancel, then the redirect is refused) is recorded but unpinned by any test — the pre-cancel fast path sits outside this run's radius. Worth a case when #649/#670 revisit `interrupt.rs`. |

No REWORK findings. No unrecorded deviations found.

## Verdict

PASS — story #646 parts 2 and 3 are satisfied: every acceptance criterion is backed by a named,
passing T-authored test or verified in the committed docs, the implementation matches the brief
v2 and A's 11 bindings at source level, all deviations are recorded with owners, and the scope is
exactly the story's. Ready for O to commit the journal artifacts and advance to merge.

**Verdict:** PASS

## Advisory notes

- The run's three tooling anomalies (delegated-advance empty output; the shared-target build
  contamination and its seed-script fix; the verdict-line parser strictness) are all journaled
  with root causes — good material for the pipeline run report, none affect this story's product.
- The gate tests' path (authored by T, applied by O into `src/**`) is now a twice-used pattern
  this repo (see the memory of run 0646 t-red); if a third run hits it, consider a brief-level
  standing arrangement so the journal does not need to re-derive it.
