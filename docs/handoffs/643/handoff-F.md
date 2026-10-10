# Handoff-F: Phase 5 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (round 3: stopped for a ruling)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on d6d22d7, A-dup's BLOCK; no production file has changed since 06320ad)
**Issue:** #643 (epic #633). Brief: `docs/handoffs/643-brief.md` (unchanged). Input: `handoff-A-dup.md` (BLOCK on D-1,
warns D-2 to D-6).

Round 2 of this handoff is in git: `git show f922dd8:docs/handoffs/643/handoff-F.md`. This file replaces it. This
cycle is the Workflow script's phase 6 (F), which the driver ran after A-dup's BLOCK.

## Outcome

`done: false`, `archChanged: false`, and no production change. F cannot clear D-1 without breaking the brief, so this
cycle stops the run so the MO can rule and amend the brief. That is what A-dup's Notes for F 1 and 2 ask for.

**The stop will read `gate-unavailable:6`.** That is the driver's name for F returning `done: false`. The driver turns
`done: false` into the verdict `unavailable` (`coding-pipeline.workflow.mjs:4798`), and `nextPhase` stops every
`unavailable` as `gate-unavailable:<phase>` (`:1107-1108`). It is not a gate failure or an infrastructure failure. The
driver commits this handoff first, as `f -- implementation incomplete` (`:4790-4796`). A-dup's note 2 calls this stop
`unrecognized-verdict:unavailable`, but the driver's code says `gate-unavailable:6`.

## What was done

- **Checked D-1 against the source,** on `origin/main` (`e612878`) and on this branch. All of A-dup's facts hold:
  - `holler_pane::reconcile::shown_differs` is `pub`. Its doc calls it "the one form of this comparison" and names
    `pane get` and the roster as readers (`reconcile.rs:171-181`).
  - ADR-0021 §11 on `main` (:454-455) says reconcile compares SHOWN with `session_of_record` and leaves
    `last_observed.driven` as stored.
  - Reconcile's `record` writes only `harness.health`, `last_observed.shown` and `at` (`observe.rs:350-386`).
  - Outside test code, `git grep` on `origin/main` finds no write of `driven`. The only hit is the test kit's fixture,
    which sets `driven: None`.
  - `SessionSync::of` compares `shown` with `driven` (`list.rs:211-221`). It is used in `PaneRow::from`
    (`list.rs:170`) and in `get`'s `detail` (`get.rs:86`). `watch` gets its SYNC through `PaneRow`.
  - `git merge-tree --write-tree origin/main HEAD` conflicts in `CHANGELOG.md` only.
  - `gh issue view 643` has no comments, and #633's latest comments have no ruling on SHOWN/DRIVEN.
- **Ran Tier 1 on the unchanged tree** (results below).
- Wrote this handoff and the `decisions.md` entry. `evidence.md` is unchanged: there is no diff for it to support,
  and no gate runs after a `done: false`.

### Why F stops instead of folding D-1

1. **Every fold changes SYNC, and AC 3 pins SYNC.** Three tests pin AC 3's three cases:
   `list_flags_a_pane_whose_shown_and_driven_differ`, `get_flags_a_mismatch` and `watch_flags_a_mismatch`. Their
   fixtures (`tests/pane_verbs/list.rs:113-122`, `observed`) set `driven` with `at: 0` and no session of record.
   Under (a) all three cases read `unobserved`, so the three tests fail. F does not edit tests, and T-green and S both
   check against AC 3 as written. A fold made now would loop through F, a T-green BLOCK and F again until the cap.
2. **(a) needs `shown_differs` on this branch.** That means merging `origin/main`, which is a commit, and F makes
   none. A half-done merge would also be swept into the driver's `git add -A` phase commit.
3. **(b) edits `holler-pane` and ADR-0021,** which are outside this story's blast radius (brief, "Out of scope").
4. **`done: true` with no change** would re-run T-green, the paid outside diff gate and A-dup on an unchanged tree.
   Each round would end in the same BLOCK, until the cap.

## For the MO: F's input to the ruling

**F recommends (a): adopt `shown_differs`.** A-dup's reasons hold: it matches `main` and ADR-0021, it keeps one rule,
and it stays inside this story's files. Two more findings from the source bear on the ruling.

- **Decision 3's reason for not flagging `None` no longer matches the writer.** Decision 3 reads `shown: None` as
  "reconcile could not tell". The writer on `main` means something else by it. `shown: None` with `at > 0` is "an
  observed home screen, or a pane with no TUI" (`reconcile.rs:31-32`, `observe.rs:79-83`). When reconcile cannot tell,
  it leaves `shown` as stored (`observe.rs:369-371`). So under #643's rule, a pane whose TUI has fallen back to its home
  screen is never flagged, even when it has a session of record. That is the mismatch the epic wants to be loud. (b)
  would have to fix this reading as well.
- **A caveat that comes with (a).** It is #647's to fix, not this story's. `at > 0` does not prove the screen was seen:
  - On a pane's first observation, `harness.shown_session` can fail while the health check answers. `record` then
    writes the health, stamps `at`, and leaves `shown` as stored, which is `None` on a record nothing has observed yet
    (`observe.rs:361-375`, `reconcile.rs:153-161`).
  - A reader that follows `shown_differs`'s doc ("provided `last_observed.at > 0`") then treats that unseen screen as
    the home screen and prints a false MISMATCH. Doctor's own pass skips the rule when the screen is unseen
    (`observe.rs:232`).
  - The case is rare, and it fails loud, not silent.
  - Suggested follow-up for #647: stamp `at` only when the screen was seen, or record that it was not.
- **The amendment must also say what DRIVEN prints until #649.**
  - **(a1)** DRIVEN keeps printing `last_observed.driven` as stored. That is `-` on every real pane until #649, and it
    keeps the issue's wording, "SHOWN and DRIVEN come from the record's `last_observed`". A flagged row shows SHOWN
    beside a `-`. Only `pane get` shows the session of record.
  - **(a2)** DRIVEN prints `session_of_record`, which is the session the hub drives by I2 and ADR-0021 §11. A flagged
    row shows both sessions. The cost: a public JSON field takes a value that #649 may later fill from somewhere else,
    and the issue text needs an amendment.
  - **F leans to (a1).** Every verb answers `not implemented` over `Unwired` until #649 (brief C7), so no user sees
    either choice before #649 takes over DRIVEN. (a1) commits #649 to nothing.
- **Under (a), the fresh run's F change is small:**
  - Merge `origin/main` into the branch first. Only `CHANGELOG.md` conflicts: both stories add an entry at the same
    place.
  - `SessionSync::of` takes the pane. It is `Unobserved` when `last_observed.at == 0` or there is no session of
    record. Otherwise it is `Mismatch` when `shown_differs(session_of_record, last_observed.shown)` and `Ok` when not.
    There are two call sites (`list.rs:170`, `get.rs:86`), and its three values and serde names do not change.
  - Docs to change:
    - `list.rs`: the module doc (:6, :13), `PaneList`'s help (:36-48), `PaneRow`'s field docs (:151-154) and
      `SessionSync`'s docs (:195-221);
    - `get.rs`: :1-3 and :24-33;
    - the CHANGELOG sentences "SYNC reads `MISMATCH` when SHOWN and DRIVEN differ" and "SHOWN, DRIVEN and health
      are what reconcile last recorded".
  - T re-authors AC 3 first, test first. The fixtures need `session_of_record` and `at > 0`, and the `shown: None`
    case becomes MISMATCH.
- **D-2 (`text_value` and `findings::quoted`), in the same ruling:** F agrees with A-dup that `quoted` cannot be dropped
  in. It always quotes and cuts at 64 characters, while AC 1 pins bare plain cells and AC 7 pins an uncut cwd. Any single
  rule would be a `holler-pane` change, made amend-first, and it would change no output of #643's. It can be a
  follow-up and does not need to block this story.
- **Whatever the ruling, the branch must take in `origin/main` before the PR merges.** W-4 (`PaneChange.pane`'s name)
  and W-9 (the brief no longer describes the code on four points; `evidence.md` carries them) can go into the same
  amendment.

## Design decisions

- **No production change.** The reasons are under "Why F stops instead of folding D-1".
- **`done: false`, not `done: true`.** The role defines `done: false` as stopping short this cycle to ask a blocking
  question. The question here is D-1's ruling. `done: true` would only repeat the gates (reason 4).
- **`archChanged: false`.** No module boundary, public interface or dependency changed this cycle. The driver stops on
  `done: false` either way.
- **No merge of `origin/main` this cycle.** It is needed whatever the ruling, but it is a commit, and the MO's fresh
  run is where it belongs (reason 2).

## Reuse / extend-vs-new

No code was written this cycle. A-dup checked round 1's reuse map row by row and found no copy of any named object.
D-1 is a rule that `main` gained during this run (`e612878`, merged at 18:56:22 MDT), after the brief, so it is not
drift from the Reuse map. Folding it is the ruling's job (above).

## Architecture notes for A

None this cycle. Under (a), the fresh run would change one public signature: `SessionSync::of` would take the pane
instead of `&LastObserved`. It would also add a dependency from `holler-cli` on `holler_pane::reconcile`, a `pub`
module that `pane doctor` already imports.

## Deviations from spec / wireframe

None. The code still matches Decision 3 and AC 3 exactly. The question is whether those two still hold now that
`main` has moved. There is no UI surface, so there is no wireframe.

## Tier 1 self-check (incl. tests now GREEN)

```
$ git diff --stat 279a1fb..HEAD -- crates CHANGELOG.md                 -> empty (no change since T-green's PASS)
$ git diff --stat 06320ad..HEAD -- crates/holler-cli/src CHANGELOG.md  -> empty (production unchanged since round 1)
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.04s
$ cargo clippy -p holler-cli --all-targets -- -D warnings              -> exit 0
$ bash scripts/lint.sh                                                 -> exit 0
$ git merge-tree --write-tree --name-only origin/main HEAD             -> CONFLICT (content) in CHANGELOG.md only
```

T-green's round-2 workspace run (1414 passed, 0 failed) ran on this same production code, so it was not repeated.

## Evidence appendix

None this cycle. No new diff relies on unchanged code, and no gate runs after `done: false`. `evidence.md` is
unchanged at 11,696 bytes. The `file:line` facts the MO needs for the ruling are cited above, from `origin/main` at
`e612878`.

## Tests that look wrong (for T)

None as the brief stands: AC 3's three tests match AC 3 exactly. If the MO rules (a), they encode the replaced rule,
and T re-authors them first in the fresh run (see "For the MO").

## Known issues

- **D-1 is open** until the MO rules and the brief is amended (above).
- **The stop's name hides the reason.** The driver has no stop of its own for "F needs a ruling", so a deliberate stop
  reads as `gate-unavailable:6`. This is a pipeline issue for the MO, and F has filed nothing.
- **A possible false MISMATCH under (a)** (the caveat above) is in `holler-pane` code from #647, already on `main`.
- These are unchanged from round 2: A-dup's warns D-2 to D-6 and the Phase 3 carries (W-4, W-5, W-6, W-9 and W-11, JSON
  mode writing DEL, C1 and bidi characters raw, #660). Until #649, the installed binary answers `error: not
  implemented` over `Unwired` (C7).

## Files changed

Production: none this cycle. Round 1's four files (`crates/holler-cli/src/pane/{list,get,watch}.rs`, `CHANGELOG.md`)
are unchanged since 06320ad.

Handoff documents (this cycle): `docs/handoffs/643/handoff-F.md`, `docs/handoffs/643/decisions.md`.
