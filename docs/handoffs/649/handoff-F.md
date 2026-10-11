# Handoff-F: Phase 6 (F, implement) - #649 hub wiring and the scratch integration scenario

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `0cc840e` plus this phase's
staged changes)
**Issue:** #649

This handoff covers all four F cycles.
- **Fourth cycle (this one): D33.** O merged `origin/main` at `652841b` (#832) into the branch (`63b4afa`) and amended the
  brief with D33 (`a87dac3`, `6e20075`). T-red pinned it (`0cc840e`). This cycle makes the two Herdr wrappers forward
  #832's two observe methods. It is described first.
- **Third cycle (`4d7f58a`):** the production half of the mixed rework after the diff gate's round-2 BLOCK. T took the
  test half (B-1) in `13010f4`.
- **Second cycle (`22aa539`):** the re-entry after the diff gate's round-1 BLOCK, run on A's fifth pass (`2280399`) and
  T-red's second cycle (`ce5da11`).
- **First cycle (`8b70771`):** its text follows. All of it still holds.

The Tier 1 section has one table per later cycle, each at that cycle's tree. The file list covers all four cycles.

## What was done

### Fourth cycle

- `crates/holler-cli/src/pane/wiring/bind.rs`, D33:
  - **`BindingHerdr`** overrides `observe_pane` and `observe_panes`. Each forwards to the inner port and answers what it
    answers.
  - **`LazyHerdr`** overrides both.
    - `observe_pane` is `self.port()?.observe_pane(pane)`. A failed build answers its stored error, as every other
      method does.
    - `observe_panes` forwards to the built port's own `observe_panes`. When the build failed, it answers
      `vec![PaneObservation::default(); panes.len()]`, one empty observation per pane.
    - Both go through `port()`, so the build runs once, on the first use, and a failure is kept and never retried.
  - **Docs.**
    - The module doc has a new paragraph, "Every method is forwarded (D31 W-4, D33)". It names the four default-armed
      methods the wrappers override, and says why `observe_panes` must reach the inner `observe_panes`.
    - The `BindingHerdr` doc names the two methods.
    - The `LazyHerdr` doc said "every method answers its error". It now names `observe_panes` as the exception.
  - `PaneObservation` joins the `holler_pane` import.
- `CHANGELOG.md`, S's advisory 6: one clause of this story's own entry.
  - It said the variables are "each read when its adapter is first used". In fact `connect` reads them all, and a bad
    value is answered at its port's first use.
  - It now says a missing or malformed one "fails only the verbs that use its adapter, when they first use it". No
    other line of the file changes.
- `docs/handoffs/649/evidence.md`: a new top section with two entries (Evidence appendix).

### Third cycle

Round 2 of the diff gate (`docs/handoffs/649-diff-result-r2.md`) blocked on two findings, and the driver classified
them as mixed. B-1, the D8 test in `tests/pane_integration/bind.rs`, was T's (evidence and a comment, `13010f4`).
B-2 is the production one. It reads `from_env`'s `LazyHerdr` closure in `wiring.rs` as borrowing the local
`herdr_config`, and so as code that cannot compile.

- **B-2 is refuted by the compiler.** The closure is `move || connect_herdr(&herdr_config)`. `move` takes the owned
  `Result<HerdrConfig, PaneError>` into the closure, and `&herdr_config` borrows the closure's own field. So the
  closure is `'static`, as `LazyHerdr::new` requires.
  - At HEAD, `cargo build -p holler-cli --all-targets` and clippy `-D warnings` are clean.
  - A mutation check confirms the reading. With `move` removed, `cargo check -p holler-cli` fails with `E0373`:
    "closure may outlive the current function, but it borrows `herdr_config`, which is owned by the current
    function". That is the error B-2 describes, for code without `move`. The file was restored with `git checkout`,
    and `git status` was clean.
- `crates/holler-cli/src/pane/wiring.rs`: a two-line comment above the closure. It says the closure owns
  `herdr_config` (`move`), so it is `'static` as `LazyHerdr::new` requires. **No behaviour change.** It is there
  because round 3's reviewer sees only the brief, the diff and `evidence.md`, never this handoff (Design decision 12).
- `docs/handoffs/649/evidence.md`, two changes:
  - **A new entry for B-2.** Neither `HerdrConfig` nor `PaneError` takes a lifetime or type parameter. Their
    declarations are quoted and checked against source. So the value the closure owns holds no borrow of `from_env`'s
    frame.
  - **The sections reordered, newest first.**
    - The gate attaches only the first 12,000 bytes of this file (`DUAL_REVIEW_EVIDENCE_MAX_BYTES`, default 12000).
      Round 2's prompt shows the file cut there, with 9,925 bytes dropped.
    - At 25,304 bytes, the file would have shown round 3 the same window as round 2. That window has none of T's
      four entries for B-1, and no entry for B-2.
    - Round 3 now sees, in full: T's B-1 entries, the B-2 entry, and round 1's four B-1 and B-2 entries. The cut
      falls in round 1's conformance entry, as it did in round 2.
    - Every existing entry is carried over byte for byte. A script assembled the file from line ranges of the old one
      and asserted that no line was lost. A sorted-line diff shows that only the title and the intro changed.

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

### Fourth cycle

15. **`observe_panes` forwards to the inner `observe_panes`, never through `observe_pane`.**
    - The Herdr adapter reads every pane from one `session.snapshot`. The trait's default would ask `observe_pane` once
      per pane, which is one snapshot each.
    - So the forward is what keeps ADR-0021 section 14's "one call" through the wrappers. T-red's three recording rows
      pin it.
16. **`LazyHerdr::observe_panes` with a failed build answers one empty observation per pane, after `port()`.**
    - The method returns no `Result`, so it cannot answer the stored error. The trait's doc gives an empty observation
      to a pane that cannot be observed because "the call failed". The default does the same over a failing
      `observe_pane`.
    - It asks `port()`, the one build path, and does not read the `OnceLock` directly. So the first use builds, and a
      failure is kept and not retried. T's failed-build row counts one build.
    - The stored error is dropped there, but it is not lost to the operator. `unmanaged::observe` calls `snapshot`
      before `observe_panes`, and `snapshot` answers the error. That is the `"unmanaged_unavailable": "usage"` that
      T's `store_only` row pins.
    - `self.port().map_or_else(..)` would be equivalent. I used a `match`, which reads as the doc does.
17. **No change to `wiring.rs`, the ADR or `Unwired`.**
    - `from_env` already chains the two wrappers over the adapter, so the forwards are live with no wiring change.
    - D33 adds no type, signature or behaviour that the ADR must record. Section 14 already says "A wrapper port must
      delegate it", and "Wiring as built (#649)" does not list port methods.
    - `Unwired::observe_pane` (answering `not-implemented`) is #832's, merged from `main`. It is not this story's to
      change.
18. **`wiring.rs`'s `holler_pane` import list stays as the merge left it.**
    - `rustfmt --check` on `wiring.rs` now reports one hunk: the import list #832 widened past 100 columns.
      `main`'s own `wiring.rs` has the same line (`git show origin/main:crates/holler-cli/src/pane/wiring.rs`).
    - The workspace on `main` has hundreds of rustfmt diffs, and CI runs no rustfmt step.
    - Reformatting another story's line after a merge is the churn this run's lesson warns against. So the PR's diff
      leaves that line alone.
    - `bind.rs`, the only Rust file this cycle edits, is rustfmt-clean.
19. **The new evidence section is kept small, to stay inside the diff gate's 12,000-byte window.**
    - I dropped a third entry, the adapter's one-snapshot `observe_panes` (`adapter.rs:429-434`). T's ADR section 14
      excerpt already states that fact.
    - With it, the third cycle's B-2 entry (the `move` closure) would have been cut.
    - Now these sections are whole inside the window: this one, T's D33 section, T's fifth pass, T's third cycle, and
      the B-2 entry. Round 1's section is past it. About 2 KB of it was still inside before T-red's D33 section was
      added.
20. **S's advisory 6 is taken, as one clause of this story's own CHANGELOG entry.**
    - The entry is this story's text, in the Blast radius, and the advisory names a false verb, not a false effect.
    - Fixing it after the merge would need a docs PR of its own.

### Third cycle

12. **Answer B-2 where the next reviewer will read it: a comment in the diff, not a code change.**
    - The closure is correct and idiomatic, so its code stays as it is.
    - The answer cannot live in this handoff alone. Round 3's reviewer is given the brief, the diff and
      `evidence.md`, and nothing else: the driver calls `dual-review.sh` without `--handoff`, and round 2's prompt
      reads "Feature Handoff: (no handoff file)".
    - The cost of a repeat is high. The run stops at the diff gate's third consecutive BLOCK
      (`PER_GATE_BLOCK_THRESHOLD = 3` in the driver), and this would be the third.
    - Two other ways were rejected:
      - **Restructure the closure** (for example a `herdr_build(config) -> impl Fn() .. + Send + Sync + 'static`
        helper) to remove the `&` that B-2 read as a borrow of a local. That is churn in reviewed, tested code, to work
        around a misreading.
      - **No change at all.** That would leave round 3 the same text that round 2 misread.
    - The comment is two lines. It states the rule at the line B-2 cites, and is true for any reader.
13. **The B-2 evidence entry quotes only the two type headers.**
    - "A type with no lifetime or type parameter holds no borrow that is not `'static`" settles the one thing B-2
      questions.
    - Quoting all 23 variants of `PaneError` would also show `Send + Sync`, but it would cost about 800 of the 12,000
      bytes, and B-2 does not question `Send + Sync`.
14. **Reorder the evidence, don't trim it.**
    - Rewriting entries to shrink them risks a misquote, and T's entries are T's to word. Reordering keeps every quote
      exactly as it was checked.
    - Within round 1's section, the two B-2 entries (`NoParams`, `params_of`) now come before the conformance entry.
      So both of round 1's blocking answers are whole inside the cap. Conformance is supporting evidence for B-1, and
      round 2 also saw it only in part.
    - The intro now tells T and S to put a later round's entries above the existing ones, so they stay inside the cap.

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

- **Fourth cycle:** nothing new. D24's `BindingHerdr` and `LazyHerdr` are extended in place, with two trait methods
  each. There is no new type, helper or module.
- **Third cycle:** nothing new. There is one comment inside the existing `from_env`, and one evidence entry.
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

- **Fourth cycle: no architecture change.**
  - There is no new module, type, public item, dependency or signature.
  - The two wrappers now implement the two `HerdrPort` methods that #832 gave defaults. The trait is #832's and is
    unchanged.
  - The production chain and its call order are unchanged: `BindingHerdr` over `LazyHerdr` over `HerdrAdapter`.
  - This cycle reports `archChanged: false`.
- **Third cycle: no architecture change.** The cycle adds one comment and no code: no type, signature, module or
  dependency changes. This cycle reports `archChanged: false`.
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

- **Fourth cycle:** none from D33.
  - Beyond it is S's advisory 6, one clause of the CHANGELOG entry (Design decision 20).
  - `wiring.rs`'s import list is left as `main` has it (Design decision 18).
- **Third cycle:** none from the brief.
  - The evidence reordering moves T's round-2 section, unchanged, above F's sections. It is recorded in
    `decisions.md`.
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

**Fourth cycle**, at `0cc840e` (the merged tree, with #832) plus this cycle's staged changes:

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test pane_integration -- observe:: bind::`, before any change | `FAILED. 13 passed; 7 failed`. These are T-red's seven D33 rows, each failing on its assertion. RED reproduced |
| The same, plus `store_only`, after the change | `ok. 24 passed; 0 failed` |
| `cargo test -p holler-cli --test pane_integration` (the default run) | `ok. 66 passed; 0 failed; 4 ignored; ... finished in 119.41s`. At T-red this was 59 passed and 7 failed |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1` | `ok. 4 passed; 0 failed; ... finished in 81.56s`. This is the scenario's first run on a tree with #832, so `pane list`, `pane get` and `pane doctor` read #832's view through the real wiring. No scratch directory or `hlr649`/`holler649` process was left by this run |
| `cargo clippy --workspace --all-targets -- -D warnings` (CI's flags) | exit 0 |
| `rustfmt --edition 2021 --check crates/holler-cli/src/pane/wiring/bind.rs` | clean. `wiring.rs` reports #832's import line only (Design decision 18) |
| `bash scripts/lint.sh` | exit 0. Size warnings only, none for a wiring file. `bind.rs` is 439 lines |
| `bash scripts/changelog-check.sh` | ok |
| `cargo machete` | no unused dependency |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's `test.unit.command`) | exit 0. 151 test binaries: **2345 passed, 0 failed**, 32 ignored.<br>- The rename scan (`skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note`) now passes. It was the one failure in every earlier cycle; O's amendment corrected its two hits.<br>- `pane_integration`: 66/0/4, in 119.44 s.<br>- `pane_cli_process`: 36 passed (D22's stub tests over a `LiveHub`).<br>- `pane_verbs`: 329 passed, #832's tests among them |
| After the last edit (the `LazyHerdr` doc's wording, made during the workspace run) | rustfmt on `bind.rs` clean; clippy (workspace, `-D warnings`) exit 0; `-- observe:: bind::` 20 passed, 0 failed.<br>`cargo doc -p holler-cli --no-deps --document-private-items`, with the intra-doc link lints as warnings: no diagnostic in a wiring file. The three it prints are in `cli.rs` and `token_cmd.rs` |
| `evidence.md` quotes (a script) | The new section's 13 quoted lines equal the cited line ranges, line for line |

**Third cycle**, at `13010f4` plus this cycle's staged changes (one comment in `wiring.rs`):

| Command | Result |
|---|---|
| `cargo build -p holler-cli --all-targets`, at HEAD and again after the comment | clean |
| Mutation: `move` removed from the closure, then `cargo check -p holler-cli` | fails as expected with `E0373` at the closure (`wiring.rs:140` at HEAD): "closure may outlive the current function, but it borrows `herdr_config`, which is owned by the current function", and "function requires argument type to outlive `'static`". Restored with `git checkout`; `git status` clean |
| `cargo clippy --workspace --all-targets -- -D warnings` (CI's flags) | clean, exit 0 |
| `rustfmt --edition 2021 --check crates/holler-cli/src/pane/wiring.rs` | clean |
| `bash scripts/lint.sh` | exit 0. The only warning is `holler-pane/src/error.rs is 713 lines`, a file this story does not touch. `wiring.rs` is 392 lines |
| `bash scripts/changelog-check.sh` | ok |
| `cargo machete` | no unused dependency |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's `test.unit.command`) | 151 test binaries: 2312 passed, **1 failed**, 32 ignored, the same figures as both earlier cycles. `pane_integration`: `ok. 61 passed; 0 failed; 4 ignored; ... finished in 119.10s`, including T's D8 test and the fake-hub timing pins. The one failure is the rename scan below. Its hits are `docs/handoffs/649-brief.md`, `decisions.md` and `handoff-T-green.md`; `git grep` finds none outside `docs/handoffs/` |
| `evidence.md` quotes (a script) | 23 entries and 344 quoted lines, each found verbatim in a file its entry cites, library sources included. The two new quotes were checked at their line numbers |
| Opt-in scenario | not re-run. This cycle changes one comment, and no code; the last run was 4/4 at `22aa539` |

**Second cycle**, at `ce5da11` plus that cycle's staged changes:

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

**Fourth cycle**, under "The #832 source D33's forwards rely on (F, D33 cycle)", at the top of the file. T-red's D33
section, just below it, quotes the trait's two defaults, ADR-0021 section 14 and case 13.
- **`PaneObservation` is `Clone` and `Default`** (`holler-pane/src/ports.rs:227-229`). So
  `vec![PaneObservation::default(); n]` builds outside its crate, and `#[non_exhaustive]` forbids only a struct literal.
- **`unmanaged::observe` takes the snapshot first, then calls `ports.herdr.observe_panes` once**
  (`holler-pane/src/unmanaged.rs:89-92`, `:104-109`). Without the forwards, that call never reaches the adapter. And a
  failed `LazyHerdr` build fails at the `snapshot` first.

**Third cycle**, under "The source that round 2 of the diff gate asked for (F, third cycle)":
- **B-2:** the declarations of `HerdrConfig` (`holler-adapter-herdr/src/adapter.rs:73-74`) and `PaneError`
  (`holler-pane/src/error.rs:402-403`). Neither has a lifetime or type parameter. Both quotes were checked line by line
  against the files.
- **The order is now newest first**, so the gate's 12,000-byte window holds:
  1. T's four B-1 entries (bytes 378-3,757);
  2. the B-2 entry (3,757-4,884);
  3. round 1's B-1 entries (`into_result`, serde's `Option`) and its B-2 entries (`NoParams`, `params_of`), which run
     to byte 10,781.
- The cut falls in round 1's conformance entry. I rendered the file through the gate's own `awk` cap to check.
- Past the cut are the rest of round 1's section (NV-4, NV-5, W-T) and the first cycle's ten entries. Six of those
  ten repeat what the brief's own Evidence quotes (E23, E27, E33, E34, E57, E70).

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

**Fourth cycle:** none. All seven D33 rows pass as T wrote them, and F touched no test.

**Third cycle:** none. Round 2's B-1 was test-only, and T answered it in `13010f4`. The D8 test passes, and F did not
touch it. This cycle only moved T's evidence section, unchanged, to the top of `evidence.md`, so that the gate shows it.

**Second cycle:** none. T's call, not a defect:
- No row pins a non-UTF-8 value for a variable other than `HOLLER_TMUX_SOCKET`. The empty-value rows of `env.rs` cover
  each reader, and the process lookup is one closure that every variable goes through. So a second process-level row
  (such as `HOLLER_HERDR_SOCKET`, against the `HERDR_SOCKET_PATH` fallback) would pin the same line twice.

## Known issues

- **Fourth cycle: `wiring.rs` is not rustfmt-clean, on `main` as on the branch** (Design decision 18).
  - The one hunk is #832's `holler_pane` import line. CI does not check it.
  - Formatting `main` is not this story's job.
- **Fourth cycle: round 1's evidence section is now past the diff gate's 12,000-byte window** (Design decision 19).
  - Round 3 passed, and no line those entries back has changed since. But a fresh reviewer that asks round 1's
    questions again will not see their answers.
  - The cap is the Aftersight gap the third cycle already reported.
- **Fourth cycle, for T-green: two `/tmp/hlr649-*` directories date from 15:52 MDT, before this cycle**
  (`hlr649-f8qsRz`, `hlr649-gsPBcx`).
  - Each holds only `wired.rs`'s `opencode` trap stub. It is a `TempDir`, so it is removed on drop, unless the run is
    killed before the drop.
  - This cycle's three runs (default, opt-in, workspace) left none. Older `hlr649-*.txt` files in `/tmp` are earlier
    agents' notes.
- **Third cycle, for O: two gaps in the diff gate's inputs.** Both are the pipeline's, not this story's, and are to be
  filed in Aftersight.
  - **The evidence cap is below this run's evidence file.**
    - `dual-review.sh` keeps only the first `DUAL_REVIEW_EVIDENCE_MAX_BYTES` (12,000) bytes of `evidence.md`. The role
      docs tell F and T to append, so each round's newest answers land past the cap.
    - Round 2's reviewer saw no `NoParams` entry, which was round 1's B-2 answer. Without this cycle's reordering,
      round 3 would not have seen T's B-1 entries either.
    - Raising the variable in `.env` for this run is the operator's call. I did not change configuration.
  - **The diff gate never sees F's handoff.** The driver passes no `--handoff`, so every round's prompt has "Feature
    Handoff: (no handoff file)". An answer written only in the handoff does not reach the reviewer.
- **Third cycle: round 3 may still find something new.** 12,000 bytes cannot hold every unchanged-source fact the
  change relies on.
  - The first cycle's ten entries are now past the cap.
  - Six of them repeat facts the brief's own Evidence quotes (E23, E27, E33, E34, E57, E70).
  - The other four back statements that the brief makes itself, in D3, D8 and D31: `transport::call`, the state dir,
    `send_over`'s error sources, and the adapter's deadline.
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

Production files, all four cycles (marked by cycle where a later cycle touched them):
- `crates/holler-cli/src/pane/wiring.rs` (second cycle; third cycle, a comment only)
- `crates/holler-cli/src/pane/wiring/bind.rs` (fourth cycle: D33)
- `crates/holler-cli/src/pane/wiring/env.rs` (second cycle)
- `crates/holler-cli/src/pane/wiring/store.rs`
- `crates/holler-cli/Cargo.toml`
- `docs/adr/ADR-0021.md` (second cycle)
- `CHANGELOG.md` (fourth cycle: one clause, S's advisory 6)

The fourth cycle changes `bind.rs` and `CHANGELOG.md` in production. The third cycle changed only `wiring.rs`.

Pipeline scratch:
- `docs/handoffs/649/handoff-F.md`;
- `docs/handoffs/649/evidence.md`. The third cycle added one entry and reordered the sections. The fourth adds one
  section at the top;
- the F entries in `docs/handoffs/649/decisions.md`.
