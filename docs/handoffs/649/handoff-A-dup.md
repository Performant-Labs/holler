# Handoff-A-dup: Phase 8 - #649 hub wiring and the scratch integration scenario  (anti-duplication gate)

**Date:** 2026-10-11 (16:07 MDT)
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`)
**Diff base:** `5d366b9` (the merge base with `origin/main`)   **Diff head:** `98280ad`
**Reuse map:** no separate file. As in every A pass of this run, the brief's D11, D24, D31 and D32, with Evidence E1 to E70 (`docs/handoffs/649-brief.md`).
**Verdict:** PASS

This is the second A-dup pass. The first (`9cafe0d`, diff `5d366b9..c13400a`) passed. S then returned a test-only
REWORK (`d387625`), and T's fifth pass made it (`98280ad`).
- **This cycle's diff is `c13400a..98280ad`.** In `crates/` it is one file,
  `crates/holler-cli/tests/pane_integration/fake_hub.rs` (+86 −57). The rest is pipeline files under `docs/handoffs/649/`.
- No production line, ADR, CHANGELOG or manifest changed.

**`origin/main` moved after the first pass.** It is now `652841b`: #832 (PR #836) merged at 15:35 MDT, nine minutes
after the first A-dup pass.
- A trial merge (`git merge-tree --write-tree origin/main HEAD`, tree `112bb17`) conflicts in `CHANGELOG.md` only. So
  the branch must take `main` before its PR can merge.
- `ADR-0021.md` and `src/pane/wiring.rs` auto-merge.
- What #832 brings in is W-5.

## Summary

**PASS. This cycle's change builds no parallel path, and it removes a duplicate.**

- **T folded two copies into one.** `each_write` and `each_read` each declared the same `run` closure: start a fake
  hub, run the call against it, check the method sent. They are now one runner, `against` (`fake_hub.rs:211-225`), over
  two tables, `writes()` and `reads()` (`:159-184`).
- **The new pieces have no analogue to extend.**
  - `Outcome` (`:187-207`) is one client call's error code, text and fake hub state dir. `verb_harness::Outcome`,
    which this target also includes, is a verb run's exit code and writers: a different job.
  - `assert_names_the_hub` pins the second arm of `hub_unavailable` (`store.rs:387`). No other test pins that text.
  - `reads()` and `writes()` are the only per-method call tables in the CLI's tests. They do not reuse the production
    `Method` table (`store.rs:71-82`), and that is right: a test that took its rows from that table could not catch a
    row missing from it.
- **The first pass's findings stand unchanged:** W-1 to W-4, and W-O (three `LinkedHost`s, no fourth).

**Two new warns.**
- **W-5 is binding at the merge with `origin/main`.** #832 added `HerdrPort::observe_pane` and `observe_panes`, both
  with default bodies, and `main`'s ADR-0021 section 14 says "A wrapper port must delegate it".
  - #649's two Herdr wrappers, `BindingHerdr` and `LazyHerdr`, exist only on this branch, so #832 could not update
    them. After the merge, both answer from the defaults: an empty observation for every pane, and never
    `pane-not-found`.
  - In production, `ports.herdr` is `BindingHerdr` over `LazyHerdr` over the adapter. So under the real wiring, #832's
    `pane list` and `pane get` would show no working directory for any unmanaged pane.
  - CI on the merged tree catches half of this. A fix to `BindingHerdr` alone passes CI and still masks the adapter.
  - It is not a parallel path, so it is not a phase 7 block (why: `decisions.md`, Hedged).
- **W-6 is a nit:** the new no-hub row spells out `no_hub.rs`'s check again.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-5 | **warn, binding at the merge with `origin/main`** (O; then F and T) | `crates/holler-cli/src/pane/wiring/bind.rs:97-128` (`BindingHerdr`) and `:201-229` (`LazyHerdr`) | **After the merge, both Herdr wrappers answer #832's two new methods from the trait's defaults.**<ul><li>On `main`, `HerdrPort` has `observe_pane` and `observe_panes`, each with a default body (`ports.rs:196-220`). ADR-0021 section 14 says "A wrapper port must delegate it" (line 913). #832 added an `observe_pane` forward to every delegating port on `main`: `ExtraHerdr`, `DisplacingHerdr`, and the conformance test's `GuardWatcher` and `Mutant`.</li><li>#649's two wrappers are not on `main`, and the trial-merged `bind.rs` has neither method. Both would also contradict their own docs: `bind.rs:11` and `:84` ("forward every method"), and `:175` ("every method answers its error").</li><li>The production chain is `BindingHerdr` over `LazyHerdr` over `HerdrAdapter` (`wiring.rs:140-152`). `unmanaged::observe` calls `ports.herdr.observe_panes` (`unmanaged.rs:108` on `main`), which never reaches the adapter's one-snapshot read (`adapter.rs:409-447`). So every unmanaged pane shows no working directory, and `observe_pane` never answers `pane-not-found`.</li><li>**What CI catches on the merged tree.** `bind::the_binding_herdr_passes_the_herdr_conformance_suite` (`tests/pane_integration/bind.rs:293`) fails at the new case 13: "expected `pane-not-found`, but it succeeded".</li><li>**What it does not catch.** No suite runs over `LazyHerdr`, and `LazyHerdr`'s every-method test (`:134`) lists the seven methods by hand. No case covers `observe_panes`. So a fix to `BindingHerdr` alone passes CI, while production still masks the adapter through `LazyHerdr`.</li></ul> | <ul><li>Merge `origin/main`.</li><li>Give both wrappers both methods:<ul><li>`observe_pane` forwards. In `LazyHerdr` it is `self.port()?.observe_pane(pane)`.</li><li>`observe_panes` forwards to the inner port's own `observe_panes`, which keeps the ADR's "one call". When `LazyHerdr`'s build has failed, it answers one empty observation per pane, the trait's contract for a failed call.</li></ul></li><li>Pins, for T:<ul><li>`observe_pane` joins the method list of the test at `:134`;</li><li>the Herdr conformance suite runs over `LazyHerdr` too, so case 13 pins it;</li><li>one recording check per wrapper that `observe_panes` reaches the inner port's `observe_panes` once, not `observe_pane` once per pane.</li></ul></li></ul> |
| W-6 | warn (nit, T; not binding) | `crates/holler-cli/tests/pane_integration/fake_hub.rs:349-350` | **The new no-hub row spells out `no_hub.rs`'s check again.**<ul><li>It builds `"no live holler hub reachable at <state dir>"` inline.</li><li>`no_hub.rs` already has `NO_HUB` (`:47`) and `names_the_missing_hub` (`:116-118`), both private. `watch_with_no_hub_is_unavailable_from_watch_itself` (`fake_hub.rs:423`) has a third spelling.</li><li>All three check one text: `hub_unavailable`'s first arm (`store.rs:386`), which `connect`'s `reach_hub` also uses (`store.rs:118-122`).</li><li>Not binding. It is a two-line check in a sibling module, not one of the overlay's helpers, and the inline form is the strictest of the three: it also pins "at" before the dir.</li></ul> | Move `NO_HUB` and one predicate, in the strict form, to `main.rs` beside `with_state_dir`, and use it in all three places. It can join the test-glue consolidation (W-O). |
| W-1 to W-4 | warn (low or nit; not binding), carried | as in the first pass (`9cafe0d`) | **Unchanged by this cycle:**<ul><li>W-1: a third `alive`;</li><li>W-2: `tmux_on_path`;</li><li>W-3: `trap_dir`, and the inline `sessions_for(&[])`;</li><li>W-4: the third `Feed` shape.</li></ul> | As in the first pass. |
| W-O | warn (carried from phase 3) | `crates/holler-cli/tests/pane_integration/bind.rs:422` | **Still three `struct LinkedHost`s, and no fourth:** `pane_integration/bind.rs:422`, `pane_verbs/launch/rig.rs:663` and `profile_verbs/apply/rig.rs:636`.<br>The consolidation is still unfiled: `gh issue list --search LinkedHost --state all` finds only #649 and the closed #802. | O files it (Notes for O). |

### Checked and clean (this cycle)

- **The fold.**
  - The lines removed are the two `run` closures of `each_write` and `each_read`, which were the same.
  - `against` keeps their method check (the fake hub's first request is the call's method) for every row that uses it:
    the lost write, the lost read, the JSON-RPC error and the data that does not decode.
- **No copy of an overlay helper.**
  - `FakeHub`, `failure`, `with_state_dir` and `support::StateDir` are used, and none is declared again.
  - No polling loop and no `thread::sleep` was added.
- **`Outcome` has nothing to extend.**
  - `verb_harness::Outcome` (`verb_harness/mod.rs:38-44`) is a verb run's `code: i32`, `out` and `err`.
  - `fake_hub` does not import it, so nothing is shadowed.
  - A module's own private `Outcome` is the codebase's pattern: `src/pane/switch.rs:117`, `holler-pane/src/import/assess.rs:51`.
- **`assert_names_the_hub` pins a different text from `no_hub.rs`'s check.**
  - It pins the second arm of `hub_unavailable` (`store.rs:387`: "the holler hub at <state dir> did not answer
    <method>: …"). `no_hub.rs`'s predicate pins the first arm.
  - Its state dir is the fake hub's own fresh temporary path, so neither the transport's text nor the fake's refusal can
    supply it.
- **`reads()` and `writes()` are the right oracle.**
  - The test kit has no per-method wire table: its `FaultSwitch` (`holler-pane-testkit/src/fault.rs`) works at the trait.
  - The production table is private (the `const`s at `store.rs:71-82`, `struct Method` at `:226`). A table of the
    test's own is the right check against it.
- **`type Call` holds `fn` pointers,** so a row captures nothing. Every call still runs under `with_state_dir`, through
  `FakeHub::with` or directly.
- **Build guards.**
  - `fake_hub.rs` is 455 lines.
  - `cargo clippy -p holler-cli --test pane_integration -- -D warnings` is clean.
  - The five reworked tests pass.
- **Blast radius.**
  - The only file changed in `crates/` is in `pane_integration/`.
  - T rewrote its own `handoff-T-green.md` line that quoted the setup wizard's old directory path, and `evidence.md`
    gained T's section. Neither is code.

For the full review of the production code and the tests, see the first pass (`9cafe0d`). Nothing it checked has changed.

## Notes for F

None for this verdict. If O routes the merge through the pipeline, W-5's change is F's (the two wrappers) and T's (the
pins). The steps are in W-5's row.

## Notes for O

1. **W-5, before the PR merges.** The PR cannot merge without `main` (the `CHANGELOG.md` conflict), and W-5 starts at
   that merge.
   - **The conflict.** Both sides add an entry at the top of `## [Unreleased]` / `### Enhancements`. Keep both.
   - **Then W-5's fix and pins, as its row gives them,** before the merged tree is pushed. Fixing only the failing test
     (case 13 over `BindingHerdr`) is not enough.
   - **Better through the pipeline.** Route the merge and the fix through F and T-green, then A-dup and S, so the gates
     review the tree that lands. A should re-check the merge itself: #832 touches 47 files, among them `wiring.rs` (an
     `Unwired::observe_pane`) and ADR-0021 (a new section 14).
   - **Re-run the opt-in scenario on the merged tree.** #832 changes what `pane list`, `pane get` and `pane doctor`
     read.
   - **A visible change, for the PR body.**
     - After the merge, an unscoped `pane list`, and a `pane get` that finds no record, also ask Herdr. This is #832's
       one exception to "the read verbs read the record only" (`list.rs:141` and `get.rs:96` on `main`).
     - Under this wiring with no Herdr configured, both still exit 0 and list the registry. So D2 and AC 2 hold:
       `store_only.rs`'s assertions stay true, by my reading.
     - They add `"unmanaged_unavailable": "usage"` to the JSON. In text mode they print a warning line naming
       `HOLLER_HERDR_SESSION`.
2. **The rename scan still fails CI** (S's item 1).
   - Two of its three hits remain: the brief's E64 Source line (`649-brief.md:1197`) and the first-pass entry in A's
     journal (`decisions.md:66`). T rewrote its own.
   - The journal is append-only, and S gave that rewording to O, so A has not edited its line.
   - The other way out is `removeHandoffsBeforePr: true` for the PR phase.
3. **Still unfiled** (`gh issue list`, 16:07 MDT): the test-glue consolidation (W-O). It can absorb W-1 to W-3 and W-6.
   S's items 2 to 5 are carried as S wrote them.
4. **W-4 is still optional.**

## Patterns referenced

- **This cycle:**
  - `crates/holler-cli/tests/pane_integration/no_hub.rs:47, 116-118` (`NO_HUB`, `names_the_missing_hub`);
  - `crates/holler-cli/tests/verb_harness/mod.rs:38-44` (`Outcome`);
  - `crates/holler-cli/src/pane/wiring/store.rs:71-82, 118-122, 226, 380-389`;
  - `crates/holler-pane-testkit/src/fault.rs` (`FaultSwitch`).
- **W-5, on `origin/main` (`652841b`):**
  - `crates/holler-pane/src/ports.rs:196-220` (the two methods) and `:229` (`PaneObservation`);
  - `docs/adr/ADR-0021.md:901-915` (section 14);
  - `crates/holler-adapter-herdr/src/adapter.rs:409-447`;
  - `crates/holler-pane/src/unmanaged.rs:89-110`;
  - `crates/holler-pane-testkit/src/conformance/herdr.rs:142, 424-447` (case 13);
  - `crates/holler-cli/tests/pane_verbs/import/seams.rs:52` and `pane_verbs/launch/displaced.rs:80` (#832's forwards);
  - the trial merge `112bb17`: `src/pane/wiring.rs:306` (`Unwired::observe_pane`), and a `bind.rs` with neither method.
- **On the branch:**
  - `crates/holler-cli/src/pane/wiring/bind.rs:11, 83-128, 169-229`;
  - `crates/holler-cli/tests/pane_integration/bind.rs:119-159, 293-315`.
