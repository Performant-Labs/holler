# Handoff-S: #846 linkedhost-fakes (spec audit)

**Date:** 2026-10-12
**Branch:** issue-846-implementation
**Issue:** #846
**Handoffs reviewed:** brief (`docs/handoffs/846-brief.md`, amended `e1eae6a`), `handoff-A.md` (PASS), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` (PASS), `handoff-A-dup.md` (PASS), `decisions.md`, `evidence.md`; the diff `origin/main...HEAD` and the new modules read in full.

## A precondition
Met. `handoff-A.md` is PASS (second pass on the amended brief); `handoff-A-dup.md` is PASS.

## T precondition
Met. `handoff-T-green.md` is PASS with "Blocking issues: None". The baseline (T-red) and the after-run record the same per-target counts. This is a characterisation story (brief D6), so the "RED" is the AC 1 duplicate greps (4 + 4 before), not a failing assertion.

## Acceptance criteria

| # | Criterion | Proving evidence | Status |
|---|---|---|---|
| 1 | One definition | Re-run by S: `grep -rnE "HostPort for LinkedHost\|struct LinkedHost" crates/holler-cli/tests` prints only `verb_harness/linked_host.rs:46` and `:51`. `LostChain` is untouched. | Met |
| 2 | Unchanged behaviour (five targets, same counts) | T-green table: pane_claude 43/1 ignored, pane_integration 71/4, pane_verbs 347/0, profile_apply_scenario 5/3, profile_verbs 118/0, identical to the T-red baseline; F's run agrees. The diff adds, removes and edits no `#[test]`, only support code and the `Recording` call-log plumbing (asserted values unchanged, read in the diff). | Met |
| 3 | No production change | S re-ran `git diff origin/main...HEAD -- 'crates/*/src' Cargo.toml 'crates/*/Cargo.toml'`: 0 lines. All code paths are under `crates/holler-cli/tests/`; the rest is `docs/handoffs/`. No CHANGELOG line: precedent #800 (`43703ba`) and #735 (`e9afc8b`) are test-only with none; `changelog-check` passes. | Met |
| 4 | Propagate/swallow kept and named | `OnStart { Propagate, Swallow }` and `OnStop { KillServed, KillNothing }` named in `linked_host.rs` and its module doc. Apply and `bind.rs` build `Propagate`; `FakeHarnessesLink` and launch's `HookedLink` swallow (launch still fires `On::ServerStart` with the started port, as in the removed code); `linked: false` maps to `KillNothing`. Kill-side modes are pinned by T's mutations (27 and 8 failing rows). **`Propagate` is pinned by no row** (T's mutation stayed green). | Met, with a standing gap (see Advisory 1) |
| 5 | Near-copies one each | `grep -rn "opencode-ran"` gives one line (`traps.rs:11`, the `OPENCODE_MARK` const; the writer `opencode_trap()` is shared by `wired.rs` and `no_hub.rs`). `NO_HUB` and `names_the_missing_hub` are defined once in `traps.rs`; `fake_hub.rs` keeps the full `"{NO_HUB} at {state}"` string and the phrase-only check. `CallLog`, `herdr_suite_over`, `unbuildable_lazy_herdr` live in `wrappers.rs` and are used by `bind.rs` and `observe.rs` (neither imports the other). `main.rs` doc lists `observe`. `scratch/`, `out.rs`, `scenario.rs` are not in the diff. | Met |
| 6 | Gates | `lint.sh` exit 0 (T); `wc -l` S-checked: launch/rig 834 (was 852), apply/rig 740 (776), bind 636 (693), pane_claude/rig 453 (482), observe 269 (321); new files 149/74/36; none over 900, none newly over 600. Clippy clean on the five touched targets. The workspace `--tests` clippy fails only at untouched `token_load_test.rs:60` (pre-existing, local clippy 0.1.93). Workspace test exits 0. | Met (pre-existing clippy exception noted) |

## Spec compliance
All MO decisions are implemented as stated: D1 (home is `verb_harness/linked_host.rs`, declared in `mod.rs`), D2 (no testkit change, module doc names the testkit's prescription at `host.rs:12-17`), D3 (four folded), D4 (modes named, not unified), D5 (only W-3, W-6, W-7; W-3b, W-1, W-2, W-4 untouched), D6 (characterisation). F's refinements are all inside the brief's "F may refine names" latitude and answer A's warns:
- `started(argv)` drops the unused `name` parameter.
- `OnStop` carries the port list, so there is no `ports` plus `kill_on_stop` pair and no redundant empty port list.
- The `kill_served` free fn became `OnStop::end`. It is defined once and called from `FakeHarnessLink::stopped` and `HookedLink::stopped` (A's phase-7 check), which satisfies the intent.

The one unmet sentence is the module doc "links the follow-up issue": no number exists yet, because O has not filed it. This is a process gap, not an implementation defect.

## Quality audit
- **Correctness:** `LinkedHost::run` is host-run then `link.started`, and `stop_owned` is host-stop then `link.stopped`, the same order and `?` propagation as the four removed copies. The kill loop's `?` on `kill` is preserved. `Borrow` serves `&FakeHost`, an owned `FakeHost` and `Arc<FakeHarness>` without a `'static` problem.
- **Build guards:** the only `unwrap` added is in test-support code (`opencode_trap`, `herdr_suite_over`), moved from the prior test code and permitted in tests. No `#[allow]` added, no `panic!`, no dead code (S grepped the added lines). All touched files are under 900 lines.
- **Protocol, goldens, docs:** none touched, none required.
- **Tests:** no sleeps added; the real hub/body harness is unchanged.
- **Privacy:** S grepped the added lines for tailnet names, IPs, personal and account names, home paths and keys. The only hit is the word "secret" in a handoff sentence ("No secret handled"). The diff is clean.
- **Commit hygiene:** the pipeline commits carry a `Co-Authored-By` trailer. The PR does not exist yet. O must check its body for the AI disclosure per `CONTRIBUTING.md`.

## Scope check
Delivered exactly the brief's scope. One small addition beyond "one writer": `OPENCODE_MARK` is a const that the readers also use, which serves AC 5's single-occurrence grep. `Recording` and `Observing` lost their thin `note`/`calls` wrappers in favour of `CallLog`, which is the W-7 fold itself. No over-delivery and no under-delivery. The edited `pane_integration` files were not rustfmt-ed, to avoid unrelated churn. That was stated and is reasonable.

## Verdict
**PASS.** All six acceptance criteria are met and verified against the tree, spec decisions are honoured, and the quality gates hold. The run stays on pipeline release 0.1.7, per the operator's instruction that 0.1.8 is for new stories only.

## Advisory notes
1. `OnStart::Propagate` is unpinned by any row (apply and `pane_integration`'s launch pin). It was equally unpinned before the fold. A follow-up could add one apply row over a port another pane's server holds.
2. O to file the D2 follow-up (fold `LinkedHost` into `holler-pane-testkit`). When it is filed, put its number in the `linked_host.rs` module doc. It should also refresh the stale line at `holler-pane-testkit/src/harnesses.rs:9-12` ("keep their own linking").
3. Before merge, check the PR body for the AI disclosure (`gh pr edit`). `cargo machete` was not run locally and is covered by CI. The sibling #845 and #848 merge ordering from the brief still applies.
