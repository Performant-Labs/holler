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

## A, up-front plan review (second run, on the merged tree): 2026-10-11T10:37-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 6 `warn` findings (`docs/handoffs/650/handoff-A.md`, which replaces the first
  run's BLOCK). The amended brief (`ac17120`) resolves all eight findings of the first review. I checked each fix against
  the code at `7284001`, the tree with #802 merged in. #802 changes nothing the import calls, and launch now writes
  `pid: None`, as the import does. The warns:
  1. `no-tmux-session` is a second name for reconcile's `tmux-session-missing`, against the brief's own vocabulary rule.
     `session-not-found` is the closed `PaneError` code, so it is not a new code.
  2. The record's health line (`:45`) is stale. Acceptance 6 also records `unknown` for `Ok(true)` followed by a failed
     list, where reconcile records `healthy`.
  3. Acceptance 13 needs two Herdr sessions, and `FakeHerdr` serves one. `DisplacingHerdr` (`Displace::Session`) in the
     launch tests is the existing wrapper for this.
  4. The brief's list of calls the import never makes still names `serve`, which #802 removed. Several line references
     are stale. Acceptance 2 should be an allow-list.
  5. Acceptance 10's remedy commands leave the profile name unquoted, where `single_quoted` is the pattern (#662 B1).
  6. The text form's finding lines name no pane, and the brief does not say where findings about no pane print.
- **Assumed:**
  - With a PASS, the automated run goes on to T and the brief is not amended, so the warns reach T and F only through
    this handoff.
  - The import stays in the CLI after the cutover, so a code it merges stays in the vocabulary.
- **Hedged:**
  - Finding 1 is a warn and not a block, by the first review's own calibration. A naming inconsistency on the versioned
    surface (the `code`/`kind` key) was a warn; reusing a code for another meaning (`observe-failed`) was a block.
  - It is the one finding that cannot be fixed after merge, and the handoff says so to O.
- **Corrected:** the first review said `FakeHerdr` can hold panes in several sessions. It cannot (`herdr.rs:112`,
  `:362-386`).
- **Evidence:**
  - The amended brief, in full through "AI assistance", and the brief-gate result `650-brief-result-r3.md`.
  - The diff `263a2d7..7284001`: `ports.rs`, `lib.rs`, `tx_launch.rs`, the test kit's `harness.rs` and `host.rs`,
    `doctor/rig.rs`, `wiring.rs`, ADR-0021, and the host adapter's `ps`.
  - Source on the merged tree:
    - `holler-pane`: `reconcile.rs`, `reconcile/observe.rs`, `findings.rs`, `error.rs` (`PaneCode`), `pane.rs`
      (`Health`, `Pane`), `tx_launch.rs` (`HOST_NAME`, the session filter).
    - `holler-cli`: `pane/{import,doctor,launch,mod}.rs`, `profile/{mod,create,show,delete}.rs`, `output.rs`.
    - `holler-body/src/config.rs` (`ConfigError`, `parse`, `load`).
    - The test kit: `herdr.rs`, `fault.rs`, `harness.rs`, `host.rs`.
    - Tests: `pane_verbs/{main,doctor/rig,doctor/surface,launch/displaced,launch/rig,park/failures}.rs`,
      `profile_verbs/rig.rs`, `holler-pane/tests/findings_test.rs`.
    - Elsewhere: `holler-hub/src/circuit/dispatch.rs:92`, `cli-surface.txt`, `process/stub.rs`, ADR 0003's row,
      `scripts/lint.sh`, and both crates' `Cargo.toml`.
