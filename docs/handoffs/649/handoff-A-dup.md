# Handoff-A-dup: Phase 8 - #649 hub wiring and the scratch integration scenario  (anti-duplication gate)

**Date:** 2026-10-11 (17:04 MDT)
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`)
**Diff base:** `c68e34e` (S's second pass)   **Diff head:** `c35916b` (T-green, D33 cycle)
- The cycle includes the merge `63b4afa` of `origin/main` at `652841b` (#832). The story's own work after the merge is
  `63b4afa..c35916b`.
- The branch's merge base with the local `origin/main` is now `652841b`.
**Reuse map:** no separate file. As in every A pass of this run, the brief's D11, D24, D31, D32 and now D33, with
Evidence E1 to E70 (`docs/handoffs/649-brief.md`).
**Verdict:** PASS

This is the third A-dup pass. The second (`c001eeb`) passed with W-5 binding at the merge with `main`. S then held
(`c68e34e`), and O merged `main`, added D33 (W-5, verbatim, with S's pins) and resumed the run at T-red.

## Summary

**PASS. F extended the two objects D33 names, in place, and built no parallel path.**

- **Production: one file, `crates/holler-cli/src/pane/wiring/bind.rs` (+35).**
  - `BindingHerdr` and `LazyHerdr` each gain `observe_pane` and `observe_panes` (`:137-143`, `:248-257`).
  - Docs: `:15-20`, `:90-92` and `:188-193`.
  - There is no new type, helper, module, dependency or signature.
  - The production chain is unchanged: `BindingHerdr` over `LazyHerdr` over the adapter (`wiring.rs:141-152`).
- **Section 14's "one call" survives the chain.** Both wrappers send `observe_panes` to the inner `observe_panes`, never
  to the trait default that asks once per pane (`ports.rs:210-220`).
- **The merge adds nothing the wiring duplicates.** `--remerge-diff` shows one conflict, `CHANGELOG.md`, resolved by
  keeping both entries. #832 changed only `HerdrPort` among the ports, so no other wrapper owes a forward.
- **D33 as a plan.** No Phase 3 pass ran on the amendment, so this pass covers it. D33 is A-dup's W-5 word for word, with
  S's pins, and agrees with ADR-0021 section 14 and D31 W-4. No drift.
- **One new warn, W-7 (nit, T; not binding).** The new test module `observe.rs` copies three pieces of `bind.rs`
  instead of sharing them.
- **`main` moved again.** `853c1e1` (#838) merged at 16:33 MDT, eleven minutes after this cycle's merge, and conflicts
  in `CHANGELOG.md` only. It needs no code fix (Notes for O 1).

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| W-7 | warn (nit, T; not binding) | `crates/holler-cli/tests/pane_integration/observe.rs:27-55`, `:62-81`, `:198-205` | **`observe.rs` copies three pieces of its sibling `bind.rs`.**<ul><li>**The conformance run.** `the_lazy_herdr_passes_the_herdr_conformance_suite` (`:27-55`) repeats `bind.rs:293-316`: the same fixture (`FakeHerdr::new("scratch").with_workspace("scratch", 2, 1)`), the same label hook and the same assertion. Only the wrapper's constructor differs. The house pattern folds repeated runs of a suite through `scratch_fake()` and `fixture(port)` (`holler-pane-testkit/tests/herdr_conformance_test.rs:50-62`).</li><li>**The call log.** `Observing`'s `calls: Mutex<Vec<&'static str>>` and `note` (`:62-81`) repeat `Recording`'s (`bind.rs:164-178`). The recording port itself is justified (Checked and clean); only its log is a second copy.</li><li>**The unbuildable `LazyHerdr`.** The counting closure that answers `usage` naming `HOLLER_HERDR_SESSION` (`:198-205`) repeats `bind.rs:135-142`.</li><li>`main.rs`'s module doc (`:9-18`) lists every module but `observe`.</li></ul>**Why not a block.** This is test glue local to one target. It is not one of the overlay's named helpers, and not a guard (D11). It is the same class as W-1 to W-4 and W-6. T's reason for the new module is sound: `bind.rs` is 580 lines, and lint warns at 600. | <ul><li>One generic helper called by both suite runs, e.g. `fn assert_passes_the_herdr_suite<H: HerdrPort>(wrap: impl Fn(Arc<dyn HerdrPort>) -> H)` in `observe.rs`. This also shortens `bind.rs`.</li><li>One `CallLog` in `main.rs`, used by `Recording` and `Observing`.</li><li>One `unbuildable_lazy_herdr() -> (LazyHerdr, Arc<AtomicUsize>)`.</li><li>Add `observe` to `main.rs`'s module doc.</li><li>It can join the test-glue consolidation (W-O).</li></ul> |
| W-1 to W-4, W-6 | warn (low or nit; not binding), carried | as in the first and second passes | **Unchanged by this cycle:**<ul><li>W-1: a third `alive`;</li><li>W-2: `tmux_on_path`;</li><li>W-3: `trap_dir`, and the inline `sessions_for(&[])`;</li><li>W-4: the third `Feed` shape;</li><li>W-6: the no-hub text spelled three ways.</li></ul> | As in the earlier passes. |
| W-O | warn (carried from phase 3) | `crates/holler-cli/tests/pane_integration/bind.rs:423` | **Still three `struct LinkedHost`s, and no fourth.** The consolidation is still unfiled: at 17:04 MDT, `gh issue list --search LinkedHost --state all` finds only #649 and the closed #802. | O files it (Notes for O 2). It can absorb W-1 to W-3, W-6 and W-7. |

W-5 (the second pass's binding warn) is **closed**. Both wrappers now forward both methods, with all three of its pins,
plus a recording check over the production chain.

### Checked and clean (this cycle)

- **F's change extends, and adds no second path.**
  - Every method of each wrapper still reaches the one inner port: `self.inner` in `BindingHerdr`, and `port()`, the one
    build path, in `LazyHerdr`. So the build still runs once, and a failure is kept.
  - **The failed-build branch has nothing to reuse.** That branch is `vec![PaneObservation::default(); panes.len()]`.
    - The trait default's `unwrap_or_default()` works per pane, over a port that exists.
    - The adapter's own failure branch (`adapter.rs:431-447`) is private, and it answers the same
      `PaneObservation::default()` per pane.
    - The branch is the trait's contract for a failed call (`ports.rs:210-213`).
- **No other wrapper owes a forward.**
  - `git diff 5d366b9..652841b -- crates/holler-pane/src/ports.rs` adds two `HerdrPort` methods and `PaneObservation`, and
    nothing else. So these are unaffected: `BindingHost`, `Wired<dyn HostPort>`, `Wired<dyn HarnessPort>`, the store
    clients and `StoreScope`.
  - The workspace has 11 `impl HerdrPort` blocks. The wiring's two are the only production wrappers.
- **`Observing` is justified, not a copy of the test kit's call log.**
  - `FakeHerdr` overrides only `observe_pane` (`herdr.rs:338`), and `HerdrOp` has no `ObservePanes` (`herdr.rs:56-66`).
  - So `faults().calls()` logs N `ObservePane` entries whether a wrapper forwards `observe_panes` or falls to the
    default. It cannot pin section 14's one call.
  - T-red's mutant, which forwarded only `observe_pane`, failed only the recording rows.
- **The chain test builds production's chain by hand** (`observe.rs:186-192`). This is the only in-process form:
  `from_env` builds the real `HerdrAdapter` and takes no injected port.
- **The merge `63b4afa`.**
  - `--remerge-diff`: the `CHANGELOG.md` conflict only, with both entries kept (#649, then #832).
  - `wiring.rs` auto-merged: `Unwired::observe_pane` (`:306-308`, #832's) and `PaneObservation` in the import.
  - That import line (`:76`) is #832's over-width line. It is not rustfmt-clean on `main` either, and CI runs no
    rustfmt. This is linter-level, not drift. I agree with F's design decision 18.
- **ADR.**
  - Section 14 already says "A wrapper port must delegate it" (`ADR-0021.md:974`).
  - "Wiring as built (#649)" (`:321-372`) lists no port methods.
  - No ADR change is owed. I agree with F's design decision 17.
- **D32's checks, re-run on the merged tree:**
  - no `TRIAL` header and no print in the wiring;
  - no `"HERDR_SESSION"` read;
  - no `UnixStream` in `holler-cli/src`, and `control::run` only through `transport.rs:35`;
  - the manifest comment (`Cargo.toml:402-404`);
  - the three `#[path]` includes (`scratch/mod.rs:38-44`);
  - no copy of the rig, the guards, `Hub`, `StateDir` or `wait_for`.
- **The other test edits add no helper.**
  - `scenario.rs:255-273` (S's advisory 4) uses `Scratch::json`.
  - `store_only.rs:68-77` (D33's last sentence) uses `Bare::holler`.
- **Build guards.**
  - `cargo clippy -p holler-cli --test pane_integration -- -D warnings` is clean.
  - `cargo test -p holler-cli --test pane_integration -- observe:: bind::`: 20 passed.
  - Line counts: `src/pane/wiring/bind.rs` 439, `tests/pane_integration/bind.rs` 580, `observe.rs` 221, `scenario.rs`
    394 and `store_only.rs` 191.
- **Blast radius.** In `crates/`, this cycle's own files are `wiring/bind.rs` and four `pane_integration` files. With
  `CHANGELOG.md`, all are inside the Blast radius.

## Notes for F

None for this verdict. W-7 is T's, and it is not binding.

## Notes for O

1. **Merge `main` again before the PR.**
   - `origin/main` is `853c1e1`: #838 ("Stage 1 asks which Herdr session to build in"), merged at 16:33 MDT.
   - The trial merge (`git merge-tree --write-tree 853c1e1 HEAD`, tree `4686bca`) conflicts in `CHANGELOG.md` only. Both
     sides add the top entry of `### Enhancements`. Keep all three entries.
   - **Unlike #832, this merge needs no code change and no code re-gate.** #838 touches no port, no wiring file, no
     `holler-cli/src` file and no file of this branch. Its new `skilltext_838.rs` reads only `agent-skills/herdr-setup/SKILL.md`
     and `docs/setup-wizard.md`.
   - GitHub runs no CI on a conflicting PR, so this merge must land before the PR phase pushes.
2. **Still unfiled:** the test-glue consolidation (W-O). It can absorb W-1 to W-3, W-6 and W-7.
3. **A follow-up for #654 or #652, not this story's.**
   - #838's wizard writes the Herdr session as `[instance] herdr_session` (a TOML file the wizard's shell scripts read).
     The wiring reads `HOLLER_HERDR_SESSION` from the environment.
   - So an operator who builds a named session with the wizard must still set the variable before `holler pane` reaches
     Herdr.
   - D18 (b) leaves naming the session to the cutover (#654). The operator page (#652) should say how the two meet.
4. **The scenario has run on the merged tree** (S's step 4): F 4/4, and T-green 4/4 with S's advisory 4 now asserted.

## Patterns referenced

- **This cycle:**
  - `crates/holler-cli/src/pane/wiring/bind.rs:15-20, 90-144, 185-258`;
  - `crates/holler-cli/src/pane/wiring.rs:76, 141-152, 306-308`;
  - `crates/holler-pane/src/ports.rs:196-235` (the two defaults and `PaneObservation`);
  - `crates/holler-adapter-herdr/src/adapter.rs:415-447`;
  - `crates/holler-pane-testkit/src/herdr.rs:56-66, 338` (`HerdrOp`, `FakeHerdr::observe_pane`);
  - `crates/holler-pane-testkit/tests/herdr_conformance_test.rs:50-62` (`scratch_fake`, `fixture`);
  - `docs/adr/ADR-0021.md:321-372, 962-1007`.
- **The tests:**
  - `crates/holler-cli/tests/pane_integration/observe.rs` (all of it);
  - `bind.rs:133-178, 293-316`;
  - `main.rs:1-43`;
  - `scenario.rs:234-274`;
  - `store_only.rs:54-99`.
- **`main` after the merge:** `853c1e1`'s file list and its `CHANGELOG.md`, `SKILL.md`, `docs/setup-wizard.md` and
  `skilltext_838.rs`.
