# Handoff-A-dup: Phase 7 - #700 `Pane.opencode_agent`, its guard and the `--agent` flag (anti-duplication gate)

**Date:** 2026-10-10
**Branch:** `issue-0700-implementation` (run worktree `<run-worktree>`)
**Diff base:** `3d95aec`   **Diff head:** `1155d69` (e49c5aa T-red, 34ca660 F, 1155d69 T-green)
**Reuse map:** `docs/handoffs/0700-implementation/survey.md`
**Verdict:** PASS

## Summary

F extended every named analogue and built no parallel path. `AgentKey` sits beside `EnvVarName`
with the identical newtype shape (same derives, `#[serde(transparent)]`, `Deserialize` through
the shared `deserialize_parsed`), the code is a `RefusalCode::from_static` const beside its owner,
`--agent` rides the existing `SpecFlags`/`SpecValues`/`validate` triad exactly as `--role` does,
and the overlay/copy/record lines are the one-liner precedents the map named. DECISION 1 is
honored verbatim (`tx_launch.rs:350`), the diff stays inside the blessed boundaries (`--stat`
verified: no `error.rs`, `ports.rs`, `profile_diff.rs`, `Cargo.toml`, `CHANGELOG`), and the
amendment's no-later-`holler-pane`-edit rule holds: nothing half-carries the field — the #642/#644/
#647 deferral sites carry it zero times, each journalled. Three warn notes, no blocks.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `docs/adr/ADR-0021.md:~677` | The older deferred bullet still words recording `Pane.opencode_agent` as #644's last part, now superseded by DECISION 1 and the dated §1 row (which states the launch record writes it). F and T both flagged it; the one-row constraint (AC 8) forbade touching it. Doc-level ambiguity only — the later, more specific amendment wins. | #644's story amends the bullet when it lands the project-defined-key refusal (its `holler-cli`-side edit); or the operator approves a one-line prose touch outside this run |
| 2 | warn | `crates/holler-cli/tests/verb_harness/parse.rs:71-100`, `crates/holler-cli/tests/pane_verbs/process/docs_rows.rs:193` | A's W6 residue: `SPEC_FLAG_SETS` and the static "every shared flag" list still name `--role` but not `--agent` (both files outside this run's test boundary; parse coverage rides the verb tests + the surface fixture, where `--agent` appears exactly once). The `docs_rows` test name slightly overstates. | Optional follow-up: add the flag to both static lists in a test-only change |
| 3 | warn | `crates/holler-cli/tests/pane_verbs/launch/rig.rs:134`, `crates/holler-pane/tests/profile_snapshot_test.rs:28` | Two file-local 3-line `agent()` constant helpers mirror each other across the crates' test trees. Each matches its own file's `name()`/`env()`/`argv()` helper convention, and the crates cannot share test code — inspected and cleared, noted only for completeness. | None |

## Duplication check (Q1) — none

- **No second guard.** `AgentKey` (argv.rs:135-165) mirrors `EnvVarName` (argv.rs:92-119) field
  for field: `pub struct X(String)`, derives `Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord,
  Serialize`, `#[serde(transparent)]`, `parse(&str) -> Result<Self, PaneError>`, `as_str`, and
  `Deserialize` through the **shared** `crate::error::deserialize_parsed` (argv.rs:163) — no
  bespoke serde helper, no re-implemented fail-closed decode.
- **No error-list edit.** `AGENT_KEY_INVALID` is an open code via `RefusalCode::from_static` beside
  the guard (argv.rs:123-125), the `tx_launch::PANE_EXISTS` precedent; `error.rs` is untouched.
- **No re-implemented parse in the CLI.** `--agent` extends `SpecFlags` (args.rs:60-63),
  `SpecValues` (:105) and `validate()` (:134) typing straight through `AgentKey::parse` — the
  `--role` triad's exact shape; no `parse_agent` twin of `parse_role` was needed or written.
- **No copy of the overlay logic.** One `values…or(base…)` line in `effective_spec`
  (launch.rs:175-178), the `command`/`check` precedent beside it (:172-174).
- **No near-duplicate test rig.** T extended the existing rig and suites; the flagship expected
  records gained `opencode_agent: None` in place (launch.rs:80, rig.rs:579) rather than new
  builders.

## Drift check (Q2) — diff matches the passed plan

- **DECISION 1 honored:** `opencode_agent: spec.opencode_agent.clone()` directly after the
  `model` line (`tx_launch.rs:349-350`); the landmine test T pinned
  (`relaunch_without_agent_keeps_the_stored_key`, relaunch.rs:541-559) fails on any `None` write.
- **Production shape as passed:** field after `model` on `Pane` (pane.rs:247-250) and
  `ProfileSpec` (profile.rs:186-189) with the neighbors' exact
  `#[serde(default, skip_serializing_if = "Option::is_none")]`; `spec_from_pane` copy + the W5
  doc-table row (profile_snapshot.rs:35, :62); root re-export in `lib.rs` (pre-blessed deviation
  1, disclosed by F); testkit `None` lines + doc wording (fixture.rs:74, :110); one dated §1 row
  naming the whole delta (W4 honored; `1 insertion(+)` verified).
- **Boundaries held:** `git diff 3d95aec..HEAD --stat` touches only F ∪ T allowed paths plus these
  handoffs. `profile_scope.rs` and `conformance/pane_store.rs` correctly untouched (A's W1: no
  literals there — F's full-workspace compile confirms). F's commit touches zero test paths;
  T-green's commit touches only T's own suite + handoff docs.

## The amendment's rule (Q3) — deferral clean, zero-carry, journalled

- `profile_diff.rs`: `SpecField::ALL` (15 fields, profile_diff.rs:56-71) has **no**
  `opencode_agent` — the reporting vocabulary is #647's whole deliverable, not a half-carried
  field this diff would force someone to clean up. Journalled (decisions.md, "Non-goals
  journalled") and anticipated by ADR-0021's deferred bullets.
- Import (#650) and reconcile: flow through JSON decode / spreads; `#[serde(default)]` reads the
  new field with no edit (workspace compiled clean). `ports.rs` untouched — #642's application.
- The consuming side already documents the split: `holler-adapter-opencode/src/lib.rs:11`
  (pre-existing) names applying `Pane.opencode_agent` as #642's last part.
- No later story needs to touch `holler-pane/**` *for the field's plumbing* — the record, serde,
  guard and snapshot layers are complete at merge.

## Notes for F

None — no rework. The three warns are doc/test-list follow-ups outside this run's boundary.

## Patterns referenced

1. `crates/holler-pane/src/argv.rs` — `EnvVarName`, the mirrored guard shape
2. `crates/holler-cli/src/pane/args.rs` + `launch.rs` — `--role` triad and Option-overlay precedents
3. `crates/holler-pane/src/tx_launch.rs` + `profile_snapshot.rs` — the record and snapshot literals
4. `docs/adr/ADR-0021.md` — §1 table, deferred-to-named-stories bullets
5. `docs/handoffs/0700-implementation/{decisions.md, handoff-F.md, handoff-T-green.md}` — journalled deferrals and Tier 2 evidence

VERDICT: PASS
