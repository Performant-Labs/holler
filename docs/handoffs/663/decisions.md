Rigor/brief gate: BLOCKED, overridden by Andre Angelantoni: The outside brief gate (deepseek-v4-pro) stopped as non-converging; the blocks in the last rounds were retracted by the reviewer in its own text or were not applicable at a brief gate. Operator approved overriding the brief gate on 2026-10-09; the diff gate stays on. (stop reason: non-converging-reviewer after 3 round(s); each round's findings are in the review artifacts under the run's handoff directory)

## A (Phase 3, up-front plan review) — 2026-10-09T17:56:37-06:00
- **Decided:** BLOCK on `docs/handoffs/663-brief.md` at `89b611f`, with one block and six warns (see `handoff-A.md`).
  - **The block (B-1):** Decision 22 rules out the ADR-0021 edit, but the plan extends or narrows the ADR in five places:
    - section 1's `ProbeResult` verdict (Decision 14, C10);
    - section 2's I5 bound, for `edit_spec` (Decision 11) and for `run_probe` (Decision 15);
    - section 8 step 2's first-write timeout (Decision 6);
    - section 8 steps 5-6, the non-conflict restore failure and the step in the scope's error (Decisions 5 and 7);
    - section 8's reconcile-step text (Decision 8, C9).
    The stack rule requires the ADR to be updated in the same change. The brief's reason (blast radius) has been overruled
    at this gate before (#683 blocked; #688 required the edit), and the practice since #634 closed is the same-change edit
    (#639, #676, #692, #697, and #644, #647 and #662 in flight). The fix is a new AC with five in-place ADR sentences, or a
    separate `docs(adr)` PR first.
  - **The rest of the plan extends the right objects.** `StoreScope` implements the frozen trait in the file ADR-0021
    section 5 names, in the fake's shape. `run_probe` uses std only and stays private. The suite and fixtures are reused
    unchanged. Inline tests follow the workspace pattern. The third membership-rule copy is justified in writing (W-5,
    accepted for Phase 7).
- **Assumed:**
  - The brief's "Reuse map" table and its Evidence sections are the Reuse map, since this run has no separate survey.md.
  - The operator's override of the outside brief gate (the entry above) does not rule on the ADR question. It names the
    non-converging outside reviewer, not the in-session A gate.
  - The epic's hot-spot line "`docs/adr/ADR-0021.md`: #634" is historical now that #634 is closed. Every ADR-0021 change on
    main since #634 came from the story that made the decision.
  - #644's brief on origin/issue-644-implementation at `7195993` is current. It is in flight and may change again.
- **Hedged:**
  - **B-1 is a block, not a warn.** The #688 review made an ADR extension only a warn (W-1), because that brief explicitly
    said "No ADR change" and the precedent was then mixed. Here there are five extensions, two of them narrowing section 2's
    stated bound, and a concurrent story (#644) is already consuming them from the brief. The dominant practice is now the
    same-change edit. This matches #683's block rather than #688's warn.
  - **W-1 (the unscoped reconcile step) is a warn.** #644's own review and amendment already settled its side. Moving the
    const into `profile_scope.rs` is additive, but it needs #644's run to switch its import, so it is a coordination call.
  - **W-2:** F1 cannot match the fake to Decisions 5-7 without a copy or a `holler-pane` hoist (ADR-0021 section 5: the
    test kit must not depend on `holler-cli`).
  - **W-4** is forward-compat for #696, not drift.
  - **The `cfg(unix)` question** is mixed in the workspace and is not a finding.
- **Evidence:**
  - Read the brief in full, the three outside-gate results (r1-r3), issues #663, #688, #683, #639, #661, #676, #696 and
    #634, and the epic's skeleton rulings and hot-spot list.
  - Read ADR-0021 sections 1, 2, 5, 8, 9 and 12, "Deferred" and "Decisions taken", ADR 0002's Windows row, and the ADR
    README.
  - Read the code:
    - `holler-cli/src/{lib.rs,pane/mod.rs,pane/wiring.rs,pane/profile_scope.rs}` and the `Cargo.toml` dependencies and
      targets;
    - `holler-pane/src/{lib.rs,probe.rs}`, `error.rs:388-517`, `ports.rs:202-235`, `profile.rs:72-119, 373-380` and
      `Cargo.toml`;
    - `holler-pane-testkit/src/profile_scope.rs` (whole), `tests/profile_scope_conformance_test.rs` and the testkit
      manifest;
    - `holler-hub/tests/pane_membership_test.rs:1-60`.
  - Grepped the workspace for:
    - membership-rule copies and `PaneInOtherProfile` producers;
    - shell-quoting helpers;
    - process spawn, timeout and group-kill code in `crates/*/src`;
    - `std::os::unix` and `cfg(unix)` use;
    - inline test-module allows;
    - conformance-suite runners;
    - real port implementations.
  - Checked the git history of ADR-0021 (`094ebfa`, `2a6f349`, `3f9fbf2`, `316b8e3`, `c76bbed`) and the PR bodies of #692,
    #697 and #698. Checked the earlier Phase 3 reviews (`a3e47a4` #683, `db4cba8` #688, `d2636ba` #644) and #644's brief
    at `7195993` (I-3, C-15, C-16, the `launch.rs` API).
  - Read the CI matrix in `.github/workflows/ci.yml`.

## A (Phase 3, up-front plan review, second pass) — 2026-10-09T18:29:59-06:00
- **Decided:** PASS on `docs/handoffs/663-brief.md` at `d7e0421`, with no block and six warns, W-7 to W-12 (see `handoff-A.md`,
  which replaces the first pass's BLOCK at `5284f91`).
  - **The first pass, dispositioned.** B-1 is resolved: AC 14 amends ADR-0021 in place at the five places, and each matches the
    ADR text at `3bdd129`. W-1, W-2, W-3, W-4 and W-6 are resolved as asked, and W-5 stays accepted for Phase 7.
  - **The new warns:**
    - W-7: the "may have landed" rule is decided by code (only `timeout`), not by outcome. Decision 5 also states a timed-out
      restore's result as a fact.
    - W-8: section 12 restates the bound AC 14b narrows, and AC 14e does not list which errors carry the step.
    - W-9: Decision 4's record-inside-the-act versus the fence bullet's "writes nothing more".
    - W-10: Decision 13's unguarded `process_group(0)` cites the one exception to a guarded pattern.
    - W-11: the inline probe tests break `holler-pane`'s local `tests/` pattern, and they carry near-copies of `StateDir` and
      `wait_for`. Both copies are pre-ruled to pass at Phase 7 if they stay private and minimal.
    - W-12: gaps in the follow-ups (F4 misses `lib.rs:7` and `lib.rs:26`; AC 14b's cite of F4 has no number at F time; F2's
      hoist belongs beside `detail()`).
  - **F can take** W-7 (a)-(c), W-8, W-9 and W-10 within the existing ACs and AC 14's hunks. W-7 (b)'s requirement on #649 is a
    cross-story note for O, beside F5.
- **Assumed:**
  - The brief's Reuse map table is the Reuse map, as in the first pass (no survey.md).
  - #644's brief at `7195993` and #641's at `0f18b80` are current. Both are in flight, and neither has code yet.
  - #649's store client does not exist yet, so W-7's precedent is the control path (`transport.rs:69-78`) and the `PaneReply`
    parse-back (`error.rs:599-611`), not a real client.
- **Hedged:**
  - **W-7 is a warn, not a block.** No code changes, nothing live moves on a first-write failure, and a rerun heals it. The cost
    is the standing spec carrying a by-code rule.
  - **W-9 predates this story.** #697 made the ASSUMPTION and #644 composes its act that way. #663 only confirms it, so the fix
    is one clause in a hunk AC 14e already makes.
  - **W-10 is a warn.** The shipping binary is already Unix-only through `holler-hub/src/control.rs:8`, and Windows is off the
    CI matrix (ADR 0002).
  - **W-11 is a warn.** The workspace mixes inline and `tests/` placement. Only the crate-local pattern is `tests/`, and the
    blast radius justifies the departure.
- **Evidence:**
  - **The brief:** read in full, with the diff from `89b611f`, and the first pass's handoff and journal entry.
  - **The ADR record:** ADR-0021 in full (lines 1-558), `docs/adr/README.md`, ADR 0002's Windows row, and issue #22 (still the
    placeholder body).
  - **The code:**
    - `holler-pane`: `error.rs:388-710`, `ports.rs:200-235`, `profile.rs:328` and `373-380`, `lib.rs:1-76`, `probe.rs` and
      `Cargo.toml`;
    - `holler-pane-testkit`: `profile_scope.rs:1-130` and `conformance/profile_scope.rs:186-200`;
    - `holler-cli`: `src/pane/{mod.rs,profile_scope.rs,launch.rs,close.rs,wiring.rs}`, `src/transport.rs`, `Cargo.toml`, and
      `tests/pane_verbs/main.rs`;
    - `holler-cli/tests/support/mod.rs:64-163`, and `holler-load-test` `hub.rs:320-345` and `main.rs:545-590`.
  - **Grepped `crates/`** for `std::os::unix` (guarded or not), process-group and kill code, `Command::new`, shell-quoting
    helpers, `StateDir`/`TempDir`/`Scratch` types, `wait_for`, and the `cfg(test)` modules in `holler-pane/src`.
  - **The in-flight branches** of #641, #644, #647 and #662, all brief-only:
    - their planned ADR-0021 edits, for merge hygiene;
    - #644's act table, append rule and const references (lines 1596-1627, 1705-1708, 1554-1558);
    - #641's runner, kill and seam lines.

## T (Phase 4, author / RED) — 2026-10-09T18:34:54-06:00
- **Decided:** RED is valid. 19 new tests: 7 in `crates/holler-cli/src/pane/profile_scope.rs` `mod tests` (AC 1-7) and 12 in
  `crates/holler-pane/src/probe.rs` `mod tests` (AC 8a-8l). All 19 fail on a feature assertion against the stubs; 8m
  (`ports_test::run_probe_stub_never_reports_success`) stays green by design. Per the brief's Test plan, T landed the
  compile stub of Decisions 1 and 8 in `profile_scope.rs` (`StoreScope` + `new`, an `impl ProfileScope` answering
  `not-implemented`, `reconcile_step` returning `""`, `RECONCILE_STEP_UNSCOPED = ""`); F replaces every body.
- **Decided:** in 8f/8g the pid-file existence assertion runs before the `timed out` assertion (still after `run_probe`
  returns), so a runner that answers without spawning fails on the existence check, as the brief's RED list says.
- **Assumed:** AC 2's "holding `s'`" is checked as `panes[0] == s'` (the in-place replace keeps index 0, Decision 3.1).
  AC 8f/8g assert no upper bound on `run_probe`'s return beyond the 2 s poll; the brief derives a 4.5 s budget but pins an
  elapsed bound only in 8e.
- **Hedged:** the 8f/8g poll and the 8e upper bound carry the brief's slack (2 s); macOS CI is their first macOS run.
- **Evidence:** `cargo test -p holler-cli --lib pane::profile_scope` → 0 passed, 7 failed (each `not-implemented` or `""`
  vs the expected value); `cargo test -p holler-pane --lib probe::tests` → 0 passed, 12 failed (each on the stub's
  `Error("the probe runner is not implemented yet (story #663)")`); clippy `-D warnings` on both crates and
  `scripts/lint.sh` exit 0; `rustfmt --check --edition 2021` on both files exits 0. Throwaway cross-check (reverted):
  the scope tests over `FakeProfileScope` pass AC 1, 6, 7 and differ from it only on Decisions 5-7's message text.
  Details in `handoff-T-red.md`.

## F (Phase 5, implementation) — 2026-10-09T19:04:11-06:00
- **Decided:**
  - **The scope.** `profile_scope.rs` now has the real `StoreScope`, built in the fake's shape over any `ProfileStore`
    and `PaneStore`:
    - `resolve`, and `edit_spec` in ADR-0021 section 8's order;
    - `reconcile_step` and `RECONCILE_STEP_UNSCOPED`, as briefed (Decisions 1-12);
    - the append helper `with_context`, one `match &mut error` with no `_` arm.
  - **The probe runner.** `probe.rs` now has the runner (Decisions 13-20), private and std-only:
    - mechanics: spawn in its own group, a detached capped reader, one deadline, and a group kill through `kill` before
      the leader is reaped, within one 1 s budget;
    - a private `Outcome`, mapped by `verdict` alone.
  - **Docs.** One CHANGELOG entry, and the five AC 14 ADR edits.
  - **Taken from Phase 3's warns:**
    - W-7 (a)-(c): `may still hold` after a timed-out restore, the rule by outcome, and the requirement on #649's store
      client;
    - W-8 (1)-(2): the section 12 cross-reference, and the three errors named;
    - W-9: the fence-bullet clause, and "the live change or its record failed" in both restore messages;
    - W-10: `#[cfg(unix)]` guards;
    - W-12 (2): no follow-up number in AC 14b.
  - **Two more fixed probe reasons, echoing nothing:** "the probe timeout is too large" (refused before a spawn,
    instead of a panic on `Instant` overflow) and "the probe's exit status could not be read" (a `try_wait` error; no
    signal follows).
  - **A `kill` process that outlasts the budget** is killed and reaped without blocking, else left for the caller's
    exit.
  - **The tests are not edited.** Both test modules are byte-identical to T's (sha256 checked). `archChanged: true`.
- **Assumed:**
  - `io::ErrorKind`'s `Display` (for example `entity not found`) carries no program name or argument. This is std's
    fixed text per kind, and 8j's absence checks pass.
  - `kill -s KILL -- -<pgid>` behaves on macOS's BSD `kill` as on Linux procps. The FreeBSD-derived source accepts `-s`,
    `--` and a negative pid, but nothing here has run it; CI's macOS job is the first evidence.
  - The Workflow script or the merge step rebases onto the moved `origin/main` and resolves the CHANGELOG conflict. F
    does not commit or rebase.
- **Hedged:**
  - **`profile_scope.rs` is at 597 lines** against the 600-line warning. The docs were tightened to fit; T's 296-line
    test module is fixed.
  - **#701 (#647 part 1) merged during this run** and gave `pane doctor` a `[PANE]` positional. The step stays
    profile-scoped, as AC 5 pins and Decision 8 chose. Naming the pane is a follow-up for O, and the ADR and rustdoc
    wording reads correctly whichever lands first.
  - **The 8e-8g timing slack** is unproven on macOS until CI runs. Here: 10 of 10 repeated runs passed, in 1.38 s
    serially.
- **Evidence:**
  - **Tests:** `cargo test -p holler-cli --lib pane::profile_scope` gives 7 passed; `cargo test -p holler-pane --lib
    probe::tests` gives 12 passed; `ports_test run_probe_stub_never_reports_success` gives 1 passed.
  - **Workspace:** `HOLLER_STATE_DIR=<scratch> cargo test --workspace --no-fail-fast` (with CI's skip) gives 1402 passed
    and 0 failed in 125 suites. Without the scratch state dir, four pre-existing `logging_test.rs` cases fail because
    this host runs a hub on the default socket.
  - **Gates:** `cargo clippy --workspace --all-targets -- -D warnings`, `scripts/lint.sh`,
    `scripts/changelog-check.sh`, `cargo machete` and `rustfmt --check --edition 2021` on both files all exit 0.
  - **AC checks:** the AC 9, 11, 12 and 14 checks hold. AC 12 is measured against the merge base `3bdd129`.
  - **Merge dry run:** an in-memory `git merge-tree` of the working tree against `origin/main` `e612878` conflicts in
    `CHANGELOG.md` only.
  - Details are in `handoff-F.md` and `evidence.md`.
