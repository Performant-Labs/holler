# Handoff-F: Phase 6 (the script's numbering; F) - `profile apply` creates column 1 first, and a real run with a refused step exits 3

**Date:** 2026-10-12
**Branch:** issue-853-implementation (on `0064fac`, T's RED)
**Issue:** #853

| Field | Value |
|-------|-------|
| GitHub issue | #853 |
| Working branch | `issue-853-implementation` (worktree `.claude/worktrees/0853-apply-grid-order`) |
| Build plan phase | none; the issue is the spec (epic #633), with the brief `docs/handoffs/853-brief.md` and the amendment in ADR-0021 section 8 |
| Input documents read | the brief, handoff-A.md (round 2), handoff-T-red.md, decisions.md, the amendment diffs `1f4ab40` and `05d8e6a`, T's RED diff `0064fac`, ADR-0021 sections 8 to 10, the guide's sections 3, 6 and 9 |
| Acceptance criteria count | 6 (the brief's "Acceptance, as it can be met") |
| Handoff document path | `docs/handoffs/853/handoff-F.md` (this file) |

The run is automated, so the table is recorded here instead of being confirmed by a person. The scope cap did not apply:
the brief scopes one story, A passed it, and the change is three small production edits plus their docs.

## What was done

- `crates/holler-pane/src/tx_apply.rs`: `cell_key` is now `(workspace, col != 1, row, col)`. That is creation order: by
  workspace, then the workspace's column-1 cells by row, then every other cell by row and column. Its doc names
  `plan_splits` as the rule it mirrors. The module doc, the doc of `Applied.steps` and the doc of `plan` say the new
  order. Nothing else in the engine changed.
- `crates/holler-cli/src/profile/apply.rs`: `refused(&Applied)` sits beside `stopped()`. `run` now asks
  `stopped(..).or_else(|| refused(..))`, so the exit is 1 for a failed step, else 3 for a real run with a refused step,
  else 0. The module doc and the `ProfileApply` clap doc (the `--help` text) state the new order and exit code.
- `crates/holler-pane-testkit/src/herdr.rs`: `check_split` (split-only placement) follows ADR-0021 section 10's rule,
  with two private helpers, `last_row` and `end_of`. The rule for holes is an `ASSUMPTION (#640)` comment. The docs of
  `Placement::SplitOnly` and `check_split` are updated, and the stale `GRID_UNREACHABLE` assumption is refreshed.
- `crates/holler-adapter-herdr/Cargo.toml`: a comment only. The test-kit dev-dependency's comment now also names the
  cross-check of `FakeHerdr` against `plan_splits`.
- `docs/pane-control.md`:
  - section 3's `profile apply` entry (A's warn 5) states the order and the exit 3;
  - section 6 narrows the take-over-note sentence (A's warn 4, option a).
- `docs/adr/ADR-0021.md`, section 8: the same narrowing of the take-over-note clause (A's warn 4, option a).
- `CHANGELOG.md`: a new `### Bug Fixes` under `## [Unreleased]`. It links #853 and states both the order and the exit
  code.
- `docs/handoffs/853/evidence.md` (new) and a decisions.md entry.

## Design decisions

- **The key.** `(&str, bool, u16, u16)` with `col != 1` as the second element, so `false` (column 1) sorts first. It is
  exactly the planner's partition (`plan.rs:98-100`): column-1 cells by row, then the rest by `(row, col)`. The sort is
  still the stable `sort_by`, so two specs of one cell keep P's order. I considered a key of
  `(workspace, col.min(2), row, col)`. It gives the same order, but `col != 1` names the rule.
- **The exit decision lives in one expression in `run`.** `stopped` first, then `refused`, then the data. The exit code
  comes from `emit` through `class_of` on the first refused step's code. Every refused step's code is refusal-class by
  construction (`halted` re-codes `usage` as `spec-not-launchable`, and a failure-class code makes a step `failed`), so
  the verb holds no exit table.
- **The message.** Built as `stopped()` builds its own: `quoted(&step.pane)`, the error's code, and each acted step's
  `detail` as `reconciled()` already left it (it ends with the reconcile step, once). `<M>` is the step count minus the
  refused steps. `refused()` returns `None` for a dry run, so a dry run keeps exit 0 and prints its plan.
- **The carried detail keys on `Step::acted` alone** (A's warn 3, and T's `rest == ""` assertions for rows a and e, a
  failed probe and the relaunch rules' `usage`).
- **A's warn 4: option (a), decided under the standing orders and recorded in decisions.md.** A relaunch refused before
  its act after a `--take-over` has `acted: false`, so its take-over note is not carried. The ADR's rule sentence
  already said only an acted step's detail follows; only its "so a take-over note is not lost" clause over-claimed. I
  narrowed that clause in the ADR and the guide, which now say the pane's new profile shows in `pane list` and
  `profile show`.
  - Rejected: (b), carrying every refused step's detail, which changes the rule and contradicts T's tests.
  - Parked for the operator: carrying a not-acted step's detail when it holds a note. That needs a new `Step` signal
    or a string test on `detail`, and A ruled out both.
- **The fake's rule.** In a row that holds a pane, a new cell is accepted only just right of its last pane. A new row
  is accepted only at `c1`, just below the last row, when that row is one pane at `c1`. An empty workspace accepts only
  `r1c1`. For a grid with no hole this is exactly the planner's single-cell rule; T's cross-check enumerates every such
  grid up to 3 by 3. The fake can have holes after `close` or `vanish`, and real Herdr cannot, so the hole rule is an
  `ASSUMPTION (#640)`: a row's last pane is its rightmost, and the last row is the highest-numbered that holds a pane.
  The new rule only narrows the old one, so no cell the old fake refused is now accepted. Overflow-safe comparisons
  (`checked_sub`) avoid a `u16` wrap at the edge.
- **One message for every fake refusal.** It names the cell, as T requires, and states the rule. The tests compare the
  engine's detail read back from the engine, never a literal, so the wording is free.
- **CHANGELOG.** #664's own Unreleased entry still states the old order and exit 0, and I left it as written. The file
  does not rewrite an earlier entry that a later story supersedes: #664's entry still says the verb answers
  `not-implemented` until #649, which has since landed. The new Bug Fixes entry says what changed and why.

## Reuse / extend-vs-new

This follows A's map (handoff-A.md, "Reuse & Analogous-Feature map"):

- **EXTEND `cell_key`.** It stays private, and there is no second order function and no public order API.
- **EXTEND the verb with `refused()` beside `stopped()`.** It reuses `reconciled()`'s details, `quoted` and
  `ErrorCode::from`. There is no second reconcile builder and no exit table.
- **EXTEND `FakeHerdr`'s `Placement::SplitOnly` (`check_split`).** There is no new double, and the test kit gains no
  dependency on the adapter.
- **`Step` is unchanged.** It gets no new field (A's warn 4).

## Architecture notes for A

- **Layers.** The changes touch the engine (`holler-pane`), one verb (`holler-cli`) and the test kit's fake
  (`holler-pane-testkit`). There is no new dependency, module or public item.
  - The new `last_row` and `end_of` are private methods of the fake's private `Workspace`.
  - `refused` is a private fn of the verb.
- **Public behaviour.**
  - The order of `steps` in the output changes, a dry run's included.
  - A real run with a refused step now exits 3 and prints the failure envelope with the one-line message.
  - The step shape, the wire, records, codes and `class_of` are unchanged.
  - The `HerdrPort`, the planner and the adapter are unchanged.
- **Docs.** The contract was amended first (`1f4ab40`, `05d8e6a`); F's doc edits are A's warns 4 and 5. A's warn 9 (the
  brief's overlap paragraph with #835, and who renumbers decisions 30 and 31) is O's text, and F changed nothing for
  it.

## Deviations from spec / wireframe

None in behaviour. One wording decision the brief left open: A's warn 4, option (a). It is above and in decisions.md.

## Tier 1 self-check (incl. tests now GREEN)

These ran in the worktree on a warm build, and no test file was edited.

```
cargo test -p holler-cli --test profile_verbs
test result: ok. 118 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   (RED: 105 passed, 13 failed)

cargo test -p holler-pane-testkit
fake_herdr_test: ok. 25 passed; 0 failed      (RED: split_only_mode_adds_a_row_only_below_a_last_row_of_one_pane failed)
herdr_conformance_test: ok. 22 passed; 0 failed   (both placements; split-only still passes)
every other target: ok

cargo test -p holler-adapter-herdr --test plan_splits_test
test result: ok. 20 passed; 0 failed   (RED: the cross-check failed at r2c1 on the grid [2])

cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load   (the pipeline's verify command)
167 test binaries: passed=2594 failed=0 ignored=37; exit=0

cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test --test cli_surface_test   (re-run on the final docs)
ok. 18 passed / ok. 3 passed / ok. 3 passed

cargo clippy -p holler-pane -p holler-pane-testkit -p holler-cli -p holler-adapter-herdr --all-targets -- -D warnings
Finished, no warnings

bash scripts/lint.sh          -> exit 0 (herdr.rs 598 lines, tx_apply.rs 565, profile/apply.rs 225; none newly warned)
bash scripts/changelog-check.sh -> changelog-check: ok
rustfmt --edition 2021 --check on the three changed .rs files -> clean (they were clean at HEAD)
./target/debug/holler profile apply --help -> renders the new order and exit-code text
```

Not run, because they are T-green's (Tier 2):

- the opted-in scratch tests (`grid.rs`, `probe.rs`, `scenario.rs`; Herdr 0.9.1 must be first on `PATH`; on this
  machine it is in `~/.local/bin`, and the default `herdr` is 0.8.2);
- A's warn 1 check, which reverts `cell_key` on the tightened fake.

## Evidence appendix

`docs/handoffs/853/evidence.md`. It covers:

- the planner's partition and its refusal below a wider row;
- `halted`'s refused-step invariants;
- `class_of` for open codes;
- `emit`'s exit code and its text form;
- the failure envelope's absent `data`;
- `reconciled()`;
- the launch act's `acted = entered` with `ensure_pane` first;
- the fake's check order;
- the conformance suite's cell use.

## Tests that look wrong (for T)

None. Every test T authored passes as written.

## Known issues

- **Public-repository rule, in T's text.** T's decisions.md entry ("T, author tests (RED)", the "Incident, no harm"
  bullet) names the operator's live Herdr sessions by their machine-derived names. The stack rules forbid machine or
  account names anywhere in this public repo. I did not edit another phase's append-only entry. T (at T-green) or O
  should replace the two names with a neutral placeholder before the PR; the PR is squash-merged, so the final file is
  what reaches `main`.
- **Parked for the operator** (the warn 4 follow-up): a relaunch refused before its act after a `--take-over` does not
  say in the one line that the pane moved. The ADR and the guide now say so, and `pane list` and `profile show` show
  the move. Carrying it would need a `Step` signal, which A ruled out.

## Files changed

Production:

- `crates/holler-pane/src/tx_apply.rs`
- `crates/holler-cli/src/profile/apply.rs`
- `crates/holler-pane-testkit/src/herdr.rs`
- `crates/holler-adapter-herdr/Cargo.toml` (comment only)

Docs:

- `docs/adr/ADR-0021.md`
- `docs/pane-control.md`
- `CHANGELOG.md`

Run artifacts:

- `docs/handoffs/853/handoff-F.md`
- `docs/handoffs/853/evidence.md`
- `docs/handoffs/853/decisions.md` (appended)
