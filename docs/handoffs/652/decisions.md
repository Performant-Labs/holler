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
    it for fleet panes (`hj-c3r1`). Whether the operator wants them out of the fleet before the cutover is option (b),
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
