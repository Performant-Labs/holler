# Handoff-A: Phase 3 - #646 part 1 of 3 (646a) `holler pane park` and `holler pane unpark`  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-646-implementation (worktree `.claude/worktrees/0646-park-close-routing`, head `e957a8d`, base `ce12cdb`)
**Brief reviewed:** `docs/handoffs/646-brief.md` (as amended in `e957a8d`)   **Reuse map:** the brief's "Reuse and analogous-feature map", under "Files" (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS: no block, six warns. The plan extends the right objects at the right layer:

- `PanePark` and `PaneUnpark` grow their own fields in their own files and keep `ProfileOpt` flattened.
- The verb copies doctor's `run`/`pass`/`emit` shape.
- Every membership decision goes through `ProfileScope::resolve`.
- The record is written by one `cas_put` at the generation it was read at, the way reconcile records.
- It reuses `findings::quoted`, `now_millis` and `class_of`.
- One engine in `park.rs` serves both verbs. The precedent is `profile/list.rs`'s `count`, shared "because the frozen
  `profile/mod.rs` admits no new module".

ADR-0021 section 5 lists the `holler-pane` engines as a closed set, and park is not on it, so a CLI-side engine matches the
crate rules. No frozen file, hub file or manifest is touched. The prompt gate stays out (646c, at `send_prompt`).

The warns:

1. A second hold-reason guard sits beside `holler_hub::holds::sanitize_reason`, with the same 200-character cap and the
   opposite policy, and the brief does not name it.
2. The ADR paragraph states the guard as if the record enforced it. The hub does not.
3. The ADR paragraph omits the exception to the reconcile-step rule of sections 8 and 12.
4. The scoping arms copy the private `reconcile::resolve`.
5. The Reuse map misses #662's rig, the closest one on main.
6. No size estimate or split point is given for the largest test file.

Warns 2 and 3 change AC 12's pinned ADR text, so they need a brief amendment or a follow-up. The other four fit inside the
existing ACs.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Decision 5 (the text guards, "longer than 200 characters"); the Reuse map has no row for the guard | pattern consistency (validation) | **A second reason guard, unnamed.**<br>- `holler-hub/src/holds.rs` already guards the other hold's reason: `MAX_REASON_CHARS = 200` (lines 66-69) and `sanitize_reason` (lines 213-223). Its rule: trim, drop control characters, cut at 200, so that "an operator's text is echoed to other people's terminals". Its doc says "Longer text is cut (not refused)". `holler-cli` already depends on `holler-hub`, and `holds` is a `pub mod`.<br>- Decision 5 adds a second guard with the same cap and the opposite policy (refuse). Neither Decision 5 nor the Reuse map names the existing one.<br>- Why warn and not block: the refuse policy is the pane domain's own pattern. `ProfileName::parse` and `Actor::parse` refuse blank, over-long and control-character text (ADR-0021 lines 59-63), and ADR-0021 line 44 keeps the park hold apart from the prompt hold. So the codebase is split by domain, with no single dominant pattern.<br>- The visible cost: an operator sees `holler hold S --reason <201 chars>` accepted and cut, and `holler pane park P --reason <201 chars> ...` refused. Both reasons later sit side by side on the roster (#648). | Add a Reuse-map row that names `sanitize_reason` and `MAX_REASON_CHARS` and says why park does not use them: it refuses, as the pane domain's guards do, because the record is shared state that every reader prints. In `park.rs`, give the cap a named constant whose doc cites `holler_hub::holds::MAX_REASON_CHARS` as the matching prompt-hold cap, rather than a bare `200`. Do not import the prompt-hold constant into a pane verb (ADR-0021 line 44). |
| 2 | warn | Decision 12's ADR paragraph: "The reason and the release condition are one line each, hold no control character, are not blank and are at most 200 characters (`usage` otherwise)." (AC 12 pins the text) | ADR (accuracy); cross-cutting (where validation lives) | **The ADR states as a record property what only the verb enforces.**<br>- `Hold::Parked` holds two plain `String`s (`holler-pane/src/pane.rs:167-177`).<br>- The hub's `pane/cas_put` checks membership and the generation, then stores whatever `hold` it is given (`holler-hub/src/panes/handlers.rs:105-116`, `panes/store.rs:163-187`). Any other control-socket client can write a control character.<br>- In ADR-0021 section 1 a guard that the ADR states is a type guard that also runs on decode: for `EnvVarName`, "Because the guard runs on decode, a record that holds a value cannot load" (lines 75-76). Read in that context, this sentence is a guarantee. A later reader (#643 `get`/`list`, #648 roster) could rely on it and print the text unescaped. | Reword Decision 12, and so AC 12: "`park` refuses (`usage`) a reason or release condition that is blank, holds a control character or is longer than 200 characters. The record does not enforce this (the hub stores what it is given), so a reader still escapes them." A guarded newtype inside `Hold` would be an amend-first contract change: a follow-up at most, not this part. |
| 3 | warn | Decision 7 (a failed write's message: `<pane>: <error text>` plus the suffix); Decision 12's ADR paragraph | ADR (consistency with sections 8 and 12) | **The reconcile-step rule is not followed, and the ADR is not told.**<br>- ADR-0021 section 12 (lines 461-462): "A verb that times out stops, compensates as section 8 says, exits 1 with `timeout`, and prints the reconcile step." Section 8 (lines 274-276) says the same for a record write that hits `generation-conflict`.<br>- Park and unpark print no reconcile step in either case.<br>- The behaviour is right. There is no live act. `holler pane doctor` neither reads nor writes `hold` (`reconcile.rs:26-40` lists what it records), so naming it would mislead. Decision 4 makes a rerun safe.<br>- The gap: neither Decision 7 nor the paragraph says this, so the merged ADR would still say every verb that times out prints the step.<br>- Also: a timed-out `cas_put` may have landed (#663's brief makes its own timeout messages say so), and the suffix cannot tell which. | Add one sentence to Decision 12's paragraph: "Having no live act, they print no reconcile step on `generation-conflict` or `timeout` (sections 8 and 12): a timed-out write may have landed, and running the verb again reports the pane as `already parked` (or `not parked`)." Note the same exception in Decision 7. |
| 4 | warn | Decision 3 (scoping); the Reuse map's "Scope" row | duplication (a pattern copied, with no shared helper) | **The scoping arms copy `holler_pane::reconcile::resolve`** (`reconcile.rs:219-243`): `(Some(P), pane) => scope.resolve(P, pane)?.panes`, and `(None, Some(n)) => pane_store.get(n)?` with `PaneNotFound { what: n.to_string() }`.<br>- Park cannot call it: the function is private, in a frozen crate.<br>- The CLI has no shared helper either. `profile_scope.rs` on main is doc-only, and #663's `StoreScope` adds none.<br>- The copies will spread: #643's plan writes the same `get`-alone arm (`643-brief.md:1013-1014` at `2e7f221`), and 646b and #645 will need it too.<br>- Not a block: the membership rule, the part that matters, stays with `ProfileScope::resolve`, as the map says. | Name `reconcile::resolve` in the Reuse map as the pattern copied. Keep park's arms identical in behaviour: the same `PaneNotFound { what: name.to_string() }`, so AC 6's `error: pane not found: demo-c9r9` matches doctor's wording. Add a follow-up: one public "panes in scope" helper that doctor, `get`, park, close and switch/reset all call. It could go beside `ProfileScope` in `holler-pane` (amend-first), or in `profile_scope.rs` with #663's agreement. |
| 5 | warn | Decision 11 (the rig); Risks ("beside doctor's and #644's planned one"); the Forward-compat row on rig consolidation | duplication (test helpers) | **The Reuse map misses the closest rig on main: #662's `crates/holler-cli/tests/profile_verbs/rig.rs`** (160 lines).<br>- It runs over fakes only, with no live world. It has `Rig::new(panes, profiles)`, `ports()`, `run(argv, format)`, `assert_no_adapter_call()`, `assert_failure(both, code, exit)`, `pane(name)` and `member(name, profile)`.<br>- Its `run_both(seed, argv)` runs both formats on fresh rigs, asserts equal exit codes and calls `check_envelope`: exactly AC 10's need.<br>- It cannot be reused as is. It belongs to another test target, owns its stores by value and wires `Unwired` as the scope. Park needs a `FakeProfileScope` over `Arc` stores, with AC 8c's wrapper under it. So per-story rigs remain the epic's pattern (#644's plan review graded the same point a warn).<br>- The count: main would hold three rigs (doctor's, #662's, park's), and #643 and #644 plan two more. The consolidation the brief cites is #644's decision 25, on an unmerged branch, with no issue and no owner. | Name `profile_verbs/rig.rs` in the Reuse map. Give park's rig its names and shapes (`Rig::new`, `ports`, `run`, `run_both`, `assert_failure`, `pane`, `member`). Add only what park needs: the scope, `seed`/`record`, `ports` over a given pane store (AC 8c) and the AC 9 check. The consolidation then becomes mechanical. File it as this story's own follow-up, with an owner, instead of citing another story's unmerged decision. |
| 6 | warn | Files: `crates/holler-cli/tests/pane_verbs/park.rs` (the only file with no size estimate) | size and structure | **The largest test file has no size estimate and no split point.**<br>- `scripts/lint.sh:43-52` warns at 600 lines and fails at 900 for every `.rs` file under `crates/`, tests included.<br>- This file carries AC 1-10 and 7e for park: two formats per case, exact messages, AC 7d's dozen guard cases, and AC 8c's wrapper store with its five edge cases.<br>- Doctor's comparable suite is split across four files: `doctor.rs` (501 lines), `doctor/surface.rs` (586), `doctor/rig.rs` (409) and `doctor/read_only.rs` (147).<br>- This repo's overlay asks the plan to name where new code goes. | Give an estimate, and name the split now: if `park.rs` would pass about 600 lines, AC 8's failure cases and the wrapper store go in `tests/pane_verbs/park/failures.rs`, declared by `mod failures;` in `park.rs`. That is doctor's layout, which the brief's own module-layout note already explains. |

### Also checked (no finding)

- **Engine placement.** ADR-0021 section 5 (lines 179-182) lists the `holler-pane` engines: `tx_launch`, `tx_switch`,
  `tx_apply`, `reconcile`/`findings`, `profile_snapshot`/`profile_diff` and `import`. Park is not on the list. No consumer in
  the Forward-compat table calls a park engine; they read `hold`. A `pub(super)` engine in `park.rs` that `unpark.rs` calls
  is the `profile/list.rs` `count` pattern (lines 8-9, 91-99), and ruling 2 is about ownership: both files are #646's.
- **Write path and concurrency.** One `cas_put` of the whole record, at the generation read in the plan, never retried, and
  the run stops on a conflict. That is reconcile's record rule (`reconcile.rs:36-40`) and ADR-0021 section 8. The hub
  replaces only the generation (`panes/store.rs:163-187`). Its membership check cannot fire, because park never changes
  `profile`. A profile of N panes costs one `resolve` and N writes, each one registry save under the pane lock. That is
  operator-initiated and bounded, never per heartbeat.
- **Choke point.** No prompt gate is added. `pane.rs:160-164` and the issue put it at `send_prompt`, in 646c (Decision 10).
- **Codes and output.** Only closed codes are raised, and ADR-0021 row 341 stays as it is. Exit codes come from `class_of`.
  A usage error is `PaneError::Usage`, as `args.rs:144-150` does it. JSON carries `Hold`'s own serde form (no hand-built
  mirror) and the raw values. Text quotes with `findings::quoted`, as doctor does. Note that `quoted` cuts at 64 characters,
  so a reason of 65 to 200 characters prints cut in text mode (JSON carries all of it), which is doctor's rule for untrusted
  values. #643 plans a `list::text_value` for stored text. If it merges first, Phase 7 should ask whether park uses it.
- **Surface.** The new ADR 0003 rows and fixture lines pass `process/docs_rows.rs`: one row per verb, no two stories'
  rows adjacent, and every line under `# #646`. They also pass `docs_cli_test`'s normalisation: it cuts the row at the first
  double space and drops `[...]`, leaving `holler pane park --reason TEXT --release-when WHEN`, which parses because `PANE`
  is optional. `flags.rs`'s `accepted()` tolerates `MissingRequiredArgument`. The `--spec-only` case stays
  `UnknownArgument`, which AC 11d checks. `docs/protocol/v2.md` carries no pane rows, so the brief's docs list is complete.
- **ADR merge hygiene.** #643's brief does not edit ADR-0021, #644 stays out of section 3, and #663 edits sections 1, 2
  and 8. The insert after line 130 is clear of all three. The heading `**Park and unpark as built (#646).**` matches #644's
  planned `Launch and relaunch as built (#644)`.
- **Upstream.** `origin/main` has moved two commits past `ce12cdb` (#702, #705). Both touch adapter crates only, and none
  of the plan's files changed.
- **AC 8c's wrapper.** The CLI tests have no existing `PaneStore` wrapper that fails on the Nth call (the test kit's
  `Mutant` stores live in its own conformance tests), so a test-local one is not a near-copy.
- **The cited evidence.** Every file and line the brief quotes from `ce12cdb` matched the source when checked, including
  ADR-0021 lines 126-130, 272-273, 332-341 and 379-398, ADR 0003 lines 54-55, 92 and 94, `args.rs:147`, `output.rs:282`,
  `pane.rs:160-177` and `fixture.rs:65`.

## Notes for O

The verdict is PASS, so this section is advisory: in the automated path the run continues to T with the brief as it is.

- **Warns 1, 4, 5 and 6** fit inside the existing ACs. F names the cap and keeps the scoping arms identical. T mirrors #662's
  rig names and splits the test file if it grows.
- **Warns 2 and 3** change the ADR paragraph that AC 12 pins word for word, so F cannot take them without departing from
  the brief. Either amend Decision 12 and AC 12 before T starts, or file one ADR-wording follow-up. 646b's brief also edits
  ADR-0021 and could carry both sentences.
- **Follow-ups to file:**
  - One shared "panes in scope" helper (warn 4).
  - The rig consolidation, with an owner (warn 5).
  - Optionally, a guarded type for the hold texts (warn 2).

## Patterns referenced

- `crates/holler-cli/src/pane/doctor.rs` and `crates/holler-cli/src/profile/list.rs:8-9,91-99`: the verb shape, and
  sharing between verb files under a frozen `mod.rs`.
- `crates/holler-pane/src/reconcile.rs:26-40,219-243`: the scoping rule and the record-write rule.
- `crates/holler-hub/src/holds.rs:66-69,213-223`, and `crates/holler-hub/src/panes/handlers.rs:105-116` with
  `panes/store.rs:163-187`: the existing reason guard, and what the hub enforces on `pane/cas_put`.
- `crates/holler-cli/tests/profile_verbs/rig.rs` and `crates/holler-cli/tests/pane_verbs/doctor/rig.rs`: the rigs on main.
- `docs/adr/ADR-0021.md`, sections 1, 3, 5, 8, 9 and 12.
