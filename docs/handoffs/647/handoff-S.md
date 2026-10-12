# Handoff-S: Phase 8 - #647 part 2: `unattached` and `agent-cannot-dispatch` (spec audit)

**Date:** 2026-10-12 (round 1 at 11:52 MDT, REWORK; round 2 at 12:42 MDT, **PASS**, see "Round 2" below)
**Branch:** `issue-647-implementation` at `f62ff16`
**Issue:** #647; brief `docs/handoffs/647-part2-brief.md`

## Round 2 (re-audit after round 1's REWORK, at `f62ff16`)

**Date:** 2026-10-12 (12:42 MDT)
**Branch:** `issue-647-implementation` at `f62ff16`. The round's commits are F's rework `6f07eff`, T's GREEN `1d12249` and A-dup's PASS `f62ff16`. The merge base with `origin/main` is still `6eff9cc`.
**Issue:** #647. That is the GitHub issue with its amendments of 2026-10-09, the operator's park of 2026-10-11 and the operator's decision of 2026-10-11 that allows the contract change. The brief is `docs/handoffs/647-part2-brief.md`.
**Handoffs reviewed:**
- `handoff-F.md` ("Rework round");
- `handoff-T-green.md` ("Re-entry 2");
- `handoff-A-dup.md` ("Re-entry");
- this round's F, T and A entries in `decisions.md`;
- the seven entries F added to `evidence.md`;
- round 1 of this handoff, below.

**Diff read:**
- the round itself, `git diff e393e87 HEAD -- crates/`: two production files and two test files;
- again, every production file of `git diff origin/main...HEAD -- 'crates/*/src'`;
- the docs diff (ADR-0021, `docs/pane-control.md`, `CHANGELOG.md`);
- every new or edited test file that proves an acceptance criterion.

I am a fresh S, so I re-audited the whole branch rather than only the round. I did not re-run Tier 1 or Tier 2 (T's job).

### A precondition

Met.
- The plan review (`handoff-A.md`) is **PASS**, with 0 blocks and 8 warns.
- The anti-duplication gate's re-entry (`handoff-A-dup.md`, at `1d12249`) is **PASS**, with 0 blocks:
  - R1 is new: `findings::single_quoted` is the third copy of a one-line POSIX quoting body.
  - R2 is carried: #839's `findings.rs` touches the same conflict region.

### T precondition

Met.
- **RED.** `handoff-T-red.md` records a valid RED, unchanged since round 1.
- **GREEN.** `handoff-T-green.md` "Re-entry 2" records **"Blocking issues: none"** and PASS at `6f07eff` plus T's two test edits (committed as `1d12249`):
  - the workspace `test.unit.command`: 171 binaries, 2689 passed, 0 failed, 39 ignored;
  - clippy `-D warnings`, `lint.sh`, `changelog-check`, `docs_cli_test`, `wire_selftest` and `cargo machete` are clean;
  - the round's four mutations (Q0, Q1, Q2, C1) were each killed by an assertion.

### Round 1's REWORK, item by item

1. **Done (production, `6f07eff`), with a deviation from round 1's literal fix that I accept.**
   - **The change.** `crates/holler-pane/src/reconcile/observe/agent.rs:132-139` no longer prints `pane.profile` through `Display`. It renders the profile in two ways:
     - the prose ("the pane is in the profile ...") through `quoted(profile.as_str())`, as round 1 asked and as `reconcile/drift.rs:80` does;
     - the suggested argument ("relaunch it with --profile ... too") through the new `findings::single_quoted` (`findings.rs:439-441`, `pub(crate)`, typed `&ProfileName`), which applies POSIX single quotes.
   - **The deviation.** Round 1 prescribed `quoted` in both places. Round 1's stated goal was "one shell word, as the repository's rule asks (#662 B1)", and that rule is POSIX single quoting. I checked it at `holler-cli/src/profile/delete.rs:202-205`: "A profile name in a command that a message suggests goes through it, so the command pastes into a shell as it is, even for `Some Profile` or a name with a quote or `$(...)`."
   - **Why `quoted` would fall short.** `quoted` is `error::excerpt`, Rust `{:?}` (`error.rs:691-698`). A shell still expands `$(...)` and backticks inside its double quotes. So `--profile "It's $(id) Demo"` would run `id` when pasted.
   - **The repository's own pattern.** Messages that both name a profile and suggest a command with it use `{:?}` in the prose and single quotes in the command:
     - `delete.rs:158-183`;
     - `pane/profile_scope.rs:54-55`;
     - ADR-0021 section 3, line 531.

     F's split is that pattern exactly.
   - **What still holds.** `quoted` never cuts a profile name: `excerpt` cuts at 64 characters, and a `ProfileName` is at most 64. A `ProfileName` has no control character, so the single-quoted form puts no control sequence on a terminal. The remedy is still the table's `holler pane relaunch PANE --agent=KEY`, with no profile, as round 1 required.
2. **Done (test, `1d12249`).**
   - **The new test.** `pane_verbs/doctor/agent.rs:172-214`: `a_profile_members_message_names_its_profile_as_one_shell_word`, over the helper `profile_members_message`, replaces the `blue` test and keeps its remedy assertion.
   - **For `Some Profile`, it asserts:**
     - the prose `the profile "Some Profile"`;
     - `--profile 'Some Profile' too`;
     - neither the raw `--profile Some Profile` nor a double-quoted `--profile "Some`.
   - **For `It's $(id) Demo`, it asserts:** `--profile 'It'\''s $(id) Demo' too`.
   - **Mutation evidence.** T's mutations show the test fails for the pre-rework clause (Q0), for round 1's literal form (Q1) and for single quotes in the prose (Q2).
3. **Round 1's advisory 1, taken.** `pane_claude/doctor.rs:199-214`, `a_claude_pane_with_no_tmux_client_is_unattached`, pins D9's "raised for a Claude Code pane too":
   - the Claude Code pane raises exactly `["unattached"]`, with a null remedy;
   - the attached OpenCode pane beside it raises nothing;
   - T's mutation C1 (skip the count for a single-process pane) fails it.

### Acceptance criteria

Rows marked "round 2" changed in this round; every other row was re-checked against the code and is as round 1 recorded it.

| # | Criterion (brief) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | One `compile_fail,E0046` per new method; `PidsOnly` pins `foreground_commands` alone | `ports.rs` doc tests `NoClientCount` (`HostPort`) and `NoAgentView` (`HarnessPort`); `PidsOnly` gains `attached_clients`, and its lead sentence names the one method it omits. RED in T's RED commit, GREEN after F (11 doc tests) | PASS |
| 2 | `AgentView`: empty `Default`, no struct literal outside the crate, builders | `holler-pane/tests/agent_view_test.rs` (5 tests); two `compile_fail,E0639` doc tests on `HarnessPort` (F checked that `E0639` is the error, from an outside crate) | PASS |
| 3 | `CODES` ends `unattached`, `agent-cannot-dispatch`; unique, kebab-case, equal to the JSON form | `findings_test::finding_kind_codes_are_stable` (18 codes, uniqueness, `is_valid_code`, serde form) | PASS |
| 4 | `remedy_for` table; `--agent=-x` parses through the real clap tree; `remedy` unchanged | `findings_test`: the five remedy tests, including `remedy_is_the_table_with_no_agent_and_the_agent_touches_only_its_kind`, over every kind, fix state and pane. Clap: `doctor::agent::a_dispatcher_whose_key_starts_with_a_dash_still_parses_as_the_agent` (`Cli::try_parse_from`, agent `-x`) | PASS |
| 5 | `FakeHost`: 0, then 2, then 0; `pane-not-found`; faults change nothing | `fake_host_test`: `attached_clients_counts_what_attach_client_added`, `..._of_a_missing_session_is_pane_not_found`, `clients_survive_stop_owned_and_a_second_ensure_session`, `a_fault_on_attached_clients_fails_that_call_only` | PASS |
| 6 | Host conformance case, its mutants, real tmux | Case 11, with the mutants `AttachedAlwaysOne` and `AttachedOfMissingIsZero`, each with a test that the suite fails on case 11. Opt-in `real_tmux_test` 9 of 9 (T and F, round 1) | PASS |
| 7 | `FakeHarness` view, check order, exact errors; conformance case and mutant; Claude empty view; routing | `fake_harness_agent_test.rs` (6, with the exact `Timeout`, `Unavailable` and `SessionNotFound` values in `abort`'s order); harness case 17 with its two mutants; Claude case 13 with `GuessesATurn`; `fake_claude_test` (2); `fake_harnesses_test::agent_view_reaches_the_fake_its_endpoint_names`; Claude `hermetic_test`. The empty view is for a `Process` endpoint only, and a `Port` is `server-unsupported` (A's W-4, written into ADR-0021 section 12) | PASS |
| 8 | Host adapter over fake tmux: 0/1/2, strict parse quoting nothing, `pane-not-found`, exact argv | `holler-adapter-host/tests/attached_test.rs` (6 tests); `fake_tmux_test` `METHODS` | PASS |
| 9 | OpenCode `agent_view` over 1.19.0 fixtures; rule table; errors; the real test proves `info.agent` | `holler-adapter-opencode/tests/agent_view_test.rs` (13 tests, an 18-row `can_dispatch` table); opt-in `ac647_agent_view_reads_a_real_servers_turns_and_agents` (passed on 1.19.0). A message with no agent is a `None` turn, not skipped: the ADR's reading (advisory 5) | PASS |
| 10 | `unattached`: one finding, null remedy, `not-fixable`, D11's two substrings; attached raises nothing; JSON envelope; equal exit codes | `doctor/attach.rs`: `a_session_with_no_client_raises_one_unattached_and_attached_raises_nothing`, `the_verb_reports_unattached_in_both_formats` | PASS |
| 11 | `unattached` edges | `doctor/attach.rs`: missing session (no `AttachedClients` call), failed count (`observe-failed` naming `host.attached_clients`), parked and orchestrator panes. **Round 2:** a Claude Code pane too, `pane_claude/doctor.rs::a_claude_pane_with_no_tmux_client_is_unattached` (D9) | PASS |
| 12 | `agent-cannot-dispatch` positive, and with `--fix` | `doctor/agent.rs`: `every_recent_turn_unable_to_dispatch_raises_one_finding_naming_the_relaunch` (four turns seeded, the oldest able to dispatch, which pins `RECENT_TURNS = 3`) and `fix_changes_nothing_for_it` (no `ServeCommand`, no `SelectSession`, record and agent unchanged) | PASS |
| 13 | Negatives, each its own test | `doctor/agent.rs`:<br>- the two `a_record_that_names_*` tests;<br>- `no_dispatcher_raises_nothing`;<br>- the two one-odd-turn tests;<br>- `an_empty_view_raises_nothing`;<br>- `several_dispatchers_give_no_remedy_and_a_message_that_lists_them` (two quoted; three of four);<br>- the five `*_is_never_asked` tests, each asserting no `AgentView` call and the case's own finding | PASS |
| 14 | Once per pass; records equal after both passes | `doctor/agent.rs::a_second_pass_reports_the_same_two_findings_once_each_and_writes_nothing`; `doctor/attach.rs::unattached_is_reported_once_per_pass_and_writes_nothing` and `fix_does_nothing_for_unattached` | PASS |
| 15 | Allowlist extended by exactly the two ops; with and without `--fix` | `doctor/read_only.rs::doctor_without_fix_makes_only_read_calls` and `doctor.rs`'s `--fix` test, through `rig::HOST_READS` (`Ps`, `AttachedClients`) and `rig::HARNESS_READS` (the three reads plus `AgentView`); the pass is asserted to make both new reads | PASS |
| 16 | Every live-world builder attaches a client; the attach, not a disabled rule, keeps worlds clean | The attaches: `doctor/rig.rs` `make_live`, `pane_claude/rig.rs` `seed_claude` and `seed_opencode`, and `pane_claude/mixed.rs` `applied()` (through `rig.host`, with `LinkedHost` forwarding only the port method). The proof: `doctor/attach.rs::a_world_built_without_a_client_raises_unattached_for_every_pane`. The suite: 2689/0 | PASS |
| 17 | Section 5's two cells as exact text; the `unattached` limit | `docs_pane_control_test::section_5_rows_of_part_2_name_their_exact_commands` (`\| none \|`, `` \| `holler pane relaunch PANE --agent=KEY` \| ``, `--profile`, "terminal") | PASS |
| 18 | `assert_end_state` takes exactly one `unattached` per scratch pane; `break_both` leaves it out | `scratch/verbs.rs::assert_end_state` (sorted `(pane, kind)` lists equal to `panes` × `unattached`); `scenario.rs::break_both`. They compile in the default run; the opt-in scratch scenarios were not run (advisory 8). Their premise is proved by the opt-in `ac647_a_scratch_server_without_project_agents_has_no_dispatcher` | PASS |
| 19 | No CLI surface change; lint; clippy; no `unsafe`; privacy | `git diff --stat origin/main...HEAD -- crates/holler-cli/tests/fixtures/cli-surface.txt 'docs/adr/ADR-0003*'` is empty. Lint and clippy are clean (T). No `unwrap`, `expect`, `panic`, `todo`, `dbg` or `unsafe` in added `src` lines (my grep). Largest touched `.rs` file: `pane_verbs/launch/rig.rs`, 865 lines. Privacy: see the Quality audit | PASS |
| 20 | Contract commit is documentation only and first | `45bff85` touches `ADR-0021.md` and only `//!` lines of `ports.rs` (my diff filter). None of the nine non-merge commits before it touches `crates/` (checked per commit) | PASS (advisory 3: the script's code-commit subjects do not cite the section) |
| 21 | Module docs name the new reads; the bound reads nine in D13's order | `reconcile.rs` step 1 and its single-process paragraph; `observe.rs` lines 6-13 ("at most nine port calls": `ps`, `attached_clients`, `health`, `list_sessions`, `shown_session`, `agent_view`, `select_session`, `shown_session`, `cas_put`); `process.rs`'s Observe bullet | PASS |
| W1-W8 | Wrapper forwarding | W1: `pane_integration/forwarding.rs` (`Counting`; three wrapper stacks; the misconfigured `WiredHost`). W2: the host conformance suite over `BindingHost` and `WiredHost` in `bind.rs`. W3: `forwarding.rs`, `wired.rs`'s two new sockets, `pane_verbs/main.rs::unwired_ports_answer_not_implemented`. W4/W5: `bind.rs` `Recording::agent_view` (`distinctive_view`, arguments `(Port(48100), "ses_1", 3)`) through `call_every_method`. W6: one-line forwards, which the compiler holds. W7: F's and T's mutation runs (round 1), all killed. W8: the `bind.rs` module doc | PASS |

### Spec compliance

- **Every decision of the brief is implemented as stated, or as A's plan review amended it.** Round 1's section below lists them (D3 to D17, and each recorded amendment: W-1, W-2, W-4, W-5, W-7, W-8 and F's sharpened `task` wording). This round changed only the profile clause of `agent-cannot-dispatch`'s message, which is A's W-3, not a brief decision. The remedy, the rule, the kinds and the JSON shape are untouched: `git diff e393e87 HEAD -- 'crates/*/src'` is `findings.rs`'s helper and module-doc sentence, plus `agent.rs`'s clause and its doc comment.
- **The deviation from round 1's literal prescription is not a deviation from the spec.** Round 1's fix instruction named `quoted`. The governing rule it cited, #662 B1, is single quoting, and F met that rule. F and T both recorded it; see "Round 1's REWORK", item 1.
- **The issue's literal cure for `unattached` is settled upstream, not by F.** The issue says "names the cure (`holler pane launch` through the adapter, or the operator's ...)". Here is how it was settled:
  - The brief's D10 and D11 replace it with "no command, and the message says why". `pane launch` refuses a registered pane, and the operator's own tool may not be named in a public repository.
  - The earlier plan check's W-3 allowed "`null` with the reason in the message".
  - Q1 asks the operator whether to build a verb.
  - F implemented the brief and named no tool. I confirmed that no production path attaches a tmux client: the TUI runs inside the session through `respawn-pane` (`holler-adapter-opencode/src/tui.rs:180-200`).
- **The brief defect of round 1 stands as an advisory (advisory 5).** Acceptance 9's "skipped" conflicts with the ADR's "`None`". The implementation follows ADR-0021, which is the standing spec.

### Quality audit

- **The rework is correct and fails safe.**
  - `single_quoted` wraps the whole name in `'...'` and turns each `'` into `'\''`, which is the body of `holler-cli`'s two copies.
  - The test pins both a space and a quote with `$(...)`.
  - A profile name with a leading `-` is quoted but still read by clap as an option. That is the same limit `profile_scope.rs`'s doc accepts for the reconcile step (advisory 7).
- **Correctness of the whole branch.** I re-read every production file this time.
  - Both reads fail closed. A failed count or view is `observe-failed` naming the call, and nothing is concluded.
  - A reply of the wrong shape is `unavailable`. That covers a message without `info.role`, an agent without a string `name` or `mode`, an empty tmux listing, and counts that disagree.
  - A rule list of an unknown shape gives `None`, which stops the rule.
  - `with_fix` is called only on the mismatch paths (`observe.rs:333`, `process.rs:171`), so nothing drops the agent remedy after `with_agent`.
  - Neither rule writes the record.
  - The pass sorts and de-duplicates (`reconcile.rs:261-263`).
- **Build guards.**
  - No `unwrap`, `expect`, `panic`, `todo`, `dbg` or `unsafe` in added `src` lines.
  - Every added `#[allow]` carries `// #647` or `// #642`.
  - Every touched `.rs` file is under 900 lines; the largest are 865, 822 and 789.
  - No dead code: clippy `-D warnings` is clean, per T. T's Q0/Q1 runs showed that the crate denies an unused helper.
- **Protocol.** None. The hub, proto, body and golden files are untouched, and there is no new error code. A finding's JSON shape is unchanged apart from two new `kind` codes.
- **Tests.**
  - Cross-process behaviour is covered hermetically, through the fake tmux and the OpenCode stub, plus opt-in real runs.
  - No fixed sleep is used as synchronization.
  - RED-first evidence is in `handoff-T-red.md`.
  - Each new test asserts behaviour, and T's mutations show each one fails when the behaviour is removed.
- **Documentation.**
  - `CHANGELOG.md` has a bullet under `## [Unreleased]` / `### Enhancements` linking #633 and #647.
  - `docs/pane-control.md` section 5 has the two rows, "eight name none", both paragraphs and the Claude Code sentence, and its deferred list names the attach verb.
  - ADR-0021 is amended in sections 2, 9 and 12, the "Deferred" list and item 34. `main` has not taken item 34 since: `origin/main` gained only #847 and #848, both test-only.
  - `findings.rs`'s module doc names `single_quoted` in its "Untrusted text" bullet.
- **Public-repository privacy: clean.**
  - I grepped every added line of the branch: 10,168 lines of code, tests, fixtures, docs and handoffs. I looked for personal host, user and machine names, home paths, tailnet and private IP patterns, the operator's attach tool by name, e-mail domains and key shapes. The only hits are `GuessesATurn` and `InventsATurn`, which contain the letters of a host name, and the brief's statement of the rule itself.
  - The fixtures carry only `/srv/demo` and `/`, no `prompt` and no `external_directory` rule.
  - The commit messages are clean.
- **Commit and PR hygiene.**
  - Every subject is a Conventional Commit, and every non-merge commit carries `Co-Authored-By`.
  - None carries a session link, which matches `main`, where no commit carries one (advisory 9).
  - The PR is not open yet, so the AI disclosure is the run's agent's step (advisory 3).

### Scope check

F delivered the brief's scope, as round 1 recorded, and this round added nothing unrelated:
- **The helper.** `findings::single_quoted`, a `pub(crate)` helper, is new and required by the fix: `holler-pane` cannot reach `holler-cli`'s copies, because the dependency runs the other way. A-dup's R1 records folding the copies as a follow-up.
- **The tests.** One test was replaced, in the brief's `doctor/agent.rs`. One test was added in `pane_claude/doctor.rs`. That file is not in the Blast radius list, but the test is T's pin of round 1's advisory 1, in the suite that the Blast radius's `pane_claude/rig.rs` builds.
- **Under-delivery.** None. D9's Claude Code clause, the one gap round 1 noted, now has its test.

### Verdict

**PASS.** Round 1's two REWORK items are resolved, the production item by a form that meets round 1's stated goal better than its literal fix. All acceptance criteria (1 to 21, W1 to W8) have a proving test or evidence. The brief's decisions are implemented as stated or as recorded amendments, and the quality audit finds nothing blocking. Ready for O.

### Advisory notes (non-blocking)

1. **Before the merge, do the brief's "Pipeline" step 3 (the run's agent, after the PR opens).**
   - **The state now.** The branch fills `docs/handoffs/647/` with part 2's record and keeps part 1's in `docs/handoffs/647-part1-record/`. Merged as it is, part 1's merged record would leave its path on `main`.
   - **The steps.**
     1. `git mv -k` the run's files into `docs/handoffs/647-part2/`.
     2. Move part 1's `647-brief.md`, then the rest of `647-part1-record/`, back.
     3. `rmdir` the emptied directory.
   - **The check before merging:** `git diff --stat origin/main -- docs/handoffs/647 docs/handoffs/647-brief.md docs/handoffs/647-brief-result-r1.md` is empty.
2. **Q2 is still open, and #654 is still open.** I checked with `gh issue view 654` at about 12:41 MDT; the issue is OPEN.
   - **The operator's position.** The operator's park (2026-10-11) holds part 2 until after the live cutover. The later decision of 2026-10-11 allows the contract change but does not say when to merge.
   - **What merging changes.** `doctor` on the live fleet would report `unattached` and `agent-cannot-dispatch` during the soak. Holler itself attaches no tmux client: a live pane counts as attached only if something outside Holler attached one.
   - **What the brief says.** It leaves the merge timing to the operator. The run's agent should surface Q2 before it merges.
3. **PR steps for the run's agent.**
   - Add the AI disclosure to the PR body (`CONTRIBUTING.md`) with `gh pr edit`.
   - Keep `Closes #647.`, which is right because both findings are delivered.
   - Cite ADR-0021 section 12 in the squash commit. The script's `chore(#647): f -- implementation done` does not, and acceptance 20 asks that the code commits cite the section.
4. **Follow-ups the gates named.**
   - **A-dup R1, XS.** Make `findings::single_quoted` `pub`, and fold `holler-cli`'s two copies onto it: `profile/delete.rs:206`, shared by `create.rs` and `import/writes.rs`, and `pane/profile_scope.rs:270`. All six call sites pass a `ProfileName`.
   - **A-dup R2 / W-2.** #839 (PR #863) is still open. Whichever of #839 and this story merges second keeps one statement of the remedy rule and the new untrusted-text sentence, and files the fold of the two remedy mechanisms.
   - **A-dup warns 1 and 3.** `assert_end_state` copies `Scratch::findings`, and the host adapter has four `list-panes -s` builders.
5. **The brief's acceptance 9 is self-contradictory (carried from round 1).**
   - **The conflict.** Acceptance 9 says "a message without an agent is skipped". The brief's own ADR text, landed in `45bff85`, says a turn's agent is "`None` when it recorded none".
   - **What the code does.** The implementation follows the ADR, the standing spec, and T pinned it (`a_turn_the_server_says_nothing_about_is_none`).
   - **Why no hold.** It is the conservative reading: it can only suppress a finding. OpenCode 1.19.0 records an agent on every user message (measured), so the case does not arise in practice. Neither fact warrants an ADVISORY-HOLD; this note is for the operator.
6. **The `GET /agent` wrong-shape error still quotes up to 60 bytes of the body (carried).** That is `verify_serving`'s existing behaviour, through `Call::unexpected`. Doctor can now reach it on every pass. The agent list is configuration, not conversation text, and 60 bytes is the crate's documented cap. Change it only if the operator wants "quote nothing" for this route too.
7. **A profile name with a leading `-` (`ProfileName` allows `-x`).** It is single-quoted, but clap still reads `--profile '-x'` as an option, so that one suggested argument does not parse. `profile_scope.rs` accepts the same limit for the reconcile step. The joined `--profile='-x'` would parse, but no message in the repository uses that form.
8. **Acceptance 18's opt-in scratch scenarios have not been run on this branch.**
   - **Why it matters.** `assert_end_state` now expects exactly one `unattached` per scratch pane, which rests on Holler attaching no tmux client. I confirmed in the code that it attaches none (`tui.rs`: the TUI runs inside the session through `respawn-pane`).
   - **What would check it.** One run of the four scenarios would confirm it against real tmux and Herdr. Herdr gates can run headless on an isolated Herdr session.
   - **Why it is not required.** The acceptance asks only that the default run compiles and passes, which it does.
9. **Smaller notes.**
   - **rustfmt.** Four test files keep their long-line style from `main`; CI runs no rustfmt (T, re-entry 1).
   - **Session links.** No branch commit carries a session link, which `CONTRIBUTING.md` describes; `main`'s commits carry none either. The squash body takes the PR body's disclosure.
   - **Remote `main`.** It is at `05471b9`, after #847 and #848, both test-only. They share no file with this branch (`comm` of the two file lists is empty).
10. **Still the operator's.**
    - **Q1:** a verb that attaches a client. It is deferred in the ADR and the guide.
    - **W-3:** whether the remedy itself should carry `--profile`. If it does, it must be single-quoted in the one table.
    - **W-7:** whether to page the message read.

---

Round 1 (at `e393e87`, verdict REWORK) follows, unchanged.

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
