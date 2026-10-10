# Handoff-S: Phase 8 — #700 `Pane.opencode_agent`, its guard and the `--agent` spec flag (spec audit)

**Date:** 2026-10-10
**Branch:** `issue-0700-implementation` (run worktree `<run-worktree>`)
**Issue:** #700 (amendment to #637's contract; epic #633 decision 8)
**Diff audited:** `git diff 3d95aec..HEAD` (e49c5aa T-red, 34ca660 F, 1155d69 T-green)
**Handoff-T reviewed:** `docs/handoffs/0700-implementation/handoff-T-green.md` (also T-red)
**Handoff-A reviewed:** `handoff-A.md` (up-front PASS, DECISION 1) + `handoff-A-dup.md` (PASS)
**Handoff-F reviewed:** `handoff-F.md`
**Operator-facing report:** N/A — no UI surface (D/U recorded N/A at brief; browser and visual-diff preconditions do not apply to this story)

## A precondition

Confirmed: A returned PASS on the plan (up-front, with binding DECISION 1: the launch record
writes `spec.opencode_agent`) and A-dup returned PASS on the finished diff. No blocks at either
architecture gate.

## T precondition

Confirmed: `handoff-T-green.md` reports GREEN CONFIRMED — configured suite exit 0, zero blocking
issues; clippy silent; fmt file-set byte-identical to base; no `unsafe`; `--agent` ×1 in the
surface fixture; ADR exactly one insertion.

## Spec-sanity check of the source of truth

Ran before the AC audit: the issue/epic contract (one guarded newtype beside `EnvVarName`, one
optional field after `model` with the crate's standard serde attributes, one flag per field beside
`--model/--effort/--role`, open refusal code) violates no standard convention for the domain —
it extends an existing, internally consistent pattern set. No ADVISORY-HOLD on the spec.

## Acceptance criteria — AC-by-AC (brief ACs 1–10, which formalize the issue's five bullets)

| AC | Requirement | Met | Evidence (diff + T's suite) |
|---|---|---|---|
| 1 | `AgentKey` accepts `orchestrator`, `feature-implementor`, `mo`; refuses empty/space/newline/`=`/`/` with `agent-key-invalid`, no echo | YES | `argv.rs` guard: non-empty, ASCII letters/digits/`-`/`_` only, `PaneError::Refused` with `AGENT_KEY_INVALID` (`RefusalCode::from_static`, open code beside the guard); message states the grammar, never the text. `argv_env_test::agent_key_accepts_names_and_refuses_malformed_keys` pins all 8 cases + code + no-echo (RED: E0432) |
| 2 | Serde plain string, fail closed on decode | YES | `#[serde(transparent)]`, `Deserialize` through the shared `deserialize_parsed`; `agent_key_serde_is_a_plain_string_failing_closed` (after T-green's one-literal repair, F's delimiter analysis upheld) + `records_test::a_record_with_an_invalid_agent_key_fails_to_decode` over both records |
| 3 | `Pane` + `ProfileSpec` round-trip set/absent; JSON `opencode_agent`; after `model`; old record → `None` | YES | Field directly after `model` on both structs with `#[serde(default, skip_serializing_if = "Option::is_none")]` (diff verified); `records_test::{pane,profile_spec}_opencode_agent_round_trips_set_and_absent` pin field name, textual after-`model` order, omission when `None`, old-record load (RED: `unknown field opencode_agent`) |
| 4 | `--agent KEY` parses on launch AND relaunch; invalid refused with the guard's code **before anything else runs**; surface shows flag once | YES | Shared `SpecFlags`/`SpecValues`/`validate()` (args.rs, after `--role`), typed through `AgentKey::parse`; `guards::an_invalid_agent_key_is_refused_before_anything` and `relaunch_refuses_an_invalid_agent_key_before_anything` assert exit 3, `agent-key-invalid`, no echo, `assert_untouched` (empty call log), no record written; `grep -c -- '--agent' cli-surface.txt` = 1 (re-verified by S; RED: `unexpected argument '--agent'`) |
| 5 | Overlay like its siblings (given→replaces / absent→base / none→`None`); DECISION 1 record write | YES | `effective_spec` one-liner, the `command`/`check` precedent; `Plan::record` writes `opencode_agent: spec.opencode_agent.clone()` after the `model` line (DECISION 1 verbatim); pinned by `launch_with_agent_stores_the_key_in_the_record`, `relaunch_with_agent_replaces_the_stored_key`, `relaunch_without_agent_keeps_the_stored_key` (the landmine pin), and the flagship expected record's `None` |
| 6 | `profile create --from-current` copies the field from a fake PaneStore | YES | `spec_from_pane` copy + the W5 doc-table row; `snapshot_copies_every_spec_field_from_the_pane_record` extended with set and None halves over the fake store's records |
| 7 | `cli-surface.txt` shows the flag exactly once; line still parses; leaf parity | YES | The `--model … --role agent` launch line gains `--agent orchestrator` (one line changed); `every_surface_line_parses` + leaf-parity green (T-green 3/3) |
| 8 | ADR-0021: exactly one dated §1 row, no other prose change (inspection AC) | YES | S inspected the hunk: one inserted dated row after `model` in §1's Pane table, self-describing per W4 (names the field on both records, the flag, the record write, the `--from-current` copy, "#642's" application); `1 insertion(+)`, zero deletions — ruling on the deferred bullet below |
| 9 | Every existing test passes unedited-in-behaviour | YES | F's commit (34ca660) touches zero test paths (`--stat` verified by S); suite exit 0, 1762 passing; T's suite edits are new tests plus behaviour-neutral field lines in expected literals |
| 10 | fmt / clippy clean; no new `unsafe` | YES | T-green Tier 2: clippy `--workspace --all-targets` exit 0 silent; fmt file-set byte-identical to base `3d95aec`; no `unsafe` in either crate at HEAD or base. S re-checked the diff itself: zero `unsafe`, zero credential-shaped strings |

Issue-bullet coverage: guard (→1-2) · serde round-trips + old records (→3) · flag parse/refuse-before-anything/surface-once (→4-7) · from-current copy (→6) · existing tests unedited (→9). **10/10 ACs met.**

## Blast-radius check

`git diff 3d95aec..HEAD --stat` (verified by S): 19 non-handoff paths, every one blessed —
`holler-pane/src/{argv,lib,pane,profile,profile_snapshot,tx_launch}.rs`,
`holler-pane-testkit/src/fixture.rs`, `holler-cli/src/pane/{args,launch}.rs`,
`holler-cli/tests/fixtures/cli-surface.txt` (one line), T's suites
(`holler-pane/tests/{argv_env_test,records_test,profile_snapshot_test}.rs`,
`holler-cli/tests/pane_verbs/{launch.rs,launch/guards.rs,launch/rig.rs,relaunch.rs}`),
`docs/adr/ADR-0021.md` (one row), plus these handoffs. The issue's `src/cli.rs` → `args.rs` and
`cli-surface.txt` → `tests/fixtures/` corrections are journalled (survey deviations 1-2, O
recorded); `lib.rs` is pre-blessed deviation 1 (inside the issue's `holler-pane/**`). **Not
touched:** `error.rs`, `ports.rs`, `profile_diff.rs`, any `Cargo.toml`, `CHANGELOG.md`, the hub
crate, any adapter. Per-commit sets verified: e49c5aa tests+handoffs only, 34ca660 src+ADR only,
1155d69 test repairs+handoffs only.

## Rules check

- **No live fleet / running pane / real Herdr session:** all new tests ride the rig, fake
  PaneStore (`concurrent_put`, `rig.live(false)`) and temporary state; no network, no fleet ids.
- **No credential anywhere in the diff:** S grepped the full diff for key/token/secret/bearer/
  password shapes — zero matches; the refusal messages never echo key text; `ALPHA_TOKEN`/
  `BETA_URL` are pre-existing env *names*.
- **Conventional Commits:** all three subjects conform (`test(pane):` ×2, `feat(pane):` ×1) with
  `Co-Authored-By` trailers.
- **No new `unsafe`:** verified at HEAD vs base and in the diff text.
- **No #644 / #642 scope theft:** no project-defined-key validation anywhere in the diff; no
  adapter application, no `HarnessPort`/ports change — exactly "declare and parse only".

## The ADR-0021 deferral-bullet ruling (the A-dup warn)

The older deferred bullet (~line 677) still bundles "(`--agent KEY`, refusing a key the project
does not define, and recording `Pane.opencode_agent` in the record write): #644's last part". The
flag and the record write are now #700's (the latter by DECISION 1), so the bullet over-claims
for #644. **Ruling: acceptable under the issue's own constraint, not misleading enough to
rework.** The issue — the operator's spec — allows exactly "one dated row … not a rewrite";
editing the bullet would itself violate the blast radius. The new dated §1 row is the later, more
specific text and states the truth in full (flag, record write, `--from-current` copy, "#642's"
application); dated amendment rows govern over older deferred prose by the ADR's own structure,
and the supersession is recorded in the journal, A-dup, F and T-green. Designated cleanup: #644
amends the bullet when it lands its actual remainder (the project-defined-key refusal). Advisory
below if the operator prefers to tidy it sooner.

## Test-quality audit (rubric)

- **Per test:** each names one behaviour, has recorded RED evidence failing for the named reason
  (E0432/E0560/E0609 compile-RED; `unknown field`/`unexpected argument`/wrong-code runtime-RED),
  and sits at the cheapest sufficient tier — guard/serde at unit level, refusal-before-anything
  and record writes at the rig tier (only the rig's call log can observe "before anything"), the
  snapshot copy at the pure-function tier. Assertions are behavioural: exit codes, refusal codes,
  record contents, JSON wire forms. The textual `"model"` < `"opencode_agent"` assertion pins the
  epic's wire-contract order, not an implementation detail.
- **Per suite:** 10 new tests + 5 extensions is proportionate to a field+guard+flag+serde story;
  no coverage padding; nothing assertion-free or tautological.
- **Repairs:** T-green's two are legitimate — the `r#"""#` lone-quote literal could never reach
  the guard (F's delimiter analysis, T's independent verification; the repair `r#""""#` exercises
  the intended empty-key refusal), and the fmt rewraps fixed T's own RED-authored lines only.
- **No delete/merge findings.** The twin 3-line `agent()` helpers live in different crates' test
  trees that cannot share code and each matches its file's `name()`/`env()` convention (A-dup
  cleared; concurred).

## Scope check

Delivered exactly the issue's scope: the type + guard, the field on both records with serde, the
one flag (declared and parsed only), the snapshot copy, the testkit fixtures, the one ADR row —
plus DECISION 1's record line, which A ruled binding and which the issue's purpose line ("merges
first so no later story edits `holler-pane/**`") requires. No over-delivery (non-goals held:
no application, no project-key validation, no diff-vocabulary, no CHANGELOG/version/golden
files); no under-delivery.

## Quality audit

| Area | Result | Notes |
|------|--------|-------|
| API/CLI consistency | PASS | `--agent` rides the exact `--role` triad; overlay/copy/record mirror their siblings line-for-line |
| Error handling | PASS | one open code, exit 3, no echo, fail-closed decode; no `error.rs` list change |
| UI/UX | N/A | no UI surface |
| Accessibility | N/A | no UI surface |
| Architecture gate | PASS | A PASS + A-dup PASS; no finding here conflicts with either |
| Code organization | PASS | extends named analogues; no parallel path, no dead code, no TODOs |
| Security | PASS | guard by construction; key treated as a name, never echoed; no secrets in diff |
| Performance | PASS | one `Option` field and clone-on-write one-liners; nothing measurable |
| Visual regression | N/A | no UI surface |
| Naming consistency | PASS | `AgentKey`/`opencode_agent`/`--agent` per the epic contract; JSON name exact |
| Test quality (rubric) | PASS | see the section above |

## Advisory notes (for O; none blocks the merge)

1. **ADR-0021 deferred bullet (~:677)** superseded-but-not-amended (ruled acceptable above);
   #644 amends it when it lands, or the operator may approve a one-line prose touch outside this
   run's boundary.
2. **CHANGELOG.md** left untouched — correct per the issue's blast radius, but sibling feature
   PRs added entries; the operator may want one at merge time (journalled since survey deviation 3).
3. **Uncommitted gate bookkeeping in the tree:** `handoff-A-dup.md` is untracked and `decisions.md`'s
   Phase 6–7 tail is unstaged — commit them with this handoff so the PR carries the full gate history.
4. **A-dup W2 residue:** `verb_harness/parse.rs` `SPEC_FLAG_SETS` and `docs_rows.rs`'s static flag
   list lack `--agent` (outside T's boundary; coverage rides the verb tests + the surface fixture;
   the `*_accepts_every_spec_flag` name slightly overstates). Optional test-only follow-up.
5. **Pre-existing flake classes** met during T-green (interrupt_test under full load; body_run_test's
   fixed-port TIME_WAIT race, #242/#259 class) — follow-up issue-worthy, unrelated to this diff.

## Verdict

PASS — all ten acceptance criteria met with diff- and test-verified evidence; blast radius and
every issue rule held; DECISION 1 honored; the ADR deferral oddity ruled acceptable under the
issue's own one-row constraint. Ready for O to commit the outstanding handoffs and proceed to the
PR/merge gates.

VERDICT: PASS
