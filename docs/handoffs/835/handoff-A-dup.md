# Handoff-A-dup: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`  (anti-duplication gate, third pass: after F cycle 5, the merge of `origin/main` after #857)

**Date:** 2026-10-12 (05:25 MDT)
**Branch:** `issue-835-implementation`
**Diff base:** `b76f4d2` (`origin/main` after `git fetch` at 05:21 MDT; it is also the merge base)   **Diff head:** `6787d99`
**This cycle's diff:** `git show --remerge-diff 59d8d59`. That is F's two-parent merge commit (`cea1c17` and `b76f4d2`),
and the remerge diff shows F's resolutions beyond git's automatic merge. T-green's `6787d99` changed only handoffs.
**Reuse map:** `docs/handoffs/835/handoff-A.md` (pass 4: "Reuse map" and its list of parallel paths to flag)
**Verdict:** PASS

This handoff replaces the second A-dup's (PASS, at `2156b5f`).

## Summary

PASS: 0 `block`, 2 `warn`. The merge goes the right way. #857 put this story's port method, constant, rule, fake,
conformance case 10 and forwards on `main`. The PR now extends `main`'s copies and carries no second copy.

- **The PR's diff fell from 37 files to 21, outside the handoffs.** The 16 files that left it each equal `main`'s
  (`git diff --quiet origin/main HEAD`): the tmux adapter, `FakeHost`, the host suite and its `Mutant`, `wiring.rs`, the
  forwarding pins and the test rigs' linked hosts. I checked every line the branch had added in them, and each one is
  on `main`, except three. Those three took `main`'s wording where S's table said to: two doc passages and one import
  list. No test pin of this story was lost.
- **S's three silent duplicates are gone.**
  - `pane.rs` has one `CLAUDE_CODE_COMMAND` (`:232`) and one `is_claude_code_command` (`:244`), at `main`'s place.
  - ADR-0021's decisions run 1 to 31, each number once. #835's two decisions are now 30 and 31.
  - "Deferred to named stories" has one adoption entry.
- **No duplicated text.** None of the PR's non-test files has a long line twice where `main`'s copy of that file has it
  once (`sort | uniq -d`, compared with `main`).
- **The code of the feature has not changed since the first A-dup.** The merge changed only lines `main` wrote:
  `HarnessEndpoint::Port` in `observe.rs` and `Some(port)` in `assess.rs` and the rig. Every other change this cycle is
  prose.
- **W1 is new: the merge made one runtime message of this story false.** Its subject is records, not duplication.
  **W2** is carried over, and is test-only.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane/src/import/assess.rs:104-105` | **New, from the merge: the `harness-claude-code` message says something false.** Every pane the import leaves alone shows the message "its tmux session runs Claude Code, which Holler cannot drive yet, so the import leaves the pane alone".<br>Since #857, Holler drives Claude Code. `main` says so in three places:<ul><li>`unmanaged.rs:68-69`: "Holler drives every `HarnessKind` a record can hold, OpenCode and, since #831, Claude Code";</li><li>the CHANGELOG, `:11-13`: "the pane verbs drive it";</li><li>ADR-0021 section 14, `:1063-1064`.</li></ul>What still waits is the import's adoption step (section 15, item 17).<br>This is the same false claim that brief round 5 and S's item 2 cover. S's line list and F's `lands`/`831` grep missed it: it is a runtime string, and it says "cannot drive".<br>**The operator reads this message in the cutover's dry run**, in text (`  info harness-claude-code: ...`) and in JSON (`message`).<br>No test pins the message itself: `harness.rs:163` checks only the line's prefix. It is not a parallel path. | One string, for S to rule on, with the same fix as S's item 2. For example: "its tmux session runs Claude Code, which the import does not adopt until #831's adoption step lands, so it leaves the pane alone". Keep it free of observed text, as now.<br>**Optional, the same cause:**<ul><li>`crates/holler-cli/tests/pane_verbs/import/harness.rs:9`, "naming #831 until that story lands": write "until #831's adoption step lands" (test-only, T's).</li><li>ADR-0021 `:1023`, "which #831 can reuse", is still true. #831's launch now uses the predicate (`process-in-use`, section 15 item 2), so the line could say so.</li></ul> |
| 2 | warn | `crates/holler-cli/tests/pane_verbs/import/rig.rs:258` | **Carried from the first and second A-dup. This cycle did not change it, and it is test-only.**<ul><li>`ImportRig::run_with_host` copies `run_with_store` (`:237`) line for line. Only the port it replaces differs.</li><li>A smaller twin: `text_row` (`tests/pane_integration/observe.rs:279`) is the import rig's `text_line` (`rig.rs:528`), in another test binary.</li></ul> | Optional, T's, and this PR does not need it. One private rig method that takes the port overrides could serve both. `text_line` could move to `verb_harness`, which both test binaries include through `#[path]`. |

No duplication of production code. The extension is clean, and the merge added no drift in code.

## This cycle, against the map

| The map's item | On the merged tree (`6787d99`) | Parallel path? |
|---|---|---|
| **The rule.** One public constant and one predicate in `pane.rs`, beside `HarnessKind`. Nothing else compares a command name with `"claude"`. | **One of each, at `main`'s place.** The PR's diff of `pane.rs` is only the predicate's three-limit doc paragraph (8 lines).<br>There are two callers:<ul><li>the import, `observe.rs:131`;</li><li>`main`'s launch check, `tx_launch/process.rs:65`.</li></ul>Every other `"claude"` in `src` is `HarnessKind::Claude`'s name or a protocol harness id. | none |
| **The port read, the adapter, the fake and its pins, the forwards and their pins.** | **Now `main`'s, byte for byte.** 16 files left the PR's diff. Every `impl HostPort` (12) answers `foreground_commands`, and every `impl HerdrPort` (11) answers `observe_pane`. The `compile_fail` doctests are not counted. The wrappers forward to the inner port. `Unwired` and the ports test's stubs have no inner port. | none: the PR extends `main`'s objects |
| **The observation.** The chain in `observe()`: one read after `ps`, on the same per-pane thread, with `tmux_finding` and `failed_message`. | **Unchanged, apart from `main`'s lines.** There is one scaffold and one `failed_message`, and `tmux_finding` maps both reads.<br>`read_tmux` (here) and `main`'s `refuse_in_use` both combine `foreground_commands` with the predicate. Their errors mean different things:<ul><li>the import turns every read error after `ps`, `pane-not-found` included, into its own finding;</li><li>the launch takes `pane-not-found` to mean that nothing runs.</li></ul>A shared helper would hold one `.any(...)` line, which would be over-abstraction. | none |
| **The outcome and the findings.** `Outcome::left_alone`, with a constructor beside `not_opencode`; `Action::Skipped`; two kinds in `ImportFinding::ALL`. | **Unchanged, apart from `HarnessClaudeCode`'s doc line.** There is no new `Action` or `Level`. | none (see W1 for the message) |
| **The switch.** Step 1 of `Input::read`, `usage()` and `not_implemented_message`. | **The code is unchanged.** Only the help, the flag's doc, the module doc and `CLAUDE_CODE_STORY`'s doc changed. | none |
| **`observe_pane` is required (R3).** | The PR's diff of `ports.rs`:<ul><li>module docs;</li><li>two `compile_fail,E0046` doctests (`main` has none for either method);</li><li>the removed default.</li></ul>The R3-3 `pane list` pins over the production chain are unchanged. | none |
| **The records** (ADR-0021, the guide, the CHANGELOG, ADR-0003). The map flags a second findings table, a guide section that restates section 13, and changes to `main`'s files beyond the named sentences. | **Edited in place:**<ul><li>the guide has one findings table, with one row for each code;</li><li>section 10's fact is the branch's text, and it links section 13 as the contract;</li><li>section 12 has one adoption bullet;</li><li>the CHANGELOG has one #835 entry, which no longer calls the method new.</li></ul>**`main`'s text, changed only as S listed:**<ul><li>the conflict hunks;</li><li>section 1;</li><li>section 12's #835 bullet;</li><li>the adoption sentence in "Deferred"'s #850 bullet;</li><li>section 14's bullet, now `main`'s.</li></ul>**Two sections describe the method.** Section 13's "Decided (#835)" and `main`'s section 15 item 2 both do. Item 2 points to "#835's amendment", and its "As built" lays down this resolution: the story that lands second keeps the API and code, and resolves the doc comments. | none |

**Beyond the map:**

- **Hub and protocol.** No hub, body, protocol or golden file is in the PR's diff.
- **File sizes.** The largest `.rs` file in the PR's diff is `ports.rs`, at 621 lines; none is above 800. The guide is
  986 lines. It is markdown, so the 900 guard does not apply.
- **Names.** No added line names a personal host, account or network (grep).
- **The merge.**
  - `59d8d59` has two parents.
  - `git merge-tree --write-tree HEAD origin/main` is clean (tree `9fd0f25`).
  - `origin/main` is still `b76f4d2`, and it is an ancestor of `HEAD`.

## Notes for F

None: the verdict is PASS.

## For S and the operator (outside this gate)

1. **W1 belongs with S's cycle-3 item 2 and with F's known issue 1.** All three are sentences written before #857 that
   the merge left behind. S decides whether W1 goes in this PR: it is one string, and no test pins it.
2. **The rule's limits are now stated for two uses.** The import's limits are in the help, the guide's section 10, the
   CHANGELOG, ADR-0021 section 13 and `pane.rs`. `main` states the launch check's limits in the guide's section 2
   (`:63-66`) and in `wiring/env.rs:107-111`. A change to the rule, such as #831's adoption step, updates both sets.
3. **The PR body should say that #857 already carries this story's port method, constant, rule, case 10 and forwards**
   (S's advisory 1). That explains why they are not in the PR's diff.

## Patterns referenced

- `docs/handoffs/835/handoff-A.md` (pass 4): the reuse map and its list of parallel paths.
- ADR-0021:
  - section 15, item 2 (`:1141-1158`: the one signal, and "As built", the rule for the story that lands second);
  - section 15, item 17 (`:1359-1379`);
  - section 14 (`:1063-1075`);
  - section 13, "Decided (#835)" (`:1006-1044`).
- Source files:
  - `crates/holler-pane/src/pane.rs:225-246`;
  - `crates/holler-pane/src/unmanaged.rs:63-73`;
  - `crates/holler-pane/src/tx_launch/process.rs:55-77`;
  - `crates/holler-pane/src/import/observe.rs`;
  - `crates/holler-pane/src/import/assess.rs:63-139`.
- `git show --remerge-diff 59d8d59`.
- The PR's diff, `git diff origin/main...HEAD`.
- Two file lists compared: `git diff --name-only d6ae539 f1d1bd0` (before the merge) and `origin/main HEAD`.
- The other handoffs: `handoff-S.md` (cycle 3, the work list), `handoff-F.md` (cycle 5) and `handoff-T-green.md`
  (cycle 4).
