# Handoff-T-red: Phase 4 - #649 hub wiring and the scratch integration scenario

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `eb94951` plus this phase's staged changes)
**Brief / wireframe reviewed:** `docs/handoffs/649-brief.md` (D1 to D32, E1 to E70); `docs/handoffs/649/handoff-A.md` (fourth pass). No wireframe (no UI surface).

## A precondition

Confirmed: A returned **PASS** on the plan (fourth pass, commit `eb94951`). A's binding warns for T are applied: W-P (the no-hub rows), W-O (the `LinkedHost` stays as test-local glue), D31's W-6 and W-12, and D32's checklist for T (the probe is deleted).

## How this RED was made

The tree carries the satisfiability trial of `046026f` (`src/pane/wiring/{bind,env,store}.rs`, with `TRIAL` headers). D32 says where RED evidence comes from: "the rows the trial fails (D4's eager request, D5's dropped fallback, D23's r1c1-first launches, W-E's fail-closed tmux socket)". So the suite was run against the trial.

- The 8 RED tests below are exactly those rows. D5 also has its stricter workspace validation.
- Every other default-run test pins behaviour the trial already has, and passes. Against `main`'s stub (`connect` hands out `Unwired`), each of them fails: none of the clients, decorators or `env` functions exist there, and every verb answers `not-implemented`.
- D23 is no longer a RED row. The WIP's grid test launched into empty workspaces, so it failed on its own setup (`grid-unreachable`). The test now launches r1c1 first, as D23 says, and passes against the trial on scratch instances.

## Tests authored

All are in the one existing target, `crates/holler-cli/tests/pane_integration/` (`[[test]] pane_integration`, already declared, E50). It has **58 default-run tests and 4 opt-in tests**. Files this phase changed or added: `main.rs` (`with_env`), `no_hub.rs`, `env.rs`, `store_only.rs`, `fake_hub.rs`, `bind.rs` (one message), **new `wired.rs`**, `guards.rs`, `scenario.rs` (rewritten) and `scratch/mod.rs`. Also `crates/holler-adapter-host/tests/real_tmux_test.rs` and `real_tmux/mod.rs` (D11, see below).

### Default run (CI)

| Test (module::name) | Criterion / behaviour pinned | Tier, and why |
|---|---|---|
| `no_hub::every_verb_with_no_hub_is_unavailable_naming_the_hub_in_json` / `_in_text` | AC 1, D2, D28. Every `pane`/`profile` verb with no hub exits 1, `unavailable`, naming the hub and its state dir. No adapter trap is touched: a held Herdr listener, a tmux socket path, an `opencode` that leaves a mark, and `PATH` an empty directory. W-P: `pane import --from … --herdr-session …` and `profile apply hlr649` now parse. | Process. Only the binary shows `main.rs`'s `connect`-first path. |
| `store_only::the_store_only_verbs_work_with_no_herdr_tmux_or_opencode` | AC 2, D2: `pane list/get/watch`, `profile create/list/show` against a hub, with nothing else on `PATH`. | Process, real hub. |
| `store_only::a_herdr_verb_with_no_session_configured_is_usage_naming_holler_herdr_session` | D5 (audit). `launch` with only `HERDR_SESSION` set answers `usage` naming `HOLLER_HERDR_SESSION` only, and writes nothing. `HOME`/`XDG_CONFIG_HOME` are scratch. **RED.** | Process: the whole path from the environment to the envelope. |
| `store_only::a_restarted_hub_serves_the_registry_it_had` | D13: the hub restarts and `pane list` is equal. | Process, real hub. |
| `clients::the_pane_client_passes_the_pane_store_conformance_suite` / `the_profile_client_…` | D3, D13, W-6: the test kit's suites over `HubPaneStore`/`HubProfileStore` against a real hub, one hub per case. | Integration (in-process client, real hub). |
| `fake_hub::a_write_whose_reply_is_lost_is_timeout` | AC 5, D3, E40: every write (`pane/cas_put`, `pane/delete`, `profile/cas_put`, `profile/delete`, `profile/rename`) whose reply is lost answers `timeout` naming its op. | In-process client against a fake socket: the cheapest way to lose a reply on purpose, D13. |
| `fake_hub::a_read_whose_reply_is_lost_is_unavailable` | D3: a lost reply to a read is `unavailable`. | Same. |
| `fake_hub::a_write_the_hub_never_answers_is_timeout_within_the_bound` | D3, I5: `timeout` naming `pane_store.cas_put`, in under 13 s. | Same. |
| `fake_hub::a_read_the_hub_never_answers_is_timeout_within_the_bound` | AC 3, D3 (B-1): `timeout` naming `pane_store.get`, in under 13 s. | Same. |
| `fake_hub::a_watch_request_the_hub_never_answers_is_timeout_naming_watch_next` | D3: a watch request the hub never answers is `timeout` naming `pane_store.watch_next`, within `WATCH_WAIT` + 5 s. Holds whether the request is `watch()`'s or `next()`'s. | Same. |
| `fake_hub::a_json_rpc_error_is_unavailable_for_reads_and_writes` | AC 3, D3: `Refused` is `unavailable` for all 10 methods. | Same. |
| `fake_hub::data_that_does_not_decode_is_unavailable_naming_the_method` | W-9: undecodable `data` is `unavailable` naming the method, never `usage`. | Same. |
| `fake_hub::a_client_call_with_no_hub_is_unavailable_naming_the_hub` | W-9: `NoLiveHub` gives `no live holler hub reachable`. | Same. |
| `fake_hub::a_watch_error_yields_one_err_and_ends_the_stream` | D4: one idle batch, one `Err`, then `None`, and no third request. | Same. |
| `fake_hub::watch_makes_its_first_request_when_called_and_keeps_the_batch_for_the_first_next` | D4 (audit). `watch()` sends request 1; the first `next()` sends nothing; the second `next()` sends one, with `since` taken from the reply's cursor. **RED.** | Same. |
| `fake_hub::a_cursor_ahead_of_the_head_is_refused_by_watch_itself` | D4 (audit). The hub's `usage` for a cursor ahead of the head comes from `watch()`, before any item, for both clients. **RED.** | Same. |
| `fake_hub::watch_with_no_hub_is_unavailable_from_watch_itself` | D4 with D3: with no hub, `watch()` itself fails. **RED.** | Same. |
| `fake_hub::a_watch_poll_waits_past_the_default_control_timeout` | D4, E57: a long-poll answered after 6 s is idle, not `timeout`. | Same. |
| `fake_hub::connect_is_unavailable_when_the_status_exchange_fails` | D2: an error, a close and a 5 s silence on `control/status` are each `unavailable`. | Same. |
| `watch::pane_watch_prints_the_put_of_a_client_write_each_line_an_envelope` | D14, AC "every watch line is an envelope". | Process, real hub. |
| `watch::the_profile_clients_watch_yields_a_put_from_cursor_0` | D14. | In-process client, real hub. |
| `env::the_herdr_session_is_hollers_variable` | D5. | Unit, through the injected lookup of D24. |
| `env::herdrs_own_session_variable_is_no_fallback` | D5 (audit): `HERDR_SESSION` alone is `usage` naming only `HOLLER_HERDR_SESSION`. **RED.** | Unit. |
| `env::no_herdr_session_is_usage_naming_holler_herdr_session` | D5 (audit): the message offers no `HERDR_SESSION`. **RED.** | Unit. |
| `env::the_herdr_socket_is_hollers_variable_then_herdrs_then_the_session_default` | D5, E25 (`HERDR_SOCKET_PATH` stays a socket fallback). | Unit. |
| `env::the_workspaces_are_label_rows_by_columns` | D5, D9 (the 10 s bound). | Unit. |
| `env::malformed_workspaces_are_usage_naming_the_variable` | D5 (audit), all listed malformations: no `=`/`x`, an empty label, a label with `=`, an empty item, a zero or non-numeric extent, a repeated label. Every failing row is reported. **RED** (`0x1`, `2x0`, the repeated label). | Unit. |
| `env::the_tmux_socket_is_an_absolute_path_or_the_default` / `env::a_relative_tmux_socket_is_usage_never_the_default_server` | D6. | Unit. |
| `env::the_opencode_binary_is_hollers_variable_else_opencode_on_path_made_absolute` / `env::no_opencode_binary_is_unavailable_naming_the_variable` / `env::the_serve_args_are_a_json_array_of_strings_default_none` | D7. | Unit. |
| `env::malformed_adapter_settings_do_not_stop_the_store_only_verbs` | D26: every setting is malformed and `pane list` still works. | Process. |
| `wired::a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server` | D6, W-E. Through `Wiring::connect` and `ports()` with a relative `HOLLER_TMUX_SOCKET`: `connect` succeeds; then `host.ensure_session`, `host.ps`, `harness.health` and `harness.shown_session` each answer `usage` naming the variable. `TMUX_TMPDIR` points at a trap, so a default-socket use would leave a `tmux-<uid>` directory; none appears, and the `opencode` trap never runs. **RED.** | In-process wiring against a real hub. This is the brief's "over the wiring" form. `connect()` takes no lookup, so the variables are set under the state-dir lock and restored afterwards (`with_env`). |
| `bind::*` (15) | D2 (`LazyHerdr` is built once, and a build error is kept); W-4/D31 (the wired harness forwards all 11 methods, the two default-armed ones included; a misconfiguration answers from each); W-12 (`BindingHerdr` and `BindingHost` pass the `HerdrPort` and `HostPort` suites); D8/W-5 (the resolver's order: bindings, then the registry by session and pane id, then the label; another session's record does not match; two records give `unavailable`; each `ensure_session` claims its own pane); and the pin of D8's call order: `pane launch` over the decorators resolves `attach_tui`'s pane from the bindings before the record exists (AC). | Unit, over the test kit's fakes. |
| `guards::*` (7) | D10 guards 1 to 5 as pure functions, including the exact binary environment (now with `LANG`, see Finding 1). | Unit. |

### Opt-in (`#[ignore]`; `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1`)

| Test | Criterion |
|---|---|
| `scenario::the_scenario_runs_in_order_on_scratch_instances` | AC "in order" and AC "end state", D12, D21, D28. Steps: launch ×2; relaunch C1 (session kept); `reset C2 --first`, after which `session_activity` reads `Busy` (D12); switch C1 to a session made through `HarnessPort::create_session` (W-G); park C2; kill C2's server (pids `@holler-pid` ∩ `HostPort::ps`) and delete C1's switched-to session (raw `DELETE`); one `doctor` finds C2 `server-down` and a finding for C1; restart the hub, after which `pane list` is equal; repair (`reset C1`, `unpark C2`, `relaunch C2`); delete strays (listed through `list_sessions`, keeping every pane's session of record). End state: `doctor` has no finding and the registry equals Herdr's snapshot. Every `watch` line is an envelope; the tmux environment is scratch only; no sentinel leaked. |
| `scenario::the_profile_scenario_runs_on_scratch_instances` | AC "profile scenario", D28. |
| `scenario::the_two_grid_spellings_land_at_row_2_column_1_in_herdrs_snapshot` | AC "grid", D23: `hlr649-g0`/`hlr649-h0` take r1c1 first, then `c1r2` and `2,1` sit at row 2, column 1 in Herdr's snapshot. |
| `scenario::the_server_argv_runs_under_a_real_tmux_pty_and_stops` | AC "server argv under a real tmux pty", D15, W-L: the default spec **and** `agent: plan`. Checks health, the model guard and `verify_serving`; `stop_owned` leaves the port refused and no owned pid alive. |

The scratch set (`scratch/mod.rs`) now also has:
- `Scratch::harness()`, the test's own `HarnessPort` (W-G);
- D28's sentinel: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` and `OPENROUTER_API_KEY` are planted in the test process. The test greps every verb's output, every file under the scratch directory, and `ps -eww` argv;
- D10 (6)'s teardown in `Drop for Scratch`: `TERM` then `KILL` to the process groups from `list-panes -a` plus `HostPort::ps`, then kill-server, then `herdr server stop` (fields reordered), then the directory.

### Deleted or merged

- `scenario::zz_probe_attach_after_run` is deleted (D24, D32).
- `fake_hub::an_empty_watch_batch_is_idle_and_the_next_poll_resumes_from_its_cursor` is merged into the eager-request test. That test asserts the same cursor resumption, so keeping both would duplicate it.
- `env::no_herdr_session_is_usage_naming_both_variables` and `env::the_herdr_session_is_hollers_variable_else_herdrs` pinned the fallback that D5 dropped. They are rewritten as the D5 tests above.

### D11: the shared tmux module now passes the adapter's default run unchanged

The WIP's move of the private tmux server broke `holler-adapter-host`'s default test `fake_tmux_test::hosts_are_built_by_one_helper_and_real_tmux_tests_use_private_sockets`. That guard counts exactly one `TmuxHost::new` in `real_tmux_test.rs`, and the move had left none there. Fixed inside the Blast radius, with `fake_tmux_test.rs` untouched:
- `real_tmux/mod.rs` now builds only the server (`private_server`, `isolated`);
- `real_tmux_test.rs` keeps its one host helper, `private`;
- the scenario builds its host on the isolated server's socket with `TmuxHost::new`.

`cargo test -p holler-adapter-host`: 29 + 0 + 0, all pass. `--ignored` `real_tmux_test`: 9/9 pass.

The other two generalised rigs, checked the same way:
- `cargo test -p holler-adapter-herdr -p holler-adapter-opencode` (default run): all pass.
- `HOLLER_HERDR_SCRATCH=1 … -p holler-adapter-herdr -- --ignored`: 2/2 pass.
- `HOLLER_TEST_OPENCODE=1 … --test real_opencode_test -- --ignored`: 11/12 in one long run. `send_prompt_queues_a_turn_and_session_activity_follows_it` failed there and passed alone right after (1/1, 3.5 s). It is a timing flake of #642's opt-in test that the shared rig did not cause (D11 made only items `pub` and split out `lay_out`/`isolated_vars`). Recorded here so GREEN can watch for it.

## RED confirmation

Command, against the trial: `cargo test -p holler-cli --test pane_integration`.
Result, final run after every edit of this phase: `test result: FAILED. 50 passed; 8 failed; 4 ignored; 0 measured; 0 filtered out; finished in 107.55s`.

Every one of the 8 fails on an assertion about the missing behaviour. None fails on a compile error, a missing `[[test]]` or a harness timeout. The target compiles clean under `cargo clippy -p holler-cli --test pane_integration -- -D warnings`.

```text
env::herdrs_own_session_variable_is_no_fallback
  panicked at env.rs:60: HERDR_SESSION alone configures nothing:
  HerdrConfig { session: "holler649-0000000b", socket: "/tmp/h/herdr.sock", workspaces: {}, timeout: 10s }

env::no_herdr_session_is_usage_naming_holler_herdr_session
  panicked at env.rs:70: no Herdr session: set HOLLER_HERDR_SESSION or HERDR_SESSION

env::malformed_workspaces_are_usage_naming_the_variable
  "hlr649=0x1": want `usage` naming the variable; got Ok(HerdrConfig { .. workspaces: {"hlr649": Extent { rows: 0, cols: 1 }} .. })
  "hlr649=2x0": want `usage` naming the variable; got Ok(HerdrConfig { .. Extent { rows: 2, cols: 0 } .. })
  "hlr649=2x1,hlr649=1x1": want `usage` naming the variable; got Ok(HerdrConfig { .. {"hlr649": Extent { rows: 1, cols: 1 }} .. })

store_only::a_herdr_verb_with_no_session_configured_is_usage_naming_holler_herdr_session
  left: "unavailable" right: "usage"  (the trial took HERDR_SESSION as the session and tried
  its default socket, inside the trap: "the Herdr socket \"/tmp/hlr649-XsVeuj/herdr/sessions/
  holler649-0000000b/herdr.sock\" cannot be connected to (entity not found)")

fake_hub::watch_makes_its_first_request_when_called_and_keeps_the_batch_for_the_first_next
  assertion `left == right` failed: watch() itself sent the first request  left: 0 right: 1

fake_hub::a_cursor_ahead_of_the_head_is_refused_by_watch_itself
  panicked at fake_hub.rs:357: watch() refuses the cursor itself: ()

fake_hub::watch_with_no_hub_is_unavailable_from_watch_itself
  panicked at fake_hub.rs:371: watch() fails at once: ()

wired::a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server
  host.ensure_session: want `usage` naming HOLLER_TMUX_SOCKET; got Err(NotImplemented)
  host.ps: want `usage` naming HOLLER_TMUX_SOCKET; got Err(NotImplemented)
  harness.health: want `usage` naming HOLLER_TMUX_SOCKET; got Ok(false)
  harness.shown_session: want `usage` naming HOLLER_TMUX_SOCKET; got Err(Unavailable { what: "the Herdr socket … cannot be connected to …" })
  touched in the trap directory: []
```

Each traces to a decision the trial does not meet yet:
- the `HERDR_SESSION` fallback (D5);
- no zero-extent or repeated-label check (D5);
- the lazy first watch request (D4);
- the host falls back to `Unwired` and the harness to `TmuxSocket::Default` (D6/W-E).

The `store_only` row's run, before `HOME` was made scratch, also showed that the trial's fallback resolves a socket under the operator's real `~/.config/herdr/`.

The opt-in scenarios, run on this machine against the trial on scratch instances: `HOLLER_TEST_PANE_INTEGRATION=1 … -- --ignored --test-threads=1` gives `4 passed; 0 failed`, in 65 s. The first launch took 2.9 to 4.9 s. Afterwards no `/tmp/hlr649-*` directory and no `hlr649` process is left. They are not RED evidence; this run shows the scenario code itself is right.

## Findings for O (outside the Blast radius; none blocks this RED)

1. **The OpenCode adapter's TUI query needs a UTF-8 locale.** Seen with tmux 3.7c. With no `LANG`/`LC_*` in the client's environment, tmux writes the tabs of `QUERY_FORMAT` (`display-message -p`) as `_`. `parse_query` then reads `Query::NoPane`, and every `attach_tui` fails with `unavailable: no tmux session for pane …`.
   - Reproduced with plain tmux: `env -i PATH=… tmux -S s display-message -p '#{session_name}\t#{pane_dead}'` prints `x_0`. With `LANG=C.UTF-8`, or with `tmux -u`, it prints a tab.
   - The scenario's binary environment is built from nothing (D10 (5)), so it now sets a test-chosen `LANG=C.UTF-8`. This is A's W-D reading of "exactly": a value the test sets, not one inherited. `guards::the_binary_env_is_built_from_nothing_inherited` pins it.
   - In production, a `holler` run with no locale (a service unit, `env -i`) cannot launch, relaunch or attach. The fix belongs to #642's crate (`tui::tmux_command` should pass `-u` or set a UTF-8 `LC_CTYPE`). **O files an issue.**
2. **`pane switch` right after `pane relaunch` times out.** The relaunched TUI shows its session, so `attach_tui` returns. But for a few seconds it does not act on `/tui/select-session`, so `harness.select_session` times out after `settle`.
   - Reproduced 3 times out of 3 in the scenario, and 2 out of 3 in a single-pane probe. A switch sent 8 s later succeeds every time.
   - A failed switch records nothing (`tx_switch.rs` step 3). So the scenario sends `pane switch` again while it answers `timeout`, for up to 20 s, and prints the number of tries ("took 2 tries" in each passing run). The race stays visible and is not hidden.
   - A `reset` right after a `relaunch` presumably hits the same race. The scenario's order avoids that pairing.
   - The fix is in #642's `attach_tui` readiness or #644's relaunch, outside the Blast radius. **O files an issue.**
3. **Servers in one project directory share OpenCode's session store.** The scenario's panes share `oc/proj` and one `XDG_DATA_HOME`, so each server lists the other pane's sessions. Doctor's stray rule ("the session of record of no pane") tolerates this, and the scenario's stray cleanup keeps every pane's session of record. Noted for #654's cutover (fleet panes in one directory). No action for #649.
4. **`cargo fmt`.** Locally, rustfmt 1.9.0 reports diffs across the whole `holler-cli` crate, merged files included. CI runs no fmt check (`ci.yml` and `lint.sh` have none), so the new test files were not reformatted. They pass `lint.sh` and clippy `-D warnings`.

## Notes for F (the API these tests build against; D24's names)

- `holler_cli::pane::wiring::Wiring::{connect, ports}`.
- `wiring::store::{HubPaneStore::new(), HubProfileStore::new()}`: both are unit-like, and each call reads `HOLLER_STATE_DIR`.
- `wiring::env::{herdr_config, tmux_socket, opencode_bin, serve_args}`, each `(&dyn Fn(&str) -> Option<String>) -> Result<_, PaneError>`. `tmux_socket` returns `holler_adapter_host::TmuxSocket`.
- `wiring::bind`:
  - `Bindings::new()`;
  - `BindingHerdr::new(Arc<dyn HerdrPort>, Bindings)`;
  - `BindingHost::new(Arc<dyn HostPort>, Bindings)`;
  - `LazyHerdr::new(impl Fn() -> Result<Arc<dyn HerdrPort>, PaneError>)`;
  - `WiredHarness::new(Result<Arc<dyn HarnessPort>, PaneError>)`;
  - `tui_session(&str, Bindings, Arc<dyn PaneStore>, Arc<dyn HerdrPort>) -> Resolver<PaneId, PaneName>`.
- Messages the tests read:
  - a `timeout` names the op (`pane_store.get`, `pane_store.cas_put`, `pane_store.watch_next`, and each write's op);
  - `NoLiveHub` is `no live holler hub reachable at <state dir>`;
  - an undecodable reply names the method (`pane/get`);
  - a configuration error names its variable.
- Every `eprintln!("TRIAL …")` must go (D32). The scenario's output is noisy with them now.

## Ready for F

**RED is valid.** The 8 failing tests fail on the assertions D4, D5 and D6/W-E define, against the trial, and the rest of the suite pins behaviour F must keep while rewriting the trial (D24). F may implement against these tests.

`T-red complete, RED is valid. F may implement against the authored tests.`
