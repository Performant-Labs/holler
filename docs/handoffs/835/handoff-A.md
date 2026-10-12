# Handoff-A: #835 - `holler pane import` leaves panes Holler does not drive alone, with `--adopt-claude-code`  (up-front plan review, fourth pass: re-entry after T-green's BLOCK)

**Date:** 2026-10-11 (19:15 MDT)
**Branch:** `issue-835-implementation` at `25ff495` (base `2d217dd`). At 19:05 MDT `origin/main` is `bda1069`, three commits past the base: `7fc00f1` (#655 journal), `0e62b51` (#652, the operator guide, 18:13 MDT) and `bda1069` (#840, #832's review fixes, 18:16 MDT).
**Brief reviewed:** `docs/handoffs/835-brief.md` at `f8a1864`, unchanged since pass 3 (sha256 `bac2232a86462e4f...`).   **Reuse map:** pass 3's, carried below with two rows for the rework   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This pass replaces pass 3 (PASS, `49e4508`). Git history keeps passes 1 to 3.

**Why A runs again.** T-green returned BLOCK (`25ff495`), and F had reported `archChanged` for cycle 1, so the script sends the run to A before T and F rework it (`nextPhase`, case 7). The brief has not changed. So this pass does two things:

- it checks F's cycle-1 code against the plan for drift;
- it reviews the rework that T-green asks for. That rework is the first change that brings other stories' merged files into this branch.

## Summary

PASS: 0 `block`, 4 `warn`.

- **No drift in cycle 1.** F extended every object the plan named, in the layer the plan named (table below).
  - F made one judgment call (design decision 1): the server of a pane whose tmux read failed is still asked. That fits pass 3's note 4, and it changes no outcome, because the pane is blocked either way.
- **The rework needs no contract decision.** Brief section 6 ("#652's guide ... should be told of the new default and flag") and R2-3 ("work from the current `main`") already cover the merge and the guide. The brief needs no amendment.
- **The warns are about the merge and what it brings in.**
  - W1: the merge comes first, and it must be a real merge.
  - T-green named the failing test and three of #840's claims about #835. The merged tree has more stale or false text than that:
    - W2: a fourth #840 claim, in a test;
    - W3: this story's own claim that #832 reuses the rule;
    - W4: five passages of the guide beyond its findings table, and one doc comment.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | R2-1 and R2-3 ("work from the current `main`"); handoff-T-green ("F must merge `origin/main` into the branch") | sequencing; coexistence with merged stories | **Nothing in the rework can start before `main` is merged in.** The guide (`docs/pane-control.md`), its test (`crates/holler-cli/tests/docs_pane_control_test.rs`) and #840's text exist only on `main`.<br>**The script's order works against T-green's routing.** T-green gave the merge to F, but T-red runs before F in this cycle (A, then T-red, F, T-green). Without the merge, T-red can show its RED only on a scratch merge, as T-green did, and an invalid RED retries T-red up to 10 times (`T_RED_BLOCK_CAP`).<br>**The merge is clean.**<ul><li>`git merge-tree --write-tree 25ff495 bda1069` gives tree `2ff3c92`, with no conflicted path.</li><li>On that tree the workspace suite fails one test only (handoff-T-green).</li><li>Its added lines pass `gitleaks` (`gitleaks stdin`: no leaks), so the pre-commit hook will not refuse the phase commit.</li><li>The phase commit's subject passes `commit-msg`.</li></ul> | **T-red merges first.**<ul><li>Run `git fetch origin`, then `git merge --no-ff --no-commit origin/main`.</li><li>`--no-commit` leaves the commit to the script: its `git add -A` and `git commit` then record a two-parent merge commit, so T still commits nothing.</li><li>The RED is then the branch's own failing test.</li></ul>**If T-red does not merge,** F merges in the same way before its first edit (`git log --merges 2d217dd..HEAD` shows whether it is done).<br>**Never:**<ul><li>a rebase (the handoffs cite this branch's commits by SHA, and the script's phase commits would be rewritten);</li><li>a squash;</li><li>copying `main`'s files over.</li></ul> |
| 2 | warn | brief section 7; pass 3, W3 | one source of truth; records that contradict each other | **#840 (merged at 18:16 MDT) says #835 calls its registry rule, in four places.** That is one more than T-green listed:<ul><li>`crates/holler-pane/src/unmanaged.rs:66`, the docs of `unregistered` ("and so do #831 ... and #835 (import decides whether Holler drives a pane)");</li><li>`unmanaged.rs:84`, the docs of `is_unmanaged` ("The question #831 and #835 ask");</li><li>ADR-0021 section 14 (`:972` on `main`, `:1013` after the merge: "that #831 and #835 call instead of writing their own");</li><li>**the doc comment of #840's test** `the_unmanaged_predicate_is_public_and_one_rule`, `crates/holler-cli/tests/pane_verbs/unmanaged.rs:467-469` ("for the stories that decide whether Holler drives a pane (#831, #835)").</li></ul>**After the merge the records contradict each other.** This story's section 13 says the opposite, in its "Not the registry's rule" bullet (`ADR-0021.md:983`).<br>**#840's rule cannot answer the import's question.** Every pane the import observes already has no record (`import.rs:181-189`), so `is_unmanaged` holds for all of them. The two rules compose: the import decides, by what a pane runs, whether to create a record, and a pane it leaves alone has no record, so `unregistered` lists it as unmanaged. | **F, in production text, edits each sentence in place:**<ul><li>`unmanaged.rs:63-67`: drop #835 from the callers, and say that the import asks what a pane runs (`pane::is_claude_code_command`), and that a pane it leaves alone has no record, so this rule lists it.</li><li>`unmanaged.rs:84`: "The question #831 asks".</li><li>ADR-0021 section 14: "that #831 calls instead of writing its own", plus a pointer to section 13's "Not the registry's rule".</li></ul>**T, in the test's doc comment:** "(#831)".<br>**Leave #840's claims about #831 as they are;** they are #831's to settle.<br>**The PR's coexistence notes name #840 and #652.** |
| 3 | warn | R2-7 ("so #831 and #832 can reuse them") | records | **This story's own text says #832 reuses the predicate.** It does not:<ul><li>#832 closed at 15:35 MDT, and #840 merged at 18:16 MDT, without it;</li><li>neither added a second signal (a grep of every `src` finds no process or agent detection besides #835's), so brief section 7 is met trivially.</li></ul>**The three places:**<ul><li>`crates/holler-pane/src/pane.rs:146` ("the stories that adopt or show such a pane (#831, #832)");</li><li>ADR-0021 section 13, `:966-967` ("which #831 and #832 reuse");</li><li>`crates/holler-pane/tests/claude_code_rule_test.rs:3-4` ("so #831 and #832 reuse it").</li></ul>This is the same kind of error as finding 2, made by this story. | **Name #831 only, as a story that can reuse it:** "which #831 can reuse".<ul><li>F fixes `pane.rs` and the ADR.</li><li>T fixes the test's module doc.</li></ul> |
| 4 | warn | brief section 6 ("#652's guide describes the import and should be told of the new default and flag") | records; the guide says its lists of "verbs, flags, findings, codes" are "taken from the code" (`pane-control.md:6-8`) | **The failing test covers one item only.** `import_findings_are_named_in_section_3_or_10` checks only that sections 3 and 10 name the two codes. No test checks flags.<br>**On the merged tree the guide is stale in five more places:**<ul><li>section 1, `:28-29`: the import's only exclusion is `harness-not-opencode`;</li><li>section 3, the `pane import` synopsis, `:277`, has no `--adopt-claude-code`. The exit-code paragraph, `:314-317`, names no reason for `usage`;</li><li>section 3, the `observe-failed` row, `:300`: "A port call failed". A failed tmux read is now `harness-unreadable`, and `observe-failed` is left for the harness reads (`import/observe.rs:179-186`);</li><li>section 10, the fact, `:684-689`: "No mechanism excludes a pane from the import yet: ... #835, not built". Its closing paragraph, `:749-752`, has the same gap as section 1;</li><li>section 12, `:790`: "an import that leaves the panes Holler does not drive alone (#835)" is listed as later work.</li></ul>**The code's doc is stale too:** `ImportFinding::ObserveFailed`'s doc (`import/findings.rs:91`) still says "A port call returned an error". | **F, in `docs/pane-control.md`, after the merge:**<ul><li>**Section 1:** the import also leaves a pane alone, with no record, when its tmux session runs `claude` (`harness-claude-code`).</li><li>**Section 3:**<ul><li>add `[--adopt-claude-code]` to the synopsis, saying it is refused (`usage`) until #831;</li><li>add two table rows: `harness-claude-code` (`info`; no record, never blocking; `pane doctor` then reports `unregistered-herdr-pane`) and `harness-unreadable` (`problem`; what the pane's tmux session runs could not be read, so the pane gets no record);</li><li>narrow the `observe-failed` row to a failed harness call;</li><li>name the switch under `usage` in the exit-code paragraph.</li></ul></li><li>**Section 10:**<ul><li>restate the fact with the rule and its limits;</li><li>step 1 says what to check: run the dry run while the fleet's Claude Code sessions run. Each Claude Code pane should show `harness-claude-code`. One shown as `record` is a limit of the rule, and the issue's fallback for it is #654's option (a), a state root of OpenCode panes only;</li><li>add `harness-claude-code` to the closing paragraph.</li></ul></li><li>**Section 12:** drop the #835 line. Adopting the panes the import leaves alone (`--adopt-claude-code`, after #831) takes its place.</li><li>**`ObserveFailed`'s doc:** narrow it the same way.</li><li>**Keep the words the guide's other tests need:** section 1 `Claude Code`; section 10 `profile-exists`, `already-imported`, `--herdr-session`, `launcher.toml`, `source-missing`, `` `pane-unlabelled` ``, `sessions.toml`, `last-session`, `owned-sessions`, `pane.env`, `--launcher` and `ADR-0021.md`; `harness-not-opencode` anywhere.</li></ul>**Optional for T:** one pin that section 3's `pane import` subsection names every long flag of the verb, read from the clap tree.<br>**No test of the prose** of sections 10 and 12; S reads it. |

## F's cycle 1 against the plan (the drift re-check)

| Plan (brief and pass 3) | Cycle 1 (`9bd101b`) | Drift |
|---|---|---|
| R2-1: a required `HostPort::foreground_commands` beside `ps` | `ports.rs:340-348`. It is implemented by `TmuxHost`, `FakeHost`, `BindingHost`, `Wired<dyn HostPort>` and `Unwired`, and by the test doubles (T's). `BindingHost` and `WiredHost` are pinned through the host suite. | none |
| R3-1 and W2: `observe_pane` required; `observe_panes` keeps its default | `ports.rs:236-247`. The module docs list four amendments. ADR-0021 section 14 is amended in place, sections 2 and 13 updated, with decisions 22 and 23. | none |
| Section 1: the adapter reads names only, through the `ps` listing's path | In `tmux.rs`: `FOREGROUND_FORMAT`, with `foreground_listing` beside `stop_listing` and `parse_foreground` beside `parse_ps`; dead panes are dropped and one bad line fails the read. `lib.rs:404-414` goes through `run_tmux`, `Call` and `refused()`, with `OP_FOREGROUND_COMMANDS` equal to `HostOp::ForegroundCommands.as_str()`. | none |
| R2-7: one constant and one predicate, beside `HarnessKind` | `pane.rs:139-159`. The only `"claude"` in `holler-pane` is that constant; the other `"claude"` strings in the workspace are protocol harness ids in `holler-proto`, the hub and the body. | none (but see W3 for its doc) |
| Pass 3, note 4: the chain | `import/observe.rs`:<ul><li>the order is `ps`, then the names once `ps` answered, then the server unless the pane runs `claude`;</li><li>a `pane-not-found` from either read is `tmux-session-missing`;</li><li>any other read error, or a lost chain, is `harness-unreadable`;</li><li>messages go through `failed_message` and `embedded`;</li><li>it still runs on one thread per pane.</li></ul> | none |
| Section 3 and R2-4: the outcome | `Outcome::claude_code` beside `not_opencode`, through `left_alone`. `Action::Skipped`. Two kinds in `ImportFinding::ALL`. No new `Action` or `Level`. `assess_all` routes a Claude Code pane before the join. | none |
| Section 4: the switch | `Input::read`, first, with `not_implemented_message(831)`. | none |
| Layering | The rule is in `holler-pane`, the read in `holler-adapter-host`, and the switch and its wording in `holler-cli`. `holler-pane` knows no CLI flag. No new dependency, protocol change, error code or state. | none |

## Notes for T and F (the rework cycle)

There is no O in this run, so the warns are written here as instructions.

1. **T-red, first: the merge** (W1).
   - Run `git fetch origin`, then `git merge --no-ff --no-commit origin/main`.
   - The RED is then `docs_pane_control_test::import_findings_are_named_in_section_3_or_10`, failing on the branch and naming `harness-claude-code` and `harness-unreadable`.
   - In the same phase, fix the two test doc comments: `tests/pane_verbs/unmanaged.rs:467-469` (W2) and `claude_code_rule_test.rs:3-4` (W3).
   - The flag pin of W4 is optional.
2. **F: no code change.** This cycle is the merge (if T-red did not make it), then text:
   - the guide (W4) and `ObserveFailed`'s doc;
   - #840's sentences (W2);
   - the #832 claims in `pane.rs` and ADR-0021 section 13 (W3).
   - These are docs and doc comments only, so report `archChanged: false`. A later T-green BLOCK then goes to F, not back to A.
3. **T-green:** run the suite on the branch, which now contains `main`, so it is what the PR's CI runs. Include `docs_pane_control_test` and `docs_cli_test`. The `--adopt-claude-code` lines in the guide must parse.

## For the operator (outside this story)

1. **#832 closed without a second signal for "runs Claude Code".** So brief section 7 needs nothing more. W3 corrects this story's text that says otherwise.
2. **Unchanged from pass 3.**
   - The real-fleet check of R2-5: read the first real import's dry run while the Claude Code sessions run. W4 puts this in the guide's section 10.
   - The adoption after #831 has no owner (brief section 5; ADR-0021, "Deferred to named stories").

## Reuse map (pass 3's, with the rework's rows; for the Phase 7 gate)

| Need | Extend, do not copy | Where |
|---|---|---|
| A pane left alone | `Outcome::left_alone`, with one constructor beside `not_opencode`; `Action::Skipped`; one kind in `ImportFinding::ALL`, with its `code` and `level` | `holler-pane/src/import/assess.rs`; `import.rs`; `import/findings.rs` |
| The unreadable signal | One more `Level::Problem` kind, through `assess`'s `blocked`, so that `Plan::refusal` refuses it as `import-blocked` | `assess.rs`; `import.rs` |
| The observation | The chain in `observe()`: one read after `ps` answers, on the same per-pane thread; `tmux_finding` and `failed_message` for its errors | `import/observe.rs` |
| The rule | One public constant (a process name) and one predicate, beside `HarnessKind`, separate from #840's registry rule `unmanaged::is_unmanaged` | `holler-pane/src/pane.rs` |
| The port read | `HostPort` beside `ps`. In the adapter: a listing beside `panes()` and `stop_listing()`; a strict parser beside `parse_ps`; `Call` and `refused()`; an `OP_*` constant | `ports.rs`; `holler-adapter-host/src/{tmux,lib}.rs` |
| The forwards and their pins | `BindingHost`, `Wired<dyn HostPort>` and `Unwired`; the host suite over `BindingHost` and `WiredHost`; D33's `observe_*` tests | `holler-cli/src/pane/wiring/bind.rs`, `wiring.rs`; `tests/pane_integration/{bind,observe,wired}.rs` |
| The fake and its pins | `FakeHost` names each process; one `HostOp` variant; case 10 and the `Mutant` breaks; `METHODS` and `call` | `holler-pane-testkit/src/{host.rs,conformance/host.rs}`; `holler-adapter-host/tests/fake_tmux_test.rs` |
| The switch | Step 1 of `Input::read`, `usage()` and `output::not_implemented_message` | `holler-cli/src/pane/import.rs` |
| **The guide (rework)** | Section 3's `pane import` subsection (synopsis, findings table, exit codes), and sections 1, 10 and 12, edited in place | `docs/pane-control.md:28-29`, `:274-317`, `:682-752`, `:790` |
| **The records of #840 and of #832's reuse (rework)** | The sentences, edited in place | `unmanaged.rs:63-67`, `:83-84`; ADR-0021 section 14 (`:1013` after the merge) and section 13 (`:966-967`); `pane.rs:146`; `tests/pane_verbs/unmanaged.rs:467-469`; `claude_code_rule_test.rs:3-4` |

At Phase 7, flag any of these as a parallel path:

- **Tmux and port reads:**
  - a second tmux runner, or a listing that bypasses `run_tmux` and `Call`;
  - a second per-pane thread scaffold in the import;
  - an import-only copy of `ps`'s error mapping or of `failed_message`;
  - a host read that reaches `TmuxHost` around `BindingHost`;
  - a decorator or test double that answers `foreground_commands` without reaching the inner host.
- **The rule and the findings:**
  - a new `Action` or `Level`;
  - a comparison with `"claude"` outside the one predicate, or the rule folded into `unmanaged::is_unmanaged`.
- **Tests:**
  - new copies of D33's `observe_*` tests;
  - a new rig.
- **New in this pass, for the guide:**
  - a second table of the import's findings;
  - a guide section that restates ADR-0021 section 13 instead of linking to it;
  - any change to `main`'s files other than the sentences W2 to W4 name.

## Patterns referenced

- The merged tree `2ff3c92` (`git merge-tree --write-tree 25ff495 bda1069`). From it:
  - `docs/pane-control.md` (sections 1, 3, 10 and 12);
  - `crates/holler-cli/tests/docs_pane_control_test.rs` (acceptance 8, 9, 14 and 17);
  - `crates/holler-pane/src/unmanaged.rs` and ADR-0021 sections 13 and 14;
  - `crates/holler-cli/tests/pane_verbs/unmanaged.rs:467-486`.
- F's cycle 1 (`36b0de8..9bd101b`):
  - `holler-pane/src/{ports.rs, pane.rs, import.rs, import/{assess,observe,findings}.rs}`;
  - `holler-adapter-host/src/{lib,tmux}.rs`;
  - `holler-cli/src/pane/{import.rs, wiring.rs, wiring/bind.rs}`;
  - `holler-pane-testkit/src/host.rs`;
  - ADR-0021, ADR 0003 and `CHANGELOG.md`.
- The pinned pipeline release (`claude-pipeline-v0.1.7`):
  - in `coding-pipeline-logic.mjs`: `nextPhase` case 7 (a T-green BLOCK with `archChanged` goes to A), `shouldEscalate` and `T_RED_BLOCK_CAP`;
  - in `coding-pipeline.workflow.mjs`: `buildPhaseCommitTaskPrompt` (`git add -A`, then `git commit`).
- The repo's hooks: `.githooks/{pre-commit,commit-msg,prepare-commit-msg}`.
