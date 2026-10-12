# Handoff-F: Phase 6 (the script's numbering; F), cycle 2 - S's REWORK: merge `origin/main`, renumber #853's ADR-0021 decisions to 32 and 33

**Date:** 2026-10-12, 06:55 MDT
**Branch:** `issue-853-implementation`, on `21b29a0` (S's REWORK), with `origin/main` (`4e691d9`) merged in but not
committed
**Issue:** #853

This replaces cycle 1's handoff, which is in git at `07a576c`. Cycle 1's design decisions, reuse and evidence all still
hold. This cycle changes no behaviour.

| Field | Value |
|-------|-------|
| GitHub issue | #853 |
| Working branch | `issue-853-implementation` (worktree `.claude/worktrees/0853-apply-grid-order`) |
| Build plan phase | none. The issue is the spec (epic #633), with the brief `docs/handoffs/853-brief.md` and ADR-0021 section 8 |
| Input documents read | `handoff-S.md` (the REWORK); `handoff-A-dup.md`, `handoff-T-green.md`, `decisions.md`, `evidence.md`; the brief; cycle 1's `handoff-F.md`. Also #835's journal on `main` (the merge precedent), the script's phase-commit prompt and the repo's git hooks |
| Acceptance criteria count | 1 REWORK item (S's item 1, parts a to d). S found the brief's 6 criteria met on the branch |
| Handoff document path | `docs/handoffs/853/handoff-F.md` (this file) |

The run is automated, so this table is recorded here rather than confirmed by a person. The scope cap does not apply:
the cycle is one REWORK item, made of a merge and text edits.

## What was done

- **(a) The merge.** I ran `git merge --no-ff --no-commit origin/main`. `origin/main` was at `4e691d9`, #835's PR #860,
  merged at 06:29 MDT.
  - It was clean. Git auto-merged the four files both sides touch: `CHANGELOG.md`, `docs/adr/ADR-0003.md`,
    `docs/adr/ADR-0021.md` and `docs/pane-control.md`.
  - Before my edits, the staged tree matched `git merge-tree --write-tree 21b29a0 4e691d9`: both are `c2941f8`.
  - **I did not commit it.** `MERGE_HEAD` (`4e691d9`) is left for the script's phase commit. That commit runs
    `git add -A` and `git commit`, which concludes the merge as a two-parent commit. #835's run did the same
    (`526bbd3`).
  - It is a merge, not a rebase. `origin/issue-853-implementation` already holds `05d8e6a`, and the PR phase's push
    must be a fast-forward.
- **(b) The decision list.** In ADR-0021's "Decisions taken", #853's `30.` is now `32.` (`:1591`) and its `31.` is now
  `33.` (`:1597`).
  - The list reads 1 to 33, each number once and in order.
  - I also removed the blank line the merge left between #835's 31 and #853's pair. The list is one tight list again,
    as it is on `main`.
- **(c) The cites in production code and docs.** These are 6 of S's 13:
  - `docs/adr/ADR-0021.md:742` now cites decision 32, and `:744` cites decision 33.
  - `docs/adr/ADR-0021.md:860`, section 9's `profile apply` row, cites decision 33.
  - `CHANGELOG.md:615` says "(32 and 33)".
  - `crates/holler-cli/src/profile/apply.rs:12` and `:150`, both doc comments, cite decision 33.
  - The other 7 are comments in test files, which are T-green's. They are listed under "Tests that look wrong" below.
- **S's advisory 2 (optional): done.** `docs/adr/ADR-0021.md:713` now reads "0 for a dry run that does not fail, or a
  real run with no failed or refused step, which prints its steps". The wording is S's.
- `docs/handoffs/853/evidence.md`: one new entry (`main`'s decisions 30 and 31 are #835's).
- `docs/handoffs/853/decisions.md`: this cycle's entry.

## Design decisions

- **The merge is staged, not committed.** F does not commit, and the script's phase commit concludes a merge that is
  in progress.
  - Precedent: #835's T-red (`526bbd3`) and its later F cycles merged `main` the same way.
  - The hooks allow it. The `commit-msg` hook accepts the script's Conventional subject. `prepare-commit-msg` adds the
    `Co-Authored-By` trailer for a `-m` message. The pre-commit gitleaks scan runs on the staged merge.
- **The blank line.** I dropped it so that `main`'s list stays tight. It sits inside the lines that S's item (b) names.
- **Advisory 2.** S offered this edit while F was in the file anyway.
  - It fixes the one sentence in section 8 that leaves out a dry run planning a refused step. The section's own
    "A `--dry-run` is not changed" paragraph, the guide and the code all give that run exit 0.
  - I used S's wording and did not rewrap the paragraph, so the diff is one line.
- **Advisories I did not take, by owner:**
  - Advisory 1 (rewriting `0064fac` before the push) belongs to the operator, and F rewrites no history. Now that the
    merge sits on top, a rewrite has to replay the merge (`git rebase --rebase-merges` from `05d8e6a`) or redo it.
  - Advisory 3 (#664's CHANGELOG entry): left as written, which is the repo's practice.
  - Advisory 4: still parked for the operator.
  - Advisories 5 and 6 are test-side options, so they are T's.
  - Advisory 7 belongs to the merging agent.
  - Advisory 8 (size): this cycle does not touch the test kit's `herdr.rs`, which stays at 598 lines.

## Reuse / extend-vs-new

No new object: this cycle is a merge and text. Cycle 1's map (handoff-A.md) still stands: `cell_key`, `refused()`
beside `stopped()`, and `check_split`.

## Architecture notes for A

Nothing from #853 changes.

- The merge brings in #835's code exactly as it was merged on `main` and reviewed in #835's own run:
  `HerdrPort::observe_pane` and `HostPort::foreground_commands` become required, and the import path changes.
- The merge changes none of #853's code or test files. The only files both sides touch are the four docs.
- Against `main`, the PR diff is #853's alone. It covers the same 23 files outside `docs/handoffs/`, with the same
  per-file line counts as against the old base. The only differences are this cycle's edits.

`archChanged: false`.

## Deviations from spec / wireframe

None. Two edits go slightly past the letter of item 1: the blank line, which is inside the list S named, and advisory
2, which S offered in S's own words.

## Tier 1 self-check (on the merged tree)

These ran in the worktree on a warm build, on the staged merge plus this cycle's edits. No test file was edited.

```
cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (the verify command)
run 1 (06:50-06:57 MDT): 167 binaries, passed=2605 failed=1 ignored=37
  failed: holler-hub profile_rename_faults_test::a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds
          (:250, assert_ne! on the pane file's inode: left 60177415, right 60177415)
run 2 (07:05-07:12 MDT): 167 binaries, passed=2605 failed=1 ignored=37
  failed: holler-cli remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen
          (:380, last_seen 1791810627 vs 1791810628), the load flake open as #847; the inode test passed
  -> every test passed in at least one of the two full runs

cargo test -p holler-hub --test profile_rename_faults_test, 20 times   -> 19 passed, 1 failed (ext4 inode reuse, below)
cargo test -p holler-cli --test remote_admin_test, 6 times             -> 6 passed
crates/holler-hub, crates/holler-body, crates/holler-proto, remote_admin_test.rs: identical to main and to 21b29a0

From run 1, per binary (S's item d names the first three):
docs_pane_control_test 19 passed (18 before the merge; #835 added one) | docs_cli_test 3 | cli_surface_test 3
profile_verbs 118 | fake_herdr_test 25 | herdr_conformance_test 22 | plan_splits_test 20 | pane_verbs 347
profile_apply_scenario 5 passed, 3 ignored (opt-in) | wire_selftest 3 | import_test 5

cargo clippy --workspace --all-targets -- -D warnings   -> exit 0 (holler-cli re-checked after touching its sources)
bash scripts/lint.sh                                    -> exit 0 (all warnings on files this cycle does not touch;
                                                           profile/apply.rs 225, tx_apply.rs 565, herdr.rs 598)
bash scripts/changelog-check.sh                         -> changelog-check: ok
cargo machete                                           -> no unused dependencies
git diff --check; git diff --cached --check             -> clean
rustfmt --edition 2021 --check crates/holler-cli/src/profile/apply.rs -> clean

S's check 1: git grep -n -i -E "decision(s)? (30|31)\b|\(30 and 31\)" -- crates docs/adr docs/pane-control.md CHANGELOG.md
  -> only the 7 test comments listed for T below
S's check 2: ADR-0021 "Decisions taken" -> 1..33, each once, in order
```

**The two failures are flakes in code that neither #853 nor the merge touches.**

- **The inode test (#665).** It asserts that the pane file has a new inode after a save and a save-back. But each save
  writes a new file and renames it over the old one, so on ext4 the save-back can be given the inode number that the
  first replace just freed. No issue tracks it (searched). Filing one is an outward action, so I leave it to the main
  session.
- **`remote_admin_test` (#508).** This is #847's open flake: `last_seen` moves 1 s inside its 2 s window under a loaded
  workspace run. The load average was 7 to 8, with other sessions running.

## Evidence appendix

`docs/handoffs/853/evidence.md`. Cycle 1's entries and T-green's still stand. This cycle adds a section with one entry:
on the merged tree, ADR-0021's decisions 30 and 31 are #835's (`docs/adr/ADR-0021.md:1584` and `:1587`). The entry
also says that the earlier "decision 31" in F's failure-envelope entry now means decision 33. As S asked, the run's
handoffs keep their history as written.

## Tests that look wrong (for T)

No test is wrong in behaviour, and every test passes on the merged tree. But 7 test comments still cite #853's old
decision numbers, and S's item (c) needs them changed. S assigns them to T-green, and F does not edit tests:

- `crates/holler-cli/tests/profile_verbs/apply/order.rs:2`, `:28` and `:95`: "decision 30" becomes 32.
- `crates/holler-cli/tests/profile_verbs/apply/refused.rs:2`, `:31` and `:222`: "decision 31" (or "Decision 31")
  becomes 33.
- `crates/holler-cli/tests/profile_apply_scenario/plan.rs:198`: "decision 31" becomes 33.

The item is done when this finds nothing:

```
git grep -n -i -E "decision(s)? (30|31)\b|\(30 and 31\)" -- crates docs/adr docs/pane-control.md CHANGELOG.md
```

Today it finds exactly these 7 lines.

## Known issues

- **S's item 1 is done for production code and docs.** Its 7 test-comment cites are waiting for T-green (above).
- **Advisory 1 (privacy in unpushed history) is still open for the operator.** T-red's `0064fac` and F's `07a576c`
  carry the operator's machine-derived Herdr session names in decisions.md. They are not in the tree, and S judged that
  they disclose nothing new. After the phase commit, a rewrite has to replay the merge.
- **Two flakes from `main` showed up in the workspace runs** (Tier 1, above). Neither is in code this change or the
  merge touches.
  - `profile_rename_faults_test`'s inode assertion (#665) has no issue yet. The main session can file one.
  - `remote_admin_test`'s `last_seen` assertion is #847.
  - CI runs the same workspace suite, so either could fail the PR's CI once. A re-run is the remedy, not a code change.

## Files changed

Production:

- `crates/holler-cli/src/profile/apply.rs` (two doc comments)

Docs:

- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Staged by the merge and not edited by F: #835's files from `main` (`git diff --cached --name-only HEAD`). None of them is
a #853 code or test file.

Run artifacts:

- `docs/handoffs/853/handoff-F.md` (this file, replacing cycle 1's)
- `docs/handoffs/853/evidence.md` (one entry appended)
- `docs/handoffs/853/decisions.md` (this cycle's entry appended)
