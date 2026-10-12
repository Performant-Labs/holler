# Handoff-A: #835 - `holler pane import` leaves panes Holler does not drive alone, with `--adopt-claude-code`  (up-front plan review, third pass: amendment rounds 2 and 3, on the merged #649)

**Date:** 2026-10-11
**Branch:** `issue-835-implementation` at `f8a1864`. The base is `2d217dd` (#649, PR #843, merged 17:35 MDT), which includes #832 (PR #836) and #838. At 17:57 MDT `origin/main` is `7fc00f1`. The one commit since the base adds `docs/handoffs/655-herdr-move/decisions.md` and nothing else, so no rebase is needed.
**Brief reviewed:** `docs/handoffs/835-brief.md` at `f8a1864` (sha256 `bac2232a86462e4f...`). It holds the issue (the same as #835's body at 15:51 MDT), the amendment (§1 to §7), round 2 (R2-1 to R2-8) and round 3 (R3-1 to R3-5).   **Reuse map:** no separate file; pass 2's, updated below   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This pass replaces pass 2 (BLOCK, `cc3d366`). Git history keeps passes 1 and 2.

## Summary

PASS: 0 `block`, 6 `warn`.

**Round 2 settles pass 2's block and its seven warns** (table below).

- `foreground_commands` is a required `HostPort` method, so the compiler finds every implementation and every decorator.
- The story is sequenced after #649, and this worktree is built on it.
- The plan still extends the right objects:
  - `HostPort` beside `ps`, and in the adapter its listing, `Call` and `refused()`;
  - the import's per-pane chain, and `Outcome::left_alone` beside `not_opencode`;
  - `ImportFinding`, with no new `Action` or `Level`;
  - the `usage` checks in `Input::read`.

**The warns correct facts; they add no design.** No contract decision is left open.

- **W1, W2: round 3 is out of date.** It was written at 15:54 MDT. #649's D33 cycle (16:32 to 16:47 MDT, merged at 17:35) already makes `BindingHerdr` and `LazyHerdr` forward `observe_pane` and `observe_panes`, and pins both. What is left of round 3:
  - removing the trait's default for `observe_pane`;
  - one test double, `TestHerdr`;
  - ADR-0021 §14, which the brief does not list;
  - a verb-level test that passes before any code is written.
- **W3: open PR #840** (#832's review fixes) says #835 calls `unmanaged::is_unmanaged`. The import cannot use that function for its question.
- **W4: R2-6 reverses a merged #650 test** that the blast radius does not list.
- **W5: two implementation lists are each one short**, and R3-2's "no default" sentence asks more than R3-1 allows.
- **W6: `WiredHost` has no forwarding pin.** The existing conformance runs are the cheaper way to pin the forwards.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | R3-1's "Why" ("#649's production wiring ... forwards only the methods that existed when it was written"); R3-2 ("One test per decorator ..."); R3-3 ("New acceptance item") | anti-duplication; the plan's premise against merged code | **#649 merged both forwards and their pins** (D33, `docs/handoffs/649-brief.md:87`).<ul><li>`wiring/bind.rs:137-143` (`BindingHerdr`) and `:248-257` (`LazyHerdr`) override both methods. The module docs say so (`:15-20`).</li><li>The pins, in `tests/pane_integration/observe.rs`: `the_binding_herdr_delegates_observe_pane_and_observe_panes` (`:168`), `the_lazy_herdr_delegates_...` (`:176`), `the_wirings_herdr_chain_delegates_observe_panes_end_to_end` (`:186`) and `the_lazy_herdr_passes_the_herdr_conformance_suite` (`:36`).</li><li>In `bind.rs`: the `BindingHerdr` conformance run (`:294`), and `observe_pane` in the list of methods that answer a failed build's error (`:134-160`).</li></ul>**What this leaves of round 3:**<ul><li>R3-1's reason no longer holds. Removing the default is still valid hardening, for the reason R2-1 gives for `foreground_commands`.</li><li>R3-2 is already met. Another per-decorator test would be a copy.</li><li>R3-3's test passes on `main` before any code: `tests/pane_verbs/unmanaged.rs` pins the verb over `FakeHerdr`, and `observe.rs` pins the chain. So it cannot be a valid RED (`tester.md:46-49`).</li></ul> | **R3-2:** T adds no test. handoff-T-red cites the existing tests.<br>**RED for "required":** a `compile_fail,E0046` doctest of an impl with no `observe_pane`, beside the `ServeSpec` doctests (`ports.rs:113-118`). It fails today and passes once the default is gone. The same is optional for `foreground_commands`: any test that calls it is RED until F adds it.<br>**R3-3:** T writes it as a regression pin that passes before F starts, and says so in handoff-T-red.<ul><li>Put it in `tests/pane_integration/observe.rs`.</li><li>Use `run_verb_with` (as `bind.rs:546` does), `seed_unmanaged_pane` (`fixture.rs:168`) and the chain of `observe.rs:186-192`.</li><li>Build no new rig.</li><li>Do not put it in `tests/pane_verbs/unmanaged.rs`, which #840 edits.</li></ul> |
| 2 | warn | R3-1: "Record the change in section 2's `HerdrPort` row and the 'amended since' sentence, and in the 'Decided (#835)' paragraph of section 13" | ADR consistency: an ADR the change contradicts is updated in the same change | **§14 says the opposite of R3-1.** Its `observe_pane` bullet says the method "has a default ..., a port that does not override it keeps compiling ... A wrapper port must delegate it" (`ADR-0021.md:972-974`). R3-1 makes that false, and the brief does not list §14.<br>**§13 is the wrong home.** It is the migration section. A port decision recorded there is hard to find, and §14 would still contradict it.<br>**The code docs are stale too.**<ul><li>`ports.rs`'s module docs say "Two amendments since" (`:15-24`). That has been stale since #832, and is wrong after #835.</li><li>The method docs at `ports.rs:196-208` describe the default.</li></ul> | **§14:** amend the `observe_pane` bullet in place. It is required since #835: the compiler finds every implementation, and the wrappers have forwarded it since #649's D33. `observe_panes` stays the defaulted batch form.<br>**§2:** the `HerdrPort` and `HostPort` rows and the "amended since" sentence, as R2-2 and R3-1 say.<br>**§13:** keep the "Decided (#835)" paragraph to the import: the signal, the rule and its limits, the two codes and the switch.<br>**`ports.rs`:** update the module docs' list (#832's and #835's amendments) and the docs of `observe_pane` and `HostPort`. |
| 3 | warn | R2-7: "The one constant (`"claude"`) and its predicate are public in `crates/holler-pane/src/pane.rs`, beside `HarnessKind`"; §7; R2-3's coexistence notes | one source of truth; coexistence with a story in flight | **What #840 claims.** Open PR #840 (branch `issue-832-review-fixes` at `8e39fcc`, updated 15:52 MDT) makes `holler_pane::unmanaged::unregistered` and a new `is_unmanaged` public, as "**the one rule of 'a pane Holler does not drive'**, public so that no story writes its own copy". Its doc comments and its §14 sentence say #835 calls them "instead of writing their own".<br>**The import cannot use them.**<ul><li>`is_unmanaged(herdr, records)` asks whether any record names a Herdr pane.</li><li>That is true of every import candidate, because `import.rs:170-178` skips any pane that already has a record.</li><li>So it cannot tell a Claude Code pane from an OpenCode pane.</li></ul>**The two rules answer different questions.** #840's answers a registry question; R2-7's answers a process question, from `HostPort::foreground_commands`. Neither replaces the other. Folding the process question into `is_unmanaged` would make a registry predicate depend on a host read.<br>**#840 edits the same files:**<ul><li>ADR-0021 §14 (+17 lines);</li><li>`tests/pane_verbs/launch/rig.rs` (+22 lines, about 840 before #835 adds its forward);</li><li>`tests/pane_verbs/unmanaged.rs` (+104 lines);</li><li>`CHANGELOG.md`.</li></ul>**One more naming hazard.** The process name `claude` is not the protocol harness id `claude` in `holler_proto::vocab::HARNESS_IDS` (`vocab.rs:273`), though the two are spelled the same. | **F keeps R2-7 as written.** F neither routes the `claude` rule through `is_unmanaged` nor folds it into it.<br>**If #840 has merged when F works,** F corrects #840's two claims (the doc comments of `unregistered` and `is_unmanaged`, and the §14 sentence). They should say that #835 asks the process question through the `pane.rs` predicate. The PR records this as coexistence.<br>**If #840 is still open,** see the operator note below.<br>**The constant's doc** says it is a process name, not the harness id, and it is not derived from `HARNESS_IDS`.<br>**The PR's coexistence notes** name #840. `launch/rig.rs` stays under 900 lines, about 844 with both changes. |
| 4 | warn | R2-6: "a failed `ps` that is not `pane-not-found`" is `harness-unreadable`; the file lists of §6 and R2-2 | blast radius; a merged contract changed on purpose | **A merged #650 test pins the old behaviour.** `crates/holler-cli/tests/pane_verbs/import/join.rs:392-404`, `a_failed_ps_is_observe_failed_and_the_pane_imports`, expects `observe-failed` (a warning) and a recorded pane. R2-6 decides the opposite: `harness-unreadable`, a problem, no record, and `import-blocked`. The file is not in the blast radius.<br>**Two corners are left open:**<ul><li>`ps` fails, but the read answers `claude`. §3's one-finding rule and R2-6 would then both apply.</li><li>`ps` answers, then the read answers `pane-not-found` because the session ended in between. §2 assumes `ps` already reported that.</li></ul> | **T** rewrites and renames that test in RED, to R2-6's behaviour.<br>**The chain** reads names only after `ps` answers `Ok`. A failed `ps` then reads no names, so the first corner cannot happen.<br>**The read's own `pane-not-found`** is mapped as `tmux_finding` maps `ps`'s: `tmux-session-missing`, a problem, so no record and never a guess.<br>**Messages about the read** reuse `failed_message` with the op `host.foreground_commands` (`observe.rs:175-181`). |
| 5 | warn | R2-1: "Implementations to update (the compiler lists them)"; R3-1's list; R3-2: "No decorator or test double may rely on a default for either method" | blast radius; internal consistency | **`HostPort`:** one more implementation, `LinkedHost` in `tests/pane_integration/bind.rs:428` (#649).<br>**`HerdrPort`:** only `TestHerdr` (`crates/holler-pane/tests/ports_test.rs:170`) lacks `observe_pane`. That file exists so that a port change breaks it (`:2-3`). Every other implementation already has the method, including `Observing` (`observe.rs:96`) and `GuardWatcher` (`herdr_conformance_test.rs:140`).<br>**R3-2's sentence cannot hold as written.** `observe_panes` keeps its default (R3-1), and `FakeHerdr`, `TestHerdr` and the test wrappers rely on it. | **Add both files** to the blast radius.<br>**Read R3-2's sentence as:** a forwarding wrapper never relies on the `observe_panes` default; a leaf implementation may. #649's wrappers already comply. |
| 6 | warn | R2-1: "Pin each decorator: a test per decorator that the new method reaches the inner host ..., and the host conformance suite run over the production decorator if #649 already does that" | reuse of the conformance suite over hand-made doubles | **`BindingHost` is already covered.** It runs the host suite (`bind.rs:318-324`), so R2-8's new case pins its forward with no new test.<br>**`WiredHost` (`Wired<dyn HostPort>`) has no forwarding pin.** `wired.rs:109-110` only checks `ensure_session` and `ps`, and only on a misconfigured socket. | **Add one conformance run** over `WiredHost::new(Ok(Arc::new(FakeHost::new())))`, beside `bind.rs:319`. It pins all five forwards.<br>**Add one line to `wired.rs`'s list:** `host.foreground_commands` answers `usage` naming `HOLLER_TMUX_SOCKET`.<br>**Write no recording host.** |

### Pass 2's findings, as rounds 2 and 3 settle them

| Pass 2 | Status now |
|---|---|
| 1 (block): `foreground_commands` default-armed, and not forwarded by #649's decorators | **Settled by R2-1.** The method is required, with no default and no open code. The `wiring.rs` rule is lifted for exactly the forwards. The pins are named, and the story runs after #649, on a worktree built on it. W5 and W6 complete the lists. |
| 2 (warn): the records | **Settled by R2-2.** §13 gets a "Decided (#835)" paragraph and no code table. `import_test.rs` is added to the blast radius, and the missing-session code is spelled `tmux-session-missing`. W2 adds §14. |
| 3 (warn): `fake_tmux_test.rs`, and #832 in the same files | **Settled by R2-3.** The `METHODS` and `call` change is about three lines (818 to about 821), and new adapter tests go in a new file. W3 adds #840. |
| 4 (warn): the codes' names | **Settled by R2-4:** `harness-claude-code` (info, row `skipped`) and `harness-unreadable` (problem). Neither clashes with a reconcile code (`holler-pane/src/findings.rs:100-112`). |
| 5 (warn): the rule's limits | **Settled by R2-5:** the limits go in the "Decided (#835)" paragraph and in the help text. |
| 6 (warn): a lost chain and a failed `ps` still recorded on a guess | **Settled by R2-6.** W4 adds the merged test that the change reverses, and the two corners. |
| 7 (warn): the home of the constant and its predicate | **Settled by R2-7:** public, in `pane.rs`, beside `HarnessKind`. W3 adds #840's competing claim. |
| 8 (warn): the conformance case on a real tmux | **Settled by R2-8:** the case never asserts an empty list, and it polls within a bound for the name. |
| For the operator: `observe_pane` not forwarded | **Taken up as round 3, then overtaken by #649's D33** (W1, W2). |

## Notes for T and F

There is no O in this run, so the warns are written here as instructions.

1. **T: no copies of #649's D33 tests** (W1).
   - Pin "required" with a `compile_fail,E0046` doctest.
   - Write R3-3 in `tests/pane_integration/observe.rs` as a regression pin that passes before F starts, and say so in handoff-T-red.
2. **T: rewrite and rename** `import/join.rs:392-404` to R2-6's behaviour (W4).
3. **T: two conformance pins** (W6).
   - Add the host suite's new case (R2-8). The existing `BindingHost` run then pins that forward.
   - Add one suite run over `WiredHost`, and one line in `wired.rs`.
4. **F: the chain** (W4).
   - The order is `ps`, then the names, then health and the session list only for a pane that is not `claude`.
   - Names are read only after `ps` answers `Ok`.
   - The read's `pane-not-found` is `tmux-session-missing`.
   - Messages go through `failed_message`.
5. **F: the docs** (W2).
   - In ADR-0021: §14's `observe_pane` bullet; §2's rows and its "amended since" sentence; §13's "Decided (#835)" paragraph, about the import only.
   - In code: `ports.rs`'s module docs and the docs of the two methods.
   - Elsewhere: ADR 0003's row, `cli-surface.txt` and `CHANGELOG.md`.
6. **F: the implementations the compiler will list** (W5).
   - `TestHerdr` gets `observe_pane`.
   - `LinkedHost` in `pane_integration/bind.rs` forwards `foreground_commands`.
7. **F: keep the process rule in `pane.rs`** (W3).
   - If #840 has merged, correct its two sentences that name #835.
   - Name #840 in the PR's coexistence notes.

## For the operator (outside this story)

1. **#840.** Its doc comment and its §14 sentence name #835 as a caller of `is_unmanaged`, but #835 asks a different question (W3). Removing "#835" from those two places before #840 merges is the cheapest fix. Otherwise #835's F corrects them.
2. **Round 3.** #649's D33 fixed the flaw before this run. Making `observe_pane` required is still done, as hardening; that was the operator's decision, and it adds a compile-time guard for future wrappers (W1).
3. **The real-fleet check, unchanged from R2-5.** tmux reports the argv[0] of the pane's foreground process group leader, so the rule sees a Claude Code pane only while `claude` is that process. Read the dry run of the first real import while the fleet's Claude Code sessions are running. A Claude Code pane that the dry run shows as `record` is a recorded limit, not a defect. For example, a `claude` that runs as a `node` script may be reported as `node`.

## Reuse map (pass 2's, updated for the merged #649 and #832; for the Phase 7 gate)

| Need | Extend, do not copy | Where |
|---|---|---|
| A pane left alone | `Outcome::left_alone`, with one constructor beside `not_opencode`; `Action::Skipped`; one kind in `ImportFinding::ALL`, with its `code` and `level` | `holler-pane/src/import/assess.rs:58-96`; `import.rs:66-89`; `import/findings.rs:62-165` |
| The unreadable signal | One more `Level::Problem` kind, through `assess`'s `blocked`, so that `Plan::refusal` refuses it as `import-blocked` | `assess.rs:126-161`; `import.rs:130-157` |
| The observation | The chain in `observe()`: one read after `ps` answers, on the same per-pane thread; `tmux_finding` and `failed_message` for its errors | `import/observe.rs:79-84`, `:155-181` |
| The rule | One public constant (a process name) and one predicate, beside `HarnessKind`. It is separate from #840's registry rule `unmanaged::is_unmanaged`. | `holler-pane/src/pane.rs:126-138` |
| The port read | `HostPort`, beside `ps`. In the adapter: a listing beside `panes()` and `stop_listing()`; a strict parser beside `parse_ps` that drops dead panes, as `PS_FORMAT` does; `Call` and `refused()`; an `OP_*` constant | `ports.rs:253-275`; `holler-adapter-host/src/tmux.rs:28-33`, `:112-136`, `:214-228`; `src/lib.rs:139-142`, `:383-392`, `:439-449` |
| The forwards | `BindingHost`, `Wired<dyn HostPort>` and `Unwired` (not-implemented) | `holler-cli/src/pane/wiring/bind.rs:162-183`, `:335-351`; `wiring.rs:311-327` |
| The forwards' pins | The host suite over `BindingHost` (it exists) and over `WiredHost` (one new run); D33's tests of the `observe_*` methods (they exist) | `tests/pane_integration/bind.rs:318-324`; `tests/pane_integration/observe.rs` |
| `observe_pane` required | Remove the default; add the method to `TestHerdr` | `ports.rs:196-220`; `holler-pane/tests/ports_test.rs:170` |
| The fake and its pins | `FakeHost` keeps a name per process, dropped by `exit_process`, `stop_owned` and `end_session`. One new `HostOp` variant. One case in the host suite and one `Break` in the `Mutant`. `METHODS` and `call` in `fake_tmux_test.rs`. The fake's own tests in `fake_host_test.rs` | `holler-pane-testkit/src/host.rs`; `src/conformance/host.rs`; `tests/host_conformance_test.rs`; `tests/fake_host_test.rs`; `holler-adapter-host/tests/fake_tmux_test.rs:25-30`, `:67-75` |
| The switch | The `PaneImport` arguments, step 1 of `Input::read`, `usage()` and `output::not_implemented_message` | `holler-cli/src/pane/import.rs:79-96`, `:149-166`, `:218-222`; `output.rs:251-255` |
| R3-3's pin | `run_verb_with`, `seed_unmanaged_pane` and the chain built in `observe.rs` | `pane_integration/bind.rs:546`; `holler-pane-testkit/src/fixture.rs:168`; `pane_integration/observe.rs:186-192` |
| The docs | ADR-0021: §2 (`:108-109`, `:120-121`), §9 (`:757`), §13 (a "Decided (#835)" paragraph beside "Decided (#650)", `:938`) and §14 (`:972-976`). ADR 0003 (`:63`). `cli-surface.txt` (`:167-168`). `CHANGELOG.md`. The module docs of `ports.rs` (`:12-24`). | as listed |

At Phase 7, flag any of these as a parallel path:

- a second tmux runner, or a listing that bypasses `run_tmux` and `Call`;
- a second per-pane thread scaffold in the import;
- an import-only copy of `ps`'s error mapping or of `failed_message`;
- a new `Action` or `Level`;
- a comparison with `"claude"` outside the one predicate, or the rule folded into `unmanaged::is_unmanaged`;
- a decorator or test double that answers `foreground_commands` without reaching the inner host;
- a host read that reaches `TmuxHost` around `BindingHost`;
- new copies of D33's `observe_*` tests;
- a new rig for R3-3.

## Patterns referenced

- `crates/holler-pane/src/import.rs` and `import/{assess,observe,findings}.rs`: #650's plan, the skip path, the levels and the lost-chain branch. `crates/holler-cli/tests/pane_verbs/import/{join,rig}.rs`.
- `crates/holler-pane/src/ports.rs`: the frozen ports, the `observe_*` defaults and the `ServeSpec` `compile_fail` doctests. ADR-0021 §2, §13, §14 and decision 12.
- #649 as merged: `crates/holler-cli/src/pane/wiring.rs` and `wiring/bind.rs` (D31 W-4 and D33, every method forwarded); `tests/pane_integration/{bind,observe,wired}.rs`; `docs/handoffs/649-brief.md:87` (D33).
- `crates/holler-adapter-host/src/{lib,tmux}.rs`, `tests/fake_tmux_test.rs`; `crates/holler-pane-testkit/src/{host.rs,conformance/host.rs}` and `tests/host_conformance_test.rs`.
- Open PR #840, branch `issue-832-review-fixes` at `8e39fcc`, read through git only: `crates/holler-pane/src/unmanaged.rs` and ADR-0021 §14.
