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
