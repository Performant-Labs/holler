# Handoff-S: Phase 10 (spec audit) - #684 test kit slice e: `FakeHost`, `FakeHarness` and their conformance suites

**Date:** 2026-10-09
**Branch:** issue-684-implementation (at d30873e; merge base e410e9d = origin/main, which has not moved)
**Issue:** #684 (slice e of #638, epic #633). Rigor: in-session. UI surface: none.
**Brief:** `docs/handoffs/684-brief.md`
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
`decisions.md` and `evidence.md` (all in `docs/handoffs/684/`), and the body of GitHub issue #684.
**Diff audited:** all of `git diff origin/main...HEAD` (17 files). I read these in full:
- the four source files: `src/host.rs` (227 lines), `src/harness.rs` (502), `src/conformance/host.rs` (270) and
  `src/conformance/harness.rs` (426);
- the four test files: `tests/host_conformance_test.rs` (224), `tests/fake_host_test.rs` (283),
  `tests/harness_conformance_test.rs` (259) and `tests/fake_harness_test.rs` (576);
- the `CHANGELOG.md` entry.

All paths below are relative to `crates/holler-pane-testkit/` unless they say otherwise.

## A precondition

Met. `handoff-A.md` (Phase 3 plan review) is **PASS** with six warns and no block. `handoff-A-dup.md` (Phase 7
anti-duplication) is **PASS** with three warns and no block.

The diff settles each Phase 3 warn that was meant for F or T:
- **W-2** (the fakes share no state): stated in both module docs, `src/host.rs:11-14` and `src/harness.rs:12-15`.
- **W-3** (decisions 5 and 6 in the suite docs): `src/conformance/host.rs:7-15` and case 7's doc (`:174-176`), and
  `src/conformance/harness.rs:10-21`. Its PR-body half is still open because no PR exists yet (advisory 1).
- **W-4** (`server()` needs a consumer and a pin): the doc names #644 (`src/harness.rs:254-255`). `ServerView` equality
  is asserted in five tests.
- **W-5(c)** (no third private `pane_name`): both suites parse with `succeeds("PaneName::parse", ..)`.
- **W-6(c)** (the ASSUMPTION comments stay in `harness.rs`): they are at `src/harness.rs:306-307`, `332-335` and
  `348-350`.

W-1, W-5(a)/(b) and W-6(a)/(b) were for O (brief amendments or a follow-up), not for F or T. They are not this diff's
defects. See advisory 2.

## T precondition

Met. `handoff-T-green.md` reports no blocking issues.

- **RED.** `handoff-T-red.md` records `cargo test -p holler-pane-testkit --no-run` failing with `E0432` only, on the
  brief's missing API items. T also compiled and clippy-checked the tests against a temporary signature-only API, so a
  mis-typed test could not hide behind `E0432`. Git confirms the RED state. At the T-red commit `b7d3ce2` the four
  source files are still slice a's stubs (3, 4, 2 and 2 lines, as the brief's evidence quotes them), and the four test
  files exist.
- **GREEN.** All 56 new tests pass: `fake_harness_test` 19, `fake_host_test` 12, `harness_conformance_test` 13 and
  `host_conformance_test` 12. Slice a's 32 tests are unchanged. The workspace gives 1164 passed, 0 failed and 5 ignored,
  the same counts F reported.
- **Mutation.** T-green made three mutations of the production code (frozen answers `Ok`, killed answers `Ok`,
  `stop_owned` keeps the pids), and a named test killed each one. F also ran every mutant of the two conformance files
  in a throwaway crate, and each failed its named case for the intended reason.
- **Checked with git:**
  - F's commit `75cca26` touches nothing under `tests/`.
  - Nothing under `tests/` changed after `b7d3ce2`.
  - Nothing under `crates/` changed after `75cca26`. The T-green and A-dup commits touch only handoffs and the journal.

## Acceptance criteria

The test files are abbreviated as follows: HC = `tests/host_conformance_test.rs`, FH = `tests/fake_host_test.rs`,
HAC = `tests/harness_conformance_test.rs` and FHA = `tests/fake_harness_test.rs`.

**Brief (AC 1 to 11):**

| AC | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | The host fake passes its suite, and the suite runs the 9 documented cases in order | HC `the_fake_passes_the_host_conformance_suite` asserts `== Ok(())`. HC `the_host_suite_runs_the_documented_cases` asserts equality with the 9 ids in the brief's order. | MET |
| 2 | Host mutation check | HC `the_unbroken_host_wrapper_passes`, then one test per break, each mapped to the case in the brief's last column. Each asserts `Err`, the named case among the failures, and that every `detail` is non-empty (HC:150-160): `a_host_whose_ps_of_a_missing_session_is_empty_fails`, `a_host_whose_run_creates_the_missing_session_fails`, `a_host_whose_run_starts_nothing_fails`, `a_host_whose_second_ensure_recreates_the_session_fails`, `a_host_that_runs_an_empty_argv_fails`, `a_host_whose_stop_owned_stops_nothing_fails`, `a_host_whose_stop_owned_of_a_missing_session_is_an_error_fails`, `a_host_whose_stop_owned_stops_every_session_fails` and `a_host_whose_ps_lists_every_session_fails`. The mutants use only the fake's port methods plus `end_session` and `sessions` (HC:78-147). | MET |
| 3 | The harness fake passes its suite; 15 ids; the sample rig | HAC `the_fake_passes_the_harness_conformance_suite`, `the_harness_suite_runs_the_documented_cases` (15 ids in order) and `the_sample_rig_is_two_ports_and_two_panes` (`[48100, 48101]`, `scratch:demo-c1r1`, `scratch:demo-c2r1`). | MET |
| 4 | Harness mutation check | The fake configured as a mutant: `a_harness_that_acks_select_without_a_tui_fails`, `a_harness_that_acks_abort_of_an_unknown_id_fails` and `a_harness_whose_servers_do_not_share_a_data_dir_fails` (`set_data_dir(48101, "other")`). The wrapper: `the_unbroken_harness_wrapper_passes`, plus one test per break for `HealthAlwaysTrue`, `PingSession`, `ReusesSessionId`, `AttachFallsBackToLatest`, `SelectUnknownGoesHome` and `SelectBroadcasts`, each naming its case. The wrappers are built as the brief describes them (HAC:145-211). | MET |
| 5 | `FakeHost` mechanisms | FH has the 10 tests the brief names. Two more pin brief rules: `exiting_a_process_is_not_a_call_through_the_port` (the call-log rule for a successful `exit_process`) and `a_fresh_session_has_no_process_and_survives_stop_owned` (decision 7). The brief's exact error values are asserted by equality. | MET |
| 6 | `FakeHarness` mechanisms | FHA has the 18 tests the brief names. One more, `attaching_again_replaces_the_tui_and_a_refused_attach_leaves_it`, pins the brief's `attach_tui` row for a pane that already has a TUI. The frozen, thaw, killed, serve-again and serve-on-running tests also assert `server(P0)` as a whole `ServerView`. | MET |
| 7 | Three ASSUMPTION comments | I re-ran `grep -c 'ASSUMPTION (#642 to confirm)' src/harness.rs`: it prints `3`, at `abort`, `select_session` and `shown_session`. The brief's text is verbatim. The comments are wrapped across lines; see Spec compliance. | MET |
| 8 | Dependency rule | `Cargo.toml` and `Cargo.lock` are not in the diff. I re-ran `cargo tree --offline -p holler-pane-testkit -e normal --prefix none` read-only. The only `holler-*` crates are `holler-pane` and its transitive `holler-proto`, and the `holler-(cli\|hub\|adapter)` count is 0. | MET |
| 9 | No other crate changes | `git diff --name-only origin/main...HEAD` lists the 4 source files, the 4 test files, `CHANGELOG.md` and `docs/handoffs/684*`, and nothing else. `lib.rs`, `conformance/mod.rs`, `fault.rs`, `feed.rs`, `fixture.rs` and every manifest are untouched. | MET |
| 10 | CHANGELOG | One entry, under `## [Unreleased]` / `### Enhancements`, right after slice a's entry. No sibling slice has merged, so that is the placement the AC asks for. It covers every element AC 10 lists: the fake host's sessions, argv log and processes; the harness's shared data directory, frozen or killed server, a session deleted under a TUI and the two quirk switches; the 9 and 15 cases; a fake with a quirk on failing; and "Test code only". It links #684 and names no person. F and T-green record `changelog-check: ok`. | MET |
| 11 | Guards | F and T-green recorded these passing: build, `clippy --workspace --all-targets -D warnings`, the workspace tests, `machete`, `lint.sh`, `changelog-check.sh`, `test-hooks.sh` and `rustfmt --check`. My own read-only checks: the largest `.rs` file is 576 lines (under 600). `src/` has no `unwrap()`, `expect(`, `panic!`, `unreachable!` or `assert!`, apart from the two usage examples in `text` fences that the brief prescribes (`src/conformance/host.rs:84`, `src/conformance/harness.rs:133`), which are never compiled. Each of the four `#![allow]` lines carries `// #684`. There is one function per case. | MET |

**Issue #684 acceptance:**

| Criterion | Proving test or evidence | Status |
|---|---|---|
| The fakes pass their own suites, and each suite rejects the mutants named (a mutation check per suite) | AC 1 to 4. The issue's named mutant, "a store run with a quirk on", is covered by `a_harness_that_acks_select_without_a_tui_fails` and `a_harness_that_acks_abort_of_an_unknown_id_fails`. | MET |
| Nothing depends on `holler-cli` or `holler-hub`; the testkit depends on `holler-pane` only | AC 8 | MET |
| The workspace gates pass, the CHANGELOG `[Unreleased]` entry links the issue, and library code has no unwrap, expect or panic | AC 10 and AC 11 | MET |

**Each item of the issue's Scope has a test that pins it:**
- **`FakeHost`:**
  - `ensure_session` is idempotent: HC case 4 and FH `ensure_session_keeps_the_first_cwd`.
  - `run` records each `Argv`, element for element: FH `run_records_each_argv_verbatim`.
  - `ps` lists the session's pids: HC cases 3 and 9.
  - `stop_owned` empties them: HC case 6 and FH `a_fresh_session_has_no_process_and_survives_stop_owned`.
  - Faults: FH `a_wedged_host_times_out_every_method` and `a_refused_run_records_nothing`.
- **`FakeHarness`:**
  - One shared data directory: HAC case 5 and its separate-directory mutant.
  - `ses_` ids: FHA `session_ids_are_ses_prefixed_and_distinct`.
  - An unknown id is `session-not-found` for `abort` and `select_session`: HAC cases 8 and 13.
  - `shown_session` is `None` with no TUI, on the home screen and after a delete: HAC case 9, and FHA
    `navigating_by_hand_changes_the_shown_session` and `a_session_deleted_under_a_tui_sends_it_home`.
  - The two quirk switches: FHA `select_without_a_tui_is_acked_with_the_quirk` and
    `abort_of_an_unknown_id_is_acked_with_the_quirk`.
  - A frozen server: FHA `a_frozen_server_answers_health_false_and_times_out_its_calls`.
  - A killed server: FHA `a_killed_server_answers_health_false_and_is_unavailable`.
  - The ASSUMPTION comments: AC 7.

## Spec compliance

- **Public API, item for item.** `grep` of `pub` items in the four source files lists exactly the brief's API:
  - `HostOp`, and `FakeHost` with `new`, `faults`, `sessions`, `cwd`, `runs`, `exit_process` and `end_session`;
  - `HarnessOp`, `Quirk`, `ServerState`, `ServerView`, `TuiView`, and `FakeHarness` with `new`, `faults`, `set_quirk`,
    `set_data_dir`, `freeze`, `thaw`, `kill`, `seed_session`, `delete_session`, `navigate`, `close_tui`, `server`, `tui`
    and `aborts`;
  - `host_cases` and `run_host_conformance`;
  - `HarnessRig` with `sample`, `harness_cases` and `run_harness_conformance`.

  The derives and the generic bounds match. Each fake also has the `PortOp`, `Default` and port-trait impls the brief
  lists. Every state struct and helper is private, nothing is re-exported, and each fake holds one `Mutex` taken with
  `unwrap_or_else(PoisonError::into_inner)`.
- **Exact error values (the brief's 7-row table).** Each value has one constructor:
  - `not_found` (`src/host.rs:223-227`);
  - the `usage` message (`src/host.rs:211-213`);
  - `unreachable_server` (`src/harness.rs:484-488`);
  - `frozen` (`src/harness.rs:477-481`), which has the fault switch's `Timeout { op: op.as_str() }` shape;
  - the in-use `unavailable` (`src/harness.rs:379-381`);
  - `no_tui` (`src/harness.rs:491-495`);
  - `unknown_session` (`src/harness.rs:498-502`).

  FH and FHA assert each value by equality.
- **`FakeHost` behaviour.** Each bullet of the brief holds:
  - `enter` comes first in every port method.
  - `ensure_session` uses `or_insert_with`, so it keeps the first cwd and the pids.
  - `run` checks a missing session, then an empty argv, then mints a pid and logs a clone of the argv.
  - `stop_owned` clears the pids, keeps the session, and is `Ok` on a missing session.
  - `ps` returns the pids in start order, or `pane-not-found`.
  - Pids come from one counter starting at 10 000 and are never reused.
- **`FakeHarness` behaviour, row by row, in the brief's check order:**
  - `serve`: frozen is `timeout`; running for the same name returns its pid; running for another name is the in-use
    error; killed or absent starts a new pid from 20 000. It creates no session.
  - `health`: reads the state directly and fails only through the fault switch.
  - `create_session`: reaches the port, then mints `ses_{n:026x}` with n counting from 1.
  - `list_sessions`: reaches the port, then lists the sessions in creation order.
  - `abort`: reaches the port, then refuses an unknown id unless `AbortUnknownAcked` is on, then logs.
  - `attach_tui`: reaches the port, then refuses an unknown id (the TUI is unchanged), then replaces the TUI.
  - `select_session`: with no TUI, the quirk gives `Ok` and anything else gives the no-TUI error. It then reaches the
    TUI's port, checks the id, and changes only that pane's TUI.
  - `shown_session`: reads the TUI and never reaches the server.

  The scenario controls (`freeze`, `thaw`, `kill`, `seed_session`, `delete_session`, `navigate` and `close_tui`) behave
  as the API section documents them.
- **Decisions already made (operator, epic, issue).** All five are implemented:
  - `holler-pane` is the only dependency.
  - Neither `lib.rs` nor `conformance/mod.rs` is edited.
  - The suites run with the quirks off.
  - An unknown id is `session-not-found`, and `shown_session` is `None` in the three cases.
  - Nothing touches a live fleet: everything is in memory.
- **Decisions made in this brief (1 to 11).** All are implemented as stated:
  - 1: `shown_session` keeps answering while the server is frozen or killed.
  - 2: the wedged adapter and the frozen server are separate switches.
  - 3: `health` is a plain `bool`.
  - 4: `delete_session` is a scenario control.
  - 5: no TUI is `unavailable`, checked before the id.
  - 6: the host's missing-session codes.
  - 7: the host suite never asserts an empty `ps` after `ensure_session`, and case 6 ensures again before it reads
    `ps`.
  - 8: `HarnessRig`.
  - 9: the id and pid counters.
  - 10: no broadcast is modelled.
  - 11: `as_str`.
- **Where the brief adjusts the issue's wording.** The issue says a frozen server makes "every other call `timeout`",
  and a killed one makes the other calls `unavailable`. The brief records two adjustments, so neither is silent:
  - `shown_session` keeps answering (decision 1, grounded in spike :193-194);
  - `serve` restarts a killed server (the brief's `serve` row, grounded in spike :195-196).
- **Behaviours the brief left open**, decided by F and journalled in `decisions.md`:
  - `navigate` does not reach the server.
  - `delete_session` sends home a TUI on any port. Ids come from one counter, so they are unique across data
    directories.
  - Freezing a frozen server and thawing a running one are `Ok`.

  None of these contradicts the brief or the issue.
- **Declared deviation: the ASSUMPTION comments are wrapped.** The brief says "one line each". Each comment is wrapped
  at the file's 100-column width, with the verbatim prefix on its first line and the brief's text verbatim. AC 7's
  check is a `grep -c`, and it prints 3. F declared this under "Deviations", so it is not silent. **Accepted as is.** I
  read "one line" as one short comment, not one physical 200-character line.
- **The spike citations in the ASSUMPTION comments are accurate.** Each cited line of
  `docs/research/opencode-pane-spike.md` says what the comment cites it for:
  - `:179` and `:267`: aborting a model turn was not run;
  - `:151-152` and `:269`: whether Herdr exposes the terminal title is unverified;
  - `:272-273`: `select-session` across project directories was not tried.

## Quality audit

- **Correctness and failure handling.**
  - **Order of work.** Every port method calls `enter`, then makes its checks, then makes one state change, all under
    one held guard. A refused call changes nothing. The wedged tests and `a_refused_run_records_nothing` pin this.
  - **No lost writes under concurrency.** Each check-then-act sequence runs under one lock, so no write is lost and no
    check goes stale: `select_session`'s TUI, reach, id and show steps; `create_session`'s reach and mint; and
    `abort`'s reach, id check and log.
  - **No lock held across the fault delay.** The fault switch's injected delay sleeps before the fake takes its own
    lock (`enter` returns first).
  - **Poisoned locks.** A poisoned lock is taken over, as in slice a.
  - **The quirks.** Each is a single branch, and neither skips the reachability check: an `abort` under the quirk
    still reaches the port first.
  - **Edge cases I traced by hand.** Each gives what the brief's rules imply:
    - a killed port served again by another pane;
    - `set_data_dir` after sessions exist;
    - a delete with TUIs on two ports;
    - a select with the quirk on while a TUI is present (it takes the normal path);
    - a run of an empty argv in a missing session (`pane-not-found` comes first, as the brief orders it).

  The `u32` pid counters cannot overflow in any realistic test.
- **Build guards.** These hold:
  - no `unwrap`, `expect`, `panic!` or `unreachable!` in library code;
  - every `#[allow]` carries a `// #684` link;
  - no touched file is at or above 900 lines (the largest is 576);
  - no dead code (`dead_code = "deny"` and clippy are clean, per F and T-green).
- **Protocol.** Nothing changes: no protocol field, error code (`error.rs` and `holler-pane` are untouched), golden
  file, ADR or `docs/protocol/v2.md`.
- **Tests.**
  - The kit is in memory, so the cross-process harness rule does not apply.
  - There are no sleeps. The only `sleep` in the diff is the argv `["sleep", "30"]`.
  - The RED-first evidence is in T's handoff, and git confirms it.
  - Every mutation test asserts that its named case is among the failures. Removing or weakening a case therefore
    fails its test, and none of them is tautological.
  - One small gap: `thaw_brings_a_frozen_server_back` (FHA:252-275) calls 5 of the 7 server-reaching methods after the
    thaw, but not `serve` or `attach_tui`. Its `server(P0)` assertion pins the state as `Running`, and both of those
    methods are pinned on a running server elsewhere (`serve_on_a_running_port`, HAC case 10). It is not blocking
    (advisory 4).
- **Documentation.**
  - The CHANGELOG entry is present and accurate.
  - The diff adds no log event, CLI surface or protocol field, so no README or `docs/` change is needed.
  - Real module docs replace the stub comments in all four files, and the suite docs state every reading that binds
    #641 and #642.
  - `lib.rs:27-28` ("The modules of slices b to e are empty stubs") becomes false for `host` and `harness`. AC 9 forbids
    this slice to edit it (advisory 2).
- **Public-repository privacy.** Clean. I grepped every added line for IPv4 addresses, home paths, personal names and
  e-mail addresses, tailnet and host names, and secrets. There are no hits, apart from two benign prose matches in the
  handoffs ("token store" and "no secrets handled"). The names are neutral: `demo-c1r1`, `scratch` and `/srv/demo`.
- **Commit and PR hygiene.**
  - All six commit subjects are Conventional Commits and match `.githooks/commit-msg`'s regex.
  - Each commit carries a `Co-Authored-By:` trailer, and the author is the GitHub no-reply address.
  - The trailers have no session link. That is the workflow script's own commit format, the same on every merged
    pipeline run (#638, #676), and the squash merge replaces these messages. I do not count it against this slice.
  - No PR exists yet, so I could not audit the PR body's AI disclosure. CLAUDE.md gives that step to the run's agent
    after the PR opens (advisory 1).

## Scope check

The work matches the brief's scope:
- F changed the four stub files and `CHANGELOG.md`, and T added the four test files. The public surface is the
  brief's, item for item.
- The `harness/world.rs` fallback was not needed, because `harness.rs` is 502 lines.
- T added three tests beyond the brief's names. Each pins a behaviour the brief specifies (decision 7, the `attach_tui`
  row, and the call-log rule), so they are not over-delivery.
- There are no unrelated refactors and nothing is missing.

The issue's own "Blast radius" names only `host.rs`, `harness.rs` and `CHANGELOG.md`. But the issue's title and scope
require the conformance suites and a mutation check per suite, and slice a's stubs assign
`src/conformance/{host,harness}.rs` to this slice ("slice e (#684) fills it"). The brief's "Files" and "Blast radius"
sections list every file the diff touches. That is not a deviation.

## Verdict

**PASS.** Every acceptance criterion of the brief (AC 1 to 11) and of issue #684 has a proving test or evidence. The
implementation follows the brief's API, behaviour tables, exact error values and decisions. The quality audit found
nothing blocking. Ready for O.

## Advisory notes (non-blocking)

1. **For the run's agent, after the PR opens.**
   - Add the AI disclosure that `CONTRIBUTING.md` asks for (`gh pr edit`).
   - In the PR body, list what binds the adapters (A's W-3, repeated in A-dup's Notes for O):
     - **#641:** `run` and `ps` of a missing session are `pane-not-found`; `stop_owned` of a missing session is `Ok`; an
       empty argv is `usage`.
     - **#642:** reachability is checked before existence; an unknown id is `session-not-found` for `abort`,
       `attach_tui` and `select_session`; `select_session` on a pane with no TUI is `unavailable`, checked before the
       id; `shown_session` with no TUI is `Ok(None)`.
   - Sibling slices #681 to #683 add CHANGELOG entries at the same spot. On a rebase, keep every entry (the brief's
     Risks).
2. **For O: the test-kit cleanup issue that A-dup asked for does not exist yet.** My `gh issue list` searches on
   2026-10-09 found none. Its scope:
   - one suite-fixture shape and one ASSUMPTION prefix: this slice's `HarnessRig` with a 3-tuple `fresh` against
     #683's `HerdrFixture<H>`;
   - `tests/support/mod.rs` with `assert_fails_on`;
   - generic `holds` and `lacks` in `conformance/mod.rs`;
   - one crate-private lock helper;
   - `fixture.rs` constants for `HarnessRig::sample`;
   - the stale `lib.rs:27-28` sentence.

   Settle the fixture shape before #640 or #642 writes its suite runner. After that, a change breaks the adapters'
   tests.
3. **For the Chain Summary.** A Phase 3 assumption is still unverified: that the real tmux adapter (#641) and OpenCode
   adapter (#642) can meet every suite case. It was reasoned against the spike but not run. The suite docs name each
   reading that binds those adapters.
4. **Minor test gap.** `thaw_brings_a_frozen_server_back` does not call `serve` or `attach_tui` after the thaw. Tests
   elsewhere cover it (Quality audit); no action needed.
