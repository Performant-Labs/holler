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
