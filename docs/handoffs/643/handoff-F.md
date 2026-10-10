# Handoff-F: Phase 5 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (amendment 1)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on 350858d, T-red's PASS on amendment 1)
**Issue:** #643 (epic #633). Brief: `docs/handoffs/643-brief.md` (amendment 1). Inputs: `handoff-A.md` (PASS, W-12 to
W-16, notes for F) and `handoff-T-red.md` (RED: 31 passed, 5 failed).

Round 3 of this handoff (the stop for the MO's ruling) is in git: `git show e499e28:docs/handoffs/643/handoff-F.md`.
This file replaces it. This cycle is the Workflow script's phase 6 (F).

## Outcome

`done: true`, `archChanged: false`. T-red's five failing tests pass, and so does everything else (Tier 1, below).
SYNC is now `shown_differs`'s answer, DRIVEN is printed as stored and never compared, and A's W-12 and W-13 are in.

## What was done

- `crates/holler-cli/src/pane/list.rs`:
  - `SessionSync::of` takes `&Pane`. It returns `Unobserved` when `last_observed.at <= 0` or the pane has no session of
    record. Otherwise it returns `Mismatch` or `Ok`, as `holler_pane::reconcile::shown_differs(session_of_record,
    last_observed.shown)` answers. Its three values and serde names are unchanged.
  - The call site in `PaneRow::from` passes the pane.
  - Rewritten docs, so that DRIVEN is no longer "what reconcile last recorded":
    - the module doc;
    - `PaneList`'s help, which now states SYNC's rule with AC 19's `session of record`, `home screen` and `#649`;
    - `PaneRow`'s `shown`, `driven` and `sync` docs;
    - `SessionSync`'s docs.
  - The `LastObserved` import is gone, and `shown_differs` is imported.
- `crates/holler-cli/src/pane/get.rs`:
  - The call site in `detail` is `SessionSync::of(&pane)`.
  - The module doc and the help say what SYNC compares and that DRIVEN is printed as stored until #649.
- `crates/holler-cli/src/pane/watch.rs`:
  - W-4's help sentence: ROW is a `pane list` row, not the full record `pane get` prints.
  - W-12: `Members` keeps the scoped `ProfileName`, and membership is `holler_pane::profile_diff::is_member`.
    `names_profile` and the `Pane` import are gone.
- `CHANGELOG.md`: AC 23's two sentences are replaced, and the entry is rewrapped.
- `docs/handoffs/643/evidence.md`, now 11,901 bytes:
  - A new amendment-1 section comes first. It has W-13, W-12 and W-16, plus a note on how this round moved the
    brief's dated verb-file cites.
  - Round 2's W-9 section is now a pointer, because the brief carries those points now.
  - Cites into the verb files are corrected.
  - Two of T's excerpts are trimmed to the lines that carry the fact.
  - Every in-repo excerpt was checked mechanically against its source (0 misses).

## Design decisions

- **The shape of `SessionSync::of`:**

  ```rust
  if observed.at <= 0 || record.is_none() { Unobserved }
  else if shown_differs(record, observed.shown.as_deref()) { Mismatch }
  else { Ok }
  ```

  I considered mirroring doctor's `let Some(record) = .. else` followed by `shown_differs(Some(record), ..)`
  (`observe.rs:242-247`), and a `match`. The `if` chain reads as the doc's three outcomes, and it passes the
  `Option` to `shown_differs` without wrapping it again. The `is_none()` test is not a second comparison. It is the
  "cannot compare" case that Decision 3 adds (A's W-14 calls it the reader's form). The comparison itself is the call.
- **W-13: `at <= 0`, not `at == 0`.** That is `shown_differs`'s "provided `at > 0`", and it is where `observed_at`
  already prints `never`. So `pane get` cannot print `observed-at: never` beside `sync: MISMATCH`.
  `a_negative_observed_at_is_never_observed` pins it.
- **W-12: call `is_member` directly.** I did not keep a `names_profile` wrapper. `Members` holds the `ProfileName`,
  which moves out of `open` after `resolve` borrows it, instead of a cached slug. The comparison is the same, so the
  output is the same.
- **Help wording.**
  - SHOWN is "the session the pane's TUI showed when reconcile last observed it": past tense, because it is a record.
  - DRIVEN is "printed as the record holds it", and `-` until #649.
  - SYNC names its three cases and says it is the rule `pane doctor` uses.
  - The watch help's example line (`... shown=- driven=- sync=- ...`) is still right: a sample pane has no session
    of record.
- **No merge of `origin/main`.** It is two commits ahead (0ad2d8a, #640 part 2; dc300ab, #642 part 1). They touch
  only the adapter crates, `Cargo.lock` and `CHANGELOG.md`. A merge is a commit, which is not F's work. The working
  tree merges cleanly (`git merge-tree` over a throwaway index, exit 0).

## Reuse / extend-vs-new

- **Reused:** `holler_pane::reconcile::shown_differs` (Reuse map, amendment 1).
  - It is called once, from `SessionSync::of`, and the verbs compare no session themselves.
  - `last_observed.driven` is only printed (`PaneRow::from`, and `get`'s `driven:` line), never compared.
- **Extended in place:** `SessionSync` (Reuse map: it "keeps its three values and serde names and extends
  `shown_differs` with the `unobserved` case").
- **Reused:** `holler_pane::profile_diff::is_member` (A's W-12, not in the Reuse map). It replaces the inline slug
  comparison in `watch.rs`, so `watch --profile` and the profile verbs share one membership rule.
- **Decided non-folds, recorded in `evidence.md`:** `findings::quoted` and `profile_diff::FieldValue`'s `Display`
  (W-16).
- **No new object.**

## Architecture notes for A

- **One public signature changed:** `holler_cli::pane::list::SessionSync::of(&LastObserved)` became `of(&Pane)`.
  - Amendment 1 specifies this change, and A reviewed it at Phase 3.
  - Its only callers are `PaneRow::from` and `get`'s `detail`. `grep -rn SessionSync crates` finds no other use.
- **New imports:**
  - `holler_pane::reconcile` in `list.rs`, which `pane doctor` already imports;
  - `holler_pane::profile_diff` in `watch.rs`, which `profile list` and `profile show` already import.
- **Unchanged:** the crate dependencies and their direction. No frozen file, manifest, wire format, golden file or ADR
  is touched.
- **Why `archChanged: false`:** no module boundary moved, and nothing beyond the plan A reviewed changed (A's Notes for
  F, 2).

## Deviations from spec / wireframe

- **W-13:** the guard is `at <= 0`, where the letter of Decision 3 says `at == 0`.
- **W-12:** membership is `is_member`, where Decision 7's wording says "slug compared, as `ProfileName::slug`". It is
  the same comparison.
- Both follow A's notes, and `evidence.md` records both with source.
- The CHANGELOG entry is rewrapped (cosmetic). The entry is new on this branch, so the diff against `main` is the
  whole entry either way.
- There is no wireframe (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::        # before the change: T-red's RED
test result: FAILED. 31 passed; 5 failed; 0 ignored; 0 measured; 91 filtered out
$ cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::        # after
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 91 filtered out; finished in 10.03s
$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test
cli_surface_test 3 passed; docs_cli_test 3 passed; pane_cli_process 34 passed; pane_verbs 127 passed; 0 failed
$ HOLLER_STATE_DIR=<empty scratch dir> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
128 result lines: 1472 passed, 0 failed, 5 ignored; exit 0
$ cargo clippy --workspace --all-targets -- -D warnings                     -> exit 0
$ bash scripts/lint.sh                                                       -> exit 0
$ bash scripts/changelog-check.sh                                            -> changelog-check: ok
$ cargo machete                                                              -> no unused dependencies
$ rustfmt --check --edition 2021 <the seven .rs files of AC 22>              -> exit 0
$ AC 17 grep (ports.herdr|host|harness|prober), AC 21 grep, `grep -n unsafe`  -> nothing
$ git diff --stat $(git merge-base HEAD origin/main) -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'   -> empty
$ wc -l src/pane/{list,get,watch}.rs                                         -> 335, 277, 223 (all ASCII)
```

I read the rendered `holler pane {list,get,watch} --help`: the SYNC, DRIVEN and W-4 sentences print as intended. The
CLI surface is unchanged.

## Evidence appendix

`docs/handoffs/643/evidence.md`, section "F (Phase 5, implement, amendment 1)": W-13 (`pane.rs:188-189`), W-12
(`profile_diff.rs:258-265`) and W-16 (`profile_diff.rs:197-199`, `profile/show.rs:181-198`).

## Tests that look wrong (for T)

None. AC 3's six rows, the W-13 guard test and AC 19's needles all match the brief and A's notes.

## Known issues

- **`evidence.md` has 99 bytes of headroom under the gate's 12,000-byte cap.** T-green should put any new entry first
  and turn older ones into pointers, as this round did.
- **The brief's line cites into the verb files are at 05e7337.**
  - This round moves `list.rs` lines down by 4 to 19, and lines in `get.rs` and `watch.rs` down by 2.
  - The brief said F's edit would move them. `evidence.md` records the shift for the gate.
- **The branch is two commits behind `origin/main`.** It merges cleanly. The run's agent should bring `main` in before
  the PR merges.
- **Unchanged, as the brief lists them:**
  - D-2, widened by W-16, plus D-3 to D-6;
  - #647's unseen first observation, which can show a false MISMATCH;
  - W-14, W-15, W-5, W-6 and W-11.
  - None of these is filed yet (A's Notes for O, 2).
- **Until #649,** the installed binary answers `not implemented` (C7).
- **Not this story's:** `cargo doc -p holler-cli` with broken intra-doc links denied fails on existing diagnostics at
  `cli.rs:193`, `:553` and `:563`. The three verb files have none. rustdoc is not a gate.

## Files changed

Production:
- `crates/holler-cli/src/pane/list.rs`
- `crates/holler-cli/src/pane/get.rs`
- `crates/holler-cli/src/pane/watch.rs`
- `CHANGELOG.md`

Handoff documents:
- `docs/handoffs/643/evidence.md`
- `docs/handoffs/643/handoff-F.md`
- `docs/handoffs/643/decisions.md`
