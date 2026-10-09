# Handoff-A-dup: Phase 7 - #669 hub plumbing for `pane/*` and `profile/*`  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-669-implementation
**Diff base:** f2602ba (origin/main)   **Diff head:** 851684d
**Reuse map:** docs/handoffs/669-brief.md §Files "Reuse map" (there is no separate survey.md)
**Verdict:** PASS

## Summary

PASS. F extended each object the Reuse map named and built no parallel path in production code:

- **The `control_hold.rs` precedent.** `control_server.rs` gains one arm that calls `pane_dispatch::forward`, a plain function that calls `panes::dispatch` or `profile::dispatch`. There is no registry of function pointers.
- **`encode_response` and `encode_error`.** They carry every reply. `pane_dispatch::reply_line` is a thin typed wrapper over `encode_response` and the only place a `PaneReply` becomes a line, so #639, #661 and #665 have no reason to write a second encoder.
- **`Holds::load` and the `Arc<Roster>`/`Arc<Lockout>` pattern.** `PaneState::load` and `ProfileState::load` have the same `(&HubState) -> Self` shape as `Holds::load`, which never fails and handles a corrupt file internally. They are shared as non-`Clone` types behind `Arc`. They are bundled in `PaneDeps`, which has the `AdminDeps` shape (`#[derive(Clone)]`, `Arc` fields): one `SharedState` field and one `accept_loop` parameter.
- **The method lists.** The names come from `holler_proto::methods`; the hub does not declare them again.

There is no near-copy of the token store, `Lockout`, `Roster`, the log helper, `support::Hub`, `Body`, `mint_token`, `join`, `wait_for` or `StateDir`. No production code changed after F: T-green changed only the two `#![allow]` lines of the test.

The three findings are all `warn`, and all are in the test file or apply to the next stories:

- Finding 1: `sample_pane` copies `holler-pane`'s `pane_json` fixture.
- Finding 2: `fresh_deps` builds the `PaneDeps` bundle by hand instead of calling `PaneDeps::load`.
- Finding 3: the in-process harness is local to one test file, and #639 and #661 will need it too.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-hub/tests/pane_dispatch_test.rs:315-336` | `sample_pane()` is a trimmed copy of the canonical fixture `pane_json()` in `crates/holler-pane/tests/common/mod.rs:20-49`. The name, generation, herdr, host, harness, role, hold, last_observed, model, context, command and probe values are the same. It drops `session_of_record`, `probe.last` and one `env` entry, and makes `profile` optional. The original is unreachable from here: `holler-pane`'s `tests/common` is private to that crate's test binaries. Its cross-crate home, `holler-pane-testkit`, is empty and belongs to #638, which is outside this slice's radius. So this is a warn, not a block. Without a fix, #639 and #661 will each copy it again. | O adds this to #638's brief: move `pane_json`/`pane` (and the `MemPaneStore` fake) from `holler-pane/tests/common` into `holler-pane-testkit`. Then #639 and #661 build their hub-test panes from the testkit and replace this `sample_pane`. The testkit is already a `holler-hub` dev-dependency (`testkit_links`), so this needs no new manifest line. |
| 2 | warn | `crates/holler-hub/tests/pane_dispatch_test.rs:95-103` | `fresh_deps()` builds `PaneDeps { panes: Arc::new(PaneState::load(..)), profiles: Arc::new(ProfileState::load(..)) }` by hand, which repeats the body of `PaneDeps::load` (`src/pane_dispatch.rs:60-65`). T wrote the test before F added `PaneDeps::load`. As a result, no test calls the constructor that `build_shared_state` calls. | T: change `fresh_deps` to call `PaneDeps::load(&state)`, a one-line change. When #639 and #661 give `load` real behaviour, the tests then exercise the production constructor and not a copy of it. |
| 3 | warn | `crates/holler-hub/tests/pane_dispatch_test.rs:49-125` | `Conn`, `connect`, `Conn::call`, `fresh_deps`, `pane_outcome` and `error_code` are now the hub's only in-process harness for the control socket, and they live in one test file. Phase 3 finding 8 asked for that. `holler-hub` has no shared test module (its test files are each self-contained), and #639 and #661 will need the same client to drive `pane/*` and `profile/*` from `tests/`. | O adds a line to the briefs of #639 and #661: the first story that needs the harness moves it into `crates/holler-hub/tests/common/mod.rs`, following the `tests/common` precedent of `holler-proto` and `holler-pane`, and does not copy it into a new test file. Do not name the module `support`, to keep it distinct from `holler-cli`'s process-based `support::Hub`. |

No duplication in production code; the extension is clean. Also checked, with nothing to flag:

- `forward`'s defensive branch is the third `encode_error(cid, Code::MethodNotFound, format!("unknown control method: {m}"))`, after `control_server.rs:121` and `:171`. That is the file's existing pattern, and Phase 3 finding 4 recommended this exact branch. A shared helper would mean a drive-by edit of `control_server.rs`.
- `forward` is not a parallel path to `dispatch_allowlisted`. That function is the shared entry for verbs that both `admin/*` and `control/*` reach, and `pane/*` stays unreachable over `admin/*`, as the brief intends.
- `reply_line` lives in `pane_dispatch.rs`, and `panes`/`profile` import it from there. That module cycle has the same shape as `control_hold` and `control_server`.
- `profile/rename.rs` holds a one-function stub, not an empty file. This is Phase 3 finding 6 applied and recorded in decisions.md: a reviewed extension, not drift. S judges it against the issue's word "empty".
- The test's `METHOD_NOT_FOUND = -32601` literal follows existing tests (`remote_admin_test.rs`, `hold_hub_test.rs`). `control::send_over`, the existing control client, is private and opens a socket by path, so the test could not have reused it.
- File sizes: `control_server.rs` is 837 lines and `serve.rs` is 838, both under the 900-line fail line. No protocol, golden-file, ADR or `send_prompt` change.

Still open from Phase 3, and F did not change them (both are O's call, recorded in decisions.md):

- Finding 2: `check_membership` cannot see the stored pane record.
- Finding 7: `pane_wiring.rs` is a permanent empty placeholder.

## Notes for F

None (PASS). Finding 2 is a one-line change for T, and findings 1 and 3 are follow-ups for O to put into the briefs of #638, #639 and #661.
