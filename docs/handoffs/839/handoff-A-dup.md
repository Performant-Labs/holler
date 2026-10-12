# Handoff-A-dup: #839 - DRIVEN follows each pane's session of record  (anti-duplication gate, S rework round 1)

**Date:** 2026-10-12 (10:47 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`)
**Diff base:** `fde8eb8` (S's REWORK; its code is the same as at `e83ea9c`, the head of the first A-dup pass)
**Diff head:** `ee874c1`
This cycle's change set: F `fde8eb8..9eb55d4`, then T-green `9eb55d4..ee874c1`. The story's whole base is `4e691d9`.
**Reuse map:** `docs/handoffs/839-brief.md`, "Reuse" (line 383) and "Where the code goes" (line 339). This run has no
`survey.md`.
**Verdict:** PASS

This file covers this cycle. The first A-dup pass reviewed the whole story (`4e691d9..8d3e7fa`), and its handoff is in
git: `git show e83ea9c:docs/handoffs/839/handoff-A-dup.md`.

## Summary

**PASS: 0 `block`, 6 `warn`, and every warn is carried from the first pass.** This cycle removed a duplicate and added
no parallel path.

**What changed in production.** S's REWORK 2 asked for one change, in `HubClient::held`. F instead moved the
`session-held` refusal into one builder, `holler_pane::hub::session_held(id, name, kind, reason) -> PaneError`. Both
implementations of the port now call it:
- the real client: `HubClient::held` (`crates/holler-cli/src/pane/wiring/hub.rs:171-182`);
- the test kit: `FakeHub::judge` (`crates/holler-pane-testkit/src/hub.rs:215`).

Before this cycle each of them spelled the sentence. That copy was the second half of the first pass's warn 2, and it is
now gone. A grep of `crates/` finds the sentence only in `holler-pane/src/hub.rs` and in one test assertion.

**How it is placed.** The builder follows the patterns that already exist:
- It sits beside `SESSION_HELD` in the file that declares that code (D7). `driven::not_driven` builds
  `pane-not-driven` in `holler-pane` the same way.
- It prints advertised text through `findings::quoted`.
- It builds the command only from text that passes the grammar: `RoutableName::parse` gates the
  `holler release NAME` command, as `parse_session_id` gates `switch_command`.
- `RoutableName::parse` is the one grammar used at the other two boundaries as well: the CLI's typed-SESSION refusal
  (`say_cmd.rs:282`) and the hub's resolver (`live/target.rs:27`).

**Nothing new in the structure.** There is no new module, crate or dependency. `holler-pane` already depends on
`holler-proto`, and the CLI and the test kit already depend on `holler-pane`. `HubPort` is unchanged.

**Two other places checked; neither should have been extended:**
- **`hold_cmd::held_refusal`** (`crates/holler-cli/src/hold_cmd.rs:147-169`) is the CLI's exit-4 held text for `say`
  and `interrupt`.
  - It is a different refusal on purpose: P5 keeps the three `--pane` verbs on it and gives only `reset --first` the
    `session-held` words of D4.
  - It also lives in `holler-cli`, which neither `holler-pane` nor the test kit may depend on.
- **A `release_command` beside `switch_command`** in `findings.rs` was rejected by F, and the rejection holds.
  - That file's builders are the commands of doctor's remedies (`holler pane …`).
  - `holler release` is the refusal of a prompt, and it has one caller.

**The tests T added this cycle reuse what exists:**
- The three new process tests in `driven_e2e_test.rs` use the file's existing `Rig`, `pane`, `stderr` and
  `prompts_to`, and `support`'s `wait_for`, `STARTUP_WAIT` and `Body::stop`.
- They add one local loop helper, `assert_not_driven_in_all_three_verbs`, which has no twin in `support/` or
  `pane_verbs/driven*`.
- None of the named Phase 7 candidates is copied: `Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`, the
  token store, `Lockout`, `Roster` and `log(Severity, ..)`.
- `hold_rig::assert_held` was not a missed reuse. It asserts an in-process control result on a spawn-mode rig, while
  the new test asserts the CLI's exit code and stderr on the attach-mode `Rig`.
- The new unit test in `driven_test.rs` calls the one builder directly and uses the file's existing `SOR1`.

**Sizes.** Every file touched this cycle is far below the 900-line lint limit:

| File | Lines |
|---|---|
| `holler-pane/src/hub.rs` | 106 |
| `wiring/hub.rs` | 223 |
| test kit `hub.rs` | 310 (down from 317) |
| `driven_e2e_test.rs` | 705 |
| `driven_test.rs` | 423 |

**The cycle added no drift** in layering, dependency direction, naming, error handling or abstraction level.

## Findings

All six warns are carried from the first pass, with line numbers re-checked on `ee874c1`. The first column says what
became of each this cycle. No hub file changed this cycle (`git diff e83ea9c ee874c1 -- crates/holler-hub` is empty).

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn (carried, unchanged) | `crates/holler-hub/src/live/target.rs:26-60` | **`resolve_prompt_target`'s id branch re-implements `Registry::resolve_session`'s scaffolding** (`live.rs:762-798`):<br>- the walk over cached presences;<br>- the `(LiveHandle, SessionAd)` matches;<br>- none/one/many mapped to `Unknown`, `Found` and `Ambiguous`;<br>- the `"{hostname}/{name}"` candidates;<br>- an offline check, `offline_advertises` (`:51`), which is `matches_offline` (`live.rs:798`) with another predicate.<br><br>D1 and P1 justify the function and its file in writing, so this is not a block. | One private `Registry::resolve_where(live_pred, offline_pred)` in `live.rs`, called by both. |
| 2 | warn (carried, **half fixed this cycle**) | `crates/holler-pane-testkit/src/hub.rs:303-310`, `:220-224` | **Fixed this cycle:** the `session-held` sentence that `FakeHub` copied. Both ports now call `session_held`.<br><br>**Still open:**<br>- `FakeHub::why` (`:303-310`) is a verbatim copy of the hub's `pane_state_refusal` (`crates/holler-hub/src/panes/gate.rs:19-26`);<br>- `judge`'s `"pane {} {}, so it refuses prompts ({code})"` (`:220-224`) re-spells `gate.rs:51-55`.<br><br>The selection (`first_refusal`) is shared, but these words are not, and `conformance/hub.rs` checks only the code and the pane name. So they can drift with no test failing. | Treat it as `session_held` was treated: put the phrase on `PromptRefusal` in `holler_pane::gate`, for example `PromptRefusal::what(self) -> &'static str`, plus a `pane_gate_message(pane, refusal)` builder. The hub's `pane_gate` and `FakeHub` then both call it. |
| 3 | warn (carried, unchanged) | `crates/holler-hub/src/talk.rs:233-238`; `crates/holler-hub/src/interrupt.rs:117-121` | **The advisory fast path is spelled twice:** the hold's `would_refuse`, then `pane_gate` under `(Some(panes), Some(id))`. Each spelling copies `send_prompt`'s order (`circuit/dispatch.rs:113-114`), which is the order that makes Acceptance 4's "an operator hold refuses first" hold. | One helper beside `admit`, for example `talk::fast_gates(..)`, which `admit` calls and `interrupt` calls with `grant: None`. |
| 4 | warn (carried, unchanged) | `crates/holler-cli/src/pane/wiring/hub.rs:216-223` | **`failed()` repeats the closure inside `store.rs::call`** (`store.rs:354-360`): `timed_out`, then `Timeout { op }`, else `hub_unavailable`. The brief limited `store.rs` to "visibility of four items only", so this was the closest reuse available. | A follow-up: make it `store.rs`'s `pub(super) fn exchange_failed(..)`, and have `call` use it too. |
| 5 | warn (carried, **extended this pass**) | `crates/holler-hub/tests/driven_support/mod.rs:55`; `crates/holler-cli/tests/pane_verbs/driven/{order.rs:299-309, evidence.rs:36-46, first.rs:53}` | **Test hygiene; all of it is T-owned, and none of it was added this cycle.**<br><br>**As in the first pass:**<br>- `driven_support::id(n)` is `holler_pane_testkit::fixture::session_id(n)` typed again (D8 and Acceptance 15 make `session_id` the one fixture);<br>- the builder for "a healthy pane on `sor`, SHOWN `sor`, DRIVEN `d`" is written in seven places over `sample_pane`.<br><br>**Noticed this pass, from T-red (`949115e`):** the check that the plan moved nothing is written three times in sibling modules of `pane_verbs/driven`:<br>- `assert_nothing_moved` in `order.rs` and in `evidence.rs`, the same apart from one message;<br>- `assert_refused_in_the_plan` in `first.rs`, a superset that also checks that no prompt was sent. | Replace `id` with `session_id`.<br><br>Add one `observed_pane(name, sor, driven)` builder to the test kit's `fixture`.<br><br>Move one `pub(crate) fn assert_nothing_moved` into the parent `pane_verbs/driven.rs`, which already holds the shared `pub(crate)` helpers (`body_name`, `drive`, `body`, `advertised`). `first.rs` then calls it and adds its prompt check. |
| 6 | warn (carried, updated) | `crates/holler-pane-testkit/src/hub.rs` (310 lines) | **The brief's "Where the code goes" says "under 250".** This cycle took it down from 317 to 310. It is not a lint failure; the limit is 900. Fixing finding 2 trims it further. | Already recorded at S (scope check); nothing more is needed. |

No duplication was introduced this cycle, and the extension is clean. The cycle also introduced no architectural
drift.

## Notes for F

None are required: the verdict is PASS.

## Notes for O

- **Findings 1 to 4 and the rest of 2** are small production refactors, a few lines each, and none changes behaviour.
  They are the same follow-up set the first pass named, and S listed them as follow-ups (advisory 3). Finding 2 now
  has a template in this story: `session_held` is how its first half was folded.
- **Finding 5** is T's test hygiene. Its new item, the three "nothing moved" helpers, is cheap to fold if another
  repair pass happens.
- **Out of scope but noticed (already tracked, not duplication):**
  - **The guide overstates `session-held`.** `docs/pane-control.md:290` says `session-held` "names the `holler release
    SESSION` that lifts it". Two cases give the fallback, which names no session:
    - a failed roster read, as in round 1;
    - an advertised name outside the routable-name grammar, which is new this cycle.

    F and T both flagged this as non-blocking. It is S's to weigh.
  - **The branch must be updated before merge.** The local `origin/main` ref is still `6eff9cc` (#853), so S's
    advisory 1 stands: the conflict is in ADR-0021 "Decisions taken", and #839's items renumber after #853's. I did
    not fetch.

## Patterns referenced

- **The builder's precedents:** `crates/holler-pane/src/driven.rs` (`not_driven`, `named`) and
  `crates/holler-pane/src/findings.rs` (`switch_command`, `parse_session_id`, `quoted`).
- **The held texts compared:** `crates/holler-cli/src/hold_cmd.rs` (`held_refusal`, the exit-4 text) and
  `crates/holler-hub/src/panes/gate.rs` (`pane_state_refusal`, `pane_gate`).
- **The failed-exchange mapping:** `crates/holler-cli/src/pane/wiring/store.rs` (`call`, `timed_out`,
  `hub_unavailable`).
- **The test harnesses:** `crates/holler-cli/tests/support/{mod.rs, hold_rig.rs}`;
  `crates/holler-cli/tests/driven_e2e_test.rs` (`Rig`); `crates/holler-cli/tests/pane_verbs/driven.rs` and its
  `driven/` children.
- **The resolvers:** `crates/holler-hub/src/live.rs` (`resolve_session`, `matches_offline`) and
  `crates/holler-hub/src/live/target.rs`.
