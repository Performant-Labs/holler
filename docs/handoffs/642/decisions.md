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
