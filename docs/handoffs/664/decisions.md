# Decision journal: #664 `holler profile apply`

## A (Phase 3, up-front plan review): 2026-10-11 07:00 MDT
- **Decided:** PASS, with 0 block and 9 warn findings (`docs/handoffs/664/handoff-A.md`). The layering and placement are right: the engine `tx_apply` is in `holler-pane` and works only through `Ports`, the verb is in `holler-cli`, and `tx_launch` is called unchanged. The twins are an operator-decided, justified duplication. The warns:
  1. Take `origin/main` (#808) before T.
  2. Name and follow `reconcile::observe_all`'s fan-out shape, and define a lost thread.
  3. Name `diff_profile` as the comparison.
  4. Carry the reconcile step in an acted failure, and quote and escape stored strings.
  5. C5 overstates what the re-read fences, and a row chosen up front can be stale by the time its step runs.
  6. Extend the profile rig additively, modelled on `launch/rig.rs`.
  7. Fill the blast-radius gaps (`profile_verbs/apply.rs`, §10's list, `tx_apply/` for overflow).
  8. Declare `orchestrator-not-relaunched` as a `RefusalCode` constant, list it in ADR-0021 section 9's apply row, and define "adopt" against the existing "never adopted" text.
  9. Name the step vocabulary (`action`, the dry-run `outcome`, the per-pane `usage` code, the step for an invalid spec name).
- **Assumed:** The brief's operator section (§1 to §13 and the decisions on O1 to O6) is final and outside A's remit, so findings that touch it (#5(b), #9) are framed as choices for the operator, not drift. No `survey.md` or Reuse map exists for this run, so the handoff's "Reuse expectations for Phase 7" stands in for one.
- **Hedged:** I did not block on the base having moved (#1). C9 already prescribes the label-free `(session, pane_id)` comparison that ADR-0021 now requires, and `parallel-agent-coexistence.md` expects a rebase. But the pipeline does not sync the branch with `main`: its diff gates use `origin/main...HEAD`. Unless someone merges `main` before T, the labelled-pane test #1 asks for cannot be written, and an equality-based check would pass every test on this base. My estimate that `tx_apply.rs` will pass 600 lines (#7(c)) is a judgment, not a measurement.
- **Evidence:** I read the whole brief (2657 lines). I read `reconcile.rs` and `reconcile/observe.rs`; `profile_diff.rs`; `profile/show.rs`, `profile/mod.rs` and `profile/apply.rs`; `pane/launch.rs` (`failure_body`), `pane/relaunch.rs` and `pane/profile_scope.rs` (`reconcile_step`); `output.rs` (`ErrorBody`); `error.rs` (open codes, `class_of`); `tx_switch.rs`; and `tx_launch.rs` (`run`, `stop_old`). On the test side: `tests/profile_verbs/{main,apply,rig}.rs`, `tests/pane_verbs/launch/rig.rs`, the `park` and `close` rig docs, the test kit's `host.rs`, `harness.rs` and `profile_scope.rs`, `cli-surface.txt`, ADR-0003:70, and ADR-0021 section 9 (515-531) and its uses of "adopt". I also ran `git log HEAD..origin/main` (#808 is `263a2d7`) and `git diff HEAD...origin/main` on `pane.rs`, `ports.rs`, `tx_launch.rs`, `holler-pane-testkit/src/herdr.rs` and `ADR-0021.md`, and grepped for `thread::scope`, `RefusalCode::from_static`, leave/join writes in `holler-pane`, and `--dry-run`.

## T (Phase 4, author / RED): 2026-10-11 07:20 MDT
- **Decided:**
  - **The merge (A warn 1).** I merged `origin/main` (`263a2d7`, #808) into the branch with `--no-commit`
    before writing tests. The driver's phase commit completes the merge, so T commits nothing.
  - **The tests.** 38 tests in `crates/holler-cli/tests/profile_verbs/apply.rs` and `apply/*.rs` cover every C14
    entry (1 to 33), C8, C6, and A warn 1's labelled pane. Each runs on the engine (`tx_apply::apply`, with steps
    read from its result, per O6) or on the in-process verb. `assert_stopped` checks O6's output at exit 1.
  - **The API.** The tests pin `apply(ports, &ApplyRequest, &TxOptions) -> Result<Applied, PaneError>` and the
    `Step`, `Action` and `Outcome` types. `Outcome`'s JSON strings come from §5. `Action`'s strings are F's
    (compared through serde).
  - **The rig.** It is additive (A warn 6): `apply/rig.rs` copies `launch/rig.rs` by name, and the read verbs' rig
    gains only `serve_server` (O3's one seeding helper).
- **Assumed:**
  - **No regression of A warn 1.** The pair comparison is checked by a labelled pane and by another-session cases.
  - **The pane-exists seam.** Its timing assumes C1's order: step 1's registry read comes before the observation,
    and the step's re-read is the first read after it.
  - **Dry-run outcomes for rows b, c and f.** They are unpinned beyond "no effect claimed", since the brief does not
    name them (A warn 9).
- **Hedged:**
  - **The prototype.** It proves the suite can be satisfied as written, but it is mine, not F's. F's design may meet a
    seam assumption differently (the pane-exists timing). Each such assumption is named in the handoff's RED notes.
  - **Timing.** Entries 26 and 27 rely on harness delays. Their margins are 3 s (26), and 300 ms on a 900 ms budget
    (27). They were stable over 15 runs here.
- **Evidence:**
  - **Compile RED.** 10 errors, all `E0432`/`E0425` on `holler_pane::tx_apply::{apply, Action, Applied, ApplyRequest,
    Outcome, Step}`.
  - **API-only stub.** 43 passed, 38 failed, all on assertions.
  - **Prototype.** 81 passed, over 5 runs of the target and 10 runs of `apply::`.
  - **Mutants.** 16, each failing at least one test (table in `handoff-T-red.md`).
  - **Lint.** `cargo clippy -p holler-cli --test profile_verbs -- -D warnings` is clean, and `bash scripts/lint.sh`
    exits 0 (`apply/rig.rs` warns at 742 lines).
  - **Source.** `src/` is identical to `origin/main` after every throwaway edit (`git diff origin/main --stat --
    'crates/*/src'` is empty).

## F (Phase 6, implement): 2026-10-11 07:55 MDT
- **Decided:**
  - **The engine.** `tx_apply.rs` holds the module doc and the bound, the API T pinned, the gate, the plan and the
    stop loop. `tx_apply/observe.rs` holds the fan-out and §2's judgment. `tx_apply/run.rs` holds one live step, the
    take-over move and the twins. `tx_launch.rs` is unchanged.
  - **The observation** runs one scoped thread per call (the snapshot, and each recorded pane's `health` and
    `shown_session`), so it takes one per-call bound, which is §9's 10 s, rather than two.
  - **Rows are chosen once, at planning.** A step acts on its re-read record and on P's current spec. Rows a and e
    re-read nothing.
  - **Classification goes through `class_of`.** `usage` for a step's spec becomes the open code `spec-not-launchable`.
    An observation error that is not failure class becomes `unavailable`, so a failed step always exits 1.
  - **The vocabulary.** Actions are `create`, `adopt`, `report`, `relaunch`, `refuse` and `observe`, and a dry run's
    acting step is `planned`. The step JSON adds `detail` (C12/O2's "the step's detail"), and absent keys print
    `null`.
  - **Reuse.** The verb reuses `show`'s `difference_line` and `text`, made `pub(crate)`, and `reconcile_step`.
  - **Docs.** ADR-0021 section 9's apply row and "Apply as built (#664)", which defines "adopt"; ADR 0003's row;
    `cli-surface.txt`; the CHANGELOG; and one inline span in `docs/research/herdr-api-spike.md`, without which
    `docs_cli_test` fails once `NAME` is required.
- **Assumed:**
  - **C3, literally.** A snapshot error fails every recorded pane that row a does not refuse.
  - **The row e test uses the record's role**, as §3 says ("record, not healthy, role orchestrator").
  - **A spec whose pane is not a pane name** is a planning refusal (action `refuse`). It never trips §4's gate.
- **Hedged:**
  - **The follow-up issues of §9 and §13 are not filed.** CONTRIBUTING.md makes filing an issue an assertion of
    human review. Both issues are drafted in `handoff-F.md` for the operator or the run's agent.
  - **`stub.rs`'s `STUBS` still lists `profile apply`.** That is a test file, so T must delete it. Until then the two
    `pane_cli_process` stub tests fail.
  - **Two warns stay open, as the brief leaves them** (Known issues): a row-f pane that recovers before its step is
    still relaunched (A warn 5(b)), and the stale-P window after a re-read remains (A warn 5(a)).
- **Evidence:**
  - **`profile_verbs`.** 81 of 81 pass (T's 38 apply tests and the 43 existing), over 5 runs.
  - **Lint and format.** Workspace clippy with `-D warnings` is clean, `scripts/lint.sh` exits 0, `changelog-check`
    is ok, and rustfmt is clean on every changed file.
  - **`cli_surface_test` and `docs_cli_test`** pass.
  - **`pane_cli_process`.** 34 of 36 pass; the 2 failures are the `STUBS` entry.
  - **The whole workspace.** See `handoff-F.md`'s Tier 1 output.

## T (Phase 7, verify / GREEN): 2026-10-11 08:45 MDT
- **Decided:** I took F's flag on `pane_verbs/process/stub.rs` as a test fix: I deleted the `("profile", "apply", 664)` `STUBS` entry and kept the `// #664` line, as the file's doc requires. That was my only test change. The verdict is PASS with no blocking issue. I added two evidence entries for unchanged-code facts my tests rely on: `herdr.version` is called only inside `tx_launch`'s observation, and the split-only fake refuses an unreachable cell.
- **Assumed:** the repo-wide `cargo fmt --check` drift is not this branch's concern. None of the files that drift is changed by the branch, and CI gates on `lint.sh`, not `cargo fmt`.
- **Hedged:** three cases stay untested: an invalid `PaneName` spec, a record that moves to another profile mid-run without `--take-over`, and duplicate specs. They are advisories for S, not blockers, because the brief does not pin them (T-red's "Not pinned").
- **Evidence:**
  - The whole workspace passes with `--no-fail-fast`: 2125 passed, 0 failed.
  - `profile_verbs` passed 81/81 on 10 of 10 runs, and `pane_cli_process` passed 36/36.
  - These are clean: clippy `-D warnings`, `lint.sh`, `changelog-check`, `cargo machete`, `docs_cli_test`, `cli_surface_test`, `wire_selftest`, `test-hooks.sh`, `pipeline-adoption-test.sh`, and rustfmt on the changed files.
  - Five mutants of F's code were all killed, each reverted with `git checkout`.
  - I compared the twins `leave` and `join` in `run.rs` with `profile::delete::detach` and `profile::create::join` by reading both.
