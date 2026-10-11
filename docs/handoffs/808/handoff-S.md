# Handoff-S: Phase 10 - #808 a Herdr snapshot pane carries its label  (spec audit)

**Date:** 2026-10-11 (06:30 MDT)
**Branch:** `issue-808-implementation` (worktree `.claude/worktrees/0808-herdr-label`, head `38ef0d5`, base `main` at `92fbc6e`).
`origin/main` is now at `64519b5`, two wizard commits (#814, #815) ahead. They touch no file this branch changes.
**Issue:** #808 (epic #633). I read it live with `gh issue view 808`:
- Its body equals the brief's copy (brief lines 3-60), apart from one trailing blank line.
- Its last edit (2026-10-11 04:20 MDT, `userContentEdits`) predates the commit that populated the brief, `04e53da` (04:26 MDT).
- It has no comments.

So the brief carries the issue as it stands.
**Brief:** `docs/handoffs/808-brief.md` (clarifications 1-7 and survey E1-E15). The outside brief gate passed in round 3.
**Handoffs reviewed:**
- `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` and `handoff-A-dup.md`;
- `decisions.md` and `evidence.md`;
- the outside diff review `808-diff-result-r1.md` (PASS) and the brief reviews r1-r3 (BLOCK, BLOCK, PASS). All four ran
  on `deepseek-v4-pro`, per their `.usage.json` files.

**Verdict:** PASS

## A precondition

Met:
- `handoff-A.md` returned PASS on the plan (0 `block`, 7 `warn`).
- `handoff-A-dup.md` returned PASS on the diff (0 `block`, 2 `warn`).

## T precondition

Met. `handoff-T-green.md` reports "Blocking issues: None".

- **RED (T-red, `bc96f89`):**
  - `records_test` is RED at runtime (`unknown field label`).
  - Six targets are RED at compile time, at the new API, which the brief's RED note allows for a contract amendment.
  - A three-stage throwaway stub then showed every behaviour test failing on its assertion.
  - The three guards (tests 2, 20 and 23) are named as guards, and each was shown failing against its mutant.
- **GREEN (T-green, `797b5bb`):**
  - The full `test.unit.command` gives 148 targets: 2068 passed, 0 failed, 25 ignored.
  - Four mutants of F's code are each caught.
  - Tier 1 is clean: clippy with `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete` and `rustfmt --check`
    on the touched files.
- **Phase boundaries, which I checked:**
  - T-red touched no `src/` (`git diff --stat 0b3e67b bc96f89 -- 'crates/*/src/**'` is empty).
  - F touched no test (`git diff --stat bc96f89 db59fc8 -- 'crates/*/tests/**'` is empty).
  - After F (`db59fc8..HEAD`), only four files under `docs/handoffs/808/` changed.

## Acceptance criteria

Every test path is under `crates/`. "Mutant" means a mutation that T applied and that the test caught (T-red or T-green).

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Issue AC 1: a snapshot entry reports `Some("x")` for a pane labelled `x`, and `None` for no label and for `""`, in an adapter test against the wire fake and in a fake test | Adapter: `holler-adapter-herdr/tests/adapter_test.rs:668` `snapshot_reports_each_placed_panes_own_label`. Labels `x`, `""`, `y` and none give `Some(x)`, `None`, `Some(y)`, `None`. A labelled workspace is listed first, so a join by index fails; the labelled pane of a second tab is not listed; the requests are exactly one `session.snapshot` and one `layout.export` per workspace. Fake: `holler-pane-testkit/tests/fake_herdr_test.rs:478` `a_labelled_pane_is_reported_with_its_label`. It labels `x`, `""` and `" "`. `ensure_pane` equals the snapshot entry, a rename replaces the label, and a new pane in a closed pane's cell has none | Met |
| 2 | Issue AC 2: a `panes.json` written before the change reads, and a `launch` record serializes to the same bytes, tested over a stored fixture | `holler-cli/tests/pane_verbs/launch/record_bytes.rs:44` `a_launch_writes_the_record_bytes_it_wrote_before_808`. It compares the hard-coded `PRE_808_RECORD`, made on `92fbc6e` (the file names the commit), byte for byte. `:54` `the_pre_808_record_and_a_labelled_one_both_read_and_round_trip`. `holler-pane/tests/records_test.rs:231` `herdr_label_round_trips_set_and_absent`. The diff changes no other serialized field of `Pane`, so the literal is still the base's output | Met |
| 3 | Issue AC 3: doctor and reconcile output is identical with every Herdr pane labelled and with none | `holler-cli/tests/pane_verbs/doctor.rs:512` `labels_change_nothing_doctor_reports`. It compares the `Report`s of a read-only and a `--fix` pass at the fixed `NOW`, the records they leave, and `pane doctor`'s JSON and text. A mutant that compares whole `HerdrPane`s in reconcile fails it | Met (advisory 1) |
| 4 | Issue AC 4: the conformance case passes for `FakeHerdr` and the wire fake | `holler-pane-testkit/tests/herdr_conformance_test.rs:83` and `:89` cover the fake and its split-only placement. `holler-adapter-herdr/tests/adapter_conformance_test.rs:90`, `:95` and `:100` cover the adapter over `WireHerdr`: empty, with a root pane, and over a real Unix socket. Each asserts `Ok(())`. The case's power: `herdr_conformance_test.rs:578`-`:601`, four label mutants that each fail the label case and only it. In T-green, an adapter `label_of` that keeps `""` and a snapshot that drops the join each fail all three adapter runs | Met |
| 5 | Issue AC 5: the ADR-0021 edits, and a `CHANGELOG.md` `## [Unreleased]` entry linking the issue | In the diff: clarification 5's four places, word for word, plus section 1's `herdr` row (see Spec compliance). The `CHANGELOG.md` Enhancements entry links #808, #650 and #633, and `changelog-check: ok` (T-green) | Met |
| 6 | E9: a relaunch in place of a labelled pane writes `herdr.label: None`, with no `label` key | `holler-cli/tests/pane_verbs/relaunch.rs:169` `relaunch_in_place_of_a_labelled_pane_records_no_label`. Mutant: `Plan::record` keeps the label | Met |
| 7 | E9 and E11 test 1: a launch passes O2 when only the label changed | `holler-cli/tests/pane_verbs/launch.rs:593` `launch_passes_o2_when_only_the_panes_label_changed`. Mutant: an O2 that also requires whole-value equality. T-red stage 2 failed it against the base's `contains` | Met |
| 8 | E11 test 2: a vanished pane fails O2 | `launch.rs:230` `launch_fails_when_its_herdr_pane_vanishes` (AC 5a, unchanged) | Met |
| 9 | E11 test 3: the pane id listed at another grid, workspace or session fails O2 | `launch.rs:620` `launch_fails_o2_when_its_pane_is_listed_elsewhere`, through `launch/displaced.rs`. Mutant: an O2 that compares only the join key passes the grid case, so the test fails | Met |
| 10 | E7: the round trip | `record_bytes.rs:54`, which round-trips the pre-#808 literal byte for byte and the labelled record by value | Met |
| 11 | E6 and round 1's NIT-2: the conformance case's empty-label pane | The case at `holler-pane-testkit/src/conformance/herdr.rs:423` labels pane `b` `""` and expects `None`. The `KeepsEmptyLabel` mutant fails it | Met |
| 12 | E10 item 1: the parsers keep each label as sent | `holler-adapter-herdr/tests/protocol_test.rs:509`, `:667` and `:692`: `"x"` and `""` are kept as sent, a non-string or `null` label is `None`, and an absent one is `None` (the updated `ac18_*` and `ac20_*` tests) | Met |
| 13 | Clarification 2: `label_of`'s direct test, with `" "`, and one test per caller | `protocol_test.rs:712` covers `None`, `""`, `"x"`, `" "` and `" hj-c1r2 "`. Per caller: in `snapshot()`, the `""` pane at `adapter_test.rs:668` reports `None`; in `ensure_pane`, the `""` pane at `adapter_test.rs:711` does | Met |
| 14 | E10 item 3: a made pane's answer carries the label of the reply that made it | `adapter_test.rs:749` covers both `workspace.create`'s `root_pane` and `pane.split`'s `pane` | Met |
| 15 | E10 item 4: a pane made between the two reads is listed with `None`, the call succeeds, and nothing is read again | `adapter_test.rs:779` | Met |
| 16 | E12: every caller is updated; the fake, its wrappers and the adapter's three runs pass a hook; the scratch run passes `None` | A grep finds all nine pre-existing callers updated; `scratch_herdr_test.rs:215` passes `None`. `herdr_conformance_test.rs:170` finds 12 fixtures built and 12 guards dropped. `:621` shows that with `None` the case is skipped and still listed | Met |
| 17 | Clarification 1 and E10 item 5: no new method and no new request | The method table in `protocol.rs` is unchanged (`METHOD_COUNT` is 9). `adapter_test.rs:668` and `:711` assert the exact requests sent. The allow-list test is unchanged and passes | Met |

## Spec compliance

Every decision in the issue and the brief is implemented as stated:

- **The operator's decision (the label is the join key).** It is recorded in ADR-0021's section 2 sentence, its section 10
  bullet and "Decisions taken" item 10.
- **Scope 1.**
  - The field is `label: Option<String>`, the last field of `HerdrPane`, with
    `#[serde(default, skip_serializing_if = "Option::is_none")]` (`holler-pane/src/pane.rs:106-107`).
  - **(a)** The persisted form is unchanged: see rows 2 and 10.
  - **(b) Reconcile and doctor still ignore the label.**
    - Reconcile keys panes by `(session, pane_id)`, and findings copy only the id and the cell (`evidence.md`, T's
      entries).
    - O2 now uses `same_place` (`tx_launch.rs:690`, helper at `:762`).
    - A grep finds no other whole-value `HerdrPane` comparison in `crates/*/src`. The one `==` left,
      `reconcile/observe.rs:372`, compares a record with its own clone (E5 row 3).
  - **(c) Launch and relaunch write no label.** `Plan::record` writes `HerdrPane { label: None, ..live.herdr }`
    (`tx_launch.rs:327-331`), exactly E14's form.
- **Scope 2.** The adapter reads `label` from the `PaneInfo` of `session.snapshot`, of `workspace.create`'s `root_pane`
  and of `pane.split`'s `pane`, not from `pane.list` or `pane.get`. Clarification 1 allowed this and survey E4 settled it.
  The outside reviewer's NV-1 asked whether `session.snapshot`'s top-level `panes` lists the grid tab's placed panes. The
  spike's verified probe settles it: `scripts/spikes/herdr-grid.sh:28` reads each grid pane's label from
  `snap["panes"]`, and its output lists them (`docs/research/herdr-api-spike.md:183-191`).
- **Scope 3.**
  - `FakeHerdr::rename` (`holler-pane-testkit/src/herdr.rs:246`) bypasses the faults and the call log, like `vanish` and
    `print`.
  - Case 12, `labelled-pane-reports-its-label`, is last in the one `CASES` table.
- **Scopes 4 and 5.** The ADR edits are covered under clarification 5 below. The pane with no cell is stated in the
  section 10 bullet and the "Deferred" bullet as the importer's finding, never recorded.
- **Clarification 2.**
  - The signature is exactly `pub fn label_of(raw: Option<&str>) -> Option<String>` (`protocol.rs:480`).
  - Every `HerdrPane` the adapter reports is built in `HerdrAdapter::pane` (`adapter.rs:254`), which holds the only call
    of `label_of`.
  - The fake's private copy (`herdr.rs:546`) and the adapter's doc name each other, following the `last_lines`
    precedent (A warn 4).
- **Clarification 3.** `WireHerdr` keeps a `labels` map, and `rename` sets it. `pane_json` sends the label in every
  `PaneInfo`, `""` included, and leaves the key out when a pane has none. The hook is a closure parameter of the runner.
- **Clarification 4.** The literal is hard-coded and names its commit, and a second test reads it with and without
  `"label":"x"`.
- **Clarification 5.** The four places are written as given:
  - section 2's sentences, right after the "provisional" note;
  - the section 10 bullet;
  - the "Deferred" `HerdrPort` bullet, whose clause is replaced;
  - "Decisions taken" item 10. Its title is bold, like items 7-9, and its words are unchanged.
- **Clarification 6.** The label is a field on `HerdrPane`, and the survey's E5 table holds for the merged code.
- **E10 items 1-5, E11, E12, E13 and E14** are implemented as pinned. E13's doc comments are at `pane.rs:91-93` and
  `:101-105`.

**Deviations, each documented by F and none silent:**

1. **E10 item 3** said "the two `HerdrPane` literals call `label_of`". Now one constructor serves both callers. That is
   a stricter form of clarification 2's "the two can never disagree". Accepted.
2. **Section 1's `herdr` row is edited**, though clarification 5's list does not name it. Scope item 4 does name section 1
   ("sections 1 and 2 (the `HerdrPane`/`HerdrSnapshot` types ...)"), and A's warn 2 asked for the edit, with #700 as
   precedent. Without it, the ratified records table would misstate the merged type. I read clarification 5's "exactly
   these places" as governing the prose it writes out, so the row is within scope item 4 and is not a deviation in
   substance. The PR body must name it (advisory 4).
3. **E12's signature is written through `pub type LabelHook`.** It is the identical type, and the alias is needed because
   clippy's `type_complexity` fails CI's `-D warnings` on the spelled-out form.

## Quality audit

- **Correctness and failure handling.**
  - A label never fails a reply. `sent_label` treats an absent or non-string label as `None`, as the brief pins in E10
    item 1. `pane_label` returns `None` for a pane made after the snapshot read.
  - `FakeHerdr::ensure_pane` looks up the id it just placed. If the id were somehow missing, the answer would be
    `pane-not-found`, never a panic.
  - The record format fails closed as before: `deny_unknown_fields` stays, and old records read through
    `#[serde(default)]`.
  - No new shared production state is added. The fake keeps its label under its existing mutex.
  - The label is untrusted, Herdr-sent text, and no message or log carries it. A grep finds no production message that
    Debug-formats a `HerdrPane`.
- **Build guards.**
  - No `unwrap`, `expect` or `panic` is added in `src/`. The one grep hit is inside a ```` ```text ```` doc example,
    which already had the same shape.
  - No `#[allow]` is added.
  - Every touched file is under 900 lines. The largest are `adapter_test.rs` (810), `tx_launch.rs` (798),
    `protocol_test.rs` (750), `launch/rig.rs` (739) and `protocol.rs` (636, newly past the 600-line warning).
  - There is no dead code: clippy with `-D warnings` is clean (T-green).
- **Protocol.** The hub and body wire is unchanged:
  - `holler-proto`, `holler-hub`, `holler-body` and `holler-cli/src` are untouched;
  - no golden file changed, and neither did `docs/protocol/v2.md`;
  - no error code was added, and the Herdr allow-list is unchanged.
- **Tests.**
  - The adapter's conformance run over a real Unix socket covers the cross-process path.
  - No fixed sleep was added, and the doctor guard uses the rig's fixed `NOW`.
  - Mutation checks cover both F's code (T-green) and the guards (T-red), and RED came first.
- **Documentation.**
  - The `CHANGELOG.md` entry's claims hold. In particular, "Nothing a user runs changes yet: #649 wires the adapter in"
    is true: the CLI's `HerdrPort` is still the `Unwired` stub in `holler-cli/src/pane/wiring.rs`, and #649 is the
    wiring story.
  - There is no new log event or CLI surface, and the ADR is updated.
  - There are two documentation nits (advisories 2 and 3).
- **Public-repository privacy.** I grepped all 3,933 added lines, code and the committed handoffs, for home paths, user
  and account names, hostnames, tailnet names, IPs and secrets, and found none.
  - The literal's version string is the test kit's own `PROTOCOL_22_VERSION`, and its `cwd` is `/srv/demo`.
- **Commit and PR hygiene.**
  - The branch has 8 commits with Conventional subjects (`chore(#808): ...`). Each has
    `Co-Authored-By: Claude <noreply@anthropic.com>`, the form the repo's `prepare-commit-msg` hook adds.
  - None carries a session link. That matches every recent commit on `main` (#813, #814, #815), so it is not a finding
    for this story.
  - The PR is not open yet, so its AI disclosure cannot be checked (advisory 4).
- **The outside diff review (round 1, PASS), checked in source:**
  - NV-1 is settled above.
  - NV-2 is the exception the brief pins.
  - NV-3 and NV-5 are facts about unchanged code, and the passing tests prove them: `adapter_test.rs:668` asserts that
    the second tab's pane is not listed, and renames the pane `add_tab` made without an error.
  - NV-4 and W-1: no whole-value comparison remains (my grep, and A-dup's).
  - W-2 is A's warn 7, a follow-up for #650 (advisory 5).

## Scope check

F delivered exactly the brief's scope, with the additions of E9 and E15:

- Every touched path is in the blast radius, or is `tx_launch.rs`, which E9 added.
- Three small extractions serve clarification 2's "never disagree", and F explained each: `HerdrAdapter::pane`,
  `WorkspaceGrid::occupant` and `State::listed`.
- The `ports.rs` change is to the trait docs only, and belongs to the contract amendment.
- There are no unrelated refactors, and nothing is under-delivered.

## Verdict

**PASS.**
- Every acceptance criterion, those of the issue and those of E9 and E15, has a test that asserts the behavior, and
  the guards are shown to fail against their mutants.
- Every decision in the brief is implemented, and each deviation is documented and justified.
- The quality gates are clean.

The PR may be opened, with the advisories below handled by the run's agent where they say so.

## Advisory notes (non-blocking)

1. **The doctor guard's labels are the pane ids** (`holler-cli/tests/pane_verbs/doctor.rs:529` labels each pane
   `pane.pane_id.as_str()`). This is test-only, a one-line change.
   - The guard catches every regression that compares labels, such as T-red's whole-value mutant.
   - It cannot catch a change that shows or keys a pane by "its label if it has one, otherwise its id". With every
     label equal to its id, both worlds give identical output.
   - The setup wizard labels panes with their names, which are never their ids.
   - **Fix:** label each pane with a value other than its id, for example its record's name or
     `format!("label-{}", id)`. Worth doing before #650 adds label-based joins.
2. **`docs/testing.md:441`** says the opt-in scratch test "runs the whole `HerdrPort` conformance suite". It now runs
   every case but the label case, which needs a hook that run does not pass. Suggested wording: "runs the `HerdrPort`
   conformance suite (every case but the label case, which needs a hook to label a pane)".
3. **`protocol.rs:18`** (the Parsers bullet) and **`:465`** (`sent_label`'s doc) call the label "the one exception" to
   the parser rule. `parse_pong` (`:310-322`) already reads `protocol` the same way: absent or of the wrong type is
   `None`, which the version gate then refuses. Name both optional reads, or drop "one".
4. **The PR** (carried forward from F, T-green and A-dup):
   - The pipeline's PR body has no AI disclosure, which `CONTRIBUTING.md` requires. Add it with `gh pr edit` once the
     script opens the PR, as `CLAUDE.md` says.
   - The body must name section 1's `herdr` row as the one ADR-0021 edit beyond clarification 5's list.
5. **#650 follow-up** (A's warn 7 and A-dup's warn 1). #650 has no comment yet, so this note is still to be posted. In
   #650's own PR:
   - move the label clearing (`tx_launch.rs:327-331`) and `same_pane` and `same_place` (`:756`, `:762`) to one place in
     `holler-pane`, for example methods on `HerdrPane`;
   - note that `HerdrPane`'s derived `==` includes the label (the outside reviewer's W-2).
6. **Merge order.** #802 is OPEN and also edits ADR-0021's "Decisions taken" and `CHANGELOG.md`. If it merges first,
   renumber this item 10 when `main` is merged in. Today `main` is two wizard commits ahead, with no file overlap.
7. **An unrelated flake** (from T-green): `holler-cli/tests/remote_admin_test.rs:365`
   `remote_admin_traffic_never_moves_the_bodys_last_seen` synchronizes with a fixed 2 s sleep. It is worth an issue
   outside #808.
