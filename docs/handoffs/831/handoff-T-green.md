# Handoff-T-green: Phase 7 - #831 adopt, adopting the fleet's existing Claude Code panes

**Date:** 2026-10-12
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-adopt`, head `2e7f426`)
**Issue:** #831
**Handoff-F reviewed:** `docs/handoffs/831/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/831/handoff-T-red.md`

**VERDICT: GREEN, no blocking issue.** No test was edited in this phase (F's "Tests that look wrong" is "None", and no case failed).

## GREEN confirmation

- `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the CI command): exit 0,
  no `FAILED` result in any binary (7 min 51 s). The 46 RED verb cases, the 19 adapter cases, the 2 docs cases and the 2
  `import_test` cases from Phase 4 all pass; `pane_verbs -- adopt` is 47 passed.
- Repeats for flakes: the adapter's `foreground_pids_test` + `foreground_pids_bound_test` x5 and `pane_verbs -- adopt` x3: all
  green every time (the bound tests are deadline invariants, not durations).
- `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored`: 10 passed (the typed-in program is found on a real tmux).
- `cargo test -p holler-pane --doc`: 12 passed (three `HostPort` `compile_fail,E0046` doctests).
- Spot-check that the tests pin behavior (production edited temporarily, then `git checkout`-reverted; the tree is clean):
  - `adopt.rs`: drop the `Several` return -> `two_matching_processes_block_and_the_walk_stops_at_the_second` fails.
  - `adopt.rs`: disable the `NoSession` arm -> `a_matching_process_with_no_session_is_unmatched_and_blocks` fails.
  - `adopt.rs`: drop `login_env` -> `a_login_variable_in_pane_env_blocks_the_pane_without_its_value` fails.
  - `exec.rs::ps_group`: only the group leader matches -> `a_member_of_the_foreground_group_that_is_not_its_leader_is_found` and
    `two_panes_answer_together_...` fail.
  - A mutant dropping `pid` from the record did not compile (dead field), so it proves nothing; the whole-record equality in
    `a_typed_in_claude_pane_is_adopted_...` is the pin.

## Tier 1 results

| Check | Result |
|---|---|
| `cargo test --workspace` (CI form) | PASS |
| `bash scripts/lint.sh` | PASS, exit 0 (largest test file `fake_tmux_test.rs` 826, `adapter_test.rs` 899 untouched; none over 900) |
| `bash scripts/changelog-check.sh` | PASS |
| `cargo clippy --workspace --all-targets --keep-going -- -D warnings` | One error only: `holler-cli` `token_load_test.rs:60` (`unneeded cloning of iterator items`). Not part of this diff (`git diff origin/main..HEAD` on that file is empty), same as F reported at main on this toolchain. Everything the story touches is clean. |
| `cargo test -p holler-cli --test docs_cli_test` | PASS, 3 |
| `cargo test -p holler-cli --test wire_selftest` | PASS, 3 |
| `cargo machete` | not installed on this machine (`no such command`); no dependency was added (F: "No new dependency"), so N/A |

## Tier 2 results

- **Coverage per criterion:** every acceptance criterion has a named test (below). PASS.
- **Test quality:** each test names one behavior; the suite is large (about 49 verb + 19 adapter cases) but split by tier
  (adapter vs fake host vs in-process verb); the verb cases that look alike (`adopt_undo.rs`) pin distinct failure points. No
  duplicate found to delete or merge. PASS.
- **Timing:** no fixed sleeps in the new tests; bound tests assert "timeout and fewer `ps` spawned" invariants; repeated x5. PASS.
- **`#[allow(...)]`:** none added without a `// #NNN` link in the changed files. PASS.
- **Secrets:** login-variable and model findings name the variable only; tests assert the value is absent. PASS.
- **Protocol:** five additive finding codes, no wire change; no golden drift. The guide's `holler ...` lines parse (`docs_cli_test`,
  `docs_pane_control_test` 23). PASS.
- **Evidence appendix:** `evidence.md` carries F's entries; the tests rely on nothing else in unchanged code. PASS.
- **Duplication notes for A (Phase 8):** one `Pane` literal (`record()`) and the shared `begin`/`finish` are in place, as A's warns 1 and 2 asked.
- **No browser/UI surface** in this repo; none run.

## Acceptance criteria status

Backed by the Phase 4 files (`adopt.rs`, `adopt_problems.rs`, `adopt_fleet.rs`, `adopt_undo.rs`, `harness.rs`, the adapter
`foreground_pids*_test.rs`, `docs_pane_control_test.rs`): AC1 (docs amendment: docs tests), AC3 (flag-off Claude pane unchanged:
`harness.rs`), AC4/D3, AC5/AC6/D1 (`adopt_fleet.rs`), AC7, AC8, AC9, AC10 (`adopt_undo.rs`, both shapes), AC11, AC12, D2 (typed-in,
real tmux). All PASS.

## Blocking issues

None.

## Advisory notes

- `token_load_test.rs:60` clippy failure and the ~100-file `cargo fmt` drift in `holler-cli` are pre-existing on this toolchain.
- The real hub store's behavior for the undo's equality check is untested here (F's known issue); the testkit store is the only proof.
- `cargo machete` could not run locally; CI covers it.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
