# Decision journal: #642 part 3, the OpenCode adapter applies and verifies the pane's agent, and sends prompts

Run handoff directory `docs/handoffs/642` · branch `issue-642-implementation` · rigor second-opinion · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry. The brief gate's rounds are in
`docs/handoffs/642-brief-result-r{1,2}.md` (round 1 BLOCK, round 2 PASS).

## A, up-front plan review: 2026-10-11T07:23-06:00 (MDT)

- **Decided:** BLOCK, with 2 `block` and 9 `warn` findings (`docs/handoffs/642/handoff-A.md`).
  - Block 1, the base: the plan extends objects that exist only in #802's unmerged tree (`3fe111d`).
    - `origin/main` is `263a2d7`, `HarnessPort` still has `serve(name, port) -> u32` (`ports.rs:186`), and no `.rs`
      file under `crates/` names `ServeSpec`, `serve_command` or `verify_serving`.
    - #802 is OPEN and in flight in `.claude/worktrees/0802-serve-command`; its tip `0dc4e8b` amended #802's brief after
      its T-green.
    - Building here means tests that do not compile, then an unplanned rebuild of #802 (40 files under `crates/`,
      +2,604/-1,029) outside the blast radius.
  - Block 2, D4(b): `mode == "primary"` is stricter than OpenCode 1.19.0's own `default_agent` rule.
    - That rule refuses only a missing agent, `mode === "subagent"`, or `hidden === true`.
    - A config agent defaults to `mode: "all"`, so the plan would refuse a valid agent as "a subagent (mode all)".
  - The warns:
    3. The rig's `Recording` must forward `send_prompt` and `session_activity`; otherwise acceptance 11's activity
       check cannot fail.
    4. One status predicate for `settled` and `session_activity`.
    5. Decide whether a child session's question counts (`holler-body`'s `session_in_scope` does).
    6. `OP_*` names for the two new methods, which have no `HarnessOp` variant; where the new `Route`s go.
    7. Re-export `AGENT_UNUSABLE` from the private `server` module.
    8. A body-free failure for `prompt_async`, so the prompt cannot leak through the 60-byte excerpt.
    9. The five "one `env` prefix" doc sites, `prefix_with`'s no-secret rule, and ADR-0021's launch paragraph.
    10. One `tests/support/` home for the hermetic fixtures.
    11. The section 8 carve-out (`pane reset --first` bypasses #646's gate) becomes executable here; the ADR says so,
        and the adapter stays pure transport.
- **Assumed:**
  - The automated script stops this run with `needs-amendment`, and recovery is a fresh run after #802 merges, not a
    resume.
  - The merged #802 will keep the shapes `3fe111d` shows (the brief says the merged code wins). If it does not, the
    `3fe111d` line numbers in my findings need re-reading.
  - The handoff and this entry are committed by the script's phase commit, so I did not commit them.
- **Hedged:**
  - Block 2 overrides the letter of an operator decision ("Each is final"). I rated it `block`, not `warn`, because the
    predicate contradicts the server the check mirrors (cut from the installed binary, as E32 is), and T would pin it
    in tests before any code exists. The operator may keep "primary only", but must say so and fix the message for
    `"all"`.
  - Finding 5 is a `warn`: `pane reset` refuses either way, and only the operator's message is wrong.
  - Not verified, and not a finding: which agent a prompt typed into the attached TUI runs as. D1 covers hub-delivered
    prompts (`prompt_async`, no `agent`), and E21 probed only that path. The OpenCode bundle shows `opencode run --attach
    --agent` checking the agent against the server's list, which suggests the client reads agents from the server, but
    I did not trace the TUI's initial choice.
  - The `env` ordering was checked on this machine's uutils `env` 0.8.0, not on CI's GNU or BSD `env`; POSIX applies
    the operands in order.
- **Evidence:**
  - Read: the brief (all 1,401 lines, E1-E32 and the thread), the gate results r1 and r2, and the role's references
    (`workflow-coding-pipeline.md` Phase 4 and the `arch` stop, `pipeline-conventions.md` section 1,
    `parallel-agent-coexistence.md`).
  - Code at `3fe111d`, exported with `git show`: the adapter's `lib.rs`, `server.rs`, `tui.rs` and `http.rs`;
    `hermetic_test.rs`, `attach_test.rs`, `real_opencode/rig.rs` and `support/stub.rs`; the test kit's `harness.rs`
    and `conformance/harness.rs`; `holler-pane`'s `ports.rs` and `tx_launch/server.rs`; ADR-0021.
  - Code at `263a2d7`: `ports.rs:184-187`, `tx_switch.rs:204-222` and `:310-335`, `holler-body`'s `connection.rs:375-432`,
    `holler-adapter-herdr`'s `plan.rs:30-42` and `lib.rs`, `say_cmd.rs:215-223`, and `holler-pane`'s `error.rs:1-30`.
  - Commands:
    - `git fetch`, `git merge-base`, `git worktree list`, `gh issue view 802`, and `git diff --stat` /
      `--shortstat 263a2d7...3fe111d`;
    - greps for `ServeSpec`, `serve_command`, `verify_serving`, `RefusalCode::from_static`, `HarnessOp`, `send_prompt`
      and `session_activity`;
    - `wc -l` over every touched file;
    - the `env` ordering check;
    - a scan of the installed `opencode-ai` 1.19.0 binary for the agent `mode` literals, the config agent's default
      (`mode:"all"`, then `t.mode=s.mode??t.mode`) and the `default_agent` checks;
    - the repo's own `opencode.json` agent modes (`orchestrator` is `primary`).

## A, up-front plan review, fresh run on the re-cut brief: 2026-10-11T10:39-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 5 `warn` findings (`docs/handoffs/642/handoff-A.md`, which replaces round 1's
  BLOCK; that one stays in the history at `a4c9ca4`).
  - Round 1's block 1 is closed: the base is `7284001` (#802 merged), and all 32 file excerpts of the brief match the tree.
  - Round 1's block 2 is closed: D4(b) is OpenCode's rule, and E33's seven cuts each occur once in the 1.19.0 binary.
  - Round 1's nine warns are folded into D2, D5 to D8, D12, D13 and the acceptance list, and each was checked against
    the merged code.
  - The new warns, all closable by T and F without a brief amendment:
    1. Acceptance 10's "existing conformance use for the stub run" does not exist; the only conformance run is the
       opt-in `real_opencode_test.rs:56-60`. T builds none; acceptance 1 pins case 16's property.
    2. D7(a) has no rule for a `/question` or `/permission` answer that is not a 200 JSON array. Recommended: read it as
       nothing held, as for `GET /session` and as in `holler-body`. Refused and timed-out stay errors. The children
       reader goes beside `top_level_ids`.
    3. D5's tests name `AGENT_UNUSABLE`, which F creates. That is compile RED, against the RED note. Follow #808's
       precedent, with a throwaway stub proving behaviour RED.
    4. D12 omits `ID`, `known_session`, `harness()` and `agent()`, which the new test files need. Move them too.
    5. The docs of `settled` and `SETTLE_POLL` go stale under D7(b). The ADR section 8 edit should say a pending
       permission request reads as `HoldingQuestion`.
- **Assumed:**
  - A PASS lets the script go on to T, with no amendment step, so the warns are written for T and F to act on.
  - The script commits this handoff and entry at the phase commit, so I did not commit them.
  - A question or permission request keeps the session's status `busy` while it waits, and for a subagent's question
    the parent's too. This rests on how OpenCode runs a tool inside a turn and on round 1's reading; I did not probe it
    live in this review.
- **Hedged:**
  - Finding 2's recommended rule (best-effort) is my reading of D7's own exceptions and `holler-body`. The operator may
    prefer fail-closed. The handoff says what changes then.
  - Finding 3 is about the test-first process, not architecture drift. I kept it because it comes from D5's decision
    and T meets it at once.
  - Settled since round 1 (it was hedged there): the server sorts `/agent` with the configured `default_agent` first,
    and the TUI's local state falls back to the first agent that is neither a subagent nor hidden, so an attached TUI
    starts on the pane's agent. Not traced: a selection the TUI remembers for a session it has shown before.
- **Evidence:**
  - Read: the whole brief (1,560 lines), round 1's `handoff-A.md` and this journal, gate result r2, and the role's
    references (Phase 5's RED rule in `workflow-coding-pipeline.md`, section 1 of `pipeline-conventions.md`).
  - Code at `7284001`:
    - the adapter: `lib.rs`, `attach.rs:1-80`, the `Garbled` texts of `http.rs`, `hermetic_test.rs:1-170` and
      `:440-813`, `real_opencode_test.rs:1-60`, and `rig.rs:190-335` and `:363`;
    - the test kit: `harness.rs:40-73`, `conformance/harness.rs:440-505`, and `herdr.rs:44-56`;
    - `holler-pane`: `ports.rs:280-331`, `tx_launch/server.rs`, and `tx_launch.rs:715-745`;
    - the host adapter: `lib.rs:40-56`, and `tmux.rs:60-160`;
    - `holler-body`: `connection.rs:385-450`;
    - `docs/adr/ADR-0021.md:140-165`, `:205-215`, `:494-502`, `:609`, `:620` and `:671`;
    - `docs/handoffs/808/handoff-T-red.md:92-118`.
  - Commands:
    - `git fetch`, `git merge-base`, `git log` and `git diff --stat`;
    - a script that compared every E-section with `sed -n A,Bp` of its file (32 of 32 match);
    - `wc -l` over the touched files;
    - greps for `run_harness_conformance`, `verify_serving`, `send_prompt`, `session_activity`, `OpenCodeHarness` and the
      D12 fixture names;
    - a scan of the installed 1.19.0 binary for:
      - E32's and E33's texts (each once);
      - the `question.list`, `permission.list`, `session.list`, `app.agents`, `config.get`, `session.prompt_async` and
        `session.messages` routes;
      - where `OPENCODE_CONFIG_CONTENT` is merged;
      - the default-first `/agent` sort and the TUI's choice of agent.

## T, author tests / RED: 2026-10-11T10:50-06:00 (MDT)

- **Decided:** RED is valid. F may start.
  - **Runtime RED:**
    - `prompt_test`: 12 of 12 fail on an assertion.
    - `hermetic_test`: 3 fail, the rewritten agent test and the two `retry` abort cases. The other 30 pass.
  - **Compile RED** only at `AGENT_UNUSABLE`, in `agent_test` and `real_opencode_test` (A's W-3, after #808's
    precedent).
  - **Behaviour RED** proven with a throwaway constant, never staged:
    - `agent_test`: 10 fail and 1 passes, the regression pin of acceptance 2 and 3.
    - The four new or changed real-OpenCode tests all fail on `agent-unsupported` or `prompt-unsupported`.
  - **A's warns acted on:**
    - W-1: no hermetic conformance run.
    - W-2: a list that cannot be read holds nothing, and a test pins it.
    - W-4: the fixtures moved to `tests/support/fixtures.rs`, with D12's list plus `ID`, `known_session`, `harness`
      and `agent`.
  - **Acceptance 11's third test** ends the turn with a raw abort. A live probe showed OpenCode 1.19.0 retrying the
    dead-end provider indefinitely (`retry`), so the turn never ends on its own. The test also checks live that `retry`
    reads `Busy`.
- **Assumed:**
  - The prompt-leak test needs no change to the stub. A canned reply that is byte-identical to the posted body stands
    in for an echo. If F's body differed, the first `send_prompt` test would fail anyway.
  - `send_prompt` treats a JSON 200 on the post as "not 204", so it is `unavailable` (D6: "any other status").
  - A raw abort that ends the turn in a real test does not conflict with "the turn's end read from the raw server". The
    end is still read from the raw message list.
- **Hedged:**
  - The `retry` behaviour is OpenCode 1.19.0's. A later version that fails the turn instead would still pass, because
    the abort is harmless on a finished turn and `time.completed` is set either way.
  - The real-test default agent for a server with no agent is computed from the raw `/agent`, not hardcoded as `build`.
    The probe saw `build`.
- **Evidence:**
  - Commands, all on the base `src/`:
    - `cargo test -p holler-adapter-opencode --test {prompt_test,hermetic_test,agent_test,attach_test,tui_test}`;
    - `HOLLER_TEST_OPENCODE=1 cargo test --test real_opencode_test -- --ignored --test-threads=1` for the four new
      tests, plus `ac13` and `ac17`;
    - `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings`;
    - `cargo fmt --check`;
    - `bash scripts/lint.sh` (exit 0).
  - Two live probes of `opencode serve` 1.19.0, with the rig's dead-end config, a scratch `HOME` and ports
    48177-48179:
    - with `default_agent` `plan`, the assistant message carries `agent` and `mode` `plan`, and the status reads
      `retry`, attempts 1 to 5 over 40 s;
    - with none, `/config` has no `default_agent`, the turn runs as `build`, and a raw abort answers `true`, after
      which the status is `{}` and the message has `time.completed`.
  - Handoff: `docs/handoffs/642/handoff-T-red.md`.
