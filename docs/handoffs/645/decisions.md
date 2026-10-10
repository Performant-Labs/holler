# Decision journal: #645 switch-reset

## A (up-front plan review) — 2026-10-09T19:51-06:00
- **Decided:** BLOCK, on one block finding (handoff-A.md):
  1. Decision 16 loads `tests/pane_verbs/doctor/rig.rs` a second time with `#[path]` from `switch.rs`, while
     `doctor.rs:10` already loads it as `mod rig;`. Clippy's `duplicate_mod` then fails AC 25 and CI
     (`cargo clippy --workspace --all-targets -- -D warnings`). This also departs from the crate's pattern of one
     declaration reached by crate path (#662's `crate::list::rig`). The brief rules out both editing `doctor.rs` and
     copying the rig, so O must choose between a named one-word visibility edit in `doctor.rs` (preferred) and a
     justified `#[allow(clippy::duplicate_mod)] // #645`.
  Six warns:
  - P5's one-pane-per-session rule is a read-check with a race against a concurrent switch, and the hub store does not
    enforce it.
  - P3's message spells `holler pane relaunch` outside the remedy table.
  - The as-built paragraph and the `unavailable` decision would land in section 11, while #644 records the same
    decision in section 8.
  - The reconcile-step lead-in is spelled in both `holler-pane` and #663's `holler-cli` function.
  - Forward-compat should name `SERVER_UNHEALTHY`, `ORCHESTRATOR_PANE` and `parse_session_id` for reuse (ADR row 345,
    646c).
  - P1 indexes `panes[0]`.
  The rest of the plan matches the codebase: one pure engine over `Ports`, refusals before any live change, live
  health, a clone-and-set single compare-and-swap, no Herdr or host call, reuse of `doctor_command`, `quoted` and
  `shown_differs`, and open codes declared with `from_static`. `unavailable` and the CLI pairing and output shape
  match #644. 645b's prompt goes through `send_prompt`.
- **Assumed:**
  - The sibling briefs on their worktrees are the plans those runs will implement: `issue-644-implementation` at
    `7195993`, `issue-646-implementation` at `e957a8d`, `issue-643-implementation` and `issue-663-implementation` at
    `137c00f`.
  - #647 part 2 (still open) will keep owning `tests/pane_verbs/doctor.rs`.
  - `origin/main` moved from `ce12cdb` to `dc300ab` (#640 part 2, #642 part 1). Among the files this story touches,
    that changed only `CHANGELOG.md`, so the brief's evidence still holds.
- **Hedged:**
  - Finding 1's choice between (a) and (b) is O's, because it trades the brief's no-#647-file rule against a lint
    suppression. I recommend (a) as the established pattern but did not decide it.
  - Finding 2 is a warn, not a block. The race is narrow (two concurrent switches to one session), no merged pattern is
    broken, and the fix is a store change outside 645a.
  - Findings 4 and 5 depend on the merge order of #644 and #663, so the fix is phrased as "the second to land cites the
    first".
- **Evidence:**
  - The whole brief (1,120 lines), and the outside brief review r1 with its usage file.
  - ADR-0021: sections 1-5, 7-9, 11, 12, "Deferred" and "Decisions taken". ADR 0003's pane rows.
  - `holler-pane/src/{lib,ports,error,findings,reconcile,generation,probe}.rs`, `reconcile/observe.rs`, and the `tx_*`
    stubs.
  - `holler-cli/src/pane/{doctor,mod,wiring}.rs` and `output.rs`.
  - `tests/pane_verbs/{main,doctor}.rs`, `doctor/rig.rs`, `doctor/surface.rs:456-514`, and `tests/profile_verbs/` (the
    rig pattern).
  - The test kit's harness and pane-store API.
  - `holler-hub/src/{holds.rs,panes/store.rs}` and `circuit/dispatch.rs`.
  - The OpenCode adapter's module doc (#642 part 1, now on `main`).
  - Issues #645 and #647. `Cargo.toml` lints, `ci.yml:280` and `scripts/lint.sh`.
  - The sibling briefs and handoffs: 644 brief and handoff-A, 646 brief, 643 brief, 663 brief.
  - Greps for open codes, `--as-operator`, session-id validators and `HarnessPort` wrappers.
  - A scratch-crate reproduction of `duplicate_mod` on clippy 0.1.98 / rustc 1.98.1, in the session scratchpad, outside
    the repo.
