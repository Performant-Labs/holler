# Handoff-S: Phase 8 - #647 part 2: `unattached` and `agent-cannot-dispatch` (spec audit)

**Date:** 2026-10-12 (11:52 MDT)
**Branch:** `issue-647-implementation` at `e393e87` (F's code `8b6030e`; T's RED `a3a9e2b`, GREEN `4bd4e75` and `b5fc4bb`; contract `45bff85`; pure move `3a09f7d`)
**Issue:** #647 (GitHub issue, amendments of 2026-10-09, and the operator's comment of 2026-10-11 allowing the contract change); brief `docs/handoffs/647-part2-brief.md`
**Handoffs reviewed:** `docs/handoffs/647/handoff-A.md` (plan review), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` (with its re-entry), `handoff-A-dup.md`, `decisions.md`, `evidence.md`; the outside diff gate `docs/handoffs/647-diff-result-r1.md` (PASS, per A-dup)
**Diff read:** `git diff origin/main...HEAD` (`origin/main` = `6eff9cc`), all production files in full and every new or edited test file

## A precondition

Met. The plan review (`handoff-A.md`) is **PASS** with 0 blocks and 8 warns. The anti-duplication gate (`handoff-A-dup.md`) is **PASS** with 0 blocks and 3 warns.

## T precondition

Met. `handoff-T-red.md` confirms a valid RED: the real tree fails to build only on the missing API, the two `E0046` doc tests fail with "Test compiled successfully", and every behavioural test fails on its assertion against a throwaway shim that was restored byte for byte. `handoff-T-green.md` confirms GREEN with **"Blocking issues: None"**. Its re-entry at `4bd4e75` reports 2688 passed, 0 failed and 39 ignored, with clippy, `lint.sh`, `changelog-check`, `docs_cli_test`, `wire_selftest` and `machete` clean. The opt-in runs pass: real tmux 9 of 9 and real OpenCode 1.19.0 14 of 14. The mutations M1 to M8, P1, P2 and both W7 mutations were killed.

## Acceptance criteria

| # | Criterion (brief) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | One `compile_fail,E0046` per new method; `PidsOnly` keeps pinning `foreground_commands` alone | `ports.rs`: `HostPort` doc test `NoClientCount`, `HarnessPort` doc test `NoAgentView`; `PidsOnly` gains `attached_clients` and its lead sentence now names only `foreground_commands`. RED at T, GREEN at F (11 doc tests) | PASS |
| 2 | `AgentView`: empty `Default`, no struct literal outside the crate, builders | `holler-pane/tests/agent_view_test.rs` (5 tests); two `compile_fail,E0639` doc tests on `HarnessPort`, which F checked give exactly `E0639` in an outside crate | PASS |
| 3 | `CODES` ends with `unattached`, `agent-cannot-dispatch` | `findings_test::finding_kind_codes_are_stable` (18 codes) | PASS |
| 4 | `remedy_for` table; `--agent=-x` parses through the real clap tree; `remedy` unchanged | `findings_test`: `agent_cannot_dispatch_names_the_relaunch_with_the_one_key`, `a_key_with_a_leading_dash_is_joined_to_the_flag`, `agent_cannot_dispatch_without_a_key_or_a_pane_has_no_remedy`, `unattached_never_has_a_remedy`, `remedy_is_the_table_with_no_agent_and_the_agent_touches_only_its_kind`; clap: `pane_verbs::doctor::agent::a_dispatcher_whose_key_starts_with_a_dash_still_parses_as_the_agent` (`Cli::try_parse_from`) | PASS |
| 5 | `FakeHost` counts, `pane-not-found`, faults | `fake_host_test`: `attached_clients_counts_what_attach_client_added`, `attached_clients_of_a_missing_session_is_pane_not_found`, `clients_survive_stop_owned_and_a_second_ensure_session`, `a_fault_on_attached_clients_fails_that_call_only` | PASS |
| 6 | Host conformance case, its mutants, real tmux | case 11 `attached-clients-of-fresh-session-is-zero-and-of-missing-is-pane-not-found`; mutants `AttachedAlwaysOne` and `AttachedOfMissingIsZero`, each with a test that the suite fails on case 11; `real_tmux_test` 9 of 9 (opt-in, T and F) | PASS |
| 7 | `FakeHarness` view, check order and exact errors; conformance case and mutant; Claude empty view; router | `fake_harness_agent_test.rs` (6); harness case 17 with mutants `AgentViewAlwaysEmpty` and `AgentViewInventsATurn`; Claude case 13 with mutant `GuessesATurn`; `fake_claude_test::agent_view_is_empty_for_a_process_and_refused_for_a_port`, `a_fault_on_agent_view_fails_that_call_and_is_logged`; `fake_harnesses_test::agent_view_reaches_the_fake_its_endpoint_names`; Claude `hermetic_test::agent_view_is_empty_for_a_process_and_refused_for_a_port`. The empty view answers only a `Process` endpoint, and a `Port` is `server-unsupported`: this is A's W-4, sanctioned and written into the ADR | PASS |
| 8 | Host adapter over fake tmux: 0/1/2, strict parse quoting nothing, `pane-not-found`, exact argv | `holler-adapter-host/tests/attached_test.rs` (6 tests, including T's `an_empty_listing_or_counts_that_disagree_are_unavailable`); `fake_tmux_test` `METHODS` | PASS |
| 9 | OpenCode `agent_view` over fixtures cut from 1.19.0; rule table; errors; the real test proves `info.agent` | `holler-adapter-opencode/tests/agent_view_test.rs` (13 tests, 18-row `can_dispatch` table); opt-in `ac647_agent_view_reads_a_real_servers_turns_and_agents` (passed on 1.19.0). A message with no agent is a `TurnAgent { None, None }` turn, not skipped: T followed the ADR's wording (see advisory 2) | PASS |
| 10 | `unattached`: one finding, null remedy, `not-fixable`, D11's two substrings; attached raises nothing; JSON envelope; equal exit codes | `doctor/attach.rs`: `a_session_with_no_client_raises_one_unattached_and_attached_raises_nothing`, `the_verb_reports_unattached_in_both_formats` | PASS |
| 11 | `unattached` edges | `doctor/attach.rs`: `a_missing_session_raises_tmux_session_missing_and_no_count_is_asked`, `a_failed_count_is_observe_failed_and_not_unattached`, `a_parked_pane_and_the_orchestrators_pane_raise_it_too` | PASS |
| 12 | `agent-cannot-dispatch` positive, and with `--fix` | `doctor/agent.rs`: `every_recent_turn_unable_to_dispatch_raises_one_finding_naming_the_relaunch` (4 turns seeded, the oldest able to dispatch, so `RECENT_TURNS = 3` is pinned), `fix_changes_nothing_for_it` (no `ServeCommand`, no `SelectSession`, record and agent unchanged) | PASS |
| 13 | Negatives, each its own test | `doctor/agent.rs`: the two `a_record_that_names_*` tests, `no_dispatcher_raises_nothing`, `one_recent_turn_that_could_dispatch_raises_nothing`, `one_recent_turn_the_server_says_nothing_about_raises_nothing`, `an_empty_view_raises_nothing`, `several_dispatchers_give_no_remedy_and_a_message_that_lists_them`, and the five `*_is_never_asked` tests (down, `Unknown`, `Healthy { sessions: None }`, session missing, no session), each asserting no `AgentView` call and the case's own finding | PASS |
| 14 | Once per pass; the stored records are equal | `doctor/agent.rs::a_second_pass_reports_the_same_two_findings_once_each_and_writes_nothing`; `doctor/attach.rs::unattached_is_reported_once_per_pass_and_writes_nothing`, `fix_does_nothing_for_unattached` | PASS |
| 15 | Allowlist extended by exactly the two ops | `doctor/read_only.rs::doctor_without_fix_makes_only_read_calls` and `doctor.rs`'s `--fix` test, both through `rig::HOST_READS` (`Ps`, `AttachedClients`) and `rig::HARNESS_READS` (the three reads plus `AgentView`) | PASS |
| 16 | Every live-world builder attaches a client; proof that the attach, not a disabled rule, keeps the worlds clean | `doctor/rig.rs` `make_live`, `pane_claude/rig.rs` `seed_claude` and `seed_opencode`, `pane_claude/mixed.rs` `applied()`; `doctor/attach.rs::a_world_built_without_a_client_raises_unattached_for_every_pane`; the whole existing suite passes (2688/0) | PASS |
| 17 | Section 5's two cells as exact text; the `unattached` limit | `docs_pane_control_test::section_5_rows_of_part_2_name_their_exact_commands`; the guide rows end `\| none \|` and `` \| `holler pane relaunch PANE --agent=KEY` \| `` | PASS |
| 18 | `assert_end_state` takes exactly one `unattached` per scratch pane; `break_both` leaves it out | `scratch/verbs.rs::assert_end_state` (a sorted list of `(pane, kind)` pairs equal to `panes` × `unattached`), `scenario.rs::break_both`. These compile in the default run. The opt-in scratch scenarios were not run (they need real Herdr); the claim they rest on is proved by the opt-in `ac647_a_scratch_server_without_project_agents_has_no_dispatcher` (passed) | PASS |
| 19 | No CLI surface change; lint; clippy; no `unsafe`; privacy | `git diff origin/main...HEAD -- crates/holler-cli/tests/fixtures/cli-surface.txt docs/adr/ADR-0003*` is empty; `lint.sh` exit 0 (T), and the largest touched `.rs` file is `pane_verbs/launch/rig.rs` at 865 lines (`wc -l`); clippy clean (T); my grep of added lines finds no `unsafe`, `unwrap`, `expect` or `panic` in `src/`; privacy grep below | PASS |
| 20 | Contract commit is documentation only and comes before every test or code commit | `45bff85` touches `docs/adr/ADR-0021.md` and only `//!` lines of `ports.rs`; every non-merge commit before it touches no `crates/` file (checked per commit) | PASS (advisory 4: the script's code-commit messages do not cite the section) |
| 21 | Module docs name the new reads, and the bound reads nine in D13's order | `reconcile.rs` lines 10-15 and 32; `observe.rs` lines 6-12 ("at most nine port calls", `ps`, `attached_clients`, `health`, `list_sessions`, `shown_session`, `agent_view`, `select_session`, `shown_session`, `cas_put`); `process.rs` lines 4-9 | PASS |
| W1-W8 | Wrapper forwarding | W1: `pane_integration/forwarding.rs` (3 recording tests). W2: `bind.rs`'s host conformance runs over `BindingHost` and `WiredHost`. W3: `a_misconfigured_wired_host_answers_its_error_from_attached_clients`, `wired.rs`'s two new sockets, `pane_verbs/main.rs::unwired_ports_answer_not_implemented`. W4/W5: `bind.rs` `Recording::agent_view` (`distinctive_view`, arguments `(Port(48100), "ses_1", 3)`) and `call_every_method`. W6: one-line forwards, which the compiler checks. W7: F's and T's mutation runs, all killed. W8: `bind.rs` module doc | PASS |

## Spec compliance

Every decision of the brief is implemented as stated, or as A's plan review amended it. Each amendment is recorded in a handoff and in `decisions.md`:

- **D3/D4 (required methods and their signatures):** both methods are required, with the exact signatures. `HarnessRouter::agent_view` goes through `by`.
- **D5 (`AgentView`, `TurnAgent`):** both are `#[non_exhaustive]`. `with_dispatchers` sorts and drops duplicates, the one place that rule lives. `TurnAgent` is also `#[non_exhaustive]`, which T allowed.
- **D6 (the rule):** rules 1 to 5 are in `reconcile/observe/agent.rs`. `RECENT_TURNS = 3` lives there, not in `observe.rs`, because A's W-8 moved the rule to the child module. The view is asked only once rules 1 to 3 hold.
- **D7/D15 (OpenCode):**
  - `can_dispatch` scans from the newest rule.
  - F sharpened the ADR wording to also count a later `*`-permission rule, and an `ask`, as reopening dispatch. The change can only remove findings, and T pinned it with 4 table rows.
  - A's W-1 replaced D15's "`usable` becomes `pub(crate)`" with a split: `server::as_default` and `server::agent_list`. The messages are byte-identical, and the opt-in `verify_serving_refuses_...` test passes.
  - `prompt::messages` is the one message reader and quotes no body. Editing `prompt.rs`, outside the Blast radius, was required by A's W-1, and F recorded it.
- **D8 (remedy):** one table. `remedy` delegates to `remedy_for`. `with_agent` is applied after `with_fix` (A's W-2), and `with_fix` is never called on this finding (`observe.rs:333` and `process.rs:171` are the mismatch paths only). `Finding` gains no field. `doctor.rs` prints the stored remedy without recomputing it.
- **D9-D11 (`unattached`):**
  - `check_tmux` asks for the count only after `ps` answered, so a chain raises at most one of `tmux-session-missing`, `unattached` and `observe-failed`.
  - The remedy is `None`, and the message carries D11's two substrings.
  - It runs before the shape branch (`observe.rs:118` against `:130`), so it covers a Claude Code pane too (see advisory 1).
- **D13 (chain):** the agent check sits between `check_session_of_record` and `check_screen`, borrowing `&server`. No existing signature changes.
- **D14 (test kit):** the four scenario methods bypass the faults and the call log. `FakeHarness::agent_view` follows `abort`'s check order.
- **D16/D17:** `--fix` does nothing new. The kinds are appended after `profile-drift`, in the brief's order.
- **ADR amendment:** the six edits of the brief, plus the three clauses the implementation required (W-4's process/port distinction, W-7's bound, and the sharpened `task` wording), all in section 12.

**Brief defect noted, not blocking:** acceptance 9 says "a message without an agent is skipped", while the brief's own ADR text, which landed in `45bff85`, says a turn's `agent` is "`None` when it recorded none". T followed the ADR, which is the conservative reading because a `None` turn stops the rule, and journalled it. The implementation is faithful to the standing spec, so this does not warrant an ADVISORY-HOLD (advisory 2).

**One silent deviation from a repository convention:** see Quality audit, item Q1, and the Verdict.

## Quality audit

- **Q1 (REWORK): a profile name reaches a finding message raw.** `crates/holler-pane/src/reconcile/observe/agent.rs:129-133` builds the W-3 clause with `format!("; the pane is in the profile {profile}, so relaunch it with --profile {profile} too, ...")`. That is `pane.profile` (a `ProfileName`) through `Display`, unquoted, twice, the second time as the argument of a suggested command.
  - **What breaks.** `ProfileName` is a display name: "spaces allowed, e.g. `Some Profile`", refusing only control characters (`profile.rs:30-35`). `relaunch --profile NAME` parses the display name (`list.rs:220-221`). So a member of `Some Profile` is told to "relaunch it with --profile Some Profile too", which a shell splits into two words.
  - **The rule it breaks.** The repository's rule, from #662 B1, is written down at `crates/holler-cli/src/profile/delete.rs:203-205`: "A profile name in a command that a message suggests goes through it, so the command pastes into a shell as it is, even for `Some Profile` or a name with a quote or `$(...)`." `findings.rs`'s module doc also says untrusted text reaches a message only through `quoted` or `embedded`.
  - **Where the rest of the code stands.** Every other message in both crates that names a profile quotes it:
    - `reconcile/drift.rs:80`, which is the sibling finding `profile-drift` in the same module tree;
    - `tx_apply.rs:247,420` and `tx_apply/run.rs:125,160,207`;
    - `pane/close.rs:78,82,117` and `pane/launch.rs:123,417,425`;
    - `pane/park.rs:359`, `pane/profile_scope.rs:54` and `pane/import/writes.rs:180`.
  - **Why the tests miss it.** The test `a_profile_members_message_names_its_profile` seeds the profile `blue`, which reads the same raw or quoted, so it passes either way.
- **Correctness and failure handling:** every new read fails closed.
  - A failed count or view is `observe-failed` naming the call, and nothing is concluded.
  - A reply of the wrong shape is `unavailable`: a message with no `info.role`, an agent with no string `name` or `mode`, an empty tmux listing, or tmux counts that disagree.
  - A rule list of a shape the adapter does not know is `None`, which stops the rule.
  - Both rules write nothing, and the read-only allowlist proves it.
- **Build guards:**
  - No `unwrap`, `expect`, `panic` or `unsafe` in added `src/` lines.
  - Every added `#[allow]` carries `// #647` or `// #642`.
  - Every touched `.rs` file is under 900 lines; the largest are `launch/rig.rs` at 865, `fake_tmux_test.rs` at 822 and `adapter-opencode/src/lib.rs` at 779.
  - No dead code: clippy `-D warnings` is clean (T).
- **Protocol:** none. Hub, proto, body and golden files are untouched, and there is no new error code. No JSON field changes: a finding gains two `kind` codes and nothing else.
- **Tests:**
  - The cross-process behaviour (tmux, OpenCode) is covered hermetically by the fake tmux and the stub, plus opt-in real runs.
  - There is no fixed sleep used for synchronization: the one `run_sleep` match starts a fake process.
  - The RED-first evidence is in `handoff-T-red.md`.
- **Documentation:**
  - `CHANGELOG.md` has a bullet under `## [Unreleased]` / `### Enhancements` linking #633 and #647.
  - `docs/pane-control.md` section 5 has both rows, both paragraphs, the "eight name none" count and the Claude Code paragraph, and section 12 gains a deferred bullet.
  - ADR-0021 has sections 2, 9 and 12, "Deferred" and item 34.
- **Public-repository privacy:** clean.
  - My grep of every added line (host, user and path names, tailnet patterns, private IP ranges, the operator's own attach tool's name, keys) finds only mutation ids ("M1"), test names that contain "secret", and the name of the key variable in the brief, never a value.
  - The fixtures use `/srv/demo` and `srv/demo-scratch`, and they carry no agent `prompt` and no `external_directory` rule.
- **Commit and PR hygiene:**
  - Subjects are Conventional (`docs(adr)`, `test(pane)`, and the script's `chore(#647): ...`).
  - Every non-merge commit carries `Co-Authored-By`, with no session link. That matches `main`'s practice: no commit on `main` carries one.
  - The PR does not exist yet, so its AI disclosure is a step after the PR opens (advisory 4).

## Scope check

F delivered the brief's scope: two methods, two types, two kinds, the rule module, the test-kit support, three adapters, the wiring and the docs. It added nothing unrelated.

- **Additions beyond the literal Blast radius, each required by a gate and recorded:**
  - the `prompt.rs` split (A's W-1);
  - `ports/agent_view.rs` in place of `src/agent_view.rs` (W-5);
  - `reconcile/observe/agent.rs` (W-8);
  - the three ADR clauses (W-4, W-7, Deviation 1);
  - the profile clause in the message and the guide row (W-3).
- **T's own moves:** the pure move `switch.rs` to `switch/doubles.rs`, which T made in `3a09f7d` because it moves test files, brings `switch.rs` from 893 to 697 lines. T's test additions at GREEN (4 table rows and 1 test) close two survived mutations.
- **Under-delivery:** none against the acceptance list. One decision clause, D9's "raised for a Claude Code pane too", holds by structure but has no test (advisory 1).

## Verdict

**REWORK** (production; item 2 is the test that must pin item 1, for T at T-green, since F writes no test):

1. **`crates/holler-pane/src/reconcile/observe/agent.rs:129-133`: quote the profile name in the W-3 clause.** In both places it appears, the prose "the pane is in the profile ..." and the argument "--profile ...", render `pane.profile` as `quoted(profile.as_str())` and not through `Display`. That is what `reconcile/drift.rs:80` does for the same value in `profile-drift`'s message, and `quoted` is already imported in `agent.rs`. A member of `Some Profile` then reads `... relaunch it with --profile "Some Profile" too ...`, one shell word, as the repository's rule asks (#662 B1, `holler-cli/src/profile/delete.rs:203-205`) and as `findings.rs`'s module rule asks for every untrusted value. Do not change the remedy: it stays the table's `holler pane relaunch PANE --agent=KEY`, with no profile, until the operator decides W-3. The doc comment at `agent.rs:89-91` should say that the profile name is quoted too.
2. **TEST, for T at T-green: pin item 1.** `crates/holler-cli/tests/pane_verbs/doctor/agent.rs:173-191` (`a_profile_members_message_names_its_profile`) seeds the profile `blue`, which reads the same raw or quoted, so it passes with item 1 reverted. Add a member of a profile whose display name has a space, for example `Seed::new(PANE, 1, 1).in_profile("Some Profile")`, which `ProfileName::parse` accepts. Assert that the message names it as one quoted value (for example that it contains `--profile "Some Profile"`) and does not contain the raw `--profile Some Profile`.

## Advisory notes (non-blocking)

1. **D9's Claude Code clause has no test.**
   - The gap: "It is raised for a Claude Code pane too" is in D9 and in the guide ("whatever the pane's hold, role or harness"). It holds because `check_tmux` runs before the shape branch (`observe.rs:118` against `:130`). But no test would fail if `check_attached` were skipped for a single-process pane: every Claude Code world (`seed_claude`, `applied()`) has a client.
   - Recommended, while T is at T-green for item 2: one test in `pane_claude` in which a Claude Code pane whose client is detached raises exactly one `unattached`.
   - Why it is not a REWORK item: it is not an acceptance criterion, and the decision is implemented.
2. **Acceptance 9's "a message without an agent is skipped"** conflicts with the brief's own ADR text, "`None` when it recorded none". T followed the ADR, journalled it, and pinned it with `a_turn_the_server_says_nothing_about_is_none`. This is the conservative reading, so no change is needed. It is noted for the operator because the brief's text is defective.
3. **The `GET /agent` wrong-shape error still quotes up to 60 bytes of the body.** It goes through `Call::unexpected`, which is `verify_serving`'s existing behaviour, kept by F (design decision 2). Doctor can now reach it on every pass. Acceptance 9's "quoting nothing" is pinned for the message list, which is conversation text. The agent list is configuration (it includes each agent's `prompt`), and 60 bytes is the crate's documented cap. It is low risk, and a follow-up only if the operator wants "quote nothing" for this route too.
4. **Steps after the PR opens that belong to the run's agent** (they are not F's):
   - **(a) File the run and restore part 1's record.** Do the brief's "Pipeline" step 3: file the run under `docs/handoffs/647-part2/` and move part 1's record back from `647-part1-record/` to `docs/handoffs/647/` and `docs/handoffs/647-brief.md`. Then check that `git diff --stat origin/main -- docs/handoffs/647 docs/handoffs/647-brief.md` is empty before the merge. Today the branch moves part 1's record aside and fills `docs/handoffs/647/` with part 2's.
   - **(b) Add the AI disclosure.** It goes in the PR body, per `CONTRIBUTING.md`, with `gh pr edit`.
   - **(c) Keep `Closes #647.`** It is right only while the PR delivers both findings, which it does.
   - **(d) Cite the section in the squash commit.** The squash commit message should cite ADR-0021 section 12. The script's code commit `chore(#647): f -- implementation done` does not, which acceptance 20's "the code commits cite the section" asks for.
5. **Still the operator's:**
   - **Q2:** merging before the live cutover (#654) is signed off changes what `doctor` reports during the soak. The operator's 2026-10-11 comment allows the contract change, but the brief leaves the merge timing open.
   - **Q1:** a verb that attaches a client. It is deferred, and the ADR and the guide say so.
   - **W-3:** whether the remedy itself should carry `--profile`. If yes, it must be POSIX-single-quoted per #662 B1, in the one remedy table.
   - **W-7:** the whole message list is read on every pass. Paging is the follow-up.
   - **W-2 (A-dup warn 2):** #839, PR #863, is still open. Whichever of #839 and this story merges second folds the two remedy mechanisms in `findings.rs` and files the follow-up.
6. **Smaller notes:**
   - **A-dup warn 1:** `assert_end_state` copies `Scratch::findings`' mapping inline. It is test-only, and T can fold it while in the file.
   - **A-dup warn 3:** the host adapter's `list-panes -s` builder now has four copies. That is an XS follow-up.
   - **rustfmt:** drift in four test files follows those files' style on `main`. CI does not run rustfmt (T's re-entry).
7. **Remote `main` has moved to `ef14b1a`** since `6eff9cc`, by one commit: #847's flake fix (PR #865). It touches only `remote_admin_test.rs`, its rig and #847's handoffs, so it does not overlap this branch, and the #847 flake T saw at GREEN is fixed there.
