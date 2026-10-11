# Handoff-F: Phase 6 (F, implement) - #649 hub wiring and the scratch integration scenario

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `e4f1d36` plus this phase's staged changes)
**Issue:** #649

## What was done

The satisfiability trial of `046026f` is gone. The three `TRIAL` files are rewritten to the house style under D24's
names, and `connect` is the real one.

- `crates/holler-cli/src/pane/wiring.rs`: `Wiring::connect` and the fields of `Wiring` are the real ports (D1, D2).
  - `connect` makes one `control/status` exchange and then builds every port from the process environment.
  - `Wiring` holds no `Unwired` field (D32). `Unwired` stays.
  - The module doc replaces the stub text and has a table of every environment variable of D5 to D7 (D17).
  - `connect` keeps its signature, so `main.rs` and `say_cmd.rs` are untouched (D1, D27).
- `crates/holler-cli/src/pane/wiring/store.rs`: `HubPaneStore`, `HubProfileStore`, `stores()` and `call`, which holds
  D3's one `ControlError` mapping.
  - A table of 12 `Method` rows: wire name, `op`, read or write, and bound.
  - D4's eager first `watch` request, and `reach_hub`, `connect`'s exchange.
- `crates/holler-cli/src/pane/wiring/env.rs`: `herdr_config`, `tmux_socket`, `opencode_bin` and `serve_args`, each over
  an injected `Lookup`.
  - No `HERDR_SESSION` fallback.
  - Zero extents and repeated labels are now `usage`.
  - Every path a variable gives must be absolute.
- `crates/holler-cli/src/pane/wiring/bind.rs`: the D8 and W-4 port types.
  - `Bindings`, `BindingHerdr`, `BindingHost` and `LazyHerdr`.
  - `Wired<P>`, with the aliases `WiredHarness` (D24's name; `WiredHarness::new` is unchanged) and `WiredHost`, the
    fail-closed host W-E needs.
  - `tui_session`, the lookup of D8.
- `crates/holler-cli/Cargo.toml`: a comment naming #649 on the three adapter dependencies (D16). The trial had already
  added the dependencies, so `Cargo.lock` is unchanged by this phase.
- `docs/adr/ADR-0021.md`:
  - a new "Wiring as built (#649)" paragraph in section 5 (D17, with W-B, W-C, B-1 and W-Q (a)/(b));
  - D17's sentence edits;
  - W-N (a), (b) and (c);
  - a one-sentence pointer in section 2 for W-C's third bounded-otherwise case.
- `CHANGELOG.md`: an `## [Unreleased]` / Enhancements entry linking #649, #825 and #828.

## Design decisions

1. **One generic `Wired<P>` for the harness and the host.**
   - W-E needs a host that answers `usage` naming `HOLLER_TMUX_SOCKET` from every method. The trial fell back to
     `Unwired` instead.
   - The harness already had that shape: a built port, or the configuration error that each method answers.
   - Choice: `Wired<P: ?Sized>(Result<Arc<P>, PaneError>)`, with `impl HarnessPort for Wired<dyn HarnessPort>` and
     `impl HostPort for Wired<dyn HostPort>`. `WiredHarness` and `WiredHost` are type aliases.
   - Alternatives: a second struct `WiredHost` (two copies of the pattern), or a `BindingHost` over a `Result` (which
     changes its D24 signature). Both were rejected.
   - Every method is still forwarded explicitly (W-4).
2. **`store.rs` is a table of `Method` rows, plus D24's `call`.**
   - Each of the 12 methods is one `const` row: wire name, the test kit's `op` name, read or write, and bound (10 s, or
     `WATCH_WAIT` + 5 s for the long polls). That makes D3's table auditable in one place.
   - `call(method, params, timeout, effect, op)` keeps D24's five parameters and is the one `ControlError` mapping.
   - `write` is an `Effect` enum (`Read`, `Write`), not a bool, so the mapping's `match` reads as D3 does.
   - Params are the typed structs of `holler_pane::reply`, not `json!`, so they match the hub's `deny_unknown_fields`.
     `json!` also hides an `unwrap` for non-literal values.
3. **The eager watch (D4).**
   - The iterator holds `batch: Option<VecDeque<E>>`. `watch()` fills it with the first reply.
   - Each `next()` takes the batch, or makes a request when there is none.
   - An empty batch yields `Ok(None)` once. A batch emptied by popping is dropped, so the next `next()` makes a request,
     resumed from the last reply's cursor.
   - An error yields one `Err`, and then `None` (D4, `a_watch_error_yields_one_err_and_ends_the_stream`).
   - I checked both conformance suites' watch cases against the eager form by reading them. Both suites pass against
     a real hub.
4. **Bindings (D8 (a)).**
   - `BindingHerdr::ensure_pane` offers the pane id it answered, or `None` when it failed, so a stale id is never
     offered.
   - `BindingHost::ensure_session` claims the offer whatever its own outcome, and binds only on success.
   - The engine calls `ensure_session` right after `ensure_pane` in `tx_launch::bring_up`, and nowhere else (evidence).
     One slot is therefore enough, including for `profile apply`'s sequential creates.
   - A poisoned lock is taken as it is. Each change is one assignment or one insert, so the state stays whole.
5. **The resolver (D8 (b), (c)).**
   - (b) matches a record only on the join key: the configured session and the pane id (W-5).
   - It matches nothing when no session is configured (`""`), so (c) then answers the Herdr port's own `usage` naming
     `HOLLER_HERDR_SESSION`. That tells the operator what to set.
   - (c) also filters the snapshot by the session, and a label that is not a pane name is no match.
   - `pane-not-found` names the pane id and the session.
6. **Environment edges D5 to D7 leave open.** Each is a judgment call, recorded under Deviations:
   - an empty `HOLLER_HERDR_SESSION` is no session;
   - every path a variable gives (`HOLLER_HERDR_SOCKET`, `HERDR_SOCKET_PATH`, `HOLLER_TMUX_SOCKET`,
     `HOLLER_OPENCODE_BIN`) must be absolute, else `usage` naming that variable. This is D6's rule, applied to all.
     Wiring passes an absolute binary (E30), and a relative socket would otherwise be the adapter's unnamed `usage`;
   - `XDG_CONFIG_HOME` that is empty or relative is ignored, as the XDG spec says. With neither it nor `HOME` absolute,
     the answer is `usage` naming `HOLLER_HERDR_SOCKET`;
   - the empty workspace list is no workspace;
   - the `PATH` search skips relative entries, so the binary found is always absolute;
   - a bad `HOLLER_OPENCODE_SERVE_ARGS` message quotes none of the value. A serde error would echo a string, and serve
     arguments go on an argv.
7. **The order of a harness's configuration errors** is the tmux socket, then the binary, then the serve arguments.
   `harness.health` with a relative socket therefore answers the socket's `usage`, as W-E's test reads. The host and the
   harness share the one socket result (D6).
8. **Message texts.**
   - With no hub: `no live holler hub reachable at <state dir>`, the CLI's own text (W-9). Any other failure of the hub
     names its state dir and the method.
   - A `timeout`'s `op` is the bare test kit name. Context for writes stays with the verbs (`profile_scope.rs` adds "may
     have landed").

## Reuse / extend-vs-new

- **Extended:** #670's `Wiring` (its body and fields replaced, `connect`/`ports` signatures kept) and the trial's three
  submodules (rewritten in place).
- **Reused unchanged:**
  - `StoreScope` (#663) as the `ProfileScope`, with the actor constant `holler pane`;
  - `SystemProber`;
  - `crate::transport::call(None, ..)`, MO 8's one dispatch point (W-9): there is no second caller of `control::run`
    and no `UnixStream` in `holler-cli/src`;
  - `holler_pane::reply`'s params structs and `PaneReply::into_result`;
  - the adapters' own constructors (`HerdrAdapter::connect`, `TmuxHost::new`, `OpenCodeHarness::new`), `HerdrConfig::new`
    and `Timeouts::default()`.
- **New objects:** only those D24 names, plus two.
  - `Wired<P>` generalises the trial's `WiredHarness` to cover W-E's host. One wrapper, not two.
  - `env::Lookup` is a type alias for D24's `&dyn Fn(&str) -> Option<String>`.
- **No analogue exists** for the clients, the bindings or the resolver. The hub's in-process stores are the server side,
  and the test kit's fakes may not be used by production code.

## Architecture notes for A

- **Layers:** `holler-cli` only. The CLI now depends on the three adapter crates. They were already in the tree, and
  ADR-0021 section 5 allows it. `holler-hub` gains nothing, and `pane_wiring.rs` is untouched (D16).
- **Contracts:** no change to the ports, the protocol, golden files or `holler-pane`.
- **New public API in `holler-cli`:** `pane::wiring::{store, env, bind}`, with D24's names plus `bind::{Wired,
  WiredHost}` and `env::Lookup`.
- **Local patterns followed:**
  - `const ACTOR` and `Actor::parse(ACTOR)?` (as `profile/create.rs` does);
  - `unwrap_or_else(PoisonError::into_inner)` for a lock;
  - the module docs carry the decision tables, as `profile_scope.rs` does.
- **File sizes:** `wiring.rs` 382, `bind.rs` 406, `store.rs` 394 and `env.rs` 234 lines, all under the 600 warn line.

## Deviations from spec / wireframe

- **D24's `call`:** the `write` parameter is the `Effect` enum, not a bool. The name, the order and the role (the one
  mapping) are as sketched.
- **D24's `stores()`:** returns `(Arc<dyn PaneStore>, Arc<dyn ProfileStore>)`, what the wiring shares, not the
  concrete types.
- **D24's `bind.rs`:** adds `Wired<P>` and `WiredHost` (Design decision 1). `WiredHarness` is now an alias, and
  `WiredHarness::new(Result<Arc<dyn HarnessPort>, PaneError>)` is unchanged.
- **Environment beyond D5 to D7's letter:** the edges of Design decision 6. Each is a refusal or a strictness, never a
  fallback, and none changes a tested row.
- **ADR, beyond D17's list:**
  - the section 2 pointer sentence (W-C asks to record the case "beside" section 2's two);
  - "from `HOLLER_HERDR_WORKSPACES`" in section 10's extent sentence;
  - section 13's line on the hub's registration now points at #654's DRIVEN wiring (one of D17's DRIVEN lines);
  - "Deferred to named stories": the DRIVEN item now names #654 alone and the first-message item #828.
- **W-R (`docs/testing.md`):** not done. It is outside the Blast radius and optional; the scenario's module doc carries
  the exact command.

## Tier 1 self-check (incl. tests now GREEN)

| Command | Result |
|---|---|
| `cargo build -p holler-cli` | clean |
| `cargo clippy -p holler-cli --all-targets -- -D warnings` (CI's flags) | clean |
| `cargo test -p holler-cli --test pane_integration` | `test result: ok. 58 passed; 0 failed; 4 ignored; ... finished in 107.39s` |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1` | `test result: ok. 4 passed; 0 failed; ... finished in 57.05s` |
| `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | 1607 passed, 1 failed, 31 ignored, then stopped (see below) |
| `cargo test --no-fail-fast -p holler-hub -p holler-load-test -p holler-pane -p holler-pane-testkit -p holler-proto -- --skip roster_stays_accurate_under_concurrent_body_load` (the crates the stopped run never reached) | 702 passed, 0 failed, 1 ignored, exit 0 |
| `cargo test --doc` for the crates the stopped run had reached (`holler-cli`, `holler-body`, the three adapters, `herdr-holler`) | ok (none has a doc-test) |
| `cargo test -p holler-cli --test docs_cli_test`, rerun after the ADR edit | 3 passed: every `holler …` span of the new ADR text parses |
| `bash scripts/lint.sh` | exit 0, no warning for a touched file |
| `bash scripts/changelog-check.sh` | ok |
| `cargo machete` | no unused dependency |
| `rustfmt --edition 2021 --check crates/holler-cli/src/pane/wiring.rs` (and its three submodules) | clean |

**The 58 default-run tests** include T's 8 RED rows, all now passing:

- `env::herdrs_own_session_variable_is_no_fallback`;
- `env::no_herdr_session_is_usage_naming_holler_herdr_session`;
- `env::malformed_workspaces_are_usage_naming_the_variable`;
- `store_only::a_herdr_verb_with_no_session_configured_is_usage_naming_holler_herdr_session`;
- `fake_hub::watch_makes_its_first_request_when_called_and_keeps_the_batch_for_the_first_next`;
- `fake_hub::a_cursor_ahead_of_the_head_is_refused_by_watch_itself`;
- `fake_hub::watch_with_no_hub_is_unavailable_from_watch_itself`;
- `wired::a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server`.

**The opt-in run** used the scratch instances the tests start themselves. Afterwards no `/tmp/hlr649-*` directory and no
`hlr649` or `holler649` process was left. The `/tmp/hlr649-*.txt` files there are T's earlier logs; I did not create or
remove them.

**The one workspace failure is not this phase's.**
- The failing test is `wizard_scripts`' `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note`. It
  scans every git-tracked file for the setup wizard's pre-#812 directory and command names.
- It hits two lines of pipeline scratch that O and A wrote:
  - `docs/handoffs/649-brief.md:1197`, the brief's E64 source path;
  - `docs/handoffs/649/decisions.md:66`, A's first-pass evidence list.
- Phase 11 deletes `docs/handoffs/` before the push, so CI will not see them. No production file or file of this phase
  carries the old name.
- cargo stops at the first failing binary. So the crates after `holler-cli` were run separately, with `--no-fail-fast`
  (row above).

## Evidence appendix

`docs/handoffs/649/evidence.md`, which covers:
- the `ControlError` sources in `send_over`;
- `transport::call` → `control::run`;
- the state dir read per call;
- `status()`'s 5 s and `WATCH_WAIT`;
- the engine's `ensure_pane`/`ensure_session`/`attach_tui` order and that they are its only calls;
- the adapter fixing its deadline before the lookup;
- the three adapters' constructors.

## Tests that look wrong (for T)

None. For Phase 6:
- `env.rs`'s malformed-workspace rows do not include the empty value `""`, which this implementation reads as no
  workspace. Pinning that is T's call.
- No test pins the absolute-path rule for `HOLLER_HERDR_SOCKET`, `HERDR_SOCKET_PATH` or `HOLLER_OPENCODE_BIN`
  (Design decision 6). They may be worth a row each.

## Known issues

- **T's findings 1 and 2 stay O's to file**, outside the Blast radius:
  - the OpenCode adapter's TUI query without a UTF-8 locale;
  - `pane switch` right after `pane relaunch`.
  The wiring does not work around either.
- **A's follow-ups for O are unchanged:**
  - comment on #825 that `profile apply` logs as `holler pane` (the ADR paragraph and the CHANGELOG now say so);
  - file the `LinkedHost` consolidation (W-O);
  - #653 and #803.

## Files changed

- `crates/holler-cli/src/pane/wiring.rs`
- `crates/holler-cli/src/pane/wiring/bind.rs`
- `crates/holler-cli/src/pane/wiring/env.rs`
- `crates/holler-cli/src/pane/wiring/store.rs`
- `crates/holler-cli/Cargo.toml`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Pipeline scratch: `docs/handoffs/649/handoff-F.md`, `docs/handoffs/649/evidence.md`, and the F entry in
`docs/handoffs/649/decisions.md`.
