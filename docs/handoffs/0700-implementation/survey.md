# Survey — #700 `Pane.opencode_agent`, its guard and the `--agent` spec flag

Run: `0700-implementation` · branch `issue-0700-implementation` · rigor `in-session` · 2026-10-10.
Sources read: issue #700 (the spec), epic #633 (contract block + decision 8 + wave/hot-spot rules),
ADR-0021 (all sections; §1 field table, §3 spec flags, §9 codes, "Deferred to named stories"),
ADR index, and every file named below.

## What exists (verified in code, at `<run-worktree>` = 3d95aec)

| Thing | Where | Shape |
|---|---|---|
| `EnvVarName` guard (I7) | `crates/holler-pane/src/argv.rs` | newtype + `parse(&str) -> Result<Self, PaneError>`; serde via `deserialize_parsed`; refusal messages never echo the text |
| `Pane` record | `crates/holler-pane/src/pane.rs` | `deny_unknown_fields`; optional fields use `#[serde(default, skip_serializing_if = "Option::is_none")]` (e.g. `session_of_record`, `host.herdr_api_version`); `model: ModelSpec` then `env` then `context` |
| `ProfileSpec` | `crates/holler-pane/src/profile.rs` | same conventions; `command?`/`check?`/`expect` optional-or-default |
| Spec flags | `crates/holler-cli/src/pane/args.rs` (#670) | `SpecFlags` (clap strings) → `SpecFlags::validate()` → `SpecValues` (typed through the `holler-pane` guards); `--role` is the closest precedent incl. its `parse_role` |
| Flag overlay | `crates/holler-cli/src/pane/launch.rs` `effective_spec(base, name, values)` | flag-given-replaces, else base's field, else default; `command`/`check` are the closest `Option`-overlay precedent |
| Record write | `crates/holler-pane/src/tx_launch.rs` `Engine::record()` (#644's file) | full `Pane { .. }` literal; copies `model`/`env`/`context`/`command`/`probe` from the spec |
| `--from-current` mapping | `crates/holler-pane/src/profile_snapshot.rs` `spec_from_pane` (#662) | full `ProfileSpec { .. }` literal; copies `model`, `role`, `env`, `context`, `command`, `check`, `expect` |
| Test-kit builders | `crates/holler-pane-testkit/src/fixture.rs` | `sample_pane` / `sample_spec` / `sample_profile`; every other `Pane {`/`ProfileSpec {` literal in tests is a struct-update spread over these (verified) |
| Open codes | `error.rs` + precedents | `RefusalCode::from_static` consts beside their owner (`tx_launch::PANE_EXISTS`, `plan.rs::GRID_UNREACHABLE`, …) |
| Surface fixture | `crates/holler-cli/tests/fixtures/cli-surface.txt` | every line must parse; the `--model/--effort/--role` launch line is the one to carry `--agent` |
| ADR row precedent | `docs/adr/ADR-0021.md` §1 | the Pane field table; "as built (#N)" paragraphs and dated amendments elsewhere in the file |

Test layout: `holler-pane/tests/{argv_env_test.rs, records_test.rs, profile_snapshot_test.rs}` are
the per-module suites to extend. `holler-cli` sets `autotests = false` with explicit targets in its
`Cargo.toml` (outside every boundary here), so its new tests extend existing files —
`tests/pane_verbs/{launch.rs, relaunch.rs, launch/guards.rs}` and the rig in `tests/pane_verbs/launch/rig.rs`.

## Reuse & Analogous-Feature map (extend-vs-new: **extend, everywhere**)

| New thing | Closest existing | Recommendation |
|---|---|---|
| `AgentKey` guard | `EnvVarName` in `argv.rs` | extend `argv.rs`; same newtype shape, `parse` + `deserialize_parsed`; message states the rule, does not echo the refused text |
| `agent-key-invalid` code | `tx_launch::PANE_EXISTS` et al. | a `pub const` beside the guard in `argv.rs`, carried by `PaneError::Refused`; **not** a closed code (list frozen) |
| `Pane.opencode_agent` / `ProfileSpec.opencode_agent` | `session_of_record` / `command` optional fields | `Option<AgentKey>` + `#[serde(default, skip_serializing_if)]`; placed after `model` (epic contract block order) |
| `--agent KEY` | `--role` in `args.rs` | extend `SpecFlags` + `SpecValues` + `validate()` (`AgentKey::parse`, guard's code surfaces at step 1 of both verbs) |
| overlay in `effective_spec` | `command`/`check` (`values.x.or(base.and_then(..))`) | same one-liner; `None` = server default, never a required flag |
| `--from-current` copy | `model` in `spec_from_pane` | `opencode_agent: pane.opencode_agent.clone()` |
| record write | `model: spec.model.clone()` in `tx_launch::record` | see brief's open question — recommended: copy from spec |
| testkit builders | `sample_pane`/`sample_spec` literals | gain `opencode_agent: None`; additive only (later stories set it by assignment) |
| surface line | the `--model … --role agent` launch line | extend that one line with `--agent orchestrator` (flag appears exactly once) |

No new files anywhere. No `Cargo.toml` edits. Compile-forced full literals that gain one line each:
`argv.rs` (the type), `pane.rs`/`profile.rs` (the fields), `tx_launch.rs::record`,
`profile_snapshot.rs::spec_from_pane`, `launch.rs::effective_spec`,
`holler-cli/src/pane/profile_scope.rs` `changed_spec` test helper (gains `None`),
`holler-pane-testkit/src/fixture.rs` and `src/conformance/pane_store.rs` (in-blast-radius testkit
literals). All other `Pane`/`ProfileSpec` construction sites are struct-update spreads (verified).

## Deviations found (issue vs tree)

1. `holler-cli/src/cli.rs` → flags live in `crates/holler-cli/src/pane/args.rs` (#670).
2. `holler-cli/cli-surface.txt` → `crates/holler-cli/tests/fixtures/cli-surface.txt`.
3. `CHANGELOG.md` is not in the blast radius though sibling feature PRs added entries; this run
   leaves it untouched (issue rule "edit only the files in the Blast radius").
4. The issue's branch rule `issue-<N>-implementation` → the plugin's `issue-0700-implementation`
   (0660 precedent: the plugin owns the naming).

## Governing rules pinned

- Closed code list frozen (ruling 3): no `error.rs` list edits; the guard's code is open.
- `deny_unknown_fields` fail-closed is intended (ADR-0021 §7): an old build reading a new-field
  record gets `store-corrupt` — acceptable and by design.
- I7: the key is a name, never a secret; nothing to echo.
- The epic's hot-spot exception: "`holler-pane/**` (for this field) and the flag are #700 only; it
  merges before #642, #644 and #647 start."
