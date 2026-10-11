# Handoff-F: Phase 6 (F, implement) - #649 hub wiring and the scratch integration scenario

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `ce5da11` plus this phase's
staged changes)
**Issue:** #649

This handoff covers both F cycles.
- **Second cycle (this one):** the re-entry after the diff gate's round-1 BLOCK, run on A's fifth pass (`2280399`) and
  T-red's second cycle (`ce5da11`). It is described first.
- **First cycle (`8b70771`):** its text follows. All of it still holds. Only the Tier 1 table and the file list are
  updated, so they describe the tree as it is now.

## What was done

### Second cycle

A's binding notes were W-T (code) and W-S (evidence), and no change for B-1 or B-2. All three are done.

- `crates/holler-cli/src/pane/wiring.rs`: `connect` now reads each variable with `std::env::var_os`. A value that is not
  UTF-8 becomes the empty string, never `None` (W-T, A's form).
  - The empty value is malformed for every path variable and for the serve arguments. So a non-UTF-8
    `HOLLER_TMUX_SOCKET` is `usage` naming it from the host and the harness, and tmux's default server is never reached.
  - The old `std::env::var(name).ok()` read such a value as unset, which fell back to the default server.
  - The `connect` doc and the module doc state the rule.
- `crates/holler-cli/src/pane/wiring/env.rs`: two messages and two docs change. No behaviour changes for any value
  a test lookup can give.
  - `absolute` answers the empty value with `<VAR> is empty or not UTF-8; it must be an absolute path`. Before, the
    message ended `not ""`, which is false for an operator whose value is not UTF-8.
  - The no-session message now says `HOLLER_HERDR_SESSION is unset, empty or not UTF-8`. It still names only that
    variable, as T's `names_only_hollers_session_variable` requires.
  - The `Lookup` doc and the module doc say how production reads, and what the empty value means for each variable.
- `crates/holler-cli/src/pane/wiring.rs`, module doc: the first cycle's link `[`env`]` becomes `[`mod@env`]`.
  Rustdoc found it ambiguous with the `env!` macro, and it was the only rustdoc diagnostic in the wiring files.
- `docs/adr/ADR-0021.md`: one sentence in "Wiring as built (#649)" records the rule (D17).
- `docs/handoffs/649/evidence.md`: eight entries for round 2's reviewer (W-S). See the Evidence appendix below.
- B-1 and B-2: no production change, as A's W-S rules. Both are refuted by source, now in `evidence.md`, and by
  conformance Case 1, which passes against a real hub.

### First cycle

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

### Second cycle

9. **A non-UTF-8 value reads as the empty value (A's form), and the empty value keeps each variable's own rule.**
   - D24 fixes `Lookup` as `&dyn Fn(&str) -> Option<String>`, and T's tests build lookups of that type. So the lookup
     cannot say "set, but not UTF-8". The empty string is the one value that every path variable, the session and the
     serve arguments already refuse as `usage` naming the variable.
   - A lossy conversion was rejected. `/tmp/x\u{FFFD}/tmux.sock` is absolute and would be taken as a socket path, as
     T-red noted.
   - Two other ways were rejected as not worth their cost for this edge:
     - an in-band marker (a NUL, which no real value can hold) that every reader would check;
     - a new `Lookup` type, which would change D24's signature and break T's tests.
   - What is left uncovered: `HOLLER_HERDR_WORKSPACES`, whose empty value is no workspace (pinned by T-green). A
     non-UTF-8 list therefore reads like an unset one. It fails closed: the Herdr adapter refuses every `ensure_pane`
     into an unlisted workspace as `unavailable`, before any request of its own (evidence), so no pane is made.
   - The rule is documented on `connect`, on `Lookup`, in both module docs and in the ADR.
10. **The lookup stays a closure in `connect`.** A named `env::process` function was the alternative. It would add a
    public item to `holler_cli::pane::wiring::env` for one caller, and A sized the fix as one line in `connect`. No public
    API changes.
11. **The two empty-value messages say "empty or not UTF-8".** These are `absolute`'s message, which quoted the value
    as `""`, and the session's, which said none was configured. This goes beyond A's one line, but it keeps the
    operator-facing text true. `serve_args`' message quotes no value, so it needs no change.

### First cycle

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
     `json!` also hides an `unwrap` for non-literal values. The two list methods send the literal `{}`, the hub's
     documented `NoParams` (evidence).
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

- **Second cycle:** nothing new. The fix is inside the existing `connect`, and the messages are inside the existing
  `env.rs` functions.
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

- **Second cycle: no architecture change.** No new module, type, public item or dependency, and no signature change.
  `connect`'s lookup is a local closure, and the two message texts are private. This agrees with A's reading of W-T.
  This cycle reports `archChanged: false`.
- **Layers:** `holler-cli` only. The CLI now depends on the three adapter crates. They were already in the tree, and
  ADR-0021 section 5 allows it. `holler-hub` gains nothing, and `pane_wiring.rs` is untouched (D16).
- **Contracts:** no change to the ports, the protocol, golden files or `holler-pane`.
- **New public API in `holler-cli`:** `pane::wiring::{store, env, bind}`, with D24's names plus `bind::{Wired,
  WiredHost}` and `env::Lookup`.
- **Local patterns followed:**
  - `const ACTOR` and `Actor::parse(ACTOR)?` (as `profile/create.rs` does);
  - `unwrap_or_else(PoisonError::into_inner)` for a lock;
  - the module docs carry the decision tables, as `profile_scope.rs` does.
- **File sizes:** `wiring.rs` 390, `bind.rs` 410, `store.rs` 397 and `env.rs` 250 lines, all under the 600 warn line.

## Deviations from spec / wireframe

- **Second cycle:** none from the brief.
  - The two message texts go beyond A's one line (Design decision 11).
  - The ADR sentence is D17's record of the rule.
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

At this cycle's tree (`ce5da11` plus the staged changes):

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test pane_integration wired::` | `ok. 2 passed; 0 failed`. T-red's RED row `a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server` now passes, beside its D6 sibling |
| `cargo test -p holler-cli --test pane_integration` (default run, final code) | `ok. 61 passed; 0 failed; 4 ignored; ... finished in 123.59s`, T's 6 s pin included |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1` (final code) | `ok. 4 passed; 0 failed; ... finished in 82.48s`: the scenario, the profile scenario, the grid, and the server argv under a real tmux pty. No `/tmp/hlr649-*` directory and no `hlr649` or `holler649` process was left |
| After the last edits, which touched only doc comments and the ADR sentence: `cargo test -p holler-cli --test pane_integration -- wired:: env:: no_hub::` | `ok. 17 passed; 0 failed`. Clippy, rustdoc, `docs_cli_test`, `lint.sh` and `rustfmt` were re-run too (rows below, at the staged tree) |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's `test.unit.command`; `--no-fail-fast` so every crate is reached) | 151 test binaries: 2312 passed, **1 failed**, 32 ignored. The failure is the rename scan below. This run includes `pane_cli_process` (the D22 stub tests over a `LiveHub`, 36 passed) and `pane_verbs` (313 passed) |
| `cargo clippy --workspace --all-targets -- -D warnings` (CI's flags) | clean, exit 0 |
| `cargo test -p holler-cli --test docs_cli_test`, after the ADR sentence | `ok. 3 passed` |
| `cargo test -p holler-cli --test wire_selftest` | `ok. 3 passed` |
| `bash scripts/lint.sh` | exit 0. The size warnings are for files this story does not touch; none names a wiring or `pane_integration` file |
| `bash scripts/changelog-check.sh` | ok |
| `cargo machete` | no unused dependency |
| `rustfmt --edition 2021 --check crates/holler-cli/src/pane/wiring.rs` (it checks the three submodules too) | clean |
| `cargo doc -p holler-cli --no-deps --document-private-items`, with the intra-doc link lints as warnings | no diagnostic in a wiring file. Other modules of the crate have existing ones; CI runs no rustdoc |

**The workspace failure that is not this phase's.**
- `wizard_scripts`' `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note` scans every git-tracked
  file for the setup wizard's pre-#812 names.
- Its only hits are pipeline scratch under `docs/handoffs/`, which Phase 11 deletes before the push, so CI will not see
  them. F and T-green both reported it in the first cycle.

## Evidence appendix

`docs/handoffs/649/evidence.md`.

**Second cycle**, under "The source that round 1 of the diff gate asked for". These are the facts W-S lists, for round
2's reviewer:
- **B-1:** `PaneReply.data` and `into_result`; serde's `Option` and `serde_json`'s `deserialize_option` at the locked
  versions; conformance Case 1 in both suites.
- **B-2:** the hub's `NoParams`, `params_of`, `decode_params`' doc, and the nine params structs of
  `holler_pane::reply`.
- **NV-4:** `HarnessPort`'s eleven methods and its two defaults.
- **NV-5:** `pane/cas_put`'s membership check and `check_membership`.
- **W-T:** the Herdr adapter refuses an unlisted workspace before any request of its own. This backs the
  non-UTF-8 `HOLLER_HERDR_WORKSPACES` sentence of the docs.

Every quoted line was checked mechanically against its file, and each line range against a numbered read. The excerpt cap of
round 1 (24 KB) is used up by the brief's own citations, so each entry quotes the lines it relies on rather than counting
on the gate to attach them.

**First cycle:**
- the `ControlError` sources in `send_over`;
- `transport::call` → `control::run`;
- the state dir read per call;
- `status()`'s 5 s and `WATCH_WAIT`;
- the engine's `ensure_pane`/`ensure_session`/`attach_tui` order and that they are its only calls;
- the adapter fixing its deadline before the lookup;
- the three adapters' constructors.

## Tests that look wrong (for T)

None. T's call, not a defect:
- No row pins a non-UTF-8 value for a variable other than `HOLLER_TMUX_SOCKET`. The empty-value rows of `env.rs` cover
  each reader, and the process lookup is one closure that every variable goes through. So a second process-level row
  (such as `HOLLER_HERDR_SOCKET`, against the `HERDR_SOCKET_PATH` fallback) would pin the same line twice.

## Known issues

- **A non-UTF-8 `HOLLER_HERDR_WORKSPACES` reads as no workspace, not as `usage`.** This is Design decision 9. It fails
  closed: every launch is refused `unavailable` ("workspace ... is not configured"), and no Herdr pane is made.
- **A non-UTF-8 `XDG_CONFIG_HOME` reads as empty.** The XDG rule takes empty as unset, so the socket default uses
  `$HOME/.config`, as it did before this cycle.
  - The wiring cannot tell such a value from an empty one through D24's `Lookup`.
  - It matters only when neither `HOLLER_HERDR_SOCKET` nor `HERDR_SOCKET_PATH` is set.
  - `PATH` and `HOME` read as before too.
- **T's findings 1 and 2 stay O's to file**, outside the Blast radius:
  - the OpenCode adapter's TUI query without a UTF-8 locale;
  - `pane switch` right after `pane relaunch`.
  The wiring does not work around either.
- **A's follow-ups for O are unchanged:**
  - comment on #825 that `profile apply` logs as `holler pane` (the ADR paragraph and the CHANGELOG now say so);
  - file the `LinkedHost` consolidation (W-O);
  - #653 and #803;
  - W-U, to be filed in Aftersight.

## Files changed

Production files, both cycles (this cycle's marked):
- `crates/holler-cli/src/pane/wiring.rs` (this cycle too)
- `crates/holler-cli/src/pane/wiring/bind.rs`
- `crates/holler-cli/src/pane/wiring/env.rs` (this cycle too)
- `crates/holler-cli/src/pane/wiring/store.rs`
- `crates/holler-cli/Cargo.toml`
- `docs/adr/ADR-0021.md` (this cycle too)
- `CHANGELOG.md`

Pipeline scratch: `docs/handoffs/649/handoff-F.md`, `docs/handoffs/649/evidence.md`, and the F entries in
`docs/handoffs/649/decisions.md`.
