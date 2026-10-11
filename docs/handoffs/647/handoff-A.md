# Handoff-A: Phase 3 - #647 part 2 (worktree `0647-agent-cannot-dispatch`)  (up-front plan review)

**Date:** 2026-10-11
**Branch:** issue-647-implementation (worktree `.claude/worktrees/0647-agent-cannot-dispatch`, head `dcfc01e`, one commit behind `origin/main` `43703ba`, which touches only `scripts/`)
**Brief reviewed:** `docs/handoffs/647-brief.md` (unchanged since #701 merged it; `git status` clean)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)", lines 924-939 (no `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

> This file replaces part 1's `handoff-A.md` on this branch only, because this run's handoff directory is part 1's
> (see W-8). Part 1's Phase 3 review is at `git show e612878:docs/handoffs/647/handoff-A.md`.

## Summary

BLOCK, with two blocks. The brief given to this run is part 1's plan (the 12-kind reconcile engine and `holler pane doctor`).
That plan merged as #701 at 18:56 MDT on 2026-10-09. It does not plan the work that is still open on #647: the two
operator-confirmed amendments, `agent-cannot-dispatch` (confirmed at 17:40 MDT) and `unattached` (added at 19:03 MDT,
after the merge). Taken literally on today's `main`, its RED-first instructions would also re-declare and stub the merged
engine, whose public API four sibling verbs now import. Nothing can be tested or built against this brief. The warns
below are what the part-2 brief must settle so the re-review can pass. Most of them come from one fact: no frozen port
can observe either new finding.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| B-1 | block | The whole brief: header (worktree `0647-reconcile-doctor`, base `3bdd129`), Problem, Evidence, ACs 1-33, Files | scope; pattern consistency | The brief plans work already on `main`. Its evidence says `reconcile.rs` and `findings.rs` "are empty stubs" and `doctor` "answers `not-implemented`". On `dcfc01e`, `findings.rs` is 351 lines, `reconcile.rs` 444, `reconcile/observe.rs` 391, and `pane/doctor.rs` is the real verb. ACs 27-29 already hold: ADR-0003:61, ADR-0021:672 (`Decided (#647)`, `Deferred to #647` count 0), CHANGELOG:240, `stub.rs:23`, and `cli-surface.txt:155-160`. The issue's open scope does not appear: a case-insensitive count of `agent-cannot-dispatch`, `unattached`, `opencode_agent` and `#700` in the brief is `0`. | Write a part-2 brief from the issue's two open amendments (Notes for O, 1-3). Leave part 1's brief as the record of part 1. |
| B-2 | block | Test plan ("T first lands the type declarations of Decision 10 exactly ... a `reconcile()` whose body is `Err(PaneError::NotImplemented)`"); Files (`findings.rs` "fill", `reconcile.rs` "fill", `doctor.rs` "replace") | dependency direction; drive-by edit of code other stories now consume | The merged API has consumers outside #647 that Decision 10 does not list: `shown_differs` (`pane/list.rs:20`, `tx_launch.rs:41`), `quoted` and `embedded` (`tx_launch.rs:38`, `pane/{launch,close,park,switch}.rs`), `doctor_command` (`pane/profile_scope.rs:31`), and `Report` and `FindingKind` (#645's `tests/pane_verbs/reset.rs:7-9`). Landing Decision 10 "exactly" breaks the build of #643's, #644's, #645's and #646's code. Stubbing `reconcile()` regresses `pane doctor` and #645's tests. Decision 10's `ObservedHealth { Wedged, Down }` would also rename the merged `--json` values `server-wedged` and `server-down` (`reconcile.rs:130-139`) on ADR 0003's versioned surface. | Base the part-2 plan on the merged API, extend-only: add variants to `FindingKind`, `ALL`, `code()` and `remedy()`, add reads to `check_pane`'s chain, and add tests in a new `doctor` submodule. RED is a failing assertion against the working engine, never a stub of it. No existing `pub` item or JSON value changes. |
| W-1 | warn (blocks on re-review if missed) | `agent-cannot-dispatch` observation source (absent) | layering; I6 | No frozen port observes any input the finding needs: the agent a turn ran as, an agent's `task` permission, or whether the project ships an orchestrator. `HarnessPort` (`ports.rs:183-230`) has the eight #637 methods plus #645's two seam methods. Reconcile "never infers state from files" (`reconcile.rs:3`; ADR-0021:208, I6). Epic decision 8 says the Aftersight orchestrator lives outside `.opencode/agents/`, so a file check would also give the wrong answer. | Add one read-only, default-armed `HarnessPort` method under an operator carve-out recorded before T. This follows #645's seam (`ports.rs:208-229`, ADR-0021 section 8 at line 401, PR #723). Its default answers "cannot tell", so the rule is skipped, as for every unobserved value, and every existing impl keeps working. Record nothing in the hub or in `Pane`: the hub is store-only (ruling 1), and `holler-pane/**` fields are #637's and #700's. |
| W-2 | warn (blocks on re-review if missed) | Order with #642 (absent) | dependency direction; parallel-agent coexistence | #642's agent part is in flight (worktree `0642-opencode-agent`). It plans that "the `HarnessPort::serve` signature gains the agent" (issue #642 comment, 20:00 MDT on 2026-10-10). A second, parallel amendment of the same frozen trait from #647 collides in `ports.rs` and in every impl: `FakeHarness`, `TestHarness` (`ports_test.rs:216`) and the wrapper doubles in W-5. #642's own new conformance case ("the first assistant message of a delivered prompt reports that agent") needs the same read #647 needs. | The brief names the order: #647's method lands after #642's `serve` change, or one recorded amendment carries both. The real read is one function in `holler-adapter-opencode` (#642's blast radius), used by #642's conformance case and by #647's port method alike, never two readers of the same data. |
| W-3 | warn (blocks on re-review if missed) | `unattached` observation and remedy (absent) | layering; pattern consistency | `HostPort` (`ports.rs:162-174`) has no attached-client read, and `FakeHost` has none. "Against the Herdr pane the record names" also needs Herdr to say what a pane displays. `HerdrSnapshot` (`ports.rs:99-101`) has only `{session, workspace, pane_id, grid}`, and ADR-0021:741 already says it "waits for a contract amendment before #647 and #650". None of the cures the issue names fits the remedy table. `pane launch` refuses a registered pane (`pane-exists`, `tx_launch.rs:47-49`). `pane relaunch` fails while the old server still answers (ADR-0021:430, until #695). The operator's own tmux-attach tool, which the issue names, is not a Holler verb (`findings.rs` module doc: no remedy is a raw tmux, Herdr or OpenCode command), and its name appears nowhere in the tracked tree (public-repository rule). | Build `unattached` through a default-armed `HostPort` read under the same carve-out, or split it to its own issue with the operator's OK (the #665 precedent). Either way, state whether the rule is "any client attached" (the issue's acceptance) or "attached in the named Herdr pane", which needs the `HerdrSnapshot` amendment. Give a remedy that exists today, or `null` with the reason in the message (part 1's C-5). Never name the operator's tool in code, tests or output. |
| W-4 | warn | Remedy `relaunch --agent KEY` | cross-cutting: output safety | Part 1's remedy rule (Decision 8a: constant words plus a `PaneName`, no space, metacharacter or leading `-`) does not hold for `AgentKey`. `AgentKey::parse` (`argv.rs:150-160`) accepts a leading `-` and capitals, so `--spec-only` is a valid key. If KEY comes from the server's agent list, it is adapter text. `relaunch` is spec-editing with `--profile` (ADR-0021:154). | Build it only in `FindingKind::remedy` (`findings.rs:121`, the one table). Render `--agent=KEY`, or answer `null` with the reason for a key that starts with `-`. Keep part 1's no-`--profile` rule. Parse-test it as AC 26 did (`verb_harness::parse::try_parse`). |
| W-5 | warn | Reuse map: tests, fakes, transcript fixture | abstraction level; anti-duplication | The test kit has no agent, transcript or attached-client support (grep is empty), and the test kit is #638's and #684's. The precedent is a test-local wrapper double in the story's own test files ("Wrapper doubles live in these files only", `0645-switch-reset/brief.md:64`). Five already exist: `HealthGate` (`doctor/surface.rs:471`), `SeamHarness` (`switch.rs:742`), `WriterInSelect` (`switch.rs:480`), and `HookedHarness` and `LinkedHost` (`launch/rig.rs:652, 599`). | List them in the Reuse map and say extend or new for each. Put the transcript fixture under `tests/pane_verbs/doctor/`. Build every world from `doctor/rig.rs`, with no second rig. New tests go in a new submodule (`surface.rs` is 586 lines). |
| W-6 | warn | Engine extension points and invariants (absent) | pattern consistency; I5 | `findings_test.rs:12` pins exactly 12 codes, and #665 appends `profile-drift` after. "At most seven port calls" per chain (`observe.rs:6-8`, `reconcile.rs` module doc) and AC 12's read-only allowlist (`doctor/read_only.rs:72-88`) change with any new read. DRIVEN is not observable before #649 (`reconcile.rs:33-35`), so "hub-delivered turns" can only mean turns of `session_of_record`, the same stand-in #645 chose (ADR-0021 section 8). The only per-pane agent state for "opted out deliberately" is `Pane.opencode_agent` (ADR-0021:48). | The brief states each: the new codes and their place in `ALL`; the new I5 bound; the extended allowlist (reads only: "`--fix` never changes a pane's agent"); "turns of the session of record"; and opt-out as a rule over `opencode_agent`, with no new `Pane` field. `unattached` extends `check_tmux` (`observe.rs:133`), and the agent check sits beside it in `check_pane` (`observe.rs:96`). |
| W-7 | warn | `SpecField` row for `opencode_agent` | scope; ownership | #700's journal routes it to #647 (`0700-implementation/decisions.md:31, 190, 229`: "#647 owns `SpecField`/`agent-cannot-dispatch`"). #647's issue does not list it. `profile_diff.rs` is #662's shared file (`ALL: [SpecField; 15]`, read by `profile show`, #664 and #665). | Take it explicitly, in the blast radius with `profile_diff_test.rs`, or route it to #665 or #664 with an issue comment. Do not leave it unstated. |
| W-8 | warn | Run bookkeeping | naming; file structure | This run's handoff directory, `docs/handoffs/647/`, is part 1's committed record, and each phase overwrites it in place (this file already does). The brief path is part 1's `647-brief.md`. Sibling runs use their own slugs (`0645-switch-reset/`, `0700-implementation/`). | Start the fresh run with its own slug and brief path (for example `docs/handoffs/0647-agent-cannot-dispatch/brief.md`). Restore part 1's `handoff-A.md` if this branch is reused. |
| W-9 | warn | Outside brief gate, round 1 (`647-brief-result-r1.md`, 02:22 MDT today) | process | It passed this brief. It checked the brief against itself and the code it was shown, not whether the work is still open. | Run the brief gate again on the part-2 brief. Do not carry this PASS forward. |

## Notes for O

The automated run stops here (`needs-amendment`); the recovery is a part-2 brief and a fresh run, never
`resumeFromRunId`. The part-2 brief must:

1. **State the scope from the live issue.** Take the agent paragraph (amended 2026-10-09, confirmed 17:40 MDT) and the
   `unattached` paragraph (19:03 MDT). #700 has merged, so the dependency holds. Build both, or split one out with the
   operator's OK. Both are operator-confirmed, which is why part 1's S held rather than passed. Say whether the PR closes
   #647. A part-2 PR that builds only `agent-cannot-dispatch` must not close it.
2. **Get the operator's carve-out before T, and record it** (`decisions.md` and ADR-0021 section 8, as #645 part 2 did).
   It covers each port amendment: which trait, which read-only method, its default ("cannot tell"), and nothing else in
   `ports.rs`. Name the order against #642's `serve` change (W-2), and name who implements the real read (#642 in
   `holler-adapter-opencode`; #641's crate for a `HostPort` read) and who adds fake support (#684). Until then, the tests
   use wrapper doubles. This is an operator decision, not one A or F can make.
3. **Write the detection rules in terms of what the new methods return.** That covers "cannot dispatch", "ships an
   orchestrator", "recent" (how many turns of the session of record) and "opted out" (a rule over
   `Pane.opencode_agent`). Write the remedies (W-3, W-4), and ACs that include the "attached raises nothing" and
   "opted out raises nothing" negatives.
4. **Re-base the evidence and the Reuse map on `origin/main`.** Extend-only, per B-2: list the merged `pub` items and
   their outside users so nothing is re-declared. Add the wrapper doubles (W-5) and the extension points and invariants
   (W-6).
5. **Settle `SpecField`** (W-7), and use a new handoff slug and brief path (W-8). Re-run the brief gate (W-9).

On re-review, A blocks if the part-2 brief does any of these:

- reads files to observe state (W-1);
- edits a frozen port without the recorded carve-out, or edits one in parallel with #642 without a stated order (W-2);
- re-declares or stubs merged items (B-2);
- builds a remedy outside `FindingKind::remedy`, or names a non-Holler command (W-3, W-4).

## Patterns referenced

- `crates/holler-pane/src/ports.rs:162-230`: the frozen `HostPort` and `HarnessPort`, and #645's default-armed seam
  methods (the amend-first precedent, ADR-0021 section 8 at line 401, PR #723, `0645-switch-reset/brief.md:11-12, 48-64`).
- `crates/holler-pane/src/findings.rs`, `reconcile.rs` and `reconcile/observe.rs`: the merged engine, its one remedy
  table, its I6 rule and its per-pane chain.
- `docs/adr/ADR-0021.md`: I6 (line 208), `opencode_agent` (line 48), spec-editing verbs (line 154), relaunch with a
  running server (line 430), and the deferred `HerdrSnapshot` amendment (line 741).
- `docs/handoffs/647/handoff-S.md` (part 1, at `e612878`): the ADVISORY-HOLD that named this gap, and the split
  precedent of option (a).
- Issue #647's body and edit history (GraphQL `userContentEdits`: 23:38Z, 23:40Z and 01:03Z, which are 17:38, 17:40
  and 19:03 MDT on 2026-10-09); epic #633 decision 8 and its hot-spot rules; issue #642's comment of 2026-10-10 20:00 MDT.
