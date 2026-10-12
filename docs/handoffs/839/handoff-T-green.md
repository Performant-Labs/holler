# Handoff-T-green: Phase 7 (S rework, round 1) - #839 DRIVEN follows each pane's session of record

**Date:** 2026-10-12 (10:45 MDT)
**Branch:** `issue-839-implementation` (worktree `.claude/worktrees/0839-driven`, head `9eb55d4`; T's new tests are in the
working tree, uncommitted)
**Issue:** #839
**Handoff-F reviewed:** `docs/handoffs/839/handoff-F.md` (S rework round 1)
**Handoff-T-red:** `docs/handoffs/839/handoff-T-red.md`
**Reworks:** `handoff-S.md` (REWORK 1 test-only, REWORK 2 production). The first round's T-green is in git:
`git show 8d3e7fa:docs/handoffs/839/handoff-T-green.md`.

**VERDICT: GREEN, no blocking issue.**
- **REWORK 2 (F, production) is verified.** The existing `session-held` pins pass, and a new unit test pins the rule.
- **REWORK 1 (test-only, T's) is written.** Three new process-level tests in `driven_e2e_test.rs` pin every clause S
  listed.
- **Whole workspace:** 2725 passed, 0 failed, 36 ignored. That is round 1's 2721 plus the four new tests.
- **Production code:** none was edited by T.

## Tests added this round (test code only)

| Test | File | Pins | Tier, and why |
|---|---|---|---|
| `pane_not_driven_comes_first_in_all_three_verbs` | `crates/holler-cli/tests/driven_e2e_test.rs` | S (c), Acceptance 18 P5. `say`, `interrupt` and `answer --pane` exit 3 `pane-not-driven` in three cases: Q's id no body advertises (`-32003`); P's id that two live sessions advertise (`-32003` `ambiguous`, never exit 2); and P's id after its body went offline (`-32004` `offline`). Nothing reaches the session. | Process. The order of the CLI's arms over the real hub's wire answers is the behaviour, and only a real hub produces the ambiguous and offline answers. One rig serves all three cases. |
| `a_hold_on_the_pane_s_body_keeps_exit_4_for_the_routed_verbs` | same | S (a), Acceptance 18 P5 and Acceptance 7's last sentence. Under an operator hold (`holler hold kiwi/oc`) and a default hold (`--join-held kiwi/*`), `say --pane P TEXT` and `interrupt --pane P TEXT` each exit 4. The held text carries that kind's tail and contains neither `pane-not-driven` nor `session-held`. Nothing reaches the session. | Process. These are the real hold paths and the real CLI arms. |
| `a_pane_hold_on_the_id_prints_pane_held_for_say_pane` | same | S (b). P is healthy. Q is parked and its recorded DRIVEN is P's id. `--json say --pane P TEXT` prints exactly `{"error":"session_held","session":<id>,"reason":"pane-parked","hold_kind":"pane"}` on stdout and exits 3. Nothing reaches the session. | Process. The refusal comes from the hub's pane gate, through another pane's record. |
| `the_release_command_is_built_only_from_a_routable_name` | `crates/holler-pane/tests/driven_test.rs` | REWORK 2 (F's suggested pin). The code is `session-held`. A routable name is printed quoted, with `run holler release kiwi/oc`. Four names are not routable names: `kiwi/oc; rm -rf ~`, `kiwi/a b`, one with a newline and ESC, and `kiwi/OC`. For each, and for no name, the text contains neither the raw name nor `holler release kiwi`, and it names the id quoted. | Unit. `session_held` is a pure function, and both ports call it. |

The e2e file is 705 lines (lint fails at 900). `driven_test.rs` is 423 lines.

## GREEN confirmation

```text
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
175 test binaries: 2725 passed; 0 failed; 36 ignored            (exit 0)

$ cargo test -p holler-cli --test driven_e2e_test     14 passed (16 s); repeated 3 more times: 14 passed each
$ cargo test -p holler-pane --test driven_test        15 passed
```

**F's `session-held` pins, re-run:**
- `pane_verbs`: 381 passed. Both `held` tests pass: `driven::first::reset_first_on_a_held_pane_is_session_held`
  and `target_flags::every_other_held_error_falls_through_to_the_generic_arm`.
- `fake_hub_test::a_hold_refuses_prompts_and_not_a_rebind` passes.
- `driven_e2e_test::the_hold_key_survives_a_rebind_and_release_lets_the_first_message_through` passes.

**Spot-checks: each removal of a behaviour fails its test.** These tests were written after the code, so there is no
RED. Instead, each mutation below was a temporary edit to production code, restored with `git checkout --`.
Afterwards, `git status` shows only the two test files and the handoff docs.

| Mutation | Fails |
|---|---|
| `classify` no longer maps `-32004` `offline` to `NotDriven` | `pane_not_driven_comes_first_in_all_three_verbs` (the offline case) |
| `classify` maps `-32003` `ambiguous` to `Other` | the same test: `say --pane` exits 2 |
| `interrupt_cmd` without its `pane_not_driven` arm | the same test: `interrupt --pane Q` |
| `classify` reads an operator or default hold as `NotDriven` | `a_hold_on_the_pane_s_body_keeps_exit_4_for_the_routed_verbs` |
| `pane_hold_refusal` always `None` (no `PaneHeld` arm) | `a_pane_hold_on_the_id_prints_pane_held_for_say_pane` |
| `session_held` spells the raw name, unquoted, in the command | `the_release_command_is_built_only_from_a_routable_name` |

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 (size warnings only, none at 900) | PASS |
| Changelog | `bash scripts/changelog-check.sh origin/main` | ok | `changelog-check: ok` | PASS |
| Workspace tests | as above | 0 failed | 2725 passed, 0 failed | PASS |
| Docs | `docs_cli_test`, `docs_pane_control_test` (in the workspace run) | pass | 3 passed; 20 passed | PASS |
| Canary | `wire_selftest` (in the workspace run) | pass | 3 passed | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Hooks | `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| Whitespace | `git diff --check` | clean | clean | PASS |
| Format (new code) | `rustfmt --check --edition 2021` on the two test files | no diff in the new code | none in the new code (line 1 of `driven_e2e_test.rs` and the `#[path]` modules were already unformatted; CI does not run rustfmt) | PASS |

**Cross-check of F's results:** they agree. F reported clippy and lint at exit 0, `changelog-check` ok, `pane_verbs`
at 381 passed, `fake_hub_test` at 13 passed, `driven_e2e_test` at 11 passed (now 14 with T's three), and the
workspace at 2721 (now 2725).

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per acceptance item | Round 1's table, plus this round's tests for Acceptance 18 P5 and Acceptance 7's last sentence. S's PARTIAL on 18 is now covered clause by clause: (a) both hold kinds × `say` and `interrupt`; (b) `PaneHeld`; (c) unknown, ambiguous and offline × all three verbs. | PASS |
| Test quality | Each test names a behaviour and asserts the exit, the code and "nothing reached the session". None asserts implementation. Each fails under the mutation that removes its behaviour, and none duplicates a pin: the old `say --pane Q` unknown pin stays as Acceptance 3's check. Readiness is a bounded `wait_for` on the hub's own answer, with no fixed sleep. | PASS |
| Secrets in asserted output | The tests assert on refusal text that carries ids and names only, with no token or secret. | PASS |
| Message hygiene (REWORK 2) | The unit test covers a shell metacharacter, a space, ESC with a newline, and an uppercase name. | PASS |
| Type safety | Clippy `-D warnings` | PASS |
| API contract | No protocol change this round, and no golden changed | PASS |
| Migrations, Playwright | none in this repo or story | N/A |

**Evidence appendix:** two entries are appended to `evidence.md` under "What T's Acceptance 18 pins rely on":
- `hold_cmd.rs`: exit 4, and the tail for each hold kind;
- `live.rs`: `snapshot_offline`, which keeps a removed body's presence as offline.

## Acceptance criteria status

All 22 pass.
- **18:** now PASS. The pins come from round 1's tests and this round's three e2e tests.
- **7:** PASS. Its last sentence is pinned by `a_hold_on_the_pane_s_body_keeps_exit_4_for_the_routed_verbs`.
- **The rest:** unchanged from round 1, all green in this run.

## Blocking issues

None.

## Advisory notes

1. **S's advisory 1 still stands for O or the run's agent before merge.** The branch conflicts with `origin/main` in
   ADR-0021 "Decisions taken", and #839's items renumber to 34 to 47.
2. **F's trade-off is now pinned as intended.** `kiwi/OC` gets the fallback, not a command. If the operator wants
   harmless off-grammar names to keep their command, that changes this test.
3. **`docs/pane-control.md:290`** says `session-held` "names the `holler release SESSION`". The fallback cases, a
   failed roster read and a crafted name, name none. This is F's known issue, and it is non-blocking.

`T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.`
