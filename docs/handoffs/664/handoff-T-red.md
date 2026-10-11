# Handoff-T-red: Phase 4 - `holler profile apply` (#664)

**Date:** 2026-10-11 (07:19 MDT)
**Branch:** `issue-664-implementation`, with `origin/main` (`263a2d7`, #808) merged in but not yet committed (see below)
**Brief / wireframe reviewed:** `docs/handoffs/664-brief.md` (§1 to §13, C1 to C20, the decisions on O1 to O6, E1 to E76) and `docs/handoffs/664/handoff-A.md`. Wireframe: N/A (`uiSurface: false`).

## A precondition

Confirmed: A returned **PASS** on the plan (0 block, 9 warn).

**A's warn 1 is done.** The base had moved under the run, so I ran `git merge --no-ff --no-commit origin/main` before
writing any test. The merge was clean, because the branch had changed only `docs/handoffs/`. The tree now holds #808
(`HerdrPane.label`, `FakeHerdr::rename`, `same_place`, ADR-0021 section 2). `MERGE_HEAD` is set, so the driver's
phase commit (`git add -A && git commit`) is the merge commit. T makes no commit of its own. The diff gates' merge base
becomes `263a2d7`, so they show only this story's work.

Warn 6 (the rig) is followed. Warn 5(a) scopes the stale-spec test, which is entry 19 below. The tests honour warn 1,
warn 3 (`diff_profile` rows, `PaneDiff` unchanged) and warn 9 as far as the brief decides them. The rest of warn 9 is
listed under "Not pinned".

## The API these tests pin (F implements it; nothing here is in `src/`)

| API | The shape the tests use | Source |
|---|---|---|
| `holler_pane::tx_apply::apply` | `pub fn apply(ports: Ports<'_>, request: &ApplyRequest, options: &TxOptions) -> Result<Applied, PaneError>`. `Err` is an error with no plan: the §4 gate's `usage`, `profile-not-found`, or a store read that fails (C8). Every step outcome, `failed` included, is in `Ok`. | C1, C8, C13, C17, O6 |
| `ApplyRequest` | `{ profile: ProfileName, herdr_session: Option<String>, take_over: bool, dry_run: bool }` | §8 |
| `Applied` | `{ profile: ProfileName, dry_run: bool, steps: Vec<Step> }`, `Debug`. The verb serializes it as §7's `data` (the test reads `data["profile"] == NAME`). | §7, C6 |
| `Step` | `{ pane: String, action: Action, outcome: Outcome, code: Option<String>, diff: Option<PaneDiff>, error: Option<PaneError> }`, `Debug`. `code` is the refusal or failure code. `error` is the step's error. JSON has `pane`, `action`, `outcome`, `code` (absent or null when there is none), and `diff` (a whole `PaneDiff`). The tests never check that a key is absent, so F may add fields. | §7, C6, R6 NIT-1 |
| `Action` | `Copy + PartialEq + Debug + Serialize` (unit variants serialize as strings). The tests name `Create`, `Adopt`, `Report` and `Relaunch`. F may add variants (e.g. for rows a and e, or an observation that failed). JSON is compared with the variant's own serde string, so the strings are F's (snake_case, as `PaneStatus`). | step table, warn 9a |
| `Outcome` | `Copy + PartialEq + Debug + Serialize`, with `Created`, `Relaunched`, `Adopted`, `Reported`, `Refused`, `Failed` and `Skipped`. JSON strings are pinned as `"created"`, `"relaunched"`, `"adopted"`, `"reported"`, `"refused"` (the brief names them, §5). F may add a dry-run outcome. | §5 |
| the verb | `holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]` (and the global `--format`). The verb calls the engine with `TxOptions::default()`. | §8, C13 |
| the verb at exit 1 | Only the failure is printed: `error.code` is the failed step's code, and `error.message` is `apply stopped at <pane> (<action>): <msg>; <N> later step(s) skipped`. The message must contain the pane name. `(<action>)` is the action's serde string. Text mode: stdout is empty and stderr has one `error: ...` line. | O6, C19 |
| `tx_apply.rs` module doc | The `//!` lines, joined, hold `**Bound (I5).**`, `its own I5 bound`, `no overall cap`, `10 s` and `DEFAULT_BUDGET`. | §9, C16 |
| `orchestrator-not-relaunched` | The `code` of row e's refused step. | §3 row e |

## Tests authored

Every test is in the `profile_verbs` target (already declared in `crates/holler-cli/Cargo.toml`), under
`crates/holler-cli/tests/profile_verbs/`. The tier is the engine (`tx_apply::apply` over the fakes, read from the
result it returns) or the verb (in-process, over the same fakes). The engine tier is the cheapest one that sees the
steps, and the verb tier is the cheapest one that sees exit codes and output. Entry numbers are C14's.

| # | Test | Pins | Tier |
|---|---|---|---|
| 1 | `apply/rows.rs:427` `apply_twice_changes_nothing_the_second_time` | Entry 1: run 1 creates, joins, relaunches and adopts; run 2 adopts all, with no write and no live call but observations | engine |
| 2 | `apply/rows.rs:70` `row_c_a_healthy_matching_pane_is_adopted_with_no_live_call` | Entry 2 | engine |
| 3 | `apply/rows.rs:88` `row_d_a_healthy_pane_that_differs_is_reported_and_never_moved` | Entries 3 and 10: `diff` is `PaneDiff{Differs, diff_spec(..)}`, the cell is in rowcol JSON, and the pane is not moved, killed or written | engine + verb JSON |
| 4 | `apply/rows.rs:141` `row_a_a_pane_of_another_profile_is_refused_without_take_over` | Entry 5; exit 0 | engine + verb |
| 5 | `apply/rows.rs:163` `row_b_missing_panes_are_created_by_splits_in_grid_order` | Entry 9: on the split-only fake, P lists the cells in reverse, which no split reaches, so this also pins §3's grid order | engine |
| 6 | `apply/rows.rs:203` `row_b_an_occupied_cell_is_refused_through_launch` | Entry 12: `grid-occupied`, the occupant is never adopted, and the run goes on | engine |
| 7 | `apply/rows.rs:236` `a_failing_probe_refuses_that_pane_and_the_others_proceed` | Entry 8: `probe-failed`, nothing is made, and the verb exits 0 | engine + verb |
| 8 | `apply/rows.rs:323` `a_create_answered_pane_exists_is_refused_and_the_run_goes_on` | Entry 13 (B-3): the seam `AppearsAfterReread` makes the record appear after apply's re-read and before launch's check | engine over a seam |
| 9 | `apply/rows.rs:361` `row_f_a_killed_agent_is_relaunched` | Entry 16: a new server and pid, the record still in P, and exactly one write | engine |
| 10 | `apply/rows.rs:389` `a_per_pane_usage_from_the_relaunch_rules_is_refused_and_the_run_goes_on` | Entry 30: a `refused` step with a code, the run goes on, exit 0, and nothing is touched | engine + verb |
| 11 | `apply/rows.rs:467` `herdr_session_is_needed_for_a_create_alone` | Entries 32 and 4 (a dry run with every pane recorded passes the gate): both formats exit 0, and the relaunched pane keeps its record's Herdr session | verb |
| 12 | `apply/rows.rs:25` `one_run_takes_every_row_in_grid_order` | §3 in one run: rows a to f in grid order, whatever P's order (C6); JSON `code` for row e | engine + verb JSON |
| 13 | `apply/takeover.rs:60` `row_c_writes_nothing_joins_once_or_leaves_then_joins` | Entry 25, first half (R2 NV-3, R3 NV-2, R5 NV-6): 0, 1 or 2 `cas_put`s, and the feed's exact records (leave, then join) | engine |
| 14 | `apply/takeover.rs:94` `take_over_adopts_a_pane_of_another_profile` | Entry 6: exit 0, and the pane ends in P | verb |
| 15 | `apply/takeover.rs:107` `row_d_take_over_moves_the_profile_and_touches_nothing_live` | Entry 14 | engine |
| 16 | `apply/takeover.rs:129` `row_e_refuses_an_unhealthy_orchestrator_and_writes_nothing` | Entry 15 / C15: in P without `--take-over` and in Q with it; no feed event, nothing live | engine |
| 17 | `apply/takeover.rs:176` `row_f_take_over_leaves_joins_then_relaunches` | Entry 29, first half (O1, C11, C17): the feed holds leave, join, then the relaunch's record write | engine |
| 18 | `apply/takeover.rs:213` `row_f_take_over_whose_relaunch_fails_stays_in_p` | Entry 29, second half: `fail_next(HerdrOp::Version)`, which only the relaunch calls. The pane stays in P with no write after the join, the next step is `skipped`, and the verb exits 1 in the pinned form `(relaunch)` | engine + verb (both formats) |
| 19 | `apply/takeover.rs:266` `a_take_over_join_that_conflicts_leaves_the_pane_in_no_profile` | Entry 25, second half (R2 W-1, O4): `NthCasPut` fails the 2nd write, and the leave alone landed. Exit 1 with `generation-conflict` and `(adopt)`. The message names Q and "no profile" | engine + verb |
| 20 | `apply/failures.rs:42` `a_failed_health_call_stops_the_run` | Entry 17: `failed` then `skipped`, only observations, and the create never ran; exit 1 | engine + verb |
| 21 | `apply/failures.rs:76` `a_dry_run_with_a_failed_health_call_prints_only_the_failure` | Entry 21 (C2, O6) | engine + verb |
| 22 | `apply/failures.rs:116` `a_store_conflict_at_a_steps_write_stops_the_run` | Entry 22 (NV-9): the join of an adopt from no profile; exit 1, `(adopt)` | engine + verb |
| 23 | `apply/failures.rs:183` `an_unreadable_snapshot_fails_the_first_step_that_needs_it` | Entry 23 (C3): a create before the failing step still runs (`created`, `failed`, `skipped`) | engine + verb |
| 24 | `apply/failures.rs:225` `a_failed_health_for_a_pane_row_a_refuses_is_not_a_failure` | Entry 33 (R5 B-3): `refused`, then `created`; no event for that pane; exit 0 | engine + verb |
| 25 | `apply/observe.rs:48` `an_orchestrator_herdr_does_not_list_is_refused_not_adopted` | Entry 24 (R2 NV-5, NV-6): after a `vanish`, and with another Herdr session beside a held pane id | engine |
| 26 | `apply/observe.rs:73` `a_labelled_pane_is_still_listed_and_adopted` | A's warn 1 (#808): a label never makes a pane "not listed" | engine |
| 27 | `apply/observe.rs:84` `the_shown_session_decides_only_on_another_session` | Entry 28 (C12, O2): home screen gives `adopted`; an `Err` gives `adopted`; another session gives `relaunched`, and the TUI shows the kept session after | engine |
| 28 | `apply/dry_run.rs:23` `a_dry_run_calls_only_its_reads_and_plans_every_row` | Entries 31, 7 and 4 (engine side): Herdr makes only `Snapshot`; the harness makes only `Health` and `ShownSession`; no host call, no probe, no store write, no pane made. Row a is refused; d is reported with its diff; e is refused; b, c and f get actions Create, Adopt and Relaunch, with no effect outcome | engine |
| 29 | `apply/dry_run.rs:70` `a_dry_run_prints_its_plan_as_the_steps_object` | Entry 4 (verb side): `{"profile","dry_run":true,"steps"}` passes the kit's checker; steps in grid order; text exits 0 | verb |
| 30 | `apply/dry_run.rs:102` `a_dry_run_with_take_over_plans_the_adopt_and_writes_nothing` | Acceptance 2's dry-run half under `--take-over` | engine |
| 31 | `apply/dry_run.rs:120` `a_dry_run_and_profile_show_agree` | Entry 11: every `differs` row of `show` equals the step's `diff`; `matches` gives Adopt with no diff; `missing` gives Create with no diff; no extra steps; grid in rowcol form | verb (both verbs) |
| 32 | `apply/bound.rs:34` `the_engine_doc_states_the_bound` | Entry 20 (C16): the five strings in the module doc | file read, no fake |
| 33 | `apply/bound.rs:68` `a_step_that_outlasts_its_budget_fails_and_stops_the_run` | Entry 26: #644's numbers (budget 800 ms, harness delay 500 ms). `Timeout{op:"pane.launch"}`, then `skipped`, in under 3 s | engine |
| 34 | `apply/bound.rs:102` `there_is_no_budget_over_the_whole_run` | Entry 27 (O5): delay 100 ms and 6 harness calls per create, budget 900 ms. Both creates are `created`, and the run is asserted to take longer than one budget, so the premise is checked | engine |
| 35 | `apply/bound.rs:133` `a_spec_edited_during_the_observation_is_not_written_back` | Entry 19, scoped by warn 5(a): another writer edits P after the up-front `health`. The created record and P keep that writer's spec | engine + hook |
| 36 | `apply.rs:76` `a_create_without_herdr_session_is_usage_before_any_call` | Entry 18 (C1, R2 W-3): live and dry, both formats. Exit 2; no Herdr, harness or host call; no probe; no write; no step printed; JSON `data` is null, coded `usage`, and the message names `--herdr-session` | verb |
| 37 | `apply.rs:104` `a_run_that_exits_0_prints_the_steps_object` | C6 and §7 for a live run, and the text names every pane | verb |
| 38 | `apply.rs:146` `an_error_before_any_plan_is_the_verbs_own` | C8: `profile-not-found` 3, an unreadable registry `unavailable` 1, a bad name `usage` 2; both formats; nothing live | verb |

The verb-level half of entries 17, 21, 22, 23, 25 and 29 goes through `rig.rs`'s `assert_stopped`. It runs each format
on a fresh rig and checks the decision on O6: exit 1; JSON `data` null; `error.code` is the step's code; the message has
the pinned prefix, names the pane (and the action where the row is known), and ends with `; 1 later step(s) skipped`.
In text mode, stdout is empty and stderr is one `error: apply stopped at ...` line.

**Test support (all under `tests/`).**

- `profile_verbs/apply/rig.rs` (new, 742 lines: over the 600-line warning, under the 900-line limit). It is apply's
  live rig, copied by name from `pane_verbs/launch/rig.rs`: the fakes with a working `FakeProfileScope`, `LinkedHost`,
  `HookedHarness` (`after`), the `Calls` span, `seed_live`, `set_specs`, the feed's `cursor`/`writes_since`,
  `every_row()` (one pane per row, with P's specs reversed), `assert_only_observed` and `assert_stopped`.
- `profile_verbs/rig.rs`: one additive helper, `serve_server`. It is the only place a test starts a fake server
  (decision O3: if #802 lands first, adapting is a rename here). `Rig::new`, `ports`, `run_both*` and
  `assert_no_adapter_call` are unchanged (A's warn 6).
- `profile_verbs/apply.rs`: #670's stub case is replaced. It declares `apply/*` (A's warn 7a).
  `profile_verbs/main.rs` is not edited.

## RED confirmation

Every command ran from the worktree, on `src/` exactly as `origin/main` has it (`git diff origin/main --stat --
'crates/*/src'` is empty).

**1. Compile RED at the new API.** `cargo test -p holler-cli --test profile_verbs` fails with 10 errors. Each names
only the missing `tx_apply` API, and none is a typo, an import or a setup error:

```
error[E0432]: unresolved imports `holler_pane::tx_apply::Applied`, `holler_pane::tx_apply::ApplyRequest`, `holler_pane::tx_apply::Outcome`, `holler_pane::tx_apply::Step`   (apply/rig.rs:28)
error[E0432]: unresolved imports `holler_pane::tx_apply::Action`, `holler_pane::tx_apply::Outcome`   (apply.rs, rows.rs, takeover.rs, failures.rs, dry_run.rs)
error[E0432]: unresolved imports `holler_pane::tx_apply::Applied`, `holler_pane::tx_apply::Outcome`  (observe.rs)
error[E0432]: unresolved import `holler_pane::tx_apply::Outcome`                                     (bound.rs)
error[E0425]: cannot find function `apply` in module `tx_apply`                                      (apply/rig.rs:282)
error[E0425]: cannot find type `Action` in module `tx_apply`                                         (apply/rig.rs:575)
```

**2. Behaviour RED against an API-only stub.** This was a throwaway edit to `src/`, never staged, and restored from
copies afterwards. It contains the types above. `apply` returns `Ok` with no steps. The verb gains the three flags and
still answers `not implemented (story #664)`.

Result: **43 passed, 38 failed.** Every new test fails on an assertion about the missing behaviour, and every existing
`profile_verbs` test passes. The failures look like this:

- Engine cases: `assertion left == right failed  left: []  right: [("pane-x", Failed), ("pane-y", Skipped)]`. Every
  engine case fails the same way on its expected steps, e.g. `right: [("pane-a", Refused), ("pane-b", Created), ...]`.
- Cases that look a step up: `a step for pane-d: Applied { profile: ProfileName("demo"), dry_run: false, steps: [] }`.
- Verb cases: `left: 1 right: 2` / `right: 0` / `right: 3`, with `err: "error: not implemented (story #664)"`.
- The doc test: `the module doc holds "**Bound (I5).**": "The \`holler profile apply\` transaction: create missing panes, ..."`.

**3. The suite can be satisfied as written.** This was a throwaway prototype of `tx_apply::apply` and the verb (about
200 lines), never staged and restored. Its parts:

- the §4 gate after the registry read;
- a scoped-thread observation;
- the rows of §3 from `diff_spec`;
- the steps through `tx_launch` unchanged, with the twins inline;
- the O6 message.

Result: **81 passed, 0 failed.** That held over 5 runs of the whole target and 10 runs of the `apply::` cases.
`cargo clippy -p holler-cli --test profile_verbs -- -D warnings` is clean on the test files. `rustfmt` is applied.

**4. Mutants of that prototype.** Each must fail at least one test, and each did:

| Mutant | Fails |
|---|---|
| gate checked after a Herdr call | `a_create_without_herdr_session_is_usage_before_any_call` |
| no grid-order sort | `row_b_..._by_splits_in_grid_order`, `one_run_takes_every_row_in_grid_order`, `a_dry_run_calls_only_its_reads...` |
| "listed" by `HerdrPane` equality (`contains`) | `a_labelled_pane_is_still_listed_and_adopted` |
| "listed" by pane id alone | `an_orchestrator_herdr_does_not_list_is_refused_not_adopted` |
| `Ok(None)` shown counted as a mismatch | `the_shown_session_decides_only_on_another_session` |
| the plan's P used, no re-read (stale spec) | `a_spec_edited_during_the_observation_is_not_written_back` |
| relaunch from the pre-join record | `row_f_take_over_leaves_joins_then_relaunches` |
| row e moves the profile under `--take-over` | `row_e_refuses_an_unhealthy_orchestrator_and_writes_nothing` |
| a dry run that acts | `a_dry_run_calls_only_its_reads...`, `a_dry_run_with_take_over_plans_the_adopt...` |
| a row-a pane whose health failed is `failed` | `a_failed_health_for_a_pane_row_a_refuses_is_not_a_failure` |
| O4 broken: a compensating re-join of Q | `a_take_over_join_that_conflicts_leaves_the_pane_in_no_profile` |
| row d carries no diff | 4 tests (rows, takeover, dry_run x2) |
| a failed step does not stop the run | 4 tests (store conflict, relaunch fails, join conflicts, budget) |
| one budget over the whole run | `there_is_no_budget_over_the_whole_run` |
| an adopt from no profile does not join | `row_c_writes_...`, `a_store_conflict_at_a_steps_write_stops_the_run` |
| a per-pane `usage` treated as a failure | `a_per_pane_usage_from_the_relaunch_rules_is_refused...` |

## Not pinned (left to F, for S to audit; A's warn 9)

- The **outcome of a dry-run step that would act** (rows b, c and f). The tests require only that it is none of
  `created`, `adopted` or `relaunched`. The brief does not name it.
- The **action** of rows a and e, and of a step whose observation failed. `assert_stopped` checks `(<action>)` only
  for adopt and relaunch.
- The **code** of a per-pane `usage` refusal (§4 says "a named code"). The tests require only a non-empty `code`.
- The **step for a spec whose pane is not a valid `PaneName`**: no test.
- The **text form** of an exit-0 run. The tests require only that it names every spec pane on stdout.
- The message wording beyond O6's pinned frame. One exception is O4's case, which must name Q and say "no profile",
  because O4 decides that.

## RED notes

- **The pane-exists seam (entry 13)** lets the record appear right after the first registry read (a `get` or a `list`)
  that follows apply's first Herdr `snapshot`. In C1's order that read is the step's re-read (§9), because step 1's
  registry read comes before the observation. An engine that read the registry again after the observation, to choose
  rows, would see the record too early. C1 rules that out (rows are chosen from step 1's reads and the observations),
  so the test follows the brief.
- **Fixtures that fail one `health` call** hold exactly one recorded spec pane, as C7 requires, since apply observes
  the panes in parallel.
- **The take-over fixtures** put the pane's record in Q and its spec in P. Q holds no spec for it, which the tests do
  not need.
- `row_d_a_healthy_pane_that_differs_is_reported_and_never_moved` covers entries 3 and 10 together (the difference is
  the cell).

## Ready for F

Confirmed: the RED is valid. Every new test fails to compile at the new API, and against an API-only stub each one
fails on the assertion the feature satisfies. A throwaway prototype shows the suite can be satisfied as written, and
16 mutants show it has teeth. The 43 existing `profile_verbs` tests are unchanged and green.

Test files are staged by explicit path. Nothing is committed. The merge of `origin/main` is staged and waits for the
phase commit.

F implements in `src/`:

- `tx_apply.rs`: the API above, the bound doc, `ORCHESTRATOR_NOT_RELAUNCHED`, and the twins. Overflow goes to
  `tx_apply/*.rs` (A's warn 7c).
- `profile/apply.rs`: the verb.
- the docs: ADR-0021's "adopt" and section 9 row, `cli-surface.txt`, ADR 0003, and `CHANGELOG.md`.

T-red complete, RED is valid. F may implement against the authored tests.
