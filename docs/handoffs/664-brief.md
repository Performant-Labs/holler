# [feat(cli): holler profile apply (plan as JSON, --dry-run, adopt not kill)](https://github.com/Performant-Labs/holler/issues/664)

Part of epic #633. (added 2026-10-08, profiles) **REQUIRED (promoted from PROPOSED by the operator, 2026-10-08): the final operator-run story #666 uses `apply` to recreate the live workspace from profiles.** Not on the critical path: it can start once its dependencies merge, and only #666 (after #654) waits for it. (amended 2026-10-08, review) #652 and #667 (the real-adapter scenario for apply) now also wait for it.

## Scope
- `holler profile apply NAME [--dry-run]`: declarative, idempotent convergence of the live panes to the profile. Create a missing pane (through the launch transaction of #644); **adopt** a healthy live pane that matches its spec (record it, set `Pane.profile`); **never kill a healthy pane**; report anything it will not change. `--dry-run` prints the plan as JSON (the envelope's `data` is the list of planned steps) and changes nothing. This is the cure for a reboot that returns the layout as empty shells. (amended 2026-10-08, review) Apply **refuses a pane that belongs to another profile unless `--take-over`** is given (confirmed by the operator, 2026-10-08), and lists such panes in the `--dry-run` plan. (amended 2026-10-08, features) Before each pane it creates or relaunches, apply runs the pane's probe (`run_probe`) and refuses that pane with `probe-failed`; it places panes through `HerdrPort::ensure_pane`, which reaches the `GridPos` by splits (#640), and **never relocates a healthy pane**.
- The plan is built from the spec-versus-live comparison in `crates/holler-pane/src/profile_diff.rs` (#662, which `holler profile show` uses). There is no separate `holler profile diff` verb (dropped by the operator, 2026-10-08): `show` reports the differences and `apply --dry-run --format=json` prints the exact plan.
- `apply` takes `--format=text|json`; positions are `GridPos`, printed `rRcC` / `{"row", "col", "pos"}`. (amended 2026-10-08, grid)
- (added at the brief gate, 2026-10-11, for review findings R3 B-3, R3 W-1 and R3 NIT-1, and tightened for R4 B-1, R4 NIT-2 and R6 B-1; it restates §4, §7 and §8 of the operator's section below and the decision on O6, which decide it, and changes no bullet above) `apply` also takes `--herdr-session NAME` (as `pane launch` has it) and `--take-over`. **When the plan holds a `create` (a spec pane with no record: row b of §3) and `--herdr-session` is absent, the whole verb exits 2 (`usage`) before any step runs, in a dry run too.** In that case `--dry-run` prints **no plan at all**, not even the steps of the recorded panes: its whole output is the usage error, and it exits 2 (C1). There the first bullet's "`--dry-run` prints the plan" gives way to §4, since the operator's section supersedes this Scope where they differ (its opening paragraph); the first bullet's text stays as the issue wrote it. A dry run prints a plan only past that gate, that is, with `--herdr-session` given or with every spec pane recorded (C1), and, by the decision on O6, only when no step of its plan is `failed` (the Acceptance section's sixth bullet; R6 B-1). With `--format=json`, the envelope's `data` is §7's object `{"profile", "dry_run", "steps"}` itself, not nested inside another field, and the first bullet's "list of planned steps" is its `steps` (C6, E61).

## Acceptance (test kit only (amended 2026-10-08, review); the real run is story #667 (amended 2026-10-08, review))
- Apply twice changes nothing the second time; a healthy matching pane is adopted, not relaunched; a healthy pane that differs is reported, not killed; `--dry-run` writes nothing and its plan parses against the envelope.
- (amended 2026-10-08, review) A pane of another profile is refused without `--take-over` and adopted with it; the dry-run plan lists it.
- (amended 2026-10-08, features) A failing probe refuses that pane with `probe-failed` and the others proceed; against #638's split-only fake, a missing pane is created by splits; a healthy pane in the wrong cell is reported, never moved.
- `apply --dry-run` and `holler profile show` agree on the same fixture (the plan lists exactly the differences `show` reports, positions in rowcol).
- (added at the brief gate, 2026-10-11, for review finding R3 B-2; it removes and narrows nothing above) **The acceptance is one list:** the bullets above, the tests of §9 and §11 of the operator's section below, and every test C7 adds to §11. The story is accepted only when every one of them passes on the test kit's fakes and S's audit finds the run's bound in the module doc of `tx_apply.rs` in C4's form. C14 indexes that whole list, one entry per test, with the source of each.
- (added at the brief gate, 2026-10-11, for review finding R6 B-1; it removes and narrows nothing above, and restates §4, C1 and the decision on O6) **What `--dry-run` prints when it prints no plan.** When §4's `--herdr-session` gate fires (the plan holds a `create`, that is, a spec pane with no record, and the flag is absent), `apply --dry-run` prints **no plan**, not even the steps of the recorded panes: with `--format=json` its whole output is one failure envelope coded `usage`, whose `data` is `null`, and in text mode the usage error goes to stderr and nothing to stdout; it exits 2, writes nothing and makes no Herdr or harness call (C1; C14 entry 18). When a step of its plan is `failed`, it prints only the failure, no plan, and exits 1 (the decision on O6; C14 entry 21). With `--format=json`, either failure envelope parses against the envelope (E62: on failure, `data` is `null`), so the output of every dry run parses against it, as the first bullet asks; a dry run prints a plan exactly when it exits 0 (C19).
- (added at the brief gate, 2026-10-11, for review findings R6 B-2 and R6 W-2; it removes and narrows nothing above) **When "passes" is judged.** The fifth bullet is judged at the end of the story, on the code phase F leaves: every test it lists passes then, and S audits the doc then. Phase T writes each of those tests before F writes any code and confirms it RED against the stubs, then confirms it GREEN after F (Rules for this story: tests first; the pipeline's phase order, T author/RED, F, T verify/GREEN). That RED check is how the pipeline orders the work, not an acceptance criterion or a deliverable of the story, and no test has to pass against a stub. "On the test kit's fakes" describes the tests of what apply does; entry 20's test (C14, C16) reads `crates/holler-pane/src/tx_apply.rs` itself and needs no fake, and it passes once F has written the module doc C4 describes.

Depends on: #637, #638, #644, #662. (amended 2026-10-08, review) (Was #640, #641, #642, #644, #647, #661, #662: the adapters and the registry are faked here.)

## Blast radius
- crates/holler-cli/src/profile/apply.rs (stub pre-created by #637)
- crates/holler-pane/src/tx_apply.rs (stub pre-created by #637)

## Scheduling
Wave 5 (the last development wave), beside #652 and #665; all its dependencies are in wave 3 or earlier, so it runs in parallel with #652 and adds no serial step. The cutover #654 does not wait for it; the final operator-run story #666 (after #654) does. (amended 2026-10-08, review) **Now wave 4**, beside #648, #649, #650, #653 and #665; #652 and #667 (wave 5) wait for it. (amended 2026-10-08, review)

## Pipeline
`rigor: second-opinion` (the outside model runs the brief and diff gates; the operator's choice, 2026-10-11).

## Rules for this story

See the epic (#633): the contract section there is fixed, so this story builds against it without waiting for another story's code. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt`, clippy clean, no new `unsafe`). Edit only the files in the Blast radius; the shared hot spots in the epic are owned by one story each. No story touches a live fleet, a running pane or a real Herdr session; verification is the fakes from the test kit and temporary directories. Never print or commit a credential.




## Operator-side decisions and resolved review points (2026-10-11) — final, complete, and overriding any earlier wording above

Two earlier runs of this story stopped at review. This section is the whole contract for `holler profile apply`; it supersedes the "Scope" bullets above where they differ and replaces an earlier version of this section.

**1. What apply looks at.** Only the **registry** (the pane records) and **live observation** (the Herdr snapshot, the harness `health`, the shown session) per I6. A live Herdr pane or tmux session **with no record** is never adopted by inference; `holler pane import` (#650) is the way to record existing panes. "No record" means: no registry entry named like the spec pane.

**2. "Healthy" is a live observation, never the stored `harness.health`.** A recorded pane is healthy when **all** hold: `HarnessPort::health(port)` returns `Ok(true)`; the Herdr snapshot lists the recorded `pane_id`; and, when the shown session can be read, it equals `session_of_record` through `reconcile::shown_differs` (an unreadable shown session does not make a pane unhealthy). `health` returning `Ok(false)` means unhealthy; `health` returning `Err(_)` is a **failed** step (see 5), never "healthy".

**3. The step table.** One row per spec pane, in `(workspace, row, col)` order; the first row that matches applies:

| # | Situation | Step | Live effect | Registry effect |
|---|---|---|---|---|
| a | the pane has a record that belongs to **another profile**, and no `--take-over` | `refused` (`pane-in-other-profile`) | none | none |
| b | **no record** | `create` through the launch transaction (it refuses an occupied cell or an existing session through its own checks: that is a `refused` or `failed` step per 5) | launch's | launch's |
| c | record, **healthy**, matches its spec (`diff_spec` empty) | `adopt` | none (never touched) | set `Pane.profile` to P if not already (with `--take-over` from another profile: leave then join, two writes) |
| d | record, **healthy**, **differs** from its spec | `reported` with the `PaneDiff` | none (apply never kills or rewrites a healthy pane; the operator uses `launch`/`relaunch --profile P`) | with `--take-over` from another profile: leave then join, otherwise none |
| e | record, **not healthy**, role **orchestrator** | `refused` (`orchestrator-not-relaunched`) | none | none, **even under `--take-over`** (no profile writes happen for a refused pane) |
| f | record, **not healthy**, role agent | `relaunch` through `tx_launch::relaunch` (`grid_given: false`, `profile: Some(P)`) | relaunch's, **after** the profile move below | with `--take-over` from another profile: **leave then join first (two writes, exactly as rows c and d), then the relaunch**, which then sees a record that already names P. `tx_launch::relaunch` is called unchanged and never refuses `pane-in-other-profile` for this step, because the record no longer names another profile when it runs. If the relaunch then fails, the pane stays in P (the intended end state) and the step is `failed`; apply does not compensate the profile move. Without a take-over (the record already names P, or none) there is no profile move, and the relaunch's own write sets `Pane.profile` to P |

Row order matters: row a is checked first and fires only for another profile **without** `--take-over`; with `--take-over` the pane falls to rows b to f, and rows c, d and f add the profile move described in the table, **always before any live effect of the same step**; row e never writes.

**4. Per-pane answers outside the table.** A step that the launch transaction answers with a **refusal** (exit class 3, for example `probe-failed`, `pane-exists`) is a `refused` step carrying that code. A step answered with `usage` (exit class 2) for a per-pane reason (a relaunch's refusal to move a pane when the spec's cell or directory differs from the record's, a spec whose port policy is not `fixed:<port>`) is also a `refused` step carrying a named code, not a run failure. **`create` without `--herdr-session`** is checked **before any step runs**: if the plan contains a `create` and the flag is absent, the whole verb exits 2 (`usage`) and no step runs, in a dry run too.

**5. The exhaustive outcome set and what each contributes.** Every step ends in exactly one of: `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`.
- `refused` and `reported` do **not** stop the run and do **not** change the exit code.
- `failed` is any failure-class answer (exit class 1: `unavailable`, `timeout`, a failed `health` call, a store conflict) and **stops the run at that step**; steps after it are not attempted and are listed as `skipped`.
- Run exit code: **1** if any step is `failed`; **2** for the up-front `usage` case in 4; otherwise **0**. This is the doctor form (ADR-0021 section 9's apply row is reworded to say so).

**6. `--dry-run`** prints the same table as the plan and changes nothing: it runs no probe, issues no split, writes nothing, and performs no health call that has a side effect; it does the read-only health and snapshot observations needed to choose each row.

**7. `data`** is the object `{"profile": NAME, "dry_run": bool, "steps": [{pane, action, outcome, code?, diff?}]}`, embedding `PaneDiff` unchanged.

**8. Flags:** `--herdr-session NAME` (as `pane launch` has it), `--take-over`, `--dry-run`, `--format`.

**9. Transactions:** re-read the record and the profile before each step so a stale spec is never written back through `edit_spec`; health is observed for all panes up front, one thread per pane (I5), and the run's time bound is recorded in the module doc of `crates/holler-pane/src/tx_apply.rs` (already in the Blast radius): the up-front health observation runs under one I5 bound (default 10 s); each step then runs under its own I5 bound (the `TxOptions` budget `tx_launch` uses); there is no overall cap, so a run of N panes is bounded by 10 s plus N step bounds. A test asserts the doc sentence exists (a `grep`-style check over the file) so the bound cannot be dropped silently. The leave and join profile writes exist only in `holler-cli` (`profile::delete::detach`, `profile::create::join`): add private twins in `tx_apply` and write the justification in the brief; file a follow-up issue to merge them.

**10. Placement and files:** `tx_apply` in `holler-pane` works only through `Ports`; the verb is `crates/holler-cli/src/profile/apply.rs`; `tx_launch` is called unchanged (note: #802 changes the one `serve` call inside it; do not edit that line here). Add `crates/holler-cli/tests/profile_verbs/rig.rs` and `crates/holler-cli/tests/profile_verbs/apply/**` to the Blast radius, plus `docs/adr/ADR-0021.md` (define "adopt"; reword the apply row of section 9) and the story's own lines in `cli-surface.txt` and ADR 0003.

**11. Tests** on the test kit's fakes, one per row and outcome: a (refused without `--take-over`); b (created; an occupied cell refused through launch; and a `create` answered by launch with `pane-exists` is a `refused` step carrying that code and the run continues); c (adopted, no live call); d (reported, nothing live touched; and with `--take-over` the profile moved and nothing live touched); e (orchestrator not healthy: refused, no registry write, also with `--take-over`); f (killed server relaunched); a failed `health` call is a `failed` step that stops the run and marks the rest `skipped`; `probe-failed` is a `refused` step and the run continues; the up-front `usage` case exits 2 with no step run; the dry run runs no probe and writes nothing; a stale spec is not written back.

**12. Evidence for the brief:** the reviewers cannot see the repo, so the brief's evidence section must quote the signatures it relies on (`diff_spec`/`diff_profile`, `HarnessPort::health`, `tx_launch::relaunch` and its `grid_given`/`profile` parameters, `reconcile::shown_differs`, the fakes' Herdr snapshot) from `crates/holler-pane/src/{profile_diff.rs,ports.rs,tx_launch.rs,reconcile.rs}` and `crates/holler-pane-testkit/src/herdr.rs`.

**13. Known limitation, not this story:** `SpecField::ALL` omits `opencode_agent` (#700 recorded it as a non-goal), so apply cannot see a pane running the wrong agent. File a follow-up issue; do not widen this story.

## Brief-gate clarifications (rounds 1 to 6, 2026-10-11)

Added at the brief gate for the outside review's findings (the B-, W-, NV- and NIT- numbers are the reviewer's). The
reviewers of rounds 2 to 6 numbered their findings from 1 again, so they carry an R2, R3, R4, R5 or R6 prefix here (R2
NV-2, R3 B-1, R4 B-2, R5 B-1, R6 B-1 and so on); a number without a prefix is round 1's. §N is paragraph N of the
operator's section above, C*n* and O*n* are items of this section, and E*n* is an excerpt in the Evidence section below.
Each item spells out what §1 to §13 and the decisions on O1 to O6 already imply, adds a test to §11's list, indexes the
tests (C14), or names the test that checks a requirement on code this story has not written yet (C16, C17). None of
them changes, narrows or overrides §1 to §13; wherever an item and §1 to §13 could be read differently, §1 to §13 win,
except where a decision below narrows one of them itself: the decision on O6 narrows §6's "prints the same table as the
plan" to runs that do not fail (C19; R6 B-1, R6 B-3).
What only the operator can decide is listed under "Open for the operator" and is not decided here.

**C1. The order of a run (W-2, W-3, R2 W-3, R5 B-1, R5 B-3, R6 B-1, R6 W-3; from §3, §4, §5, §6 and §9, and the decision
on O6).**

1. Read the registry and P.
2. Apply §4's `--herdr-session` gate: when the flag is absent and a spec pane has no record, the verb exits 2 and no step
   runs, in a dry run too. A spec pane with no record is row b, a `create`, since row a needs a record (§3), so whether
   the plan holds a `create` is known from 1 alone, and the gate answers here as it would after 4. The gate's condition
   is exactly row b's (R3 W-3): a spec pane for which the registry holds no entry of its name (§1). A spec pane whose name
   has a record never trips it, whatever profile that record names (P, another profile or none) and with or without
   `--take-over`, since such a pane is row a or one of rows c to f, never row b.
3. Make every observation §2 needs, up front (§9): the Herdr snapshot and, for each spec pane that has a record, its
   `health` and its shown session, one thread per pane (I5). All of them finish before 4.
4. Choose every pane's row (§3). This is the whole plan.
5. A dry run prints the plan and stops (§6), unless a step of the plan is `failed` (C2, C3): then it prints only the
   failure, no plan, and exits 1 (the decision on O6, which narrows §6 to runs that do not fail; R6 W-3).
6. Otherwise the steps run one at a time, in §3's order, each after re-reading its record and P (§9). A step that ends
   `failed` stops the run: the steps after it are `skipped` in the result `tx_apply` returns, and the verb prints only
   the failure and exits 1 (§5; the decision on O6; C19).

The gate runs before the observations (R2 W-3) because nothing it decides depends on them. A verb that exits 2 there makes
no Herdr or harness call, so a wedged port cannot hold a usage error back for up to I5's bound (E4), and it is the order
`tx_launch::launch` keeps, which answers a missing `--herdr-session` after one registry read, its `pane-exists` check
(`pane_store.get`), and before `run`'s plan reads, which make its first prober, Herdr and harness calls (E14, E15;
R5 B-1). §4's "before any step runs" holds either way.

A dry run meets the gate exactly as a live run does (R4 B-1). When the gate fires, the verb prints the usage error and no
plan, not even the steps of the recorded panes, and exits 2: in JSON the envelope is a failure coded `usage`, whose `data`
is `null` (E61, E62), and in text the error goes to stderr and nothing to stdout (E61). The first Scope bullet's
"`--dry-run` prints the plan" applies only past the gate (the Scope's last bullet): §4 decides it, and the operator's
section supersedes the Scope where they differ. Past the gate it applies, by the decision on O6, only to a dry run whose
plan has no `failed` step (step 5). The Acceptance section's sixth bullet states both (R6 B-1).

(R5 B-1) The gate is the verb's one check of `--herdr-session`. Its condition is step 2's, which is §4's "the plan
contains a `create`": a `create` is row b's step, and a spec pane is row b exactly when it has no record (§3).
`tx_launch::launch` keeps a check of its own (E14: after `pane-exists`, `usage` when the request's `herdr_session` is
`None`), which this story leaves as it is (§10) and which is not a second gate on the verb, because apply never trips it:
of the steps step 4 chooses, a `create` alone calls `launch`; apply builds each `create`'s `LaunchRequest` with the
flag's value as its `herdr_session` (E13); and a plan that holds a `create` gets past the gate only when the flag is
given. A spec pane that has a record meets neither check, since none of rows a and c to f calls `launch`: rows a and e
call no transaction, rows c and d only write through the twins (§9, C10), and row f calls `relaunch`, whose request has
no `herdr_session` field and which takes the Herdr session from the record (`old.herdr.session`; E13, E14). So a run
whose spec panes all have records needs no `--herdr-session`, live or dry, and no step of it is answered `usage` for the
flag; C7's R5 B-1 test checks it with a row f pane.

So no step runs while an observation is outstanding, and a failed observation is known before the first step runs. The
step it fails is still that pane's own, for a pane that row a does not refuse (row a is checked first, §3): the steps
before it run, and the steps after it are `skipped` (§5).

(R5 B-3) A pane that row a refuses is a recorded pane, so step 3 observes it like every other (C7: one `health` call for
each spec pane that has a record), but nothing observed there can change its row. Row a is checked first, and its
condition, a record that names another profile with no `--take-over`, is known from step 1's reads (§3). So step 4 gives
it row a whatever its observations answered, an `Err(_)` from its `health` included: its step is `refused`
(`pane-in-other-profile`), not `failed`; it adds nothing to the exit code; and the run goes on (§5). §2's "a failed step"
is the outcome of a pane whose row needs its health, one of rows c to f, as C3 has it for the snapshot ("Rows a and b need
no snapshot"). The call that answered is a read: `health` asks whether the server on the port answers (E7; the OpenCode
adapter's is a timed `GET /global/health`, E50; the fake's answers from the server's state, E25), so observing a pane that
row a then refuses changes nothing and costs one call inside step 3's parallel observation, under its I5 bound (C4).
C7's R5 B-3 test checks it.

**C2. A failed `health` call in a dry run (B-2; from §2, §5 and §6, and the decision on O6).** §6 makes no exception to
§2 and §5, so a dry run treats a failed observation as a live run does: a `health` call that answers `Err(_)` makes that
pane's step `failed` (§2), unless row a, which is checked first, refuses the pane (§3); the plan `tx_apply` returns for
the dry run lists the step as `failed` and every step after it as `skipped`, and the verb exits 1 (§5). The dry run
still runs no probe, issues no split and writes nothing (§6). `health`'s error is a plain `PaneError` (E7), so
"read-only" in §6 describes the calls, not a kind of error: they are the reads §2 needs to choose each row (`health`,
the Herdr `snapshot`, `shown_session`). A dry run calls neither `tx_launch::launch` nor `tx_launch::relaunch`, since
their runs probe, split and write (E10, E15), all of which §6 forbids. (R6 B-3, R6 W-3) By the decision on O6, the verb
then prints only the failure, no plan: with `--format=json` the failure envelope, whose `data` is `null` (E61, E62), and
in text mode one `error:` line on stderr and nothing on stdout. C7's B-2 test reads the `failed` and `skipped` steps
from the result `tx_apply` returns (C19).

**C3. A Herdr snapshot that cannot be read (W-1; from §2, §3 and §5).** §2's "healthy" needs the snapshot for every
recorded pane. A `snapshot` call that answers a failure-class error (`unavailable`, `timeout`: E42) is one of §5's
failure-class answers, so it is a `failed` step: the first step, in §3's order, whose row needs the snapshot (a recorded
pane that row a does not refuse). The run stops there and the steps after it are `skipped` (§5). Rows a and b need no
snapshot. A dry run does the same (C2).

**C4. The engine's doc and the run's bound (B-4; from §9 and §10).** The engine is `tx_apply` (§10), one of
`holler-pane`'s transaction engines (E36), and "the engine's doc" is the module doc comment (`//!`) at the top of
`crates/holler-pane/src/tx_apply.rs` (E33). That file is already in the Blast radius above, so no file is added. The doc
records the run's bound in the form `tx_launch.rs` records its own ("**Bound (I5).**", E10; ADR-0021 section 12, E43): the
most one run takes, in I5's per-call bound (default 10 s, E4, E35) and in `tx_launch`'s run bound (`DEFAULT_BUDGET` plus
one port call's own bound plus its rollback calls, E10, E11). Under §9's structure that is:

- the reads of the registry and of P;
- the up-front observations: the Herdr snapshot, and the slowest pane's chain, since the panes are observed in parallel;
- then, for each step in turn, its re-reads (§9) and its own calls: one `tx_launch` run for a `create` or a `relaunch`,
  the record and profile writes of an `adopt` or of a take-over, and none for a step that row a or row e refuses.

This brief sets no budget over the whole apply run, so the bound grows with the number of steps, and the doc states it per
step. Whether the run should also have a budget over the whole run was open for the operator (O5, R2 W-2), and the
decision on O5 is none. Acceptance: S's audit checks that the module doc of `tx_apply.rs` states the bound in this form.
C13 adds tests that the engine keeps this structure, with no budget over the whole run as O5 decides (R3 B-4). That doc
does not exist yet: E33 is the whole file today, the three-line stub #637 declared, and phase F writes the doc after T's
tests. So the brief cannot quote it, and E10 is quoted as the form it follows, not as `tx_apply`'s own text (R4 NV-1).

(R6 NV-2) §9 says "I5 bound" of two things, which the code it cites keeps apart. A step's "own I5 bound" is "the
`TxOptions` budget `tx_launch` uses" (§9): `tx_launch`'s `Budget`, one deadline per run, started when the engine is
entered (E14) and compared with the elapsed time before each live step, with no port call (E73, E75). A check never cuts
a call short, so a run takes at most the budget plus one port call's own bound plus its rollback calls (E10). A port
call's bound is I5's per-call bound: every port method returns within it (default 10 s) or with `timeout` (E4), which is
how ADR-0021 section 12 has I5 met until an operation id exists (E54), while I5 itself states a verb's bound (E35). §9
names no budget for the up-front observation, and C4 reads its "one I5 bound (default 10 s)" as that per-call bound,
kept for every pane at once because the `health` calls run in parallel, "one thread per pane (I5)" (§9): C4 counts the
observation in per-call bounds, the Herdr snapshot and the slowest pane's chain, and names no budget for it.

**C5. Store conflicts (NV-9; from §5 and §9).** The stores fence writers by compare-and-swap on a generation: nothing
claims a pane before acting, and a conflict is not retried (E5, E8, E28, E37). A write at a stale generation answers
`generation-conflict` (a pane record, or P's first write), or `profile-conflict` when P moved after the act (E8); both are
failure class, exit 1 (E42). So a store conflict is a `failed` step that stops the run, and the steps after it are
`skipped` (§5). The re-read of §9 gives each step the current record and P. A writer that gets in between that re-read
and the step's own write makes that write conflict, which is how a stale spec is never written back.

**C6. `data` and the dry-run output (NIT-1, NIT-2, R6 NIT-1, R6 B-3; from §3, §6 and §7, and the decision on O6).** In
§7, `pane` is the spec's pane name (`ProfileSpec.pane`), the same string `PaneDiff.pane` holds (E1, E3). `steps` has one
entry per spec pane, in §3's `(workspace, row, col)` order. `diff_profile` lists its rows in the profile's own order
(E3), so apply orders its steps itself. §7's step object names no position field; a cell that differs from the spec
shows inside `diff` as one of its field differences (E1, E2). A cell that differs is a `FieldDiff` on `herdr.grid` whose
`spec` and `live` are `FieldValue::Grid`s, which serialize through `GridPos`'s own `Serialize` as
`{"row":R,"col":C,"pos":"rRcC"}` and print as `rRcC` (E56): the two forms the Scope's third bullet names (R3 NIT-2). A
workspace that differs is a `FieldDiff` on `herdr.workspace` with text values (E56). In §6, "the same table" is §3's
step table: a dry run's output is the plan, one step per spec pane chosen by that table, and a dry run prints it exactly
when it exits 0 (C19). With `--format=json` it is §7's `data` with `"dry_run": true`, the "exact plan" of the Scope's
second bullet. The Scope's two wordings of that output, "the envelope's `data` is the list of planned steps" (first
bullet) and "`apply --dry-run --format=json` prints the exact plan" (second bullet), are issue #664's own text, which this
brief carries unchanged (R2 NIT-1, R2 NIT-2). The operator's section supersedes the Scope where they differ (its opening
paragraph), so both mean §7's object: `data` is `{"profile", "dry_run", "steps"}`, and the list of planned steps is its
`steps`. The Scope's last bullet, added at the brief gate (R3 NIT-1), says so where the Scope is read first. The envelope
wraps that object directly (R4 NIT-2): a run that exits 0 prints one envelope
`{"schema_version": 1, "ok": true, "data": ..., "error": null}` whose `data` is §7's object itself, since each verb
supplies only its `data` (E61), and the test kit's envelope checker, which every `--format=json` test of a pane or
profile verb reuses, takes any value there (E62). A run that exits 1, 2 or 3 prints a failure envelope, whose `data` is
`null` (E61, E62). At exit 1 for a `failed` step, by the decision on O6, its `error.code` is that step's error code and
its `error.message` takes the decision's pinned form, `apply stopped at <pane> (<action>): <step error message>; <N>
later step(s) skipped` (C19).

(R6 NIT-1) A step's `diff` is a whole `PaneDiff` (E1), embedded unchanged as §7 says: the object
`{"pane", "status", "differences"}` that `PaneDiff`'s derived `Serialize` writes, its `status` in snake case (`"differs"`
for row d) and its `differences` the list of `FieldDiff`s, each `{"field", "spec", "live"}` (E1, E56). It is neither
`PaneDiff.differences` alone nor a renamed type: `diff` is only §7's name for the key of the step object that holds it.
Row d's `reported` step carries it (§3: "`reported` with the `PaneDiff`").

**C7. Tests added to §11 (B-2, B-3, NV-9, W-1; R2 NV-3, R2 NV-5, R2 NV-6, R2 W-1, R2 W-3; R3 B-4, R3 B-5, R3 NV-1 to
R3 NV-4; R5 B-1, R5 B-3; their fixtures and checks also for R4 B-4, R4 NV-4, R4 W-2, R5 NV-6 and R6 B-3),** on the test
kit's fakes, as §11's are:

- (B-2) A dry run in which the `health` call of a recorded pane that row a does not refuse fails: the plan `tx_apply`
  returns for the dry run lists that pane's step as `failed` and every later step as `skipped`; no probe runs and nothing
  is written; the verb exits 1 and prints only the failure, no plan (the decision on O6; C2, C19).
- (B-3) A `create` step that the launch transaction answers with `pane-exists` is a `refused` step carrying
  `pane-exists`, and the run goes on to the next step. The record has to appear after apply's re-read (§9) and before
  launch's own check (E14). `fail_next` injects only errors (E26), so the test writes the record through a pane-store
  seam passed to `Rig::run_over` (E34).
- (NV-9) A store conflict at a step's write is a `failed` step that stops the run; the steps after it are `skipped`; the
  verb exits 1. The rig's `NthCasPut` seam (E34) answers a numbered `cas_put` with `generation-conflict`.
- (W-1) A Herdr snapshot that cannot be read: the first step whose row needs it is `failed`, the steps after it are
  `skipped`, and the verb exits 1.
- (R2 NV-5, R2 NV-6) A recorded orchestrator pane that matches its spec, whose `health` answers `Ok(true)` and whose TUI
  shows its session of record, is still not healthy when Herdr does not list its session and pane id (C9): once after
  `vanish` of its Herdr pane, and once with a record that names a Herdr session other than the fake's beside a pane id
  the fake holds. Each time its step is `refused` (`orchestrator-not-relaunched`, row e), not `adopted`, and nothing is
  written or touched live. The second case fails an apply that matches the pane id alone.
- (R2 NV-3, R2 W-1) Row c's writes, as the fake pane store's call log counts them (C10): none for a pane already in P,
  one `cas_put` (the join) for a pane in no profile, and two under `--take-over` for a pane of another profile, the leave
  and then the join. An adopt whose join the rig's `NthCasPut` (E34) answers with `generation-conflict` is a `failed`
  step: nothing live was touched, the steps after it are `skipped`, and the verb exits 1. (R3 NV-2) A call log names
  each call's method only (E57), so it counts the writes but cannot tell the leave from the join: the test reads their
  order from the fake pane store's `watch`, opened at a cursor taken before the run, which yields every later write in
  order with the record as written (E57). The leave's event has `profile` none, and the join's, after it, has `profile` P.
- (R2 W-3) The up-front `usage` case of §11 makes no Herdr or harness call: the fake Herdr's and the fake harness's call
  logs are empty (C1).
- (R3 B-4) A step runs under its own budget (C13). The engine is called with a short `TxOptions` budget and the fake
  harness slowed by `set_delay` (E59), over spec panes that have no record, so apply's own observations make no harness
  call (C1) and every step is a `create`: the first `create` outlasts its budget, so its step is `failed` with `timeout`
  whose `op` is `pane.launch` (E42, E59), the steps after it are `skipped`, and the engine returns well inside a generous
  bound, as #644's test does (E59).
- (R3 B-4) No budget over the whole run (O5, C13). Over the same kind of fixture, with a budget and a delay under which
  one `create` fits in the budget and two in a row do not, both steps are `created`.
- (R3 B-5) The shown session (C12), for a recorded agent pane that matches its spec, whose `health` answers `Ok(true)`
  and whose Herdr session and pane id the snapshot lists: when its TUI is on the home screen (`navigate` to `None`, E60,
  so `shown_session` answers `Ok(None)`, E25) and when `shown_session` answers an error
  (`fail_next(HarnessOp::ShownSession, ..)`, E26), the step is `adopted` and nothing live is touched; when its TUI shows
  another session of its server (`seed_session`, then `navigate` to it, E60), the pane is not healthy, so the step is
  `relaunched` (row f). (R4 NV-4) The pane has a TUI only once `attach_tui` has put one there, showing the session it was
  attached to: the pane's launch does that (E64), or the fixture calls it. `navigate` answers `unavailable` for a pane
  with no TUI, and `navigate` to `None` sets the TUI's `shown` to `None`, which `shown_session` returns as `Ok(None)`
  (E25, E60, E64).
- (R3 NV-1) Row f under `--take-over` (C11): an agent pane of another profile Q whose server is killed (E25) is
  `relaunched` and its record names P, and the fake pane store's `watch` (E57) shows the leave (`profile` none), then the
  join (`profile` P), then the relaunch's record write (an event like every write that lands: C17, E65, E76). When the
  relaunch fails instead, with a failure-class error such as `PaneError::Unavailable { .. }` (E46) injected by
  `fail_next` (E26) at a port method that only the relaunch calls (apply's own observations call only `snapshot`,
  `health` and `shown_session`, C1), the step is `failed`, the record still names P, the watch shows no write after the
  join, the steps after it are `skipped`, and the verb exits 1 (O1).
- (R3 NV-3) A per-pane `usage` (§4): an agent pane that is not healthy and whose spec's `host.cwd` differs from its
  record's is answered `usage` by the relaunch rules (E17), so its step is `refused`, carrying a named code; the run goes
  on to the next step, and the verb exits 0 when no step is `failed` (§5).
- (R3 NV-4) What a dry run calls (§6, C1, C2): over a fixture with a step of each of rows a to f, run with
  `--herdr-session`, the fake Herdr's call log holds no method but `Snapshot`, the fake harness's none but `Health` and
  `ShownSession`, the fake host's log and the prober's are empty, and neither store's log holds a `CasPut` or a `Delete`
  (E57). So no launch or relaunch run started, since a run's first live reads are the probe and Herdr's `version` (E15).
- (R5 B-1) `--herdr-session` is needed for a `create` alone (C1): over a fixture in which every spec pane has a record,
  among them an agent pane of P whose server is killed (E25) and whose spec keeps its record's directory and cell (so the
  relaunch rules pass, E17), run without `--herdr-session`, the verb does not exit 2: that pane's step is `relaunched`,
  its new record names the Herdr session the old one named (E14), no step is `refused` for the flag, and the verb exits 0.
- (R5 B-3) A failed `health` call for a pane that row a refuses (C1): over a fixture whose one recorded spec pane belongs
  to another profile Q, followed in §3's order by a spec pane with no record, run with `--herdr-session` and without
  `--take-over`, with `fail_next(HarnessOp::Health, ..)` and a failure-class error such as `PaneError::Unavailable { .. }`
  (E26, E46), which the Q pane's up-front `health` call takes, since it is the only one (C1 step 3): the Q pane's step is
  `refused` (`pane-in-other-profile`), not `failed`; the next step runs and is `created`; the fake pane store's `watch`
  yields no event for the Q pane; and the verb exits 0 (§5).

With one thread per pane (§9), `fail_next(HarnessOp::Health, ..)` fails whichever pane's call reaches the fake first
(E26). §9's "all panes" are the panes whose health there is to observe (R4 B-4): C1's step 3 calls `health` once for each
spec pane that has a record and for no other pane, since §2 defines "healthy" for a recorded pane only and a spec pane
with no record is row b whatever its health (§3). So a test that fails one named pane's `health` (§11's test and the B-2
test above) uses a fixture in which that pane is the only one whose health is observed, because it is the only spec pane
with a record. The fixture still has several spec panes: at least one spec pane with no record comes after the failing
pane in §3's order, and the test asserts that its step is `skipped` (§5). Since the plan then holds a `create`, the test
passes `--herdr-session` (§4, C1). (R6 B-3) By the decision on O6 the verb prints only the failure at exit 1, so this
test, like every test of §11 and of this list that asserts a `failed` step and the `skipped` steps after it (the B-2,
NV-9, W-1, R2 W-1, R3 B-4 and R3 NV-1 tests), reads those steps from the result `tx_apply` returns; for the situations of
C14's entries 17, 21, 22, 23, 25 and 29, a verb-level test also asserts exit 1 and the failure (C19). The R3 B-5 test's
`fail_next(HarnessOp::ShownSession, ..)` likewise uses a fixture with one recorded pane.

(R4 W-2) Every test of §11 and of this list that states what a run writes to the registry ("no registry write", "nothing
is written", "the profile moved", or a number of writes) asserts it from the fake pane store's `watch`, opened at a cursor
taken before the run (E57): no event after that cursor when it states no write, and otherwise exactly the events it
names, in order, each with the record as written. A write that the rig's `NthCasPut` fails never reaches the fake (E34),
so it has no event. A test that also states that P is not written asserts that the fake profile store's call log holds no
`CasPut` or `Delete` (E57), as the R3 NV-4 test does. The call logs remain the check of which methods a run called.

(R5 NV-6) For the writes of a leave and a join (C10), "the record as written" is exact. The fake pane store stores a
written record as it was submitted, with only its generation replaced, by one more than the stored one (E65). So the
record of each such event is the record the write started from, with `profile` none for a leave or P for a join, its
generation one higher, and every other field unchanged. The record it started from is the record as re-read before the
step (§9) for a leave, or for a join with no leave before it, and the record the leave stored for a join after a leave
(C10). The tests that read a leave or a join from the `watch` (R2 NV-3, R3 NV-1, R3 NV-2, and §11 d's take-over) assert
that record, so they also check that the twins write what `detach` and `join` write for one pane (E30, E31, E66; C17).

**C8. An error before any plan exists (NV-7; from §5).** §5 calls its exit codes "the doctor form". `pane doctor` fails
only when it cannot run its pass (a pane or profile that does not exist, a store that cannot be read), and everything its
pass observes is a finding, not an error (E40, E45). So §5's codes are those of a run that has a plan. An error before any
plan exists (P not found, the registry or P unreadable) is the verb's own error, answered under ADR-0021 section 9's
contract (E41, E42): `profile-not-found` exits 3, and a failure such as `unavailable` exits 1.

**C9. What "healthy" observes (R2 NV-2, R2 NV-5, R2 NV-6; from §2, §5 and §10).**

- *An error from `health`.* §2 makes every `Err(_)` from `health` a `failed` step, whatever class `class_of` gives its
  code, for a pane that row a does not refuse (C1, R5 B-3), and a `failed` step makes the run exit 1 (§5). The OpenCode
  adapter's `health` never answers `Err`: a server that refuses, times out or answers garbled is `Ok(false)` (E50). In
  the fakes, `health` answers `Err` only through the harness's fault switch (E49):
  `fail_next(HarnessOp::Health, error)` answers, once, the `error` the test passes (E26); a standing
  `Fault::Fail(error)` answers `error`, and a standing `Fault::Wedged` answers `timeout` whose `op` names the
  method called (`harness.health` for `health`), to every harness call, `shown_session` included (E48, E49). A frozen,
  killed or never-served fake server is `Ok(false)`, not an error (E25, E49). (R5 NV-4) E25 quotes the fake's `health`
  whole: its one `?` is the fault switch's `enter`, and the fake's `lock` returns its guard, taking over a poisoned lock
  rather than failing (E69). `enter` answers `Err` with the standing fault, or else with the oldest error `fail_next`
  queued for the method, and otherwise `Ok` (E69), so the fault switch is the fake `health`'s one source of `Err`. So the
  tests of §11 and C7 fail one pane's `health` with `fail_next` and a failure-class error, such as
  `PaneError::Unavailable { .. }` or `PaneError::Timeout { .. }` (E46), whose codes `class_of` puts in exit class 1 (E47).
- *What the snapshot must list.* §2's "the Herdr snapshot lists the recorded `pane_id`" holds when the snapshot has a pane
  whose `session` and `pane_id` equal the record's `herdr.session` and `herdr.pane_id` (E9). Every place the code matches
  a Herdr pane uses that pair: `tx_launch`'s `same_pane` (E21) and `pane doctor`'s `herdr-pane-missing` (E51), and
  neither compares the workspace or the cell. `same_pane` is private to `tx_launch.rs`, which this story calls unchanged
  and does not edit (§10, Blast radius), so `tx_apply` compares the pair itself, as `reconcile` does.
- *How a test makes the fake leave a recorded pane out (R2 NV-5).* The port's snapshot is every pane Herdr has (E6), so in
  the port's terms a pane the snapshot does not list is one Herdr does not have in that session, and no pane exists
  unlisted. The fake has no such state either: it serves one session and lists every pane it holds, and a fault on
  `snapshot` fails the whole call rather than leaving a pane out (E24, E52; C3). It leaves a recorded pane out when the
  pane is gone (`vanish`, its shell exited, or `close`; E24, E52), when the record names a pane id the fake never minted
  (an id is `w<N>:p<M>` and is never reused, E52), or when the record names a session other than the one the fake serves
  (E52). C7 tests the first and the last.

**C10. The writes of `adopt` and of a take-over, and a conflict at each kind of step (R2 NV-3, R2 W-1; from §3, §5 and
§9).**

- *Row c's registry effect* (§3: "set `Pane.profile` to P if not already"), on the record as re-read before the step
  (§9): no write when its `profile` is already P; one write, the join, when it is none; and, under `--take-over`, when it
  is another profile Q, two writes, the leave and then the join. Row d makes the same two writes under `--take-over` and
  none otherwise. The leave is E30's write: the record with `profile` none, by `cas_put` at the generation it was read at.
  The join is the write E31 makes for each pane: the record with `profile` P, by `cas_put` at the generation it was read
  at; after a leave, it writes the record the leave returned (E30), at that record's generation, so a writer between the
  two writes makes the join conflict (C5). Neither row writes P or calls `edit_spec`: §3 gives rows c and d no effect on
  P, and P already holds the pane's spec, since the step comes from it.
- *The twins (§9) are those writes.* `join`'s failure path, `undo`, ends by deleting the profile that `profile create`
  made (E31, E53). §3's registry effects hold no profile delete, and apply did not make P, so the twin of `join` never
  deletes P. (R4 W-1) The follow-up issue that §9 files to merge the twins says in its body that, until the merge, the
  twins must stay semantically identical to `detach` and to `join`'s write for one pane: the record as read, with
  `profile` none or with `profile` P, written by one `cas_put` at the generation it was read at, answering what that
  `cas_put` answers (E30, E31).
- *A conflict at each kind of step.* Rows c and d: apply's own write at a stale generation answers `generation-conflict`
  (`PaneError::Conflict`; E5, E30, E46), failure class (E47), so the step is `failed`, the run stops there, the steps
  after it are `skipped` and the verb exits 1 (§5, C5); nothing live was touched, since rows c and d never touch a live
  pane (§3). What apply does when a take-over's join fails after its leave landed is open (O4). Rows b and f: the
  conflict comes from inside the launch transaction (E8, E10, E18). When the run names P, a conflict at P's first write
  is `generation-conflict`, before anything live changed, and one after the act is `profile-conflict` (E8); a conflict
  at the record write, which follows the live change, is `generation-conflict` and is not rolled back (E10, E19, E37).
  Each reaches apply as the `error` of a `TxFailure`, whose `acted` says whether the act was entered (E55), and each is
  failure class, so the step is `failed` in the same way. A conflict after the live change is the case E37 has a verb
  answer with the reconcile step. Rows a and e write nothing.

**C11. The write that sets `Pane.profile` to P in row f (R3 B-1, R3 NV-1; from §3 and O1).** A relaunch makes one record
write (E19), which stores the profile its request names, or else the record's own (E16), so a relaunch with
`profile: Some(P)` records P. Row f's two cases:

- *No take-over* (the record names P, or no profile): apply moves no profile, and the relaunch's own record write is the
  one that sets `Pane.profile` to P (row f's last sentence). A record in no profile may join P: the membership check of
  the scope's plan step refuses a `Set` only for a pane of another profile (E28, E29), and the registry allows joining
  from none (E27, E38).
- *Under `--take-over`* (the record names another profile Q): apply's leave and join (C10) come first, and they are the
  writes that set `Pane.profile` to P. The relaunch then runs on a record that names P, so the scope does not refuse it
  `pane-in-other-profile` (E28, E29), and its own record write keeps P (E16). The `record` of its `RelaunchRequest` is
  the record the join returned (E5: `cas_put` returns the stored record with its bumped generation), because the
  request's record carries the generation the relaunch's record write expects (E13, E19). A request built from the
  record as read before the move would make that write a `generation-conflict` after the live change (C5).

In both cases apply writes nothing after the relaunch, also when it fails (O1). C7's R3 NV-1 test checks the take-over
case.

(R5 NV-2) The relaunch's record write is its only pane-store write. `run` hands P's edit and the act to one `edit_spec`
call (E18), and the act, `act_live`, writes the record by one `cas_put`, after the pane is up (E19, E67). That `cas_put`
is the only one in `tx_launch.rs`; the file's other pane-store calls are reads, `get` at line 182 (launch's
`pane-exists`, E14) and `list` at line 462 (which words a `grid-occupied` refusal). Neither scope writes a pane record:
recording the pane is the act's (E68). With `profile: Some(P)` the run also writes P, which is not the record: `edit_spec`
writes P's spec for the pane by one compare-and-swap before the act, and puts P's specs back by one more when the act
fails (E8, E68). So the record write sets `Pane.profile` (E16), and no other write of the run touches it.

(R6 NV-1) The request's P reaches the record the relaunch writes, whatever profile the old record names. `relaunch`
hands `request.profile` to `Plan::new` (E14), which keeps it as the plan's `profile` (E74, line 282). `record` takes
`self.profile.or(old.profile.as_ref())` (E16), which is the plan's P whenever the request names one, so an old record in
no profile does not change it. The record `record` builds carries that value in its `profile` field (E74, line 348), and
`act_live` writes that record, `plan.record(seen, live, now_ms())`, by its one `cas_put`, after the pane is up (E19,
E67). So in the no-take-over case a relaunch with `profile: Some(P)` of a record in no profile writes a record that names
P, which is the write row f's last sentence relies on; under `--take-over` the record already names P when the relaunch
runs (C10), and the write keeps it.

**C12. The shown-session comparison (R3 B-5; from §2 and O2).** `shown_differs` answers `true` for a `None` shown (E22),
while O2 rules that an `Ok(None)` reading does not make a pane unhealthy, so apply never passes `shown_differs` a `None`
shown. For each recorded pane it reads `shown_session(&record.herdr.pane_id)` (E7, E9) and takes §2's third condition,
the shown session, from the answer alone:

| `shown_session` answers | §2's third condition | Source |
|---|---|---|
| `Ok(Some(s))` | fails exactly when `reconcile::shown_differs(record.session_of_record.as_deref(), Some(s.as_str()))` is `true` | §2; O2 (the only mismatch) |
| `Ok(None)` | holds: cannot tell (reported in the step's detail, not acted on) | O2 |
| `Err(_)` | holds: cannot tell (reported in the step's detail, not acted on) | §2; O2 |

A pane whose `session_of_record` is `None` never fails it (E22). §2's other two conditions, `health` and the snapshot
(C9), are unchanged. C7's R3 B-5 test checks the three answers.

(R6 NV-7) The `Err(_)` row's words are the decision on O2's, not §2's: "`Ok(None)` and `Err(_)` both mean "cannot tell"
and do not make a pane unhealthy. Only `Ok(Some(s))` with `shown_differs` true counts as a mismatch." §2's "(an
unreadable shown session does not make a pane unhealthy)" agrees with that row, and "cannot tell" and "reported in the
step's detail, not acted on" are the decision's terms ("an unreadable reading is reported in the step's detail, not acted
on"). So the source column's "§2; O2" means §2 for the outcome and the decision on O2 for the wording; no code decides
it, since the comparison is code this story writes (E33).

(R5 NV-5) What the fake harness answers in the other cases E23 lists for the adapter. A session another client deleted
(`delete_session`) sends every TUI showing it to its home screen, and a TUI whose process exited (`close_tui`) leaves its
pane with no TUI, so `shown_session` answers `Ok(None)` in both (E70). A frozen or killed server leaves its TUI's screen
as it was, since `shown_session` does not reach the server (E25, E70), so a TUI attached to `s` still answers
`Ok(Some(s))`.
A Herdr pane that `vanish` removes keeps its TUI in the fake harness, since `vanish` changes only the fake Herdr's own
state (E24) and the fake Herdr holds no harness state (`crates/holler-pane-testkit/src/herdr.rs` names neither the
harness nor a TUI). Each answer is one of the table's three rows, so the table decides every case; the tests of C7 reach
`Ok(None)` through `navigate` to `None` alone (R3 B-5).

**C13. Testing the bound, not only its doc (R3 B-4; from §9, C4 and O5).** §9's `grep`-style test keeps the doc
sentence from being dropped; two of C7's tests check that the engine keeps the structure that sentence states. A step
that outlasts its own budget ends `failed` with `timeout` and stops the run (§5, §9), and two steps that each fit one
budget both complete, since there is no budget over the whole run (O5). Both follow #644's bound test (E59): the engine
is called with a `TxOptions` whose `budget` is short, and `set_delay` slows every call of the fake harness. So
`tx_apply`'s entry point takes the `TxOptions` its steps run under from its caller, as `tx_launch::launch` and `relaunch`
do (E14, E58), and hands it to each step's `tx_launch` run, whose budget starts when that run starts (E14); the verb
passes `TxOptions::default()`, as `pane launch` and `pane relaunch` do (E58). That entry point is code this story writes,
so no excerpt can show it (E33); C17 names the tests that check it (R5 NV-1).

**C14. The acceptance list (R3 B-2, R6 B-2, R6 B-3, R6 W-2).** The Acceptance section's fifth bullet makes this list the
story's whole acceptance: every entry passes on the test kit's fakes, judged at the end of the story, after phase F (the
Acceptance section's seventh bullet); entry 20's test reads a source file and uses no fake (C16). "Acceptance *n*" is
the *n*-th bullet of the Acceptance section. A test may cover several entries; each entry's source states its test, and
this index adds none. Entries 17, 21, 22, 23, 25 and 29 assert the `failed` and `skipped` steps of a run that exits 1. By
the decision on O6, each of them is checked by an in-process test that calls `tx_apply` over the fakes and reads the
`failed` and `skipped` steps from the result it returns, and by a verb-level test that asserts exit 1, the failure
envelope's `error.code` (the failed step's code) and its `error.message` (the decision's pinned form) and, in text mode,
that stdout is empty; the verb prints no plan and no `skipped` list at exit 1 (C19). Entry 26's test calls the engine
itself (C7, C13), so it reads its steps from the same result.

| # | The test | Source |
|---|---|---|
| 1 | Apply twice: the second run changes nothing. | Acceptance 1 |
| 2 | Row c: a healthy pane that matches its spec is `adopted`, not relaunched, with no live call. | Acceptance 1; §11 c |
| 3 | Row d: a healthy pane that differs is `reported`, not killed, and nothing live is touched. | Acceptance 1; §11 d |
| 4 | A dry run past C1's gate (with `--herdr-session`, or with every spec pane recorded) runs no probe and writes nothing, and its plan parses against the envelope: `data` is §7's object itself, with `"dry_run": true` (C6, E61, E62). A dry run whose plan has a `failed` step prints no plan and is entry 21's (O6). | Acceptance 1; §11 |
| 5 | Row a: a pane of another profile is `refused` (`pane-in-other-profile`) without `--take-over`. | Acceptance 2; §11 a |
| 6 | With `--take-over`, that pane is `adopted` (the leave, then the join). | Acceptance 2; C7 (R2 NV-3) |
| 7 | A dry run's plan lists the pane of another profile. | Acceptance 2 |
| 8 | A failing probe: that pane's step is `refused` (`probe-failed`), and the others proceed. | Acceptance 3; §11 |
| 9 | Row b against #638's split-only fake: a missing pane is `created`, by splits. | Acceptance 3; §11 b |
| 10 | A healthy pane in the wrong cell is `reported`, never moved. | Acceptance 3; §11 d |
| 11 | `apply --dry-run` and `holler profile show` agree on the same fixture. | Acceptance 4 |
| 12 | Row b: an occupied cell is refused through launch. | §11 b |
| 13 | Row b: a `create` that launch answers with `pane-exists` is `refused` with that code, and the run goes on. | §11 b; C7 (B-3) |
| 14 | Row d with `--take-over`: the profile moved, and nothing live is touched. | §11 d |
| 15 | Row e: an orchestrator that is not healthy is `refused` (`orchestrator-not-relaunched`) with no registry write, also with `--take-over`: no leave and no join (C15). | §11 e; C15 |
| 16 | Row f: a pane whose server was killed is `relaunched`. | §11 f |
| 17 | A failed `health` call: a `failed` step that stops the run, the rest `skipped`, read from the result `tx_apply` returns; exit 1, and the verb prints only the failure (O6, C19). | §11; the decision on O6 |
| 18 | The up-front `usage` case: exit 2, no step run, and no Herdr or harness call, in a live run and in a dry run, which prints the usage error and no plan (C1). | §4; §11; C7 (R2 W-3) |
| 19 | A stale spec is not written back. | §11 |
| 20 | The module doc of `tx_apply.rs` states the run's bound: the `grep`-style test, which T writes first and confirms RED against E33's stub, which checks the strings C16 names, and which passes once F has written the doc; and S's audit of C4's form. The test reads the file and uses no fake (C16; the Acceptance section's seventh bullet). | §9; C4; C16 |
| 21 | A dry run with a failed `health` call: `failed`, the rest `skipped`, read from the result `tx_apply` returns for the dry run; no probe, no write; exit 1, and the verb prints only the failure, no plan (O6, C19). | C7 (B-2); the decision on O6 |
| 22 | A store conflict at a step's write: `failed`, the rest `skipped`, read from the result `tx_apply` returns; exit 1, and the verb prints only the failure (O6, C19). | C7 (NV-9); the decision on O6 |
| 23 | An unreadable Herdr snapshot: the first step that needs it `failed`, the rest `skipped`, read from the result `tx_apply` returns; exit 1, and the verb prints only the failure (O6, C19). | C7 (W-1); the decision on O6 |
| 24 | An orchestrator that Herdr does not list (after `vanish`; under another Herdr session): `refused` (`orchestrator-not-relaunched`), nothing written or touched. | C7 (R2 NV-5, R2 NV-6) |
| 25 | Row c's writes, counted and in order, read from the fake pane store's call log and `watch` (C7); a join that conflicts: `failed`, the rest `skipped`, read from the result `tx_apply` returns; exit 1, and the verb prints only the failure (O6, C19). | C7 (R2 NV-3, R2 W-1, R3 NV-2); the decision on O6 |
| 26 | A step that outlasts its budget: `failed` with `timeout`, the rest `skipped`, read from the result of the engine the test calls (C7, C13), inside a generous bound. | C7 (R3 B-4) |
| 27 | No budget over the whole run: two `create` steps that each fit one budget, though not both together, are both `created`. | C7 (R3 B-4) |
| 28 | The shown session: the home screen and an error leave the pane `adopted`; another session makes it `relaunched`. | C7 (R3 B-5) |
| 29 | Row f under `--take-over`: the leave, the join, then the relaunch, read from the fake pane store's `watch` (C7, C17); a relaunch that then fails leaves the pane in P, with no write after the join, and its `failed` step and the `skipped` steps after it are read from the result `tx_apply` returns; exit 1, and the verb prints only the failure (O6, C19). | C7 (R3 NV-1); the decision on O6 |
| 30 | A per-pane `usage` from the relaunch rules: `refused` with a named code; the run goes on and the exit code is unchanged. | C7 (R3 NV-3) |
| 31 | A dry run calls only the reads C1 names. | C7 (R3 NV-4) |
| 32 | `--herdr-session` is needed for a `create` alone: with every spec pane recorded and the flag absent, a killed agent pane is `relaunched` in its record's Herdr session, and the verb exits 0, not 2. | C7 (R5 B-1) |
| 33 | A failed `health` call for a pane that row a refuses: `refused` (`pane-in-other-profile`), not `failed`; the run goes on and exits 0. | C7 (R5 B-3) |

**C15. Row e under `--take-over` (R4 B-2; from §3).** No leave or join occurs for row e, even under `--take-over`. §3's
preamble gives the take-over's profile move to rows c, d and f only ("rows c, d and f add the profile move described in
the table") and says "row e never writes", and row e's own registry effect is "none, **even under `--take-over`**". So an
orchestrator pane of another profile Q that is not healthy keeps its record exactly as it was, still naming Q, and nothing
live is touched: its step is `refused` (`orchestrator-not-relaunched`), and apply calls neither the leave twin nor the
join twin (§9) for it. R4 B-2 asked for this sentence in row e's own text; §3 is the operator's and stays as written, so
the sentence stands here. Entry 15 of C14 runs the pane with and without `--take-over` and asserts, both times, that the
fake pane store's `watch`, opened at a cursor taken before the run, yields no event for that pane (an event names its
pane, E57), so its record is unchanged, generation included.

**C16. §9's doc test, and when entry 20 can pass (R5 B-4, R5 W-1, R5 W-2, R6 B-2, R6 NIT-2; from §9, C4 and the Rules
for this story).**

- *When it is written.* The test is one of T's, like every test C14 lists: tests come first (Rules for this story), so T
  writes it in phase T and confirms it RED there, since `tx_apply.rs` is still E33's stub, whose module doc states no
  bound. F writes the doc (C4), which turns it GREEN, and S then audits the doc against C4's form. So entry 20, like every
  entry of C14, is checked against the code this story writes, not against the file as it is today, and the brief quotes
  E10 as the form the doc follows, not as the doc (C4). (R6 B-2) Entry 20 is passing when this test passes at the end of
  the story, after F has written the doc, which is when every entry of C14 is judged (the Acceptance section's seventh
  bullet). Its RED in phase T is the pipeline's test-first check, which every test of C14 goes through, not a deliverable
  of the story, and the test is not expected to pass against E33's stub.
- *What it checks.* It reads `crates/holler-pane/src/tx_apply.rs` and takes the module doc, the run of `//!` lines at the
  top of the file (C4), with each line's `//!` prefix dropped, each line trimmed, and the lines joined by single spaces,
  so that neither a line break nor a continuation line's indent inside a phrase matters (R6 NIT-2). It asserts that this
  text holds each of five strings verbatim: `**Bound (I5).**`, the marker of E10's form (C4); `its own I5 bound`, §9's
  words for the bound of each step; `no overall cap`, §9's words for the whole run, which the decision on O5 (none)
  keeps; and (R6 NIT-2) two of the bound's values, `10 s`, the default of I5's per-call bound, which §9 gives the
  up-front observation ("one I5 bound (default 10 s)"; E4), and `DEFAULT_BUDGET`, a step's budget by default (E11, E58),
  which C4's form names. F writes the doc so that it holds all five verbatim, so the test matches the doc the story
  writes. A doc that drops the bound's paragraph, its per-step bound, §9's "no overall cap" or the bound's values fails
  it. What the paragraph states beyond those strings, the bound itself in C4's form, is S's audit, and C13's two tests
  check that the engine keeps the structure the paragraph states (C7, R3 B-4).

**C17. Requirements on code that does not exist yet, and the test that checks each (R5 NV-1, R5 NV-3, R5 NV-6, R6 NV-4).**
`tx_apply.rs` is E33's three-line stub, and the verb's file is #637's stub (Blast radius), so four things this brief says
of apply are requirements on the code this story writes, not descriptions of code there is, and no excerpt can show them.
A test on the fakes checks each:

- *The engine takes the `TxOptions` its steps run under from its caller and hands it to each step's `tx_launch` run*
  (C13). C7's two R3 B-4 tests (C14 entries 26 and 27) call the engine with a short budget of their own, so they cannot
  be written against an entry point without that parameter, and the first fails unless that budget reaches the
  `create`'s `tx_launch` run.
- *The `--herdr-session` gate answers before any Herdr or harness call* (C1, §4). C7's R2 W-3 test (C14 entry 18) runs
  the verb in the up-front `usage` case, live and dry, and asserts exit 2, no step run, and empty call logs on the fake
  Herdr and the fake harness (E57).
- *The twins write what `detach` writes and what `join` writes for one pane* (§9, C10). E30 is `detach` whole, and E31
  with E66 is `join` whole; the twins are private to `tx_apply` (§9, E32), and F writes them. The tests that read a leave
  or a join from the fake pane store's `watch` check each write's record, as C7's R5 NV-6 paragraph sets out, and C7's
  R2 W-1 test checks that a join answered `generation-conflict` makes its step `failed`.
- *Under `--take-over`, row f's writes come in the order leave, join, relaunch* (§3 row f, O1; R6 NV-4). That apply makes
  the leave and the join before it calls `tx_launch::relaunch` is a requirement on the code this story writes (§3: "leave
  then join first (two writes, exactly as rows c and d), then the relaunch"). That the relaunch's record write is an event
  of the fake pane store's `watch`, after the join's, is existing code: it is the relaunch's one pane-store write, a
  `cas_put` made after the pane is up (E19, E67; C11); the fake's `cas_put` stores through `put`, which appends one
  `PaneEvent` to the store's feed for each write that lands (E65); and `watch` reads that same feed, every change after
  its cursor, in order (E57, E76). C7's R3 NV-1 test checks the order, and, in its failing-relaunch case, that no write
  follows the join.

**C18. §13's follow-up and #700 (R5 NIT-2; from §13).** #700 does not stand in for the follow-up issue §13 asks for. It is
the story that added `Pane.opencode_agent`, closed on 2026-10-10, and its brief lists the `profile_diff`/`profile show`
comparison among its "Non-goals (recorded, not silent)", naming #647 (E71): it recorded the gap and tracks no work on it.
#647 is open, and an operator comment on it, which parks its part 2, lists "the `SpecField` row for `opencode_agent`
(#700)" among what that part's brief must settle (E72). §13 stands as written: this story files the follow-up as a new
issue and does not widen its own scope; the new issue can link #647's part 2.

**C19. What the verb prints, and where the `skipped` steps are listed, after the decision on O6 (R6 B-1, R6 B-3, R6 W-3;
from §4, §5, §6 and the decision on O6).** §5 and §6 are the operator's and stay as written, as §3 does in C15; the
decision on O6 narrows §6 itself ("This narrows §6's "prints the same table as the plan" to runs that do not fail"), and
this item sets out what follows from it.

- *§5's `skipped` steps are listed in the result `tx_apply` returns.* §5's "steps after it are not attempted and are
  listed as `skipped`" is about the run's result: the `failed` step and the `skipped` steps after it are in what
  `tx_apply` returns to its caller, and the verb does not print them ("No plan table and no `skipped` list is printed at
  exit 1, for a dry run too", the decision on O6). So every test that asserts a `failed` step and the `skipped` steps
  after it reads them from that result, in-process over the fakes: the tests of C14's entries 17, 21, 22, 23, 25 and 29,
  which the decision on O6 names, and the test of entry 26, which calls the engine already (C7, C13). The verb-level
  test of each situation of entries 17, 21, 22, 23, 25 and 29 asserts exit 1, the failure envelope's `error.code` (the
  failed step's code) and its `error.message` in the pinned form `apply stopped at <pane> (<action>): <step error
  message>; <N> later step(s) skipped`, and, in text mode, the one `error:` line on stderr and an empty stdout (the
  decision on O6).
- *What the verb prints, by its exit code.* At exit 0 it prints its steps: with `--format=json`, the envelope whose
  `data` is §7's object (C6); for a dry run, those steps are the plan (§6). At exit 1 for a `failed` step it prints only
  the failure (the decision on O6). At exit 2 (§4's gate, C1, or any other usage error) it prints only the usage error.
  An error before any plan exists (C8) prints only that error. In JSON each of those failures is a failure envelope whose
  `data` is `null`, and in text mode an error goes to stderr and nothing to stdout (E61, E62). So the verb prints a plan,
  or a live run's steps, exactly when it exits 0.
- *A dry run* prints its plan when it gets past §4's gate and no step of its plan is `failed`, and otherwise prints only
  what its exit code above says. C1's steps 2, 5 and 6 order it, and the Acceptance section's sixth bullet states it.

**C20. §10's note on #802 (R6 NV-6; from §10 and the decision on O3).** §10's note says #802 "changes the one `serve`
call" inside `tx_launch` (line 628, E20). O3 records that #802's re-scope of 2026-10-11 is wider (E44: "around lines 587
and 630-640"). E75 quotes those lines: around line 587 is `stop_old` (lines 568-588), whose doc comment holds the "until
#695 stops a server by its recorded pid" that #802 deletes, and lines 627-640 hold the `serve` call and the health check
after it, which #802 replaces with `serve_command`, the host's `run` and a wait for `health` (E44). §10 is the operator's
and stays as written. The two readings ask nothing different of this story: §10 has it call `tx_launch` unchanged, and
`tx_launch.rs` is not in the Blast radius, so this story edits no line of it, the `serve` call or any other, and §10's
"do not edit that line here" holds for every line #802 names. Which harness API this story's tests are written against
is the decision on O3.

**C21. The observation's threads and bound (outside diff gate r1, A round 2 finding 1; from §2, §9 and the precedence rule above).**
§9 observes each recorded pane's `health` on a thread of its own and bounds the up-front observation by one I5 per-call
bound, so a run is bounded by 10 s plus N step bounds. The shown session that §2 also needs (C1 step 3, C12) is read on a
thread of its own beside that pane's `health`, and Herdr's snapshot on one more: every call of the observation runs at
once, one thread per call, and the observation takes one per-call bound, as §9's arithmetic says. A pane's two reads are
independent, so neither waits on the other. Where C1 step 3 says "its `health` and its shown session, one thread per
pane" and C4 counts "the slowest pane's chain", they read "one thread per call" and "the slowest call", since §9 wins
where they differ. The fan-out is `reconcile`'s mechanism (scoped `thread::Builder` threads, each joined, a thread that
cannot start or panics answered `unavailable` naming its call) at one thread per call rather than per pane, because
`reconcile`'s chain has dependent calls and apply's two reads of a pane do not. `tx_apply/observe.rs`'s doc says exactly
that, and not that the observation has "the shape of `observe_all`" (A round 2 finding 2). The same applies to C7's note
above ("With one thread per pane (§9)"): it reads "with one thread per call".

**C22. The step vocabulary as built (A round 2 finding 3; fills what §4, §5, §6, §7 and O2 leave open, and amends none of
them).** A step's `action` is one of `create`, `adopt`, `report`, `relaunch`, `refuse` (rows a and e, and a spec whose
pane is not a pane name) and `observe` (a failed observation). A dry-run step that would act has the outcome `planned`;
row d's is `reported`. §5's "exactly one of" outcomes therefore reads as the six of §5 plus `planned` for dry-run steps
only. A `usage` answer for a step's spec is refused with the code `spec-not-launchable`. The step object of §7 adds
`detail` (O2's "the step's detail"), and an absent `code`, `diff` or `detail` prints `null`. ADR-0021 names every one of
these, and S and the outside gate judge #666's contract against this list.

**C23. Follow-ups before the PR (A round 2 finding 4).** The run's agent files both follow-up issues drafted in
`docs/handoffs/664/handoff-F.md` ("Known issues"), the twins `leave`/`join` of §9 and the one of §13, before the PR opens,
and F cites the twins issue in the module doc of `tx_apply/run.rs`.

### Open points O1 to O5: ALL DECIDED, see "Decisions on O1 to O5 (final)" below (the text of each point is kept as the question that was asked)

- **O1. Row f under `--take-over` (NV-8, and NV-3's take-over half; raised again as R2 NV-1).** With `tx_launch`
  unchanged (§10), a relaunch with `profile: Some(P)` of a pane whose record names another profile Q is refused
  `pane-in-other-profile` at `edit_spec`'s plan step, before anything is written or moved (E18, E28, E29, E39). The
  registry's `cas_put` refuses the same move (E27, E38). So under `--take-over` the pane has to leave Q first, by a
  write outside the relaunch's transaction, and only the join is part of that transaction (the relaunch records P, E16).
  Not decided: whether row f does that leave first, and what a relaunch that then fails leaves behind (the pane in no
  profile, or a compensating re-join of Q).
- **O2. "When the shown session can be read" (§2; NV-4; raised again as R2 NV-4).** `shown_session` answers
  `Result<Option<String>, PaneError>` (E7). The OpenCode adapter answers `Ok(None)` "whenever it cannot tell": the home
  screen, a deleted session, a dead pane, no tmux session or server, and more (E23). The fake answers `Ok(None)` for a
  home screen or no TUI (E25). `shown_differs(Some(record), None)` is `true` (E22). If an `Ok(None)` counts as read, the
  pane is not healthy and an agent pane is relaunched (row f); if it counts as unreadable, it does not make the pane
  unhealthy. An `Err(_)` is unreadable either way. Not decided.
- **O3. #802 is wider than §10 says (NV-6).** §10's note predates #802's re-scope of 2026-10-11 (E44). #802 now replaces
  `HarnessPort::serve` with `serve_command`, changes `tx_launch.rs` around lines 587 and 630-640 rather than one line, and
  changes the test kit's `FakeHarness` (the fake host's `run` is what makes a fake server exist). Both stories edit
  `docs/adr/ADR-0021.md`: §10's edits there are a definition of "adopt" and section 9's apply row, and #802's are section
  2, section 8's "`HarnessPort` as built", the record's section and the Deferred bullet. This story's tests seed running
  servers through `FakeHarness`, so which of #664 and #802 merges first decides which API those tests are written
  against. Not decided.
- **O4. A take-over whose join fails after its leave landed (rows c and d; R2 NV-3, R2 W-1).** Under `--take-over`, rows c
  and d move a pane from Q to P in two writes, the leave and then the join (§3, C10). When the join fails (a
  `generation-conflict`; or a `timeout` or `unavailable`, after which the write may have landed, E53), the step is
  `failed` and the run stops (§5), and the leave has landed: unless the join landed after all or another writer changed
  the record, the pane is in no profile. Not decided: whether apply then re-reads the record and puts the pane back in Q
  (a further write, which can fail too), or leaves it in no profile with the step's error saying so. Row f's take-over
  asks the same question (O1).
- **O5. A budget over the whole apply run (R2 W-2).** C4 states the run's bound per step and sets no budget over the
  whole run. I5 reads "A verb returns in bounded time (default 10 s)" (E35). ADR-0021 section 12 holds that, until an
  operation id exists, "I5 is satisfied by the per-call bound and the loud `timeout`" (E54), and its decision of
  2026-10-11 gives `pane launch` and `pane relaunch` a budget over their whole run (E43). A launch takes about 3 s (E43),
  so a run of four `create` steps takes about 12 s. Not decided: whether apply's run also gets a budget over the whole
  run, and its size, or meets I5 per step as C4 states.

### Open for the operator: O6 (round 4, raised again in round 5): DECIDED, see "Decision on O6 (final)" below (the text is kept as the question that was asked)

- **O6. What the verb prints when its run ends with a `failed` step (R4 B-3; raised again as R5 B-2).** §5 makes such
  a run exit 1 and lists the steps after the `failed` one as `skipped`; C2 reads that, for a dry run too, as a plan
  that lists the `failed` step and the `skipped` ones, and C7's B-2 test and C14's entry 21 check it. The output
  contract leaves no place for that plan at exit 1. In JSON mode `ok` is false for exit 1, 2 and 3 alike, and `data`
  is `null` whenever `ok` is false (ADR-0021's decision 6 and `output.rs`, E61); the test kit's envelope checker fails
  an envelope whose `ok` is false and whose `data` is not `null` (`DataOnFailure`, E62); and in text mode an error
  writes `error: <message>` to stderr and nothing to stdout (E61). `pane doctor`, whose form §5 names, states the same
  limit: "an envelope cannot carry data with a failure" (E63). The same holds for a live run, so the tests behind
  C14's entries 17, 21, 22, 23, 25 and 29 have nowhere in the verb's output to read their `skipped` steps. Each way to
  settle it changes or narrows something the operator decided: (a) at exit 1 the verb prints only the failure
  envelope, the `failed` step's error, and those tests read the `failed` and `skipped` steps from `tx_apply`'s own
  result, not from the verb's output; a dry run with a `failed` step then prints no plan, which narrows §6's "prints
  the same table as the plan" and changes C2; (b) apply's failure envelope may carry `data`, which amends ADR-0021's
  decision 6, `output.rs` and the test kit's checker, none of them in the Blast radius; (c) a `failed` step no longer
  sets the exit code, so the run exits 0 with that step in `data`, as `pane doctor` reports a failed call as a finding
  (E45), which changes §5. R4 B-3 asks only about the dry run, but the answer applies to a live run too. R5 B-2 asks
  again, and holds C2, C7's B-2 test and C14's entry 21 blocked until O6 is decided. Not decided.

## Decisions on O1 to O5 (final, 2026-10-11, taken under the operator's standing rules; they replace "Not decided" above)

- **O1 (row f under `--take-over`): leave then join first, then relaunch.** The profile move (leave Q, join P: two writes, as rows c and d) happens before the relaunch, so `tx_launch::relaunch` (unchanged) sees a record that names P and does not refuse `pane-in-other-profile`. If the relaunch then fails, the pane stays in P (the intended end state), the step is `failed`, and apply writes nothing to compensate. Row f's table text says the same.
- **O2 (a shown session that cannot be read): `Ok(None)` and `Err(_)` both mean "cannot tell" and do not make a pane unhealthy.** Only `Ok(Some(s))` with `shown_differs` true counts as a mismatch. Reason: apply never kills a pane because its TUI shows the home screen; an unreadable reading is reported in the step's detail, not acted on.
- **O3 (#802 vs this story):** this story is written against the test kit as it is on `main` when the run starts, seeding servers through one rig helper (`profile_verbs/rig.rs`), so that if #802 merges first, adapting is a one-line rename of that helper and nothing else. Both stories edit `docs/adr/ADR-0021.md` in different sections (this one: the definition of "adopt" and section 9's apply row); the merge of main resolves it.
- **O4 (a take-over whose join fails after its leave landed): leave the pane in no profile and say so.** The step is `failed`, its error text says the pane left Q and is in no profile, and the run stops (section 5). Apply does not write a compensating re-join of Q (a further write that can fail too); the operator re-runs apply, which then sees a pane with no profile and takes it through rows b to f.
- **O5 (a budget over the whole run): none.** The bound is per step plus one up-front observation, recorded in the module doc of `crates/holler-pane/src/tx_apply.rs` (section 9) and asserted by a test over that file.

## Decision on O6 (final, 2026-10-11; overrides every "O6 ... open / NOT DECIDED" sentence above)

- **O6: option (a).** A run that ends with a `failed` step exits 1 and the verb prints **only the failure**: in `--format=json` the failure envelope (`ok` false, `data` null, `error.code` the failed step's error code, `error.message` naming the pane, the step's action and how many later steps were skipped); in text mode one `error: <same message>` line on stderr and nothing on stdout. No plan table and no `skipped` list is printed at exit 1, for a dry run too (a dry run with a `failed` step prints no plan). This narrows §6's "prints the same table as the plan" to runs that do not fail, and C2 reads accordingly. ADR-0021's decision 6, `output.rs` and the test kit's envelope checker are **not** changed.
- **Where the `failed` / `skipped` steps are asserted:** the tests behind C14's entries 17, 21, 22, 23, 25 and 29 read the steps from `tx_apply`'s own returned result (an in-process test over the fakes), not from the verb's output. The verb-level test for the same situations asserts exit 1, the failure envelope's code and the message text, and that stdout is empty in text mode.
- The `error.message` format is pinned: `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`.

## Evidence (verbatim)

The reviewers cannot see the repository (§12), so the signatures §12 names, and the code and ADR text the items of this
section rest on, are quoted here. Each excerpt is cut from the file and lines its heading cites, at `origin/main` `92fbc6e`
(this branch changes only this brief), except E44 and E72, which are GitHub text: E44 is issue #802 as `gh issue view
802` returned it at 04:44 MDT on 2026-10-11, and E72 is a comment on issue #647 as `gh issue view 647 --comments`
returned it at 06:04 MDT on 2026-10-11. The line under each heading says what the excerpt supports.

**E1. `FieldDiff`, `PaneStatus`, `PaneDiff`** (`crates/holler-pane/src/profile_diff.rs:224-252`). What §7 embeds as
`diff`; `pane` is the pane's name (C6).

```rust
/// One field where a live pane differs from its spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldDiff {
    pub field: SpecField,
    pub spec: FieldValue,
    pub live: FieldValue,
}

/// How one pane of a comparison stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneStatus {
    /// A spec and a live pane of its name, equal on every compared field.
    Matches,
    /// A spec and a live pane of its name that differ on at least one compared field.
    Differs,
    /// A spec with no live pane of its name.
    Missing,
    /// A live pane that no spec names.
    Extra,
}

/// One row of a comparison. `differences` is non-empty exactly when `status` is `Differs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaneDiff {
    pub pane: String,
    pub status: PaneStatus,
    pub differences: Vec<FieldDiff>,
}
```

**E2. `diff_spec`** (`crates/holler-pane/src/profile_diff.rs:267-272`). Row c's "`diff_spec` empty": no `FieldDiff`, every
compared field equal; the names are not compared.

```rust
/// Every field where `live` differs from `spec`, in `SpecField::ALL` order; empty when
/// they match. The names are not compared (the caller pairs them). `env` and `expect`
/// are compared as sets (sorted, duplicates dropped); every other field by equality.
/// Pure: it compares `spec` with `spec_from_pane(live)`, so
/// `diff_spec(&spec_from_pane(p), p)` is always empty.
pub fn diff_spec(spec: &ProfileSpec, live: &Pane) -> Vec<FieldDiff> {
```

**E3. `diff_profile` and `compared`** (`crates/holler-pane/src/profile_diff.rs:287-298` and `311-324`). The comparison
`profile show` uses: rows in the profile's order, paired with live panes by name (C6). `compared` builds each paired row
from `diff_spec` (E2), so a row is `Matches` exactly when `diff_spec` is empty, which is row c's test (R4 NIT-1).

```rust
/// The comparison of `profile` with the panes in `live` (the caller decides which panes
/// count as live; `profile show` passes the profile's members). One row per spec, in the
/// profile's order (`Matches` or `Differs` against the pane of `live` with that name,
/// else `Missing`), then one `Extra` row per pane of `live` that no spec names, in the
/// order of `live`.
pub fn diff_profile(profile: &Profile, live: &[Pane]) -> Vec<PaneDiff> {
    let rows = profile.panes.iter().map(|spec| {
        match live.iter().find(|pane| pane.name.as_str() == spec.pane) {
            Some(pane) => compared(spec, pane),
            None => row(&spec.pane, PaneStatus::Missing),
        }
    });
```

```rust
/// The row of `spec` compared with `pane`, the live pane of its name.
fn compared(spec: &ProfileSpec, pane: &Pane) -> PaneDiff {
    let differences = diff_spec(spec, pane);
    let status = if differences.is_empty() {
        PaneStatus::Matches
    } else {
        PaneStatus::Differs
    };
    PaneDiff {
        pane: spec.pane.clone(),
        status,
        differences,
    }
}
```

**E4. I5's per-call bound** (`crates/holler-pane/src/ports.rs:8-10`). C4.

```rust
//! **Every port is synchronous and blocking, and `Send + Sync`.** In async code call
//! one from `spawn_blocking` (or a thread). Every method returns within I5's bound
//! (default 10 s) or with [`PaneError::Timeout`].
```

**E5. `PaneStore`** (`crates/holler-pane/src/ports.rs:63-74`). The registry's compare-and-swap (C5); `get` is the lookup
behind "no record" (§1).

```rust
/// Every write is a compare-and-swap on the pane's `generation` (see
/// [`crate::generation`]); a stale one is `generation-conflict`.
pub trait PaneStore: Send + Sync {
    /// The pane named `name`, or `None`.
    fn get(&self, name: &PaneName) -> Result<Option<Pane>, PaneError>;

    /// Every pane.
    fn list(&self) -> Result<Vec<Pane>, PaneError>;

    /// Store `pane` if the stored one is still at `expected_generation` (0 for a
    /// new pane); returns the stored record with its bumped generation.
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError>;
```

**E6. `HerdrSnapshot` and `HerdrPort::snapshot`** (`crates/holler-pane/src/ports.rs:96-101` and `149-150`). The snapshot of
§2 (NV-5).

```rust
/// What Herdr reports about every pane it has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrSnapshot {
    pub panes: Vec<HerdrPane>,
}
```

```rust
    /// Every pane Herdr has, with its position.
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError>;
```

**E7. `HarnessPort::serve`, `health` and `shown_session`** (`crates/holler-pane/src/ports.rs:183-188` and `205-206`).
`health` answers `Ok(true)`, `Ok(false)` or a plain `PaneError` (C2, NV-2); `shown_session`'s `None` covers "cannot tell"
(O2).

```rust
pub trait HarnessPort: Send + Sync {
    /// Start the harness server for `name` on `port`; returns its process id.
    fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError>;

    /// Whether the server on `port` answers.
    fn health(&self, port: u16) -> Result<bool, PaneError>;
```

```rust
    /// The session the TUI of `pane` shows, if it can tell.
    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError>;
```

**E8. `ProfileScope::edit_spec`** (`crates/holler-pane/src/profile.rs:395-407`). The I8 transaction in which `tx_launch`
runs P's edit and its act (C5, O1).

```rust
    /// Edit the spec of `pane` in `profile` and make the live change (`act`) as one
    /// transaction (I8): the profile is written with a compare-and-swap on its
    /// generation first, then `act` runs, then the result is recorded; if `act`
    /// fails nothing is recorded, and a conflict after `act` is `profile-conflict`.
    /// Returns the edited profile. With `profile: None` it runs only `act` and
    /// touches no profile (and returns `None`).
    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError>;
```

**E9. `HerdrPane` and the start of `Pane`** (`crates/holler-pane/src/pane.rs:91-100` and `226-243`). A record's `herdr` is
a `HerdrPane`, so §2's "recorded `pane_id`" is `record.herdr.pane_id`, in `record.herdr.session`; `profile` is the
membership §3 reads (NV-5).

```rust
/// Where a pane sits in Herdr: its session, workspace, Herdr's id for it and its
/// grid cell. Also what `HerdrPort::ensure_pane` returns for a pane that exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerdrPane {
    pub session: String,
    pub workspace: String,
    pub pane_id: PaneId,
    pub grid: GridPos,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pane {
    pub name: PaneName,
    /// Bumped on every change.
    pub generation: u64,
    pub herdr: HerdrPane,
    pub host: HostInfo,
    pub harness: HarnessInfo,
    /// THE session: the TUI is attached to it and the hub drives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_of_record: Option<String>,
    pub role: PaneRole,
    pub hold: Hold,
    pub last_observed: LastObserved,
    /// The profile the pane belongs to; a pane belongs to at most one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
```

**E10. The launch transaction's module doc** (`crates/holler-pane/src/tx_launch.rs:1-29`). Row b's "its own checks"
(`pane-exists`, `grid-occupied`, `port-in-use`, the probe, Herdr's version), the act inside `edit_spec`, the relaunch
rules, and the bound C4 follows.

```rust
//! The launch/relaunch transaction (plan, act, observe, record; I3 and I8), story #644: the
//! engine `holler pane launch` and `holler pane relaunch` run, and `profile apply` (#664) will.
//! It works only through the [`Ports`] (ADR-0021 section 5) and holds no I/O of its own.
//!
//! - **Plan**, writing nothing and changing nothing live: the spec names the pane; a launched
//!   name has no record (`pane-exists`) and its Herdr session is given; the port policy names a
//!   port ([`port_of_policy`]); the probe passes when the spec has a check (a failing result is
//!   not stored); Herdr's version is supported; the target cell holds no Herdr pane
//!   (`grid-occupied`, decided by Herdr's snapshot alone, recorded or not: a pane is never
//!   adopted); and no server answers on the port (`port-in-use`, never adopted).
//! - **Act**, inside `ProfileScope::edit_spec` (I8: with a profile, P is written first and its
//!   specs are put back if the act fails): ensure the Herdr pane and the tmux session, run the
//!   spec's command there, start the harness server and check it, create the session of record
//!   through the harness API (never a "ping" session, never "the most recent"), attach the TUI to
//!   exactly that session, and observe that the TUI shows it and Herdr still lists the pane (a
//!   mismatch is `unavailable`). Only then is the record written, by one compare-and-swap.
//! - **Rollback.** A failed step stops the tmux session's processes and closes the Herdr pane this
//!   run created (one the plan's snapshot did not list), and nothing is recorded. The tmux session
//!   and a session the server created stay: no port call ends either. A record write that
//!   conflicts is not rolled back: another writer owns the record now.
//! - **Relaunch** is the same act after stopping only what the pane owns. It keeps the pane's
//!   directory, moves its cell only when the caller was asked to place it (`grid_given`), keeps
//!   the session of record the restarted server still lists, and after a move records the new
//!   pane before it closes the old one. Its rules are the engine's first step (E0), so every
//!   caller is held to them.
//! - **Bound (I5).** A run's budget ([`DEFAULT_BUDGET`]) starts when the engine is entered and is
//!   checked before every live step (a monotonic clock, no port call). When it has run out the run
//!   rolls back and answers `timeout`. A run takes at most the budget plus one port call's own
//!   bound plus its rollback calls.
```

**E11. Launch's refusal codes and its budget** (`crates/holler-pane/src/tx_launch.rs:47-57`). Exit 3, so `refused` steps
(§4); `DEFAULT_BUDGET` is one run's budget (C4).

```rust
/// `pane-exists`: `launch` of a name that already has a record (relaunch it instead). A
/// refusal, exit 3.
pub const PANE_EXISTS: RefusalCode = RefusalCode::from_static("pane-exists");
/// `grid-occupied`: the target cell already holds a Herdr pane (never adopted, recorded or
/// not). A refusal, exit 3.
pub const GRID_OCCUPIED: RefusalCode = RefusalCode::from_static("grid-occupied");
/// `port-in-use`: a harness server already answers on the port before this verb started one
/// (never adopted). A refusal, exit 3.
pub const PORT_IN_USE: RefusalCode = RefusalCode::from_static("port-in-use");
/// The budget of one whole verb run (I5's default bound), checked before every step.
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(10);
```

**E12. `port_of_policy`** (`crates/holler-pane/src/tx_launch.rs:68-73`). §4's per-pane `usage` for a port policy that is
not `fixed:<port>`.

```rust
/// The port a policy names. The grammar beyond #662a's one form is this story's, and it is that
/// form alone: [`FIXED_PORT_POLICY_PREFIX`] then `<port>`, in 1..=65535 in canonical decimal
/// (digits only, no leading zero). So `port_of_policy(&fixed_port_policy(p)) == Ok(p)` for every
/// such `p`, and a policy it accepts equals `fixed_port_policy` of its port. Anything else (the
/// bare `fixed`, `fixed:048100`, `fixed:0`, `auto`) is `usage`, and the message names the form.
pub fn port_of_policy(policy: &str) -> Result<u16, PaneError> {
```

**E13. `LaunchRequest` and `RelaunchRequest`** (`crates/holler-pane/src/tx_launch.rs:113-140`). Row f's request
(`grid_given: false`, `profile: Some(P)`; NV-3); the record's generation is the one its write expects (C5).

```rust
/// What `launch` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub name: PaneName,
    /// Required for a live launch (`usage` otherwise); unused with `spec_only`.
    pub herdr_session: Option<String>,
    /// The effective spec, complete; the engine checks `spec.pane == name` (`usage`).
    pub spec: ProfileSpec,
    /// The profile whose spec for the pane the run sets (I8), by its stored name.
    pub profile: Option<ProfileName>,
    /// Only with `Some(profile)`; the engine re-checks it (`usage`).
    pub spec_only: bool,
}

/// What `relaunch` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaunchRequest {
    /// As the caller read it; its generation is the expected generation of the record write.
    pub record: Pane,
    /// The effective spec, complete.
    pub spec: ProfileSpec,
    /// The caller was asked to place the pane (`--grid`); the engine derives the move.
    pub grid_given: bool,
    /// The profile whose spec for the pane the run sets (I8), by its stored name.
    pub profile: Option<ProfileName>,
    /// Only with `Some(profile)`; the engine re-checks it (`usage`).
    pub spec_only: bool,
}
```

**E14. `launch` and `relaunch`** (`crates/holler-pane/src/tx_launch.rs:169-211`). `pane-exists` (C7) and then the missing
`--herdr-session` `usage` (§4) are launch's first checks after the spec's name (`check_named`), and both come before
`run` and its first prober, Herdr and harness calls (C1, R5 B-1); relaunch runs its rules (E17) first, and takes its
Herdr session from the record.

```rust
/// Create a pane: plan, act, observe, record (see the module docs). Step 0, the spec naming the
/// pane, runs first, also with `spec_only`, which then sets P's spec and calls nothing else.
pub fn launch(
    ports: Ports<'_>,
    request: &LaunchRequest,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    let budget = Budget::start(options.budget, OP_LAUNCH);
    let (name, spec, profile) = (&request.name, &request.spec, request.profile.as_ref());
    check_named(spec, name)?;
    if request.spec_only {
        return spec_only(ports, profile, name, spec);
    }
    if ports.pane_store.get(name)?.is_some() {
        let message = format!("{name} already has a record; relaunch it instead");
        return Err(refused(PANE_EXISTS, message).into());
    }
    let Some(session) = request.herdr_session.clone() else {
        return Err(usage("--herdr-session is required unless --spec-only".to_owned()).into());
    };
    let plan = Plan::new(name, spec, session, None, profile)?;
    run(ports, &plan, &budget, options)
}

/// Launch a pane again, replacing its processes: E0 (the relaunch rules), then launch's plan
/// and act after stopping only what the pane owns (see the module docs). With `spec_only` only
/// E0's spec check runs, and P's spec is set and nothing else is called.
pub fn relaunch(
    ports: Ports<'_>,
    request: &RelaunchRequest,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    let budget = Budget::start(options.budget, OP_RELAUNCH);
    let (old, spec, profile) = (&request.record, &request.spec, request.profile.as_ref());
    check_named(spec, &old.name)?;
    if request.spec_only {
        return spec_only(ports, profile, &old.name, spec);
    }
    check_relaunch_rules(old, spec, request.grid_given)?;
    let session = old.herdr.session.clone();
    let plan = Plan::new(&old.name, spec, session, Some(old), profile)?;
    run(ports, &plan, &budget, options)
}
```

**E15. The plan's reads** (`crates/holler-pane/src/tx_launch.rs:286-292`). A launch or relaunch run probes before anything
else, so a dry run calls neither (C2).

```rust
    /// Steps 3 to 6, none of which writes or changes anything live: the probe, Herdr's version,
    /// the target cell (where a relaunch's own pane is no occupant) and the port, which for a
    /// relaunch is checked only when it changes (its own server answers there until B1).
    fn observe(&self, ports: Ports<'_>, probe_timeout: Duration) -> Result<Seen, PaneError> {
        let probe = probe(ports, self.spec, probe_timeout)?;
        let herdr_api_version = ports.herdr.version()?;
        let listed = ports.herdr.snapshot()?.panes;
```

**E16. The record a run writes** (`crates/holler-pane/src/tx_launch.rs:311-322`). A relaunch with `profile: Some(P)`
records P (O1).

```rust
    /// The record R writes: what the act brought up and observed, and the spec's values. A
    /// relaunch keeps the record's hold, its stored DRIVEN (never inferred) and, without a
    /// profile, its profile.
    fn record(&self, seen: &Seen, live: Live, at: i64) -> Pane {
        let (hold, driven, profile) = match self.old {
            Some(old) => (
                old.hold.clone(),
                old.last_observed.driven.clone(),
                self.profile.or(old.profile.as_ref()).cloned(),
            ),
            None => (Hold::None, None, self.profile.cloned()),
        };
```

**E17. The relaunch rules** (`crates/holler-pane/src/tx_launch.rs:374-395`). §4's per-pane `usage` from a relaunch: a
changed directory, or a cell move without `grid_given`.

```rust
/// E0 past the spec's pane, for a live relaunch: it never changes a pane's directory, and moves
/// its cell only when the caller was asked to place it (decision 11).
fn check_relaunch_rules(old: &Pane, spec: &ProfileSpec, grid_given: bool) -> Result<(), PaneError> {
    if spec.host.cwd != old.host.cwd {
        return Err(usage(format!(
            "relaunch cannot change the directory of {} (from {} to {}); close it and launch it \
             again",
            old.name,
            quoted(&old.host.cwd),
            quoted(&spec.host.cwd)
        )));
    }
    let moved = spec.herdr.workspace != old.herdr.workspace || spec.herdr.grid != old.herdr.grid;
    if moved && !grid_given {
        return Err(usage(format!(
            "relaunch moves a pane only with --grid: the record is at {}, the spec gives {}",
            cell(&old.herdr.workspace, old.herdr.grid),
            cell(&spec.herdr.workspace, spec.herdr.grid)
        )));
    }
    Ok(())
}
```

**E18. `run`** (`crates/holler-pane/src/tx_launch.rs:504-522`). P's edit and the act are one `edit_spec` call with a
`SpecEdit::Set`, the edit E28 and E29 refuse for a pane of another profile (NV-3, O1).

```rust
/// Steps 3 to 7 of both verbs: the plan's reads, then P's edit, when the run names P, and the
/// act, as one transaction (I8).
fn run(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    budget: &Budget,
    options: &TxOptions,
) -> Result<Launched, TxFailure> {
    let seen = plan.observe(ports, options.probe_timeout)?;
    let edit = SpecEdit::Set(Box::new(plan.spec.clone()));
    let (mut entered, mut acted) = (false, None);
    let mut act = || {
        entered = true;
        acted = Some(act_live(ports, plan, &seen, budget, options.now_ms)?);
        Ok(())
    };
    let edited = ports
        .scope
        .edit_spec(plan.profile, plan.name, &edit, &mut act);
```

**E19. The record write** (`crates/holler-pane/src/tx_launch.rs:559-561`). One `cas_put` at the generation read (C5).

```rust
    let expected = plan.old.map_or(0, |old| old.generation);
    let record = plan.record(seen, live, now_ms());
    let stored = ports.pane_store.cas_put(&record, expected)?;
```

**E20. The `serve` call** (`crates/holler-pane/src/tx_launch.rs:628`). The one `serve` call §10 names (NV-6; see E44, O3).

```rust
    let pid = ports.harness.serve(plan.name, plan.port)?;
```

**E21. `same_pane`** (`crates/holler-pane/src/tx_launch.rs:750-753`). The codebase's join key for "the same Herdr pane"
(§2's snapshot test).

```rust
/// Whether two Herdr panes are one: the same Herdr session and pane id (the join key).
fn same_pane(a: &HerdrPane, b: &HerdrPane) -> bool {
    a.session == b.session && a.pane_id == b.pane_id
}
```

**E22. `shown_differs`** (`crates/holler-pane/src/reconcile.rs:171-181`). It takes values already read, not a `Result`, so
an `Err(_)` from `shown_session` never reaches it; a `None` shown is a mismatch (NV-4, O2). Apply calls it only on an
`Ok(Some(s))` reading, never with a `None` shown, so its `None` case never decides a pane's health (O2, C12).

```rust
/// Whether a pane whose session of record is `session_of_record` shows something else: the
/// shown/driven mismatch, since I2 makes the session of record the session the hub drives.
/// `shown: None` is the home screen, a mismatch too. A pane with no session of record has
/// no mismatch (it is `no-session-of-record`).
///
/// The one form of this comparison: a pass calls it with what the TUI shows now, and a
/// reader of a record (the roster, `pane get`, the gate at `send_prompt`) with
/// `last_observed.shown`, provided `last_observed.at > 0` (`0` is never observed).
pub fn shown_differs(session_of_record: Option<&str>, shown: Option<&str>) -> bool {
    session_of_record.is_some_and(|record| shown != Some(record))
}
```

**E23. The OpenCode adapter's `shown_session`** (`crates/holler-adapter-opencode/src/lib.rs:58-63`). Its `Ok(None)` (O2).

```rust
//! - **SHOWN is read from the TUI's terminal title, through tmux** (fact 1): the
//!   `#{pane_title}` of the pane's tmux session, addressed by its exact name as `=<name>:`,
//!   which tmux resolves to the active pane of the session's current window. That pane is the
//!   TUI's. `shown_session` is `None` whenever it cannot tell (fact 3): the home screen, a
//!   deleted session, a title that is not a whole id, a dead pane, another program, no tmux
//!   session or server. It never guesses and never asks the server.
```

**E24. The fake Herdr: `vanish`, `snapshot` and what it lists** (`crates/holler-pane-testkit/src/herdr.rs:223-229`,
`302-311` and `479-487`). The fake's snapshot lists every pane of every declared workspace with its session, workspace,
`pane_id` and cell; `vanish` makes it lack a recorded `pane_id` (NV-5).

```rust
    /// The pane's shell exited (Herdr's `pane_exited`): the pane is gone, its cell is
    /// free, its id is never reused, and every later call naming it is
    /// `pane-not-found`. It bypasses the faults and the call log. `pane-not-found` when
    /// no such pane exists.
    pub fn vanish(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.lock().remove(pane)
    }
```

```rust
    fn snapshot(&self) -> Result<HerdrSnapshot, PaneError> {
        self.faults.enter(HerdrOp::Snapshot)?;
        let state = self.lock();
        let panes = state
            .workspaces
            .iter()
            .flat_map(|workspace| workspace.listed(&state.session))
            .collect();
        Ok(HerdrSnapshot { panes })
    }
```

```rust
    /// Every pane of the workspace, by row and then by column, as `snapshot` lists it.
    fn listed<'a>(&'a self, session: &'a str) -> impl Iterator<Item = HerdrPane> + 'a {
        self.panes.iter().map(move |(&(row, col), pane)| HerdrPane {
            session: session.to_owned(),
            workspace: self.name.clone(),
            pane_id: pane.id.clone(),
            grid: GridPos { row, col },
        })
    }
```

**E25. The fake harness: `kill`, `health` and `shown_session`** (`crates/holler-pane-testkit/src/harness.rs:211-216`,
`287-290` and `351-354`). A killed or frozen server answers `health` with `Ok(false)` (§11's row f test); an `Err(_)` comes
only through the fault switch (E26); `shown_session` is `Ok(None)` for a home screen or no TUI (O2).

```rust
    /// SIGKILL the server on `port`: it is dead, its sessions stay in the data
    /// directory, and a TUI attached to it keeps its screen. `Ok` on a server already
    /// killed; `unavailable` when no server was ever served on `port`.
    pub fn kill(&self, port: u16) -> Result<(), PaneError> {
        self.lock().signal(port, ServerState::Killed)
    }
```

```rust
    fn health(&self, port: u16) -> Result<bool, PaneError> {
        self.faults.enter(HarnessOp::Health)?;
        Ok(self.lock().state(port) == Some(ServerState::Running))
    }
```

```rust
    fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
        self.faults.enter(HarnessOp::ShownSession)?;
        Ok(self.lock().tuis.get(pane).and_then(|tui| tui.shown.clone()))
    }
```

**E26. `FaultSwitch::fail_next`** (`crates/holler-pane-testkit/src/fault.rs:72-77`). One-shot error injection, per method
(C7).

```rust
    /// Fail the next call of `op` with `error`, once. The errors queued for one method
    /// come out in the order they were queued, and a call of another method leaves them
    /// queued. A standing fault answers first, also leaving them queued.
    pub fn fail_next(&self, op: Op, error: PaneError) {
        self.lock().queued.push((op, error));
    }
```

**E27. The fake registry's membership rule** (`crates/holler-pane-testkit/src/pane_store.rs:220-225`). A move from one
profile to another is refused; leaving and joining from none are allowed (O1).

```rust
/// The membership rule of a `cas_put` (ADR-0021 section 8, decided to run inside the
/// registry's compare-and-swap): a pane stored with profile P cannot be written with
/// another profile Q. Leaving (`None`), joining from `None` and keeping P are allowed.
/// Its second caller is `FakeProfileScope`, which runs it for a `SpecEdit::Set` before
/// it writes the profile.
pub(crate) fn check_membership(stored: Option<&Pane>, submitted: &Pane) -> Result<(), PaneError> {
```

**E28. The fake scope: no retry, and `check_joins`** (`crates/holler-pane-testkit/src/profile_scope.rs:223` and
`256-271`). A conflict is not retried (C5); a `Set` for a pane of another profile is refused before P is written (O1).

```rust
        // ASSUMPTION (#663): the first write is not retried on a conflict.
```

```rust
// ASSUMPTION (#661/#663): the scope checks `pane-in-other-profile` itself for a `Set`,
// before the profile write, as well as the pane registry doing so inside its
// compare-and-swap (ADR-0021 "Decisions taken", item 2), so nothing is written to the
// profile and nothing live moves for a pane that cannot join it. It runs with or
// without `--spec-only`, since the scope cannot see that an act is empty. ADR-0021
// section 8, step 1 states it. A `Remove` is not checked: a detached spec stays
// removable.
/// The refusal of a `Set` for `record` in `profile`: the fake pane store's membership
/// rule (slugs compared), applied as if the record were written into `profile`.
fn check_joins(record: &Pane, profile: &ProfileName) -> Result<(), PaneError> {
    let joined = Pane {
        profile: Some(profile.clone()),
        ..record.clone()
    };
    check_membership(Some(record), &joined)
}
```

**E29. The real scope's plan step** (`crates/holler-cli/src/pane/profile_scope.rs:10-13`). It refuses the same `Set` (O1).

```rust
//! 1. *Plan.* Read P at generation g (`profile-not-found` first) and the pane's record, for every
//!    edit, so a pane store that cannot be read fails before P moves. A `Set` whose spec names
//!    another pane is `usage`; a `Set` for a pane in another profile is `pane-in-other-profile`.
//!    A `Remove` is never refused for membership: a detached spec stays removable.
```

**E30. `detach`, the leave write** (`crates/holler-cli/src/profile/delete.rs:145-155`). The leave that §9 twins: one
`cas_put` of the record as read, with `profile` none, at its generation.

```rust
/// `pane` leaves its profile: the record as read, with `profile` none, written by
/// compare-and-swap at the generation it was read at. Returns the stored record. Leaving is
/// never a move between profiles, so the registry's `pane-in-other-profile` rule cannot refuse
/// it; a record that changed since it was read is `generation-conflict`.
pub(crate) fn detach(pane_store: &dyn PaneStore, pane: &Pane) -> Result<Pane, PaneError> {
    let left = Pane {
        profile: None,
        ..pane.clone()
    };
    pane_store.cas_put(&left, pane.generation)
}
```

**E31. `join`, the join write** (`crates/holler-cli/src/profile/create.rs:266-282`). The join that §9 twins: one `cas_put`
per pane, with `profile` set, at the generation listed.

```rust
/// Make each of `panes` a member of `profile`, in order, and return their names: each record
/// as listed, with `profile` set, written by compare-and-swap at the generation it was listed
/// at. A failed join is undone (see `undo`), and the failure says how far the undo got.
fn join(
    pane_store: &dyn PaneStore,
    profile_store: &dyn ProfileStore,
    profile: &Profile,
    panes: &[Pane],
    actor: &Actor,
) -> Result<Vec<PaneName>, JoinFailed> {
    let mut joined = Vec::with_capacity(panes.len());
    for pane in panes {
        let member = Pane {
            profile: Some(profile.name.clone()),
            ..pane.clone()
        };
        match pane_store.cas_put(&member, pane.generation) {
```

**E32. `holler-cli` depends on `holler-pane`** (`crates/holler-cli/Cargo.toml:400-401`). Why the twins are private copies
(§9): `tx_apply` is in `holler-pane` (§10), which cannot depend back on `holler-cli`, and `detach` is `pub(crate)` while
`join` is private (E30, E31).

```toml
# the holler-pane crate (`Ports`, `PaneError`, `GridPos`, `Argv`, `EnvVarName`).
holler-pane = { path = "../holler-pane" }
```

**E33. `tx_apply.rs` today, the whole file** (`crates/holler-pane/src/tx_apply.rs:1-3`; the file has these three lines
and no others). The engine's file, whose module doc holds the bound (C4). The story is not implemented yet: this brief
comes before its code, so neither the engine nor the doc that states its bound exists to quote (R4 NV-1).

```rust
//! The `holler profile apply` transaction: create missing panes, adopt a healthy live
//! pane that matches its spec, never kill a healthy pane. Empty stub declared by #637
//! so that no two stories edit `lib.rs`; story #664 fills it.
```

**E34. The profile verbs' rig: `run_over` and `NthCasPut`** (`crates/holler-cli/tests/profile_verbs/rig.rs:79-90` and
`240-256`). A substitute pane store for one run, and the numbered-write failure (C7).

```rust
    /// Run `holler <argv...>` over this rig with `pane_store` in place of its own (a seam
    /// such as [`NthCasPut`] over `self.panes`).
    pub fn run_over(&self, pane_store: &dyn PaneStore, argv: &[&str], format: Format) -> Outcome {
        run_verb_with(
            argv,
            format,
            Ports {
                pane_store,
                ..self.ports()
            },
        )
    }
```

```rust
/// What [`NthCasPut`] does to one numbered `cas_put`.
#[derive(Debug, Clone)]
pub(crate) enum NthPut {
    /// Answer this error and write nothing.
    Fail(PaneError),
    /// Apply the write, then answer this error: a write that landed but whose answer was
    /// lost (a `timeout` after the store committed).
    ApplyThenFail(PaneError),
}

/// The failing-N-th-write seam (#662 Decision 13, C11): a `PaneStore` that delegates
/// every call to a [`FakePaneStore`] and acts on the `cas_put`s its plan numbers (1 is the
/// first `cas_put` made through the seam). The fakes' `fail_next` fails only the next
/// call of a method, so a failure at the second write needs this.
///
/// Its only state is the counter. A [`NthPut::Fail`] does not reach the fake, so the
/// fake's call log does not record it; every other call does.
```

**E35. I5** (`docs/adr/ADR-0021.md:207`). C4.

```text
| I5 | A verb returns in bounded time (default 10 s); long work returns an operation id. | A wedged fake port makes the verb exit with `timeout` within the bound plus a margin; the operation-id half is not built (section 12, decided 2026-10-11). |
```

**E36. The engines** (`docs/adr/ADR-0021.md:221-223`). `tx_apply.rs` is one of `holler-pane`'s engines (C4).

```text
- `holler-pane` holds types, traits and the pure transaction engines (`tx_launch.rs` #644, `tx_switch.rs` #645, `tx_apply.rs`
  #664, `reconcile.rs` and `findings.rs` #647, `profile_snapshot.rs` and `profile_diff.rs` #662, `import.rs` #650), which work
  only through the ports. It depends on `serde`, `serde_json` and `holler-proto`, and takes no async runtime. Its one direct
```

**E37. Generations** (`docs/adr/ADR-0021.md:314-324`). `generation-conflict`, and no claim on a pane before acting (C5).

```text
- A write whose expected generation is not the current one is `generation-conflict` and changes nothing. `delete` checks
  that the record exists first (`pane-not-found` or `profile-not-found`, whatever the generation) and the generation second.
- A verb takes its expected generation when it plans, and writes the record with it after the act. If another writer got in
  between, the verb's record write fails with `generation-conflict` **after** the live change: the verb fails loudly, exits 1,
  writes nothing more, and prints the reconcile step (the pane doctor command line; the profile-scoped or the bare form of
  step 6, which names no pane, for the reason given there). With `--profile` the record step runs inside the act, so a
  record conflict also restores P's specs (step 5) before the verb prints the reconcile step (#663, confirming the
  assumption of #638).
- **Generations fence records, not live acts.** Two verbs on the same pane can both act; only one records. The frozen
  `Pane` has no field to claim a pane before acting, so this ADR adds none; the reconcile pass (#647) is the safety net. A
  claim, if one is wanted, comes with the operation id (section 12).
```

**E38. The membership rule on `pane/cas_put`** (`docs/adr/ADR-0021.md:327-329`). O1.

```text
- No store transaction spans the two registries. The membership rule is enforced on `pane/cas_put`: setting `Pane.profile`
  to P when the stored pane already belongs to another profile is `pane-in-other-profile`, and P must exist. A spec that
  names a pane of another profile (a detached spec) is not refused.
```

**E39. I8 step 1** (`docs/adr/ADR-0021.md:334-337`). A `Set` for a pane of another profile is refused before anything is
written or moved (O1).

```text
1. **Plan.** Read P at generation `g` and the pane record; compute P with the edit (`SpecEdit::Set` or `SpecEdit::Remove`).
   A `Set` for a pane whose record belongs to another profile is refused here with `pane-in-other-profile`, before anything
   is written or moved, with or without `--spec-only` (the scope cannot see that an act is empty); a `Remove` is not (a
   detached spec stays removable), and the pane registry's check ("Decisions taken", item 2) stays the authority (#688).
```

**E40. The `pane doctor` and `profile apply` rows of section 9** (`docs/adr/ADR-0021.md:523` and `530`). Doctor's findings
are not errors; the apply row as it stands, which §10 rewords (NV-7, C8).

```text
| `pane doctor` | `pane-not-found`, `profile-not-found`, `pane-not-in-profile`; findings are kinds in `findings.rs` (#647), not errors, and include the reported `herdr-version-unsupported` |
```

```text
| `profile apply` | `profile-not-found`, `pane-in-other-profile` (without `--take-over`), `probe-failed` (per pane; the others proceed), `herdr-version-unsupported`, `grid-out-of-range`, `generation-conflict`, `profile-conflict`; open (#640) for a cell that no single split reaches, `grid-unreachable` |
```

**E41. The exit codes** (`docs/adr/ADR-0021.md:545-549`). The contract §5 and C8 sit in; a conflict between writers is a
failure (NV-7, C5).

```text
- **Exit codes are the same in both formats (operator, 2026-10-09):** 0 ok, 1 runtime failure, 2 usage, 3 refusal. A
  refusal is a request the system understood and declined, working as designed, so nothing was wrong with the system
  (`pane-in-other-profile`, `profile-secret-refused` and the like); this is [ADR 0003](ADR-0003.md)'s exit 3, a fail-closed
  policy refusal, carried to the new verbs. A failure is something that went wrong while doing the work (a timeout, an
  unreachable hub or adapter, a conflict between writers). A usage error under the `pane` or `profile` subcommands in JSON
```

**E42. The classes** (`docs/adr/ADR-0021.md:571` and `574-577`). The classes C3, C5 and C8 rely on.

```text
  | `profile-not-found` | Refusal (3) | The named profile does not exist; the request was understood and declined. |
```

```text
  | `generation-conflict` | Failure (1) | A race between writers; running the verb again can succeed. |
  | `profile-conflict` | Failure (1) | The profile moved after the live change (the I8 write order); a race. |
  | `timeout` | Failure (1) | The I5 bound ran out while doing the work. Its `op` names the port method that ran out, as `<port>.<method>` (`herdr.ensure_pane`, `harness.health`), in every implementation and fake, except that `launch` and `relaunch` name their own bound, `pane.launch` and `pane.relaunch` (section 12); an adapter's own wire steps do not appear in it. |
  | `unavailable` | Failure (1) | The hub, the Herdr socket or a harness cannot be reached (also a garbled reply), or the live state a verb needs is not there after its act (#644). |
```

**E43. Launch's whole-run budget** (`docs/adr/ADR-0021.md:663-670`). The run bound C4 builds on.

```text
**Decided (operator, 2026-10-11; proposed by #644): no operation id yet.** Neither option is built: both need frozen
files outside #644's reach (the `Pane` record or `PANE_METHODS`, and the hub), and the spikes show that a launch is short.
The OpenCode server is healthy 630-740 ms after it is started, a TUI shows its session in about 1.6-1.7 s and a restarted
server is healthy in about 645 ms, so a launch takes about 3 s of the 10 s bound. `pane launch` and `pane relaunch` meet
I5 with a budget for the whole run (10 s from when the engine starts, checked before every live step; once it has run out
the run rolls back and exits 1 with `timeout`, whose op is `pane.launch` or `pane.relaunch`), each port call's own bound and
the reconcile step. A run takes at most the budget plus one port call's own bound plus its rollback calls. If a measured
launch later exceeds the budget, options (a) and (b) stay on record and a new decision reopens this section.
```

**E44. Issue #802, as re-scoped on 2026-10-11** (its body, lines 1, 16, 18, 23, 26 and 47; state OPEN when read, with no
pull request, so no diff exists yet). Its plan for `tx_launch.rs` (E20) is wider than §10's note, it changes the test
kit's `FakeHarness`, and it edits ADR-0021 too (O3).

```text
Part of epic #633. Step 1 of the agent amendment (decision 8 of the epic, from pfleet#372) **and** the fix for #695 (a wedged harness server cannot be stopped). Re-scoped 2026-10-11 by the operator; this text replaces the earlier body (which only added `ServeSpec`). Where an older comment disagrees, this text is the decision.
```

```text
   - `ports.rs`: replace `HarnessPort::serve(&self, name, port) -> Result<u32, PaneError>` with `serve_command(&self, name: &PaneName, port: u16, spec: &ServeSpec) -> Result<Argv, PaneError>`. It is **pure**: it builds the argv the host runs (no process, no port bound, no file), so it needs no timeout and is trivially testable. It refuses `AGENT_UNSUPPORTED` when it cannot apply `spec.agent`; it never returns a command that would start a server without the agent it was asked for. Add `AGENT_UNSUPPORTED` (`RefusalCode`, `agent-unsupported`, exit 3) beside `PROMPT_UNSUPPORTED`.
```

```text
2. **Engine** (`crates/holler-pane/src/tx_launch.rs`, around lines 587 and 630-640): launch and relaunch both do `ensure_session`, then `argv = harness.serve_command(name, port, &plan.serve_spec)?`, then `host.run(name, &argv)?`, then wait for `harness.health(port)` inside the same budget (I5), with the existing rollback if it never answers. `stop_old` keeps `host.stop_owned(name)` then the "does the old port still answer" check; with the server inside the session, `stop_owned` now stops it, and a server that still answers after TERM, grace and KILL is a loud `unavailable` (no adoption), unchanged in spirit. The comment "until #695 stops a server by its recorded pid" is deleted.
```

```text
5. **`FakeHarness`:** `serve_command` returns a recognisable argv for a fake server; the fake host's `run` of that argv is what makes the fake server exist (so a test sees the same chain as production: `ensure_session`, `serve_command`, `run`, `health`). `FakeHarness` records the `ServeSpec` each `serve_command` received (readable by a test), and applies the agent (it does not refuse). The brief decides how the fake host and the fake harness are tied together (the smallest change that keeps `FakeHarness::server(port)` working for the existing tests) and quotes both fakes.
```

```text
8. **ADR-0021:** section 2 (lines ~108-114, "`HarnessPort` is final") and "`HarnessPort` as built (#642)" (section 8, ~line 402) record this as the second amendment since the freeze: `ServeSpec`, `serve_command` replacing `serve`, the server living in the session, `agent-unsupported`, decisions 1 to 4. Section 2 fact 7 (~line 134, "the agent is not applied yet") and the Deferred bullet (~line 742) say the port now carries the agent and #642 part 3 applies it. The `pid` field's new meaning is written in the record's section. `README.md` and `docs/` where a `harness.pid` or "the server is a process outside tmux" statement appears (grep).
```

```text
`crates/holler-pane/src/{ports.rs,serve_spec.rs,lib.rs,tx_launch.rs,pane.rs}` and `crates/holler-pane/tests/**`; `crates/holler-pane-testkit/**`; `crates/holler-adapter-opencode/{src,tests}/**`; `crates/holler-adapter-host/**` only if the TERM-ignoring test needs a helper there; `crates/holler-cli/src/pane/wiring.rs` and `crates/holler-cli/tests/pane_verbs/**` (call sites and doubles only); `docs/adr/ADR-0021.md`; `README.md` and `docs/` where they state the old server placement; `CHANGELOG.md`. No other crate. `tx_launch.rs` is 787 lines and `scripts/lint.sh` fails at 900: the engine edit is small and any growth goes to a sibling module.
```

**E45. `pane doctor`'s engine, and what makes it fail** (`crates/holler-pane/src/reconcile.rs:1-5` and `183-188`). The
doctor form C8 follows: an error only when the pass cannot run; everything observed is a finding.

```rust
//! The reconcile engine (#647): observe Herdr, tmux and the harness, compare what it saw with
//! the registry, repair what the record decides, and report the rest as typed
//! [`Finding`]s. It never infers state from files (I6), and it reaches the world only through
//! the [`Ports`], so `holler pane doctor` and any later caller (the roster, a periodic loop)
//! call [`reconcile`] unchanged.
```

```rust
/// Run one reconcile pass over `ports` (see the module docs).
///
/// It fails only when it cannot run the pass: a named pane or profile that does not exist
/// or a pane outside the profile (`pane-not-found`, `profile-not-found`,
/// `pane-not-in-profile`), or a pane store or profile scope that cannot be read. Everything
/// else the pass observes, a failed call included, is a finding of the report.
```

**E46. The failure-class errors a test injects** (`crates/holler-pane/src/error.rs:425-427`, `456-459` and `468-470`).
`PaneError::Conflict` is `generation-conflict`; `Timeout` and `Unavailable` are the errors C9's tests inject (C9, C10).

```rust
    /// `generation-conflict`: a compare-and-swap write named a generation the record
    /// has moved past (the CAS conflict; see [`crate::generation`]).
    Conflict,
```

```rust
    /// `timeout`: an operation did not return within the bound of I5 (default 10 s);
    /// `op` names the port method, as `<port>.<method>` (`herdr.ensure_pane`).
    /// (#638-#642.)
    Timeout { op: String },
```

```rust
    /// `unavailable`: something the verb needs cannot be reached: the hub, the Herdr
    /// socket, a harness. `what` names it. (#638-#642.)
    Unavailable { what: String },
```

**E47. The failure class and its exit code** (`crates/holler-pane/src/error.rs:239-245` and `292-301`, the second inside
`class_of`). Every code of E46 is exit class 1 (C9, C10).

```rust
    pub const fn exit_code(self) -> i32 {
        match self {
            ErrorClass::Usage => 2,
            ErrorClass::Refusal => 3,
            ErrorClass::Failure => 1,
        }
    }
```

```rust
        // Went wrong while doing the work: a race between writers, a bound that ran
        // out, something unreachable or unreadable, live state that disagrees with
        // its spec, or work the verb cannot do yet.
        PaneCode::GenerationConflict
        | PaneCode::ProfileConflict
        | PaneCode::Timeout
        | PaneCode::Unavailable
        | PaneCode::StoreCorrupt
        | PaneCode::NotImplemented
        | PaneCode::ProfileDrift => ErrorClass::Failure,
```

**E48. The fault switch's standing faults** (`crates/holler-pane-testkit/src/fault.rs:25-35`). A standing fault answers
every call of the port, and a wedged port answers each method with its own `op` (C9).

```rust
/// A standing fault: it applies to every call until it is cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// The port does not answer: every call fails with `PaneError::Timeout { op }`,
    /// where `op` is the method's [`PortOp::as_str`]. The fake answers at once, without
    /// waiting out I5's bound. Add [`FaultSwitch::set_delay`] to make a caller's own
    /// timer fire.
    Wedged,
    /// Every call fails with this error (e.g. `store-corrupt` or `unavailable`).
    Fail(PaneError),
}
```

**E49. The fake harness's `health` and its faults** (`crates/holler-pane-testkit/src/harness.rs:120-121` and `139-143`).
`Ok(false)` for a server that is not running; an error only through the fault switch (C9).

```rust
/// - `health(port)`: `true` when the server runs, `false` when it is frozen, killed or
///   never served, as a timed request that times out or is refused would tell.
```

```rust
/// Every port method first passes [`FakeHarness::faults`], which models a wedged
/// *adapter*: under `Fault::Wedged` every method answers `timeout`, `health` and
/// `shown_session` included. [`FakeHarness::freeze`] models a wedged *server* behind a
/// working adapter. A method that fails changes nothing. The configuration, scenario
/// and inspection methods bypass the faults and the call log.
```

**E50. The OpenCode adapter's `health`, and `server::healthy` whole** (`crates/holler-adapter-opencode/src/lib.rs:244-247`
and `crates/holler-adapter-opencode/src/server.rs:47-63`). It never answers `Err` (C9): `health` wraps `server::healthy`
in `Ok` and has no `?`, and `healthy` returns a `bool`, `true` only for a reply whose JSON is an object with
`"healthy": true`, so every `Err` of the request and every other reply are `false` (R6 NV-3).

```rust
    fn health(&self, port: u16) -> Result<bool, PaneError> {
        let timeouts = &self.config.timeouts;
        Ok(server::healthy(port, timeouts.health.min(timeouts.call)))
    }
```

```rust
/// Whether the server on `port` answers `GET /global/health` with `{"healthy": true}`
/// within `within`. Refused, timed out, garbled or anything else is `false`.
pub(crate) fn healthy(port: u16, within: Duration) -> bool {
    matches!(
        http::request(port, "GET", HEALTH, None, within),
        Ok(reply) if is_healthy(&reply)
    )
}

/// A `200` whose JSON is an object with `"healthy": true`.
fn is_healthy(reply: &Reply) -> bool {
    json_of(reply)
        .as_ref()
        .and_then(|health| health.get("healthy"))
        .and_then(Value::as_bool)
        == Some(true)
}
```

**E51. `pane doctor`'s test for a recorded Herdr pane** (`crates/holler-pane/src/reconcile.rs:326-347`). The pair of
session and pane id, as in `same_pane` (E21) (C9).

```rust
/// `herdr-pane-missing` for each pane of `scope` whose Herdr session and pane id the
/// snapshot lacks.
fn missing_herdr_panes(snapshot: &HerdrSnapshot, scope: &[Pane]) -> Vec<Finding> {
    let live: BTreeSet<(&str, &PaneId)> = snapshot
        .panes
        .iter()
        .map(|herdr| (herdr.session.as_str(), &herdr.pane_id))
        .collect();
    scope
        .iter()
        .filter(|pane| !live.contains(&(pane.herdr.session.as_str(), &pane.herdr.pane_id)))
        .map(|pane| {
            let message = format!(
                "Herdr has no pane {} in session {}, where the record puts this pane",
                quoted(pane.herdr.pane_id.as_str()),
                quoted(&pane.herdr.session)
            );
            Finding::about(FindingKind::HerdrPaneMissing, pane, message)
                .with_herdr_pane(&pane.herdr)
        })
        .collect()
}
```

**E52. The fake Herdr: one session, and its pane ids** (`crates/holler-pane-testkit/src/herdr.rs:112-122`). What the
fake's snapshot can lack (C9).

```rust
/// An in-memory `HerdrPort` serving one Herdr session.
///
/// A test declares each workspace with a size ([`FakeHerdr::with_workspace`]).
/// `ensure_pane` checks, in this order: the session and the workspace (one the fake
/// does not serve is `unavailable`), the range (a cell outside the workspace is
/// `grid-out-of-range`), an occupant (an occupied cell answers its pane and changes
/// nothing, in either placement), the placement (see [`Placement`]), and then it mints
/// the next id. An id is `w<N>:p<M>`: the workspace's number, then its pane counter in
/// base 36 (spike section 5). The counter only goes up, so an id is never reused after
/// a close or a vanish. `close` and [`FakeHerdr::vanish`] free a pane's cell, and no
/// other pane is ever moved, replaced or renumbered.
```

**E53. `undo`, the failure path of `profile create`'s `join`** (`crates/holler-cli/src/profile/create.rs:304-308`). It
ends by deleting the profile that `create` made, and a write that failed may have landed (C10, O4).

```rust
/// Undo a failed join of `failed` (#662 B2), stopping at the first step that fails: `failed`
/// leaves if its record shows it joined after all (a write that answered `timeout` or
/// `unavailable` may have landed); each pane in `joined` leaves, newest first; then the
/// profile is deleted at the generation it was stored at. Stopping at the first failure keeps
/// the profile whenever a member may remain, so the remedy the error names can always run.
```

**E54. ADR-0021 section 12 on I5 before an operation id** (`docs/adr/ADR-0021.md:648-650` and `660-661`). O5.

```text
**Decided:** the hub runs no adapters (ruling 1), so it executes no long work. The CLI process that runs a verb executes
every step, and every port call is bounded by I5 (default 10 s) or ends in `timeout`. A verb that times out stops,
compensates as section 8 says, exits 1 with `timeout`, and prints the reconcile step. A crash between steps leaves state that
```

```text
executor for long work only, which narrows ruling 1. Until one is chosen, I5 is satisfied by the per-call bound and the
loud `timeout`, not by an operation id.
```

**E55. `TxFailure`** (`crates/holler-pane/src/tx_launch.rs:151-158`). What a failed `create` or `relaunch` hands back to
apply (C10).

```rust
/// Why a run failed, and whether it got as far as the act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxFailure {
    pub error: PaneError,
    /// `edit_spec` entered the act, or a step after it failed: the CLI prints the reconcile
    /// step.
    pub acted: bool,
}
```

**E56. `GridPos`'s serde form, `SpecField`'s path and `FieldValue`** (`crates/holler-pane/src/grid.rs:13` and `129-138`;
`crates/holler-pane/src/profile_diff.rs:74-80`, `155-172` and `174-180`). How a cell that differs shows inside a step's
`diff` (C6, R3 NIT-2).

```rust
//! - **Output**: text `r2c1`; JSON `{"row":2,"col":1,"pos":"r2c1"}`, row first.
```

```rust
impl Serialize for GridPos {
    /// `{"row":R,"col":C,"pos":"rRcC"}`, in that key order.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer.serialize_struct("GridPos", 3)?;
        out.serialize_field("row", &self.row)?;
        out.serialize_field("col", &self.col)?;
        out.serialize_field("pos", &self.to_string())?;
        out.end()
    }
}
```

```rust
    /// The dotted path of the field in a `ProfileSpec`'s JSON (`"herdr.grid"`), which is
    /// also its serde form.
    pub const fn as_str(self) -> &'static str {
        match self {
            SpecField::Workspace => "herdr.workspace",
            SpecField::Grid => "herdr.grid",
            SpecField::Cwd => "host.cwd",
```

```rust
/// One field's value, typed so that it serializes exactly as the field does inside a
/// `ProfileSpec` (a grid through `GridPos`'s own `Serialize`, row first) and prints for a
/// person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    /// `herdr.workspace`, `host.cwd`, `harness.kind`, `harness.port_policy`, the three
    /// `model` fields and `role`.
    Text(String),
    /// `herdr.grid`.
    Grid(GridPos),
    /// `context.soft`, `context.hard`.
    Number(u32),
    /// `env` (the names) and `expect`, in stored order.
    List(Vec<String>),
    /// `command`, `check`; `None` serializes as `null`.
    Argv(Option<Argv>),
}
```

```rust
impl fmt::Display for FieldValue {
    /// The text form:
    ///
    /// - `Text` with every control character escaped by `char::escape_default`, so ESC
    ///   prints as `\u{1b}` and a newline as `\n` (the rule Holler's text log lines
    ///   escape a field value with, in `holler_proto::log`); every other character as is.
    /// - `Grid` as `r2c1`, and `Number` in decimal.
```

**E57. The fakes' call log, and the pane store's `watch`** (`crates/holler-pane-testkit/src/pane_store.rs:20-31`;
`crates/holler-pane-testkit/src/profile_store.rs:24-37`; `crates/holler-pane-testkit/src/fault.rs:85-88`;
`crates/holler-pane/src/ports.rs:39-45`; `crates/holler-pane/src/pane.rs:261-273`;
`crates/holler-pane-testkit/src/feed.rs:14-16` and `26`). A call log records each call's method alone (an `Op` such as
`PaneStoreOp::CasPut`, which carries no argument), so it counts writes; the fake profile store's log names its methods
the same way, as `ProfileStoreOp`s (R4 W-2); the watch yields every change after a cursor, in order, with the record
after it, and the fake keeps its whole history (C7: R3 NV-1, R3 NV-2, R3 NV-4).

```rust
/// A method of the `PaneStore` port, as a fault targets it and the call log records
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneStoreOp {
    Get,
    List,
    CasPut,
    Delete,
    Watch,
    /// One `next()` of an open watch.
    WatchNext,
}
```

```rust
/// A method of the `ProfileStore` port, as a fault targets it and the call log
/// records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileStoreOp {
    Get,
    List,
    CasPut,
    Delete,
    Watch,
    /// One `next()` of an open watch.
    WatchNext,
    Log,
    Rename,
}
```

```rust
    /// Every call made through the port, oldest first, the failed ones included.
    pub fn calls(&self) -> Vec<Op> {
        self.lock().calls.clone()
    }
```

```rust
/// The stream `watch` returns: changes in order, each carrying its [`Cursor`].
///
/// - `watch(since)` yields every change after `since`. `Cursor(0)` starts from the
///   beginning: the store first yields a put for every record it holds now (the
///   current state), then every later change.
/// - Passing the cursor of the last event seen back as `since` resumes without a
///   gap or a repeat.
```

```rust
/// One change to the pane store, as `PaneStore::watch` yields it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaneEvent {
    /// The store-wide sequence number of this change; hand it back as the `since`
    /// of the next watch to resume without a gap or a repeat.
    pub cursor: Cursor,
    /// The pane the change concerns.
    pub name: PaneName,
    /// The record after the change; `None` when the record was deleted.
    #[serde(default)]
    pub pane: Option<Box<Pane>>,
}
```

```rust
//! Every change takes the next cursor, `head + 1`. An overflow is `store-corrupt` and
//! changes nothing. Unlike the hub's registry, the feed keeps its whole history, so it
//! never collapses writes.
```

```rust
//! 3. From any other cursor: every change after it, in order.
```

**E58. `TxOptions`, and the options the `pane` verbs pass** (`crates/holler-pane/src/tx_launch.rs:93-111`;
`crates/holler-cli/src/pane/launch.rs:78`; `crates/holler-cli/src/pane/relaunch.rs:55`). A run's budget is its caller's
to set, and the verbs pass the default (C13).

```rust
#[derive(Debug, Clone, Copy)]
pub struct TxOptions {
    /// The budget of the whole run ([`DEFAULT_BUDGET`]).
    pub budget: Duration,
    /// The timeout of the health probe ([`PROBE_TIMEOUT`]).
    pub probe_timeout: Duration,
    /// The wall clock stamped into `last_observed.at` (`holler_proto::clock::now_millis`).
    pub now_ms: fn() -> i64,
}

impl Default for TxOptions {
    fn default() -> Self {
        Self {
            budget: DEFAULT_BUDGET,
            probe_timeout: PROBE_TIMEOUT,
            now_ms: holler_proto::clock::now_millis,
        }
    }
}
```

```rust
            let result = launch(ctx.ports, &request, &TxOptions::default());
```

```rust
            let result = relaunch(ctx.ports, &request, &TxOptions::default());
```

**E59. `set_delay`, and #644's bound test** (`crates/holler-pane-testkit/src/fault.rs:79-83`;
`crates/holler-cli/tests/pane_verbs/launch.rs:529-563`). The pattern C7's R3 B-4 tests follow: a short budget, a slowed
fake, a `timeout` that names `pane.launch`, inside a generous bound (C13).

```rust
    /// Make every call sleep for `delay` before it answers, as a slow port does
    /// (`None`: no delay).
    pub fn set_delay(&self, delay: Option<Duration>) {
        self.lock().delay = delay;
    }
```

```rust
/// AC 24 (I5): a run whose budget runs out stops before the next step, rolls back and answers
/// `timeout`, well inside a generous bound.
#[test]
fn the_budget_bounds_a_slow_launch() {
    let rig = Rig::new();
    let request = LaunchRequest {
        name: name(PANE),
        herdr_session: Some("scratch".to_owned()),
        spec: launch_spec(),
        profile: None,
        spec_only: false,
    };
    let options = TxOptions {
        budget: Duration::from_millis(800),
        ..TxOptions::default()
    };
    rig.fakes
        .harness
        .faults()
        .set_delay(Some(Duration::from_millis(500)));
    let started = Instant::now();
    let (answer, calls) = rig.engine(|ports| tx_launch::launch(ports, &request, &options));
    let took = started.elapsed();
    rig.fakes.harness.faults().set_delay(None);
    let timeout = PaneError::Timeout {
        op: "pane.launch".to_owned(),
    };
    assert_eq!(
        answer,
        Err(TxFailure {
            error: timeout,
            acted: true
        })
    );
    assert!(took < Duration::from_secs(3), "bounded: {took:?}");
```

**E60. The fake harness's `seed_session` and `navigate`** (`crates/holler-pane-testkit/src/harness.rs:218-223` and
`233-243`). How C7's R3 B-5 test puts a TUI on its home screen or on another session.

```rust
    /// Another client created a session in the data directory of `port`: a stray
    /// "ping" session, or one a person started by hand on a TUI's home screen. It needs
    /// no running server. Returns its id.
    pub fn seed_session(&self, port: u16) -> String {
        self.lock().mint_session(port)
    }
```

```rust
    /// A person moved the TUI in `pane` by hand: `Some(id)` shows that session, `None`
    /// goes to the home screen. `unavailable` when there is no TUI in `pane`;
    /// `session-not-found` when the data directory of the TUI's port lacks the id.
    pub fn navigate(&self, pane: &PaneId, session: Option<&str>) -> Result<(), PaneError> {
        let mut world = self.lock();
        let port = world.tui_port(pane)?;
        if let Some(session) = session {
            world.known(port, session)?;
        }
        world.show(pane, session)
    }
```

**E61. The `--format=json` envelope, and a failure in text mode** (`docs/adr/ADR-0021.md:533-534`, `537-538`, `541-543`
and `550`; `crates/holler-cli/src/output.rs:150-160` and `281-284`). A verb's `data` is the envelope's `data` itself; it
is `null` whenever `ok` is false, which it is at exit 1, 2 and 3; in text mode an error goes to stderr alone (C1, C6, O6;
R4 B-1, R4 NIT-2).

```text
**Decided (decision 6): the `--format=json` envelope.** Every pane and profile verb and the roster take
`--format=text|json` (default text). In JSON mode stdout carries exactly one envelope, and every diagnostic goes to stderr:
```

```json
{"schema_version": 1, "ok": true, "data": {}, "error": null}
{"schema_version": 1, "ok": false, "data": null, "error": {"code": "pane-not-found", "message": "pane not found: hj-c1r1"}}
```

```text
- `error` is `null` exactly when `ok` is true. `error.code` is the `PaneError` code verbatim and `error.message` its one-line
  text; the envelope has no `detail`. Each verb supplies only its `data`. The types and `emit()`, `emit_stream()` and
  `emit_usage_error()` are in `holler-cli/src/output.rs` (#670 creates it, #660 completes it).
```

```text
  mode is an envelope with code `usage` and exit 2. In JSON mode `ok` is false for exit 1, 2 and 3 alike. Which `PaneError`
```

```rust
/// The one envelope of every `holler pane` and `holler profile` result in JSON mode.
///
/// Keys are written in this order: `schema_version`, `ok`, `data`, `error`. `data` is `null` when
/// `ok` is false and `error` is `null` when it is true.
#[derive(Debug, Serialize)]
pub struct Envelope<T> {
    pub schema_version: u32,
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<ErrorBody>,
}
```

```rust
        Err(error) => {
            let written = write_line(sink.err, &format!("error: {}", error.message));
            settle(written, exit_code(&error))
        }
```

**E62. The test kit's envelope checker** (`crates/holler-pane-testkit/src/envelope.rs:1-2`, `25-28` and `135-136`). What
"parses against the envelope" checks: any `data` on success, and `null` on failure (C6, O6; R4 NIT-2).

```rust
//! The JSON-envelope checker that the `--format=json` tests of every `holler pane` and
//! `holler profile` verb reuse (ADR-0021 section 9).
```

```rust
//! 6. `ok` is the boolean `true` when the exit code is 0, and `false` otherwise.
//! 7. On success, `error` is `null`. `data` may be any value, `null` included.
//! 8. On failure, `error` is an object.
//! 9. On failure, `data` is `null`.
```

```rust
    /// `ok` is false, but `data` is not `null`.
    DataOnFailure,
```

**E63. `pane doctor`'s exit codes** (`crates/holler-cli/src/pane/doctor.rs:4-9`). The doctor form §5 names: a pass that
completes exits 0, because an envelope cannot carry data with a failure (O6; R4 B-3).

```rust
//! - **Exit codes.** A pass that completes exits 0, with or without findings: findings are
//!   kinds in the report, not errors (ADR-0021 section 9), and an envelope cannot carry data
//!   with a failure, so a script reads `data.findings`. The verb fails only when it cannot
//!   run the pass: a bad `PANE` or `--profile` value is `usage` (2); a pane or profile that
//!   does not exist, or a pane outside the profile, is a refusal (3); a pane store or profile
//!   scope that cannot be read is a failure (1).
```

**E64. The fake harness's TUIs** (`crates/holler-pane-testkit/src/harness.rs:93-100`, `150-157`, `319-330` and
`459-472`; `crates/holler-pane/src/ports.rs:199-200`; `crates/holler-pane/src/tx_launch.rs:638-640`). How C7's R3 B-5
test reaches a home screen (R4 NV-4). The fake's `World` starts with no TUI (`#[derive(Default)]`), and `attach_tui`, a
`HarnessPort` method that the launch transaction's act calls, is the one place the fake adds a TUI, showing the session it
was attached to. For a pane with no TUI, `navigate` (E60) answers `unavailable` through `tui_port`; otherwise `show` sets
the TUI's `shown`, to `None` for the home screen, and `shown_session` returns it as it is (E25), so `Ok(None)`.

```rust
/// What [`FakeHarness::tui`] reports about the TUI in a pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiView {
    /// The port of the server it is attached to.
    pub port: u16,
    /// The session it shows; `None` on its home screen.
    pub shown: Option<String>,
}
```

```rust
#[derive(Default)]
struct World {
    servers: HashMap<u16, ServerView>,
    /// The ports given a data directory of their own.
    dirs: HashMap<u16, String>,
    /// The session ids of each data directory, in creation order.
    sessions: HashMap<String, Vec<String>>,
    tuis: HashMap<PaneId, TuiView>,
```

```rust
    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
        self.faults.enter(HarnessOp::AttachTui)?;
        let mut world = self.lock();
        world.reach(port, HarnessOp::AttachTui)?;
        world.known(port, session)?;
        let tui = TuiView {
            port,
            shown: Some(session.to_owned()),
        };
        world.tuis.insert(pane.clone(), tui);
        Ok(())
    }
```

```rust
    /// The port of the TUI in `pane`; the no-TUI `unavailable` when there is none.
    fn tui_port(&self, pane: &PaneId) -> Result<u16, PaneError> {
        self.tuis
            .get(pane)
            .map(|tui| tui.port)
            .ok_or_else(|| no_tui(pane))
    }

    /// The TUI in `pane` shows `session` (`None`: its home screen).
    fn show(&mut self, pane: &PaneId, session: Option<&str>) -> Result<(), PaneError> {
        let tui = self.tuis.get_mut(pane).ok_or_else(|| no_tui(pane))?;
        tui.shown = session.map(str::to_owned);
        Ok(())
    }
```

```rust
    /// Attach the TUI of `pane` to `session` on the server at `port`.
    fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError>;
```

```rust
    ports
        .harness
        .attach_tui(&herdr.pane_id, plan.port, &session)?;
```

**E65. How the fake pane store stores a write, and `next_generation`**
(`crates/holler-pane-testkit/src/pane_store.rs:127-148` and `187-190`; `crates/holler-pane-testkit/src/feed.rs:218-238`;
`crates/holler-pane/src/generation.rs:18-34`). A `cas_put` that lands is stored as it was submitted, with only its
generation replaced, one higher than the stored one; at a stale generation it is `generation-conflict` and stores nothing
(C7: R5 NV-6; C17).

```rust
    /// The one write path of a put (`cas_put`, `seeded`, `concurrent_put`): the
    /// stored record is `pane` with only its generation replaced.
    fn put(&self, pane: &Pane, writer: Writer) -> Result<Pane, PaneError> {
        self.feed.write(|log| {
            let stored = log.get(&pane.name);
            let current = stored.map_or(0, |stored| stored.generation);
            let generation = next_generation(current, writer.expected(current))?;
            if let Writer::Port(_) = writer {
                check_membership(stored, pane)?;
            }
            let next = Pane {
                generation,
                ..pane.clone()
            };
            log.append(|cursor| PaneEvent {
                cursor,
                name: next.name.clone(),
                pane: Some(Box::new(next.clone())),
            })?;
            Ok(next)
        })
    }
```

```rust
    fn cas_put(&self, pane: &Pane, expected_generation: u64) -> Result<Pane, PaneError> {
        self.faults.enter(PaneStoreOp::CasPut)?;
        self.put(pane, Writer::Port(expected_generation))
    }
```

```rust
/// Who makes a write to a fake store.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Writer {
    /// A caller of the port, or a seed: a compare-and-swap at the generation it read,
    /// and the store's port-only rule (membership for panes, the name rule for
    /// profiles).
    Port(u64),
    /// Another writer (`concurrent_*`): applied to whatever is stored now, without the
    /// store's port-only rule.
    Other,
}

impl Writer {
    /// The generation the write expects, the record being at `current` now.
    pub(crate) fn expected(self, current: u64) -> u64 {
        match self {
            Writer::Port(expected) => expected,
            Writer::Other => current,
        }
    }
}
```

```rust
/// The generation a record moves to when a write that read `expected` is applied
/// to a record now at `current`.
///
/// `Err(PaneError::Conflict)` when `expected != current`: the caller read an older
/// version and must read again before retrying. The counter cannot realistically
/// overflow; if it ever did the store is not trustworthy, so that is
/// `store-corrupt` rather than a wrap.
pub fn next_generation(current: u64, expected: u64) -> Result<u64, PaneError> {
    if current != expected {
        return Err(PaneError::Conflict);
    }
    current
        .checked_add(1)
        .ok_or_else(|| PaneError::StoreCorrupt {
            what: "a generation counter overflowed".to_owned(),
        })
}
```

**E66. The rest of `join`** (`crates/holler-cli/src/profile/create.rs:282-302`). With E31 (lines 266-282), `join` whole:
one pane's join answers the stored record or the `cas_put`'s error, after which `profile create` runs `undo` (E53), which
the twin does not (C10, C17; R5 NV-6).

```rust
        match pane_store.cas_put(&member, pane.generation) {
            Ok(stored) => joined.push(stored),
            Err(error) => {
                let undone = undo(
                    pane_store,
                    profile_store,
                    profile,
                    &pane.name,
                    &joined,
                    actor,
                );
                return Err(JoinFailed {
                    pane: pane.name.clone(),
                    error,
                    undo_error: undone.err(),
                });
            }
        }
    }
    Ok(joined.into_iter().map(|pane| pane.name).collect())
}
```

**E67. The rest of `run`, and `act_live`** (`crates/holler-pane/src/tx_launch.rs:523-566`). With E18 (lines 504-522),
`run` and its act whole: the act's `cas_put` (E19) is the run's one pane-store write (C11; R5 NV-2).

```rust
    let profile = edited.map_err(|error| TxFailure {
        error,
        acted: entered,
    })?;
    let Some(acted) = acted else {
        let what = format!(
            "the profile scope did not run the live change of {}",
            plan.name
        );
        return Err(unavailable(what).into());
    };
    match acted.unclosed {
        None => Ok(Launched {
            pane: Some(acted.stored),
            profile,
        }),
        Some(error) => Err(TxFailure { error, acted: true }),
    }
}

/// The act of both verbs: B1 and B2 for a relaunch, then A1 to O2 (B3 to O2), the record (R)
/// and, after a relaunch's move, B10. A step before R that fails, or a budget that runs out
/// before one, rolls back what this run made; R's own failure is returned as it is.
fn act_live(
    ports: Ports<'_>,
    plan: &Plan<'_>,
    seen: &Seen,
    budget: &Budget,
    now_ms: fn() -> i64,
) -> Result<Acted, PaneError> {
    if let Some(old) = plan.old {
        stop_old(ports, plan.name, old, budget)?;
    }
    let mut made = Made::default();
    let live = bring_up(ports, plan, seen, budget, &mut made)
        .map_err(|error| roll_back(ports, plan.name, &made, error))?;
    let expected = plan.old.map_or(0, |old| old.generation);
    let record = plan.record(seen, live, now_ms());
    let stored = ports.pane_store.cas_put(&record, expected)?;
    let unclosed = plan
        .old
        .and_then(|old| close_old(ports, plan, seen, old, &stored.herdr));
    Ok(Acted { stored, unclosed })
}
```

**E68. The scopes write no pane record** (`crates/holler-cli/src/pane/profile_scope.rs:14-22`;
`crates/holler-pane-testkit/src/profile_scope.rs:166-170` and `207-231`). `edit_spec` writes P before the act and, when
the act fails, puts P's specs back; recording the pane is the act's (C11; R5 NV-2).

```rust
//! 2. *Write P first*, by one compare-and-swap at g: a conflict is `generation-conflict` and the
//!    act never runs. A `timeout` may have landed, so its message says so and carries the
//!    reconcile step; any other error passes through as it is.
//! 3. *Act*, once. The scope writes no pane record: recording the pane is the verb's, inside its
//!    act, so a pane-record conflict is an act failure like any other.
//! 4. *If the act fails*, put P's specs back by one compare-and-swap at g + 1, not retried, and
//!    answer the act's own error (P's generation has then moved by two). A conflict there is
//!    `profile-conflict`; any other error keeps its own code. Both messages name P, the pane, the
//!    act's error and the reconcile step, on one line (the fake's three messages are shorter).
```

```rust
        // ASSUMPTION (#663): a restore is one compare-and-swap at g + 1, not retried.
        match self
            .profiles
            .cas_put(&restored, restored.generation, &self.actor)
        {
```

```rust
    // ASSUMPTION (#663): the scope writes no pane record. Recording the pane is the
    // verb's, inside its act, whose signature returns `()`.
    fn edit_spec(
        &self,
        profile: Option<&ProfileName>,
        pane: &PaneName,
        edit: &SpecEdit,
        act: &mut dyn FnMut() -> Result<(), PaneError>,
    ) -> Result<Option<Profile>, PaneError> {
        // ASSUMPTION (#663): without a profile the scope makes no store call at all.
        let Some(profile) = profile else {
            return act().map(|()| None);
        };
        // ASSUMPTION (#663): `profile-not-found` comes before `pane-in-other-profile`.
        let stored = self.stored(profile)?;
        let edited = self.plan(&stored, pane, edit)?;
        // ASSUMPTION (#663): the first write is not retried on a conflict.
        let written = self
            .profiles
            .cas_put(&edited, stored.generation, &self.actor)?;
        match act() {
            Ok(()) => Ok(Some(written)),
            Err(failure) => Err(self.restore(stored, written, pane, failure)),
        }
    }
```

**E69. The fault switch's `enter`, and the fake harness's `lock`** (`crates/holler-pane-testkit/src/fault.rs:90-107` and
`116-134`; `crates/holler-pane-testkit/src/harness.rs:270-272`). `enter` answers `Err` only with the standing fault or the
oldest error `fail_next` queued for the method; each `lock` takes over a poisoned lock rather than failing, so E25's
`health` has no other source of `Err` (C9; R5 NV-4).

```rust
    /// What a fake calls first in every port method. It records the call, sleeps for
    /// the delay (without holding the lock, so other calls proceed), and then answers
    /// the standing fault if there is one, or else the oldest error queued for `op`.
    pub(crate) fn enter(&self, op: Op) -> Result<(), PaneError> {
        let delay = {
            let mut state = self.lock();
            state.calls.push(op);
            state.delay
        };
        if let Some(delay) = delay {
            thread::sleep(delay);
        }
        self.lock().take_fault(op)
    }

    fn lock(&self) -> MutexGuard<'_, State<Op>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
```

```rust
impl<Op: PortOp> State<Op> {
    /// The error a call of `op` answers now: the standing fault, else the oldest
    /// one-shot error queued for `op`, which this consumes.
    fn take_fault(&mut self, op: Op) -> Result<(), PaneError> {
        match &self.standing {
            Some(Fault::Wedged) => {
                return Err(PaneError::Timeout {
                    op: op.as_str().to_owned(),
                })
            }
            Some(Fault::Fail(error)) => return Err(error.clone()),
            None => {}
        }
        match self.queued.iter().position(|(queued, _)| *queued == op) {
            Some(at) => Err(self.queued.remove(at).1),
            None => Ok(()),
        }
    }
}
```

```rust
    fn lock(&self) -> MutexGuard<'_, World> {
        self.world.lock().unwrap_or_else(PoisonError::into_inner)
    }
```

**E70. The fake harness's `shown_session` in the other cases** (`crates/holler-pane-testkit/src/harness.rs:135-137`,
`225-231`, `245-252` and `442-457`). A deleted session sends a TUI to its home screen, a TUI that exits leaves no TUI,
and a frozen or dead server leaves its TUI's screen as it was (C12; R5 NV-5).

```rust
/// - `shown_session(pane)`: the session the TUI shows, or `None` on its home screen or
///   with no TUI. It does not reach the server: a TUI keeps its screen while its server
///   is frozen or dead (opencode-pane-spike.md:193-194).
```

```rust
    /// Another client deleted `session` (`DELETE /session/:id`): it leaves its data
    /// directory, and every TUI showing it goes to its home screen and stays attached
    /// (opencode-pane-spike.md:222). `session-not-found` when no data directory holds
    /// it.
    pub fn delete_session(&self, session: &str) -> Result<(), PaneError> {
        self.lock().delete(session)
    }
```

```rust
    /// The TUI process in `pane` exited. `unavailable` when there is no TUI in `pane`.
    pub fn close_tui(&self, pane: &PaneId) -> Result<(), PaneError> {
        self.lock()
            .tuis
            .remove(pane)
            .map(drop)
            .ok_or_else(|| no_tui(pane))
    }
```

```rust
    /// `delete_session`'s work: the session leaves its data directory, and every TUI
    /// that shows it goes home.
    fn delete(&mut self, session: &str) -> Result<(), PaneError> {
        let ids = self
            .sessions
            .values_mut()
            .find(|ids| ids.iter().any(|id| id == session))
            .ok_or_else(|| unknown_session(session))?;
        ids.retain(|id| id != session);
        for tui in self.tuis.values_mut() {
            if tui.shown.as_deref() == Some(session) {
                tui.shown = None;
            }
        }
        Ok(())
    }
```

**E71. #700's non-goals** (`docs/handoffs/0700-implementation/brief.md:90-95`). #700 recorded the
`profile_diff`/`profile show` comparison of its new field as outside its scope, naming #647 (C18; R5 NIT-2).

```text
## Non-goals (recorded, not silent)

- Applying the key anywhere: no `HarnessPort` change, no OpenCode adapter change, no
  `create_session`/`serve` parameter (#642).
- Validating the key against a project's `.opencode/agents/` (#644's last part).
- `profile_diff`/`profile show` comparison and the `agent-cannot-dispatch` doctor code (#647).
```

**E72. An operator comment on #647** (GitHub issue #647: the first paragraph of the comment that begins "Parked
(operator, 2026-10-11)", as `gh issue view 647 --comments` returned it at 06:04 MDT on 2026-10-11; #647 was open when
read). The `SpecField` row for `opencode_agent` is among what #647's parked part 2 must settle (C18; R5 NIT-2).

```text
Parked (operator, 2026-10-11): the two open amendments, `agent-cannot-dispatch` and `unattached`, wait until after the live cutover (#654), when the live fleet shows which of these incidents recur. Part 1 (the reconcile engine and `pane doctor`, #701) is merged and is what #649 and #648 build on. When this resumes, a part-2 brief must settle what the reviewer found: a read-only, default-armed port method for each new read (following #645's seam and #802's `ServeSpec` amendment), the `HerdrSnapshot` amendment the ADR defers for `unattached`, a cure for `unattached` that is a Holler verb, the `SpecField` row for `opencode_agent` (#700), and a fresh handoff slug so part 1's record is not overwritten.
```

**E73. `Budget`** (`crates/holler-pane/src/tx_launch.rs:213-234`). A run's budget: `start` reads the clock once, and
`check` compares the elapsed time with the limit and makes no port call, so a check never cuts a call short. `launch`
and `relaunch` start one when they are entered (E14), and the act checks it before each live step (E75) (C4; R6 NV-2).

```rust
/// The budget of one run, started when the engine is entered.
struct Budget {
    started: Instant,
    limit: Duration,
    op: &'static str,
}

impl Budget {
    fn start(limit: Duration, op: &'static str) -> Self {
        let started = Instant::now();
        Self { started, limit, op }
    }

    /// `timeout` once the budget has run out; a comparison of elapsed time, no port call.
    fn check(&self) -> Result<(), PaneError> {
        if self.started.elapsed() < self.limit {
            return Ok(());
        }
        let op = self.op.to_owned();
        Err(PaneError::Timeout { op })
    }
}
```

**E74. `Plan`, `Plan::new`, and the rest of `record`** (`crates/holler-pane/src/tx_launch.rs:236-246`, `259-284` and
`323-348`). The plan keeps the profile it is given as its `profile` (line 282), and the record `record` builds carries
in its `profile` field the value E16 computes (line 348), with the old record's generation (line 326) (C11; R6 NV-1).

```rust
/// What a live run is asked to bring up.
struct Plan<'a> {
    name: &'a PaneName,
    spec: &'a ProfileSpec,
    /// Where the pane goes: the Herdr session, and the spec's workspace and grid.
    target: HerdrSpec,
    port: u16,
    /// The record a relaunch replaces; `None` for a launch.
    old: Option<&'a Pane>,
    profile: Option<&'a ProfileName>,
}
```

```rust
impl<'a> Plan<'a> {
    /// The plan of a run that puts `name` in the Herdr session `session`, at the spec's cell, on
    /// the port its policy names (`usage` when it names none).
    fn new(
        name: &'a PaneName,
        spec: &'a ProfileSpec,
        session: String,
        old: Option<&'a Pane>,
        profile: Option<&'a ProfileName>,
    ) -> Result<Self, PaneError> {
        let port = port_of_policy(&spec.harness.port_policy)?;
        let (workspace, grid) = (spec.herdr.workspace.clone(), spec.herdr.grid);
        let target = HerdrSpec {
            session,
            workspace,
            grid,
        };
        Ok(Self {
            name,
            spec,
            target,
            port,
            old,
            profile,
        })
    }
```

```rust
        let spec = self.spec;
        Pane {
            name: self.name.clone(),
            generation: self.old.map_or(0, |old| old.generation),
            herdr: live.herdr,
            host: HostInfo {
                name: HOST_NAME.to_owned(),
                tmux: self.name.as_str().to_owned(),
                cwd: spec.host.cwd.clone(),
                herdr_api_version: Some(seen.herdr_api_version.clone()),
            },
            harness: HarnessInfo {
                kind: spec.harness.kind,
                port: self.port,
                pid: Some(live.pid),
                health: Health::Healthy,
            },
            session_of_record: Some(live.session),
            role: spec.role,
            hold,
            last_observed: LastObserved {
                shown: live.shown,
                driven,
                at,
            },
            profile,
```

**E75. `stop_old`, and `bring_up`'s server steps** (`crates/holler-pane/src/tx_launch.rs:568-588` and `627-640`). The
lines #802 names, "around lines 587 and 630-640" (E44): `stop_old`, whose doc comment holds the "until #695" that #802
deletes, and the `serve` call (line 628, E20) with the health check after it, which #802 replaces. Every live step
follows a budget check (E73). This story edits none of these lines (C20; R6 NV-6, R6 NV-2).

```rust
/// B1 and B2: stop what the pane owns, then check that its old server no longer answers. Until
/// #695 stops a server by its recorded pid, one that survives fails the run, loudly, never by
/// adopting it.
fn stop_old(
    ports: Ports<'_>,
    name: &PaneName,
    old: &Pane,
    budget: &Budget,
) -> Result<(), PaneError> {
    budget.check()?;
    ports.host.stop_owned(name)?;
    budget.check()?;
    let port = old.harness.port;
    if ports.harness.health(port)? {
        return Err(unavailable(format!(
            "the harness server on port {port} still answers after the processes of {name} \
             were stopped (#695)"
        )));
    }
    Ok(())
}
```

```rust
    budget.check()?;
    let pid = ports.harness.serve(plan.name, plan.port)?;
    budget.check()?;
    if !ports.harness.health(plan.port)? {
        let port = plan.port;
        let what = format!("the harness server on port {port} is not healthy after it was started");
        return Err(unavailable(what));
    }
    budget.check()?;
    let session = session_of_record(ports, plan)?;
    budget.check()?;
    ports
        .harness
        .attach_tui(&herdr.pane_id, plan.port, &session)?;
```

**E76. The fake pane store's feed and `watch`** (`crates/holler-pane-testkit/src/pane_store.rs:76-79` and `197-200`).
The store holds one feed, to which `put` appends each write that lands (E65), and `watch` reads that same feed, so
every `cas_put` that lands, a relaunch's record write included, is an event of the watch, in order (C17; R6 NV-4).

```rust
pub struct FakePaneStore {
    feed: Arc<Feed<PaneEvent>>,
    faults: Arc<FaultSwitch<PaneStoreOp>>,
}
```

```rust
    fn watch(&self, since: Cursor) -> Result<Watch<PaneEvent>, PaneError> {
        self.faults.enter(PaneStoreOp::Watch)?;
        Feed::watch(&self.feed, since, &self.faults, PaneStoreOp::WatchNext)
    }
```
