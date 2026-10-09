# Handoff-F: Phase 6 (the script's numbering; the pipeline's F phase) - #688 `FakeProfileScope` and the `ProfileScope` conformance suite

**Date:** 2026-10-09
**Branch:** issue-688-implementation
**Issue:** #688 (part of #638, epic #633)

## What was done

- `crates/holler-pane-testkit/src/profile_scope.rs` (the stub is now 293 lines): `FakeProfileScope`, its `ProfileScope` impl
  and `before_next_restore`. It follows the brief's "Fake behaviour" step by step. Steps 1 to 5 are profile `get`
  (`profile-not-found` first), the `usage` guard for a `Set` filed under another pane, the pane-record `get` for every
  edit, the membership check for a `Set` only, and the profile `cas_put` at g. Step 6 runs the act. Step 7, on a failed
  act, runs the one-shot hook and then the restoring `cas_put` at g + 1: `Conflict` becomes `profile-conflict`, and any
  other error is returned as it is. Each step is a small private function (`stored`, `members`, `member`, `plan`,
  `restore`, `belongs`, `check_filed_under`, `check_joins`, `with_edit`). It carries the brief's `ASSUMPTION (#663)` and
  `ASSUMPTION (#661/#663)` comments at the code concerned.
- `crates/holler-pane-testkit/src/conformance/profile_scope.rs` (the stub is now 492 lines). The module docs carry the
  fixture, the `ASSUMPTION (#663)` list with the open restore-failure point, the `ASSUMPTION (#661/#663)` paragraph and
  the mapping of #663's acceptance bullets to cases. The file also holds the 15-row `CASES` table,
  `profile_scope_cases`, `run_profile_scope_conformance`, the seed (`seed`, `seeded_panes`), cases 1 to 8 and the shared
  helpers (`stored`, `records`, `specs`, `changed_spec`, `set`, `edit_with`, `edit`, `expect_edited`, `gained`).
- `crates/holler-pane-testkit/src/conformance/profile_scope/act.rs` (new, 219 lines): cases 9 to 15 as `pub(super)`
  case functions, plus `act_failed` and `no_write`.
- `crates/holler-pane-testkit/src/conformance/profile_store.rs` (visibility only): `actor`, `sample`, `history`,
  `shown`, `unchanged`, the types `Shown` and `Step`, and the constants `UPDATED`, `ALPHA`, `BETA`, `GAMMA`, `C1`, `C2`
  and `C3` are now `pub(super)`. Each doc gains "The profile scope suite reuses it" (or names what it reuses).
- `crates/holler-pane-testkit/src/conformance/pane_store.rs` (visibility only): `pane_name` is now `pub(super)` and gets
  the doc line "The pane name `text`. The profile scope suite reuses it."
- `crates/holler-pane-testkit/src/pane_store.rs` (visibility only): `check_membership` is now `pub(crate)`, and its doc
  names `FakeProfileScope` as its second caller.
- `docs/adr/ADR-0021.md`: one sentence appended to section 8, step 1 (+3 lines). It includes A's W-7 clause.
- `CHANGELOG.md`: one entry at the end of `## [Unreleased]` / `### Enhancements`, after the #681 entry.
- `docs/handoffs/688/evidence.md` (new) and this handoff; a `decisions.md` entry appended.

## Design decisions

- **The suite's internal shape.** The brief recommended an owned `Bench<S>`; I refined it into two structs.
  `Seeded<S>` is the owned subject that `run_cases` keeps per case: the scope and the two `Arc` fakes. `Bench<'a>` is
  the borrowed view a case gets, with `scope: &dyn ProfileScope`. A case is `fn(&Bench<'_>) -> Result<(), String>`, so
  the `CASES` table is not generic. A seeding failure fails every case with its reason, and nothing in `src/` unwraps.
- **The pane fixture is seeded through one `PaneError` function** (`seeded_panes`), whose error is mapped to a `String`
  once. The alternative was a local `fn member(name, profile) -> Result<Pane, String>`. That would have copied the
  pane suite's private `sample`, or needed a fourth visibility change.
- **The log checks read only what the log gained** (`gained`), as `(generation, kind)` pairs. They also require that
  the log still starts with its earlier contents and that the new entries share one actor. The suite cannot pin the
  scope's actor (the brief: "only that the edit and its reversal share one"), so it compares no `Step` that holds an
  actor.
- **`expect_edited`** is the one check behind cases 5, 6, 7 and 15. The answer is `Some(r)` with `r == get(Alpha)` at
  g + 1, the specs are exactly the expected ones, and the act ran once.
- **`SEEDED` (= 1)** names the fixture's generation g, so every case states g + 1 and g + 2 as the module docs do.
- **Case 11 checks the error's `Display`** (`e.to_string()` contains `Demo Alpha`), not a field. `ProfileConflict`
  displays as `profile conflict: {what}`, so any implementation's `what` is checked without matching the variant.
- **The hook lock.** `restore` takes the hook out in a `let` statement of its own, so the guard drops at the `;`, and
  only then calls it (W-8). `before_next_restore` also drops a replaced hook after the lock is released, so a captured
  value's drop code never runs under the lock either. The mutex is read through `crate::feed::lock`, the crate's
  shared take-over-a-poisoned-lock helper. It is the same `lock().unwrap_or_else(PoisonError::into_inner)` that
  `FakeProber` writes inline, reused rather than copied.
- **`Remove` drops every entry that names the pane** (`retain`). A well-formed profile has at most one, so this matches
  the brief's "drops it (the others keep their order)". A `Set` replaces the first entry in place, or appends one.
- **Messages.** `pane-not-in-profile` is `"{pane} is not in profile {P:?}"`. `usage` is
  `"a spec for the pane {spec.pane:?} cannot be set as the spec of {pane}"`. `profile-conflict` names P, the pane and
  the act's failure, says P's specs were not restored and that the other writer's version stays. The suite pins only
  case 11's profile name.
- **Refusals name the stored profile's display name.** That name has the requested name's slug, so a request for
  `DEMO-ALPHA` reads `"Demo Alpha"`.
- **Case 12 pins the profile store's calls only**, as the brief's table does. The fake makes no pane store call either
  (its doc says so), but the suite does not bind #663 to that.

## Reuse / extend-vs-new

- **Extended:** the two stubs declared by #638, which implement the frozen `ProfileScope` port, as the Reuse map says.
- **Reused, not copied:** `FakeProfileStore` (`seeded`, `faults`, `concurrent_put`) and `FakePaneStore` (`seeded`,
  `faults`) under the scope; `check_membership`, the only producer of the `pane-in-other-profile` error in `src/`,
  called only on the `SpecEdit::Set` path; `crate::feed::lock`; `run_cases`, `succeeds`, `expect_code` and `expect_eq`;
  part 1's `actor`, `sample`, `history`, `shown`, `unchanged`, `Step` and name constants; `profile_name`, `pane_name`;
  `sample_pane`, `sample_spec`; `ProfileName::slug` for every membership comparison.
- **New, as the brief justifies:** the scope itself (the I8 sequence and its compensation), the suite's two-store seed,
  the edit application (`with_edit`), and the suite's own small helpers (`gained`, `expect_edited`, `changed_spec`,
  `edit_with`, `no_write`). `holler-pane`'s test-local `MemScope` is not reused, as the brief says.
- **AC7:** both greps print nothing. No local `fn actor|sample|history|shown|unchanged|pane_name|profile_name` exists in
  `conformance/profile_scope*`, and no file outside `pane_store.rs` names the variant behind `pane-in-other-profile`.

## Architecture notes for A

- **Layers:** test kit only (`crates/holler-pane-testkit/**`). No change to `holler-pane`, `holler-hub` or `holler-cli`,
  and no manifest change (AC6 holds). `lib.rs` and `conformance/mod.rs` are untouched.
- **New public surface,** exactly as the brief pins it:
  - `holler_pane_testkit::profile_scope::FakeProfileScope`, with `new(Arc<dyn ProfileStore>, Arc<dyn PaneStore>, Actor)`
    and `before_next_restore(impl FnOnce() + Send + 'static)`;
  - `holler_pane_testkit::conformance::profile_scope::{profile_scope_cases, run_profile_scope_conformance}`.
- **New module:** `conformance/profile_scope/act.rs`, a child of the scope suite, planned from the start (W-2).
- **New crate-internal edges:**
  - `profile_scope` uses `pane_store::check_membership` (now `pub(crate)`) and `feed::lock` (already `pub(crate)`);
  - `conformance::profile_scope` uses `conformance::profile_store` (its helpers and constants, now `pub(super)`) and
    `conformance::pane_store::pane_name` (now `pub(super)`).
- **Send + Sync:** `FakeProfileScope` holds `Arc<dyn ProfileStore>` and `Arc<dyn PaneStore>`, both `Send + Sync` by the
  ports' supertraits, plus a `Mutex<Option<Box<dyn FnOnce() + Send>>>`. T's `the_scope_is_send_and_sync` pins it.
- **Locks:** no lock is held across `act()` or the hook, and the scope takes only its own hook mutex. The stores' locks
  are their own concern.

## Deviations from spec / wireframe

No wireframe (no UI surface). Deviations from the brief, each small:

1. **`CREATED` stays private** in `conformance/profile_store.rs`. The brief listed it among the items made `pub(super)`,
   but the scope suite never reads a `created` entry, because its log checks read only what the log gained. A
   `pub(super)` whose doc says "The profile scope suite reuses it" would be false. Every other listed item is reused
   and is `pub(super)`.
2. **The ADR sentence includes A's W-7 clause and ends with `(#688)`**, as A's Notes for O suggest: "with or without
   `--spec-only` (the scope cannot see that an act is empty)". It is still one sentence in section 8, step 1, and
   nothing else in the ADR changed (AC8). It takes +3 lines; the brief estimated about 2. The same clause is mirrored in
   the suite's `ASSUMPTION (#661/#663)` paragraph and in the fake's comment. The brief's own forward-compat row is O's
   document, and I left it unedited.
3. **`pane_name` gained a doc line**, as `profile_name` has: ±2 lines where the brief said ±1.
4. **The suite's internal shape** is `Seeded<S>` plus `Bench<'a>` rather than the brief's single `Bench<S>`. The brief
   left this open ("F may refine; not pinned by a test").

## Tier 1 self-check (incl. tests now GREEN)

T's two new targets, 24 of 24 GREEN:
```
$ cargo test -p holler-pane-testkit --test profile_scope_conformance_test --test fake_profile_scope_test
     Running tests/fake_profile_scope_test.rs
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/profile_scope_conformance_test.rs
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
The whole test kit. AC5's four earlier targets pass unedited (`pane_store_conformance_test` 10,
`fake_pane_store_test` 22, `profile_store_conformance_test` 9, `fake_profile_store_test` 23):
```
$ cargo test -p holler-pane-testkit
lib 4, envelope 10, fake_harness 19, fake_herdr 22, fake_host 12, fake_pane_store 22, fake_prober 6,
fake_profile_scope 18, fake_profile_store 23, harness_conformance 13, herdr_conformance 15,
host_conformance 12, pane_store_conformance 10, profile_scope_conformance 6, profile_store_conformance 9,
doc-tests 1: every "test result: ok", 0 failed
```
The guards (AC6, AC7, AC10):
```
$ cargo build --workspace                                   -> Finished, exit 0
$ cargo clippy --workspace --all-targets -- -D warnings     -> Finished; 0 warning/error lines
$ cargo test --workspace                                    -> exit 0; 117 test targets "test result: ok",
                                                               1279 passed, 0 failed, 5 ignored (pre-existing #[ignore]s)
$ rustfmt --check --edition 2021 <the 6 changed src files and T's 2 test files>   -> exit 0
$ cargo machete                                             -> "didn't find any unused dependencies", exit 0
$ bash scripts/lint.sh                                      -> exit 0 (only warns on files outside this diff)
$ bash scripts/changelog-check.sh                           -> changelog-check: ok
$ bash scripts/test-hooks.sh                                -> every case ok, exit 0
$ git diff --quiet origin/main -- crates/holler-pane-testkit/Cargo.toml Cargo.toml Cargo.lock   -> exit 0
$ cargo tree -p holler-pane-testkit -e normal --prefix none | grep -E '^holler-(cli|hub|adapter)'  -> nothing
$ grep -rn "PaneInOtherProfile" crates/holler-pane-testkit/src | grep -v pane_store.rs              -> nothing
$ grep -rnE "^(pub\(super\) )?fn (actor|sample|history|shown|unchanged|pane_name|profile_name)\b" \
    crates/holler-pane-testkit/src/conformance/profile_scope*                                       -> nothing
```
Sizes: `src/profile_scope.rs` 293, `conformance/profile_scope.rs` 492, `conformance/profile_scope/act.rs` 219,
`conformance/profile_store.rs` 527, `conformance/pane_store.rs` 535, `pane_store.rs` 240. Every suite file is under the
600-line warn, and no function is near 100 lines or complexity 15 (clippy denies both and is clean).

**The suite rejects each mutant for the right reason.** I checked this with a throwaway probe test that printed each
mutant's full failure list. It was deleted before this handoff and is not in the diff.
- `WritesAfterAct` fails `the-act-sees-the-edit` ("the generation get(Alpha) shows inside the act: expected 2, got 1")
  and `failed-act-restores-the-specs` ("expected 3, got 1"), and also cases 10, 11, 13 and 14 (the act ran when it must
  not).
- `NoRestore` fails `failed-act-restores-the-specs` (the specs keep `s'`) and case 11.
- `MembershipOnRemove` fails `remove-of-a-detached-spec-is-not-refused` alone.
- Two extra mutants also fail where expected: a scope that retries a conflicting first write fails case 10 ("expected
  `generation-conflict`, but it succeeded"), and one that turns the act's error into `Ok(None)` fails case 9.

## Evidence appendix

`docs/handoffs/688/evidence.md` has 14 entries. They cover the frozen port, ADR-0021:282-283 and the `pane close` row,
`check_membership`'s body, the fault switch's one-shot and wedged rules, both seeds, `concurrent_put`, the `Updated`
log entry, `feed::lock`, `run_cases`, `expect_code`, and `ProfileConflict`'s `Display`.

## Tests that look wrong (for T)

None. All 24 pass against the brief's "Fake behaviour" without any reading that strains it.

## Known issues

- **Open by design, not a defect:** a restoring write that fails with anything but a conflict returns its own error.
  The profile then keeps the edit and the act's error is lost. This is listed for #663 to decide, in the suite's module
  docs and at the fake's code.
- **For O/S:** issue #688's body is still the pre-review text. It says "the 14 cases and 2 scope mutants" and "~1,020
  lines", and it mentions neither the restore hook nor the ADR line. A's Notes for O item 2 asked for a dated
  "(amended 2026-10-09, plan review)" note there. S audits against the issue, so this matters there. It is outside F's
  remit, and I made no change to the issue.

## Files changed

Production files only (T's two test files are not listed):
- `crates/holler-pane-testkit/src/profile_scope.rs`
- `crates/holler-pane-testkit/src/conformance/profile_scope.rs`
- `crates/holler-pane-testkit/src/conformance/profile_scope/act.rs` (new)
- `crates/holler-pane-testkit/src/conformance/profile_store.rs`
- `crates/holler-pane-testkit/src/conformance/pane_store.rs`
- `crates/holler-pane-testkit/src/pane_store.rs`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`
- `docs/handoffs/688/handoff-F.md`, `docs/handoffs/688/evidence.md`, `docs/handoffs/688/decisions.md` (pipeline
  artifacts)
