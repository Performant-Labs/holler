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
