# Decision journal: #644 launch-relaunch

## A (up-front plan review) — 2026-10-09T17:07-06:00
- **Decided:** BLOCK, on four block findings (handoff-A.md):
  1. `tx_launch::spec_of_pane` duplicates the Pane-to-spec mapping. ADR-0021 section 3 places that mapping in
     `profile_snapshot.rs`, and #662a is writing it there as `spec_from_pane`, with `FIXED_PORT_POLICY_PREFIX` and
     `fixed_port_policy`.
  2. Relaunch's cwd and cell rules live only in `relaunch.rs`, and `RelaunchRequest.move_grid` is computed by the caller,
     so the engine #664 will call can record a false cwd or leave a Herdr pane open with no record.
  3. The record at R infers `last_observed.driven`, which the frozen `LastObserved` contract and I6 forbid.
  4. `launch::reconcile_step` is a second copy of #663's planned shared `profile_scope::reconcile_step`, and under the
     real scope the step would be printed twice.
  Six warns: reuse `holler_proto::clock::now_millis`; three parallel test rigs; B10 before R; no Holler path frees a cell
  left by a crash; a lost-update window on P between two reads; coordination of the ADR-0021 edits, including marking the
  "no operation id" decision PROPOSED or getting the operator's confirmation.
- **Assumed:** the sibling briefs on `issue-662-implementation` (`5b47d82`), `issue-663-implementation` (`aaf8fb5`),
  `issue-647-implementation` (`878a5b9`) and `issue-643-implementation` (`2e7f221`) are the plans those runs will
  implement, and none of them has merged code yet. #662's run is scoped to 662a, which includes `profile_snapshot.rs`.
- **Hedged:** finding 4 depends on #663's own plan review, which has not happened yet. Its fix is phrased as "one function,
  one owner, one doctor form, agreed with #663" rather than naming the final text. For finding 1, the order of the stories
  (#662a first, or #644 filling #662a's API in `profile_snapshot.rs`) is the MO's call; either way there is one mapping, in
  `profile_snapshot.rs`.
- **Evidence:** read the whole brief (2,070 lines) and both outside-review rounds (r1, r2); ADR-0021 in full; issues #644,
  #662 and #664; `holler-pane/src/{lib,ports,pane,profile}.rs` and the module stubs; `holler-cli/src/pane/{args,mod,wiring,
  profile_scope,close}.rs`, `output.rs` and the profile verb stubs; the test kit's `fixture.rs`, `herdr.rs` and
  `profile_scope.rs`; `holler-proto/src/clock.rs`; `tests/pane_verbs/main.rs`, `launch.rs` and `verb_harness/mod.rs`; the
  four sibling briefs (`git show origin/issue-<N>-implementation:docs/handoffs/<N>-brief.md`); and greps for open codes, the
  clock helpers and shell-quoting helpers.

## O (brief gate, manual rerun and operator override): 2026-10-09T23:15:00-06:00
- **Decided:** The outside brief gate is overridden for this run, on the operator's explicit instruction ("Override and resume").
  - Round 3 of the workflow's own gate hit the reviewer's 8192-token completion cap and returned no Verdict (`gate-unavailable`, `sanity-check-failed`).
  - A hand rerun of the same round-3 prompt (`docs/handoffs/644-brief-result-r3.md.prompt.txt`) returned a complete review (`docs/handoffs/644-brief-result-r4.md`).
  - Its verdict line reads `BLOCK — 0 blocking finding(s); no defects demonstrable from the prompt's shown excerpts ... Implementation may proceed.` The label and the finding count contradict each other.
  - Rounds 1 and 2 returned real BLOCKs that were amended (15 and 13 applied).
- **Assumed:** The review's own count (0 blocking findings) is the substance and the BLOCK label is a model slip; the architecture review and the diff gate still run.
- **Hedged:** The override waives the brief gate's formal pass for this one story only; it does not extend to any other story or gate.
- **Evidence:** `docs/handoffs/644-brief-result-r4.md`; the stop result `{"stopped":"gate-unavailable","reason":"sanity-check-failed","gate":"brief"}` from run wf_323bb0af-347.

## A (up-front plan review, re-review of the amended brief) — 2026-10-09T23:29:32-06:00
- **Decided:** PASS (handoff-A.md, which replaces the BLOCK of `d2636ba`).
  - All four earlier blocks are fixed in the brief, and each fix holds against the merged code:
    - `spec_from_pane` and `FIXED_PORT_POLICY_PREFIX` (#662a);
    - E0 with `grid_given`;
    - `driven: None`, as #647's reconcile expects;
    - #663's `reconcile_step` with the exact-substring append rule, now recorded in ADR-0021 section 8 step 6.
  - T's preflight greps all pass.
  - Eight warns, all from code merged after the brief's baseline (`3bdd129`):
    1. untrusted text must go through `findings::quoted`/`embedded` or `list::text_value`;
    2. O1 must call `reconcile::shown_differs`;
    3. the merged doctor does not see a launch's leftovers (scoped passes never, whole-fleet only the Herdr pane), and a crash also leaves the port in use;
    4. decision 25's condition has fired: build on `crate::list::Rig`, and put any rig file in `launch/rig.rs`;
    5. the Herdr adapter serves one configured session, so #649 should default `--herdr-session`;
    6. the clock is a `fn()` here and a value in reconcile and park; keep it and document why;
    7. stale ADR anchors and statements, including the "step names the pane" follow-up, which contradicts step 6, and section 9's `unavailable` reason;
    8. name the positional field `pane`.
- **Assumed:** T and F work from the brief as committed at `40f486e`, on `origin/main` `d9eabbb`. The merged #643, #646a, #647 part 1, #640 part 2, #641, #642 part 1, #662 and #663 are the baseline the plan builds on.
- **Hedged:**
  - Warn 1 is a warn, not a block: no binding API or acceptance criterion changes, F can apply it within the brief's latitude on message wording, and Phase 7 re-checks it.
  - Warn 6 keeps the brief's binding `TxOptions` shape for this run rather than asking T to deviate from it.
  - Warn 3's acceptance question (ADR-0021 line 499, "the next pane doctor run finds and reports") is S's to judge; this review flags only the contract gap.
- **Evidence:**
  - Brief and prior reviews: the whole brief (2,688 lines), the first handoff-A, this journal, and the r4 gate result.
  - ADRs: `git diff 3bdd129 HEAD` of ADR-0021 and ADR-0003, plus ADR-0021 sections 3, 8, 9 and 12 as they stand now.
  - `holler-pane` and `holler-cli` source: `holler-pane` `lib.rs`, `profile_snapshot.rs`, `reconcile.rs`, `reconcile/observe.rs` and `findings.rs`; `holler-cli` `pane/{profile_scope,park,doctor,list,get}.rs` and `profile/create.rs`.
  - The adapters: `holler-adapter-herdr/src/adapter.rs`, the crate docs of `holler-adapter-opencode` and `holler-adapter-host`.
  - The test rigs: `tests/pane_verbs/{main,list,park,doctor}.rs`, `park/rig.rs` and `doctor/rig.rs`.
  - Checks: greps for the three open codes, `HostInfo.name` and the sanitizer users; T's preflight greps; the derives on the binding types.

## T (RED, test authorship) — 2026-10-09T23:45:59-06:00
- **Decided:** RED is valid: 57 tests in the `pane_verbs` target. 54 fail on the missing behaviour, and 3 pass: the two kept #670 flag tests and the rig self-check (handoff-T-red.md).
  - **API stubs.** `tx_launch.rs` holds every public item of the brief's API, with stub bodies. `launch.rs` and `relaunch.rs` hold the new `Args`, and `run` is still `not_implemented(644)`. The fixture, ADR-0003 rows 48-49 and the `stub.rs` deletion are done; the surface, docs and process tests pass.
  - **`spec: Box<SpecFlags>`** in both `Args`. Unboxed, the frozen `Command` enum fails `clippy::large_enum_variant` (408 vs 176 bytes) once the positional is added. `cli.rs` and `pane/mod.rs` cannot change, and clap 4.6 flattens a `Box<T: Args>`.
  - **The positional field is `pane`** (A warn 8).
  - **The rig** builds on `crate::list::Rig` and lives in `launch/rig.rs` (A warn 4). It adds the linked host, harness hooks only (the vanish runs from the `attach_tui` hook, so no Herdr hook is needed), `StoreScope` for AC 16k, a call-log span, and `seed_live`.
  - **Relaunch's cases start from `seed_live`**, a launched pane built through the fakes' own port methods, not from a `LAUNCH` run. With `LAUNCH` as setup, all 20 relaunch cases failed in setup on the launch stub, so their own assertions were never reached. A self-check pins the seed: `assert_matches` holds, and P's spec equals `LAUNCH`'s.
  - **Merged, to avoid duplicate scenarios:**
    - AC 16d into 10a's test;
    - AC 16f and 11b's "no command" check into AC 1;
    - AC 19b's six named engine cases into two tests, keeping the brief's names as sub-case labels.
  - AC 17's 10a case scripts the failing probe, as 10a does.
- **Assumed:**
  - Asserting the brief's text-mode "stderr carries the step" on the JSON `error.message` is equivalent, because AC 17 pins text stderr = `error: ` + the JSON message.
  - AC 8's "Close of the created pane" is shown by `Close` in the log plus an empty snapshot, because the fake's call log carries no arguments.
- **Hedged:**
  - The rig is 711 lines, its largest file; it stays under 800, so the size check's split fallback is not needed.
  - The suite (about 1,960 lines with the rig) is larger than the brief's 1,200-line estimate. That is mostly the rig's eight-method hooked harness and the seed, after rustfmt.
  - Message assertions use only the brief's contract phrases (`no record`, `was not closed`, `still answers`, `to reconcile, run` once), so F keeps its latitude on wording and on A warn 1's quoting.
- **Evidence:**
  - Preflight greps on `profile_snapshot.rs:18,21,46` and `profile_scope.rs:49,59,68`.
  - The RED run: `cargo test -p holler-cli --test pane_verbs -- launch:: relaunch::` gives 3 passed, 54 failed, and each failure's first assertion is listed in handoff-T-red.md.
  - Clean gates: `cargo clippy --workspace --all-targets -- -D warnings`, `scripts/lint.sh`, rustfmt on the 8 touched files, `cli_surface_test` + `docs_cli_test` + `pane_cli_process`, the other 133 `pane_verbs` tests, `holler-pane`, and `test-hooks.sh`.
  - Sources: `clap_builder-4.6.6/src/derive.rs:366`; the test kit's `harness.rs`, `herdr.rs`, `host.rs`, `prober.rs`, `fixture.rs` and `profile_scope.rs`; `tests/pane_verbs/{list.rs, park/rig.rs, doctor/rig.rs, process/*.rs}`.
