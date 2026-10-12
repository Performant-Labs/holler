# Handoff-S: Phase 10 - `holler profile apply` (#664)  (spec audit)

**Date:** 2026-10-11 (11:51 MDT)
**Branch:** `issue-664-implementation` (worktree `.claude/worktrees/0664b-profile-apply`, head `81cec27`, merge base `7284001`, #802).
`origin/main` moved during this audit. It is now `b8242d7` (11:45 MDT): #644 part 2 (PR #822), after #820 and #821. A trial
merge (`git merge-tree --write-tree origin/main HEAD`) is clean. **This flips C18's order clause** (Notes for O, item 1).
**Issue:** #664 (epic #633), read live with `gh issue view 664`. The issue body is the brief's top half (Scope, Acceptance,
Blast radius, Rules, §1 to §13). The brief adds C1 to C25, the decisions on O1 to O6 and the Evidence, and it supersedes
the issue in three places, each recorded: rigor `second-opinion` (the issue says `in-session`), C24's rustfmt rule (the issue
says `cargo fmt`), and §13's follow-up, not filed (C18).
**Brief:** `docs/handoffs/664-brief.md` as amended in `af230b6`.
**Handoffs reviewed (all under `docs/handoffs/664/`):**
- `handoff-A.md` (round 4) and `handoff-A-dup.md`;
- `handoff-T-red.md` (fresh run; the first run's, which authored the 38 tests of C14, is `git show e773945:...`);
- `handoff-F.md` (fresh run; the first run's at `7f23e0f`);
- `handoff-T-green.md` (fresh run; the first run's at `fc11ee9`);
- `decisions.md` and `evidence.md`.

**Outside gates** (`deepseek-v4-pro`, per each `.usage.json`): the brief gate's last round (r2, 06:41 MDT) PASS; the diff gate
(r1, 11:30 MDT, on this run's code, after T-green) PASS with no BLOCK. Its NV-2 (the timing tests may flake on a slow runner)
is advisory 5 below.

**Verdict:** PASS

## A precondition

Met.
- `handoff-A.md` (round 4, on the brief as of `af230b6`): **PASS**, 0 blocks, 4 warns. Warns 1 and 3 are brief edits (E40,
  the 27 paths), not F's. Warn 2 was answered by T's `pane-g` fixture. Warn 4 (the reconcile step in the C22 sentence and in
  `Step.acted`'s doc) was F's, and I checked it under Spec compliance (C22, C25).
- `handoff-A-dup.md` (`7284001..8ed5f8c`): **PASS**, 0 blocks. All nine round-4 reuse expectations are met. Its two warns
  are follow-up work (advisory 4).

## T precondition

Met. `handoff-T-green.md`: "Blocking issues: None".

- **RED, first run (`e773945`).** The 38 tests of C14's entries 1 to 33, C6 and C8 failed to compile against the new API
  (10 `E0432`/`E0425` errors). Against an API-only stub, 38 failed on their assertions and 43 passed. A throwaway prototype
  passed all 81, and 16 mutants of it were each killed.
- **RED, fresh run (`d7c92de`).** 83 passed and 1 failed: `a_refusal_after_the_act_began_names_the_reconcile_step`, on its
  C25 assertion only, in 10 of 10 runs. Prototype mutants M1 to M4 were each killed.
- **GREEN (`8ed5f8c`).** `profile_verbs` passed 84 of 84 in 10 of 10 runs. The verify command (`cargo test --workspace
  --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`) gives 148 binaries: 2157 passed, 0 failed,
  25 ignored, the same counts as F. Mutants M1 to M5 of F's real `reconciled` and `stopped` were each killed. The first run's
  five engine mutants still apply, since this run changed only doc comments in the engine. Tier 1 is clean: clippy
  `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, rustdoc `-D warnings`, rustfmt on the story's files,
  `docs_cli_test`, `cli_surface_test`, `wire_selftest`, `pane_cli_process` and `test-hooks.sh`.
- **Phase boundaries** (checked with `git show --stat` on each phase commit):
  - F's commits (`7f23e0f`, `0b26110`) touch no test file. The one exception is `cli-surface.txt`, ADR 0003's table, which
    §10 and the first T-red assign to F.
  - T's commits touch no `crates/*/src/**`.
  - Since T-green, only `handoff-A-dup.md` and `decisions.md` changed (`git diff --stat 8ed5f8c..HEAD`).

## Acceptance criteria

The acceptance is C14's list (the Acceptance section's fifth bullet). Every entry is judged at the end of the story, on
F's code. Test paths are under `crates/holler-cli/tests/profile_verbs/`.
- "Engine" means `tx_apply::apply` over the fakes, read from the result it returns.
- "Verb" means the in-process verb over the same fakes.
- `assert_stopped` (`apply/rig.rs:585`) is the O6 check at the verb, run in both formats: exit 1; a JSON `data` of null;
  `error.code` equal to the step's code; the pinned message prefix, pane and action, and the `; N later step(s) skipped`
  tail; in text mode, an empty stdout and one `error:` line.

| # | Criterion (C14) | Proving test | Status |
|---|---|---|---|
| 1 | Apply twice: the second run changes nothing | `apply/rows.rs:447` `apply_twice_changes_nothing_the_second_time`. Run 1 adopts, joins, relaunches and creates. Run 2 adopts every pane, with `assert_only_observed` and no feed event | Met |
| 2 | Row c: adopted, not relaunched, no live call | `apply/rows.rs:89`: `Adopted`, `Action::Adopt`, only observations, no write, the record unchanged | Met |
| 3 | Row d: reported, not killed, nothing live touched | `apply/rows.rs:107`. The diff equals the `diff_spec` `PaneDiff`, and its JSON is `{"pane","status":"differs","differences":[{"field":"herdr.grid",...}]}`. The server stays `Running`, the pane stays at r2c1, and there is no write | Met |
| 4 | A dry run past the gate runs no probe and writes nothing, and its plan parses against the envelope (`data` is §7's object with `"dry_run": true`) | `apply/dry_run.rs:24` (engine: `assert_only_observed`, so no probe and no store write, and no pane made). `apply/dry_run.rs:66` (verb: `data_of` runs the kit's `check_envelope`, `dry_run` true, steps in grid order). The "every spec pane recorded" form is `apply/rows.rs:487` (a dry run without the flag exits 0 in both formats) | Met |
| 5 | Row a: refused `pane-in-other-profile` without `--take-over` | `apply/rows.rs:160`: the code, no write, the record still in Q, the verb exits 0 | Met |
| 6 | With `--take-over`, that pane is adopted (leave, then join) | `apply/takeover.rs:94` (verb: exit 0, the pane ends in P) and `apply/takeover.rs:60` (the case of Q: two `cas_put`s; the feed holds the leave, then the join, as exact records) | Met |
| 7 | A dry run's plan lists the pane of another profile | `apply/dry_run.rs:24` (`pane-a` `Refused`, `pane-in-other-profile`) and `:66` (the same in JSON). `:98` covers `--take-over`: the plan's adopt, nothing written, still in Q | Met |
| 8 | A failing probe: refused `probe-failed`, and the others proceed | `apply/rows.rs:255`: `Refused`, then `Created`. The probe ran once, nothing was made at r1c1, and the verb exits 0 | Met |
| 9 | Row b on #638's split-only fake: created, by splits | `apply/rows.rs:182`: `Placement::SplitOnly`, with P listing the cells in reverse order. Only grid-order creation reaches r2c1 (`check_split`, `holler-pane-testkit/src/herdr.rs:449`), so the test pins both the splits and the order | Met |
| 10 | A healthy pane in the wrong cell is reported, never moved | `apply/rows.rs:107`. Its one difference is the cell, and the pane is still listed at r2c1 | Met |
| 11 | `apply --dry-run` and `profile show` agree on one fixture | `apply/dry_run.rs:119`. Each of `show`'s rows is compared with the plan's step: `differs` gives `show`'s `PaneDiff`, `matches` an adopt with no diff, `missing` a create with no diff. There are 4 rows and 4 steps, positions in rowcol form. `pane-g` adds the agent case (C18, Notes for O item 1) | Met |
| 12 | Row b: an occupied cell is refused through launch | `apply/rows.rs:222`: `grid-occupied`, the occupant never adopted, and the run goes on | Met |
| 13 | Row b: `pane-exists` from launch is refused and the run goes on | `apply/rows.rs:342`. The `AppearsAfterReread` seam lands the record after apply's re-read and before launch's check. Result: `Refused` `pane-exists`, then `Adopted`, then `Created` | Met |
| 14 | Row d with `--take-over`: profile moved, nothing live touched | `apply/takeover.rs:107`: `Reported` with a diff, the feed holds the leave and then the join (exact records), nothing live | Met |
| 15 | Row e: refused `orchestrator-not-relaunched`, no registry write, also with `--take-over` | `apply/takeover.rs:129` runs it in P without the flag and in Q with it. Both times: no feed event, the record unchanged (generation included), only observations, the server still `Killed` (C15) | Met |
| 16 | Row f: a killed server is relaunched | `apply/rows.rs:380`: a new server on the port, `pid: None` (#802), the record in P, exactly one write (the relaunch's) | Met |
| 17 | A failed `health`: failed, the rest skipped, exit 1, only the failure | `apply/failures.rs:43`. Engine: `Failed` (action `observe`, `unavailable`), then `Skipped`; only observations, no write, the create never ran. Verb: `assert_stopped` | Met |
| 18 | The `--herdr-session` gate: exit 2, no step, no Herdr or harness call, live and dry, no plan | `apply.rs:76`, in all four combinations of live or dry and text or JSON. Exit 2; the Herdr, harness, host and probe logs empty; no store write; no feed event; `data` null, coded `usage`; no `pane-c` in the output; the message names `--herdr-session` | Met |
| 19 | A stale spec is not written back | `apply/bound.rs:136`. Another writer edits P during the observation, after `health`. The pane is created from the edited spec, and P keeps that writer's spec (scoped by A round 1 warn 5(a); see advisory 2) | Met |
| 20 | The module doc states the bound: the grep test, plus S's audit of C4's form | `apply/bound.rs:34` checks the five strings of C16 in the joined `//!` lines. My audit of C4's form is under Spec compliance (C4): the form is met | Met |
| 21 | A dry run with a failed `health`: failed, skipped, no probe, no write, exit 1, no plan | `apply/failures.rs:78`. Engine: `dry_run`, `Failed` then `Skipped`, only observations, no write. Verb: `assert_stopped` with `--dry-run` | Met |
| 22 | A store conflict at a step's write: failed, skipped, exit 1 | `apply/failures.rs:118`. `NthCasPut` fails the join of an adopt with `generation-conflict`: `Failed` (`adopt`), then `Skipped`, no feed event. Verb: `assert_stopped` | Met |
| 23 | An unreadable snapshot: the first step that needs it fails, the rest skipped, exit 1 | `apply/failures.rs:185`: `Created` (a create needs no snapshot), then `Failed` (`observe`, `unavailable`), then `Skipped`. Verb: `assert_stopped` | Met |
| 24 | An orchestrator Herdr does not list (after `vanish`; under another Herdr session) is refused, with nothing written or touched | `apply/observe.rs:51`, both cases. `Refused` `orchestrator-not-relaunched`, only observations, no write. The second case fails an apply that matches on the pane id alone (a first-run mutant) | Met |
| 25 | Row c's writes counted and in order; a join that conflicts: failed, skipped, exit 1 | `apply/takeover.rs:60`: 0, 1 or 2 `cas_put`s, with the feed's exact records. `apply/takeover.rs:266`: a take-over join that conflicts after its leave landed. That gives `Failed` with `generation-conflict` and the feed holds the leave alone. The record is in no profile and nothing live was touched. Verb: `assert_stopped`, whose message names Q and "no profile" (O4). `apply/failures.rs:118` is the join of an adopt from no profile | Met |
| 26 | A step that outlasts its budget: failed `timeout`, the rest skipped, inside a generous bound | `apply/bound.rs:70`: an 800 ms budget and a 500 ms delay. `Timeout { op: "pane.launch" }`, then `Skipped`, in under 3 s, with no record made | Met |
| 27 | No budget over the whole run | `apply/bound.rs:105`: a 1200 ms budget and a 100 ms delay, eight harness calls per create (C25). Both `Created`, and the run outlasts one budget. That lower bound is deterministic: 16 delayed calls take at least 1.6 s | Met |
| 28 | The shown session: home screen and an error give adopted, another session gives relaunched | `apply/observe.rs:87`. `navigate(None)` gives `Adopted`. `fail_next(ShownSession)` gives `Adopted`. Another session gives `Relaunched`, and afterwards the TUI shows the kept session of record | Met |
| 29 | Row f under `--take-over`: leave, join, relaunch; a relaunch that fails stays in P with no write after the join, then failed, skipped, exit 1 | `apply/takeover.rs:176`: three feed events in order, the relaunch's at the join's generation plus 1, the record in P. `apply/takeover.rs:213`: `fail_next(HerdrOp::Version)`, which only the relaunch calls. That gives `Failed`, then `Skipped`, and the feed holds the leave and the join and nothing after them. The record is the join's. Verb: `assert_stopped` with `(relaunch)` | Met |
| 30 | A per-pane `usage` from the relaunch rules: refused with a named code, the run goes on, exit unchanged | `apply/rows.rs:409`. A moved `host.cwd` gives `Refused` `spec-not-launchable` (C22), then `Adopted`. No write, the server still `Killed`, exit 0 | Met |
| 31 | A dry run calls only its reads | `apply/dry_run.rs:24`, over `every_row()` (rows a to f) with `--herdr-session`. Herdr makes only `Snapshot`, the harness only `Health` and `ShownSession`. The host and prober logs are empty, and neither store's log holds a `CasPut` or a `Delete` | Met |
| 32 | `--herdr-session` is needed for a create alone | `apply/rows.rs:487`. With every pane recorded and no flag, dry runs exit 0 in both formats. The live run relaunches `pane-f` in its record's Herdr session and exits 0 in both formats | Met (advisory 6: one assertion could be tighter) |
| 33 | A failed `health` for a pane that row a refuses: refused, the run goes on, exit 0 | `apply/failures.rs:228`: `Refused` `pane-in-other-profile`, then `Created`, no feed event for `pane-a`, the verb exits 0. If the observation had not consumed the queued failure, the create would have taken it, so the test has teeth | Met |

**Beyond C14:**

| Criterion | Proving test or evidence | Status |
|---|---|---|
| C6, §7: a live run that exits 0 prints `{"profile","dry_run","steps"}` as `data` itself, and the text names every pane | `apply.rs:104` `a_run_that_exits_0_prints_the_steps_object` | Met |
| C8: an error before any plan is the verb's own: `profile-not-found` 3, an unreadable registry `unavailable` 1, a bad name `usage` 2, in both formats, with nothing live | `apply.rs:146` `an_error_before_any_plan_is_the_verbs_own` | Met |
| C21: one thread per call | `apply/observe.rs:123` `a_panes_health_and_shown_session_are_observed_at_once`: each of the two calls finds the other already made. A chained mutant fails it | Met |
| C22: the step vocabulary | `apply/rows.rs:27` checks the six action strings, the exact keys `{pane, action, outcome, code, diff, detail}` and `null` for an absent key. `apply/dry_run.rs:24` checks `planned`, `apply/rows.rs:409` `spec-not-launchable`, and `apply/failures.rs:43`, `:78` and `:185` `observe`. The ADR sentence is checked under Spec compliance | Met |
| C25: an acted step names the reconcile step | `apply/rows.rs:545` covers a `refused` step from `verify_serving`, for a create and for a row f relaunch, in the engine and in JSON and text; row a's detail must not carry it. `apply/failures.rs:275` covers a `failed` acted step, in the O6 message, in both formats | Met |
| Acceptance 1 to 4 and 6 (the issue's own bullets) | Entries 1 to 4 and 31; 5 to 7; 8 to 10; 11; 18 and 21 | Met |

Each test asserts behavior, not implementation. The checks are outcomes, codes, call logs, feed events and printed output,
and the mutation runs (16, then 4, then 5 and 5) show each critical branch has a test that fails without it. The one test
that builds its expected value with the code under test, `diff_spec` in entry 3, also pins the concrete JSON diff.

## Spec compliance

**The operator's §1 to §13**, as implemented:

- **§1.** Records are only the registry's entries that a spec names, in any profile (`tx_apply.rs:246-255`). A spec pane
  with no record is always row b, so a Herdr pane with no record is never adopted (entry 12).
- **§2 and C9.** `observe::judge` (`observe.rs:109-135`) checks these in order:
  - `health`'s `Err` is a failed step;
  - the snapshot's `Err` is a failed step;
  - `Ok(false)` is unhealthy;
  - the snapshot must list the `(session, pane_id)` pair (`listed`, `observe.rs:167-172`). The cell and the label are
    never compared.
  - then C12's shown-session table (`shown`, `observe.rs:141-162`): only `Ok(Some(s))` with `shown_differs` true is a
    mismatch. `Ok(None)` and `Err` add a "could not tell ... not acted on" note to the step's detail (O2).
  - The stored `harness.health` is never read.
- **§3.**
  - `choose` (`tx_apply.rs:355-388`) takes the rows in order: row a, checked before any observation is consulted (C1,
    R5 B-3); b; c and d, by `diff_profile`'s `Matches` or `Differs`; e, from the record's role; f.
  - Steps are sorted by `(workspace, row, col)` with a stable sort (`tx_apply.rs:343-351`).
  - Rows c, d and f do the take-over move (leave, then join) before any live effect (`run.rs:67-136`). Row e never
    writes.
  - Row f calls `tx_launch::relaunch` with `grid_given: false` and `profile: Some(P)`. Under a take-over its record is the
    one the join stored (C11, `run.rs:110-122`).
- **§4.**
  - A refusal from launch or relaunch is `refused` with its code. A `usage` from a step's spec becomes `refused`
    `spec-not-launchable` (`Planned::halted`, `tx_apply.rs:513-534`).
  - The `--herdr-session` gate runs after the two reads and before any observation (`tx_apply.rs:215-218`, `260-285`), in
    a dry run too. A spec whose pane is not a `PaneName` is refused at planning, so it is never a `create` and does not trip
    the gate. That is C22's `refuse` row and §4's "the plan contains a `create`".
- **§5.**
  - There are six outcomes, plus `skipped` and the dry run's `planned` (C22).
  - `until_failed` (`tx_apply.rs:431-444`) stops at the first `failed` step and marks the rest `skipped`.
  - The exit code follows the code's class (`output.rs:338`). A step is `failed` exactly when `class_of` says failure, so a
    failed step always exits 1.
- **§6.** `Planned::dry` is pure, and `run.rs` is never reached by a dry run. Entry 31 proves the call set.
- **§7.** `data` is `Applied`, serialized as `{profile, dry_run, steps}`. Each step is `{pane, action, outcome, code, diff,
  detail}`. `diff` is the whole `PaneDiff`, cloned from `diff_profile`'s row (C6, R6 NIT-1). `error` and `acted` are
  `#[serde(skip)]`.
- **§8.** `NAME`, `--dry-run`, `--take-over`, `--herdr-session NAME`, and the global `--format` (`profile/apply.rs:52-65`).
- **§9.**
  - `Fresh::read` re-reads the record and P before every acting step (`run.rs:149-169`).
  - One thread per call (C21).
  - The bound is in the module doc (C4 below), with the grep test (entry 20).
  - The twins `leave` and `join` (`run.rs:219-240`) match `detach` (`profile/delete.rs:149-155`) and one pane's write of
    `join` (`profile/create.rs:278-282`), line for line. They have no undo and no profile delete.
  - The follow-up is filed: #818 is open, and its body carries C10's "semantically identical" clause.
- **§10.**
  - The engine is in `holler-pane` and works only through `Ports`. The verb is `profile/apply.rs`.
  - `tx_launch.rs` and `tx_launch/` are untouched (`git diff` is empty), as are `reconcile*`, `profile_diff.rs`, `lib.rs`
    and every Cargo file.
  - ADR-0021 defines "adopt" and rewords section 9's apply row. `cli-surface.txt` and ADR 0003 carry the story's lines.
- **§11.** C14's entries cover each item (table above).
- **§12** is the brief's own Evidence section.
- **§13** is superseded by C18: not filed, and no `profile_diff.rs` line edited.

**The decisions on O1 to O6:**

- **O1.** Row f under a take-over does the leave, then the join, then the relaunch. A failed relaunch stays in P with no
  compensation. Its detail says the pane "left profile Q and joined P before the relaunch, and stays in P" (`run.rs:123-133`;
  entry 29).
- **O2.** Implemented as above (entry 28).
- **O3.** The one seeding helper is `serve_server` (`profile_verbs/rig.rs:240-255`), adapted to #802's `serve_command` and
  `start_server`.
- **O4.** A take-over join that fails after its leave landed leaves the pane in no profile. The note names Q and says "is in
  no profile now", or "may be" for a `timeout` or `unavailable` join that may have landed, and no re-join is written
  (`run.rs:184-214`; entry 25).
- **O5.** No overall cap (entry 27).
- **O6.** At exit 1 the verb prints only the failure (`stopped`, `profile/apply.rs:114-134`). The message has the pinned
  form `apply stopped at <pane> (<action>): <step error>; <N> later step(s) skipped`. `<pane>` goes through
  `findings::quoted` because a spec's pane is unchecked text. That is the module's documented rule (A round 1 warn 4), and
  the message still names the pane.

**The C items that leave something for S to check:**

- **C1.** The run's order is: P, the registry, the gate, the observation (all joined before any step), the plan, then a dry
  run's stop or the steps in order (`tx_apply.rs:210-235`).
- **C3.** Applied literally: a snapshot error fails every recorded pane that row a does not refuse, the first in grid order
  stops the run, and an earlier `create` still runs (entry 23). This matches C3's own parenthetical, "a recorded pane that
  row a does not refuse".
- **C4, the bound's doc, audited as the Acceptance section's fifth bullet asks.** `tx_apply.rs:40-51` has the
  `**Bound (I5).**` form that `tx_launch.rs:30-35` uses, and it states each part C4 lists:
  - the two reads (P and the registry), each within I5's per-call bound;
  - the up-front observation, whose calls all run at once, one thread each, so it takes at most one port call's own bound
    (I5's per-call bound, default 10 s). This is "the slowest call" in C21's reading.
  - then each step in turn under its own I5 bound:
    - a create or relaunch is one `tx_launch` run under the caller's `TxOptions` budget (the verb passes
      `DEFAULT_BUDGET`), starting when that run starts, so it takes at most the budget, plus one port call's own bound,
      plus its rollback calls;
    - an adopt or a take-over's move makes at most two record writes;
    - each of those steps re-reads first, two calls;
    - a step refused at planning (rows a and e) calls nothing.
  - "There is no overall cap": N steps take at most 10 s plus N step bounds, beyond the reads (O5).

  All five of C16's strings are there verbatim. C4 says "record and profile writes", and the doc says "record writes". The
  doc is the exact one: C10 gives rows c and d no write of P.
- **C10 and C11.** No write of P and no `edit_spec` call by apply itself. A join after a leave writes the record the leave
  returned, at its generation.
- **C12.** Implemented as the table says. A `None` session of record never fails the check and adds no note.
- **C13.** `apply` takes `&TxOptions` from its caller and hands it to each `tx_launch` run (`run.rs:60`, `:123`). The verb
  passes `TxOptions::default()`.
- **C15.** Row e's refusal is decided at planning. It calls nothing and writes nothing, with or without `--take-over`.
- **C19.** Implemented as O6 above. A gate failure and an error before any plan print only the error (`profile/apply.rs:79-86`).
- **C21.** `observe.rs:4-16` names `reconcile`'s mechanism: scoped `thread::Builder` threads, each joined, and a lost thread
  answered `unavailable` naming its call. It runs one thread per call because a pane's two reads are independent. It no
  longer claims "the shape of `observe_all`". It mentions `observe_all` only to say why that function is not reused.
- **C22, the ADR sentence (ADR-0021 section 8, "Apply as built (#664)", lines 573-583).** It names everything C22 lists:
  - the step object `{pane, action, outcome, code, diff, detail}` in `data.steps[]`, with an absent `code`, `diff` or
    `detail` printed as `null`;
  - the six actions, with `refuse`'s three sources and `observe`'s;
  - §5's six outcomes, plus `skipped`, plus `planned` for a dry run's acting step;
  - that a step's `detail` is inside `data`, so it "does not conflict with section 9's 'the envelope has no `detail`'"
    (line 669);
  - A round 4 warn 4's reconcile step: on a `refused` step at the end of its `detail`, and on a `failed` step in the failure's
    message.

  `Step.acted`'s doc (`tx_apply.rs:122-126`) and the module's Outcomes bullet cover `refused`.
- **C23.** `run.rs:8` cites #818. Nothing else was filed.
- **C24.** The diff against the merge base is 28 paths. They are A's 27 plus `handoff-A-dup.md`, all of them story paths
  (Scope check). T-green ran rustfmt `--check` on the story's files only.
- **C25.**
  - `reconciled` (`profile/apply.rs:94-108`) ends the detail of every `acted` step, `refused` or `failed`, with
    `reconcile_step(Some(P))`. It appends after `"; "`, with `failure_body`'s "already contains" guard.
  - `stopped` builds the O6 message from that detail, so the guard exists once.
  - The rigs seed servers through `serve_command`, and a relaunch's record has `pid: None` (entry 16).
  - Entries 26 and 27 were re-derived, and T states the numbers.

No silent deviation found. Everything the brief left open, F settled in a documented decision (`handoff-F.md`, "Design
decisions"), and each fits the brief:
- a re-read that finds the record or the spec gone is refused `pane-not-found` or `pane-not-in-profile`;
- a non-failure-class observation error becomes `unavailable`, so a failed step always exits 1;
- the reconcile step is added in one place.

## Quality audit

- **Correctness and failure handling.**
  - **Fail-closed.**
    - A registry or P that cannot be read is the verb's own error, before any step (C8).
    - A `health` or snapshot error fails the step and stops the run (entries 17, 21 and 23).
    - A thread that cannot start or panics becomes `unavailable` naming its call (`observe.rs:93-104`).
    - An observation error of another class becomes `unavailable`, keeping exit 1 consistent with the envelope's code
      class (`observe.rs:174-190`).
  - **No lost writes.**
    - Every write apply makes itself is a compare-and-swap at the generation it re-read.
    - A conflict is a `failed` step (entries 22 and 25).
    - Nothing compensates (O1, O4).
    - The one residual window is in `tx_launch`'s `edit_spec`, which §10 forbids editing (advisory 2).
  - **Planning.** Row a is decided before any observation is looked at (entry 33). The zip of P's specs with
    `diff_profile`'s rows is one to one, because the records are filtered to spec names, so no `Extra` row exists.
- **Build guards.**
  - No `unwrap`, `expect`, `panic!`, `todo!` or `dbg!` in the story's `src` files (grep).
  - No added `#[allow]` or `unsafe` anywhere in the diff.
  - Sizes: `tx_apply.rs` 550, `observe.rs` 190, `run.rs` 240, `profile/apply.rs` 173, `show.rs` 202. The tests are at most
    759 (`apply/rig.rs`, in lint's 600-line warning band). No file is at or above 900.
  - No dead code: every private function has a caller, and clippy `-D warnings` is clean.
- **Contract.**
  - No wire protocol change, so `docs/protocol/v2.md` and the golden files are untouched (no golden file is in the diff).
  - The verb's JSON `data` is new, since the verb was a stub.
  - The two new open codes are `RefusalCode::from_static` constants (`tx_apply.rs:71-78`) and are listed in ADR-0021
    section 9's apply row, which is reworded to the doctor form. No closed code was added.
- **Tests.**
  - In-process, over the test kit's fakes, as the brief requires. Nothing here is cross-process, so no hub or body harness
    is needed.
  - No test sleeps to synchronize. The delays are the fakes' `set_delay`, which C13 and E59 prescribe, and they bound or
    overlap calls rather than wait for an event (advisory 5).
  - RED-first evidence is in both T-red handoffs (T precondition).
- **Documentation.**
  - `CHANGELOG.md` has an `## [Unreleased]` / Enhancements entry that links #664, #633 and #649, and it says the real
    binary answers `not-implemented` until #649. That is accurate: `pane/wiring.rs` hands out `Unwired`, and apply's first
    read is `profile_store.get`.
  - ADR 0003's row, ADR-0021 ("Apply as built (#664)" with "adopt" defined against the ADR's "never adopted" sentences, and
    the section 9 row), `cli-surface.txt`, and one inline span in the spike doc are updated.
  - No README or `docs/` page lists the profile verbs, so the ADRs are their CLI documentation, as for #662.
  - No new log event.
- **Public-repository privacy.** I grepped every added line of the diff, handoffs included, for:
  - personal and account names, hostnames, tailnet and `.ts.net`, `.local`/`.lan`/`.internal`;
  - private IP ranges and home-directory paths;
  - key, token and secret patterns.

  The only hits are `127.0.0.1` (a test probe URL), the public model name `deepseek-v4-pro`, mutant labels, and GitHub issue
  links. Nothing to fix.
- **Commit and PR hygiene.**
  - Every commit subject is Conventional (`chore(#664): ...`), and every commit carries a `Co-Authored-By` trailer.
  - No PR exists yet (`gh pr list --head issue-664-implementation` is empty), so the AI disclosure is the PR step's
    (advisory 7).

## Scope check

**Delivered as the brief asks.** The engine and its two child modules, the verb, the ADR-0021 and ADR 0003 edits,
`cli-surface.txt`, the CHANGELOG, the tests and rigs, and the doc work of C21 to C25.

**Outside the original Blast radius bullets.** Each path is on A's 27-path list, explained in `handoff-F.md`, and accepted by
A-dup expectation 9:
- `tx_apply/{observe,run}.rs`: the house overflow pattern (`tx_launch/server.rs`), which C21 and C23 name;
- `profile_verbs/apply.rs`: #637's test stub, the root of `apply/**`;
- `profile/show.rs`: two helpers go to `pub(crate)` for reuse, with no behavior change;
- `pane_verbs/process/stub.rs`: the story's own `STUBS` line, deleted by T because the verb is no longer a stub;
- `docs/research/herdr-api-spike.md`: one inline span gains `NAME`, without which `docs_cli_test` fails;
- `CHANGELOG.md`: `changelog-check.sh` requires it.

**Neither over- nor under-delivered.** There is no unrelated refactor, and no shared engine changed. §13's follow-up is
correctly not filed (C18), and #818 is filed and cited (C23).

## Verdict

**PASS.** Every C14 entry and every check beyond it has a proving test that asserts behavior. The brief's decisions (§1 to
§13, O1 to O6, C1 to C25) are implemented as stated. The module doc states the bound in C4's form. Quality, documentation
and privacy are acceptable. Ready for O, with item 1 of the notes below to do before the PR merges.

## Notes for O, and advisory notes (non-blocking)

1. **C18's order clause has flipped: merge `origin/main` before the PR merges.**
   - **What happened.** #644 part 2 (`b8242d7`, PR #822) merged at 11:45 MDT. That is after T-green and A-dup, and before
     this story's PR, which does not exist yet. It adds `SpecField::OpencodeAgent` to `SpecField::ALL`. C18 therefore takes
     its first branch: "this story merges `main` and T adds that test (a healthy pane whose agent differs is `reported`,
     nothing written)". #644's own brief left apply's test to #664 (its item 7).
   - **The test exists, and holds before and after the merge.** It is `pane-g` in entry 11 (`apply/dry_run.rs:119`), T's
     answer to A round 4 warn 2. After the merge, `profile show` reports `pane-g` as `differs` on `opencode_agent`. The
     test then requires the plan's step to carry that same `PaneDiff`, and only row d's `reported` step carries a diff. It
     is a dry run, so nothing is written. Entry 3 proves that row d writes and touches nothing in a live run.
   - **The merge itself.** It is textually clean (`git merge-tree` exit 0). The shared files are `CHANGELOG.md`,
     `ADR-0003.md` and `ADR-0021.md`. The story's code names neither `SpecField` nor `FieldValue`, so #822's new
     `FieldValue::Key` cannot break it to compile. `tx_launch` records the spec's agent (`tx_launch.rs:404`), and every
     fixture spec comes from `spec_from_pane` or `sample_spec` (agent `None`), so the other apply tests keep matching.
   - **Still to do.** No test of this story has run on a tree that has #822. Before the merge:
     - merge `origin/main` into the branch (CI's `pull_request` run also tests the merge ref);
     - re-run the verify command and `changelog-check.sh`;
     - confirm `a_dry_run_and_profile_show_agree` passes with `pane-g` as `differs`;
     - have the PR body say how C18 resolved: the agent case is covered by entry 11 on the merged tree, and there is no
       follow-up.
   - **A stale comment, harmless.** The first clause of the comment at `dry_run.rs:114-117` ("while `SpecField::ALL` omits
     `opencode_agent`") then describes the earlier state.
2. **Brief defects with no effect on the code, and an open point that stays the operator's.**
   - E40's second excerpt is empty and cites `ADR-0021.md:530`; the row was at 620 on `7284001` (A round 4 warn 1).
   - The Blast radius does not list the 27 paths (A round 4 warn 3).
   - C5's sentence "a writer that gets in between that re-read and the step's own write makes that write conflict" holds
     for apply's record writes, but not for P. `tx_launch`'s `edit_spec` re-reads P itself, so a P edit that lands between
     apply's re-read and that read is overwritten by `SpecEdit::Set`. `handoff-F.md` lists this as "the stale-P window",
     `pane relaunch --profile` has the same window, and C23 keeps it open for the operator.

   Entry 19, as A round 1 warn 5(a) scoped it, is met. None of this is an ADVISORY-HOLD, because the brief decides each
   point and the code follows it.
3. **F's Known issues stand for the operator.**
   - A row is chosen once, so a row-f pane that recovers before its step is still relaunched.
   - Duplicate spec panes each get a step.
   - An acted refusal leaves its tmux session behind, and for a row f relaunch, no server. The step's reconcile step points
     the operator at it.
4. **A-dup's two warns are follow-ups for O to file, or to fold into #818.**
   - `observe.rs` copies `joined`, the three call-name constants, and a third spelling of the `(session, pane_id)` join
     key from `reconcile` and `tx_launch`.
   - `apply/rig.rs` re-creates about 176 lines of `pane_verbs/launch/rig.rs`, and the copy already differs on `pid`.
5. **The timing-dependent tests** are the C21 overlap test (a 400 ms injected delay against thread start-up), entry 26 (2 s
   of margin under its 3 s bound) and entry 27 (about 400 ms of room in a 1200 ms budget). The outside diff gate's NV-2
   says the same. T ran them 10 of 10 under load. If CI ever flakes, widen the margins rather than add retries.
6. **Edges no criterion names, so they are untested, and acceptable as they are.**
   - A spec whose pane is not a `PaneName` is refused `spec-not-launchable` at planning and stays outside the gate (C22).
   - A non-take-over step whose record moved to another profile during the run is refused `pane-in-other-profile`.
   - If P is deleted during the run, each later step is refused `profile-not-found`, and the run exits 0.
   - A refusal-class `health` error is turned into `unavailable`.
   - `reconciled`'s "already contains" guard is unreachable today.
   - Optional tightening: entry 32's test does not assert C7 R5 B-1's "no step is refused for the flag" for `pane-c`. One
     line would pin it: `json_step(&data, "pane-c")["outcome"] == "adopted"`.
7. **The PR step.**
   - After the script opens the PR, add the AI disclosure to its body, per `CONTRIBUTING.md` and this repo's `CLAUDE.md`
     (`gh pr edit`), together with item 1's C18 sentence.
   - The phase commits carry the `prepare-commit-msg` hook's `Co-Authored-By: Claude <noreply@anthropic.com>` with no
     session link, the same as the merged #802 (`7284001`) and #644 (`b8242d7`). The squash message should keep the
     trailer.
