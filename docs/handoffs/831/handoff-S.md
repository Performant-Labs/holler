# Handoff-S: #831 adopt - adopting the fleet's existing Claude Code panes  (spec audit)

**Date:** 2026-10-12
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-adopt`, head `63e147e`; `origin/main` is `ef881fc`)
**Issue:** #831
**Handoffs reviewed:** `handoff-A.md` (third pass, PASS), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md` (PASS), and `docs/handoffs/831-adopt-brief.md` (the brief amended twice); the issue; the diff `git diff origin/main...HEAD` (code in `af8709a`..`1f962c0`).

## A precondition

Met. `handoff-A.md` is PASS (0 blocks, 5 warns, all folded in); `handoff-A-dup.md` is PASS (0 blocks, 2 non-blocking warns).

## T precondition

Met. `handoff-T-red.md`: RED valid (one compile-level root cause, `HostPort::foreground_pids`; 46 + 2 + 2 assertion failures with the minimal contract in). `handoff-T-green.md`: GREEN, no blocking issue, CI-form `cargo test --workspace` exit 0, repeats for flakes, mutation spot-checks (drop `Several`, drop `NoSession`, drop `login_env`, leader-only `ps_group`) each fail a named test.

## Acceptance criteria

| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| AC1 | Amend first: ADR-0021 + guide (method, mechanism, 5 codes, item 17 "As built", section 9 row, section 13 switch, deferred, decision 23); no decision number written | `docs_pane_control_test.rs::the_adoption_is_documented_as_built`, `the_import_help_no_longer_says_the_flag_is_refused`, plus the existing vocabulary tests (23 pass); ADR diff read: `HostPort` row, section 2 "Amended by #831" paragraph, section 5 runner paragraph, section 9 `pane import` row, section 13 "The switch", "Deferred", decision 23 note, two decisions listed with "next free number at merge" | MET (ordering: see Advisory 1) |
| AC2 | Flag no longer `usage`; `CLAUDE_CODE_STORY` gone | `harness.rs::adopt_claude_code_is_not_refused_and_a_dry_run_writes_nothing` (replaces the usage test); `grep CLAUDE_CODE_STORY crates/holler-cli/src` prints nothing (re-run) | MET |
| AC3 | Flag off changes nothing; wording reworded, code/level/action unchanged | `harness.rs::the_claude_code_finding_says_how_to_adopt_the_pane`, `without_the_switch_a_claude_pane_makes_no_foreground_pids_call`, `a_flag_off_claude_pane_whose_launcher_entry_has_a_cwd_and_no_port_stays_skipped`, `a_port_less_launcher_entry_for_an_opencode_pane_is_source_missing_without_the_switch`, `a_port_less_entry_with_a_relative_cwd_is_source_unreadable_without_the_switch`; `grep "until #831"` over crates and the guide finds only the docs test's own comment | MET |
| AC4 | Flag on: `kind: claude`, no port, pid and session via `foreground_pids` then `shown_session`, health unknown, no command, `fleet`, cwd from launcher; D3 model/env/context/no agent; none of the three OpenCode helpers; `sessions.toml: claude` stays skipped; three-state `Adopt` | `adopt.rs::a_typed_in_claude_pane_is_adopted_with_the_process_and_session_agents_json_lists` (whole record vs `expected_claude`), `the_chain_of_an_adopted_pane_reads_tmux_then_one_session_per_pid`, `the_opencode_only_files_checks_do_not_apply_to_an_adopted_pane`, `a_default_agent_in_pane_env_is_ignored_and_leaves_no_difference`, `a_model_that_is_not_anthropics_is_a_warning_and_the_pane_is_still_adopted`, `an_anthropic_model_in_any_case_gets_no_warning`; `adopt_problems.rs::an_unlabelled_claude_pane_reads_no_pid_and_no_session`, `an_ambiguously_labelled_...`, `a_pane_whose_sessions_toml_names_claude_is_still_skipped_with_the_switch`, `a_launcher_entry_with_a_cwd_and_no_port_is_enough_to_adopt`; code: `adopt.rs` (`Adopt::{Off,On,OnUnlabelled}` in `observe.rs`, `adopt::read`, `assess_adopted`) | MET |
| AC5 | Existing `fleet` extended by one `cas_put` at the generation read; without flag `profile-exists`; `FleetPlan`; empty `added` skips the write; `spec-kind-differs` | `adopt_fleet.rs::an_existing_fleet_is_extended_by_one_cas_put`, `with_the_switch_an_existing_fleet_takes_opencode_panes_too`, `an_extension_with_no_new_spec_writes_the_record_and_leaves_fleet_alone`, `an_existing_spec_of_another_kind_is_kept_and_warned_about`, `a_pane_added_since_is_recorded_into_fleet_beside_the_adopted_ones`; existing `an_existing_fleet_with_a_pane_to_record_is_profile_exists` unchanged; code: `import.rs` `FleetPlan`, `extend`, `writes.rs::write` | MET |
| AC6 | Idempotent re-run | `adopt_fleet.rs::a_second_run_after_an_adoption_is_a_no_op` (with and without the switch; no host, Claude or profile read) | MET |
| AC7 | Acts on nothing live | `adopt.rs::the_adoption_reads_and_acts_on_nothing_live` (dry and real call logs) | MET |
| AC8 | Next-step info finding, summary counts | `adopt.rs::an_adopted_pane_has_one_info_finding_naming_relaunch`, `the_text_summary_counts_the_adopted_panes_only_when_there_are_some` | MET |
| AC9 | Never on a guess: no pid, several (walk stops at the second), empty session, `foreground_pids`/`shown_session` failure, login variable; all-or-nothing `ps` | `adopt_problems.rs::a_process_agents_json_does_not_list_is_unmatched_and_blocks`, `two_matching_processes_block_and_the_walk_stops_at_the_second`, `a_matching_process_with_no_session_is_unmatched_and_blocks`, `a_failed_foreground_pids_is_harness_unreadable_and_asks_no_session`, `a_failed_shown_session_is_harness_unreadable`, `a_login_variable_in_pane_env_blocks_the_pane_without_its_value`, `a_launcher_entry_that_cannot_give_the_directory_blocks_the_adopted_pane`, `a_missing_pane_env_blocks_the_adopted_pane`; adapter `foreground_pids_bound_test.rs::a_deadline_reached_on_the_second_pane_is_a_timeout_and_no_partial_list`; each blocked case also runs the real run (`import-blocked`, nothing written) | MET |
| AC10 | All or nothing with undo in two shapes, one generalised undo; planned-record equality; extension remedy names no `profile delete`/`pane close` | `adopt_undo.rs`: `an_extension_whose_first_record_write_fails_keeps_nothing`, `..._second_record_write_fails_deletes_the_first_only`, `an_extension_record_that_landed_and_timed_out_is_deleted_too`, `a_failed_fleet_write_deletes_every_record_and_leaves_fleet_as_it_was`, `a_concurrent_change_of_fleet_is_a_conflict_that_keeps_the_other_actors_version` (`concurrent_put`), `the_undo_leaves_another_actors_record_of_the_same_name_alone`, `a_failed_undo_step_of_an_extension_names_what_remains_and_no_deleting_command`, `a_creation_with_an_adopted_pane_is_undone_with_the_profile`, `a_creation_whose_profile_delete_fails_still_names_profile_delete`; code: `writes.rs` `Write::{Record,Fleet}`, `created: Option<(&Profile,&Actor)>`, `ours` equality | MET |
| AC11 | Dry run: "to extend: N specs added, M total", JSON keys unchanged, equals real plan | `adopt.rs::a_dry_run_shows_the_adoption_and_a_real_run_does_what_it_showed`; `adopt_fleet.rs::a_dry_run_against_an_existing_fleet_says_to_extend_and_equals_the_real_plan` | MET |
| AC12 | Mixed kinds, round-trip with no `differs` | `adopt.rs::one_run_adopts_a_claude_pane_and_records_an_opencode_pane_into_one_fleet` (`profile show`, `profile apply --dry-run` plans `adopt` for both), `in_a_mixed_run_only_the_opencode_pane_asks_a_server` | MET |
| AC13 | Gates | `handoff-T-green.md`: CI-form workspace test exit 0; `bash scripts/lint.sh` exit 0 and `scripts/changelog-check.sh` ok (both re-run by S, read-only); clippy clean for the story's crates (one pre-existing failure in `token_load_test.rs`, untouched by the diff); no test starts a real `claude`, tmux, Herdr (the one real-tmux case is `#[ignore]`) | MET |
| D2/D4 | `foreground_pids` through `ps -t`, strict tty parse, group members, all or nothing, seams, conformance | `foreground_pids_test.rs` (13), `foreground_pids_bound_test.rs` (6), `host_conformance_test.rs` case 12 and four mutants, `fake_host_test.rs`, `forwarding.rs`, real-tmux `foreground_pids_includes_a_program_typed_into_the_shell` (`#[ignore]`, run by F and T: 10 passed); code read: `tmux.rs::parse_ttys`/`is_tty_name`, `exec.rs::ps_group`, `lib.rs::foreground_pids` | MET |

## Spec compliance

Every decision in the brief is implemented as stated:

- **D1** (flag extends `fleet` for every recorded pane): `plan` reads `fleet` only with the flag and a record; `Extend` covers OpenCode panes too (`with_the_switch_an_existing_fleet_takes_opencode_panes_too`).
- **D2/D4** (`foreground_pids` via `ps -t`): the mechanism matches the brief step by step (tmux listing `#{pane_dead} #{pane_tty}`, strict parse incl. `..`, leading `-`, `/dev/` prefix; `ps -o pid= -o pgid= -o tpgid=`, `LC_ALL=C`; foreground-group members; sorted and de-duplicated; one deadline; `with_ps_binary`). F also rejects a leading `/` in the relative tty name (listed in the F handoff as a refinement, stricter than the brief, harmless).
- **D3** (copy as read, warn, never an agent): `keeps_agent: false`, provider compared case-insensitively, variable name only in the finding.
- **D5** (extension undo is record deletes only): no `Stop::Profile` for an extension; remedy text has neither `profile delete` nor `pane close`.
- **Launcher**: only "cwd present, port absent" changed; the port-less `source-missing` is emitted on the OpenCode path with the identical words (from `LauncherEntry.about`); the flag-off Claude path still ignores `files.problems`.
- **Final finding codes**: the five new codes are in `ALL`, `code()`, `level()` with the levels in the brief; the kinds doc says twenty-three.
- **Files**: `adopt.rs` is the sibling module (225 lines); one `Pane` literal remains (`record()`), `begin`/`finish` shared (A warns 1 and 2).
- The brief's self-consistency: no defect found, so no `ADVISORY-HOLD`.

## Quality audit

- **Correctness and failure handling:** failed `foreground_pids`/`shown_session` fail closed (`harness-unreadable`); the pane is blocked, never guessed; the walk stops at the second match; a failed undo step is `profile-conflict` naming what remains; the failed-record deletion requires membership and equality with the planned record (another actor's record of the same name survives, tested).
- **Build guards:** no `unwrap`/`expect`/`panic`/`todo!` in any touched `src/` file (diff scan); no `#[allow]` added; `bash scripts/lint.sh` exit 0; largest touched files: `fake_tmux_test.rs` 826, `ports.rs` 785, `adapter lib.rs` 592, `assess.rs` 479, all under 900; no dead code (a `pid` mutant failed to compile for dead field, per T).
- **Protocol:** additive finding codes only; no golden file touched; `docs/pane-control.md` sections 3 and 10 and ADR-0021 updated in the same change; JSON keys of the verb unchanged (`extended` is `#[serde(skip)]`).
- **Tests:** real fake-host/Claude/Herdr harness in-process (`ImportRig`), a real fake `ps`/`tmux` script for the adapter, no fixed sleeps as synchronization (bound tests assert deadline invariants and spawn counts); RED-first evidence in `handoff-T-red.md`.
- **Documentation:** `CHANGELOG.md` has an `## [Unreleased]` entry linking #831 and #633; the guide and the CLI help describe the flag; `scripts/changelog-check.sh` ok.
- **Public-repository privacy:** grep of the diff (code, docs, CHANGELOG) for personal hostnames, tailnet names, IPs, account names, private domains and secrets: no hit. The only path-like fixture, `/home/demo/.keys/demo.key` in a test's `pane.env`, is a made-up value (the test asserts values never reach a finding). Findings name pane and variable names only (I7).
- **Commit and PR hygiene:** pipeline commits are `chore(#831): ... -- verdict: ...` Conventional Commits with a `Co-Authored-By` trailer (no session link, see Advisory 4). No PR exists yet; the AI disclosure per `CONTRIBUTING.md` is for O to add after the script opens it.

## Scope check

Delivered exactly the brief's scope. No unrelated refactor: the `assess` split into `begin`/`finish`, `tmux::decimal` made `pub(crate)`, `unread_finding` factored out and `Lent` in the test kit are all named in the brief's reuse map or A's warns, or required by the new method. `holler-body` and the hub are untouched, and no decision number or item 18 text was edited. The test kit gained `FakeHost::add_process` and `Lent`, the "testkit helper to run a Claude process in a fake host session with a pid both fakes share" the brief calls for. Nothing under-delivered.

## Verdict

**PASS.** All acceptance criteria and decisions are met and proved by tests that assert behaviour (T's mutation spot-checks confirm they fail without the change), quality gates hold, and nothing in the diff leaks private data.

## Advisory notes (non-blocking)

1. **AC1 ordering.** The brief says the ADR and guide amendment is the FIRST commit, before any test or code. In this run T-red's tests (`af8709a`) came first and the docs landed with F's code (`2e7f426`). The content satisfies AC1 and the docs tests pin it; the PR is squash-merged, so the order does not survive. No rework.
2. **Process deviation, stated:** `fake_tmux_test.rs` got one `match` arm (the brief said no edit) because `HostOp` gained a variant and the match is exhaustive. T recorded it in "Deviations"; the file is 826 lines, under 900, and `METHODS` stays six.
3. **Cosmetic:** the new doc line in `crates/holler-cli/src/pane/import.rs` (`strings); a pane that runs Claude Code needs no `port`. A pane is joined to ...`) runs past 100 columns, as the `ErrorBody` doc line in `adapter-host/src/lib.rs` (`refused`) does; rustfmt accepts both.
4. Pipeline commit trailers read `Co-Authored-By: Claude <noreply@anthropic.com>` with no session link, as for the repo's other pipeline commits; the squash commit and PR body should carry the link and the AI disclosure.
5. **Not proven by tests:** the undo's equality check relies on the real hub store keeping a record verbatim apart from `generation` (A warn 5; stated in the guide, `docs/pane-control.md:419`). Only the testkit store exercises it. Worth one hub-store test in a later story.
6. The real-tmux check of a typed-in `claude` is `#[ignore]` (opt-in, by design); F and T ran it once on tmux 3.7c. The fakes cannot prove the real process tree on other hosts (macOS `ttys004`, BusyBox `ps` without `-t` answers `unavailable`).
7. Pre-existing, outside this diff: `token_load_test.rs:60` fails clippy on toolchain 1.93; ~100 `holler-cli` files drift under the local `rustfmt`.
