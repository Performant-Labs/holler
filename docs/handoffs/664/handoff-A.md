# Handoff-A: Phase 3 - `holler profile apply` (#664)  (up-front plan review, round 4)

**Date:** 2026-10-11 (10:58 MDT)
**Branch:** `issue-664-implementation`, head `af230b6` (the brief amended for round 3), based on `origin/main` `7284001` (which matches the remote)
**Brief reviewed:** `docs/handoffs/664-brief.md` as of `af230b6`   **Reuse map:** none (no `survey.md`); "Reuse expectations for Phase 7" below stands in for one   **Wireframe:** N/A (`uiSurface: false`)
**Why a round 4:** this is a fresh run after round 3's BLOCK (`630c98c`). Since then the MO has made two commits. `3c57c10` restored the 183 reformatted files. `af230b6` amended the brief: it adds C24 and C25, settles C18 and C23, and re-cuts the Evidence from `7284001`. F's code from the first run is still on the branch, so this pass reviews the plan and uses that code as evidence.
**Verdict:** PASS

## Summary

PASS, with no block.

- **Round 3's block is fixed, and I verified it.** `bash scripts/lint.sh` exits 0. The branch's diff against `origin/main` is exactly the story's 27 paths. C24 limits formatting to the story's own files, and all 15 of those `.rs` files are already rustfmt-clean, so following the rule cannot cause a drive-by.
- **The design is unchanged and holds.** C25 puts the reconcile step for an acted refusal in the verb, which is where `pane launch` puts it.

Four warns remain. None changes the design:

1. One Evidence excerpt (E40) is broken.
2. The test for an agent difference still has no owner if #664 merges first.
3. The Blast radius does not name files the brief tells F to edit.
4. C25's behavior is missing from the ADR sentence C22 asks for.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | E40 (brief :1575-1584), the Evidence behind C8 and NV-7 | Plan accuracy; review integrity at `second-opinion` | **The re-cut left E40's second excerpt empty.** It should quote the apply row "as it stands, which §10 rewords".<ul><li>Its heading still cites `ADR-0021.md:530`. At `7284001` that line is blank; the row is at line 620.</li><li>I checked every other excerpt mechanically. Every code line of E1 to E76 (except E44 and E72, which are GitHub text) occurs verbatim in its cited files at `7284001`. C11's line numbers (198, 332, 376, 402, 516) and E33 match too.</li><li>The outside model last read the brief at 06:41 MDT. It next reads it at the diff gate, where it will find an empty block. The harm is small, because the ADR diff shows the row.</li></ul> | Re-cut E40's second block from `docs/adr/ADR-0021.md:620` at `7284001`: the row that starts `` | `profile apply` | `profile-not-found`, `pane-in-other-profile` (without `--take-over`) ``. Change the heading's `530` to `620`. Any later edit of the brief can carry this. |
| 2 | warn | C18's order clause: "if it merges after, the test is a one-line follow-up that #644 part 2's own PR picks up" | Cross-story ownership | **#644's brief was not amended to match.** On `issue-644-implementation` at `93b2cd7` (A PASS at 10:43 MDT):<ul><li>Item 7 (:44) still says "`profile apply` is not covered here ... #664 owns it".</li><li>"Not in this story" (:62) lists `profile apply (#664)`.</li><li>Its run is past A, so its T will not write an apply test.</li></ul>So if #664 merges first, no one writes the test. That is the likelier order, since #664's code exists and #644 part 2 has none. Round 3 asked for the choice to be recorded in each brief; only #664's records it.<br><br>**No code is at risk.** Apply reads `diff_profile`, and no file of this story's code or tests names `FieldValue` or `SpecField`. #644's new `FieldValue::Key` variant cannot break it. | **Simplest, inside this story:** T extends entry 11's fixture (`apply/dry_run.rs:115-175`: the dry-run plan agrees with `profile show`). The new pane is a healthy recorded pane whose spec's `opencode_agent` differs from its record's.<ul><li>Today both verbs say "matches", so the plan has an adopt.</li><li>After #644 both report the difference (row d).</li><li>The test loops over `show`'s rows, so it passes in both cases and never flips. With #644's `show` test, it covers the agent case by composition.</li></ul>**Otherwise:** the MO adds C18's order clause to #644's brief before #644's T runs. |
| 3 | warn | The Blast radius (brief :22-24 and §10), against C21, C23, C24 and the branch | Size and structure; parallel-agent coexistence | **Carried from round 1's warn 7.** The Blast radius names `tx_apply.rs`, but not `tx_apply/observe.rs` or `tx_apply/run.rs`, and C21 and C23 tell F to edit both. The branch also changes five other unlisted paths, each justified in `handoff-F.md`:<ul><li>`profile/show.rs`: `difference_line` and `text` made `pub(crate)`, for reuse.</li><li>`profile_verbs/apply.rs`: #637's test stub.</li><li>`pane_verbs/process/stub.rs`: the story's `STUBS` entry.</li><li>`CHANGELOG.md`.</li><li>`docs/research/herdr-api-spike.md`, which `docs_cli_test` needs.</li></ul>The Rules say "Edit only the files in the Blast radius". C24 says the PR "holds only this story's paths" but does not list them. So S or the outside gate could read a justified edit as a drive-by. | Add one sentence to C24, or to the Blast radius, naming the 27 paths of Phase 7 expectation 9. Until then, expectation 9 pins the set for A-dup.<br><br>The `tx_apply/` child modules follow the house pattern for overflow: `tx_launch/server.rs` (#802). |
| 4 | warn | C25 (the reconcile step on an acted `refused` step), against C22 (the ADR sentence F adds) | Consistent error handling; the contract that #666 and #803 consume | **C25 adds output that a consumer can see, but nothing documents it.** An acted step's `detail` now carries `reconcile_step(Some(P))`.<ul><li>C22's list, the "Apply as built" sentence F adds, and S's check of that sentence cover only the vocabulary.</li><li>ADR-0021's "Apply as built (#664)" (:548-574) does not mention the reconcile step even for a `failed` step.</li><li>The doc on `Step.acted` (`tx_apply.rs:119-122`) says "A `failed` step's launch or relaunch had started its act". After C25 that is wrong, because `refused` steps carry it too.</li></ul> | F's C22 sentence also states this: when a step's launch or relaunch had started its act, the step carries the reconcile step. A `refused` step carries it in its `detail`. A `failed` step carries it in the failure's message at exit 1, as `pane launch` does (ADR-0021 section 8, step 6).<br><br>F also updates `Step.acted`'s doc to cover `refused`. S checks both. |

### Round 3's findings, now

| # | Status |
|---|---|
| 1 (block: the merge's workspace reformat) | **Fixed and verified.**<ul><li>`3c57c10` restores the 183 paths, and each now equals `origin/main`.</li><li>`lint.sh` exits 0.</li><li>`git diff --name-only origin/main...HEAD` lists exactly the 27 paths.</li><li>`af230b6` changes only the brief.</li><li>C24 is in the brief, and the Rules line points to it.</li></ul> |
| 2 (warn: an acted refusal loses the reconcile step) | **Taken, as C25.** The verb adds the step, as `pane launch` does.<ul><li>`class_of` sorts every well-formed open code as a refusal (`error.rs:266-273`). So `fail_next(HarnessOp::VerifyServing, Refused { .. })` gives a `refused` step.</li><li>`HarnessOp::VerifyServing` exists in the test kit (`harness.rs:49`).</li><li>No existing apply test pins the reconcile step, so no test contradicts C25.</li><li>The ADR side is finding 4.</li></ul> |
| 3 (warn: who owns the agent-difference test) | **Half settled.** C18 settles #664's side, but #644's brief does not match it (finding 2). |
| 4 (warn: the twins issue is not filed) | **Fixed.** #818 is open (filed 10:38 MDT), its body carries C10's clause that the twins stay semantically identical to `detach` and `join`, and C23 names it. |
| 5 (warn: the ADR does not name the vocabulary) | **Planned, as C22.** F adds the sentence, and finding 4 adds C25 to it. |
| 6 (warn: the Evidence predates #802; entry 27's margin) | **Fixed, except E40 (finding 1).** C25's count of eight harness calls per `create` is right:<ol><li>`serve_command`</li><li>the plan's port `health`</li><li>`start`'s port re-check `health`</li><li>the wait's first `health`</li><li>`verify_serving`</li><li>`create_session`</li><li>`attach_tui`</li><li>`shown_session`</li></ol>Sources: `tx_launch.rs:340-351` and `641-702`, and `server.rs:94-107`. T re-derives the numbers of entries 26 and 27. |
| Round 1's warn 5 (the stale-P window; a row is chosen only once) | **Unchanged, and still open for the operator** (`handoff-F.md`, "Known issues"). |

### Drift re-check (verified, no action)

- **Layering and placement are unchanged.**
  - The engine is in `holler-pane` and works only through `Ports`.
  - `tx_launch.rs` and `tx_launch/` are untouched.
  - The verb is in `holler-cli`.
  - C25's change belongs in the verb, because `reconcile_step` is `holler-cli`'s (`pane/profile_scope.rs:49`). That is the same place `failure_body` calls it from (`pane/launch.rs:434-442`).
- **Formatting.**
  - The workspace edition is 2021 (`Cargo.toml:13`), so C24's `rustfmt --edition 2021` matches it.
  - All 15 of the story's `.rs` files pass `rustfmt --check` at HEAD.
  - `rustfmt` on `tx_apply.rs` or `profile_verbs/apply.rs` follows `mod` lines only into the story's own child modules.
- **Coexistence.**
  - The in-flight branches (#642, #644, #647, #648, #649, #650, #652) have so far changed only their own handoff files.
  - Of the paths this story changes or depends on, #644 part 2 plans to edit two. One is `ADR-0021.md`, in other sections. The other is `profile_diff.rs`, which this story does not edit.
  - #652 is docs-only and is built after #664 merges.
- **File sizes.**
  - `tx_apply.rs` is 546 lines, `observe.rs` 184, `run.rs` 240, `profile/apply.rs` 149 and `apply/rig.rs` 759. None reaches 800.
  - New tests belong outside `apply/rig.rs`, for example in `apply/failures.rs` (250 lines).
- **Public repository.** The story's diff names no personal infrastructure.

## Notes for O

None are required, since the verdict is PASS.

- **If the brief is edited again for any reason,** fold in finding 1 (E40) and finding 3 (the 27 paths). Together they are two lines and change no requirement.
- **Finding 2's choice is the MO's** and should be made before #644's T writes its tests.
- **Out of scope, noticed.** #644's brief still has the bare "`cargo fmt`" in its Rules (`644-brief.md:74`).
  - Its merge `fefa434` reformatted nothing: its diff against `main` is three handoff files.
  - Its F could still run it, as round 3's Notes for O item 5 said.

## Reuse expectations for Phase 7 (A-dup)

1. "Listed" compares only the `(session, pane_id)` pair.
2. The observation runs one thread per call (C21). `observe.rs:4` must stop saying "the shape of `reconcile`'s `observe_all`".
3. Each row's `PaneDiff` is that pane's row from `diff_profile`.
4. The twins are exact private copies of `detach`, and of `join` for one pane, with no undo and no profile delete. `run.rs`'s module doc cites #818 (C23).
5. Every step is classified through `class_of`.
6. **The reconcile step goes on every acted step (C25), `refused` steps included.**
   - The verb (`profile/apply.rs`) adds it through `reconcile_step`, with the same "already contains" guard `failure_body` uses.
   - It is not a second copy of `failure_body`'s text.
   - `Step.acted`'s doc covers `refused` (finding 4).
7. The rigs are additive, and the #802 changes stay inside the story's own rig files.
8. `orchestrator-not-relaunched` and `spec-not-launchable` are `RefusalCode` constants and are listed in ADR-0021's apply row.
9. **The PR's diff against `origin/main` holds only these 27 paths,** plus later edits to them and this run's own handoff files under `docs/handoffs/664/`:
   - `CHANGELOG.md`
   - `crates/holler-cli/src/profile/{apply,show}.rs`
   - `crates/holler-cli/tests/fixtures/cli-surface.txt`
   - `crates/holler-cli/tests/pane_verbs/process/stub.rs`
   - `crates/holler-cli/tests/profile_verbs/{apply,rig}.rs`
   - `crates/holler-cli/tests/profile_verbs/apply/{bound,dry_run,failures,observe,rig,rows,takeover}.rs`
   - `crates/holler-pane/src/tx_apply.rs` and `crates/holler-pane/src/tx_apply/{observe,run}.rs`
   - `docs/adr/ADR-0003.md` and `docs/adr/ADR-0021.md`
   - `docs/handoffs/664-brief.md`
   - `docs/handoffs/664/{decisions,evidence,handoff-A,handoff-F,handoff-T-green,handoff-T-red}.md`
   - `docs/research/herdr-api-spike.md`

   Any other path is a drive-by edit. `bash scripts/lint.sh` must still exit 0.

## Patterns referenced

- **The reconcile step on an acted failure, added in the verb:** `crates/holler-cli/src/pane/launch.rs:434-442` (`failure_body`) and `crates/holler-cli/src/pane/profile_scope.rs:49` (`reconcile_step`).
- **The act-time refusals, and the harness calls a `create` makes:** `crates/holler-pane/src/tx_launch/server.rs:87-107` and `crates/holler-pane/src/tx_launch.rs:340-351` and `641-702`.
- **An open code is a refusal:** `crates/holler-pane/src/error.rs:266-303` (`class_of`).
- **`docs/adr/ADR-0021.md` on HEAD:** :548-574 ("Apply as built (#664)"), :648 (the apply row) and :660 ("the envelope has no `detail`"). On `origin/main`, :620 is the row E40 should quote.
- **#644 part 2's brief:** `docs/handoffs/644-brief.md` on `issue-644-implementation` (`93b2cd7`), :19, :29, :44, :53-62 and :74.
