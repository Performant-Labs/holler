# Decisions — #669 hub plumbing for `pane/*` and `profile/*` (skeleton slice b)

## A (Phase 3, up-front plan review) — 2026-10-09T06:12:39-06:00
- **Decided:** PASS on docs/handoffs/669-brief.md (Revision 1) at e33dab2, with 8 warns and no blocks; see handoff-A.md.
  - The plan extends the existing seams: one arm in `dispatch_control` that delegates to a new module (the `control_hold.rs` pattern); `encode_response` and `encode_error`; state loaded in `build_shared_state` beside `Holds::load`; non-`Clone` types shared as `Arc` the way `Lockout` is.
  - It leaves `send_prompt`, `holler-proto`, `CATALOG`, the golden files and the ADRs untouched.
  - It names where new code goes for the two files above 800 lines.
  - The dependency direction is right: `holler-hub` depends on `holler-pane`, and `holler-pane` depends on neither the hub nor any runtime.
- **Assumed:**
  - The issue's signatures are shorthand that F may meet through deref coercion, so a call site of `&deps.panes` satisfies `&PaneState` (W1).
  - #661's `pane-in-other-profile` rule is meant to run in the hub's `cas_put` hook, as #661's issue says, and not on the CLI side (W2).
  - The `mod.rs` exception recorded in #670's brief reflects the same reasoning that applies to this slice (W5).
- **Hedged:**
  - W1: the hub has no single pattern for blocking store I/O. `token.rs` uses `spawn_blocking`; `holds.rs` writes on the caller's thread. So this is a warn, not a block.
  - W2, W6 and W7 change the issue text or another story's radius, so O or the operator decides. A raised them as warns rather than amending the brief.
  - W3: AC 3's test as worded cannot observe the dispatcher's arguments from `tests/`. A recommends testing at the bundle seam and adding a `compile_fail` doctest, rather than adding new surface to `serve.rs`.
- **Evidence:**
  - Code: `control_server.rs` (whole file), `serve.rs` (whole file), `state.rs`, `lib.rs`, `control_hold.rs`, `control.rs:55-140, 419-518`, `live.rs:395-470`, `holds.rs:1-120, 196-240`, `roster.rs:70-110, 240-300`, `lockout.rs:337-345`, `token.rs:715-799`, `circuit/admin.rs:1-80, 255-300`, `circuit/auth.rs:38-62`.
  - `holler-pane`: `lib.rs`, `reply.rs`, `ports.rs:1-120`, `error.rs:295-460`, `pane.rs:1-80`, `profile.rs:160-200, 328-370`. `holler-pane-testkit/src/lib.rs`. `holler-proto/src/methods.rs`.
  - Repo docs and config: `scripts/lint.sh`, `clippy.toml`, `docs/testing.md` (layout and rules), `docs/adr/README.md`, `ADR-0006.md`, `ADR-0020.md`.
  - Issues #669, #633 (epic, including the Skeleton split), #639, #649 and #661 via `gh issue view`. `docs/handoffs/670-brief.md` in the #670 worktree.
  - Searches: `find crates -path '*/src/*' -name mod.rs` returned none. `grep -rn UnixStream::pair crates` returned none. Line counts are from `wc -l` on every file the plan touches.

## T (Phase 4, author/RED) — 2026-10-09
- **Decided:**
  - One test file, `crates/holler-hub/tests/pane_dispatch_test.rs`, 10 tests, in process over `UnixStream::pair()` (real `Registry`/`Roster`/`Lockout`, no binary, no sleep). RED is a build failure (missing modules, missing manifest lines, `handle_control_conn` arity), as the brief's test plan allows.
  - Took A's names for the open API: `PaneDeps { panes, profiles }` (`Clone`, pub `Arc` fields) in `pane_dispatch.rs`, passed as the last parameter of `handle_control_conn` (A finding 4). `panes::dispatch` and `profile::dispatch` are `pub` and async so their AC 4 signatures can be called from the test.
  - AC 3's "not `Clone`" is pinned with a stable-Rust autoref probe in the test file, not the `compile_fail` doctest A suggested, because a doctest would have to live in F's production files. The pointer-equality check runs at the bundle seam (`Arc::ptr_eq` and `strong_count == 3` across two live connections), as A finding 3 advised, so `serve.rs` gains no `pub` and no test hook.
  - Left out: `CATALOG.len() == 22` (already in `holler-proto`'s own tests), and any test of the stub's treatment of `params` or of "load writes nothing", which #639 and #661 would have to delete.
- **Assumed:** `control/status` is a fair stand-in for "existing control methods behave as before" (it goes through the same `dispatch_control` and needs only registry and lockout); the existing hub and CLI suites cover the rest in GREEN.
- **Hedged:** a compile-time RED cannot show the assertions are right, so T ran the file against a throwaway stub in a scratch worktree (10/10 pass x3; clippy clean; with the arm disabled the three forwarding tests fail on assertions). The scratch worktree is removed.
- **Evidence:** `cargo test -p holler-hub --test pane_dispatch_test` in the 0669 worktree (12 errors: E0432 x5, E0061, E0277 x5 plus the summary); scratch run output as above; `crates/holler-proto/src/id.rs:46-56` (correlation ids need an `h-`/`b-` prefix); `crates/holler-pane/src/reply.rs:59-110` (`PaneReply`, `into_result`).

## F (Phase 6, implement) — 2026-10-09T06:32:35-06:00
- **Decided:**
  - Met T's contract as written: `pane_dispatch::PaneDeps { panes, profiles }` (`Clone`), `handle_control_conn(.., pane_deps)` with the bundle last, `panes::dispatch(..&PaneState, &ProfileState)`, `profile::dispatch(..&ProfileState, &PaneState)`, `check_membership(&Pane, &ProfileState)`. 10/10 of T's tests pass; the workspace suite passes (937 passed, 0 failed).
  - One bundle (A finding 4): `PaneDeps` is the single new `SharedState` field and the single new `accept_loop` parameter; `PaneDeps::load(&HubState)` builds both `Arc`s, so `build_shared_state` gains one call after the `Holds::load` lines. `serve.rs` grew by 5 lines (838), `control_server.rs` by 8 (837). The `#184` allow note now says 6 handles.
  - `PaneState` and `ProfileState` are fieldless and do not derive `Clone`; `load` returns an empty value. #639 and #661 add their fields and their corrupt-file behaviour.
  - A finding 1: `forward` passes `&deps.panes` and `&deps.profiles` as they are, so #639 and #661 can widen their own `dispatch` to `&Arc<_>` (for `spawn_blocking`) without editing `pane_dispatch.rs`.
  - Reply encoding (MO decision 3) lives in one function, `pane_dispatch::reply_line`: a `PaneReply` as the JSON-RPC result through `encode_response`. The three stubs call it; #639 and #661 call it for their real replies. The `forward` branch for a method in neither list answers `MethodNotFound` through `encode_error` (A finding 4).
  - A finding 6, first option: `profile::dispatch` routes `profile/rename` to a stub `rename::dispatch`, so `rename.rs` is not an empty file. This departs from the issue's word "empty" to keep the issue's purpose ("#665 fills it and edits no #661 file"): #661 owns `profile/mod.rs` and its scope lists six handlers without `profile/rename`, so with an empty `rename.rs` nothing would route to it.
  - A finding 5, the `mod.rs` exception: `src/panes/mod.rs` and `src/profile/mod.rs` are the first `mod.rs` files under any crate's `src/`. They are kept because each module root has to sit inside its owner's blast-radius glob (`panes/**` is #639's, `profile/**` is #661's); a sibling `panes.rs` or `profile.rs` would be outside both. Worded as #670's brief words the same exception. `pane_dispatch.rs`, `pane_wiring.rs` and `rename.rs` are plain files.
  - A finding 7: `pane_wiring.rs` is declared and empty, as the issue says, and is a deliberate permanent placeholder: the hub wires no adapters (epic ruling 0), and removing the module later needs a `lib.rs` edit.
  - A finding 2 is not applied: `check_membership` keeps the issue's two-argument shape, which T's tests fix. The hook cannot see the stored pane record; see Hedged.
  - Found a failure that is T's: `bash scripts/lint.sh` rejects `tests/pane_dispatch_test.rs:1` (multi-line `#![allow(` whose `// #669` link is on line 6; check 1 is per-line). Not edited (F writes no tests); the handoff gives two layouts that pass both lint and `rustfmt --check`, checked in a scratch file.
  - Added no tests, no log event, no CLI or protocol change, so no README or `docs/` edit; the CHANGELOG entry is the only doc change.
- **Assumed:**
  - That A's finding 6 reflects the intent of #665's text ("pre-creates the empty rename.rs ... so you edit none of #661's files"), and that O would amend the word "empty" rather than want #665 to edit `profile/mod.rs`.
  - That `serde_json::to_value(&PaneReply)` cannot fail (a bool and two optional values), so `unwrap_or_default()`, the file's existing idiom, is safe; a `null` result would in any case not parse as a `PaneReply` and never reads as success.
  - That `PaneDeps::load` being `pub` is acceptable: it mirrors the `pub` `PaneState::load` that T's tests need, and later in-process hub tests can build a real bundle with it.
  - That the CI command for the full suite is the one recorded in `.github/workflows/ci.yml` (`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`); the separately retried load test and the `--ignored` interop step were not run here.
- **Hedged:**
  - A finding 2 stays open for O: the membership hook receives one `Pane` and no stored record, so #661's `pane-in-other-profile` rule either moves into #639's `cas_put` or widens this signature (a call-site change in #639). The `Ok(())` stub also accepts every pane in the window after #639 merges and before #661 does; no release should ship that window.
  - No automated test drives `serve.rs`'s own wiring (private items; A finding 3 rules out new `pub` surface or a test hook). Covered by the build, the one call site, and a manual run of a real `holler hub serve` on an isolated state dir (all four method kinds answered as required, exit 0 on SIGTERM); the script is not committed.
  - `forward`'s neither-list branch is untested (unreachable from `tests/`).
  - The `rename.rs` stub is a judgment call outside the literal issue text; reverting it is two small deletions (the function and one match arm) if O wants a literally empty file.
- **Evidence:**
  - Read: the brief, handoff-A, handoff-T-red, decisions.md; issues #669, #639, #649, #661, #665 (`gh issue view`); `control_server.rs`, `serve.rs`, `state.rs`, `lib.rs`, `control_hold.rs` (head), `holds.rs` (head), `holler-pane` (`lib.rs`, `reply.rs`, `error.rs`, `pane.rs`), `holler-proto/src/methods.rs`, `Cargo.toml`, `clippy.toml`, `scripts/lint.sh`, `scripts/changelog-check.sh`, `.github/workflows/ci.yml`, the F handoff and evidence of the #508 run for house style, and (read-only) the #670 brief for the `mod.rs` wording.
  - Ran: `cargo test -p holler-hub --test pane_dispatch_test` (10/10, four runs; the RED was T's); `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (exit 0; 937 passed, 0 failed, 5 ignored); `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings` (clean); `cargo machete` (clean); `bash scripts/lint.sh` (exit 1, only T's line 1); `bash scripts/changelog-check.sh` (ok); `rustfmt --check --edition 2021` on each new file (clean); `git diff --check` (clean); `cargo doc -p holler-hub --no-deps` (no warning from the new files); a live-hub smoke (isolated state dir, real control socket); scratch-file checks of three `#![allow(` layouts against `rustfmt` and lint check 1.
  - Not run: CI's separately retried load-roster test (nothing here touches the roster). CI's `--ignored` interop step was run and matches 0 tests (those cases were un-ignored and already run in the default suite, together with every other holler-cli test that starts a real `holler hub serve`, so the changed `serve_forever` / `accept_loop` path is exercised end to end).

## T (Phase 7, verify GREEN + Tier 2)

- **Decided:**
  - Repaired T's own test: split the multi-line `#![allow(...)] // #669` into two one-line attributes each carrying `// #669`, which satisfies both `lint.sh` check 1 and `rustfmt --check`.
  - Returned PASS: suite GREEN, no production change required.
- **Assumed:** That CI's command in `.github/workflows/ci.yml` is still the one F quoted (skipping `roster_stays_accurate_under_concurrent_body_load`).
- **Hedged:** `serve.rs`'s wiring has no automated test; accepted per A finding 3, covered by the single call site and F's live-hub run.
- **Evidence:** Ran `lint.sh` (exit 0), `changelog-check.sh`, clippy `-D warnings`, `cargo machete`, `cargo test --workspace` (937 passed, exit 0), `docs_cli_test`, `wire_selftest`, `pane_dispatch_test` x3 (10/10). Mutation check: disabling the control_server arm fails 3 tests; restored. Evidence excerpts spot-checked against source.

## A (Phase 7, anti-duplication gate) — 2026-10-09T06:47:07-06:00
- **Decided:**
  - PASS on the diff f2602ba...851684d, with 3 warns and no blocks; see handoff-A-dup.md.
  - F extended every object the Reuse map named. One arm hands off to plain-function modules, as `control_hold.rs` does. Every reply goes through `encode_response`/`encode_error`, and `reply_line` is the one thin `PaneReply` wrapper. `PaneState::load`/`ProfileState::load` have the `Holds::load(&HubState) -> Self` shape and are shared as `Arc`s of non-`Clone` types. `PaneDeps` has the `AdminDeps` bundle shape. The method lists come from `holler_proto::methods`.
  - No production code is a near-copy of the token store, `Lockout`, `Roster`, the log helper, or the test harness on the rejection list (`Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`). No production code changed after F.
  - Warns, all in the test file or for the next stories:
    - W1: `sample_pane` copies `holler-pane`'s `tests/common::pane_json`. It should move to the testkit in #638.
    - W2: `fresh_deps` builds `PaneDeps` by hand instead of calling `PaneDeps::load`. This is T's one-line fix.
    - W3: the in-process control harness should move to `crates/holler-hub/tests/common/mod.rs` the first time #639 or #661 needs it, not be copied.
- **Assumed:**
  - #638's scope (fill `holler-pane-testkit` with fakes and the conformance suite) is the right home for the shared `Pane` fixture, and O can widen #638's brief to move `pane_json`/`pane`/`MemPaneStore` there.
  - #639 and #661 will write hub tests that drive `pane/*` and `profile/*` over the control socket, so they need the same harness.
- **Hedged:**
  - W1 is a warn and not a block. The copied fixture lives in `holler-pane`'s private `tests/common`, which `holler-hub`'s tests cannot import. Its cross-crate home (the testkit) is empty and belongs to #638, outside this slice's radius, so the test could not have reused it here.
  - The deviation of `rename.rs` from "empty" applies Phase 3 finding 6, so A does not treat it as drift. Whether it meets the issue's text is S's call.
  - Phase 3 findings 2 (the `check_membership` inputs) and 7 (the `pane_wiring.rs` placeholder) are still open for O. F did not change them.
- **Evidence:**
  - Diff: `git diff origin/main...HEAD` (19 paths, all in the blast radius or `docs/handoffs/669*`). `git show --stat` of 3a7194e, 795b343 and 851684d (T-green touched only the test's `#![allow]` lines).
  - Read in full: `pane_dispatch.rs`, `panes/mod.rs`, `profile/mod.rs`, `profile/rename.rs`, `pane_wiring.rs`, `control_server.rs`, `tests/pane_dispatch_test.rs`; the `serve.rs` hunks and `serve.rs:337-381`.
  - Compared against: `control_hold.rs:1-80`, `holds.rs:280-330`, `circuit/admin.rs:36-44`, `circuit/auth.rs:40-53`, `control.rs:418-518` (`send_over` is private and opens by path), `holler-pane/src/reply.rs`, `holler-pane/src/error.rs` (22 codes), `holler-pane/tests/common/mod.rs`, `holler-cli/tests/support/mod.rs` (`StateDir`, `Hub`), and the existing hub tests' temp-dir setup.
  - Searches: `struct *Deps`, `handle_control_conn` callers, `is_pane_method`/`PANE_METHODS` users, `-32601` in tests, `as _;` dev-dependency links, `compile_fail`/`IS_CLONE` probes, the `hj-c1r1`/`sample_pane`/`pane_json` fixtures. ADR-0003 and ADR-0006 contain no rule on control-socket error replies. Issue #669 was read with `gh issue view`. The sibling #670 diff touches only `holler-cli` and `Cargo.lock`, so it does not overlap this slice's hub code.
