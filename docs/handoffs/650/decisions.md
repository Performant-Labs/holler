# Decision journal: #650, import today's panes from the launcher's files

Run handoff directory `docs/handoffs/650` · branch `issue-650-implementation` · rigor second-opinion · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry. The brief gate's rounds are in
`docs/handoffs/650-brief-result-r{1,2}.md` (round 1 BLOCK, round 2 PASS).

## A, up-front plan review: 2026-10-11T07:28-06:00 (MDT)

- **Decided:** BLOCK, with 2 `block` and 6 `warn` findings (`docs/handoffs/650/handoff-A.md`). The plan's shape is sound:
  - the split matches #662's: the plan in `holler-pane`, the writes and undo in `holler-cli` through the reachable
    `insert_profile`;
  - no manifest changes;
  - the hub accepts the profile-first write order;
  - `ImportFinding` can take every reused code from its one source.

  The two blocks:
  1. A TOML error's text quotes the offending source line. The plan sends such text through `quoted`/`embedded`, which
     keep content, so a `source-unreadable` finding about `sessions.toml` or the launcher table can print a secret.
     A malformed `pane.env` line has no safe "variable" to name.
  2. The plan reports a server that answers unhealthy as `observe-failed`. That merged, stable code means a call
     returned an error, while reconcile names this condition `server-down`/`server-wedged` and records
     `Unhealthy(<code>)`. Acceptance 6 pins it, so neither T nor F can fix it within the brief.

  The warns:
  3. `--herdr-session` has no stated effect on the join.
  4. A failed undo's remedy is unnamed, and no verb removes a record without closing the pane.
  5. The JSON finding key is `code` where doctor's is `kind`.
  6. The ADR-0021 section 9 row and section 13's table shape are incomplete.
  7. `push_spec` is copied rather than widened.
  8. Panes are observed in sequence where reconcile uses a thread each.
- **Assumed:**
  - The real `sessions.toml` may hold spawn rows (the brief contemplates them), and the real launcher table may hold an
    extra key. The repository has no sample of either, which the brief states.
  - With no O in the automated path, this BLOCK stops the run (`needs-amendment`). Recovery is an amended brief and a
    fresh run.
- **Hedged:**
  - Finding 2's fix. I recommend reconcile's classification, which adds one `list_sessions` call on an unhealthy server,
    used only to classify. The alternative is an import-owned code. The operator chooses.
  - Finding 4's four-step remedy relies on no record belonging to another profile, which `--from-current` requires.
- **Evidence:**
  - Read the brief in full, both outside brief-gate results, and #808's A handoff (precedent for a brief that serves as
    the survey).
  - Crates: `crates/holler-cli/Cargo.toml` and `crates/holler-pane/Cargo.toml`.
  - `holler-cli`: `profile/{mod,create,delete,show}.rs`, `pane/{mod,doctor,launch,list}.rs`, `output.rs`, `query_cmd.rs`.
  - `holler-pane`: `findings.rs`, `reconcile.rs`, `reconcile/observe.rs`, `tx_launch.rs` (relevant parts), `error.rs`
    (relevant parts).
  - Elsewhere: `holler-body/src/config.rs`, `holler-adapter-herdr/src/adapter.rs` (snapshot),
    `holler-hub/src/panes/store.rs` (membership rule), `tests/pane_verbs/doctor/rig.rs`, `scripts/lint.sh`.
  - ADR-0021 sections 5, 7, 8, 9, 12 and 13; `docs/testing.md`.
  - `toml` 0.8.23 and `toml_edit` 0.22.27 sources in the local cargo registry: `TomlError`'s `Display` writes the raw
    line, and `from_str` attaches the raw input.
