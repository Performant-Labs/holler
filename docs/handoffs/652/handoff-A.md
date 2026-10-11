# Handoff-A: #652 - The operator guide for `holler pane`  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-652-implementation` (base `origin/main` at `263a2d7`; the branch's two commits touch only `docs/handoffs/`)
**Brief reviewed:** `docs/handoffs/652-brief.md` (commit `4b79de0`, amended after outside round 1; outside round 2: PASS)   **Reuse map:** the brief's own evidence E1 to E34 and its Blast radius (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK: 1 `block`, 8 `warn`.

**The test plan is consistent with existing patterns.**

- The plan reads the guide in a new `[[test]]` target that takes its lists from the code's own tables. That follows
  `docs_cli_test.rs` and `holler-proto/tests/docs_errors_test.rs`.
- Every table the test reads is public on the base: `Cli::command()`, `pane::list::COLUMNS`, `FindingKind::ALL`,
  `ALL_CODES`, `GridPos::parse` and `class_of`.
- No existing doc or test is duplicated, and `docs/pane-control.md` is #652's file under the epic's hot-spot list.

**The block is in what the guide states.** Three parts of the plan treat `holler pane` as managing every fleet pane:
section 1's rule, section 11's rollback and the `docs/orchestrating.md` rewrite. The plan also has the guide announce
that `scripts/pane-restart` is removed after the sign-off. The code says otherwise:

- `HarnessKind` is closed to `opencode` (ADR-0021 section 1; `pane.rs:136-138`).
- #650's import skips a pane that does not run OpenCode (`harness-not-opencode`, no record).
- `scripts/pane-restart` has restarted Claude Code panes since #784.

So a Claude Code pane never enters the registry, and a whole-fleet `doctor` pass reports it as `unregistered-herdr-pane`
every time. The script is the only restart path this repository has for such a pane, before the sign-off and after it.
The interim section that the brief rewrites contains that Claude Code path (`docs/orchestrating.md:125-145`). The
amendment needs one operator decision (finding 1 and Notes for O).

**The warns:**

- 2: what a rollback with the script does to a registry pane.
- 3: the import command form, against ADR-0021 section 13 and #650's required flags.
- 4: which commands the guide may show and still pass `docs_cli_test`.
- 5: the section contract between the guide and its test, and #665.
- 6: single-source points.
- 7: the test file's name and shape.
- 8: the Herdr display plugin (#651).
- 9: the script's own header line.

**Run context.** Per the operator's plan this is a briefs-ahead run, stopped after this verdict. The base lacks the
behaviour that sections 2, 3, 9 and 10 describe: #802, #642 part 3, #644 part 2, #649, #650 and #664 are all OPEN. The
build run must re-take that evidence from merged code and repeat this review on its own base, as the brief's
"Depends on" already requires.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | Scope 1 and 11, the `docs/orchestrating.md` rewrite, operator decision 2 | ADR, cross-cutting (which panes the registry manages) | The plan presents `holler pane` as the restart path for every pane and says the script is removed after the sign-off. But `HarnessKind` is closed to `opencode`, #650 skips non-OpenCode panes, and the script's Claude Code path (#784) is the only restart path for those panes. Built as briefed, the agent guidance points orchestrators to a verb that cannot restart a Claude Code pane, and announces the removal of the one tool that can. | The brief states which panes `holler pane` manages. Sections 1, 5, 10 and 11 and the rewritten interim section say what a non-OpenCode pane is: import skips it, doctor reports it as `unregistered-herdr-pane`, and only the script restarts it. The operator decides whether the removal after the sign-off covers only the OpenCode path. |
| 2 | warn | Scope 1, 10 and 11: a rollback with `scripts/pane-restart` on a registry pane | cross-cutting (source of truth) | Section 1 states I1 as absolute, section 11 sanctions the script, and section 10 says the old files are kept read-only. On an OpenCode pane the script writes `pane.env`, starts a session that is no pane's session of record, and leaves the record's model and session stale. Doctor then reports the pane, and `doctor --fix` would select the old session of record again. | Section 11 says that the fallback takes that pane out of the new regime. It names what doctor will report and says not to run `--fix` on the pane. It names the way back (for example `holler pane relaunch PANE --model ...`) and says #654's journal records the use as a rollback. Section 10's "read-only" excepts the fallback. |
| 3 | warn | Scope 10 (cutover), "Depends on" (NV-3) | ADR | The procedure runs `holler pane import --dry-run`. ADR-0021 section 13 step 1 is `pane import --from <state-root> --dry-run`, and #650's synopsis requires `--from <state-root>` and `--herdr-session NAME`. The brief's premise that the bare span parses once #650 adds `--dry-run` is wrong: clap answers `MissingRequiredArgument`, which `docs_cli_test` does not tolerate. Separately, "now derived" restates section 13's table with a different verb: section 13 says "replaced by registry state", nothing in Holler derives these files, and #653 retires their writers. | When the build starts, quote the import synopsis and section 13's order from the merged #650. Fix the "Depends on" sentence. Say the four files are replaced by registry state and are no longer read after the cutover, and link section 13's table instead of restating it. |
| 4 | warn | Scope 3 ("an example" per verb), acceptance 6 | pattern consistency (`docs_cli_test`) | `docs_cli_test` sees a command only in a backtick fence tagged `bash`, `sh`, `shell`, `console`, `text` or untagged, or in a one-backtick span. A `~~~` or `zsh` fence escapes it silently. Typed values fail on a bare placeholder: `--since CURSOR` (u64), `--ctx-soft N` and `--ctx-hard N` (u32). The three PROPOSED verbs have empty `Args` (`ProfileExport {}` and the others), so any example with arguments fails. The pending list is #670's file and its lines are #665's, so this story cannot add them there. | The brief says how to write these. PROPOSED verbs appear bare (`holler profile export`), with their planned arguments described in prose. Typed placeholders go inside `[...]` or become numbers. Every command example uses a backtick fence with an allowed tag. |
| 5 | warn | Acceptance 1, 3, 4, 7, 9 ("section N") and 11 | pattern consistency, forward-compat (#665) | The tests address "section 4", "section 9" and "the grid section", but the brief pins no heading form, so the guide and the test can disagree on what a section is. Acceptance 1 accepts a PROPOSED verb under a PROPOSED heading, but acceptance 11 requires every line that names `profile export` or `profile import` to say PROPOSED. #665 will add a subsection and lift the PROPOSED marks, so it will edit this story's test. | Number the twelve `##` headings in the brief's order (`## 7. Grid notation`). Slice a section from `\n## N.` to the next `\n## `, as `docs_errors_test.rs::errors_section` does. Use one PROPOSED rule for both tests, the line rule. Keep the PROPOSED set in one constant whose doc names #665 as the story that changes it. |
| 6 | warn | Scope 9 ("every error code with its class"), the `profile apply` reference, scope 1 | single source, ADR | The class column is a hand-kept copy of `class_of`, and acceptance 3 checks only that each code is present. Exit codes have moved before (refusals went from 1 to 3). The apply reference names the #664 brief as its authority, but after #664 merges the standing spec is ADR-0021 (section 9's apply row and the definition of "adopt", which #664 edits), with `tx_apply.rs` and `profile/apply.rs`. Section 1's rule names `holler pane` and `holler profile`, while ADR-0021's I1 says "only pane verbs". | The test checks each code's row against `class_of(code)`, as `docs_errors_test` checks its rows against `Code`'s own values. The guide cites ADR-0021 for the apply contract, and the build re-takes the wording from the merged ADR and code. Section 1 quotes I1 as ADR-0021 words it and adds that profile verbs change panes only through the same transactions. Optional: run each JSON envelope example through `holler_pane_testkit::envelope::check_envelope`. |
| 7 | warn | The new test file | naming, file structure | A new target beside `docs_cli_test` is the right home. A module in `pane_cli_process` would edit #670's `process/main.rs`, and that target's `PANE_VERBS`/`PROFILE_VERBS` are the hand-kept lists that operator decision 3 rules out. Three details remain. The name breaks the `docs_<subject>_test` cadence. Clap's own `help` subcommand must be filtered, as `cli_surface_test.rs::clap_leaves` does. Acceptance 10 reads `docs/README.md`, which the brief's list of files read leaves out. | Name the file `docs_pane_control_test.rs` (optional). Filter `help`. Collect every failure before asserting, as `assert_no_failures` does. Give the `#![allow(...)]` line its `// #NNN` link (lint check 1). |
| 8 | warn | Scope 4, operator decision 1 | scope | `pane list` and `doctor` are not the only places to read SHOWN against DRIVEN. The merged Herdr display plugin (#651, `plugins/herdr-holler`) shows "project, SHOWN vs DRIVEN, hold" in Herdr's sidebar, and no page under `docs/` describes it. | Section 4 names the plugin in one sentence (display only, never changes a pane), or the brief says why the guide leaves it out. |
| 9 | warn | Scope 11, decision 2 | ADR, consistency | `scripts/pane-restart:3` still says the script is a stop-gap "until `holler pane relaunch --model` (#644) and the pfleet adoption (#653) land". That contradicts the retention rule the guide will state, and the script is outside the Blast radius. | Add that comment line to the Blast radius (a comment, no behaviour change), or record it for the removal story. |

### Details

**1. Panes that `holler pane` does not manage.**

Evidence:

- `crates/holler-pane/src/pane.rs:136-138` is `pub enum HarnessKind { Opencode }`. ADR-0021 line 41 says "`kind` is the
  closed enum `HarnessKind` (only `opencode`)".
- #650's body, line 36 (scope item 6) and line 75 (decision 7): "A pane that is not an OpenCode pane ... is the
  information-level skip `harness-not-opencode`: no record, never blocking".
- `crates/holler-pane/src/findings.rs:51-52` defines `UnregisteredHerdrPane`: "A Herdr pane that no record names ...
  Whole-fleet runs only".
- `scripts/pane-restart:6` and `:19-24` (#784, closed) give the Claude Code usage line and path.
- `docs/orchestrating.md:108-145` is the interim section that the brief rewrites. It runs to the end of the file and
  contains the Claude Code subsection (lines 125-145).

What the brief would produce:

- **Section 1:** "Nothing but `holler pane` and `holler profile` changes a pane". This is false for a Claude Code pane
  in the same Herdr workspace: the script changes it, and nothing else can.
- **Section 11 and the rewrite:** `holler pane relaunch` "is the way", and the script is a fallback "until the 7-day
  sign-off" that "is removed ... after the sign-off". For a Claude Code pane, relaunch has no record to act on
  (`pane-not-found`, exit 3), and the script is the only path.
- **Sections 5 and 10:** they do not mention that each Claude Code pane is a standing `unregistered-herdr-pane` finding
  in every whole-fleet pass. ADR-0021 section 12 says a pass keeps no memory, so the finding repeats every time. The
  sign-off in section 10 reads doctor every day, so the operator needs to know this.

Why this is a block: the guide is the contract for the operator and for orchestrators (the brief calls
`orchestrating.md` "the agent guidance for orchestrators"). As briefed, it states an architecture the code does not
have. Fixing it means changing the brief's own text ("the script is removed ... after the sign-off"), which is a
decision, so F cannot be left to resolve it.

Recommendation, for O to propose and the operator to confirm:

- The removal after the sign-off covers the script's OpenCode path only. The Claude Code path stays until
  `HarnessKind` gains a harness for it.
- Section 1 scopes I1 to registry panes, which today means OpenCode panes.
- Section 10 says the import skips such a pane (`harness-not-opencode`).
- Section 5 says such a pane is reported as `unregistered-herdr-pane` in whole-fleet passes.
- The rewritten interim section keeps the Claude Code subsection as the way to restart those panes.

**2. A rollback on a registry pane.**

On an OpenCode pane, the script (header lines 10-12) does two things:

- It backs up and rewrites the `OC_` lines of `pane.env`.
- It stops the TUI, starts it fresh, and waits for the new session to register.

Once the import has given that pane a record, the record's `model` and `session_of_record` are stale, and the TUI shows
a session that is no pane's session of record. The next pass reports the pane:

- `shown-driven-mismatch`, or `tui-foreign-session` if the TUI's server is not the recorded one;
- `stray-session` for the new session, when that session is on the pane's server.

`--fix` repairs only the mismatch: it selects the session of record whenever the server is healthy and still has that
session (E6, `FixState::Fixed`). So an operator who follows section 10 ("run `doctor` until SHOWN equals DRIVEN") with
`--fix` undoes the rollback's session switch.

The code does nothing wrong here: the fallback leaves the regime by design. But the guide states I1 (section 1), the
read-only files (section 10) and the fallback (section 11) without connecting them. One paragraph in section 11 closes
the gap. O can write it without an operator decision, but the way back (a relaunch with the new model) should be named
explicitly.

**3. The import command and ADR-0021 section 13.**

- ADR-0021 line 701: "`pane import --from <state-root> --dry-run` (#650) reports every disagreement ...".
- #650's body, line 5:
  `holler pane import --from <state-root> --herdr-session NAME [--launcher FILE] [--orchestrator PANE]... [--dry-run] [--format=text|json]`.
  Line 95: "`--herdr-session` is required".
- `docs_cli_test.rs:193-199` tolerates only `DisplayHelp`, `DisplayVersion`,
  `DisplayHelpOnMissingArgumentOrSubcommand` and `MissingSubcommand`.

So the brief's section 10 wording, taken from #654's body, fails the docs test after #650 merges, not only before it,
if F puts it in a code span. If F puts it in prose instead, it contradicts the ADR's step 1. The guide should also name
`--orchestrator PANE` (#650 decision 2): the dry run prints each pane's role, and the orchestrator (MO) pane is named
with that flag.

On "derived": ADR-0021 section 13's table (lines 689-695) maps each file to the registry field that replaces it. Step 3
(line 707) says "the launcher's files are no longer read. Retiring the launcher's writers is pfleet's follow-up
(#653)". So nothing derives these files, and until #653 lands something may still write them. "Kept, read-only, for
one release" is the operator's rule from #654 and stays. Link the section 13 table instead of restating it, and say
"replaced by" instead of "derived".

**4. What `docs_cli_test` can see and parse.**

What the test sees:

- It recognises a fence only by three backticks (`docs_cli_test.rs:95`).
- It scans only fences tagged `""`, `bash`, `sh`, `shell`, `console` or `text` (`:101`).
- Its inline spans are pairs of single backticks on one line (`:115-125`).

A command inside a `~~~` fence, or a fence with any other tag, is never checked. Acceptance 6 would then pass with an
unparsable command in the guide.

What fails to parse:

- **Typed values.** Clap types some values before any placeholder rule applies: `watch.rs:54-55` (`--since`, `u64`)
  and `args.rs:68-72` (`--ctx-soft`, `--ctx-hard`, `u32`). Normalisation keeps `CURSOR` and `N` as literal values, so
  `holler pane watch --since CURSOR` fails with a value error. `[--since CURSOR]` is dropped, so it passes.
- **The PROPOSED verbs.** `profile/{rename,export,import}.rs:17` are empty structs, so
  `holler profile export NAME --out FILE` fails with an unknown argument. This story cannot list them in
  `cli-surface.pending.txt`, which is #670's file with lines owned by #665. Scope 3's "an example" per verb therefore
  has to be the bare verb for these three.
- **`profile apply`.** An example such as `holler profile apply NAME --dry-run ...` needs #664's merged `Args`; today
  the verb has only `--take-over` (`profile/apply.rs:14-17`). The brief's dependency on #664 already covers this.

**5. The section contract.**

- **Precedent:** `crates/holler-proto/tests/docs_errors_test.rs:17-22` slices from `\n## 8. Errors` to `\n## 9.`. The
  wizard tests slice by a heading prefix (`section(text, heading_start)` in the `skilltext*.rs` files).
- **Numbered headings:** the test can then find sections 4, 5, 7, 9 and 10 without guessing, and the subsection #665
  adds later (`###`) does not move any number.
- **The PROPOSED rules:** acceptance 1 accepts either a line or a section whose heading contains PROPOSED. Acceptance 11
  uses only the line rule, so a body line under a `(PROPOSED)` heading passes 1 and fails 11. Use the line rule for
  both.
- **`profile-drift` (#665):** when it joins `FindingKind::ALL`, acceptance 3's intersection grows, and the test then
  requires `profile-drift` in sections 5 and 9. Acceptance 11 still requires PROPOSED on every line that names it. #665
  has to edit this test either way, so the test's doc should say so.

**6. Single-source points.**

- **The class column.** Exit codes have changed before: epic #633, "Decisions 2026-10-09" item 5, moved refusals from 1
  to 3 (#676). A class column that no test checks drifts silently. ADR-0021 section 9 already keeps a per-code class
  table (lines 545-590), which `class_of`'s doc names as the reasons. `class_of` and `ErrorClass::exit_code` are public
  (`error.rs:239`, `:266`), so the test can check the guide against them.
- **The apply reference.** The #664 brief is pipeline scratch. ADR-0021 is the standing spec (CLAUDE.md), and #664
  edits its definition of "adopt" and section 9's apply row (#664 brief, O3).
- **Open codes.** `orchestrator-not-relaunched` is #664's own open code. Scope 9's sentence on open codes can name it
  beside `agent-unusable`.
- **I1.** ADR-0021 line 206 says "Only pane verbs change a pane or its registration". The guide's wording adds
  `holler profile`. That is accurate: `apply` changes panes through `tx_launch`, and `create --from-current` sets
  `Pane.profile`. But unless the guide says so, it reads as a new rule.

**7. The test file.**

- **The manifest.** `crates/holler-cli/Cargo.toml:473-475` holds #670's rule that a verb story never edits the
  manifest. That rule is for verb stories. A docs story's own target follows `docs_cli_test` (`:88-90`) and
  `pipeline_config_test`.
- **The verb lists.** `pane_verbs/process/main.rs:40-49` holds the hand-kept `PANE_VERBS`/`PROFILE_VERBS`. Operator
  decision 3 rules them out, and the brief correctly reads `Cli::command()` instead.
- **Clap's `help`.** `cli_surface_test.rs:34-51` filters clap's own `help` subcommand when walking the tree.
- **Failures.** `pane_verbs/process/main.rs:95-101` (`assert_no_failures`) and `docs_cli_test.rs:206-211` collect
  every failure before asserting.
- **Lint.** `scripts/lint.sh` check 1 requires `// #NNN` on every `#[allow]`. The line-count gate (600 warns, 900 fails)
  applies to `.rs` files only, so the guide itself has no length limit.

**8. The Herdr display plugin.** `plugins/herdr-holler/herdr-plugin.toml:19` describes it as "Holler's pane state in
the sidebar: project, SHOWN vs DRIVEN, hold". Its actions run the `holler pane` verbs (`:10-13`), so it agrees with
section 1's rule. It is the only other surface that shows SHOWN against DRIVEN, and no page under `docs/` names it.

**9. The script's header.** The line is `scripts/pane-restart:3`. The older CHANGELOG entries are history and stay as
they are.

## Notes for O

Required for BLOCK.

**Finding 1 needs an operator decision before the brief is amended.** The brief's scope 11 says `scripts/pane-restart`
"is removed (in a later story, not this one) after the sign-off". But the code has no other restart path for a Claude
Code pane: `HarnessKind` is `opencode` only, #650 skips such a pane (`harness-not-opencode`), and relaunch answers
`pane-not-found`. The options:

- **(a), recommended.** Only the script's OpenCode path is removed after the sign-off. Its Claude Code path stays until
  `HarnessKind` gains a harness for it. The guide scopes I1 to registry (OpenCode) panes and documents non-OpenCode
  panes in sections 1, 5, 10 and 11. The `orchestrating.md` rewrite keeps the Claude Code subsection as the way to
  restart those panes.
- **(b)** Every fleet pane runs OpenCode before the cutover, as a step in section 10's procedure. The guide says a Claude
  Code pane is outside the fleet, and the whole script is removed after the sign-off. This changes #654's procedure, so
  it is also the operator's call.

Then add to the acceptance criteria:

- the guide names `harness-not-opencode` and says that a Claude Code pane is restarted with `scripts/pane-restart`;
- `docs/orchestrating.md` still names `Claude Code` and `scripts/pane-restart`.

**Warns 2 to 9:** fold them into the brief together with finding 1.

- 2, 4 and 6 are text changes only.
- 3 should be re-taken from #650's merged synopsis.
- 5 and 7 pin the test's shape for T.

**This is a briefs-ahead run.** Start the build as a fresh run once #802, #642 part 3, #644 part 2, #649, #650 and #664
have merged:

- Re-take E2, E3, E19, E20, E25, E27 and E29 from the merged code, and re-run this review on that base.
- Do not resume this run: its A verdict is cached.

## Patterns referenced

1. `crates/holler-cli/tests/docs_cli_test.rs` (the whole file): which fences and spans the docs test sees, its
   normalisation, and the parse errors it tolerates.
2. `crates/holler-proto/tests/docs_errors_test.rs:12-34`: a docs section sliced by its numbered heading, with one row per
   code of the closed table, checked against the table's own values.
3. `crates/holler-cli/tests/cli_surface_test.rs:34-51`: the clap-tree walk that filters clap's own `help`.
4. `crates/holler-cli/tests/pane_verbs/process/{main.rs,docs_rows.rs}`: docs-layout tests of the pane and profile
   surface, `assert_no_failures`, and the hand-kept verb lists the brief rightly does not reuse.
5. ADR-0021 section 1 (line 41), section 4 (lines 206-211), section 9 (lines 505-590) and section 13 (lines 685-707),
   read against `crates/holler-pane/src/pane.rs:136-138`, `scripts/pane-restart:1-24` and
   `docs/orchestrating.md:108-145`.
