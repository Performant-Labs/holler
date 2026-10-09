# Handoff-A: Phase 3 - #669 hub plumbing for `pane/*` and `profile/*`  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-669-implementation (at e33dab2)
**Brief reviewed:** docs/handoffs/669-brief.md (Revision 1)   **Reuse map:** docs/handoffs/669-brief.md §Files "Reuse map" (there is no separate survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS. The plan uses the right seams:

- One forwarding arm in `dispatch_control` that hands off to a new module, which is the `control_hold.rs` pattern.
- `encode_response` and `encode_error` for the replies.
- State loaded once in `build_shared_state` beside `Holds::load`, and shared as a non-`Clone` type behind `Arc`, the way `Lockout` is.
- `holler-hub` depending on `holler-pane`, which is the right direction: the contract crate has no runtime and no I/O, and it does not depend back on the hub.

The plan does not touch `send_prompt`, `holler-proto`, the closed 22-row `CATALOG`, any golden file or any ADR. No ADR covers the control socket's method set or its error replies. Replying with a `PaneReply` as a result already has a precedent (`control/wait` returns `matched:false` as a result) and follows the reply contract merged in `holler-pane/src/reply.rs`. The two files above 800 lines gain only the arm, one field and one parameter. The new code goes in named files. Both files stay well under 900 lines.

All eight findings are `warn`:

- Findings 1, 2 and 6 are about contracts this slice freezes for #639, #661 and #665: the handle type `spawn_blocking` needs, the inputs of the membership hook, and the rename seam.
- Finding 3: AC 3's test cannot observe what it claims without new public surface.
- Finding 4: the bundle type is public but has no name or home in the plan.
- Finding 5: these would be the first `mod.rs` files in the workspace.
- Finding 7: `pane_wiring.rs` would stay an empty module forever.
- Finding 8: the Reuse map cites a test precedent that does not exist.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | AC 4: `panes::dispatch(.., &PaneState, &ProfileState)` and `profile::dispatch(.., &ProfileState, &PaneState)` | concurrency | The ports are blocking by contract (`holler-pane/src/ports.rs:8-10`: "call one from `spawn_blocking`"). #639 implements `PaneStore` on `PaneState` with `atomic_file` writes. A function that receives only `&PaneState` cannot move an owned handle into `spawn_blocking`, which needs `'static`. That leaves #639's `cas_put` only the `holds.rs` precedent (a blocking write on the worker thread), not the `token.rs:721-799` one. `watch` is fine either way, because `Watch<T>` is an owned `'static` iterator. Where a function may need to clone, the hub already passes `&Arc<T>`: `AuthDeps` (`circuit/auth.rs:47-53`) and `dispatch_authenticate` (`serve.rs:799-808`). | In `pane_dispatch.rs`, pass the bundle's `Arc` fields as they are: `panes::dispatch(method, cid, obj, &deps.panes, &deps.profiles)`. Never write `&*deps.panes` or `.as_ref()`. That call compiles against the stubs' `&PaneState` today through deref coercion, and #639 and #661 can later widen their own `dispatch` to `&Arc<_>` inside their own glob without editing the frozen `pane_dispatch.rs`. Alternatively, take `&Arc<_>` now: the issue writes `&PaneState`, so that is O's call. Record the choice in decisions.md for #639 and #661. |
| 2 | warn | AC 5: `check_membership(&holler_pane::Pane, &ProfileState)` returns `Ok(())` | contract shape / file ownership | #661 has to fill this hook with `pane-in-other-profile`. Its issue says: setting `Pane.profile` on a pane whose `profile` is already another profile is refused, with no edit to #639's files. That rule compares the stored record's `profile` with the proposed one. The hook gets one `Pane` and no pane store. `ProfileState` cannot hold a pane-to-profile index without a second copy of membership, and `Pane.profile` is the only membership (`holler-pane/src/profile.rs:177`). As frozen, #661 can only check that the named profile exists, so the other-profile rule would either move into #639's `cas_put` or need a cross-story edit. The `Ok(())` stub is also fail-open for `profile: Some(_)` in the window after #639 merges and before #661 does. | While the hook still has no callers, widen it for free: `check_membership(current: Option<&Pane>, proposed: &Pane, profiles: &ProfileState)`. #639 passes the stored record, which it already holds inside the compare-and-swap. Otherwise, record in decisions.md and in #639's brief that #639 does the old-versus-new comparison and the hook only checks that the profile exists. Either way, make sure no release ships #639 without #661 while the stub accepts every profile. Both options change the issue's text, so O decides. |
| 3 | warn | AC 3: "a test shows two connections share the same handle (pointer equality of the `Arc`s the dispatcher receives)" | layering (test-only surface) | `SharedState`, `build_shared_state` and `accept_loop` are private to `serve.rs`. Under AC 4 the dispatcher receives `&PaneState`, not an `Arc`. An integration test cannot see what the dispatcher receives. The obvious ways to meet AC 3 as worded would widen `serve.rs`'s private API or add a production test hook, and neither belongs in this slice. | Test at the seam the test owns. Build one bundle and hand clones to two `handle_control_conn` tasks on two `UnixStream::pair()`s. While both connections are open, assert `Arc::ptr_eq` and `Arc::strong_count == 3`. Pin "does not derive `Clone`" with a `compile_fail` doctest on each type (the precedent is `holler-pane/src/error.rs:385`). Review and S check that `build_shared_state` builds the handles once. Add no `pub` to `serve.rs` and no `HOLLER_TEST_HOOKS` branch. |
| 4 | warn | "one bundled shared-state parameter" and "`SharedState` gains the two `Arc` handles (or one bundle struct)" | naming / pattern consistency | The brief does not name or place the bundle. It is new public API anyway, because `handle_control_conn` is `pub` and the integration test has to build the bundle. The hub's precedent for an owned bundle handed to spawned tasks is `AdminDeps` (`circuit/admin.rs:36-44`): `#[derive(Clone)]` with `Arc` fields. The forwarding helper also re-matches the method lists, and its "in neither list" branch has to answer something. | Define one `#[derive(Clone)] pub struct` with `pub` `Arc` fields, for example `PaneDeps { panes, profiles }`, once in `pane_dispatch.rs`. Store that one field in `SharedState`, not the two `Arc`s and a bundle, so `serve.rs` gains one destructured name and one `accept_loop` parameter. Keep the `#184` note on `accept_loop`'s allow true; it says "5 shared hub-wide handles" (`serve.rs:543`). In the helper, the unreachable branch answers `encode_error(cid, Code::MethodNotFound, ..)`, as `dispatch_session_control` does (`control_server.rs:163`). Do not use `unreachable!`, which is denied. Do not copy `dispatch_allowlisted`'s `Option` plus `.unwrap_or_default()` shape either, because it would write an empty line to the socket. |
| 5 | warn | `src/panes/mod.rs`, `src/profile/mod.rs` | file structure | No crate has a `mod.rs` under `src/` today. Every directory module in the workspace uses `foo.rs` plus `foo/`; in the hub these are `circuit`, `holds`, `lockout`, `roster` and `token`. There is a real reason for the exception: the module root then sits inside the owners' `panes/**` and `profile/**` globs (#639 and #661). The sibling slice #670 records the same exception (670-brief.md, "`src/**/mod.rs` is new to this codebase ... recorded in decisions.md"). This brief does not record it. | Record the exception and its reason in `docs/handoffs/669/decisions.md`, worded as #670 words it. Limit it to these two module roots: `pane_dispatch.rs`, `pane_wiring.rs` and `profile/rename.rs` stay plain files. |
| 6 | warn | "`src/profile/rename.rs` is an empty module declared in `profile/mod.rs` (#665 fills it without editing a #661 file)" | contract shape | An empty file gives `profile::dispatch` nothing to call. For #665 to edit no #661 file, `profile::dispatch` (in `profile/mod.rs`, which #661 owns after this slice) must already route `profile/rename` to a function in `rename.rs`. As written, no story owns adding that route. | In this slice, have the `profile::dispatch` stub route `"profile/rename"` to a stub `rename::dispatch(cid, obj, &ProfileState, &PaneState)` that answers `not-implemented`. It is not dead code because it is called, and it is plumbing, not logic. Otherwise, record that #661 adds the route and the stub inside its `profile/**` glob. |
| 7 | warn | "`src/pane_wiring.rs` (empty, declared; no job ..., left for #649 to drop or fill)" | abstraction level | #649 cannot drop it. Its radius lists `pane_wiring.rs` but not `lib.rs`, and the epic keeps `holler-hub/src/lib.rs` as #669's file, so removing `pub mod pane_wiring;` would be an edit outside #649's radius. Under ruling 1 (adapters are wired on the CLI side) the module also has nothing to hold. As planned, it becomes an empty module that stays forever. | Leave it out of this slice and strike it from #649's radius; O amends both issues. Otherwise, record in decisions.md that it is a deliberate permanent placeholder. |
| 8 | warn | Reuse map: "the existing `handle_control_conn` test style (a raw `UnixStream` pair)" | pattern consistency | That precedent does not exist. No test calls `handle_control_conn` (the brief's own Risks says so), and `UnixStream::pair` appears nowhere in the repo. `pane_dispatch_test.rs` therefore starts a new in-process harness for the control socket. The nearest real precedent is `control_server.rs`'s `wait_tests` module, which runs in process against a real `Roster`. | Keep the harness inside the test file and keep it small: one helper that sends a line and reads a line, and `tempfile::tempdir()` with `HubState::from_root` for state. Do not set `HOLLER_STATE_DIR`, spawn processes, or use blind sleeps (`docs/testing.md`). That keeps it from turning into a second `support::Hub` or `StateDir`, both on the Phase 7 rejection list. Note that `Lockout::new()` already returns `Arc<Lockout>` (`lockout.rs:341`), so do not wrap it again. |

Apart from these findings, the plan matches the existing patterns. Checked:

- `handle_control_conn` has one caller (`serve.rs:565`). The admin loop calls `dispatch_allowlisted`, not `dispatch_control`, so it is unaffected, and `pane/*` stays unreachable over `admin/*` and over a body socket.
- `control::run` can already send `pane/*` and `profile/*` (`ControlCall.method` is a `pub &'static str`), so the brief is right that `control.rs` needs no edit for #649.
- The only arguments `log_control` logs are the method and the id, never the params.
- Projected sizes: about 840 lines for `control_server.rs` and about 845 for `serve.rs`.

## Notes for O

PASS, so nothing is required before T. Who handles what:

- **T and F, during RED and GREEN:** findings 1, 3, 4 and 8.
- **O, in decisions.md:** finding 5.
- **O or the operator:** findings 2, 6 and 7. Each one changes the issue text or another story's radius, so O decides whether to amend #669, #639, #649 and #661, or to record the trade-off and move on.

Out of scope, but noticed: `docs/adr/README.md` reserves the ADR 0021 slot for issue #22 (the rule there is "ADR NNNN = issue #(NNNN+1)"), while the epic assigns ADR-0021 to #634. That is #634's problem, not this slice's.

## Patterns referenced

- `crates/holler-hub/src/control_server.rs:24-117, 119-194, 697-713` (dispatch, sub-dispatch fallback, reply encoders) and `control_hold.rs` (the plain-function module the dispatcher calls)
- `crates/holler-hub/src/serve.rs:338-377, 542-585` (`SharedState`, `build_shared_state`, `accept_loop`); `circuit/admin.rs:36-44` (`AdminDeps`); `circuit/auth.rs:40-53` (`AuthDeps`, `&Arc` fields)
- `crates/holler-hub/src/token.rs:721-799` (`spawn_blocking` wrappers) against `holds.rs:1-60` (write on the caller's thread)
- `crates/holler-pane/src/{ports.rs:1-80, reply.rs, profile.rs:173-200, error.rs:310-400}`; `crates/holler-proto/src/methods.rs:18-25, 113-143`
- Issues #639, #649 and #661 (radii and hook expectations); `docs/handoffs/670-brief.md` in the #670 worktree, line 293 (the recorded `mod.rs` exception)
