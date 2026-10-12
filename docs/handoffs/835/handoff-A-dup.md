# Handoff-A-dup: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`  (anti-duplication gate, fourth pass: after F cycle 6, the `harness-claude-code` message)

**Date:** 2026-10-12 (06:05 MDT)
**Branch:** `issue-835-implementation`
**Diff base:** `b76f4d2` (`origin/main` after `git fetch` at 06:00 MDT; it is also the merge base)   **Diff head:** `05936e0`
**This cycle's diff:** `git diff ecb04db 05936e0 -- . ':!docs/handoffs'`. That covers F cycle 6 (`5dd002e`) and
T-green cycle 5 (`05936e0`). S's `bb233f0` changed only handoffs. Two files changed, with 7 lines added and 5 removed:
- `crates/holler-pane/src/import/assess.rs`: the message of `Outcome::claude_code`;
- `crates/holler-cli/tests/pane_verbs/import/harness.rs`: two sentences of the module doc.
**Reuse map:** `docs/handoffs/835/handoff-A.md` (pass 4: "Reuse map" and its list of parallel paths to flag)
**Verdict:** PASS

This handoff replaces the third A-dup's (PASS, at `ecb04db`).

## Summary

PASS: 0 `block`, 1 `warn` (carried, test-only). F changed one string literal in place, in the object the map names
(`Outcome::claude_code`, through the one `left_alone`). That is extension, not a parallel path. No other production
line changed.

- **The third pass's W1 is closed.** The `harness-claude-code` message no longer says that Holler cannot drive Claude
  Code. `git grep -n "cannot drive" -- crates/holler-pane/src/import` prints nothing. The only "cannot drive" left in
  `crates/` is `unmanaged.rs:70-71`. That is `main`'s text, and it says that case "has no case", which is accurate.
- **T-green's edit is a doc comment.** It is S's optional Advisory 3. It changes no assertion and adds no helper or rig.
- **No drift.** The message names a CLI flag, and `holler-pane` already does that in eight places. It also names an issue
  (#831), which is new for `holler-pane`'s runtime text. S's condition 3 asked for that on purpose, so it is not a
  finding. See "Beyond the map".
- **W2 is carried, unchanged and test-only.** It keeps its number from the earlier passes.

## Findings

The third pass's W1 is closed (above), so it has no row. W2 keeps its number.

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 2 | warn | `crates/holler-cli/tests/pane_verbs/import/rig.rs:258` | **Carried from the first three passes. This cycle did not change it, and it is test-only.**<ul><li>`ImportRig::run_with_host` copies `run_with_store` (`:237`) line for line. Only the port it replaces differs.</li><li>A smaller twin: `text_row` (`tests/pane_integration/observe.rs:279`) is the import rig's `text_line` (`rig.rs:528`), in another test binary.</li></ul> | Optional. It is T's, and this PR does not need it. One private rig method that takes the port overrides could serve both. `text_line` could move to `verb_harness`, which both test binaries include through `#[path]`. |

No duplication of production code. The extension is clean, and this cycle added no drift.

## This cycle, against the map

| The map's item | On `05936e0` | Parallel path? |
|---|---|---|
| **The outcome and the findings.** `Outcome::left_alone`, with a constructor beside `not_opencode`; `Action::Skipped`; two kinds in `ImportFinding::ALL`. | **Changed in place: one `&str` literal** (`assess.rs:104-106`).<ul><li>One `left_alone` (`:65`), with its three callers: `already_imported`, `not_opencode` and `claude_code`.</li><li>`ImportFinding::HarnessClaudeCode` is built only at `:110`.</li><li>No new constructor, `Action`, `Level` or finding kind.</li></ul>The message is still a constant with no observed text, passed as before. | none |
| **The switch.** Step 1 of `Input::read`, `usage()` and `not_implemented_message`. | **Unchanged.** The refusal is still built once, through `not_implemented_message(CLAUDE_CODE_STORY)` (`holler-cli/src/pane/import.rs:177-183`).<br>`holler-pane` has no logic for the switch. There is no field and no branch: in the import's files, `pane.rs` and `findings.rs`, "adopt" appears only in prose (doc comments and the message). | none |
| **The rule, the port read, the observation, the forwards and their pins, the fake.** | **Unchanged since the third pass.** None of their files is in this cycle's diff. One `CLAUDE_CODE_COMMAND` and one `is_claude_code_command`, as on `main`. | none |
| **Tests** (no new copies of D33's `observe_*` tests; no new rig). | **`harness.rs`'s module doc only** (lines 2-3 and 10-11): the header now paraphrases the port's own doc (`ports.rs:388-392`). It has the grep phrase "until #831's adoption step lands" on one line. No assertion, helper or rig changed. | none |
| **The records** (ADR-0021, the guide, the CHANGELOG, ADR 0003). | **Unchanged since the third pass.** The phrase "until #831's adoption step lands" now also appears in the message and the test's doc. Each copy is a different sentence in a different artifact, repeated on purpose so that one grep finds every place #831's step must update (S's condition 3 and Advisory 2). It is not a second table or a restated section. | none |

**Beyond the map:**

- **Layering: a CLI flag named in `holler-pane`'s runtime text.** The crate already does this in eight places:
  - `import.rs:164` (`--dry-run`);
  - `tx_apply.rs:282` (`--herdr-session`) and `:408` (`--take-over`);
  - `tx_launch.rs:228` (`--herdr-session`, `--spec-only`) and `:553` (`--spec-only`, `--profile`);
  - `tx_launch/rules.rs:133` (`--grid`);
  - `tx_switch.rs:350` (`--as-operator`);
  - `findings.rs:341` (`--fix`).

  So it is the crate's pattern. Pass 4's layering row ("`holler-pane` knows no CLI flag") still holds for the code.
- **An issue number in `holler-pane`'s runtime text, the first one.** F's design decision 4 records it, and a grep
  confirms it. This is not a finding:
  - S's condition 3 asked for it, so it is a reviewed decision, not drift.
  - The dependency runs from `holler-cli` to `holler-pane`, so the library cannot use the CLI's `CLAUDE_CODE_STORY` or
    `not_implemented_message`. Sharing one constant would put a story number into `holler-pane`'s public API. That is
    over-abstraction for a sentence #831's adoption step rewrites.
  - The two texts say different things. The CLI's refusal says the flag is "not implemented (story #831)". The finding
    explains why a pane gets no record.
  - S's Advisory 2 lists the message among the places #831's step updates.
- **Repetition.** I compared long lines that repeat within each changed file against the third pass:
  - `assess.rs`: 2, the same as `main` and the third pass;
  - `harness.rs`: 11 test assertion lines, the same as the third pass.
- **Sizes.** `assess.rs` is 435 lines and `harness.rs` 341. The largest `.rs` file in the PR's diff is still `ports.rs`,
  at 621.
- **Hub and protocol.** No hub, body, protocol or golden file is in the PR's diff.
- **Names.** This cycle's added lines name no personal host, account or network (grep). `HEAD`'s tree has no
  home-directory path naming the local account.
- **The PR and the merge.**
  - Outside the handoffs, the PR's diff is still 21 files: 952 lines added and 124 removed. The third pass counted 950
    added; the 2 extra lines are this cycle's.
  - `origin/main` is still `b76f4d2` (`git fetch` at 06:00 MDT), and it is an ancestor of `HEAD`.
  - `git merge-tree --write-tree HEAD origin/main` exits 0 (tree `81c78d9`).

## Notes for F

None: the verdict is PASS.

## For S and the operator (outside this gate)

1. **S's cycle-4 item 1 is done in place. No other production line changed.** The only other change since S's audit is
   T-green's edit to `harness.rs`'s module doc (S's Advisory 3). It is test-only and changes no assertion.
2. **One more place for #831's adoption step to update** (add it to S's Advisory 2): `crates/holler-pane/src/reconcile.rs:449`.
   - That is `main`'s `unregistered-herdr-pane` message: "... is in no pane record; no holler verb adopts a pane".
   - The guide tells the operator that a pane the import leaves alone shows up as that finding. The import's message now
     says that the import adopts such a pane once #831's adoption step lands.
   - Once that step lands, the doctor's sentence is false for those panes. The file equals `main`'s and is outside this
     PR.
3. **Carried from S, unchanged.**
   - Five older unpushed commits carry a home-directory path that names the local account: `dc64e11`, `cea1c17`,
     `59d8d59`, `6787d99` and `ecb04db`. This cycle's commits add none. Whether to rewrite them before the first push is
     the operator's decision (S's Advisory 1).
   - The PR's title and body, the AI disclosure, and a note that #857 already carries this story's port method,
     constant, rule, case 10 and forwards (S's Advisory 4).
   - W2 above is optional and T's.

## Patterns referenced

- `docs/handoffs/835/handoff-A.md` (pass 4): the reuse map and its list of parallel paths.
- Source files:
  - `crates/holler-pane/src/import/assess.rs` (in full);
  - `crates/holler-pane/src/import/findings.rs:64-80`;
  - `crates/holler-cli/src/pane/import.rs:1-200` (module doc, help, `CLAUDE_CODE_STORY`, `Input::read`);
  - `crates/holler-cli/src/output.rs:253-262` (`not_implemented_message`);
  - `crates/holler-pane/src/ports.rs:380-397` (`foreground_commands`'s doc);
  - `crates/holler-pane/src/reconcile.rs:444-456`;
  - the flag-naming runtime strings listed under "Beyond the map".
- Git:
  - `git diff ecb04db 05936e0 -- . ':!docs/handoffs'` (this cycle's diff);
  - the PR's diff, `git diff origin/main...HEAD`.
- The other handoffs: `handoff-S.md` (cycle 4, the work list), `handoff-F.md` (cycle 6) and `handoff-T-green.md`
  (cycle 5). Also this run's `decisions.md`, from the third A-dup on.
