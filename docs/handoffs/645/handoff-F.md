# Handoff-F: Phase 5 (Workflow script Phase 6), rework round 4 - #645a `pane switch` and `pane reset` (S's REWORK: merge `origin/main`)

**Date:** 2026-10-10 (00:24 MDT)
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`). This round starts at `ddb6fc3`,
S's REWORK. **The merge of `origin/main` (`abdcbb6`) is staged and not committed.** `MERGE_HEAD` is `abdcbb6`, and the
Workflow script's phase commit (`git add -A`, then `git commit`) records it with both parents, as `37f2101` did in round 2.
Round 3's handoff is in git at `62aae72`, round 2's at `37f2101` and round 1's at `fa59fe7`. Their content that still holds
is kept below.
**Issue:** #645 (part 1 of 2, 645a)
**Rework of:** `handoff-S.md`, REWORK item 1: merge `origin/main` (`abdcbb6`) and resolve the `docs/adr/ADR-0021.md`
conflict by keeping both sides. Then AC 24 and AC 25 are checked again on the merged tree.

## What was done (round 4)

- **Merged `origin/main` (`abdcbb6`)**, which brings #713 (#660, the output envelope conformance suite) and #714 (#640
  part 3, the Herdr adapter over the socket API). I ran `git merge --no-commit --no-ff origin/main`. `CHANGELOG.md` and
  `crates/holler-cli/tests/pane_verbs/process/stub.rs` merged cleanly. `docs/adr/ADR-0021.md` had one conflict in two hunks,
  which S had predicted. ADR-0021 sections 8 and 11 merged cleanly, and their #645 text is unchanged.
- **`docs/adr/ADR-0021.md`: both conflicts resolved by keeping both sides**, as S's item 1 asks:
  - **The section 9 table** (`:403-404`). The `pane launch`, `pane relaunch` row is `origin/main`'s, ending `; open
    (#640) for a cell that no single split reaches, `grid-unreachable``. The `pane switch`, `pane reset` row is this branch's
    Decision 17(a) row.
  - **"Deferred to named stories"** (`:608-615`). This branch's mismatch-code bullet and its **PROPOSED (#645, pending
    the operator)** bullet come first. `origin/main`'s rewritten bullet follows ("`HarnessPort` in its final form: #635,
    then #642. `HerdrPort`: #640 implements it as merged; ..."). It replaces the old "`HerdrPort` and `HarnessPort` in
    their final form" bullet, which neither side still has.
  - I staged the resolved file by path (`git add docs/adr/ADR-0021.md`), and no unmerged path remains.
- **`docs/handoffs/645/evidence.md`: citations re-checked on the merged tree.** A script in the session scratchpad compared
  each of the 24 excerpts with its cited lines. #714 added three doc-comment lines to `crates/holler-pane/src/error.rs`
  (now `:415-417` and `:457-458`). That moved three citations down by 3: `error.rs:688-695` became `:691-698`, and
  `:651-653`, `:673-679` became `:654-656`, `:676-682`. Two excerpts had stopped one line short of their cited ranges since
  round 1: `quoted` (`findings.rs:332-334`) lacked its closing `}`, and `emit_json` (`output.rs:288-297`) lacked its
  `settle(...)` line. Both now quote the whole range. The file's header records the round-4 merge. The script now reports
  0 mismatches.
- No code, test, ADR 0003, fixture or CHANGELOG change.

## Design decisions (round 4)

1. **Keep both sides, as S specified.** Taking "ours" whole would revert #714's section 9 row and its `HerdrPort` bullet.
   Taking "theirs" whole would drop this story's row and two of its bullets. Either way AC 24 fails.
2. **Merge, not rebase**, as in round 2. The script commits each phase, and the branch's gate commits (`chore(#645): ...`)
   are the run's record. A rebase would rewrite them, and pushing that branch would need a force-push.
3. **`main`'s class-table sentence is not edited.** On the merged tree, ADR-0021 `:465` reads "the open codes so far
   (#640's `grid-unreachable`, merged; #645's and #646's, planned) are all refusals". Once 645a lands, #645's three codes
   are merged, so "planned" will be out of date for them. I left it alone for three reasons:
   - The rule the sentence states ("are all refusals") still holds. The three codes exit 3.
   - AC 24 and Decision 17 say "No other ADR-0021 line changes", and S's check expects exactly four hunks.
   - The same stale-on-landing status was there before this merge. The base `d9eabbb` read "the open codes planned so far
     (#645, #646)", Decision 17 left it as it was, and every gate passed that.

   It is listed under "Known issues" for O as a one-phrase edit for the next ADR-0021 change.
4. **Evidence fixed in place, not appended.** The entries whose line numbers moved are mine, from round 1. A wrong line
   number would make the outside gate attach the wrong source lines beside the excerpt. So I corrected them, and added no
   new entry: the merge creates no new fact that the diff relies on.

## What was done (rounds 1-3, still standing)

- **Round 3** (the outside diff gate's B-1). `check_health`'s remedy fallback is an explicit `match`, with the same behaviour
  as `unwrap_or_else`:
  ```rust
  let remedy = match FindingKind::ServerDown.remedy(pane, FixState::NotFixable) {
      Some(remedy) => remedy,
      None => doctor_command(pane, false),
  };
  ```
  The `SESSION_ID_MAX` doc says why the two limits agree: `quoted` counts characters and `parse_session_id` counts bytes,
  and an accepted id is ASCII (NIT-1). Two `evidence.md` entries settle NV-1, that `resolve(P, Some(n))` answers exactly
  the pane `n`.
- **Round 2** (A-dup round 1's BLOCK). Merged `origin/main` (`d9eabbb`), and the commit `37f2101` recorded it with both
  parents. Four sentences were added to the "Switch and reset as built (#645)" paragraph. They say when the reconcile step
  is printed: on every failure once `select_session` has been called, and none before. The step names the pane with
  `--fix` even with `--profile`, and the paragraph says why that differs from #663's step 6. Two doc comments in
  `tx_switch.rs` were corrected.
- **Round 1.** `tx_switch.rs` is filled as the brief's P0-R table. Both verbs share one after-clap path
  (`switch::execute`). The ADR-0021 edits follow Decision 17, and there is a CHANGELOG entry.

## Design decisions (rounds 1-3, unchanged)

- Round 3: B-1 was fixed in code rather than argued, because a `match` meets both readings of P3's "no
  `unwrap`/`expect`". NIT-2 was T's, and T took it in round 3. NV-1 was answered with evidence, and P1's `find` stays.
- Round 2: the pane-scoped `--fix` step stays, and the ADR states the exception. The fix is in the #645 paragraph only. The
  boundary is `select_session`, not "the act".
- Round 1: one after-clap path for both verbs. `SESSION` is typed early but reported after `PANE` and `--profile`. P1
  picks the resolved pane by name. The error's text is used unchanged, and every value the engine writes is quoted.
  `data` is `{verb, pane, previous}`. Comments cite ADR-0021 and issues, never the brief. The CHANGELOG entry sits after
  #647's doctor entry.

## Reuse / extend-vs-new

Unchanged. Round 4 extends nothing and copies nothing. A-dup round 2's table still holds:
- The objects extended are the `tx_switch.rs` stub and the two verb stubs, on doctor's `--fix` repair (select, observe,
  record).
- Reused as they are: `findings::doctor_command(Some(pane), true)`, `findings::quoted`, `reconcile::shown_differs`,
  `FindingKind::ServerDown.remedy(..)` (by `match`), `ports.scope.resolve`, `output::{emit, emit_error, ErrorBody,
  ErrorCode, VerbCtx}`, `ProfileOpt` and `holler_proto::clock::now_millis`.
- The one justified copy is `screen_text` (O1).

## Architecture notes for A

- **Round 4:** none in this story's code. The merge brings in #713's tests and #714's Herdr adapter. Neither touches a
  file of this story but `ADR-0021.md`, `CHANGELOG.md` and `stub.rs`, each resolved or merged as above. #714's
  `holler-pane` edits (`error.rs`, `pane.rs`, `ports.rs`, `reconcile.rs`) are doc comments only (`git diff d9eabbb
  abdcbb6`). `git diff origin/main -- '*Cargo.toml'` and `-- Cargo.lock` are both empty.
- **Rounds 1-3:** the engine is in `holler-pane`, pure over `Ports`, with no I/O, no async and no new dependency.
  `tx_switch` uses `findings` and `reconcile::shown_differs`, `pane/switch.rs` uses `holler_pane::tx_switch`, and
  `pane/reset.rs` uses `pane/switch.rs` (`execute`, `Verb`). No frozen file and no #647 file is touched, apart from
  `doctor.rs:10` (Decision 16, T's).

## Deviations from spec / wireframe

Round 4: none. The resolved ADR-0021 holds exactly Decision 17's edits against `origin/main` (AC 24).

Rounds 1-3, still standing and accepted by S:
- P1 uses `find` by name, where the brief says `into_iter().next()`.
- The "Deferred" bullet takes A's round-2 form, "#644 to follow". #644's paragraph is still not on `origin/main` at
  `abdcbb6`.
- `reset.rs` reuses `switch::{execute, Verb}`.
- The #645 paragraph has round 2's four sentences.

No wireframe applies (no UI surface).

## Tier 1 self-check (incl. tests now GREEN), round 4

Every build ran with `CARGO_BUILD_JOBS=4`, on the merged working tree. `origin/main` was fetched again at 00:23 MDT and is
still `abdcbb6`. The open PRs, #715 (`docs/handoffs/0660-output/decisions.md`) and #673 (`Cargo.lock`), touch no file of
this story.

```
$ git merge --no-commit --no-ff origin/main     -> CONFLICT (content): docs/adr/ADR-0021.md only;
                                                   CHANGELOG.md and process/stub.rs auto-merged
$ git add docs/adr/ADR-0021.md; git diff --name-only --diff-filter=U | wc -l   -> 0
$ cat "$(git rev-parse --git-dir)/MERGE_HEAD"   -> abdcbb66691abaa7567fd012bb71340f17c1953e
AC 24:
$ git diff origin/main -- docs/adr/ADR-0021.md | grep -c '^@@'                  -> 4
   (the #645 paragraph in section 8, row 404, the section 11 sentence at :526, the "Deferred" bullets at :608-612)
$ grep -n '^### 8\.\|^### 9\.\|Switch and reset as built (#645)' docs/adr/ADR-0021.md
   -> 285 (### 8.), 345 (the paragraph), 372 (### 9.)
$ the closed failure `unavailable` stated in section 8                         -> once (:352)
$ grep -c '#645' docs/adr/ADR-0021.md                                          -> 9 (origin/main: 6)
AC 25:
$ git diff origin/main -- '*Cargo.toml' | wc -l                                -> 0 (Cargo.lock: 0)
$ cargo clippy --workspace --all-targets -- -D warnings                        -> exit 0
$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test
   cli_surface_test 3 passed; docs_cli_test 3 passed; pane_cli_process 35 passed; pane_verbs 161 passed; 0 failed
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::               -> 21 passed, 0 failed
$ cargo test -p holler-cli --test pane_cli_process -- a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope
   -> ok (#660's new test; with switch and reset gone from STUBS it picks `pane launch`, #644)
$ cargo test -p holler-cli --test pane_verbs -- output_api::                   -> 28 passed (#660's suite)
$ rustfmt --check --edition 2021 tx_switch.rs pane/switch.rs pane/reset.rs \
    tests/pane_verbs/switch.rs tests/pane_verbs/reset.rs                       -> exit 0
$ bash scripts/lint.sh                    -> exit 0 (size warnings only; tests/pane_verbs/switch.rs 613, under 900)
$ bash scripts/changelog-check.sh         -> changelog-check: ok
$ gitleaks protect --staged --redact --no-banner   -> no leaks found (the staged merge, about 154 KB)
$ gitleaks protect --redact --no-banner            -> no leaks found (the unstaged edits)
$ python3 -I check_evidence.py <worktree> docs/handoffs/645/evidence.md   -> entries: 24, mismatches: 0
```

**`cargo test --workspace`** in CI's form (`-- --skip roster_stays_accurate_under_concurrent_body_load`, `ci.yml:128`),
with `--no-fail-fast`, ran on the merged tree from 00:19 to 00:23 MDT and **exited 0**. There are 137 result lines, with
1663 passed, 0 failed and 16 ignored. Round 3 had 1642 passed and 14 ignored over 135 lines. The difference is the tests
the merge brings in:
- #713: 7 tests in `pane_verbs` (154 to 161) and 1 in `pane_cli_process` (34 to 35).
- #714: two new `holler-adapter-herdr` targets. `adapter_messages_test` has 6 tests, and `scratch_herdr_test` has 7
  passed and 2 ignored (the opt-in real-Herdr runs).

## Evidence appendix

`docs/handoffs/645/evidence.md` holds 24 facts in unchanged code or text. 15 are from F round 1, 2 from T, 4 from F round
2, 1 from T round 2 and 2 from F round 3. Round 4 adds no fact and corrects five citations or excerpts, as listed under
"What was done". Every excerpt matches its cited lines on the merged tree.

## Tests that look wrong (for T)

None. No test changed in this round. The merge brings #713's and #714's tests, and all of them pass on the merged tree.

## Known issues

- **ADR-0021 `:465`, a status word that will go stale (for O).** Once 645a lands, "#645's and #646's, planned" will be
  wrong for #645's three codes. A later ADR-0021 edit could write, for example, "(#640's `grid-unreachable` and #645's
  `orchestrator-pane`, `server-unhealthy` and `session-of-other-pane`, merged; 645b's and #646's, planned)". Not edited
  here, because AC 24 allows no fifth hunk (Design decision 3).
- **More ADR-0021 conflicts may come (R-1).** #644 and #642 part b, which the operator is restarting alongside this run,
  both plan ADR-0021 edits next to this story's lines. Whichever lands second resolves the conflict by anchor text.
  `origin/main` is `abdcbb6` as of 00:23 MDT.
- **Carried from round 3, unchanged:**
  - A-dup's warns and follow-ups F-1 to F-4 are for O to file. Once the `screen_text` fold is filed, the comment in
    `tx_switch.rs` should name its issue.
  - A timed-out `create_session` in reset may leave a session that the message cannot name. That is a 645b question.
  - The accepted risks remain: R-3, R-5 and F-1.

## Files changed

Production (this story, all rounds; none changed in round 4):
- `crates/holler-pane/src/tx_switch.rs`
- `crates/holler-cli/src/pane/switch.rs`
- `crates/holler-cli/src/pane/reset.rs`

Docs (this story):
- `docs/adr/ADR-0021.md` (round 4: the two conflicts resolved, keeping both sides; this story's four hunks unchanged)
- `CHANGELOG.md` (round 1; merged cleanly with #714's entry in round 4)

Merged in from `origin/main` (`abdcbb6`), not this story's work: #713's and #714's files, such as
`crates/holler-adapter-herdr/**`, `crates/holler-cli/tests/pane_verbs/output_api.rs`, `process/stub.rs`'s new test, the
`holler-pane` doc comments, `docs/testing.md` and `docs/handoffs/0660-output/`.

Pipeline records:
- `docs/handoffs/645/handoff-F.md` (this file)
- `docs/handoffs/645/evidence.md` (round 4: citations corrected, header updated)
- `docs/handoffs/645/decisions.md` (the F round-4 entry appended)
