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

## A (up-front plan review, round 2) — 2026-10-09T20:14-06:00
- **Decided:** PASS on the brief as amended in `8cf4f00` (handoff-A.md, which replaces round 1's; round 1 stays in git at
  `2bbb237`).
  - Round 1's block is fixed by option (a). `doctor.rs:10` becomes `pub(crate) mod rig;`, and `switch.rs` and `reset.rs`
    reach the rig as `crate::doctor::rig`, the `profile_verbs` pattern on `main`. I verified it on a built copy of the tree.
  - All six round-1 warns are written into the brief: P1, P3 and O1 wording, R-5, F-3, F-4, Decision 17 in section 8,
    AC 22 and AC 24, and the Forward-compat rows. The amendments add no drift.
  - Two new warns, neither blocking:
    1. O1's private copy of `screen_text` has no follow-up to fold it. Widen F-4 to cover the screen wording.
    2. Decision 17(c)'s "Deferred" bullet credits #644 if #645 lands first. Write "#644 to follow" in that case.
- **Assumed:**
  - The sibling plans are the ones on their branches now: `issue-644-implementation` at `7195993`,
    `issue-646-implementation` at `b996a8f`, and `issue-663-implementation` at `e46b427` (its A gate passed). #647 part 2
    is not briefed in the repo, and still owns `tests/pane_verbs/doctor.rs` and `reconcile*.rs`.
  - `origin/main` moved from `dc300ab` to `e327569` during the review. That commit (#641) touches only
    `holler-adapter-host`, `Cargo.lock` and `CHANGELOG.md`, so the brief's evidence holds.
- **Hedged:**
  - Both new findings are warns. The `screen_text` copy is justified in writing, which is a PASS at Phase 7, and round 1
    accepted it. The bullet wording is a matter of accuracy in the standing spec, not a contradiction, because the rule
    stated in section 8 is general.
  - Giving F-3 a "before #654" ordering, as P2 does for 645b, is offered to O as optional, not raised as a finding.
  - That P0 to R in one function would exceed `cognitive_complexity` 15 is an estimate. It is passed to F as a note.
- **Evidence:**
  - The whole amended brief (1,253 lines), the deleted lines of the amendment diff, round 1's handoff, and the outside
    brief review r1.
  - A scratch copy of `HEAD` built outside the repo, in its own seeded `target/`, with the one-word edit and probe tests in
    both verb files:
    - `cargo clippy -p holler-cli --test pane_verbs -- -D warnings` was clean.
    - 36 tests passed (the probes and all of doctor's).
    - The probes confirmed that the rig's items can be reached from a sibling file, AC 2's call-log sequences, that P's
      server lists Q's session (AC 4), and the single `stray-session` after a reset (AC 16).
  - Source read for this round:
    - `tests/pane_verbs/{main,doctor}.rs`, `doctor/rig.rs`, and `tests/profile_verbs/{main,list,show}.rs`.
    - `findings.rs` (the remedy table, `doctor_command`, `quoted`), `reconcile.rs` (`shown_differs`, `resolve`) and
      `reconcile/observe.rs:286-391`.
    - `error.rs` (`class_of`, `excerpt`, the variants), `lib.rs`, `pane.rs` (`PaneName`, `PaneId`), and the empty
      `tx_apply.rs` and `tx_launch.rs` stubs.
    - `holler-cli/src/pane/doctor.rs`, `output.rs` (its public items), and `cli.rs` and `profile/apply.rs` (the existing
      bool flags).
    - `holler-hub/src/panes/store.rs:160-182, 330-352`, `Cargo.toml` lints and `clippy.toml`.
    - ADR-0021: section 8, rows 336-346, section 11, "Deferred" and "Decisions taken". ADR 0003, lines 48-62.
    - `cli-surface.txt`'s #645 and #647 blocks.
  - The sibling briefs, grepped for codes, failure types, the reconcile step and the orchestrator and unhealthy handling:
    644 (`TxFailure`, `LaunchRequest`, decision 20), 646, 643, and 663 (decision 8, follow-ups F1, F2 and F5).

## T (author / RED) — 2026-10-09T20:23-06:00
- **Decided:**
  - RED is valid (handoff-T-red.md).
  - 19 behavior tests cover ACs 1-19: 13 in `tests/pane_verbs/switch.rs` and 6 in `reset.rs`. Each runs in both formats
    on fresh doctor rigs, reached as `crate::doctor::rig`.
  - The shared runner `both` asserts on every run that the exit code is the same in both formats, that the envelope is
    valid, that each format writes one line, and that there is no Herdr or host call (I4). The same runner serves AC 2's
    call sequences.
  - All 19 fail on the verb's answer: `not-implemented` with exit 1, where the test expects 0, 2 or 3, or another code.
  - AC 23 (help) passes in RED, because the brief puts the real `Args` structs in RED. It fails on `origin/main`.
  - ACs 20-22 are edited in RED, and their existing suites pass.
  - The `doctor.rs:10` edit is visibility only.
- **Assumed:**
  - The brief's test plan overrides the role doc's "T writes no production code" for the RED skeleton. That plan was
    reviewed by A and passed. The skeleton is the `tx_switch.rs` API with `not-implemented` bodies, and the two `Args`
    structs.
  - The CLI `run` bodies still emit `not_implemented(645)` instead of calling the stub engine. Calling it would mean
    writing step 1's parsing and `emit_outcome`, which are F's behavior. `Verb` and `emit_outcome` are left out because
    `dead_code` is denied.
- **Hedged:**
  - AC 1 sets `last_observed.driven` to `ses_driven` before the run (by `rig.rewrite`), so that "driven kept" is
    observable. The brief's literal case has `driven: None`, which no implementation could fail.
  - AC 17's session list is read through Q's server, which shares the data directory. P's killed server cannot be listed.
  - AC 5 also pins the P3 message's `run holler pane relaunch demo-c1r1`. AC 6 pins `--as-operator` in the refusal. Both
    strings are from the brief's Behaviour table.
- **Evidence:**
  - The brief (all of it) and handoff-A round 2.
  - The test-side sources: `doctor/rig.rs`, `verb_harness/{mod,parse}.rs`, the test kit's `harness.rs`, `envelope.rs`,
    `fault.rs` and `pane_store.rs`, and `fixture::sample_pane`.
  - The `holler-pane` sources: `findings.rs` and the `error.rs` variants.
  - The process suites: `process/{stub,flags,docs_rows}.rs`.
  - The commands, with their results:
    - `cargo test -p holler-cli --test pane_verbs`: 94 passed, 19 failed, each on the line 207, 212 or 221 assertions.
    - `pane_cli_process`, `cli_surface_test` and `docs_cli_test`: all green.
    - `cargo clippy --workspace --all-targets -- -D warnings`: clean.
    - rustfmt `--check`: clean. `scripts/lint.sh`: exit 0.
    - The real binary takes each AC 12 argv and answers `not-implemented`, not a clap error.
