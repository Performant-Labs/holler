# Decisions — #662b the profile write verbs (`holler profile create`, `holler profile delete`)

## A (Phase 3, up-front plan review) — 2026-10-09T19:48:00-06:00
- **Decided:** PASS on `docs/handoffs/662-brief.md` at `79fc998` (sha256 `c8ac6f6a5ec99f39...`), with 0 blocks and 4 warns
  (see `handoff-A.md`).
  - The objects and layers are right:
    - thin CLI verbs over the two store ports, as in `list` and `show`;
    - `emit`, the closed codes and `class_of`;
    - reuse of `profile_from_panes`, `is_member` and `count`;
    - the write orders of ADR-0021 section 8;
    - no frozen file or manifest touched.
  - W-1, the most consequential:
    - B7 and the Forward-compat row promise #650 `insert_profile`.
    - #650's engine is in `holler-pane` (ADR-0021 section 5 and the `import.rs` stub), which cannot import `holler-cli`.
    - The Follow-up should be "move it down", not "follow the pattern".
  - W-2: two widened meanings that AC 6 keeps out of ADR-0021:
    - Decision 7 widens `profile-conflict` beyond ADR-0021's "after the live change (I8)";
    - `delete --keep-panes` is an exception to I1 that the ADR does not record.
  - W-3: name the helper `single_quoted`, #663's name, and give it a documented home, so the later fold is a move.
  - W-4: put the N-th-call seam in `rig.rs`, not in `create.rs`.
- **Assumed:**
  - #663's branch and its brief's list of ADR-0021 edits are #663's current plan. The branch is `issue-663-implementation`
    at `1d6a5ab` and is not merged. I used them as evidence of intent, not as patterns.
  - #650 will build its engine in `holler-pane/src/import.rs`, as ADR-0021 section 5 and the stub's doc say. #650's brief is
    not written yet.
- **Hedged:**
  - W-1 is a warn, not a block:
    - 662b's code is the same under the fix; only the record changes;
    - putting the helper in `holler-pane` now would need a file outside the issue's blast radius.
  - W-2 is a warn, not a block, for four reasons:
    - the issue lists `profile-conflict` among these verbs' codes;
    - C2 records the decision to use it;
    - the wider meaning matches section 8 step 6's own use (a compensation that lost after part of the work landed);
    - the defect is two sentences of ADR text that change no code and no test.

    The overlay's rule is that the ADR is updated in the same change, so the MO may still prefer to relax AC 6 now.
  - W-3 and W-4 are future duplication between plans that have not merged, so by the role rule they are warns. At Phase 7 I
    will reject a second copy of anything that is on `main` by then.
- **Evidence:**
  - Read in full:
    - the brief and the outside model's result for round 1;
    - `holler-cli/src/profile/{mod,list,show,create,delete,apply,import}.rs`, `pane/{doctor,import,profile_scope}.rs` and
      `output.rs`;
    - `holler-pane/src/{profile,profile_snapshot,ports}.rs` and the stubs `import.rs` and `tx_*.rs`;
    - `holler-pane-testkit/src/fault.rs` and `tests/profile_verbs/rig.rs`;
    - ADR-0021 sections 3, 5, 8 and 9 and "Decisions taken", and ADR 0003's verb rows.
  - Also read:
    - `holler-hub/src/profile/store.rs:186-258`, `docs_cli_test.rs:130-157` and `scripts/lint.sh`;
    - 662a's `handoff-A.md` (`31062ce`).
  - Ran:
    - `gh issue view 650` and `gh pr list`;
    - `git grep` on the #644, #646 and #663 branches;
    - `grep` for helpers that quote for a shell, for `PaneStore` implementations, for writes of `Pane.profile` and for code
      that builds an `Actor`.

## T (Phase 4, author / RED) — 2026-10-09T19:58:00-06:00
- **Decided:**
  - **The tests.**
    - 25 new tests: 15 in `tests/profile_verbs/create.rs` and 11 in `delete.rs`. They cover every AC 2a-2m and 3a-3h
      name.
    - Four tests beyond the ACs:
      - `create --from-current` of no panes (`members: none`);
      - a pane that already names NAME's slug joins;
      - the second detach fails, giving `detached so far: demo-c1r1` (A's W-4 gap);
      - `'\''` quoting (B1).
  - **The seam.** The N-th `cas_put` seam is `rig.rs`'s `NthCasPut` / `NthPut`, not local to `create.rs` (A's W-4), and
    `delete.rs` uses it as well.
  - **The rig.** I added `Ran`, `run_both_with`, `run_both_seamed`, `Rig::run_over` and `assert_message_contains`.
    `run_both` delegates and behaves the same.
  - **The signature stubs.** I landed only the clap structs in `src/profile/{create,delete}.rs`, which the brief's Test
    plan directs. I left out the `insert_profile` and `shell_word` stubs: no test calls them, and unused `pub(crate)` stubs
    would fail `clippy -D warnings`.
  - **The surface files.** I deleted the two STUBS entries, kept `// #662`, and set the `# #662` fixture group to the AC 5
    list.
- **Assumed:**
  - B2's undo order: the `get` of the failed pane, then the joined panes in reverse, then the profile delete. AC 2i's
    "3rd `cas_put` = undo of pane 1" depends on that order.
  - The plain `create` text is the first line only. The carried text adds a second line only for `--from` and
    `--from-current`.
- **Hedged:**
  - The decode half of AC 2j overlaps `holler-pane/tests/argv_env_test.rs:96`. I kept it because AC 2j asks for it, and
    it decodes the verb's own output.
  - `create_flags_conflict` passes before F, as a clap property, as the brief predicts.
- **Evidence:**
  - `cargo test -p holler-cli --test profile_verbs` gave 18 passed and 25 failed, identically in 6 runs. Every failure is
    `not-implemented` (exit 1, or code `not-implemented`) against the expected exit or code.
  - `cli_surface_test` ok (3). `pane_cli_process` ok (34).
  - `docs_cli_test` FAILED only on ADR-0003:65-66 (F's AC 5 rows). This is the intended RED.
  - `cargo clippy -p holler-cli --all-targets -- -D warnings` is clean. `bash scripts/lint.sh` exits 0.
    `rustfmt --check --edition 2021` on the six touched `.rs` files exits 0.

## F (Phase 5, implement) — 2026-10-09T20:12:30-06:00
- **Decided:**
  - **Shape.** Both verbs run a plan, then the writes. The plan (`Result<Plan, PaneError>`) runs every B3 check, in
    B3's order, before the first write. The writes produce the result, and `run` prints it once through `emit`.
  - **No CLI types in the reusable writes (A's W-1).** `insert_profile`, `join`, `undo` and `detach` take the store
    ports and return `PaneError`, or for `join` the plain value `JoinFailed`. Only `JoinFailed::body` and `run` build an
    `ErrorBody`.
  - **`JoinFailed` holds no profile name.** The caller passes the name to `body`. Clippy's `result_large_err` rejected
    the struct with the name in it: 144 bytes against a threshold of 128. I dropped the field rather than boxing
    because it only repeated a value the caller holds.
  - **The quoting helper is `single_quoted`, B1's body under #663's name (A's W-3).** It is not the brief's
    `shell_word`. Its home is `delete.rs`, `pub(crate)`. `delete.rs`'s module doc says it is shared, as `list.rs`'s
    does for `count`. `list.rs` is outside the blast radius.
  - **Two more shared helpers in `delete.rs`, so neither verb copies the other.**
    - `pane_list` is B4's `<list>` and `create`'s `members:` line.
    - `detach` is the one compare-and-swap that clears `Pane.profile`. `delete --keep-panes` and `create`'s undo both
      use it.
  - **The undo clears `profile` to none (B2), not to its earlier value.** A pane that named NAME's slug before the
    create named a profile that did not exist (`get(NAME)` was `None`), and the undo deletes NAME again.
  - **ADR-0021 changes in the two section 9 rows only (AC 6).** A's W-2 is left for a follow-up: it widens
    `profile-conflict`'s definition and records `--keep-panes` as an I1 exception.
- **Assumed:**
  - A store error that B4 does not map passes through with its own code and text, as the carried text says. Two cases:
    - `insert_profile` answers `timeout` after its write landed;
    - a non-`Conflict` error from `delete`'s final profile delete after some detaches.
  - Running the verb again converges in both cases (`profile-exists`, or a plain delete).
- **Hedged:**
  - **`archChanged: true`.** There is no new module and no public interface change. But this cycle adds four
    `pub(crate)` helpers and a `create -> delete` edge inside `profile/`, and two of the helpers are beyond the brief's
    Reuse map. The role rule says to report `true` when in doubt.
  - **`cargo test --workspace` fails on this machine for reasons outside this change.**
    - *Plain run:* it stops at `holler-cli --test logging_test` (4 failures). Those tests expect `holler roster` to exit
      1 with no hub, and the shell's `HOLLER_STATE_DIR` points at a live hub.
    - *Isolated run:* with `HOLLER_STATE_DIR` set to an empty directory, the result is 1463 passed, 2 failed, 5
      ignored across 128 targets, and `logging_test` passes 11 of 11.
    - *The 2 failures* are both in `interrupt_test`. The hub's 45 s liveness check expires a few milliseconds after
      the body connects, in unchanged hub and body code that no profile verb reaches. Run alone, the same isolated
      way, `interrupt_test` passed 11 of 11 three times in a row: a timing flake under the parallel run.
- **Evidence:**
  - `cargo test -p holler-cli --test profile_verbs`: 43 passed, 0 failed. RED before F was 18 passed, 25 failed.
  - `cli_surface_test` 3 passed, `docs_cli_test` 3 passed (ADR-0003 lines 65-66 now parse), `pane_cli_process` 34
    passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` is clean.
  - `bash scripts/lint.sh` exits 0: `create.rs` has 346 lines and `delete.rs` 218.
  - `rustfmt --check --edition 2021` on both source files exits 0. `bash scripts/changelog-check.sh` prints ok.
  - AC 1's grep prints nothing. AC 6's two greps print one line each, and the ADR-0021 diff is the two rows. AC 9 shows
    no `unsafe` and no manifest change. AC 12's file list is the brief's Files.
  - 16 unchanged-code facts are in `docs/handoffs/662/evidence.md`.

## T (Phase 6, verify / GREEN) — 2026-10-09T20:23:17-06:00

- **Decided:**
  - **GREEN with no blocking issue.** `profile_verbs` passed 44 of 44 in 10 runs in a row. The isolated workspace run
    had 1464 passed and 0 failed. Clippy, lint, changelog, machete, the canary, the hooks and fmt are all clean.
  - **Added one test**, `create_from_current_undoes_the_joined_panes_newest_first`. B2 step (2)'s reverse order
    survived a mutation (M6: undo oldest first, and all 43 tests still passed), so the order the brief requires was
    not pinned. The new test kills M6. No other test was changed, and no production code was changed.
  - **Added three test-kit facts to `evidence.md`**: `list` is sorted by name, `fail_next` fires once, and the call
    log includes failed calls. The tests rely on all three, and F did not list them.
- **Assumed:**
  - An isolated `HOLLER_STATE_DIR` is the right way to run `cargo test --workspace` on this machine, because the shell
    points at a live hub. F found this; CI has no live hub.
- **Hedged:**
  - F's 2 `interrupt_test` failures did not reproduce in T's isolated run (0 failures). T treats them as the flake F
    diagnosed, in hub liveness code that this story does not touch. T did not investigate them further.
- **Evidence:**
  - The 9-mutation table in `handoff-T-green.md`. Each mutation was reverted with `git checkout`, and `git status`
    shows only T's staged test file plus the handoff files.
  - The AC 1, 4, 6, 9 and 12 greps and diffs, reproduced as F reported them.

## A (Phase 7, anti-duplication gate) — 2026-10-09T20:31:59-06:00
- **Decided:** PASS on `ce12cdb..5396f54`, with 0 blocks and 5 warns (see `handoff-A-dup.md`).
  - F reused every object the Reuse map named. That covers `emit` and the output types, the closed codes,
    `profile_from_panes` for every record, `is_member` in all three places, `count` and the rig.
  - The new objects are `insert_profile`, `single_quoted` and the `NthCasPut` seam, all justified in the brief. `detach`
    and `pane_list` are each defined once and used by both verbs.
  - Nothing on `main` has a counterpart to any of these, and nothing on `main` is copied.
  - Phase 3's W-1, W-3 and W-4 are applied.
  - The warns:
    - W-1: the profile lookup is written three times, and the members filter twice, in `profile/`. #664 is the next
      consumer.
    - W-2: the suggested command lines are written in more than one place, against `doctor_command`'s
      one-builder pattern.
    - W-3: #663's private `single_quoted` is still there, so a second copy reaches `main` with whichever story merges
      second. `profile/` also has two shared-helper homes.
    - W-4: Phase 3's W-2 (the ADR-0021 `profile-conflict` meaning and the I1 exception for `--keep-panes`) is still open,
      and no issue is filed.
    - W-5: the `stored` test helper is defined twice, and the two test files count writes in two ways.
- **Assumed:**
  - `main` means `origin/main` at `e327569`. Its three commits past the base touch only the adapter crates.
  - #663's state is the local branch `issue-663-implementation` at `4bbe607` (F done, not merged). The remote branch is
    still at its brief.
- **Hedged:**
  - W-1 to W-3 are warns, not blocks, for three reasons. `main` has no object to extend: the lookup is inline in the
    private `view` of `show.rs`, a file this run may not touch. `findings.rs` builds only pane-verb commands. #663's copy
    has not merged.
  - W-4 stays a warn as at Phase 3. The brief records the wider meaning (C2, Decision 7), and AC 6 forbids the edit. The
    MO should settle it before merge.
- **Evidence:**
  - Read in full:
    - the diff;
    - `src/profile/{create,delete,show,list}.rs`;
    - `tests/profile_verbs/{create,delete,rig,show}.rs`;
    - `holler-pane/src/findings.rs`;
    - the handoffs from A, F and T-green, and the brief's Reuse map and Forward-compat.
  - Read in part: the hub's `profile/rename.rs` and the test kit's conformance mutants.
  - Grepped `crates/*/src` and `origin/main` for:
    - POSIX quoting helpers, `", "` joins, `"none"` and `ProfileNotFound {`;
    - `PaneInOtherProfile {`, writes of `Pane.profile`, `cas_put(.., 0, ..)` and `impl PaneStore for`;
    - the `FaultSwitch` API, `fn argv(` and hand-built `ErrorBody`.
  - Grepped #663 at `4bbe607` for `single_quoted`, `reconcile_step`, `with_context`, `belongs` and `stored`.
  - Checked file sizes, `#[allow]` additions, frozen files and personal names, and searched GitHub for filed
    follow-ups (none).

## S (Phase 8, spec audit) — 2026-10-09T21:38:22-06:00
- **Decided:** PASS (see `handoff-S.md`).
  - The A precondition is met: A and A-dup both PASS. The T precondition is met: RED was 18/25 with every failure an
    assertion, GREEN is 44/44 and the workspace 1464/0, and there are no blocking issues.
  - Each issue criterion and each of the brief's ACs 1-12 (2a-2m, 3a-3h) has a test or a check that proves its
    behaviour.
  - B1-B8 and the carried decisions are built as stated. I checked the order of the checks, all 11 B4 messages and the
    undo order against the code. Four changes from the brief are documented, none silent:
    - the helper is `single_quoted`, not `shell_word`;
    - `detach` and `pane_list` are shared helpers;
    - the seam is in `rig.rs`;
    - the actor's error goes through the one `emit` call.
  - The quality checks are clean: guards, sizes, no `#[allow]`, no `unsafe`, no manifest change, privacy, gitleaks and
    the docs.
- **Assumed:**
  - The live issue and epic texts, and their GraphQL edit times, are authoritative. #662 last changed at 2026-10-08
    18:56 MDT and #633 at 2026-10-09 17:39 MDT, both before the brief (19:28 MDT).
  - I relied on T's recorded results for the Tier 1 and Tier 2 commands and did not re-run them, per S's role. Only docs
    changed after T-green (`git diff --stat 5396f54..HEAD`).
- **Hedged:**
  - The ADR-0021 gap is A's W-2 (Phase 3) and W-4 (Phase 7): the `profile-conflict` Reason at line 395 is narrower than
    its new use, and section 3 does not record `--keep-panes`. This could have been an ADVISORY-HOLD on AC 6, since the
    architecture overlay asks for the ADR edit "in the same change". I chose PASS with advisory 1 for three reasons:
    - A rated it a warn twice and named the follow-up as an acceptable route;
    - the issue itself lists `profile-conflict` among these verbs' codes;
    - the real binary answers `not-implemented` until #649, so no script can see the code yet.

    The MO files the follow-up, or relaxes AC 6, before merge. No such issue exists yet: I searched GitHub for
    `profile-conflict` and `keep-panes`.
  - AC 2l's test runs `create Fresh --from "   "`, not the AC's `Demo`. It proves the criterion. The tie between two
    errors that B3 settles is unpinned (advisory 2), so this is not REWORK.
- **Evidence:**
  - Read in full:
    - the brief;
    - the five phase handoffs, `decisions.md` and the outside model's diff review (`662-diff-result-r1.md`, PASS);
    - `src/profile/{create,delete}.rs` and `tests/profile_verbs/{create,delete}.rs`;
    - the diffs of `rig.rs`, `stub.rs`, the fixture, ADR-0003, ADR-0021 and the CHANGELOG.
  - Read in part: `pane/wiring.rs` (`Unwired` answers `NotImplemented`), and ADR-0021 sections 8 and 9 and lines 149 and
    161.
  - Read-only checks I ran:
    - the AC 1, 4, 5 (column 67), 6, 9 and 12 greps and diffs;
    - `wc -l` on the touched files;
    - greps for `unwrap`/`expect`/`panic`, `#[allow]`, sleeps and debug prints;
    - a privacy grep of the whole diff;
    - `gitleaks git --log-opts=origin/main..HEAD` (6 commits, no leaks);
    - `gh issue view` and GraphQL edit history for #662 and #633;
    - `gh issue list` searches for a filed follow-up;
    - a read-only `git merge-tree ce12cdb origin/main HEAD` (one conflict, in `CHANGELOG.md`).
