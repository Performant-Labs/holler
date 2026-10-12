# Handoff-T-red: Phase 4 - #803, the setup wizard builds panes through `holler pane` and `holler profile apply`

**Date:** 2026-10-12
**Branch:** `issue-803-implementation` at `c1d9ff9` (no drift: `origin/main` is still `6eff9cc`, and #839 and #850 have not merged, so no REFRESH excerpt changes)
**Brief / wireframe reviewed:** `docs/handoffs/803-brief.md` (Acceptance 1 to 24, D1 to D22, the Blast radius); `docs/handoffs/803/handoff-A.md` (W1 to W12). Wireframe: N/A (no UI surface).

## A precondition

Confirmed: A returned **PASS** on the plan (0 block, 12 warn). The warns that T had to act on are folded in below: W1 to W8 and W12, plus W5's two pins.

## The printed contract these tests fix (T's choices, which F implements)

The brief names `panes.sh`'s verbs but not their output, so the tests pin one shape. The module header of `panes_803.rs` says the same.

- **`panes.sh check CONFIG`**
  - Exits 0 when the config is valid.
  - Otherwise exits 1 with one stderr message per problem, each naming the key.
  - `hub_host` alone gives exit 0 and exactly one non-empty stderr line naming `hub_host`.
  - Problems from `instance.sh`, such as the `workspace` grammar, come through as exit 1.
- **`panes.sh grid CONFIG`**
  - Prints tab-separated lines: `workspace<TAB><label>`, `extent<TAB><R>x<C>` and `profile<TAB><P>`.
  - Then one `cell<TAB><rRcC><TAB><name><TAB><harness><TAB><role><TAB><dir, ~/ expanded><TAB><port or ->` per pane, in layout order.
- **The profile name P is the workspace label (W2)**, `holler` when `workspace` is unset. It is pinned by `grid`'s `profile` line.
- **`panes.sh env CONFIG`**
  - Prints shell assignments that `eval` in bash: `WIZARD_HERDR_WORKSPACES='<label>=<R>x<C>'`.
  - `WIZARD_HERDR_SOCKET` is printed for the default instance only. Its value is `$XDG_CONFIG_HOME/herdr/herdr.sock` when that variable is absolute, else `$HOME/.config/herdr/herdr.sock` (W12b).
  - `WIZARD_TMUX_SOCKET=<state_dir>/tmux.sock` is printed for a non-default instance only.
  - A label with a space survives as one value (W12a).
- **`panes.sh plan CONFIG P [--rerun]`**
  - Prints one line per call: `<tag> <holler arguments, shell-quoted>`.
  - The tags are `profile-create`, `spec` (one per pane, in layout order), `dry-run` and `apply`.
  - `--rerun` leaves out the `profile-create` line (W3).
  - The `dry-run` line carries `--dry-run --herdr-session S --format=json`.
  - The `apply` line carries `--herdr-session S` and no `--dry-run`.
  - S is `default` for the default instance, else the instance's `herdr_session`.
- **`panes.sh teardown CONFIG P`** prints, in the same `<tag> <args>` form, `pane close NAME --profile P` for each pane in reverse layout order, then `profile delete P`. The tags are not pinned.
- **`herdr.sh holler <args...>`**
  - Runs `holler <args>` unchanged, under the shared base allow-list. It adds no `HERDR_*` and passes the caller's `HOLLER_*` through to no one (W1a).
  - It sets these literals:
    - `HOLLER_STATE_DIR` = `WIZARD_STATE_DIR`;
    - `HOLLER_HERDR_SESSION` = `WIZARD_HERDR_SESSION`, or `default` when that is empty;
    - `HOLLER_HERDR_WORKSPACES`;
    - `HOLLER_HERDR_SOCKET` and `HOLLER_TMUX_SOCKET` when their `WIZARD_*` is set.
  - **Exit 2, with `holler` never run, and the message naming the variable:**
    - `WIZARD_STATE_DIR` unset or empty;
    - a non-default instance whose `WIZARD_TMUX_SOCKET` is unset, empty or relative (W1b);
    - the default instance with `WIZARD_HERDR_SOCKET` unset.
- **`herdr.sh run`** reaches Herdr only for `status`, `session list` and `workspace list`. Any other word exits 2, with a message naming `holler pane` and no Herdr call. `label` is gone: it is now the ordinary usage error (non-zero, with "usage").

## Tests authored

Every new module is in `crates/holler-cli/tests/wizard/`, the one `wizard_scripts` target, and declared in `main.rs`. **No `Cargo.toml` change is needed**: the target already exists.

| Test (module::name) | Acceptance / warn it pins | Tier, and why |
|---|---|---|
| `panes_803::the_first_run_grid_keeps_its_eight_names_at_their_cells_with_extent_2x4` | 1 (the E67 fixture; `check` exits 0; `~/` expanded) | script unit: a pure function of a file |
| `panes_803::a_mixed_4x4_grid_has_sixteen_distinct_cells` | 1 (mixed: Claude pane port `-`; orchestrator at r4c4) | script unit |
| `panes_803::the_profile_is_named_for_the_workspace_label_which_defaults_to_holler` | W2; D4 (default label `holler`) | script unit |
| `panes_803::a_claude_pane_with_an_agent_a_port_or_no_model_of_its_own_is_refused` | 3 (`agent`, `model`), D4 / D11 (`backend_port` on Claude) | script unit |
| `panes_803::a_column_taller_than_the_one_to_its_left_is_refused_naming_layout_and_the_column` | 4 (message names `layout` and `column 2`) | script unit |
| `panes_803::every_retired_table_and_key_is_refused_by_name_with_the_word_attach` | 5 (8 fixtures; `hub_host` gives one warning line) | script unit |
| `panes_803::every_other_validity_rule_is_refused_naming_its_key` | 6 (name grammar, duplicate, layout missing or extra, relative `dir`, `harness`, `role`, no orchestrator, each of model / effort / ctx_soft / ctx_hard, ctx_soft > ctx_hard, `=` or `,` in the label) | script unit |
| `panes_803::opencode_ports_are_base_plus_index_or_their_own_and_bad_ports_are_refused` | 7 (base+i over OpenCode panes, where a pane's own port keeps its index as in `instance.sh`; the default range; a duplicate; >65535) | script unit |
| `panes_803::the_default_range_panes_sh_refuses_starts_where_instance_sh_says` | W6 (a cross-check against `instance.sh`'s printed `backend_port_base`) | script unit |
| `panes_803::no_wizard_script_uses_a_bash_4_feature` | 21 and W7e (every `lib/*.sh`, and `panes.sh` must exist) | static scan |
| `panes_803_plan::the_plan_creates_the_profile_writes_every_spec_in_layout_order_then_dry_runs_then_applies` | 2 (the tags, every spec flag, no `--herdr-session` on a spec, the dry-run and apply flags) | script unit |
| `panes_803_plan::a_rerun_plan_leaves_out_the_profile_create_and_keeps_the_rest` | W3 | script unit |
| `panes_803_plan::a_workspace_label_with_a_space_is_one_argument_in_every_plan_line` | 2, W12a | script unit |
| `panes_803_plan::a_non_default_instance_plans_with_its_own_herdr_session_and_port_base` | 2, 7 | script unit |
| `panes_803_plan::a_claude_pane_gets_its_harness_and_no_port_and_ports_count_opencode_panes_only` | 3 (`--harness claude --port-policy none`, no `--agent`; an OpenCode pane's `--agent build`) | script unit |
| `panes_803_plan::env_prints_the_workspace_extent_and_the_default_instances_herdr_socket` | 8, W12b (absolute, empty and relative XDG) | script unit |
| `panes_803_plan::env_gives_a_non_default_instance_its_own_tmux_socket_and_no_herdr_socket` | 8 (rows are column 1's height: 3x2) | script unit |
| `panes_803_plan::teardown_closes_every_pane_in_reverse_layout_order_then_deletes_the_profile` | 16 (the `panes.sh` side) | script unit |
| `herdr_803::label_is_gone_and_is_an_ordinary_usage_error` | 9, D16 | script against the fake `herdr` |
| `herdr_803::run_refuses_every_pane_and_layout_word_naming_holler_pane_and_calls_no_herdr` | 9 (12 words, for both the default and a named instance) | script against the fake `herdr` |
| `herdr_803::the_header_names_the_holler_door_and_no_pane_verb` | 9, W11 | static scan |
| `herdr_803::holler_runs_with_the_instances_literals_and_none_of_the_agents_variables` | 9, W1a (11 sentinels: `ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, a made-up secret, `HERDR_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_SESSION`, `TMUX`, `TMUX_PANE`, `HOLLER_OPENCODE_BIN`, `HOLLER_CLAUDE_BIN`, a stray `HOLLER_STATE_DIR`; no `HERDR_*` at all; no secret value printed) | script against a fake `holler` that logs its env |
| `herdr_803::holler_passes_its_arguments_through_unchanged` | 9 (a quoted label stays one argument) | script against the fake `holler` |
| `herdr_803::the_default_instance_names_its_session_default_and_sets_the_herdr_socket` | 9, D8 | script against the fake `holler` |
| `herdr_803::holler_refuses_an_unset_or_empty_state_directory_and_runs_nothing` | 9 | script against the fake `holler` |
| `herdr_803::holler_refuses_a_non_default_instance_without_an_absolute_tmux_socket` | W1b | script against the fake `holler` |
| `herdr_803::holler_refuses_the_default_instance_without_a_herdr_socket` | W1b | script against the fake `holler` |
| `proof_803::instance_b_builds_its_panes_through_the_door_with_its_own_state_and_a_is_untouched` | 2, 17, 19 (every `pane` and `profile` call carries B's state; A's snapshot is unchanged; there is no `$HOME/.holler`; doctor gives no finding) | process-level scenario: the plan run literally through `herdr.sh holler` |
| `proof_803::a_rerun_of_the_same_sequence_converges_with_no_create_and_every_pane_adopted` | W3, D9 | scenario |
| `proof_803::the_teardown_lines_leave_an_empty_registry_and_no_profile` | 16 | scenario |
| `proof_803::a_pane_of_another_profile_is_a_refused_step_exit_3_and_an_empty_stdout` | 2 (D9's Gate: exit 3, empty stdout, one `error: apply refused` line) | scenario (a test of the fake's exit contract) |
| `proof_803::a_new_row_below_a_live_row_of_two_panes_is_a_refused_grid_unreachable_step` | 2 (the planner rule E40), F5's one limit | scenario (a test of the fake) |
| `proof_803::the_default_instances_apply_agrees_with_its_environment_on_the_session_default` | W12c (the plan's `--herdr-session default` equals the door's `HOLLER_HERDR_SESSION`, and the fake refuses a mismatch as `usage`) | scenario |
| `skilltext_803::no_direct_herdr_pane_control_and_no_remote_session_in_the_skill_docs_or_readme` | 10 | text |
| `skilltext_803::the_twelve_stages_keep_their_slots_and_four_to_nine_follow_the_hub_first_order` | 11 (headings `## Stage N — <D2 name>`, the checklist rows `\| N \| <name> \|`, "a 12-stage wizard ...") | text |
| `skilltext_803::the_title_and_frontmatter_no_longer_promise_remote_sessions` | Scope 3, Amendments (a) | text |
| `skilltext_803::every_pane_call_runs_through_the_door_with_the_instances_literals` | 12, D8, W6 (Stage 0's `ls` line also lists `panes.sh`), W12 | text |
| `skilltext_803::stage_zero_checks_tmux_opencode_claude_and_the_pane_verbs_and_nothing_remote` | 22 | text |
| `skilltext_803::stage_one_asks_dirs_harnesses_the_orchestrator_and_the_model_once_and_refuses_old_configs` | 23, W2 (`<profile>`) | text |
| `skilltext_803::stage_two_is_local_reads_the_local_model_and_checks_every_pane` | 22 | text |
| `skilltext_803::stage_three_shows_the_grid_and_dry_runs_only_on_a_rerun_and_names_the_cases` | 13, 14, W4 (the `holler pane list` pre-check; no `pane-exists`), D13 | text |
| `skilltext_803::stage_four_starts_the_hub_on_loopback_with_no_advertise_and_no_serve` | 15, D7 (`--role hub --stage 4`; `serve_https_port` marked unused) | text |
| `skilltext_803::stage_five_starts_the_herdr_server_and_proves_the_adapter_reaches_it` | D7 (`--role herdr --stage 5`, `pane doctor`, `herdr-version-unsupported`) | text |
| `skilltext_803::stage_six_creates_the_profile_and_writes_only_spec_only_launches` | 24 (no live launch anywhere), D9 | text |
| `skilltext_803::stage_seven_dry_runs_then_applies_once_and_states_every_gate` | 13, 24, D9, W10 ("timeout") | text |
| `skilltext_803::stage_eight_runs_one_local_body_from_the_registry` | 15, D13 | text |
| `skilltext_803::stage_nine_reads_the_registry_and_gates_on_any_finding_but_one` | 17 (**REFRESH AFTER MERGE of #839**: it pins "DRIVEN" and "#839") | text |
| `skilltext_803::the_login_is_never_touched_and_its_findings_name_the_by_hand_remedy` | 18, D21 | text |
| `skilltext_803::teardown_closes_through_holler_then_stops_through_the_ledger` | 16, D15, W9 ("tmux") | text |
| `skilltext_803::the_adr_carries_the_wizard_sentence_the_decision_and_item_16_as_built` | 20, W11 (the sentence directly after the kept one) | text |
| `skilltext_803::the_docs_readme_index_guide_and_changelog_carry_the_change` | 20 (the two docs sections, README with no `(B)` and no `attach-mode`, `docs/README.md`, the section 6 pointer, CHANGELOG Enhancements then Breaking Changes) | text |

**Edits to harness and test code, owned by T:**

- **`proof_fakes/holler`** gains the `pane` and `profile` verbs, built from the real shapes:
  - the envelope;
  - `data.steps`, with `{pane, action, outcome, code, diff, detail}`;
  - rows a to f;
  - the `cell_key` order;
  - exits 0, 1 and 3, a JSON failure envelope, and a text `error:` line;
  - `data.pane.harness.port` and `data.pane.session_of_record`;
  - `data.panes[]` with `pos.pos`, `profile` and `sync`;
  - `data.findings`;
  - `profile list`'s array.

  It finds its registry from its own path, because the door drops `FAKE_ROOT` and every `HERDR_*` (W1). It refuses a live `pane launch` and a `--herdr-session` that differs from `HOLLER_HERDR_SESSION`. Its only finding is `profile-drift`, for a spec whose pane is not a member. `hub`, `body` and `roster` are unchanged.
- **`herdr.rs`:**
  - `Env`, `script` and `text` become `pub(super)` (W7a).
  - The six `label_*` tests are removed.
  - `every_stage_command_...` keeps only the read-only words.
  - `the_default_instance_runs_commands_as_before` uses `run workspace list`.
  - `a_pane_of_a_different_session_stops_before_any_split` becomes `..._is_refused_by_check_pane`.
  - `the_skill_runs_stages_8_and_9_...` becomes `..._stages_5_to_9_...`, which pins "Start the Herdr server" and "bare `herdr server stop`" in Stage 5.
- **`proof.rs`:** only visibility (`pub(super)`) changes. It is still 825 lines.
- **`skilltext.rs`:**
  - `read`, `skill`, `flat`, `docs`, `readme`, `all_texts`, `code_lines` and `section` become `pub(super)` (W7c).
  - The scoping word list gains `holler pane` and `holler profile`. Such a line must also contain `herdr.sh holler` (W7d).
  - `panes` joins the verb and flag check (W7b). `script_source` reads a missing script as empty, so the check names the verb rather than panicking before `panes.sh` exists.

**One deviation from the brief's module list.** `panes.sh`'s `plan`, `env` and `teardown` tests are in `panes_803_plan.rs`, split from `panes_803.rs`, so that each file stays under the 600-line warning (535 and 222 lines). It reuses `panes_803`'s fixtures. That makes five new modules, not four.

## The old pins: retired and moved

| Module | Retired (the subject is gone) | Moved or re-pinned to the new form |
|---|---|---|
| `skilltext.rs` | `stage_two_fetches_each_remote_ledger_...`, `the_join_url_and_advertise_carry_the_serve_port`, `the_remote_teardown_form_names_its_third_argument` | logs rule → Stage 5 sets `WIZARD_LOG_DIR`; herdr ledger row `--stage 5`; derived body config needs `[instance]`, `` `backend_port` ``, `` `[[pane]]` ``, `holler pane get`; ports from `panes.sh grid` (Stages 3 to 4); body config path `<state_dir>/herdr/setup/<prefix>-sessions.toml`; teardown test is now "named Herdr session" and serve-reset-never only; token label `<prefix>-local` (no `-<prefix>-<remote_host>`); `<label>` defined as the **Stage 8** token label; Stage 2 Herdr checks without the remote `$HOME` rule; the inventory flag is on every inventory line and no line is remote; `stage_three_names_the_herdr_refusals...` looks in Stage 5 and drops the serve-entry assert; check-port before the Stage **4** hub start |
| `skilltext_772a.rs` | the five zero-session tests, and `stage_six_skips_tailscale_serve...` (Stage 4 is now covered by `skilltext_803`) | `stage_ten_verifies_the_hub_and_the_orchestrator_panes` (with no "with no sessions") |
| `skilltext_772b.rs` | `stage8_labels_every_pane...`, `stage8_builds_the_grid_row_first` | status line → Stage 5; restored panes are never reused and the workspace is built "beside them" "with the user's yes" (Stage 3); stopped saved session without `workspace create`; exactly one collide line, carrying `WIZARD_HERDR_HOST=1`; docs Herdr section needs `holler pane` |
| `skilltext_w3_stage8.rs` | reuse branch on request; column widths | "a new workspace in that server" in Stage 7 and Stage 3, with no `workspace create`; docs never reuse restored panes |
| `skilltext_w4_stage8.rs` | `base_pane_is_the_new_workspaces_first_pane...` | the one-yes rule → Stage 7; the briefing block sits between Stage 7 and Stage 8 and stays name-free (the N=0 sentence is dropped); docs keep the one-yes and teardown phrases |
| `skilltext_w5_workspace.rs` | `stage8_shows_the_create_label_call...` | docs name "the top-level `workspace` key" and `workspace = "<name>"` |
| `skilltext_w3_stage1.rs` | `hub_host_is_optional_for_zero_sessions...`, `stage_six_leaves_advertise_out...` | none |
| `skilltext_w3_stage3.rs` | two collide lines; the docs' remote call | `the_collide_call_leaves_the_serve_port_empty`; `stage10_checks_the_hub...` drops `herdr.sh run pane read` (**a pin the brief's table missed**) |
| `skilltext_w4_stage2.rs` | **the whole file** (8 tests) and its `mod` line | none |
| `skilltext_w6_preflight.rs` | the `tailscale` and `ssh` rows and commands | Stage 2 no longer needs "orchestrator `dir`/`cmd`" or "model endpoint" (**missed by the brief**); the H1 lookup is `# Herdr` (the H1 changes) |
| `skilltext_w5_stage0.rs` | none | the H1 lookup is `# Herdr` (**missed by the brief**) |
| `skilltext_838.rs` | none | "before `[[pane]]`" (W5) |
| `instance.rs` | the `tailscale serve --bg ...` and the two `endpoint = ...` pins (W5) | the hub serve and `[instance]` comment pins stay |

## RED confirmation

The command is `cargo test -p holler-cli --test wizard_scripts`. It compiles cleanly, and `cargo clippy -p holler-cli --test wizard_scripts -- -D warnings` is clean.

```
test result: FAILED. 231 passed; 72 failed; 0 ignored; 0 measured; 0 filtered out
```

Every failure is an assertion about the missing behaviour or text. None is a compile error, a missing `[[test]]` or a harness timeout. Representative lines:

- **The `panes.sh` tests (18).** A check or grid that must exit 0 or 1 sees exit 127, for example `assertion left == right failed: check: bash: .../lib/panes.sh: No such file or directory`. The `refuses` cases report `accepted:` with the config, and `no_wizard_script_uses_a_bash_4_feature` reports `no lib/panes.sh: ["collide.sh", "herdr.sh", ...]`.
- **`herdr_803` (9):**
  - The `holler` verb fails with `herdr.sh: usage: herdr.sh run|server-start|label|...`. For the refusal cases that is exit 1, not 2.
  - `run_refuses_...` fails with `["run","pane","split",...]: left: Some(0) right: Some(2)`.
  - `label_is_gone` fails because `label` still succeeds.
  - The header test fails with ``label <` still in the header``.
- **`proof_803` (6).** The scenario stops at `panes.sh: No such file or directory`. The fake `holler` itself works: each scenario was run end to end against it, as described below.
- **The text pins (39)**, for example:
  - `SKILL.md still has 'pane split'`;
  - `Stage 0 lacks the row '| `tmux`'`;
  - `section 11's sentence (W11)`;
  - `named branch lacks 'after the bare top-level keys and before '[[pane]]''`;
  - `#803 D10: the write moves to Stage 7`;
  - `assertion failed: text.contains("... --role herdr --stage 5 ...")`.

**I checked that the RED suite can be satisfied** (test validation only; nothing was committed or handed to F). I wrote a throwaway reference `panes.sh` and a `holler` verb for `herdr.sh` in my scratch directory, copied them into `lib/` for one run, then restored the tree with `git checkout` and `rm`. Against them, all 33 behavioural tests passed three times in a row: `panes_803`, `panes_803_plan`, `herdr_803` and `proof_803`. `herdr`, `proof`, `instance`, `preflight`, `ownership` and `start` stayed green. That run caught one test bug, a port pool exhausted by building `Inst::second` twice, which I fixed. It also showed that `skilltext::documented_verbs_and_flags_exist_in_the_scripts` turns red for the right reason once `label` leaves `herdr.sh` while the old skill still calls it.

**What passes before the feature, on purpose.** These are guards, not part of the RED:

- the scoping test, with its new word list;
- the verb and flag check;
- `the_collide_call_leaves_the_serve_port_empty`, whose phrase is already in Stage 3;
- the subset re-pins in `skilltext_w4_stage8::docs_carry_the_same_rules` and `skilltext_w5_workspace`;
- the rename scan.

Each pins a property the new text must keep.

## Ready for F

**RED is valid; F may implement against these tests.** Notes for F:

- **Implement `lib/panes.sh` to the contract above.** Read `[instance]` and `workspace` only from `instance.sh`'s printed lines (W6), and locate `instance.sh` through `WIZARD_LIB`, else the script's own directory.
- **Add the `holler` verb and the `run` allow-list to `herdr.sh`**, with one base list shared with `server-start` (W1), and remove `label`.
- **Rewrite `SKILL.md`, `docs/setup-wizard.md`, the README, `docs/README.md`, ADR-0021, the section 6 pointer in `docs/pane-control.md` and the CHANGELOG** so the text pins pass.
- **Wording pinned literally** (F may write more around it):
  - "a new workspace in that server", "beside them", "with the user's yes";
  - "unnamed Herdr server" and "build in it" in Stage 5;
  - "`<label>` is the Stage 8 token label";
  - "Every log Stages 4 to 9 create";
  - "the \"Proceed with this plan?\" yes covers creating it" and "a \"no\" means stop" in Stage 7;
  - "hub is gone" and "tmux" in the teardown entry;
  - "not an alias" in Stage 0;
  - "plain terminal";
  - "unused" in the paragraph that names `serve_https_port`;
  - "`WIZARD_SERVE_HTTPS_PORT` empty" in Stage 3.
- **Do not keep retired text to satisfy an old pin (W5).** If an old pin I missed blocks a correct rewrite, report it and I will move it in Phase 6.
