# Handoff-S: Phase 10 - #642a the OpenCode adapter, server side (spec audit)

**Date:** 2026-10-09 (audited 19:05-19:20 MDT)
**Branch:** issue-642-implementation (worktree `.claude/worktrees/0642-opencode-adapter`, head `4b60b80`, merge base `3bdd129`).
`origin/main` moved twice during the run: `e612878` (#701, 18:56 MDT) and `ce12cdb` (#703, 19:13 MDT). See advisory 2.
**Issue:** #642 (epic #633), read live with `gh issue view 642`. Last edited 17:40 MDT, when the "agent" amendment was added.
**Brief:** `docs/handoffs/642-brief.md`, unchanged since `5a4d68e` (17:11 MDT). **This run is 642a only**, so the PR says `Part of #642`.
**Handoffs reviewed:**
- `handoff-A.md` (the re-review at `c052d02`), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` and `handoff-A-dup.md` (rounds 1 and 2 of each);
- `decisions.md` and `evidence.md`;
- the outside diff gate's rounds 1 (BLOCK) and 2 (PASS), `642-diff-result-r{1,2}.md`, and the brief gate's round 3 (PASS). All three are git-ignored, and the model was `deepseek-v4-pro`.

**Verdict:** PASS. Before the merge, the run's agent must fix two things: the PR body has to say `Part of #642`, and the CHANGELOG conflict has to be resolved (advisories 1 and 2).

## A precondition

Met. Phase 3's re-review at `c052d02` returned PASS: 0 blocks, 3 new warns (N-1 to N-3), and the 8 earlier warns carried
forward. A-dup returned PASS at `884d772`, with 0 blocks and 2 warns (D-1, D-2).

## T precondition

Met. `handoff-T-green.md` reports "Blocking issues: None" in both rounds.

- **RED:** T-red round 1 (`0ce98a4`) ran against the brief's API, stubbed. Result: 1 passed, 26 failed, every failure an
  assertion. The one that passed is AC 11's compile-time pin, and T declared it as such.
- **GREEN:** T-green round 2 (`884d772`) gave 30 of 30. The workspace in CI's form, with `HOLLER_STATE_DIR` isolated, gave
  1413 passed, 0 failed, 5 ignored.
- **Mutations:** M1-M7 and the round-2 M1-M4 each failed exactly the test they target. The round-2 tests pass against
  existing code by design, and T-red says so and backs it with those mutations.
- **Since T-green:** only docs changed. `git diff --stat 884d772..HEAD` lists only `decisions.md` and
  `handoff-A-dup.md`.

## Acceptance criteria

Every test is in `crates/holler-adapter-opencode/tests/hermetic_test.rs`. "S" marks a read-only check I ran myself. Per my
role, I did not re-run Tier 1 or Tier 2.

| Criterion | Proving test or evidence | Status |
|---|---|---|
| Issue scope: `health` is a real round trip with a short timeout, and a wedged server reads as unhealthy | AC 2's frozen case | Met |
| Issue scope: `create_session` is the only way a session of record is made | AC 3. `serve` never adopts a server (AC 8) | Met |
| Issue acceptance: passes the conformance suite from #638 | 642b's AC 12. Not in this run | Not this run (`Part of #642`) |
| Issue acceptance: against scratch `opencode serve`: create, list, abort, switch, report shown; a deleted session is reported | 642b's AC 13 and 14. Not in this run | Not this run |
| **Issue amendment (17:40 MDT, operator-confirmed):** the pane's `opencode_agent` is applied as the server's `default_agent` or per prompt, plus one extra conformance case | Not in the brief and not in 642a. It depends on #700 (OPEN): `Pane.opencode_agent` is not on `main`, and that field belongs to #700 alone (epic #633). A's N-1 | Not this run. #642 must stay open (advisories 1 and 4) |
| AC 1 | `ac1_content_length_and_chunked_replies_read_to_the_same_bytes`, `ac1_a_closed_port_is_refused_within_a_second`, `ac1_a_frozen_server_times_out_within_the_timeout`, `ac1_a_reply_that_is_not_http_is_garbled`. Also F's reply bound and the trailer: `ac1_an_unframed_reply_past_64_mib_is_garbled_and_one_under_it_is_read`, `ac1_a_chunked_reply_with_an_extension_and_a_trailer_reads_its_body` | Met |
| AC 2 | `ac2_health_is_true_for_the_healthy_json`, `ac2_health_is_false_for_unbound_frozen_html_unhealthy_and_500` (it also covers `{"healthy":false}`) | Met |
| AC 3 | `ac3_create_session_titles_the_session_with_its_id` (the raw request lines are exactly POST then PATCH, and the PATCH body's title is the id), `ac3_a_failed_title_patch_deletes_the_session_and_is_unavailable`, `ac3_a_patch_reply_with_another_title_is_unavailable` | Met |
| AC 4 | `ac4_list_sessions_returns_the_listed_ids_in_order` | Met |
| AC 5 | `ac5_abort_of_an_unknown_id_is_session_not_found_and_sends_no_abort` (both 404 and 400, so it also covers the Behaviour rule that a 400 reads as a 404), `ac5_abort_of_an_idle_known_session_is_ok`, `ac5_abort_of_a_session_that_stays_busy_times_out` (`op` equals `HarnessOp::Abort.as_str()`) | Met |
| AC 6 (the 642a clause, which is all of AC 6 in this run) | `ac6_server_calls_to_an_unbound_port_are_unavailable` (each message also names the port) | Met |
| AC 7 | `ac7_the_call_bound_caps_the_request_timeout` | Met |
| AC 8 | `ac8_serve_refuses_a_port_that_already_answers_healthy`, `ac8_serve_of_a_missing_binary_names_it`, `ac8_serve_on_a_frozen_port_times_out_and_spawns_nothing`, `ac8_serve_of_a_program_that_exits_at_once_is_unavailable_with_its_status`. The test runs `false` to show the quick return and a script that exits 42 to show the status, which it requires as a whole number. The marker file shows the server's working directory | Met |
| AC 11 | `ac11_the_adapter_is_send_sync_and_static` (compile-time), `ac11_default_timeouts_are_10s_5s_2s_500ms_5s` | Met |
| AC 11d (without its `attach_tui` clause) | `ac11d_an_html_200_for_the_session_is_unavailable_and_sends_no_abort`, `ac11d_a_session_reply_for_another_id_is_unavailable_and_sends_no_abort`, `ac11d_an_html_200_for_the_abort_is_unavailable`, `ac11d_an_html_200_for_the_list_is_unavailable` (each checks one line, at most 200 bytes, the route and the status), `ac11d_a_session_id_is_percent_encoded_in_the_request_line` (the raw wire line) | Met |
| AC 11e | `ac11e_child_sessions_are_left_out_of_the_list` | Met |
| AC 20 (the 642a clause) | T-green round 2: `cargo test --workspace` in CI's form exits 0. Only docs changed since then (S) | Met |
| AC 21 | T-green: clippy `-D warnings`, rustfmt 2021, `lint.sh`, machete and rustdoc are all clean. S: the added code lines contain 0 `unsafe` (the 22 hits in the whole diff are all prose under `docs/handoffs/`, which the PR step removes); there is no `features` list; the largest touched file is 775 lines | Met |
| AC 22 | S: `git diff --name-only origin/main...HEAD` lists only the crate, `CHANGELOG.md`, `Cargo.lock` and `docs/handoffs/642*`. Nothing in `holler-pane`, the test kit or the workspace `Cargo.toml` changes. `Cargo.lock` adds only the crate's four dependencies. I used three dots because `main` moved | Met |
| AC 23 | S: there is one entry under `[Unreleased]` / `Enhancements` (line 167). It links #633 and #642, says what the adapter does, and says that part 2 brings the opt-in real-OpenCode tests. That is A's W-5 reading of a part-1 entry. T-green: `changelog-check: ok` | Met |
| AC 24 | S: the `4700[0-9]\|--continue` grep finds nothing. No test uses a fixed port: the stub binds `127.0.0.1:0`, and `closed_port` binds a port and then drops it, which is the brief's own hermetic design (A's W-5). No test reads HOME, and none runs tmux (`tmux_bin` is `/nonexistent/...`) or prints a pane title. The only processes started are `false`, `sh` on scratch scripts, and `kill` on pids the test's own script recorded | Met |
| Behaviour without an AC number: on the deadline, `serve` kills the process group it started | `serve_kills_its_process_group_when_the_deadline_passes` | Met |
| Behaviour: `workdir` is resolved only after the health GET refuses | The counting resolver in the two `ac8_serve_*` in-use and frozen tests | Met |

The tests assert behaviour: error codes and payloads, `op` strings, raw request lines, request bodies, process liveness and
bounds. Removing a behaviour fails a test, as the RED run and the mutations show. No test passes by construction except
the declared compile-time pin.

## Spec compliance

I read every file in the crate in full and checked it against the brief's API, Behaviour and Decisions.

- **The public API is exactly the brief's.**
  - `Resolver`, `ProcessEnv` and `Timeouts` (with their defaults), and `OpenCodeConfig` with all seven fields.
  - `OpenCodeHarness::new` and the `HarnessPort` impl.
  - `pub mod http` (`Reply`, `HttpError { Refused, TimedOut, Garbled }`, `request`) and `pub mod tui` (`TmuxSocket { Default, Name, Path }`, `TmuxConfig`), both re-exported.
  - `exec` and `server` are private, and `MAX_REPLY` and every helper are private.
- **Behaviour, for each method:**
  - **The deadline.** Every method sets a deadline at entry, and every request is cut to what is left of it.
  - **HTTP outcomes.** Refused is `unavailable`, "the harness server on port N". `TimedOut` is `timeout` with
    `harness.<method>`. A garbled reply, or a status the step does not expect, is `unavailable` naming the route.
  - **Address.** Only `Ipv4Addr::LOCALHOST` is contacted.
  - **Reply shapes.** The required shape is checked for health, `POST /session` (an `id` starting with `ses`), the PATCH
    title, the list (an array of objects, each with a string `id`), `GET /session/:id` (the same `id`), the abort (`true`)
    and the status (an object).
  - **Ids.** A session id is percent-encoded outside `[A-Za-z0-9_-]`.
  - **400 reads as 404**, but only on the existence check.
  - **`create_session`** sends POST, then the PATCH. A failed PATCH is followed by a best-effort DELETE, and the PATCH's
    error is returned.
  - **`list_sessions`** returns top-level sessions only. A string `parentID` marks a child.
  - **`abort`** checks the session, then aborts, then polls the status within `settle`.
  - **`serve`:**
    - one health GET first, and it never adopts a server;
    - then the resolve, then the spawn in its own process group, with null stdio and the env per `ProcessEnv`;
    - only health GETs about every 150 ms;
    - exit-first is `unavailable` with the status;
    - on the deadline it kills the group, then `Child::kill` and `wait`;
    - on success it returns the pid and a detached reaper takes the child. `server.rs`'s module doc restates the brief's
      W-4 rule that one owner reaps each child.
  - **The TUI methods.** The three TUI methods answer `NotImplemented`, as the split table says.
- **Decisions 2, 11, 12, 13, 14 (the `workdir` half) and 15(a)** are built and stated in the crate docs. Decision 10's
  `TmuxSocket` mirrors #641's variants exactly (A-dup checked it against the #641 branch). Decisions 1, 3-7 and 9 are
  642b's, and this diff leaves them alone (ADR-0021 and `holler-pane` are untouched).
- **Changes from the brief. Each is documented, and none is silent:**
  - `HttpError::Garbled`'s doc is widened to cover a connect failure other than a refusal or a timeout, so `serve` never
    starts a server it cannot poll (F, Deviations). The variants are unchanged.
  - `tempfile` is not declared, because 642a does not use it (A's W-6).
  - `serve` lives in `server.rs` and the stub in `tests/support/stub.rs`. Both are the brief's own named splits.
  - Any answer to `serve`'s first health GET means the port is in use. The brief specifies only the healthy answer, so this
    fills a gap (F's decision 4).
  - The 64 MiB reply bound and the 64-header bound are F's decision 9. They are not in the brief, and A asks O to record
    them at the next amendment.
  - The CHANGELOG has a part-1 entry (A's W-5).
  - `serve_kills_its_process_group_when_the_deadline_passes` runs `kill` on the macOS leg. The brief's Risk 6 says "must
    not call ... `kill`". T-green states the choice and its reason: only POSIX `sh`, `sleep` and `kill` are used, and the
    test checks the production kill path on macOS. This is a risk note, not a decision (advisory 9).
- **Against the issue:** the only gap is the "agent" amendment. It cannot be built before #700 merges, and a later part of
  #642 has to plan it (see Verdict and advisory 4).

## Quality audit

- **Correctness and failure handling.** I traced each failure branch: refused, frozen, garbled, cut short, too long, a
  wrong status, a wrong shape, the HTML catch-all, a missing binary, early exit, the deadline and a `try_wait` error. Each
  one fails closed with a closed code. I found no defect. Two more checks:
  - An id that collides with a route fails closed instead of aborting: `status` gets `GET /session/status`, an object
    with no `id`, and the answer is `unavailable`.
  - The adapter keeps no state, so no write can be lost under concurrency. The two-callers port race in `serve` is
    inherent to choosing a port. It is documented, and #644's registry gives each pane its own port.
- **Build guards.**
  - `src/` has no `unwrap`, `expect`, `panic!`, `todo!` or `unreachable!` (S grep).
  - The one `#![allow]` is in the test file and carries `// #642`.
  - There is no `unsafe`.
  - The files are 33-483 lines in `src/`, and 243 and 775 in `tests/`, all under 900.
  - Clippy is clean with dead code denied.
- **Protocol.** None: no wire method, field, error code, golden file or `docs/protocol/v2.md` change.
- **Tests.** The tests run against a loopback stub and real child processes (`sh`, `false`, `kill`), which is the cheapest
  tier that reaches the socket behaviour. Nothing synchronises with a fixed sleep: the one wait is a 2 s bounded poll on
  process liveness. The stub records each request before it answers, so no assertion on `lines()` races. The RED-first
  evidence is in T-red.
- **Documentation.**
  - The CHANGELOG entry matches the code, including `not-implemented` for the TUI and "#649 wires it in".
  - No new log event, CLI surface or protocol field exists, so no README or `docs/` change is due.
  - The crate docs carry decisions 1, 2, 13, 14 and 15(a) and the part-1 status.
  - Two pointers will dangle on `main`, and one comment is inaccurate (A-dup's D-1). See advisories 7 and 8.
- **Public-repository privacy.**
  - I grepped every added line, the handoffs included, for personal names, hosts, tailnet names, IPs, accounts, private
    domains and secrets. No hits. The only IPs are `127.0.0.1` and `192.0.2.1` (TEST-NET-1). The only URLs are the public
    repo, `opencode.ai/config.json` and loopback. The `~/.cargo` and `~/.rustup` paths name no account.
  - `gitleaks git --log-opts=origin/main..HEAD` scanned 15 commits: no leaks.
  - The outside gate's prompt and result files are git-ignored.
- **Commit and PR hygiene.**
  - The commits are the pipeline's Conventional `chore(#642): ...` subjects, plus one merge commit. Each has a
    `Co-Authored-By` trailer. Like `main`'s recent squash commits, none has a session link (advisory 11).
  - No PR exists yet. When the script opens one, the title and body need the fixes in advisory 1.

## Scope check

- **Delivered:** the 642a row of the split table, all of it.
  - `Cargo.toml` and `src/lib.rs` with the five server methods, and the three TUI methods answering `NotImplemented`.
  - `src/http.rs`; `src/exec.rs` with only the runner and `kill`; `src/tui.rs` with only the two types.
  - `CHANGELOG.md`, `Cargo.lock` and `hermetic_test.rs`. 642a has no `real_opencode_test.rs`, by design.
- **Over-delivery:**
  - The brief's named splits (`server.rs` and `support/stub.rs`).
  - F's private reply and header bounds.
  - T-green's group-kill test, which pins a specified Behaviour.
  - Nothing unrelated was touched.
- **Under-delivery:** none against 642a. Against #642 as a whole, 642b remains, and so does the agent amendment, which
  the brief does not place anywhere (advisory 4).

## Verdict

**PASS.** Every 642a criterion has a test that asserts the behaviour. The brief's API, Behaviour and decisions are built as
stated or with a documented change, and the code quality is acceptable.

**Why not ADVISORY-HOLD.** #647's S held its run over this same operator-confirmed amendment
(`docs/handoffs/647/handoff-S.md` on `main`). This run is different in three ways:

- The brief makes 642a a part. Its PR leaves #642 open, so this PR can drop nothing.
- Nothing in this diff contradicts the amendment. The agent, as a `serve`-time `default_agent`, can be added later as one
  more resolver (A's N-1(c)), and nothing outside the crate constructs `OpenCodeConfig` yet.
- The amendment depends on #700, which is open.

The defect sits in the 642b row of the brief's split, which reads `Closes #642` without the agent part. That is for the
amendment before 642b (advisory 4).

**The PASS assumes the PR does not close #642.** The script opens it with the body `Closes #642.`, so advisory 1 is
required before the merge.

## Advisory notes (non-blocking for F; items 1 to 3 are required of O or the run's agent before the merge)

1. **The PR title and body (required).** `coding-pipeline.workflow.mjs:4820-4822` opens the PR with title `Implements #642`
   and body `Closes #642.`.
   - **Body.** Before merging, edit it with `gh pr edit` to say `Part of #642`, never `Closes`. Otherwise the merge closes
     #642 while 642b and the agent part are still open. The body also needs:
     - the AI disclosure (`CONTRIBUTING.md`);
     - divergence 15(a), citing #695, and that the ADR-0021 note lands with 642b (A's W-3(2));
     - the agent part as #642's open remainder, waiting on #700 (A's N-1).
   - **Title.** Give the squash a Conventional subject, as `main` does, for example
     `feat(adapter-opencode): the server side of HarnessPort over OpenCode's HTTP API (#642 part 1 of 2)`.
2. **The CHANGELOG conflict (required).** `git merge-tree origin/main HEAD` (at `ce12cdb`) conflicts only in
   `CHANGELOG.md`: #701, #703 and this branch each add an `[Unreleased]` Enhancements entry after the #688 entry. Keep all
   of them. Neither new `main` commit touches this crate, `holler-pane`'s ports, errors or `lib.rs`, the test kit, or
   `Cargo.lock`, so no code interaction is expected. Re-run CI on the merged result.
3. **File the `FakeHarness` parity follow-up (required).** This is A's W-4(c). It is still not filed:
   `gh issue list --search "FakeHarness parity"` finds nothing.
4. **Before 642b, amend the brief to place the agent part (A's N-1).** Put it in 642b, or in a 642c after #700 merges.
   The amendment states:
   - the agent resolver and its precondition (decision 14);
   - the rule under `Inherit` for an operator's own `OPENCODE_CONFIG_CONTENT`;
   - the crate-local test that stands for "one extra conformance case". The shared suite cannot observe an agent, and that
     test sends a prompt, which the rig forbids today.

   Until then, 642b's row (`Closes #642`) would drop an operator-confirmed requirement, which is the same defect as #647's
   hold.
5. **Before 642b: N-2.** `hermetic_test.rs` is at 775 lines and `lib.rs` at 483. 642b's TUI bodies need a private module,
   and its builder and parser tests need a new test target.
6. **Keep the brief for 642b.** The PR step's cleanup (`coding-pipeline.workflow.mjs:3126`, `git rm -r docs/handoffs/642
   docs/handoffs/642-*`) removes `642-brief.md` and these handoffs from the PR, and the branch is deleted on merge. Before
   that, save a copy of the brief and of the N-1, N-2, D-1 and D-2 notes for 642b's worktree. #647 kept its brief on
   `main` for part 2.
7. **Pointers that will dangle on `main`.** `lib.rs:37` cites "the decisions of `docs/handoffs/642-brief.md`", a file the
   cleanup removes. `exec.rs:1` ("decision 11 of the brief") and `server.rs:27` ("the brief") do the same. The test
   comments at `hermetic_test.rs:243, 265, 510` cite the outside gate's B-1, NV-2 and NV-4, which are never committed.
   The decisions are restated inline, so nothing is lost. When 642b's F adds ADR-0021 §2's "`HarnessPort` as built
   (#642)" note, repoint these comments at it. `main` already has the same pattern (`control_status.rs` cites
   `506-brief.md`).
8. **A-dup's D-1 and D-2.** Correct the comments at `http.rs:9-10` and `stub.rs:5-6` at their next edit. `holler-body`
   has a blocking status probe (`query.rs:239`), and the cousin fake could be included with `#[path]`. Record the
   differences between the two `kill` runners on #696.
9. **CI watch: the macOS `kill`.** `serve_kills_its_process_group_when_the_deadline_passes` is the first test to run
   `kill -s KILL -- -<pgid>` on BSD `kill`. If it fails there, the fix is F's, in `exec.rs`, as T-green says.
10. **CI watch: zombies (TEST-ONLY if it fires).** `alive()` (`hermetic_test.rs:627-634`) uses `kill -0`, which succeeds
    on a zombie. The ubuntu leg runs on the self-hosted runner for same-repo PRs. If that host's init (or a container
    without one) does not reap the killed orphan `sleep 30`, the test fails after 2 s. If it fires, `alive()` should
    treat a `Z` state (`ps -o stat= -p <pid>`) as gone.
11. **Minor.**
    - `OP_LIST_SESSIONS` equals `HarnessOp::ListSessions.as_str()` by inspection, but no test pins it. `lib.rs:83-84` and
      the evidence entry both say "the tests pin". A frozen-stub `list_sessions` case beside AC 7 would close that.
    - `serve`'s success path has no automated test in 642a: the pid being its group id, the reaper, and the server
      outliving the adapter. F checked it by hand. 642b's AC 12 (`serve-then-healthy`) and AC 19 must cover it.
    - The cap test holds roughly 200-300 MB while it runs. That is fine on the runners.
    - `CONTRIBUTING.md` describes a session link beside `Co-Authored-By`. If one is available, the squash commit is where
      it would go.
