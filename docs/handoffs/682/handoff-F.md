# Handoff-F: Phase 6 - #682 part 1, `FakeProfileStore` and the `ProfileStore` conformance suite

**Date:** 2026-10-09
**Branch:** issue-682-implementation (working tree on 79548b4; the script commits this phase)
**Issue:** #682 (slice c of #638, part 1 of 2; the PR is "Part of #682", see A's W-6)

## What was done

All under `crates/holler-pane-testkit/` except the CHANGELOG. The refactors came first, and slice a stayed green
after them (22 + 10).

- `src/feed.rs`: `Writer` (with `expected`) moved here from `pane_store.rs` as `pub(crate)`, with its doc made general:
  "the store's port-only rule (membership for panes, the name rule for profiles)". `lock` is now `pub(crate)`, and
  the module doc names both as the write-side helpers the fake stores share (A's W-3).
- `src/pane_store.rs`: imports `Writer` from `crate::feed` and keeps no copy. No behaviour change.
- `src/conformance/pane_store.rs`: `profile_name`, `expect_change`, `changes`, `cursors` and `increasing` are now
  `pub(super)`, each documented as reused by the profile suite. The last four are generic over `crate::feed::Change`
  except `increasing`, which takes `&[Cursor]`. The pane cases are unchanged.
- `src/fixture.rs`: adds `sample_spec(pane)` and `sample_profile(name, panes)`. `sample_pane` and `sample_spec` now
  share private `sample_model()`, `sample_context()`, `SAMPLE_GRID`, `SAMPLE_CWD` and the existing `SCRATCH`, so the
  two fixtures cannot drift (A's W-5). The module doc names all three fixtures.
- `src/profile_store.rs` (stub filled, 360 lines): `ProfileStoreOp` (8 ops, `profile_store.<method>`),
  `FakeProfileStore` (`new`, `seeded`, `set_idle_wait`, `set_now`, `faults`, `concurrent_put`,
  `concurrent_delete`, `Default`, the `ProfileStore` impl) and `impl Change for ProfileEvent` (key = the name's slug).
- `src/conformance/profile_store.rs` (stub filled, 521 lines): `profile_store_cases`, `run_profile_store_conformance`,
  the 23-row `CASES` table, cases 1 to 14 and the shared helpers (`actor`, `sample`, `stored_as`, `revised`, `put`,
  `listed`, `entries`, `history`, `step`, `shown`, `unchanged`). The module doc states the four rules the suite fixes
  beyond the port, with their reasons, for #661 to read (A's W-1).
- `src/conformance/profile_store/watch.rs` (new, 144 lines): cases 19 to 23, plus `open` and `last_cursor`.
- `src/conformance/profile_store/log.rs` (new, 129 lines): cases 15 to 18, plus `non_decreasing` and the env
  constants. This is the split the brief's Risks section prescribes; see "Deviations".
- `CHANGELOG.md`: one entry at the end of `[Unreleased]` / `Enhancements`, after the #676 entry (AC9).
- `docs/handoffs/682/evidence.md`: evidence appendix (see below).

## Design decisions

- **One write path per kind, the whole write inside one `feed.write`.** `put` (used by `cas_put`, `seeded` and
  `concurrent_put`) runs these steps in the brief's order. It applies the name rule to a port writer only, before the
  generation check. Then it runs `next_generation`, builds the stored record (the name's slug, the next generation,
  `created` kept from the stored record or `now`, `updated = now`), and calls `log.append(event)?`. Only after that
  does it append the log entry. `remove` checks existence first (`ProfileNotFound { what: <requested name> }`), then
  the generation, then the event (`name: <stored name>`, `profile: None`), then the `Deleted` entry at the deleted
  generation + 1. A failing `append` (cursor overflow) therefore logs nothing, and `Feed::write` wakes no watcher on
  `Err`, so a refused write publishes nothing (evidence entries 2 and 3).
- **`change_logs: Mutex<BTreeMap<String, Vec<ProfileLogEntry>>>`, not `history`** (A's W-4; the field is private).
  The lock order is documented on the field: the feed's lock first, then this one, never the reverse. A write takes
  it inside `feed.write`, and `log` takes it alone. No path takes the feed lock while holding it. It is locked through
  the shared `feed::lock`.
- **The clock is read inside the write**, under the feed's lock, so a write is stamped with the clock as it was when
  the write was applied. `AtomicI64` with `SeqCst`. Speed does not matter here; `SeqCst` was the obvious correct choice.
- **The `profile-exists` text** names the submitted name, the slug and the stored name, as the brief asks. It uses
  the `{:?}`-quoted style of `check_membership` in `pane_store.rs`.
- **`revised(profile, rev)` moves the first spec's `context.soft` up by `rev`**. That is the analogue of the pane
  suite's harness-port bump, so `p2` and `p3` are distinct writes. Every suite profile has at least one spec. With
  `soft` at 100,000 and `hard` at 150,000, a registry that checks `soft <= hard` still accepts it.
- **`stored_as(profile, generation, reply)`** takes `created` and `updated` from the store's own reply and sets the
  name's slug, so the suite pins no timestamp and no slug other than `name.slug()` (brief decision 6).
- **`history` validates every `Updated` summary** (non-empty, no `\n` or `\r`) each time a case reads the log. Every
  case that reads the history therefore checks the summary rule too.
- **`list` order:** `listed` sorts by `name.slug()` (stable) before any comparison, including inside `shown`, so
  "unchanged" does not depend on a store's list order (brief decision 9).
- **Case 18** builds its profile from `sample_spec(C1)` with the three env names, not by mutating `panes[0]`, so no
  index can panic. It compares whole records, so a store that dropped or rewrote an env name fails.
- **The generic `expect_change` bounds:** the brief's signature plus `E: Debug`. The failure detail prints the
  unexpected event (`{other:?}`), as before; `PaneEvent` and `ProfileEvent` both derive `Debug`.

## Reuse / extend-vs-new

Following the brief's "Extend vs new":
- **Extended:** the two stubs (`profile_store.rs`, `conformance/profile_store.rs`) and `fixture.rs`. The frozen
  `ProfileStore` is implemented through `next_generation` for every CAS. Only closed `PaneError` variants are returned
  (`Conflict`, `ProfileNotFound`, `ProfileExists`, `NotImplemented`, plus `Timeout`, `StoreCorrupt` and `Usage`, which
  come from the fault switch and the feed).
- **Reused, not copied:** `FaultSwitch` and `PortOp`; `Feed`, `Log`, `Change` and the `Watch` stream (`Feed::watch` with
  `ProfileStoreOp::WatchNext`); `Writer` and `lock`, now in `feed.rs` and shared by both fakes; `run_cases`, `succeeds`,
  `expect_code`, `expect_eq`, `next_item` and `drain` from `conformance/mod.rs`; `profile_name` and the four watch
  helpers from `conformance/pane_store.rs`, now generic; `ProfileName::slug` for every key and comparison; and
  `EnvVarName` as the only env guard.
- **New, as the brief justified:** the per-slug change log beside the feed, the clock, the name rule (`check_name`),
  `rename`'s refusal, the update `summary`, and the small `entry` constructor.
- **Accepted typed duplicates (A's handoff):** `open`, `last_cursor`, `put`, `shown`, `unchanged`, `sample` and
  `revised` over `ProfileStore`, and the two impl blocks.
- **Phase-7 checks pre-run:**
  - AC7's three greps print nothing.
  - `fn lock` exists only in `feed.rs:243` and as `FaultSwitch`'s private method at `fault.rs:105`.
  - `pane_store.rs` keeps no `Writer`.
  - `impl Change for ProfileEvent` is in `profile_store.rs`.
  - The profile suite, `watch.rs` and `log.rs` define no local copy of a shared helper.

## Architecture notes for A

- **Layers touched:** `holler-pane-testkit` only. The dependency direction is unchanged (`-> holler-pane` only), and
  `Cargo.toml` and `Cargo.lock` are untouched.
- **New public API:** exactly the brief's. In `profile_store`: `ProfileStoreOp` and `FakeProfileStore`. In
  `conformance::profile_store`: `profile_store_cases` and `run_profile_store_conformance`. In `fixture`:
  `sample_spec` and `sample_profile`. No flat re-exports, and `lib.rs` and `conformance/mod.rs` are not edited.
- **Crate-internal seams widened (planned):** `feed::Writer` and `feed::lock` are `pub(crate)`. Five pane-suite
  helpers are `pub(super)`, so the profile suite and its children import from the sibling `pane_store` (A's W-2; the
  follow-up to move them into `conformance/mod.rs` is O's).
- **New module boundaries:** `conformance/profile_store/{watch,log}.rs`, both private child modules whose case
  functions are `pub(super)`. They reach the parent's private helpers and constants through `super::`, as the
  `foo.rs` + `foo/` pattern allows.

## Deviations from spec / wireframe

1. **`src/conformance/profile_store/log.rs` exists.** It is not in the brief's Blast radius or Files lists, so AC8's
   file list, read literally, does not hold. With cases 1 to 18 in one file, `conformance/profile_store.rs` came to
   640 lines after rustfmt, past AC10's "no `.rs` file in the diff reaches 600 lines". The brief's Risks section
   prescribes this fix word for word: "move cases 15 to 18 into a second child module beside `watch.rs`
   (`profile_store/log.rs`), declared in the same file. `mod.rs` is still not touched." It is applied exactly that
   way. The files are now 521, 129 and 144 lines. The name `log` covers cases 15 and 16. Cases 17 (rename) and 18
   (env) went with them, as the brief says, and the module doc lists all four.
2. **The `expect_change` signature adds `E: Debug`** to the brief's bounds, so the unexpected event can still be
   printed. The function is `pub(super)`, and nothing outside the crate sees the bound.
3. **Pane-suite failure details change in text only.** The generic `expect_change` prints the key with `{key:?}`, so
   a pane failure now reads `PaneName("demo-c1r1")` where it used to read `demo-c1r1`. `changes` clones each record
   where it used to move it. Pass and fail are identical, slice a's tests are unchanged and green (AC5), and no test
   pins detail text: both suites only assert that a detail is non-empty.
4. **`lib.rs`'s one-line summary of `fixture` is slightly stale.** It names only `sample_pane`. AC8 forbids editing
   `lib.rs`, and A accepted the same staleness for `feed`. `fixture.rs`'s own module doc is current.

## Tier 1 self-check (incl. tests now GREEN)

RED before F: `cargo test -p holler-pane-testkit --no-run` failed with E0432 on the six missing items (T's RED).

```
$ cargo test -p holler-pane-testkit            # T's authored tests now GREEN, slice a still green
tests/fake_pane_store_test.rs              test result: ok. 22 passed; 0 failed
tests/fake_profile_store_test.rs           test result: ok. 22 passed; 0 failed   <- T's AC3/AC4 (+1 fixture test)
tests/pane_store_conformance_test.rs       test result: ok. 10 passed; 0 failed
tests/profile_store_conformance_test.rs    test result: ok. 9 passed; 0 failed    <- AC1, AC2 (suite, 23 ids, wrapper, 6 mutants)

$ cargo test --workspace                       # exit 0: 107 result lines, 1140 passed, 0 failed, 5 ignored (pre-existing)
$ cargo build --workspace                      # Finished, no warnings
$ cargo clippy --workspace --all-targets -- -D warnings
error: the function has a cognitive complexity of (16/15)
   --> crates/holler-pane-testkit/tests/fake_profile_store_test.rs:559:4     <- T's test, see "Tests that look wrong"
$ cargo clippy --workspace --all-targets --keep-going -- -D warnings   # that is the only error in the workspace
$ cargo clippy --workspace --all-targets --exclude holler-pane-testkit -- -D warnings      # Finished, clean
$ cargo clippy -p holler-pane-testkit --lib --test profile_store_conformance_test \
      --test pane_store_conformance_test --test fake_pane_store_test -- -D warnings      # Finished, clean
$ rustfmt --check --edition 2021 <every new or changed .rs, the two T test files included>   # clean
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane-testkit --no-deps [--document-private-items]   # clean
$ cargo machete                                # "didn't find any unused dependencies"
$ bash scripts/lint.sh                         # exit 0; no warn for any testkit file (largest: 534)
$ bash scripts/test-hooks.sh                   # exit 0
$ bash scripts/changelog-check.sh              # changelog-check: ok
$ git diff --quiet origin/main -- crates/holler-pane-testkit/Cargo.toml            # AC6 ok
$ cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'   # AC6: nothing
$ AC7's three greps                            # nothing
$ git diff --quiet origin/main -- tests/pane_store_conformance_test.rs tests/fake_pane_store_test.rs   # AC5 ok
$ git diff --quiet HEAD -- crates/holler-pane-testkit/tests/                         # F edited no test file
$ git diff --quiet origin/main -- src/lib.rs src/conformance/mod.rs src/profile_scope.rs src/conformance/profile_scope.rs   # AC8 ok
```

Function limits: clippy's `too_many_lines` (100) and `cognitive_complexity` (15) pass on the whole library, with one
function per case.

## Evidence appendix

`docs/handoffs/682/evidence.md`, 11 entries, each verbatim with `file:line`:
- `next_generation`.
- `Log::append` (cursor first, nothing changed on overflow) and `Feed::write` (notify only on `Ok`).
- The feed's `BTreeMap` by key: slug order, and lookup by slug.
- `FaultSwitch::enter` records the call before answering.
- `ProfileName::slug` (one slug, one profile), and the `usage` for a name with no ASCII letter or digit.
- `EnvVarName::parse`.
- `ProfileLogEntry.generation`'s doc (A's W-1).
- The code strings of `Conflict`, `ProfileNotFound` and `ProfileExists`.
- `run_cases` (a fresh subject per case).

## Tests that look wrong (for T)

- **`crates/holler-pane-testkit/tests/fake_profile_store_test.rs:559`
  `sample_spec_is_deterministic_harmless_and_agrees_with_sample_pane` fails clippy:** "the function has a cognitive
  complexity of (16/15)" (`clippy::cognitive_complexity` is denied workspace-wide). So `cargo clippy --workspace
  --all-targets -- -D warnings` fails (AC10), and so would CI, which runs exactly that command (`ci.yml:280`).
  - **Cause:** the function's own 17 assertion macros. Production code cannot change the count, and the test passes
    at runtime.
  - **Not edited:** F does not touch tests.
  - **Fix options for T:**
    - Move the three "agrees with `sample_pane`" assertions into their own `#[test]` (about +5 lines, so the file
      would be about 598, under the 600 warn).
    - Or fold related assertions into one tuple comparison, e.g. `assert_eq!((&spec.model, spec.context,
      &spec.host.cwd), (&pane.model, pane.context, &pane.host.cwd))`. That lowers the count and the line count.
    - An `#[allow(clippy::cognitive_complexity)] // #682` would also pass `scripts/lint.sh`, but a split is cleaner.

No other authored test looks wrong. All 31 new tests pass against the implementation, with no test-specific code in
`src/`.

## Known issues

- **AC10 (clippy) is red until T fixes the test above.** Every other AC10 guard passes, and clippy is clean on all
  production code.
- **AC8, read literally:** see Deviation 1 (`log.rs`, sanctioned by the brief's Risks section).
- **For the run's agent, not F:**
  - W-6: change the script's `Closes #682.` to `Part of #682.` and add the CONTRIBUTING.md AI disclosure with
    `gh pr edit`. After the merge, check that #682 is still open.
  - W-2: O's follow-up, moving the five shared helpers into `conformance/mod.rs` once slices b, d and e have merged.
- **W-1 stands as the brief decided:** a `Deleted` entry carries the deleted generation + 1. Both rules (g + 1, and the
  log never cut) and their reasons are stated in `conformance/profile_store.rs`'s module doc, which #661 reads. A
  switch to 0 would change cases 11 and 14, T's `the_clock_stamps_created_updated_and_at`, and one line of `remove`.

## Files changed

Production (F):
- `crates/holler-pane-testkit/src/feed.rs`
- `crates/holler-pane-testkit/src/pane_store.rs`
- `crates/holler-pane-testkit/src/fixture.rs`
- `crates/holler-pane-testkit/src/profile_store.rs`
- `crates/holler-pane-testkit/src/conformance/pane_store.rs`
- `crates/holler-pane-testkit/src/conformance/profile_store.rs`
- `crates/holler-pane-testkit/src/conformance/profile_store/watch.rs` (new)
- `crates/holler-pane-testkit/src/conformance/profile_store/log.rs` (new)
- `CHANGELOG.md`

Pipeline artifacts: `docs/handoffs/682/handoff-F.md`, `docs/handoffs/682/evidence.md`, `docs/handoffs/682/decisions.md`
(appended).

Not changed by F: both test files (T's), `lib.rs`, `conformance/mod.rs`, the `profile_scope` stubs, every manifest,
`Cargo.lock`, and every other crate.
