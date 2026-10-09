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
