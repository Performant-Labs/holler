# Decision journal: #644 launch-relaunch

## A (up-front plan review) — 2026-10-09T17:07-06:00
- **Decided:** BLOCK, on four block findings (handoff-A.md):
  1. `tx_launch::spec_of_pane` duplicates the Pane-to-spec mapping. ADR-0021 section 3 places that mapping in
     `profile_snapshot.rs`, and #662a is writing it there as `spec_from_pane`, with `FIXED_PORT_POLICY_PREFIX` and
     `fixed_port_policy`.
  2. Relaunch's cwd and cell rules live only in `relaunch.rs`, and `RelaunchRequest.move_grid` is computed by the caller,
     so the engine #664 will call can record a false cwd or leave a Herdr pane open with no record.
  3. The record at R infers `last_observed.driven`, which the frozen `LastObserved` contract and I6 forbid.
  4. `launch::reconcile_step` is a second copy of #663's planned shared `profile_scope::reconcile_step`, and under the
     real scope the step would be printed twice.
  Six warns: reuse `holler_proto::clock::now_millis`; three parallel test rigs; B10 before R; no Holler path frees a cell
  left by a crash; a lost-update window on P between two reads; coordination of the ADR-0021 edits, including marking the
  "no operation id" decision PROPOSED or getting the operator's confirmation.
- **Assumed:** the sibling briefs on `issue-662-implementation` (`5b47d82`), `issue-663-implementation` (`aaf8fb5`),
  `issue-647-implementation` (`878a5b9`) and `issue-643-implementation` (`2e7f221`) are the plans those runs will
  implement, and none of them has merged code yet. #662's run is scoped to 662a, which includes `profile_snapshot.rs`.
- **Hedged:** finding 4 depends on #663's own plan review, which has not happened yet. Its fix is phrased as "one function,
  one owner, one doctor form, agreed with #663" rather than naming the final text. For finding 1, the order of the stories
  (#662a first, or #644 filling #662a's API in `profile_snapshot.rs`) is the MO's call; either way there is one mapping, in
  `profile_snapshot.rs`.
- **Evidence:** read the whole brief (2,070 lines) and both outside-review rounds (r1, r2); ADR-0021 in full; issues #644,
  #662 and #664; `holler-pane/src/{lib,ports,pane,profile}.rs` and the module stubs; `holler-cli/src/pane/{args,mod,wiring,
  profile_scope,close}.rs`, `output.rs` and the profile verb stubs; the test kit's `fixture.rs`, `herdr.rs` and
  `profile_scope.rs`; `holler-proto/src/clock.rs`; `tests/pane_verbs/main.rs`, `launch.rs` and `verb_harness/mod.rs`; the
  four sibling briefs (`git show origin/issue-<N>-implementation:docs/handoffs/<N>-brief.md`); and greps for open codes, the
  clock helpers and shell-quoting helpers.
