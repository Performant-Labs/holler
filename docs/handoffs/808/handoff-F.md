# Handoff-F: Phase 6 (implement) - #808 a Herdr snapshot pane carries its label

**Date:** 2026-10-11
**Branch:** `issue-808-implementation` (base `main` at `92fbc6e`; T-red at `bc96f89`)
**Issue:** #808

Phase numbering is the Workflow script's (F is its Phase 6); the role doc calls this phase 5.

## What was done

- `crates/holler-pane/src/pane.rs`: `HerdrPane` gains `label: Option<String>` as its last field, with
  `#[serde(default, skip_serializing_if = "Option::is_none")]`. The struct and field docs say what E13 asks for.
- `crates/holler-pane/src/ports.rs`: the `HerdrPort` trait docs say that `snapshot` and `ensure_pane` report the label.
  The contract is the trait, so its docs are amended with the type. This is a doc-only change.
- `crates/holler-pane/src/tx_launch.rs`: `Plan::record` writes `herdr: HerdrPane { label: None, ..live.herdr }`
  (constraint (c)). O2 compares four fields through a new `same_place` helper beside `same_pane`, which it reuses,
  instead of the derived `PartialEq` through `Vec::contains` (E11). The message is unchanged.
- `crates/holler-adapter-herdr/src/protocol.rs`:
  - `PaneRef.label` holds the raw label as sent.
  - `parse_pane_info` returns `(PaneId, Option<String>)`, and `parse_workspace_created` returns
    `(WorkspaceRef, PaneId, Option<String>)`.
  - The new private `sent_label` is the one optional read. The Parsers bullet now names it as the rule's exception.
  - `SessionState::pane_label` looks a label up by pane id.
  - `pub fn label_of` is clarification 2's coercion: `None` and `""` give `None`, and other text is kept untrimmed.
    Its doc cross-references the fake's copy.
- `crates/holler-adapter-herdr/src/adapter.rs`:
  - The new `HerdrAdapter::pane` is the one place the adapter builds a `HerdrPane`, and it calls `label_of`. Both
    `snapshot()` and `ensure` use it.
  - `snapshot()` joins each placed cell's pane id to the call's one `session.snapshot` read.
  - `WorkspaceGrid` now carries the `SessionState` it was read from.
  - The new `WorkspaceGrid::occupant` is the existing occupied-cell lookup, moved out of `ensure`, plus the label.
  - `place` also returns the label of the `workspace.create` / `pane.split` reply.
  - The module docs gain the label rules.
- `crates/holler-pane-testkit/src/herdr.rs`:
  - `LivePane.label` stores the label as given.
  - `FakeHerdr::rename` is an outside-world hook like `vanish`/`print`: it bypasses the faults and the call log, and
    an unknown pane is `pane-not-found`.
  - `ensure_pane` now answers the pane's own snapshot entry, found through the new `State::listed()`. So
    `Workspace::listed` is the fake's one `HerdrPane` construction site.
  - A private `label_of` copy sits beside `last_lines` with a cross-reference.
  - The struct and module docs list the new hook.
- `crates/holler-pane-testkit/src/conformance/herdr.rs`:
  - Case 12, `labelled-pane-reports-its-label`, is last in the one `CASES` table.
  - `run_herdr_conformance(fresh, label: Option<LabelHook<'_, H, K>>)`; with `None`, the runner filters the case out.
  - `pub type LabelHook` and the private `Rename` alias.
  - `Scratch.label`.
  - The module doc names the case as the suites' one hook-driven exception, and the runner's doc example shows the new
    parameter.
- `crates/holler-pane-testkit/src/fixture.rs`: `sample_pane`'s `HerdrPane` literal gains `label: None`.
- `docs/adr/ADR-0021.md`: clarification 5's four places, written out as given:
  - section 2's new sentences after the "provisional" note;
  - section 10's new bullet;
  - the "Deferred" `HerdrPort` bullet replaced;
  - "Decisions taken" item 10.

  Plus section 1's `herdr` row (A warn 2): `label?` and a note.
- `CHANGELOG.md`: an `## [Unreleased]` / Enhancements entry linking #808, at the top of the list.
- `docs/handoffs/808/evidence.md`: five facts from unchanged code the diff relies on.

## Design decisions

1. **One `HerdrPane` constructor per implementation, instead of two literals that each call `label_of`.**
   - Clarification 2 asks that `snapshot()` and `ensure_pane` "can never disagree".
   - In the adapter, both go through `HerdrAdapter::pane`, which is the only `HerdrPane` literal and the only
     `label_of` call.
   - In the fake, `ensure_pane` returns the snapshot entry itself (A warn 1's fix, in its strongest form).
   - Alternative considered: keep the two adapter literals and call `label_of` in each, as E10 item 3 words it. The
     literals differed only in where `workspace` and `grid` come from, so they collapse with no loss.
2. **The fake's `ensure_pane` finds its answer in `listed()`, not through a `place` that returns `&LivePane`.**
   - Returning a borrowed pane from `place` hits the conditional-borrow limit of NLL (occupant or mint). The fixes
     were an impossible-error branch or an entry-API dance.
   - Looking up the id that `place` returned in `listed()` reuses the existing code, and its only failure is a natural
     `pane-not-found`.
   - It is O(panes), which is irrelevant for a test fake.
3. **The label case is skipped in the runner, and fails closed in the case.**
   - With `None`, the runner filters the one `CASES` table, so no fixture is built. That matters for the opt-in run,
     where a fixture is a real scratch Herdr server.
   - If the case ever ran without a hook, it would return `Err` rather than pass vacuously.
   - `herdr_cases()` still lists all 12.
   - A warn 5 allowed the skip "in the runner or in the case".
4. **`LabelHook<'a, H, K>` type alias** for E12's parameter type. It expands to exactly
   `&'a dyn Fn(&HerdrFixture<H>, &K, &PaneId, &str) -> Result<(), PaneError>`. Spelled out in the signature (and in
   `Scratch`'s field), clippy's `type_complexity` fires, and CI runs clippy with `-D warnings`.
5. **The subject is `(fixture, guard)` and the guard is `()`.** This follows E12 and the harness runner's own
   repacking. A tuple drops its fields in order, so the fixture is still dropped before its guard (evidence fact 1).
6. **`sent_label` reads `label` raw, and `label_of` is the only coercion.** `PaneRef.label` and the parsers' tuples
   carry the label as sent (`""` included), and T's parser tests pin that. The fold happens once, at construction.
   - I did not reuse `pane_ref` for the split and create replies (A warn 3). It requires `workspace_id` and `tab_id`,
     which would make those parsers stricter.
7. **`same_place` reuses `same_pane`** and adds `workspace` and `grid`. That keeps the join key in one place, and E11
   asked for the helper beside it.
8. **The `ensure` occupied-cell branch became `WorkspaceGrid::occupant`.** The behavior is the same, plus the label.
   Inline, rustfmt reflowed the whole match, so the method keeps `ensure`'s shape and diff small.

## Reuse / extend-vs-new

Extended:

- `HerdrPane`, not a new snapshot-entry type: clarification 6, the survey's E5 and A's confirmation.
- `PaneRef`, `SessionState` (beside `workspace`), `WorkspaceGrid`, and the two `PaneInfo` parsers.
- `FakeHerdr` (`rename` beside `vanish`/`print`) and `LivePane`.
- `run_herdr_conformance` (one more parameter) and the one `CASES` table.
- `Plan::record` and `observe_live` in place.

New, each named by the brief or by A:

- `label_of`: clarification 2.
- `sent_label`: A warn 3's "one helper".
- `SessionState::pane_label`: E10 item 3, named per A warn 3.
- `same_place`: E11's "four-field helper beside `same_pane`".
- The fake's private `label_of` copy: A warn 4, following the documented `last_lines` precedent. The two docs name
  each other.
- `LabelHook`/`Rename`: type aliases only.

Extracted from existing code, not new behavior:

- `HerdrAdapter::pane`: the two former literals.
- `WorkspaceGrid::occupant`: the old `[]` arm.
- `State::listed`: the old body of `snapshot()`.

## Architecture notes for A

- **Contract change (`holler-pane`, public):** `HerdrPane.label: Option<String>`.
  - The persisted form of an unlabelled record is byte-identical, pinned by T's `PRE_808_RECORD`.
  - The only production writer of a record's `herdr` (`Plan::record`) clears the label.
  - O2 no longer compares whole values.
  - No other production comparison of `HerdrPane` values exists (A's re-check).
- **Public API changes, `holler-adapter-herdr::protocol`:**
  - `PaneRef` gains a pub field. Only tests construct it, and T updated them.
  - `parse_pane_info` and `parse_workspace_created` return tuples. Their only production caller is `adapter.rs`.
  - New `pub fn label_of` and `pub fn SessionState::pane_label`.
- **Public API changes, `holler-pane-testkit`:**
  - `run_herdr_conformance` takes a second parameter. All nine callers are T's tests, already updated.
  - New `pub type LabelHook` and `FakeHerdr::rename`.
- **Unchanged:** no new dependency, module or crate, and no change of dependency direction. No method was added to
  the Herdr allow-list (`METHOD_COUNT` is still 9). Nothing changed in the hub, `holler-proto`, `holler-cli/src`,
  golden files or `docs/protocol/v2.md`.
- **Follow-up (A warn 7, input for #650):** the label clearing and the four-field comparison live in `tx_launch.rs`.
  When the importer becomes a second writer, move both to one named place in `holler-pane`.

## Deviations from spec / wireframe

- **E10 item 3.** It says "the two `HerdrPane` literals call `label_of`". There is now one literal, through
  `HerdrAdapter::pane`, which both callers use (design decision 1). Clarification 2's requirement (both use the
  coercion, so they never disagree) holds more strictly.
- **E12's signature is written through the `LabelHook` alias.** It is the identical type (design decision 4), and T's
  call sites compile unchanged.
- **ADR-0021 section 1's `herdr` row is edited beyond clarification 5's four places** (A warn 2, with the #700
  precedent). **The PR body must name it** as the one ADR edit beyond clarification 5's list.
- **"Decisions taken" item 10 has its title in bold**, matching items 7 to 9. The words are clarification 5's.
- **Scope item 2's "read `label` from `pane.list` / `pane.get`".** As the amended brief settled (clarification 1,
  survey E4), the label is read from the `PaneInfo` of `session.snapshot`, `workspace.create` and `pane.split`. Those
  are replies the adapter already receives. No new method is called.
- Wireframe: N/A (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

All from the worktree, on the code as handed off:

```text
$ cargo test -p holler-pane-testkit --test herdr_conformance_test --test fake_herdr_test
test result: ok. 24 passed; 0 failed   (fake_herdr_test: a_labelled_pane_is_reported_with_its_label,
                                         rename_of_an_unknown_pane_is_pane_not_found ... ok)
test result: ok. 21 passed; 0 failed   (herdr_conformance_test: the four label mutants, a_label_break_fails_no_other_case,
                                         without_a_label_hook_the_label_case_is_not_run,
                                         the_suite_builds_a_fresh_fixture_per_case ... ok)
$ cargo test -p holler-pane-testkit          -> every target ok
$ cargo test -p holler-adapter-herdr
  adapter_conformance_test 4 passed; adapter_test 22 passed; protocol_test 37 passed;
  scratch_herdr_test 7 passed, 2 ignored (opt-in); every other target ok
$ cargo test -p holler-pane                  -> every target ok (records_test 15 passed)
$ cargo test -p holler-cli --test pane_verbs --test profile_verbs
  pane_verbs: 252 passed; 0 failed  (record_bytes x2, launch_passes_o2_when_only_the_panes_label_changed,
             launch_fails_o2_when_its_pane_is_listed_elsewhere, launch_fails_when_its_herdr_pane_vanishes,
             relaunch_in_place_of_a_labelled_pane_records_no_label, labels_change_nothing_doctor_reports ... ok)
  profile_verbs: 44 passed; 0 failed
$ cargo clippy --workspace --all-targets -- -D warnings
  Finished `dev` profile [unoptimized + debuginfo] target(s)   (no warning, no error)
$ bash scripts/lint.sh        -> exit 0 (new warn: crates/holler-adapter-herdr/src/protocol.rs is 636 lines; fail is 900)
$ bash scripts/changelog-check.sh   -> changelog-check: ok
$ rustfmt --edition 2021 --check <the 8 touched .rs files>   -> clean
```

The full suite is `test.unit.command`, the CI workspace test:

```text
$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
  148 test targets: 2068 passed; 0 failed; 25 ignored (the opt-in suites); exit 0
```

After that run I made one doc-only edit, the `HerdrPort` trait docs in `ports.rs`. Re-checked after it:

```text
$ cargo clippy --workspace --all-targets -- -D warnings            -> Finished, clean
$ cargo test -p holler-pane -p holler-pane-testkit -p holler-adapter-herdr
  40 targets: 432 passed; 0 failed; 2 ignored
```

## Evidence appendix

`docs/handoffs/808/evidence.md` has five facts, each with `file:line` and a verbatim excerpt:

1. `run_cases` drops the subject before the guard.
2. The fake's `place` returns the id of the pane at the cell.
3. `workspace_mut` matches the spec's session and workspace.
4. `rename`'s `pane-not-found` comes through `pane_mut`.
5. O2 and the record both use the `ensure_pane` answer.

## Tests that look wrong (for T)

None.

## Known issues

- `crates/holler-adapter-herdr/src/protocol.rs` crossed lint's 600-line warning (595 → 636; the fail is 900).
  `label_of` must live in `protocol` (T's tests import `holler_adapter_herdr::protocol::label_of`, E10 item 2).
  `tx_launch.rs` is 798 lines, under A's ~850 ceiling.
- Merging `main` will need care:
  - #802 also edits ADR-0021 ("Decisions taken" item 10) and `CHANGELOG.md`. If it lands first, renumber this item
    to 11 when merging `main` in before the squash (A's merge note).
  - `origin/main` has also moved one commit (#814, a wizard fix) past this branch's base.
- The workspace is not rustfmt-clean in crates this story does not touch (`cargo fmt --all -- --check` lists
  `holler-body`, `holler-hub`, `holler-cli` and others). That is pre-existing, and CI does not run `cargo fmt`. I
  formatted only the files I touched.

## Files changed

Production (F's):

- `crates/holler-pane/src/pane.rs`
- `crates/holler-pane/src/ports.rs` (docs only)
- `crates/holler-pane/src/tx_launch.rs`
- `crates/holler-adapter-herdr/src/protocol.rs`
- `crates/holler-adapter-herdr/src/adapter.rs`
- `crates/holler-pane-testkit/src/herdr.rs`
- `crates/holler-pane-testkit/src/conformance/herdr.rs`
- `crates/holler-pane-testkit/src/fixture.rs`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Run artifacts: `docs/handoffs/808/handoff-F.md`, `docs/handoffs/808/evidence.md`, and the appended
`docs/handoffs/808/decisions.md`. No test file was created, edited or deleted.
