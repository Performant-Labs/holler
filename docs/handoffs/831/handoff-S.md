# Handoff-S: #831 - A Claude Code harness for panes  (spec audit)

**Date:** 2026-10-11, 21:40 MDT
**Branch:** `issue-831-implementation` at `adad6a2` (worktree `.claude/worktrees/0831-claude-harness`). Its merge base with
`origin/main` is `d6ae539`, which F's phase commit `881ebe1` merged (two parents). `origin/main` is now `0f5835e` (#855;
`git fetch` at 21:20 MDT), not yet merged into the branch (Advisory 3).
**Issue:** [#831](https://github.com/Performant-Labs/holler/issues/831), OPEN, read live with `gh issue view 831`: the body
equals the brief's issue section, and it has no comments. The epic's amendment comment on #633 is posted (18:12 MDT).
**Brief:** `docs/handoffs/831-brief.md` and the contract amendment `008a995` (ADR-0021 section 15, the normative text).
**Handoffs reviewed:** `handoff-A.md` (PASS, 0 block, 15 warn), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`
(PASS), `handoff-A-dup.md` (PASS, 0 block, 9 warn), `decisions.md`, `evidence.md`.
**Diff audited:** `git diff origin/main...HEAD`: 119 files, +12019/-1302; 111 files, +10453/-1302 outside `docs/handoffs/`. I
read the whole production and docs diff, and the tests the table below names.

**Verdict: REWORK** (production; two contract statements and one literal; no behaviour changes), three items. Every
acceptance line has proving evidence or is deferred by the brief's own rule, and the code does what section 15 decides, as
built. But the PR would put three statements into the contract that the code does not bear out: section 15 says the
`HARNESS_IDS` doc comment records the two `claude` ids, that `CLAUDE_CODE_COMMAND` is the one place the name is written, and
that the #835 copy is "text for text". None of the three holds. #835's S applied the same bar to its own contract statements
(its REWORK 1 and 2, docs only).

## A precondition

Met.

- `handoff-A.md` is **PASS**, with 0 block and 15 warn. Each warn was carried out (Spec compliance, below), and
  `decisions.md` journals the two that corrected section 15's own text (W1, W2) as decided under the standing rules.
- `handoff-A-dup.md` is **PASS**, with 0 block and 9 warn. W7 and W8 are REWORK items 2 and 3 here. The rest are
  follow-ups (Advisory 10).

## T precondition

Met.

- **RED** (`b3f0295`). It fails at compile level on the contract: every error names a contract item, and none is a typo or a
  missing `[[test]]`. It also fails at assertion level: `cli_surface_test` rejects the 2 `--harness` lines of 148.
- **GREEN** (T-green, on `881ebe1` plus T's two test repairs):
  - the CI test step: 2488 passed, 0 failed, 34 ignored;
  - all clean: `cargo clippy --workspace --all-targets -- -D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`,
    `docs_cli_test`, `wire_selftest`, `cargo test -p holler-proto`, `test-hooks.sh` and `pane-restart-test.sh`;
  - 0 blocking issues.
- **Behaviour, not implementation.** Six mutants that remove a behaviour from production code were each killed by the test
  that pins it (W1, W2, W4, W11, the frame, W9), and the first W1 and W2 mutants, which did not compile, were replaced. The
  12 conformance cases each carry a mutant that the suite must fail.
- **T's two repairs are test-only and sound.** `findings_test`'s stable-codes list gains the two kinds that the contract
  appends. In `mixed.rs`, the ignored `say` test's `panic!` moves after its loop, for `clippy::never_loop`.
- My checks were read-only: `git diff`/`show`/`log`, `grep`, `wc -l`, `git diff --check` (clean), `changelog-check.sh` (ok),
  and `gh issue view` (831, 633, 828, 835, 839, 850 and 648). I re-ran no build or test.

## Acceptance criteria

The issue's Acceptance, read as the brief's item 18 says it can be met, then the Goal and the Work list.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | One OpenCode pane and one Claude Code pane from one profile | `pane_claude/mixed.rs::one_profile_launches_an_opencode_pane_and_a_claude_pane` checks each pane's kind, the OpenCode port, the Claude pid with no port, the session of record and the profile membership. `a_second_apply_of_the_mixed_grid_changes_nothing` checks that a second apply writes no record | Met |
| 2 | Both in `holler pane list` with SHOWN and DRIVEN | `mixed.rs::the_acceptance_scenario_on_the_fakes`: each row's `shown` and `driven` equal its record's, and `sync` is `ok`. DRIVEN is `null` for both, as the records hold it: no port observes DRIVEN until #839, and the brief reads the line "as the records give them" | Met, as the brief reads it |
| 3 | A message delivered to each through `holler say` | **Blocked by #839 and #850**, by the brief's design (item 18). `mixed.rs::say_delivers_a_message_to_each_pane_of_the_scenario` is `#[ignore]` and its reason names both issues. Delivery is proven at the port instead: the scenario's `send_prompt` (the Claude inbox holds the message), conformance case 9 (on the fake and the router), `hermetic_test::a_prompt_reaches_the_socket_the_launch_hook_reported_as_one_framed_line` (its frame mutant is killed) and `lifecycle::reset_first_reaches_the_new_process` (its W4 mutant is killed) | Deferred (the PR is "Part of #831") |
| 4 | Interrupt one | `lifecycle::interrupt_relaunches_a_busy_pane_on_its_session_of_record`: `stop_owned` before `run`, no `SendText` or `SendKeys`, a new pid, the session unchanged. Also the scenario, `interrupt_with_text_on_a_claude_pane_is_refused_and_touches_nothing`, `interrupt_of_a_parked_claude_pane_is_refused_by_the_pane_gate`, `interrupt_of_an_opencode_pane_still_goes_to_the_hub` and `abort_of_a_claude_process_is_abort_unsupported` | Met |
| 5 | Park one | The scenario parks the OpenCode pane. `lifecycle::park_holds_a_claude_pane_and_leaves_its_process_alone` checks that no host or Claude call is made | Met |
| 6 | `doctor` clean | The scenario, after the interrupt and the park, and `doctor::doctor_is_clean_for_a_healthy_idle_claude_pane_beside_an_opencode_pane`. Beside them: a `/clear` is a fixable mismatch; `--fix` relaunches an idle pane and leaves a busy one alone; a gone process is `server-down`; a wait is `pane-waiting`; and a missing login is reported once per pass | Met |
| 7 | The subscription login is never read or printed | `hermetic_test::login_status_reads_the_auth_method_alone` (its fixture carries an email and an org, and only `authMethod` decides), `an_unreadable_login_answer_quotes_none_of_it` and `every_login_variable_is_unset_before_the_program_and_none_is_set`. `launch::a_login_variable_in_the_specs_env_is_refused_by_its_name` covers all 7 names. `doctor::a_missing_login_is_one_problem_per_pass` checks that the finding never says "auth status". In the code, `seams.rs` drops stderr, and no error quotes the program's output (`lib.rs:349-362`, `seams.rs:41-75`) | Met |
| 8 | The roster, the watchdog and `doctor` see the Claude pane | `doctor`: rows 6 and 2. No roster or watchdog reads the registry yet, and the brief says "where they read the registry": `holler roster` reads the hub's body roster (`roster_cmd.rs`), and its pane view is #648 (OPEN); no watchdog code exists in this repository (pfleet#396 will read `holler pane get/list --format=json`). `pane list --format=json` shows the Claude pane (row 2) | Met where a registry reader exists; the roster waits for #648 |
| 9 | Goal: `--harness` on `launch` and `relaunch` | `spec_flags.rs` (2 tests), `launch::an_unknown_harness_is_usage`, `cli_surface_test` (the 2 RED lines) and `lifecycle::relaunch_naming_another_harness_is_harness_change_refused` | Met |
| 10 | Goal: the same verbs (list, get, watch, switch, reset, park, say, interrupt, close, doctor) | `list`, `switch`, `reset`, `park`, `interrupt`, `close` and `doctor` are pinned in `lifecycle.rs`, `doctor.rs` and `mixed.rs`; `say` is row 3. `get` and `watch` read the record and call no harness, and the record's form is pinned (`claude_kind_test::a_claude_record_has_a_pid_and_no_port`), but no test runs them over a Claude record (Advisory 4) | Met, with a test gap |
| 11 | Work 1: the variant; `HARNESS_IDS` with a test that pins both lists; `Pane` and `ProfileSpec`; ADR-0021 and the epic's block | `rework_test.rs::every_kind` with its vocabulary check, `claude_kind_test` (15 tests), ADR-0021 section 15, and the #633 comment. The `HARNESS_IDS` doc comment that item 1 promises is missing (REWORK 1) | Met, except REWORK 1 |
| 12 | Work 2: the `HarnessPort` impl in a new adapter crate, behind the same seam; the host starts and stops the process "by the recorded pid" | The crate `holler-adapter-claude`, a dependency of `holler-cli` only; `hermetic_test` (15 tests); the conformance suite over the fake and the router. The host stops the process with `stop_owned`, by tmux session, as section 15 decides (its opening and item 6). This differs from the issue's words, and the PR body must say so (A's W15; Advisory 2) | Met, as amended |
| 13 | Work 3: a fake and its conformance suite; the real-tool test opt-in and `#[ignore]` | `fake_claude_test` (13), `claude_conformance_test` (4 tests, 12 mutants), `fake_harnesses_test` (5), and `real_claude_test`, which is `#[ignore]` and runs only with `HOLLER_TEST_CLAUDE=1` | Met. The real test was not run (Advisory 1) |
| 14 | Work 4: `profile show` and `apply` compare the harness; doctor codes for a harness that cannot be reached | `mixed.rs::profile_show_compares_the_harness_kind` and `apply_never_changes_a_live_panes_harness`; `doctor::a_claude_process_that_is_gone_is_server_down`, the login tests and `pane-waiting` | Met |
| 15 | Work 5: the wizard (#803) | Not in this story: section 15 item 16 decides so, and #803 is open | Out of scope, by decision |
| 16 | The operator's abort decision, recorded in ADR-0021 and in the research note's "Decisions" | Section 15's opening and decision 22; the note's "Decisions" section. `claude_kind_test::a_kind_knows_its_shape_and_its_abort_style`, `abort-unsupported`, and no keystroke in any lifecycle test | Met |
| 17 | Adopt the existing Claude Code panes: decide, build, record | Decided and recorded (item 17, decision 23). Not built: #835 is not on `main` (OPEN at 21:30 MDT), and the brief's rule then stops the story with a "Part of #831" PR | Deferred, by the brief's rule |
| 18 | The user's question: does the `say` step need #839? Record the answer | Item 18 (it needs #839, and #850 too), decision 24 and the #633 comment | Met |
| 19 | Coordinate with #835: one shared constant and one signal | `CLAUDE_CODE_COMMAND`, `is_claude_code_command` and the required `HostPort::foreground_commands`, with the same API and host code as `issue-835-implementation` (diffed at `f1d1bd0`). A second spelling of the name remains (REWORK 2), and the record of the copy is inaccurate (REWORK 3) | Met in code; REWORK 2 and 3 |

## Spec compliance

Section 15, item by item, against the code:

- **Item 1, the kind.** Built: `HarnessKind::Claude`, `shape()` and `abort_style()` (`pane.rs:126-222`). The test that pins
  every kind to `HARNESS_IDS` covers the new variant. **Not built:** "the doc comment of `HARNESS_IDS` says so"
  (`ADR-0021.md:1050-1053`). `holler-proto/src/vocab.rs:269-271` is unchanged and records none of the id's two uses. No
  as-built note says why (**REWORK 1**).
- **Item 2, one name and one signal.** Built: #835's API. **Two deviations:**
  - `wiring/env.rs:107` spells `"claude"` a second time (**REWORK 2**).
  - The as-built sentence "text for text, so that whichever of the two lands second merges identical hunks"
    (`ADR-0021.md:1064-1067`) is not true (**REWORK 3**).
- **Item 3, the endpoint.** Built as decided:
  - `HarnessEndpoint` (`ports/process.rs`) keys every call that took a port.
  - `HarnessInfo.port` is `Option<u16>`, and `#[serde(try_from = "RawHarnessInfo")]` refuses a record that does not fit its
    kind. An OpenCode record keeps its bytes.
  - `HarnessInfo::endpoint` is the one place a record becomes an endpoint. I grepped every `HarnessEndpoint` construction:
    the others start from a located pid, a plan's port or the launcher's port.
- **Item 4, the router.** One `HarnessRouter` (`ports/router.rs`). The wiring builds it over the two `Wired` adapters, and
  `FakeHarnesses` wraps it. As built, it routes by call; that is recorded.
- **Items 5 and 6, the process methods and the relaunch flows.** Built as recorded:
  - `SessionChoice` carries the verb's intent: `Resume` is `session-not-found` in the plan, before anything stops.
  - All four defaults refuse `process-unsupported`.
  - One sequence, `relaunch_on` and `relaunch_recorded`, serves switch, reset, interrupt and doctor's repair.
  - The plan runs `process_command` first. `process-in-use` is checked for a launch only.
  - The wait is `wait_until`, until the process is located and idle. Rollback is `stop_owned`.
  - `reset --first` goes to the stored record's endpoint.
  - Doctor's repair runs only while the pane is idle, through the caller's relaunch, and writes no record afterwards.
    Reconcile does not import `tx_launch`.
- **Items 7 and 8, delivery and activity.** Built as recorded:
  - The hook is fixed text, and its data reaches it only through the environment. It writes `<dir>/$PPID` atomically
    under `umask 077`.
  - The reader fails closed: the directory and the file must be the user's alone, neither may be a link, and the file
    must hold one absolute path. It trusts the file only for a listed pid and never reads `sessions/*.json`.
  - The frame is one line. The held window is 600 ms.
  - Activity: a `waiting` on `dialog open` is `harness-dialog-open`, and an unknown status is `unavailable`.
- **Item 9, abort, interrupt and answer.** Built as recorded:
  - `abort` is `abort-unsupported`.
  - `interrupt_pane` applies the pane gates, then routes by `abort_style()`. With text, it is
    `interrupt-text-unsupported`.
  - `answer` is `answer-unsupported`.
- **Items 10 and 11, the login and the launch.** Built as recorded:
  - `HarnessKind::login_variables()` is read both by the plan's `claude-login-env-refused` and by the adapter's
    `env -u`. The adapter's own runs remove the same names.
  - `login_status` reads `authMethod` alone.
  - Every launch carries `--model`, `--effort` and `--disallowed-tools AskUserQuestion`. An agent is `agent-unsupported`.
- **Item 12, the flag and the policy.** Built as recorded:
  - `--harness` is in `SpecFlags` (`pane/args.rs`).
  - `port_for` is the one policy rule: `usage` for the grammar, `port-policy-unsupported` for the kind.
  - `port_policy_of` gives `none` for a Claude record.
  - `profile apply` refuses a change of harness before it judges health.
- **Items 13 and 14, doctor and the codes.** Built as recorded:
  - `pane-waiting` and `claude-login-missing` are appended, with no remedy, and the login is asked once per pass and only
    with a single process in scope.
  - Each open code is declared where it is raised: `ports/process.rs`, `tx_launch/{rules,process}.rs`, `tx_switch.rs`,
    `say_cmd.rs` and `agents.rs`.
- **Item 15, the test kit.** Built as recorded.
- **Items 16 and 17.** Not built, as decided.
- **Item 18.** Recorded, and the ignored test names #839 and #850.

**The plan review's warns.** W1 to W14 are implemented. T's tests pin W1 to W5 and W9 to W13, and A-dup found none of the
ten parallel paths. Of W15:

- #840's `unmanaged.rs` sentences are corrected;
- `docs/pane-control.md` documents the flag, the policy, the relaunch-based verbs, `say` waiting for #839 and #850, and the
  adoption;
- the `stop_owned` wording is for the PR body (Advisory 2).

**The relayed user request.**

- The amendment came first: `008a995`, before any test or code.
- The abort decision is recorded in both places.
- The pipeline ran at `in-session` rigor, in its own worktree.
- The answer about `say` and #839 is in the amendment.
- The #835 coordination holds in code (row 19).

## Quality audit

- **Correctness and failure handling.** I found no defect in behaviour.
  - **Fail-closed reads:**
    - a record that does not fit its kind does not load, and one built in code that does not fit is `store-corrupt`;
    - a missing or stale socket file is `unavailable`, and no path is guessed;
    - an unreadable login answer is `unavailable`, never `Ok`;
    - an unknown status is never idle.
  - **Refusals before anything stops:**
    - a switch target with no conversation is refused in the plan;
    - a launch into a session that already runs Claude Code is `process-in-use`;
    - a busy pane or one waiting on a permission is refused by `switch`, `reset` and the repair.
  - **Writes and races:**
    - every write is one compare-and-swap at the generation read;
    - a relaunch that finds the old process still running fails loudly and adopts nothing;
    - the repair skips the chain's own write after it relaunches, so no write races the relaunch's.
  - Several edge cases fail closed: pid reuse, a person's `/clear` and a SIGKILL that leaves files behind.
  - One concurrency note is in Advisory 8.
- **Build guards.**
  - The added lines have no `unwrap`, `expect` or `panic!` outside tests. The one exception is the test kit's
    `constant_name` (`conformance/claude.rs:116`), with `#[allow(clippy::expect_used)] // #831`.
  - Every `#[allow]` carries a link.
  - The largest touched `.rs` file is `pane_verbs/switch.rs` at 893 lines, and `tx_launch.rs` is down to 747. All are under
    900.
  - T-green ran clippy with `-D warnings` and `cargo machete`, both clean: no dead code.
- **Protocol.** No wire change:
  - `docs/protocol/v2.md` and the golden files are untouched;
  - the CLI envelope is unchanged;
  - the record's `harness.port` becomes optional, and it is absent only on a `claude` record.
- **Tests.** The default run uses fakes only: no real Claude Code, login, Herdr session or fleet. There are no fixed sleeps:
  every wait is a bounded poll with a deadline (`wait_until`, the suite's `LOCATE_WITHIN` and `SETTLE_WITHIN`, and host case
  10's `SETTLE`). The RED-first evidence and the mutants are listed under the T precondition. Gaps are in Advisory 4.
- **Documentation.**
  - `CHANGELOG.md` has an `## [Unreleased]` entry linking #831, and `changelog-check` passes.
  - ADR-0021 is updated: sections 1, 2, 8, 9, 14 and 15, "Deferred", and decisions 22 to 28.
  - ADR-0003 has the amendment line, the rows and the SPEC FLAGS.
  - `docs/pane-control.md` and `docs/orchestrating.md` are updated, and `cli-surface.txt` has its two lines.
  - The new `HOLLER_CLAUDE_BIN` is in the wiring's env table and in the guide.
  - The research note's "Decisions" is updated.
  - REWORK 1 to 3 and Advisory 5 correct statements in these docs.
- **Public-repository privacy.** I grepped all added lines (about 12,000). None holds a personal host, a tailnet name, an IP
  (only `127.0.0.1`), a machine or account name, a private domain, a home path or a secret. The names are neutral:
  `demo-c1r1`, `/srv/demo`, `scratch` and `example.invalid`.
- **Commit and PR hygiene.**
  - The six subjects are Conventional, and each commit carries `Co-Authored-By`.
  - None carries a session link. That matches the workflow driver's commits already on `main` (`0f5835e`, `d6ae539`), and
    the squash at merge writes the final message.
  - The PR is not open yet: its AI disclosure and its "Part of #831" are for that step (Advisory 2).

## Scope check

- **Delivered:** the brief's scope, less two items. Item 17 waits for #835 and item 16 is #803's; the brief and section 15
  decide both.
- **No unrelated change.** Everything beyond the minimum is sanctioned or needed:
  - the `tx_launch.rs` split into five siblings (W14);
  - #835's API and host code, copied (the brief's coordination rule);
  - `server-unsupported`, `session-holds-permission` and `wait_until`, each recorded as built;
  - a generalised `say_cmd::resolve_pane_record`.
- **Two OpenCode message wordings changed** so that they also fit the single process:
  - `session-busy`: "reset it once it ends" became "try again once it ends";
  - `profile apply`'s unhealthy step: "its harness server on port N" became "its harness at port N".
  
  No test asserted either, and the codes are unchanged. Acceptable.

## Verdict

**REWORK** (`reworkKind: production`: one production literal and two doc statements; no test change is needed). F fixes all
three in one pass. T-green then re-runs clippy, `docs_cli_test`, `docs_pane_control_test` and `cargo test -p holler-proto`.

1. **`crates/holler-proto/src/vocab.rs:269-271`, the doc comment of `HARNESS_IDS`.**
   - **What section 15 decides** (item 1, `ADR-0021.md:1050-1053`): "The id has two uses that do not meet: a **pane**
     harness `claude` is the interactive CLI and is read only by the pane engine; a **body** harness `claude` (ADR 0012) is
     the ACP bridge and is read only by `holler-body`. The doc comment of `HARNESS_IDS` says so."
   - **What the code says.** The comment is unchanged. It still reads "Only `opencode` is a v2 adapter; the rest are
     reserved for future adapters", and no as-built note records the omission.
   - **Change:** say what item 1 says, naming `holler_pane::pane::HarnessKind` for the pane harness and
     `holler-body`/ADR 0012 for the body harness, and that a test pins every `HarnessKind` serde name to this list. Reword
     "the rest are reserved" so that `claude` does not read as only reserved.
   - This is a doc comment only: no wire or golden change.
2. **`crates/holler-cli/src/pane/wiring/env.rs:107`.**
   - **The defect.** `program_bin(env, CLAUDE_BIN, "claude")` spells Claude Code's command name a second time. Item 2
     (`ADR-0021.md:1057-1059`) makes `holler_pane::pane::CLAUDE_CODE_COMMAND` "the one place the name is written".
   - **Why it is the same fact.** The default program's basename is the foreground command name that `process-in-use`
     (W11) and #835's import rule compare. A-dup raised this as W7.
   - **Change:** pass `CLAUDE_CODE_COMMAND`.
3. **`docs/adr/ADR-0021.md:1064-1067`, the "As built" of section 15 item 2.**
   - **The claim:** "text for text, so that whichever of the two lands second merges identical hunks". It is not true.
   - **What is identical** to `issue-835-implementation` (I diffed it at `f1d1bd0`; A-dup at `d71b091`): the code. That is
     the constant and the rule, the required method's signature, `TmuxHost`'s and `FakeHost`'s implementations, host suite
     case 10 and the forwards.
   - **What differs:**
     - the doc comments of `CLAUDE_CODE_COMMAND` and `is_claude_code_command` (`pane.rs:225-244`);
     - the doc comment of `HostPort::foreground_commands` (`ports.rs:330-339`);
     - the constant's place, after `impl HarnessKind`.

     #835 has since rewritten its own doc comments again.
   - **Change:** state what holds. The API and the host-side code are #835's, unchanged. Whichever story lands second keeps
     the other's API and resolves a conflict in the doc comments of `pane.rs` and `ports.rs`. The operator asked for this
     coordination, so its record must be accurate (A-dup W8).

## Advisory notes

None blocks the merge.

1. **The opt-in real test was not run.** The issue makes it opt-in and `#[ignore]`, so it is not a merge condition. Run it
   before #831 closes, and before any Claude Code pane is launched on the fleet:

   ```text
   HOLLER_TEST_CLAUDE=1 HOLLER_TEST_CLAUDE_CWD=<a directory Claude Code already trusts> \
     cargo test -p holler-adapter-claude --test real_claude_test -- --ignored --test-threads=1
   ```

   Still unverified against the real binary, each failing closed:
   - the hook's `$PPID` key (if it is wrong, `send_prompt` is `unavailable`);
   - the inbox frame;
   - the launch through tmux;
   - whether `claude agents --json` drops a SIGKILLed process (if it does not, a relaunch's old-process check fails
     loudly).
2. **For the PR body:**
   - say "Part of #831";
   - list what remains: item 17 after #835; the `say` step after #839 and #850; the roster line with #648; the real test's
     first run;
   - list the as-built edits to the amendment (items 2, 4 to 10, 12 to 15, 17 and 18; decisions 26 to 28);
   - name the `stop_owned` difference from the issue's "by the recorded pid";
   - add the AI disclosure that `CONTRIBUTING.md` requires, with `gh pr edit` (the repo's `CLAUDE.md`).
3. **Merge `origin/main`** (`0f5835e`, #855) before the PR. A-dup's trial merge conflicts in `CHANGELOG.md` only.
4. **Test gaps.** These are test-only and for T-green in this loop, since it runs anyway; each is one row or one assertion.
   - **`harness-dialog-open` is pinned by no test.** F's and T-green's handoffs say the adapter's hermetic mapping pins it,
     but `activity_maps_status_and_waiting_for` has no `dialog open` row and no unknown-status row. Add both:
     `harness-dialog-open`, and `unavailable`.
   - **`session-holds-permission`.** `lifecycle::switch_refuses_a_busy_pane_and_one_waiting_on_a_permission` asserts only
     exit 3 for the permission case. Assert the code too.
   - **Lower value:**
     - `pane get` (text `port=-`, and JSON) and `pane watch` over a Claude record;
     - the CLI output of `holler interrupt --pane` for a Claude pane: `interrupt_cmd::route` and `relaunched`, and the line
       that ADR item 9 records;
     - `server-unsupported`.
5. **Two docs overstate.**
   - `harnesses.rs:9` says "No rig writes a linker of its own", but the three OpenCode-only rigs keep their own (A-dup W6).
   - `docs/pane-control.md:67` and `CHANGELOG.md:16` say that the launch gives every Claude Code pane the subscription
     login. In fact the launch removes the overriding variables, and whether the login is the subscription's is what
     `claude-login-missing` checks.
6. **#828 is closed** (not planned, 15:56 MDT; #839 absorbs it). Section 15 item 18's "As built", and older ADR-0021 and
   CHANGELOG lines, still name #828 as the story that moves `reset --first` to the hub's prompt path. A docs sweep can fix
   them together. `reset_first_reaches_the_new_process` now follows #839.
7. **`HOLLER_CLAUDE_BIN` with another basename** defeats `process-in-use` and #835's import rule, because tmux names the
   foreground process by its `argv[0]`. One sentence in `docs/pane-control.md` would say so.
8. **`doctor --fix` can relaunch several Claude Code panes at once**, one per pane thread. The wiring's `Bindings` doc says
   "a wiring runs one launch at a time". Today this is harmless: only OpenCode's TUI resolver reads the bindings, and the
   Claude relaunches never call it. Update that doc line.
9. **Rollback.** A pre-#831 binary cannot load a registry that holds a `claude` record: `port` is missing and the kind is
   unknown, so the registry fails closed by design. Close the Claude Code panes before a downgrade, and section 11 of
   `docs/pane-control.md` could say so.
10. **A-dup's remaining warns are follow-ups:**
    - W1: `parse_harness` repeats `harness_kind`, and `as_str` is not pinned to the serde names;
    - W2: the chain skeleton;
    - W3: the triple unwrap of `Launched.pane`;
    - W4: a generic `route_target_by`;
    - W5: `exec.rs` belongs to #696;
    - W9: `switch.rs` is at 893 lines and `launch/rig.rs` at 852.
