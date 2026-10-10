# Handoff-S: Phase 8 - #645a `pane switch` and `pane reset` (spec audit, round 2)

**Date:** 2026-10-10 01:50 MDT
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`), head `67fbe33` (A-dup round 3). The
code is F round 6's (`6dd44f3`): `git diff 6dd44f3 HEAD -- crates/ CHANGELOG.md docs/adr/` is empty.
**Issue:** #645, part 1 of 2 (645a), epic #633. **Brief:** `docs/handoffs/645-brief.md`, as amended in `8cf4f00`
**Handoffs reviewed:**
- `handoff-A.md` (round 2, PASS), `handoff-T-red.md`, `handoff-F.md` (round 6), `handoff-T-green.md` (round 6) and
  `handoff-A-dup.md` (round 3, PASS).
- `decisions.md` (from `:406` closely) and `evidence.md`.
- The outside diff gate's results: r5, which failed (`gate-unavailable`), and r6, O's manual rerun of r5's prompt, which
  is a PASS. r5's prompt was built at 01:15 MDT, after F's round-6 merge, so r6 reviewed this code.
**Diff audited:** `git diff origin/main...HEAD`. The merge base is `bd5e825`, and `origin/main` is still `bd5e825`
(`git ls-remote` at 01:46 MDT), so the three-dot diff is the tree that will land. `git merge-tree --write-tree HEAD
origin/main` exits 0.
**Supersedes:** round 1 (REWORK, `ddb6fc3`). Its one item, the merge of `origin/main`, is done. F did it in rounds 4 and 6,
and T re-verified both times.

## A precondition

Met. `handoff-A.md` round 2 is **PASS** (the plan). `handoff-A-dup.md` round 3 is **PASS** (the diff, on `cce6f00`, which
carries `6dd44f3`'s code).

## T precondition

Met.
- **RED.** `handoff-T-red.md`: 94 passed and 19 failed, each on its assertion, with no compile error, setup panic or
  timeout.
- **GREEN.** `handoff-T-green.md` round 6 lists no blocking issues:
  - `pane_verbs` 162/162, with `switch::` and `reset::` at 22. `pane_cli_process` 35, `cli_surface_test` 3,
    `docs_cli_test` 3, `wire_selftest` 3 and `holler-pane` 98.
  - Clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, rustfmt on the five story files and `cargo machete` are clean.
- **The one workspace failure is not this story's.** T's run in CI's form had one failure,
  `body_run_test::fresh_hello_and_presence_on_every_reconnect` ("hub did not report listening within 10s").
  - The story touches none of that code (the diff's file list confirms it).
  - The target passed 5 of 5 re-runs.
  - F's run of the same command on the same tree (01:02-01:06 MDT) exited 0 with 1700 passed.

## Acceptance criteria

All test paths are under `crates/holler-cli/tests/pane_verbs/`. Every case runs through `both_with` (`switch.rs:121-150`).
On every run of every test, it checks that:
- text and JSON exit with the same code;
- the JSON is one valid envelope, with nothing on `err`;
- text writes one line, to the right stream;
- no Herdr or host call was made (`:141-148`).

The rig's records start at `Health::Unknown` with an empty `last_observed`, so every field AC 1 asserts is a real write.
T's mutations, rounds 1-6, were each caught by the right test: P4, P5, O1, the record fields, the reconcile step, the
created note, the remedy's `Some` arm, and `acted` on the select and on the observation. So these tests fail without the
change.

| AC | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | TUI and record move together; exactly four fields; generation +1 | `switch.rs:261` `switch_moves_the_tui_and_the_record_together`. `assert_recorded` compares the whole record. `driven` is pre-set to `ses_driven` and kept. The exact text line. JSON `verb`, `previous` and `data.pane` equal the stored record | MET |
| 2 | No keystroke; exact call sequences | the I4 check in `both_with` on every run. Switch: `[Health, ListSessions, SelectSession, ShownSession]` and `[Get, List, CasPut]` (`switch.rs:277-286`). Reset: `[Health, CreateSession, SelectSession, ShownSession]` and `[Get, CasPut]` (`reset.rs:71-79`) | MET (under the AC 1 and AC 14 test names, not a test of its own) |
| 3 | Deleted session: `session-not-found`, nothing changes | `switch.rs:306` | MET |
| 4 | Another pane's session: `session-of-other-pane` | `switch.rs:324`: shared data directory; names `demo-c2r1`; both panes unchanged | MET |
| 5 | Unhealthy: 3 `server-unhealthy`; health timeout: 1 `timeout` | `switch.rs:343`: kill and freeze stop at `[Health]`; the message has `run holler pane relaunch demo-c1r1` | MET |
| 6 | Orchestrator's pane needs `--as-operator` | `switch.rs:378` | MET |
| 7 | Mismatch after select records nothing (I3) | `switch.rs:397`: `its home screen`, ends with the reconcile step | MET |
| 8 | Select failure names the reconcile step | `switch.rs:416`. Also `switch.rs:436` (T round 5): the observation errs after a good select, `acted` is set, no `CasPut`, and the TUI moved | MET |
| 9 | Record conflict after the act | `switch.rs:509`: `WriterInSelect`; the other writer's record stands, the TUI shows S2, no retry | MET |
| 10 | `--profile` scoping | `switch.rs:546` | MET |
| 11 | Unknown pane | `switch.rs:570` | MET |
| 12 | Usage before any port; no raw ESC | `switch.rs:582`: every `calls.*` empty, `probes == 0` | MET |
| 13 | Switch to the current session | `switch.rs:613` | MET |
| 14 | Reset creates, shows and records a fresh session | `reset.rs:59` | MET |
| 15 | Reset is doctor's remedy, (a) and (b) | `reset.rs:110` and `reset.rs:127`. Doctor's own remedy string, `holler pane reset demo-c1r1`, parses and is the argv that runs | MET |
| 16 | The old session stays as the one stray | `reset.rs:153` | MET |
| 17 | Reset refusals create nothing | `reset.rs:166`: four refusals, with the sessions listed through Q's server | MET |
| 18 | A failure after create names the unrecorded session | `reset.rs:250` | MET |
| 19 | Reset mismatch records nothing | `reset.rs:272` | MET |
| (A1) | A failed create: no step and no created note | `reset.rs:229` | MET |
| 20 | `stub.rs` entries removed, `// #645` kept | `process/stub.rs:23` keeps `// #645`, with no entries; `pane_cli_process` 35/35 | MET |
| 21 | The `cli-surface.txt` `# #645` block, exactly | the diff matches the brief line for line; `cli_surface_test` 3/3, `docs_cli_test` 3/3 | MET |
| 22 | ADR 0003 rows 51-52 | the exact text, with `#645` at column 67, like rows 44-45, 48-49, 55-56 and 61 | MET |
| 23 | Help names the arguments, and no `--first` | `switch.rs:632` (it fails on `origin/main`, per T-red) | MET |
| 24 | ADR-0021 holds exactly Decision 17's edits | against `origin/main` (`bd5e825`): 4 hunks (row `:424`, the end of section 8, the end of section 11, "Deferred"). `grep -n 'Switch and reset as built (#645)'` gives one line, `:365`, between `### 8.` (`:305`) and `### 9.` (`:392`). The `unavailable` decision is stated once in section 8. `#645` appears 9 times, against 6 on `origin/main`. #644's paragraph is not on `main`, so the #645 paragraph states the decision itself, as Decision 17(b) says | MET |
| 25 | clippy, rustfmt, lint, workspace tests, no new dependency | T-green round 6 (see the T precondition). `git diff origin/main -- '*Cargo.toml' Cargo.lock` is 0 lines | MET |
| 26 | CHANGELOG entry | `CHANGELOG.md:217-231`, under `## [Unreleased]` (`:8`) / `### Enhancements` (`:10`), ending "Part of [#645]"; `changelog-check: ok` | MET |

**The issue's criteria** (the source of truth):

| Issue criterion | Evidence | Status |
|---|---|---|
| No call types into a TUI, and the test fails if one does | the I4 check in `both_with` on every run, over the rig's logged `FakeHerdr` and `FakeHost` | MET |
| Refusals are named, with stable codes | every refusal test asserts its code, in JSON and by exit code | MET for 645a's refusals |
| A switch to a deleted session fails and changes nothing | AC 3 | MET |
| The first message lands in the new session and nowhere else | no port sends a prompt (brief C-2, B-2, F-1) | **Deferred to 645b** (Decision 1). The switch form of the wrong-session incident is AC 4 |
| A pane outside P is refused and nothing changes | AC 10, AC 17 | MET |
| Every refusal and success passes the envelope helper; exit codes equal across formats | `both_with` (`switch.rs:126-140`) | MET |
| Scope: unhealthy server and orchestrator's pane refused; not idle or a held question | AC 5, 6, 17 | MET; not idle and held question are 645b |
| `--format=text\|json` through `output::emit()` | `emit_outcome` calls `emit` (`pane/switch.rs:143`). The usage path calls `emit_error`, which is `emit` | MET |
| Only #638's fakes and the envelope helper; no new dependency; no new `unsafe` | the doctor rig over the test kit; `check_envelope`; the Cargo diff is empty; no `unsafe` in the added lines | MET |

## Spec compliance

- **The public API** is the brief's, item for item.
  - In `tx_switch.rs` (`:52-155`): the three `RefusalCode`s, `SESSION_ID_MAX`, `parse_session_id`, `Target`,
    `SwitchRequest`, `Switched`, and `SwitchFailure` with `From<PaneError>` and `message`, then `switch`.
  - In `pane/switch.rs`: `PaneSwitch`, `run`, `Verb` and `emit_outcome` (`:33`, `:48`, `:63`, `:123`).
  - In `pane/reset.rs`: `PaneReset` and `run` (`:22`, `:36`).
- **P0 to R run in the brief's order.**
  - The plan is `plan` (`:191`): `parse_session_id` (the P0 re-check), `read`, `refuse_orchestrator`, `check_health`, and
    then, for `Existing` only, `check_listed` and `check_unclaimed`.
  - Then `switch` (`:155-187`):
    - A1 `create_session` converts with `acted: false`.
    - A2 `select_session` (`:173-176`), O1 `observe` (`:177`) and R `cas_put` (`:179-182`) each map their error with
      `acted` and `created`.
  - `recorded` (`:327`) clones the record and sets exactly four fields.
  - The messages of P2, P3, P5 and O1, and `SwitchFailure::message`, are word for word the brief's. `screen_text` (`:317`) is
    word for word reconcile's.
- **Decisions 1-19 are implemented as stated.** Unchanged since round 1, which listed them one by one. F round 5's split
  of the private `select_and_observe` changed no call, order, error or message, and T's mutations pin the `acted`
  boundary on both sides.
- **The outside gate's items that O routed to S** (`decisions.md:668-669`) all hold:
  - NV-1: `read` searches by name (`tx_switch.rs:212-225`), as below.
  - NV-2: `ServerDown` maps to `pane.map(relaunch_command)` (`findings.rs:127-131`), so `check_health`'s `None` arm cannot
    be reached with a named pane, and AC 5 pins the relaunch text.
  - NV-3: AC 17 compares the sessions through Q's server (`reset.rs:202-206`).
  - NV-4: the code is `ErrorCode::from(&failure.error)` (`pane/switch.rs:138-141`), and `both_with` asserts equal exit
    codes.
  - NV-5: the paragraph is at `:365`, inside section 8.
  - W-3: see the "Deferred" deviation below.
- **Deviations.** Each is documented, and each is still acceptable:
  - P1 uses `find` by name, not `into_iter().next()`. Under `resolve`'s contract it is the same pane, and it does no
    indexing, which was the point of the brief's rule.
  - `reset.rs` reuses `switch::{execute, Verb}`, and `execute` prints through `emit_outcome`, so nothing is copied.
  - The "Deferred" bullet reads "#645 for switch and reset; #644 to follow". Decision 17(c)'s single form applies only if
    #644's edit of that bullet is on `main`, and it is not (`bd5e825`), so this form is accurate. Whichever of #644 and
    #645 lands second writes the single form.
  - The #645 paragraph has four sentences that Decision 17(b) does not list (A-dup round 1's fix). They are inside AC 24's
    "end of section 8".
- **No ADVISORY-HOLD.** The brief's one gap, ADR-0021 `:485`, is minor (advisory 8). The split is reasoned and recorded
  (C-2, Decision 1).

## Quality audit

- **Correctness and failure handling.**
  - Every refusal comes before any write or live change.
  - The record write is one compare-and-swap at the plan's generation, never retried.
  - Nothing is compensated after the act.
  - A mismatch fails closed as `unavailable`, records nothing and ends with the reconcile step.
  - A failure before `select_session` prints no step, which AC 5, the usage cases and the create failure pin.
  - A session that reset created and did not record is named in the message.
  - A port's `Err` is passed on unchanged.
  - Accepted risk: two concurrent switches to one session can both pass P5 (R-5). The ADR states this, and F-3 is its
    follow-up.
- **Build guards.**
  - No `unwrap`, `expect`, `panic!`, `todo!`, `unreachable!`, indexing, `unsafe` or logging in the three production
    files.
  - No `#[allow]` in any touched file.
  - Every touched file is under 900 lines: `tx_switch.rs` 334, `pane/switch.rs` 160, `pane/reset.rs` 45, test `switch.rs`
    649, test `reset.rs` 288, `doctor.rs` 501, `ADR-0021.md` 661, `CHANGELOG.md` 790.
  - No dead code: clippy `-D warnings` is clean (T).
- **Protocol.** No wire, golden or `docs/protocol/v2.md` change, and none is needed. The three codes are open
  `RefusalCode`s, declared with `from_static` in the verb's own file. The section 9 row lists them, and the closed-code
  table is untouched.
- **Tests.**
  - The tests run in-process over the test kit's fakes, the tier the issue names.
  - No sleeps. The clock is asserted as a window.
  - AC 9's concurrent writer is injected deterministically.
  - The RED-first evidence is in T-red.
- **Security.**
  - `SESSION` is typed to `[A-Za-z0-9_-]{1,64}` before any port is called.
  - Harness ids are quoted in every message and text line.
  - The reconcile step is built from a validated `PaneName`.
  - JSON `data.pane` is the stored record, which `pane get` already prints.
- **Documentation.**
  - The CHANGELOG entry, the ADR 0003 rows and the ADR-0021 edits.
  - No README or `docs/` page describes the pane verbs. The merged verb stories #643 and #646a changed only the ADRs.
- **Public-repository privacy.** Clean.
  - Scanned: the 5,080 added lines of the diff, and the patch and message of every branch commit against its first
    parent.
  - The pattern set was the one the operator's handoff rule names (on `docs/660-placeholder-scrub`, `8f81cf6`): home and
    machine paths, user and host names, pane and agent identifiers. Also tailnet names, IPs, emails and key-like
    strings.
  - This story's own lines hit nothing. The only hit is the brief's own rule statement.
  - The history hits (two fleet pane identifiers, an operator name, and a placeholder home path in test data) are all in
    `origin/main`'s files that the merges brought in: `docs/handoffs/640/`, `docs/handoffs/0660-output/`, the adapters'
    test data. The scrub branch handles the handoff ones.
  - Repo-relative `.claude/worktrees/...` paths match `CLAUDE.md` and the scrub branch's own handoffs.
- **Commit hygiene.**
  - 23 commits, all with Conventional subjects, a `Co-Authored-By` trailer, and the GitHub no-reply address as author
    and committer.
  - None carries a session link. `main`'s squash commits (`bd5e825`, `abdcbb6`, `d9eabbb`, `13edbb4`) carry none either,
    so this is repo practice, not this story's defect.
  - No PR exists yet (advisories 1-3).
- **Pipeline records.** One hygiene defect, not in F's work. O's commit `cce6f00` force-added
  `docs/handoffs/645-diff-result-r6.md` and its `.usage.json`.
  - `.gitignore:23` and `:25` exclude both kinds of file ("Handoffs are tracked, but their debug artifacts are not").
  - The playbook's `new-repo-setup.md:121-124` says the same.
  - `origin/main` tracks no such file.
  - F's round-4 entry calls r4's files gitignored.
  - Their content is clean: a review of public code, and token counts. Advisory 4 removes them at the PR step.

## Scope check

F's files are exactly the brief's Files list:
- Three production files.
- Two test files, the two `stub.rs` deletions and `doctor.rs:10`.
- `cli-surface.txt`, ADR 0003, ADR-0021 and the CHANGELOG.

No frozen file, test-kit file, #647 file beyond `doctor.rs:10`, or `Cargo.toml` is touched; I checked with
`git diff --stat` over each. `execute`, `request`, `Outcome`, `render` and `session_text` are small private structure. T's
two extra tests pin the brief's A1 row and the `acted` boundary, so they are inside the story. The two r6 gate files are
the one item outside the scope, and they are O's records, not F's code (advisory 4). The issue's not-idle and
held-question refusals, `--first`, and "the first message lands in the new session" are 645b's, by Decision 1. This PR is
"Part of #645" and must not close it.

## Verdict

**PASS.** Every brief AC and every issue criterion in 645a's scope is met on the tree that will land. The work is
spec-compliant, and its quality is acceptable. Ready for O.

**Why not REWORK for the r6 files.** S's REWORK sends F through the whole loop: F, T, A-dup, the outside gate and S. Two
force-added debug files are O's own commit. Untracking them changes no product file, spec, test or verdict, and O is the
next to act on this branch anyway, at the PR step (advisories 1-4). They do not break CI, the merge or privacy, which is
what made round 1's merge a REWORK. Another loop would also risk `origin/main` moving again, since open PR #718 conflicts
in ADR-0021. So it is a required pre-merge action for O, not a rework item for F.

## Advisory notes

**Before merge (O, at the PR step).**
1. **The PR body must not close #645.** The script opens the PR with `Closes #645.` (`coding-pipeline.workflow.mjs:4097`
   and `:4822`; no argument overrides it). Merging that closes the issue while 645b's scope is undelivered. Replace it with
   "Part of #645 (645a); #645 stays open for 645b", as PR #711 did for #646a (and #646 is still open).
2. **Title and squash subject.** The script's `Implements #645` is not Conventional. Use, for example, `feat(cli): holler
   pane switch and reset (#645 part 1 of 2)`, as #711 and `bd5e825` did.
3. **The AI disclosure** in the PR body (`CONTRIBUTING.md:25-29`), added with `gh pr edit` (`CLAUDE.md`).
4. **Untrack the two r6 gate files:** `git rm --cached docs/handoffs/645-diff-result-r6.md
   docs/handoffs/645-diff-result-r6.md.usage.json`, then commit. The files stay on disk, ignored. The verdict is already in
   `decisions.md:652-671`, whose evidence line can say the files are gitignored, as F's r4 entry does.
5. **Re-check `git merge-tree --write-tree HEAD origin/main` just before merging.** Open PR #718 (#644 part 1) conflicts
   with this branch in ADR-0021 (A-dup's notes). If #718 lands first, re-resolving it by anchor text (R-1, Decision 17(b)
   and (c)) and re-checking AC 24 is an F round, not an O edit.

**After merge.**
6. **P3.** #645's body still does not mention the split (last updated 2026-10-08 18:45 MDT, no comments). Epic #633's
   wave table should list 645b. P1 and P2 are still PROPOSED for the operator.
7. **Follow-ups to file.** None was filed as of A-dup's search at 01:34 MDT.
   - F-1: the stray that every reset leaves.
   - F-2: the rig consolidation, which fires when #718 merges.
   - F-3: one session per pane in the registry's compare-and-swap, before #654.
   - F-4, widened: the lead-in, the append rule and the screen wording move into `findings`, including the `None` that
     means "cannot tell" from #642's adapter.
   - The scoped-read fold (A-dup warn 6).
   Once F-4 is filed, the comment at `tx_switch.rs:314-316` names its issue.
8. **ADR-0021 `:485`** says "#645's ... planned", which goes stale once 645a lands. Decision 17 ("No other ADR-0021 line
   changes") missed this line, and AC 24 allows no fifth hunk, so F was right to leave it. It is too small for an
   ADVISORY-HOLD: the sentence's claim ("are all refusals") stays true, and row `:424` lists the codes. Fix it in the next
   ADR-0021 edit, by whichever of #644 and #645 lands second.
9. **ADR-0021 section 12's** "prints the reconcile step" is broader than the #645 paragraph (A-dup warn 4). #718 edits
   section 12.
10. **The flake.** `body_run_test::fresh_hello_and_presence_on_every_reconnect` failed once under load (T). It is outside
    this story. File it if it recurs in CI.
11. **The journal.**
    - The O entry at `decisions.md:652` is stamped 01:40 MDT, but `cce6f00` committed it at 01:26:38 MDT, 14 minutes
      earlier.
    - The r6 file calls itself "Round 1", which is the script's numbering.
12. **For 645b.**
    - `--profile` typing through `list::profile_name` (A-dup warn 7).
    - The switch test builds as a `faulted`-style helper (A-dup warn 2).
    - Whether a timed-out `create_session` can leave a session that the message cannot name.
