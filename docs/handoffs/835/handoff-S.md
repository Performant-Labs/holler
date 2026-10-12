# Handoff-S: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`  (spec audit)

**Date:** 2026-10-11, 20:05 MDT
**Branch:** `issue-835-implementation` at `e5e35ee`. `origin/main` is `bda1069` (`git fetch` at 20:03 MDT), and it is an ancestor of the branch: T-red's two-parent merge `526bbd3` brought it in. So the branch tree is the PR's merge result.
**Issue:** [#835](https://github.com/Performant-Labs/holler/issues/835), OPEN. I read it live with `gh issue view 835`. Its body matches the brief's issue section line for line (I diffed the two).
**Brief:** `docs/handoffs/835-brief.md`: the issue, the contract amendment and rounds 2 and 3. It is unchanged since `f8a1864`.
**Handoffs reviewed:**
- `handoff-A.md`: pass 4, PASS. Passes 1 to 3 are in git: `3abedc8`, `cc3d366` and `49e4508`.
- `handoff-T-red.md`: cycle 2. Cycle 1 is at `36b0de8`.
- `handoff-F.md`: cycle 2. Cycle 1 is at `9bd101b`.
- `handoff-T-green.md`: cycle 2, PASS. Cycle 1's BLOCK is at `25ff495`.
- `handoff-A-dup.md`: PASS.
- `decisions.md` and `evidence.md`.

**Diff audited:** `git diff origin/main...HEAD` outside `docs/handoffs/`, which is 37 files:
- 14 production files: 7 in `holler-pane`, 2 in `holler-adapter-host`, 3 in `holler-cli` and 2 in the test kit's `src`;
- 19 test files, the CLI surface fixture included;
- ADR-0003, ADR-0021, `docs/pane-control.md` and `CHANGELOG.md`.

**Verdict:** **REWORK** (production, docs only). There is one item: the user-facing text says the import reads "each process" of the tmux session. It actually reads the foreground process of each tmux pane, which is the signal the brief defines. Everything else passes.

## A precondition

Met.
- `handoff-A.md` (pass 4) is **PASS**, with 0 block and 4 warn. Pass 4 re-checked F's cycle 1 for drift and found none. It then planned the rework: the merge with `main`, the guide, and #840's and this story's stale records.
- `handoff-A-dup.md` is **PASS**, with 0 block and 1 warn (test-only, optional). It reviewed the whole feature diff, `bda1069..b488a33`, and found no parallel path.

## T precondition

Met.
- **RED, cycle 1** (`36b0de8`). The final RED was a compile RED: the test kit calls the new required method. T also proved the RED at assertion level, in two steps:
  - On unchanged code, these failed on assertions: `import_test`, both `compile_fail` doctests (each "compiled successfully"), `cli_surface_test` and six `import::` tests.
  - Under a bare-API stub, which was reverted and checked with `sha256sum`, 28 tests failed, each on an assertion and each in this story.
- **RED, cycle 2** (`526bbd3`). Two docs tests failed on their assertions: #652's `import_findings_are_named_in_section_3_or_10` and the new `section_3_names_every_flag_of_pane_import`. On the merged tree nothing else failed (2396 passed, 2 failed).
- **GREEN** (`b488a33`). CI's test step on the merged tree gave 2398 passed, 0 failed and 32 ignored. These are clean too: clippy with `-D warnings`, `lint.sh`, `changelog-check.sh`, `docs_cli_test`, `wire_selftest`, `cargo machete`, rustdoc with `-D warnings` and `git diff --check`. There are 0 blocking issues.
- **Mutations.** Each one fails the tests that name the behaviour:
  - the rule always `false`: 4 tests fail;
  - dead panes kept by `parse_foreground`: 1 test fails;
  - the guide's two codes and the flag renamed: 2 tests fail.
- **Real tmux.** Host conformance case 10 passed on tmux 3.7c (`real_tmux_test -- --ignored`, T-green cycle 1).

## Acceptance criteria

Paths are under `crates/`. `harness.rs` is `holler-cli/tests/pane_verbs/import/harness.rs`.

| # | Criterion (source) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | A pane whose tmux session runs `claude` is not registered. It gets one `info` finding, `harness-claude-code`, and nothing is written for it (issue Work 1; brief §2, §3 and R2-4). | `harness.rs::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode` (:93), a real run:<br>- B has exactly one finding, `harness-claude-code` at `info`.<br>- B's row is `skipped`, with null role, position and session.<br>- B has no record, and `fleet` holds A and C only.<br>The mutation "rule always `false`" fails it. | Met |
| 2 | Not a block: the import goes on for the other panes. One finding and no others, and no harness call for the pane (Work 1; §3). | - The same test: A and C are recorded (`recorded` is 2). There are 2 `health` and 2 `list_sessions` calls for 3 panes, so B's server is never asked.<br>- `a_pane_left_alone_blocks_nothing_even_unlabelled` (:172): an unlabelled B still has only `harness-claude-code`, and the real run is not refused. | Met |
| 3 | A dry run shows the same finding and writes nothing (Work 1; the issue's fixture). | `a_dry_run_reports_the_pane_left_alone_and_writes_nothing` (:143):<br>- JSON, and text (`B skipped - - -`, `  info harness-claude-code: `);<br>- no store write, no file changed, no record and no profile. | Met |
| 4 | A pane left alone shows as `unregistered-herdr-pane` in a whole-fleet `doctor` pass, which is expected (Work 1). | Two pins combine:<br>- criterion 1: the pane gets no record;<br>- `holler-cli/tests/pane_verbs/doctor.rs::unregistered_herdr_pane_is_reported` (:131, #647): a whole-fleet pass reports any Herdr pane that no record names.<br>`reconcile.rs:362-372` (`evidence.md`, F cycle 2, fact 3). This is a consequence, not new behaviour. | Met |
| 5 | `--adopt-claude-code` is refused as `usage` (exit 2), naming #831, with nothing written. A dry run is refused too, before any file or port is read (Work 2; §4). | `adopt_claude_code_is_usage_naming_831_and_writes_nothing` (:193), alone and with `--dry-run`, in JSON and text:<br>- exit 2, code `usage`, and the text contains `not_implemented_message(831)`;<br>- one stderr line;<br>- no Herdr, host or harness call, no store write and no file changed.<br>The code is the first check of `Input::read` (`holler-cli/src/pane/import.rs:175-181`). | Met |
| 6 | "After #831 it registers them with that harness" (Work 2). | Not built: brief §5 puts it out of scope. `HarnessKind` has only `Opencode`, and no story owns the adoption. It is recorded in ADR-0021 "Deferred to named stories" (`:1105-1107`) and the guide's section 12. | Out of scope by a recorded decision (operator item 2) |
| 7 | The signal: a read-only `HostPort::foreground_commands`. It is required, reads names only and never a command line, and a missing session answers `pane-not-found` (Work 3; §1; R2-1). | - `holler-pane/src/ports.rs:303`: a `compile_fail,E0046` doctest (`PidsOnly`), RED before F.<br>- Conformance case 10 (`holler-pane-testkit/src/conformance/host.rs:226`), with two `Mutant` breaks (`host_conformance_test.rs:241` and `:249`).<br>- The fake: `fake_host_test.rs:316`, `:337` and `:352`.<br>- The adapter, `holler-adapter-host/tests/foreground_test.rs`, 4 tests: live panes in tmux order, the dead one dropped; one exact-target `list-panes -s`; no `start_command` in the format (I7); a missing session is `pane-not-found`; any other failure is `unavailable` with stderr's first line only; one bad line fails the whole read.<br>- `fake_tmux_test.rs` `METHODS`: the bound, the reaped child, the missing binary, and `op` equal to `HostOp::as_str`. | Met |
| 8 | A pane whose OpenCode server is merely down is still adopted and flagged `server-down` (Work 3). | `holler-cli/tests/pane_verbs/import/join.rs::an_unhealthy_server_is_down_or_wedged_as_reconcile_says` (:334, #650) now goes through the new chain: no process, so not `claude`, so the server is asked. It records `Unhealthy("server-down")`. Criterion 9 adds that `bash` is also imported as OpenCode. | Met |
| 9 | Only an exact `claude` is left alone. `bash`, `opencode` and an empty session are OpenCode. There is one constant and one predicate (§2; R2-7). | - `holler-pane/tests/claude_code_rule_test.rs:14` and `:20`: these are not Claude Code: `bash`, `sh`, `opencode`, `node`, `env`, the empty name, case and padding variants, `claude-code`, `xclaude` and a path.<br>- `harness.rs:93`: A (`bash`) and C (no process) are recorded as `expected_record`.<br>- `pane.rs:147` and `:157` hold the only command-name comparison with `claude` in any `src` (grep). The other `"claude"` strings are protocol harness ids. | Met |
| 10 | An unreadable signal fails closed. A read error other than `pane-not-found`, a failed `ps` and a lost chain are each `harness-unreadable` (a problem): no record, and a real run is `import-blocked`. A `pane-not-found` from either read is `tmux-session-missing` (§2; R2-6; R2-2). | - `a_failed_ps_is_harness_unreadable_and_blocks` (:227). It replaces #650's opposite test, with a pointer left at `join.rs:391-393`.<br>- `a_failed_command_name_read_is_a_problem_and_blocks` (:261), both cases. The message names `host.foreground_commands`.<br>- `a_lost_chain_is_harness_unreadable_and_blocks` (:327). | Met |
| 11 | The surface: the `cli-surface.txt` line, ADR 0003's row, ADR-0021 section 9's row and the help text. `--format=json` shows the finding like every other (Work 4; §3, §4). | - `holler-cli/tests/fixtures/cli-surface.txt:171`, through `cli_surface_test::every_surface_line_parses`, RED before F.<br>- `holler-cli/tests/pane_verbs/import.rs::the_adrs_name_the_switch_and_the_two_new_codes` (:123).<br>- The JSON assertions of criteria 1, 3 and 10.<br>- I read the help: `holler-cli/src/pane/import.rs:67-78` and `:115-118`. | Met. The help's wording is REWORK 1. |
| 12 | The code table: two new codes, at `info` and `problem`, and the prose counts (R2-2; R2-4). | `holler-pane/tests/import_test.rs::import_finding_codes_are_the_pinned_set` (:99, RED before F) and `each_import_finding_has_its_level` (:147). `import/findings.rs` says "eighteen kinds" and "twelve" are the import's own, which matches `ALL` (18 entries, 6 shared). | Met |
| 13 | ADR-0021 records the change (§1 records; R2-2; R2-5):<br>- section 2's port table and its "amended since" sentence;<br>- section 13's "Decided (#835)": the method, the rule, its limits, the two codes and the switch;<br>- section 9's row;<br>- the closing lists. | `the_adrs_name_the_switch_and_the_two_new_codes` pins the row, the paragraph and both codes. I read ADR-0021 `:108-109`, `:121-124`, `:760`, `:950-986`, `:1105-1107` and `:1168-1174`. The limits there are stated exactly, as "a pane's foreground process". | Met |
| 14 | Every host decorator forwards `foreground_commands`, and each is pinned (R2-1). | - `holler-cli/tests/pane_integration/bind.rs::the_binding_host_passes_the_host_conformance_suite` (:319), which now includes case 10.<br>- `the_wired_host_passes_the_host_conformance_suite` (:330, new).<br>- `wired.rs:111`: with a misconfigured socket, the new method answers `usage`.<br>- `Unwired` answers `NotImplemented` (`holler-cli/src/pane/wiring.rs:328`).<br>The production chain is `BindingHost` over `WiredHost` over `TmuxHost` (`wiring.rs:153`, `:172`). | Met |
| 15 | `HerdrPort::observe_pane` is required, `observe_panes` keeps its default, and `BindingHerdr` and `LazyHerdr` forward both (R3-1; R3-2). | - `holler-pane/src/ports.rs:180`: a `compile_fail,E0046` doctest (`NoObservation`), RED before F.<br>- D33's pins, which already existed: `holler-cli/tests/pane_integration/observe.rs:184`, `:192`, `:202` and `:52`, and `bind.rs:294`. They were not copied, as A's reuse map asks.<br>- All 11 `HerdrPort` implementations answer `observe_pane`, since the workspace builds. The doctest's `NoObservation` leaves it out on purpose. | Met |
| 16 | Over the real wiring, `pane list` shows an unmanaged pane's working directory, and its last screen change when the adapter reports one. Otherwise the time is null in JSON and `-` in text (R3-3). | `holler-cli/tests/pane_integration/observe.rs:290` and `:308`, in JSON and text. They run over `BindingHerdr(LazyHerdr(FakeHerdr))`, as `Wiring::from_env` builds it (`wiring.rs:139-152`). | Met, as regression pins (see Spec compliance, item 4) |
| 17 | #652's guide is told of the new default and the flag (§6). | - `holler-cli/tests/docs_pane_control_test.rs::import_findings_are_named_in_section_3_or_10` (:451, #652's), RED in cycle 2.<br>- `section_3_names_every_flag_of_pane_import` (:531, new), RED in cycle 2.<br>- I read the prose of sections 1, 3, 10 and 12 against the code. | Met, except the wording of REWORK 1 |
| 18 | A `CHANGELOG.md` `[Unreleased]` entry links #835 and names the follow-up to #832 (§6; R3-4). | `CHANGELOG.md:67-80`, under `## [Unreleased]`, `### Enhancements`. It links #633, #835, #831 and #832. `changelog-check.sh` is ok. | Met. Its wording is part of REWORK 1. |
| 19 | The fixture additions: a pane left alone writes nothing, in a real run and in a dry run. | Criteria 1 and 3. | Met |
| 20 | Case 10 holds on a real tmux (R2-8):<br>- it asserts no empty list;<br>- it polls within a bound;<br>- it asserts the name the process settles to;<br>- a `Mutant` break fails it. | `holler-pane-testkit/src/conformance/host.rs:226-252` polls every 20 ms for up to 2 s. Both breaks fail case 10. It passed on tmux 3.7c. | Met |

No test is a tautology. Each criterion's test either was RED before F, failed under a mutation, or is a regression pin that is named as one (criterion 16).

## Spec compliance

**As stated:**
- the signature;
- names only, from the `ps`-shaped `list-panes` listing;
- a parser as strict as `parse_ps`;
- the errors, through `Call` and `refused()`;
- `ps` left unchanged;
- `HostOp::ForegroundCommands`, whose `as_str` is the adapter's `OP_*`;
- the rule and its one home;
- fail closed, with no record on a guess;
- the two codes and their levels;
- `Outcome::claude_code`, through `left_alone`, with no new `Action` or `Level`;
- the switch at one site, with `not_implemented_message(831)`;
- the records.

**Two differences that the brief or A recorded, and that I treat as decisions:**
- The level is `info` where `harness-not-opencode`'s is `skipped`. Brief §3 calls it a deliberate, recorded difference.
- The switch answers `usage`, not `not-implemented` (brief §4).

**The code departs from the brief's literal text in four places, each recorded and none silent:**
1. **Where `observe_pane`'s change is recorded.** R3-1 says to record it in section 13's "Decided (#835)" paragraph. It is recorded instead in section 14 (in place), in section 2's "amended since" sentence and in decision 23.
   - A recommended this in pass 3 (W2). Section 14 specifies the method and would otherwise contradict the change.
   - The reason is given in all three places.
2. **R3-2's second bullet** ("No decorator or test double may rely on a default for either method") cannot hold while R3-1 keeps `observe_panes`'s default.
   - A (pass 3, W5) read it as applying to forwarding wrappers. The production wrappers comply: `BindingHerdr` and `LazyHerdr` override both methods (`bind.rs:138-142`, `:253-257`).
   - See Advisory 3 for the test-only wrappers.
3. **F's design decision 1.** A pane whose tmux read failed still has its server asked.
   - Brief §3 forbids a harness call only for a Claude Code pane, and that pane makes none (criterion 2).
   - A (pass 4) accepted it. The pane is blocked either way.
4. **R3-3's two tests passed before F.** #649's D33 had already made both Herdr wrappers forward.
   - The behaviour they assert is the acceptance item. They are pinned end to end over the production chain, in both formats.
   - The part of R3 that is #835's change, "no default", is proved by criterion 15's doctest, which was RED before F.
   - I accept the pins as R3-3's evidence, as A (pass 3, W1) proposed.

**One departure no one recorded: REWORK 1.** The brief defines the signal as "the command name of the foreground process of every live pane in the named tmux session" (§1). R2-5 states why the rule has limits: tmux reports "the argv[0] basename of the pane's foreground process". The ADR (`:953`, `:973-976`), the port docs (`ports.rs:340-344`) and the rule's docs (`pane.rs:150-156`) say this exactly. The user-facing text says something else (see Verdict).

## Quality audit

- **Correctness and failure handling.**
  - **Every way the read can fail is closed** (`import/observe.rs:104-132`, `:203-234`):
    - `ps` runs first. The names are read only after `ps` answers.
    - A `pane-not-found` from either read is `tmux-session-missing`.
    - Any other error from either read is `harness-unreadable`, and so is a lost chain.
    - Both codes are problems, so the pane gets no record and a real run is refused before any write (`Plan::refusal`, `writes.rs:38-42`).
  - **The Claude Code path calls nothing after the read.** It returns before the server is asked, and `assess_all` routes the pane to `Outcome::claude_code` before the join.
  - **Messages.** They quote no observed name. An adapter error goes through `failed_message` and `embedded`, as #650's did.
  - **The adapter.** It adds no new subprocess path. `foreground_commands` is `ps`'s shape: `Call::start`, `run_tmux`, a strict parse, and `refused()`.
  - **The import still writes nothing until the plan.** It also adds no state and no concurrency hazard. The extra read runs on the pane's own scoped thread.
- **Build guards.**
  - No `unwrap`, `expect` or `panic` is added in any `src`. The test kit's conformance case uses `succeeds` and `expect_code`.
  - The two new `#![allow]` lines are in new test files and carry `// #835`.
  - The largest touched `.rs` file is `pane_verbs/launch/rig.rs`, at 839 lines. Every other file is under that.
  - `dead_code = "deny"` and the build is clean.
  - The production files of this diff are `rustfmt`-clean. `wiring.rs`'s one drift is #832's line 76, outside this diff.
- **Protocol.** None changed.
  - No file in `holler-proto` changed, and neither did `docs/protocol/v2.md` or any golden file.
  - No closed error code was added: `usage` already exists, and the two new codes are import findings.
  - ADR-0021 section 9's row is updated.
- **Tests.**
  - The import is a CLI verb over ports, so no behaviour here crosses processes. The tests run in-process over the test kit's fakes, and the real tmux runs in the `--ignored` suite on a scratch server.
  - There is no fixed sleep. Case 10 polls within a bound, as R2-8 requires.
  - The RED-first evidence is in both T-red handoffs (see T precondition).
- **Documentation.**
  - These are updated: `CHANGELOG.md`, ADR 0003, ADR-0021, the guide and the verb's help.
  - #840's four sentences saying #835 calls the registry rule are corrected: `unmanaged.rs:63-69` and `:85-86`, ADR-0021 section 14 (`:1013-1016`), and the test doc comment at `pane_verbs/unmanaged.rs:467-470`. So is this story's own claim about #832 (`pane.rs:145-146`, ADR-0021 `:966-967`).
  - The intra-doc link resolves (rustdoc with `-D warnings`, T-green).
  - **One accuracy defect, which is REWORK 1.**
- **Public-repository privacy.**
  - I grepped the 2,882 added lines: no IP, no `ts.net` or tailnet name, no personal host or account name, no home path and no secret.
  - Every path is neutral: `/srv/demo`, `/srv/state`, `/opt/demo/bin/claude`, `/usr/local/bin/claude`, `/usr/bin/sleep` and `/work/seeding`.
  - I7 holds. The adapter asks tmux for `#{pane_current_command}` only, and `foreground_test.rs:54-58` pins that the format has no `start_command`.
- **Commits and PR.**
  - Every phase commit has a Conventional subject (`chore(#835): ...`) and `Co-Authored-By: Claude <noreply@anthropic.com>`. That is the form `.githooks/prepare-commit-msg` writes, and every commit on `main` has it, with no session link.
  - The PR is not open yet. Its AI disclosure and its coexistence notes are the run agent's (Advisory 5).

## Scope check

**Delivered:**
- the issue, as amended by rounds 1 to 3;
- A's recorded warns: the merge with `main`, #840's sentences, and this story's #832 claims;
- the guide's update that brief §6 asks for.

**Small, justified additions:**
- `section_3_names_every_flag_of_pane_import` (A pass 4, W4, optional);
- the `WiredHost` suite run (A pass 3, W6);
- the guide's step-1 sentence that the real run also runs while the Claude Code sessions run (F's DD3). It is true: `plan` observes in both modes.
- the four `usage` causes in the guide's exit-code paragraph (F's DD5). They match `Input::read`.

**Changes to #649's files** are only the forwards and the module doc that describes them. R2-1's lift allows this.

**Under-delivery:** only the adoption after #831, which brief §5 puts out of scope.

**Not over-delivered:** no unrelated refactor and no new rig, page, table or helper.

## Verdict

**REWORK** (production; there is no test change, so this goes to F). **Report `archChanged: false`.**

1. **The user-facing text misstates what the import reads.** It says the import reads "the command name of each process in the pane's tmux session" and leaves alone "a pane with a process named exactly `claude`". The import actually reads one name per tmux pane of the session: the name of that pane's foreground process (brief §1; R2-5's preamble). The ADR, `ports.rs` and `pane.rs` already say this.

   **Why it matters.** Under "each process", a `claude` that runs behind another foreground process would be found. That includes a `claude` started as a child by a wrapper script that stays in the foreground, and a suspended or backgrounded `claude`. The import does not find it. The limits sentence then contradicts the text's own model: it calls "a wrapper" a naming issue, and it does not say that a `claude` outside the foreground is missed.

   This is the guide's standing text for the cutover (#654). Section 10, step 1 asks the operator to judge a pane that is not recognised against "the rule's limits". A-dup (note 1) left this call to S.

   **What to change.** Reword these in place. Change no behaviour, and do not grow the guide past its 817 lines.

   | Where | Lines | Change |
   |---|---|---|
   | `crates/holler-cli/src/pane/import.rs` (the `PaneImport` doc comment, which is `holler pane import --help`) | 67-75 | "reads the command name of each process in the pane's tmux session" becomes "reads, for each pane of the pane's tmux session, the command name of its foreground process, as tmux reports it". "A pane with a process named exactly `claude`" becomes "A pane whose tmux session has a foreground process named exactly `claude`". The limits sentence states the foreground rule, for example: "tmux names only each pane's foreground process, by the name it was started under, so a Claude Code pane whose `claude` has exited or runs behind another foreground process (a wrapper script, a suspended job), or one started under another name, is taken for OpenCode." |
   | `docs/pane-control.md`, section 1 | 29 | "whose tmux session runs a process named `claude`" becomes "whose tmux session has a foreground process named `claude`". Keep the words "Claude Code" in section 1. |
   | `docs/pane-control.md`, section 3's `harness-claude-code` row | 299 | "A process in its tmux session is named `claude`" becomes "The foreground process of a pane of its tmux session is named `claude`". |
   | `docs/pane-control.md`, section 10's fact | 696-697 and 700-702 | The same two changes and the same limits sentence as the help. |
   | `CHANGELOG.md` | 70-71 and 76-77 | The same two changes. In the limits, add "or is not the foreground process (a wrapper script, a suspended job)". |
   | `crates/holler-pane/src/import/observe.rs` (`read_tmux`'s doc) | 114-116 | For consistency, "the command names of its processes" becomes "the command name of each pane's foreground process". |

   The wording is F's to choose, as long as every place says "foreground" and names a `claude` outside the foreground among the limits.

   **Then run:**
   - `cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test`. Keep the words those tests need: "Claude Code" in section 1, both codes in sections 3 or 10, and every flag in section 3's import subsection.
   - `bash scripts/changelog-check.sh`.
   - rustdoc with `-D warnings`.
   - `git diff --check`.

   **No test change is needed.** No test pins this prose, and none should be added for it.

## Advisory notes (non-blocking)

1. **The guide is 817 lines,** past #652's soft "about 800". No check enforces that. This story's 23 lines are the content brief §6 asked for. The rework above should add none. If the guide grows again, section 3's per-verb reference could move to its own page.
2. **`rustfmt` drift in this story's new test code.** It is in `foreground_test.rs`, `claude_code_rule_test.rs`, `import/harness.rs`, `pane_integration/observe.rs` (the R3-3 tests), `fake_host_test.rs`, the `use` line of `ports_test.rs` and the new test in `docs_pane_control_test.rs`.
   - CI has no format step, and `main` already drifts in `docs_pane_control_test.rs`, `bind.rs` and `wired.rs`.
   - It is test-only and optional.
3. **The test-only Herdr wrappers still use `observe_panes`'s default.** They are `ExtraHerdr` (`pane_verbs/import/seams.rs`), `DisplacingHerdr` (`launch/displaced.rs`), and the Herdr conformance test's `Mutant` and `GuardWatcher`. These files are unchanged by this story, and the brief's R3-1 lists them as fine.
   - This is harmless. The verbs they serve never list unmanaged panes, and the Herdr suite has no `observe_panes` case.
   - If the brief template is reused, R3-2's bullet should say "a production wrapper".
4. **A-dup's warn 1 (test-only, optional).** `ImportRig::run_with_host` (`import/rig.rs:258`) copies `run_with_store`. `text_row` (`pane_integration/observe.rs:279`) is a twin of the import rig's `text_line`.
5. **For the run's agent, after the script opens the PR:**
   - Add the AI disclosure that `CONTRIBUTING.md` asks for, with `gh pr edit`.
   - Add the coexistence notes asked for by R2-3 and A's W2: #832 (PR #836), #840 and #652 edited the same places, namely `ports.rs`, `wiring.rs`, `bind.rs`, ADR-0021 sections 2, 9, 13 and 14, and the guide.
   - Times in the PR body should be MDT.
6. **For the operator (outside the pipeline):**
   1. **The epic's contract block.** #633's issue body (line 123, "`HostPort ensure_session / run / stop_owned / ps`") does not name `foreground_commands`. Nor does its `HerdrPort` line name #832's `observe_pane` and `observe_panes`. The brief leaves this edit to the operator.
   2. **The adoption after #831 has no owner** (brief §5). Add a line to #831 or file a follow-up. Its write step meets `profile-exists` once `fleet` exists.
   3. **#831's owner replaces the refusal** at one code site, `Input::read` (`pane/import.rs:175`). "Refused until #831" is also prose in these places:
      - ADR-0021: section 9's row, section 13's "The switch", and "Deferred";
      - the guide: sections 3 and 12;
      - the help;
      - the CHANGELOG.
   4. **R2-5's real-fleet check.** Run the first real import's dry run while the fleet's Claude Code sessions run. The guide's section 10, step 1 now says so.
