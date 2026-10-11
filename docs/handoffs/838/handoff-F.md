# Handoff-F: #838 - Stage 1 asks which Herdr session to build in

**Date:** 2026-10-11
**Branch:** issue-838-implementation
**Issue:** #838

## What was done
- `agent-skills/herdr-setup/SKILL.md`:
  - Stage 0: the `| Herdr server state` row's Detail now says it lists every session from `herdr session list` with its status (running or stopped). The prefix is kept.
  - Stage 1: a new "Herdr session question" sits between the scenario menu and the workspace-name paragraph. It offers two answers. "The default, unnamed session" is recommended and writes no `[instance]` table. "A new named session" states the pattern, says the name must not be `default`, and refuses a name already in `herdr session list`, running or stopped, with the choice of another name or the default. It proposes the `[instance]` table (`state_dir = "~/.holler-<name>"`, 41808 / 8443 / 47101), says where the table goes, and has the file shown and confirmed before `instance.sh`. It states the outside-a-pane rule (pointing at "Two rules") and `HOLLER_STATE_DIR=<state_dir>` once. It says an existing config is not re-asked. The opener of the workspace-name paragraph is reworded ("once the orchestrators and the session are settled, ask the last question"), and the w5-pinned phrases are kept.
  - Stage 1 step 4: one sentence on where a named session's `[instance]` table is written.
  - Stage 1 Verify: states the targeted session ("Herdr session: `<name>`" or "Herdr session: default (unnamed)").
  - Stage 3 plan: a new "The Herdr session" bullet with both required strings.
- `docs/setup-wizard.md`: a new `### Choosing the Herdr session` subsection at the end of "Running an instance beside another". It covers both answers, the proposed values, the refusal, the outside-a-pane rule and `HOLLER_STATE_DIR`.
- `CHANGELOG.md`: an `[Unreleased]` Enhancements entry that links #838 and epic #726.

## Design decisions
- **Skip for (C) and for an existing config.** With (C) the user writes the file, and an existing config keeps its `[instance]` table (or lack of one). The alternative was asking anyway and editing the user's file, which the brief rules out ("not re-asked").
- **The name must not be `default`.** `instance.sh` refuses `name = "default"` for a non-default instance (evidence.md), so it is stated up front rather than left to fail at Verify.
- **The outside-a-pane rule is referenced, not restated.** This follows A warn 1: one short sentence that points at rule (2) of "Two rules", so there is no second full copy.
- **The docs heading is `###`** and does not start with `## Herdr session`. This follows A warn 2, so the `herdr.rs` slices are unchanged.

## Reuse / extend-vs-new
Extended the existing Stage 0 row, the Stage 1 interview, Verify, the Stage 3 plan list and the existing "Running an instance beside another" docs section. Nothing new was created: no script, no change to `instance.sh`, `herdr.sh` or `collide.sh`.

## Architecture notes for A
None. The change is text only (skill text, docs and changelog), with no Rust, no protocol and no script change.

## Deviations from spec / wireframe
None. One addition: the explicit "not `default`" rule on the name, which `instance.sh` already enforces.

## Tier 1 self-check (incl. tests now GREEN)
- `cargo test -p holler-cli --test wizard_scripts`: `test result: ok. 285 passed; 0 failed`. This covers all 9 `skilltext_838` tests and the older w5/w6/herdr text pins.
- `bash scripts/lint.sh`: exit 0, with pre-existing size warnings only (no touched file is listed).
- `bash scripts/changelog-check.sh`: `changelog-check: ok`.
- `cargo clippy --workspace --all-targets -- -D warnings`: one error, `redundant_iter_cloned` in `crates/holler-cli/tests/token_load_test.rs:60`. It is **pre-existing**: it reproduces on the base commit with this change stashed. It is a new lint in the local rustc 1.93.1, and the file is untouched by this story. See Known issues.
- `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (the CI test command): everything passed except two load-sensitive integration tests, which failed once under the parallel whole-workspace run. `interrupt_test` failed in `wait_warm` on one run, and `remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen` failed on another, a different test each run. Rerun alone, both binaries pass (`interrupt_test`: 11 passed; `remote_admin_test`: 21 passed). This change touches no Rust.

## Evidence appendix
`docs/handoffs/838/evidence.md`

## Tests that look wrong (for T)
None.

## Known issues
- A pre-existing clippy failure on the local toolchain (`token_load_test.rs:60`, `redundant_iter_cloned`) is out of scope: it is a test file, untouched here. CI's toolchain decides whether it fires there.
- The `interrupt_test` / `remote_admin_test` integration tests flake under the full parallel workspace run on this host. They pass in isolation, and the change is not related to them.

## Files changed
- `agent-skills/herdr-setup/SKILL.md`
- `docs/setup-wizard.md`
- `CHANGELOG.md`
