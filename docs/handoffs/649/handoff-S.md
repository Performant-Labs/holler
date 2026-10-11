# Handoff-S: Phase 10 - #649 hub wiring and the scratch integration scenario  (spec audit)

**Date:** 2026-10-11 (15:55 MDT)
**Branch:** `issue-649-implementation` (worktree `.claude/worktrees/0649-live-harness`, head `9cafe0d`, merge base `5d366b9`, #650).
`origin/main` is `4366fb8` (#830's research spike, `docs/research/` and `scripts/spikes/` only). A trial merge
(`git merge-tree --write-tree origin/main HEAD`) is clean.
**Issue:** #649 (epic #633), read live with `gh issue view 649` (last edited 12:16 MDT). Its body is the brief's top half
with an evidence index. One difference: the issue's D10 (5) lacks the `LANG=C.UTF-8` clause that `c13400a` added to the
brief (Notes for O, item 3).
**Brief:** `docs/handoffs/649-brief.md` at `c13400a`.
**Handoffs reviewed (under `docs/handoffs/649/`):**
- `handoff-A.md` (fifth pass, `2280399`) and `handoff-A-dup.md` (`9cafe0d`);
- `handoff-T-red.md` (second cycle, `ce5da11`; the first cycle's is `e4f1d36`);
- `handoff-F.md` (all three cycles);
- `handoff-T-green.md` (fourth pass, `6ce945d`);
- `decisions.md` and `evidence.md`.

**Outside gates** (`deepseek-v4-pro`, per each `.usage.json`):
- brief gate r1: PASS (10:32 MDT);
- diff gate r1: BLOCK (13:47). A's fifth pass refuted both blocks against the code and a real hub;
- diff gate r2: BLOCK (14:39). The compiler refutes both (`13010f4`, `4d7f58a`);
- diff gate r3 (15:13): "BLOCK — 0 blocking finding(s)". Its one WARN (D10 did not list `LANG`) was settled by amending
  D10 (`c13400a`).

**Verdict:** REWORK, test-only. Two rows are missing in `crates/holler-cli/tests/pane_integration/fake_hub.rs`, and no
`src/` change is needed. One more item blocks the merge but is not F's or T's code: the rename scan in Notes for O,
item 1.

## A precondition

Met.
- `handoff-A.md` (fifth pass, the re-entry after diff gate r1): **PASS**. Its binding warns were W-S (evidence, no code
  for B-1 and B-2) and W-T (a non-UTF-8 value reads as malformed). F did both in the second cycle, and T pinned W-T.
- `handoff-A-dup.md` (`5d366b9..c13400a`): **PASS**, no block. Warns W-1 to W-4 and W-O are follow-up material
  (advisory 5).

## T precondition

Met. `handoff-T-green.md` (fourth pass) says "Blocking issues: None".
- **RED.** First cycle: 8 of 58 default-run tests failed against the trial of `046026f`. Each failure is on a row D32
  names: D4's eager request (3), D5's dropped fallback (3), D5's extents (1) and W-E's fail-closed socket (1).
  Second cycle: the new W-T row failed on its assertion (`host.ensure_session … got Ok(())`, `tmux-1000` in the trap).
- **GREEN.** `pane_integration` passes 61, fails 0 and ignores 4. The workspace run (CI's command, `--no-fail-fast`)
  gives 2312 passed, 1 failed, 32 ignored. The failure is the rename scan, Notes for O item 1.
  Mutation checks M1 to M8, plus the 5 s pin check and the `move` check, each failed when the behavior was removed.
- **Opt-in.** `scenario::*` passed 4 of 4 at `22aa539` (F's second cycle, 82.48 s; T-green's second cycle), with no
  `/tmp/hlr649-*` directory and no `hlr649` or `holler649` process left. Since then, `git diff 22aa539 HEAD -- crates/`
  shows comments in three files and one table value in a default-run test. No scenario code changed, so that run still
  stands.
  I did not re-run it.

## Acceptance criteria

Paths are under `crates/holler-cli/tests/pane_integration/` unless given in full.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | No hub: every `pane` and `profile` verb exits 1 `unavailable`, naming the missing hub, and touches no adapter (D28). The process stub tests run against a started hub (D22) | `no_hub::every_verb_with_no_hub_is_unavailable_naming_the_hub_in_json` and `…_in_text` cover all 20 verbs: 12 `pane`, 8 `profile`, checked against `src/pane/` and `src/profile/`. Each run checks one envelope (`check_envelope`), exit 1 and `unavailable`. The message must name "no live holler hub reachable" and the state dir. The traps must stay untouched: a held Herdr listener, a tmux socket path, an `opencode` that leaves a mark, and an empty `PATH`. D22: `pane_verbs/process/stub.rs`'s three stub tests now run through `LiveHub`, with their assertions unchanged | Met |
| 2 | `pane list`, `get` and `watch` and `profile list` work with no Herdr, tmux or OpenCode (D2) | `store_only::the_store_only_verbs_work_with_no_herdr_tmux_or_opencode` runs with an empty `PATH` and no adapter setting, and covers `profile show` too. `env::malformed_adapter_settings_do_not_stop_the_store_only_verbs` and `store_only::a_herdr_verb_with_no_session_configured_is_usage_naming_holler_herdr_session` also cover D5 and D26 | Met |
| 3 | A read the hub took and never answered is `timeout` naming the store method, within 10 s. Every other failure of a read is `unavailable` *naming the hub*. A JSON-RPC error is `unavailable` for reads and writes | **Timeout:** `fake_hub::a_read_the_hub_never_answers_is_timeout_within_the_bound` (`pane_store.get`, under 13 s); `a_read_and_a_write_answered_after_6_s_succeed` (6 s, from below); `a_watch_request_the_hub_never_answers_is_timeout_naming_watch_next`. **Unavailable:** `a_read_whose_reply_is_lost_is_unavailable` (5 reads), `a_json_rpc_error_is_unavailable_for_reads_and_writes` (10 methods) and `a_client_call_with_no_hub_is_unavailable_naming_the_hub` | **Partly met.** Every row pins the code. "Naming the hub" is asserted only on the no-hub row (REWORK 2) |
| 4 | A malformed `HOLLER_TMUX_SOCKET` is `usage` naming it from the host and the harness, never the default socket (D6) | `wired::a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server` and `wired::a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server` run through `Wiring::connect` and `ports()` against a real hub. `TMUX_TMPDIR` is a trap that tmux's default socket would land in. The reader's own row is `env::a_relative_tmux_socket_is_usage_never_the_default_server` | Met |
| 5 | A write whose reply is lost is `timeout` (D3) | `fake_hub::a_write_whose_reply_is_lost_is_timeout`: all five writes, each naming its `op`. Also `a_write_the_hub_never_answers_is_timeout_within_the_bound` | Met |
| 6 | The scenario passes on scratch instances only; nothing outside them is touched; real names are refused; the guards are tested in the default run | The opt-in 4 of 4 above. `guards::*` has 7 tests: the gate on exactly `1`; the session (`default` refused); pane names; paths inside the base; the binary's exact environment, `LANG` included; its refusals; and the tmux server's exact environment | Met (recorded runs; not re-run by S) |
| 7 | The scenario's order, then the repairs of D21 | `scenario::the_scenario_runs_in_order_on_scratch_instances`. Steps: 1, two launches; 2, relaunch C1; 3, reset C2 `--first`; 4, switch C1; 5, park C2; 6 and 7, kill C2's server and delete C1's session, with one `doctor` finding both; 8, a hub restart with `pane list` unchanged. Then `reset C1`, `unpark C2`, `relaunch C2` and the stray deletes | Met (advisory 3) |
| 8 | No step types into a TUI. At the end, `doctor` reports no finding and the registry equals Herdr's snapshot for the scenario's workspace | No `send_text`, `send_keys` or `send-keys` in `scenario.rs` or `scratch/mod.rs`. `assert_end_state` checks `findings == []` and compares the records' `(pane_id, grid)` with Herdr's snapshot for workspace `hlr649` | Met (advisory 4) |
| 9 | The profile scenario | `scenario::the_profile_scenario_runs_on_scratch_instances`: create from current; launch `--profile` adds the spec; close `--profile` removes it and the port then refuses; show and list; `delete` refused with `profile-has-live-panes` (exit 3); `--keep-panes`; then both rows have a null profile and their servers still answer | Met |
| 10 | The grid launches sit at row 2, column 1 in Herdr's own snapshot, each in a workspace whose r1c1 a Holler launch made first (D23) | `scenario::the_two_grid_spellings_land_at_row_2_column_1_in_herdrs_snapshot`: `g0` and `h0` at r1c1, then `c1r2` and `2,1`, both exiting 0 | Met |
| 11 | Every `--format=json` output parses against the envelope, each `watch` line too | `Scratch::json` runs #638's `check_envelope` on every JSON run. `switch_once_the_tui_takes_it` checks each try. The scenario's final `pane watch --until-idle` checks each line. In the default run: `watch::pane_watch_prints_the_put_of_a_client_write_each_line_an_envelope` | Met |
| 12 | The scenario's launch resolves `attach_tui` before the record exists, pinned by a test | Indirect in the scenario. Launch writes no record before `attach_tui` and no label (E26), so a launch that exits 0 resolved from the bindings. The direct pin is row 14 | Met |
| 13 | The server argv runs under a real tmux pty (D15) | `scenario::the_server_argv_runs_under_a_real_tmux_pty_and_stops` runs the default spec and an agent spec. It makes `ensure_session` with an absolute, existing `cwd` first, then `run`, `wait_healthy`, the model guard, `verify_serving` and `stop_owned`. It checks that the port refuses and the owned pids are gone | Met |
| 14 | `attach_tui` resolves before the record exists, with the decorators over fakes in the default run (D8, D24) | `bind::launch_resolves_attach_tuis_pane_from_the_bindings_before_the_record_exists` runs the real `pane launch` over `BindingHerdr` and `BindingHost` wrapping the kit's fakes. No record and no label exist, so only (a) can answer. It sees `[(Ok(demo-c1r1), false)]`, then one record. Supporting tests: the binding order rows, and the Herdr and host conformance suites over the decorators (W-12) | Met |
| 15 | No credential is printed, written to a file or put on an argv (D28) | **By construction, read in the code.** The wiring reads only the six `HOLLER_*` variables of the module table, plus `HERDR_SOCKET_PATH`, `XDG_CONFIG_HOME`, `HOME` and `PATH`. Its OpenCode environment is `ProcessEnv::Inherit`, and `serve_args`' error quotes none of the value. **Pinned:** guards 4 and 5 pin that the binary's and the tmux server's environments hold nothing else. D28's sentinel check, `Scratch::assert_no_credential_leaked`, runs twice in the main scenario and once in the profile scenario | Met (advisory 1) |

Each test asserts outcomes, not the implementation: codes, messages, call logs, files, sockets, processes and Herdr's own
snapshot. The mutation runs (M1 to M8, the 5 s pin, `move`) show the critical branches fail when the behavior is removed.
The two gaps are REWORK 1 and 2.

## Spec compliance

The decisions D1 to D32, as built:

- **D1, D2, D26, D27.**
  - `connect()` keeps its signature, so `main.rs` and `say_cmd.rs` are untouched. It makes one `control/status`
    exchange (`reach_hub`, `ControlCall::status()`'s 5 s). Every failure of that exchange is `unavailable` naming the
    hub.
  - `from_env` builds every port with no I/O. The one exception is the `PATH` search for `opencode`, which is a `stat`
    and never runs it.
  - Configuration errors are stored and answered at the port's first use. `Wiring` holds no `Unwired`, and `Unwired`
    stays.
  - The actor is `holler pane`, through `Actor::parse`.
- **D3.** `call` and `timed_out` (`store.rs:342-378`) are the one mapping. I read them against `send_over`'s error
  sources:
  - a failed connect is `NoLiveHub`;
  - a failed write, flush or read is `Io`;
  - EOF, a reply that does not decode, or a reply with no result is `BadReply`;
  - a JSON-RPC error is `Refused`.
  So a read's `Io` with `WouldBlock` or `TimedOut` is `timeout`, a write's `Io` or `BadReply` is `timeout`, and the
  rest are `unavailable`. That is D3's table exactly.
  - Every request carries 10 s, and `*/watch` requests carry `WATCH_WAIT` + 5 s.
  - The `op` names match the test kit's (`pane_store.*`, `profile_store.*`, checked by grep).
  - Undecodable `data` is `unavailable` naming the method (W-9).
  - The only `control::run` caller in `holler-cli/src` is `transport.rs:35`. There is no `UnixStream` in `src`.
- **D4.** `watch()` makes the first request eagerly, so a refused cursor and a missing hub are `watch()`'s own answer.
  An empty batch yields `Ok(None)` once, the next request resumes from the reply's cursor, and an error yields one `Err`
  and ends the stream.
- **D5.**
  - The session comes from `HOLLER_HERDR_SESSION` only.
  - The socket chain is `HOLLER_HERDR_SOCKET`, then `HERDR_SOCKET_PATH`, then
    `$XDG_CONFIG_HOME|$HOME/.config/herdr/sessions/<s>/herdr.sock`.
  - The workspace grammar matches, and each listed malformation is `usage` naming the variable, at the Herdr port's
    first use.
  - `--herdr-session` is not defaulted.
- **D6.** One `HOLLER_TMUX_SOCKET` feeds both `TmuxSocket` types. A relative, empty or non-UTF-8 value is `usage` from
  the host and the harness (`Wired<P>`), never the default socket and never `Unwired`.
- **D7.** `opencode_bin` is absolute: the variable, or the first executable on an absolute `PATH` entry. A missing
  binary is `unavailable` naming `HOLLER_OPENCODE_BIN`. `serve_args` is a JSON array of strings. The harness uses
  `ProcessEnv::Inherit`, `Timeouts::default()`, and `tmux` from `PATH`.
- **D8.**
  - The resolver tries (a) the bindings, (b) the one record on the configured session and pane id (two or more are
    `unavailable`), then (c) a label in the session's snapshot. Otherwise it answers `pane-not-found`.
  - The single pending slot is overwritten by every `ensure_pane`, and a failed one offers `None`. So a launch that
    fails between `ensure_pane` and `ensure_session` cannot hand its pane id to the next launch of a `profile apply`
    run.
  - The engine's only call sites are `tx_launch.rs:650` and `:656`.
  - The W-C bound is recorded in ADR section 2 and in "Wiring as built".
- **D9, D29, D30.** There is no flag, option or variable for a bound. The scenario asserts no timing margin; it only
  prints the first launch's time.
- **D10 to D15, D21 to D23, D28.** The tests implement them as written, rows 1 to 15 above. That includes the six
  guards, the exact environments (`LANG` per `c13400a`), the r1c1-first grid launches, the agent spec of D15 (W-L), the
  `Busy` read after `reset --first` (D12), and teardown by TERM then KILL, then `kill-server`, then `herdr server stop`,
  then removal.
- **D11.** The three rigs are included by `#[path]` and generalised in place:
  - `Flavor` and the `_as`/`_in` variants in `scratch_herdr`;
  - `pub` and the extracted `lay_out` and `isolated_vars` in the rig;
  - `real_tmux/mod.rs` moved out of `real_tmux_test.rs`.
  The adapter crates' default-run tests pass unchanged (T-green's workspace run).
- **D16.** Only `crates/holler-cli/Cargo.toml` changed, with a #649 comment on the three adapter dependencies, plus
  `Cargo.lock`. `holler-hub` is untouched, `pane_wiring.rs` included.
- **D17, D18, D19.** The ADR's "Wiring as built (#649)" paragraph records D1 to D9, W-B (#828 and the operator's
  stopgap), W-C, B-1 and the #825 actor. Each sentence D17 lists is edited:
  - the `HostPort` amendment;
  - `--herdr-session` (deferred to #654, no story planned);
  - DRIVEN (#654) at all four places;
  - the first message (#828) at both places.
  The CHANGELOG `## [Unreleased]` entry links #649, and the module doc has the variable table.
  - One wording follows a later decision. ADR line 511 now names #829, where D17's parenthetical says "#642 only".
    D18 (f) and A's W-N (a) name #829, so the ADR is right and D17's parenthetical is stale (Notes for O, item 4).
- **D24, D25.** D24's names are kept, and every file is under 600 lines. Deviations A reviewed:
  - `call`'s fourth parameter is an `Effect` enum, not a bool. `call` is private, so no test is affected;
  - `stores()` returns trait objects;
  - `Wired<P>` is added, with `WiredHarness` as an alias whose `new` is unchanged, and `WiredHost`;
  - `env::Lookup` is an alias of D24's type.
- **D31.**
  - W-4: every decorator forwards every method, `send_prompt` and `session_activity` included, and a recording test
    pins it.
  - W-6: the conformance suites run over the clients against a real hub, one hub per case.
  - W-12: the D8 pin and the decorator suites.
  - W-9: "T pins each row of D3's mapping" is **not fully met**. The write half of the no-hub row is unpinned
    (REWORK 1).
- **D32.** No `TRIAL`, `eprintln!`, `Unwired` field, `HERDR_SESSION` read or default-socket fallback. The manifest
  comment is present. There is no `UnixStream` in `src`. The rigs are not copied (A-dup).

No silent deviation in production code. The one silent test deviation is W-9, REWORK 1.

## Quality audit

- **Correctness and failure handling.**
  - Each port fails closed on a bad configuration.
  - A lost write reply is `timeout`, and the client never retries a write, so nothing is applied twice.
  - `LazyHerdr` builds once (`OnceLock`) and keeps a failed build.
  - The bindings' lock tolerates poison; each change is one assignment or one insert.
  - The resolver's reads are thread-safe for `profile apply`'s concurrent observations.
  - I found no lost-write or race defect.
- **Build guards.**
  - No `unwrap`, `expect`, `panic!`, `unreachable!` or print in `src/pane/wiring.rs` or `src/pane/wiring/*`.
  - The six `#[allow]`s added are all in tests, and each carries `// #649`.
  - The largest touched `.rs` file is 579 lines (`pane_integration/bind.rs`). `support/mod.rs` is 857 lines (+8, only
    `StateDir::at`, as D22 and W-I allow). Every touched `.rs` file is under 900.
  - Clippy `-D warnings`, `lint.sh` and `cargo machete` were clean at T-green.
  - No `unsafe`.
- **Protocol.** No change: no new method, field or error code, no golden file, and no `docs/protocol/v2.md` edit is
  needed. The client speaks the hub's existing `pane/*` and `profile/*` methods with `holler_pane::reply`'s structs.
- **Tests.**
  - Cross-process behavior runs against real hub subprocesses (`support::Hub`). The fake hub is used only for the
    failure rows (D13).
  - No fixed sleep is used for synchronization. The fake hub's sleeps simulate a slow hub, and the scenario polls with
    `wait_for`.
  - RED-first evidence is in T's handoffs.
  - The suite is proportionate: 61 default-run and 4 opt-in tests. I found no tautological or duplicate test: the
    reader rows in `env.rs` and the ports rows in `wired.rs` sit at different tiers for different clauses of D6.
  - Gaps: REWORK 1 and 2, and advisories 1 to 4.
- **Documentation.**
  - The CHANGELOG `## [Unreleased]` entry links #649, #633, #825 and #828 (`changelog-check` ok).
  - ADR-0021 has the "Wiring as built" paragraph and D17's edits.
  - `wiring.rs`'s module doc tables every variable.
  - There is no operator page: D17 leaves `docs/pane-control.md` to #652 (advisory 7).
  - One wording slip in the CHANGELOG (advisory 6).
- **Public-repository privacy.** I grepped the added lines, handoffs included, for hostnames, tailnet names, IPs,
  account names, private domains, home paths and key shapes. The only hits are the generic `/home/u` in `env` and guard
  rows and the three sentinel variable names. The sentinel value is made at run time, so it gives gitleaks nothing to
  match, unlike #650's.
- **Commit and PR hygiene.**
  - All 24 branch commits have `chore(#649): …` Conventional subjects and a `Co-Authored-By` trailer, the form of
    #650's and #664's pipeline commits.
  - The squash title and the PR body's AI disclosure belong to the PR phase (Notes for O, item 5).
- **CI readiness.** The branch as it stands fails one CI test, the rename scan. It is not F's or T's code (Notes for O,
  item 1).

## Scope check

F delivered the brief's scope: the wiring of D1 to D9 in the three D24 submodules, the manifest, the ADR and the
CHANGELOG. T delivered D10 to D15, D21 to D23 and D28, with D11's in-place generalisations. Every changed file is inside
the Blast radius.

The extras are small, and A reviewed each one (fifth pass and A-dup):
- `Wired<P>`, which generalises the harness wrapper so the host fails closed as W-E requires;
- the environment edges in F's Design decision 6 (every path absolute, the XDG rule, an empty session is no session);
- the ADR's section-2 pointer and three consistency edits.

Nothing is under-delivered. W-R (`docs/testing.md`) was optional and is outside the Blast radius, and D5 and D18 (b)
defer defaulting the flag.

## Verdict

**REWORK**, test-only. **No `src/` change is required.** Both changes are in `crates/holler-cli/tests/pane_integration/fake_hub.rs`.

1. **D3's no-hub row for writes is not pinned** (D31 W-9: "T pins each row of D3's mapping").
   - The module doc says "The rows (A's W-9), each pinned below" (`fake_hub.rs:10`). Its table lists "no socket
     (`NoLiveHub`) | … | `unavailable`, naming the hub" for writes (`:14`).
   - But the only no-hub client row, `a_client_call_with_no_hub_is_unavailable_naming_the_hub` (`:318-323`), calls
     `HubPaneStore::list`. No test sends a write with no hub.
   - So a mutant that adds `ControlError::NoLiveHub` to `timed_out`'s write arm (`store.rs:375`) survives every test.
     That mutant would wrongly tell the verbs a write "may have landed" when the request never left.
   - **Change:** add a row that runs each of the five writes (`pane/cas_put`, `pane/delete`, `profile/cas_put`,
     `profile/delete`, `profile/rename`) against a state dir with no hub. It asserts `unavailable`, never `timeout`, and
     a message naming the missing hub: "no live holler hub reachable" and the state dir.
2. **AC 3's "every other failure of a read is `unavailable` naming the hub" is asserted only for the code.**
   - `a_read_whose_reply_is_lost_is_unavailable` (`fake_hub.rs:216-220`, `BadReply`) and the read half of
     `a_json_rpc_error_is_unavailable_for_reads_and_writes` (`:296-303`, `Refused`) check only `code == "unavailable"`.
   - **Change:** also assert that each read's message names the hub and its state dir. That is `hub_unavailable`'s
     second arm, `store.rs:387`: "the holler hub at <state dir> did not answer <method>: …". For example, `each_read`
     can return the fake hub's state path.
   - A bare `contains("hub")` would not discriminate. `BadReply`'s display ("bad reply from the hub") and the fake's
     refusal text ("the fake hub refuses") both contain the word. Today a mutant of that arm that drops the hub and its
     state dir survives.

## Notes for T

- Both items fit in `fake_hub.rs`. After them, the module table at lines 10 to 18 is true as written.
- When you rewrite `handoff-T-green.md`, do not quote the setup wizard's old skill directory path. Line 48 of the
  current one is one of the three rename-scan hits (Notes for O, item 1).

## Notes for O

1. **This blocks the merge. It is not F's or T's code.**
   - CI's workspace run includes `wizard_scripts`. Its test `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note`
     scans every git-tracked file for the setup wizard's pre-#820 directory path: `agent-skills/` followed by the old
     skill name.
   - Three tracked lines on this branch carry that path:
     - `docs/handoffs/649-brief.md:1197`, E64's Source line. The path has been stale since #820 renamed the skill: the
       file is now `agent-skills/herdr-setup/lib/herdr.sh`, with the same comment at lines 9 to 11;
     - `docs/handoffs/649/decisions.md:66`, the same citation in A's first entry;
     - `docs/handoffs/649/handoff-T-green.md:48`.
   - T-green and F record the failure (2312 passed, 1 failed). They expect Phase 11 to remove `docs/handoffs/` before
     the push. Since Aftersight #558, the release does that only when the run passes `removeHandoffsBeforePr: true`
     (`coding-pipeline.workflow.mjs:4734-4754`). This repo keeps its handoffs: `main` carries `docs/handoffs/650/`,
     `664/` and others.
   - **Fix before the PR**, in one of two ways:
     - correct E64's path in the brief and reword the two handoff lines (T rewrites its own);
     - run the PR phase with `removeHandoffsBeforePr: true`.
   - Otherwise `test (ubuntu-latest)` and `test (macos-latest)` fail, and the branch-protected PR cannot merge.
   - **Follow-ups:**
     - (a) the wizard scan could skip `docs/handoffs/`, as `docs_cli_test` does (#462). That is outside this Blast
       radius;
     - (b) for Aftersight: `pipeline-conventions.md` §1 still says Phase 11 always removes the handoffs. That
       contradicts the script since #558, and it misled F and T here.
2. **Still unfiled.** `gh issue list --search` finds none of these:
   - T's two product findings:
     - the OpenCode adapter's TUI query fails without a UTF-8 locale. The scenario sets `LANG=C.UTF-8` for it;
     - a `pane switch` right after `pane relaunch` times out until the TUI accepts `/tui/select-session`. The scenario
       retries it for up to 20 s and prints each retry.
   - the `LinkedHost` consolidation (W-O, with A-dup's W-1 to W-3);
   - for Aftersight: W-U, and F's two diff-gate input gaps (the 12,000-byte evidence cap, and no `--handoff`);
   - carried from A: the #825 comment, #653 and #828 (`pfleet o reset`), and #803 not listing #649.
3. **Sync the issue.** The issue body's D10 (5) lacks the `LANG=C.UTF-8` clause that `c13400a` added to the brief.
   The issue is the source of truth, so it should carry the clause.
4. **A brief nit.** D17's "(#642 only)" for ADR line 511 is stale against D18 (f) and A's W-N (a). The ADR's "#829" is
   right.
5. **For the PR.** The PR body needs CONTRIBUTING's AI disclosure (`gh pr edit`), and the squash title must be a
   Conventional Commit subject. The issue's title already is one.

## Advisory notes

1. **D28's sentinel cannot reach the code under test.**
   - `Scratch::start` plants it in the test process's environment after the hub, Herdr and the tmux server have
     started (`scratch/mod.rs:235-243`).
   - The binary runs with `env_clear()` and `binary_env`, per D10 (4). The tmux server, and so every OpenCode server
     and TUI, has exactly the rig's pairs, per D10 (5).
   - Only the hub restarted at step 8 and the test's own children ever hold the sentinel. So the check could not catch
     the wiring forwarding a credential onto an argv.
   - Today AC 15 holds by construction (row 15), and T implemented D28 as written. But D10's exact environments make
     D28's observation nearly vacuous.
   - #829 will carry a pane's environment to its server. **Recommendation for #829:** plant the sentinel in the private
     tmux server's global environment, the path a production credential takes (E29). Amend D10 (5)'s exact set and
     guard 5 to admit it, then grep `ps` argv, the outputs and the files.
2. **Read timeouts under a real hang name only `pane_store.get` and `pane_store.watch_next`.** The other read `op`s
   are unpinned constants. Each pin would cost a 10 s wait, and D9 admits no knob. I checked each name against the test
   kit's by grep. No change asked.
3. **Scenario step 7 asserts only that `doctor` reports some finding for C1** (`scenario.rs:228-231`). That is not
   vacuous: a stray session is reported with no pane (`reconcile.rs:397`), so C1 should have none before the delete.
   Asserting the kind (`session-of-record-missing` or `tui-foreign-session`) would pin what the delete caused.
4. **The end state reads the registry with `pane get` of C1 and C2** (`scenario.rs:245-251`), not with `pane list` as
   AC 8 words it. The two are equivalent while the scenario registers exactly those two panes, but `pane list` would
   also catch a stray record.
5. **A-dup's W-1 to W-4 stand as follow-up material:** the third `alive`, `tmux_on_path`, the two small copies inside
   the target, and the third `Feed` shape.
6. **`CHANGELOG.md:17` says the variables are "each read when its adapter is first used".** In fact `connect` reads
   them all once, and a bad value is answered at the port's first use. The effect the entry describes is right; the
   verb is not.
7. **The operator needs a page for the six variables.** Today they are in the CHANGELOG, ADR-0021 and `wiring.rs`'s
   module doc only. D17 leaves `docs/pane-control.md` to #652, which should carry the table.
8. **A non-UTF-8 `HOLLER_HERDR_WORKSPACES` reads as no workspace** (F's Design decision 9). It fails closed: every
   launch is refused `unavailable`, naming the workspace, not the variable. Carried from T-green.
