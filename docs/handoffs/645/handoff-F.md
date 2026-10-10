# Handoff-F: Phase 5 (Workflow script Phase 6), rework round 3 - #645a `pane switch` and `pane reset` (the outside diff gate's BLOCK)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`). This round starts at `4443dd4`,
T's round-2 GREEN. The outside diff gate then ran its round 2 (`deepseek-v4-pro`, `docs/handoffs/645-diff-result-r2.md`,
gitignored, written 23:37 MDT) and returned BLOCK on one finding, B-1. This round's work is uncommitted, and the
Workflow script commits it after this phase. Round 2's handoff is in git at `37f2101`, and round 1's is at `fa59fe7`. Their
content that still holds is kept below.
**Issue:** #645 (part 1 of 2, 645a)
**Rework of:** `docs/handoffs/645-diff-result-r2.md`, B-1 (block). It also takes NIT-1 (doc only) and adds the evidence
NV-1 asked for. The gate's other entries say "No finding" or are not blocks (see Design decisions, round 3).

## What was done (round 3)

- `crates/holler-pane/src/tx_switch.rs`, `check_health` (`:229-250`): **B-1.** The remedy fallback is now an explicit
  `match`, not `Option::unwrap_or_else`:
  ```rust
  let remedy = match FindingKind::ServerDown.remedy(pane, FixState::NotFixable) {
      Some(remedy) => remedy,
      None => doctor_command(pane, false),
  };
  ```
  The behaviour is the same. For a named pane the remedy table answers `Some("holler pane relaunch <pane>")`
  (`findings.rs:127-131`, already in `evidence.md`), so the `None` arm is never taken, and neither form could panic. The
  doc comment now says that the table has a remedy for every named pane, and what stands in if it ever had none.
- `crates/holler-pane/src/tx_switch.rs`, the `SESSION_ID_MAX` doc (`:58-60`): **NIT-1.** It now says why the two limits
  agree: `quoted` counts characters (`error.rs:690`, `text.chars().count()`), `parse_session_id` counts bytes, and an
  accepted id is ASCII.
- `docs/handoffs/645/evidence.md`: two entries under "Added by F (round 3, after the outside diff gate's BLOCK)". They are
  the real `StoreScope::resolve` and `member` (#663), `PaneStore::get`'s contract, and the fake's `resolve` and `member`.
  With them NV-1 can be settled: `resolve(P, Some(n))` answers exactly the pane `n`, or `pane-not-in-profile`.
- `docs/handoffs/645/decisions.md`: the F round-3 entry. It also journals both outside diff-gate rounds, whose result files
  are gitignored: r1 PASS (23:03 MDT) and r2 BLOCK (23:37 MDT).

## Design decisions (round 3)

1. **Change the code, don't argue the finding.** B-1's reasoning is partly wrong. `unwrap_or_else` cannot panic: it runs
   the closure on `None`. Clippy's `unwrap_used` does not cover it, and r1 reviewed the same line without a block (its
   NV-3). But the brief's P3 says "with no `unwrap`/`expect`", and a literal reading of that covers the call's name. A
   `match` satisfies both readings, changes no behaviour, and settles the finding for good. Leaving the code alone would
   only send the same line back to the gate.
2. **`match`, not another combinator.** The gate's remedy names "an explicit `match`/`if let`". Another combinator, such
   as `map_or_else(|| .., |remedy| remedy)`, would leave the same question open. Clippy's `manual_unwrap_or` did not fire
   on the `match` (its `None` arm is a function call), and the workspace clippy run with `-D warnings` is clean.
3. **NIT-1 taken, NIT-2 left for T.** NIT-1 is one doc sentence in this story's own file, and the gate asked for it. NIT-2 is
   about the module doc of `tests/pane_verbs/reset.rs`, a test file, so it is T's (see "Tests that look wrong").
4. **NV-1 answered with evidence, not code.** P1's `find` by name (round 1's deviation 1) stays. The gate could not see the
   real resolver. It is on `main` since #663, and both it and the fake answer one pane for a named pane, so `find` and
   `next()` agree. `find` stays because it adds a check and costs nothing: a scope that broke its contract would give
   `pane-not-found`, not a record of another pane.
5. **No other change.** NV-2, NV-3 and W-1 each end "No finding" or "Not a BLOCK", and their facts are in the diff or in
   `evidence.md`. There is no test change, ADR change or CHANGELOG change.

## What was done (round 2, unchanged)

- **Merged `origin/main` (`d9eabbb`: #641, #643, #646 part 1, #662 part 2, #663, #704)**, as A-dup's note 1 asked. The merge
  was clean, and the Workflow script's phase commit recorded it with both parents (`37f2101`).
- `docs/adr/ADR-0021.md`: four sentences added to the "Switch and reset as built (#645)" paragraph (section 8, lines
  345-370). They say when the reconcile step is printed (every failure once `select_session` has been called, none
  before), that it names the pane even with `--profile`, and why it differs from the two forms of #663's step 6.
- `crates/holler-pane/src/tx_switch.rs`: two doc comments only. "once the act has begun" became "once `select_session` has
  been called".
- `docs/handoffs/645/evidence.md`: four entries under "Added by F (round 2)".

## Design decisions (rounds 1 and 2, unchanged)

Round 2:
1. **The pane-scoped `--fix` step stays, and the ADR states the exception** (A-dup's recommendation). Step 6's reason, that
   a pane-scoped doctor refuses a pane with no record, does not apply. The plan read the record, the step carries no
   `--profile` (`reconcile.rs:226-236`), and `--fix` is doctor's own remedy for the mismatch (`findings.rs:135-141`).
2. **The fix is in the #645 paragraph only**, inside AC 24's "end of section 8". The generations bullet and step 6 are
   #663's text.
3. **The boundary is `select_session`, not "the act".** A failed `create_session` goes through `From<PaneError>`, so
   `acted` is false and the message has no step.

Round 1:
1. **One after-clap path for both verbs** (`switch::execute`), so `reset.rs` is its `Args` struct and a one-call `run`.
2. **The order of usage errors.** `switch::run` types `SESSION` first (pure), and `execute` reports it after `PANE` and
   `--profile`, so the brief's step-1 order holds.
3. **P1 takes the resolved pane by name** (`find`), not blindly the first (Deviations, round 1).
4. **The error's text goes into the message unchanged.** Every value the engine itself writes into a message (the target,
   SHOWN, the created id) goes through `quoted`.
5. **The `data` struct** is `{verb, pane, previous}`, with `previous` `null` when there was none. `Verb` serializes lowercase.
6. **Comments cite ADR-0021 sections and issues, never the brief's labels** (`docs/handoffs/` is deleted before the push).
7. **The CHANGELOG entry sits after #647's doctor entry.**

## Reuse / extend-vs-new

Unchanged. A-dup's table confirms that every Reuse map row is extended or reused, and round 3 changes how one reused call
is written, not what is reused. The objects extended are the `tx_switch.rs` stub and the two verb stubs, on doctor's
`--fix` repair (select, observe, record). These are reused:

- `findings::doctor_command(Some(pane), true)`, `findings::quoted` and `reconcile::shown_differs`;
- `FindingKind::ServerDown.remedy(..)`, with a `doctor_command` fallback, now by `match` (the Reuse map's "Remedy" row);
- `ports.scope.resolve`;
- `output::{emit, emit_error, ErrorBody, ErrorCode, VerbCtx}` and `ProfileOpt`;
- `holler_proto::clock::now_millis`.

The one justified copy is `screen_text` (O1).

## Architecture notes for A

- **Round 3:** none. One private function's body is rewritten with the same behaviour, and two doc comments change. No
  module, public interface, dependency or behaviour changed. `git diff origin/main -- '*Cargo.toml'` is empty.
- **Round 2:** ADR prose, two doc comments and the merge of `origin/main`.
- **Round 1, as built:** the engine is in `holler-pane`, pure over `Ports`, with no I/O, no async and no new dependency.
  `tx_switch` uses `findings` and `reconcile::shown_differs`. `pane/switch.rs` uses `holler_pane::tx_switch`, and
  `pane/reset.rs` uses `pane/switch.rs` (`execute`, `Verb`). No frozen file and no #647 file is touched.

## Deviations from spec / wireframe

Round 3: none. The P3 row now holds in its literal reading too ("with no `unwrap`/`expect`").

Rounds 1 and 2, still standing: P1 uses `find` by name, where the brief says `into_iter().next()`; for every scope the
two agree (round-3 evidence). The "Deferred" bullet takes A's round-2 form, "#645 for switch and reset; #644 to follow",
which is still right because #644's paragraph is not on `origin/main`. `reset.rs` reuses `switch::{execute, Verb}`. Two
of round 1's doc comments were corrected in round 2.

No wireframe applies (no UI surface).

## Tier 1 self-check (incl. tests now GREEN), round 3

Every build ran with `CARGO_BUILD_JOBS=4`. `origin/main` is still `d9eabbb` (fetched at 23:45 MDT), an ancestor of HEAD.

```
$ cargo clippy --workspace --all-targets -- -D warnings      -> exit 0, 0 warnings (holler-pane re-checked)
$ cargo build -p holler-pane -p holler-cli                    -> Finished (exit 0)
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 154 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 133 filtered out
   (13 switch cases, 7 reset cases, help_names_the_arguments; switch_refuses_an_unhealthy_server
    asserts "run holler pane relaunch demo-c1r1", the match's Some arm)
$ cargo test -p holler-cli --test pane_cli_process --test cli_surface_test --test docs_cli_test
   cli_surface_test 3 passed; docs_cli_test 3 passed; pane_cli_process 34 passed; 0 failed
$ cargo test -p holler-pane                                   -> every target ok, 0 failed
$ rustfmt --check --edition 2021 tx_switch.rs pane/switch.rs pane/reset.rs \
    tests/pane_verbs/switch.rs tests/pane_verbs/reset.rs       -> exit 0
$ bash scripts/lint.sh                                        -> exit 0 (pre-existing size warnings only, none for a story file)
$ bash scripts/changelog-check.sh                             -> changelog-check: ok
$ git diff origin/main -- '*Cargo.toml' | wc -l               -> 0
$ wc -l crates/holler-pane/src/tx_switch.rs                   -> 324
$ git add crates/holler-pane/src/tx_switch.rs                 (staged by path; the docs are left to the script's commit)
$ gitleaks protect --redact --no-banner                       -> no leaks found (exit 0), over every uncommitted change
```

**`cargo test --workspace`** in CI's form (`-- --skip roster_stays_accurate_under_concurrent_body_load`), with
`--no-fail-fast`, on the final tree: **exit 0**. There are 135 result lines: 1642 passed, 0 failed and 14 ignored. That is
round 2's 1641 plus T's new `reset_create_failure_changes_nothing`. It includes `pane_verbs` (154) and `wire_selftest` (3).

## Evidence appendix

`docs/handoffs/645/evidence.md` has 24 facts in unchanged code or text: 15 from F round 1, 2 from T, 4 from F round 2, 1
from T round 2 and 2 from F round 3. The round-3 entries are:

- `crates/holler-cli/src/pane/profile_scope.rs:159-173`, `:92-100` and `crates/holler-pane/src/ports.rs:63-64`: the real
  `StoreScope::resolve(P, Some(n))` answers exactly the record stored under `n`, when it names P, else
  `pane-not-in-profile`;
- `crates/holler-pane-testkit/src/profile_scope.rs:190-204`, `:117-126`: the fake answers the same way.

Their excerpts were checked against the source with `diff`, and they match. None of the three files differs from
`origin/main`. The fact B-1's fix relies on, that `ServerDown` has a remedy for every named pane, was already there
(`findings.rs:127-131`, `:319-320`).

## Tests that look wrong (for T)

None is wrong, and no test changed in any round. One cosmetic note: the gate's NIT-2 says the module doc of
`crates/holler-cli/tests/pane_verbs/reset.rs:4-6` reads "both formats, one envelope", which is imprecise, because each
format is a separate run. That is T's call, and nothing needs to change.

## Known issues

- **The gate's needs-verification and warn entries need no change.** NV-1 (`find` against `next()`) now has its evidence.
  NV-2, NV-3 and W-1 each end "No finding" or "Not a BLOCK".
- **A-dup's warns 2-4 are for O to file**, and none needs a change in 645a:
  - the lead-in `to reconcile, run ` and `screen_text`, each spelled twice. `reconcile_step` is on `main`, in
    `holler-cli/src/pane/profile_scope.rs:37-56`;
  - the third private copy of the scoped-read arms;
  - the fourth both-format test runner (F-2 has triggered).

  Follow-ups F-1 and F-3 still need filing too. Once O files the `screen_text` fold, the comment in `tx_switch.rs` should
  name that issue. Today it says "a follow-up of #645".
- **An observation, with the behaviour unchanged.** When reset's `create_session` times out, the server may still hold a
  new session that the message cannot name, and the message has no step. Doctor reports it as a `stray-session`. That is a
  645b question.
- **The accepted risks are unchanged:** the `session-of-other-pane` race (R-5, F-3), the window before 645b (R-3), and
  the `stray-session` that every successful reset leaves (F-1).

## Files changed

Production (this story, all rounds):
- `crates/holler-pane/src/tx_switch.rs` (round 3: `check_health`'s fallback as a `match`, and two doc comments)
- `crates/holler-cli/src/pane/switch.rs`
- `crates/holler-cli/src/pane/reset.rs`

Docs (this story):
- `docs/adr/ADR-0021.md` (round 2: the #645 paragraph; unchanged in round 3)
- `CHANGELOG.md` (round 1; unchanged since)

Pipeline records:
- `docs/handoffs/645/handoff-F.md` (this file)
- `docs/handoffs/645/evidence.md` (round-3 entries appended)
- `docs/handoffs/645/decisions.md` (the F round-3 entry appended)
