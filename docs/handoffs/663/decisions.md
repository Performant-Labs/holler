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

## T (Phase 7, verify / GREEN + Tier 2) — 2026-10-09T19:12:48-06:00

- **Decided:**
  - **GREEN with no test edit.** Both test modules are byte-identical to the RED commit `8fe683b`. F flagged no test as
    wrong, and T found none.
  - **The tests are checked by mutation, not only re-run.** Ten one-behaviour mutations of F's code were each killed by
    at least one authored test. The first pass reported four survivors; all four were compile errors from
    `dead_code = "deny"`, which the script had not classified. Redone with `if false` guards, all four were killed.
  - **Three fixture facts were added to `evidence.md`** (`seeded` at generation 1, `fail_next` one-shot, `concurrent_put`
    at generation + 1), each copied by T from source.
  - **Verdict: PASS.** No blocking issue, so no production change for F.
- **Assumed:**
  - F's four `logging_test.rs` failures came from a hub on the default socket at the time. They did not reproduce here
    (11 passed without `HOLLER_STATE_DIR`), the file is outside the diff, and CI has no hub.
- **Hedged:**
  - The macOS timing and BSD `kill` group-signal evidence is CI's.
  - `profile_scope.rs` is at 597 of 600 lines.
  - A restoring write that fails with a payload-less `PaneError` loses its context. No store answers that today; this is
    advisory, for F2.
- **Evidence:**
  - Scope 7/7 and probe 12/12 GREEN, probe serial in 1.39 s; 20 parallel and 5 serial probe runs all passed.
  - The workspace (no skip) gave 1403 passed, 0 failed in 125 suites; the testkit gave 202 passed.
  - clippy, lint, changelog-check, machete, rustfmt, docs_cli_test and wire_selftest all pass.
  - The AC 9, 11, 12 and 14 checks all hold.
  - Details are in `handoff-T-green.md`.

## A (Phase 3, up-front plan review, third pass) — 2026-10-09T19:28:00-06:00
- **Decided:** BLOCK on `docs/handoffs/663-brief.md` at `93fb653` (the brief is unchanged since `d7e0421`), with one block
  and three warns, B-2 and W-13 to W-15 (see `handoff-A.md`, which replaces the second pass's PASS at `2dad8bf`).
  - **The re-entry.** The outside diff gate blocked (round 1) and F reported `archChanged: true`, so the driver routed the
    run back to Phase 3.
  - **The block (B-2).**
    - **On main:** #701 merged at 18:56 MDT, after the brief's last amendment. It put
      `holler_pane::findings::doctor_command` on main, documented as the reconcile step's builder ("rather than spelling it
      again").
    - **The review record:** #647's A-dup, D-3, names #663's `RECONCILE_STEP_UNSCOPED` and `reconcile_step` as the copy.
      It says each later story's A-dup gate should reject its own copy.
    - **The plan:** it still builds a second spelling (Decision 8, AC 5), writes it into ADR-0021 (AC 14e), and promises it
      to #644 (F5). AC 14e's "until #647 gives `pane doctor` a pane positional" is false on main.
    - **Why F cannot fix it in this run:** the base `3bdd129` has no `doctor_command`, and D-3 makes the choice O's.
  - **The diff gate's blocks.** B-1's remediation contradicts Decision 15's pid-reuse rule, so F's no-signal path stands,
    and the brief should state the rule (W-13). B-2's suggested `saturating_add` does not exist for `Instant` (W-14).
  - **W-15:** AC 10's 600-line rule leaves no room in `profile_scope.rs`, which is at 597 lines.
- **Assumed:**
  - The brief's Reuse map table is the Reuse map, as in the earlier passes.
  - #644's brief at `7195993` is current. Its branch has not moved, and its issue was last updated at 17:40 MDT, before
    #701 merged.
  - #647's D-3 has not been acted on: neither the #663 nor the #644 brief mentions `doctor_command`, and neither issue
    does.
- **Hedged:**
  - **B-2 is a block, not a warn.** The copied code is one literal. But:
    - the plan writes the copy into the standing spec and into a cross-story contract;
    - the review record on main already marks it for rejection at Phase 7;
    - every fix needs a rebase and an O decision, which F cannot make.

    A PASS would have sent F into a rework it cannot finish, and then into a predictable A-dup BLOCK.
  - **W-13 is a warn.** The code follows the brief's rule; only the brief's text is silent.
  - **B-2 does not ask to name the pane.** The profile-scoped step is right on main, because the pane forms refuse a pane
    with no record. B-2 asks only to restate the reason.
- **Evidence:**
  - **On `origin/main` (`ce12cdb`):**
    - `findings.rs:12-18, 36, 303-316` and `reconcile.rs:219-242`;
    - `doctor.rs` (the `[PANE]` positional and `ReconcileRequest`) and `lib.rs:42` (`pub mod findings`);
    - ADR 0003 lines 61 and 68, and the ADR-0021 diff `3bdd129..ce12cdb`;
    - `docs/handoffs/647/handoff-A.md:31, 76-86, 200-206`, `handoff-A-dup.md:38, 105-118` and `handoff-S.md:258-268`.
  - **On the branch:**
    - the code: `findings.rs` (a 3-line stub), `profile_scope.rs:1-300` and `probe.rs` in full;
    - the ADR-0021 diff;
    - `handoff-F.md`, `handoff-T-green.md`, `evidence.md` and `663-diff-result-r1.md`.
  - **std 1.98.1 (rust-docs):** the `Child::try_wait` source, `ExitStatus::code`'s doc and `Instant`'s method list.
  - **Grepped `crates/*/src` on main** for POSIX quoting helpers, `holler pane doctor` literals, and `SIGCHLD` and
    `waitpid`.
  - **Merge checks against `ce12cdb`:**
    - a three-way `git merge-file` of ADR-0021 (base, branch, main) gave no conflict;
    - `git merge-tree` showed only `CHANGELOG.md` conflicting.
  - **The driver's routing:** `coding-pipeline.workflow.mjs:1179` and `:4642`, and `decideArchReReview`.

## A (Phase 3, up-front plan review, fourth pass) — 2026-10-09T20:07:00-06:00
- **Decided:** PASS on `docs/handoffs/663-brief.md` at `9762a97`, with no block and three warns, W-16 to W-18 (see
  `handoff-A.md`, which replaces the third pass's BLOCK at `71f9ae2`).
  - **B-2 is resolved.** Both reconcile-step forms are built on `findings::doctor_command(None, false)`. The profile form
    composes on top in `profile_scope.rs`, and the const is gone. AC 5's greps, AC 14e and F5 agree. W-13 to W-15 are
    resolved as asked.
  - **W-16:** #705 (`dc300ab`, 19:43 MDT) merged a private `kill_group`/`run` in `holler-adapter-opencode/src/exec.rs`.
    - `holler-pane` cannot use it (dependency direction; it is `pub(crate)`), so the plan's private runner stands, and
      #696 plans the fold.
    - #696 must settle three divergences: the pgid 0/1 guard, a signal after a `try_wait` error, and the program named in
      errors.
    - #705's macOS CI leg is the first macOS run of the same `kill` form, and it passed.
  - **W-17:** a profile name that starts with `-` makes both printed commands fail to parse (clap 4.6.6, observed). The
    root is the `ProfileName` grammar, so this is a follow-up, ideally before #662b's `create` merges.
  - **W-18:** #662b plans a byte-identical `shell_word` in `profile/delete.rs`, which its brief accepts as a follow-up.
    The pre-ruling: no Phase 7 block for #663 either way (fold into a shared non-verb module, F6). F5's #646 citation moved
    to `ec2b6b3:521-526`.
- **Assumed:**
  - The brief's Reuse map table is the Reuse map, as in the earlier passes.
  - The in-flight branches are current as read: #644 at `7195993` (brief only), #646 at `ec2b6b3` (brief only, 646b
    after #663), and #662b at `76c774f` (T-red).
  - #705's macOS leg ran the test it reports: its log shows `serve_kills_its_process_group_when_the_deadline_passes ... ok`.
- **Hedged:**
  - **W-16 is a warn, not a block.** Reuse is impossible across that dependency direction, and #696 is the planned fold. The
    brief does justify a new object in writing; only its premise is out of date.
  - **W-17 is a warn.** Nothing harmful runs (clap refuses to parse), such names are unusual, and the fix is upstream of
    this story's blast radius. Changing the step's text now would ripple into AC 2-5, ADR step 6 and #644.
  - **W-18 is a warn.** The copy is another story's, its brief already records it, and folding it into a verb file would
    invert the layering.
- **Evidence:**
  - **The brief:** read in full at `9762a97`, with the diff from `d7e0421`, the outside brief gate's rounds r1 and r2
    (r2 PASS), and the third pass's handoff and journal entry.
  - **On the branch:** `profile_scope.rs` (whole) and `probe.rs:1-305`. The ADR-0021 diff against main, and its section
    12 text. The C2 quotes: `lib.rs:42`, `findings.rs:12-18, 36, 303-316`, `reconcile.rs:219-243`, `ADR-0003.md:61-68`
    and `647/handoff-A-dup.md:38`.
  - **On `origin/main` (`dc300ab`):**
    - `holler-adapter-opencode/src/exec.rs` (whole) and `server.rs` (lines 1-120 and 170-189);
    - `tests/hermetic_test.rs` (lines 1-40 and 620-690);
    - PR #705's checks, and the macOS job log (run `38013074383`, job `114099775442`).
  - **Grepped `crates/*/src` on main and the branch** for:
    - group-kill and `process_group` code;
    - `holler profile show` and `holler pane doctor` literals, and `to reconcile`;
    - POSIX quoting helpers (the `'\''` escape, `quote_id`);
    - `SIGCHLD`/`waitpid`, and `catch_unwind`/`AssertUnwindSafe`.
  - **Clap:** `holler-cli/src/pane/args.rs:21-25` and `ProfileName::parse`/`slugify`. A scratch clap 4.6.6 crate in the
    session scratchpad parsed `doctor --profile -Demo`, `--profile --fix`, `--profile=-Demo`, `show -Demo` and
    `show -- -Demo`.
  - **The in-flight briefs:** #662b (lines 20-40 and 1300-1330, Decision B1), #646 at `ec2b6b3` (lines 31 and 506-528)
    and #644 at `7195993` (its section 12 plan).
  - **Checks:** `git merge-tree` against `dc300ab` conflicts in `CHANGELOG.md` only. AC 5, 11, 12 and 14's commands were
    run at `9762a97`.

## T (Phase 4, author / RED, re-entry) — 2026-10-09T20:10:42-06:00
- **Decided:** the re-entry RED is valid, as the brief's Test plan lays it out.
  - T amended AC 5's test `reconcile_step_single_quotes_the_profile_name` to the `Option` signature. It adds two
    assertions: `reconcile_step(None)` equals #644's exact text, and it equals `format!("to reconcile, run {}",
    findings::doctor_command(None, false))`.
  - T landed Decision 8's signature as a stub (`String::new()`), deleted `RECONCILE_STEP_UNSCOPED`, and passed `Some(..)` at
    the scope's two call sites. That is the only production change, and it exists so the RED is not a compile error.
  - RED: 4 of 7 scope tests fail on the reconcile step (AC 5 on the equality, AC 2-4 on the missing
    `holler pane doctor --profile 'Demo Alpha'` substring). AC 5's `doctor_command(None, false)` grep prints `0`, and AC 14's
    ADR grep prints `3`.
- **Assumed:**
  - Overwriting `handoff-T-red.md` is right: the previous run's RED handoff stays in git history at `8fe683b`.
  - AC 5's other two greps (`holler pane doctor` and `RECONCILE_STEP_UNSCOPED` on production lines) are already `0` on the
    stub, because the Test plan has T delete the const. That is not a defect in the RED: the `doctor_command` grep and the
    equalities carry it.
- **Hedged:**
  - **The regression guards are green on purpose:** AC 1, AC 6, AC 7, the 12 probe tests (AC 8a-8l) and AC 8m. The amendment
    states what the probe code already does (Decisions 13-15 and 19), so no probe test changes.
  - **No new test for W-17** (a profile name with a leading `-`). The brief keeps the step's text, and the fix is a
    follow-up upstream in `ProfileName::parse`.
- **Evidence:**
  - `cargo test -p holler-cli --lib pane::profile_scope`: `3 passed; 4 failed`, with the failure lines in `handoff-T-red.md`.
  - `cargo test -p holler-pane --lib probe::tests`: `12 passed`. `ports_test run_probe_stub_never_reports_success`:
    `1 passed`.
  - `cargo clippy -p holler-pane -p holler-cli --all-targets -- -D warnings`, `bash scripts/lint.sh` and `rustfmt --check
    --edition 2021` on both files all exit 0. `profile_scope.rs` is 592 lines.
  - No `hlr-probe-663-*` directory and no `sleep 30` was left behind.

## F (Phase 6, implementation, re-entry) — 2026-10-09T20:23:39-06:00
- **Decided:**
  - **Decision 8, built.** `reconcile_step(profile: Option<&ProfileName>)` starts from
    `holler_pane::findings::doctor_command(None, false)` for both forms. `None` gives `to reconcile, run holler pane
    doctor`. `Some(P)` adds `--profile '<P>' and then holler profile show '<P>'`, which is byte-identical to the
    previous run's text.
    - T's stub, its rustdoc and its `let _` line are replaced.
    - The rustdoc gives C9's reason that the step names no pane, says verbs call the function (F5), and takes W-17's
      sentence.
  - **AC 14's ADR text.**
    - a gains the exceptions sentence.
    - e's step 6 is re-worded in place: the builder (#701), the no-pane reason and `reconcile_step(None)`.
    - The fence bullet's parenthesis now points at step 6's reason.
    - f extends section 12's last sentence with the probe runner's three cases.
    - b, c and d already matched and are unchanged.
  - **One rustdoc bullet on `run_probe`** for the `try_wait`-error rule (no signal, `Error`, a child still in the group
    is left). The public doc now names all three exceptions the ADR calls "documented". No code changed.
  - **W-16's evidence recorded.** `evidence.md` now has #705's `kill` form, its group setup and its test (from
    `dc300ab`), and the macOS job's log line. Three facts about the reconcile step's builder are added too.
  - **The tests are not edited.** Both test modules hash the same as at `f79cd05`. `archChanged: true`: this cycle
    changed `reconcile_step`'s signature and removed its const (T's stub, F's body), and it amends the ADR.
- **Assumed:**
  - `cargo doc` is not a CI gate, and it prints no warning for the changed files anyway.
  - The merge step resolves the `CHANGELOG.md` conflict with `origin/main` (`e327569`). F does not merge.
- **Hedged:**
  - **`profile_scope.rs` is at 605 lines,** past the 600-line warning. AC 10 accepts this, and the full rustdoc is worth
    more than the warning for a function other stories call.
  - **The `probe.rs` bullet goes beyond the brief,** which expected that file untouched in this run. It is rustdoc only.
  - **The `dc300ab` evidence is not on the branch,** so the diff gate may not attach it. The CI log is not a repo file.
- **Evidence:**
  - **Scope tests:** `cargo test -p holler-cli --lib pane::profile_scope` gives 7 passed (RED: 3 passed, 4 failed).
  - **Probe tests:** `probe::tests` gives 12 passed (1.39 s serially), and `ports_test run_probe_stub_never_reports_success`
    gives 1 passed.
  - **Workspace:** `HOLLER_STATE_DIR=<scratch> cargo test --workspace --no-fail-fast` with CI's skip gives 1494 passed and
    0 failed in 131 suites.
  - **Gates:** clippy `-D warnings`, `lint.sh` (one accepted warn), `changelog-check.sh`, `cargo machete`,
    `rustfmt --check --edition 2021` on both files and `docs_cli_test` all pass.
  - **AC 5, 9, 11, 12 and 14 checks** hold on the working tree against the merge base `0ad2d8a`.
  - **Sources:** the macOS log via `gh run view 38013074383 --job 114099775442 --log` (line 641), and #706's
    `holler-adapter-host/src/exec.rs:202` at `e327569`. Details are in `handoff-F.md` and `evidence.md`.

## T (Phase 7, verify / GREEN, re-entry) — 2026-10-09T20:31:14-06:00
- **Decided:**
  - **GREEN is valid.** The 7 scope tests, the 12 probe tests and 8m pass. Both test modules hash the same as at the RED
    commit `f79cd05`. F flagged no test as wrong and I found none, so no test was repaired.
  - **The amended test pins behaviour.** Four mutations of `reconcile_step` (the bare form, the quoting, the
    `profile show` half, `doctor_command`'s `fix` argument) each turn at least one authored test red. The reuse of
    `doctor_command` itself is pinned by AC 5's source grep, not a test, because the output does not change.
  - **No blocking issue.** Tier 1 and the AC 5, 9, 11, 12 and 14 checks all pass.
- **Assumed:**
  - `cargo doc` is not a CI gate. Its 7 warnings are all in `cli.rs`, which is not in the diff.
  - The `CHANGELOG.md` conflict with `origin/main` (`e327569`) is resolved by the merge step, not T.
- **Hedged:**
  - `profile_scope.rs` is at 605 lines, which is the accepted `lint.sh` warn (AC 10), journalled here.
  - The probe tests' macOS timing and BSD `kill` are covered by CI's macOS job; on this Linux host they passed 10 of 10
    more runs.
- **Evidence:**
  - `cargo test -p holler-cli --lib pane::profile_scope`: 7 passed. `probe::tests`: 12 passed (0.53 s; 1.39 s serially).
  - `HOLLER_STATE_DIR=<scratch> cargo test --workspace --no-fail-fast`: exit 0, 131 suites, 1495 passed, 0 failed.
  - clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, `rustfmt --check`, `docs_cli_test`,
    `wire_selftest` and the testkit all pass.
  - F's 7 new evidence excerpts match the source at the cited lines (branch, and `dc300ab`). Details are in
    `handoff-T-green.md`.

## A (Phase 7, anti-duplication gate) — 2026-10-09T21:45:12-06:00
- **Decided:** PASS on the diff `0ad2d8a..d9210bc`, with no block and two warns, D-1 and D-2 (see `handoff-A-dup.md`). This
  cycle is `e46b427..d9210bc`. The gate also reviewed the whole feature diff, because no earlier run reached it.
  - **F extended every object the map named.** There is one real `ProfileScope` and one `run_probe`. The reconcile step is
    built on `doctor_command`, so `holler pane doctor` is spelled in production only at `findings.rs:36`. Every
    deliberate copy is one the brief justifies in writing: the fake's rules, `check_joins` (W-5), the private runner
    (W-16), `single_quoted` (W-18), and the probe tests' scratch directory and poll (W-11).
  - **D-1:** `StoreScope`'s private `belongs` (`profile_scope.rs:218-222`) is a line-for-line copy of the public
    `holler_pane::profile_diff::is_member` (#703). Every other CLI caller of the rule uses `is_member`: profile list,
    profile show, and #709's `pane watch`. `pane watch` mixes `resolve`'s member set with `is_member`. The fold is 5 lines:
    F takes it in any rework this run, else O adds it to F2, and the fake's copy to F1.
  - **D-2:** #706 merged a second private bounded runner (`holler-adapter-host/src/exec.rs`). W-16's ruling stands, and
    #696 gets six more differences to settle, (d) to (i).
- **Assumed:**
  - The brief's Reuse map table is the Reuse map, as at Phase 3.
  - "The analogous object the map named" bounds a Phase 7 block, as `workflow-coding-pipeline.md` ("A's anti-duplication
    gate") and the role's verdict rules state it. A copy of an object the map omitted is therefore a warn.
  - The Workflow script commits this handoff and this entry, as it committed the earlier phases'.
- **Hedged:**
  - **D-1 is a warn, not a block.** It is a real near-copy of a documented single rule, and the fold is cheap. But F followed
    the map, and the brief's written reason to re-implement the fake's rules covers this function. Main moved under the
    branch: F wrote `belongs` at `2d9c7a0` (19:06 MDT), `is_member` reached main at `ce12cdb` (19:13 MDT), and the branch
    merged it at `1d6a5ab` (19:34 MDT). The re-based brief and A's fourth Phase 3 pass both missed it, and the miss is A's.
    The two functions are identical today, so nothing is wrong yet.
  - **D-2 is a warn.** The dependency direction rules out reuse, and #696 is the planned fold.
- **Evidence:**
  - **The diffs:** `git diff e46b427 HEAD` and `git diff 0ad2d8a HEAD`, for the crates, the ADR and the CHANGELOG.
  - **Read in full:** `profile_scope.rs` (605 lines) and `probe.rs` (577 lines); the brief's Evidence, ACs, Decisions,
    Reuse map and Follow-ups; `handoff-A.md`, `handoff-F.md`, `handoff-T-green.md` and this journal.
  - **On the branch:** `profile_diff.rs:1-40, 245-265` and `lib.rs:40-76`; `profile/{list,show}.rs`; the fake's
    `profile_scope.rs:92-126, 234-239`; the hub's profile not-found sites; and ADR-0021 section 3 (line 165).
  - **On `origin/main` (`519947a`):**
    - `pane/{list,get,watch}.rs` (#709);
    - `holler-adapter-host/src/exec.rs` (#706, whole) and the function lines of `holler-adapter-opencode/src/exec.rs`;
    - the public items added since `3bdd129`.
  - **Greps over `crates`, on both refs:**
    - slug comparisons and `is_member` callers;
    - `ProfileNotFound`/`PaneNotInProfile` producers;
    - spec-edit helpers;
    - the `'\''` escape and shell-quote function names;
    - `holler pane doctor`, `holler profile` and `to reconcile` literals;
    - context-append helpers;
    - `try_wait`, `process_group`, `recv_timeout`, `checked_add` and `.windows(`;
    - `impl ProfileScope for` and `impl Prober for`.
  - **Checks:** AC 5's three greps (0, 1, 0); `git merge-tree --write-tree HEAD origin/main` (conflicts in `CHANGELOG.md`
    only); and the commit times of `2d9c7a0`, `ce12cdb`, `1d6a5ab`, `efd9a00` and `e327569`.
  - **Calibration:** the earlier A-dup handoffs for #508, #640 and #647; `workflow-coding-pipeline.md:737-783`.
