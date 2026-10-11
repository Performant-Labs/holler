# Handoff-T-red: Phase 4 - #808 a Herdr snapshot pane carries its label

**Date:** 2026-10-11
**Branch:** `issue-808-implementation` (base `main` at `92fbc6e4186acc717f4aa2b6465c55a266b9043c`; the branch's
`crates/` and `Cargo.*` are identical to the base, checked with `git diff --stat 92fbc6e HEAD -- crates Cargo.toml Cargo.lock`)
**Brief / wireframe reviewed:** `docs/handoffs/808-brief.md` (clarifications 1-7, survey E1-E15) and `docs/handoffs/808/handoff-A.md`.
Wireframe: N/A (no UI surface).

## A precondition

Confirmed: A returned **PASS** on the plan (0 block, 7 warn). The tests below also enforce A's warn 1: `ensure_pane` of a
labelled cell must answer the snapshot's entry, label included, in the fake and in the conformance case. A's warns 2-7 are
F's.

## The API these tests pin (F implements it; nothing here is in `src/`)

T stays inside `paths.test` (`crates/*/tests/**`). The conformance case itself, `FakeHerdr`'s hook and the runner's new
parameter are in `crates/holler-pane-testkit/src/`, so they are F's. The tests pin their names and shapes:

| API | Shape the tests use | Source |
|---|---|---|
| `HerdrPane.label` | `pub label: Option<String>`, `#[serde(default, skip_serializing_if = "Option::is_none")]` | scope 1, constraint (a) |
| `FakeHerdr::rename` | `pub fn rename(&self, pane: &PaneId, label: &str) -> Result<(), PaneError>`. It is an outside-world hook like `vanish`/`print`: it bypasses faults and the call log, and is `pane-not-found` for an unknown pane. The fake stores the label as given, reports `""` as `None`, and both `snapshot` and `ensure_pane` report it. | scope 3, A warn 1 and 4 |
| `run_herdr_conformance` | `(fresh: F, label: Option<&dyn Fn(&HerdrFixture<H>, &K, &PaneId, &str) -> Result<(), PaneError>>)` | E12 (verbatim) |
| the conformance case | id **`labelled-pane-reports-its-label`**, **last** in `CASES` (12 cases). It is not run when `label` is `None`, and `herdr_cases()` still lists it. | scope 3, clar. 3, NIT-2, A warn 5 |
| `protocol::label_of` | `pub fn label_of(raw: Option<&str>) -> Option<String>` | clarification 2 |
| `PaneRef.label` | `pub label: Option<String>`, the raw label as sent (absent or non-string becomes `None`) | E10 item 1 |
| `parse_pane_info` | `-> Result<(PaneId, Option<String>), PaneError>` (raw label) | E10 item 1 |
| `parse_workspace_created` | `-> Result<(WorkspaceRef, PaneId, Option<String>), PaneError>` (root pane's raw label) | E10 item 1 |

**What the conformance case must do.** The testkit mutants pin this from outside. Ensure `r1c1` (a) and `r2c1` (b). Both
are `None` before any label. Then the hook labels a `"x"` and b `""`. The snapshot then reports a `Some("x")` and b `None`.
`ensure_pane(r1c1)` and `ensure_pane(r2c1)` each answer that pane's snapshot entry as a whole value. Four mutants must each
fail this case, and only this case:

- one that forgets labels;
- one whose `ensure_pane` alone forgets the label;
- one that reports `Some("")`;
- one that invents a label for an unlabelled pane.

## Tests authored

Tiers: unit (protocol parsers, serde), fake-level (`FakeHerdr`), adapter over the in-process wire fake, conformance
(run against both fakes, plus mutants), and verb/engine over the CLI rigs. Each test sits at the cheapest tier where the
behaviour is observable.

| # | Test | Pins | Tier, why |
|---|---|---|---|
| 1 | `crates/holler-pane/tests/records_test.rs:231` `herdr_label_round_trips_set_and_absent` | (a): a record and a snapshot entry with `herdr.label: "x"` read and write back; a record without one writes back unchanged, with no `label` key | serde unit; JSON-driven, so it compiles today (runtime RED) |
| 2 | `crates/holler-cli/tests/pane_verbs/launch/record_bytes.rs:44` `a_launch_writes_the_record_bytes_it_wrote_before_808` | clar. 4, part 1: `launch` (engine, constant clock) stores the exact bytes of the hard-coded `PRE_808_RECORD` literal, made on `92fbc6e` (commit named in the file) | engine over the launch rig. **Guard**: it holds on the base by design (see RED notes) |
| 3 | `record_bytes.rs:54` `the_pre_808_record_and_a_labelled_one_both_read_and_round_trip` | clar. 4, part 2 and E7: the literal reads with `label: None` and round-trips byte for byte; with `"label":"x"` added it reads as `Some("x")`, differs only in the label, and round-trips | same file as the literal (A warn 6) |
| 4 | `crates/holler-pane-testkit/tests/fake_herdr_test.rs:478` `a_labelled_pane_is_reported_with_its_label` | acceptance 1 (the fake): `x` gives `Some("x")`; `""` gives `None`; `" "` gives `Some(" ")` (not trimmed). `ensure_pane` of the cell equals its snapshot entry; a rename replaces; a new pane in a closed pane's cell has no label; `rename` is not a port call | fake unit |
| 5 | `fake_herdr_test.rs:523` `rename_of_an_unknown_pane_is_pane_not_found` | the hook's error contract (E12) | fake unit |
| 6 | `crates/holler-pane-testkit/tests/herdr_conformance_test.rs` `the_suite_runs_the_documented_cases` (extended: `DOCUMENTED_CASES` adds `LABEL_CASE`, line 29) and `the_suite_builds_a_fresh_fixture_per_case` (now 12 fixtures and 12 guards) | the case exists, last; one fixture and one guard per case, with the hook passed | conformance meta |
| 7 | `herdr_conformance_test.rs:578`, `:585`, `:590`, `:595` (four `a_herdr_that_..._fails_the_label_case`) and `:601` `a_label_break_fails_no_other_case` | the case catches each label defect (forgets; `ensure_pane` forgets, which is A warn 1; keeps `""`; invents), and only the label case fails | mutation check, the suite's established pattern |
| 8 | `herdr_conformance_test.rs:621` `without_a_label_hook_the_label_case_is_not_run` | with `None` the case is skipped (a label-forgetting Herdr passes) and is still listed | conformance meta |
| 9 | every existing conformance runner passes the hook: the fake, split-only, `GuardWatcher`, `Mutant`, unsupported version | E12: "every caller is updated" | (call-site updates) |
| 10 | `crates/holler-adapter-herdr/tests/adapter_conformance_test.rs:45` `run_in_process` and `:70` `run_over_a_socket` | the adapter's three runs over `WireHerdr` run the label case. The guard carries the case's `Arc<WireHerdr>` (over the socket, beside `Served`) | conformance against the real adapter |
| 11 | `crates/holler-adapter-herdr/tests/scratch_herdr_test.rs` (opt-in, ignored) | passes `None` | E12 |
| 12 | `crates/holler-adapter-herdr/tests/protocol_test.rs:509` `a_snapshot_pane_carries_its_label_as_sent` | E10 item 1: `"x"`→`Some("x")`, `""`→`Some("")`, non-string→`None`. The existing `ac18_..._each_pane...` test gains `label: None` (absent gives `None`) | parser unit |
| 13 | `protocol_test.rs:667` `a_created_workspaces_root_pane_carries_its_label`; `:692` `pane_info_carries_the_panes_label`; the existing `ac20_*` tests use the new tuple shapes (no label gives `None`) | E10 item 1 for both reply parsers, raw label as sent (`null` gives `None`) | parser unit |
| 14 | `protocol_test.rs:712` `label_of_folds_no_label_and_the_empty_string_to_none` | clar. 2: `None` and `""` give `None`; `"x"`, `" "` and `" hj-c1r2 "` are kept as they are | unit |
| 15 | `crates/holler-adapter-herdr/tests/adapter_test.rs:668` `snapshot_reports_each_placed_panes_own_label` | E10, the join: four placed panes labelled `x`, `""`, `y` and none report `Some(x)`, `None`, `Some(y)`, `None`. The other workspace (listed first, labelled `o`) defeats a join by index. The labelled second-tab pane is not listed. Exactly one `session.snapshot` and one `layout.export` per workspace (no new request) | adapter over the wire fake |
| 16 | `adapter_test.rs:711` `ensure_pane_of_an_occupied_cell_answers_its_panes_label` | E10, the occupied cell: `x` gives `Some("x")` and `""` gives `None`, each equal to the snapshot's entry; only the two reads are sent | same |
| 17 | `adapter_test.rs:749` `ensure_pane_of_a_new_pane_answers_the_label_its_reply_carries` | E10 item 3: a made pane's answer carries the `PaneInfo` label of `workspace.create` (`root_pane`) or `pane.split` (`pane`). A `Tap` adds the label to those replies | same |
| 18 | `adapter_test.rs:779` `a_pane_made_between_the_two_reads_is_listed_with_no_label` | E10 item 4: a pane split in (and labelled `late`) between `session.snapshot` and `layout.export` is listed at its cell with `None`; the call succeeds; nothing is read again | same, with a `Tap` hook |
| 19 | `crates/holler-cli/tests/pane_verbs/launch.rs:593` `launch_passes_o2_when_only_the_panes_label_changed` | E11 test 1 (B-2): a rename after `attach_tui` changes only the label; the launch succeeds, the record's label is `None`, and `assert_matches` holds | verb over the launch rig |
| 20 | `launch.rs:620` `launch_fails_o2_when_its_pane_is_listed_elsewhere` | E11 test 3: O2's snapshot lists the pane id at another grid, workspace or session; each is `unavailable`, names `w1:p1`, and records nothing | verb over the launch rig with `DisplacingHerdr` (`launch/displaced.rs`, armed by `Rig::displace_after_attach`, `launch/rig.rs:273`). **Guard** on the base (see RED notes) |
| 21 | AC 5a `launch_fails_when_its_herdr_pane_vanishes` (unchanged) | E11 test 2 | existing |
| 22 | `crates/holler-cli/tests/pane_verbs/relaunch.rs:169` `relaunch_in_place_of_a_labelled_pane_records_no_label` | E9 / (c): relaunching in place a pane Herdr reports as labelled writes `herdr.label: None`, with no `label` key in its JSON; `herdr` is otherwise unchanged; nothing is closed | verb over the launch rig |
| 23 | `crates/holler-cli/tests/pane_verbs/doctor.rs:512` `labels_change_nothing_doctor_reports` | acceptance 3 / (b). Two copies of one world: an unregistered pane, a missing Herdr pane, and a moved TUI. In one copy every Herdr pane is labelled. Both give equal `Report`s (read-only and `--fix`), equal records, and identical `pane doctor` JSON and text. **Guard** (see RED notes) | engine and verb over the doctor rig |

Test support changed (all under `tests/`):

- `wire_herdr/mod.rs`: `WireHerdr::rename` (`:160`) stores a per-pane label. `pane_json` sends it in every `PaneInfo`
  (`session.snapshot`, `workspace.create`, `pane.split`), `""` included, and omits the key when unset. Closing a pane
  drops its label. No method is added to the nine it serves.
- `launch/displaced.rs` (new) and the launch rig. The verb's `HerdrPort` is the rig's `FakeHerdr` behind a pass-through
  wrapper whose `snapshot` reports one armed pane elsewhere. Un-armed it changes nothing: all 249 existing `pane_verbs`
  tests pass through it (stub run below).
- Compile-forced literal: `launch/guards.rs:82` gains `label: None`, and so does `adapter_test.rs`'s `pane()` helper.
  The other `HerdrPane` sites in tests (`guards.rs:51`, `profile_verbs/show.rs:45`) are spreads and need nothing.

## RED confirmation

Every command ran from the worktree on its unchanged base `src/`.

**Runtime RED (compiles today, fails on the assertion the feature satisfies):**

- `cargo test -p holler-pane --test records_test`: 14 passed, 1 failed.
  - `herdr_label_round_trips_set_and_absent` fails with `called Result::unwrap() on an Err value: Error("unknown field
    `label`, expected one of `session`, `workspace`, `pane_id`, `grid`")`.

**Compile RED at the new API** (valid for a contract amendment, per the brief's RED note). Every error names the missing
API, and none is a typo, an import or a setup error:

- `holler-pane-testkit --test fake_herdr_test`: 9 errors.
  - `E0599 no method named rename found for struct FakeHerdr`.
  - `E0609 no field label on type HerdrPane`.
- `holler-pane-testkit --test herdr_conformance_test`: 13 errors.
  - `E0061 this function takes 1 argument but 2 arguments were supplied` (`run_herdr_conformance`).
  - `E0599 rename`, `E0609` / `E0560 label`.
- `holler-adapter-herdr --test protocol_test`: 7 errors.
  - `E0432 unresolved import holler_adapter_herdr::protocol::label_of`.
  - `E0560` / `E0609 label on PaneRef`.
  - `E0308`: expected `(WorkspaceRef, PaneId)` and found a 3-tuple; expected `Result<PaneId, _>` and found
    `Result<(PaneId, Option<_>), _>`.
- `holler-adapter-herdr --test adapter_test`: 2 errors, `E0560 HerdrPane has no field named label`.
- `holler-adapter-herdr --test adapter_conformance_test`: 2 errors; `--test scratch_herdr_test`: 1 error. All are `E0061`
  (the runner's second argument).
- `holler-cli --test pane_verbs`: 12 errors, all `E0560`/`E0609 label` on `HerdrPane` or `E0599 rename` on `FakeHerdr`.
- `holler-cli --test profile_verbs`: 44 passed. It is untouched and still green.

**Behaviour RED, proven before F.** The point is to show that once the API compiles, the behaviour tests fail on
assertions and not on compile errors. I applied a throwaway stub to `src/` (never staged, reverted with `git checkout`,
`src/` clean afterwards).

*Stage 1: API only, no behaviour.* The stub added:

- the `label` field and its serde attributes;
- `label: None` at every production literal;
- `FakeHerdr::rename`, which accepts and forgets;
- a `label_of` that always returns `None`;
- parsers that return `None` labels;
- a runner that takes the hook and ignores it, with no new case.

Results:

- `fake_herdr_test` 23/1. `a_labelled_pane...` gets `left: [("w1:p1", None), ("w1:p2", None), ("w1:p3", None)]` and
  expects `right: [("w1:p1", Some("x")), ("w1:p2", None), ("w1:p3", Some(" "))]`.
- `herdr_conformance_test` 13/8:
  - Each of the four label mutants: `a broken Herdr must not pass the suite: ()`.
  - `the_suite_runs_the_documented_cases`: the 11-case list lacks `labelled-pane-reports-its-label`.
  - Fresh fixture: `left: 11, right: 12`.
  - `without_a_label_hook...`: `herdr_cases().contains(&LABEL_CASE)` fails.
  - `a_label_break_fails_no_other_case`: the broken Herdr passes.
- `protocol_test` 33/4. The label parsers return `None` where `Some("x")` / `Some("")` / `Some("r")` was sent, and
  `label_of(Some("x"))` gives `None`.
- `adapter_test` 18/4. Every snapshot entry and `ensure_pane` answer has `label: None` where `Some("o")`, `Some("x")`,
  `Some("y")`, `Some("root")` and `Some("made")` are expected.
- `pane_verbs` 249/3. The doctor, launch and relaunch label tests fail at their precondition that Herdr reports the
  label. The fake reports none yet.
- `adapter_conformance_test` 4/0 and `records_test` 15/0. The runs pass because the stub runner never runs a label case;
  the stub's serde attributes are the field's declared form.

*Stage 2: the fake now stores and reports labels; the engine is unchanged.*

- `fake_herdr_test` passes, so test 4 is satisfied by a correct fake.
- `pane_verbs` 250/2, with both failures for the right reason:
  - `launch_passes_o2_when_only_the_panes_label_changed` fails with `unavailable: the Herdr pane "w1:p1" of demo-c1r1 is
    gone from r2c1 in workspace "main"`. That is today's whole-value O2 (`tx_launch.rs:685`).
  - `relaunch_in_place_of_a_labelled_pane_records_no_label` fails with `the record carries no label: left:
    Some("demo-c1r1") right: None`. That is today's `Plan::record` (`tx_launch.rs:327`).

*Stage 3: four-field O2 and `herdr: HerdrPane { label: None, ..live.herdr }`.*

- `pane_verbs` 252/0. The CLI tests are satisfiable as written.

**Guards checked with mutants.** These tests hold on the base by design and fail if F's change breaks the constraint:

- Test 2: dropping `skip_serializing_if` (so `"label":null` is written) fails tests 2 and 3, and test 1.
- Test 20: an O2 that compares only `(session, pane_id)` succeeds the `Grid` case (`code: 0`). The test fails.
- Test 23: a reconcile check that compares whole `HerdrPane`s makes the labelled world's `Report` differ (`fix false`).
  The test fails.

## RED notes

- **Three tests are guards, not REDs, by the brief's own terms:** tests 2, 20 and 23 (constraints (a), (b) and the
  `grid`/`workspace` half of E11 test 3).
  - Each holds on the base's logic once it compiles, because each pins behaviour that must not change.
  - Their RED today is compile-only (they sit in the `pane_verbs` target, which cannot compile before the field exists).
  - Each was shown above to fail against the defect it guards.
- **Order of a labelled record's keys is not pinned.** Test 3 compares a labelled record by JSON value, not by bytes,
  so F may put `label` anywhere in `HerdrPane`. Only the unlabelled form is byte-pinned (clarification 4).
- **The made-pane test (17) goes slightly past E10's test list.** It pins E10 item 3's split and create branches, which no
  listed test reaches. Without it, F could answer `None` for a made pane and every other test would still pass.
- **The `" hj-c1r2 "` case in test 14** pins "no trimming" with a realistic pane name.

## Ready for F

Confirmed: the RED is valid. Every new test fails to compile at the new API or fails on its assertion, and against an
API-only stub each behaviour test fails on the assertion the feature satisfies. The three guards are named, and each
failed against its mutant. Every existing outcome is unchanged: `profile_verbs` 44/44 today, and all 249 existing
`pane_verbs` tests pass through the new rig wrapper under the stub.

F implements, in `src/`:

- the field;
- `FakeHerdr::rename` and its private label rule (A warn 4);
- one `HerdrPane` constructor for `ensure_pane` and `listed` (A warn 1);
- the conformance case and runner parameter (A warn 5 docs);
- `label_of` and the parser label reads through one helper (A warn 3);
- the two adapter literals through `label_of`;
- O2's four-field comparison and `Plan::record`'s `label: None`;
- ADR-0021 (A warn 2) and `CHANGELOG.md`.

`cargo fmt` reports nothing for the touched test files. `scripts/lint.sh` exits 0: `adapter_test.rs` (810 lines),
`protocol_test.rs` (750) and `wire_herdr/mod.rs` (692) are past the 600-line warning and under 900. No `#[allow]` was
added. The test files are staged by explicit path; nothing is committed.

T-red complete, RED is valid. F may implement against the authored tests.
