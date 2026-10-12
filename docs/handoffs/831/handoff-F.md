# Handoff-F: Phase 5 - #831 A Claude Code harness for panes

**Date:** 2026-10-11
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-claude-harness`)
**Issue:** #831 (this PR is "Part of #831": the adoption step, section 15 item 17, waits for #835)

## What was done

F implemented T's contract (`handoff-T-red.md`) in A's order (W14 first, then the endpoint step alone, the variant, the
process methods and the router, the fakes and the suite, the adapter crate, the engines and the verbs, the docs). Every
authored test passes except one stable-codes list that T must extend (below). Nothing was committed or pushed.

**`holler-pane`**

- `pane.rs`: `HarnessKind::Claude`; the facts about a kind (`as_str`, `shape() -> Shape`, `abort_style() -> AbortStyle`,
  `login_variables()` with `CLAUDE_LOGIN_VARIABLES`, the seven names); #835's `CLAUDE_CODE_COMMAND` and
  `is_claude_code_command`, copied verbatim; `HarnessInfo.port: Option<u16>`, checked against its kind through
  `#[serde(try_from = "RawHarnessInfo")]` (`misfit`), and `HarnessInfo::endpoint(&name) -> Result<HarnessEndpoint, _>`
  (`store-corrupt` for a record that does not fit its kind).
- `ports.rs`: every `HarnessPort` method that took `port: u16` takes `&HarnessEndpoint` (`shown_session(pane, at)`,
  `attach_tui(pane, at, session)`); four default-armed process methods (`process_command(name, spec, &SessionChoice)`,
  `new_session_id`, `locate_process`, `login_status`), each defaulting to `process-unsupported`; `ServeSpec` gains
  `model` and `cwd` (`with_model`, `with_cwd`); `Activity::WaitingPermission`; #835's required
  `HostPort::foreground_commands`, verbatim.
- `ports/process.rs` (new): `HarnessEndpoint {Port, Process {pane, pid}}` with its two conversions (`port()`,
  `process()`), `SessionChoice {Fresh, Keep, Resume}`, `LoginStatus {Ok, Missing}`, and the port's codes
  `process-unsupported`, `server-unsupported` (new), `abort-unsupported`, `session-mismatch`, `inbound-held`.
- `ports/router.rs` (new): `HarnessRouter<S, P>`, the one router (W5); re-exported with `HarnessEndpoint` from `lib.rs`.
- `tx_launch.rs` split (W14: 831 lines before, 747 now) into `budget.rs`, `rules.rs` (the policy rule `port_for`, the
  relaunch rules with the harness check, `refuse_login_env`, codes `port-policy-unsupported`, `harness-change-refused`,
  `claude-login-env-refused`), `server.rs` (the server's act, `wait_healthy` through `wait_until`), `process.rs` (new:
  the single process's act, `process-in-use`, `wait_located`) and `undo.rs` (rollback, endpoint-generic). Public:
  `ProcessSession {Keep, Resume(String), Fresh}`, `relaunch_on`, `relaunch_recorded`, `wait_until`.
- `tx_switch.rs`: a single process's switch and reset are a relaunch (`relaunch_recorded` with `Resume` or `Fresh`)
  after the idle gate (`session-holds-permission`, new) and, for switch, `session-of-other-pane`; `--first` goes to the
  endpoint of the record the relaunch stored (W4).
- `reconcile.rs`, `reconcile/observe.rs`, `reconcile/observe/process.rs` (new): `reconcile_with(ports, request,
  Option<Relaunch>)`; the single-process chain (W3); the repair only while idle, through the caller's relaunch, and no
  record write after it (W2); the login check once per pass when a pane in scope is a single process.
- `findings.rs`: `PaneWaiting` and `ClaudeLoginMissing`, appended, no remedy.
- `profile_snapshot.rs` (`NO_PORT_POLICY = "none"`, `port_policy_of`), `tx_apply.rs` (`harness-change-refused` before the
  health judgment), `tx_apply/observe.rs`, `import/{assess,files,observe}.rs` (W7's `!= Some(Opencode)` fix and the
  endpoint/`Option` migration), `unmanaged.rs` (#840's doc sentences: both kinds are drivable).

**`holler-pane-testkit`**: `claude.rs` (new, `FakeClaude`), `harnesses.rs` (new, `FakeHarnesses` over the router, the
rigs' one linker), `conformance/claude.rs` (new, the 12-case suite, a mutant per case), the endpoint migration of
`harness.rs`, `conformance/{harness,pane_store}.rs` and `fixture.rs`, #835's `FakeHost::foreground_commands` and host
suite case 10 (verbatim).

**`holler-adapter-claude`** (new crate; T wrote the skeleton): `lib.rs` (`ClaudeConfig`, `Timeouts`, `ClaudeHarness`,
the seams `AgentsList`, `InboxWrite`, `LoginCheck`, the `HarnessPort` impl), `argv.rs`, `agents.rs`
(`harness-dialog-open`), `inbox.rs`, `seams.rs`, `exec.rs`.

**Other adapters**: `holler-adapter-host` (`TmuxHost::foreground_commands`, #835 verbatim); `holler-adapter-opencode`
(endpoint-keyed: `at.port()?` at each method).

**`holler-cli`**: `wiring.rs` (`HarnessRouter<WiredHarness, WiredHarness>`, `claude_harness`, the env table row),
`wiring/env.rs` (`claude_bin`, `claude_config_dir`), `wiring/bind.rs` (forwards), `pane/args.rs` (`--harness`),
`pane/launch.rs` (kind default and override, the grammar-only policy check, `harness_text`), `pane/get.rs`,
`pane/doctor.rs` (`reconcile_with` + `relaunch_recorded(Keep)`), `say_cmd.rs` (`resolve_pane_record`,
`interrupt-text-unsupported`, `answer-unsupported`), `interrupt_cmd.rs` (`interrupt_pane`, `PaneInterrupt`),
`answer_cmd.rs` (`answer_pane_target`), `Cargo.toml`.

**Docs**: ADR-0021 (as-built paragraphs in sections 8, 14 and 15, the section 9 rows, "Deferred", decisions 26 to 28;
the three `docs_cli_test` fragments fixed), `docs/pane-control.md` (the Claude Code pane in sections 1 to 6 and 9 to
12), `docs/orchestrating.md` (one paragraph), ADR-0003 (amendment line, rows, SPEC FLAGS), `CHANGELOG.md`.

**Merge of `origin/main`.** `origin/main` moved (#849, #840, #851) after the base. Rebasing was impossible: `008a995` is
pushed and the driver pushes without force. So F ran `git merge --no-commit --no-ff origin/main` (clean, no conflict) and
left `MERGE_HEAD` set: **the driver's phase commit concludes it as a merge commit.**

## Design decisions

1. **Endpoint conversions in one place each** (item 3). `HarnessEndpoint::port()` is the server harness's one
   conversion and `process()` the single process's. The mirror case gets its own code, `server-unsupported` (new),
   rather than a second meaning of `process-unsupported`: the message says which shape was asked what. The alternative,
   one code for both, makes "a forgotten forward" and "a Claude pane asked for a TUI step" read the same.
2. **`SessionChoice` carries the verb's intent** (W1). `Resume` keeps switch's `session-not-found`: the adapter refuses an
   id with no transcript, and the plan runs it before anything stops. At the engine level, `ProcessSession::Fresh`
   takes its id from the harness (`new_session_id`), and a server pane's relaunch accepts `Keep` only (`usage`
   otherwise).
3. **One stop/run/wait/record sequence** (W4, W10). `relaunch_on` and `relaunch_recorded` serve switch, reset,
   interrupt and doctor. `process_command` runs first among the plan's reads, so its refusals write nothing. The wait
   is `wait_until`, the engine's one deadline-checked loop, until the process is located and idle.
4. **Doctor's repair through a caller-supplied relaunch** (W2; A left the mechanism to F). `reconcile_with` receives
   `Option<Relaunch>`, and `holler pane doctor` passes `relaunch_recorded` with `Keep`. The alternative, a relaunch after
   the pass, was rejected because the chain must know that it relaunched: it skips its own record write (the relaunch
   stored one), and it reports `fixed` or `fix failed: CODE` on the same finding. Reconcile still imports no `tx_launch`.
5. **`process-in-use`** (W11) through #835's `foreground_commands` and `is_claude_code_command`, for a launch only (a
   relaunch has stopped what the pane owns). A tmux session that does not exist yet (`pane-not-found`) runs nothing.
6. **The login variables are a fact of the kind.** The adapter's `env -u` list and the engine's
   `claude-login-env-refused` (raised in the plan, before any port call) both read `HarnessKind::login_variables()`, so
   they cannot drift. Decision 28 in the ADR.
7. **The login check** runs once per reconcile pass, and only when a pane in scope is a single process: a fleet with no
   Claude pane never asks the Claude harness anything. A failed check is `observe-failed`, never `claude-login-missing`.
8. **The socket hook** (W8; A left the key to F). Its command is fixed text. Its data comes through the environment:
   `CLAUDE_CODE_MESSAGING_SOCKET` and the argv's `HOLLER_CLAUDE_SOCKET_DIR`. It writes `<dir>/$PPID` through a temporary
   file and a rename, under `umask 077`. The reader fails closed: the directory and the file must be the user's alone
   and not links, and the content one absolute path. A file is trusted only for a pid that `claude agents --json`
   lists now. The pid was chosen over the session id as the key because a person's `/clear` changes the session and not
   the process, so the same file is rewritten and no stale socket outlives its process.
9. **The `inbound-held` window** is 600 ms (`Timeouts::held_window`), ended early when the entry turns `busy`. An entry
   that was already waiting before the write is not judged.
10. **`--harness` and the policy** (W13). The flag is in `SpecFlags`, typed by `validate`, and never echoed. The CLI's flag
    check reports only `port_for`'s `usage` class. A policy the kind does not take is the engine plan's refusal
    (`port-policy-unsupported`), so `--spec-only` and `profile apply` meet the same rule.
11. **`HarnessInfo` validation** follows `Profile`'s slug pattern: a record with a port on a Claude pane, or with none on
    an OpenCode pane, does not deserialize. `HarnessInfo::endpoint` answers `store-corrupt` for one built in memory.
12. **Interrupt routing** is by `abort_style()`. A busy Claude pane is relaunched too, since that is the interrupt (the
    operator's decision of 2026-10-11). Nothing is typed.
13. **The wiring's inbox directory** is `holler_hub::state::resolve_state_dir()/claude-inbox`, the directory through
    which the CLI already finds the hub, rather than a new variable.

## Reuse / extend-vs-new

- **Extended in place**, as the brief's Reuse map and A's plan review asked:
  - `HarnessPort` (#802 decision 2 precedent), `HostPort` (with #835's method), `HarnessInfo`, `HarnessKind`, `ServeSpec`
    and `Activity`;
  - the launch engine (`tx_launch`, with no parallel launcher), `tx_switch`'s plan, reconcile's chain (a sibling module
    for the single process), `tx_apply`'s choice, `say_cmd`'s pane gates (`resolve_pane_record`, which
    `resolve_pane_target` now calls), and the wiring's decorators (forwards only).
- **`Ports` keeps one `harness`**: the router is the new object, and it is the type A asked for (W5).
- **New objects with the brief's justification:**
  - the crate `holler-adapter-claude`: one adapter crate per harness, like `holler-adapter-opencode`, and a dependency of
    `holler-cli` only;
  - `FakeClaude`, `FakeHarnesses` and the Claude conformance suite: the test kit's established fake-and-suite pattern;
  - `ports/process.rs`, `ports/router.rs` and `tx_launch/process.rs`: sibling modules of the objects they extend.
- **For A's anti-duplication gate:** `holler-adapter-claude/src/exec.rs` is a bounded capture runner shaped like the
  OpenCode and host adapters' runners. An adapter may not depend on another, and the shared runner is #696, so it is a
  copy by design, written so that #696 is a move.

## Architecture notes for A

- **New dependency edge:** `holler-cli -> holler-adapter-claude -> holler-pane` (plus `serde_json` and `getrandom`,
  both workspace dependencies). No `holler-hub` or `holler-proto` dependency was added to the adapter. `holler-cli`
  already depended on `holler-hub`, which `claude_harness` calls for the state directory.
- **Public interface changes** (`archChanged: true`):
  - `HarnessPort`: endpoint-keyed, plus four default-armed methods;
  - `HostPort::foreground_commands`: required, as #835 decided;
  - `HarnessInfo.port: Option<u16>`;
  - `ServeSpec.{model, cwd}`, `Activity::WaitingPermission`, two `FindingKind`s;
  - new public types: `HarnessEndpoint`, `SessionChoice`, `LoginStatus`, `HarnessRouter`, `ProcessSession` and
    `reconcile::Relaunch`;
  - new public functions: `relaunch_on`, `relaunch_recorded`, `wait_until`, `wait_located`, `reconcile_with`,
    `say_cmd::resolve_pane_record`, `interrupt_cmd::interrupt_pane` and `answer_cmd::answer_pane_target`.
- **No wire or protocol change:** `docs/protocol/v2.md` is not touched, and no golden file moved.
- **Sizes (`scripts/lint.sh` exit 0):** `tx_launch.rs` 747 lines, and `holler-pane-testkit/src/harness.rs` about 630
  (a warn).

## Deviations from spec / wireframe

- **Section 15 is corrected "as built"**, as A's journal assumed (there is no O in this path). Each correction is an
  "As built" paragraph in its item:
  - item 2: the constant is `CLAUDE_CODE_COMMAND` with `is_claude_code_command` (W6);
  - item 5: `process_command` takes a `SessionChoice` (W1); `LoginStatus` has no `NotApplicable`, and `login_status`'s
    default refuses (W9);
  - item 6: one entry point, and doctor's repair only while idle (W2, W3, W4, W10, W11);
  - item 12: `--harness` in `pane/args.rs` (W13);
  - item 14: each code declared where it is raised (W10), plus the new `server-unsupported`.

  **The PR body should list these edits to the amendment.**
- **Item 17 (adoption) is not built.** #835 is not on `main` (checked at F's start and again at the end of F), so the
  story stops there, as the item says.
- **#835's code is copied verbatim:** its branch at `9bd101b` and `36b0de8` (the constant and the rule, the required
  method, the `TmuxHost` and `FakeHost` implementations, host suite case 10, the decorators' forwards, and its tests
  `claude_code_rule_test.rs`, `foreground_test.rs` and the hunks of `fake_host_test.rs`, `fake_tmux_test.rs` and
  `host_conformance_test.rs`). Whichever lands second merges identical hunks. These files are left exactly as #835 wrote
  them, rustfmt-unclean lines included. `HerdrPort::observe_pane`'s loss of its default is #835's and is not copied.

## Tier 1 self-check (incl. tests now GREEN)

Run after the merge of `origin/main` and after the docs, at 2026-10-11T20:43-06:00 (MDT):

```text
$ cargo build --workspace --tests                                    # clean
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
  passed 2487, failed 1, ignored 34 (163 test binaries)
  FAILED: holler-pane tests/findings_test.rs finding_kind_codes_are_stable   <- T's list, see below
$ cargo test -p holler-cli --test pane_claude                        42 passed, 1 ignored (the say step, #839/#850)
$ cargo test -p holler-cli --test cli_surface_test                   3 passed (the two --harness lines parse)
$ cargo test -p holler-cli --test pane_integration                   67 passed, 4 ignored (opt-in real), 119 s
$ cargo test -p holler-pane --test claude_kind_test                  15 passed
$ cargo test -p holler-pane --test rework_test                       7 passed
$ cargo test -p holler-pane --test claude_code_rule_test             2 passed
$ cargo test -p holler-pane-testkit --test fake_claude_test          13 passed
$ cargo test -p holler-pane-testkit --test claude_conformance_test   4 passed (42 s: the mutation case)
$ cargo test -p holler-pane-testkit --test fake_harnesses_test       5 passed
$ cargo test -p holler-adapter-claude                                hermetic 15 passed; real_claude_test 1 ignored (opt-in)
$ cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test   3 + 16 passed
$ cargo test -p holler-proto --test docs_errors_test --test docs_wire_test       3 + 12 passed
$ cargo clippy --workspace --all-targets -- -D warnings
  error: this loop never actually loops --> crates/holler-cli/tests/pane_claude/mixed.rs:242:5   <- T's test, see below
  (every other target clean: --exclude holler-cli --all-targets, and holler-cli --lib --bins plus each --test but
  pane_claude, both with -D warnings, finish with no diagnostic)
$ bash scripts/lint.sh                 exit 0 (size warnings only; none of the new files)
$ cargo machete                        no unused dependencies
$ bash scripts/changelog-check.sh      changelog-check: ok
$ rustfmt --check                      clean on every new or changed file of F's, except #835's verbatim copies and the
                                       lines main already carried unformatted (answer_cmd.rs, interrupt_cmd.rs,
                                       say_cmd.rs, pane_integration/{bind,scenario,wired}.rs, pane_verbs/switch.rs)
```

## Evidence appendix

`docs/handoffs/831/evidence.md`: six facts of unchanged code (`class_of` for open codes, the closed `session-not-found`,
`shown_differs`, `TmuxHost::run` and `stop_owned`, tmux's `env --` argv and its escaping, and `resolve_state_dir`).

## Tests that look wrong (for T)

1. **`crates/holler-pane/tests/findings_test.rs`, `finding_kind_codes_are_stable`.** `CODES` (`[&str; 13]`) must gain
   `"pane-waiting"` and `"claude-login-missing"`, in that order, after `"unmanaged-stalled"` (`[&str; 15]`), since the
   contract appends the two kinds (item 13). Its module doc's "twelve kinds" is also stale. This is test-only: nothing in
   production code changes.
2. **`crates/holler-cli/tests/pane_claude/mixed.rs:242`, `say_delivers_a_message_to_each_pane_of_the_scenario`**
   (ignored). Its loop ends in an unconditional `panic!`, so `cargo clippy --workspace --all-targets -- -D warnings`, a
   CI step (`ci.yml:348`), fails with `clippy::never_loop`. For example, move the `panic!` after the loop, or add
   `#[allow(clippy::never_loop)] // #831`. This is test-only.

**The endpoint migration of the existing tests** (T's "Not changed" list, assigned to F's endpoint step) was done with
no assertion edited:

- `u16` became `&HarnessEndpoint` at each harness call;
- `HarnessInfo { port: N }` became `Some(N)`;
- three expectations of a port read became `Some(PORT)`: `fake_pane_store_test.rs`, `pane_verbs/launch.rs` (the
  expected record) and `pane_verbs/launch/serve.rs`;
- the doubles of `HarnessPort` and `HostPort` gained the forwards their trait now requires.

The files are:

- `holler-pane`: `ports_test.rs`, `profile_snapshot_test.rs`, `profile_diff_test.rs`;
- the test kit: `fake_harness_test.rs` (an `at()` helper), `harness_conformance_test.rs`, `fake_pane_store_test.rs`;
- the OpenCode adapter: `hermetic_test.rs`, `attach_test.rs`, `prompt_test.rs`, `agent_test.rs`,
  `real_opencode_test.rs`, `real_opencode/rig.rs`, then rustfmt;
- `holler-cli`: `pane_verbs/{launch/rig,apply/rig,doctor/rig,doctor/surface,import/rig}.rs`, `pane_verbs/{switch,
  launch,launch/serve,relaunch,reset}.rs`, `profile_verbs/{rig,create,apply/rig}.rs`, `pane_integration/{scenario,
  wired,bind}.rs`.

Phase 6 checks that no assertion moved.

## Known issues

- **Item 17 is not built** (above): the PR is "Part of #831". The adoption is built test first once #835 has merged.
- **Not run by F: the opt-in real test**, `HOLLER_TEST_CLAUDE=1 cargo test -p holler-adapter-claude --test
  real_claude_test -- --ignored --test-threads=1`. It uses the real login and one model turn, and it starts real
  processes. So these are unverified against Claude Code 2.1.296:
  - the hook's `$PPID` key, which assumes Claude Code runs a hook's shell as its direct child, so that `$PPID` is the pid
    `claude agents --json` lists. If it is not, `send_prompt` fails closed (`unavailable`) and nothing is misdelivered;
  - the inbox frame;
  - the end-to-end launch through real tmux.

  The spike verified the socket variable at `SessionStart`, `--session-id` and `--resume`, `agents --json` and
  `authMethod`.
- **`harness-dialog-open` has no fake state** (T's advisory), so it is pinned only by the adapter's mapping (hermetic) and
  the launch's generic wait-error rollback.
- **The `inbound-held` window** is pinned on `FakeClaude` only (timing at the real seam).
- **The claude conformance suite's mutation case takes about 42 s:** each of its mutants waits out a bound.
- **`reset_first_reaches_the_new_process`** depends on #828 not having moved `--first` yet. If #828 lands first, it goes
  back to T.
- **`MERGE_HEAD` is set** in the worktree (the merge of `origin/main`, above). The driver's commit concludes it.

## Files changed

Production and docs only. The test files are listed in "Tests that look wrong", in the migration paragraph.

- `Cargo.lock`
- `CHANGELOG.md`
- `crates/holler-adapter-claude/Cargo.toml`
- `crates/holler-adapter-claude/src/{lib,agents,argv,exec,inbox,seams}.rs`
- `crates/holler-adapter-host/src/{lib,tmux}.rs`
- `crates/holler-adapter-opencode/src/lib.rs`
- `crates/holler-cli/Cargo.toml`
- `crates/holler-cli/src/{answer_cmd,interrupt_cmd,say_cmd}.rs`
- `crates/holler-cli/src/pane/{args,doctor,get,launch,wiring}.rs`
- `crates/holler-cli/src/pane/wiring/{bind,env}.rs`
- `crates/holler-pane/src/{findings,lib,pane,ports,profile_snapshot,reconcile,tx_apply,tx_launch,tx_switch,unmanaged}.rs`
- `crates/holler-pane/src/import/{assess,files,observe}.rs`
- `crates/holler-pane/src/ports/{process,router}.rs` (new)
- `crates/holler-pane/src/reconcile/observe.rs`
- `crates/holler-pane/src/reconcile/observe/process.rs` (new)
- `crates/holler-pane/src/tx_apply/observe.rs`
- `crates/holler-pane/src/tx_launch/{budget,process,rules,undo}.rs` (new)
- `crates/holler-pane/src/tx_launch/server.rs`
- `crates/holler-pane-testkit/src/{claude,harnesses}.rs` (new)
- `crates/holler-pane-testkit/src/{fixture,harness,host,lib}.rs`
- `crates/holler-pane-testkit/src/conformance/claude.rs` (new)
- `crates/holler-pane-testkit/src/conformance/{harness,host,mod,pane_store}.rs`
- `docs/adr/ADR-0003.md`
- `docs/adr/ADR-0021.md`
- `docs/orchestrating.md`
- `docs/pane-control.md`
- `docs/handoffs/831/{handoff-F,evidence}.md` (new)
- `docs/handoffs/831/decisions.md` (F's entry)
