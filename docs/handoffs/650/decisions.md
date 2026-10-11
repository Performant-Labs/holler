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

## T, Phase 4 (author / RED): 2026-10-11T10:53-06:00 (MDT)

- **Decided:** RED is valid. 39 new verb tests and 5 code-table tests:
  - 38 verb tests fail because the verb has none of its flags yet. With the flags declared on the stub (a scratch edit,
    reverted), each fails on its own assertion against `not-implemented`.
  - The ADR row test fails on its assertion.
  - `cli_surface_test` fails on the two new fixture lines.
  - `import_test` fails to build on exactly `import::findings` and `IMPORT_BLOCKED`.

  On A's six warns, since the brief was not amended:
  1. Reconcile's `tmux-session-missing`, applying the brief's own spelling rule. A code is stable once merged.
  2. `unknown` health after a failed list, keeping acceptance 6's text over A's suggestion.
  3. The import rig has its own `ExtraHerdr`.
  4. Acceptance 2 is an allow-list.
  5. Remedy commands are single-quoted (`'fleet'`).
  6. Where pane-less findings print is left to F. Every pinned `import-blocked` count involves no pane-less problem.

  In `stub.rs`, the `// #650` line is kept per the file's own rule, and the forced-diagnostic test now takes any stub.
- **Assumed:**
  - **Field values.** A `pane.env` variable other than the five read ones goes to `env` by name; the five are not in
    `env`. Rows use `"record"` as the action in both the dry run and the real run.
  - **Message wording.** `source-unreadable` positions read `line N` (and `column C` for the launcher table). An
    `ignored-entry` message names the directory. A `sources-disagree` message names `last-session`.
  - **The fixtures.** A launcher table that does not parse blocks the one pane of its fixture. The fake harness's shared
    data directory lists every pane's sessions on every port, which the agreement cases do not depend on.
- **Hedged:**
  - Decision 2 differs from A's recommendation. If O amends the brief to reconcile's `healthy`, change one assertion in
    `join::a_failed_session_list_is_observe_failed`.
  - Decision 1 deviates from the brief's literal code list. It follows the brief's own rule and A's finding, and the
    handoff says so for S and the diff gate.
  - The three property tests in `import_test.rs` are guards that pass vacuously on a one-variant stub.
- **Evidence:**
  - The amended brief in full, `handoff-A.md` and this journal.
  - Test kit: `herdr.rs`, `harness.rs` (`reach`, `frozen`, `mint_session`), `host.rs`, `fault.rs`, `pane_store.rs`,
    `profile_store.rs`, `fixture.rs`, `envelope.rs`.
  - Tests: `doctor/rig.rs`, `launch/displaced.rs`, `launch/rig.rs` (`error_of`, `assert_untouched`),
    `profile_verbs/rig.rs` (`NthCasPut`), `process/{stub,flags,docs_rows}.rs`, `cli_surface_test.rs`,
    `verb_harness/mod.rs`.
  - Source: `holler-body/src/config.rs` (`parse`, `RawFile`, `parse_mode`), `profile/{create,delete,show}.rs`,
    `holler-pane/src/{error,argv,grid,pane}.rs`, and ADR-0021's section 9 and open-codes rows.
  - Runs: `cargo test -p holler-cli --test pane_verbs` (267 pass, 39 new fail), `--test cli_surface_test`,
    `--test pane_cli_process` (36 pass), `cargo test -p holler-pane --test import_test` (E0432 on the two items),
    `cargo clippy -p holler-cli --no-deps --tests -- -D warnings` (clean), `bash scripts/lint.sh` (exit 0).

## F, Phase 6 (implement): 2026-10-11T11:24-06:00 (MDT)

- **Decided:** the verb is built as the brief splits it (`docs/handoffs/650/handoff-F.md` lists every file).
  - **The plan** is in `holler_pane::import`: `plan()`, `Plan`, `PaneRow`, `Action`, `IMPORT_BLOCKED` and `FLEET`. It has
    the submodules `files` (the plain values the CLI hands it), `findings` (`ImportFinding`, `Level`, `Finding`), `observe`
    (a thread per pane) and `assess` (the join and the record).
  - **The CLI** (`pane/import.rs` and its submodules `sources`, `pane_env`, `launcher`, `writes`) reads the files, writes
    and undoes. `push_spec` is widened (visibility only).
  - **Read order:** Herdr's version, then its snapshot (either error ends the run); `pane_store.get` per pane; the
    not-OpenCode check; then the remaining panes are observed at once. A pane left alone keeps only its own finding.
  - **The launcher table is strict** (`deny_unknown_fields`). A refusal names the file, line and column for every pane.
    Port 0 and a cwd that is not absolute are `source-unreadable`. A shell-string command's text is never kept.
  - **The text form** (A finding 6): each pane's line (`PANE  ACTION  ROLE  POS  SESSION`, two spaces apart, no header),
    followed by its findings; findings about no pane first, under `-`; the dry run's profile through `push_spec`; a
    summary line.
  - **`import-blocked`'s N** counts panes with a problem once each, plus problems about no pane.
  - **`profile-exists`** is checked with `profile_store.get` before any write.
  - **The undo** answers the write's own code when it completes. When a step fails it answers `profile-conflict`, naming
    the records kept and acceptance 10's two remedies, with `'fleet'` quoted.
  - **Docs:** ADR-0003's row; ADR-0021 sections 9 (import row, open-codes row), 10 (the join) and 13 (the launcher
    table's shape, step 2 narrowed); a CHANGELOG entry.
  - `text` in `show.rs` is not widened, since the import never calls it.
- **Assumed:**
  - A `sessions.toml` or `owned-sessions` that is absent imposes no constraint on the session. A present one with no id
    (a `sessions.toml` with no row) disagrees.
  - A `sessions.toml` with no non-OpenCode row marks an OpenCode pane, and so does a pane with no `sessions.toml`.
  - `pane.env`:
    - the last assignment wins, as the shell's does;
    - an empty `OC_DEFAULT_AGENT` is no agent;
    - an empty `OC_EFFORT` is a bad value;
    - only the first bad line is reported.
  - A missing launcher file, `--launcher` included, is `source-missing` per pane, not a usage error.
  - A state root that cannot be listed is `unavailable`, not `usage`.
- **Hedged:**
  - Two tests read the log window after the rig's own `List`, so they fail on the window only:
    `writes::a_failed_write_is_undone_and_answers_its_own_code` and `a_write_that_landed_and_timed_out_is_undone_too`. Their
    observed tails are `[Get, Delete, List]` and `[Delete, Delete, List]`, which are the intended undo order plus that
    `List`. The fix is test-only, for T. Everything else in both passes.
  - `gitleaks protect --staged` (the pre-commit hook) flags T's made-up value at
    `tests/pane_verbs/import/sources.rs:170` as a `generic-api-key`. The commit of the staged set is refused until T
    changes that line (`// gitleaks:allow`, or another name and value). F's files are clean.
  - The follow-up issue acceptance 10 names (a verb that removes only a record) is not filed. That is an outward act for
    O or S.
- **Evidence:**
  - Every file of T's RED suite, `handoff-T-red.md`, `handoff-A.md` and the brief through E46.
  - Source read before use: `reconcile.rs` and `reconcile/observe.rs`, `findings.rs`, `error.rs`, `argv.rs`, `grid.rs`,
    `pane.rs`, `profile.rs`, `profile_snapshot.rs`, `profile_diff.rs`, `ports.rs` and `tx_launch.rs`.
  - CLI and body: `profile/{create,delete,show,list}.rs`, `pane/{mod,doctor,launch,list}.rs`, `output.rs`, and
    `holler-body/src/config.rs`.
  - The test kit's `herdr.rs`, `harness.rs`, `host.rs`, `fault.rs`, `pane_store.rs` and `profile_store.rs`, and toml
    0.8.23's `de.rs`.
  - Runs:
    - `cargo test -p holler-pane --test import_test` (5 pass);
    - `cargo test -p holler-cli --test pane_verbs` (304 pass, the 2 above fail);
    - `cli_surface_test`, `docs_cli_test`, `pane_cli_process`, `profile_verbs` (all pass);
    - `cargo clippy -p holler-cli -p holler-pane --no-deps --all-targets -- -D warnings` (clean);
    - `rustfmt --check` on F's files (clean);
    - `bash scripts/lint.sh` (exit 0);
    - the pipeline's workspace command with `--no-fail-fast` (149 targets: 2158 pass, 25 ignored, the 2 above fail and
      nothing else);
    - the real binary's usage paths (exit 2, and `not-implemented` past them until #649).
  - The facts in unchanged code are in `docs/handoffs/650/evidence.md`.

## T, Phase 7 (verify / GREEN): 2026-10-11T11:42-06:00 (MDT)

- **Decided:**
  - F's two "tests that look wrong" were wrong in the test: they read the log window after `rig.records()`, whose own
    `List` is logged. The repair takes the tail right after the run; the expected undo orders are unchanged.
  - No production change is needed. Verdict PASS.
- **Assumed:** the gitleaks finding F reported was settled before `660efe8`, since the sentinel is now
  `zq-env-secret-c3c3` and a scan of the commit is clean. T made no change for it.
- **Hedged:** AC 10's follow-up issue (a verb that removes only a record) is still not filed. That is an outward act, for O
  or S.
- **Evidence:**
  - `cargo test -p holler-cli --test pane_verbs`: 306 passed.
  - `cargo test -p holler-pane --test import_test`: 5 passed.
  - The workspace command with `--no-fail-fast`: 149 targets, 2160 passed, 0 failed, 25 ignored.
  - Workspace clippy with `-D warnings`: clean.
  - `lint.sh`, `changelog-check.sh`, `docs_cli_test`, `wire_selftest`, `cargo machete` and `test-hooks.sh`: all pass.
  - Two production mutations, both reverted, were each caught by the test that names the behaviour.
  - Three evidence entries were appended to `evidence.md`.

## A, Phase 7 (anti-duplication gate): 2026-10-11T11:50-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 2 `warn` findings (`docs/handoffs/650/handoff-A-dup.md`).
  - **Production has no parallel path.** F reused every object the map named: `insert_profile`, `profile_from_panes`,
    `push_spec` (widened), `holler_body::config::load`, `FindingKind::code` and `PaneCode::as_str`, `quoted` and
    `embedded`, `is_member`, `single_quoted`, `pane_list`, `count`, `optional_text` and `HOST_NAME`.
  - **The new objects are the brief's own:** `ImportFinding`, the four readers and the deleting undo.
  - **The only copies are reconcile's deliberate ones:** the scaffold, the server and tmux classification,
    `failed_message` and the call-name constants.
  - Warn 1: `import/rig.rs`'s `findings`, `data_of` and `error_of` re-implement doctor's `json_findings` and `json_data`
    and launch's `error_of`. All three are reachable in `pane_verbs`.
  - Warn 2: two pass-through `HerdrPort` wrappers and three failing-N-th-write seams now exist. A follow-up should move
    them into the test kit.
- **Assumed:**
  - The diff to review is `243f910..f5c75d3`, the story's code after A's plan review. The handoff and journal files are
    not reviewed for duplication.
  - The PR's CI builds and tests the merge with `origin/main`. I did not build it.
- **Hedged:**
  - Warn 1 is a warn and not a block, by the role's rule for a codebase with no dominant pattern. Close, launch and doctor
    each keep their own envelope helper, and switch and park read the envelope inline. The copies are also test-only and
    small.
  - The overlay's "a near-copy is a rejection" names the hub's harness helpers (`Hub`, `Body`, ...), not these.
- **Evidence:**
  - Read in full:
    - every changed source file in `holler-pane/src/import*` and `holler-cli/src/pane/import*`;
    - the `show.rs` diff;
    - every new test file;
    - the `doctor/rig.rs`, `stub.rs`, `cli-surface.txt`, ADR-0003, ADR-0021 and CHANGELOG diffs.
  - Compared against:
    - `reconcile.rs` and `reconcile/observe.rs`, `findings.rs`, `tx_launch.rs` (`record`, `HOST_NAME`, the
      `--herdr-session` check) and `profile_diff.rs`;
    - `profile/{create,delete,list,show}.rs`, `pane/{list,get,doctor,args,launch}.rs` and `output.rs`;
    - the test files `doctor/rig.rs`, `launch/rig.rs`, `launch/displaced.rs`, `close/rig.rs`, `park/failures.rs`,
      `relaunch.rs`, `profile_verbs/rig.rs` and `holler-pane/tests/findings_test.rs`.
  - Searched the workspace for an env-file reader, a helper that turns an offset into a line and column, an `hj-`
    constant, a role-text helper, other `NthCasPut` copies and other users of `json_data` and `json_findings`.
  - `scripts/lint.sh`'s size gate covers `*.rs` only, and every touched `.rs` file is under 600 lines.
  - `git log 7284001..origin/main`: #644 merged as `b8242d7`, adding `SpecField::OpencodeAgent`, which `push_spec` now
    prints. `git merge-tree --write-tree HEAD origin/main` exits 0. The files both sides touch are `CHANGELOG.md`,
    ADR-0003 and ADR-0021.

## S, spec audit (Phase 10): 2026-10-11T12:10-06:00 (MDT)

- **Decided:** PASS on `7284001..542b5dc`. `docs/handoffs/650/handoff-S.md` has the detail.
  - All fourteen acceptance criteria have a test that asserts the behaviour, and T-green's two mutations fail the named
    tests.
  - Acceptance 10's last clause, "a follow-up issue is filed", is an outward act and is not done. It is a pre-merge
    action for the run's agent (advisory 1), not F's rework.
  - The operator decisions and the brief's sections are implemented as stated. The code departs from the brief's
    literal text only where a decision is recorded:
    - `tmux-session-missing`: the brief's own spelling rule, and A's warn 1;
    - the health after a failed list: the operator's comment;
    - the kept `// #650` line: the operator's comment;
    - `text` not widened;
    - the text form's layout: A's warn 6.
- **Assumed:**
  - The brief governs where it differs from the issue body, as the operator's comment on #650 says (2026-10-11 11:33
    MDT).
  - T's and F's recorded runs stand. I re-ran no build or test, per the role. My only checks were read-only:
    `rustfmt --check` on the touched files, `git merge-tree`, greps and `wc -l`.
  - The script commits this handoff and this entry at the phase commit.
- **Hedged:**
  - The brief contradicts itself on `no-tmux-session`: its list names it, while its rule and its join item 3 point to
    reconcile's code. I did not call this an `ADVISORY-HOLD`, because the code follows the rule and the operator's own
    reason ("reuse `pane doctor`'s vocabulary"). A code is stable once merged, so the issue should record the choice
    before the merge (advisory 3).
  - Three message clauses are pinned by reading only (advisory 4, test-only, size XS).
  - `origin/main` moved to `3286486` (#642 part 3) after A-dup's trial merge. The merge now conflicts in `CHANGELOG.md`
    and in ADR-0021's open-codes row, both text only (advisory 2).
- **Evidence:**
  - Live:
    - `gh issue view 650`: the body is the text from before the amendments, and its one comment is the operator's
      amendment.
    - `gh issue list --search` for the follow-up: none found.
    - `git merge-tree --write-tree HEAD origin/main`: two doc conflicts.
  - Read:
    - the brief through "Evidence", with E10 and the `holler_body::config` evidence;
    - every handoff, this journal and `evidence.md`;
    - the outside diff gate's result (PASS; NV-1 settled by `RawSession.harness`, which has no default).
  - Diffs: `git diff origin/main...HEAD` for every production, test, fixture, ADR and CHANGELOG file.
  - Source checked:
    - `holler_body::config`: `RawSession`, `ConfigError`, `load` and `parse`;
    - `Argv`, `PaneName::parse`, `insert_profile`, `is_member`, `pane_list`, `optional_text` and `class_of`;
    - the fake harness's `health`, `list_sessions` and `reach`;
    - five `evidence.md` excerpts.
  - Greps over the added lines:
    - panics and `#[allow]`;
    - logging calls on the import path;
    - sleeps in the new tests;
    - private identifiers.
