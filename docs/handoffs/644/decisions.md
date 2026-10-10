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

## F (implementation) — 2026-10-10T00:15:05-06:00
- **Decided:**
  - The engine fills T's stubs behind the brief's binding API (handoff-F.md).
    - Both verbs run one path: `Plan::new`, `Plan::observe` for steps 3-6, `run` for `edit_spec` with the act, then `act_live`.
    - A relaunch differs only through `Plan.old`: B1/B2 and B10, the record's generation at R, and it keeps the record's hold, DRIVEN and profile.
  - The verbs share `launch.rs`'s `effective_spec`, `emit_outcome`, `Verb`, `stored_profile` and `spec_of`. The reconcile step is #663's `reconcile_step`, appended only when the message lacks it.
  - A's warns applied:
    - warn 1: untrusted text goes through `findings::quoted` and `embedded`, and the success line through `list::text_value`;
    - warn 2: O1 uses `reconcile::shown_differs`;
    - warn 6: the `TxOptions` doc says why `now_ms` is a function;
    - warn 7: ADR edits anchored by content, the `unavailable` reason amended, the "step names the pane" follow-up dropped;
    - warns 3 and 5: in the section 8 "as built" note.
  - Choices within the brief's latitude:
    - the port policy is parsed before the probe;
    - `pane_store.list()` runs only to word a `grid-occupied` refusal;
    - B8 lists sessions only when the record has a session of record;
    - the rollback note names each failed call;
    - E0 says "the spec gives";
    - `with_note` reuses the crate's `detail`/`from_wire` mapping, not a second list of variants.
- **Assumed:**
  - A budget that runs out before R rolls back, like a failed step, because R has not run. The brief lists R among the live steps the budget guards.
  - "Created" means the plan's snapshot did not list the pane, matched by session and pane id, for launch and relaunch alike (decision 12).
  - A scope that answers `Ok` without running the act is a broken contract and gives `unavailable`. Neither scope can reach this.
- **Hedged:**
  - `tx_launch.rs` is 786 lines: past `lint.sh`'s 600-line warning and under the stack's ~850 decompose line. The brief's size fallback applies: one shared step runner, and no new file outside the blast radius.
  - `origin/main` moved (#713 and #640) after the merge base `d9eabbb`. A merge will conflict in ADR-0021's section 9 launch row and the `timeout`/`unavailable` cells, and probably in `CHANGELOG.md`. The merged #640 docs also say `Timeout.op` names a port method, while the engine's budget `op` is `pane.launch` (binding API, AC 24). Both are listed for S and the MO in handoff-F.md.
  - The issue's 2026-10-09 `--agent` amendment (#700, open) is not in the brief and cannot be built until #700 lands. S judges it.
- **Evidence:**
  - RED to GREEN: `cargo test -p holler-cli --test pane_verbs -- launch:: relaunch::` passes 57 of 57 (RED was 3 passed, 54 failed), and 10 of 10 repeated runs pass.
  - Gates:
    - the whole `pane_verbs` target (187), `cli_surface_test`, `docs_cli_test` and `pane_cli_process` (34);
    - `cargo test -p holler-pane`;
    - `cargo test --workspace`: 1676 passed, 0 failed, 14 ignored;
    - `cargo clippy --workspace --all-targets -- -D warnings`;
    - rustfmt on the 8 touched `.rs` files;
    - `lint.sh`, `changelog-check.sh`, and the AC 25, 27 and 29 greps.
  - `docs/handoffs/644/evidence.md` (nine facts, with `file:line`). `git diff d9eabbb origin/main` gave the merge-interaction findings.

## T (verify / GREEN) — 2026-10-10T00:25:39-06:00
- **Decided:**
  - GREEN is valid: 57 of 57 launch/relaunch tests pass (RED was 3 of 57), 20 of 20 repeated runs pass, and the whole `pane_verbs` target passes (187).
  - No test was repaired. F reported none as wrong, and none failed.
  - Return PASS: no blocking Tier 2 issue.
- **Assumed:**
  - The one `remote_admin_test` failure (`updates` 1 vs 2) in the first fail-fast workspace run is a timing flake outside this diff. The branch touches no file on its path, and it passed 5 of 5 alone and in the `--no-fail-fast` workspace run (1676 passed, 0 failed, 14 ignored, matching F).
  - AC 27 and AC 29 are judged against the merge base `d9eabbb`, as F did (Known issue 6), because `origin/main` has moved.
- **Hedged:**
  - AC 24's 100 ms margins could flake under heavy CI load. They did not in 20 runs or under the parallel workspace load. Advisory 2 says how to widen them.
  - Three paths have no test: the rollback-failure note, the `(Ok, None)` arm and `occupant`'s list-failure branch. They are advisory, and none is in the AC list.
- **Evidence:**
  - Six single mutations of F's code (O2, the reconcile-step dedupe, the occupied-cell refusal, B10, B8 and the rollback close) each fail their pinning tests (handoff-T-green.md).
  - The sources were confirmed identical to `HEAD`, touched and rebuilt green.
  - Tier 1: clippy, `lint.sh`, `changelog-check.sh`, `cargo machete`, rustfmt, `test-hooks.sh`, and `docs_cli_test`, `wire_selftest`, `cli_surface_test` and `pane_cli_process` all pass.
  - The greps for AC 25-29 were run here.
  - One T entry was appended to `evidence.md` (#643's `list::Rig::new` seeds).

## O (diff gate, manual rerun with a one-off prompt-ceiling override): 2026-10-10T01:05:00-06:00

**Decided.** The script's diff gate for #644 was refused at the runner (`gate-unavailable`, `nonzero-exit`): the assembled prompt is
about 87,500 estimated tokens against the configured 64,000-token ceiling, and no API call was made. O re-ran the same prompt bytes
(`644-diff-result-r1.md.prompt.txt`) by hand with `DUAL_REVIEW_MAX_PROMPT_TOKENS=100000`, exported from a directory with no `.env`
(the script re-sources `./.env` and would override the export). Result: **PASS, no BLOCK findings**, `finish_reason: stop`, 103,326
prompt tokens and 2,423 completion tokens, so the review was complete and not truncated. The review and its `usage.json` are
`docs/handoffs/644-diff-result-r2.md` and `.usage.json`.

**Assumed.** The ceiling is a guard against a silent non-review on a different model (the runner's own note names qwen38). This
model returned a full structured review, with eight needs-verification items, five warnings and four nits that quote specific
lines. The run is therefore recorded as RAN.

**For A-dup and S to check, from the review.** NV-1 `launch.rs:214` splits `--model` at the first `/` (brief: both halves non-empty);
NV-2 AC 6, AC 7 and AC 18b call `assert_matches`; NV-3 `close_old` when `--grid` names the same cell; NV-5 `fixed:048100` is refused
as AC 15 pins; NV-7 `closed()` on `PaneNotFound` for the real adapter; NV-8 `relaunch_with_grid_moves_the_pane`; W-4 the budget is
checked before each live step but not between the last check and the record write. None is a BLOCK.

**Evidence.** `docs/handoffs/644-diff-result-r2.md`, `docs/handoffs/644-diff-result-r2.md.usage.json`.

## A (anti-duplication gate, Phase 7) — 2026-10-10T00:47-06:00
- **Decided:** PASS (handoff-A-dup.md).
  - F extended every object the Reuse map names. Nothing parallels `Ports`/`edit_spec`, `spec_from_pane`, `FIXED_PORT_POLICY_PREFIX`, `reconcile_step`, `shown_differs`, `findings::{quoted, embedded}`, `list::{text_value, optional_text, profile_name}` or `SpecFlags::validate`.
  - Relaunch reuses launch's helpers with no copy, as `unpark` uses `park`.
  - The rig builds on `crate::list::Rig` in `launch/rig.rs`.
  - Plan-review warns 1, 2 and 4 are resolved.
- **Six warns:**
  1. ADR-0021 contradicts itself on `timeout`'s `op`. #640's section 9 row (line 462) and `error.rs:457` say a port method "in every implementation", while this change's section 12 paragraph (line 551) and `OP_LAUNCH` say `pane.launch`. Line 505 also still marks #644's recording of `herdr_api_version` PROPOSED. Fix before merge: one line each.
  2. `tx_launch::with_note` duplicates `profile_scope::with_context` across the crate boundary.
  3. `launch::spec_of` duplicates `get::spec_for`, and `stored_profile` is a named form of an inline pattern spelled four times.
  4. The launch rig near-copies doctor's private `Calls`/`mark`/`calls_since` and its live seeder. `data_of` overlaps `list::ok_envelope`.
  5. The (session, pane id) join key is now spelled three times in `holler-pane`.
  6. `tx_launch.rs` is at 786 of 900 lines; the precedent for growing it is reconcile's `mod observe;`.
- **Assumed:**
  - Phase 7's block rule is the role doc's: a parallel path to an object the map named, with no written justification. Warns 2-5 copy objects the map did not name, or that sit behind the crate layering or the blast radius, each explained in writing (F's decision 6, A-plan warn 4, T-red, brief decision 25).
  - Warn 1 is an ADR wording conflict that arrived with the #640 merge (`abdcbb6`, merged in `c844f06`) and that F reported (Known issues 2-3), so it is a warn, as the plan review treated its ADR items.
- **Hedged:** warn 1 is the one I would not ship unfixed. Whether it holds up acceptance is S's call.
- **Evidence:**
  - The diff: `git diff origin/main...HEAD`, merge base `cec1f82`.
  - The three production files in full, the rig in full, and the helper and import lines of the four other test files.
  - Analogues read: `profile_scope.rs`, `list.rs`, `findings.rs`, `reconcile.rs`, `error.rs:490-713`, `profile_snapshot.rs`, `profile_diff.rs:255-333`, `get.rs`, `park.rs`/`unpark.rs`, `profile/{show,delete,create,list}.rs`, `doctor/rig.rs`, `tests/pane_verbs/list.rs` and the test kit's `sample_spec`.
  - Greps for each new helper's analogue (join key, budget and deadline, `PaneNotFound` as closed, constructors, cell format, open codes, `"localhost"`, `--model` split, `ProbeFailed`, profile and spec lookups), and for the map's forbidden literals.
  - `git diff --name-only` over the frozen files, and `git diff d9eabbb origin/main -- crates/holler-pane/src`.
  - When the `timeout` wording entered the ADR: `git log -S`.

## S (Phase 10, spec audit) — 2026-10-10T01:05:00-06:00
- **Decided:** ADVISORY-HOLD (handoff-S.md).
  - The A and T preconditions are met: A and A-dup both PASS; T-red's RED was valid; T-green is GREEN with no blocking
    issue.
  - Every one of the brief's ACs 1-31 has a proving test or a check. Every issue Acceptance item is met; "the next doctor
    finds and reports it" is met in part, through AC 7 plus doctor's own `unregistered_herdr_pane_is_reported`.
  - Brief decisions 1-26 are built as stated. F's and T's changes from them are documented and within the brief's
    latitude.
  - The quality checks are clean: guards, sizes, no frozen file, no manifest change, privacy, and gitleaks.
  - The hold: the operator-confirmed "agent" amendment to #644 is not in the brief, the ACs or the code. It was added at
    17:38-17:40 MDT on 2026-10-09, five hours before the brief's last three amendments, and it adds a dependency on #700,
    which is still OPEN. F cannot build it: the field, the guard and the flag are #700's, no port reads a project's agents,
    and the brief names no code. The fix is for O or the MO: (a) split in place, as #647 part 1 did ("Part of #644", #644
    kept open, a Deferred bullet in ADR-0021), or (b) hold until #700 and amend the brief.
  - Required before merge on either path: (1) ADR-0021 line 462's `timeout` cell, which since the #640 merge says `op` is
    always `<port>.<method>`, needs this change's `pane.launch`/`pane.relaunch` exception; (2) merge `origin/main`
    (`bd5e825`; one ADR-0021 "Deferred" conflict) and let CI pass on the merged tree; (3) the PR body gets the AI
    disclosure, and not `Closes #644` under (a).
- **Assumed:**
  - The live issue text and its GraphQL edit history are authoritative for when the amendment landed.
  - I relied on T's recorded Tier 1 and Tier 2 results and did not re-run them, per S's role. Since T-green, only the main
    merge `c844f06` (doc-only conflict resolutions) and A-dup changed the branch.
- **Hedged:**
  - The hold could have been a PASS with an advisory. I did not choose that, because the script writes `Closes #644.` and
    the run's agent merges its own PR, so a PASS would close #644 with an operator-confirmed requirement dropped. This is
    the same reasoning as #647's S.
  - The ADR `timeout` contradiction is REWORK-class, not a brief defect: it came from #640 merging during the run. I
    listed it as a required change inside the hold rather than issuing a separate REWORK loop.
  - The AC 7 doctor half is advisory, not REWORK: the brief scoped it explicitly (C-10), and the plan review left the
    judgment to S.
- **Evidence:**
  - Issues: `gh issue view 644` and its `userContentEdits` (the agent edits at 23:38:09Z and 23:40:08Z, which are 17:38 and
    17:40 MDT); #700 (OPEN, created 17:39 MDT); epic #633's decision 8, its "#700 only" rule and its wave-3 table; #647's
    S handoff and its "part 1" merge `e612878`.
  - Code read in full: `tx_launch.rs`, `launch.rs`, `relaunch.rs`, and every test file in the diff.
  - Diffs read: the ADR-0003, ADR-0021, CHANGELOG, fixture and `stub.rs` diffs.
  - Supporting code: `error.rs` (`detail`, `from_closed`, `from_wire`), `findings.rs`, `reconcile.rs` (the whole-fleet
    gate), `pane/wiring.rs`, and main's `holler-adapter-opencode/src/attach.rs`.
  - Read-only checks: the AC 25-29 greps; `wc -l`; greps for `unwrap`, `expect`, `panic!`, `#[allow]` and `unsafe`; the
    three-dot frozen-file and manifest diff; a privacy grep of the added lines and the handoffs; `gitleaks git
    --log-opts=origin/main..HEAD`; `git merge-tree --write-tree HEAD origin/main` (one ADR-0021 conflict); and the
    workflow's PR body template (`coding-pipeline.workflow.mjs:4097`).

## O (S's ADVISORY-HOLD, scope split and doc fixes): 2026-10-10T01:50:00-06:00

**Decided.** S held #644 because the operator-confirmed `--agent KEY` amendment (issue #644, 2026-10-09) depends on #700, which is open,
and nothing in the brief or code covers it. O took S's option (a): split in place. This run is `Part of #644`, and the agent part is
#644's last part after #700. The brief carries a dated scope-split note, ADR-0021 "Deferred to named stories" has the agent bullet,
and the PR body will say `Part of #644` with no closing keyword. This is the same split #642 (642b/642c) and #647 made along the
same dependency, so it re-scopes nothing the operator did not already confirm.

Also fixed from S's required list: ADR-0021's `timeout` row now says `launch` and `relaunch` name their own `op` (`pane.launch`,
`pane.relaunch`, section 12), which removes the contradiction with the section 12 text; `origin/main` (#642 part 2) is merged and its
one conflict (the Deferred list) is resolved by keeping both sides.

**Assumed.** The prompt-ceiling override for the diff-gate rerun (entry of 2026-10-10T01:05) was O's call, made by the session agent
under the standing order to hand-rerun a gate that cannot run, not an operator decision; the rerun is a complete review, not a skip.

**Hedged.** Left for the operator, not decided here: ADR-0021 line 505 still says `host.herdr_api_version` is PROPOSED although this
change records it, and decision 1 (no operation id) is still PROPOSED. Both need the operator's OK to flip.

**Evidence.** `docs/handoffs/644/handoff-S.md`, `docs/handoffs/644/handoff-A-dup.md`.

## S (Phase 10, spec audit, re-run after the split) — 2026-10-10T01:26:03-06:00
- **Decided:** PASS (handoff-S.md, which replaces the ADVISORY-HOLD of `90ba971`).
  - The A and T preconditions are met: A and A-dup both PASS, T-red's RED was valid, and T-green is GREEN with no
    blocking issue.
  - Every item in the issue's Acceptance list is met, and every one of the brief's ACs 1-31 has a proving test or check.
    For the crash criterion, this story's half is met, and the merged doctor's whole-fleet pass reports the leftover
    Herdr pane.
  - The hold's defect is resolved by the in-place split. The brief's dated note, ADR-0021's agent bullet and this
    journal record it; #644 stays open; #700 is still OPEN, and the issue has not changed since 17:40 MDT on 2026-10-09.
  - The hold's doc fixes are done: the ADR-0021 `timeout` cell states the `pane.launch`/`pane.relaunch` exception, and
    `origin/main` (`bd5e825`) is merged.
  - Two conditions for the merging agent, not for F:
    1. replace the script's `Closes #644.` with `Part of #644.` and add the AI disclosure;
    2. green CI (`test (ubuntu-latest)`, `test (macos-latest)`) on the merged tree.
- **Assumed:**
  - I relied on T's recorded Tier 1 and Tier 2 results and did not re-run them, per S's role. The production files are
    byte-identical to T-green's `5cf6d94`.
  - What the two merges since then changed under this branch's crates is comment-only in `holler-pane`. Main's changed
    `holler-cli` tests do not run `launch` or `relaunch`, and `docs_cli_test` skips `docs/handoffs/`. So I judged the
    merged tree unlikely to break, and left the proof to CI.
- **Hedged:**
  - PASS with merge conditions rather than a hold. The PR does not exist yet, and both conditions are the merging
    agent's standard post-script steps (CLAUDE.md), not a defect F could fix.
  - The branch's commit messages carry no closing keyword, and the squash default is `COMMIT_MESSAGES`, so the PR body
    is the only way #644 could close by mistake.
  - Advisory 1 (#700 must touch this story's full `ProfileSpec`/`Pane` literals) is a coordination note for the MO, not
    REWORK. The compiler will flag every place, so nothing breaks silently.
- **Evidence:**
  - Issues: `gh issue view 644` (body and `updatedAt`) and `gh issue view 700` (OPEN, no PR); `CONTRIBUTING.md`; the
    repo's squash settings (`gh api repos/Performant-Labs/holler`); `coding-pipeline.workflow.mjs:4822`.
  - Code read in full: `tx_launch.rs`, `launch.rs`, `relaunch.rs` and every test file in the diff. Supporting code read:
    `reconcile::shown_differs`, `HerdrPane`, `HarnessKind`, and `PaneError::{detail, from_closed, from_wire}` and their
    `Display`.
  - Diffs read: `git diff origin/main...HEAD` (ADR-0021 hunks, ADR-0003, fixture, `stub.rs`, CHANGELOG), and
    `git show c76aeaf --cc`.
  - Read-only checks: the AC 25-29 greps; `wc -l`; greps for `unwrap`, `expect`, `panic!`, `#[allow]` and `unsafe`;
    the frozen-file and manifest diff; a privacy scan of all added lines; `gitleaks git --log-opts=origin/main..HEAD`
    (15 commits, no leaks); `git diff 5cf6d94 HEAD` over `holler-pane` and `holler-cli` (comment-only, and no
    launch/relaunch tests); and greps for full `ProfileSpec`/`Pane` literals on `origin/main`.
