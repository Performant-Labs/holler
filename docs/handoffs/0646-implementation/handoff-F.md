# Handoff-F: Phase 5 - Implement (rework 2)

**Date:** 2026-10-10
**Branch / worktree:** `issue-0646-implementation` (`<run-worktree>`)
**Contract:** handoff-T-red.md (the tests and the behaviors they pin), plus the gate tests O applied
verbatim from `gate-tests-dispatch.md` into `circuit/dispatch.rs`'s `mod hold_tests`.

## Summary: the suite is RED for test-side reasons only, and a verified patch is below

The tests are unchanged since T-red (`cbec3bd`). `git diff HEAD -- 'crates/*/tests'` is empty, so
the plugin's RED is the same RED as in rework 1. **No production change can make it GREEN.** I
checked the rework 1 analysis again from the start, and it holds:

| # | Where (test side) | What | Can production code fix it? |
|---|---|---|---|
| 1 | `target_flags.rs:471` | E0277: `error.code()` (`&str`) compared with `code`, which is a `&&str` because it is bound by iterating a `&[(…)]` | **No.** The orphan rule rules out a `PartialEq` impl between std types. `PaneError::code() -> &str` is in frozen `holler-pane` |
| 2 | `target_flags.rs:493, 502, 511` | E0599 ×4: `.unwrap().code()` on the `Ok` value `String`, in cases that each expect a refusal (the same cases the test above it reaches with `expect_err`) | **No.** `String` cannot gain an inherent `.code()`, and the test imports no trait. Under any one `Result` type, test 2 (`expect_err`) and test 3 (`unwrap`) contradict each other |
| 3 | `target_flags.rs:575, 592, 614, 618` | E0061 ×4: `pane_hold_refusal` takes `session` first | **Not in a way that passes.** With T's 2-argument shape it compiles, but the JSON assertion fails: the expected `"session":"ses-demo-c1r1"` is in no input. `ErrorData` has no session field, and the message names only `demo-c1r1` |
| 4 | `close.rs:232` | `close_spec_only_…_touches_nothing_live` asserts P's generation is 2 (one profile `CasPut`), then calls `assert_untouched`, which requires **zero** profile `CasPut`/`Delete`/`Rename` | **No.** A spec removal is a profile write, and `ProfileStoreOp` has no other write op |

**This rework adds proof that fixing the tests is enough.** I applied the patch below to a
scratch copy outside the worktree: HEAD, plus this worktree's diff, plus the patch, built on its
own seeded target directory. The full workspace suite then **exits 0: 1779 passed, 0 failed**, and
CI's clippy gate also passes (details under Self-check). The patch applies cleanly to this worktree
as it stands (`git apply --check` passes). I did **not** apply it here: every hunk is test code,
which T owns.

So the loop needs T, or the human once the rework cap is hit, to apply the patch. A third F pass
would change nothing.

### Decision kept: `pane_hold_refusal(session, e, json)` (3 arguments)

T pinned `pane_hold_refusal(e, json)`. T's own JSON assertion needs `session`, and only the caller
has it. The 3-argument shape:
- matches `hold_cmd::held_refusal(session, e, json)`, the "held form" that binding 2 says to mirror;
- is the only shape under which that assertion can hold.

Reverting to 2 arguments would replace 4 compile errors with one assertion failure that can never
pass, and the user's JSON would lose `session`. I made the call and am reporting it. The patch
updates the 4 call sites. If T would rather keep 2 arguments, T has to drop `session` from the
expected JSON too, and I switch the signature back in a rework.

### The patch (T's to apply; verified GREEN in the scratch copy)

Hunks 1-4 are the four defects above. Fix 4 also drops the import it would leave unused, and
replaces `assert_untouched` with the check that the test can mean: no pane-store write. The last
two hunks are lint-only. CI runs `cargo clippy --workspace --all-targets -- -D warnings`, and these
two lints in test code fail it even after the suite is GREEN:
- `option_map_unit_fn` ×2 in `target_flags.rs`;
- `clippy::unreachable` in T's gate block in `dispatch.rs` (`mod hold_tests`, line 650). That
  block is test code, so I left it alone even though the file is in my write scope.

```diff
diff --git a/crates/holler-cli/tests/pane_verbs/close.rs b/crates/holler-cli/tests/pane_verbs/close.rs
--- a/crates/holler-cli/tests/pane_verbs/close.rs
+++ b/crates/holler-cli/tests/pane_verbs/close.rs
@@ -28,7 +28,7 @@ use holler_pane_testkit::pane_store::PaneStoreOp;
 use holler_pane_testkit::profile_store::ProfileStoreOp;
 use serde_json::json;
 
-use rig::{alpha_world, assert_untouched, bare_step, profile, profile_step, run_both, Rig};
+use rig::{alpha_world, bare_step, profile, profile_step, run_both, Rig};
 
 /// `pane close <name>` with the flags of the case appended.
 fn close<'a>(name: &'a str, flags: &[&'a str]) -> Vec<&'a str> {
@@ -229,7 +229,14 @@ fn close_spec_only_removes_the_spec_and_touches_nothing_live() {
         assert_eq!(run.calls.host, vec![], "no host call");
         assert_eq!(run.calls.harness, vec![], "no harness call");
         assert_eq!(run.calls.probes, vec![], "no probe run");
-        assert_untouched(&run.calls);
+        assert!(
+            !run.calls
+                .panes
+                .iter()
+                .any(|op| matches!(op, PaneStoreOp::CasPut | PaneStoreOp::Delete)),
+            "no pane-store write: {:?}",
+            run.calls.panes
+        );
     }
 }
 
diff --git a/crates/holler-cli/tests/pane_verbs/target_flags.rs b/crates/holler-cli/tests/pane_verbs/target_flags.rs
--- a/crates/holler-cli/tests/pane_verbs/target_flags.rs
+++ b/crates/holler-cli/tests/pane_verbs/target_flags.rs
@@ -468,7 +468,7 @@ fn the_engine_refuses_every_routed_case_with_exit_3() {
         let rig = routing_world();
         let error = resolve_pane_target(rig.ports(), pane, *profile)
             .expect_err(&format!("{pane}: refused with {code}"));
-        assert_eq!(error.code(), code, "{pane}: {error}");
+        assert_eq!(error.code(), *code, "{pane}: {error}");
         assert_eq!(
             class_of(error.code()),
             ErrorClass::Refusal,
@@ -490,7 +490,7 @@ fn the_engine_refuses_every_routed_case_with_exit_3() {
 fn the_engine_checks_the_name_the_scope_the_record_then_the_session() {
     let rig = routing_world();
     // A bad pane name is usage (exit 2), before anything else.
-    let usage = resolve_pane_target(rig.ports(), "a/b", None).unwrap();
+    let usage = resolve_pane_target(rig.ports(), "a/b", None).unwrap_err();
     assert_eq!(usage.code(), "usage");
     assert_eq!(class_of(usage.code()), ErrorClass::Usage);
 
@@ -499,7 +499,7 @@ fn the_engine_checks_the_name_the_scope_the_record_then_the_session() {
     let fresh = Rig::new([], [sample_profile("Demo Alpha", &["demo-c1r1"]).unwrap()]).unwrap();
     assert_eq!(
         resolve_pane_target(fresh.ports(), "demo-c9r9", Some("Demo Alpha"))
-            .unwrap()
+            .unwrap_err()
             .code(),
         "pane-not-in-profile"
     );
@@ -508,7 +508,7 @@ fn the_engine_checks_the_name_the_scope_the_record_then_the_session() {
     // of record is `session-not-found`, not `pane-parked`.
     assert_eq!(
         resolve_pane_target(rig.ports(), "demo-c8r1", None)
-            .unwrap()
+            .unwrap_err()
             .code(),
         "session-not-found"
     );
@@ -572,7 +572,7 @@ fn a_pane_hold_renders_one_line_exit_3_with_the_right_remedy() {
         ("pane-unhealthy", "holler pane doctor"),
         ("pane-shown-driven-mismatch", "holler pane doctor"),
     ] {
-        let text = pane_hold_refusal(&pane_held(code), false)
+        let text = pane_hold_refusal("ses-demo-c1r1", &pane_held(code), false)
             .unwrap_or_else(|| panic!("{code}: the pane arm renders"));
         assert_eq!(text.exit_code, 3, "{code}: exit 3, not the held 4");
         assert!(text.to_stderr, "{code}: a refusal goes to stderr");
@@ -589,7 +589,7 @@ fn a_pane_hold_renders_one_line_exit_3_with_the_right_remedy() {
         );
     }
 
-    let json = pane_hold_refusal(&pane_held("pane-parked"), true)
+    let json = pane_hold_refusal("ses-demo-c1r1", &pane_held("pane-parked"), true)
         .unwrap_or_else(|| panic!("json: the pane arm renders"));
     assert_eq!(json.exit_code, 3);
     assert!(
@@ -607,13 +607,14 @@ fn a_pane_hold_renders_one_line_exit_3_with_the_right_remedy() {
 #[test]
 fn every_other_held_error_falls_through_to_the_generic_arm() {
     let mut operator = pane_held("freeze");
-    operator
-        .data
-        .as_mut()
-        .map(|d| d.hold_kind = Some("operator".into()));
-    assert!(pane_hold_refusal(&operator, false).is_none());
+    if let Some(data) = operator.data.as_mut() {
+        data.hold_kind = Some("operator".into());
+    }
+    assert!(pane_hold_refusal("ses-demo-c1r1", &operator, false).is_none());
 
     let mut unkinded = pane_held("freeze");
-    unkinded.data.as_mut().map(|d| d.hold_kind = None);
-    assert!(pane_hold_refusal(&unkinded, false).is_none());
+    if let Some(data) = unkinded.data.as_mut() {
+        data.hold_kind = None;
+    }
+    assert!(pane_hold_refusal("ses-demo-c1r1", &unkinded, false).is_none());
 }
diff --git a/crates/holler-hub/src/circuit/dispatch.rs b/crates/holler-hub/src/circuit/dispatch.rs
--- a/crates/holler-hub/src/circuit/dispatch.rs
+++ b/crates/holler-hub/src/circuit/dispatch.rs
@@ -454,7 +454,7 @@ impl LastSeenFlusher {
 }
 
 #[cfg(test)]
-#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #442
+#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unreachable)] // #442, #646
 mod hold_tests {
     use super::*;
```

All hunks are rustfmt-clean (edition 2021). The `allow` keeps an issue link on its line, so
`scripts/lint.sh` check 1 still passes.

## What I changed

**Nothing in this rework.** No production change moves any of the four defects. The
implementation from the first pass stands as is. In short:

- **Hub gate** (Objective 3, bindings 1/2/6):
  - `dispatch.rs`:
    - `HoldGate.panes: Option<Arc<PaneState>>`. The struct drops `Copy` and keeps `Clone`.
    - `SendPromptError` derives `Debug`.
    - `pane_state_refusal` checks parked, then unhealthy, then SHOWN≠DRIVEN, where a mismatch
      needs both sides present.
    - `pane_gate` makes one synchronous `list()` and matches `session_of_record` against the bare
      session. It fails closed: the first bad pane in name order refuses, with
      `session_held` + `hold_kind: "pane"` + `data.reason` = the code, and no `since`.
    - It is called right after `holds.admit`.
  - `live.rs`: `Registry::with_panes` / `panes()`.
  - `circuit.rs`: one line, `panes: self.registry.panes().cloned()`.
  - `serve.rs`: `PaneDeps::load` moved above the registry chain, which gains `.with_panes(..)`.
- **CLI routing** (Objective 2, bindings 2/3/5), in `say_cmd.rs`:
  - The three consts.
  - `resolve_pane_target`, in T's chain order: name → profile scope → record → session → parked →
    unhealthy → mismatch.
  - `route_target`, called by all three verbs. A SESSION with `--profile` is usage (exit 2) naming
    `--pane`. A pane target resolves over `Wiring::connect()`, and a refusal carries its code
    exactly once.
  - `QUEUE_ACCEPT_WAIT` (2 s) replaces only the control call's wait, and `queue_outcome` maps how
    the wait ended. The early return applies only to `--pane --queue` with no `--server`.
  - `pane_hold_refusal(session, e, json)` runs before `is_held`.
  - `interrupt_cmd.rs` calls `route_target`, plus the pane arm. `answer_cmd.rs` calls
    `route_target` only (the hub does not gate `session/answer`).
- **`pane close`** (Objective 1, bindings 4/7/8), `pane/close.rs`:
  - **Args:** optional `PANE`, and its absence is the verb's own usage error.
  - **Plan:** type PANE and P, read P, then the spec-presence pre-check, all before any write.
  - **Act:** `stop_owned` → herdr `close` → `pane_store.delete`, run inside
    `scope.edit_spec(P, Remove, act)`.
  - **`--spec-only`:** `edit_spec` with an empty act, and no record needed.
  - **Failures:** once the act begins, the message names the step reached and appends
    `reconcile_step` exactly once.
- **Docs:** the ADR-0003 rows for close and say. In ADR-0021: §8 "as built" (Close, Prompting a
  pane, the pane-state gate), §9 code rows, the open-code note and a pointer from park.

This handoff replaces the rework 1 version. That file was never committed. Its per-file
rationale is condensed above, and its CHANGELOG draft is carried over verbatim below.

## Self-check

These are local runs. The authoritative verdict is the plugin's T-green run.

| Run | Where | Result |
|---|---|---|
| `cargo test -p holler-cli --test pane_verbs --no-run` | worktree | **9 compile errors, all in `target_flags.rs`** (E0277 ×1 at 471, E0599 ×4 at 494/495/503/512, E0061 ×4 at 575/592/614/618), the same as rework 1. None is in production code. This is why the plugin's run exits 101 |
| `cargo test -p holler-hub --lib` | worktree | **83 passed**, 0 failed (6 gate tests and 2 existing hold tests included) |
| `cargo test -p holler-cli --test pane_cli_process` | worktree | **36 passed**, 0 failed (the 3 T-red process cases are green) |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the plugin's command) | scratch copy: HEAD + worktree diff + fixes 1-4 (without the two lint hunks), seeded target | **exit 0. 140 test binaries, 1779 passed, 0 failed.** The only compiler warning was the unused `assert_untouched` import, which the final patch removes |
| the same tree plus the final patch (lint hunks, import fix) | scratch | `pane_verbs` **234 passed**, no warnings. `holler-hub --lib` **83 passed** |
| `cargo clippy --keep-going --workspace --all-targets -- -D warnings` (CI's gate) | scratch, final patch | **exit 0.** Without the patch, the only errors in the workspace are the 3 test-side lints above, so production is clippy-clean under `-D warnings` |
| `scripts/lint.sh` | scratch, final patch | **exit 0.** File-size advisories only, and every touched file is under 900 lines (`circuit.rs` 898, `live.rs` 884, `serve.rs` 842, `dispatch.rs` 820) |
| `git apply --check <patch>` | worktree | applies cleanly (dry run only; nothing was written) |

## Tests I think are wrong

The four defects in the table at the top, plus the two CI-only lints. The patch above fixes all of
them, and I did **not** edit any test. T, please confirm and apply.

Also not a failure: `crates/holler-cli/tests/fixtures/cli-surface.txt` still parses, because PANE is
optional, so `cli_surface_test` passes. The surface should still show the positional. Suggested
lines for its `# #646` block:
- `pane close | demo-c1r1`
- `pane close | demo-c1r1 --profile demo --format=json`
- `pane close | demo-c1r1 --profile demo --spec-only`
- `say | --pane demo-c1r1 --profile demo hello --queue`

## For O: outside my write scope (unchanged from rework 1)

- **`CHANGELOG.md`**: my edit was denied in the first pass. Draft entry for `[Unreleased] /
  Enhancements`, after the park/unpark entry:
  > Pane control, `holler pane close PANE [--profile NAME] [--spec-only]` (epic #633): stops the
  > pane's owned processes, closes its Herdr pane and deletes its record, in that order, never
  > moving another pane. With `--profile P` the spec removal and the close are one transaction
  > (a failed close puts P's specs back), and P must hold a spec for the pane
  > (`pane-not-in-profile` otherwise, before anything is written). `--spec-only` removes the
  > spec and closes nothing, so it also cleans up a spec whose pane is gone. A failure once the
  > close has started names the step it reached and ends with the `holler pane doctor` command
  > that reconciles. Takes `--format=json` (#646, part 2 of 3).
  >
  > Pane control, `say`, `interrupt` and `answer` with `--pane NAME [--profile NAME]` (epic
  > #633): they prompt the pane's session of record instead of refusing with `not implemented
  > (story #646)`. They refuse, with exit 3 and the code in the message, a pane with no record,
  > outside the profile or with no session of record, and one whose record says it is parked
  > (`pane-parked`), unhealthy (`pane-unhealthy`) or showing a session other than the one it
  > drives (`pane-shown-driven-mismatch`). `--profile` without `--pane` is a usage error. The
  > hub refuses a prompt to such a pane's session of record by any route, a bare `say SESSION`
  > included, as `session_held` with `hold_kind: "pane"`, which `holler release` does not lift.
  > `say` and `interrupt` print it as exit 3 with the `holler pane unpark` or `holler pane doctor`
  > remedy. `say --pane NAME --queue` prints `queued <session>` and exits 0 once the hub has
  > accepted the prompt (`say --queue SESSION` still waits for the reply). Until the stores are
  > wired into the binary (#649), `--pane` answers `not implemented` (#646, part 3 of 3).
- **Stale text outside the radius:**
  - `cli.rs:515/519/538/542/589/593`: the `--pane` help strings still say "refused until #646", and
    `holler say --help` shows that.
  - `prompt_target.rs:11-12, 67` still says the forms are not routed.

  `cli.rs` and `prompt_target.rs` are not in the brief's blast radius, so this needs a one-line
  follow-up or a radius extension.

## Decisions S should audit (unchanged)

1. An unreadable pane registry makes the gate refuse nothing, mirroring `holds.rs`.
2. `holds.admit` spends a `--grant` before the pane gate runs (T pinned the gate after admit).
3. A bare `interrupt SESSION TEXT` to a gated session has its cancel applied, and then the
   redirect is refused. `interrupt.rs`'s pre-cancel fast path is outside the radius.
4. A remote `--server` with a routed `--queue` keeps waiting for the reply: the remote client
   cannot tell a read deadline from a dropped connection.
5. Closing a Herdr pane that is already gone fails loudly, with the step named. Doctor reconciles
   it.
6. Close reads the record in the plan, before `edit_spec`. It does no snapshot observe, per T's
   pin.
7. `answer --pane` checks pane state on the CLI side, though the hub does not gate
   `session/answer`.

## Ready for T(green)

The implementation is done against the RED contract, and this rework changes no production code.
The plugin's run will stay **RED (exit 101)** until the test patch above is applied. With it
applied, the plugin's own command exits 0 in an independent scratch build (1779 passed, 0 failed),
and CI's clippy gate passes too. **Route this to T (or the human), not back to F**: a further F pass
cannot change the verdict.
