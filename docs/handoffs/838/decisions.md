# Decision journal: #838, Stage 1 asks which Herdr session to build in

Run handoff directory `docs/handoffs/838` · branch `issue-838-implementation` · rigor in-session · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry.

## F, implementation: 2026-10-11T16:16-06:00

- **Decided:** text-only change to `SKILL.md` (Stage 0 row, the Stage 1 session question, step 4, Verify, Stage 3 plan),
  `docs/setup-wizard.md` (`### Choosing the Herdr session`) and `CHANGELOG.md`. The question is skipped for (C) and
  for an existing config. A named session's name must not be `default` (instance.sh refuses it). The outside-a-pane
  rule points at "Two rules" instead of restating it (A warn 1). The docs use a `###` heading (A warn 2).
- **Assumed:** the stated Stage 3 strings are enough for a rerun with an existing named session; the "reused vs.
  started fresh" bullet already covers reuse.
- **Hedged:** a pre-existing local clippy lint (`token_load_test.rs:60`, rustc 1.93.1) and two load-flaky
  integration tests are recorded as known issues, not fixed (out of scope, untouched files).
- **Evidence:** `wizard_scripts` 285/285 passed; `scripts/lint.sh` exit 0; `changelog-check` ok; `evidence.md`.
