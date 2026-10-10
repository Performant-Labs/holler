# Handoff-T-red: Phase 4 - #662a profile verbs: the pure core and the read verbs

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `d175825`; this run is 662a only)
**Brief / wireframe reviewed:** `docs/handoffs/662-brief.md` (as amended in `31062ce`), `docs/handoffs/662/handoff-A.md`; no wireframe (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (round 2, `d175825`), with five warns and no block.

## What T landed (staged, not committed)

**Signature stubs**, following the brief's Test plan ("the allowed approach"). They contain no logic, and F replaces every body:
- `crates/holler-pane/src/profile_snapshot.rs`: `FIXED_PORT_POLICY_PREFIX` (correct), `fixed_port_policy` -> `""`,
  `spec_from_pane` -> a spec of empty strings at r1c1 with zero ceilings, `profile_from_panes` -> no specs, empty slug.
- `crates/holler-pane/src/profile_diff.rs`: every type of the API. `SpecField::ALL` holds the right 15 entries in the right
  order. `as_str` -> `""`, `value` -> `Text("")`, `Display` -> empty, `is_member` -> `false`,
  `diff_spec` and `diff_profile` -> empty. **`SpecField` serializes through `impl Serialize` via `as_str()`**, with no
  per-variant renames (A warn 1). The JSON and the API are unchanged. F may keep this or use the brief's renames: the tests
  accept either.
- `crates/holler-cli/src/profile/show.rs`: `ProfileShow { pub name: String }`; `run` still returns `not_implemented(662)`.
  `list.rs` is unchanged (`ProfileList {}`).

**Surface changes** (the brief's Files (T) list):
- `crates/holler-cli/tests/pane_verbs/process/stub.rs`: the `("profile","list",662)` and `("profile","show",662)` entries
  are deleted. `// #662` is kept, and `grep -c '662),'` prints `2`.
- `crates/holler-cli/tests/fixtures/cli-surface.txt`: in the `# #662` group, `profile show |` / `profile show | --json`
  become `profile show | Demo` / `profile show | "Some Profile" --json`. The `create`/`delete` lines stay as they are
  until 662b.

## Tests authored

| Test | Pins (AC) | Tier, and why |
|---|---|---|
| `holler-pane/tests/profile_snapshot_test.rs` | | Integration test over `tests/common` records (Decision 11). Pure functions: no port, no fake. |
| `snapshot_copies_every_spec_field_from_the_pane_record` | 1a: the whole mapping table against a spec written out by hand, `port_policy == "fixed:48100"` | same |
| `snapshot_ignores_the_fields_a_spec_does_not_hold` | 1a (the "Not copied" list): generation, session, host name, tmux, API version, pid, session of record, profile and `probe.last` do not leak into the spec | same |
| `snapshot_takes_the_position_from_the_record_not_the_name` | 1b: `demo-c1r2`@r2c1 -> r2c1, `demo-c2r1`@r1c1 -> r1c1 | same |
| `fixed_port_policy_is_the_prefix_and_the_port` | Decision 3 / #644's pin: `fixed:<port>` for 0, 8095, 48100, 65535; a port-0 record snapshots to `fixed:0` | same |
| `profile_from_panes_keeps_order_and_starts_at_generation_zero` | 1c: name, slug, generation 0, `created`/`updated` 0, one spec per pane in the order given, empty input | same |
| `holler-pane/tests/profile_diff_test.rs` | | same tier (Decision 11) |
| `diff_reports_each_differing_field_once` | 2a: grid, cwd and effort are changed, which gives 3 `FieldDiff`s in `ALL` order with their values; the raw grid JSON string is the brief's, exactly | same |
| `diff_compares_env_and_expect_as_sets_and_argv_in_order` | 2b: a reorder or a repeat is not a difference; a different set is, with stored-order values; a reversed command or check differs; an absent argv differs from a present one | same |
| `snapshot_round_trips_to_no_difference` | 2c: `common::pane()` (fully populated) and a sparse orchestrator pane (no command, check, env or expect) | same |
| `diff_profile_classifies_matches_differs_missing_extra` | 2d: spec rows in the profile's order, then the extras in the order of `live`; `differences` is non-empty exactly for `Differs`; a `Missing` row's JSON | same |
| `spec_field_values_serialize_like_the_spec` | 2e: `as_str()` equals the 15 dotted paths in order; `to_value(field)` is its path; `to_value(field.value(spec))` equals the spec's JSON at that path (an absent `command`/`check` is `null`, role `orchestrator`) | same |
| `field_text_escapes_control_characters` | 2f: ESC and a newline print escaped; argv prints as a JSON array, and a spaced element stays one element | same |
| `field_values_print_for_a_person` | The rest of the `Display` table: `r2c1`, decimal, a list as JSON, `Argv(None)` as `none`, plain text | same |
| `diff_compares_port_policy_as_the_live_port` | 2g: `fixed:48100` against live port 48101 gives one diff (`fixed:48101`); the bare `fixed` differs | same |
| `is_member_compares_slugs` | 2h | same |
| `holler-cli/tests/profile_verbs/rig.rs` (new; declared from `list.rs` by `#[path]`) | The shared rig: the two fake stores, the fake adapters and prober, and `scope` = `Unwired`. `run_both` runs each format on a fresh seed and asserts equal exit codes, `check_envelope`, and `assert_no_adapter_call()` (AC 3) on both rigs. `assert_failure` checks code, exit, and text-mode out empty / err `error: ...`. | Test seam (Decision 12) |
| `list::list_reports_name_slug_panes_live_generation_in_both_formats` | 4a: data sorted by slug (seeded in reverse); the exact text lines; the raw JSON key order of a derived struct | In-process verb test over the fakes: the cheapest tier that runs the real verb through `emit` |
| `list::list_counts_members_by_slug` | 4b: a pane with profile `SOME-PROFILE` counts for `Some Profile`; a member of `Other` counts only for `Other` | same |
| `list::list_of_no_profiles` | 4c | same |
| `show::show_reports_differences_field_by_field_in_both_formats` | 5a: the block under `pane demo-c1r1: differs` is exactly `  herdr.grid: spec r1c1, live r2c1`, `  probe: none`; the JSON row; `data.profile` is the profile as stored; the raw `"live":{"row":2,"col":1,"pos":"r2c1"}` | same |
| `show::show_reports_nothing_for_a_matching_pane` | 5b | same |
| `show::show_reports_the_last_probe_result_without_running_one` | 5c: members with a stored `check` and `probe.last` Ok / Failed give `  probe: ok` and `  probe: failed (missing "qwen38")`, plus the JSON forms; prober calls are empty (through `run_both`) | same |
| `show::show_prints_command_and_check_as_argv_arrays` | 5d: the spec block's `command` and `check` lines; a differing command prints `spec [..], live [..]` as JSON arrays (with a spaced element); JSON arrays | same |
| `show::show_lists_missing_and_extra_panes` | 5e, plus A warn 5: the header `generation 1, 3 specs, 2 live`; matches, missing (no record; a detached spec naming a pane of `Other`), extra; **a spec pane holding ESC prints escaped and no raw ESC reaches `out`**; a missing row has no probe line; an extra row has `probe: none` | same |
| `show::show_of_a_missing_profile_is_profile_not_found_in_both_formats` | 5f: exit 3, out empty in text mode, the message names the profile | same |
| `show::show_passes_a_store_failure_through` | 5g: a wedged `FakePaneStore` gives `timeout`, exit 1 | same |
| `show::show_passes_a_profile_store_failure_through` | Added: a failing profile store is `unavailable` (exit 1), never mistaken for `profile-not-found` | same |
| `show::show_with_a_bad_name_is_usage_in_both_formats` | Added: a blank NAME is `usage`, exit 2, from the verb (Behaviour, show) | same |

## RED confirmation

The suite runs at `d175825` with the staged changes (logs: `cargo test -p holler-pane --test profile_snapshot_test --test profile_diff_test --no-fail-fast; cargo test -p holler-cli --test profile_verbs`):

```text
profile_diff_test:     FAILED. 1 passed; 8 failed
profile_snapshot_test: FAILED. 0 passed; 5 failed
profile_verbs:         FAILED. 6 passed; 12 failed   (the 6 passing are the untouched stub cases of create/delete/apply/rename/export/import)
```

Every failure is an assertion about the missing behaviour, and none is a compile, setup or seed error. Representative lines:

```text
snapshot_copies_every_spec_field...  left: ProfileSpec { pane: "", ... port_policy: "" ... }  right: ProfileSpec { pane: "demo-c1r2", ... "fixed:48100" ... }
fixed_port_policy_is_the_prefix...   left: ""  right: "fixed:0"
profile_from_panes_keeps_order...    left: ""  right: "some-profile"
diff_reports_each_differing_field... left: []  right: [FieldDiff { field: Grid, ... }, FieldDiff { field: Cwd, ... }, FieldDiff { field: Effort, ... }]
spec_field_values_serialize...       left: ["", "", ...]  right: ["herdr.workspace", "herdr.grid", ...]
field_text_escapes_control_chars...  left: ""  right: "/srv/a\\u{1b}[31mb\\nc"
is_member_compares_slugs             assertion failed: is_member(&p, &some)
list_* / show_* (10 of 12)           Outcome { code: 1, out: "", err: "error: not implemented (story #662)\n" }  left: 1  right: 0 (or 2, 3)
show_passes_a_*_store_failure_through  left: "not-implemented"  right: "timeout" / "unavailable"
```

**Passes for want of behaviour (expected, per the brief):** `snapshot_round_trips_to_no_difference` (2c). The stub's
`diff_spec` returns nothing, so 2c holds vacuously. Its real force is at GREEN. AC 5b is not vacuous: it fails on exit 1.

**The tests are satisfiable.** I checked this before restoring the stubs. In a scratch pass, a throwaway reference
implementation written only from the brief's API and "What each verb prints" made all 14 pure tests and all 18
`profile_verbs` tests pass, and `cli_surface_test` and `pane_cli_process` passed too. I then reverted the throwaway code
byte for byte to the stubs above. None of it is staged, and F writes its own.

**Surface state at RED:**
- `cli_surface_test`: PASS (3/3). `profile show | Demo` parses against the stub `Args`.
- `pane_cli_process`: PASS (34/34). The list/show STUBS entries are gone.
- `docs_cli_test`: **FAIL, F's to fix (AC 9).** `docs/adr/ADR-0003.md:68` `holler profile show   #662` no longer parses
  because NAME is now required. F's ADR-0003 row edit (`holler profile show NAME`) fixes it.

**Lints at RED:**
- `cargo clippy -p holler-pane -p holler-cli --all-targets -- -D warnings` exits 0.
- `bash scripts/lint.sh` exits 0. Each new test file stays under 900 lines (the largest is 313), and every `#[allow]`
  carries `// #662`.
- `rustfmt --check --edition 2021` passes on every touched `.rs` file.
- No manifest or `Cargo.lock` change.

**Unrelated failure seen in `cargo test --workspace`:** four `logging_test` cases fail with
`Unexpected success` on `holler roster`. A hub is reachable from this machine, so `roster` succeeds where the test expects
the failure it gets with no hub. The roster path is untouched by this diff. This is environmental, not a regression of
this story. Re-check it at GREEN in a clean environment (CI).

## Notes for F

- The verb tests pin the text forms of "What each verb prints" exactly:
  - two-space indents;
  - `<path>: spec <v>, live <v>`;
  - `pane <name>: matches|differs|missing (no live pane)|extra (no spec)`;
  - a probe line on every row except `Missing`;
  - spec blocks (`spec <pane>` plus 15 field lines) before the pane rows;
  - the header `profile <name> (<slug>): generation G, N specs, L live`, where L is the number of members.
- `list`'s text is `<name> (<slug>): N panes, L live, generation G`. The tests use only counts of 0 or 2, so how a count of 1
  is pluralized is left to F.
- Every spec pane prints through the control-character escape (`FieldValue::Text` `Display`): the `spec <pane>` header and
  the pane-row name alike.
- `show`'s JSON row is `{"pane","status","differences","probe"}`, as one flat object.

## Ready for F

RED is valid. F may implement against these tests: `profile_snapshot.rs`, `profile_diff.rs`, `profile/list.rs`,
`profile/show.rs`, the ADR-0003 `list`/`show` rows, ADR-0021 Decision 14 (i) and (iii), and CHANGELOG.
