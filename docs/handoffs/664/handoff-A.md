# Handoff-A: Phase 3 - `holler profile apply` (#664)  (up-front plan review, round 3)

**Date:** 2026-10-11 (10:40 MDT)
**Branch:** `issue-664-implementation`, head `780a578`: the merge of `origin/main` `7284001` (#802) into `d56707b` (the brief with C21 to C23)
**Brief reviewed:** `docs/handoffs/664-brief.md` as of `d56707b`   **Reuse map:** none (no `survey.md`); "Reuse expectations for Phase 7" below stands in for one   **Wireframe:** N/A (`uiSurface: false`)
**Why a round 3:** this is a fresh run after round 2's BLOCK (`0e64a87`). Since then, the brief has gained C21 to C23 and the branch has merged #802. F's code from the first run (`7f23e0f`) is still on the branch, so this pass reviews the plan and uses that code and the merge as evidence.
**Verdict:** BLOCK

## Summary

BLOCK. The block is about the branch the run would build on, not the brief's design.

- **The design holds.** C21 settles round 2's block: one thread per call, under §9's single per-call bound. C22 names the step vocabulary. Merging #802 leaves the plan's architecture intact. `tx_launch` is still called unchanged, and #802 did not change the three calls apply makes itself (`snapshot`, `health`, `shown_session`).
- **The merge broke CI.** The merge commit `780a578` also ran `cargo fmt` over the whole workspace. That rewrote 183 files in five crates. All of them are outside the Blast radius, and the rewrite is in neither merge parent.
  - `scripts/lint.sh`, CI's first gate, now exits 1. It exits 0 on `main`, and 0 on this branch once those files are restored.
  - The rewrite also adds 1.4 MB of formatting to the diff that every later gate reads.
- **No pipeline agent can undo it.** F and T are held to the Blast radius, so the fix has to come before T. It is one `git checkout` and one clarification in the brief (Notes for O).

Five warns follow:

- Two come from #802. A refusal can now arrive after the act, and that step loses the reconcile step. The Evidence describes code that no longer exists.
- Three are about the follow-ups and the ADR text that C22 and C23 promise.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | The branch the run builds on (merge `780a578`). The bare "`cargo fmt`" in the Rules line (brief:34). | Size and structure. A drive-by edit of shared code that other stories own (parallel-agent coexistence). | **What the merge did.** 183 `.rs` files differ from both merge parents: holler-body 37, holler-cli 81, holler-hub 29, holler-load-test 14 and holler-proto 22, for +15,254/−3,750 lines.<ul><li>Each file is byte-identical to `rustfmt --edition 2021` of its `origin/main` version (183 of 183).</li><li>No branch commit before the merge touched any of them (`git diff --name-only 263a2d7 d56707b`).</li><li>`main` is not rustfmt-clean, so `cargo fmt` at the root rewrites every crate.</li></ul>**What it breaks.** `bash scripts/lint.sh` (ci.yml:140) exits 1 on `780a578` and 0 on `7284001`. Three of its checks fail:<ul><li>15 files now have 900 lines or more; `main` has none. Examples: `holler-hub/src/control_server.rs` 837→1172, `circuit.rs` 898→1057, and `circuit/dispatch.rs`, the `send_prompt` choke point, 820→1017.</li><li>91 `#[allow(` lines lost their trailing `// #NNN` link, which rustfmt moved to the next line: 50 in holler-cli, 18 in holler-body, 11 in holler-hub, 11 in holler-proto and 1 in holler-load-test.</li><li>That last one, the `allow(dead_code)` at `holler-load-test/src/main.rs:56`, also fails the dead-code check.</li></ul>**What it costs the run.**<ul><li>The diff the gates read (`origin/main...HEAD`) is 1.86 MB over 210 files. Without the rewrite it is 415 KB over 27 files. For scale, the r1 outside diff prompt was 358 KB.</li><li>Every in-flight branch that touches one of these files will conflict with #664.</li></ul>**Why it cannot wait.**<ul><li>F and T are held to the Blast radius. F's Tier 1 would hit 17 lint failures in files it may not edit.</li><li>A-dup and S would each have to clear 183 drive-by files.</li><li>T-green had declined exactly this: "the repo-wide `cargo fmt --check` drift is not this branch's concern" (decisions.md, T Phase 7).</li></ul> | **Before the fresh run (MO):** restore the 183 paths from `origin/main` and commit (Notes for O 1). I checked the result on a copy of the tree: `lint.sh` exits 0, and the diff holds only the story's 27 paths.<br><br>Then add the clarification in Notes for O 2 to the brief: run rustfmt only on the files the story changes, and never run `cargo fmt` at the root, also not while merging `main`. |
| 2 | warn | §4 ("a refusal ... is a `refused` step"). C10's conflicts "from inside the launch transaction". The verb's output (O6). | Error handling: consistency with `pane launch` | **#802 made act-time refusals reachable.** Two refusals can now come from inside the launch act, after its rollback:<ul><li>`server::start` re-checks the port and refuses `port-in-use` (`tx_launch/server.rs:95-96`). By then `ensure_pane`, `ensure_session` and the spec's command have run.</li><li>`verify_serving` refuses a server that does not honour the spec (`server.rs:107`), after the server has started. The code belongs to the harness: the fake's is `agent-mismatch`, and #642 part 3 names one for an undefined agent key (644-brief.md:20).</li></ul>ADR-0021's decision 7 for #802 ("every refusal stays in the plan", :158) covers only `serve_command`. For both refusals above, `TxFailure.acted` is true.<br><br>**What apply prints.**<ul><li>F's `halted` keeps `acted` on a `refused` step (`tx_apply.rs:509-530`).</li><li>But `acted` is `#[serde(skip)]` (:121-122), and the verb adds the reconcile step only to the `failed` step at exit 1 (`profile/apply.rs:98-101`).</li><li>So an acted `refused` step prints at exit 0 with no sign that anything ran.</li><li>`pane launch`, by contrast, adds the reconcile step to every acted failure, refusals included (`pane/launch.rs:435-442`).</li></ul>**Why it matters for row f.** By the time of the refusal, the relaunch has already stopped the pane's processes (`stop_old`). Take a pane chosen only because its TUI showed another session: it ends with no server, and apply reports it only as "refused". | Name this case in the brief, for example in a C25. The verb should add `reconcile_step(Some(P))` to the `detail` of an acted `refused` step, as it already does for the failed step.<br><br>T pins one case: `fail_next(HarnessOp::VerifyServing, PaneError::Refused { .. })` on a `create`. The test asserts that the step is `refused`, that the run goes on, and that the reconcile step is in the detail. |
| 3 | warn | §13 and C18 (brief:81, 579-584). The second issue of C23. | Cross-story ownership | **#644 part 2 is closing §13's gap now.**<ul><li>Its brief, in flight on `issue-644-implementation`, adds `opencode_agent` to `SpecField::ALL` (644-brief.md:19, :58).</li><li>It says this "supersedes" #664's item 13, and that "#664's acceptance covers an agent difference in its report" (:29, :44).</li><li>#664's §13 says the agent is not this story's concern, and no C14 entry covers it. So neither brief owns the test.</li><li>Once the field is added, apply needs no code change, since it reads `diff_profile`. But a healthy pane whose agent differs from its spec moves from row c (`adopted`) to row d (`reported`).</li></ul> | **Settle who owns the test now, while both briefs are open.** Recommended:<ul><li>Do not file §13's follow-up, because #644 part 2 supersedes it.</li><li>The apply test belongs to #664, as #644 part 2's brief already says.</li><li>If #644 part 2 merges before #664's PR, #664 merges `main` and T adds the test: a healthy pane whose agent differs from its spec is `reported`, with a `FieldDiff` on `opencode_agent`.</li><li>Otherwise, the test goes in a one-line follow-up that #644 part 2's merge picks up.</li></ul>Record the choice as a clarification in each brief. |
| 4 | warn | C23 (brief:641-643) | Sequencing: the paper trail for the deliberate duplication | **F cannot cite an issue that does not exist yet.**<ul><li>C23 has F cite the twins issue in `tx_apply/run.rs`'s module doc. Today that doc says only "a follow-up of #664 merges them" (`run.rs:8`).</li><li>C23 has the run's agent file the issue only "before the PR opens", which is after F has run.</li><li>Neither issue is filed. A search of open and closed issues for the twins, `detach`, `SpecField` and `opencode_agent` found none.</li></ul> | The operator or MO files the twins issue before the relaunch; the draft is in handoff-F.md under "Known issues". The brief's C23 names the issue number, so that F cites a real one. For §13's issue, see finding 3. |
| 5 | warn | C22 ("ADR-0021 names every one of these", brief:638) | ADR, and the contract that #666 and #803 consume | **C22 states as done what is still a requirement on F.**<ul><li>ADR-0021's "Apply as built (#664)" (:548-574) and its apply row (:648) name `planned`, `spec-not-launchable` and `orchestrator-not-relaunched`.</li><li>They do not name the six `action` strings, the step object's keys `{"pane", "action", "outcome", "code", "diff", "detail"}`, or the rule that an absent `code`, `diff` or `detail` prints `null`.</li><li>The ADR also says "the envelope has no `detail`" (:660). The step's `detail` is a key of `data.steps[]`, not of the envelope, but a reader can take the two statements as contradicting each other.</li></ul> | F adds one sentence to "Apply as built" covering the step object; its six actions; its outcomes (§5's six, plus `skipped`, plus `planned` for a dry run); the `null` keys; and the fact that `detail` belongs to the step, not to the envelope's `error`. S checks the sentence against C22. |
| 6 | warn | The Evidence (header brief:717-721; E7, E10, E15, E20, E58, E67, E73, E75). C11's line numbers (:418-419). C20, O3 and E44. | Plan accuracy after #802; review integrity at `second-opinion` | **The Evidence describes code that no longer exists.** It is cut "at `origin/main` `92fbc6e` (this branch changes only this brief)", and neither half of that holds now. Since #802:<ul><li>`HarnessPort::serve` is now `serve_command` plus `verify_serving` (E7, E20).</li><li>A launch's plan starts with `serve_command` (E10, E15).</li><li>`TxOptions` gained `boot_poll` (E58), and `act_live` takes `&TxOptions` (E67).</li><li>`Budget` is now `{end, op}` (E73).</li><li>`stop_old` lives in `tx_launch/server.rs`, and the server steps are `server::start` (E75).</li><li>C20, O3 and E44 still call #802 open.</li></ul>The outside model cannot see the repo, so it will judge the diff against these excerpts.<br><br>**One effect lands on a test, not on the design.**<ul><li>A `create` now makes eight harness calls, not six: the port re-check, the health wait and `verify_serving` are new.</li><li>Entry 27 (`bound.rs:98-110`) still says "six harness calls".</li><li>Its margin is now about 100 ms of a 900 ms budget (8 × 100 ms), not 300 ms, which risks flakes on slow runners.</li></ul> | Re-cut the Evidence from `7284001`, as the operator asked for #642 and #644. Add a short clarification on what #802 changes for apply, covering findings 2 and 6.<br><br>T re-derives entry 27's numbers so that one create fits the budget with room to spare and two do not. For example, a 100 ms delay with a 1200 ms budget gives 800 ms per create. |

### Round 2's findings, now

| # | Status |
|---|---|
| 1 (block: the observation had two shapes) | **Settled by C21.** It takes option 1, one thread per call under §9's single bound, which A recommended. The code already has that shape (`observe.rs:48-77`). |
| 2 (warn: `observe.rs`'s doc) | **Planned, not done.** C21 requires the doc to drop "the shape of `observe_all`", and `observe.rs:4` still says it. F changes one sentence. |
| 3 (warn: the vocabulary) | **Named in the brief (C22).** The ADR does not name it yet (finding 5). |
| 4 (warn: the follow-ups) | **Planned (C23).** The filing order is finding 4. The overlap of §13 with #644 part 2 is finding 3. |
| 5 (warn: the stale-P window, and a row chosen only once) | **Unchanged and still open for the operator**, as in rounds 1 and 2 (handoff-F.md, "Known issues"). |
| Rigor | **Done.** The Pipeline section says `second-opinion`. |

### Drift re-check after #802 (verified, no action)

- **Untouched shared code.** Against `origin/main`, the branch does not change `tx_launch.rs`, `tx_launch/`, `ports.rs`, `lib.rs`, `holler-pane-testkit`, any `Cargo.toml`, `Cargo.lock` or the golden files. There is no new dependency and no wire or protocol change.
- **Layering.** The engine still works only through `Ports`. Apply reaches the port methods #802 changed only through `tx_launch`. Its own observation calls `snapshot`, `health` and `shown_session`, which #802 did not change.
- **The merge's changes to the story's own files are scoped:**
  - `LinkedHost::run` in `apply/rig.rs` starts the fake server through `start_server`, as #802's test kit asks.
  - `HookedHarness` implements `serve_command` and `verify_serving`.
  - Row f in `rows.rs` asserts that no pid is recorded.
  - `profile_verbs/rig.rs` changes only `serve_server`, the one helper O3 named, so O3 held.
- **`TxOptions` in the tests.** The tests build it with `..TxOptions::default()` (`apply/rig.rs:100-105`), so they pick up `boot_poll`.
- **File sizes after the restore:** `tx_apply.rs` 546 lines, `observe.rs` 184, `run.rs` 240, `profile/apply.rs` 149 and `apply/rig.rs` 759. Lint warns at 600 lines, and no file reaches 900.
- **Public repository.** The story's own diff names no personal infrastructure.

## Notes for O

1. **Undo the merge's reformat (finding 1)** in `.claude/worktrees/0664b-profile-apply`, before the fresh run:

   ```bash
   git checkout origin/main -- $(LC_ALL=C comm -12 \
       <(git diff --name-only d56707b HEAD | LC_ALL=C sort) \
       <(git diff --name-only origin/main HEAD | LC_ALL=C sort) \
     | grep -vE '^(CHANGELOG\.md|docs/|crates/holler-cli/tests/profile_verbs/)')   # the 183 paths
   git commit -m "chore(#664): undo the merge's workspace reformat (183 files outside the blast radius)"
   ```

   The filter selects exactly the 183 files I verified and none of the story's own. Then check two things:
   - `bash scripts/lint.sh` exits 0.
   - `git diff --name-only origin/main...HEAD` lists exactly 27 paths: `CHANGELOG.md`, `profile/{apply,show}.rs`, `cli-surface.txt`, `pane_verbs/process/stub.rs`, `profile_verbs/{apply.rs,rig.rs,apply/*}`, `tx_apply.rs` and `tx_apply/{observe,run}.rs`, ADR-0003, ADR-0021, `docs/handoffs/664*` and `docs/research/herdr-api-spike.md`.

2. **Amend the brief.** This also changes the brief's hash, which the re-review needs. Add, for example:

   > **C24. Formatting, and a merge of `main` (A round 3 finding 1; from the Rules for this story).** The Rules' "`cargo fmt`" means rustfmt on the `.rs` files this story changes and on no other file (`rustfmt --edition 2021 <file>`). `cargo fmt` is never run at the workspace root, including while merging `main`. `main` is not rustfmt-clean, so a root run rewrites every crate, and the result fails `scripts/lint.sh`: some files reach 900 lines, and rustfmt moves the `// #NNN` link off each `#[allow]` line. A merge of `main` changes no path outside the Blast radius beyond what `main` itself changed. The merge `780a578` broke this rule, and `<sha>` restored those 183 files from `origin/main`.

   In the same amendment, these are recommended but not blocking:
   - a short C25 on what #802 changes for apply (findings 2 and 6);
   - the Evidence, re-cut from `7284001`;
   - the owner of the agent-difference test (finding 3).

3. **File the twins issue now**, and put its number in C23, so that F can cite it (finding 4). Do not file §13's issue, because #644 part 2 supersedes it (finding 3).

4. **Then start a fresh run.** Never use `resumeFromRunId`, which replays this verdict. Little is left for F:
   - the sentence at `observe.rs:4` (C21);
   - ADR-0021's vocabulary sentence (C22, finding 5);
   - the issue citation at `run.rs:8` (C23);
   - the reconcile step on an acted refusal, if you take finding 2.

   T re-derives entry 27's numbers.

5. **The same hazard is in other briefs.** The epic's Rules boilerplate carries the same bare "`cargo fmt`" (for example `docs/handoffs/644-brief.md:74` on `issue-644-implementation`). No merge of `main` into the #642, #644 or #649 branches may run `cargo fmt` at the root either.

## Reuse expectations for Phase 7 (A-dup)

1. **Met.** "Listed" compares only the `(session, pane_id)` pair.
2. **Mechanism met.** The grain is one thread per call (C21). `observe.rs`'s doc must say so, not "the shape of `observe_all`".
3. **Met.** Each row's `PaneDiff` is the row from `diff_profile`.
4. **Met.** The twins are exact private copies of `detach` and of `join` for one pane, with no undo and no profile delete. `run.rs` must cite the filed issue (C23).
5. **Met.** Every step is classified through `class_of`.
6. **Met for a `failed` step.** An acted `refused` step is finding 2.
7. **Met.** The rigs are additive, and the #802 changes stay inside the story's own rig files.
8. **Met.** `orchestrator-not-relaunched` and `spec-not-launchable` are `RefusalCode` constants and are listed in ADR-0021's apply row.
9. **New.** The PR's diff against `origin/main` holds only the 27 paths in Notes for O 1, plus later edits to them. Any other path is a drive-by edit.

## Patterns referenced

- `scripts/lint.sh`, CI's first gate (`.github/workflows/ci.yml:140-142`): the 900-line limit and the `#[allow]` link checks.
- `crates/holler-pane/src/tx_launch/server.rs` and `crates/holler-pane/src/ports.rs` (#802): `serve_command`, `verify_serving`, the port re-check during the act, and the health wait.
- `crates/holler-cli/src/pane/launch.rs:435-442` (`failure_body`): the reconcile step on every acted failure.
- `docs/adr/ADR-0021.md`:
  - :158, decision 7 for #802;
  - :548-574, "Apply as built (#664)";
  - :648, the apply row;
  - :660, "the envelope has no `detail`".
- `docs/handoffs/644-brief.md` on `issue-644-implementation`, :19, :29, :44 and :58: `opencode_agent` joins `SpecField::ALL`.
