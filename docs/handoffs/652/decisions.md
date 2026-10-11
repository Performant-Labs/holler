# Decision journal: #652, the operator guide for `holler pane`

- **Run handoff directory:** `docs/handoffs/652`.
- **Branch:** `issue-652-implementation`.
- **Settings:** rigor second-opinion; uiSurface false, so D and U are N/A.
- **The brief gate's rounds:** `docs/handoffs/652-brief-result-r{1,2}.md` (round 1 BLOCK, round 2 PASS).

This file is append-only: every phase adds its entry.

## A, up-front plan review: 2026-10-11T07:29-06:00 (MDT)

- **Decided:** BLOCK, with 1 `block` and 8 `warn` findings (`docs/handoffs/652/handoff-A.md`).
  - **The test plan passes.** The new target follows `docs_cli_test.rs` and `holler-proto/tests/docs_errors_test.rs`.
    The test reads the public tables on the base. It does not reuse the hand-kept `PANE_VERBS`/`PROFILE_VERBS`
    (operator decision 3), and it duplicates no existing doc or test.
  - **The block is finding 1.** Several parts of the plan treat `holler pane` as managing every fleet pane: section 1's
    rule (I1), section 11's rollback, the `docs/orchestrating.md` rewrite, and the statement that the script is removed
    after the sign-off. But `HarnessKind` is `opencode` only (ADR-0021 section 1), #650 skips a non-OpenCode pane
    (`harness-not-opencode`), and `scripts/pane-restart`'s Claude Code path (#784) is that pane's only restart path. The
    interim section being rewritten contains that path (`orchestrating.md:125-145`). The fix needs an operator
    decision: (a) remove only the script's OpenCode path after the sign-off (recommended), or (b) require every fleet
    pane to run OpenCode before the cutover.
  - **The warns:**
    - 2: a rollback with the script on a registry pane leaves the record stale, and `doctor --fix` would undo its session.
    - 3: `holler pane import` needs `--from` and `--herdr-session` (ADR-0021 section 13, #650), and "derived" should be
      "replaced by".
    - 4: `docs_cli_test` limits: fence types, the typed `--since`, `--ctx-soft` and `--ctx-hard`, and PROPOSED verbs
      only in bare form.
    - 5: numbered `##` headings and one PROPOSED rule; #665 will edit the test.
    - 6: check the class column against `class_of`; ADR-0021 is the apply reference's authority; the I1 wording.
    - 7: the test's name and shape.
    - 8: the Herdr display plugin (#651).
    - 9: `scripts/pane-restart:3`'s header.
- **Assumed:**
  - Per the operator's plan, this briefs-ahead run stops after this verdict commit. The build is a fresh run after #802,
    #642 part 3, #644 part 2, #649, #650 and #664 merge, with the evidence re-taken (`gh issue view` on 2026-10-11 shows
    all six OPEN).
  - Claude Code panes are in the live fleet. #784 added the script's Claude Code path, and `orchestrating.md` documents
    it for fleet panes. Whether the operator wants them out of the fleet before the cutover is option (b),
    the operator's call.
- **Hedged:**
  - Finding 1 is a `block`, not a `warn`. The guide is the agent guidance for orchestrators, and the fix changes the
    brief's own text ("the script is removed ... after the sign-off"), so it needs a decision that F cannot take.
  - In finding 2, which doctor kinds a rollback produces depends on whether the script's new session lands on the
    recorded server (`shown-driven-mismatch` and `stray-session`) or on another one (`tui-foreign-session`). Both
    readings are named.
  - The brief has no separate Reuse map, so the evidence E1 to E34 and the Blast radius stood in for it, as in #808's
    review.
- **Evidence:**
  - **Read:** the brief (all 1165 lines) and the gate results r1 and r2.
  - **Pipeline and role docs:** `workflow-coding-pipeline.md` (Phase 2 and 4 details, the `arch` gate) and
    `pipeline-conventions.md` section 1.
  - **Epic and issues:** epic #633 (the hot spots, the rulings, the Decisions of 2026-10-09); the bodies of issues #650
    and #654; the #664 brief in worktree `0664b-profile-apply`.
  - **Code:**
    - `docs_cli_test.rs` (whole), `tests/fixture/mod.rs` and `cli_surface_test.rs`;
    - `pane_verbs/process/{main.rs,docs_rows.rs}` and `holler-proto/tests/docs_errors_test.rs`;
    - `holler-cli/Cargo.toml` (the test targets, the #670 comment);
    - `scripts/lint.sh` and `changelog-check.sh`;
    - `holler-cli/src/lib.rs` and `pane/mod.rs`; `holler-pane/src/lib.rs`, `error.rs`, `findings.rs` and `pane.rs`
      (`HarnessKind`);
    - the `Args` in `watch.rs`, `args.rs`, `relaunch.rs` and `profile/{apply,export,import,rename}.rs`;
    - `plugins/herdr-holler/herdr-plugin.toml` and `src/main.rs`;
    - `scripts/pane-restart:1-24`.
  - **Docs:** `docs/orchestrating.md:100-145`, `docs/README.md`, ADR-0021 sections 1, 4, 9, 12 and 13,
    `docs/agent-overlays/tester.md` and `docs/releasing.md` (the CHANGELOG structure).
  - **Greps:** the docs tests and doc readers across `crates/`, the clap-tree walks, `pane-restart` across `*.md`,
    `holler pane` and `holler profile` across `docs/`, and the typed flag values in the pane and profile `Args`.

## A, up-front plan review, round 2: 2026-10-11T14:34-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 5 `warn` findings (`docs/handoffs/652/handoff-A.md`, overwriting round 1's
  BLOCK, which stays in git at `060d328`).
  - Round 1's block is resolved by operator decision (a); warns 2 to 9 of round 1 are folded into the brief.
  - Warns: (1) section 3's `switch` and `park` heading forms disagree with the clap tree and with acceptance 17's
    computed forms (`switch` needs `SESSION`; `park`'s `PANE` is optional); (2) section 10's closing sentence is
    unqualified against known fact (1), and a follow-up issue for an import exclusion should gate #654; (3) #649 is
    still open, so sections 2 and 4 wait for its merge; (4) acceptance 15 is a second hand-kept list, against decision
    3's wording; (5) the new test file may need a directory target to stay under the lint limit.
- **Assumed:** this is the build run on base `5d366b9`; #649 is the only open dependency (`gh issue view 649`: OPEN).
- **Hedged:** finding 2's hazard (Claude Code panes recorded as OpenCode panes) is a product gap for #654, not drift in
  this docs story, so it is a `warn` here; the brief states it as a fact and invents no procedure, as decided.
- **Evidence:** the brief at `05d7804`; round 1's handoff; `pane/{switch,park,import,launch,list,watch}.rs`,
  `profile/apply.rs`, `pane/import/writes.rs`, `holler-pane/src/{pane,error,grid,import,argv,tx_apply}.rs`,
  `import/findings.rs`, `holler-adapter-opencode/src/server.rs:31`, `holler-pane-testkit/src/envelope.rs:202`,
  `holler-cli/Cargo.toml`, `pane/wiring.rs:1-40`, `scripts/pane-restart:1-12`, `docs/README.md` and
  `docs/orchestrating.md` headings, ADR-0021 line 302; greps for the interim anchor, the stop-gap sentence and `hj-c`.

## T, author tests (RED): 2026-10-11T14:38-06:00 (MDT)

- **Decided:** RED is valid. There is one new target, `docs_pane_control_test`, with 16 tests: acceptances 1 to 5 and
  7 to 17 each have a test, and acceptance 6 is the existing `docs_cli_test`. All 16 fail on missing-content
  assertions (`docs/handoffs/652/handoff-T-red.md`). The file has 511 lines, so it stays a single file and needs no
  directory target (A warn 5).
  - Section 3's heading forms are computed from the clap tree: required positionals, then required flags, plus
    `--herdr-session NAME` for `pane import`. The result agrees with A's warn 1 (`switch PANE SESSION`;
    `park --reason TEXT --release-when WHEN`). The handoff lists the forms for F.
  - The class column in the error-code table is built as `<word> (<class_of(code).exit_code()>)`. The word comes from
    matching on `ErrorClass`, and the number is never typed.
- **Assumed:**
  - Acceptance 15's "apply reference" is the `` ### `holler profile apply `` subsection of section 3 together with
    section 6. The brief does not fix where the reference sits, and these are the two places it points to.
  - Acceptance 4 is checked with a whole-word match for each column name, with no backticks required. The brief says
    only that each name "appears".
- **Hedged:**
  - A missing file reads as empty instead of panicking, so each test shows its own list of missing items.
  - Tests 12 and 13 each add a non-vacuity check (at least one scanned `holler ` command; at least one success and one
    failure envelope), so neither passes on an empty guide. Without these checks, both would be green before the
    guide exists, which is not a valid RED.
  - The section 3 heading must start with the exact computed span. A suffix after the closing backtick is allowed.
- **Evidence:**
  - Ran `cargo test -p holler-cli --test docs_pane_control_test`: 0 passed, 16 failed.
  - Ran a positive control with a temporary scratch guide, deleted afterwards: tests 5, 11, 12 and 13 passed, and the
    classed rows and the `switch` heading matched.
  - `cargo clippy ... -D warnings` was clean, and `bash scripts/lint.sh` exited 0.
  - Read: `findings.rs` (`FindingKind::ALL`), `import/findings.rs`, `import.rs:45`, `error.rs:146,221-266`,
    `grid.rs:44`, `pane/list.rs:30`, `holler-pane-testkit/src/envelope.rs:202`, and the verb `Args` in
    `pane/{switch,park,import,get,launch,relaunch,reset,unpark,close,doctor,list,watch}.rs`.

## F, implementation: 2026-10-11T14:55-06:00 (MDT)

- **Decided:** wrote `docs/pane-control.md` (twelve numbered sections), rewrote the "Restarting a pane on another model"
  section of `docs/orchestrating.md` (heading without "(interim)"), added the `docs/README.md` Guidance line, replaced
  `scripts/pane-restart` line 3 with two comment lines, and added the CHANGELOG entry (`docs/handoffs/652/handoff-F.md`).
  - Section 3 uses T's computed heading forms (A warn 1); section 10's closing sentence is qualified (A warn 2).
  - Every list was taken from the built binary's `--help` and the code's tables, not from the brief.
  - 15 of 16 tests of `docs_pane_control_test` pass, and `docs_cli_test` passes. The 16th,
    `readme_indexes_the_guide_under_guidance`, cannot pass as written (`from_heading` with a start that begins `\n## `
    returns empty); flagged for T, the test is not edited.
- **Assumed:** this run reached F before #649 merged, so the #649 statements of sections 2 and 4 follow the #649
  branch's `wiring.rs` module doc and the brief's R1 to R3, which agree on every point the guide states.
- **Hedged:** the guide names only `HOLLER_HERDR_SESSION` (the issue body and the branch differ on its fallback), and the
  second known fact of section 10 is stated as true until #649 merges.
- **Evidence:** `holler pane|profile <verb> --help` for every verb on the built binary; `import/findings.rs` (`ALL`,
  `level`); `findings.rs` (`ALL`, `remedy`, `FixState`); ADR-0021 sections 3, 4, 8 (launch, switch, close, apply), 9, 10,
  11, 12, 13; `pane/launch.rs:385-430`; `pane/doctor.rs:60-112`; `adapter.rs:175-190` (Herdr); `profile_diff.rs:193-204`;
  `envelope.rs:202-360`; `docs_cli_test.rs` (whole); the #649 branch's `pane/wiring.rs:1-60`; the import test fixtures for
  the launcher table's shape. Facts in unchanged code are in `docs/handoffs/652/evidence.md`.

## T, verify (GREEN): 2026-10-11T15:05-06:00 (MDT)

- **Decided:** the RED test `readme_indexes_the_guide_under_guidance` was wrong, as F reported, so I repaired it in the
  helper `from_heading`. The end search now skips the first byte, so a start of `\n## Guidance` no longer ends at
  offset 0. No production file changed. With the repair the suite is GREEN: 16/16 `docs_pane_control_test`, 3/3
  `docs_cli_test`, and the workspace unit command exits 0. Verdict PASS (`handoff-T-green.md`).
  - I added three evidence entries: the `tx_apply` `as_str` words, `pane import`'s runtime-required `--herdr-session`,
    and `profile-drift` as a code but not yet a kind.
- **Assumed:** the #649 statements of sections 2 and 4 are acceptable while #649 is open (A warn 3; F's known issue).
  They agree with the #649 branch's `wiring.rs` module doc, lines 22-38.
- **Hedged:** no test reads #649's code. If #649 changes its contract before it merges, the guide drifts without a
  failing test. S, or a check after #649 merges, re-reads that module doc.
- **Evidence:**
  - Nine doc mutations, each reverted. Every one failed exactly the matching test, including a README link moved out
    of Guidance.
  - Tier 1: `lint.sh`, `changelog-check.sh`, clippy `-D warnings`, `wire_selftest`, `docs_cli_test`, `cargo machete`,
    `pane-restart-test.sh`, `test-hooks.sh` and `cargo test --workspace --no-fail-fast -- --skip
    roster_stays_accurate_under_concurrent_body_load` all exit 0.
  - `gitleaks detect --no-git` on the guide found no leaks.
  - `gh issue view 649` shows OPEN.

## A, anti-duplication (Phase 7): 2026-10-11 (MDT)

- **Decided:** PASS, 0 block, 3 warn (`handoff-A-dup.md`). F extended what the brief named: one new page under the
  existing "Guidance" index, the obsolete `orchestrating.md` section rewritten in place, a comment-only script change, one
  CHANGELOG entry; ADR-0021 sections 9 and 13 and the `wiring.rs` module doc are linked, not restated.
- **Warns:** section 3's `profile apply` subsection repeats section 6's reference; the test's `SCANNED_TAGS` hand-mirrors
  `docs_cli_test.rs:101` (follow-up: share it via `tests/fixture/mod.rs`); the #649 statements still rest on the open branch.
- **Evidence:** `git diff 5d366b9..3d318df`; `docs/pane-control.md` (whole); `docs_pane_control_test.rs` (helpers and
  tests); `docs_cli_test.rs:29,87-130`; `tests/fixture/mod.rs`; ADR-0021 heading lines 662 and 863 (anchors resolve).

## F, rework after S ADVISORY-HOLD (#649 and #832 merged): 2026-10-11 (MDT)

- **Decided:**
  - R1 to R3 re-cut from the merged `wiring.rs` module doc (lines 22-38). Section 2 states D2 as built (the hub is the
    only precondition; a missing or malformed variable fails at the port's first use; `HERDR_SESSION` is not read; unset
    `HOLLER_HERDR_SESSION` is `usage`).
  - The session-mismatch sentence is split by verb, per the patched brief: `launch` (and apply's create step) answers
    `unavailable`; `pane import` reports every pane `pane-unlabelled`, so its dry run exits 0 and a real run is
    `import-blocked`. Stated in section 2 and in section 10, step 1.
  - Section 10's second known fact (the stubbed dry run) is dropped; the first fact names #835 as the follow-up.
  - **#832 (merged in `8dc1aca`) is documented**, because the docs test went RED on it (`section_5_names_every_finding_kind`:
    `unmanaged-stalled` missing) and the guide said things #832 made false. Unmanaged panes in sections 1, 3 (list, get,
    the acting verbs' `pane-unmanaged`, doctor's `--unmanaged-stalled-after`), 4 (the second table, JSON
    `unmanaged`/`unmanaged_unavailable`, IDLE `-` against Herdr 0.8.2), 5 (the `unmanaged-stalled` row; "four name none"),
    9 (open code `pane-unmanaged`) and 10.
  - **#829 stated** (model, effort, env names and ceilings recorded, not applied): section 2, `relaunch`, step 5 of the
    cutover, section 11's way back, section 12, `orchestrating.md` and the CHANGELOG line. `orchestrating.md` no longer
    says the relaunch "starts the harness server again on the new model".
  - **The `reset --first` caveat** (ADR-0021 lines 365-367; #828 folded into #839, open) is in `reset`, `park` and step 5.
    DRIVEN's follow-up is named as #839.
- **Assumed:** a relaunch whose record names a Herdr session other than `HOLLER_HERDR_SESSION` meets the same
  `extent_of` refusal as a launch (it goes through the same launch engine and adapter); not run.
- **Hedged:** #829 and the `reset --first` caveat go beyond the brief's text (it names neither). They are stated because
  the brief's rule is that the guide states what is on `main`, and without them `orchestrating.md` and section 11 told
  the operator a relaunch changes the model the server runs.
- **Evidence:** `evidence.md`, "F rework" entries; `cargo test -p holler-cli --test docs_cli_test --test
  docs_pane_control_test` (3 and 16 passed); `scripts/changelog-check.sh` ok; `scripts/lint.sh` exit 0; `gh issue view`
  829 OPEN, 835 OPEN, 839 OPEN, 828 CLOSED (not planned, folded into #839), 649 and 832 CLOSED.

## T, GREEN rework after F's rework: 2026-10-11 17:55 MDT

- **Decided:** PASS. The suite is GREEN: `docs_pane_control_test` 16/16 and `docs_cli_test` 3/3. Tier 1 is clean apart
  from the known flake #847.
- **Decided:** acceptance 17's test is extended so that sections 2 and 10 name `pane-unlabelled` (the re-cut R2) and
  section 9 names `PANE_UNMANAGED` (read from the code). I chose this over a new test function, to keep the suite in
  proportion. Each new assertion failed alone under a mutation.
- **Assumed:** the `remote_admin_test` failure is #847, not this diff: no Rust source changed, and it passed 4 of 5
  re-runs alone.
- **Hedged:** the unmanaged table's columns are not pinned (`UNMANAGED_COLUMNS` is private). This is an advisory note,
  not a production change.
- **Evidence:** `handoff-T-green.md` (rework section); `evidence.md`, "T (Phase 6 rework)".
