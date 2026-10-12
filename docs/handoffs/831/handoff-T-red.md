# Handoff-T-red: Phase 4 - #831 adopt, adopting the fleet's existing Claude Code panes

**Date:** 2026-10-12
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-adopt`, head `23d7ff6`; `origin/main` is `ef881fc`)
**Brief / wireframe reviewed:** `docs/handoffs/831-adopt-brief.md` (amended twice); `docs/handoffs/831/handoff-A.md` (third pass, PASS, 0 blocks, 5 warns). No wireframe (no UI).

**VERDICT: RED is valid.** The contract is compile-level for one root cause and assertion-level everywhere else. The single
compile error is `HostPort::foreground_pids` not existing (`E0407` in `FakeHost`'s impl and in `TestHost`, `E0599` in the new
conformance case, `E0282` downstream of it); it is the new required port method of the brief, not a typo, a path or a missing
`[[test]]`. To prove that nothing else is wrong, T put the minimal contract in temporarily (the trait method, its four
wrapper impls, and a prototype of the adapter; the production edits were then reverted, see "RED confirmation"), and with it the
suite compiles with no warning and **fails on assertions about the missing behavior**: 46 cases in `pane_verbs`, 2 in the
docs test, 2 in `import_test`. Every other test of the workspace stays green with it.

## A precondition

Confirmed: A returned PASS on the plan (third pass, brief amended twice). T pins the corrections A asked for:

- **Finding 3:** port 0 and a relative cwd are `source-unreadable`, not `source-missing`; a port-less entry with a relative cwd is
  `source-unreadable` (flag off: `harness.rs`; flag on: `adopt_problems.rs`).
- **Finding 4:** the flag-on cases live in new files, not in `harness.rs`/`writes.rs`. T split them into four (not two), see
  "Deviations".
- **Finding 5:** the planned record holds no field a store fills (the undo test relies on the fake keeping a put verbatim apart
  from `generation`).
- Findings 1 and 2 (one `Pane` literal, one join tail) are F's and Phase 7's; no test can pin them, and none is written to
  tolerate a copy.

## Files

New (all tests or test support; none is production):

- `crates/holler-adapter-host/tests/foreground_pids_test.rs` (13 tests, 344 lines) and `foreground_pids_bound_test.rs` (6, 204).
- `crates/holler-cli/tests/pane_verbs/import/adopt.rs` (14, 523), `adopt_problems.rs` (13, 385), `adopt_fleet.rs` (7, 293),
  `adopt_undo.rs` (9, 359) and the rig `adopt_rig.rs` (330). `import.rs` declares the five modules; the target is the existing
  `pane_verbs` `[[test]]`, so `Cargo.toml` needs nothing.

Changed:

- Test kit (a non-test crate; its `src` is test support, which the brief lists under "Tests"): `holler-pane-testkit/src/host.rs`
  (`HostOp::ForegroundPids`, `FakeHost::foreground_pids`, `FakeHost::add_process`), `src/conformance/host.rs` (case 12),
  `src/harnesses.rs` (`Lent`, a borrowed fake for the router). Tests: `fake_host_test.rs`, `host_conformance_test.rs` (four
  mutants), `fake_harnesses_test.rs` (`Lent`; the pid pairing test).
- `holler-adapter-host/tests/common/mod.rs` (a fake `ps`: `Fake::with_ps`, `ps()`, `ps_calls()`, `ps_env()`; it names no
  `TmuxHost` method, so the six existing adapter test binaries still compile), `fake_tmux_test.rs` (one line, see "Deviations"),
  `real_tmux_test.rs` (one `#[ignore]` case).
- `holler-cli/tests/pane_verbs/import/harness.rs` (replaces `adopt_claude_code_is_usage_naming_831_and_writes_nothing`; four flag-off
  cases), `import/rig.rs` (`write_launcher_with`), `docs_pane_control_test.rs` (two tests), `verb_harness/linked_host.rs` and
  `pane_integration/forwarding.rs` (forwarding), `holler-pane/tests/{import_test,ports_test}.rs`.

## Tests authored

Tier = cheapest sufficient: a unit-level port/fake test where the rule is one function; an adapter test against a fake `tmux` and
`ps` script for the host adapter; the in-process verb over the fakes (`ImportRig`, as every import test) for the story; a docs test
for the amendment. No test starts a real `claude`, tmux, Herdr or the live fleet; the one real-tmux case is `#[ignore]`.

### Host adapter (`foreground_pids`, D2 + D4, the mechanism paragraph)

`foreground_pids_test.rs` (fake `tmux` lists panes, fake `ps` answers by its tty argument):

- `a_program_typed_into_the_shell_is_found_and_the_shell_is_not`: the typed-in case. `pane_pid` would be the shell; the foreground
  group is another pid. Also pins the one `list-panes -s -t =demo-c1r1: -F` call with `#{pane_dead}` and `#{pane_tty}` and no
  `start_command`, the exact `ps` argv `-t pts/3 -o pid= -o pgid= -o tpgid=` (numbers only, I7) and `LC_ALL=C`.
- `a_member_of_the_foreground_group_that_is_not_its_leader_is_found`; `two_panes_answer_together_ascending_and_ps_runs_once_per_pane_in_tmux_order`;
  `a_dead_pane_is_dropped_and_its_tty_is_not_asked`; `a_tty_with_no_foreground_group_matches_nothing` (`tpgid` -1);
  `an_empty_ps_answer_is_an_empty_pane_not_a_failure`; `a_session_with_no_live_pane_answers_nothing_and_runs_no_ps`.
- `a_linux_and_a_macos_tty_reach_ps_as_the_names_ps_takes` (`/dev/pts/3` -> `pts/3`, `/dev/ttys004` -> `ttys004`).
- `a_malformed_listing_is_unavailable_and_runs_no_ps`: twelve bad lines (a dead flag that is not 0 or 1, no `/dev/`, empty, `..`
  segment, leading `-`, a character outside `[A-Za-z0-9._/-]`, extra field, one field, empty line), each `unavailable` for the whole
  answer with no `ps` run, even after a good first pane.
- `another_ps_failure_is_unavailable_with_its_first_line_and_fails_the_whole_answer`;
  `a_ps_line_that_is_not_three_decimal_fields_is_unavailable`; `a_missing_session_is_pane_not_found_and_runs_no_ps`;
  `another_tmux_failure_is_unavailable_with_its_first_line`.

`foreground_pids_bound_test.rs` (the new file the brief asks for, with its own list of the seven operations):
`every_method_of_the_seven_is_bounded_and_reaps_its_child`; `a_missing_tmux_binary_is_unavailable_from_every_method_of_the_seven`;
`a_hung_ps_is_a_timeout_and_is_reaped`; `a_deadline_reached_on_the_second_pane_is_a_timeout_and_no_partial_list` (the fake `ps` that
outlives the deadline on the second pane, as the brief says); `the_deadline_is_shared_by_every_ps_of_the_call` (four panes at 0.2 s
under a 0.5 s bound: a timeout, and fewer than four `ps` spawned); `a_missing_ps_binary_is_unavailable_and_names_no_argument`.

`real_tmux_test.rs::foreground_pids_includes_a_program_typed_into_the_shell` (`#[ignore]`, D2): a script named `claude` typed into the
pane's shell with `send-keys`; its pid must be in the answer.

### Test kit and the port (`FakeHost`, conformance, wrappers)

- `host_conformance_test.rs`: the documented case list becomes twelve; four mutants (`foreground_pids` names nothing; a missing session
  answers `[]`; descending; repeated) each fail case 12 by name. `conformance/host.rs` case 12
  (`foreground-pids-hold-what-run-started-and-of-missing-is-pane-not-found`): `pane-not-found` for a missing session, two `run`s of
  `sleep 30` both in the answer within a bound, ascending without a repeat; it asserts no more (a real session lists a shell).
- `fake_host_test.rs`: `foreground_pids` (ascending, per session, missing), follows the processes out, fault on it only, and
  `add_process` (the pid pairing: in `ps`, `foreground_commands` and `foreground_pids`; not a call through the port).
  `fake_harnesses_test.rs`: `Lent` routes by endpoint; `a_hand_run_claude_process_has_one_pid_in_the_host_and_the_harness` is the
  brief's FakeHost/FakeClaude pairing test.
- `forwarding.rs`: the wiring's `BindingHost` and `WiredHost` forward `foreground_pids` as they forward `attached_clients` (answer
  and error as they are, one inner call per ask with the name given, and the misconfigured `WiredHost`'s error).
  `bind.rs`'s host conformance over both wrappers runs case 12 with no edit. `LinkedHost`, `LostChain`, `TestHost`, `Counting` and
  the conformance `Mutant` forward or implement the method.

### The verb (`holler pane import --adopt-claude-code`)

`adopt.rs`: AC4 and D3, AC7, AC8, AC11, AC12.
- `a_typed_in_claude_pane_is_adopted_with_the_process_and_session_agents_json_lists` (the record, whole, against `expected_claude`; the
  session is the process's, not `last-session`'s; spec `claude` / `none`; sources unchanged);
  `an_adopted_pane_has_one_info_finding_naming_relaunch` (AC8);
  `the_chain_of_an_adopted_pane_reads_tmux_then_one_session_per_pid` (`[Ps, ForegroundCommands, ForegroundPids]`, then `shown_session`
  per pid in order; no OpenCode server call);
  `a_claude_pane_with_no_shell_beside_it_is_adopted_too`; `an_adopted_pane_named_by_orchestrator_takes_that_role`.
- `a_dry_run_shows_the_adoption_and_a_real_run_does_what_it_showed`; `the_text_summary_counts_the_adopted_panes_only_when_there_are_some`;
  `the_adoption_reads_and_acts_on_nothing_live` (dry and real: Herdr snapshot/version, host `ps`/`foreground_commands`/`foreground_pids`,
  Claude `shown_session`; stores written only by the real run).
- D3: `a_default_agent_in_pane_env_is_ignored_and_leaves_no_difference` (`opencode_agent = None`, `profile show` `matches`, `profile apply
  --dry-run` plans `adopt` with no diff); `a_model_that_is_not_anthropics_is_a_warning_and_the_pane_is_still_adopted` (the variable
  `OC_MODEL`, never a value); `an_anthropic_model_in_any_case_gets_no_warning`.
- `the_opencode_only_files_checks_do_not_apply_to_an_adopted_pane` (a shell-string launcher `command` and a disagreeing
  `last-session`: no `command-not-argv`, no `sources-disagree`, no OpenCode call);
  `one_run_adopts_a_claude_pane_and_records_an_opencode_pane_into_one_fleet` (AC12, with `profile show` and `profile apply --dry-run`);
  `in_a_mixed_run_only_the_opencode_pane_asks_a_server`.

`adopt_problems.rs` (AC9, the three states of `Target.adopt`, the launcher): `a_process_agents_json_does_not_list_is_unmatched_and_blocks`;
`two_matching_processes_block_and_the_walk_stops_at_the_second` (three processes, `shown_session` called twice); 
`a_matching_process_with_no_session_is_unmatched_and_blocks`; `a_failed_foreground_pids_is_harness_unreadable_and_asks_no_session`;
`a_failed_shown_session_is_harness_unreadable`; `a_login_variable_in_pane_env_blocks_the_pane_without_its_value` (every name
`HarnessKind::Claude.login_variables()` gives, not retyped); `an_unlabelled_claude_pane_reads_no_pid_and_no_session` and
`an_ambiguously_labelled_...`; `a_launcher_entry_with_a_cwd_and_no_port_is_enough_to_adopt`;
`a_launcher_entry_that_cannot_give_the_directory_blocks_the_adopted_pane` (six entries with A's codes);
`a_missing_pane_env_blocks_the_adopted_pane`; `a_port_less_entry_for_an_opencode_pane_is_source_missing_with_the_switch_too`;
`a_pane_whose_sessions_toml_names_claude_is_still_skipped_with_the_switch`. Every blocked case also runs the real run: exit 3,
`import-blocked`, no record at all (the other pane's neither), no profile.

`adopt_fleet.rs` (AC5, AC6, AC11, D1): `an_existing_fleet_is_extended_by_one_cas_put` (one profile `CasPut`, actor `holler pane import`,
generation + 1); `with_the_switch_an_existing_fleet_takes_opencode_panes_too` (D1 = A);
`an_extension_with_no_new_spec_writes_the_record_and_leaves_fleet_alone`; `an_existing_spec_of_another_kind_is_kept_and_warned_about`
(`spec-kind-differs`); `a_dry_run_against_an_existing_fleet_says_to_extend_and_equals_the_real_plan`;
`a_second_run_after_an_adoption_is_a_no_op` (with and without the switch; no host, Claude or profile read);
`a_pane_added_since_is_recorded_into_fleet_beside_the_adopted_ones` (without the switch `profile-exists`).

`adopt_undo.rs` (AC10, both shapes): extension: first record fails; second record fails; a record that landed and timed out; the
`fleet` write fails (the pane store's writes are `CasPut CasPut Delete Delete`); a concurrent change of `fleet` through
`concurrent_put` after the last record (`fleet` keeps the other actor's version); the failed record is another actor's, a member of
`fleet`, injected with `concurrent_put` before the write (it survives; the run's error is `generation-conflict`); an undo step that
fails (`profile-conflict`, no `profile delete`, no `pane close`, names the kept record, `holler pane list`,
`holler profile show 'fleet'`, says a re-run skips it). Create: a failed record write with an adopted pane; a failed profile delete
still names `holler profile delete 'fleet'`.

`harness.rs` (AC2, AC3, flag off): `adopt_claude_code_is_not_refused_and_a_dry_run_writes_nothing` (replaces the `usage` test);
`the_claude_code_finding_says_how_to_adopt_the_pane`; `without_the_switch_a_claude_pane_makes_no_foreground_pids_call`;
`a_flag_off_claude_pane_whose_launcher_entry_has_a_cwd_and_no_port_stays_skipped` (the brief's named test: green before and after);
`a_port_less_launcher_entry_for_an_opencode_pane_is_source_missing_without_the_switch` (green before and after);
`a_port_less_entry_with_a_relative_cwd_is_source_unreadable_without_the_switch` (RED: today `source-missing`).

### Docs and vocabulary

`import_test.rs`: `NEW_CODES` has seventeen, with the five codes' levels, and `ALL` has 23. `docs_pane_control_test.rs`:
`the_adoption_is_documented_as_built` (section 3's table rows with their levels; the stale sentences gone from the guide and the ADR;
`foreground_pids` in section 10 and the ADR; `ps -t` in the ADR) and `the_import_help_no_longer_says_the_flag_is_refused`.
The existing docs tests already derive from `ImportFinding::ALL` and the clap tree, so they pick the new codes and flag text up.

## RED confirmation

**1. The tree as it is (no production change).** `cargo build --workspace --all-targets --keep-going`:

```
error[E0407]: method `foreground_pids` is not a member of trait `HostPort`
   --> crates/holler-pane-testkit/src/host.rs:296:5
error[E0407]: method `foreground_pids` is not a member of trait `HostPort`
   --> crates/holler-pane/tests/ports_test.rs:224:5
error[E0599]: no method named `foreground_pids` found for reference `&dyn HostPort` in the current scope   (x2, the conformance case)
error[E0282]: type annotations needed   (downstream)
error: could not compile `holler-pane-testkit` (lib) due to 4 previous errors
```

One cause. F's first step is the endpoint-style one: the trait method, then the ten `impl HostPort for` sites of the brief, then
`TmuxHost::with_ps_binary` and `foreground_pids`.

**2. The contract in, to prove the failures are the right ones.** T applied a temporary change to `holler-pane/src/ports.rs`
(the required method), `wiring.rs` and `wiring/bind.rs` (forwarding) and a prototype of `TmuxHost::foreground_pids`/`with_ps_binary`
in `holler-adapter-host/src/lib.rs`, ran the workspace, and reverted all four files (`git status` shows them clean; the diff is
kept in the scratchpad as `t_red_prototype_stub.patch`, not for F to use). With it:

- `cargo build --workspace --all-targets` and `cargo clippy` over the touched crates (`-D warnings`): no error, no warning, including
  every `HostPort` implementer, so the list of sites in the brief is complete.
- **Passing, and meant to:** the prototype passes all 19 adapter tests against the fake `tmux`/`ps` (stable over 15 repeated runs),
  the real-tmux conformance suite with case 12 and the typed-in-shell case (tmux 3.7c, Linux: **D2's mechanism works**, the answer
  holds the typed script's pid), the test kit's 23 `FakeHost` tests, `host_conformance_test` with its four new mutants failing
  case 12 by name, the forwarding tests and every other existing test of the workspace.
  `fake_tmux_test.rs`' six-row tests are untouched and green.
- **Failing on assertions (46 + 2 + 2):**
  - `import_test`: `import_finding_codes_are_the_pinned_set` (left: 18 codes; right: the 18 and the five new ones) and
    `the_adoption_codes_are_listed_at_their_levels` (`ImportFinding::ALL lacks claude-code-adopted`).
  - `docs_pane_control_test`: `the_adoption_is_documented_as_built`, `the_import_help_no_longer_says_the_flag_is_refused`.
  - `pane_verbs`, by file: `adopt.rs` 14 of 14, `adopt_problems.rs` 13 of 13, `adopt_fleet.rs` 7 of 7, `adopt_undo.rs` 9 of 9, and in
    `harness.rs` the three RED cases (`adopt_claude_code_is_not_refused_and_a_dry_run_writes_nothing`,
    `the_claude_code_finding_says_how_to_adopt_the_pane`, `a_port_less_entry_with_a_relative_cwd_is_source_unreadable_without_the_switch`).
    The three green-before cases of `harness.rs` pass, as the brief says they should. The reason of nearly all is the first one:
    `--adopt-claude-code` answers `usage` (exit 2) and `data_of` fails with "the run succeeds: Outcome { code: 2 ... }", which is
    RED 1 of the brief's test plan. To see past it, T also lifted that one refusal temporarily: each case then fails on the behavior
    it pins (no record for the Claude pane, `skipped` where `record` is expected, `profile-exists` where an extension is expected,
    no `foreground_pids` or `shown_session` call, no `claude-*` finding, "to extend" absent), none on a setup error. One test
    (`the_adoption_reads_and_acts_on_nothing_live`) passed vacuously then; it now also asserts the pane is adopted.
- Unrelated and not caused by this work: `cargo clippy --workspace --all-targets -- -D warnings` fails on
  `crates/holler-cli/tests/token_load_test.rs` (`redundant_iter_cloned`, local toolchain 1.93) in a file this run does not touch, and
  `holler-body --test rebind_test` failed once under load and passed on three re-runs.

## Contract F implements (the names and words the tests pin)

- `HostPort::foreground_pids(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>`, required, ascending, no repeat; `HostOp::ForegroundPids`
  is `host.foreground_pids` (the `Timeout` op). `TmuxHost::with_ps_binary(PathBuf)`; the `ps` argv is exactly
  `-t <tty without /dev/> -o pid= -o pgid= -o tpgid=` with `LC_ALL=C`; the listing format has `#{pane_dead}` and `#{pane_tty}`.
- Finding codes (levels): `claude-code-adopted` (info, names `holler pane relaunch NAME`), `claude-process-unmatched` (problem),
  `claude-login-env` (problem, the variable's name), `claude-model-not-anthropic` (warning, names `OC_MODEL`, no value),
  `spec-kind-differs` (warning, names the two kinds). A failed read is `harness-unreadable` and its message names the call:
  `host.foreground_pids` or `harness.shown_session`, as `host.ps` is named today.
- The `harness-claude-code` message says "pass --adopt-claude-code to adopt it" and "otherwise the import leaves the pane alone",
  and no longer mentions a story or "until". The flag's clap help says it adopts Claude Code panes and extends `fleet`; neither it
  nor the verb's long help says "Refused (usage)" or "adoption step lands".
- A dry run against an existing `fleet` prints `profile fleet (fleet) to extend: N specs added, M total`, then `spec NAME` lines for
  the added specs; JSON keys unchanged, `profile.generation` the one read. The extension's error text contains "nothing was kept" and
  "was not changed"; its `profile-conflict` text contains `holler pane list` and `holler profile show 'fleet'`, one of
  `already-imported`/`already imported`, and never `profile delete` or `pane close`.
- The text summary line contains the word `adopted` when a pane was adopted (and not when none was).

## Deviations from the brief (stated, each with its reason)

1. **`fake_tmux_test.rs` gets one line.** The brief says it gets no edit. `HostOp` gains a variant, and that file's `call` function
   matches `HostOp` exhaustively, so it does not compile without one arm. The arm is added (822 -> 826 lines); `METHODS` stays
   `[HostOp; 6]`, and the seven-operation bound cases are in the new file, as the brief wants.
2. **Four flag-on test files, not two** (`adopt.rs`, `adopt_problems.rs`, `adopt_fleet.rs`, `adopt_undo.rs`, plus `adopt_rig.rs`).
   A asked for `adopt.rs` and `adopt_undo.rs`; 43 cases in two files would be over `lint.sh`'s 600-line warning. Largest is 523.
3. **The third `compile_fail` doctest in `ports.rs` is not authored here.** It lives in the production file, in the trait's own
   docs; adding it is part of F's edit of the trait (the two existing doctests need the new method too). T checks in Phase 6 that
   the new fence fails for the right reason, not for a typo.
4. **Pinned beyond the brief's words** (each is the nearest reading of the brief, listed so S and F can object): the call names in
   the `harness-unreadable` messages; the exact `harness-claude-code` phrases above; "nothing was kept" / "was not changed" in the
   extension error; `adopted` in the summary line (AC8 says only "counts them"); a malformed-tty list; the order `Ps,
   ForegroundCommands, ForegroundPids` for a single pane.
5. **Weak by design:** AC8's summary wording, and the shell's exclusion in the real-tmux case (job control makes it shell-dependent; the
   test asserts the typed pid is in the answer, the D2 question, and reports the shell pid only in the failure text).

## Ready for F

Confirmed: the RED is valid and F may implement against these tests. Start with the port method and its sites (the build is the
check), then the adapter, then `import`. Gates for Phase 6 that are commands, not tests: `grep -rn "CLAUDE_CODE_STORY"
crates/holler-cli/src` prints nothing; `bash scripts/lint.sh`; `bash scripts/changelog-check.sh`;
`cargo test -p holler-cli --test docs_cli_test`; `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored`; and the first
commit's ADR-0021 and `docs/pane-control.md` amendment, which has not been made yet (`grep -c foreground_pids` is 0 in both) and which
the two docs tests above read.
