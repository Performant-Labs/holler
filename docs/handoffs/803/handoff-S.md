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
