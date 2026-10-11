# Handoff-F: Phase 6 (Workflow script; the role doc's Phase 5) - #802 `HarnessPort::serve_command` takes a `ServeSpec`; the harness server runs in the pane's tmux session

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, on `26148ba`)
**Issue:** #802 (closes #695)

Implemented against T's RED suite (`handoff-T-red.md`) and A's thirteen warns (`handoff-A.md`), with
W-1 and W-2 applied as binding. No test file was edited. Nothing is committed: the Workflow script
commits this phase.

## What was done

**`holler-pane`**
- `src/ports.rs`: `ServeSpec` (`#[non_exhaustive]`, `Debug + Clone + Default + PartialEq + Eq`,
  `pub agent: Option<AgentKey>`, builder `with_agent(self, AgentKey) -> Self`) beside `HerdrSpec`,
  with the two `compile_fail,E0639` doctests on its doc. `HarnessPort::serve` replaced by the pure
  `serve_command(&PaneName, u16, &ServeSpec) -> Result<Argv, PaneError>` and the required
  `verify_serving(u16, &ServeSpec) -> Result<(), PaneError>`. `HostPort::run(name, cwd, argv)`.
  `AGENT_UNSUPPORTED` beside `PROMPT_UNSUPPORTED`. Module and trait docs record the amendment.
- `src/lib.rs`: re-exports `ServeSpec`; the "frozen" paragraph names the two amendments.
- `src/tx_launch.rs`: one `ServeSpec` per run, built in `Plan::new` as an exhaustive literal (W-5);
  `Plan::observe` calls `serve_command` first and keeps the argv in `Seen`; `TxOptions.boot_poll`
  (`BOOT_POLL`, 150 ms); `Budget::expired` and `Budget::deadline`; the `port-in-use` refusal is one
  helper (`port_in_use`) for the plan and the act; `bring_up` passes the spec's `cwd` to both
  `run`s and calls the child module for the server; launch writes `harness.pid: None`; the
  "(#695)" survivor message and comment are gone. Module docs updated.
- `src/tx_launch/server.rs` (new child module, declared `mod server;` as `reconcile.rs` declares
  `observe`): `pub fn wait_healthy` (re-exported as `tx_launch::wait_healthy`, the one wait, W-4),
  `stop_old` (B1/B2, moved here from the parent), and `start`: budget check, the `port-in-use`
  re-check, `host.run` of exactly the planned argv in the spec's directory, the health wait to the
  budget's end (`timeout` with the verb's op), budget check, `verify_serving`.

**`holler-pane-testkit`**
- `src/harness.rs`: `HarnessOp::ServeCommand` / `VerifyServing` (`harness.serve_command`,
  `harness.verify_serving`) in place of `Serve`; `ServerView.agent`; `FakeHarness::serve_command`
  (through the fault switch, records each spec, returns
  `holler-fake-harness-server --pane P --port N [--agent K]`); `verify_serving` (reaches the port,
  then `Ok` iff the spec's agent is the running server's, else the fake's own refusal
  `AGENT_MISMATCH`, `agent-mismatch`); scenario method `start_server(&Argv) -> Option<Result<u32,
  PaneError>>` (today's `World::serve` rules, `None` for any other argv, bypasses faults and the
  log); inspection `serve_specs()`. Module and struct docs updated (W-8d).
- `src/host.rs`: `run(name, cwd, argv)`; `runs() -> Vec<(PaneName, String, Argv)>` (W-8a); docs.
- `src/conformance/harness.rs`: `run_harness_conformance(fresh, launch)` with
  `launch: Fn(&S, &PaneName, &Argv) -> Result<(), PaneError>`, then `wait_healthy` (every
  `BOOT_POLL`, within `DEFAULT_BUDGET`); case 2 renamed `launch-then-healthy` (checks the port is not
  healthy between `serve_command` and the launcher; the pid check is gone); case 16
  `serve-command-carries-the-agent`. Docs and the `text` usage block updated.
- `src/conformance/host.rs`: every `run` passes the system temp dir as `cwd`.

**`holler-adapter-opencode`**
- `src/lib.rs`: `ProcessEnv::prefix()` (crate-private, beside `ProcessEnv`, W-12b), the one env
  prefix of the server's and the TUI's argv (`TITLE_OFF` moved here from `tui.rs`);
  `OpenCodeConfig::bin()` (crate-private; `attach.rs` used the same check inline);
  `serve_command` / `verify_serving` delegate to `server`; deleted `OP_SERVE`, `Timeouts.boot_try`,
  `OpenCodeConfig.workdir`. Crate docs rewritten (facts 2 and 6, the TUI-window precondition W-11b,
  "No per-pane environment", the `Inherit` and `Isolated` docs).
- `src/server.rs`: now `serve_command` (refuses an agent, else
  `<prefix> <bin> serve --port P --hostname 127.0.0.1 <serve_args>`), `refuse_an_agent` (also
  `verify_serving`'s answer, W-3 fail closed), and the unchanged `healthy`. Deleted `serve`,
  `start`, `wait_until_up`, `stop`, `exited`, `reap_when_it_exits`, `refuse_a_held_port`,
  `BOOT_INTERVAL`, `KILL_BOUND`, `timeout`.
- `src/exec.rs`: deleted `kill_group`, its runner `run` and `KILL`; module docs.
- `src/tui.rs`: `tui_argv` starts from `ProcessEnv::prefix()`.
- `src/attach.rs`: uses `config.bin()`; module doc.

**`holler-adapter-host`**
- `src/lib.rs`: `run(name, cwd, argv)`: the existence read (`list-panes`, now `tmux::panes`, ps's
  listing), then `check_argv`, then `check_cwd` (`usage` unless an absolute path to an existing
  directory, W-1, no echo), then `new-window -c <escape_cwd(cwd)>`. `is_usable_dir` is the one
  directory rule of `ensure_session` and `run`. Deleted `BAD_SESSION_DIR`. Crate docs (what `run`
  starts, the error table, narrowing 5, the session-directory bullet; W-9).
- `src/tmux.rs`: `new_window(name, cwd, argv)`; deleted `SESSION_PATH`, `session_dir`,
  `is_session_dir`; `escape_cwd` and `panes` docs.

**`holler-cli`**: `src/pane/wiring.rs`: `Unwired` gains the amended methods (`not-implemented`).

**Docs**: `docs/adr/ADR-0021.md` (see "Architecture notes"); `CHANGELOG.md` `[Unreleased]` entry
linking #802 and #695 and replacing the part 1 entry's statement about `serve`.

## Design decisions

- **W-9, the existence read: reuse `tmux::panes` (ps's listing).** T chose option (a) and pinned
  only that the read is a `list-panes`. Reusing the builder `ps` already has, rather than a new
  `list-panes -F '#{session_path}'` whose output nobody reads, deletes `session_dir`,
  `SESSION_PATH` and `is_session_dir`. The output is ignored; only a refusal (`pane-not-found`
  or `unavailable`) matters.
- **Check order in `run`: session, argv, `cwd`.** The session first keeps the documented "`run`
  checks the session before the argv" (and the fake's order); `cwd` last keeps the fake's
  behaviour a prefix of the adapter's (the fake does not check `cwd`). All three come before any
  `new-window`.
- **The fake's argv is a self-description with a program no real binary has**
  (`holler-fake-harness-server`), so `start_server` can answer `None` for every other argv
  (`["opencode", "serve", "--port", "48100"]`, `sleep 30`, empty). The key is its own element.
- **The fake's mismatch code is its own open code, `agent-mismatch`** (T left it to F). Not
  `agent-unsupported`: the fake can apply an agent; a mismatch means the running server was started
  with another one.
- **`start_server` on a frozen port answers `timeout` with `op = harness.serve_command`**: a
  `<port>.<method>` op is the rule for every fake timeout, and that is the method whose argv was run.
- **The idempotent start keeps the running server's agent**, as a second real `opencode serve` on
  that port would fail to bind and leave the first running; `verify_serving` then sees the
  mismatch. No engine path reaches it (the plan and the act both refuse a port that answers).
- **`wait_healthy` mirrors `probe.rs`'s `wait_until`**: ask, then `left = deadline - now`; zero is
  `Ok(false)`; else sleep `min(left, poll)`. It asks at least once and once more at the deadline.
- **`Budget::deadline` uses `checked_add`, falling back to `started`**: an absurd budget the clock
  cannot hold ends the wait after one poll rather than panicking (clippy denies `panic`; `Instant +`
  would panic on overflow).
- **The conformance suite's wait bound is the engine's own** (`DEFAULT_BUDGET`, `BOOT_POLL`), so
  "the same way the engine waits" is literal.
- **Case 16's key is `conformance-agent`**: a key no binary path or flag holds, so "some element
  contains the key" cannot pass by accident (a key like `build` could match a path).
- **`stop_old` moved into the child module.** It is the server's other step (B1/B2) and the move
  keeps `tx_launch.rs` at 807 lines (it reached 829 with the stop in place; the guideline is to
  decompose before about 850, and #644 part 2 and #664 will edit this file).
- **`AGENT_UNSUPPORTED` message does not echo the key.** An agent key is not a secret, but the
  refusal needs no key to be understood.

## Reuse / extend-vs-new

Extended throughout, per the brief's Scope and A's map; no parallel path:
- The ports amended in place (`HarnessPort`, `HostPort`).
- `FakeHarness` / `World` / `ServerView` / `HarnessOp`, and `FakeHost` / `runs()`, extended (no
  second log, no recording wrapper: W-8a).
- The one conformance table, runner and `run_cases` (W-4: the new case is a 16th row; the runner
  gained a parameter).
- One wait (`tx_launch::wait_healthy`) for the engine, the suite and the real rig (W-4).
- One env prefix (`ProcessEnv::prefix`) for the TUI's and the server's argv (Scope 3).
- `tmux::escape_cwd` reused for `new-window -c` (no new escape); `tmux::panes` reused for the
  existence read; `is_usable_dir` shared by `ensure_session` and `run`.
- `OpenCodeConfig::bin` replaces the inline UTF-8 check `attach.rs` had, now that two builders need it.
- The one new module, `tx_launch/server.rs`, is the brief's "sibling module" (Scope 2) in the
  `reconcile/observe.rs` pattern.

## Architecture notes for A

- **Ports (contract amendment, amend-first):** `HarnessPort::serve` -> `serve_command` + required
  `verify_serving`; `HostPort::run` gains `cwd`; new `ServeSpec` and `AGENT_UNSUPPORTED`. Every
  implementation in the workspace is updated (fakes, adapters, `Unwired`, T's doubles).
- **Process ownership moved**: the OpenCode adapter now starts, waits on and signals no process
  (its `exec.rs` keeps only `capture` for tmux). The host adapter is the one place a pane's
  processes start and stop, the server included.
- **New module boundary**: `holler-pane/src/tx_launch/server.rs` (private), with one public
  item, `wait_healthy`, re-exported from `tx_launch`. The test kit now depends on that function and
  on `tx_launch::{BOOT_POLL, DEFAULT_BUDGET}` (it already depended on `holler-pane`; no manifest
  change). No new crate dependency anywhere; `cargo machete` is clean.
- **Engine side effect**: the launch engine now sleeps between health polls (ADR section 5 updated).
- **No record or wire change**: `HarnessInfo.pid` keeps its serde (`skip_serializing_if`); launch
  writes `None`. No protocol, hub or `holler-body` change.
- **ADR-0021** (Scope 7 and W-10): port table rows; a new subsection "The harness server as amended
  (#802)" recording decisions 1-4 as "Decided (operator, 2026-10-11)" and 5-7 as decided in review
  round 1, the health wait and what it gives up, and the limits that move with the server
  (W-10f: OpenCode's own `setsid` shell commands, the credential path and a possible `HostPort`
  amendment for #649, tmux's command size); facts 1 (TUI-window precondition, W-11b), 2, 6, 7;
  section 1's `harness` row; section 5's side effect; the section 8 seam phrase ("first
  amendment"); "Launch and relaunch as built" (plan, act, rollback vs crash, W-10d); "Close as
  built" (`stop_owned` now stops the server); section 11; the code table (launch/relaunch and
  `profile apply` rows, the open-code list); the two Deferred bullets (#695 decided); "Decisions
  taken" items 10-13.

## Deviations from spec / wireframe

None in behaviour. Three structural choices the brief left open, recorded above: the existence
read reuses `tmux::panes`; `stop_old` moved into the child module; `OpenCodeConfig::bin` is shared.
Line counts asked for by W-6 (final): `tx_launch.rs` 807 (+ `tx_launch/server.rs` 101),
`switch.rs` 833, `fake_tmux_test.rs` 803, `hermetic_test.rs` 812, `launch/rig.rs` 793 (the last
four are T's files). `holler-adapter-opencode/src/lib.rs` is 599 and `holler-pane-testkit/src/
harness.rs` 599 (both kept under the 600-line warning).

## Tier 1 self-check (incl. tests now GREEN)

```text
$ cargo build --workspace --tests                     # every target compiles (RED was 11 targets)
Finished `dev` profile

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile                                # no warning (dead_code denied: Scope 3's deletions are complete)

$ bash scripts/lint.sh        -> exit 0 (warnings only, none new: tx_launch.rs was already past 600)
$ bash scripts/changelog-check.sh -> changelog-check: ok
$ cargo machete               -> didn't find any unused dependencies
$ rustfmt --edition 2021 --check <the changed crates' roots> -> clean
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane -p holler-pane-testkit \
    -p holler-adapter-opencode -p holler-adapter-host -> clean

$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
passed=2070 failed=0 ignored=25, exit=0              # the pipeline's verify command, on the final code
                                                     # (clippy on the same tree: exit 0)
$ cargo test -p holler-cli --test docs_cli_test      # again after the last ADR/CHANGELOG wording edits
3 passed

# T's targets, after the last code change:
pane_verbs                 258 passed      fake_harness_test        23 passed
harness_conformance_test    15 passed      host_conformance_test    12 passed
fake_host_test              12 passed      hermetic_test            31 passed
attach_test                 23 passed      fake_tmux_test           29 passed
ports_test                   8 passed      pane_registry_test       15 passed
holler_pane doc-tests        5 passed      (the two new compile_fail,E0639 among them)

# The two compile_fail snippets, compiled outside the crate, fail with E0639 (rustdoc on stable
# checks only that they fail):
error[E0639]: cannot create non-exhaustive struct using struct expression   (x2)

# Opt-in, real binaries on private servers (tmux 3.7c, a local opencode):
cargo test -p holler-adapter-host --test real_tmux_test -- --ignored                      9 passed
HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test \
  -- --ignored --test-threads=1                                                           9 passed
# (ac12 runs the conformance suite through the new launcher against a real `opencode serve`
#  started from the argv `serve_command` builds, `env -i K=V ... opencode serve ...`; no stray
#  process was left.)
```

## Evidence appendix

`docs/handoffs/802/evidence.md` (rollback's `stop_owned`, `acted` and the CLI's reconcile step,
`class_of` for open codes, the `pid` serde attribute, the host's TERM/grace/KILL and `argv[0]`
rule, the fault switch logging failed calls).

## Tests that look wrong (for T)

None. Two notes, not defects:
- `hermetic_test.rs` and `fake_tmux_test.rs` are 812 and 803 lines (lint fails at 900).
- `ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies` names "sends nothing" for the
  no-agent `verify_serving` but does not assert the stub saw no request; the implementation sends
  nothing (`refuse_an_agent` is pure). T may add `stub.lines().is_empty()` if it wants it pinned.

## Known issues

- **W-11(a), for O:** no test starts the server under a tmux pty through `new-window ... -- env --
  <argv>`; the real rig's launcher spawns the argv directly (that crate may not depend on the host
  adapter). The argv itself was proven against a real `opencode serve` (ac12, above). An opt-in
  end-to-end check in `holler-cli` (the one crate that depends on both adapters) belongs with #649.
- **W-13:** #664 has not merged; `crates/holler-cli/tests/profile_verbs/rig.rs` calls no `serve`,
  so no edit outside the blast radius was needed. Whichever of #664 and #802 merges second adapts.
- Until #649 wires the adapters, nothing a user runs changes; until #642 part 3, `--agent` against
  the real adapter is refused (`agent-unsupported`, exit 3), as the brief intends.

## Files changed

Production (F):
- `crates/holler-pane/src/ports.rs`
- `crates/holler-pane/src/lib.rs`
- `crates/holler-pane/src/tx_launch.rs`
- `crates/holler-pane/src/tx_launch/server.rs` (new)
- `crates/holler-pane-testkit/src/harness.rs`
- `crates/holler-pane-testkit/src/host.rs`
- `crates/holler-pane-testkit/src/conformance/harness.rs`
- `crates/holler-pane-testkit/src/conformance/host.rs`
- `crates/holler-adapter-opencode/src/lib.rs`
- `crates/holler-adapter-opencode/src/server.rs`
- `crates/holler-adapter-opencode/src/exec.rs`
- `crates/holler-adapter-opencode/src/tui.rs`
- `crates/holler-adapter-opencode/src/attach.rs`
- `crates/holler-adapter-host/src/lib.rs`
- `crates/holler-adapter-host/src/tmux.rs`
- `crates/holler-cli/src/pane/wiring.rs`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Handoff: `docs/handoffs/802/handoff-F.md`, `docs/handoffs/802/evidence.md`, `docs/handoffs/802/decisions.md` (appended).
