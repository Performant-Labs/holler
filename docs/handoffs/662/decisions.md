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
