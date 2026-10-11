# Handoff-F: Phase 6 (Workflow script; the role doc's Phase 5), round 3 - #802 the diff gate's B-1: `verify_serving` answers for the spec, not for whether the server is up

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, on `2a32fc8`)
**Issue:** #802 (closes #695)

Earlier rounds of this handoff:
- Round 1, the whole #802 implementation: `git show 7ede9c7:docs/handoffs/802/handoff-F.md`.
- Round 2, the health wait's deadline check: `git show f27d5d0:docs/handoffs/802/handoff-F.md`.

Both still describe every other file of the PR. This round answers the outside diff gate,
`docs/handoffs/802-diff-result-r1.md`: BLOCK on B-1 (B-2 to B-4 were withdrawn), plus NV-1 to
NV-4, W-1 to W-3 and NIT-1 to NIT-3. This round changes no behaviour and edits no test. Nothing is
committed, because the Workflow script commits this phase.

| Field | Value |
|---|---|
| GitHub issue | #802 |
| Working branch | `issue-802-implementation` |
| Build plan phase | Workflow Phase 6 (F), after T-green PASS (round 2) -> the outside diff gate BLOCK (B-1) |
| Input documents read | `802-diff-result-r1.md`; `802-brief.md` (decision 5, Scope 2 and 3, AC 7, Out of scope); `handoff-T-green.md`; `decisions.md`; `evidence.md`; round 2's `handoff-F.md`; ADR-0021 section 2 (decisions 5 to 7) and the "Launch and relaunch as built" paragraph |
| Acceptance criteria count | 12. This round touches AC 7 (verify) and AC 12 (the docs) |
| Handoff document path | `docs/handoffs/802/handoff-F.md` |

## What was done

- **`crates/holler-pane/src/ports.rs` (B-1).** `HarnessPort::verify_serving`'s doc no longer says
  that every implementation answers an unreachable server as `unavailable` or `timeout`. B-1 read
  that sentence as a duty to check that the server is up. The doc now says:
  - The method answers for the spec, not for whether the server is up. The engine's health wait
    has just seen the server answer, and a server gone since then fails the engine's next call to
    it (the session of record's), which rolls back the same way.
  - An implementation sends a request only for what it must check on the server, and that request
    answers an unreachable server as any call does.
  - A spec that the implementation's own argv already settles is answered `Ok(())` with nothing
    sent. That is OpenCode's answer to a spec without an agent.
- **`crates/holler-adapter-opencode/src/lib.rs` (B-1).** A doc on `OpenCodeHarness::verify_serving`
  says why it sends nothing (#802 decision 5): the argv `serve_command` built applies no agent, so
  the server keeps its own default, and the health wait has just seen it answer. With an agent it
  answers `agent-unsupported`, failing closed, until #642 part 3. The code is unchanged.
- **`crates/holler-pane/src/tx_launch/server.rs` (B-1).** In step 4 of the module doc and in
  `start`'s doc, "`verify_serving` asks the running server" became "the harness says whether the
  running server honours the plan's `ServeSpec`". Step 4 also says what happens to a server that
  is gone after the wait.
- **`crates/holler-pane/src/tx_launch.rs` (W-1).** `Plan::observe`'s doc listed five reads under
  "Steps 3 to 6". It now reads "first the server's argv (#802), then steps 3 to 6: ...", so the
  numbering of #644's steps holds again.
- **`docs/adr/ADR-0021.md` (B-1).**
  - Decision 5 gains the same two sentences as the trait doc: the method answers for the spec, not
    for the server being up, and the OpenCode adapter's `Ok(())` sends nothing.
  - The launch paragraph's "asks the running server whether it honours the spec" became "has the
    harness say whether the server honours the spec".
- **`CHANGELOG.md` (B-1).** In the #802 entry, "asks the running server whether it honours the spec"
  became "has the harness say whether the running server honours the spec".
- **`docs/handoffs/802/evidence.md`.** Three new entries:
  - `session_of_record` (`tx_launch.rs:676-684`): the engine's next call to the server after
    `verify_serving`, inside `bring_up`, whose errors all go through `roll_back`;
  - OpenCode's `create` (`lib.rs:451-452`): `create_session` sends through `Call::send`;
  - OpenCode's `Call::send_by` (`lib.rs:409-415`): a refused connection is `unavailable`, and no
    reply in time is `timeout`.

  The rollback entry's fact now names this case too. A script checked all 13 excerpts against
  their cited lines, and all match.

## Design decisions

- **B-1's remediation was not applied as written. The contradiction B-1 found is fixed in the
  contract text instead.** The reviewer asked `OpenCodeHarness::verify_serving` to call
  `health(port)` and require `Ok(true)` for a spec without an agent. Four things rule that out:
  1. **The brief says the opposite.** Decision 5: "here the OpenCode implementation checks nothing
     beyond `Ok(())` for a spec with no agent". Scope 3: "`verify_serving` returns `Ok(())` for a
     spec without an agent". AC 7: "a launch with no agent calls `verify_serving` and it returns
     `Ok(())`". Out of scope: "implementing `verify_serving` for real (#642 part 3)".
  2. **T pins it.** `hermetic_test::ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies`
     asserts that the stub server received nothing (`stub.lines().is_empty()`). T added that
     assertion at round 1's Phase 7. A health call would turn it RED, and F may not edit it.
  3. **It would duplicate the engine's health wait.** `server::start` calls `verify_serving` right
     after `wait_healthy` returned `true`. A second health ask cannot see anything that one did not.
  4. **The failure B-1 describes is already caught.** If the server dies between the wait and
     `verify_serving`, the next call to it, `create_session` (or a relaunch's `list_sessions`),
     gets a refused connection and answers `unavailable`. That call is inside `bring_up`, so it
     rolls back and stops the session's processes, just as a `verify_serving` refusal does. The
     evidence entries show each step. The only difference is which call reports the dead server.
     AC 7 concerns a *refusal* from `verify_serving`, and that path is unchanged.

  What B-1 did show from the prompt alone was the trait doc's sentence "one that cannot be reached
  answers as any call to it does (`unavailable`, `timeout`)". Read as a rule for every
  implementation, it contradicts the brief's OpenCode answer. That sentence was mine, so I fixed
  it. The trait doc, the ADR's decision 5, the engine's module doc and the OpenCode doc now all say
  what the brief decided.
- **The contract's rule is "what the argv settles, nothing is asked".** A `None` agent means "the
  server's own default" (`ServeSpec.agent`'s doc). An argv that carries no agent therefore starts a
  server that honours it by construction. The fake still compares its running server's agent,
  reaching the port, because its scenario method can start a server from any argv. The contract
  allows that: it says what an implementation *need* send, not what it must skip.
- **W-1 was applied, because it was my own doc's drift.** NIT-1 was not. `tui_argv` is `pub`, and
  its doc shows the whole argv a caller gets. `ProcessEnv::prefix` is `pub(crate)`, so the public
  doc cannot point to it and keeps the full form. NIT-2 was not applied either. A zero `poll` asks
  back to back, but the loop-top deadline check still ends the wait, and every caller passes
  `BOOT_POLL`. NIT-3 is not a defect, by the reviewer's own note. W-2 stays as it is, as the
  reviewer itself recommends.

## Reuse / extend-vs-new

No new object, and no code path changed. Docs and evidence only, on the objects round 1 extended
(`HarnessPort`, `OpenCodeHarness`, `tx_launch::server`).

## Architecture notes for A

None structural. No signature, module boundary or dependency changed, and no implementation's
behaviour changed. `verify_serving`'s documented contract was brought in line with the brief's
decision 5, which A approved in rounds 1 and 2 (W-3). The doc now:
- states that `verify_serving` does not report whether the server is up;
- says that an implementation may answer what its own argv settles without a call.

Both the fake and the OpenCode adapter met this before and still meet it. A may want to read the
new paragraph at Phase 8, since #642 part 3 builds on it.

## Deviations from spec / wireframe

None. The diff gate's suggested remediation would have been a deviation from the brief (decision 5,
Scope 3, AC 7 and Out of scope), so it was not applied (see Design decisions).

## Tier 1 self-check (incl. tests now GREEN)

```text
$ cargo build --workspace --tests                                   -> Finished, exit 0
$ cargo clippy --workspace --all-targets -- -D warnings             -> exit 0
  (after touching both crate roots: cargo clippy -p holler-pane -p holler-adapter-opencode
   --all-targets -- -D warnings re-checked both, exit 0)
$ bash scripts/lint.sh                                              -> exit 0 (size warnings only)
$ bash scripts/changelog-check.sh                                   -> changelog-check: ok
$ cargo machete                                                     -> no unused dependencies
$ rustfmt --edition 2021 --check crates/holler-pane/src/{ports,tx_launch}.rs \
    crates/holler-pane/src/tx_launch/server.rs crates/holler-adapter-opencode/src/lib.rs  -> exit 0
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane -p holler-pane-testkit \
    -p holler-adapter-opencode                                      -> exit 0
$ cargo test -p holler-pane --doc                                   -> 5 passed (both ServeSpec compile_fail doctests: NV-3)
$ cargo test -p holler-cli --test docs_cli_test                     -> 3 passed
$ cargo test -p holler-adapter-opencode --test hermetic_test        -> 31 passed
  (ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies ... ok: nothing sent)
$ cargo test -p holler-cli --test pane_verbs                        -> 261 passed
$ cargo test -p holler-pane-testkit                                 -> every target ok
  (fake_harness_test 23, harness_conformance_test 15, host_conformance_test 12, fake_host_test 12)

# The pipeline's verify command (whole workspace):
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
148 binaries, passed=2073 failed=0 ignored=25, exit 0     # the same counts as round 2
```

The opt-in `real_opencode_test` was not re-run. This round changes no code path, only doc comments,
and round 2's run (9/9 against opencode 1.19.0) was on the same code.

## Evidence appendix

`docs/handoffs/802/evidence.md`. Round 3 added three entries (listed under What was done) and
widened the rollback entry's fact. Every excerpt, old and new, was re-checked by script against its
cited lines, and all 13 match.

## Tests that look wrong (for T)

None. One note on the diff gate's W-3 (`fake_tmux_test.rs`, `run_escapes_the_cwd_for_tmux`): it
asks for a comment or an assertion that `fake.dir_str()` holds no `#` or `;`, so that the expected
`-c` value cannot drift with the fixture. The test is not wrong. The suggestion is T's to take or
leave.

## Known issues

- **The diff gate's needs-verification findings.**
  - NV-1: the AC 6 tests run in the workspace command above.
  - NV-2: the order of `run`'s checks (session, then argv, then `cwd`) is the code shown in the
    diff. `run_checks_the_session_before_the_argv` covers the first pair. No test combines a missing
    session with a bad `cwd`; that is T's call.
  - NV-3: settled by `cargo test -p holler-pane --doc`. Round 1 also compiled both literals outside
    the crate and saw `error[E0639]`.
  - NV-4: needs the opt-in real run, 9/9 in round 2.
- **Carried from rounds 1 and 2, unchanged.**
  - W-11(a): no test starts the server under a tmux pty. It goes with #649.
  - W-13: the merge order with #664.
  - W-15: the plan's reads have no budget check between them. This predates #802.
  - `tx_launch.rs` is 820 lines. The lint warns at 600 and fails at 900.

## Files changed

This round:
- `crates/holler-pane/src/ports.rs` (doc only)
- `crates/holler-pane/src/tx_launch.rs` (doc only)
- `crates/holler-pane/src/tx_launch/server.rs` (doc only)
- `crates/holler-adapter-opencode/src/lib.rs` (doc only)
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

The PR's production files, all rounds (round 1's list, unchanged):
- `crates/holler-pane/src/{ports.rs,lib.rs,tx_launch.rs,tx_launch/server.rs}`
- `crates/holler-pane-testkit/src/{harness.rs,host.rs,conformance/harness.rs,conformance/host.rs}`
- `crates/holler-adapter-opencode/src/{lib.rs,server.rs,exec.rs,tui.rs,attach.rs}`
- `crates/holler-adapter-host/src/{lib.rs,tmux.rs}`
- `crates/holler-cli/src/pane/wiring.rs`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Handoff files:
- `docs/handoffs/802/handoff-F.md` (this file);
- `docs/handoffs/802/evidence.md` (three entries, one fact widened);
- `docs/handoffs/802/decisions.md` (appended).
