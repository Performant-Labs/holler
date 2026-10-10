# Handoff-S: Phase 10 - #640 part 3 of 3: the opt-in scratch-Herdr test, the contract docs and the ADR-0021 rows (spec audit)

**Date:** 2026-10-09 (11:53 PM MDT)
**Branch:** issue-640-implementation (head `663460b`; diff `origin/main...HEAD` = `dc300ab...663460b`. `origin/main` is now
`d9eabbb`, and `git merge-tree` against it is clean, per T-green and A-dup)
**Issue:** #640 (epic #633), part 3 of 3. The PR says `Closes #640` (Decision 1).
**Source of truth:** `docs/handoffs/640-brief.md` (all of it); `gh issue view 640`, read 2026-10-09. The issue was last
updated at 8:16 PM MDT and carries the operator's two "(amended 2026-10-09)" edits, which the brief asked for in "For the
operator", item 1.
**Handoffs reviewed:**
- `docs/handoffs/640/handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`,
  `evidence.md` and `decisions.md`.
- The outside diff gate (DeepSeek V4 Pro, git-ignored): round 1 (`docs/handoffs/640-diff-result-r1.md`, cut off at the
  8192-token cap) and its hand rerun (`640-diff-r2.md`, PASS, in the run's session scratchpad).

**Read in full:**
- `crates/holler-adapter-herdr/src/adapter.rs` (after the change), and the `src/protocol.rs` and `Cargo.toml` hunks.
- `tests/scratch_herdr/mod.rs`, `tests/scratch_herdr_test.rs` and `tests/adapter_messages_test.rs`.
- The `tests/wire_herdr/mod.rs` hunk and its module doc.
- The `holler-pane` doc hunks (`ports.rs`, `error.rs`, `pane.rs`, `reconcile.rs`).
- The `docs/adr/ADR-0021.md`, `docs/testing.md` and `CHANGELOG.md` hunks.
- `origin/main`'s own ADR-0021 diff since `dc300ab`.

This file replaces part 2's `handoff-S.md`, as the brief's Handoffs line says. Part 2's is in git at `0ad2d8a`.

## A precondition

**Met.**

- `handoff-A.md` (the plan review) is **PASS**, with 0 block and 6 warn findings.
- `handoff-A-dup.md` (the anti-duplication gate) is **PASS**, with 0 block and 2 warn findings.
- A-dup carries plan warns 4 and 6 forward unchanged. Neither of its own warns is drift that F introduced.

## T precondition

**Met.**

**RED** (`handoff-T-red.md`) is valid:
- AC 16 fails on all 14 rows, each reporting the wire `op`.
- The four AC 18 tests fail quoting the whole `LONG`, each in the arm it names.
- AC 1-7 and AC 17 pass by design, and every part 1 and part 2 target stays green.

**GREEN** (`handoff-T-green.md`) shows zero blocking issues:
- The crate's tests all pass (2 ignored), and the workspace run shows 1519 passed.
- Both of the Test plan's mutants die as required. Mutant (a) fails exactly the four AC 18 tests, and mutant (b) fails
  exactly AC 16. An extra mutant (rename every error) shows AC 17 is not vacuous.

**Opt-in runs** (AC 12-14) passed four times at RED and four times at GREEN, against real Herdr
`0.9.1-preview.2026-09-21-0ff0f27e2226`. No `h640.` process or directory was left behind after any run.

## Acceptance criteria

"Default run" is `cargo test -p holler-adapter-herdr` with no variable set. Test names are in
`crates/holler-adapter-herdr/tests/scratch_herdr_test.rs` (AC 1-13) and `tests/adapter_messages_test.rs` (AC 16-18),
unless a row names another file.

| AC | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | The gate runs only on exactly `"1"` | `the_gate_runs_only_on_exactly_1`: `Run` for `Some("1")`, and all six skip values of the AC skip | MET |
| 2 | `default` refused by name | `the_name_guard_refuses_the_default_session_by_name`: `default` and `Default` are both `Err`, and the text contains `default` | MET |
| 3 | Only `holler640-` and 8 lower-case hex | `the_name_guard_accepts_only_holler640_and_8_hex`: the one valid form is `Ok`, and all ten names of the AC are refused | MET |
| 4 | Generated names are valid and distinct | `generated_names_pass_the_guard_and_differ`: 64 names, all valid, all distinct | MET |
| 5 | The socket stays inside the root | `the_socket_guard_keeps_the_socket_inside_the_root`: the valid socket is `Ok`. Refused: the sibling sharing the prefix (`/r/h640.abc/...`), a path outside the root, a relative path, `..`, `.`, the root itself, and a 100-byte path. A 99-byte path is accepted | MET |
| 6 | The server is proven by its own status | `the_server_is_proven_by_its_own_status`: `Ok` with an extra field present. The eight refusals of the AC each fail, and the `default` refusal names `default` | MET |
| 7 | The scratch environment carries nothing live | `the_scratch_env_carries_nothing_of_the_live_herdr`: built from the AC's literal pairs, with every assertion of the AC | MET |
| 8 | One way to run `herdr` | Greps re-run by S: one `Command::new` (`tests/scratch_herdr/mod.rs:247`). `std::env` and `env::var` appear under `tests/` only in `scratch_herdr/mod.rs`, and nowhere in `src/`. S read `command` (`mod.rs:245-257`); details are in Spec compliance, Decision 3 | MET; advisory 3 |
| 9 | Ignored by default | `#[ignore` appears exactly at `scratch_herdr_test.rs:214` and `:268`, each with the pinned reason. T: `--list --ignored` lists exactly the two tests, and the default run reports `2 ignored` | MET |
| 10 | Unset gate with `--ignored` | T-red and T-green: under `env -i` both tests pass, each printing `skipped (HOLLER_HERDR_SCRATCH is not 1): no scratch Herdr server was started`. S read the code: `opt_in()` is the first statement of both tests (`:216`, `:270`) | MET |
| 11 | Gate set, no `herdr` | T-red and T-green: both tests FAIL with ``HOLLER_HERDR_SCRATCH=1 asks for a scratch Herdr, but no executable `herdr` is on PATH`` | MET |
| 12 | The conformance suite against real Herdr | `scratch_herdr_passes_the_conformance_suite`. Details below the table. 8 opt-in runs, all ok | MET |
| 13 | Create, run, send, read, close and snapshot | `scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane`: steps 1-6 in order, as written. Details below the table. 8 runs, all ok | MET |
| 14 | The after-run record | T-red: the `herdr --version` line, each test's duration, `pgrep` printing nothing, and no `h640.*` directory left. T-green: the same, with the version taken from the server's `version()` reply | MET; reading 4 |
| 15 | A real-Herdr gap is fixed here | No gap was found at RED or at GREEN | N/A |
| 16 | A timeout names the port method | `a_timeout_names_the_port_method`. Details below the table. RED: all 14 rows failed with the wire `op`. Mutant (b) fails this test and no other | MET |
| 17 | Every other error passes unchanged | `every_other_error_passes_through_unchanged`: the same 14 exchanges, each failing with an injected `unavailable`, which the call returns exactly. Part 2's split `pane-not-found` → `unavailable` stays pinned by `adapter_test.rs::a_split_target_closed_under_the_adapter_is_unavailable_not_pane_not_found` (untouched, passing). This guard passes at RED by design; T's overreaching-rename mutant fails it | MET |
| 18 | Herdr-sent text is cut to 64 characters | `a_misplaced_new_panes_id_is_cut_to_64`, `a_new_pane_with_no_cell_has_its_id_cut_to_64`, `a_vanished_split_targets_id_is_cut_to_64` and `a_tabless_workspaces_label_is_cut_to_64`. Details below the table. RED: all four quoted the whole `LONG`. Mutant (a) (64 → 65) fails all four | MET |
| 19 | One `excerpt`, shared | The first grep prints exactly `protocol.rs:580:pub(crate) fn excerpt(text: &str) -> String {`. The second prints `adapter.rs:312`, `:318`, `:422` and `:436`, each inside `excerpt(..)`. It also prints `:383`, the label `snapshot` returns, which is an accepted exemption (reading 3). Caller values keep `{:?}` | MET |
| 20 | The transport is unchanged | `git diff` of `src/transport.rs` is empty against both `origin/main` and `dc300ab`. T: `transport_test` passes 13 tests and still asserts `herdr.ping` | MET |
| 21 | The `Key` doc | `grep -nE 'C-c\|Enter'` on `ports.rs` prints nothing. The text is D1, verbatim | MET |
| 22 | The `ensure_pane` doc | Names both `grid-out-of-range` and `grid-unreachable`. The text is D2, verbatim | MET |
| 23 | The `GridOutOfRange` and `Timeout` docs | `error.rs:416` and `:457` carry the required phrases. The text is D3 and D4, verbatim | MET |
| 24 | No "#640 records it" | The grep over `crates/` and `docs/adr/` prints nothing. The text is D5 and D6, verbatim | MET |
| 25 | ADR-0021, A1-A9 | `grid-unreachable` occurs 4 times. Both old sentences are gone. `<port>.<method>` occurs only at `:396`, the §9 `timeout` row. The diff (+18/-11) has nine hunks, each exactly one of A1-A9 and verbatim; A6-A9 are only re-wrapped. `**Date:**` is unchanged | MET |
| 26 | The `docs/testing.md` section | "Opt-in: a scratch Herdr server (#640)" at `:439`, between "Beyond loopback: `interop.yml`" and "Windows is deferred". It names `HOLLER_HERDR_SCRATCH`, section C's command, the gate rule, the isolation, "CI never runs them" and protocol 22 (equal to `SUPPORTED_PROTOCOLS`). It adds the sibling tmux convention (A finding 2) | MET |
| 27 | The CHANGELOG entry | Under `## [Unreleased]` / `### Enhancements`, right after the part-2 entry. It is Decision 14's text and links #633, #649 and #640. T: `changelog-check: ok` | MET |
| 28 | The gates | T-green's Tier 1 table: the crate, `holler-pane` (86), the workspace (1519), clippy with `-D warnings`, fmt, `lint.sh` and machete all pass, plus `docs_cli_test`, `wire_selftest` and rustdoc. CI on the PR is the final word | MET |
| 29 | Part 2's AC 35-37 and 39; the narrowed AC 38 | Re-run by S. Part 2's AC 35, 36, 37 and 39 each print nothing. The narrowed pattern prints nothing over `src/` and `Cargo.toml`, and under `tests/` it matches only `scratch_herdr/mod.rs` and `scratch_herdr_test.rs` | MET |
| 30 | Dependencies | The only manifest or lock change since `dc300ab` is the `tempfile` comment (+2/-1). T: `cargo tree` lists only `holler-pane` and `serde_json`, and machete is clean | MET |
| 31 | Size and complexity | The largest touched `.rs` file is `error.rs` at 713 lines (`adapter.rs` is 493). `too_many_lines` and `cognitive_complexity` are `deny` in `[workspace.lints.clippy]` (`Cargo.toml:24-25`), and clippy is clean | MET |
| 32 | No platform-sensitive pattern | S read both new files and found no socket option and no `io::ErrorKind`. The only `thread::sleep` is inside the bounded `poll` (`mod.rs:238`). The base, the root and the proven socket are compared only after `fs::canonicalize`. `check_socket` and `prove` call no `fs` function, and section A's tests touch no file | MET |

**AC 12 in detail.** `run_herdr_conformance` runs with one `ScratchHerdr` per case, kept as that case's guard. Each case
connects through `HerdrAdapter::connect(HerdrConfig::new(<session>, <socket>).with_workspace("holler640-grid", 2 by 1))`.
The test asserts `assert_eq!(result, Ok(()))`.

**AC 13 in detail.** The steps, as the test runs them:

1. `version()` returns a non-empty string on one line, and the test prints it.
2. `a` is placed at r1c1, and the workspace's panes are exactly `[a]`.
3. The test types `printf 'holler640-%s\n' ran` as a raw string, then sends `enter`. A read finds a line that, trimmed,
   equals `holler640-ran`.
4. The test types `holler640-typed` with no Enter, and a read finds a line containing it.
5. `b` is placed at r2c1. The workspace's panes, sorted by row, are exactly `[a, b]`, and every pane in the snapshot carries
   the scratch session.
6. `close(b)` is `Ok`, and the remaining pane ids are exactly `[a]`. Closing `b` again and `read(b, 1)` each return
   `PaneNotFound`.

The read polls go through `poll` (100 ms apart, 10 s at most). A failed poll quotes only a line count, never the screen.

**AC 16 in detail.** The test covers all 14 rows of the brief's table. The expected strings come from `HerdrOp::as_str`,
plus `herdr.connect`. Each row asserts `Err(Timeout { op })` exactly, and asserts that its trap fired.

**AC 18 in detail.** Each of the four tests asserts that the error is `Unavailable { what }` where `what`:
- contains `format!("{:?}...", "x".repeat(64))`;
- does not contain `TAIL-NOT-QUOTED`;
- has no `\n`.

Two of the tests also assert that their reply rewrite or their split actually happened. Part 1's surviving mutant
(64 → 1000) would now fail these tests as well.

**The issue's Acceptance list.**
- Line 4 (the scratch session) is AC 1-14 above.
- Lines 1-3 were covered by parts 1 and 2 (their S handoffs), and AC 12 now also runs line 1 against real Herdr.
- The operator amended line 3 and the Scope's version sentence on 2026-10-09 to match the merged decisions. So the
  brief's contradictions 1-3 with the issue text are resolved.

## Spec compliance

Decisions already made (MO), one by one:

1. **Closing part.** The brief replaced part 2's, as stated. The PR is not open yet: there is no PR for this branch, and
   the remote branch was deleted after #702 merged. So `Closes #640` and the AI disclosure are checked at PR time
   (advisory 1).
2. **The scratch root.** Implemented as stated, with one declared deviation: the base limit is 29 bytes, not 40
   (reading 1). The rest is as stated:
   - a fresh `tempfile` root `h640.XXXXXX`, canonicalized;
   - `home/` with the spike's `config.toml` (no onboarding, `/bin/sh`, no version or manifest check);
   - `home/.local/state`, `home/.local/share` and `home/.cache`;
   - `run/` at mode `0700`, `work/`, and `server.log`;
   - the session from `new_name()`.
3. **One function runs `herdr`.** `command` does all of this:
   - calls `check_name(name)?` first;
   - puts `--session <name>` before every other argument;
   - calls `env_clear()`, then `envs(scratch_env(root, std::env::vars_os()))`;
   - sets the working directory to `root/work`, with stdin null.

   `run_bounded` bounds a short command at 5 s: it polls `try_wait` every 50 ms, then kills and reaps on expiry. The
   server is the guard's child. Advisory 3 covers the stdout reader.
4. **Start and prove.** As stated:
   - The guard exists before the proof, so a refusal still stops the server.
   - It polls every 100 ms for at most 15 s.
   - A child that has exited panics, naming its status.
   - It returns only when the status command exits 0, `prove` is `Ok`, the socket exists, and the socket equals its
     canonical path.
   - On timeout it panics, saying the server did not prove itself in 15 s.
5. **Stop only this server.** `server stop` runs through `command`, so it carries the checked `--session`; a failure is
   one `eprintln!`. `Drop` then polls `try_wait` for 10 s, and kills and waits on its own `Child` if it is still alive.
   The `TempDir` field is dropped last. `Drop` never panics, never starts tmux, and never runs `session stop` or
   `session delete`.
6. **The gate.** `opt_in` reads `std::env::var_os` and never calls `set_var`. It searches `PATH` by hand (`split_paths`,
   a file with an exec bit). If none is found, it panics, naming `herdr` and `PATH`.
7. **Only T ran `herdr`.** F says so in `handoff-F.md`, and S ran nothing.
8. **Real-Herdr gaps.** None were found, so there was nothing to fix or escalate.
9. **The `op` rule.** `run_as` wraps the whole body of all seven port methods and of `connect_with`, and renames only a
   `Timeout`, so no `Timeout` escapes unrenamed. The transport is untouched and keeps the wire vocabulary. AC 16 pins the
   seven strings equal to `HerdrOp::as_str`. `herdr.connect` is documented in the module doc, in `connect_with`'s doc
   and in the constants' comment.
10. **`excerpt`.** `pub(crate)` is the only change to `protocol.rs`. The four Herdr-sent quotes go through it, and the
    caller's values keep `{:?}`. I read every other placeholder in `adapter.rs`: each quotes a caller or config value,
    a `GridPos` or a count.
11. **D1-D6.** Verbatim. They are doc comments only: no signature, derive, attribute or code line changes.
12. **A1-A9.** Verbatim and in place, with no other ADR line changed and `**Date:**` unchanged. `origin/main`'s own
    ADR edits since `dc300ab` (#646, #662 and #663) touch none of these lines and make none of them stale. A5's
    "#645's and #646's, planned" still holds, because #646's merged park and unpark declare no open code.
13. **`docs/testing.md`.** As stated, plus A finding 2's sentence on the tmux sibling.
14. **The CHANGELOG.** As stated.
15. **RED.** As T-red records it.
16. **Ordering.** The two ignored tests use separate servers, and the documented command passes `--test-threads=1`.

**Accepted readings and declared deviations.** None of them is silent: each is journalled or recorded in a handoff.

1. **Decision 2's 40-byte base limit.** The brief's own numbers conflict:
   - Every socket path is the base plus a fixed 70-byte tail,
     `/h640.XXXXXX/home/.config/herdr/sessions/holler640-XXXXXXXX/herdr.sock`.
   - A 40-byte base therefore gives a 110-byte socket, which AC 5's `check_socket` (limit 100) refuses. So any base of
     30 to 40 bytes would always have made `start` panic.
   - T derives `BASE_LIMIT = SOCKET_PATH_LIMIT - 1 - SOCKET_TAIL.len()`, which is 29, in code, and journalled the reason
     (`decisions.md`, T at RED).
   - This keeps Decision 2's intent: a long `TMPDIR` falls back to `/tmp`.

   It is not a hold: the shipped code is right, and only the brief's number was wrong.
2. **`herdr.connect` against D4 and A4** (A finding 4). D4 and A4 say "`op` names the port method", with no exception
   for the constructor. F kept them verbatim, as Decision 11 and AC 25 require. Their subject is a port method, and
   `connect` is not one, so the exception is documented in the adapter instead. I checked every `PaneError::Timeout`
   producer on this branch and on `origin/main`:
   - the OpenCode adapter (`harness.serve`, `.create_session`, `.list_sessions`, `.abort`);
   - the host adapter (`host.*`);
   - the test kit's `PortOp` and `HarnessOp` strings;
   - `holler-cli`'s `profile_store.cas_put` test values.

   Every one is a port method. So A4's "in every implementation and fake" holds, except for this one documented
   constructor case (advisory 2(c)).
3. **AC 19's second grep** also prints `adapter.rs:383`, `workspace: workspace.label.clone()`, which is the label that
   `snapshot` returns. Quoting and cutting it would corrupt `HerdrPane.workspace`, and the suite filters by exact label.
   F left it whole, as A finding 5 asked. It is exempt by name.
4. **AC 14's version line.** AC 14 asks for `herdr --version`, while the Operating rules let only the harness run
   `herdr`.
   - At RED, T ran `herdr --version` with an empty environment except `PATH`, and a throwaway `HOME` and `XDG_*`. That
     contacts no server or session.
   - At GREEN, T took the version from the scratch server's own `version()` reply.

   Both are journalled, and both name the same build.
5. **The issue's blast radius against the doc edits outside it** (the brief's contradiction 5). Both earlier briefs assign
   these edits to part 3, and the test kit's `ASSUMPTION (#640)` comments ask for them by name. No code outside the crate
   changes. Accepted.

## Quality audit

**Correctness and failure handling.**
- `run_as` takes the call's deadline once, before the body runs. An error from `deadline()` itself is `usage`, never a
  `Timeout`: T-green confirms this, and part 2's `config_is_validated_before_any_request` still pins it. `run_as` renames
  a `Timeout` and nothing else.
- For an id of 64 characters or fewer, `excerpt` returns exactly the old `{:?}` text. So existing messages are unchanged,
  and `adapter_test.rs`'s `w1:p2` assertions still pass.
- `ensure` is `ensure_pane`'s old body, moved without change.
- The harness fails closed:
  - every refusal and every failed proof panics after the guard exists, so the server is still stopped;
  - the adapter only ever gets the proven, canonical socket;
  - the adapter reads no environment.
- No shared state is added: part 2's AC 37 grep prints nothing.

**Build guards.**
- `src/` has no `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or `#[allow`.
- Both new test files open with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #640`, the crate's
  pattern, with its link.
- Every touched `.rs` file is under 900 lines. `lint.sh`'s size gate covers `crates/**/*.rs`.
- There is no dead code: clippy is clean with `-D warnings`.

**Protocol.** Nothing in Holler's protocol changes:
- no `holler-proto` file, no golden file, and no edit to `docs/protocol/v2.md`;
- no new error code. `grid-unreachable` was merged in part 1; this part adds only its ADR rows.

**Tests.**
- Real Herdr behaviour is tested against a real, throwaway server. The error messages are tested in process through the
  wire fake's `Tap`, the cheapest place that reaches the adapter's call path.
- Nothing waits on a fixed sleep. Every wait is the bounded `poll`, which checks before it sleeps.
- T-red has the RED evidence, quoted assertion by assertion.
- Each behaviour test fails without F's change, as RED and the two mutants show. Each guard test is shown to catch the
  failure it guards against: AC 2-7 by their own assertions, and AC 17 by T's extra mutant.

**Documentation.**
- `CHANGELOG.md` has an `## [Unreleased]` entry that links #640.
- `docs/testing.md` documents the opt-in test, its gate and its isolation.
- There is no new log event, CLI surface or protocol field, so no README change is needed.

**Public-repository privacy.** I grepped every added line of `origin/main...HEAD`, the brief and the handoffs included.
- I found no hostname, tailnet, IP, account, private domain, secret or key.
- The test literals are all generic (`/home/someone`, `/live/herdr.sock`, `/r/h640.ab`).
- The handoffs write `<home>` and `<scratch-root>`.
- The brief quotes the maintainer's name from part 2's journal. That text has been on `main` since `0ad2d8a`, and the
  same name is the author of every commit, so it is not a new disclosure.
- The gate files stay untracked (`.gitignore:22` and `:25`).

**Commit and PR hygiene.**
- The branch's commits are `docs(handoffs): ...` and the Workflow script's `chore(#640): <phase>` checkpoints. Squashing
  the PR at merge replaces them with one commit.
- Each commit carries a `Co-Authored-By:` trailer, and none has a session link. That matches `main`, where none of the
  last 60 commits has one. So `CONTRIBUTING.md`'s "and a session link" describes a practice the repo does not follow, and
  fixing that wording belongs to a docs change, not this PR.
- The PR's title, `Closes #640` and its AI disclosure are checked when the PR opens (advisory 1).

## Scope check

This part delivered exactly the brief's Files table.

- **T** wrote the harness, its test file, `adapter_messages_test.rs`, `Tapped::Fail` and the `Cargo.toml` comment.
- **F** changed `adapter.rs`, one word of `protocol.rs`, D1-D6, A1-A9, `docs/testing.md` and `CHANGELOG.md`.
- **Not touched**, as the brief requires: `transport.rs`, `holler-pane`'s code (doc comments only),
  `holler-pane-testkit/**`, `holler-cli/**`, part 2's test files and `tests/common/mod.rs`.

There are two small additions, each explained:
- F's private `ensure`, a move of `ensure_pane`'s body without change. F made it because rustfmt re-wrapped the body
  badly inside the closure.
- T's public `poll`, beyond the pinned items. A's finding 3 asks for one poll helper, shared by AC 13's read polls.

There is no over-delivery or under-delivery. The items the brief put out of scope are correctly absent: the test kit's
comments, a D7 and a line in the Deferred list.

## Verdict

**PASS.**
- All 32 criteria are met (AC 15 is N/A, since no gap was found).
- Decisions 1-16 are implemented as stated, with the declared deviations above.
- Code, test and documentation quality are acceptable.
- Ready for O: open the PR, apply advisory 1, verify CI, then merge.

## Advisory notes (non-blocking)

1. **When the PR opens** (Decision 1, `CONTRIBUTING.md`, the repo's CLAUDE.md):
   - Give it a title in the form of parts 1 and 2, because the title becomes the squash subject. For example:
     `feat(adapter-herdr): the opt-in scratch-Herdr test, the contract docs and the ADR-0021 rows (#640 part 3 of 3)`.
   - The body says `Closes #640`.
   - Add the AI disclosure with `gh pr edit` if the script's body lacks it.

   CI on the PR's merge ref is the final word for AC 28. `origin/main` has moved on to `d9eabbb`, and the merge with it
   is clean.
2. **Follow-ups for the operator.** Each one is outward-facing, so none is done here.
   - **(a)** File the #638 test-kit follow-up before or right after merge (the brief's item 4; A warn 6(c)). These
     `ASSUMPTION (#640)` comments describe the port docs as they were before D1-D3, so they become false when this
     merges:
     - `holler-pane-testkit/src/herdr.rs:272-274`;
     - `src/conformance/herdr.rs:189-196` and `:337-339`.

     No issue exists yet: a search of open issues finds only the epic.
   - **(b)** Add the harness's `run_bounded` to #696's scope (A-dup warn 1). #696 names only the host and OpenCode
     adapters.
   - **(c)** Optionally, add one clause to D4 and A4 naming `herdr.connect` (A warn 4).
   - **(d)** Optionally, add a D7 for the `snapshot` port doc and a Deferred-list line for A7's PROPOSED owner of
     `host.herdr_api_version` (A warn 6(a) and 6(b)).
3. **`run_bounded` waits for its stdout reader with no bound** (`tests/scratch_herdr/mod.rs:283`; A-dup warn 2).
   - The risk: if a `herdr` client command exited but left a child process holding stdout, `start` or `Drop` could run
     past the 15 s that the brief's Risks promise.
   - Why it is minor: the tests are opt-in only, no run has shown it (8 runs), and no live session is at risk.
   - The fix is T's: send the text over an `mpsc` channel and `recv_timeout` with the time left, or switch to #696's
     runner once it lands.
4. **A wire fake label is out of date.** `tests/wire_herdr/mod.rs:9-11` still calls `pane_not_found` for `send_text`,
   `send_keys` and `read` of a closed pane INFERRED. This run confirmed it against real Herdr 0.9.1, through conformance
   case 9 and AC 13's step 6. AC 15 requires a relabel only when a gap is found, so this is optional.
5. **A comment nit.** `tests/scratch_herdr/mod.rs:56` says "a 31-byte base made a 104-byte socket". With the 70-byte tail,
   a 104-byte socket means a 34-byte base, which is this machine's `TMPDIR` length. `handoff-T-red.md` and the journal
   also say 31. The code is unaffected, because `BASE_LIMIT` is computed from the constants.
6. **Where the diff gate's review is kept.** The second-opinion diff gate's complete review (the hand rerun, PASS) exists
   only in the run's session scratchpad, not next to round 1 in `docs/handoffs/`. Copy it to
   `docs/handoffs/640-diff-result-r2.md` (git-ignored) so a later reader can find it. I checked O's waiver of its two BLOCK
   entries:
   - B-1's premise is false. `Path::starts_with` matches whole components only, and AC 5's `/r/h640.abc/herdr.sock` case
     pins exactly the sibling it worries about.
   - B-2 retracts itself.
   - Its W-1 (use `strip_prefix`) is optional hardening, not a defect.
