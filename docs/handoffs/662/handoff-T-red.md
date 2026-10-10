# Handoff-T-red: Phase 4 - #662b profile write verbs (`holler profile create`, `holler profile delete`)

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `795717e`)
**Brief / wireframe reviewed:** `docs/handoffs/662-brief.md`; wireframe N/A (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (`docs/handoffs/662/handoff-A.md`, 0 blocks, 4 warns).

## What T changed

| File | Change |
|---|---|
| `crates/holler-cli/src/profile/create.rs`, `delete.rs` | **Signature stubs only, as the brief's Test plan directs.** The carried `ProfileCreate { name, from_current, from }` and `ProfileDelete { name, keep_panes }` clap structs. `run` still answers `not_implemented(662)`. Without the structs, `run_verb_with` panics on clap parse, which is not a valid RED. I did not stub `insert_profile` or `shell_word`. No test calls them (they are `pub(crate)`), and unused stubs would fail `clippy -D warnings` (`dead_code`). F adds them. |
| `crates/holler-cli/tests/profile_verbs/create.rs` | AC 2, replacing the stub case. |
| `crates/holler-cli/tests/profile_verbs/delete.rs` | AC 3, replacing the stub case. |
| `crates/holler-cli/tests/profile_verbs/rig.rs` | **Added only; no existing helper changes behaviour.** `Rig::run_over`. `Ran` (both formats plus the two rigs, so tests can read the stores afterwards). `run_both_with`. `run_both_seamed`. `assert_message_contains`. The Decision 13 / C11 seam `NthCasPut` + `NthPut::{Fail, ApplyThenFail}`. The seam lives in `rig.rs`, not `create.rs`, per A's W-4, and `delete.rs` uses it too. `run_both` now delegates to `run_both_with(.., Rig::run).both` and behaves the same; the existing list/show tests are still green. |
| `crates/holler-cli/tests/pane_verbs/process/stub.rs` | The two `("profile", ..., 662)` entries are deleted. The `// #662` line is kept (AC 4). |
| `crates/holler-cli/tests/fixtures/cli-surface.txt` | The `# #662` group is exactly the AC 5 list. |

## Tests authored

All tests are in-process through `run_verb_with` over the test kit's fakes. That is the cheapest tier that runs the real
verb, clap tree and `emit`: there is no subprocess and no temporary directory. Every test that runs the verb goes through
`run_both` or `run_both_with`, which checks that the exit codes match across formats, that the JSON passes `check_envelope`,
and that the run made no adapter or probe call (AC 1). Store assertions hold on **both** rigs (`Ran::rigs()`).

### create (`tests/profile_verbs/create.rs`)

| Test | Pins | Tier |
|---|---|---|
| `create_makes_an_empty_profile_in_both_formats` | AC 2a. Exit 0. Stored at generation 1 with no specs. `data.members == []`. `data.profile` is the record as stored. The exact single text line. No pane write. | in-process verb |
| `create_refuses_a_taken_name_or_slug` | AC 2b. Both `Some Profile` and `some-profile` give `profile-exists` exit 3 with `"Some Profile" (slug some-profile)`. The record is untouched. No profile write (the check runs before any write, B3). | in-process verb |
| `create_reports_a_create_race_as_profile_exists` | AC 2c, B7. A `Conflict` from `cas_put` becomes `profile-exists` exit 3 with `"Demo" (slug demo)`. | in-process verb |
| `create_from_makes_a_detached_copy` | AC 2d, B5. `Beta.panes == Alpha.panes`, including a spec for a pane that has no record. No pane write. Every pane record is unchanged. The two text lines. Runs again with the source spelled `ALPHA`: the text still says `copied from Alpha`. | in-process verb |
| `create_from_a_missing_profile_is_profile_not_found` | AC 2e. Exit 3, with `"Alpha"` in the message. Beta is not stored. No profile write. | in-process verb |
| `create_from_current_snapshots_every_pane_and_joins_it` | AC 2f. Three panes at r1c1, r1c2 and r2c1, each with a different cwd, port, model, effort, role, env names, ceilings, command, check and expect. Specs equal `profile_from_panes(NAME, seeded).panes`. Each record is its seed with only `profile` and `generation + 1` changed. `members` is in `list()` order. The r2c1 grid is `{"row":2,"col":1,"pos":"r2c1"}`. There is exactly one `members: ` line. | in-process verb |
| `create_from_current_of_no_panes_is_an_empty_profile_with_no_members` | Carried Decision 2 and the carried text `members: none`. | in-process verb |
| `create_from_current_joins_a_pane_that_already_names_the_profile` | Carried Decision 2. A pane whose profile already has NAME's slug is not refused, and it joins. | in-process verb |
| `create_from_current_refuses_a_pane_in_another_profile_and_writes_nothing` | AC 2g, B4. Two panes, in `Other` and `Alpha`, are refused together, `; `-joined, in `list()` order. Exit 3. No profile or pane write. | in-process verb |
| `create_from_current_undoes_everything_when_a_join_fails` | AC 2h. The seam fails the 2nd `cas_put` with `Conflict`. Result: `generation-conflict` exit 1, `nothing was kept`, the profile is gone, every pane's `profile` is None. | in-process verb + seam |
| `create_from_current_reports_profile_conflict_when_the_undo_fails` | AC 2i, B2. The seam fails the 2nd `cas_put` (`Conflict`) and the 3rd (`Unavailable`). Result: `profile-conflict` exit 1, a one-line message with `holler profile delete 'Some Profile' --keep-panes` and `holler profile show 'Some Profile'`. The profile still exists and pane 1 is still a member. | in-process verb + seam |
| `create_from_current_undoes_a_join_that_landed_but_timed_out` | AC 2m, B2/C8. The seam applies the 2nd `cas_put` and then answers `Timeout`. Result: `timeout` exit 1 and `nothing was kept`. The profile is gone and both panes' `profile` is None (pane 2 was re-read and cleared). | in-process verb + seam |
| `create_holds_env_names_never_values` | AC 2j, I7. `data` and the stored record hold `["ALPHA_TOKEN","BETA_URL"]`, with no `=`. The created spec, with `env` set to `TOKEN=s3cr3t`, fails to decode with `profile-secret-refused`, and the error text does not contain `s3cr3t`. | in-process verb + serde |
| `create_flags_conflict` | AC 2k. `ErrorKind::ArgumentConflict`. | clap parse |
| `create_with_a_bad_name_is_usage_in_both_formats` | AC 2l. A blank NAME, a blank `--from`, and a blank NAME with `--from-current` each give `usage` exit 2, with no write. | in-process verb |

### delete (`tests/profile_verbs/delete.rs`)

| Test | Pins | Tier |
|---|---|---|
| `delete_removes_a_profile_without_members_in_both_formats` | AC 3a + B6. Run both with and without `--keep-panes`. Exit 0, the profile is gone, the other profile is kept, `data == {"name":"Alpha","slug":"alpha","detached":[]}` (and the compact key order), the text is exactly one line, no pane write. A pane of another profile does not count as a member. | in-process verb |
| `delete_refuses_while_panes_are_live` | AC 3b. `profile-has-live-panes` exit 3. The message contains `"Some Profile"`, `2 live panes`, `demo-c1r1, demo-c2r1` and `holler profile delete 'Some Profile' --keep-panes`. No pane `CasPut`, no profile `Delete`. | in-process verb |
| `delete_keep_panes_detaches_then_deletes` | AC 3c. The profile is gone. Every pane record is kept: members get `profile` None and generation + 1, and the non-member is unchanged. `detached` is in member order. The two exact text lines. | in-process verb |
| `delete_of_a_missing_profile_is_profile_not_found` | AC 3d. Exit 3, with `"Some Profile"` in the message. | in-process verb |
| `delete_with_a_bad_name_is_usage_in_both_formats` | B3 (parse NAME first). `usage` exit 2. | in-process verb |
| `delete_conflict_without_members_is_generation_conflict` | AC 3e. Run with and without `--keep-panes`: exit 1 `generation-conflict`, and the profile is kept. | in-process verb |
| `delete_conflict_after_a_detach_is_profile_conflict` | AC 3f. `profile-conflict` exit 1, the message has `holler profile show 'Some Profile'` and `demo-c1r1`, the member's `profile` is None, and the profile still exists. | in-process verb |
| `delete_stops_at_a_failed_detach` | AC 3g. `generation-conflict` exit 1, `detached so far: none`, `the profile was not deleted`. Both panes are still members. No profile `Delete`. | in-process verb |
| `delete_stops_at_a_failed_second_detach_naming_the_first` | **Added (A's W-4 gap).** The seam fails the 2nd detach. The message has `detached so far: demo-c1r1;`. Pane 1 is detached and not re-attached (Decision 8), pane 2 is still a member, and the profile is not deleted. | in-process verb + seam |
| `delete_counts_members_by_slug` | AC 3h. A pane in `SOME-PROFILE` blocks the delete of `"Some Profile"` (exit 3). The message has `1 live pane (demo-c1r1)`, the singular through `count`. | in-process verb |
| `delete_quotes_a_name_with_a_quote_in_its_suggested_command` | **Added (B1).** The only test of the `'\''` branch: `holler profile delete 'Bob'\''s Panes' --keep-panes`. | in-process verb |

AC 4 and AC 5 are surface edits that T made (STUBS and the fixture), checked by the existing `pane_cli_process`,
`cli_surface_test` and `docs_cli_test` targets. AC 1's grep, AC 6 and AC 8-12 are F's or S's gates.

## RED confirmation

`cargo test -p holler-cli --test profile_verbs` gives:

```
test result: FAILED. 18 passed; 25 failed; 0 ignored; 0 measured; 0 filtered out
```

The result was identical in 6 consecutive runs. The 18 passes are the 17 existing list, show and stub tests, plus
`create_flags_conflict`.

Every one of the 25 new verb tests fails on its **first behavioural assertion**, against the stub's `not-implemented`
(exit 1). None fails on a compile error, a missing `[[test]]`, a clap parse panic or a harness error. Representative lines,
one per kind:

```
create::create_makes_an_empty_profile_in_both_formats @ create.rs:120  left: 1  right: 0
    Outcome { code: 1, out: "", err: "error: not implemented (story #662)\n" }
create::create_refuses_a_taken_name_or_slug @ rig.rs:175 (assert_failure)  left: 1  right: 3
create::create_with_a_bad_name_is_usage_in_both_formats @ rig.rs:175  left: 1  right: 2
create::create_from_current_undoes_everything_when_a_join_fails @ rig.rs:187
    left: "not-implemented"  right: "generation-conflict"
create::create_from_current_reports_profile_conflict_when_the_undo_fails @ rig.rs:187
    left: "not-implemented"  right: "profile-conflict"
create::create_from_current_undoes_a_join_that_landed_but_timed_out @ rig.rs:187
    left: "not-implemented"  right: "timeout"
delete::delete_keep_panes_detaches_then_deletes @ delete.rs:124  left: 1  right: 0
delete::delete_stops_at_a_failed_second_detach_naming_the_first @ rig.rs:187
    left: "not-implemented"  right: "generation-conflict"
```

(The rest are the same two shapes: exit 1 where 0, 2 or 3 is expected, or code `not-implemented` where the expected code
is `profile-conflict`, `generation-conflict` or `timeout`. In those failure-code cases the exit is already 1, so the code
comparison is what fails.)

**Passes without the behaviour (journalled):**
- `create_flags_conflict` passes as soon as the clap structs exist. It is a property of clap, as the brief predicts.
- The decode half of `create_holds_env_names_never_values` is a property of `EnvVarName` that is already true. The test
  still fails, because the verb half runs first. It overlaps `holler-pane/tests/argv_env_test.rs:96`, but it decodes the
  spec the verb created. AC 2j asks for it, so I kept it.

**Surface targets:**
- `cargo test -p holler-cli --test cli_surface_test`: ok, 3 passed. The fixture lines parse against the stub structs.
- `cargo test -p holler-cli --test pane_cli_process`: ok, 34 passed. With the STUBS entries gone, nothing runs a bare
  `profile create`.
- `cargo test -p holler-cli --test docs_cli_test`: **FAILED. This is the intended RED for AC 5, and the file is F's.**

  ```
  docs/adr/ADR-0003.md:65: `holler profile create   ...   #662` → ["holler", "profile", "create"]: one or more required arguments were not provided
  docs/adr/ADR-0003.md:66: `holler profile delete   ...   #662` → ["holler", "profile", "delete"]: one or more required arguments were not provided
  ```

  It goes green when F rewrites lines 65-66 to `holler profile create NAME [--from-current | --from PROFILE]` and
  `holler profile delete NAME [--keep-panes]`.

**Hygiene:**
- `cargo clippy -p holler-cli --all-targets -- -D warnings` is clean.
- `bash scripts/lint.sh` exits 0. The touched files are 522, 299, 306 and 215 lines, all under 900, and none adds an
  `#[allow]`.
- `rustfmt --check --edition 2021` on all six touched `.rs` files exits 0.
- `grep -n assert_stub_routes` on the two test files prints nothing. `grep -c '662),' stub.rs` prints 0.

## Notes for F

- Store assertions run on both rigs, so text mode must write exactly as JSON mode does.
- The seam counts `cas_put`s **made through it** (the joins, B2's re-read clear, and the undo writes), not the fake's
  call log. A `NthPut::Fail` never reaches the fake.
- `create_from_current_reports_profile_conflict_when_the_undo_fails` expects this undo order:
  1. B2 step (1) does a `get` of the failed pane. That pane is not a member, so there is no write.
  2. Then the 3rd `cas_put` is pane 1's undo, which is where the undo stops.

  An implementation that does the steps in another order fails this test.
- Prose `{name:?}` means `format!("{:?}", name.as_str())` (B4), not `{:?}` of `ProfileName`. The tests match
  `"Some Profile"`, `"Other"` and `"Alpha"` with plain double quotes.
- A's W-3 (naming the helper `single_quoted`) is F's call. The tests only check the quoted output.

## Ready for F

Confirmed: RED is valid. F may implement against these tests.
