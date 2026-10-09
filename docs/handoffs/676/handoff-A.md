# Handoff-A: Phase 3 - #676 pane and profile verbs exit 3 on a refusal, 1 on a failure  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-676-implementation (at 31fec64, based on 55dba00)
**Brief reviewed:** docs/handoffs/676-brief.md   **Reuse map:** docs/handoffs/676-brief.md §Files "Reuse map"   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS. The plan extends the right object. `ErrorClass`, `ErrorClass::exit_code` and `class_of` go into
`holler-pane/src/error.rs`, which is the crate's one home for its error vocabulary (`lib.rs:58-61`), next to the single
`PaneCode` table and `is_valid_code`. The lookup reuses `PaneCode::parse` and `is_valid_code`, so there is no second code
list and no second validator. The CLI changes only the one place that chooses an exit code today: `output.rs::exit_code`.
`emit_text` and `emit_json` call it, and `emit_stream`, `emit_error`, `emit_usage_error` and `main.rs`'s wiring-error path
all reach it through `emit`. The CLI holds no second table.

The one exhaustive `match` over `PaneCode` with no `_` arm follows the same pattern as `as_str`, `PaneError::classify` and
`from_closed`, and the free function `class_of(&str)` follows `is_valid_code(&str)`. Dependency direction holds:

- `holler-cli` already depends on `holler-pane`.
- The test kit can call `class_of` without depending on `holler-cli` (ADR-0021 §5).
- Nothing touches `lib.rs`, a manifest, `holler-proto` or a golden file.

I weighed one choice and accepted it: ADR 0003's exit numbers now sit on `ErrorClass` in the types crate. Decision 4
justifies this in writing. #638's envelope helper must pair `ok: false` with 1, 2 or 3 (its acceptance as amended
2026-10-09), and it may not depend on `holler-cli`. It also follows the `is_valid_code` precedent: the one shared
definition lives in `holler-pane` and the CLI calls it.

All five findings are `warn`. W-1 matters most. Under the open-code rule, every open code exits 3, so the post-act mismatch
of ADR-0021's I3 ("exits 1 with a code") has no code that can carry it.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | MO decision 5 and the table row "open `Refused` code → Refusal (3)"; the §9 text that AC 4 writes | cross-cutting (error handling); ADRs | The rule sends every open code to exit 3, and the closed list cannot grow ("No story edits that list", `ADR-0021.md:320`). ADR-0021 I3 (`:162`) says a mismatch observed after `act` "exits 1 with a code" (epic #633: "a mismatch fails loudly"). No closed failure code describes that mismatch (`generation-conflict`, `profile-conflict`, `timeout`, `unavailable`, `store-corrupt`). §9's normal path for a new code (`:320-322`: declare it with `RefusalCode::from_static`, raise it as `Refused`) would make #644/#645 exit 3, which contradicts I3. The reason given for the open-code row checks only the open codes in §9's per-verb table (#645, #646), not I3. | Keep `class_of` as designed: open means Refusal, which matches the frozen names `RefusalCode` and `PaneError::Refused`. In the §9 text that AC 4 writes, add one sentence: "An open code is always a refusal (exit 3); a verb that reports a runtime failure uses a closed failure code." Add a Risks bullet for the operator that names the I3 mismatch as the known failure with no closed code. The options are to carry it as `unavailable`, to amend the closed set under the epic's amend-first rule, or to accept exit 3 and amend I3's test idea. That way #644/#645 settle it in their briefs, not at their own Phase 7. |
| 2 | warn | MO decision 7, Blast radius "Not changed: `main.rs`", and AC 3's residue list | naming / doc consistency | `crates/holler-cli/src/main.rs:253-255` (the `run_with_stdio` doc) says a pane/profile wiring error is reported "(exit 1, with the error's code)". After #676 that error exits by its class (`emit_error` → `emit` → `exit_code`). ADR-0021 §10 (`:390-392`) reads the Herdr version on connect, so once #649 fills `Wiring::connect` it can return `herdr-version-unsupported`, which is Refusal (3). AC 3 greps only for "refused or failed" and misses this line. | Add the doc lines `main.rs:253-255` to the blast radius and to AC 3's list. Change the comment only, as decision 7 does for `pane/mod.rs`, `profile/mod.rs` and `args.rs`, for example "(with the error's code, and the exit code of its class)". No code in `main.rs` changes, so decision 7's reasoning ("it applies whatever the verb returns") still holds. |
| 3 | warn | AC 4: the new sentence on ADR 0003 line 97 | pattern consistency (ADR bookkeeping) | ADR 0003 records in its header each issue that changed its rules: `**Clarified by:** #454` (`:8`) and `**Amended by:** #670` (`:9`). ADR 0005, 0006, 0007 and 0020 do the same. AC 4 changes the text of ADR 0003's exit-code rule but adds no header line, so the header would no longer list every change. | Add a header line after `:9` to AC 4, for example: `**Clarified by:** [#676](https://github.com/Performant-Labs/holler/issues/676) (2026-10-09) — the pane and profile verbs use these four exit codes the same way in text and JSON mode; holler_pane::error::class_of decides which error code is a refusal (3) and which a runtime failure (1).` A code span that starts with `holler_` is safe: `docs_cli_test.rs:121` matches only `holler ` with a space. |
| 4 | warn | New names `class_of` / `ErrorClass` | naming | `error.rs` already has `pub(crate) PaneError::classify(&self) -> Result<PaneCode, &RefusalCode>`, which sorts closed from open (`:412`). `output.rs` has a private `classify` for command-line tokens (`:419`). A near-identical name with a third meaning (usage, refusal or failure) now sits next to both. A wave-3 author who holds a `PaneError` could reach for `classify()` and expect an `ErrorClass`. Nothing else collides: no `ErrorClass` or `class_of` exists in the workspace, and `ErrorClass` avoids `clap::error::ErrorKind`, which `output.rs` imports. | Keep the names, since the ACs pin them. The doc comment on `class_of` should say that it is not `PaneError::classify` (closed or open) and that a caller holding a `PaneError` writes `class_of(err.code())`. Decision 2 adds no `PaneError::class`. |
| 5 | warn | Out of scope: the legacy verbs | naming (vocabulary) | Once #676 writes ADR-0021 §9's taxonomy, `not-implemented` is a failure (1) and "refusal" means 3. The legacy `--pane`/`--profile` stop still calls its exit 1 a refusal in `prompt_target.rs:60` ("The exit code of a refusal (ADR 0003: a runtime failure)", `REFUSED_EXIT = 1`) and `:12`, and in the doc comments at `say_cmd.rs:113`, `interrupt_cmd.rs:36` and `answer_cmd.rs:36`. The legacy CLI already uses "refusal" loosely (`token_cmd.rs:30`, `hub_cmd.rs:209-210`), so there is no dominant pattern to enforce. Leaving the legacy verbs alone in this story is correct. | No change in #676. Note it for #646, which replaces these stops: when it does, it should align the wording and the `REFUSED_EXIT` name with ADR-0021 §9's taxonomy. |

Apart from these, the plan is consistent with existing patterns. I verified:

- The exit code is chosen in exactly one place (`output.rs:330-341`).
- `settle` (`:322-328`) only turns a failed write on a success into 1, so a refusal stays 3.
- `emit_stream` returns the first non-zero code.
- `main.rs:112` applies the returned code unchanged.

The CLI has no shared exit-code vocabulary that `ErrorClass::exit_code` would duplicate. Every other verb writes its exit as
a literal or a local constant (`hold_cmd.rs:19,141`, `prompt_target.rs:58-61`). The new test file
`holler-pane/tests/error_class_test.rs` follows the crate's one-file-per-topic test layout. `error_test.rs` (285 lines)
belongs to #637's AC 5, and a separate file keeps it as it is.

## Notes for O

PASS, so nothing is required before T.

- W-1 and W-2 are brief amendments O can make before T. Both are doc-only, and neither changes the code plan or the tests
  T writes.
- W-3 adds one line to AC 4.
- W-4 is guidance for F.
- W-5 is a note to carry to #646.

Also:

- **Evidence fix.** The brief cites `ADR-0021.md:196` for the legacy `--pane`/`--profile` "not implemented" exit 1, but
  `:196` is ruling 6 (control-socket methods). The real sources are `ADR-0003.md:93` and epic #633's body ("The refusal is
  plain stderr text, exit 1, on every format"). This has no design impact.
- **Evidence for the `session-not-found` close call.** #645's acceptance reads "a switch to a deleted session fails and
  changes nothing", which leans toward Failure. It adds to the existing Risks bullet, beside `v2.md:757`.
- **What A checks at Phase 7:**
  - There is exactly one exhaustive `match` over `PaneCode`, with no `_` arm, inside `class_of`.
  - The lookup goes through `PaneCode::parse` plus `is_valid_code`, with no second code list and no second validator.
  - `output.rs::exit_code` holds no table and no comparison with `usage`.
  - There is no new module, no `lib.rs` re-export and no manifest change.
- **Sizes.** `error.rs` goes from 617 to about 670 lines (lint warns at 600, which is accepted, and fails at 900).
  `output_api.rs` goes from 389 to about 470. `output.rs` stays at about 468.

## Patterns referenced

- `crates/holler-pane/src/error.rs:33-182, 401-519`: the one `PaneCode` table, `as_str`/`parse`, `is_valid_code`,
  `PaneError::classify` and `from_wire`.
- `crates/holler-pane/src/lib.rs:31-33, 58-61`: the frozen rule, and the error vocabulary staying under `error::`.
- `crates/holler-cli/src/output.rs:197-341` and `crates/holler-cli/src/main.rs:220-269`: the one exit-code decision point
  and every path into it.
- `docs/adr/ADR-0021.md`: §5 (`:176-188`), §9 (`:301-371`), I3 (`:162`) and §10 (`:390-392`). `docs/adr/ADR-0003.md:8-9,
  93, 97`.
- Issues #638 (the envelope helper pairs `ok: false` with 1, 2 or 3), #633 (I3 and the exit-code line), #644, #645 and
  #646.
