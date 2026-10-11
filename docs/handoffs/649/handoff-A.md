# Handoff-A: Phase 3 - #649 hub wiring and the scratch integration scenario  (up-front plan review, fourth pass: the third-pass amendment and the merges of #664 and #650)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `d0c1531`)
**Brief reviewed:** `docs/handoffs/649-brief.md` at `451e2b3` (sha256 `afe88dc4916d7e09...`).
- The two merges after it left the brief unchanged: `f59ba21` (#664, `profile apply`) and `d0c1531` (#650, `pane import`). The second landed during this review.
- I read the brief against the tree at `d0c1531`.
- The issue's body matches the brief from Scope to Pipeline. The issue was updated at 12:16 MDT.

**Reuse map:** no separate file. As in the earlier passes, the brief's Evidence (E1 to E70) and D11 serve as the map.   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This pass replaces the third one (BLOCK on the brief at `6c07882`, committed in `be87a09`). It reviews the amendment `451e2b3`, then what the merges of #664 and #650 changed under the plan.

## Summary

**Verdict: PASS.** The amendment closes B-2, and no finding blocks. There are five warns. Three bind F, T or phase 7 as A's readings, and none needs a new operator decision.

- **B-2 is closed.** #828 is open (filed at 12:14 MDT) and scoped to the first message's path. The brief names it in four places:
  - D18 (c);
  - D17, for both ADR sentences;
  - the W-B sentence, with the stopgap "until #828 lands, the operator avoids `reset --first` on a parked or held pane";
  - the Depends on line.
- **W-N (binding, F): three ADR edits.**
  - D17 still turns the model, effort and environment sentence into "#642 only", but D18 (f) gives it to #829.
  - D18 (b)'s wording of the `--herdr-session` deferral is too broad.
  - The Consequences bullet says CLI tests never use a real Herdr, tmux or OpenCode, and #649 adds the first that do.
- **W-O (binding, phase 7): a third `LinkedHost`.** #664 added it before this story could, which triggers D32's "a third copy moves to the test kit". The test kit is outside the Blast radius. W-O restates A's own note: the copy stays, and O files the consolidation.
- **W-P (binding, T): two no-hub rows no longer parse.** Since #664 and #650, `profile apply` and `pane import` take a required argument, so the no-hub test's bare rows fail at clap (exit 2) before `connect` runs.
- **W-Q (F's paragraph and O's follow-ups): what the merges make live through the wiring.**
  - Apply's creates and relaunches are logged under `holler pane`, which #825's premise does not cover.
  - Apply's observation bound meets W-C's case.
  - Import reports a mismatched `--herdr-session` as `pane-unlabelled` for every pane.
- **W-R (low): `docs/testing.md`.** The new opt-in gate is documented nowhere but its module doc.

**The rest of the third pass is resolved.**

- **W-K (a):** #829 is filed and named in D18 (f) and the Depends on line, but not in D17 (W-N (a)).
- **W-K (b):** D18 (b) says no code story is planned for the default, and #654's comment records it.
- **W-K (c):** #654's comment (12:14 MDT) gives the DRIVEN wiring to the cutover.
- **W-L:** D15 runs two specs, the second with an agent.
- **W-M:** D12, D19 and the Depends on line are updated.

The merges change nothing the plan builds on (listed below the table), and the work the brief blesses type-checks at `d0c1531`.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-N | warn (**binding on F**) | **D17's list of ADR edits**, with D18 (b) and D18 (f) | ADRs | **(a) Model, effort and environment** (ADR 547-548). The ADR reads "no port carries them yet (#642, #649; they will join the `ServeSpec`)", and D17 still turns that into "#642 only". But D18 (f) and the Depends on line give the work to #829, which is open and scoped to it. #642 closed at 11:56 MDT without it. Followed literally, D17 writes a closed owner into the ADR, the failure B-2 blocked on. **(b) The `--herdr-session` deferral** (568-569). D18 (b) says "the flag stays required on every verb that reaches Herdr". In fact:<ul><li>`pane launch` requires it;</li><li>`pane import` requires it (#650);</li><li>`profile apply` needs it only to create a pane (#664, `apply.rs:62-64`);</li><li>`relaunch`, `close` and `doctor` reach Herdr with no such flag.</li></ul>D5's "required by `launch`" is the accurate wording. **(c) The Consequences bullet** (916-917). It says: "tests that drive the CLI use the test kit's fakes, never a real Herdr, tmux or OpenCode, and refuse real session names". #649's scenario is the first CLI test that runs real Herdr, tmux and OpenCode, on scratch instances. D17 does not list the bullet. The overlay requires an ADR that a change contradicts to be amended in the same change. | **A's reading, binding on F.**<ul><li>(a) The sentence names #829. D18 (f) is the later, explicit decision, and D17's "#642 only" was written before #829 existed.</li><li>(b) The sentence says the flag stays required by `launch`, and is needed by `pane import`, and by `profile apply` to create a pane. Defaulting it from the adapter's configured session has no story (D18 (b)).</li><li>(c) The bullet says the default run uses the fakes. The opt-in scenarios (#649, and #667 for apply) run scratch instances they start themselves and refuse real names (D10).</li></ul>All three edits are in ADR-0021, inside the Blast radius. No decision changes. B-2 was a block because the brief named #646 three times with no alternative. Here the brief names #829 twice, so F has a reading that is both compliant and true. |
| W-O | warn (**binding at phase 7**; restates D32) | **D32**: "the one `LinkedHost` in `pane_integration/bind.rs` is acceptable test-local glue (a third copy moves to the test kit)" | reuse | **#664 made the WIP's copy the third.** The three copies of the fake host whose `run` also starts the fake harness's server:<ul><li>`profile_verbs/apply/rig.rs:636` (#664, merged);</li><li>`pane_verbs/launch/rig.rs:663` (#644, #802);</li><li>`pane_integration/bind.rs:422` (the WIP).</li></ul>**Why D32 cannot be read literally.** It would have F move #649's copy into `holler-pane-testkit`. That is outside the Blast radius, and the other two copies belong to #644 and #664. **The test kit prescribes local glue.** Its docs leave this to each test as a `HostPort` wrapper (`host.rs:12-17`, `harness.rs:13-17`). **Nobody filed the consolidation.** #664's anti-duplication gate called the copy the house pattern (its warn 2) and asked O for a consolidation issue. None is open. | **A's reading of its own note.**<ul><li>#649's copy stays as test-local glue.</li><li>Phase 7 checks that #649 adds no fourth copy and that its copy serves only D8's pin.</li><li>For T: the test kit's documented wrapper also kills the harness server in `stop_owned`, and the WIP's does not (`bind.rs:438-440`). That matters only if the pin rolls back, so it is T's call.</li></ul>**O files the consolidation:** one wrapper in the test kit for all three, for example a `HostPort` that links `FakeHost` to `FakeHarness`, as #664's gate asked. |
| W-P | warn (**binding on T**) | **The no-hub test** (`pane_integration/no_hub.rs:18-39`, AC 1) and **D2's list of stubs** | none (test arguments) | **Two rows no longer parse.** `EVERY_VERB` runs `["pane", "import"]` (line 30) and `["profile", "apply"]` (line 35) with no arguments.<ul><li>Since #664, `profile apply` takes a required `NAME` (`apply.rs:55`).</li><li>Since #650, `pane import` takes a required `--from` (`import.rs:83`).</li></ul>With no arguments, each is clap's usage error (exit 2), raised before `connect`. So both no-hub tests fail on these rows, whatever the wiring does. **D2's stub list is out of date.** `pane import` and `profile apply` are verbs now. Only #665's three PROPOSED verbs remain in `STUBS` (`stub.rs:18-31`), and the forced-diagnostic test now takes `STUBS.first()` (`stub.rs:216-217`). D22's plan is unchanged, and the WIP's `LiveHub` conversion survived both merges. | **T gives each row arguments that parse,** for example `profile apply hlr649` and `pane import --from <a scratch directory> --herdr-session holler649-00000000`. With no hub, `connect` then answers `unavailable` first. **These failures come from the test's own arguments.** They are **not** RED evidence for the wiring (D32). No brief change is needed. |
| W-Q | warn (not binding; F's paragraph and O's follow-ups) | **What #664 and #650 make live through the wiring**: D18 (e), D8's W-C sentence and D18 (b) | cross-cutting concerns (audit trail, bounds, configuration) | **(a) Apply's profile writes are logged as `holler pane`.**<ul><li>`profile apply` creates through `tx_launch::launch` with `profile: Some(P)`, and relaunches with the same (`tx_apply/run.rs:51-57`, `:120`).</li><li>The engine then writes P through `ports.scope.edit_spec` (`tx_launch.rs:467`, `:576`).</li><li>`StoreScope` writes P with its fixed actor, even when the spec is unchanged (`profile_scope.rs:194`). Through D1's wiring that actor is `holler pane`.</li></ul>So a profile verb's writes are logged as a pane verb's. D18 (e) and #825 rest on "each profile verb names its own actor" (E20). Import is not affected: it writes under `holler pane import` (`import/writes.rs:27`). **(b) Apply's observation bound.** `tx_apply.rs:40-50` says the observation takes "at most one port call's own bound". It runs one `shown_session` per recorded pane at once. `shown_session` fixes its deadline and only then calls the resolver (`attach.rs:100-101`, `:169`). Through the wiring that is W-C's case: one `pane/list`, or, when the registry misses, Herdr's connect ping plus a snapshot. **(c) Import with a mismatched `--herdr-session`.** #650's join keeps only the snapshot panes whose session equals `--herdr-session` (`import/assess.rs:168`). The adapter stamps every pane with its own configured session (`adapter.rs:262`). A flag that differs from `HOLLER_HERDR_SESSION` therefore reports every pane as `pane-unlabelled`, and refuses a real run as `import-blocked`, without naming the mismatch. `launch` and apply's creates get a clear `unavailable` instead (E24). | **(a)** F's "Wiring as built (#649)" paragraph names `profile apply`'s creates and relaunches among the writes logged as `holler pane`. O comments on #825 so its scope covers `profile apply`. **(b)** That paragraph's W-C sentence names `shown_session` beside `attach_tui`, and says the case also qualifies the observation bound in `tx_apply.rs`. That file is outside the Blast radius and stays unedited. **(c)** No change for #649. #654's comment already tells the operator to pass the same name. With this evidence, the operator may revisit D18 (b)'s "no code story is planned for the default". |
| W-R | warn (low; O decides) | **`docs/testing.md`** | pattern consistency (no dominant pattern) | **The new opt-in gate is undocumented.** `docs/testing.md` documents #640's scratch Herdr gate (lines 439-449) but not #642's real-OpenCode gate, so the precedent is mixed. #649's `HOLLER_TEST_PANE_INTEGRATION=1` is the end-to-end check an operator is most likely to run by hand. Only `pane_integration/main.rs`'s module doc names it, without the cargo command. `docs/testing.md` is not in the Blast radius. | **Optional.** Either add `docs/testing.md` to the Blast radius for one section after #640's, "Opt-in: the pane integration scenario (#649)", or have the module doc carry the exact command. Not binding: the codebase has no dominant pattern. |

**Checked against the merges, and consistent.**

- **The ports.** Neither merge touched `ports.rs`. `HarnessPort` still has eleven methods, so D31's "all eleven methods forwarded" holds.
- **ADR-0021.** Neither #664's "Apply as built (#664)" paragraph nor #650's import decisions name #649. Every sentence D17 lists is still found by its words. At `d0c1531`:
  - the `HostPort` amendment, 186-187;
  - the seam, 521-525;
  - the model, effort and environment, 547-548;
  - the `--herdr-session` default, 568-569;
  - DRIVEN, 823-828, 875 and 926;
  - the first message, 932-934.
- **Prompts.** Neither merge sends one: `tx_apply` and `import` call no `send_prompt`. `reset --first` stays the only path around the hub's gate (#828).
- **Threads (#664).** Apply's observation calls ports from scoped threads (`tx_apply/observe.rs:54-83`). The WIP's parts are safe for that:
  - the store clients hold no state, one control exchange per call (`wiring/store.rs:17-28`);
  - `LazyHerdr` is a `OnceLock`;
  - the bindings sit behind a `Mutex`.
- **Several launches in one process (#664).** Apply runs its creates in sequence over one wiring. The WIP keeps one `pending` slot, replaced by each `ensure_pane` and taken only after the inner `ensure_session` succeeds (`bind.rs:37`, `:55`). That keeps D8 (a) correct across launches, as D8's W-5 sentence requires.
- **Import (#650).**
  - Its observation makes only `host.ps`, `harness.health` and `harness.list_sessions` (`import/observe.rs:11-12`). It calls no TUI method, so it makes no resolver call for a pane with no record.
  - The adapter's snapshot covers its one configured session (`adapter.rs:388-406`), so D8 (c) needs no filter of its own.
- **Lint.** D11's `#![allow(dead_code)] // #649` passes `lint.sh` rule 1, which accepts a trailing `// #NNN`, as `real_tmux_test.rs:14` and the adapters' `tests/common/mod.rs` already do. Every touched file is under 900 lines. `tests/support/mod.rs` is still 857 lines (W-I).
- **Compilation.** `cargo check --tests` passes for `holler-cli`, `holler-pane` and the three adapter crates, at `f59ba21` and at `d0c1531`.
- **Protocol.** No wire field or golden file changes, and `holler-hub` gains no dependency.

## Notes for O

PASS, so nothing is required. The binding readings above stand without a brief edit.

**Optional, in one brief edit** (so the next reader of the brief needs no handoff):
- W-N's three ADR items into D17;
- W-P's two argument rows into D2 or D22.

**Follow-ups:**
- comment on #825 that `profile apply` logs as `holler pane` through the wiring (W-Q (a));
- file the `LinkedHost` consolidation (W-O).

**Noticed outside this plan:**
1. **#653 and #828.** #653 (pfleet's MO adoption) will route `pfleet o reset` to `holler pane reset`, and it has no link to #828. The ADR's stopgap ("the operator avoids `reset --first`...") does not cover an automated caller. Consider a comment on #653, or on the pfleet epic it tracks.
2. **#803 and #649.** #803 (the setup wizard through `holler pane` and `profile apply`) is blocked by #644, #664 and #642, but does not list #649. Without #649, every verb it calls answers `not-implemented`.

**Unchanged for T-red and phase 7.**
- D32 as W-O restates it.
- The WIP's known gaps stay T's and F's to fix:
  - the scenario creates and lists sessions over raw HTTP, where D21 says `HarnessPort`;
  - the grid test launches into empty workspaces, where D23 says r1c1 comes first;
  - the probe `zz_probe_attach_after_run` is still there (D24).

## Patterns referenced

- **`docs/adr/ADR-0021.md` at `d0c1531`:**
  - the lines D17 edits (listed above);
  - the Consequences bullet, 916-917;
  - #664's "Apply as built (#664)";
  - #650's "Decided (#650): the import's join".
- **#664's engine and the scope it writes through:** `crates/holler-pane/src/tx_apply.rs` (40-50), `tx_apply/observe.rs` (54-83) and `tx_apply/run.rs` (51-62, 106-126). `tx_launch.rs` (462-468, 570-578), `crates/holler-cli/src/pane/profile_scope.rs` (180-198).
- **The adapters:** `crates/holler-adapter-opencode/src/attach.rs` (96-102, 161-176), and `crates/holler-adapter-herdr/src/adapter.rs` (254-266, 388-406), against `crates/holler-pane/src/import/{observe.rs, assess.rs:168}`.
- **The test kit and the copies:** `crates/holler-pane-testkit/src/host.rs:12-17` and `harness.rs:13-17`, and the three `LinkedHost` copies. `docs/handoffs/664/handoff-A-dup.md` (warn 2).
- **The issues:** #828, #829, #825, #653 and #803 (bodies); #654 (comment of 12:14 MDT).
