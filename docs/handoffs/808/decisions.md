# Decision journal: #808, a Herdr snapshot pane carries its label

Run handoff directory `docs/handoffs/808` · branch `issue-808-implementation` · rigor second-opinion · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry. The brief gate's rounds are in
`docs/handoffs/808-brief-result-r{1,2,3}.md` (round 1 BLOCK, round 2 BLOCK, round 3 PASS).

## A, up-front plan review: 2026-10-11T05:26-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 7 `warn` findings (`docs/handoffs/808/handoff-A.md`). The plan extends the
  existing objects and builds no parallel path:
  - the label is a field on `HerdrPane`, in the established optional-field serde form;
  - the adapter reads `PaneInfo` from replies it already receives (no new method, so I5 and the allow-list are unchanged);
  - the fakes' label hooks are shaped like their existing outside-world hooks;
  - O2 becomes a four-field comparison beside `same_pane`;
  - E11's Herdr double follows the launch rig's own per-run port wrappers.

  Clarification 6's choice of a field on `HerdrPane` stands: O2 is the only whole-value comparison in production code.

  The warns are:
  1. `FakeHerdr`'s `ensure_pane` must report the label as its snapshot does, checked by the conformance case.
     Otherwise E9's relaunch test cannot fail.
  2. ADR-0021 section 1's `herdr` row needs `label?`. Scope item 4 names section 1, clarification 5's list omits it,
     and #700 is the precedent.
  3. Read the label in `protocol.rs` through one helper whose doc states the exception to the parser rule.
  4. `FakeHerdr`'s copy of the `""` mapping is documented as `last_lines`'s copy is.
  5. Document the label case's hook as an exception to the suites' policy on events the port cannot cause, and keep
     the one `CASES` table.
  6. File sizes: `tx_launch.rs` 787 and `launch/rig.rs` 718 lines. The record serde tests go to `records_test.rs`.
  7. "A record carries no label" is held only at `Plan::record`. A follow-up for #650.
- **Assumed:** T reads `handoff-A.md` before authoring RED (the run context names it). There is no O in this
  automated path to fold the warns into the brief, so findings 1 and 5 reach F through T's tests and S's audit.
  #802 has not merged, so "Decisions taken" item 10 is still free (checked `gh issue view 802`: OPEN).
- **Hedged:** finding 2 conflicts with the letter of clarification 5 ("the PR edits exactly these places"). I rated it
  `warn` with a concrete one-cell fix rather than `block`: the edit adds no prose to the four pinned places and only
  keeps a ratified table true. If the operator holds clarification 5 literally, section 1 stays stale and the PR body
  should say so.
- **Evidence:**
  - Read: the brief (E1-E15) and the gate results r2 and r3.
  - ADR-0021: sections 1, 2 and 10, "Deferred" and "Decisions taken"; commits `6d19f91` and `c6485b0`.
  - Code: `pane.rs`, `ports.rs` and `tx_launch.rs` (lines 1-40, 660-700, 740-787); the adapter's `adapter.rs` and
    `protocol.rs`; the test kit's `herdr.rs` and its conformance `herdr.rs`, `harness.rs` and `mod.rs`;
    `herdr_conformance_test.rs`; `wire_herdr/mod.rs` and `serve.rs`; `adapter_conformance_test.rs`; `launch/rig.rs`;
    `records_test.rs`; `get.rs`; the hub's `store.rs`.
  - Greps: every `impl HerdrPort` and `HerdrPane {` literal; whole-value comparisons in `holler-pane/src`; `herdr`
    writes in `crates/*/src`; `herdr` in `holler-hub/src` (none); `HerdrPane` in `holler-proto/src` (none); golden
    files and `v2.md` (none).
  - `wc -l` over every touched file; `scripts/lint.sh` limits (600 warn, 900 fail).

## T, RED (Phase 4): 2026-10-11T05:40-06:00 (MDT)

- **Decided:** RED is valid (`docs/handoffs/808/handoff-T-red.md`).
  - 23 rows of tests over 13 test files plus two new test modules (`launch/record_bytes.rs`, `launch/displaced.rs`), all
    under `crates/*/tests/**`.
  - One target is runtime-RED: `records_test`, failing with unknown field `label`. Six targets are compile-RED at the
    new API.
  - API names the brief left open, pinned for F:
    - `FakeHerdr::rename(&PaneId, &str) -> Result<(), PaneError>` (and `WireHerdr::rename`, T's);
    - the case id `labelled-pane-reports-its-label`, last in `CASES`;
    - `parse_pane_info -> (PaneId, Option<String>)` and `parse_workspace_created -> (WorkspaceRef, PaneId, Option<String>)`,
      each with the raw label as sent.
  - E11's double is a pass-through `DisplacingHerdr` in a sibling module (`launch/displaced.rs`), so the rig stays at 739
    lines (A warn 6).
- **Assumed:**
  - T may not edit `crates/*/src/**` (`pipeline.config.json` `paths.production`). So the conformance case, `FakeHerdr`'s
    hook and the runner parameter are F's code, pinned by the testkit's mutants and `DOCUMENTED_CASES`.
  - `WireHerdr` is under `tests/`, so it is T's.
  - A made pane's `ensure_pane` answer carries the label of its creating reply (E10 item 3). I added test 17 for it,
    though E10 does not list one.
- **Hedged:**
  - Tests 2, 20 and 23 are guards: they hold on the base's logic once compiled, because constraints (a) and (b) and E11
    test 3's place check pin behaviour that must not change. Each was shown to fail against its mutant (`"label":null`; a
    join-key-only O2; a whole-value reconcile compare).
  - The labelled record's key order is not byte-pinned. Only the unlabelled form is (clarification 4).
- **Evidence:**
  - The literal: produced on `92fbc6e` (the branch's `crates/` equals it) by a throwaway probe test through
    `tx_launch::launch` with `now_ms = 1_760_000_000_000`, then `serde_json::to_string` of the stored record.
  - The stub: three stages in `src/`, reverted with `git checkout`. Stage 1 (API only) makes every behaviour test fail
    on its assertion. Stage 2 (fake reports labels) fails O2 `unavailable ... is gone from r2c1` and the relaunch record
    `Some("demo-c1r1")`. Stage 3 (four-field O2 plus `label: None`) gives `pane_verbs` 252/0.
  - `scripts/lint.sh` exits 0, and `cargo fmt --check` lists none of the touched test files.

## F, implement (Phase 6): 2026-10-11T05:57-06:00 (MDT)

- **Decided:** done, GREEN on every test T authored (`docs/handoffs/808/handoff-F.md`).
  - The label is the last field of `HerdrPane`, in the record format's optional-field form.
  - Each implementation builds its `HerdrPane` in one place:
    - the adapter in `HerdrAdapter::pane`, the only `label_of` call;
    - the fake in `Workspace::listed`, with `ensure_pane` answering the pane's own snapshot entry.

    So `snapshot` and `ensure_pane` cannot disagree (clarification 2, A warn 1).
  - `sent_label` is the parsers' one optional read (A warn 3), and `SessionState::pane_label` is the join by pane id.
  - O2 compares through `same_place` (`same_pane` plus workspace and cell), and `Plan::record` clears the label.
  - The label case is the last of the one `CASES` table. Without a hook, the runner filters it out, so no fixture is
    built, and the case fails closed if ever run without one (A warn 5).
  - E12's parameter is written through the `LabelHook` alias, the identical type, because clippy's
    `type_complexity` rejects the spelled-out type under CI's `-D warnings`.
  - ADR-0021: clarification 5's four places as written, plus section 1's `herdr` row (A warn 2).
  - `CHANGELOG.md` entry at the top of Enhancements.
  - `archChanged: true`: the public interfaces of the contract type, the protocol parsers and the conformance runner
    changed, as planned and as A reviewed.
- **Assumed:**
  - E10 item 3's "the two `HerdrPane` literals call `label_of`" is satisfied, and strengthened, by one shared
    constructor.
  - A bold title on "Decisions taken" item 10 is formatting, not a change to clarification 5's text.
  - `crates/*/src/**` of the test kit is production code, so F's (`pipeline.config.json` `paths.production`).
- **Hedged:**
  - `protocol.rs` crosses lint's 600-line warning (636; the fail is 900). `label_of` must live there because T's
    tests import it from `protocol`.
  - The section-1 ADR row goes beyond the letter of clarification 5; the PR body must name it.
  - If #802 merges first, "Decisions taken" item 10 is renumbered when `main` is merged in.
- **Evidence:**
  - Tests: `cargo test -p holler-pane-testkit` (fake 24/24, conformance 21/21), `-p holler-adapter-herdr` (adapter
    22, protocol 37, conformance 4), `-p holler-pane`, `-p holler-cli --test pane_verbs` 252/0 and
    `--test profile_verbs` 44/0.
  - The full `test.unit.command`: 148 targets, 2068 passed, 0 failed, 25 ignored, exit 0. A doc-only edit to the
    `HerdrPort` docs (`ports.rs`) followed; I re-ran clippy and the three contract crates (432 passed, 0 failed).
  - Gates: `cargo clippy --workspace --all-targets -- -D warnings` is clean; `scripts/lint.sh` and
    `scripts/changelog-check.sh` exit 0; `rustfmt --check` is clean on the 8 touched `.rs` files.
  - Five unchanged-code facts are in `docs/handoffs/808/evidence.md`.

## T, verify GREEN (Phase 7): 2026-10-11T06:08-06:00 (MDT)

- **Decided:** GREEN with no blocking issue; no test repaired (F changed no test and flagged none).
  - Full `test.unit.command` with `--no-fail-fast`: 148 targets, 2068 passed, 0 failed, 25 ignored. This matches F.
  - Four mutants on F's code (the record keeps the label; O2 requires whole-value equality; the adapter `label_of` keeps
    `""`; the adapter snapshot drops the join) are each caught. `src/` was reverted after each.
  - Tier 1 is clean: clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `rustfmt --check` on the
    touched files, `docs_cli_test`, `wire_selftest`.
  - Two evidence entries appended (reconcile keys by `(session, pane_id)`; findings carry no label), because the
    doctor guard relies on them.
- **Assumed:** the one failure on the first full pass, `remote_admin_traffic_never_moves_the_bodys_last_seen`
  (`last_seen` +1 s), is a pre-existing fixed-sleep flake unrelated to #808. The diff touches no hub, CLI source, body
  or proto code, and the test passed three isolated reruns plus the full rerun.
- **Hedged:** the flake is not filed. It is named in `handoff-T-green.md` as a follow-up outside this story.
- **Evidence:** `docs/handoffs/808/handoff-T-green.md`; `docs/handoffs/808/evidence.md` "T (Phase 7, verify)".

## A, anti-duplication gate: 2026-10-11T06:17-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 2 `warn` findings (`docs/handoffs/808/handoff-A-dup.md`). The diff is `92fbc6e..797b5bb`.
  - F extended every object the map named. The label is a field on `HerdrPane`. The adapter extends `PaneRef`, the two
    `PaneInfo` parsers, `SessionState` (`pane_label`) and `WorkspaceGrid`. The fake gains `rename` beside
    `vanish`/`print`. The suite keeps its one `CASES` table. O2 uses `same_place`, which reuses `same_pane`.
  - The diff removes duplication: each implementation now builds a `HerdrPane` in exactly one place
    (`HerdrAdapter::pane`; the fake's `Workspace::listed`, through `State::listed`).
  - Each new object has a written justification: clarification 2, E10, E11, or A's Phase 3 warns 3, 4 and 6.
  - Warn 1: `Plan::record`'s label clearing and `same_pane`/`same_place` are private to `tx_launch.rs`. #650's body
    mentions neither, though its importer is the second writer. O should note on #650 that the rules move to one place
    in `holler-pane` (for example `HerdrPane` methods in `pane.rs`) in #650's own PR.
  - Warn 2: `adapter_test.rs` is 810 lines and `tx_launch.rs` 798. The next adapter tests go in a new file with shared
    helpers in `tests/common`, and nothing more lands in `tx_launch.rs`.
- **Assumed:** that #650's brief has not yet been written with this follow-up in it. I read #650's issue body, not any
  local draft brief.
- **Hedged:**
  - The test kit's `label_of` copy is a near-copy of the adapter's. I did not flag it, because it is deliberate and
    documented both ways. The dependency direction forces it: the adapter dev-depends on the test kit.
  - The `LaunchRequest` literal now appears in three test files. I did not flag it, because the rig has no request
    helper and the existing tests inline it.
- **Evidence:**
  - The full diff `92fbc6e..HEAD` (production, test and docs).
  - Greps over `crates/`:
    - every `impl HerdrPort for`;
    - every `HerdrPane {` literal in `src`;
    - the `label_of` and `last_lines` definitions;
    - whole-value comparisons (none left in `src`);
    - place comparisons in `src`;
    - readers of `SessionState.panes`;
    - `HerdrPane` and `HerdrSnapshot` in the hub, CLI and proto `src`;
    - `profile_snapshot.rs`.
  - `crates/holler-adapter-herdr/Cargo.toml:25` (the dev-dependency on the test kit).
  - `wc -l` and `bash scripts/lint.sh` (size warns only).
  - `gh issue view 650` and `gh issue view 802` (OPEN).
  - `git diff --name-only 92fbc6e origin/main`: wizard files only, with no overlap.
  - A grep of the added lines for personal names: none.

## S, spec audit (Phase 10): 2026-10-11T06:30-06:00 (MDT)

- **Decided:** PASS (`docs/handoffs/808/handoff-S.md`).
  - Each acceptance criterion (the issue's AC 1-5, E9's and E15's) has a test that asserts the behaviour (17 rows), and
    the guards fail against their mutants.
  - Every brief decision is implemented. Three documented deviations are accepted:
    - one `HerdrPane` constructor in place of E10 item 3's two `label_of` calls;
    - section 1's `herdr` row, which is within scope item 4;
    - the `LabelHook` alias, the identical type.
  - The quality gates, privacy and scope are clean.
  - Seven advisories:
    1. the doctor guard labels each pane with its own id (a test-only, one-line change);
    2. `docs/testing.md:441` still says "the whole suite";
    3. the "one exception" wording in `protocol.rs`;
    4. the PR body needs the AI disclosure and the section-1 note;
    5. the note for #650 is not yet posted;
    6. the #802 merge order;
    7. the unrelated flake.
- **Assumed:**
  - The commits' `Co-Authored-By` trailer without a session link follows repo practice (`main`'s recent squash commits
    match), so it is not a finding.
  - The PR body cannot be checked before the script opens the PR.
- **Hedged:**
  - I rated advisory 1 non-blocking rather than a test-only REWORK:
    - the criterion is met as written;
    - the class of regression it guards (a comparison that sees the label) is shown caught by a mutant;
    - the gap is a regression that substitutes the label for the id, which no code in this diff makes.
  - I read section 1's row as within scope item 4 ("sections 1 and 2 (the `HerdrPane`/`HerdrSnapshot` types ...)"), not
    as a breach of clarification 5's "exactly these places".
- **Evidence:**
  - Issue #808, read live: its body equals the brief's lines 3-60. Its last edit was at 04:20 MDT, before `04e53da` at
    04:26 MDT. It has no comments.
  - The whole `git diff origin/main...HEAD`.
  - Greps:
    - whole-value `HerdrPane` comparisons in `crates/*/src`;
    - `unwrap`, `expect`, `panic`, `#[allow]` and `sleep` in the added lines;
    - privacy, over 3,933 added lines;
    - the `run_herdr_conformance` callers;
    - Debug formatting of `HerdrPane`.
  - `wc -l` on every touched file.
  - NV-1: `scripts/spikes/herdr-grid.sh:28` and `docs/research/herdr-api-spike.md:183-191`.
  - The CHANGELOG claim: the `Unwired` stub in `holler-cli/src/pane/wiring.rs`, and `gh issue view 649`.
  - `gh issue view 650` and `gh issue view 802`.
  - The outside reviews' `.usage.json`: all four on `deepseek-v4-pro`.
