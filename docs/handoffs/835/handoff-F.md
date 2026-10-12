# Handoff-F: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 3: S's REWORK 1, the foreground wording, and the merge with `main`)

**Date:** 2026-10-11 (20:32 MDT)
**Branch:** `issue-835-implementation` at `ccc53a0` (S's REWORK commit). On top of it:
- `origin/main` `d6ae539` (#851) is merged and **uncommitted**: `MERGE_HEAD` is set and #851's files are staged;
- F's rework is unstaged.

The phase commit (`git add -A`, then `git commit`) concludes both as one two-parent commit, as T-red's `526bbd3` was.

**Issue:** #835

This handoff replaces cycle 2's, which is at `4604693` (cycle 1's is at `9bd101b`). This cycle is text only: S's REWORK 1
(`handoff-S.md`, "Verdict") and the merge with the `main` that moved after S's audit.

| Field | Value |
|---|---|
| GitHub issue | #835 |
| Working branch | `issue-835-implementation` |
| Phase | F, cycle 3: the rework after S's REWORK (A pass 4 PASS, T-green cycle 2 PASS, A-dup PASS) |
| Input documents read | `handoff-S.md`; the brief (issue, amendment, rounds 2 and 3); handoff-F cycle 2; `decisions.md`; the six places S names; ADR-0021 section 13; `ports.rs` and `pane.rs` (the rule's docs); the adapter's `tmux.rs` and `lib.rs`; `docs_pane_control_test.rs`; #851's diff (`git diff HEAD...origin/main`) |
| What this cycle must change | S's REWORK 1: six places that say the import reads "each process" of the tmux session, reworded to the foreground process of each tmux pane, with a `claude` outside the foreground among the limits. No behaviour change, no test change, and the guide no longer than 817 lines from this story |
| Handoff document path | `docs/handoffs/835/handoff-F.md` |

## What was done

**S's REWORK 1, in place.** Every place now says the import reads the command name of the **foreground process of each
tmux pane** in the pane's tmux session. Every limits sentence names a `claude` outside the foreground.

| Where | Change |
|---|---|
| `crates/holler-cli/src/pane/import.rs`: the `PaneImport` doc, which is `pane import --help` | The read, the rule ("a pane whose tmux session has a foreground process named exactly `claude`") and the limits sentence (design decision 1). |
| `docs/pane-control.md`, section 1 | "whose tmux session has a foreground process named `claude`". "Claude Code" stays. The paragraph is still 4 lines. |
| `docs/pane-control.md`, section 3's `harness-claude-code` row | "Its tmux session has a foreground process named `claude` (Claude Code)". |
| `docs/pane-control.md`, section 10's fact | The read, the rule and the same limits sentence. The bullet is still 12 lines (design decision 3). |
| `CHANGELOG.md`, the #835 entry | The read, the rule and the same limits sentence, then "Read the dry run first." |
| `crates/holler-pane/src/import/observe.rs`: `read_tmux`'s doc | "the command name of the foreground process of each of its tmux panes". |
| `crates/holler-pane/src/import.rs`: the module doc (beyond S's table) | "reads the command names of the pane's tmux session" becomes the same foreground wording (design decision 5). |

**The merge with `main`.** `origin/main` moved to `d6ae539` (#851, PR #854, merged 20:07 MDT). That was after S's
`git fetch` at 20:03 MDT. F merged it with `git merge --no-ff --no-commit origin/main`:
- it auto-merged `docs/adr/ADR-0003.md` and `docs/pane-control.md`, with no conflict;
- it is left in progress for the phase commit, since F does not commit (design decision 6).

## Design decisions

1. **One limits sentence, the same in the help, the guide's section 10 and the CHANGELOG.** It reads: "tmux names only
   the foreground process, so a Claude Code pane is taken for OpenCode when its `claude` has exited (it shows its shell),
   is not in the foreground (a wrapper script runs it, or it is suspended) or was started under another name."
   - The subject comes first, so the three cases read as one list.
   - **"Is not in the foreground"** is S's missing case. A wrapper script that stays in the foreground runs `claude` as its
     child, so tmux reports the script's interpreter. A suspended `claude` leaves its shell in the foreground.
   - **"Was started under another name"** is the case the old text called "a wrapper". ADR-0021 and `pane.rs` give
     `node` running Claude Code's script as an example.
   - **"By the name it was started under" is dropped** from the old sentence. The third case says it, and the guide's
     bullet had to stay at 12 lines.
2. **"tmux pane" and "pane".** The read is defined as "the command name of the foreground process of each tmux pane in the
   pane's tmux session". The words "tmux pane" separate tmux's panes of the session from the Holler pane, which is a pane
   directory with a record. The short form is "whose tmux session has a foreground process named `claude`". It is used
   in section 1, in section 3's row and in the rule sentences, as S proposed for section 1.
3. **The guide's line budget.** S asked that the rework add no line to the guide (817). The new read and limits are
   longer, so section 10's bullet keeps its 12 lines by two cuts:
   - "It is a default for this import, not a proof of what a pane runs; step 1 says how to check it" becomes "It is a
     default, not a proof; step 1 checks it". The help already says "The rule is a default, not a proof".
   - The dropped clause of design decision 1.
   I measured each wording with a wrapper at the guide's 120 columns. Every other sentence of the bullet is unchanged.
4. **ADR-0021, `ports.rs` and `pane.rs` are unchanged.** S found that they say this exactly (`handoff-S.md`, "Spec
   compliance", and REWORK 1). They state the foreground model. Their limits put "a wrapper" under another `argv[0]`,
   which is true of a wrapper that `exec`s Claude Code under another name.
5. **One place beyond S's table: the `holler-pane` import module doc.** It said "the import reads the command names of the
   pane's tmux session". That is vaguer than "each process" but not wrong, and S did not list it. It now uses the same
   foreground wording, for the same reason S gives for `observe.rs` ("for consistency"). It stays one paragraph of 9
   lines.
6. **The merge with `main`.**
   - **Why:**
     - S's audit rests on "the branch tree is the PR's merge result" (`handoff-S.md`, header). That stopped being true
       two minutes after S's fetch.
     - Brief R2-1 says this story "also needs the current `main`".
     - A's pass 4 (W1) asked for the same merge after #649 and #832, and T-red made it.
     - Merging now lets T-green and S verify the tree the PR will merge. Otherwise S could find the branch behind `main`
       and send it round the loop again.
   - **Clean:**
     - `git merge-tree --write-tree` reported no conflict before the merge.
     - #851 does not overlap #835. It changes the version string (`build.rs`, `build_info`), adds a CHANGELOG entry and
       ADR-0003's header and version line, and adds four guide lines before section 10's fact.
   - **How, without a commit or the shared stash.** The rework was saved as a patch and as file copies in the
     scratchpad. The five files were restored and `main` merged with `--no-commit`. Then the patch was applied again.
     - The worktree diff on the merged tree is, line for line, the saved patch.
     - The three files #851 does not touch are byte-identical to their copies.
     - The other two (`CHANGELOG.md` and the guide) differ from their copies only by #851's lines.

## Reuse / extend-vs-new

All edits are in place, in the sentences S names (plus design decision 5's one sentence). There is no new section, page,
table, helper or test, and no edit to a file that is not already in this story's diff.

## Architecture notes for A

None. Prose and doc comments only: no code, interface, dependency, protocol, error code or state change. The merge brings
in #851's code (the commit in the version string). That is #851's, it is reviewed and merged on `main`, and it overlaps
nothing of #835. `archChanged: false`, as S says.

## Deviations from spec / wireframe

None. S left the wording to F ("as long as every place says 'foreground' and names a `claude` outside the foreground among
the limits"). Two places differ from S's suggested words, and both meet that rule:
- section 3's row reads "Its tmux session has a foreground process named `claude`", parallel to section 1, where S
  suggested "The foreground process of a pane of its tmux session is named `claude`";
- the limits say "is not in the foreground (a wrapper script runs it, or it is suspended)", where S suggested "runs
  behind another foreground process (a wrapper script, a suspended job)".

## Tier 1 self-check (incl. tests now GREEN)

There is no RED for this cycle: S asked for no test, and no test pins this prose. On the **merged tree** (branch, #851 and
the rework):

```
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
  exit 0. 155 result groups: 2406 passed, 0 failed, 32 ignored
  (cycle 2's T-green: 154 groups, 2398 passed; the 8 more and the 155th group are #851's build_version_test and others)

$ cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test
  docs_pane_control_test: 17 passed, 0 failed     docs_cli_test: 3 passed, 0 failed

$ bash scripts/changelog-check.sh                                  -> changelog-check: ok
$ cargo clippy --workspace --all-targets -- -D warnings            -> exit 0
$ bash scripts/lint.sh                                             -> exit 0
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane     -> exit 0 (the module doc's intra-doc link resolves)
$ git diff --check; git diff --cached --check;
  git diff --check origin/main -- . ':(exclude)docs/handoffs'      -> clean, clean, clean
$ wc -l docs/pane-control.md                                       -> 821 (817 from this branch, plus #851's 4)
```

Before the merge, on the branch with the rework alone, the same checks passed except for two workspace tests:
- `interrupt_test`'s `cancel_not_queued_behind_large_update_flush` failed with "alpha's body is not connected", and
  `full_journey_with_stub` with "t/alpha is reconnecting";
- the machine's load average was 23 at the time;
- both are hub and body end-to-end races in code this change cannot reach, and the binary passed 11 of 11 alone;
- the run on the merged tree, above, had no failure.

`holler-cli`'s rustdoc with `-D warnings` fails on `main` as it stands (see Known issues). None of its errors is in
`pane/import.rs` (0 diagnostics there).

## Evidence appendix

None: no behaviour outside the diff is relied on. The wording rests on two things this story's diff already shows:
- the adapter's `FOREGROUND_FORMAT`, `#{pane_dead} #{pane_current_command}` (`crates/holler-adapter-host/src/tmux.rs:35`),
  read once per live tmux pane;
- `HostPort::foreground_commands`'s docs (`crates/holler-pane/src/ports.rs`).

## Tests that look wrong (for T)

None.

## Known issues

1. **The guide is 821 lines after the merge:** 817 from this branch, plus the 4 lines #851 added before section 10's fact.
   This rework adds none, as S asked. S's advisory 1 (#652's soft "about 800", enforced by no check) still applies.
2. **`holler-cli` rustdoc with `-D warnings` fails on `main` already.** The errors are unclosed `<label>`/`<session>` tags
   in `cli.rs`, and private intra-doc links in `pane/launch.rs`, `park.rs`, `reset.rs` and `switch.rs`. None of these
   files is in this story's diff, and CI has no doc step. The gate T-green used is `-p holler-pane`, which is clean. This
   is for the operator, if a doc gate is wanted.
3. **For the run's agent, after the PR opens** (adds to S's advisory 5): name #851 (PR #854) among the coexistence notes.
   It edited the guide's section 10, ADR-0003 and `CHANGELOG.md`, and merged cleanly.

## Files changed

This cycle's production files (5), all already in this story's diff:

- `CHANGELOG.md`
- `crates/holler-cli/src/pane/import.rs`
- `crates/holler-pane/src/import.rs`
- `crates/holler-pane/src/import/observe.rs`
- `docs/pane-control.md`

The in-progress merge also stages #851's files from `main` (`git diff --cached`). They are not this story's, and
`git diff origin/main...HEAD` will not show them once the phase commit concludes the merge. The PR's production files
across all cycles are cycle 2's list, unchanged.

Run artifacts: `docs/handoffs/835/handoff-F.md` (this file) and `docs/handoffs/835/decisions.md` (appended).
