# Handoff-S: spec audit - #667 profile apply on scratch Herdr, tmux and OpenCode

**Date:** 2026-10-11 (20:45 MDT)
**Branch:** `issue-667-implementation` (worktree `.claude/worktrees/0667-apply-scenario`, head `4bf1524`). Its merge base with
`origin/main` is `bda1069`.
- `origin/main` is now `d6ae539` (#851).
- A trial merge (`git merge-tree --write-tree HEAD origin/main`) conflicts in `CHANGELOG.md` only.

**Issue:** #667 (epic #633), read live with `gh issue view 667`: open, no comments.
- The brief says it supersedes the issue text where the two differ (its opening paragraph, decisions 1 to 3 and 14).
- I audited against the brief, and mapped the issue's own Acceptance onto it (see "Spec compliance").

**Brief:** `docs/handoffs/667-brief.md` at `bc965ae`. I read in full Scope, the step table, Acceptance A1 to A17 and decisions 1 to 14. I checked E48, E50 and E61 against the code.

**Handoffs reviewed (under `docs/handoffs/667/`):**
- `handoff-A.md` (PASS), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md` (PASS);
- `decisions.md` (all five entries) and `evidence.md`.

**Diff audited:** `git diff bda1069...HEAD`, 21 files.
- Eleven code files, all under `crates/holler-cli/tests/`.
- `CHANGELOG.md` (+5) and `docs/pane-control.md` (-1).
- The pipeline's own brief and handoffs.

**Verdict:** PASS.

## A precondition

Met.
- `handoff-A.md`: **PASS**, with no block and nine warns. The five binding warns (W-1 to W-5) are each honoured in the diff (see "Spec compliance").
- `handoff-A-dup.md`: **PASS**, with no block, two low warns and a nit. None is binding.

## T precondition

Met.
- **RED** (`handoff-T-red.md`). Each default-run pin fails on its own assertion against the broken variant the Acceptance names. Each failing output is recorded, and the files were then restored. None of these REDs is a compile error.
  - A12: one `#[ignore]` removed.
  - A13: a guard blind to the flag, and a guard blind to the `=` spelling.
  - A14: `launch_args` without the session pair, a reader that takes a missing key as `null`, and a reader that maps an unknown name to a variant.
  - The scenario's liveness was shown with M2.
- **GREEN** (`handoff-T-green.md`). No blocking issue.
  - **Default-run pins:** green (5 in the new target, plus 2 in `guards.rs`).
  - **#649's default run:** 68 passed and 4 ignored, that is 66 plus the 2 new pins.
  - **Opted in:** the target passed twice in a row (42.84 s, then 36.92 s).
  - **Tier 1:** CI's workspace command, workspace clippy, `lint.sh`, `changelog-check.sh`, the docs tests and `cargo machete` are all clean.
  - **A16:** all seven mutants (and M3b) were killed.

## Acceptance criteria

The test file paths below are relative to `crates/holler-cli/tests/`.

| Criterion | Proving test or evidence | Status |
|---|---|---|
| **A1** Both tests pass opted in, twice in a row, leaving nothing behind | **Two runs.** T-green ran `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test profile_apply_scenario -- --ignored --test-threads=1` twice, at 20:19 MDT, and both passed.<br>**`ps` and `/tmp`.** Afterwards they matched the baseline: no `hlr649`/`holler649` process.<br>**Two transient `/tmp/hlr649-*` directories.** T-green traced them to another worktree's `pane_integration` run. I confirmed that `wired.rs`'s `trap_dir()` creates `/tmp/hlr649-*` directories that hold only an `opencode` script, and that this target does not include `wired.rs`. | Met |
| **A2** Step 1 | `profile_apply_scenario/scenario.rs` `step_1` (:133).<br>**Members:** `create`'s members are `[A1, A2]`, and Q's are `[]`.<br>**Records:** A1 and A2 are in `P`, and B1 is in no profile (`null`).<br>**`show P`:** every row `matches` with empty differences, and its text has no `differs`.<br>**Control:** `show Q` is `["missing","missing"]`, and its text says `missing (no live pane)` twice. | Met |
| **A3** Step 2 | `step_2` (:182), `step_2_dry_runs` (:226), `assert_text_plan` (:251), and `plan.rs` `assert_plan_agrees_with_show` (:201).<br>**`show P`:** A2 `differs`, on exactly the agent row; A1 `matches`; X and B1 are `missing`.<br>**Dry run without the session:** exit 2 `usage`.<br>**Dry run with it:** `dry_run` is true, the order is `[A1, A2, B1, X]`, and every row agrees with show's. That check is E61's shape, and stricter, since it checks outcomes too. B1's stated exception is asserted as `adopt`/`planned`.<br>**Text:** the header line, then one line per step.<br>**Nothing changed:** `Still` is equal and the marker is absent. M1 and M5 die here. | Met |
| **A4** Step 3 | `step_3` (:287), `assert_created_after_its_probe` (:318), `assert_joined_untouched` (:336), `step_3_again_and_converge` (:356).<br>**A1:** its `PaneStill` is unchanged, with non-empty pids.<br>**A2:** reported, and left untouched.<br>**B1:** joins P. Its generation goes up; its pids, port and session of record stay the same.<br>**X:** created in P with `probe.last` `ok`. The marker now exists, and X's pids are new.<br>**Herdr:** the snapshot is the earlier one plus exactly X, at `hlr649b` r2c1.<br>**Second apply:** nothing `created`, and `Still` is equal.<br>**`pane relaunch A2 --profile P`:** new pids, the same session, agent `plan`, and `show P` all `matches`. | Met |
| **A5** Step 4 | `step_4` (:394).<br>**Dry run, after `kill_server`:** no `--herdr-session`; B1 is planned `relaunch`/`planned`, with the detail "its harness server on port N did not answer"; `Still` is equal.<br>**Live run, without the flag:** B1 is `relaunched`. Its port answers again, its session is the same, and its pids differ from those before the kill.<br>**The other panes:** the pids of A1, A2 and X are unchanged, and A1's generation is unchanged.<br>**Model guard:** it runs on B1's new server. M4 dies here. | Met |
| **A6** Step 5 | `step_5` (:449).<br>**Without `--take-over`:** `[A1, A2]` are both `refuse`/`refused` with `pane-in-other-profile`, and `Still(Q)` is equal.<br>**With it:**<ul><li>A1 is adopted, and A2 is reported on `opencode_agent`.</li><li>Both are in Q with their pids unchanged.</li><li>A1's generation is exactly +2.</li><li>The `PaneStill` of B1 and of X is equal, so both stay in P.</li></ul>M3, M3b and M6 die here. | Met |
| **A7** Step 7 | `step_7` (:500).<br>**Relaunch of A2 into Q:** its agent is `null` and its pids are new.<br>**`apply Q`, without the flag:** A1 and A2 are adopted.<br>**Afterwards:** `show Q` is `["matches","matches"]`, and every port answers. | Met |
| **A8** Step 6, three phases | `profile_apply_scenario/probe.rs` test (:100), with phases (i) :130, (ii) :161 and (iii) :186.<br>**(i)** <ul><li>The order is `[W, Z, Y]`, and Z and W are adopted.</li><li>Y is `create`/`refused` with `probe-failed`, and `pane get Y` exits 3 with `pane-not-found`.</li><li>`Still` is equal, and Y's port refuses.</li><li>There is no tmux session `Y`. That check has teeth: a pane's tmux session bears its name, as `owned_pids` targets `={pane}`.</li></ul>**(ii)** Z is `relaunch`/`refused` with `probe-failed`, and Y is refused again. Z's port refuses, Z owns no pid, and `Still` is equal.<br>**(iii)** The same `curl` is aimed at W's port.<ul><li>Z is relaunched, Y is created, and both record `probe.last` `ok`.</li><li>Y is the only Herdr pane added, at `hlr649h` r2c1.</li><li>The model guard runs on both ports. It needs a reply from `raw`, so it also proves Y's server is up, the twin of (i).</li></ul>M2 dies here. | Met |
| **A9** End state | `end_state` (:534), and the end of test B.<br>**The shared `Scratch::assert_end_state`:** doctor is clean, the records equal Herdr's snapshot of the named workspaces, and none is unmanaged.<br>**`profile list`:** live `P` is 2 and live `Q` is 2.<br>**`profile apply hlr649-nonexistent`:** exit 3 with `profile-not-found`. | Met |
| **A10** Every JSON output parses against the envelope | **JSON verbs:** each goes through `Scratch::json`, which runs `check_envelope` and checks the exit code (`pane_integration/scratch/mod.rs:442-450`).<br>**Text-mode calls:** the only ones are `profile show P` and `Q` (step 1) and step 2's text dry run. The step table names each, and so does `scenario.rs`'s header. | Met |
| **A11** Nothing outside the scratch set is touched | **Guards:** #649's guards are unchanged and green in the default run. `check_herdr_session_args` runs before every verb (`scratch/mod.rs:349`).<br>**The binary's environment:** still `env_clear` plus `binary_env`, unchanged.<br>**End checks:** `assert_no_credential_leaked` and `assert_tmux_env_is_scratch` run at the end of both tests.<br>**The marker file:** it lies inside the scratch directory. | Met |
| **A12** Source pin | `profile_apply_scenario/pins.rs` `every_scratch_test_is_ignored_and_names_the_opt_in_variable` (:55).<br>**Its reader is not vacuous:** `the_source_pin_finds_a_scratch_test_and_its_attributes` (:82) shows it finds a scratch test with no `#[ignore]`.<br>**RED:** shown at T-red.<br>**Both `#[ignore]`s:** each is on one line and names `HOLLER_TEST_PANE_INTEGRATION=1 (#667)` (`scenario.rs:108`, `probe.rs:99`). | Met |
| **A13** Argument guard | `pane_integration/guards.rs` `a_herdr_session_argument_passes_only_when_it_names_the_scratch_session` (:152).<br>**Cases:** 4 passes and 9 refusals. The refusals include W-2's `=` forms, and a second flag in either order.<br>**RED:** shown for both broken variants.<br>**No third spelling:** clap declares `--herdr-session` only as `#[arg(long, value_name = "NAME")]`, with no short form, alias or env binding. The fields are in `crates/holler-cli/src/`: `pane/launch.rs:63`, `pane/import.rs:85` and `profile/apply.rs:63`. | Met |
| **A14** Reader and `launch_args` | **The reader** (`pins.rs`):<ul><li>`the_applied_reader_reads_the_guides_shape` (:115);</li><li>`the_applied_reader_refuses_a_missing_key` (:136), for each of the 3 top keys and the 6 step keys;</li><li>`the_applied_reader_refuses_an_unknown_action_or_outcome` (:154).</li></ul>**`launch_args`:** `guards.rs` `launch_args_is_the_flag_list_launch_sent_before_the_move` (:183). I compared its literal list with the inline list deleted from `bda1069`'s `Scratch::launch`: the same flags, in the same order.<br>**The name lists:** `ACTIONS` and `OUTCOMES` match production's 6 and 8 variants exactly. | Met |
| **A15** Default runs, clippy and lint | **Workspace:** T-green ran CI's workspace command, exit 0.<br>**Targets:** `pane_integration` 68 passed and 4 ignored; `docs_pane_control_test` 16 passed; `docs_cli_test` 3 passed.<br>**Lint:** workspace `clippy -D warnings` and `lint.sh` are clean. | Met |
| **A16** Mutants | **The run:** T-green's table. M1 to M7, and M3b, were each applied by script and the file then restored. `git diff --stat -- crates/` was empty after each. All were killed.<br>**Where:** M5 fails in its named step (2). M1 fails at step 2's dry run, and M7 at step 1's `pane launch`, which shares `below()`: earlier steps than A16 names. | Met (advisory 1) |
| **A17** CHANGELOG and the guide | **The entry:**<ul><li>decision 14's text word for word, placed first under `[Unreleased]` / `### Enhancements`;</li><li>it links #633 and #667, gives the run line, and says "Tests only: no behaviour change."</li></ul>**The guide:** section 12's #667 bullet is gone, and the list's punctuation is intact.<br>**The check:** `changelog-check.sh` is ok (F and T-green). | Met |

## Spec compliance

Each decision is implemented as stated, or deviates with a recorded reason.

- **1. The real binary through a real hub.** Every step is `Scratch::json` or `Scratch::holler`, a subprocess under `env_clear`. The test builds no store.
- **2. Reuse whole; build only what is new.**
  - The new target includes `support`, `out` and `scratch` by `#[path]` (`profile_apply_scenario/main.rs:29-36`).
  - What is new is decision 2's "Built new" list: the reader, `Still`, the plan-against-show check, the probe test and the pins.
- **3. The moves, `launch_args` and the one guard.**
  - **`Out` and `output_of`** moved to `out.rs`, with byte-identical bodies.
  - **The six helpers** moved into a second `impl Scratch` in `scratch/verbs.rs`: `record`, `session_of_record`, `findings`, `assert_tmux_env_is_scratch`, `kill_server` and `assert_end_state`. That is A's W-6, and it keeps `mod.rs` at 539 lines.
    - The bodies are the same. `C1`, `C2` and `"hlr649"` became parameters.
    - #649's call sites became method calls, with their assertions unchanged.
  - **`launch_args`** is a pure free function (W-5). `Scratch::launch` reaches it through `launch_argv`.
  - **The guard,** `check_herdr_session_args` (`scratch/mod.rs:127`):
    - it is pure and sits beside `check_session`;
    - it covers both spellings (W-2);
    - `Scratch::holler` calls it, and `guards.rs` pins it.
- **4. The gate is #649's, unchanged.**
  - Both tests carry `#[ignore = "opt-in: ...; HOLLER_TEST_PANE_INTEGRATION=1 (#667)"]`.
  - The run line is in `main.rs`'s module doc and in the CHANGELOG.
  - Opted in, `curl` (test B) and `sh` (test A) are asserted to be on the binary's `PATH`.
- **5. CI compiles the scenarios and never runs them.** No workflow was edited, and the default run is pins only.
- **6. Names.** Every pane and profile name is as the decision lists, and the module doc explains "649".
- **7. Two tests on two scratch sets,** with the exact names.
- **8. `Still`.**
  - **Where:** it lives in `plan.rs` and holds every measure the decision lists.
  - **Built only from reads:** doctor runs only inside `assert_end_state`, after the last window (W-4 (b)).
  - **Twins:** every "unchanged" claim has a twin in the same run whose reading changes:
    - X's create (step 3);
    - B1's relaunch (step 4);
    - the take-over's generation +2 (step 5);
    - test B's (iii).
- **9. The probe.**
  - The `curl` command and `expect ["healthy"]` are exactly as written.
  - The bad port is proven `refused`, and the control names W's live port.
  - The marker check is `sh -c "touch <scratch file>; echo ok"`.
- **10. The topology.**
  - The hazard shape is neither used nor pinned, and was reported to the operator (`handoff-A.md`, "Notes for O" item 4).
  - "Checked against Herdr's snapshot" is `Still::assert_one_herdr_pane_added`: the snapshot after holds every earlier `(id, workspace, cell)`, plus exactly one new one.
- **11. What the scenario does not assert.** A grep of the new target finds:
  - no `reset`, `switch` or `--first`;
  - no DRIVEN, `shown_session` or `attach_tui`;
  - no fake host and no TUI typing.

  No `detail` of an `adopt` step is asserted. The one `detail` assertion is B1's `relaunch` in step 4.
- **12. Controls and mutants:** as A8 and A16 show.
- **13. The pipeline.** A's binding W-1 settled it in the run (the brief's Question 2). T wrote all the test code, and F made the two documentation edits, touching nothing under `crates/`.
- **14. CHANGELOG, the guide and the manifest.**
  - The CHANGELOG entry is the proposed text, verbatim.
  - One bullet of the guide was deleted.
  - Neither the ADR nor `Cargo.toml` was edited.

**Deviations, each recorded with its reason** (`decisions.md`, T's Phase-4 entry, and the Notes of `handoff-T-red.md`). None is silent.

- **Test B's (ii) and (iii) pass `--herdr-session`.** `Y` has no record until (iii), so every run plans a create, and E32 makes the flag required. This is necessary.
- **The checks use `--check-json`, not `--check-arg=-s`.** The stored argv is the same; E52 documents both flags. This is equivalent.
- **`Still.registry` is `pane list`'s `data.panes`, not the whole `data`.** The unmanaged table's `idle_seconds` moves with the clock, so a `Still` built on the whole `data` could never be equal. Herdr's snapshot is still part of `Still`, so no information is lost. This is necessary.
- **`delete_strays` was not moved.** #667 never needed it: doctor was clean at both end states. W-4 (a) asks for a move only when it is needed.
- **`scenario.rs` is 551 lines**, against Scope 1's "about 450". That is under lint's warning at 600 and its gate at 900.

**The issue's own Acceptance** is covered:
- "Every step passes" is A1.
- "The registry equals what the adapters observe" is A9.
- "Every JSON output parses" is A10.
- "Nothing outside the scratch set is touched" is A11 and A13.

**No brief defect.** Two points of the brief were refined in the run, and neither leaves a contradiction for the implementation:
- Question 2 (phase F with no production code) was settled by A's W-1;
- decision 3's placement of `launch_args` in `impl Scratch` was refined by W-5.

## Quality audit

- **Correctness and failure handling.** The scenarios fail closed:
  - **The reader is strict.** A missing key or an unknown name panics with the data.
  - **`Scratch::json` checks every call.** Both the envelope and the exit code.
  - **`Applied::step` wants exactly one step per pane.**
  - **No vacuous equality.** Each pid-equality claim rests on a measure shown non-empty in the same run:
    - A1 (`scenario.rs:305`);
    - X (`:320`);
    - A2 (`:374`);
    - B1 (`:428`).

    Also, B1's `adopt` in step 3 implies a healthy server.
- **Build guards.**
  - **No `unwrap`, `expect` or `panic` outside tests.** Every changed `.rs` file is under `crates/holler-cli/tests/`.
  - **Allows.** `profile_apply_scenario/main.rs:1-5` has five crate-level allows, each ending `// #667`. They are the same five as `pane_integration/main.rs` (Scope 1, W-3). There is no `too_many_lines` or `cognitive_complexity` allow.
  - **Sizes (`wc -l`).** The largest files are `profile_apply_scenario/scenario.rs` at 551 lines and `scratch/mod.rs` at 539. No file reaches 600.
  - **No dead code.**
    - The crate-wide `dead_code` allow is for the included modules, as in #649.
    - Every non-test function in the new files has a caller. Only the seven `#[test]`s occur once.
    - `ACTIONS`, `OUTCOMES`, `STARTS_A_SCRATCH_SET` and every struct field are read.
  - **No `unsafe`.**
- **Protocol.** Nothing changed:
  - no `src/` file;
  - no golden file;
  - no wire format.

  So `docs/protocol/v2.md` needs no edit.
- **Tests.**
  - **Real processes.** Every cross-process claim runs on the real binary and a real hub.
  - **No fixed sleep.** The one `Duration` the diff adds is the timeout of `kill_server`'s moved `wait_for` poll.
  - **RED first.** The evidence is in `handoff-T-red.md`.
  - **Portable pins.** They are pure, so CI's macOS leg builds and runs them.
- **Formatting.** `rustfmt --edition 2021 --check` is clean on the six new non-`main.rs` files: `out.rs`, `verbs.rs`, `pins.rs`, `plan.rs`, `probe.rs` and `scenario.rs`. #649's existing files were not reformatted, which follows the brief's "own files only" rule.
- **Documentation.** The CHANGELOG has an `[Unreleased]` entry that links #667. There is no new log event, CLI surface or protocol field, so nothing else is due beyond section 12's bullet.
- **Public-repository privacy.**
  - **The search.** I grepped every added line (code, docs and handoffs) for personal hosts, tailnet names, private IPs, home paths, account names and key patterns.
  - **The result.** No hit. The only matches were the mutant label "M1".
  - **The fixture names** in the pins (`/tmp/hlr649-abcdef`, `holler649-0123abcd`) are invented.
- **Commit and PR hygiene.**
  - **Subjects.** All six pass `.githooks/commit-msg`'s Conventional Commit regex.
  - **Author.** The GitHub no-reply address.
  - **Trailers.** Every commit has `Co-Authored-By`, but none has a session link, which `CONTRIBUTING.md` describes as the convention. The run squash-merges, so the link belongs in the squash message (advisory 3).
  - **No PR yet.** The AI-disclosure check comes after the script opens it. CLAUDE.md gives it to the run's agent.

## Scope check

The diff delivers exactly the brief's scope.
- **Edited:** the files in the Blast radius, plus `scratch/verbs.rs`.
  - A's W-6 named that file as the place for the moves if `mod.rs` neared 600 lines.
  - It is a necessary decomposition, and `handoff-T-red.md` explains it.
- **Not edited** (`git diff --name-only`):
  - `crates/*/src/**` and any `Cargo.toml`;
  - `docs/adr/**` and `crates/holler-cli/tests/support/**`;
  - `.github/**` and the adapter crates' test directories.
- **No over-delivery.**
  - F made the two documentation edits and nothing else.
  - The `docs/testing.md` run line was left to #666 or a follow-up, as A's nit says.
- **No under-delivery.** Every row of the step table and every Acceptance item has its test or check.

## Verdict

**PASS.** Ready for O.
- A1 to A17 are each met by a named test or by recorded evidence.
- The fourteen decisions are implemented as stated, or deviate with a recorded reason.
- The quality is acceptable, and the scope is exactly the brief's.

## Advisory notes (non-blocking)

1. **A16: M1 and M7 die earlier than the brief names.**
   - **What was never mutant-tested.** Step 3's own placement check (`assert_one_herdr_pane_added` for X) has never been seen failing under a mutant, because step 1's `pane launch` shares `below()` and fails first.
   - **Why it still counts.** Its twin, a snapshot that grows by exactly one pane, is shown in both tests, so the measure can change. And it compares exact `(id, workspace, cell)` tuples.
   - **If a story wants it tested alone,** it needs a mutant in apply's create path. T-green says the same.
2. **Test B (ii) measures probe order from the end state only.** Its claim is "the probe ran before the relaunch started anything", read as "the port refuses, Z owns no pid, `Still` is equal".
   - That would not tell "never started" from "started, then fully cleaned up before returning".
   - A16 has no probe-after-start mutant.
   - Today M2 and E37 cover the order. Add such a mutant if the launch transaction's step order is reworked.
3. **The PR phase, for the run's agent.**
   - (a) Resolve the `CHANGELOG.md` conflict with #851 (`d6ae539`) by keeping both entries. `docs/pane-control.md` merges cleanly.
   - (b) Add the AI disclosure to the PR body with `gh pr edit` (`CONTRIBUTING.md`).
   - (c) Give the squash commit a `Co-Authored-By` trailer with a session link.
4. **For O or the operator (outward).**
   - A's W-9 is still open: a comment on #667 that the brief's decisions 1 to 3 and 14 supersede three things in the issue:
     - its harness text;
     - "does not depend on #649";
     - its blast radius.

     The issue has no comment yet.
   - The brief's Question 1 (decision 10's creation-order hazard, a candidate follow-up) and Question 3 (naming) remain the operator's.
5. **Follow-ups carried from A-dup (not binding).**
   - **W-1.** Move `assert_on_binary_path` beside `binary_env` in `scratch/mod.rs`, using `std::env::split_paths`, so the scratch scenarios of #831 and #839 find it.
   - **W-2.** Add `Action::ALL` and `Outcome::ALL` in production at the next story that edits `tx_apply.rs`. Until then a new variant is refused only in an opted-in run. An exhaustive `match` in `plan.rs` would make CI notice.
   - **W-3.** Add `Scratch::launch_spec_only`, and keep one `Run::port`.
6. **`docs/testing.md`** documents #640's opt-in run but not #649's or #667's (A's nit). That is for #666 or a docs follow-up.
