# Handoff-T-green: Phase 7 - #642a the OpenCode adapter, server side (`serve`, `health`, `create_session`, `list_sessions`, `abort`, `http::request`)

**Date:** 2026-10-09
**Branch:** issue-642-implementation (round 2 on 9fc44f6, F's round-2 commit; round 1 was on 999d7e3)
**Issue:** #642, part 1 of 2 (642a)
**Handoff-F reviewed:** docs/handoffs/642/handoff-F.md (round 2)
**Handoff-T-red:** docs/handoffs/642/handoff-T-red.md (round 2, at 4c9b710)

## Round 2: verify after the rework for the outside diff gate's round 1

### What changed since round 1

- F's round-2 commit changes `src/http.rs` (B-1: a `taken` count in `Reader::fill`, checked before a read's bytes are
  kept), and docs only in `src/server.rs` (W-1) and `src/tui.rs` (A's N-1(e)).
- F changed no test file: `git diff --stat 4c9b710 9fc44f6 -- crates/holler-adapter-opencode/tests` is empty.
- F's "Tests that look wrong (for T)" note, the bare `"42"` check in
  `ac8_serve_of_a_program_that_exits_at_once_is_unavailable_with_its_status`, was already fixed in round 1. The check
  requires `42` as a whole number. Round 1's mutation M1 confirmed it, so no test change was made in this round.

### GREEN confirmation

```
$ cargo test -p holler-adapter-opencode
test result: ok. 30 passed; 0 failed; 0 ignored; finished in 1.05s
```

- **Flake check.** 8 concurrent runs of the test binary each gave 30/30 (1.02-1.03 s), and a `--test-threads=1` run gave
  30/30 (3.94 s). No `sleep 30` process and no `hlr642-*` directory was left afterwards.
- **Mutation spot-checks of F's new code.** Each change below was made by hand in `src/http.rs`, the suite was run, and the
  file was restored with `git checkout` (the tree is clean afterwards). Each mutation failed exactly the targeted test:

| Mutation | Test that failed |
|---|---|
| M6: the `taken` check disabled (`if false && self.taken > MAX_REPLY`) | `ac1_an_unframed_reply_past_64_mib_is_garbled_and_one_under_it_is_read` (29 passed, 1 failed) |
| M7: the cap counted 1 MiB low (`MAX_REPLY - (1 << 20)`) | the same test, on its under case (29 passed, 1 failed) |

  So the test pins the bound from both sides on F's new counting, which now includes the head. The under case is
  `64 MiB - 1024` of body plus a short head, and it still reads whole.

### Tier 1 results (round 2)

| Check | Command | Result |
|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0. This crate's only note is the 600-line warning on `hermetic_test.rs` (775, under 900 and under the brief's 800 split point). PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok`. PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0. PASS |
| Format | `rustfmt --check --edition 2021` on the crate's `src`, `tests` and `tests/support` files | exit 0. PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-opencode --no-deps` | clean. PASS |
| Workspace | `HOLLER_STATE_DIR=<scratch> cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0: 126 suites, 1413 passed, 0 failed, 5 ignored. That is round 1's 1410 plus the 3 adapter tests added since. PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed. PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | 3 passed. PASS |
| Unused deps | `cargo machete` | nothing found. PASS |
| Hooks | `bash scripts/test-hooks.sh` | exit 0. PASS |
| AC 24 | `grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode` | nothing (exit 1). PASS |
| No new `unsafe` | `git diff origin/main...HEAD -- crates \| grep '^+' \| grep -c unsafe` | 0. PASS |

**Cross-check against F.** Every command F reported in round 2 gives the same result here: 30/30, clippy, rustfmt,
rustdoc, lint, changelog, machete, the AC 24 grep and the `unsafe` count. For the workspace I ran only the isolated form.
The `logging_test` failures F saw without isolation come from a live local hub, and CI has none.

### Tier 2 (round 2 deltas)

- **Coverage.** Unchanged from round 1, plus round 2's tests:
  - the cap's over and under cases (AC 1, B-1);
  - the chunk extension and trailer (AC 1, NV-2);
  - the counting resolver in the two in-use and frozen `serve` tests (AC 8, NV-4).

  **PASS**
- **Test quality.** No new tests were written in this round. The round-2 tests fail alone for the right reason (T-red's
  M1-M4, and M6-M7 above). The 64 MiB test costs about 200 MB of transient memory, as T-red recorded. That is acceptable,
  and the 8 concurrent runs passed on this machine. **PASS**
- **API contract.** No public surface changed. `MAX_REPLY` stays private, and there is no test-only knob (A's N-3).
  **PASS**
- **Scope (AC 22).** `git diff --name-only origin/main...HEAD` still lists only the crate, `CHANGELOG.md`, `Cargo.lock` and
  `docs/handoffs/642*`. **PASS**
- **Evidence appendix.** The round-2 tests rely on `MAX_REPLY` and `fill`, which are in the diff, and on `httparse`'s
  chunk-size parse, which T-red already logged. No entries were added. **PASS**

### Acceptance criteria status (round 2)

The same as the round-1 table below, all PASS. AC 1 is now also backed by
`ac1_an_unframed_reply_past_64_mib_is_garbled_and_one_under_it_is_read` and
`ac1_a_chunked_reply_with_an_extension_and_a_trailer_reads_its_body`.

### Blocking issues (round 2)

None.

### Advisory notes (round 2)

- The round-1 advisories still stand:
  - `serve`'s success path has no hermetic test;
  - the macOS form of `kill -s KILL -- -<pgid>` is unverified until CI's macOS leg runs;
  - the port race is #644's.
- F's B-1 change counts the head and framing toward the cap, so the largest readable body is a little under 64 MiB. That
  is within the brief's bound, and the under test shows a body of `64 MiB - 1024` is still read.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.

---

# Round 1 (kept for the record)

## GREEN confirmation

F changed no test file (`git diff --stat 0ce98a4 999d7e3` touches only `src/`, `Cargo.toml`, `CHANGELOG.md`, `Cargo.lock`
and the handoffs).

```
$ cargo test -p holler-adapter-opencode
running 27 tests ... test result: ok. 27 passed; 0 failed; 0 ignored; finished in 1.02s
```

After this phase's two test changes (below), the suite has 28 tests:

```
$ cargo test -p holler-adapter-opencode --test hermetic_test
test result: ok. 28 passed; 0 failed; 0 ignored; finished in 1.03s
```

**Flake check.** I ran the test binary 12 times at once, and every run gave 28/28 (1.01 s). A `--test-threads=1` run also
gave 28/28 (3.56 s). No `sleep 30` and no `hlr642-*` scratch directory was left afterwards.

**Mutation spot-checks.** I made each change to `src/` by hand, ran the suite, and restored the file with `git checkout`
straight after. Each mutation failed exactly the test that targets it, and no other:

| Mutation | Test that failed |
|---|---|
| M1: the exit status left out of `serve`'s exited-early message (`server.rs:159`) | `ac8_serve_of_a_program_that_exits_at_once_is_unavailable_with_its_status` (this also shows the tightened "42" check means something) |
| M2: child sessions kept (`lib.rs:415`, `if true`) | `ac11e_child_sessions_are_left_out_of_the_list` |
| M3: no `DELETE` after a failed title PATCH (`lib.rs:208`) | `ac3_a_failed_title_patch_deletes_the_session_and_is_unavailable` |
| M4: the settle poll never sees `busy` (`lib.rs:398`) | `ac5_abort_of_a_session_that_stays_busy_times_out` |
| M5: the process-group kill on the deadline aimed at pgid 0, which `kill_group` refuses (`server.rs:167`) | the new `serve_kills_its_process_group_when_the_deadline_passes`, which failed with "the child process N outlived serve's timeout" and then killed the leftover itself |

### Test changes in this phase (tests only; no production code touched)

1. **F's fragility note, applied.** In `ac8_serve_of_a_program_that_exits_at_once_is_unavailable_with_its_status`, the
   check that the message holds the exit status 42 was a bare `contains("42")` on the message with the port removed. A
   later message that quoted the `hlr642-...` scratch path, or a pid holding the digits, would have passed it vacuously.
   It now requires `42` as a whole number: `message.split(|c| !c.is_ascii_digit()).any(|n| n == "42")`. Port removal is no
   longer needed. M1 confirms that the check still fails when the status is missing.
2. **New: `serve_kills_its_process_group_when_the_deadline_passes`.** It pins the brief's Behaviour rule for `serve` ("The
   deadline passing -> kill the process group it started ... and answer `timeout`"). F's Known issues noted that no
   hermetic test reached this path. Without it, a regression would leak a live server process on every failed `serve`.
   - **What it runs.** `sh` runs a scratch `serve` script that never binds the port. The script records its own pid and
     the pid of a background `sleep 30` (both in the group `serve` created), then waits.
   - **What it asserts.** With `call` at 1 s, `serve` answers `timeout` with `op == HarnessOp::Serve.as_str()` within
     `call` plus 300 ms. Both recorded processes are then gone (`kill -0` fails), polled for up to 2 s, because init
     reaps the orphaned `sleep` shortly after the kill. If either process is still alive, the test kills it itself
     before it fails.
   - **Why this tier.** It is the cheapest test that reaches the path, at the same tier as the other `serve` tests. It needs
     only `sh`, `sleep` and `kill`, so it runs on Linux and macOS.
   - **What it does not cover.** `serve`'s success path (a server that turns healthy) still has no hermetic test. That
     needs a child that binds and answers HTTP, which `sh` cannot do portably, so it stays with 642b's real-OpenCode rig
     (Advisory notes).

`hermetic_test.rs` is now 704 lines, under the 900-line lint limit and the brief's 800-line split point.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. This crate's only note is the 600-line warning on `hermetic_test.rs` (704); the other warnings are files elsewhere in the repo | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean (run again after the test changes) | PASS |
| Format | `rustfmt --check --edition 2021` on every `src/*.rs`, `tests/*.rs` and `tests/support/*.rs` file | exit 0 | exit 0 | PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-opencode --no-deps` | clean | clean | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Workspace | `HOLLER_STATE_DIR=<scratch> cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form) | exit 0 | exit 0: 126 suites, 1410 passed, 0 failed, 5 ignored (identical to F's isolated run) | PASS |
| Unused deps | `cargo machete` | nothing | nothing | PASS |
| Hooks | `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |

**Cross-check against F.** Every command F reported gives the same result here: 27/27 before my changes, clippy, rustfmt,
rustdoc, lint, changelog, machete, the AC 24 grep, and 1410/0/5 for the workspace. On the environment: I ran the workspace
only with `HOLLER_STATE_DIR` isolated, and did not repeat F's non-isolated run. The `logging_test` failures F reported there
come from a live local hub, and CI has none.

## Tier 2 results

- **Coverage of the 642a ACs.** Every behavioural AC has a test: AC 1-8, 11, 11d without its `attach_tui` clause, and 11e
  (table below). The gate ACs 20-24 are checked below. **PASS**
- **Test quality** (`test-quality.md` §7).
  - Every test is named for a behaviour, and each fails alone for the right reason (the RED run, and M1-M5 above).
  - All sit at one tier, the public API against a loopback stub, the cheapest tier that reaches real socket behaviour. No
    two tests cover the same thing.
  - The one test that cannot be RED at runtime, `ac11_the_adapter_is_send_sync_and_static`, is a compile-time pin and was
    declared as such at RED.
  - The suite is proportionate: 28 tests for 5 methods plus the HTTP client. I flag nothing for deletion. **PASS**
- **Type safety.** Rust, clippy `-D warnings` clean. No new `unsafe`: `git diff origin/main...HEAD -- crates | grep '^+' |
  grep -c unsafe` = 0. **PASS**
- **Error handling.**
  - Tested: refused, timed out, garbled, a 404 or 400 (session-not-found), a 500, a 200 that is HTML or another shape, a
    missing binary, a child that exits early, a deadline on a frozen server, and a deadline on a server that never boots.
  - Every reply-shape message is checked to be one line of at most 200 bytes. **PASS**
- **Data integrity.** A failed title PATCH deletes the session (AC 3, M3). An abort checks that the session exists first and
  sends no abort for an unknown id (AC 5, 11d). Child sessions are left out of the list (AC 11e). `serve` never adopts a
  server already on the port (AC 8) and leaves no process behind on its deadline (the new test). **PASS**
- **API contract.** The public surface is exactly the brief's API: `Resolver`, `ProcessEnv`, `Timeouts`, `OpenCodeConfig`,
  `OpenCodeHarness::new`, `http::{Reply, HttpError, request}`, `tui::{TmuxSocket, TmuxConfig}` and their re-exports. `exec`
  and `server` are private. The op strings are pinned through `HarnessOp::as_str`. **PASS**
- **Security.**
  - Only `127.0.0.1` is contacted.
  - A session id is percent-encoded in the request line (AC 11d, which pins the raw wire bytes).
  - Messages carry no environment value. **PASS**
- **AC 22 (scope).** `git diff --name-only origin/main...HEAD` lists only the following, with no test kit, no workspace
  `Cargo.toml` and no `holler-pane` file:
  - `crates/holler-adapter-opencode/**`
  - `CHANGELOG.md`
  - `Cargo.lock`, which only adds the crate's dependency list (`holler-pane`, `holler-pane-testkit`, `httparse`,
    `serde_json`)
  - `docs/handoffs/642*`

  **PASS**
- **AC 23 (changelog).** There is one `[Unreleased]` Enhancements entry. It links #642 and #633, says what the adapter does,
  and says that part 2 brings the opt-in real-OpenCode tests. F records why it is worded that way (handoff-F Deviations),
  and S judges whether that meets AC 23. **PASS**
- **AC 24 (safety).**
  - `grep -rn "4700[0-9]\|--continue" crates/holler-adapter-opencode` finds nothing (exit 1).
  - No test names a fixed port: the stub binds `127.0.0.1:0`, and a refused port is bound and then dropped.
  - No test reads HOME, uses tmux or prints a pane title.
  - The new test signals only the pids its own script recorded, and only while they are still alive. **PASS**
- **Migration safety, Playwright, browser surface.** N/A: no schema, no UI.
- **Evidence appendix.** The tests rely on these facts in unchanged code: the `HarnessOp` strings, the one-line message
  rule, the I5 bound and OpenCode's HTML catch-all. `evidence.md` already lists each of them (F's entries 2, 5, 7 and 8). The
  new test relies on nothing outside the diff and the brief. No entries added.

## Acceptance criteria status

| AC | Status | Backing test(s) |
|---|---|---|
| 1 | PASS | `ac1_content_length_and_chunked_replies_read_to_the_same_bytes`, `ac1_a_closed_port_is_refused_within_a_second`, `ac1_a_frozen_server_times_out_within_the_timeout`, `ac1_a_reply_that_is_not_http_is_garbled` |
| 2 | PASS | `ac2_health_is_true_for_the_healthy_json`, `ac2_health_is_false_for_unbound_frozen_html_unhealthy_and_500` |
| 3 | PASS | `ac3_create_session_titles_the_session_with_its_id`, `ac3_a_failed_title_patch_deletes_the_session_and_is_unavailable`, `ac3_a_patch_reply_with_another_title_is_unavailable` |
| 4 | PASS | `ac4_list_sessions_returns_the_listed_ids_in_order` |
| 5 | PASS | `ac5_abort_of_an_unknown_id_is_session_not_found_and_sends_no_abort`, `ac5_abort_of_an_idle_known_session_is_ok`, `ac5_abort_of_a_session_that_stays_busy_times_out` |
| 6 (642a clause) | PASS | `ac6_server_calls_to_an_unbound_port_are_unavailable` |
| 7 | PASS | `ac7_the_call_bound_caps_the_request_timeout` |
| 8 | PASS | `ac8_serve_refuses_a_port_that_already_answers_healthy`, `ac8_serve_of_a_missing_binary_names_it`, `ac8_serve_on_a_frozen_port_times_out_and_spawns_nothing`, `ac8_serve_of_a_program_that_exits_at_once_is_unavailable_with_its_status` |
| 11 | PASS | `ac11_the_adapter_is_send_sync_and_static`, `ac11_default_timeouts_are_10s_5s_2s_500ms_5s` |
| 11d (without its `attach_tui` clause) | PASS | `ac11d_an_html_200_for_the_session_is_unavailable_and_sends_no_abort`, `ac11d_a_session_reply_for_another_id_is_unavailable_and_sends_no_abort`, `ac11d_an_html_200_for_the_abort_is_unavailable`, `ac11d_an_html_200_for_the_list_is_unavailable`, `ac11d_a_session_id_is_percent_encoded_in_the_request_line` |
| 11e | PASS | `ac11e_child_sessions_are_left_out_of_the_list` |
| 20 (642a clause) | PASS | `cargo test --workspace` exit 0 (Tier 1) |
| 21 | PASS | clippy, rustfmt, no new `unsafe`, lint, machete (Tier 1 and 2) |
| 22 | PASS | the diff scope check (Tier 2) |
| 23 | PASS (S to confirm the wording) | the CHANGELOG entry (Tier 2) |
| 24 | PASS | the grep and the rig review (Tier 2) |
| Behaviour: `serve`'s deadline kill (no AC number) | PASS | `serve_kills_its_process_group_when_the_deadline_passes` |

ACs 9, 10, 11a-c, 11f, 12-19a, 25 and 26 are 642b's and are not in this run.

## Blocking issues

None.

## Advisory notes

- **`serve`'s success path has no hermetic test.** That path covers the returned pid, the pid being its own process
  group, the detached reaper, and the server outliving the adapter. F checked it by hand against a python fake (in
  handoff-F). It needs a child that answers HTTP, so 642b's real-OpenCode rig (AC 19's `serve` cases and the guard's group
  kill) is the natural place to pin it.
- **`kill -s KILL -- -<pgid>` on macOS.** The kill path's command is checked on Linux only (procps-ng 4.0.4).
  `serve_kills_its_process_group_when_the_deadline_passes` now runs on the macOS CI leg and will show whether BSD `kill`
  accepts that form. If it fails there, the fix belongs to F (`exec.rs`), not the test.
- **The port race in `serve`** (handoff-F Known issues) is inherent to choosing a port, and #644's registry addresses it.
  No 642a test targets it.
- **For O:**
  - The issue's 2026-10-09 amendment (`Pane.opencode_agent`, which depends on #700) is in neither the brief nor 642a, as F
    notes.
  - A's W-4(c) follow-up and W-3(2)'s PR-body items are still open.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
