# A review — #660

**Date:** 2026-10-09 · **Phase:** up-front plan review (no code yet)
**Brief:** `docs/handoffs/660/brief.md` · **Survey/Reuse map:** `docs/handoffs/660/survey.md` · **Wireframe:** N/A

VERDICT: PASS

## Summary

The brief's reframing of #660 is correct against current main: `output.rs` is already the
complete module (#670/#676), so the deliverable is the conformance layer, and the survey's
four gaps (checker wiring, GridPos golden, forced-diagnostic test, signature pins) map
one-to-one onto the acceptance bullets with nothing dropped. The Reuse map names the right
objects to extend, the boundaries are disjoint and protect the parallel session, and the
RED policy's green-on-contact stop is the honest handling of a dated premise. Two
warn-level findings below are scope wordings O may tighten; neither blocks.

## Findings

| # | Severity | Plan element | Finding (evidence) | Suggested fix |
|---|---|---|---|---|
| 1 | warn | Reuse map: "no new test files (autotests=false)" (brief:44-46; survey:46-48) | Overbroad. `autotests = false` (`crates/holler-cli/Cargo.toml:14`, comment :40-42) makes a new **top-level** `tests/*.rs` silently skip building, but a new **submodule** under `pane_verbs/` (declared `mod x;` in `tests/pane_verbs/main.rs:20-35`, inside T's glob) builds with no manifest edit. The file-size gate warns at 600 / fails at 900 (`scripts/lint.sh:43-50`) and `output_api.rs` is already 470 lines, so a split is a legitimate outcome, not drift. | Scope the rule: "no new top-level test targets; a `pane_verbs/` submodule split (declared in the target's `main.rs`) is in-bounds if the size gate demands it." |
| 2 | warn | Boundaries: T glob `crates/holler-cli/tests/**` (brief:44) | The glob is wider than the named working set and reaches shared files (`tests/verb_harness/mod.rs`) and sibling stories' suites (`pane_verbs/list.rs`, `get.rs`, `watch.rs`, `profile_verbs/**`) that the parallel session (a sibling pane) may also hold. The brief's expectation line ("extend `output_api.rs`; process tests under `pane_verbs/process/`") is the real boundary; only discipline enforces it. | Restate T's boundary as the expected files, or O holds T to them at stage record. |
| 3 | note | Checker wiring = duplication risk? | No: it is the established pattern, not a parallel path. `check_envelope`/`check_ndjson` are already the authority at `pane_verbs/list.rs:154,161,198-200`, `get.rs:267-269`, `watch.rs:186`, `doctor/surface.rs:134`, `profile_verbs/rig.rs:105`; `output_api.rs:16` (local `one_envelope`) is the outlier the story fixes. The checker keeps one validator and one classifier (`holler-pane-testkit/src/envelope.rs:62` imports `is_valid_code`/`class_of` from `holler_pane`; no table of its own), honoring ADR-0021 §5. a-dup checkpoint: the wiring must make the checker the conformance authority — `one_envelope` (`tests/verb_harness/mod.rs:82-97`, deliberately minimal per its doc :10-12) must not grow into a second rule set. | None now; re-check at a-dup. |

## Verified sound (no action)

- **Acceptance complete and current:** the module is genuinely built — routing, NDJSON,
  `one_line`, `settle`, `scan_args`, `usage_message` all present in
  `crates/holler-cli/src/output.rs` — so "fill the module" is satisfied by #670 and the
  remaining acceptance is exactly the suite the brief delivers.
- **Exit-code resolution (brief:30-33):** matches landed code — `output.rs:338-340`
  (`class_of(code).exit_code()`), `error.rs:239-245` (2/3/1) and `:266-303` (refusal=3),
  and the checker's rule 13 (`envelope.rs:33`, `:345`). Parity between formats is the
  tested property; the ADR wins over the issue's older "1 refused or failed" wording.
- **Reuse map mechanics:** `holler-pane-testkit` already a dev-dep
  (`Cargo.toml:435`); `pane_verbs` target declared (`Cargo.toml:480-482`) with
  `mod output_api;` (`pane_verbs/main.rs:20`); `pane_cli_process` target declared
  (`Cargo.toml:495-497`) for the real-binary diagnostic test. No manifest change needed.
- **Goldens are checkable:** GridPos row-first serde is real (`grid.rs:129-137`,
  `{"row":R,"col":C,"pos":"rRcC"}`) and `data` may be any value (checker rule 7,
  `envelope.rs:26`), so the golden pairs `check_envelope` with a raw-text key-order
  assertion like the existing compact-golden test (`output_api.rs:176-189`).
- **Signature pins feasible** in an integration target (public fns `emit`/`emit_stream`/
  `emit_usage_error` at `output.rs:207/225/247`; any signature change breaks the build).
- **RED policy:** green-on-contact → T records evidence, run stops for the operator, no
  sham F — honest test-first for a near-green premise, and consistent with the
  no-mutation rule (survey:58-59).

## Patterns referenced

`crates/holler-cli/tests/pane_verbs/list.rs`, `get.rs`, `watch.rs` (checker consumers) ·
`crates/holler-pane-testkit/src/envelope.rs` + `tests/envelope_test.rs` (checker and its
self-tests) · `crates/holler-cli/Cargo.toml` (target declarations) ·
`crates/holler-pane/src/error.rs` (`class_of`) · `crates/holler-pane/src/grid.rs:129-137`.
