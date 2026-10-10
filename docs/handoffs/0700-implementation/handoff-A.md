# Handoff-A: Phase 3 — #700 `Pane.opencode_agent`, its guard and the `--agent` flag (up-front plan review)

**Date:** 2026-10-10
**Branch:** `issue-0700-implementation` (run worktree `<run-worktree>`)
**Brief reviewed:** `docs/handoffs/0700-implementation/brief.md` · **Reuse map:** `docs/handoffs/0700-implementation/survey.md` · **Wireframe:** N/A (no UI surface)
**Verdict:** PASS (with one binding DECISION and warn notes below)

## Summary

The plan is faithful to issue #700 and ADR-0021 and is extend-not-new at every point the
survey claims — I verified each analogue in the tree. The open question is ruled below
(DECISION 1): `record()` plumbs `spec.opencode_agent` now. The one material inaccuracy is
bookkeeping, not architecture: the plan's compile-forced-literal enumeration is wrong in
both directions, but I swept the workspace and **every** full-literal site falls inside the
stated F/T boundaries, so no boundary change is needed. No block findings.

## Answers to the plan-gate questions

### 1. Faithfulness to the issue and ADR-0021 — yes

Every claimed analogue checks out in code:

| Plan element | Verified evidence |
|---|---|
| `AgentKey` beside `EnvVarName` in `argv.rs` | `EnvVarName` newtype + `parse` + `deserialize_parsed` at `crates/holler-pane/src/argv.rs:88-117`; module doc pins the no-echo refusal convention (argv.rs:10-12) |
| Optional-field serde attrs | `Pane.command` (`crates/holler-pane/src/pane.rs:252-253`), `ProfileSpec.command` (`crates/holler-pane/src/profile.rs:192-193`): `#[serde(default, skip_serializing_if = "Option::is_none")]` under `deny_unknown_fields`; insertion points after `model` exist (pane.rs:246, profile.rs:185) |
| `--agent` beside `--role` in `args.rs` | `SpecFlags`/`SpecValues`/`validate()` at `crates/holler-cli/src/pane/args.rs:41-139`; `--role` at :57-59, :100, :126 is the exact precedent; the survey's deviation 1 (issue says `src/cli.rs`) is correct — the flags live here since #670 |
| Overlay like its siblings | `effective_spec`'s Option-overlay precedent (`values.command.clone().or(base.and_then(...))`) at `crates/holler-cli/src/pane/launch.rs:170-174` |
| `--from-current` copy like `model` | `spec_from_pane` full literal at `crates/holler-pane/src/profile_snapshot.rs:46-68` (copies `model` at :60) |
| Open code, no `error.rs` edit | `CODE_COUNT = 22` frozen (`crates/holler-pane/src/error.rs:39`); `agent-key-invalid` is kebab-valid and not closed, so `RefusalCode::from_static` compiles (error.rs:331-337) and `class_of` sorts it Refusal/exit 3 (error.rs:266-273); the const-beside-owner precedent is `tx_launch::PANE_EXISTS` etc. (`crates/holler-pane/src/tx_launch.rs:47-55`) |
| Surface fixture one line | The `--model … --role agent` launch line is `crates/holler-cli/tests/fixtures/cli-surface.txt:119`, inside the `# #644` block; the leaf-set test (`tests/cli_surface_test.rs:83-93`) and story-grouping test (`tests/pane_verbs/process/docs_rows.rs:225-255`) constrain leaves, not flags, so extending that line is safe |
| ADR scope | §1's Pane table ends at `probe` (`docs/adr/ADR-0021.md:51`); §2 point 7 (:133) and the deferred bullets (:674, :676-677) already anticipate #700. No frozen-list edits, no `ports.rs`, no `profile_diff.rs`, no adapter changes — the non-goals match the ADR's deferrals exactly |

No scope creep found. The four survey deviations are all real and correctly journalled.

### 2. DECISION 1 (binding for F): `record()` writes `spec.opencode_agent` now

`tx_launch`'s record literal (it is `Plan::record` at `crates/holler-pane/src/tx_launch.rs:314-359`; the brief's "Engine::record" name is off, the file/line are right) gains
`opencode_agent: spec.opencode_agent.clone()` — placed after `model: spec.model.clone()`
(tx_launch.rs:349) per the contract order. Not `None`. Reasons, in order of weight:

1. **The hot-spot rule makes `None` a dead end.** The epic's exception (survey, "Governing
   rules pinned") gives `holler-pane/**` for this field to #700 **only**. If #700 writes
   `None`, no later story can change that line without breaking the very rule the issue
   exists to enforce ("merges first so no later story edits `holler-pane/**` for this
   field"). A plan whose stated purpose cannot be achieved by its other reading is decided
   by that purpose.
2. **The ADR's deferral bundles three things that #700's amendment splits.** The deferred
   bullet (ADR-0021:676-677) packs the flag, the project-key validation and the recording
   into "#644's last part, after #700". #700 takes the flag (and, by 1, the record line);
   #644's remainder is the CLI/verb-side refusal of a key the project does not define.
   Reading "recording" as #644's would make the bullet contradict the hot-spot exception;
   the amendment is the later, more specific text and wins.
3. **The C1 precedent is mechanical.** The same literal copies `model`/`env`/`context`/
   `command`/`check`/`expect` straight from the spec (tx_launch.rs:349-356). `None` would
   be the literal's first deliberate drop of a spec value — drift inside the object F is
   editing.
4. **The relaunch landmine is verified, not hypothetical.** A flag-less relaunch bases its
   spec on `spec_from_pane(&record)` (`crates/holler-cli/src/pane/relaunch.rs:75-79`), which
   (AC 6) copies the stored key back; the engine would then run with the key and write
   `None` over it — a silent reset shipped under a "no behaviour change" banner, which AC 9
   itself cannot intend. Corollary: with `None`, AC 6 tests a record state (`Some(key)`) no
   verb can produce.

Cost is one line in a literal F must already touch. No port, validation or adapter
semantics move (#642 applies; #644 validates; #647 reports — untouched).

### 3. Boundaries and compile-forced sites — sufficient; enumeration inaccurate

I swept every `Pane {` / `ProfileSpec {` site (grep + required-field markers
`session_of_record:` / `port_policy:` + shorthand check). The **definitive** full-literal
set — everything else is a spread, a mutation, or a JSON decode (e.g.
`crates/holler-pane/tests/common/mod.rs:87-93` builds from JSON; hub tests and
`profile_verbs` spread over the testkit samples; `profile_diff_test.rs:164` is a spread):

| Struct | Site | Owner |
|---|---|---|
| `Pane` | `crates/holler-pane/src/tx_launch.rs:324` (record) | F ✓ |
| `Pane` | `crates/holler-pane-testkit/src/fixture.rs:43` (`sample_pane`) | F ✓ |
| `Pane` | `crates/holler-cli/tests/pane_verbs/launch.rs:57` | T ✓ (`pane_verbs/**`) |
| `Pane` | `crates/holler-cli/tests/pane_verbs/launch/rig.rs:~330` | T ✓ |
| `Pane` | `crates/holler-cli/tests/pane_verbs/list.rs:121` | T ✓ |
| `ProfileSpec` | `crates/holler-pane/src/profile_snapshot.rs:47` | F ✓ |
| `ProfileSpec` | `crates/holler-pane-testkit/src/fixture.rs:93` (`sample_spec`) | F ✓ |
| `ProfileSpec` | `crates/holler-cli/src/pane/launch.rs:198` (`effective_spec`) | F ✓ |
| `ProfileSpec` | `crates/holler-pane/tests/profile_snapshot_test.rs:51` | T ✓ |
| `ProfileSpec` | `crates/holler-cli/tests/pane_verbs/launch/rig.rs:~555` | T ✓ |

Every forced site is inside F ∪ T's allowed paths — the workspace compiles within the
boundaries as written. But AC 9's parenthetical and the survey's list (survey.md:43-48) are
wrong in **both** directions: `profile_scope.rs`'s `changed_spec` is a mutation helper
(profile_scope.rs:379-383) and `member` a spread (:358-363) — no literal, no edit needed;
`conformance/pane_store.rs` holds only spreads (:156, :388, :394, :401, :430); and the three
`pane_verbs` test literals above are unlisted. See W1.

### 4. Acceptance criteria — testable; two wording notes

AC 1-7, 9, 10 each map to a concrete mechanism (guard tests → `argv_env_test.rs`; serde
round-trip → `records_test.rs`; refusal-before-anything → `SpecFlags::validate` runs first
in `launch_request`, launch.rs:94, assertable via the rig's call log; overlay →
`effective_spec`; `--from-current` → `profile_snapshot_test.rs`; fixture →
`cli_surface_test.rs` parse + leaf parity + `docs_rows.rs` coverage). With DECISION 1, AC 9's
"no other crate changes behaviour" stays true: `holler-pane`'s behaviour change (the field +
the record line) **is** this story's change; every other crate is untouched. Notes: AC 8 has
no home as a *test* inside T's boundary (W3); AC 9's enumeration needs the W1 correction.

### 5. Risks — all covered or noted

- **Serde/`deny_unknown_fields`**: new-reads-old → `#[serde(default)]` → `None` (AC 3);
  old-reads-new → `store-corrupt`, which ADR-0021 §7 declares intended (:302-303). The
  `EnvVarName` fail-closed decode precedent (guard runs on deserialize, error.rs:704-713)
  covers AC 2. No golden files in scope.
- **Guard message conventions**: crate rule is "say what was refused, never the text"
  (error.rs:24-26, argv.rs:10-12); AC 1 pins it. `=` on an agent key is an invalid
  character, not a secret signal — one code, one grammar message is correct.
- **cli-surface leaf parity**: no new leaf, so the leaf-set test is unaffected; the
  flag-coverage list is static (`docs_rows.rs:182-203`) and stays green either way (W6).
- **fmt/clippy/hooks**: mechanical additions beside existing precedents; no new `unsafe`;
  Conventional Commit subject and `Co-Authored-By` are in the brief's Delivery section.
- Nothing else constructs these records outside the swept set (import #650 and reconcile
  paths flow through spreads/JSON).

## Findings

| # | Severity | Plan element | Dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | AC 9 / survey.md:43-48 | accuracy | Compile-forced enumeration wrong both ways (see §3 table): misses the three `pane_verbs` `Pane` literals + two `ProfileSpec` test literals; wrongly lists `profile_scope.rs` (`changed_spec` is a mutation, :379-383) and `conformance/pane_store.rs` (spreads only) | Journal the corrected ten-site table; boundaries unchanged |
| 2 | warn | brief.md:82, survey table | naming | The record writer is `Plan::record` (`tx_launch.rs:314`), not "Engine::record" | Note in the journal; no code impact |
| 3 | warn | AC 8 ("docs check … every box is a T-authored test") | testability | No file in T's boundary can host an ADR-table assertion (`autotests = false` forbids new test files without a `Cargo.toml` edit, which is out of bounds) | Treat AC 8 as inspection-checked (the dated row lands with F's docs edit), or add `tests/docs_cli_test.rs` to T's list; do not widen otherwise |
| 4 | warn | AC 8 ("no other ADR prose changes") | doc drift | §1's `ProfileSpec` enumeration (ADR-0021:65-66) and §3's flag table (:172-187, "one per `ProfileSpec` field") go stale | Word the one dated §1 row to name the whole delta — the field on both records, the `--agent` flag, the `--from-current` copy — so the amendment is self-describing without other prose edits |
| 5 | warn | F's edit of `profile_snapshot.rs` | doc drift | The module's copy/not-copied table (profile_snapshot.rs:27-42) should gain its `opencode_agent` row when the copy lands | Same file, in boundary; fold into F's edit |
| 6 | warn | `docs_rows.rs:182-203` | accuracy | The static "every shared flag" list stays green without `--agent`; the test name then slightly overstates | Optional: T extends the list (its own tree); not required for green |

## Notes for O

- DECISION 1 is binding for F: the record literal writes `opencode_agent:
  spec.opencode_agent.clone()` (after the `model` line, tx_launch.rs:349). #644's "last
  part" reduces to the project-defined-key refusal, per ADR-0021:676-677 read against the
  epic's hot-spot exception. Journal the ruling against survey decision 3.
- W1/W2 are journal corrections only — amend the AC 9 parenthetical or let T read this
  handoff's §3 table; do not change the boundaries (they are complete).
- With the ruling, T is expected to pin the positive path (a launch with `--agent` stores
  the key) in the `pane_verbs` rig — the expected-record literals there are in T's tree.
  Not an AC change; noted so T is not surprised that "expected" records gain `Some(...)`.

## Patterns referenced

1. `crates/holler-pane/src/argv.rs` — `EnvVarName` guard shape, no-echo refusals
2. `crates/holler-pane/src/error.rs` — frozen 22-code list, `RefusalCode::from_static`, `class_of`
3. `crates/holler-pane/src/pane.rs` / `profile.rs` — optional-field serde conventions under `deny_unknown_fields`
4. `crates/holler-cli/src/pane/args.rs` + `launch.rs` — `--role` flag precedent and the Option-overlay precedent
5. `crates/holler-pane/src/tx_launch.rs` + `docs/adr/ADR-0021.md` (§1, §3, §7, §9, "Deferred to named stories") — the record literal and the governing rulings

VERDICT: PASS
