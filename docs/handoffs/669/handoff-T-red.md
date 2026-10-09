# Handoff-T-red: #669 hub plumbing for `pane/*` and `profile/*`

**Date:** 2026-10-09
**Branch:** issue-669-implementation
**Brief / wireframe reviewed:** docs/handoffs/669-brief.md (Revision 1); no wireframe (no UI surface). Handoff-A read: docs/handoffs/669/handoff-A.md.

## A precondition
Confirmed: A returned PASS on the plan (Phase 3), with 8 warns and no blocks.

## Tests authored
One file, `crates/holler-hub/tests/pane_dispatch_test.rs` (staged by path; hub tests are auto-discovered, no `[[test]]` entry needed in `holler-hub`). All run in process over `UnixStream::pair()` with a real `Registry`, `Roster` and `Lockout`; no subprocess, no sleeps, bounded read on the reply line.

| Test | Criterion | Tier |
|---|---|---|
| `every_pane_method_is_forwarded_to_the_stub_not_method_not_found` | AC1: every `PANE_METHODS` entry (incl. long-poll `pane/watch`) returns a JSON-RPC result that parses with `PaneReply::into_result` to `Err(NotImplemented)`; id echoed; one connection serves the whole table | hub integration |
| `every_profile_method_is_forwarded_to_the_stub_not_method_not_found` | AC1: same for every `PROFILE_METHODS` entry (incl. `profile/rename`) | hub integration |
| `unknown_methods_still_answer_method_not_found` | AC2: `control/x`, `foo/bar` stay `-32601`; `pane/frobnicate`, `profile/frobnicate` too (the arm forwards the two lists, not a prefix) | hub integration |
| `an_existing_control_method_still_answers_through_the_new_dispatcher` | AC2: `control/status` still succeeds and is not swallowed by the pane arm | hub integration |
| `two_connections_share_the_one_pair_of_state_handles` | AC3: two connections cloned from one bundle share the `Arc`s (`ptr_eq`, `strong_count == 3` while both are live) | hub integration |
| `the_state_types_do_not_derive_clone_so_a_connection_cannot_fork_a_copy` | AC3: `PaneState` and `ProfileState` are not `Clone`; `PaneDeps` is | unit (stable-Rust autoref probe; replaces the `compile_fail` doctest A suggested, which would live in production files) |
| `panes_dispatch_takes_both_handles_and_answers_not_implemented_as_a_result` | AC4: `panes::dispatch(method, &cid, &obj, &PaneState, &ProfileState)` signature and reply | unit-level, direct call |
| `profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub` | AC4: `profile::dispatch(.., &ProfileState, &PaneState)` signature (opposite handle order), `profile/rename` answers not-implemented | unit-level, direct call |
| `check_membership_accepts_any_pane` | AC5: `Ok(())` for a pane with no profile and one naming a missing profile | unit |
| `testkit_links` | AC6: `holler-pane-testkit` is a linked dev-dependency (the `use ... as _` is the consumer) | build-level |

Not duplicated: `CATALOG.len() == 22` is already asserted in `holler-proto/src/methods.rs` tests; "no golden change" and "existing hub tests unchanged" are Tier 1 facts for GREEN. AC 7 and 8 are gates, not tests.

### Public API names this test file fixes for F
(The brief and A left them open; T chose A's names so F has one contract to meet.)
- `holler_hub::pane_dispatch::PaneDeps { pub panes: Arc<PaneState>, pub profiles: Arc<ProfileState> }`, `#[derive(Clone)]` (A finding 4).
- `handle_control_conn(stream, registry, roster, lockout, deps: PaneDeps)`: the one added bundled parameter, last.
- `holler_hub::panes::{PaneState::load(&HubState), dispatch(&str, &CorrelationId, &Value, &PaneState, &ProfileState) -> String}` and `holler_hub::profile::{ProfileState::load(&HubState), dispatch(&str, &CorrelationId, &Value, &ProfileState, &PaneState) -> String, check_membership(&Pane, &ProfileState) -> Result<(), PaneError>}`, all `pub`; `dispatch` async.
- Manifest: `holler-pane` dependency and `holler-pane-testkit` dev-dependency on `holler-hub` (F's, per the brief; T did not edit it).

## RED confirmation
Command: `cargo test -p holler-hub --test pane_dispatch_test`. Result: the test target does not build, with 12 errors that are all the missing feature, none a typo or setup mistake:
- `E0432 unresolved import holler_hub::pane_dispatch` / `holler_hub::panes` / `holler_hub::profile` (the new modules)
- `E0432 unresolved import holler_pane` / `holler_pane_testkit` (the missing manifest lines)
- `E0061 this function takes 4 arguments but 5 arguments were supplied` at `handle_control_conn(..., deps)` (the added parameter)
- `E0277 the size for values of type str cannot be known` x5 at the two `dispatch(...)` calls (cascade from the unresolved modules)

The brief's test plan accepts a build failure as the RED ("it cannot compile until the modules exist"). Because a compile-time RED cannot show that the assertions are right, T also validated the file against a throwaway stub implementation in a scratch worktree (discarded; nothing of it is in this tree):
- With the minimal stub plumbing, all 10 tests pass, three runs in a row, and `cargo clippy -p holler-hub --tests -- -D warnings` is clean.
- With the forwarding arm disabled (the pre-#669 behaviour), `every_pane_method_...`, `every_profile_method_...` and `two_connections_share_...` fail on their assertions (a JSON-RPC error where a result is required), while the MethodNotFound, control/status, non-Clone, membership and direct-dispatch tests still pass. So the forwarding tests pin the behaviour, not the structure.
- The first stub run caught a bug in the test's own request ids (a correlation id needs an `h-` or `b-` prefix); fixed.

`rustfmt --check --edition 2021` passes on the file. 362 lines (under 900). The one `#[allow(clippy::...)]` carries `// #669`.

## Ready for F
Confirmed RED is valid; F may implement against these tests.
