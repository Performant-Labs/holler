# Handoff-A: Phase 3 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `89b611f`; base `3bdd129` = origin/main)
**Brief reviewed:** `docs/handoffs/663-brief.md` (as of `89b611f`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)" table under "Files" (this run has no separate survey.md)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, on one finding. The code plan is sound:

- `StoreScope` implements the frozen `ProfileScope` in the file ADR-0021 section 5 names. It has the fake's
  `Arc<dyn ProfileStore>`, `Arc<dyn PaneStore>`, `Actor` shape, so the suite's `build` closure and #649's `Wiring` can
  construct it.
- `run_probe` stays in `holler-pane`, whose one side effect ADR-0021 allows it to be. It uses std only: no shell, no `libc`,
  no `unsafe`, no new dependency.
- The tests are inline, the workspace's `#[cfg(test)]` plus `#[allow(...)] // #NNN` pattern. The conformance suite is reused
  as is.
- The one copy (the membership rule) is justified in writing.

**The block:** the plan makes several decisions that extend or narrow ADR-0021. Decision 22 then rules out the ADR edit and
defers it to a follow-up, F3, because the ADR is "outside the blast radius". The stack rule says such a decision needs the
ADR updated in the same change. This repo's practice since #634 closed is to do exactly that, and this gate has twice
widened a story's blast radius to make it happen (#683 and #688). The cost of deferring is already visible: #644's in-flight
brief pastes this brief's Decisions 5-8 as its contract, because the standing spec does not carry them. Six warns besides.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| B-1 | block | Decision 22 ("No ADR or test-kit edit"); F3; C6, C7, C10; Decisions 5, 6, 7, 8, 11, 14, 15; Blast radius and AC 12 | ADRs; cross-cutting (failure taxonomy, the I5 bound) | **The plan extends or narrows ADR-0021 in five places:**<br>(1) Section 1 (B1, lines 78-80) gives `ProbeResult`'s three shapes but not when each is produced. Decision 14 (C10) makes a non-zero exit or a signal `error` whatever the output. That changes when a ratified record answers `ok`, and the answer is persisted in `Pane.probe.last`. #696 will expose this runner to the adapters "with no behaviour change".<br>(2) Section 2 (lines 86-88) says every port method "returns within I5's bound (default 10 s) or with `timeout`". Decision 11 makes `edit_spec`'s bound the sum of up to four store calls plus the act. Decision 15 makes `run_probe` return at the deadline plus up to 1 s.<br>(3) Section 8, step 2: a first write that times out "may have landed" (Decision 6).<br>(4) Section 8, steps 5-6, decide only the conflict. Decision 5 decides the other restore failures: the error keeps its code and names P, the unrestored edit and the act's error. Decision 7 puts the reconcile step inside the scope's `profile-conflict`, where step 6 and section 12 say "the verb ... prints the reconcile step".<br>(5) Section 8 asks for "the pane doctor command line for that pane" (lines 273-275, 299-301). Decision 8 fixes a profile-scoped text instead, and C9 gives the reason.<br>**Why this is a block.** The stack rule: "A decision that contradicts or extends an ADR ... needs the ADR updated in the same change." The brief's reason, the blast radius, is one this gate has already overruled:<br>- #683's review blocked a brief that ruled the ADR out, and the ADR was added to its radius. PR #692: "The architecture review required the ADR update in the same change."<br>- PR #697 (#688): "The architecture review of the plan required this in the same change (as in #692)."<br>- #639 and #676 amended ADR-0021 in their own PRs. "Decisions taken" item 2 widened #661's radius for the same purpose.<br>- #644, #647 and #662 plan ADR-0021 edits in their own changes now (#644's review, finding 10).<br>- The epic's hot-spot line "`docs/adr/ADR-0021.md`: #634" (epic line 195) is historical. #634 is closed, and every amendment since has come from the deciding story.<br>- This brief already widens the issue's radius by `CHANGELOG.md`.<br>**Cost of deferring.** #644's brief (origin/issue-644-implementation at `7195993`, I-3, C-15, C-16) already codes against Decisions 5-8 as pasted brief text. Until F3 lands, #646, #664, #647 and #696 would read a standing spec that is silent on the probe verdict and the restore failure, and wrong on both bounds. F3 is filed, not scheduled. | Amend the brief:<br>- Add `docs/adr/ADR-0021.md` to Files, Blast radius and AC 12.<br>- Replace Decision 22 and F3's ADR items with a new AC that lists the edits, (a) to (e) in Notes for O. Each is a sentence or two, made in place and cited `(#663)`. None changes a code, `class_of` or a table's class.<br>- Keep the frozen trait doc lines a follow-up (W-6); the ADR text cites it.<br>- Alternative, as #683's review allowed: a separate `docs(adr)` PR merged before the fresh run. The AC then checks that the merged ADR says (a) to (e). |
| W-1 | warn | Decision 8 ("`pub` because the spec-editing verbs (#644, #646) print the same step ...; one copy"); Forward-compat row #644 | duplication (cross-story); abstraction level | `reconcile_step(&ProfileName)` cannot produce the step for a run without `--profile`, which section 8 also requires. So it is not "one copy" for #644 and #646. #644's review blocked on this ("One function, one owner and one doctor form, agreed with #663", finding 4). #644's amended brief now declares `pub(crate) const RECONCILE_STEP_UNSCOPED: &str = "to reconcile, run holler pane doctor"` in `launch.rs`. #646 (`close`) needs the same text, and would have to import it from another verb's file or copy it. | Two options:<br>- Preferred: host the unscoped form beside `reconcile_step` in `profile_scope.rs`, as a `pub const` with #644's exact text. This is additive: `reconcile_step(&ProfileName)` keeps the signature #644 pastes. Say in Decision 8 that the two forms are the whole set, and tell #644 to import the const.<br>- Otherwise: say in Decision 8 that the unscoped form is `launch::RECONCILE_STEP_UNSCOPED` and that #646 imports it.<br>Either way, B-1 (e) records where the forms live. |
| W-2 | warn | F1 ("amend `FakeProfileScope` ... to Decisions 5, 6 and 7"); the Risk "the fake and the real scope differ in three messages until F1" | dependency direction | ADR-0021 section 5 (lines 184-186) says the test kit "must not depend on `holler-cli`", and its manifest has only `holler-pane` and `serde_json`. Decisions 5-7 put `reconcile_step`'s text, which is in `holler-cli`, into the scope's messages. The fake can match them only in two ways: copy the formatter and its POSIX quoting into the test kit, or wait until `reconcile_step` is hoisted into `holler-pane`, which is frozen (`holler-pane/**` is #637 only). F1 names neither. Until then, a verb test over the fake cannot see the step. #644 works around this by pinning it against the real `StoreScope` (its C-15 and AC 16k). | Reword F1 to name its route:<br>- (a) F2's `holler-pane` hoist takes `reconcile_step` too, since it is pure formatting over a `ProfileName`, and the fake calls it.<br>- (b) F1 aligns only the fake's codes and its open-point behaviour, and states that verb stories pin the step text against `StoreScope`.<br>Do not make a test-kit copy the default: it would be the second duplicate this epic carries (after the membership rule, W-5). |
| W-3 | warn | Decision 5 ("the payload extended is the variant's one string ..."), Decision 6; Files ("the message augmentation") | pattern consistency (error handling); abstraction level | `error.rs:399-401` fixes the payload names: "`what` is the thing the error is about ..., `message` is free text, `op` is the operation that timed out". Appending "; profile ... still holds the edit ...; to reconcile, run ..." turns `Timeout.op` into a sentence. `what` already carries advice in places (`GridAmbiguous` "says what to write instead"; `ProfileConflict` "names the profile and the reconcile step"), so `what` is the smaller stretch. The append helper is also a second exhaustive walk of all 23 variants, after `classify` and `Display`, and it lives in another crate. The closed enum and the blast radius force this, so it is not a block. | Three changes:<br>- Match exhaustively, with no `_` arm, so that a variant a later amendment adds fails to compile here instead of passing through unextended.<br>- Say in the rustdoc that `Timeout.op` carries the context on purpose.<br>- Add the helper to F2's hoist, as a `PaneError` method in `error.rs` beside `code()`. |
| W-4 | warn | Decision 20 and the Forward-compat row for #696; Files ("private helpers (spawn, the stdout reader thread, the wait, the group kill, the match)") | abstraction level (forward-compat) | #696 (open) says: "When #663 lands, expose its bounded runner from `holler-pane` and switch the host and OpenCode adapters to it ... No behaviour change ... including the fake-`kill` seam". Its title names captured stderr. This runner discards stderr, sends `KILL` with no `TERM` grace and runs `kill` from `PATH` with no seam. #641's planned runner (unmerged) uses `kill -s TERM -- -PGID` and a fake-`kill` seam. Each choice is right for a probe, but #696 can lift the runner without a rewrite only if the process mechanics stay apart from the probe's verdict. | Inside `probe.rs`:<br>- The mechanics (spawn in its own group, bounded and capped read, deadline, group kill, reap) return a private outcome type: exit status and bytes, timed out, overflow, spawn error by `ErrorKind`, or read error.<br>- A separate function maps that outcome to `ProbeResult` (the expect match and the fixed reasons).<br>- The rustdoc lists the three points #696 must parameterize: stderr, the signal and grace, and the kill program.<br>No public item is added, so Decision 20 holds. |
| W-5 | warn | Reuse map row "the membership rule" ("a third private copy"); F2; Risks | duplication | A third copy of the `pane-in-other-profile` rule, after the test kit's `check_membership` (`pane_store.rs:225`, `pub(crate)`) and the hub's `refuse_profile_move` (`panes/store.rs:344`, private). It is justified in writing. `holler-pane`, the right home, is frozen and outside the radius. `holler-cli` does depend on `holler-hub`, but calling hub-internal registry code from a CLI-side helper would cut across ruling 1. The message shape and the slug comparison are copied, and case 14 pins the code. **Accepted: this will not be a Phase 7 rejection.** | No change; F2 removes the copies. At Phase 7 I check three things: it is the only producer of `PaneInOtherProfile` in `crates/holler-cli/src`; it runs for a `Set` only; and it keeps the hub's `"{} is in profile {:?}, not {:?}"` text. |
| W-6 | warn | C6, C7; F3 and F4 | ADRs / contract docs | Two frozen trait docs will contradict the shipped code, and no follow-up names them:<br>- `Prober` (`ports.rs:204-207`): "It returns within the `timeout` it is given".<br>- `ProfileScope` (`profile.rs:377-379`): "Every method returns within I5's bound (default 10 s) or with `PaneError::Timeout`".<br>F4 covers only `lib.rs`, `Cargo.toml` and the test rename. Keeping `holler-pane` out of this change is right (#637 only, amend-first). | Add both doc lines to F4 under the amend-first rule, and have B-1 (b)'s ADR sentence cite it. |

Apart from these findings, the plan matches existing patterns. Checked against the code at `89b611f`:

- **No missed reuse candidate.**
  - No production shell-quoting helper exists. The only one is the test-local `sh_quote` in
    `crates/holler-cli/tests/multiword_command_test.rs:63`.
  - No bounded runner or process-group kill exists in `crates/*/src`. `holler-proto`, the one crate `holler-pane` can
    reach, has no process code at all. The adapter crates `holler-adapter-host` and `holler-adapter-opencode` are still
    5-line stubs.
  - So `run_probe`'s private helpers and Decision 8's quoting duplicate nothing.
- **`dead_code` does not fire on a scope nothing wires yet.** `holler-cli/src/lib.rs` has `pub mod pane`, and
  `pane/mod.rs` has `pub mod profile_scope`, so `StoreScope` and `reconcile_step` are public library items.
- **The inline test placement is forced and fits the workspace.** The hub runs its suites from `tests/`
  (`holler-hub/tests/pane_membership_test.rs:31`, `profile_registry_test.rs:56`). But `holler-cli` has `autotests = false`
  and explicit `[[test]]` targets owned by other stories, and the manifest is out of radius. Inline
  `#[cfg(test)] #[allow(...)] // #NNN` modules are the workspace pattern (about 30 of them, including `token_cmd.rs:429`),
  and the test kit is already a dev-dependency.
- **Wiring fits.** `Ports<'_>` lends `&dyn` ports (`ports.rs:227-235`), so #649 holds the stores as `Arc`s and lends
  `&*arc`; `StoreScope::new` does no I/O.
- **Platform.** Windows is off the CI matrix (ADR 0002; `ci.yml` runs Ubuntu and macOS only). `holler-hub/src/control.rs:8`
  already uses `std::os::unix` without a guard, so an unguarded `process_group(0)` adds no new platform limit to the binary.
  `holler-proto` and the test support guard theirs with `cfg(unix)`, so the workspace is mixed; no finding.
- **Naming.** Real ports are `PaneState`, `ProfileState` and `SystemProber`, so there is no dominant pattern and
  `StoreScope` is not drift.
- **Every quoted brief excerpt I spot-checked matches the source:**
  - `ProfileScope`, `ProfileStore`, `Prober` and `Ports`;
  - `PaneError`, including `Refused { code, message }` at `error.rs:491`;
  - the fake at `testkit/src/profile_scope.rs:56-293`;
  - the suite's `pub fn` at `conformance/profile_scope.rs:193`, `lib.rs:31` and `conformance/mod.rs:21`;
  - the fixture functions and the `*StoreOp` enums.
- **Sizes.** Both target files are stubs today (36 and 5 lines); the plan keeps each under 600.

## Notes for O

**Amend the brief, then start a fresh run.** Do not use `resumeFromRunId`, which replays this verdict.

1. **B-1 (required).**
   - **Files:** add `docs/adr/ADR-0021.md` (prose only).
   - **Blast radius and AC 12:** add `docs/adr/ADR-0021.md`, and remove "no ADR" from the blast-radius line.
   - **Decision 22:** "ADR-0021 is amended in this change (the new AC); the test kit is not (F1)".
   - **F3:** drop the items now in the new AC.
   - **New AC (the ADR):** each edit is made in place, is a sentence or two and is cited `(#663)`. None changes the closed
     code list, `class_of` or any table's classes.
     - **(a) Section 1, the `ProbeResult` paragraph (lines 78-80):** the verdict rule of Decisions 14, 17, 18 and 19:
       - `ok` only when the program exited 0 and every `expect` string occurs in its stdout (bytes, case-sensitive);
       - a non-zero exit or an end by a signal is `error`, whatever the output;
       - the deadline passing, or more than 1 MiB of stdout, is `error`;
       - no reason echoes an argv element or an output byte;
       - the argv is never run through a shell.
     - **(b) Section 2, after "every method returns within I5's bound ... or with `timeout`" (line 87):** two narrowings.
       `ProfileScope::edit_spec` is bounded by the sum of its port calls (at most four) plus the verb's own act
       (Decision 11). `run_probe` returns at its deadline plus at most 1 s for the kill and the reap (Decision 15).
       Cite W-6's follow-up for the trait docs.
     - **(c) Section 8, step 2:** a first write that times out may have landed, so the act does not run, and the answer is
       `timeout` with the reconcile step (Decision 6).
     - **(d) Section 8, steps 5-6:** a restoring write that fails other than by a conflict keeps its own code, and its
       message names P, the unrestored edit, the act's error and the reconcile step (Decision 5). The step-6
       `profile-conflict` carries the reconcile step in its message (Decision 7).
     - **(e) Section 8, step 6 (and the "fence" bullet at lines 273-275):**
       - the reconcile step's exact text is `to reconcile, run holler pane doctor --profile '<P>' and then holler profile show '<P>'`,
         with P POSIX-single-quoted;
       - it is profile-scoped until #647 gives `pane doctor` a pane positional (Decision 8, C9);
       - the scope's own errors carry the step, and a verb appends it only to errors that do not (the rule #644 plans
         against, its C-15);
       - name where the unscoped form lives (W-1).
   - **Merge hygiene:** #644, #647 and #662 also edit ADR-0021 now. Edit sentences in place, not whole sections, so a rebase
     conflict stays one hunk.
   - **Alternative:** a separate `docs(adr)` PR with (a) to (e), merged before the fresh run. The AC then checks the merged
     text instead.
2. **W-1:** in Decision 8, either host `pub const RECONCILE_STEP_UNSCOPED` (with #644's exact text) beside `reconcile_step`,
   or name `launch::RECONCILE_STEP_UNSCOPED` as the one #646 imports. Then tell #644's run which one it is.
3. **W-2:** reword F1 to route (a) or route (b).
4. **W-3:** in Decision 5, require an exhaustive match with no `_` arm and a rustdoc note on `Timeout.op`, and add the
   helper to F2.
5. **W-4:** in Decision 20, split the mechanics from the verdict and list the three #696 points in the rustdoc.
6. **W-6:** add `ports.rs:204-207` and `profile.rs:377-379` to F4.
7. **W-5:** no brief change.

**What Phase 7 will check:**

- `PaneInOtherProfile` is produced in `crates/holler-cli/src` only by the scope's `Set`-only helper, and uses the hub's text
  shape.
- `profile_scope.rs` holds exactly one reconcile-step formatter (two forms if W-1 is taken), one quoting function and one
  payload-append helper. There is no copy of the fake's helpers beyond the deliberate re-implementation the Reuse map names.
- `probe.rs` adds no public item. It spawns only `argv[0]` and `kill`, and uses no `libc` or `unsafe` (AC 9 and 11). If W-4
  is taken, the mechanics are separate from the verdict.
- The ADR diff touches only the sentences the new AC names.

## Patterns referenced

- `docs/adr/ADR-0021.md`: sections 1 (lines 78-80), 2 (86-98), 5 (176-192), 8 (264-305), 9 (321-329, 352-374) and 12
  (455-460).
- `crates/holler-pane-testkit/src/profile_scope.rs` (the reference behaviour), `crates/holler-pane-testkit/src/pane_store.rs:220-240`
  and `crates/holler-hub/src/panes/store.rs:338-359` (the membership rule), and
  `crates/holler-hub/tests/pane_membership_test.rs:31-47` (how a real implementation runs a suite).
- `crates/holler-pane/src/error.rs:392-491` (the payload naming convention), `ports.rs:202-235` and `profile.rs:373-404`.
- The ADR-in-the-same-change precedent:
  - the #683 Phase 3 review (`a3e47a4:docs/handoffs/683/handoff-A.md`) and the #688 review
    (`db4cba8:docs/handoffs/688/handoff-A.md`);
  - the bodies of PRs #692 and #697;
  - commits `2a6f349` (#639) and `3f9fbf2` (#676);
  - the epic's hot-spot list (#633, line 195) and #634 (closed).
- The consumer contract: `origin/issue-644-implementation:docs/handoffs/644-brief.md` at `7195993` (I-3, C-15, C-16 and the
  `launch.rs` API), #644's review (`d2636ba:docs/handoffs/644/handoff-A.md`, findings 4 and 10), and issue #696.
