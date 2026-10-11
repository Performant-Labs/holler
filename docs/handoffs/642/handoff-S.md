# Handoff-S: Phase 10 - #642 part 3, the OpenCode adapter applies and verifies the pane's agent, and sends prompts  (spec audit)

**Date:** 2026-10-11 (11:45 MDT)
**Branch:** `issue-642-implementation` (worktree `.claude/worktrees/0642c-opencode-agent`, head `fa998d7`, merge-base `7284001`).
`origin/main` is now at `dc1398c` (#820, 11:33 MDT), one commit ahead. It overlaps this branch only in `CHANGELOG.md`, and
a trial merge (`git merge-tree --write-tree origin/main HEAD`) is clean.
**Issue:** #642 (part 3; epic #633). I read it live with `gh issue view 642`:
- Its body, through "AI assistance", is identical line for line to the brief's lines 3-80 (`diff` prints nothing). The
  issue holds an index of E1-E36, and the brief holds the cuts.
- Its three comments are the brief's "Comments from the issue thread". The third one's open check (does a child
  session's held question count?) is settled by D7(a) and pinned by acceptance 8.

**Brief:** `docs/handoffs/642-brief.md` as re-cut at `6752b70`.
**Handoffs reviewed (all under `docs/handoffs/642/`):**
- `handoff-A.md` (round 2; round 1's BLOCK is at `a4c9ca4`) and `handoff-A-dup.md`;
- `handoff-T-red.md` and `handoff-T-green.md`;
- `handoff-F.md`;
- `decisions.md` and `evidence.md`.

**Outside gates.** Each ran on `deepseek-v4-pro`, per its `.usage.json`.
- Brief gate: round 1 BLOCK, round 2 PASS (06:56 and 07:11 MDT). Both rounds reviewed the brief from before the re-cut:
  their prompts have `mode == "primary"` and no E33. See advisory 3.
- Diff gate, round 1 (11:24 MDT, on this diff and the re-cut brief): PASS, with no block.
  - Its NV-3 asked what `session_path` builds. It builds the bare, percent-encoded `/session/<id>` (`src/lib.rs:682`),
    so `send_prompt` posts to the bare route.

**Verdict:** PASS

## A precondition

Met.

- `handoff-A.md` (round 2, on the re-cut brief): PASS, with 0 blocks and 5 warns.
  - T acted on W-1, W-3 and W-4, and wrote the tests for W-2. F acted on W-2 and W-5. Each is checked below.
- `handoff-A-dup.md` (the diff `7284001..3f898ca`): PASS, with 0 blocks and 2 warns: a status fixture written twice,
  and two files within 60 lines of the 800-line flag. Neither blocks; see advisory 6.

## T precondition

Met. `handoff-T-green.md` reports "Blocking issues: None".

- **RED.**
  - Runtime RED, each case failing on an assertion: `prompt_test` 0 of 12, and `hermetic_test` 30 passed and 3 failed.
  - Compile RED occurred only at the new `AGENT_UNUSABLE` (A's W-3, following #808's precedent). Behaviour RED was then
    shown with a throwaway constant:
    - `agent_test`: 1 passed and 10 failed. The one pass is the regression pin of acceptance 2 and 3.
    - The four new or changed real tests: 0 of 4.
- **GREEN.**
  - Hermetic: `agent_test` 11/11, `prompt_test` 12/12, `hermetic_test` 33/33, `attach_test` 23 and `tui_test` 13.
  - The verify command: 2,142 passed, 0 failed, 28 ignored.
  - The opt-in real suite against OpenCode 1.19.0 and tmux 3.7c passed 12/12, in F's run and in T-green's second run.
    T-green's first run lost one test to a rig launch panic (advisory 5).
- **Mutations.** Each of M1-M11 turned a targeted test red.
- **Phase boundaries,** checked with `git diff --stat` on each phase commit:
  - T-red (`3d9ef9a`) touched only `tests/` and its handoffs.
  - F (`437fbc3`) touched only `src/`, the `ports.rs` comment, ADR-0021, `CHANGELOG.md` and its handoffs.
  - T-green and A-dup touched only `docs/handoffs/642/`.

## Acceptance criteria

Test paths are under `crates/holler-adapter-opencode/tests/`. "Mutant" names T-green's production mutation that the
test caught.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | With an agent, `serve_command` adds `OPENCODE_CONFIG_CONTENT={"default_agent":KEY}` (D2). Under `Inherit` the whole argv is pinned. Under `Isolated` the pair follows the configured pairs, and the adapter's pair follows a configured `OPENCODE_CONFIG_CONTENT=x`. The keys `plan`, `orchestrator` and `a-b_c` each appear in exactly one element. The server's prefix is the TUI's plus one pair, and removing that pair gives the `None` argv. | `agent_test.rs:65` `ac1_an_agent_is_the_servers_default_agent_on_its_argv_under_inherit` (the whole argv, `assert_eq!`). `:89` `ac1_under_isolated_the_pair_follows_the_configured_pairs`. `:115` `ac1_the_adapters_assignment_follows_a_configured_one_of_the_same_name` (configured pair, then the adapter's, then the binary). `:134` `ac1_each_key_is_carried_in_exactly_one_element` (and that element's JSON is exactly `{"default_agent":KEY}`). `:153` `ac1_with_an_agent_the_server_prefix_is_the_tuis_plus_one_pair` (both envs, against `tui_argv`). Mutant M1 | Met |
| 2 | Without an agent, the argv is byte for byte the one before this part, and nothing is sent | `hermetic_test.rs:495` `ac8_serve_command_is_the_server_argv_behind_the_env_prefix` pins both envs. I diffed its body against `7284001`, and it is unchanged. `agent_test.rs:178` `ac2_ac3_without_an_agent_nothing_is_applied_and_nothing_is_sent` checks that no pair is added and the stub saw nothing | Met |
| 3 | `verify_serving` with `None` is `Ok`, and the stub saw no request | `agent_test.rs:178`, the same test | Met |
| 4 | With `Some(plan)`, `/config` naming `plan`, and `/agent` listing `plan` as `primary` (with `hidden: null`, or with no `hidden` key) or as `all`, the answer is `Ok` after exactly `GET /config` then `GET /agent` | `agent_test.rs:196` `ac4_a_server_that_runs_the_agent_verifies`: three agent lists, and `stub.lines()` equals the two request lines | Met |
| 5 | Each mismatch is `agent-unusable`, in one line naming the key and D4's reason, with no reply body | `agent_test.rs:224` `ac5_a_server_that_does_not_run_the_agent_is_refused_agent_unusable`. The cases: default `build`, no `default_agent`, the agent absent, a `subagent`, `hidden: true`. The code is compared with `AGENT_UNUSABLE.as_str()`. Every reply carries a marker that the message must not contain. Mutants M2-M4 | Met |
| 6 | An HTML 200 on `/config` or `/agent` is `unavailable` naming the route and port. A refused port is `unavailable`. A frozen server is `timeout` with `harness.verify_serving`, within `call + SLACK`. An element without a string `mode` is `unavailable` | `agent_test.rs:282` `ac6_an_html_200_for_config_or_agent_is_unavailable`. `:301` `ac6_an_agent_without_a_string_mode_is_unavailable` (no `mode`, `null`, `7`). `:318` `ac6_a_refused_or_frozen_server_is_unavailable_or_timeout` compares the `op` with `HarnessOp::VerifyServing.as_str()` (D8) | Met |
| 7 | `send_prompt` | `prompt_test.rs:56`: the exact request lines on the bare route, a body equal to the text part with no `agent`, and `204` is `Ok`. `:81`: a 404 and a 400 are `session-not-found`, with no post. `:100`: a 500, or a 200 for another id, is `unavailable`, with no post. `:122`: an HTML 200, a 500 and a JSON 200 on the post, each echoing the posted body with the prompt in its first 60 bytes, are `unavailable`; the message is one line naming the route, the status and the port, and neither `Display` nor `Debug` carries the prompt. `:160`: a frozen server gives `harness.send_prompt`, pinned as a literal (D8). Mutants M5 and M11 | Met |
| 8 | `session_activity` | `prompt_test.rs:262`: a question and a permission of the session, and of a direct child, give `HoldingQuestion` while the status says `busy`, so the lists come first. `:294`: an item of another session, of another session's child, or of a grandchild gives `Busy`. `:323`: with `GET /session` answering HTML, an exact match counts and a child does not. `:375`: `busy`, `retry` and a missing `type` give `Busy`; `{}`, `idle` and another session's `busy` give `Idle`. `:418`: when the status is HTML, a 500 with an object body, or a 200 `[]`, the message list decides. `:441`: with neither usable, `unavailable`. `:160`: `harness.session_activity`. A's W-2 adds `:341` `ac8_a_list_that_cannot_be_read_holds_nothing`. Mutants M6-M10 | Met |
| 9 | `hermetic_test.rs` changed in place, a `retry` abort case added, no test file at 900 lines, the fixtures moved | `hermetic_test.rs:581` `ac8_a_spec_with_an_agent_is_applied_and_checked_on_the_server` replaces the `agent-unsupported` pin: no `agent-unsupported`, the argv carries the key, and the check asks `GET /config` first. `:394` `ac5_abort_of_a_session_that_stays_in_retry_times_out`. `:411` `ac5_abort_settles_once_a_retrying_session_reads_idle`. The largest test file is 746 lines. `support/fixtures.rs` holds D12's list plus A's four from W-4, and no test file copies them (A-dup). Mutant M6 | Met |
| 10 | `OpenCodeHarness` passes the conformance suite, with case 16 on the "argv carries the key" branch | `real_opencode_test.rs:58` `ac12_the_adapter_passes_the_harness_conformance_suite` (opt-in, through `Recording::launch`) passed in F's run and in T-green's second run (12/12 each). The stub conformance run that the criterion names does not exist: the suite needs a launcher and tmux (A's W-1). Acceptance 1's whole-argv pin is the hermetic stand-in for case 16's property | Met, on A's W-1 basis (advisory 4) |
| 11 | Real OpenCode: the agent reaches the first message; a subagent, a hidden agent and an unknown key are refused; a prompt and its activity | `real_opencode_test.rs:452` `agent_default_reaches_the_first_message`. The `plan` server is `Ok`. The no-agent server is refused `agent-unusable` with "the server's default agent is not plan". A raw `prompt_async` sends exactly the `hello` body. The first assistant message's `agent` and `mode` are `plan` on the first server, and on the second its own default, read from the raw `/agent`. `:502` `verify_serving_refuses_a_subagent_a_hidden_agent_and_an_unknown_key` covers `explore`, `title` and an unknown key. `:524` `send_prompt_queues_a_turn_and_session_activity_follows_it`: `Ok`, then `Busy` while the turn is in the raw `/session/status`, a raw abort, the raw message list showing `time.completed`, then `Idle` within `Timeouts.settle`. The rig: `Recording` forwards both methods (`rig.rs:300` and `:303`), and `serve` calls `serve_with` (`:419` and `:427`). `serve_with` runs `launch_on`, which runs the dead-end provider guard after every server. The raw abort is T's documented change, because D11's premise is false for 1.19.0 (Spec compliance, D11) | Met |
| 12 | Docs | Every site the criterion names is edited; Spec compliance lists them. `CHANGELOG.md:11` is under `## [Unreleased]` / Enhancements and links #633 and #642 | Met |

## Spec compliance

**The decisions, as implemented.**

- **D1.** The agent is the server's `default_agent`, set at start. `send_prompt` names no agent.
- **D2.**
  - `default_agent` (`server.rs:70`) builds the pair from `json!({"default_agent": key}).to_string()`, which is compact
    JSON.
  - `serve_command` (`:48`) hands the pair to `ProcessEnv::prefix_with` (`lib.rs:226`), which appends it after both
    prefix forms.
  - `prefix()` (`:212`) delegates with `&[]`, so `tui_argv` and the no-agent argv are unchanged. `serve_command` is the
    only caller that passes a pair.
  - `prefix_with`'s doc carries the rule "never put a secret here".
- **D3.** The real test shows the variable reaching OpenCode 1.19.0, and that a server it did not reach is refused. There
  is no config file and no agent per prompt.
- **D4.**
  - `verify_serving` (`server.rs:83`) returns for `None` before it builds a `Call`.
  - Otherwise one `Call` with `OP_VERIFY_SERVING` sends `/config` and then `/agent`.
  - The refusal messages are exactly D4's four reasons, built from the key alone.
  - A reply that is not the JSON a step needs goes through `Call::unexpected`, as D4 says.
  - `usable` (`:114`) applies OpenCode's own rule: it refuses a missing agent, a `subagent` and `hidden == true`, and it
    passes `primary` and `all`. An element without a string `mode` is `unavailable`.
- **D5.**
  - `AGENT_UNUSABLE` is at `server.rs:31`, built through `PaneError::Refused` (`unusable`, `:133`), and re-exported at
    `lib.rs:145` beside `pub use tui::...`.
  - The tests compare `.as_str()`.
  - `AGENT_UNSUPPORTED` stays in `holler-pane`.
- **D6.**
  - `send_prompt` (`prompt.rs:26`) runs `known` first, then posts the text part alone to the bare route.
  - It is `Ok` only on a 204. Any other reply goes through `Call::unexpected_status` (`lib.rs:538`), which quotes no body.
  - `unexpected` and `unexpected_status` share `answered` (`:545`), and `unexpected`'s text is unchanged.
- **D7.**
  - (a) `holds_a_question` (`prompt.rs:74`) reads both lists through `string_at`, and asks `GET /session` only when an
    item belongs to another session.
    - `child_ids` (`lib.rs:661`) sits beside `top_level_ids`, and both read `parentID` through one `ids_where`.
      `top_level_ids` behaves as before.
    - When `GET /session` is not such a list, only an exact match counts.
  - (b) `status_object` (`lib.rs:613`) and `running` (`:621`) sit beside `settled` (`:600`), which now uses them.
  - (c) `from_messages` (`prompt.rs:118`).
  - **The one gap the brief left.** A's W-2 found that D7(a) gives no rule for a `/question` or `/permission` answer that
    is not a 200 JSON array.
    - It is now read as "holds nothing". A refused connection or a timeout is still an error.
    - That is the reading D7(c)'s "older server, HTML page" fallback needs, since without it the fallback could never be
      reached.
    - It is recorded in F's handoff, pinned by `ac8_a_list_that_cannot_be_read_holds_nothing`, and stated in the crate
      doc and in ADR section 8. It is not a silent deviation.
- **D8.**
  - The agent code is in `server.rs`, and the prompt seam is in the new `prompt.rs`. `lib.rs` delegates to them in one
    line per method.
  - There are three new `OP_*` strings, and the table's comment names the two that have no `HarnessOp` yet.
  - The six new `Route`s are in the one route table, and `STATUS` and `LIST` are reused.
  - `refuse_an_agent` and the `AGENT_UNSUPPORTED` import are gone (grep).
  - `lib.rs` grew by 138 lines, against "about +60", to 741 lines, under the 900 gate. Each addition is an item that D8 or
    acceptance 12 puts there; A-dup checked them.
- **D9.**
  - `ports.rs` changes only the `AGENT_UNSUPPORTED` doc.
  - The `serve_command` and `verify_serving` docs there are untouched, and `holler-pane/src/lib.rs` is not edited.
- **D10.** Nothing to build.
- **D11.**
  - The rig's model guard runs after every server (`launch_on`). The headers of the rig and of the real test are
    rewritten.
  - One premise of D11 is false: OpenCode 1.19.0 does not end a turn whose provider refuses. It keeps retrying (`retry`;
    T-red's probe). So the tests end their turns with a raw abort, and the rig's header says so (`rig.rs:9-12`).
  - I read "a prompt is sent only by the agent test" as acceptance 11's three tests, since its third test must call
    `send_prompt`.
- **D12.** The fixtures moved, each with its own `allow`, and `attach_test.rs` is untouched.
- **D13.**
  - `send_prompt` is pure transport: no hold, no limit and no park.
  - ADR section 8 says the adapter now sends the first message, that the gate still does not see this path, and that
    #646 and #649 decide whether it moves.

**Acceptance 12's doc sites,** each checked in the diff:

- **The crate doc** (`lib.rs:1-126`):
  - the "until then ... refuse" sentences are replaced;
  - a bullet on the agent and one on the prompt seam are added;
  - the "No per-pane environment" bullet says what D2 requires: the one variable, on the server's argv only, only with
    an agent, carrying `{"default_agent":KEY}` alone, placed after the prefix's own words.
- **The five "one `env` prefix" docs:** `ProcessEnv`'s (`lib.rs:187`), `prefix`'s (`:210`), `serve_command`'s
  (`server.rs:40-47`, which no longer says a spec with an agent is refused), `tui_argv`'s (`tui.rs:163-167`) and the crate
  doc's fact-2 bullet (`lib.rs:51-61`).
- **The rule for secrets:** `prefix_with`'s doc (`lib.rs:224`).
- **`AGENT_UNSUPPORTED`'s doc** (`ports.rs:316-321`).
- **The headers** of `rig.rs` (`:7-12`) and `real_opencode_test.rs` (`:11-15`).
- **ADR-0021:**
  - decisions 4 (`:134`), 5 (`:143`) and 7 (`:162`);
  - facts 2 and 7 (`:217`);
  - "Launch and relaunch as built (#644)" (`:524-531`), in the same words as decision 5;
  - section 8 (`:497`), with D13's sentence;
  - the two Deferred bullets (`:862` and `:868`);
  - `agent-unusable` in the launch/relaunch row (`:642`), the `profile apply` row (`:653`) and the `Refused` row (`:704`),
    which also names the fake's own `agent-mismatch`.
- **A's W-5:**
  - the docs of `settled` and `SETTLE_POLL` are rewritten;
  - ADR section 8 says that a pending permission request reads as `HoldingQuestion`;
  - `Activity`'s doc stays as D9 requires.
- **Leftovers (grep).** Outside the handoffs, nothing still says "until #642", `refuse_an_agent` or "No prompt is ever
  sent". The one "for every agent" outside ADR-0021 is in #802's own CHANGELOG entry (`CHANGELOG.md:425`), a record.

## Quality audit

**Correctness and failure handling.**

- **Errors.** Every new request goes through `Call::send`, so:
  - a refused port is `unavailable`;
  - a frozen server is `timeout` with the method's `op`;
  - a garbled reply is `unavailable` with the client's own text. I checked `evidence.md`'s `http.rs:110-116` excerpt
    against source.
- **`verify_serving` fails closed.** Any reply that is not the JSON a step needs is `unavailable`, and the engine rolls
  back on any error. I checked `tx_launch/server.rs:107` and `tx_launch.rs:726-742` against `evidence.md`.
- **No prompt in a message.** `send_prompt` is `Ok` only on a 204. The one path that quoted a body on the post is now
  `unexpected_status`, and a `Garbled` text carries no reply bytes.
- **`session_activity` leans toward `Busy`.** An entry that is not an object, or that has no `type`, counts as running.
  A list that cannot be read holds nothing, but the status still reads a waiting turn as `Busy`.
- **`abort` is stricter by design** (D7(b)): a `retry` entry, or one with no `type`, is not settled. T's probe showed that
  OpenCode's abort removes a retrying session's entry, so an abort still settles.
- **Concurrency.** The adapter keeps no state, so there is no write to lose.
- **Checked by reading only: the pair through the host's `tmux new-window`.**
  - The argv goes after `-- env --` (`holler-adapter-host/src/tmux.rs:69-87`), so tmux runs it without a shell.
  - `escape` (`:152-157`) changes only a trailing `;`, and an `AgentKey` cannot contain one.
  - The rig launches without tmux (ADR-0021 section 5), so #649's wiring is the first real run (advisory 2).

**Build guards.**

- The added `src/` lines hold no `unwrap`, `expect`, `panic!`, `todo!` or `unsafe` (grep).
- Every added `#[allow]` is in a test file and carries `// #642`.
- The largest touched files are `hermetic_test.rs` (746 lines), `lib.rs` (741), `real_opencode_test.rs` (552) and
  `rig.rs` (545). None is at 900.
- The crate denies dead code, and clippy `-D warnings` is clean (T-green).

**Protocol.** No change. There is no wire, golden-file or `v2.md` change, and `HarnessPort`'s signatures are unchanged.
`agent-unusable` is an open refusal code, which ADR-0021's error-class table already maps to exit 3 (`:704`).

**Tests.**

- The cross-process behaviour is HTTP to OpenCode. The hermetic stub covers every branch, and the opt-in real suite
  covers what a stub cannot prove.
- No fixed sleep stands in for waiting on an outcome: the real tests poll (`observed`, and the settle loop). The one
  added fixed sleep (`ac19`, 500 ms) proves a negative, that nothing bound the port, and repeats the base's own check in
  the same opt-in test.
- The RED-first evidence is in `handoff-T-red.md`.

**Documentation.**

- The `CHANGELOG.md` entry is in place.
- There is no new log event, CLI surface or protocol field, so neither the README nor `docs/` needs a change. No user
  doc lists the pane refusal codes (grep); ADR-0021 is their one home.

**Public-repository privacy.**

- I grepped all 4,415 added lines (code, tests, ADR, CHANGELOG and the committed handoffs) for:
  - home paths;
  - user, account and machine names;
  - hostnames and tailnet names;
  - IPs other than loopback;
  - private domains;
  - secrets.

  I found none. The placeholders are generic (`/opt/oc/bin/opencode`, `/tmp/hlr802/...`, `<npm prefix>`).
- One identity appears: the brief's copy of the issue thread names the commenter by GitHub handle. That handle is public
  on the issue, and it is already in every branch commit's no-reply author address. It is not a leak; see advisory 1.

**Commit and PR hygiene.**

- The branch has 10 commits, each with a Conventional subject (`chore(#642): ...`) and a `Co-Authored-By` trailer.
- None has a session link. That matches main's #802 squash, which came from the same script.
- The PR does not exist yet. Per CLAUDE.md, the run's agent adds the AI disclosure to its body with `gh pr edit`
  (advisory 1).

## Scope check

Every change is within the blast radius:

- `crates/holler-adapter-opencode/**`:
  - `src/`: `lib.rs`, `server.rs`, the new `prompt.rs`, and `tui.rs` (a doc only);
  - `tests/`: `hermetic_test.rs`, the new `agent_test.rs`, `prompt_test.rs` and `support/fixtures.rs`,
    `support/stub.rs` (one `allow` comment), `real_opencode_test.rs` and `real_opencode/rig.rs`;
  - no `Cargo.toml` change, which is right: the crate does not set `autotests = false`.
- `crates/holler-pane/src/ports.rs`: a comment only (D9).
- `docs/adr/ADR-0021.md` and `CHANGELOG.md`, plus the pipeline's own `docs/handoffs/642*`.
- No other crate, and no hub, protocol, `holler-body` or `Cargo.lock` change.

**Over-delivery:** none. Three small refactors each serve something the brief or A asked for:
- `Call::answered` is shared by `unexpected` and D6's sibling;
- `ids_where` follows A's W-2;
- `assignment` gives both sources of pairs one `K=V` form.

**Under-delivery:** none. The out-of-scope items are untouched: the `--agent` wiring, the CLI wiring, `held_questions()`,
and the model and effort.

## Verdict

**PASS.**

- All twelve acceptance criteria are met. Each has a named test that asserts the behaviour, and M1-M11 show the tests
  fail without it.
- D1-D13 are implemented as stated. The one gap the brief left (A's W-2) is closed the way D7 implies, and the decision
  is recorded.
- Code, tests and docs are of acceptable quality.

Ready for O.

## Advisory notes (non-blocking)

1. **The PR body and the brief's thread copy.**
   - After the script opens the PR, add the AI disclosure line per `CONTRIBUTING.md` with `gh pr edit`, as CLAUDE.md
     says.
   - The brief's copy of the issue thread heads each comment with the commenter's GitHub handle. That is already public,
     but no file on main names it yet. If the operator prefers no handles in committed files, replace them with "the
     maintainer" before merge. That is a doc edit for the run's agent, not F.
2. **The pair through real tmux is not exercised yet.** `HostPort::run`'s `tmux new-window` path is sound by reading, but
   no test runs it. #649 should check it end to end, for example with `GET /config` of a server that `HostPort::run`
   started.
3. **The outside brief gate predates the re-cut.** Rounds 1 and 2 reviewed the brief before `6752b70`. The re-cut's
   changes (D4(b)'s `all`, D5-D9, D12, D13, and the acceptance edits) had two reviews: the in-session plan review (PASS)
   and the outside diff gate (PASS). They had no outside brief round. This is a process note for the orchestrator.
4. **The brief's wording, for the record** (no code change). If the issue text is edited again, correct three points:
   - Acceptance 10 names a stub conformance run that does not exist (A's W-1).
   - D11's "the assistant message may carry an `error`" assumes the turn ends. On 1.19.0 it retries until it is
     aborted.
   - D11 and acceptance 12 say "only by the agent test", but acceptance 11's third test sends a prompt too.
5. **The opt-in real suite can flake under load.** T-green's first full run lost `send_prompt_queues_…` to the panic in
   `serve_with` on `launch_on`, not to an adapter assertion. The test passed 3 of 3 alone, and the next full run passed.
   If it recurs, make the launch panic name its cause before tuning `BOOT`.
6. **A-dup's warns.**
   - If the test files are touched again, fold `retrying()` and `status_of` into `fixtures.rs`.
   - `lib.rs` (741 lines) and `hermetic_test.rs` (746) are near the 800-line flag. The next adapter brief should say
     where its code goes. A-dup's candidate split for `lib.rs` is the route table and `Call`.
7. **A session in `retry` can keep `pane reset` refused.** `retry` reads as `Busy` (D7(b), as decided).
   - `pane reset` then says the session "is running a turn; reset it once it ends", even while OpenCode keeps retrying a
     provider that refuses. T-red's probe saw 5 attempts over 40 s, the backoff growing, and no end.
   - Resetting would drop a live turn, so `Busy` is the honest answer. The operator's way out is to interrupt the turn
     first.
   - A later story could add a hint for this case to `check_idle`'s wording, which lives in `holler-pane`, outside this
     blast radius.
8. **A weak assertion (test only, optional).**
   - `ac7_a_post_not_answered_204_…` and the shared `assert_one_line_naming` check the status with
     `message.contains("200")` or `contains("500")`. An ephemeral port number can contain those digits.
   - The code does name the status. Anchoring the check on the status as its own word would make the test strict.
