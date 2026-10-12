# Handoff-A: #846 linkedhost-fakes  (up-front plan review)

**Date:** 2026-10-12
**Branch:** issue-846-implementation
**Brief reviewed:** docs/handoffs/846-brief.md   **Reuse map:** the brief's "Files" / "Reuse map" paragraph (no separate survey.md)   **Wireframe:** N/A
**Verdict:** BLOCK

## Summary
The core design is sound: one `LinkedHost<H, L>` in `tests/verb_harness/linked_host.rs`, a `HarnessLink` trait, two named
error modes, and the launch rig keeping a private link. It extends the right home, with no testkit or production change.
But the brief states four facts about the tree that are false at `6eff9cc`, and three acceptance criteria
(AC 1, AC 2, AC 5) cannot be satisfied or checked as written. F and T would stumble on them, and S would fail the story
on a criterion no code can meet. These are cheap to amend in the brief.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | AC 1 `grep -rn "impl HostPort for" crates/holler-cli/tests` prints exactly one line | pattern consistency (checkable criterion) | Two defects. (a) `pane_verbs/import/harness.rs:307` `impl HostPort for LostChain<'_>` also matches, so the grep prints **5** lines today, not 4, and **2** after the fold. `LostChain` is a deliberate panicking-`ps` wrapper, not a linked host. (b) The planned `impl<H: Borrow<FakeHost>, L: HarnessLink> HostPort for LinkedHost<H, L>` does not match `impl HostPort for` (generics sit between `impl` and `HostPort`), so the grep would print **0** lines for the new module. | Reword to `grep -rnE "HostPort for LinkedHost" crates/holler-cli/tests` (exactly 1 line, in `verb_harness/linked_host.rs`) and `grep -rnE "struct LinkedHost"` (exactly 1 line, same file). Say that `LostChain` stays, in Out of scope. Correct the baseline "RED" to 4 and 4 for these two greps. |
| 2 | block | AC 5: "`sessions_for(&[])` appears once in `pane_integration`" and W-3b | reuse map accuracy | `sessions_for` does not exist in `holler-cli/tests`. It is in `holler-adapter-opencode/tests/real_opencode/rig.rs:567`, another crate's test rig, which the brief itself puts out of scope (D5). `Scratch::harness` (`scratch/mod.rs:392-396`) spells an inline `Err(PaneNotFound)` closure **once**; there is no second copy. The AC is vacuous (0 occurrences today and after) and W-3b has nothing to fold. | Drop W-3b from scope and from AC 5. Do not touch `scratch/mod.rs` (see finding 3). |
| 3 | block | Edit list: `scratch/mod.rs`; AC 2 command | blast radius / layering | `pane_integration/scratch/mod.rs` and `out.rs` are included **by path** into the `profile_apply_scenario` target (`profile_apply_scenario/main.rs:31-35`). That target is not in AC 2's command, and it does not build `pane_integration/common.rs`. A `scratch/mod.rs` edit that reaches `crate::common` would break it. | Add `--test profile_apply_scenario` to the characterisation command (its default-run pins, with the `#[ignore]` scenario skipped). State that nothing under `pane_integration/scratch/` or `out.rs` may import from `common.rs`. With finding 2, F should not need to touch `scratch/mod.rs` at all. |
| 4 | block | AC 5: the no-hub assertion helper "used by `no_hub.rs`, `fake_hub.rs` and `scenario.rs`" | reuse map accuracy | `scenario.rs` has no no-hub assertion (no "no live holler hub", "reachable" or "unavailable" text). The real copies are four, in two files: `no_hub.rs:117-119` (`names_the_missing_hub`: phrase and state dir), `fake_hub.rs:349` (full "... reachable at {state}" string), `fake_hub.rs:423` (phrase only, no state dir), and `no_hub.rs`'s `NO_HUB` const. | Rewrite AC 5 and W-6: one helper (phrase plus state dir) in `common.rs`, used by `no_hub.rs` (two tests) and `fake_hub.rs` (two tests). `fake_hub.rs:423` asserts the phrase only; say whether it keeps that weaker form or takes the helper with a state dir. A change in what a test asserts is out of scope, so it should keep the phrase-only check unless the state dir is already in scope. Remove `scenario.rs` from the edit list. |
| 5 | warn | Risks: "dead-code and `unused` warnings ... must not add an un-linked `#[allow]`" | pattern consistency | All four targets already carry a crate-level `#![allow(dead_code)]` (`pane_verbs/main.rs:5` #670, `profile_verbs/main.rs:5`, `pane_claude/main.rs:4` #831, `pane_integration/main.rs:5` #649). The unused-link risk is already covered and F needs no `// #846` allow. | Replace the risk with the fact. Keep "adds no new `#[allow]`" as a check. |
| 6 | warn | `FakeHarnessLink { harness, ports: &[u16], ... }` | lifetimes / abstraction | `pane_integration` needs `Arc<FakeHarness>`: the same `Arc` also goes into `Resolving { inner: harness, .. }` (`bind.rs:~636`), and `Arc<dyn HostPort>` needs `'static`. The other rigs borrow an owned `FakeHarness` (`rig.harness`). The sketch leaves `harness`'s type open. `RIG_PORTS` also differs per rig (`[u16; 3]` in launch, `[u16; 8]` in apply), so `ports` must be a slice parameter, not a constant of the module. | Give `FakeHarnessLink` the same `Borrow<FakeHarness>` generic as the host (`Arc<FakeHarness>` and `&FakeHarness` both satisfy it), and take `ports: &'static [u16]` (the const arrays promote), or document the `Arc` fallback. Bound `H`/`L` by `Send + Sync` so `HostPort: Send + Sync` holds. |
| 7 | warn | The shared kill loop "lives once" | duplication | Launch keeps a private `HarnessLink` and still needs the kill loop (`kill_on_stop`). It can only reuse the loop if it is a `pub fn` (for example `kill_served(harness, ports, name)`) in `linked_host.rs`, called by both `FakeHarnessLink::stopped` and launch's link. | Name that function in the brief so Phase 7 can check it is called from two places and defined once. |
| 8 | warn | New `pane_integration/common.rs` | naming and file structure | `pane_integration` modules are named by topic (`bind`, `clients`, `env`, `guards`, `out`, `watch`); shared helpers otherwise live in `main.rs` (`with_env`, `holler_on`). `common.rs` is a catch-all with no sibling precedent. It is acceptable (it keeps one target's glue out of `verb_harness`), but narrower names fit better. | F may split by topic (for example `traps.rs` for the trap builder and no-hub check, and a `CallLog` plus suite runner in `observe.rs` imported by `bind.rs`). If `common.rs` stays, its module doc names what it holds. |
| 9 | warn | Size | file size | `bind.rs` is 693 lines (above the 600 warning). The plan moves `Recording`'s log, the conformance runner and the `LazyHerdr` builder out of `bind.rs`/`observe.rs` into shared code, so it should shrink. `launch/rig.rs` at 852 is close to the 900 fail line and gains a `HarnessLink` impl of about 15 lines while losing about 50. | Keep AC 6's "shrinks, never grows" and record `wc -l` before and after for the four rigs. |
| 10 | warn | D2 (testkit home) | cross-cutting | The testkit's own `host.rs` module doc (lines 12-17) prescribes exactly this "`HostPort` wrapper" pattern, which supports folding it long term into `holler-pane-testkit`. The brief's deferral is right for scope. | Have the new module's doc say so and link a follow-up issue number (to be filed), as D2 intends. |

## Notes for O
Amend `docs/handoffs/846-brief.md` before a fresh run (the script stops on an unchanged brief; do not `resumeFromRunId`):

1. AC 1: use `grep -rnE "HostPort for LinkedHost"` and `grep -rnE "struct LinkedHost"`, each exactly one line in
   `verb_harness/linked_host.rs`. Baseline is 4 and 4. Add `LostChain` (`pane_verbs/import/harness.rs:307`) to Out of scope.
2. AC 2: add `--test profile_apply_scenario` to the command, so the path-included `scratch`/`out` stay proven to build.
3. AC 5 and the near-copy list: drop W-3b (`sessions_for` is another crate's helper; the inline closure is one copy). Correct
   W-6 to `no_hub.rs` plus `fake_hub.rs` only, with `scenario.rs` removed from the edit list. Keep W-3 (two `opencode-ran`
   writers; verified at `wired.rs:36` and `no_hub.rs:67`) and W-7.
4. Say that `scratch/mod.rs` and `out.rs` are not edited, and nothing there imports from `pane_integration/common.rs`.
5. Replace the dead-code risk with: the four targets already allow `dead_code` crate-wide; no new `#[allow]`.
6. Optional (warn 6, 7): `FakeHarnessLink` generic over `Borrow<FakeHarness>`, `ports: &'static [u16]`, and a named
   `pub fn` for the shared kill loop.

None of these changes the design. After the amendment the plan should PASS.

## Patterns referenced
- `crates/holler-pane-testkit/src/host.rs:12-17` (the documented `HostPort` wrapper pattern)
- `crates/holler-cli/tests/{profile_verbs/apply,pane_verbs/launch,pane_claude}/rig.rs` and `pane_integration/bind.rs` (the four `LinkedHost`s)
- `crates/holler-cli/tests/profile_apply_scenario/main.rs` (path-includes `scratch` and `out`)
- `crates/holler-cli/tests/pane_integration/main.rs` (module layout and crate-level allows)
- `scripts/lint.sh:43-50` (size gate)
