# Handoff-F: Phase 5, rework round 2 - #645a `pane switch` and `pane reset` (the anti-duplication BLOCK)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`). This round starts at `da53aba`,
the A-dup BLOCK. Its work is uncommitted, and the Workflow script commits it after this phase. The work is a merge of
`origin/main` (`d9eabbb`), staged with `git merge --no-commit --no-ff`, and the edits below. Round 1's handoff stays in
git at `fa59fe7`, and its content that still holds is kept below.
**Issue:** #645 (part 1 of 2, 645a)
**Rework of:** `docs/handoffs/645/handoff-A-dup.md`, finding 1 (block). Findings 2-4 are warns that O files as
follow-ups. They need no change in 645a.

## What was done (round 2)

- **Merged `origin/main` (`d9eabbb`: #641, #643, #646 part 1, #662 part 2, #663, #704)**, as A-dup's note 1 asks.
  - The merge is clean. The files both sides touch are ADR-0021, ADR 0003, the CHANGELOG, `cli-surface.txt` and
    `process/stub.rs`, and git merged each of them without a conflict.
  - The merge is left **staged, not committed**, because F does not commit. The script's phase commit (`git add -A`,
    then `git commit -m ...`) runs while `MERGE_HEAD` is set, so it records the merge commit with both parents.
  - `gitleaks protect --staged` on the staged merge found no leaks, so the pre-commit hook will pass.
- `docs/adr/ADR-0021.md`: **four sentences** added to the "Switch and reset as built (#645)" paragraph (section 8,
  now lines 345-370). Nothing else in the ADR changed this round. The new text, after "...which selects that session
  again.":
  > That step follows every failure once `select_session` has been called, a record conflict or a `timeout` included; a
  > failure before then has moved neither the TUI nor the record, and its message has no step. Whether or not the verb
  > ran with `--profile`, the step names the pane, unlike the two forms of step 6 that the generations rule above gives.
  > Those name no pane because a failed launch can leave a pane with no record, which a pane-scoped doctor refuses. A
  > switch or reset reads the pane's record before it acts, so a doctor run for that pane is not refused, and `--fix` is
  > the pane doctor's own remedy for a TUI that does not show the session of record, while a fix can repair it. Neither
  > verb edits P's specs, so no `profile show` is needed.

  The rest of the paragraph is round 1's text. Its last five lines are re-wrapped and their words are unchanged.
- `crates/holler-pane/src/tx_switch.rs`: **two doc comments only**, in the module doc and in `SwitchFailure::message`.
  "once the act has begun" becomes "once `select_session` has been called", and the module doc gains the clause on a
  failure before it. No code changed.
- `docs/handoffs/645/evidence.md`: four new entries under "Added by F (round 2)". They cover #663's generations rule and
  step 6 on `origin/main`, doctor's read of a named pane, and doctor's `--fix` remedy. A header note says the merge
  changed none of the quoted source files.
- `docs/handoffs/645/decisions.md`: the F round-2 entry.

## Design decisions (round 2)

1. **The pane-scoped `--fix` step stays; the ADR states the exception.** This follows A-dup's recommendation. Step 6
   gives the reason its two forms name no pane: a pane-scoped doctor refuses a pane with no record, which is what a
   failed launch leaves. That does not apply to switch and reset:
   - Their plan read the pane's record.
   - The printed step carries no `--profile`, so the doctor reads the pane by name and refuses only a pane with no
     record (`reconcile.rs:226-236`).
   - `--fix` is doctor's own remedy for that mismatch (`findings.rs:135-141`). The bare form would repair nothing.

   Taking step 6's form instead would change behaviour in ACs 7, 8, 9, 18 and 19. The step would also have to move into
   `holler-cli`, because `holler-pane` cannot call `reconcile_step`. A did not recommend that, and the brief's contract
   is the pane form.
2. **Where the fix goes.** It goes in the #645 paragraph only, inside AC 24's "end of section 8". The generations bullet
   and step 6 are #663's text, and editing them is outside AC 24.
3. **The boundary is `select_session`, not "the act".** A's draft said "once the act has begun". The paragraph defines
   reset's act as `create_session` and then `select_session`. A failed `create_session` carries no step: the error goes
   through `From<PaneError>`, so `acted` is false. That matches `SwitchFailure::acted`'s doc and the brief (A1: "`acted:
   false`"). So the sentence names `select_session`, and says that a failure before it moved neither the TUI nor the
   record. This is the brief's Decision 13 ("with the reconcile step when `acted`"), stated for section 12's "prints the
   reconcile step". It is the same reasoning as step 2 of the I8 order, where a conflict before anything live changed
   prints no step. My round-1 doc comments in `tx_switch.rs` had the same imprecision, so they now say the same thing.
4. **"Whether or not the verb ran with `--profile`".** The generations rule gives a `--profile` verb the profile-scoped
   form (doctor `--profile` and `profile show`). Switch and reset print the same pane line with `--profile` too. The
   paragraph now says so and gives the reasons. Neither verb edits P's specs, so `profile show` has nothing to report,
   and a doctor run by name is never refused for a pane outside P.
5. **Wording limits.**
   - The new text has no inline `holler ...` code span (E-7, `docs_cli_test`).
   - It does not say "the remedy table", which the ADR never defines. It says "the pane doctor's own remedy".
   - It says "is not refused" rather than A's "is never refused" or "always has one". A record that another writer
     deletes after the plan would still be refused (`pane-not-found`), and that edge is not the reason step 6 gives.
6. **Merge with `--no-commit`.** A-dup asked F to merge, and F's role makes no commits. A staged merge does both: the
   script's own phase commit completes it. The other way, F making a merge commit of its own, would break the role
   rule for no gain.

## Design decisions (round 1, unchanged)

1. **One after-clap path for both verbs** (`switch::execute`): typing `PANE` and `--profile`, the clock, the engine call
   and the output are the same for both, so `reset.rs` is its `Args` struct and a one-call `run`.
2. **The order of usage errors.** `switch::run` types `SESSION` first (pure) and `execute` reports it after `PANE` and
   `--profile`, so the brief's step-1 order holds.
3. **P1 takes the resolved pane by name** (`find`), not blindly the first (see Deviations, round 1).
4. **The error's text goes into the message unchanged** (no `findings::embedded` pass). Every value the engine itself
   writes into a message (the target, SHOWN, the created id) goes through `quoted`.
5. **The `data` struct** is `{verb, pane, previous}`, `previous` `null` when there was none; `Verb` serializes lowercase.
6. **Comments cite ADR-0021 sections and issues, never the brief's labels** (`docs/handoffs/` is deleted before the push).
7. **The CHANGELOG entry sits after #647's doctor entry.** It still merged cleanly with `origin/main`.

## Reuse / extend-vs-new

Unchanged from round 1. A-dup's table confirms that every Reuse map row is extended or reused, and round 2 adds no code.
The objects extended are the `tx_switch.rs` stub and the two verb stubs, on doctor's `--fix` repair: select, observe,
record. These are reused:

- `findings::doctor_command(Some(pane), true)`, `findings::quoted` and `reconcile::shown_differs`;
- `FindingKind::ServerDown.remedy(..)`, with a `doctor_command` fallback;
- `ports.scope.resolve`;
- `output::{emit, emit_error, ErrorBody, ErrorCode, VerbCtx}` and `ProfileOpt`;
- `holler_proto::clock::now_millis`.

The one justified copy is `screen_text` (O1).

## Architecture notes for A

- **Round 2:** none. No module, public interface, dependency or behaviour changed: only ADR prose and two doc comments.
  The merge brings `origin/main`'s own reviewed changes. Among this story's files they touch only the shared
  docs and fixtures listed above. `git diff origin/main -- '*Cargo.toml'` is empty.
- **Round 1, as built:** the engine is in `holler-pane`, pure over `Ports`, with no I/O, no async and no new dependency.
  `tx_switch` uses `findings` and `reconcile::shown_differs`. `pane/switch.rs` uses `holler_pane::tx_switch`, and
  `pane/reset.rs` uses `pane/switch.rs` (`execute`, `Verb`). No frozen file and no #647 file is touched.

## Deviations from spec / wireframe

Round 2:

1. **A-dup's note 4 says "no code or test change".** The behaviour is unchanged: `SwitchFailure::message` and
   `RECONCILE_P` are as they were, and no test changed. Two doc comments in `tx_switch.rs` were corrected to match the
   new ADR sentence (design decision 3).
2. **The ADR text differs from A's draft in three places** (design decisions 3-5): "once `select_session` has been
   called", the `--profile` sentence, and "is not refused".

Round 1, still standing: P1 uses `find` by name; the "Deferred" bullet takes A's round-2 form, "#645 for switch and
reset; #644 to follow". That is still right, because #644's paragraph is not on `origin/main`. `reset.rs` reuses
`switch::{execute, Verb}`, and the CHANGELOG entry keeps its position.

No wireframe applies (no UI surface).

## Tier 1 self-check (incl. tests now GREEN), on the merged tree

Every build ran with `CARGO_BUILD_JOBS=4`.

```
$ git merge --no-commit --no-ff origin/main
Automatic merge went well; stopped before committing as requested
$ gitleaks protect --staged --redact --no-banner          -> no leaks found (exit 0)

$ cargo build -p holler-pane -p holler-cli                 -> Finished (exit 0)
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 133 filtered out
   (all 13 switch cases, all 6 reset cases, and help_names_the_arguments)
$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test
     Running tests/cli_surface_test.rs                          test result: ok. 3 passed; 0 failed
     Running tests/docs_cli_test.rs                             test result: ok. 3 passed; 0 failed
     Running tests/pane_verbs/process/main.rs (pane_cli_process) test result: ok. 34 passed; 0 failed
     Running tests/pane_verbs/main.rs (pane_verbs)              test result: ok. 153 passed; 0 failed
   (153 = round 1's 113 + the list, get, watch and park cases that origin/main brought)
$ cargo clippy --workspace --all-targets -- -D warnings     -> exit 0, no warnings
$ rustfmt --check --edition 2021 crates/holler-pane/src/tx_switch.rs crates/holler-cli/src/pane/switch.rs \
    crates/holler-cli/src/pane/reset.rs crates/holler-cli/tests/pane_verbs/switch.rs \
    crates/holler-cli/tests/pane_verbs/reset.rs              -> exit 0
$ bash scripts/lint.sh                                       -> exit 0 (only the pre-existing size warning for error.rs)
$ bash scripts/changelog-check.sh                            -> changelog-check: ok
$ git diff origin/main -- '*Cargo.toml' | wc -l              -> 0
$ wc -l tx_switch.rs pane/switch.rs pane/reset.rs ADR-0021.md -> 321, 160, 43, 633

AC 24 on the merged tree:
$ git diff origin/main -- docs/adr/ADR-0021.md   -> four hunks: section 8's end (the #645 paragraph), the switch and
                                                    reset row of section 9, section 11's end, and "Deferred"
$ grep -n 'Switch and reset as built (#645)' docs/adr/ADR-0021.md
345:**Switch and reset as built (#645).** ...       (### 8. is line 285, ### 9. is line 372)
$ grep -c '#645' docs/adr/ADR-0021.md    -> 9 (origin/main: 6)
```

**`cargo test --workspace`**, in CI's form (`-- --skip roster_stays_accurate_under_concurrent_body_load`) with
`--no-fail-fast`, on the merged tree: **exit 0**. That is 135 result lines, 1641 passed, 0 failed and 14 ignored, and
includes `pane_verbs` (153), `pane_cli_process`, `cli_surface_test`, `docs_cli_test` and `wire_selftest`.

Two edits came after that run started. One re-wrapped three lines of the `tx_switch.rs` module doc and changed no
words. The other was the last wording pass on the ADR paragraph. So clippy (workspace, all targets), the four test
targets above, rustfmt, `lint.sh` and `changelog-check.sh` were run again on the final tree, and all passed. The test
counts were the same.

## Evidence appendix

`docs/handoffs/645/evidence.md` has 21 facts in unchanged code or text: 15 from F round 1, 2 from T and 4 from F round 2.
The round-2 entries are:

- `docs/adr/ADR-0021.md:294-299`: #663's generations rule (the step after a post-act conflict names no pane);
- `docs/adr/ADR-0021.md:331-337`: step 6's two forms and its reason;
- `crates/holler-pane/src/reconcile.rs:226-236`: a doctor run named by pane, without `--profile`, refuses only a pane
  with no record;
- `crates/holler-pane/src/findings.rs:135-141`: `doctor <pane> --fix` is doctor's remedy for a fixable mismatch.

No source file quoted there differs from `origin/main`, so every round-1 line number still holds.

## Tests that look wrong (for T)

None. No test changed in either round. The A-dup finding was about the standing spec, not the tests. AC 9's assertion
(the message ends with `; to reconcile, run holler pane doctor demo-c1r1 --fix`) is now what the ADR states.

## Known issues

- **A-dup's warns 2-4 are for O to file.** Each needs a filed issue, and none needs a change in 645a:
  - the lead-in `to reconcile, run ` and `screen_text`, each spelled twice. Follow-up F-4 is out of date: `reconcile_step`
    is now on `main`, in `holler-cli/src/pane/profile_scope.rs:37-56`.
  - the third private copy of the scoped-read arms;
  - the fourth both-format test runner (F-2 has triggered).

  Follow-ups F-1 and F-3 still need filing too. Once O files the `screen_text` fold, the comment in `tx_switch.rs`
  should name that issue. It says "a follow-up of #645" today.
- **Observation, behaviour unchanged.** When reset's `create_session` times out, the server may still hold a new
  session. The message cannot name it (`timed out: harness.create_session`) and has no step. That is the brief's A1
  row (`acted: false`, `created: None`). Doctor reports such a session as a `stray-session` on its next run. If O wants
  the message to say that a session may exist, it is a small 645b change.
- **The accepted risks are unchanged:** the `session-of-other-pane` race (R-5, F-3), the window before 645b (R-3), and
  the `stray-session` that every successful reset leaves (F-1).
- **Unrelated and pre-existing:** the real binary prints a `logging_started` line on stderr before every pane verb's
  output. Round 1 saw the `join_held_test` flake once under load.

## Files changed

Production (this story, both rounds):
- `crates/holler-pane/src/tx_switch.rs` (round 2: two doc comments only)
- `crates/holler-cli/src/pane/switch.rs`
- `crates/holler-cli/src/pane/reset.rs`

Docs (this story):
- `docs/adr/ADR-0021.md` (round 2: the #645 paragraph only)
- `CHANGELOG.md` (round 1; unchanged in round 2)

Merged from `origin/main` (`d9eabbb`, not this story's work; staged as the merge): the 42 files of
`git diff dc300ab d9eabbb --stat`.

Pipeline records:
- `docs/handoffs/645/handoff-F.md` (this file)
- `docs/handoffs/645/evidence.md` (round-2 entries appended)
- `docs/handoffs/645/decisions.md` (the F round-2 entry appended)
