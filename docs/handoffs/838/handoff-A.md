# Handoff-A: #838 herdr-session-question  (up-front plan review)

**Date:** 2026-10-11
**Branch:** issue-838-implementation
**Brief reviewed:** docs/handoffs/838-brief.md   **Reuse map:** (in brief, "Reuse map" line)   **Wireframe:** N/A
**Verdict:** PASS

## Summary
Text-only plan: extends the existing Stage 0/1/3 skill text, the `[instance]` docs and `instance.sh` validator, with no new script, no Rust, and no change to validators or the wire. It touches no choke point. It is consistent with existing patterns; three warns for F/T to handle.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | AC3 "states, once" the outside-a-pane rule and `HOLLER_STATE_DIR` | pattern consistency | SKILL.md:665-670 ("Two rules for a non-default instance") already states the outside-a-pane rule. A full restatement in the new question is a second copy. | New question states the rule briefly and points at "Two rules"; add `HOLLER_STATE_DIR=<state_dir>` once. The test should count the new Stage 1 text, not demand the rule appear exactly once file-wide. |
| 2 | warn | docs subsection under "Running an instance beside another" | naming / structure | `herdr.rs:690,864` split docs on the literal `## Herdr session` heading (line 494). A new heading beginning with that text, or a `##`-level heading, would change what those tests slice. | Use a `###` heading (e.g. "Choosing the Herdr session") and do not reuse `## Herdr session`. Run the whole wizard_scripts target. |
| 3 | warn | Stage 1 reword of the workspace paragraph opener | pattern consistency | w5 test pins phrases inside `## Stage 1 ` ... `## Stage 2 `, and AC1 asserts offsets of three markers (menu, session question, workspace question). Markers must be unique strings, or the offset test is ambiguous. | Pick distinctive marker strings; T asserts the order. Keep the pinned w5 phrases verbatim. |

Notes: ports 41808/8443/47101 are defaults only; `collide.sh` in Stage 3 stays the authority, per the brief. A different proposal would collide with an existing second instance, but that is caught later and is not drift. SKILL.md is 1814 lines and docs/setup-wizard.md 557. SKILL.md is not in the `scripts/lint.sh` 900-line check if that check covers only code; F should confirm `bash scripts/lint.sh`. Do not use personal infrastructure names (the live `jupiter` session) as examples; use `work`/`second`.

## Notes for O
None required (PASS).

## Patterns referenced
- agent-skills/herdr-setup/SKILL.md (Stage 0 row 420, Stage 1 605-670, Stage 3 975)
- docs/setup-wizard.md ("Running an instance beside another", "## Herdr session")
- crates/holler-cli/tests/wizard/skilltext_w5_workspace.rs, skilltext_w6_preflight.rs, herdr.rs
