# Handoff-F: Phase 6 (F) - `holler profile apply` (#664), fresh run after A round 4

**Date:** 2026-10-11 (11:20 MDT)
**Branch:** `issue-664-implementation`, head `d7c92de` (T-red's commit), based on `origin/main` `7284001` (#802)
**Issue:** #664
**Brief:** `docs/handoffs/664-brief.md` as of `af230b6` (C21 to C25 added since the first run)

## This cycle

The first run's F (`7f23e0f`, handoff at `git show 7f23e0f:docs/handoffs/664/handoff-F.md`) wrote the engine and the
verb. That code is still on the branch, and this handoff describes all of it as it stands now. This run adds only what
the amended brief asks of F:

- **C25 (the RED).** The verb now adds the reconcile step to every step whose launch or relaunch had started its act,
  `refused` steps included, not only to a `failed` step's message.
- **A round 4 warn 4.** `Step.acted`'s doc now covers `refused` steps, and the ADR sentence states the reconcile step.
- **C21.** `tx_apply/observe.rs`'s module doc names the mechanism and no longer claims "the shape of `observe_all`".
- **C22.** One sentence in ADR-0021's "Apply as built (#664)" names the step vocabulary.
- **C23.** `tx_apply/run.rs`'s module doc cites #818.
- **C24.** rustfmt ran only on the story's own `.rs` files. `cargo fmt` was never run.
- **Evidence.** `evidence.md` is re-cut to HEAD, and four C25 entries are added (see "Evidence appendix").

## What was done

This cycle's edits are marked **(this run)**.

- `crates/holler-pane/src/tx_apply.rs`: the engine, which replaces #637's stub.
  - The module doc covers the rules and the run's bound in `tx_launch`'s **Bound (I5).** form (C4, C16).
  - The public API that T's tests pin: `apply`, `ApplyRequest`, `Applied`, `Step`, `Action`, `Outcome`.
  - Two open codes: `ORCHESTRATOR_NOT_RELAUNCHED` and `SPEC_NOT_LAUNCHABLE`.
  - The §4 gate, the plan (rows a to f in grid order, from `diff_profile`'s rows) and the stop-at-first-failed loop.
  - **(this run)** `Step.acted`'s doc: a `refused` or `failed` step whose launch or relaunch had started its act. The
    module doc's "Outcomes" bullet says the same. The module doc also states that a `PaneName` is checked text
    (`SessionName`'s grammar), printed as it is. That answers the first diff gate's W on `other_profile`.
- `crates/holler-pane/src/tx_apply/observe.rs`: the up-front observation and §2's health judgment.
  - Scoped threads, one per call: Herdr's snapshot, and each recorded spec pane's `health` and `shown_session`.
  - The judgment uses the `(session, pane_id)` pair (never the cell or the label), and C12's shown-session table.
  - **(this run)** The module doc follows C21. The fan-out is `reconcile`'s mechanism (scoped `thread::Builder` threads,
    each joined, a lost thread answered `unavailable` naming its call), at one thread per call rather than per pane,
    because a pane's two reads are independent.
- `crates/holler-pane/src/tx_apply/run.rs`: one live step.
  - It first re-reads the record and P.
  - Create and relaunch go through `tx_launch` unchanged. Adopt and report touch nothing live.
  - The take-over move: leave, then join.
  - The two private twins, `leave` and `join`, of `detach` and of `join`'s write for one pane.
  - **(this run)** The module doc cites #818, which merges the twins (C23).
- `crates/holler-cli/src/profile/apply.rs`: the verb, which replaces #670's stub.
  - It takes `NAME`, `--dry-run`, `--take-over` and `--herdr-session`, and calls the engine with `TxOptions::default()`.
  - At exit 0 it prints the steps object. At exit 1 it prints only the failure, in O6's pinned form.
  - The text form uses `profile show`'s diff lines.
  - **(this run)** A new `reconciled` pass ends the detail of every `acted` step with `reconcile_step(Some(P))`
    (C25). It appends after `"; "`, and only when the detail lacks it, as `failure_body` does. `stopped` no longer
    appends the step itself: the failed step's detail already carries it, so the step is added in one place.
- `crates/holler-cli/src/profile/show.rs`: `difference_line` and `text` go from private to `pub(crate)`, and the
  module doc says so.
- `crates/holler-cli/tests/fixtures/cli-surface.txt`: #664's three lines now carry `NAME` and cover every flag. This
  is ADR 0003's machine-readable table, which §10 and the first T-red handoff assign to F.
- `docs/adr/ADR-0003.md`: the `profile apply` row is now
  `holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]`.
- `docs/adr/ADR-0021.md`:
  - Section 9's `profile apply` row is in the doctor form and lists the step codes, the new open codes included.
  - Section 8's "Apply as built (#664)" defines "adopt" against the ADR's existing "never adopted" sentences.
  - **(this run)** One sentence there names the step vocabulary (C22): the step object
    `{pane, action, outcome, code, diff, detail}` with `null` for an absent `code`, `diff` or `detail`; the six actions;
    the outcomes (§5's six, `skipped`, and `planned` for a dry run); that a step's `detail` is inside `data` and does
    not conflict with "the envelope has no `detail`"; and the reconcile step on an acted step (A round 4 warn 4).
- `docs/research/herdr-api-spike.md:280`: the inline span `holler profile apply` becomes `holler profile apply NAME`.
  `docs_cli_test` parses every inline `holler ...` span, and the bare form no longer parses once `NAME` is required.
- `CHANGELOG.md`: an `[Unreleased]` Enhancements entry that links #664. **(this run)** It adds one clause: a step
  refused or failed after its launch or relaunch had begun says how to reconcile the profile.

## Design decisions

- **The reconcile step goes on the detail, in one pass at the verb (this run, C25).**
  - **Where.** `reconcile_step` is `holler-cli`'s, so the engine only marks the step (`acted`, from
    `TxFailure.acted`), and the verb adds the text. That is where `pane launch` adds it.
  - **On the detail.** The step's detail is the one place both outputs read. At exit 0, the JSON `detail` and the text
    line print it. At exit 1, the O6 message is built from the failed step's detail. So one pass covers `refused` and
    `failed`, and the guard exists once.
  - **Considered:** a second append in the exit-0 renderer beside `stopped`'s. That would keep two copies of the guard.
    A-dup expectation 6 asks for one.
  - **The fallback.** An acted step always has a detail (`halted` sets it), but the pass is total anyway. A missing
    detail falls back to the error's text, as `stopped` already did, and then to the step alone.
- **One thread per call, not per pane (now C21).** `health` and `shown_session` each run on their own scoped thread,
  beside the snapshot's thread. So the observation takes one port call's own bound (10 s), which is §9's arithmetic:
  "10 s plus N step bounds". A per-pane chain would take two bounds. `shown_session` is called for every recorded pane,
  unconditionally (C1 step 3, C12). T's C21 pin (`a_panes_health_and_shown_session_are_observed_at_once`) checks that
  the two calls are in flight together.
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
- **Every step answer is sorted through `class_of`** (A-dup expectation 5).
  - A failure is `failed`.
  - A refusal is `refused` with its own code. That includes a refusal from inside the act (`port-in-use` from the
    re-check, the harness's `verify_serving` code).
  - `usage` for the step's spec becomes `refused` with the open code `spec-not-launchable`, carrying the transaction's
    message (§4's "a named code").
  - A spec whose pane is not a valid pane name gets the same refusal at planning, with action `refuse`. That keeps it
    out of §4's gate, since it can never be created.
- **An observation error that is not failure class becomes `unavailable`** (C9). §2 makes every `health` error a failed
  step, and §5 makes a failed step exit 1. The CLI and the envelope checker (rule 13) both derive the exit code from the
  code's class. A refusal-class error from `health` (only a fault-injected fake can answer one) would therefore print
  exit 3. Failure-class errors are kept unchanged, since the tests compare them. A lost thread is `unavailable` naming
  the call.
- **C3 is applied literally.** A snapshot error fails every recorded pane that row a does not refuse, even one whose
  `health` already said false.
- **The step vocabulary (now C22).**
  - Actions: `create`, `adopt`, `report`, `relaunch`, `refuse` (rows a and e, a bad name) and `observe` (an
    observation failed).
  - The dry-run outcome of rows b, c and f is `planned`; row d in a dry run is `reported`.
  - Each `Action` and `Outcome` has one `as_str` table, which serde and the text both use. This is `SpecField`'s
    pattern.
  - The step JSON adds `detail`, one line: the error of a refused or failed step (plus a note on what the step changed
    first, and the reconcile step when it acted), why a relaunched pane was not healthy, or C12's "could not tell" for
    an unreadable shown session.
  - `error` and `acted` are `#[serde(skip)]`. `code`, `diff` and `detail` print `null` when absent.
- **O4 and O1 notes.**
  - A take-over join that fails after its leave landed answers the join's error, with a note that the pane left Q and
    is in no profile now ("may be", for a `timeout` or `unavailable` join that may have landed). No re-join is written.
  - A take-over relaunch that fails notes that the pane left Q, joined P, and stays in P. It is acted, so it also ends
    with the reconcile step.
- **The gate's message** names the spec panes that have no record (quoted), and never a recorded pane (entry 18 checks
  that `pane-c` is absent).

## Reuse / extend-vs-new

There is no Reuse map (no `survey.md`). A round 4's "Reuse expectations for Phase 7" stand in for one:

1. **"Listed" compares only the pair.** It compares `(session, pane_id)` only, never `HerdrPane` `==`, `contains` or
   `same_place` (`observe.rs`, `listed`).
2. **One thread per call (C21).** The mechanism is `reconcile`'s: `thread::Builder::spawn_scoped` threads, each joined,
   with a lost thread handled. There are no channels, no `thread::spawn` and no new dependency. `observe_all` itself is
   not extended: it is private, calls `host.ps` and `list_sessions`, and turns errors into findings. **(this run)**
   `observe.rs`'s doc no longer says "the shape of `observe_all`".
3. **`diff_profile`'s rows.** `plan` zips P's specs with `diff_profile(&P, records_of_its_spec_panes)`: `Missing` is
   row b, `Matches` row c, `Differs` row d. Row d's `diff` is that row, unchanged. No `PaneDiff` is built by hand.
4. **The twins.** `leave` and `join` in `run.rs` are exactly `detach` and one pane's `join` write. They have no `undo`
   and no profile delete, and they are private. **(this run)** Their module doc cites #818.
5. **`class_of`.** Steps are classified through it (`Planned::halted`, `observe::as_failure`).
6. **The reconcile step (this run).**
   - It goes on every acted step, `refused` included, through `reconcile_step`, in the verb's one `reconciled` pass.
   - That pass has `failure_body`'s "already contains" guard and its `"; "` form. It is not a second copy of
     `failure_body`'s text: `failure_body` builds an `ErrorBody` from a `TxFailure`, and this edits a step's detail.
   - `stopped` reads the detail and appends nothing.
   - `Step.acted`'s doc covers `refused`.
7. **The rigs.** They are T's. F touched no test file.
8. **The open codes.** `orchestrator-not-relaunched` and `spec-not-launchable` are `RefusalCode::from_static`
   constants in `tx_apply.rs`, and both are listed in ADR-0021 section 9's apply row.
9. **The 27 paths.** The working tree's diff against `origin/main` is still exactly A's 27 paths (`git diff
   --name-only origin/main | wc -l` is 27). `bash scripts/lint.sh` exits 0.

Also reused: `tx_launch::{launch, relaunch, LaunchRequest, RelaunchRequest, TxOptions, TxFailure}`, unchanged;
`profile_diff::is_member` for every membership test; `reconcile::shown_differs`; `profile::list::count`;
`pane::profile_scope::reconcile_step`.

## Architecture notes for A

- **Layers.** The engine is in `holler-pane` and works only through `Ports`. The verb in `holler-cli` types the name,
  calls the engine and prints. `tx_launch.rs` and `tx_launch/server.rs` are not edited (§10, C20).
- **This run changes no interface, module boundary or dependency.** The C25 change is one private function in the verb
  and a different call order inside it. The rest is docs.
- **Public API in `holler-pane`:** `tx_apply::{apply, ApplyRequest, Applied, Step, Action, Outcome,
  ORCHESTRATOR_NOT_RELAUNCHED, SPEC_NOT_LAUNCHABLE}`. `lib.rs` is untouched, because `tx_apply` was already declared.
- **Modules:** `tx_apply/observe.rs` and `tx_apply/run.rs`, declared from `tx_apply.rs` the way `reconcile.rs` declares
  `reconcile/observe.rs`. The sizes are `tx_apply.rs` 550, `observe.rs` 190, `run.rs` 240 and `profile/apply.rs` 173
  lines, all under the 600-line warning.
- **Shared code touched:** `profile/show.rs`, where two helpers go from private to `pub(crate)`. This is the profile
  verbs' existing sharing pattern (`super::delete::detach`, `super::list::count`), since the frozen `profile/mod.rs`
  admits no new module. No behavior changes.
- **Contract (which #666 consumes):**
  - Two new open codes.
  - `profile apply`'s JSON `data` is `{"profile", "dry_run", "steps": [{pane, action, outcome, code, diff, detail}]}`.
  - **(this run)** An acted step's `detail` ends with the reconcile step. That is visible output, and it is now in the
    ADR sentence.
  - There is no wire or protocol change and no new dependency.

## Deviations from spec / wireframe

- **Edits outside the brief's Blast radius bullets** (all inside A's 27 paths, expectation 9):
  - `profile/show.rs`: a visibility change, which A's round 1 warn 4(c) offered as the alternative to copying.
  - `docs/research/herdr-api-spike.md`: a one-word change, `NAME`, needed so `docs_cli_test` still parses that span.
  - `CHANGELOG.md`: required by `changelog-check.sh`.
- **Settled since the first run, no longer deviations:**
  - One thread per call (C21).
  - The step object's `detail` (C22).
  - #818 is filed and cited (C23). §13's follow-up is not filed, by C18.

No wireframe (`uiSurface: false`).

## Tier 1 self-check (incl. tests now GREEN)

Every command ran in the worktree on the final source, on 2026-10-11 at about 11:20 MDT.

**T's authored tests are GREEN, unedited.** `cargo test -p holler-cli --test profile_verbs` passed 7 runs of 7. That
is T's 41 apply tests and the 43 other `profile_verbs` tests. Before the change it was 83 passed and 1 failed, the RED
`a_refusal_after_the_act_began_names_the_reconcile_step`.

```
test result: ok. 84 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.60s
```

**The verify command, with `--no-fail-fast` so every target runs.** The command is `cargo test --workspace
--no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`, and it exits 0:

```
binaries: 148  passed: 2157  failed: 0  ignored: 25
```

After the last doc edit (`observe.rs`), `holler-pane` (108 passed) and `profile_verbs` (84 passed) were re-run.

**Other targets:** `docs_cli_test` passed 3, `cli_surface_test` 3 and `pane_cli_process` 36, each with 0 failed.

**Lint and format:**

- `cargo clippy --workspace --all-targets -- -D warnings`: no warning. It was re-run after the last edit.
- `bash scripts/lint.sh`: exit 0. No story file is near a limit; the only story warning is T's `apply/rig.rs` at 759
  lines.
- `bash scripts/changelog-check.sh`: `changelog-check: ok`.
- `rustfmt --edition 2021 --check` on `profile/apply.rs` and `tx_apply.rs`, which follows `mod` into `observe.rs` and
  `run.rs`: exit 0. Nothing was reformatted, and `cargo fmt` was never run (C24).
- `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane --no-deps`: clean, so the new ``[`Step::acted`]`` link resolves.

**The bound doc's five strings** are still present: `**Bound (I5).**`, `its own I5 bound`, `no overall cap`, `10 s`
and `DEFAULT_BUDGET` (`the_engine_doc_states_the_bound` passes).

## Evidence appendix

`docs/handoffs/664/evidence.md`, with 15 entries.

- **Re-cut.** #802 moved `tx_launch.rs`, so the first run's line numbers for `pane-exists` (now 198-201), the relaunch's
  session (224) and `record`'s profile (369) were stale.
- **T's two entries.** I re-cut their line numbers too. The `tx_launch` observation entry is now 340-346, and its
  excerpt shows the `serve_command` read #802 put at its head. The facts' wording is T's, unchanged, and a note there
  says so.
- **Four new C25 entries:**
  - an act failure is `TxFailure { acted: true }` (`tx_launch.rs:167-174`, `568-580`);
  - `server::start` refuses inside the act, after the pane, the session, the command and the server ran
    (`tx_launch.rs:611`, `662`; `tx_launch/server.rs:87-107`);
  - `reconcile_step`'s text (`pane/profile_scope.rs:49-56`);
  - `failure_body`'s guard (`pane/launch.rs:434-442`).
- **Checked mechanically.** A script found every code line of all 15 excerpts verbatim within its cited lines at
  HEAD (15 of 15 OK).
- **For T.** T-red's "Notes for my Phase 6" listed three facts. Two are now covered: `verify_serving` runs inside
  `server::start` after `host.run`, and a launch reaches `create_session` after the server is up (the `bring_up` and
  `server.rs` entries). Only the fault switch's log-before-delay fact (`holler-pane-testkit/src/fault.rs`, `enter`)
  is left for T.

## Tests that look wrong (for T)

None. All 84 `profile_verbs` tests pass unedited.

## Known issues

- **Left open, as C23 says.** These open points from the first run are the operator's, not F's:
  - **A row is chosen once** (A round 1 warn 5(b)). A pane that recovers, or that another verb relaunches, between the
    observation and its row-f step is still relaunched.
    - The step's re-read refreshes the record and its generation. So C5's fence catches only a write that lands between
      that re-read and the relaunch's own record write.
    - The window is at most the earlier steps' bounds.
    - There are two remedies for the operator to pick from: re-check `health` right before a relaunch, or fail a step
      whose record generation moved.
  - **The stale-P window** (A round 1 warn 5(a)). A P edit that lands after a create's or relaunch's re-read, and before
    `edit_spec`'s own read, is overwritten by `SpecEdit::Set`. `pane relaunch --profile` has the same window.
  - **Duplicate spec panes.** Two specs of one pane in P each get a step. A second create is refused `pane-exists` by
    launch, and two specs of one recorded pane both act on it. The profile verbs cannot produce this, since they set a
    spec in place.
- **An acted refusal leaves things behind.** Launch's rollback stops the session's processes, the new server among
  them, and closes a Herdr pane the run made. The tmux session stays, as for `pane launch`.
  - For a row f relaunch, `stop_old` had already stopped the old server. The pane is left with no server, and its
    record is unchanged, since the record write comes only after the act succeeds.
  - The reconcile step in the detail points the operator at this. Apply does nothing further, which is C25's design.
- **Brief edits for O, not F's** (A round 4):
  - warn 1: E40's apply-row excerpt is empty;
  - warn 3: the Blast radius does not list the 27 paths.
  - C18's order clause for #644 part 2 is the MO's (warn 2). T's `pane-g` fixture covers the agent case either way.

## Files changed

Production and docs. There is no test file, except `cli-surface.txt` (ADR 0003's table, assigned to F by §10).

- `crates/holler-pane/src/tx_apply.rs` (this run: docs)
- `crates/holler-pane/src/tx_apply/observe.rs` (this run: module doc)
- `crates/holler-pane/src/tx_apply/run.rs` (this run: module doc)
- `crates/holler-cli/src/profile/apply.rs` (this run: `reconciled`, `stopped`, the module doc)
- `crates/holler-cli/src/profile/show.rs`
- `crates/holler-cli/tests/fixtures/cli-surface.txt`
- `docs/adr/ADR-0003.md`
- `docs/adr/ADR-0021.md` (this run: the C22 sentence)
- `docs/research/herdr-api-spike.md`
- `CHANGELOG.md` (this run: one clause)
- The handoffs: `docs/handoffs/664/handoff-F.md`, `docs/handoffs/664/evidence.md` and `docs/handoffs/664/decisions.md`.
