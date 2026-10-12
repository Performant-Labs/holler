# Handoff-S: #803 - The setup wizard builds panes through `holler pane` and `holler profile apply`  (spec audit)

**Date:** 2026-10-12, 10:37 MDT
**Branch:** `issue-803-implementation` at `f2ecfae` (worktree `.claude/worktrees/0803-wizard-panes`). `git fetch`: `origin/main` is
still `6eff9cc`, which is the merge base. No PR is open.
**Issue:** [#803](https://github.com/Performant-Labs/holler/issues/803), OPEN. Its four comments (the last at 04:27 MDT) are all
covered by the brief, read live with `gh issue view 803`. #839, #842, #850, #829 and #848 are OPEN.
**Brief:** `docs/handoffs/803-brief.md` (D1 to D22, Acceptance 1 to 24, Amendments, Rules).
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`decisions.md` and `evidence.md`.
**Diff audited:** `git diff origin/main...HEAD`: 40 files, +8625/-2585 (32 files and +4668/-2585 outside `docs/handoffs/`).
After T-green's verified commit `2698960`, only `docs/handoffs/` changed (`f2ecfae`).

**Verdict: REWORK** (`reworkKind: production`), four items: three correctness defects and one spec deviation. Items 1 and 3
are text in the skill and docs. Item 2 is `lib/panes.sh`, plus one test that T adds.

## A precondition

Met.

- `handoff-A.md`: **PASS**, with 0 block and 12 warns.
- `handoff-A-dup.md`: **PASS**, with 0 block and 4 warns.

## T precondition

Met.

- **RED** at `41a692b`: `wizard_scripts` 231 passed, 72 failed. Every failure is an assertion about the missing script, verb or
  text, and none is a compile or harness error.
- **GREEN** at `2698960`, the head for code:
  - `wizard_scripts`: 303 passed, 0 failed;
  - the CI command, `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`: 2624 passed, 0 failed;
  - clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test` and `docs_pane_control_test`: all pass;
  - 0 blocking issues;
  - six mutation spot-checks each fail the named test.
- **My checks were read-only.** I re-ran no build or test.
  - `git fetch`, `diff`, `show`, `log`, `diff --check`; `grep` and `wc -l`.
  - `gh issue view` of 803, 839, 842, 850, 829 and 848, and `gh pr list`.
  - `panes.sh` and `instance.sh` run against scratch configs in my scratch directory. Both only read a file and print.

## Acceptance criteria

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Same grid and names; 4 x 4 mixed grid has 16 cells | `panes_803::the_first_run_grid_keeps_its_eight_names_at_their_cells_with_extent_2x4`, `::a_mixed_4x4_grid_has_sixteen_distinct_cells` | Met |
| 2 | Apply alone: plan tags and flags, quoting, the sequence against the fake, `cell_key` order, the E40 refusal, exit 3 | `panes_803_plan::the_plan_creates_the_profile_writes_every_spec_in_layout_order_then_dry_runs_then_applies`, `::a_workspace_label_with_a_space_is_one_argument_in_every_plan_line`; `proof_803::instance_b_builds_its_panes_through_the_door_with_its_own_state_and_a_is_untouched` (dry-run steps in creation order, every pane at its cell in P), `::a_new_row_below_a_live_row_of_two_panes_is_a_refused_grid_unreachable_step`, `::a_pane_of_another_profile_is_a_refused_step_exit_3_and_an_empty_stdout` | Met (advisory 3) |
| 3 | Mixed harness flags; ports over OpenCode panes only; `agent` and `model` refusals on a Claude pane | `panes_803_plan::a_claude_pane_gets_its_harness_and_no_port_and_ports_count_opencode_panes_only`; `panes_803::a_claude_pane_with_an_agent_a_port_or_no_model_of_its_own_is_refused` | Met |
| 4 | A taller column is refused | `panes_803::a_column_taller_than_the_one_to_its_left_is_refused_naming_layout_and_the_column` | Met |
| 5 | Retired keys refused by name with "attach"; `hub_host` one warning | `panes_803::every_retired_table_and_key_is_refused_by_name_with_the_word_attach` | Met |
| 6 | The other validity rules, `workspace` with `=` or `,` included | `panes_803::every_other_validity_rule_is_refused_naming_its_key` | Met as tested; **bypassed by a multi-line `layout`** (REWORK 2) |
| 7 | Ports | `panes_803::opencode_ports_are_base_plus_index_or_their_own_and_bad_ports_are_refused`, `::the_default_range_panes_sh_refuses_starts_where_instance_sh_says` | Met |
| 8 | The environment literals; default label `holler` | `panes_803_plan::env_prints_the_workspace_extent_and_the_default_instances_herdr_socket`, `::env_gives_a_non_default_instance_its_own_tmux_socket_and_no_herdr_socket`; `panes_803::the_profile_is_named_for_the_workspace_label_which_defaults_to_holler` | Met (the label is wrong after a multi-line `layout`: REWORK 2) |
| 9 | `herdr.sh`: no `label`, `run` allow-list, the scrubbed door, the refusals | the nine `herdr_803` tests; `herdr::every_stage_command_carries_the_session_and_no_bare_server_stop_is_issued` | Met |
| 10 | No direct Herdr control or remote session in the skill, docs or README | `skilltext_803::no_direct_herdr_pane_control_and_no_remote_session_in_the_skill_docs_or_readme` (repaired by T-green: whole words over flattened text) | Met |
| 11 | Stage order and names; "12 stages" | `skilltext_803::the_twelve_stages_keep_their_slots_and_four_to_nine_follow_the_hub_first_order`, and the `Stage N/11` pins | Met |
| 12 | Scoping: `holler pane` and `holler profile` lines go through `herdr.sh holler` | `skilltext::holler_and_herdr_commands_in_code_blocks_are_scoped_to_the_instance` (word list and door rule); `skilltext_803::every_pane_call_runs_through_the_door_with_the_instances_literals` | Met for code blocks (prose routes: REWORK 1) |
| 13 | The dry run in Stage 3 and Stage 7 | `skilltext_803::stage_three_shows_the_grid_and_dry_runs_only_on_a_rerun_and_names_the_cases`, `::stage_seven_dry_runs_then_applies_once_and_states_every_gate` | Met |
| 14 | The migration cases | `skilltext_803::stage_three_shows_the_grid_and_dry_runs_only_on_a_rerun_and_names_the_cases` | Met |
| 15 | The body | `skilltext_803::stage_eight_runs_one_local_body_from_the_registry`, `::stage_four_starts_the_hub_on_loopback_with_no_advertise_and_no_serve` | Met |
| 16 | Teardown through Holler | `skilltext_803::teardown_closes_through_holler_then_stops_through_the_ledger`; `panes_803_plan::teardown_closes_every_pane_in_reverse_layout_order_then_deletes_the_profile`; `proof_803::the_teardown_lines_leave_an_empty_registry_and_no_profile` | Met |
| 17 | The registry check; DRIVEN `-` until #839 | `skilltext_803::stage_nine_reads_the_registry_and_gates_on_any_finding_but_one`; `proof_803` (doctor gives no finding) | Met (#839 open, so the REFRESH row stands) |
| 18 | The login | `skilltext_803::the_login_is_never_touched_and_its_findings_name_the_by_hand_remedy` | Met |
| 19 | Two-instance proof; `proof.rs` changes visibility only | `proof_803::instance_b_builds_its_panes_through_the_door_with_its_own_state_and_a_is_untouched`. `proof.rs` with `pub(super) ` stripped is byte-identical to `main`, at 825 lines. | Met |
| 20 | The amendments | `skilltext_803::the_adr_carries_the_wizard_sentence_the_decision_and_item_16_as_built`, `::the_docs_readme_index_guide_and_changelog_carry_the_change` | Met (Amendment (d) deviation: REWORK 4) |
| 21 | Hygiene | `panes_803::no_wizard_script_uses_a_bash_4_feature`; T-green's Tier 1 table | Met |
| 22 | Stages 0 and 2 are local | `skilltext_803::stage_zero_checks_tmux_opencode_claude_and_the_pane_verbs_and_nothing_remote`, `::stage_two_is_local_reads_the_local_model_and_checks_every_pane` | Met |
| 23 | Stage 1 | `skilltext_803::stage_one_asks_dirs_harnesses_the_orchestrator_and_the_model_once_and_refuses_old_configs`; `skilltext_838` (question order) | Met |
| 24 | The grid is built by apply alone | `skilltext_803::stage_seven_dry_runs_then_applies_once_and_states_every_gate`, `::stage_six_creates_the_profile_and_writes_only_spec_only_launches` | Met |

The operator-run repeat of #778, and U1 to U8, are outside the PR's gate by the brief's own rule. The text marks each as
UNVERIFIED.

## Spec compliance

- **D1 to D22 are built as decided,** with these exceptions:
  - A's warns change D3, D8, D9 and D18 as A asked, and each is recorded in `decisions.md`:
    - W1: one base list, no `HERDR_*` for `holler`, and the two socket refusals;
    - W2: P is the workspace label;
    - W3: `plan --rerun`;
    - W4: the `pane list` pre-check, and no `pane-exists`.
  - Items 1 to 4 below.
- **F's declared deviations are accepted** (handoff-F "Deviations"):
  - `WIZARD_HOLLER_BIN`;
  - the stricter door: a relative state directory is refused, and `check_pane` runs;
  - `plan` and `teardown` refuse a profile that is not this config's;
  - Stage 0's `holler pane --help` is in prose;
  - D18 case 1's wording;
  - the unlettered README menu.

  T's split of `panes_803_plan.rs` and the re-pin of `instance.rs` (W5) are declared too.
- **One silent deviation (item 4):** Amendment (d) says the creation order is "stated once, where #853 states it". The docs
  restate it.
- **Undeclared over-delivery (item 2):** `panes.sh` parses a multi-line `layout`, but `instance.sh` cannot.

## Quality audit

- **Correctness and failure handling.**
  - **`herdr.sh` behaves as the brief says:**
    - the `run` allow-list;
    - `label` is gone;
    - `holler` runs under `env -i` with the base list and the instance's literals;
    - the three refusals exit 2 and run nothing.
  - **`panes.sh` matches T's printed contract.** I confirmed the old documented config: only the two retirement messages and
    the `hub_host` warning, exit 1. I also confirmed shell quoting of a `dir` that holds a space and a `'`.
  - **Two defects reach a user or an agent following the text:**
    - item 1, a pane verb outside the door;
    - item 3, `pane reset` without `--as-operator`.
  - **One defect is a silent misread by the new script:** item 2.
- **Build guards.**
  - No Rust production code, wire, protocol or golden file changed.
  - Each new module carries `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #803`.
  - No `.rs` file is at 900 lines. The largest touched files are `preflight.rs` (890, untouched), `proof.rs` (825, unchanged)
    and `herdr.rs` (751). The new modules are 222 to 544 lines.
  - Clippy is clean, so there is no dead code.
- **Tests.**
  - Script-level tests for `panes.sh` and `herdr.sh`, with fakes first on `PATH`.
  - `proof_803` runs every plan line literally through the door, on `proof.rs`'s host.
  - No fixed sleep in any new test.
  - RED-first evidence is in T-red, and mutation evidence is in T-green.
- **Documentation.** The new script surface (`panes.sh`, `herdr.sh holler`) is documented in `docs/setup-wizard.md`.
  - **CHANGELOG:** Enhancements and Breaking Changes items link #803, in `docs/releasing.md`'s bucket order.
  - **ADR-0021:** section 11's sentence sits right after the kept one; item 16 reads "As built"; "Not built here" is updated;
    and decision 34 is added.
  - **Other files:** the README, `docs/README.md`, the section 6 pointer in `pane-control.md`, and the two-instances bullet.
- **Public-repository privacy: clean.** I grepped all 8625 added lines.
  - No real hostname, tailnet name, account, machine name, home path or key-shaped string appears.
  - The only IPs are `127.0.0.1`, and `0.0.0.0` in retired-text evidence.
  - The `.ts.net` hits are `<placeholder>` forms inside the brief's verbatim evidence.
  - The only credential-shaped value is the test sentinel `sk-sentinel-anthropic`.
- **Commit and PR hygiene.**
  - Subjects are Conventional Commits.
  - Every non-merge commit carries `Co-Authored-By: Claude <noreply@anthropic.com>`. That is the pipeline's convention on
    `main`, with no session link.
  - The PR is not open. Its body needs the AI disclosure line (CONTRIBUTING.md, added with `gh pr edit`) and `Closes #803`.

## Scope check

- **Delivered:** Scope items 1 to 4 (the contract and docs, `panes.sh` and `herdr.sh`, `SKILL.md`, the tests).
- **Not touched, as item 5 says:** Rust production code, the selector (#859), remote sessions, #829, DRIVEN and #850,
  `profile import`, and an adoption verb.
- **Over-delivery:** declared and accepted (above), except multi-line `layout` parsing, which causes item 2.
- **Under-delivery:** none.

## Verdict

**REWORK** (`reworkKind: production`). F changes the text and the script. T adds the one test that item 2 names.

1. **A pane verb from the user's own shell, around the door, can stop another instance's pane.**
   - **Where:** `agent-skills/herdr-setup/SKILL.md:1510-1513` and `docs/setup-wizard.md:645-647`. Both lines are new in this
     story.
   - **What they say:**
     - Stage 11's note says to close or relaunch a pane with `holler pane close` / `holler pane relaunch` "with the
       instance's `HOLLER_STATE_DIR` in your own shells".
     - The docs give the same verbs with no environment at all.
   - **What the code does:**
     - A pane verb keeps each configuration error until that port's first use (`crates/holler-cli/src/pane/wiring.rs:148-162`).
     - With `HOLLER_TMUX_SOCKET` unset, the host uses tmux's default server (`wiring/env.rs:86-94`).
     - `close_live` calls `host.stop_owned` before `herdr.close` (`pane/close.rs:165-186`).
     - `stop_owned` signals every owned process group of the session of that name on that server
       (`holler-adapter-host/src/lib.rs:370-391`).
   - **The failure, for a named instance:** `HOLLER_STATE_DIR=<state_dir> holler pane close r1c1` reads the named
     instance's record, then stops the owned processes of the session `r1c1` on tmux's default server. When the default
     instance (or the fleet) runs a Holler pane `r1c1` there, those are its processes. Only then does the call fail with
     `usage` (no `HOLLER_HERDR_SESSION`), and the named instance's own pane keeps running.
   - **Following the docs' line instead,** a named-instance user acts on the default instance's registry and pane.
   - **For the default instance,** the harness is stopped and the close then fails, which leaves the record and the
     Herdr pane in place.
   - **A-dup's warn 1** assumed the call "touches nothing". It does not stop before acting.
   - **Change:** in both files, give the user the door: `<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane close NAME
     --profile <profile>` (and `... pane relaunch NAME`). Have Stage 10's report print the literal `<herdr_env>` line and
     the absolute `lib` path. Alternatively, print all of the instance's `HOLLER_*` values for the user's shell: state dir,
     session, workspaces, and the Herdr socket or tmux socket. Never print a pane verb with `HOLLER_STATE_DIR` alone, or
     with no environment.
2. **`panes.sh` accepts a multi-line `layout` that `instance.sh` misreads, so a later `workspace` is silently dropped.**
   - **Where:** `agent-skills/herdr-setup/lib/panes.sh`:
     - `:206-211` and `:448-454`: the multi-line `layout` parse;
     - `:404-415`: the label is taken only from `instance.sh`'s `workspace:` line;
     - `:39-41`: the header claims panes.sh accepts "exactly the same TOML subset".
   - **Why:** the unchanged `instance.sh:69-81` reads every line that starts with `[` as a table header. So a
     continuation line (`["o1", "a1"],`) ends its top level, and a `workspace` after it is never read or validated.
   - **Repro** (a scratch run; the rest of the config is valid):
     - **The config:** `layout = [` / `["o1", "a1"],` / `["b1"]` / `]`, then `workspace = "my relay"`.
     - **What the scripts do:**
       - `instance.sh` prints no `workspace:` line;
       - `panes.sh check` exits 0, saying `workspace holler, profile holler`;
       - `plan` prints `--workspace holler --profile holler`;
       - `workspace = "a=b"` in the same place also exits 0, which bypasses Acceptance 6;
       - on a one-line `layout`, that value is refused (exit 1).
   - **Effect:**
     - The wizard builds and records a workspace and profile the user never named.
     - Two such configs with different names both become `holler`, so the second reads as D18 case 2, a rerun of the first.
   - **Change:** either fix keeps A's W6, with no second parse of `workspace`'s value:
     - (a) refuse a `layout` that does not close on its own line (exit 1, naming `layout`: "write it on one line"); or
     - (b) when panes.sh's own pass saw a top-level `workspace` key and `instance.sh` printed no `workspace:` line, refuse
       naming `workspace`.

     T adds a `panes_803` case with a multi-line `layout` followed by `workspace`.
3. **`holler pane reset PANE` on the orchestrator is refused as written.**
   - **Where:** `agent-skills/herdr-setup/SKILL.md:1466-1468` and `docs/setup-wizard.md:282-284`. Both lines are new in this
     story.
   - **Which pane:** the orchestrator of the briefing proof, which D8 makes the proof of U2. Its record has
     `role = orchestrator`.
   - **What the code does:** `holler-pane/src/tx_switch.rs:345-350` refuses it as `orchestrator-pane` without
     `--as-operator`. See also `pane/reset.rs:29-31` and `docs/pane-control.md:250,275`.
   - **Change:**
     - Write `holler pane reset PANE --as-operator` through the door, with the user's yes. The text already asks for it.
     - Say what the reset leaves, as "If a stage fails" says for a relaunch:
       - an OpenCode orchestrator gets a new session of record, so Stage 8's body config is derived again and the body is
         restarted;
       - the old session stays as a `stray-session` finding (`docs/pane-control.md:268-269`).
4. **The docs restate apply's creation order** (`docs/setup-wizard.md:159-161`: "(a workspace's column 1 by row, then the
   rest by row and column)").
   - **The deviation:** Amendment (d) asks for pointers, not copies: "the creation order is stated once, where #853
     states it". This is A-dup's warn 3.
   - **Change:** drop the parenthetical. The section 6 link at lines 164-165 covers it. SKILL.md Stage 7's statement stays,
     because D3 compares the dry run against it.

## Advisory notes (non-blocking)

1. **The door's socket guard is one-sided** (A-dup warn 2, T-green advisory 2).
   - A stray `WIZARD_HERDR_SOCKET` reaches a named instance as `HOLLER_HERDR_SOCKET`.
   - A stray `WIZARD_TMUX_SOCKET` moves the default instance off tmux's default server.
   - The risk is low, because `<herdr_env>` is built from `panes.sh env` in a fresh shell.
   - A refusal in `holler_guard` and two `herdr_803` cases would close it.
2. **`panes.sh` refuses a valid `[ext.<namespace>]` table that holds a multi-line array.** Its continuation line is read as
   an "unknown table", yet the docs say the wizard ignores `[ext.*]`. It fails closed, and item 2's fix may cover it.
3. **Gaps in the fake `holler`:**
   - It has no `failed` step (exit 1).
   - No scenario pins a `reported`-only exit 0. Those rows of D9's Gate are pinned in text only.
   - For a 2 x 2 grid, layout order equals `cell_key` order, so `proof_803` does not tell the fake's sort apart. A
     three-column fixture would.
4. **A removed pane that is still live.** Stage 3 case 2 and "If a stage fails" offer `pane close NAME --profile P
   --spec-only` for a pane the config dropped.
   - For a live pane, that leaves it running as a member of P with no spec.
   - `profile-drift` fires only for a member that differs from its spec (`reconcile/drift.rs:57-65`), so Stage 9 does not
     flag it.
   - For a live pane, offer the live close instead.
5. **The default instance's panes and tmux's environment.**
   - The host starts a harness with `new-session` / `new-window ... -- env --` and passes no `-e`
     (`holler-adapter-host/src/tmux.rs`). So a pane inherits the tmux server's environment.
   - For the default instance, a tmux server the user already runs supplies that environment, not the door.
     `HOLLER_STATE_DIR` then defaults to `$HOME/.holler`, which is still right.
   - Stage 6's sentence about the first call starting the tmux server holds only when no server runs on that socket.
     Worth checking in #778's repeat.
6. **ADR-0021 decision 34 wraps `holler profile` / `apply` inside one code span** (T-green advisory 1). With T's repaired
   pin, `holler profile apply NAME` on one line is safe.
7. **`noslash` is an unlisted copy of `strip_slash`** (A-dup warn 4).
8. **The REFRESH rows stand.** #839 and #850 are open, so Stage 9's DRIVEN sentence and D13's "no body row for a Claude
   Code pane" are as written. Re-check before the PR.
9. **`git diff --check`:** 153 trailing-whitespace lines, all inside the brief's verbatim evidence. None is in a deliverable.

---

# Handoff-S (rework 1): #803 - The setup wizard builds panes through `holler pane` and `holler profile apply`  (spec audit, second pass)

**Date:** 2026-10-12, 11:33 MDT
**Branch:** `issue-803-implementation` at `741cb7c` (worktree `.claude/worktrees/0803-wizard-panes`). `git fetch`: `origin/main` is
still `6eff9cc`, the merge base. No PR is open for this branch.
**Issue:** [#803](https://github.com/Performant-Labs/holler/issues/803), OPEN, with no comment since 04:27 MDT. #839 has PR #863 open
(opened 11:01 MDT, not merged). #842, #850, #829 and #848 are OPEN.
**Brief:** `docs/handoffs/803-brief.md`, unchanged since `7819016`.
**Handoffs reviewed:**
- this file's first pass (REWORK items 1 to 4);
- `handoff-F-rework.md`, and the rework 1 sections of `handoff-T-green.md` and `handoff-A-dup.md`;
- `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `decisions.md` and `evidence.md` (F's seven new entries included).

**Diff audited:**
- **The branch:** `git diff origin/main...HEAD`, 42 files, +9690/-2588. Outside `docs/handoffs/`, that is 32 files and +4828/-2588.
- **This cycle:** `f2ecfae..HEAD`, read in full. It changed `panes.sh`, `SKILL.md`, `docs/setup-wizard.md`, `CHANGELOG.md`,
  `panes_803.rs` and `skilltext_803.rs`, plus the handoffs.
- **After T-green's verified commit `43f9047`:** only `decisions.md` and `handoff-A-dup.md` changed.

**Verdict: PASS.**
- All four REWORK items are fixed.
- Each fix has a test, and T showed that each test fails against the file as it was before the rework.
- I found nothing new that blocks. The advisory notes are for O and for follow-ups.

## A precondition

Met.

- `handoff-A.md`: **PASS**, with 0 block and 12 warns.
- `handoff-A-dup.md` (rework 1): **PASS**, with 0 block and 2 warns. Both are advisory notes 2 and 3 below.

## T precondition

Met.

- **RED** at `41a692b`: 231 passed and 72 failed. Every failure is an assertion (handoff-T-red.md).
- **GREEN** at `43f9047` (rework 1):
  - `wizard_scripts`: 307 passed, 0 failed, also with mawk and with busybox awk first on `PATH`;
  - the CI command with `--no-fail-fast`: 167 binaries, 2628 passed, 0 failed;
  - clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test`, `docs_pane_control_test` and
    `wire_selftest`: all pass;
  - 0 blocking issues.
- **Mutation evidence.** Each of the four new tests fails against the file from `7a9dbec`, before the rework. One added bare
  verb also fails the door-verb test.
- **My checks were read-only.** I re-ran no build or test. I used:
  - `git fetch`, `diff`, `log` and `show`, `diff --check`, `grep` and `wc -l`;
  - `gh issue view` and `gh pr view` / `gh pr list`;
  - `panes.sh` and `instance.sh` on scratch configs in the session scratchpad. Both only read one file and print.

## The four REWORK items

| # | First pass | Fix | Proof | Status |
|---|---|---|---|---|
| 1 | A bare pane verb from the user's own shell, with `HOLLER_STATE_DIR` alone or no environment, can stop another instance's pane | <ul><li>Stage 10's report prints `<herdr_env>` and the absolute `lib` path, with every value written out (`SKILL.md:1489-1498`).</li><li>Stage 11's first note (`SKILL.md:1530-1538`) and the docs' "Basic Herdr commands" (`docs/setup-wizard.md:654-663`) give `<herdr_env> bash <lib>/herdr.sh holler pane close NAME --profile <profile>` and `... pane relaunch NAME`, and say what a bare verb does.</li><li>Both "own shells" sentences now say a pane or profile call needs the door: Stage 1 (`SKILL.md:582-583`) and "Running an instance beside another" (`docs/setup-wizard.md:361-362`).</li><li>The door's flags exist: `pane close` and `pane relaunch` take `--profile` (`pane/args.rs:21-24`). Neither needs `--herdr-session`: only `launch`, `import` and `profile apply` take it.</li></ul> | `skilltext_803::the_users_own_pane_verbs_go_through_the_door_with_the_literals_stage_ten_prints` | Fixed |
| 2 | With a multi-line `layout`, `instance.sh` never reads the `workspace` after it. The wizard then builds under `holler`, and `workspace = "a=b"` bypasses Acceptance 6 | <ul><li>`panes.sh` refuses a `layout` left open on its own line: exit 1, naming `layout` and "on one line" (`panes.sh:217`, `364`).</li><li>It skips the continuation lines, so the mistake gives one message.</li><li>The rule is stated in the header (`:41-45`), the `LAYOUT` comments, Stage 1's list of refusals and the CHANGELOG.</li></ul> | <ul><li>`panes_803::a_layout_over_several_lines_is_refused_so_a_later_workspace_is_never_dropped`.</li><li>My scratch runs: S's repro exits 1. So does `workspace` written before an open `layout`. `plan` exits 1 with an empty stdout. The one-line form prints `workspace my relay` and `profile my relay`, and `workspace = "a=b"` after it is refused by `instance.sh`.</li></ul> | Fixed |
| 3 | `holler pane reset PANE` on the orchestrator is refused as `orchestrator-pane` | <ul><li>Stage 10 (`SKILL.md:1471-1480`) and the docs' `AGENTS.md` bullet (`docs/setup-wizard.md:285-292`) run `pane reset PANE --as-operator` through the door, with the user's yes.</li><li>They say what the reset leaves. An OpenCode orchestrator gets a new session of record: derive the body config again and restart the body. The old session stays as a `stray-session`. A Claude Code orchestrator leaves neither.</li><li>The flag exists (`pane/reset.rs:31-32`), and the refusal is `tx_switch.rs:345-355`.</li></ul> | `skilltext_803::the_orchestrators_reset_is_as_operator_and_names_the_stray_session_it_leaves` | Fixed |
| 4 | The docs restate apply's creation order, against Amendment (d) | <ul><li>The parenthetical is gone. The section 6 pointer now names "Apply's creation order" (`docs/setup-wizard.md:161-167`).</li><li>`SKILL.md` Stage 7's statement stays, because D3 compares the dry run with it.</li></ul> | `skilltext_803::the_docs_point_to_applys_creation_order_and_do_not_restate_it` | Fixed |

**Also taken: the first pass's advisory 7 (A-dup warn 4).** `noslash` is now `strip_slash`, byte-identical to
`instance.sh:43-44`, and the header lists it.

## Acceptance criteria

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | The same grid and names; a 4 x 4 mixed grid has 16 cells | `panes_803::the_first_run_grid_keeps_its_eight_names_at_their_cells_with_extent_2x4`, `::a_mixed_4x4_grid_has_sixteen_distinct_cells` | Met |
| 2 | Apply alone: the plan's tags and flags, the quoting, the sequence against the fake, `cell_key` order, the E40 refusal, exit 3 | <ul><li>`panes_803_plan::the_plan_creates_the_profile_writes_every_spec_in_layout_order_then_dry_runs_then_applies`, `::a_workspace_label_with_a_space_is_one_argument_in_every_plan_line`.</li><li>`proof_803::instance_b_builds_its_panes_through_the_door_with_its_own_state_and_a_is_untouched`, `::a_new_row_below_a_live_row_of_two_panes_is_a_refused_grid_unreachable_step`, `::a_pane_of_another_profile_is_a_refused_step_exit_3_and_an_empty_stdout`.</li><li>Checked this pass: every `spec` flag exists in `pane/args.rs`'s `SpecFlags`, `rRcC` parses as `GridPos` (`grid.rs:44`), and `fixed:` / `none` are `profile_snapshot.rs:20,24`.</li></ul> | Met (advisory 6) |
| 3 | Mixed harness flags; ports over OpenCode panes only; the `agent` and `model` refusals on a Claude Code pane | `panes_803_plan::a_claude_pane_gets_its_harness_and_no_port_and_ports_count_opencode_panes_only`; `panes_803::a_claude_pane_with_an_agent_a_port_or_no_model_of_its_own_is_refused` | Met |
| 4 | A taller column is refused | `panes_803::a_column_taller_than_the_one_to_its_left_is_refused_naming_layout_and_the_column` | Met |
| 5 | Each retired key is refused by name, with "attach"; `hub_host` gives one warning | `panes_803::every_retired_table_and_key_is_refused_by_name_with_the_word_attach` | Met |
| 6 | The other validity rules, including `=` or `,` in the label | `panes_803::every_other_validity_rule_is_refused_naming_its_key`. The bypass is closed by `::a_layout_over_several_lines_is_refused_so_a_later_workspace_is_never_dropped` | Met (item 2 fixed) |
| 7 | Ports | `panes_803::opencode_ports_are_base_plus_index_or_their_own_and_bad_ports_are_refused`, `::the_default_range_panes_sh_refuses_starts_where_instance_sh_says` | Met |
| 8 | The environment literals; the default label `holler` | `panes_803_plan::env_prints_the_workspace_extent_and_the_default_instances_herdr_socket`, `::env_gives_a_non_default_instance_its_own_tmux_socket_and_no_herdr_socket`; `panes_803::the_profile_is_named_for_the_workspace_label_which_defaults_to_holler`; the positive control of item 2's test | Met |
| 9 | `herdr.sh`: no `label`, the `run` allow-list, the scrubbed door, the refusals | the nine `herdr_803` tests; `herdr::every_stage_command_carries_the_session_and_no_bare_server_stop_is_issued` | Met |
| 10 | No direct Herdr control and no remote session in the skill, the docs or the README | `skilltext_803::no_direct_herdr_pane_control_and_no_remote_session_in_the_skill_docs_or_readme` | Met |
| 11 | The stage order and names; "12 stages" | `skilltext_803::the_twelve_stages_keep_their_slots_and_four_to_nine_follow_the_hub_first_order`; the `Stage N/11` pins | Met |
| 12 | Scoping: every pane and profile call goes through `herdr.sh holler` | <ul><li>`skilltext::holler_and_herdr_commands_in_code_blocks_are_scoped_to_the_instance`;</li><li>`skilltext_803::every_pane_call_runs_through_the_door_with_the_instances_literals`;</li><li>the routes the user runs: `::the_users_own_pane_verbs_go_through_the_door_with_the_literals_stage_ten_prints`.</li></ul> | Met (item 1 fixed) |
| 13 | The dry run in Stages 3 and 7 | `skilltext_803::stage_three_shows_the_grid_and_dry_runs_only_on_a_rerun_and_names_the_cases`, `::stage_seven_dry_runs_then_applies_once_and_states_every_gate`. Checked this pass: in a dry run a refusal is `refused`, a report is `reported`, and an act is `planned` (`tx_apply.rs:504-523`), so the dry-run Gate reads the right field | Met (advisory 4) |
| 14 | The migration cases | `skilltext_803::stage_three_shows_the_grid_and_dry_runs_only_on_a_rerun_and_names_the_cases` | Met |
| 15 | The body | `skilltext_803::stage_eight_runs_one_local_body_from_the_registry`, `::stage_four_starts_the_hub_on_loopback_with_no_advertise_and_no_serve` | Met |
| 16 | Teardown through Holler | `skilltext_803::teardown_closes_through_holler_then_stops_through_the_ledger`; `panes_803_plan::teardown_closes_every_pane_in_reverse_layout_order_then_deletes_the_profile`; `proof_803::the_teardown_lines_leave_an_empty_registry_and_no_profile` | Met |
| 17 | The registry check; DRIVEN shows `-` until #839 | `skilltext_803::stage_nine_reads_the_registry_and_gates_on_any_finding_but_one`; `proof_803`, where doctor gives no finding; `::the_orchestrators_reset_is_as_operator_and_names_the_stray_session_it_leaves`, for the `stray-session` the reset leaves | Met (the REFRESH row stands: advisory 1) |
| 18 | The login | `skilltext_803::the_login_is_never_touched_and_its_findings_name_the_by_hand_remedy` | Met |
| 19 | The two-instance proof; `proof.rs` changes visibility only | `proof_803::instance_b_builds_its_panes_through_the_door_with_its_own_state_and_a_is_untouched`. Checked again: `proof.rs` with `pub(super) ` stripped equals `main`, at 825 lines | Met |
| 20 | The amendments | `skilltext_803::the_adr_carries_the_wizard_sentence_the_decision_and_item_16_as_built`, `::the_docs_readme_index_guide_and_changelog_carry_the_change`, `::the_docs_point_to_applys_creation_order_and_do_not_restate_it` | Met (item 4 fixed) |
| 21 | Hygiene | `panes_803::no_wizard_script_uses_a_bash_4_feature`; T-green's Tier 1 table (rework 1) | Met |
| 22 | Stages 0 and 2 are local | `skilltext_803::stage_zero_checks_tmux_opencode_claude_and_the_pane_verbs_and_nothing_remote`, `::stage_two_is_local_reads_the_local_model_and_checks_every_pane` | Met |
| 23 | Stage 1 | `skilltext_803::stage_one_asks_dirs_harnesses_the_orchestrator_and_the_model_once_and_refuses_old_configs`; `skilltext_838` (the question order) | Met |
| 24 | The grid is built by apply alone | `skilltext_803::stage_seven_dry_runs_then_applies_once_and_states_every_gate`, `::stage_six_creates_the_profile_and_writes_only_spec_only_launches` | Met |

**Outside the PR's gate.** The operator-run repeat of #778, and U1 to U8, are outside the gate by the brief's own rule. The text
marks each one as UNVERIFIED.

## Spec compliance

- **D1 to D22 are built as decided.** The exceptions are recorded in `decisions.md`:
  - A's W1 to W4;
  - F's declared deviations from the first cycle, accepted in the first pass.
- **This cycle's deviation is declared, and I accept it.**
  - **What it is.** Option (a) narrows `panes.sh check` by one input shape: a `layout` written over several lines. That also
    refuses two shapes that `instance.sh` reads correctly: a `layout` after `workspace`, and continuation lines that do not
    start with `[`.
  - **Why it holds.** D1 makes `panes.sh` depend on `instance.sh`'s output, and the header now states their shared subset:
    one value per line. No fixture, doc example or test writes a multi-line `layout`.
  - **Where users see it.** The config docs, Stage 1 and the CHANGELOG all state the rule.
- **No silent deviation remains.** The first pass's one (item 4) is fixed.
- **This cycle's extra edits stay within the items' class:**
  - the `strip_slash` rename;
  - the one-line rule in user-facing text;
  - the two "own shells" sentences.

## Quality audit

- **Correctness and failure handling.**
  - **The fix for item 2 holds.** It holds before and after `workspace`, for an unterminated `layout`, and under `plan`.
  - **I probed `panes.sh` against `instance.sh` for other disagreements:**
    - an `[instance]` table after the `[[pane]]` tables (both read it alike);
    - `workspace` inside a `[[pane]]` table (refused);
    - quoted or dotted top-level keys (refused);
    - an `[ext.*]` table with one-line values (accepted, and ignored).

    Only two leniencies of `instance.sh`'s own grammar remain, and both fail closed (advisory 5).
  - **Items 1 and 3 route the user through the unchanged door,** which runs the scrub, `holler_guard` and `check_pane`.
  - **The words Stage 7 reads match the code** (`tx_apply.rs:156-201`): the actions, the outcomes and `planned`.
- **Build guards.**
  - No Rust production, protocol, golden, `cli-surface` or Cargo file changed.
  - `instance.sh`, `collide.sh`, `inventory.sh`, `ledger.sh` and `stop-owned.sh` are unedited, as are `preflight.rs`,
    `ownership.rs` and `start.rs`.
  - Every `#![allow]` in a changed test file carries a `// #NNN` link.
  - No touched `.rs` file reaches 900 lines. The largest are `proof.rs` (825, unchanged), `herdr.rs` (751) and
    `skilltext_803.rs` (625, past the 600-line warning).
- **Tests.**
  - The new test for item 2 is at the script tier. The three text pins are at the text tier.
  - Each new test asserts what a user sees: the exit code, the key it names, no bare verb, `--as-operator`.
  - No fixed sleep was added.
- **Documentation.** `CHANGELOG.md` has two items under `## [Unreleased]`, both linking #803:
  - the Enhancements item;
  - the Breaking Changes item, now with the one-line rule, placed before Bug Fixes.

  The docs and the skill state every new refusal.
- **Public-repository privacy: clean.** I grepped all 9690 added lines.
  - No IP but `127.0.0.1` and `0.0.0.0` appears.
  - No home path, user, machine or account name appears.
  - The only key-shaped value is the test sentinel `sk-sentinel-anthropic`.
  - The two `.ts.net` hits are `*.example.ts.net` placeholders inside the brief's verbatim evidence. `main` already uses the
    same form.
- **Whitespace.** `git diff --check` on everything outside `docs/handoffs/` is clean.
- **Commit hygiene.**
  - Every subject is a Conventional Commit.
  - Every non-merge commit carries `Co-Authored-By`, without a session link. That is how `main`'s recent squash commits look.
  - The PR body is the run agent's step (advisory 8).

## Scope check

- **Delivered:** the four REWORK items, plus the extra edits named under Spec compliance.
- **Untouched, as Scope item 5 says:** Rust production code, the selector (#859), remote sessions, #829, DRIVEN and #850,
  `profile import`, and an adoption verb.
- **Over-delivery:** none undeclared.
- **Under-delivery:** none. S's advisories 1 to 6 were not taken. They are non-blocking, and F declared them.

## Verdict

**PASS.**

- The four REWORK items are fixed and tested.
- Every acceptance criterion from 1 to 24 has a test that asserts the behavior.
- The implementation follows D1 to D22, with every deviation declared.
- The build guards, the documentation and the privacy checks pass.

Ready for O.

## Advisory notes (non-blocking)

1. **The REFRESH rows: #839 is close.**
   - **The state.** #839's PR #863 is open: opened at 11:01 MDT, `MERGEABLE`. The brief orders #839 to merge before #803.
   - **If #863 merges first,** the run must do these things before this PR merges:
     - re-cut R6 and R15;
     - patch Stage 9's sentence "DRIVEN prints `-` until #839";
     - add any sentence #839 asks of Stage 8 (D13);
     - update the pin in `skilltext_803::stage_nine_...`.

     The only file the two PRs share is `CHANGELOG.md`, and that conflict is mechanical.
   - **#850 is open,** so D13's "no body row for a Claude Code pane" stands.
2. **The door's socket guard is one-sided** (A-dup rework warn 2, and the first pass's advisory 1).
   - **Why it matters more now.** The user's shell is now a sanctioned caller of the door.
   - **What a leak needs.** An *exported* `WIZARD_HERDR_SOCKET` (for a named instance) or `WIZARD_TMUX_SOCKET` (for the
     default instance). The printed `<herdr_env>` is a command prefix and exports nothing.
   - **The fix.** It is XS: refuse those two in `holler_guard`, and add two `herdr_803` cases. Take it in the first follow-up
     that touches `herdr.sh`.
3. **A CLI follow-up for O** (A-dup rework warn 1).
   - **The text.** Three passages describe what a bare `holler pane close` does: it stops the tmux session's processes, then
     fails on the missing Herdr session.
   - **The root.** The cause is in the CLI: `wiring.rs:148-160` and `close.rs:180-184`.
   - **The follow-up.** On epic #633: a verb that acts answers its `usage` configuration errors before its first act. Then the
     three passages can shrink to "never a bare pane verb; use the door".
4. **The dry run's exit code.**
   - **What the text says.** Stage 3 case 2 and Stage 7 say "a dry run exits 0 whatever it plans" (the brief's D3 and D9).
   - **What the code does.** `profile/apply.rs:8-10` exits 1 for a `failed` step "for a dry run too". That happens when an
     observation fails, and then only the failure is printed.
   - **Why it is safe.** It fails closed: with no `data.steps`, the steps differ from Stage 3's table, so the agent stops.
   - **A possible later edit:** "exits 0 for any plan, 1 when an observation failed".
5. **Two leniencies of `instance.sh`'s unchanged grammar.** Both fail closed.
   - **A duplicate top-level `workspace`** takes the last value.
   - **A label that ends in a space** is allowed by `instance.sh:195`, for example `"relay "`. It becomes a P that
     `ProfileName::parse` trims (`profile.rs:43-45`), so `grid` prints `relay ` while the registry names `relay`. The result is
     a refusal (`profile-exists`, or `panes.sh`'s exit 2), not a misbuild.
6. **Carried from the first pass, still open:**
   - **Gaps in the fake `holler`:** it has no `failed` step and no three-column fixture. T suggests a follow-up test story.
   - **`--spec-only` for a removed pane that is still live:** Stage 3 case 2 and "If a stage fails" offer it.
   - **ADR-0021 decision 34** wraps the `holler profile` / `apply` code span across two lines.
   - **The default instance's tmux environment:** check it in #778's repeat.
7. **Process.**
   - **What the brief asks.** Its "Amend first" rule wants the ADR and docs edits as the first commit after the brief.
   - **What happened.** They landed in F's commit, after T's RED, which is the pipeline order the brief itself prescribes. The
     squash merge hides the order.
   - **What holds.** The rule's second half holds: the tests were RED before any script or skill text changed.
8. **For the run's agent, when the PR is open:**
   - the body needs `Closes #803`;
   - it needs the AI disclosure that CONTRIBUTING.md requires ("Disclosure is required"), added with `gh pr edit` if the
     script's body lacks it;
   - it should carry the pipeline-run summary, as `b76f4d2`'s does.
9. **Other notes:**
   - **U1 to U8** stay UNVERIFIED by design until the operator's repeat of #778.
   - **`profile_rename_faults_test` flakes on an ext4 `TMPDIR`** (F and T-green). It is unrelated to #803, and the hub crate
     is unchanged. O may file F's suggested follow-up against #665.
