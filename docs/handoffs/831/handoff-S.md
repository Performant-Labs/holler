# Handoff-S: #831 - A Claude Code harness for panes  (spec audit)

Three passes, newest first: the audit after T's test-only rework (the second REWORK), the audit after F's rework (the
first REWORK), then the first pass.

## Rework 2: the audit after T's test-only rework (2026-10-11, 23:05 MDT)

**Date:** 2026-10-11, 23:05 MDT
**Branch:** `issue-831-implementation` at `dc9de0f` (worktree `.claude/worktrees/0831-claude-harness`). `git fetch` at 23:04
MDT: `origin/main` is still `037405f`, which is the merge base. No PR is open.
**Issue:** [#831](https://github.com/Performant-Labs/holler/issues/831), OPEN, with no comments (read live with
`gh issue view 831`).
**Brief:** `docs/handoffs/831-brief.md`, with ADR-0021 section 15 as the normative text.
**Handoffs reviewed:** `handoff-T-green.md` ("Rework 2"), `handoff-A-dup.md` ("Rework 2"), this cycle's entries in
`decisions.md` and `evidence.md`, and this file's two earlier passes; `handoff-A.md`, `handoff-T-red.md` and `handoff-F.md`
from the earlier cycles.
**Diff audited:** `git diff origin/main...HEAD`: 122 files, +13834/-1311 (113 files and +10624/-1311 outside
`docs/handoffs/`).
- **Since the last S verdict (`a1ef88a`):** two test files (`roster_panes/registry.rs` +82, `pane_claude/launch.rs` +23) and
  four handoffs. `git diff a1ef88a HEAD -- 'crates/*/src'` is empty.
- **Beyond that delta,** I re-read the production code against section 15 myself rather than lean on the earlier passes:
  - the engine: `pane.rs` (the kind, the record's fit, the constant), `tx_launch.rs` with `process.rs`, `rules.rs` and
    `undo.rs`, `tx_switch.rs`, and `reconcile.rs` with `reconcile/observe/process.rs` and `findings.rs`;
  - `ports/router.rs`, against every method of the `HarnessPort` trait;
  - the adapter crate: `lib.rs`, `agents.rs`, `argv.rs`, `inbox.rs` and `seams.rs`;
  - the wiring (`wiring.rs`, `env.rs`, `bind.rs`) and the prompt verbs (`say_cmd.rs`, `interrupt_cmd.rs`,
    `answer_cmd.rs`);
  - `pane/{args,launch,get}.rs`, the import's `files.rs`, and the docs.

**Verdict: PASS.** The second REWORK's one required item is done as asked, and so is its optional item. No production code
changed in this cycle, and every line the earlier passes found met is still met. Every acceptance line now has a proving
test, or is deferred by the brief's own design (the `say` step: #839 and #850) or by its rule (item 17: #835 is not on
`main`), so the PR is "Part of #831".

### A precondition

Met.

- `handoff-A.md`: **PASS**, with 0 block and 15 warn.
- `handoff-A-dup.md`, "Rework 2": **PASS**, with 0 block and no new warn. Six warns are carried: W1 (narrowed) to W5, and
  W9.

### T precondition

Met.

- **RED** at `b3f0295`: compile-level on the contract, and assertion-level on `cli_surface_test`'s two `--harness` lines.
- **GREEN** at `f0a3f1a` (T-green, "Rework 2"):
  - the pipeline's test command, `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`:
    2526 passed, 0 failed, 36 ignored (Rework 1's 2524, plus the two new tests);
  - clippy with `-D warnings`, `lint.sh`, `changelog-check.sh` and `cargo machete` all pass;
  - 0 blocking issues.
- **The two new tests assert behaviour.** T-green applied a mutant for each and reverted it, and `crates/*/src` is clean:
  - the roster test: filtering `HarnessKind::Claude` records out of `Registered::of` turns the Claude row's six cells to
    `-`. It is the only one of the 26 roster tests that fails;
  - the `pane get` test: dropping `pid=` from `get.rs`'s `harness_text` fails it.
- **My checks were read-only:**
  - `git fetch`, `diff`, `show`, `log`, `merge-base`, `grep`, `diff --check` (clean) and `wc -l`;
  - `changelog-check.sh` (ok);
  - `gh issue view` of 831, 835, 839, 850, 696 and 803 (with the comments of 835, 839 and 803), and `gh pr list`.

  I re-ran no build or test.

### The second REWORK, checked

| # | Asked | On `dc9de0f` | |
|---|---|---|---|
| 1 | Required: the roster reads a registry that holds a Claude Code record and prints that record's cells on the hub row joined to it, in text and `--format=json`, with an empty stderr and the base rows unaffected | `roster_panes/registry.rs:51-129`, `a_claude_pane_record_fills_its_row_as_pane_list_prints_it`, beside `the_cells_are_pane_lists_cells_for_the_same_record` and reusing its `pane_list_cells`. The fixture is the one asked for: `base_panes()` plus `demo-c1r2` with `HarnessInfo { kind: Claude, port: None, pid: Some(4242), health: Healthy }`, a session of record, SHOWN equal to it and no DRIVEN; the reply is `BASE_ROWS` with `demo-c1r2/ses-claude` inserted in name order. It asserts exit 0 and an empty stderr in both modes, the six cells against `pane_list_cells`, the six JSON keys against `PaneRow::from` serialized, `mismatches` `[]`, and every base row equal to a render without the Claude record. I checked the harness it calls: `render` is the pure `roster_cmd::render` over the fakes, and the reply keeps its order, so `lines[3]` is the Claude row. The fixture sits at `r3c1` because `hj-c1r2` holds `r2c1`. `evidence.md`'s excerpt matches `roster_cmd.rs:262-267`, which this branch does not change | Fixed |
| opt. | `pane get` of a Claude pane: the text line and the JSON | `pane_claude/launch.rs:258-277`, `pane_get_of_a_claude_pane_shows_its_pid_and_no_port`: the whole line `harness: claude port=- pid=<pid>`, and in JSON `kind` `claude`, the record's pid and no `port` key | Done |

### Acceptance criteria

The issue's Acceptance, read as the brief's item 18 says it can be met, then the Goal and the Work list. Against Rework 1's
table, rows 8 and 10 change.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | One OpenCode and one Claude Code pane from one profile | `pane_claude/mixed.rs::one_profile_launches_an_opencode_pane_and_a_claude_pane`; `a_second_apply_of_the_mixed_grid_changes_nothing` | Met |
| 2 | Both in `pane list` with SHOWN and DRIVEN | `mixed.rs::the_acceptance_scenario_on_the_fakes`: each row's `shown` and `driven` are its record's, and `sync` is `ok` | Met, as the brief reads it |
| 3 | A message to each through `holler say` | Blocked by #839 and #850, by the brief's design: `mixed.rs::say_delivers_a_message_to_each_pane_of_the_scenario` is `#[ignore]` and its reason names both. Delivery is proven at the port: the scenario's `send_prompt`, conformance case 9, `hermetic_test::a_prompt_reaches_the_socket_the_launch_hook_reported_as_one_framed_line` and `lifecycle::reset_first_reaches_the_new_process` | Deferred (the PR is "Part of #831") |
| 4 | Interrupt one | `lifecycle::interrupt_relaunches_a_busy_pane_on_its_session_of_record` and the scenario; the refusals `interrupt_with_text_on_a_claude_pane_is_refused_and_touches_nothing`, `interrupt_of_a_parked_claude_pane_is_refused_by_the_pane_gate` and `abort_of_a_claude_process_is_abort_unsupported`; `interrupt_of_an_opencode_pane_still_goes_to_the_hub` | Met |
| 5 | Park one | The scenario; `lifecycle::park_holds_a_claude_pane_and_leaves_its_process_alone` | Met |
| 6 | `doctor` clean | The scenario, after the interrupt and the park; `doctor::doctor_is_clean_for_a_healthy_idle_claude_pane_beside_an_opencode_pane` | Met |
| 7 | The login is never read or printed | `hermetic_test::login_status_reads_the_auth_method_alone`, `an_unreadable_login_answer_quotes_none_of_it` and `every_login_variable_is_unset_before_the_program_and_none_is_set`; `launch::a_login_variable_in_the_specs_env_is_refused_by_its_name` (all 7 names); `doctor::a_missing_login_is_one_problem_per_pass`. In the code: `exec.rs` sends stderr to `Stdio::null()`, and no error quotes the program's output | Met |
| 8 | The roster, the watchdog and `doctor` see the Claude pane | **`doctor`:** the 8 tests of `pane_claude/doctor.rs`. **The watchdog:** none in this repository; pfleet#396 will read `holler pane get/list --format=json`, and both now show the Claude pane: `pane list` (row 2) and `pane get` (`launch.rs::pane_get_of_a_claude_pane_shows_its_pid_and_no_port`). **The roster:** `roster_panes/registry.rs::a_claude_pane_record_fills_its_row_as_pane_list_prints_it`. Whether a live Claude pane gets a hub row is #850's (section 15 item 18) | Met |
| 9 | Goal: `--harness` on `launch` and `relaunch` | `spec_flags.rs` (2 tests), `launch::an_unknown_harness_is_usage`, `cli_surface_test`, `lifecycle::relaunch_naming_another_harness_is_harness_change_refused` | Met |
| 10 | Goal: the same verbs | `list`, `get`, `switch`, `reset`, `park`, `interrupt`, `close` and `doctor` in `lifecycle.rs`, `doctor.rs`, `mixed.rs` and `launch.rs`; `say` is row 3. `watch` is not run over a Claude record (Advisory 6) | Met, with one low-value gap |
| 11 | Work 1: the variant, `HARNESS_IDS` and the test that pins both lists, `Pane` and `ProfileSpec`, ADR-0021, the epic's block | `rework_test.rs::every_harness_kind_serde_name_is_in_the_protocol_vocabulary` (its `every_kind` lists `Claude` through an exhaustive match), `claude_kind_test` (15 tests), section 15, the #633 comment, and the doc comment of `HARNESS_IDS` | Met |
| 12 | Work 2: the adapter crate behind the same seam; the host starts and stops the process | `holler-adapter-claude`: only `holler-cli`'s `Cargo.toml` names it, and `holler-hub` names no adapter. `hermetic_test` (16 tests); the conformance suite over the fake and the router. The host stops the process by tmux session (`stop_owned`), as section 15 decides | Met, as amended (PR body: Advisory 2) |
| 13 | Work 3: a fake and its conformance suite; the real test opt-in and `#[ignore]` | `fake_claude_test` (13), `claude_conformance_test` (4 tests, 12 mutants), `fake_harnesses_test` (5), `real_claude_test` (`#[ignore]`, and it returns at once unless `HOLLER_TEST_CLAUDE=1`) | Met. The real test was not run (Advisory 1) |
| 14 | Work 4: `profile show` and `apply` compare the harness; doctor codes | `mixed.rs::profile_show_compares_the_harness_kind` and `apply_never_changes_a_live_panes_harness`; `doctor::a_claude_process_that_is_gone_is_server_down`, the login tests and the `pane-waiting` test | Met |
| 15 | Work 5: the wizard (#803) | Section 15 item 16: not in this story. #803 is OPEN and, by its comment at 21:15 MDT, parked until #831 merges | Out of scope, by decision |
| 16 | The operator's abort decision, recorded in ADR-0021 and the note's "Decisions" | Section 15's opening and decision 22; the note's "Decisions" section; `claude_kind_test::a_kind_knows_its_shape_and_its_abort_style`; no keystroke in any lifecycle test | Met |
| 17 | Adopt the existing Claude Code panes: decide, build, record | Decided and recorded (item 17, decision 23). Not built: #835 is OPEN and parked (local branch at `f1d1bd0`, no PR, not on `main` at 23:04 MDT) | Deferred, by the brief's rule |
| 18 | The relayed question: does the `say` step need #839? | Item 18 (it does, and #850 too), decision 24 and the #633 comment | Met |
| 19 | One constant and one signal with #835 | `CLAUDE_CODE_COMMAND`, `is_claude_code_command` and the required `HostPort::foreground_commands`, identical in code to #835's branch at `f1d1bd0`; #835's names, not the brief's `CLAUDE_COMMAND`, by A's W6 (journalled in `decisions.md`, recorded in item 2 as built); `claude_bin` uses the constant | Met (Advisory 3) |

### Spec compliance

Section 15 is built as recorded, items 1 to 18. This cycle changed no behaviour. What I checked again in the code myself:

- **Items 1 to 3.**
  - `HarnessKind::Claude` with `as_str`, `shape()`, `abort_style()` and `login_variables()`.
  - `HarnessInfo` loads only through `try_from = "RawHarnessInfo"` and `misfit`: `opencode` needs a port, and `claude`
    needs a pid and no port. `endpoint()` is `store-corrupt` for anything else.
- **Item 4.** `HarnessRouter` forwards all 15 `HarnessPort` methods, so no default answers in place of the routed harness.
  The wiring builds `HarnessRouter<WiredHarness, WiredHarness>`. The decorators gain forwards only, each pinned by
  `pane_integration/bind.rs`'s `call_every_method`.
- **Items 5 and 6.**
  - `Plan::new` runs `port_for`, then `refuse_login_env`.
  - `observe` builds the argv first (`process_command`, with `new_session_id` for a fresh choice), then the probe, Herdr,
    the cell, and `process-in-use` for a launch only.
  - `stop_old` stops what the pane owns, checks the old endpoint's `health`, and adopts nothing.
  - `process::bring_up` waits through `wait_until` until the process is located and idle, then checks that it holds the
    planned session.
  - `relaunch_recorded` is the one sequence for `switch`, `reset`, `interrupt` and doctor's repair.
  - `tx_switch` applies the idle gate (`session-holds-permission`) and `check_unclaimed` to a single process's switch, and
    sends `--first` to the endpoint of the stored record.
- **Items 7, 8 and 10, the adapter.**
  - The hook is fixed text.
  - `socket_of` fails closed: the directory and the file must be the user's alone and not links, and the file must hold
    one absolute path. `send_prompt` trusts it only for a pid that `agents --json` lists now.
  - The frame is one JSON line.
  - The held window is 600 ms and ends early on `busy`.
  - `dialog open` is `harness-dialog-open`, and an unknown status is `unavailable`.
  - `login_status` reads `authMethod` alone.
  - Both `claude` runs remove the 7 variables, and so does the launch argv's `env -u`.
- **Items 9 and 12 to 14.**
  - `interrupt_pane` routes by `abort_style()` after `resolve_pane_record`'s gates.
  - `answer_pane_target` refuses a single process.
  - `say --pane` keeps the hub's routing and gets no CLI-side path.
  - `--harness` is in `SpecFlags`, and `port_for` is the one policy rule.
  - `check_harness_unchanged` serves E0 and `profile apply`.
  - The login is asked once per pass, only with a single process in scope. `pane-waiting` and `claude-login-missing` have
    no remedy.
- **A hazard the earlier passes did not name, which is covered.** `claude` is now a valid `HarnessKind`, so the import's
  `not_opencode` compares with `Some(HarnessKind::Opencode)` (plan review W7). `import/join.rs`'s
  `a_pane_that_is_not_opencode_is_skipped_and_the_others_import` uses a `claude` row, so it pins that check.

**The relayed user request:**

- the amendment came first (`008a995`, 18:11 MDT, with only ADR-0021, the brief and the research note);
- the abort decision is recorded in both places;
- the run is `in-session`, in its own worktree off `origin/main`;
- the answer about `say` and #839 is in the amendment;
- the #835 coordination holds in code, and its record is accurate.

### Quality audit

- **Correctness and failure handling.** No defect in behaviour, as in the earlier passes. This cycle adds tests only.
- **Build guards** (my own greps of the diff):
  - In the added `src` lines, the only `expect` is the test kit's `constant_name`, with `#[allow(clippy::expect_used)] //
    #831`. There is no `unwrap`, `panic!`, `unreachable!` or `todo!`.
  - All 13 added `#[allow]`s carry a link. `#![allow(dead_code)] // #831` is on `pane_claude/main.rs`, a shared test rig,
    as in the repo's other test binaries.
  - The largest touched `.rs` files are `pane_verbs/switch.rs` (893) and `launch/rig.rs` (852), and every other is under
    850. This cycle's two files are 318 and 277 lines.
  - Clippy with `-D warnings` and `cargo machete` are clean (T-green).
- **Protocol.** No file under `docs/protocol/`, and no golden file, is touched. `harness.port` is optional, and it is absent
  only on a `claude` record.
- **Tests.**
  - The default run uses fakes only.
  - Every wait is a bounded poll with a deadline: `wait_until`, the suite's `LOCATE_WITHIN` and `SETTLE_WITHIN`, and host
    case 10's `SETTLE`.
  - The opt-in real test uses a scratch tmux server and directory, and reads only `authMethod`.
  - The two new tests use no sleep.
- **Documentation.**
  - `CHANGELOG.md` has an `## [Unreleased]` entry that links #831, and `changelog-check` passes.
  - Updated: ADR-0021, ADR-0003 (the amendment line, the rows and SPEC FLAGS), `docs/pane-control.md`,
    `docs/orchestrating.md`, `cli-surface.txt` (its two lines) and the research note's "Decisions".
  - The CHANGELOG's and the guide's roster sentences (no row for a Claude Code pane until #850) still hold: the new test
    pins the join of a record, not a hub row that production does not yet have.
- **Public-repository privacy.** I grepped all 13,834 added lines, the handoffs included.
  - No personal host, tailnet name, IP other than `127.0.0.1`, machine or account name, home path or secret.
  - The only domains are `github.com` links to this repository and `claude.ai`, which is Claude Code's `authMethod`
    value.
  - The one email is `someone@example.invalid`.
- **Commit and PR hygiene.**
  - All 13 commit subjects are Conventional, and every commit carries `Co-Authored-By`.
  - None carries a session link (Advisory 5).
  - The PR is not open yet.

### Scope check

As in the earlier passes: the brief's scope, less item 17 (#835 is not on `main`) and item 16 (#803's). This cycle added
two tests and nothing else. Both are the ones S asked for, and neither adds a helper or a rig (A-dup).

### Verdict

**PASS.** It is ready for O. Nothing gates the PR. The PR step's items are in Advisory 2 and 3.

### Advisory notes

None blocks the merge.

1. **The opt-in real test has still not been run.** It is not a merge condition: the issue makes it opt-in and
   `#[ignore]`. Run it before #831 closes, and before a Claude Code pane is launched on the fleet. These are unverified
   against the real binary, and each fails closed:
   - the hook's `$PPID` key;
   - the inbox frame;
   - the launch through tmux;
   - `agents --json` after a SIGKILL;
   - which statuses `agents --json` reports while a process starts. The launch's wait takes an unknown status as
     `unavailable` and rolls back, rather than waiting on.
2. **For the PR body:**
   - say "Part of #831", and list what remains: item 17 after #835, the `say` step after #839 and #850, a live Claude
     pane's roster row with #850, and the real test's first run;
   - list the as-built edits to the amendment: items 2, 4 to 10, 12 to 15, 17 and 18, the "Not built here" paragraph, and
     decisions 26 to 28;
   - name the `stop_owned` difference from the issue's "by the recorded pid";
   - add the AI disclosure that `CONTRIBUTING.md` requires, with `gh pr edit`;
   - comment on #696, adding `holler-adapter-claude/src/exec.rs` to its scope (A-dup's W5);
   - list W1 to W4 and W9 as follow-ups.
3. **A stale spelling on #835's issue.**
   - The coordination comment on #835 (18:12 MDT) tells #835 that #831's constant is `CLAUDE_COMMAND`, as the amendment
     `008a995` and the brief's line 61 also said.
   - This branch built #835's own `CLAUDE_CODE_COMMAND` and `is_claude_code_command` (A's W6), so the two branches agree
     in code.
   - #835 is parked at `f1d1bd0`, so whoever resumes it could rename the constant on that comment's word. At the PR step,
     a comment on #835 should say that #831 adopted #835's names, and #835 keeps them.
   - The brief is the frozen input and needs no edit, because item 2 of section 15, the normative text, says what was
     built.
4. **Two sentences of section 15 lean on the roster** (carried).
   - Item 8's "and the roster can say which wait it is" should name `doctor` (`pane-waiting`).
   - Item 18 and "Not built here" say that #850 gives a Claude pane its roster row. That holds only if #850's body
     advertises a name the roster joins to the pane. A comment on #850 at the PR step would carry it.
5. **The commit trailers carry no session link** (carried). `CONTRIBUTING.md:19-21` describes one. The squash message and
   the PR body are written at the PR step.
6. **Lower-value test gaps** (carried, narrowed):
   - `pane watch` over a Claude record. Its text renders no harness field, and the record's serde form is pinned by
     `claude_kind_test`;
   - the CLI output of `holler interrupt --pane` for a Claude pane;
   - `server-unsupported`.

   `pane get` is now covered.
7. **The older #828 sentences** (carried), for example in ADR-0021's "Deferred" ("whether it moves to the hub's one prompt
   path is #828's"). They are other stories' words, and #839's issue says it edits them.
8. **A note for #839's next brief, not for this PR.** #839's parked comment (21:26 MDT) says this story's router adds a
   second `.send_prompt(` call (`router.rs:118`) that its planned scan must allow for. #839 re-cuts its brief after #831
   merges.
9. **Sizes and format** (carried).
   - `pane_verbs/switch.rs` is 893 lines and `launch/rig.rs` is 852. The next story that touches either splits it first.
   - `cargo fmt --check` fails on drift that predates this story. Neither CI nor `lint.sh` runs it.

---

## Rework 1: the audit after F's rework (2026-10-11, 22:40 MDT)

**Date:** 2026-10-11, 22:40 MDT
**Branch:** `issue-831-implementation` at `8aef610` (worktree `.claude/worktrees/0831-claude-harness`). F's phase commit
`7260b5e` merged `origin/main` at `037405f` (#855, #856), so the merge base is `origin/main` itself (`git fetch` at 22:22
MDT: still `037405f`).
**Issue:** [#831](https://github.com/Performant-Labs/holler/issues/831), OPEN, read live with `gh issue view 831`: the body
equals the brief's issue section, and it has no comments. #633's amendment comment is posted (18:12 MDT).
**Brief:** `docs/handoffs/831-brief.md`, with ADR-0021 section 15 as the normative text.
**Handoffs reviewed:** `handoff-F.md` ("Rework 1"), `handoff-T-green.md` ("Rework 1"), `handoff-A-dup.md` ("Rework 1"),
`decisions.md` and `evidence.md`; from the first pass, `handoff-A.md` and `handoff-T-red.md`.
**Diff audited:** `git diff origin/main...HEAD`: 121 files, +13096/-1310 (112 files and +10522 outside `docs/handoffs/`).
This cycle's own edits, against a trial merge of its two parents (`git merge-tree --write-tree ccec57c 037405f`, tree
`cc5793b`): one literal and three doc comments in `src`, T-green's two test files, `CHANGELOG.md`, ADR-0003 (the conflict
only), ADR-0021 and `docs/pane-control.md`. Beyond those and `main`'s own files (#856's `roster_cmd.rs`, `cli.rs` and
`main.rs`), no `crates/*/src` file changed since the first pass, which read the whole production diff. Against section 15
I re-read the engines (`tx_launch` and its five modules, `tx_switch`, `reconcile` and
the single-process chain), the port, the router, the adapter crate, the wiring and the prompt verbs, and the roster code
the merge brought in (`roster_cmd.rs`, #648).

**Verdict: REWORK** (`reworkKind: test-only`), one item. The first REWORK's three items are fixed as asked, and every line
the first pass found met is still met. What changed is the roster:

- #648 merged at 21:25 MDT and the branch now carries it, so `holler roster` reads the pane registry.
- The brief makes "the roster and the watchdog seeing the Claude pane where they read the registry" a test.
- Section 15 item 18 now says, as built, that the roster "reads the registry and prints a pane's cells on the hub row
  joined to it".
- No test puts a Claude Code record through the roster. F listed the test for T, and T-green left it out on a premise
  that does not hold for the roster's own harness (Verdict, item 1).

It is one test, and no production code changes.

### A precondition

Met.

- `handoff-A.md`: **PASS**, with 0 block and 15 warn.
- `handoff-A-dup.md`, "Rework 1": **PASS**, with 0 block and no new warn. Six warns are carried as follow-ups: W1 to W5,
  and W9.

### T precondition

Met.

- **RED** at `b3f0295` (first pass): compile-level on the contract, and assertion-level on the two `--harness` lines of
  `cli_surface_test`.
- **GREEN** on `7260b5e`, plus T-green's two test edits:
  - the CI test step: 2524 passed, 0 failed, 36 ignored (165 binaries);
  - all pass: clippy with `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `docs_cli_test` and
    `docs_pane_control_test`, `wire_selftest`, `cargo test -p holler-proto` and `test-hooks.sh`;
  - 0 blocking issues.
- **T-green's additions are test-only, and they assert behaviour.**
  - `hermetic_test::a_dialog_and_an_unknown_status_are_never_an_activity`: an `if false` mutant on the `dialog open` arm
    fails it.
  - `lifecycle::switch_refuses_a_busy_pane_and_one_waiting_on_a_permission` now asserts the code
    `session-holds-permission`, not only exit 3.
- **My checks were read-only:** `git diff`, `show`, `log`, `merge-tree` and `grep`; `wc -l` and `git diff --check`;
  `changelog-check.sh` (ok); and `gh issue view` of 831, 633, 835, 839, 850, 648, 828, 803 and 696. I re-ran no build or
  test.

### The first REWORK, checked

| # | Asked | On `8aef610` | |
|---|---|---|---|
| 1 | The doc comment of `HARNESS_IDS` says what section 15 item 1 says | `vocab.rs:269-279` names the two uses of `claude` that do not meet: the pane harness (`HarnessKind::Claude`, read only by the pane engine) and the body harness (ADR 0012, read only by `holler-body`). It names `HarnessKind` as the second list and the test that pins every serde name to this one (`rework_test.rs`). "Reserved" is gone. A doc comment only: no wire or golden change | Fixed |
| 2 | `env.rs` passes `CLAUDE_CODE_COMMAND` | `claude_bin` is `program_bin(env, CLAUDE_BIN, CLAUDE_CODE_COMMAND)`, and its doc names the `HOLLER_CLAUDE_BIN` basename limit. In `crates/*/src`, `"claude"` is now written only in `HarnessKind::as_str`, the constant, `HARNESS_IDS`, a hub test and `holler-body` | Fixed |
| 3 | Section 15 item 2's "As built" states what holds | `ADR-0021.md:1068-1078`, checked again. #835's branch is still at `f1d1bd0`. The host-side files are identical: `holler-adapter-host/src`, the test kit's `host.rs` and `conformance/host.rs`, `foreground_test.rs` and `fake_tmux_test.rs`. `claude_code_rule_test.rs` differs only in its module doc, as the paragraph says | Fixed |

F also merged `origin/main` (#855 and #856):

- the three conflicts (`CHANGELOG.md`, ADR-0003, and ADR-0021's "Deferred") keep both sides' intent;
- no conflict marker is left (`git grep`);
- none of this story's code changed for the merge.

### Acceptance criteria

The issue's Acceptance, read as the brief's item 18 says it can be met, then the Goal and the Work list. Against the first
pass's table (below), rows 8, 11 and 19 change.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | One OpenCode and one Claude Code pane from one profile | `pane_claude/mixed.rs::one_profile_launches_an_opencode_pane_and_a_claude_pane`; `a_second_apply_of_the_mixed_grid_changes_nothing` | Met |
| 2 | Both in `pane list` with SHOWN and DRIVEN | `mixed.rs::the_acceptance_scenario_on_the_fakes`: each row's `shown` and `driven` are its record's, and `sync` is `ok` | Met, as the brief reads it |
| 3 | A message to each through `holler say` | Blocked by #839 and #850, by the brief's design: `mixed.rs::say_delivers_a_message_to_each_pane_of_the_scenario` is `#[ignore]` and names both. Delivery is proven at the port: the scenario's `send_prompt`, conformance case 9, `hermetic_test::a_prompt_reaches_the_socket_the_launch_hook_reported_as_one_framed_line` and `lifecycle::reset_first_reaches_the_new_process` | Deferred (the PR is "Part of #831") |
| 4 | Interrupt one | `lifecycle::interrupt_relaunches_a_busy_pane_on_its_session_of_record` and the scenario; the refusals `interrupt_with_text_on_a_claude_pane_is_refused_and_touches_nothing`, `interrupt_of_a_parked_claude_pane_is_refused_by_the_pane_gate` and `abort_of_a_claude_process_is_abort_unsupported`; `interrupt_of_an_opencode_pane_still_goes_to_the_hub` | Met |
| 5 | Park one | The scenario; `lifecycle::park_holds_a_claude_pane_and_leaves_its_process_alone` | Met |
| 6 | `doctor` clean | The scenario, after the interrupt and the park; `doctor::doctor_is_clean_for_a_healthy_idle_claude_pane_beside_an_opencode_pane` | Met |
| 7 | The login is never read or printed | `hermetic_test::login_status_reads_the_auth_method_alone`, `an_unreadable_login_answer_quotes_none_of_it` and `every_login_variable_is_unset_before_the_program_and_none_is_set`; `launch::a_login_variable_in_the_specs_env_is_refused_by_its_name` (all 7 names); `doctor::a_missing_login_is_one_problem_per_pass` | Met |
| 8 | The roster, the watchdog and `doctor` see the Claude pane | **`doctor`:** the 8 tests of `doctor.rs` (rows 6 and 7). **The watchdog:** none in this repository; pfleet#396 will read `holler pane get/list --format=json`, and `pane list --format=json` shows the Claude pane (row 2). **The roster:** since the merge it reads the registry (`roster_cmd.rs:285-308`), joins a hub row to the pane its label names (`:317-333`) and prints `pane list`'s cells for that pane (`PaneRow::from`). A Claude Code pane has no hub row until a body serves its session (#850), and section 15 item 18 records that. But the part this story can meet has no test: the roster reading a registry that holds a Claude record, and printing that record's cells on a row joined to it | `doctor` and the watchdog: met. **The roster: REWORK 1** |
| 9 | Goal: `--harness` on `launch` and `relaunch` | `spec_flags.rs` (2 tests), `launch::an_unknown_harness_is_usage`, `cli_surface_test`, `lifecycle::relaunch_naming_another_harness_is_harness_change_refused` | Met |
| 10 | Goal: the same verbs | `list`, `switch`, `reset`, `park`, `interrupt`, `close` and `doctor` in `lifecycle.rs`, `doctor.rs` and `mixed.rs`; `say` is row 3. No test runs `get` or `watch` over a Claude record (Advisory 6) | Met, with a test gap |
| 11 | Work 1: the variant, `HARNESS_IDS` and the test that pins both lists, `Pane` and `ProfileSpec`, ADR-0021, the epic's block | `rework_test.rs` (`every_kind`), `claude_kind_test` (15 tests), section 15, the #633 comment, and now the doc comment of `HARNESS_IDS` | Met |
| 12 | Work 2: the adapter crate behind the same seam; the host starts and stops the process | `holler-adapter-claude`, a dependency of `holler-cli` only; `hermetic_test` (16); the conformance suite over the fake and the router. The host stops the process by tmux session (`stop_owned`), as section 15 decides | Met, as amended (PR body: Advisory 2) |
| 13 | Work 3: a fake and its conformance suite; the real test opt-in and `#[ignore]` | `fake_claude_test` (13), `claude_conformance_test` (4 tests, 12 mutants), `fake_harnesses_test` (5), `real_claude_test` (`#[ignore]`, runs only with `HOLLER_TEST_CLAUDE=1`) | Met. The real test was not run (Advisory 1) |
| 14 | Work 4: `profile show` and `apply` compare the harness; doctor codes | `mixed.rs::profile_show_compares_the_harness_kind` and `apply_never_changes_a_live_panes_harness`; `doctor::a_claude_process_that_is_gone_is_server_down`, the login tests and the `pane-waiting` test | Met |
| 15 | Work 5: the wizard (#803) | Section 15 item 16: not in this story (#803 is OPEN) | Out of scope, by decision |
| 16 | The operator's abort decision, recorded in ADR-0021 and the note's "Decisions" | Section 15's opening and decision 22; the note's "Decisions"; `claude_kind_test::a_kind_knows_its_shape_and_its_abort_style`; no keystroke in any lifecycle test | Met |
| 17 | Adopt the existing Claude Code panes: decide, build, record | Decided and recorded (item 17, decision 23). Not built: #835 is OPEN and not on `main` (checked at 22:22 MDT) | Deferred, by the brief's rule |
| 18 | The relayed question: does the `say` step need #839? | Item 18 (it does, and #850 too), decision 24 and the #633 comment | Met |
| 19 | One constant and one signal with #835 | `CLAUDE_CODE_COMMAND`, `is_claude_code_command` and the required `HostPort::foreground_commands`; no second spelling of the name; the record of the copy is accurate | Met |

### Spec compliance

Section 15 is built as recorded, items 1 to 18, as the first pass found. This cycle changed no behaviour. What I checked
again in the code:

- **Items 1 to 4.** `HarnessKind::Claude` with `shape()`, `abort_style()` and `login_variables()` (`pane.rs`).
  `HarnessInfo` loads only a record that fits its kind (`try_from = "RawHarnessInfo"`, `misfit`), and
  `HarnessInfo::endpoint` is the one place a record becomes an endpoint (`store-corrupt` otherwise). One
  `HarnessRouter` holds the routing table (`ports/router.rs`).
- **Items 5 and 6.**
  - `SessionChoice` carries the verb's intent: `Resume` is `session-not-found` in the plan, before anything stops.
  - The plan runs `port_for`, then `refuse_login_env`, then `process_command` first among its reads. `process-in-use` is
    checked for a launch only.
  - `stop_old` checks the old endpoint after `stop_owned` and adopts nothing.
  - The wait is `wait_until`, until the process is located and idle.
  - `relaunch_on` and `relaunch_recorded` are the one sequence: `tx_switch`, `interrupt_pane` and doctor's relaunch all
    call it.
  - `tx_switch`'s single-process branch applies the idle gate (`session-holds-permission`) and `check_unclaimed`, and
    sends `--first` to the endpoint of the record the relaunch stored.
  - The single-process chain repairs only while idle, through the caller's relaunch, writes no record after a relaunch,
    and asks the login once per pass, only with a single process in scope.
- **Items 7 to 10, the adapter.**
  - The hook is fixed text.
  - The reader fails closed: the directory and the file must be the user's alone, neither may be a link, and the file
    must hold one absolute path. It is trusted only for a listed pid.
  - The frame is one line, and the held window is 600 ms.
  - `dialog open` is `harness-dialog-open`, and an unknown status is `unavailable`.
  - `abort` is `abort-unsupported`.
  - `login_status` reads `authMethod` alone, and both `claude` runs remove the 7 variables and drop stderr.
- **Items 12 to 14.** `--harness` in `SpecFlags`; `port_for` as the one policy rule; `harness-change-refused` in E0 and in
  `profile apply` before health; each open code declared where it is raised.

**The relayed user request:**

- the amendment came first (`008a995`);
- the abort decision is recorded in both places;
- the run is `in-session`, in its own worktree;
- the answer about `say` and #839 is in the amendment;
- the #835 coordination is now accurate in the code and in the record.

Two sentences of section 15 lean on the roster and are advisory (Advisory 3).

### Quality audit

- **Correctness and failure handling.** No defect in behaviour, as in the first pass. The merge added none: #648's
  roster reads the records through `PaneRow::from`, which reads no harness port or kind, and the wiring builds the Claude
  harness lazily (`WiredHarness`), so a roster or a pane verb on a machine with no `claude` binary fails nothing until a
  Claude call is made.
- **Build guards.**
  - The added `src` lines have no `unwrap`, `expect` or `panic!`, except the test kit's `constant_name`
    (`conformance/claude.rs`), which carries `#[allow(clippy::expect_used)] // #831`.
  - Every `#[allow]` carries a link.
  - The largest touched `.rs` file is `pane_verbs/switch.rs`, at 893 lines; `lint.sh`'s 900-line gate covers
    `crates/**/*.rs` only.
  - Clippy with `-D warnings` and `cargo machete` are clean (T-green).
- **Protocol.** `docs/protocol/v2.md` and the golden files are untouched. The record's `harness.port` is optional, and it
  is absent only on a `claude` record.
- **Tests.** The default run uses fakes only, and every wait is a bounded poll with a deadline. RED-first evidence and the
  mutants are under the T precondition. The gap is REWORK 1.
- **Documentation.**
  - The CHANGELOG has an `## [Unreleased]` entry that links #831, and `changelog-check` passes.
  - Updated: ADR-0021, ADR-0003, `docs/pane-control.md`, `docs/orchestrating.md`, `cli-surface.txt` and the research
    note.
  - The rework's doc sentences are accurate against the code: the roster paragraph, the `HOLLER_CLAUDE_BIN` basename
    limit, the `Bindings` paragraph, and the rollback to a `holler` older than #831.
- **Public-repository privacy.** I grepped all 13,096 added lines, the handoffs included. None holds a personal host, a
  tailnet name, an IP other than `127.0.0.1`, a machine or account name, a private domain, a home path or a secret. The
  one email is `someone@example.invalid`.
- **Commit and PR hygiene.** The subjects are Conventional, and every commit carries `Co-Authored-By`. None carries a
  session link (Advisory 4). The PR is not open yet.

### Scope check

As in the first pass: the brief's scope, less item 17 (#835 is not on `main`) and item 16 (#803's). This cycle added:

- the merge of `origin/main`, which was needed, since the branch conflicted with `main` in three files;
- one sentence or two for each of the first pass's Advisories 5 to 9, each a statement of this PR's own.

There is no unrelated change and no roster code change. Listing a registry pane that has no hub row would change #648's
decided output, so leaving that to #850 is right.

### Verdict

**REWORK** (`reworkKind: test-only`). TEST-ONLY: no `src/` change is required. T writes one test, and nothing else gates.

1. **Required: the roster reads a registry that holds a Claude Code pane and prints that pane's cells on the hub row
   joined to it.**
   - **Why.**
     - The brief's "The acceptance, as it can be met" says: "Every other acceptance line is a test: ... the roster and the
       watchdog seeing the Claude pane where they read the registry."
     - Since the merge of #648 the roster reads the registry, and section 15 item 18 says so, as built
       (`ADR-0021.md:1318-1320`).
     - Every roster test uses OpenCode records (`roster_panes_test.rs` and `roster_panes/*.rs`: `grep -i claude` finds only
       a hub row's own `harness` cell), and `mixed.rs` pins `pane list`, not the roster.
     - So nothing pins that a Claude record neither fails the registry read (that would be one `note:` line, with every
       row's six cells `-`, the OpenCode panes' included) nor prints wrong cells on its row.
   - **Where.** `crates/holler-cli/tests/roster_panes/registry.rs` (238 lines), beside
     `the_cells_are_pane_lists_cells_for_the_same_record`, whose `pane_list_cells` it can reuse. The harness calls the
     pure `roster_cmd::render` with a literal hub reply and the test kit's fakes: `render` at `roster_panes_test.rs:103-111`,
     `Rig` at `:115-162`, `hub_row` and `reply` at `:168-214`, and `pane` at `:217-233`.
   - **Fixture.**
     - The base records, plus one Claude Code record: `pane("demo-c1r2", ...)` with `harness` set to
       `HarnessInfo { kind: HarnessKind::Claude, port: None, pid: Some(4242), health: Health::Healthy }`, a session of
       record, `last_observed.shown` equal to it, and `driven` `None`.
     - The reply: `BASE_ROWS` plus one row named `demo-c1r2/<session>`, as a body for that pane would advertise it (#850).
   - **Assert**, in text and under `--format=json`:
     - exit 0, and an empty stderr (no `note:` line);
     - the Claude row's six cells equal `pane_list_cells` of its record: PANE `demo-c1r2`, its POS and PROFILE, HEALTH
       `healthy`, SHOWN its session of record, DRIVEN `-`. In JSON, its keys `pane`, `pos`, `profile`, `health`, `shown`
       and `driven` carry the same values;
     - every base row's cells are what they are without the Claude record.
   - **Mutant.** For example, drop `claude` records in `Registered::of` (`roster_cmd.rs:262-267`): the test must fail.
     Then revert the mutant. The test passes on today's code. It pins how #831's record meets #648's roster.
   - **T-green's premise does not hold here.** `handoff-T-green.md:103-104` says that "a roster test for a hub row joined
     to a Claude Code pane cannot exist until a body serves that session (#850)". In the roster's harness, every hub row
     is a literal reply, so this row exists in the test as every other fixture row does. What #850 adds is the row in
     production. F listed this test for T (`handoff-F.md:140-141`).

   **Optional, in the same pass (does not gate):** `pane get` of a Claude pane.
   - The text line is `harness: claude port=- pid=<pid>` (`get.rs:284-290`). It is the one output line this story
     changed for the kind.
   - The JSON's `harness` has `kind` `claude`, the pid and no `port` key. This is the watchdog's read path (pfleet#396).

### Advisory notes

None blocks the merge.

1. **The opt-in real test was not run** (the first pass's Advisory 1). It is not a merge condition: the issue makes it
   opt-in and `#[ignore]`. Run it before #831 closes, and before a Claude Code pane is launched on the fleet. Still
   unverified against the real binary, and each fails closed:
   - the hook's `$PPID` key;
   - the inbox frame;
   - the launch through tmux;
   - `agents --json` after a SIGKILL.
2. **For the PR body** (the first pass's Advisory 2, as F updated it):
   - "Part of #831", and what remains: item 17 after #835; the `say` step after #839 and #850; a Claude pane's roster row
     with #850; the real test's first run;
   - the edits to the amendment, as built: items 2, 4 to 10, 12 to 15, 17 and 18, the "Not built here" paragraph, and
     decisions 26 to 28;
   - the `stop_owned` difference from the issue's "by the recorded pid";
   - the AI disclosure that `CONTRIBUTING.md` requires, added with `gh pr edit` (the repo's `CLAUDE.md`);
   - A-dup's W5: a comment on #696 adding `holler-adapter-claude/src/exec.rs` to its scope. W1 to W4 and W9 go in as
     follow-ups.
3. **Two sentences of section 15 lean on the roster.**
   - Item 8 (`ADR-0021.md:1190`) says "and the roster can say which wait it is". The roster #648 built calls no adapter
     and shows no activity, while `doctor` names the wait (`pane-waiting`). Change one word ("doctor") when the ADR is
     next touched.
   - Item 18 and "Not built here" say that #850 "also gives a Claude pane its roster row". That holds only if #850's body
     advertises a name that the roster joins to the pane: today the row's label (`roster_cmd.rs:323-333`), or whatever
     #839 moves the join to. #850's issue says neither. A comment on #850 at the PR step would carry it.
4. **The commit trailers carry no session link.**
   - `CONTRIBUTING.md:19-21` asks for a `Co-Authored-By` trailer and a session link.
   - Every commit on the branch has the trailer, and none has the link. The driver's commits already on `main` are the
     same (`037405f`).
   - The squash message and the PR body are written at the PR step.
5. **`git diff --check`** flags one trailing space, in this file's first pass, and this rewrite removes it.
   `docs/handoffs/667-brief.md`'s flags are `main`'s.
6. **Lower-value test gaps, carried:**
   - `pane get` (unless the optional item above is done) and `pane watch` over a Claude record;
   - the output of `holler interrupt --pane` for a Claude pane;
   - `server-unsupported`.
7. **The older #828 sentences** (ADR-0021 sections 5 and 8, "Deferred", and #649's CHANGELOG entry) are other stories'
   words, and #839's issue says it edits them.
8. **Sizes and format.**
   - `pane_verbs/switch.rs` is 893 lines and `launch/rig.rs` 852. The next story that touches either splits it first.
   - `cargo fmt --check` fails across the workspace, on drift that predates this story. Neither CI nor `lint.sh` runs it.

---

# First pass: the spec audit (2026-10-11, 21:40 MDT)

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
