# Handoff-A-dup: Phase 7 - #669 hub plumbing for `pane/*` and `profile/*`  (anti-duplication gate, pass 2)

**Date:** 2026-10-09
**Branch:** issue-669-implementation
**Diff base:** f2602ba (origin/main; the remote `main` is still f2602ba)   **Diff head:** 820e385
**This cycle's diff:** 8275e73..820e385, T's test-only rework after S's REWORK. One code file changed: `crates/holler-hub/tests/pane_dispatch_test.rs` (+16/-14).
**Reuse map:** docs/handoffs/669-brief.md §Files "Reuse map" (there is no separate survey.md)
**Verdict:** PASS

## Summary

PASS. This cycle changed no production code: `git diff 851684d 820e385 -- crates/holler-hub/src` is empty. Pass 1's
result therefore still holds for the whole branch: F extended every object the Reuse map named and built no parallel
path. The rework touched only the test file. It removed a duplicate and added none:

- **`fresh_deps` now calls `PaneDeps::load(&state)`** (`:95-99`) instead of building the bundle by hand. This resolves pass
  1's W2: the tests now run the constructor that `build_shared_state` calls (`serve.rs:366`), not a copy of its body.
- **The AC 2 probe sends `control/roster`, not `control/status`** (S's REWORK item). It goes through the existing harness
  (`Conn::call`) and adds no helper. I traced the route: `control/roster` has no exact arm in `dispatch_control`, so it
  passes the new pane arm (`control_server.rs:117`), reaches the `control/` prefix arm (`:118`), and goes on to
  `dispatch_session_control` (`:148`) and `roster_control` (`:461-475`). That handler reads only the `Roster` and
  `Registry::holds()`. For `Registry::new()`, `holds()` is `Holds::in_memory()` (`live.rs:427-429`, `holds.rs:207-211`).
  The probe can now catch a pane arm that swallows `control/` methods, and it does not touch `$HOME`.
- The comment fix in the forwarding-table test changes wording only.

The rework added no near-copy of the token store, `Lockout`, `Roster`, the log helper, or `Hub`, `Body`, `mint_token`,
`join`, `wait_for` or `StateDir`. Two warns remain from pass 1, both about the test file and for the next stories. The
rework did not cause them and did not change them.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-hub/tests/pane_dispatch_test.rs:317-338` | Pass 1 W1, unchanged. `sample_pane()` is a trimmed copy of the canonical fixture `pane_json()` (`crates/holler-pane/tests/common/mod.rs:20-51`). The original is private to `holler-pane`'s test binaries. Its cross-crate home, `holler-pane-testkit`, is still a 6-line skeleton that belongs to #638, outside this slice. | O adds this to #638's brief: move `pane_json`/`pane` (and the `MemPaneStore` fake) into `holler-pane-testkit`. #639 and #661 then replace `sample_pane` with the testkit fixture. This needs no manifest line, because the testkit is already a `holler-hub` dev-dependency. |
| 2 | warn | `crates/holler-hub/tests/pane_dispatch_test.rs:49-121` | Pass 1 W3, unchanged. The hub's only in-process harness for the control socket is `Conn`, `connect`, `Conn::call`, `fresh_deps`, `pane_outcome` and `error_code`, and it lives in this one test file. `crates/holler-hub/tests/` has no shared module, and #639 and #661 will need the same client. | O adds a line to the briefs of #639 and #661: the first story that needs the harness moves it into `crates/holler-hub/tests/common/mod.rs` (the `tests/common` precedent of `holler-proto` and `holler-pane`; not `support`, which is `holler-cli`'s process harness) and does not copy it. |

**Resolved since pass 1:** W2. `fresh_deps` repeated the body of `PaneDeps::load`; it now calls it (`:98`).

There is no new duplication and the rework introduced no drift. I also checked these and found nothing to flag:

- The rework adds no `pub` item, test hook or production edit. The request id `h-roster-1` follows the `h-` prefix that
  the other probes use.
- `fresh_deps` keeps the per-file temp-dir pattern of the hub tests (`tempfile::tempdir()` with `HubState::from_root`;
  `tempfile` was already a dev-dependency). It is not a near-copy of `holler-cli`'s process-based `StateDir`.
- The sibling slice #670 (the 0670 worktree, read-only) refers to none of `PaneDeps`, `reply_line`, `check_membership`,
  `PaneState`, `ProfileState`, `forward` or the method lists, and it does not touch `holler-hub`. The two slices do not
  overlap.
- A wording note for T or S, not an architecture finding: the new probe comment (`:185-188`) says "The other control
  handlers resolve the state dir from the environment". Only five do: `control/status`, `control/caps`,
  `control/query_local`, `control/say` and `control/interrupt` (`control_status.rs:26`; `control_server.rs:103, 226,
  230, 303, 370`, the first three through `read_listening_here` at `:623-625`). The overstatement errs on the safe side,
  so it needs no follow-up.

Still open from Phase 3, and unchanged (O decides; both are recorded in decisions.md):

- Finding 2: `check_membership` cannot see the stored pane record.
- Finding 7: `pane_wiring.rs` is a permanent empty placeholder.

## Notes for F

None (PASS). Findings 1 and 2 are follow-ups for O to put into the briefs of #638, #639 and #661.

## Pass history

- **Pass 1** (2026-10-09 06:47 MDT), diff f2602ba...851684d: PASS with 3 warns (W1 fixture copy, W2 hand-built `PaneDeps`,
  W3 harness location). The full text is in commit 8275e73.
- **S** (2026-10-09 06:56 MDT): REWORK, test-only. The `control/status` probe could write `$HOME/.holler/hub/identity.key`
  and could not fail because of the new arm.
- **T rework** (commit 820e385): the probe now sends `control/roster`, W2 is folded in, and one comment is corrected. No `src/`
  change.
- **Pass 2** (this pass), diff f2602ba...820e385: PASS with 2 warns (pass 1 W1 and W3). W2 is resolved.
