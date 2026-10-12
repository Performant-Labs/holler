[S]

# Handoff-S: #835, `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (spec audit, cycle 5: after F cycle 6, the `harness-claude-code` message)

**Date:** 2026-10-12, 06:15 MDT
**Branch:** `issue-835-implementation` at `fa630f6` (A-dup's fourth pass). Since my cycle-4 audit (`bb233f0`, REWORK, one
item), three phase commits have landed: F cycle 6 (`5dd002e`), T-green cycle 5 (`05936e0`) and A-dup's fourth pass (`fa630f6`).
- **`git fetch` at 06:05 MDT.** `origin/main` is still `b76f4d2`, an ancestor of `HEAD`, and
  `git merge-tree --write-tree HEAD origin/main` exits 0.
- **The PR's diff.** `git diff origin/main...HEAD` outside `docs/handoffs/` is 21 files, with 952 lines added and 124 removed.
- **This cycle's change.** Since `bb233f0`, outside the handoffs, 2 files changed (7 lines added, 5 removed):
  - `crates/holler-pane/src/import/assess.rs`: the message, from F;
  - `crates/holler-cli/tests/pane_verbs/import/harness.rs`: the module doc, from T.
  - `fa630f6` changes handoffs only.

**Issue:** [#835](https://github.com/Performant-Labs/holler/issues/835), OPEN, read live at 06:07 MDT.
- Its body equals the brief's issue section (diffed).
- No comment is new since cycle 4. The last one is from 03:49 MDT ("Parked again").

**#831:** OPEN. Its comment of 00:50 MDT still lists "Adopt the existing Claude Code panes (`pane import
--adopt-claude-code`): after #835 is on `main`".

**PR:** none. `gh pr list --head issue-835-implementation --state all` is empty, and the branch is not on `origin`.

**Brief:** `docs/handoffs/835-brief.md`, round 5 (`cea1c17`), unchanged.

**Handoffs reviewed:**
- `handoff-A.md`: pass 4, PASS.
- `handoff-T-red.md`: cycle 2. Cycle 1 is at `36b0de8`.
- `handoff-F.md`: cycle 6.
- `handoff-T-green.md`: cycle 5, PASS.
- `handoff-A-dup.md`: fourth pass, PASS.
- My cycle-4 handoff (`bb233f0`), which this file replaces.
- `decisions.md` and `evidence.md`: the sections for F cycle 6, T-green cycle 5 and A-dup's fourth pass.

**Diff audited:**
- This cycle's diff in full: `git diff bb233f0 HEAD`.
- The PR's production diff on the current tree: `git diff origin/main...HEAD -- crates/*/src`.
- `import/assess.rs:55-135`.
- `harness.rs` in full.
- The documents that describe the finding or the switch: the guide (sections 1, 3, 10 and 12), the CHANGELOG entry, and
  ADR-0021 sections 9, 13, 14 and 15 (item 17), "Deferred" and the decisions.

**Verdict:** **PASS.** Cycle 4's one REWORK item is done as specified, and no other production line changed. Every
criterion, and every finding of cycles 1 to 4, is closed.

## A precondition

Met.
- `handoff-A.md` (pass 4) is **PASS**, with 0 block.
- `handoff-A-dup.md` (fourth pass) is **PASS**, with 0 block and 1 warn.
  - The third pass's W1 is closed (it was my cycle-4 item).
  - W2 is test-only and carried (Advisory 5).

## T precondition

Met.
- **RED.**
  - Cycle 1 (`36b0de8`): a compile RED on the missing required method, then an assertion-level RED with a stub that was
    reverted.
  - Cycle 2 (`526bbd3`): the two guide tests.
  - F cycle 6 authored no test, so no RED is due. No test pins the message's prose, and none should: its contract is the
    code and the level, which are pinned.
- **GREEN.** T-green cycle 5 is **PASS**, with 0 blocking issues, on `5dd002e`.
  - CI's workspace step: 167 groups, 2600 passed, 0 failed, 36 ignored.
  - After T's doc edit: `pane_verbs` 347 passed, and clippy `-D warnings` and `lint.sh` are clean.
  - `changelog-check`, `docs_cli_test`, `wire_selftest`, `cargo machete`, rustdoc `-D warnings`, `git diff --check` and
    `merge-tree` are clean.
  - The cross-check of F's numbers found no discrepancy.
  - The behaviour-removal check was repeated on `5dd002e`. With `is_claude_code_command` set to `false`, the three Claude
    Code tests fail.

## Acceptance criteria

All rows are checked on `fa630f6`.
- Paths are under `crates/`.
- `harness.rs` means `holler-cli/tests/pane_verbs/import/harness.rs`. Its line numbers moved by 1 since cycle 4.
- `observe.rs` in the table means `holler-cli/tests/pane_integration/observe.rs`.

| # | Criterion (source) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | A pane whose tmux session has a foreground `claude` is not registered. It gets one `info` finding, `harness-claude-code`, and nothing is written for it (Work 1; §2, §3, R2-4). | `harness.rs:94` `a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode`. T-green's mutation (predicate set to `false`) fails it. The finding's message (`holler-pane/src/import/assess.rs:104-106`) is now true on the merged tree (see Spec compliance). | Met |
| 2 | Not a block, and no harness call for that pane (Work 1; §3). | The same test (2 `health` and 2 `list_sessions` calls for 3 panes), and `harness.rs:173` `a_pane_left_alone_blocks_nothing_even_unlabelled`. | Met |
| 3 | A dry run shows the same finding and writes nothing (Work 1; the fixture). | `harness.rs:144` `a_dry_run_reports_the_pane_left_alone_and_writes_nothing`, in JSON and text. The text line's prefix is checked at `:164`. | Met |
| 4 | A pane left alone is `unregistered-herdr-pane` in a whole-fleet `doctor` (Work 1). | Criterion 1, together with `pane_verbs/doctor.rs:133` `unregistered_herdr_pane_is_reported`. | Met (a consequence) |
| 5 | `--adopt-claude-code` is `usage` (exit 2) and names #831. It writes nothing and is refused in a dry run too, before any read (Work 2; §4). | `harness.rs:194` `adopt_claude_code_is_usage_naming_831_and_writes_nothing`: `not_implemented_message(831)`, in JSON and text, and no Herdr, host or harness call. It is the first check in `Input::read` (`holler-cli/src/pane/import.rs:177-183`). | Met |
| 6 | "After #831 it registers them" (Work 2). | Out of scope by brief §5. #831 owns it: ADR-0021 section 15, item 17 ("As built: not yet") and decision 23, and #831's comment of 00:50 MDT. | Out of scope, recorded |
| 7 | `HostPort::foreground_commands` is required, reads names only, and answers `pane-not-found` for a missing session (Work 3; §1; R2-1). | The `compile_fail,E0046` doctest at `holler-pane/src/ports.rs:351`; case 10 (`holler-pane-testkit/src/conformance/host.rs:226`); `holler-pane-testkit/tests/fake_host_test.rs:316`, `:337` and `:352`; and `holler-adapter-host/tests/foreground_test.rs`. All but the doctest are `main`'s since #857. | Met |
| 8 | A pane whose server is merely down is still adopted and flagged `server-down` (Work 3). | `pane_verbs/import/join.rs:334` `an_unhealthy_server_is_down_or_wedged_as_reconcile_says`. Criterion 1's test records the `bash` pane and the pane with no process. | Met |
| 9 | Only an exact `claude` is left alone. There is one constant and one predicate (§2; R2-7). | `holler-pane/tests/claude_code_rule_test.rs` (`main`'s). The constant is at `pane.rs:232` and the predicate at `:244`. The predicate's callers are the import (`import/observe.rs:131`) and `main`'s launch check (`tx_launch/process.rs:65`). No `src` file compares anything else with `"claude"`. | Met |
| 10 | An unreadable signal fails closed: a failed read, a failed `ps` or a lost chain is `harness-unreadable`, and a real run is `import-blocked` (§2; R2-6). | `harness.rs:228` `a_failed_ps_is_harness_unreadable_and_blocks`; `:262` `a_failed_command_name_read_is_a_problem_and_blocks`, where `pane-not-found` is `tmux-session-missing`; `:328` `a_lost_chain_is_harness_unreadable_and_blocks`. | Met |
| 11 | The surface: the `cli-surface.txt` line, ADR 0003, ADR-0021 section 9, the help, and JSON (Work 4). | `cli_surface_test.rs:54` (it parses `cli-surface.txt:173`); `pane_verbs/import.rs:123` `the_adrs_name_the_switch_and_the_two_new_codes`; ADR 0003's row; the JSON assertions in `harness.rs`. | Met |
| 12 | Two new codes with their levels, and the prose counts updated (R2-2; R2-4). | `holler-pane/tests/import_test.rs:99` and `:147`. The `findings.rs` module doc says "eighteen kinds". | Met |
| 13 | ADR-0021 records it: section 2, "amended since", section 13's "Decided (#835)", section 9 and the closing lists (R2-2; R2-5). | `:108-109`, `:121-124`, `:814`, `:1006-1045`, `:1077-1080` (section 14), `:1455-1457` and `:1544-1550`. | Met |
| 14 | Every host decorator forwards `foreground_commands`, and each is pinned (R2-1). | The forwards are `holler-cli/src/pane/wiring/bind.rs:168` (`BindingHost`) and `:381` (`Wired<dyn HostPort>`), and `Unwired` answers the method (`wiring.rs:347`). `pane_integration/bind.rs:406` and `:417` run the host suite over both decorators. | Met |
| 15 | `observe_pane` is required, `observe_panes` keeps its default, and both decorators forward both (R3-1; R3-2). | The `compile_fail,E0046` doctest at `ports.rs:228`; the required signature at `ports.rs:295` and the default at `:303`; `observe.rs:184`, `:192` and `:202`; and the forwards at `holler-cli/src/pane/wiring/bind.rs:143-147` and `:258-262`. | Met |
| 16 | Over the real wiring, `pane list` shows the working directory and the last change, or null in JSON and `-` in text (R3-3). | `observe.rs:290` and `:308`, in JSON and text, over `BindingHerdr(LazyHerdr(FakeHerdr))`. | Met (regression pins) |
| 17 | #652's guide is told of the default and the flag (§6). | `holler-cli/tests/docs_pane_control_test.rs:458` and `:538`. The guide's sections 1, 3 (the `pane import` subsection, `:329-379`), 10 (`:845-940`) and 12 (from `:975`) name both. I read them in full in cycle 4; this cycle I re-read the lines that name the flag or the finding. | Met |
| 18 | A CHANGELOG `[Unreleased]` entry links #835 and names the follow-up to #832 (§6; R3-4). | `CHANGELOG.md:131-146`; `changelog-check: ok`. | Met |
| 19 | A pane left alone writes nothing, in a real run and a dry run (fixture additions). | Criteria 1 and 3. | Met |
| 20 | Case 10 holds on a real tmux: no empty-list assertion, and a bounded poll (R2-8). | `conformance/host.rs:226-252`. It polls for `sleep` every 20 ms for up to `SETTLE` (2 s) and asserts only the name. | Met |

## Spec compliance

**Cycle 4's work list (item 1) is done, in place.** The message of `Outcome::claude_code` (`holler-pane/src/import/assess.rs:104-106`)
is now:

> its tmux session runs Claude Code, and the import does not adopt such a pane until #831's adoption step lands
> (--adopt-claude-code), so it leaves the pane alone

It meets the four conditions:
1. **No claim that Holler cannot drive Claude Code.** The sentence is about the import only.
   - Outside the handoffs, `git grep -n "cannot drive"` finds only `main`'s `holler-pane/src/unmanaged.rs:70-71`.
   - That text says #832's wording "has no case", which is accurate.
2. **True on the merged tree.** The import does not adopt such a pane until #831's adoption step, and the switch is that
   step. ADR-0021 section 15, item 17 (`:1359-1379`, "As built: not yet") and #831's comment of 00:50 MDT say so.
3. **The help's words, on one source line (`assess.rs:105`).** `git grep -n "until #831's adoption step lands"` now finds
   the message together with:
   - the CLI module doc (`holler-cli/src/pane/import.rs:9`) and the help (`:80`);
   - ADR-0021 `:814` and `:1546`;
   - the guide `:379`;
   - the test's doc (`harness.rs:11`).
4. **A constant.** It is a `&str` literal with no observed text, passed to `Outcome::left_alone` as before.
   - The `\` continuations make it one line.
   - F's `strings` check of the built test binary shows the whole sentence.

**F's design decisions 1 to 4 are accepted.**
1. **"such a pane", not my example's "which".** It fixes the referent: the import adopts panes, not a harness. My wording
   was an example only.
2. **The flag is written bare.** That matches the import's other runtime text (`holler-pane/src/import.rs:164`,
   "--dry-run") and the CLI's refusal of this flag.
3. **"leaves the pane alone".** It keeps the issue's words, the constructor's name and the guide's.
4. **The first issue number in a `holler-pane` runtime string.** My condition 3 required it. A-dup ruled that it is not
   drift: the library cannot use the CLI's `CLAUDE_CODE_STORY`.

**The documents agree with the message.** Each says that adopting these panes waits for #831's adoption step, the
switch:
- the guide: `:30`, `:335-337`, `:379` and `:982-983`;
- the CHANGELOG: `:139-140`;
- ADR-0021: `:814`, `:1018`, `:1035`, `:1455-1457` and `:1546`.

The guide's code table row (`:352`) says the pane is left alone with no record, which is also what the message says. No
document or test quotes the message.

**T-green's edit is my cycle-4 Advisory 3.**
- It touches `harness.rs:1-3` and `:10-11`, the module doc only. No assertion changed.
- The header now paraphrases the port's own doc (`holler-pane/src/ports.rs:388-392`).

**The recorded decisions still hold.** None of their files changed since cycle 4.
- The level is `info`, where `harness-not-opencode`'s is `skipped` (§3).
- The switch is `usage`, not `not-implemented` (§4).
- Rounds 2, 3 and 5 are as I verified in cycle 4. That includes one constant and one predicate, the decisions numbered 30
  and 31, and one adoption entry under "Deferred".

## Quality audit

- **Correctness and failure handling.** Unchanged since cycle 4, apart from the constant message. I re-read the
  production diff on the current tree:
  - in `import/observe.rs`, `ps` runs first, then the name read;
  - a `claude` pane ends the chain before any harness call;
  - `pane-not-found` from either read is `tmux-session-missing`;
  - any other read error, and a lost chain, is `harness-unreadable`, a problem, so a real run is `import-blocked`;
  - observed error text reaches a message only through `failed_message`'s `embedded`;
  - the write step is unchanged.
- **Build guards.**
  - The PR's added `src` lines have no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` or `unreachable!`.
  - The PR adds no `#[allow]` in code. The five added lines that mention it are prose in handoffs.
  - Every touched `.rs` file is under 900 lines. The largest is `ports.rs` (621); `assess.rs` is 435 and `harness.rs` 341.
  - There is no dead code. `ObserveFailed` is still built for the two harness-server calls (`import/observe.rs:182` and
    `:186`).
- **Protocol.** No change. No hub, body, protocol or golden file is in the diff.
- **Tests.**
  - The tests use fakes, plus a real tmux in `--ignored`.
  - No fixed sleep is used for synchronization: case 10's 20 ms poll is bounded by `SETTLE`.
  - RED-first evidence is in both T-red cycles.
- **Documentation.**
  - The CLI surface is in the help, `cli-surface.txt`, ADR 0003, ADR-0021 section 9 and the guide's section 3.
  - The two codes are in the guide's table and ADR section 13.
  - The CHANGELOG's `[Unreleased]` entry links #835.
  - The README documents no `pane` verb, so it needs nothing.
- **Public-repository privacy.**
  - **What I grepped:** the PR's added lines (handoffs included) and this cycle's added lines.
  - **For:** IPs (including the tailnet range), `ts.net`, tailnet, tailscale, home paths, this machine's account and host
    names, e-mail addresses other than the `noreply` trailers, and keys or tokens.
  - **Result:** no hit that names anything.
    - The only matches are prose: the brief's rule sentence, and my cycle-4 handoff's mention of the generic `/home/u`
      and `/home/demo`.
    - `HEAD`'s tree is clean.
  - **History, unchanged since cycle 4.** Five unpushed commits carry an absolute path under the local machine account's
    home directory (a Tier-1 log path) in `docs/handoffs/835/handoff-S.md:32`.
    - The commits are `dc64e11`, `cea1c17`, `59d8d59`, `6787d99` and `ecb04db`.
    - The path names the account, which is spelled as the operator's public author name. It names no host, tailnet, IP
      or secret.
    - `main`'s history never had it: `git log -S` on `origin/main` finds nothing.
    - The four commits since cycle 4 add none.
    - The role's REWORK rule is for the diff, and the diff is clean. F cannot fix history: the pipeline does not rewrite
      it, and A's W1 forbids a rebase. So it stays the operator's decision (Advisory 1).
- **Commits and PR.**
  - All 30 commits after `origin/main` have Conventional subjects.
  - Each carries the hook's `Co-Authored-By: Claude <noreply@anthropic.com>` trailer, as `main`'s pipeline phase commits
    do. Neither carries a session link.
  - No PR exists yet. The title, the body and the AI disclosure are the run agent's (Advisory 3).

## Scope check

- **Delivered:**
  - the issue as amended by rounds 1 to 3;
  - round 5's merge and prose;
  - A's warns and the guide update;
  - the four REWORKs of cycles 1 to 4.
- **This cycle** did exactly my cycle-4 item (F) and my optional Advisory 3 (T). There is no over-delivery.
- **Under-delivery** is only the adoption, which brief §5 puts out of scope and #831 owns.
- **#857 carries part of this story.** The port method, constant, rule, case 10 and forwards reached `main` through #857.
  That is why the PR's diff is 21 files, and the PR body should say so.

## Verdict

**PASS.** The issue, as amended, is met. Every acceptance criterion has a test that asserts the behaviour, and the run is
spec-compliant. The quality is acceptable. Ready for O and the PR phase.

## Advisory notes (non-blocking)

1. **Privacy in history (operator).** The script's PR phase pushes the branch right after this PASS, so the five commits
   in the Quality audit become visible in the PR.
   - **Exposure.** The account name equals the operator's name, which every commit in the repo already shows as its
     author. No host, tailnet, IP or secret is exposed.
   - **To remove it,** rewrite that one line in those five commits and force-push before the merge. GitHub may still
     serve the old commits by SHA until it purges them.
   - **The squash merge keeps it off `main` either way.** This file carries no such path.
2. **For #831's adoption step,** which replaces the refusal. Three groups of text need updating then:
   - **This story's text.** `git grep -n "adoption step"` finds every place:
     - the help, the module doc, the flag's doc and `CLAUDE_CODE_STORY`'s doc (`holler-cli/src/pane/import.rs`);
     - the message (`assess.rs:105`) and `HarnessClaudeCode`'s doc (`findings.rs:72`);
     - the test's doc (`harness.rs:11`);
     - ADR-0021 `:814`, `:1018`, `:1035`, `:1455` and `:1546`;
     - the guide `:336` and `:379`, and section 12's `:982`;
     - the CHANGELOG `:140`.
   - **#831's own text:** ADR-0021 `:1361` ("until this harness exists") and the CHANGELOG `:14-15` ("waits for #835").
   - **`main`'s text that says no verb adopts a pane** (A-dup's note, extended). It becomes false for these panes once the
     step lands:
     - `holler-pane/src/reconcile.rs:449`, the `unregistered-herdr-pane` message;
     - the assertion text at `pane_verbs/doctor.rs:149`;
     - the guide `:641`;
     - ADR-0021 `:1111`.
3. **The PR (run agent).**
   - **Title.** The script titles it "Implements #835", and the squash subject becomes `main`'s record. The issue's title
     says "--adopt-claude-code adopts them", which this PR does not do. A title such as "feat(pane): holler pane import
     leaves Claude Code panes alone; --adopt-claude-code is refused until #831's adoption step (#835)" matches the
     CHANGELOG.
   - **Body.**
     - Say that #857 already carries this story's port method, constant, rule, case 10 and forwards.
     - Add the coexistence notes: #832/#836, #840, #652, #851/#854, #667/#855, #648/#856, #665/#858 and #831/#857.
     - Add the AI disclosure from `CONTRIBUTING.md` with `gh pr edit`.
     - Write times in MDT.
   - CI's "PR size" step will warn. It is advisory.
4. **For the operator, carried.**
   - #633's contract block still omits `foreground_commands`, `observe_pane` and `observe_panes`.
   - R2-5's real-fleet dry run (the guide's section 10, step 1) is still the only check of the rule's limits.
5. **Carried, test-only.**
   - There is `rustfmt` drift in this story's new test code. CI has no format gate (`.github/workflows/` has no `fmt`).
   - The test-only Herdr wrappers use `observe_panes`'s default.
   - A-dup's W2: the `run_with_host` and `text_row` twins.
