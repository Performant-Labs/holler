# Handoff-F: Phase 6 (F) - `holler profile apply` (#664)

**Date:** 2026-10-11 (07:48 MDT)
**Branch:** `issue-664-implementation` (head `e773945`, T-red's commit, which merges `origin/main` `263a2d7`; `main`
has not moved since)
**Issue:** #664

## What was done

- `crates/holler-pane/src/tx_apply.rs`: the engine, which replaces #637's stub.
  - The module doc covers the rules and the run's bound in `tx_launch`'s **Bound (I5).** form (C4, C16).
  - The public API that T's tests pin: `apply`, `ApplyRequest`, `Applied`, `Step`, `Action`, `Outcome`.
  - Two open codes: `ORCHESTRATOR_NOT_RELAUNCHED` and `SPEC_NOT_LAUNCHABLE`.
  - The §4 gate, the plan (rows a to f in grid order, from `diff_profile`'s rows) and the stop-at-first-failed loop.
- `crates/holler-pane/src/tx_apply/observe.rs` (new): the up-front observation and §2's health judgment.
  - Scoped threads, one per call: Herdr's snapshot, and each recorded spec pane's `health` and `shown_session`.
  - The judgment uses the `(session, pane_id)` pair (never the cell or the label), and C12's shown-session table.
- `crates/holler-pane/src/tx_apply/run.rs` (new): one live step.
  - It first re-reads the record and P.
  - Create, adopt, report and relaunch each go through `tx_launch` unchanged.
  - The take-over move: leave, then join.
  - The two private twins, `leave` and `join`, of `detach` and of `join`'s write for one pane.
- `crates/holler-cli/src/profile/apply.rs`: the verb, which replaces #670's stub.
  - It takes `NAME`, `--dry-run`, `--take-over` and `--herdr-session`, and calls the engine with `TxOptions::default()`.
  - At exit 0 it prints the steps object.
  - At exit 1 it prints only the failure, in O6's pinned form, plus the reconcile step when the act had started.
  - The text form uses `profile show`'s diff lines.
- `crates/holler-cli/src/profile/show.rs`: `difference_line` and `text` go from private to `pub(crate)`, and the
  module doc says so (see "Deviations").
- `crates/holler-cli/tests/fixtures/cli-surface.txt`: #664's three lines now carry `NAME` and cover every flag. This
  is ADR 0003's machine-readable table, which §10 and T's handoff assign to F.
- `docs/adr/ADR-0003.md`: the `profile apply` row is now
  `holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]`.
- `docs/adr/ADR-0021.md`, two edits (§10):
  - Section 9's `profile apply` row is reworded to the doctor form and lists the step codes, including the new open
    codes.
  - A new paragraph in section 8, "Apply as built (#664)", **defines "adopt"** against the ADR's existing "never
    adopted" sentences.
- `docs/research/herdr-api-spike.md:280`: the inline span `holler profile apply` becomes `holler profile apply NAME`.
  `docs_cli_test` parses every inline `holler ...` span, and the bare form no longer parses once `NAME` is required.
- `CHANGELOG.md`: an `[Unreleased]` Enhancements entry that links #664.
- `docs/handoffs/664/evidence.md`: nine source facts the diff relies on.

## Design decisions

- **Two threads per recorded pane, not one.** `health` and `shown_session` each run on their own scoped thread, beside
  the snapshot's thread. The whole observation then takes one port call's own bound (10 s), which is exactly §9's
  "the up-front health observation runs under one I5 bound (default 10 s) … a run of N panes is bounded by 10 s plus N
  step bounds". With one thread per pane (health, then shown_session) it would take two per-call bounds and break that
  sentence. C4's "the slowest pane's chain" becomes "the slowest call". `shown_session` is called for every recorded
  pane, unconditionally (C1 step 3, C12).
- **Rows are chosen once, at planning (C1 step 4), and never re-decided.** A step acts on what it re-reads (§9):
  - the spec handed to `launch` or `relaunch` is P's spec at that moment (entry 19);
  - the membership writes are decided on the re-read record (C10): none if it is in P, the join if it is in no profile,
    and the leave then the join under `--take-over`;
  - the relaunch's record is the re-read record, or the record the join stored (C11).

  Rows a and e, and any step refused or failed at planning, re-read nothing and call nothing, because they write
  nothing. A step's own re-read can still refuse or fail it:
  - the record is gone: `pane-not-found`;
  - P no longer holds the spec: `pane-not-in-profile` (the precedent is `pane close`'s use of that code);
  - P is gone: `profile-not-found`;
  - the pane moved to another profile without `--take-over`: row c refuses it `pane-in-other-profile` with no write.
- **Every step answer is sorted through `class_of`** (A-dup expectation 5). A failure is `failed`. A refusal is
  `refused` with its own code. `usage` for the step's spec becomes `refused` with the new open code
  `spec-not-launchable`, carrying the transaction's message (§4's "a named code"). A spec whose pane is not a valid
  pane name gets the same refusal at planning, with action `refuse`. That keeps it out of §4's gate, since it can never
  be created.
- **An observation error that is not failure class becomes `unavailable`** (C9). §2 makes every `health` error a failed
  step, and §5 makes a failed step exit 1. Both the CLI and the envelope checker (rule 13) derive the exit code from
  the code's class. A refusal-class error from `health` (only a fault-injected fake can answer one) would therefore print
  exit 3. Failure-class errors are kept unchanged, as the tests compare them. A lost thread is `unavailable` naming the
  call (A warn 2).
- **C3 is applied literally.** A snapshot error fails every recorded pane that row a does not refuse, even one whose
  `health` already said false.
- **The step vocabulary that A's warn 9 left open.** Actions: `create`, `adopt`, `report`, `relaunch`, `refuse` (rows a
  and e, a bad name) and `observe` (an observation failed). The dry-run outcome of rows b, c and f is `planned`; row d
  in a dry run is `reported`. Each `Action` and `Outcome` has one `as_str` table, and serde and the text both use it.
  This is `SpecField`'s pattern.
- **The step JSON adds `detail`.** It is one line: the error of a refused or failed step (plus a note on what the step
  changed first), why a relaunched pane was not healthy, or C12's "could not tell" for an unreadable shown session (the
  decision on O2: "reported in the step's detail"). `error` and `acted` are `#[serde(skip)]`. `code`, `diff` and
  `detail` print `null` when absent, as `pane launch` and `profile show` print theirs.
- **O4 and O1 notes.**
  - A take-over join that fails after its leave landed answers the join's error, with a note that the pane left Q and
    is in no profile now ("may be", for a `timeout` or `unavailable` join that may have landed). No re-join is written.
  - A take-over relaunch that fails notes that the pane left Q, joined P, and stays in P.
- **The reconcile step lives in the verb.** `reconcile_step` is `holler-cli`'s, so the engine marks a failed step whose
  launch or relaunch `acted`. The verb appends the step as `failure_body` does, only when the message lacks it.
- **The gate's message** names the spec panes that have no record (quoted), and never a recorded pane (entry 18 checks
  that `pane-c` is absent).

## Reuse / extend-vs-new

There is no Reuse map (no `survey.md`). A's "Reuse expectations for Phase 7" stand in for one:

1. **#808 is in the branch, and the pair check is used.** "Listed" compares `(session, pane_id)` only. It never uses
   `HerdrPane` `==`, `contains` or `same_place` (`observe.rs`, `listed`).
2. **`observe_all`'s shape.** The calls run on `thread::Builder::spawn_scoped` scoped threads, each joined, with a lost
   thread handled. There are no channels, no `thread::spawn` and no new dependency. `observe_all` itself is not
   extended: it is private, calls `host.ps` and `list_sessions`, and turns errors into findings. `observe.rs`'s module
   doc says so.
3. **`diff_profile`'s rows.** `plan` zips P's specs with `diff_profile(&P, records_of_its_spec_panes)`: `Missing` is
   row b, `Matches` row c, `Differs` row d. Row d's `diff` is that row, unchanged. No `PaneDiff` is built by hand.
4. **The twins.** `leave` and `join` in `run.rs` are exactly `detach` and one pane's `join` write. They have no `undo`
   and no profile delete, they are private, and their module doc records the justification. The follow-up issue is
   **drafted below, not filed** (see Known issues).
5. **Steps are classified through `class_of`** (`Planned::halted`, `observe::as_failure`).
6. **Quoting, and the reconcile step.**
   - An acted failure's message carries `reconcile_step(Some(P))` (the verb's `stopped`).
   - Every stored string in a message goes through `findings::quoted` (spec panes, profiles, sessions, Herdr ids) or
     `embedded` (an error's text inside another error).
   - The text form escapes the pane and the detail with `show`'s `text`, and prints differences with `show`'s
     `difference_line`. Both are reused, not copied.
7. **The rig.** It is T's. F touched no test file.
8. **`orchestrator-not-relaunched`** is a `RefusalCode::from_static` constant in `tx_apply.rs`, and it is listed in
   ADR-0021 section 9's apply row. So is `spec-not-launchable`.

Also reused: `tx_launch::{launch, relaunch, LaunchRequest, RelaunchRequest, TxOptions, TxFailure}`, unchanged;
`profile_diff::is_member` for every membership test; `reconcile::shown_differs`; `profile::list::count`.

## Architecture notes for A

- **Layers.** The engine is in `holler-pane` and works only through `Ports`. The verb in `holler-cli` types the name,
  calls the engine and prints. `tx_launch.rs` is not edited, so #802's lines are untouched (§10, C20).
- **New public API in `holler-pane`:** `tx_apply::{apply, ApplyRequest, Applied, Step, Action, Outcome,
  ORCHESTRATOR_NOT_RELAUNCHED, SPEC_NOT_LAUNCHABLE}`. `lib.rs` is untouched, because `tx_apply` was already declared.
- **New modules:** `tx_apply/observe.rs` and `tx_apply/run.rs`, declared from `tx_apply.rs` the way `reconcile.rs`
  declares `reconcile/observe.rs` (A warn 7c). With `tx_apply.rs` they are 546, 184 and 240 lines, all under the
  600-line warning.
- **Shared code touched:** `profile/show.rs`, where two helpers go from private to `pub(crate)`. This is the profile
  verbs' existing sharing pattern (`super::delete::detach`, `super::list::count`), since the frozen `profile/mod.rs`
  admits no new module. No behavior changes.
- **Contract:** two new open codes. `profile apply`'s JSON `data` is
  `{"profile", "dry_run", "steps": [{pane, action, outcome, code, diff, detail}]}`, which #666 consumes. There is no
  wire or protocol change and no new dependency. The ADR 0003 and ADR-0021 rows are updated.

## Deviations from spec / wireframe

- **Two edits outside the brief's Blast radius.**
  - `crates/holler-cli/src/profile/show.rs`: a visibility change, which A's warn 4(c) offered as the alternative to
    copying.
  - `docs/research/herdr-api-spike.md`: a one-word change, `NAME`, needed so `docs_cli_test` still parses that span.
    CLAUDE.md asks for `docs/` to be updated when the CLI surface changes.
- **Two threads per pane instead of §9's "one thread per pane".** This keeps §9's own bound arithmetic (10 s for the
  observation). See Design decisions.
- **The step object adds `detail`** beyond §7's `{pane, action, outcome, code?, diff?}`, and absent keys print `null`.
  T's handoff allows added fields, and C12/O2 ask for the step's detail.
- **Follow-up issues are not filed** (§9 and §13). See Known issues.

No wireframe (`uiSurface: false`).

## Tier 1 self-check (incl. tests now GREEN)

Every command ran from the worktree on the final source (2026-10-11, about 08:10 MDT).

**T's authored tests are GREEN, unedited:** `cargo test -p holler-cli --test profile_verbs`. That covers T's 38 apply
tests and the 43 existing `profile_verbs` tests. It was stable over 5 more runs.

```
test result: ok. 81 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s
```

**The verify command, with `--no-fail-fast` so that every target runs:**
`cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`. Without
`--no-fail-fast`, cargo stops at the first failing binary.

```
binaries: 148  passed: 2123  failed: 2
test stub::stub_verb_not_implemented ... FAILED
test stub::stub_verb_not_implemented_json_is_one_envelope ... FAILED
error: 1 target failed:
    `-p holler-cli --test pane_cli_process`
```

Those are the two tests of "Tests that look wrong (for T)". After them:

```
== cli_surface_test   test result: ok. 3 passed; 0 failed
== docs_cli_test      test result: ok. 3 passed; 0 failed
== pane_cli_process   test result: FAILED. 34 passed; 2 failed   (the STUBS entry)
holler-pane: 14 binaries, 104 passed, 0 failed
```

**Lint and format:**

- `cargo clippy --workspace --all-targets -- -D warnings`: no warning. It re-ran after the last edit and re-checked
  `holler-pane`, `holler-hub`, `holler-pane-testkit` and `holler-cli`.
- `bash scripts/lint.sh`: exit 0. The new files are under its 600-line warning: `tx_apply.rs` 546,
  `tx_apply/observe.rs` 184, `tx_apply/run.rs` 240, `profile/apply.rs` 149.
- `bash scripts/changelog-check.sh`: `changelog-check: ok`.
- `rustfmt --edition 2021 --check` on every changed `.rs` file: exit 0. `cargo fmt --check` reports diffs only in
  `crates/holler-body/src/acp_driver/answerable.rs`, which this branch does not touch (it is unchanged from
  `origin/main`).

**The bound doc's five strings**, joined as entry 20's test joins them: `**Bound (I5).**`, `its own I5 bound`,
`no overall cap`, `10 s` and `DEFAULT_BUDGET` are all present.

## Evidence appendix

`docs/handoffs/664/evidence.md` (nine entries).

## Tests that look wrong (for T)

- **`crates/holler-cli/tests/pane_verbs/process/stub.rs`, `STUBS`.** Delete the entry `("profile", "apply", 664),` and
  **keep the `// #664` line**, as that file's own doc requires ("A verb story deletes its own entries when its verb
  stops being a stub and keeps its `// #NNN` line").
  - Two tests of the `pane_cli_process` target fail until then: `stub::stub_verb_not_implemented` and
    `stub::stub_verb_not_implemented_json_is_one_envelope`.
  - They fail because `holler profile apply` is no longer a stub: with no `NAME` it is clap's usage error, exit 2.
  - This is a test-only change, with nothing to change in production code. The other `pane_cli_process` tests pass
    (34 of 36).
    - `flags.rs` counts "only a required positional is missing" as accepted.
    - `usage.rs`'s `--bogus` case is still a usage envelope.

None of T's 38 apply tests looks wrong: all pass unedited.

## Known issues

- **The follow-up issues of §9 and §13 are drafted here, not filed.** CONTRIBUTING.md says filing an issue "asserts
  that you, the human submitting it, have personally reviewed and verified its content". Filing on the operator's
  account is an outward action outside F's role. The run's agent, or the operator, should file both, ideally before
  the PR, and then cite the twins issue's number in `run.rs`'s module doc ("a follow-up of #664 merges them"):

  > **Title:** refactor(pane): one home for the profile leave and join writes (#664's twins)
  >
  > Follow-up of #664 (§9 of its operator section). `holler profile apply` writes a pane's profile membership
  > itself: the join of an adopt, and the leave and join of a `--take-over`. Both writes already exist in
  > `holler-cli`: `profile::delete::detach`, and one pane's write of `profile::create::join`. The engine
  > (`holler_pane::tx_apply`) lives in `holler-pane`, which cannot depend on `holler-cli`, so #664 added private
  > twins, `leave` and `join`, in `crates/holler-pane/src/tx_apply/run.rs`. Merge them into one implementation that
  > `profile delete --keep-panes`, `profile create --from-current` and `profile apply` all call, placed where both
  > crates reach it.
  >
  > Until the merge, the twins must stay semantically identical to `detach` and to `join`'s write for one pane: the
  > record as read, with `profile` none (leave) or P (join), written by one `cas_put` at the generation it was read
  > at, answering what that `cas_put` answers. `profile create`'s `undo` and its profile delete stay `create`'s own,
  > and the join twin never deletes a profile.
  >
  > Drafted with AI assistance (Claude Code).

  > **Title:** feat(pane): compare `opencode_agent` in `profile_diff` (`SpecField::ALL` omits it)
  >
  > Follow-up of #664 (§13 of its operator section). `SpecField::ALL` in `crates/holler-pane/src/profile_diff.rs`
  > has no row for `opencode_agent`. #700 added the field and recorded its comparison as a non-goal, naming #647.
  > So `diff_spec` never sees it: `profile show` reports a pane that runs another OpenCode agent than its spec as
  > matching, and `profile apply` adopts it. Add the `SpecField` row. `spec_from_pane` already copies the field, so
  > `profile show`, `profile apply` and the planned `profile-drift` finding (#665) all see the difference. #647's
  > parked part 2 lists this row among what its brief must settle (operator comment, 2026-10-11). This issue can
  > be that item, or stand alone.
  >
  > Drafted with AI assistance (Claude Code).
- **A row is chosen from the up-front observation and is not re-checked when its step runs** (A's warn 5(b), left to
  the operator and not decided).
  - A pane that recovers, or that another verb relaunches, between the observation and its row-f step is still
    relaunched.
  - The step's re-read refreshes the record and its generation. So C5's fence catches only a write that lands between
    that re-read and the relaunch's own record write (`generation-conflict`, after the act). It does not catch a
    relaunch by another verb that finished before the re-read, or a pane that recovered with no record write.
  - The window is at most the earlier steps' bounds. The brief chose this structure (§9, C1), and its two remedies
    (re-check `health` right before a relaunch, or fail a step whose record generation moved) are the operator's to
    pick.
- **The stale-P window of A's warn 5(a) remains.** A P edit that lands after a create's or relaunch's re-read, and
  before `edit_spec`'s own read, is overwritten by `SpecEdit::Set`. `pane relaunch --profile` has the same window.
  Entry 19's test (an edit during the observation) passes, as the brief scoped it.
- **Duplicate spec panes** (two specs of one pane in P) each get a step. A second create is refused `pane-exists` by
  launch, and two specs of one recorded pane both act on it. No test or rule covers this, and it is not reachable
  through the profile verbs, which set a spec in place.

## Files changed

Production and docs, no test file except `cli-surface.txt` (ADR 0003's table, assigned to F by §10):

- `crates/holler-pane/src/tx_apply.rs`
- `crates/holler-pane/src/tx_apply/observe.rs` (new)
- `crates/holler-pane/src/tx_apply/run.rs` (new)
- `crates/holler-cli/src/profile/apply.rs`
- `crates/holler-cli/src/profile/show.rs`
- `crates/holler-cli/tests/fixtures/cli-surface.txt`
- `docs/adr/ADR-0003.md`
- `docs/adr/ADR-0021.md`
- `docs/research/herdr-api-spike.md`
- `CHANGELOG.md`
- `docs/handoffs/664/handoff-F.md`, `docs/handoffs/664/evidence.md`, `docs/handoffs/664/decisions.md` (the handoffs)
