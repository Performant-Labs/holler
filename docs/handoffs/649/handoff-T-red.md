# Handoff-T-red: Phase 4 - #649 hub wiring and the scratch integration scenario  (D33 cycle: #832's observe methods)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `6e20075`, which merged `origin/main` at `652841b` (#832) in `63b4afa`, plus this phase's staged changes)
**Brief / wireframe reviewed:** `docs/handoffs/649-brief.md` at `6e20075` (D33 added after S's ADVISORY-HOLD); `handoff-S.md` (second pass, defect 1 and its proposed pins); `handoff-A-dup.md` (W-5). No wireframe (no UI surface).

This handoff replaces the second cycle's (`ce5da11`), which git history keeps. Every test those cycles authored still stands. On the merged tree, all of them pass except `bind::the_binding_herdr_passes_the_herdr_conformance_suite`, which #832's case 13 now breaks (row 2 below).

## A precondition

- **A's last plan verdict is PASS** (fifth pass, `2280399`). A-dup's second pass is PASS too, and its W-5 is the text D33 adopts.
- **No A pass has run since O's amendment.** The amendment adds D33, corrects E64's Source, and adds #832 to Depends on. O's journal entry says the run resumes at this phase.
- **D33 adds no design.** It is A-dup's W-5 as A-dup worded it, with the pins S proposed. I therefore took A's PASS as standing and did not stop.

## Tests authored or changed

All paths are under `crates/holler-cli/tests/pane_integration/`. Every test below runs in the default run.

| # | Test (module::name) | D33 behavior pinned | Tier, and why |
|---|---|---|---|
| 1 | `observe::the_lazy_herdr_passes_the_herdr_conformance_suite` **(new, RED)** | The Herdr conformance suite runs over `LazyHerdr` (built over `FakeHerdr`), so #832's case 13 covers it. | In-process, over the test kit's fake: the same form as the existing `BindingHerdr` run (W-12). |
| 2 | `bind::the_binding_herdr_passes_the_herdr_conformance_suite` (unchanged; **RED on the merged tree**) | The same suite over `BindingHerdr`. Case 13 fails it now. | Unchanged. |
| 3 | `bind::a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method` (**one row added, RED**) | `observe_pane` joins the method list: when the build failed, it answers the stored `usage` naming `HOLLER_HERDR_SESSION`. | Unit-level over a failing build closure. |
| 4 | `observe::the_binding_herdr_delegates_observe_pane_and_observe_panes` **(new, RED)** | The recording check for `BindingHerdr`. `observe_pane` answers the inner port's observation, not the default empty one. `observe_panes` of three panes answers the inner port's three, from one inner `observe_panes` call and no inner `observe_pane` call (section 14's one call). | In-process, with a recording inner `HerdrPort` (`Observing`) whose answers the defaults never produce. |
| 5 | `observe::the_lazy_herdr_delegates_observe_pane_and_observe_panes` **(new, RED)** | The same check for `LazyHerdr`. | Same. |
| 6 | `observe::the_wirings_herdr_chain_delegates_observe_panes_end_to_end` **(new, RED)** | The same check over production's chain, `BindingHerdr` over `LazyHerdr` (`Wiring::connect`). The adapter still gets one `observe_panes`. | Same. It catches a fix applied to only one link. |
| 7 | `observe::a_lazy_herdr_that_cannot_be_built_observes_nothing_for_each_pane` **(new, RED)** | When the build failed, `observe_panes` answers one empty observation per pane (the trait's contract for a failed call), answers `[]` for no panes, and the build ran once and was not retried. | Unit-level over a failing build closure. |
| 8 | `store_only::the_store_only_verbs_work_with_no_herdr_tmux_or_opencode` (**rows added; passes at HEAD; a pin, not a RED**) | D33's last sentence. With no Herdr configured, `pane list --format=json` still exits 0 with the record and carries `"unmanaged_unavailable": "usage"`. Text `pane list` exits 0 and warns `unmanaged panes not listed`, naming `HOLLER_HERDR_SESSION`. | Process-level against a real hub, in the existing test. That is the only tier that sees the real wiring's `LazyHerdr` failing its build. |

**Support:** `main.rs` declares `mod observe;`. The new tests live in their own module rather than in `bind.rs`, which is 580 lines (lint warns at 600). `observe.rs` is 221 lines.

**Evidence:** `evidence.md` has a new top section. It lists the unchanged #832 source these tests rely on: the two trait defaults (`ports.rs:205-220`), ADR section 14 (`ADR-0021.md:972-976`), case 13 and the fake's `observe_pane`, and `pane list`'s `unmanaged_unavailable` and warning. Each excerpt is copied from source.

## RED confirmation

Command: `cargo test -p holler-cli --test pane_integration` (default run, merged tree, production code untouched).
Result: `test result: FAILED. 59 passed; 7 failed; 4 ignored; 0 measured; 0 filtered out; finished in 119.92s`.

The 7 failures are exactly the D33 rows. Each fails on its assertion, and each failure is the trait default answering in place of the inner port:

```text
---- bind::a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method ----
panicked at bind.rs:155:27: every method fails            # observe_pane answered Ok(default), not the stored usage

---- bind::the_binding_herdr_passes_the_herdr_conformance_suite ----
  left: Err([CaseFailure { case: "observe-pane-answers-a-listed-pane-and-refuses-an-unknown-one",
        detail: "observe_pane(w999:p999), an id never handed out: expected `pane-not-found`, but it succeeded" }])
---- observe::the_lazy_herdr_passes_the_herdr_conformance_suite ----
  left: Err([CaseFailure { case: "observe-pane-answers-a-listed-pane-and-refuses-an-unknown-one",
        detail: "observe_pane(w999:p999), an id never handed out: expected `pane-not-found`, but it succeeded" }])

---- observe::the_binding_herdr_delegates_observe_pane_and_observe_panes ----   (and the LazyHerdr and chain rows, identically)
assertion failed: BindingHerdr: observe_pane answers the inner port's observation, not the default empty one
  left: Ok(PaneObservation { cwd: None, last_screen_change: None })
 right: Ok(PaneObservation { cwd: Some("/observed/w1:p9"), last_screen_change: Some(1000) })

---- observe::a_lazy_herdr_that_cannot_be_built_observes_nothing_for_each_pane ----
assertion failed: the failed build is kept, not retried
  left: 0
 right: 1                                                   # observe_panes never reached the port: it did not delegate
```

None of the failures is a compile error, a missing `[[test]]` or a timeout.

**The one-call half discriminates on its own.** I ran a mutant locally that forwarded `observe_pane` in both wrappers but left `observe_panes` on the default. The three recording rows then fail on the call log, `left: ["observe_pane", "observe_pane", "observe_pane"]` against `right: ["observe_panes"]`, and every other row passes. So a fix that forwards only `observe_pane`, which still turns case 13 green, is caught.

**The tests can be satisfied.** I applied D33's fix as written as a local trial: both methods forwarded, and `LazyHerdr::observe_panes` answering `vec![PaneObservation::default(); panes.len()]` on a failed build. With it, `-- observe bind:: store_only` gave 24 passed and 0 failed. I then restored `src/pane/wiring/bind.rs` from a scratchpad copy. `git status` shows no change under `src/`.

**Other checks, at this phase's tree:**
- `cargo clippy -p holler-cli --all-targets -- -D warnings`: clean.
- `bash scripts/lint.sh`: exit 0. Its only warnings are the existing size warnings in other crates.
- `observe.rs` is formatted with `rustfmt --edition 2021` on that file alone, not with a workspace `cargo fmt`, per this run's lesson.

## Notes for F

- **Code:** `crates/holler-cli/src/pane/wiring/bind.rs`. Override `observe_pane` and `observe_panes` in `impl HerdrPort for BindingHerdr` and in `impl HerdrPort for LazyHerdr`, as D33 says.
  - `observe_panes` forwards to the inner port's `observe_panes`. Do not loop over `observe_pane`: rows 4 to 6 pin one call.
  - In `LazyHerdr`, a failed build answers one `PaneObservation::default()` per pane, and the build is attempted through `port()` (row 7 expects the build count to be 1).
  - Update the module and type docs, which say every method forwards, if they list the methods.
- **No test of this phase needs a production rename.** The tests use only `BindingHerdr::new`, `LazyHerdr::new` and `Bindings::new`, which already exist.
- **T-green owes the opt-in scenario re-run on the merged tree** (S's step 4). #832 changes what `pane list`, `pane get` and `pane doctor` read. S's advisory 4 also asks that check to confirm the scenario's own panes never appear in the unmanaged table.

## Ready for F

**RED is valid.** On the merged tree, seven tests fail on D33's assertions against HEAD: the five new `observe::*` rows, the new row in the `LazyHerdr` every-method test, and the `BindingHerdr` conformance run. The rest of the target (59 tests, including the new `store_only` pin) passes and must stay green.

`T-red complete, RED is valid. F may implement against the authored tests.`
