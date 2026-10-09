# Handoff-T-red: Phase 4 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on fafd138, A's PASS commit)
**Brief / wireframe reviewed:** docs/handoffs/643-brief.md; docs/handoffs/643/handoff-A.md. Wireframe: N/A (no UI surface).

## A precondition

Confirmed: A returned PASS on the plan (handoff-A.md, 0 blocks, 8 warns). W-1 (which SHOWN/DRIVEN reading) is the MO's
call; A says T pins AC 3 as written, and T did. W-7 (T writes the `Args` fields before RED) is taken as the brief's
explicit exception: fields and one-line doc comments only, `run` untouched.

## Surface landed before the tests (the brief's Test plan; no behaviour)

- `crates/holler-cli/src/pane/list.rs`: `pane: Option<String>` (`value_name = "PANE"`).
- `crates/holler-cli/src/pane/get.rs`: `pane: String` (`value_name = "PANE"`).
- `crates/holler-cli/src/pane/watch.rs`: `pane: Option<String>`, `since: Option<u64>` (`value_name = "CURSOR"`),
  `until_idle: bool`.
  In all three, `ProfileOpt` is still flattened, and `run` is still the stub that answers `not_implemented(STORY)`. F
  keeps the fields and writes their docs.
- `docs/adr/ADR-0003.md`: the three `#643` rows are now exactly AC 20's rows.
- `crates/holler-cli/tests/fixtures/cli-surface.txt`: the `# #643` block is now exactly the nine lines of Decision 13.
- `crates/holler-cli/tests/pane_verbs/process/stub.rs`: the three `#643` `STUBS` rows are deleted, and the `// #643`
  line is kept.

`cargo test -p holler-cli --test cli_surface_test --test docs_cli_test --test pane_cli_process`: 3 + 3 + 34 passed, 0
failed.

## Tests authored

Every test runs in-process (`verb_harness::run_verb_with`) over the `Rig` of Decision 10, which uses test-kit fakes
only. This is the cheapest tier that runs the real verb through clap, dispatch and `output.rs`. The verbs are not
unit-testable in isolation, because their `run` is the unit. Every JSON check goes through `check_envelope` or
`check_ndjson`.

**`tests/pane_verbs/list.rs`** (519 lines). It holds `pub(crate) struct Rig` (`new`, `ports`, `run`,
`assert_nothing_observed`) and the shared helpers `pane`, `member`, `observed`, `scoped_rig`, `sync_rig`, `ok_envelope`,
`ok_stream`, `ok_text`, `assert_fails`, `cells`, `field` and `kv`. `testkit_links` and its `use holler_pane_testkit as
_;` are kept, so the manifest comment stays true.

| Test | Pins |
|---|---|
| `list_prints_a_header_and_one_row_per_pane_sorted_by_name` | AC 1 |
| `list_json_is_one_envelope_with_a_row_per_pane` | AC 2 |
| `list_flags_a_pane_whose_shown_and_driven_differ` | AC 3 (list, text and JSON) |
| `list_shows_an_unhealthy_server` | AC 4 (list and get) |
| `every_verb_prints_positions_row_first` | AC 5 (all three verbs, text and raw JSON) |
| `list_profile_lists_only_the_profile_s_members` | AC 6 (list) |
| `profile_refusals_exit_3_in_both_formats` | AC 6 refusals, all verbs |
| `bad_names_are_usage_in_both_formats` | AC 10 |
| `store_failures_exit_1_in_both_formats` | AC 11 (`unavailable` and `timeout`, all verbs) |
| `read_verbs_call_no_adapter_or_probe` | AC 17 (and Decision 9's "they write nothing": no `CasPut` or `Delete` call) |
| `list_help_documents_the_columns_and_the_json_shape` | AC 19 (list) |

**`tests/pane_verbs/get.rs`** (252 lines). It holds the `help(argv)` helper.

| Test | Pins |
|---|---|
| `get_shows_every_field_of_the_record` | AC 7: the text lines, and the JSON both without `--profile` (the `profile_store.get` path) and with it (the `resolve` path) |
| `get_pane_without_a_profile_shows_its_ceilings` | AC 8 |
| `get_profile_member_is_shown` | AC 6 (get) |
| `get_flags_a_mismatch` | AC 3 (get) |
| `get_missing_pane_is_pane_not_found_in_both_formats` | AC 9 |
| `get_requires_a_pane_name` | AC 9 (surface) |
| `get_help_documents_the_json_shape` | AC 19 (get) |
| `text_output_escapes_control_characters` | AC 18, all verbs |

**`tests/pane_verbs/watch.rs`** (267 lines). It holds `history_rig`, the AC 12 store.

| Test | Pins |
|---|---|
| `watch_from_the_start_prints_each_live_pane_once` | AC 12 |
| `watch_since_prints_each_later_change_once` | AC 13 (JSON, text, and `--since 6` in both formats) |
| `watch_prints_a_concurrent_change_exactly_once` | AC 14, 5 fresh-rig repeats |
| `watch_named_pane_follows_only_that_pane` | AC 15 |
| `watch_ends_at_a_store_error` | AC 15 |
| `watch_since_ahead_of_the_head_is_usage` | AC 15 |
| `watch_with_nothing_owed_prints_nothing` | AC 16 |
| `watch_profile_prints_only_member_changes` | AC 6 (watch, JSON and text) |
| `watch_flags_a_mismatch` | AC 3 (watch) |
| `watch_help_documents_the_stream` | AC 19 (watch) |

Test design choices that F and S should know about:

- **Refusal cases on `watch` carry `--until-idle`.** The cases are `--profile nope`, `demo-c4r1 --profile demo` and the
  bad names. This deviates on purpose from AC 6's literal argv. If an implementation failed to refuse before the stream
  opened, the case would then fail instead of hanging forever, because the fake's idle wait is zero. It does not change
  what is pinned: the refusal must still come before any line.
- **AC 18's `get` line count** is compared with the same pane holding clean strings. It is not a hard-coded 27, so the
  test pins "no forged line" and not the field count.
- **AC 14** polls the fake's call log for `PaneStoreOp::WatchNext`, bounded at 5 s, and stops early if the verb thread
  has already finished. It does not sleep blindly. The invariant it asserts is "exactly one envelope, cursor 7".
- **AC 17's grep half** (`ports\.(herdr|host|harness|prober)` in the three source files) and **AC 20-24** are command
  checks for T-green. They are not cargo tests.

## RED confirmation

Command: `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::`

```
test result: FAILED. 2 passed; 28 failed; 0 ignored; 0 measured; 60 filtered out
```

The whole `pane_verbs` target gives 62 passed and 28 failed. The only failures are the 28 new tests: no other module
regressed.

Every failure is an assertion about the missing behaviour, with no compile error, no parse panic and no harness
timeout. The stub answers `not-implemented` (exit 1) to every argv, so the failures fall into four kinds:

- **Exit 0 expected, got 1** (`ok_text`/`ok_envelope`/`ok_stream` at list.rs:144/151/158). The 19 tests are
  `list_prints_a_header…`, `list_json_is_one_envelope…`, `list_flags…`, `list_shows_an_unhealthy_server`,
  `every_verb_prints_positions_row_first`, `list_profile_lists…`, `get_shows_every_field…`,
  `get_pane_without_a_profile…`, `get_profile_member_is_shown`, `get_flags_a_mismatch`,
  `text_output_escapes_control_characters`, `watch_from_the_start…`, `watch_since_prints…`,
  `watch_prints_a_concurrent_change…`, `watch_named_pane…`, `watch_profile_prints…` and `watch_flags_a_mismatch`, plus
  `read_verbs_call_no_adapter_or_probe` (list.rs:485, `["pane", "list"] Text succeeds`) and
  `watch_with_nothing_owed_prints_nothing` (watch.rs:209). For example:
  ```
  assertion `left == right` failed: exit 0: Outcome { code: 1, out: "", err: "error: not implemented (story #643)\n" }
    left: 1
   right: 0
  ```
  In JSON mode, `out` is the stub's one `{"ok":false,...,"code":"not-implemented"}` envelope.
- **Exit 3 or 2 expected, got 1** (`assert_fails`, list.rs:168):
  - `profile_refusals_exit_3_in_both_formats`: `["pane", "list", "--profile", "nope"] text: exit 3 ... left: 1 right: 3`.
  - `get_missing_pane_is_pane_not_found_in_both_formats`: exit 3, got 1.
  - `bad_names_are_usage_in_both_formats`: `["pane", "list", "--profile", ""] ... left: 1 right: 2`.
  - `watch_since_ahead_of_the_head_is_usage`: exit 2, got 1.
- **Exit 1 expected (it matches), wrong code**:
  - `store_failures_exit_1_in_both_formats` (list.rs:196): `left: Some("not-implemented") right: Some("unavailable")`.
  - `watch_ends_at_a_store_error` (watch.rs:177): `err` is `error: not implemented (story #643)`, not
    `error: unavailable: ...`.
- **Help text missing** (AC 19):
  - list.rs:514: ``` `pane list --help` names "POS" ```.
  - get.rs:196: ``` `pane get --help` names "\"pane\"" ```.
  - watch.rs:262: ``` `pane watch --help` names "NDJSON" ```.

Two tests pass at RED, both by design:

- `list::testkit_links`: the pre-existing link marker, kept so the manifest comment stays true.
- `get::get_requires_a_pane_name` (AC 9): it pins the `get PANE` surface that T landed above. It turns red if the
  positional is made optional.

AC 17 is **not** vacuous at RED. Its test first requires each read to succeed (exit 0), so it fails today. At GREEN it
proves both that the verbs read the pane store and that no adapter or probe call is made.

Static checks on the touched files at RED:

| Check | Result |
|---|---|
| `rustfmt --check --edition 2021` on the 7 `.rs` files | exit 0 |
| `cargo clippy -p holler-cli --all-targets -- -D warnings` | exit 0 |
| `bash scripts/lint.sh` | exit 0 |
| Test file sizes | 519, 252 and 267 lines, all under 900 |
| New `#[allow]` | none |
| Secrets | none: neutral `demo-*`, `scratch`, `/srv/demo` and `localhost` only |

## Ready for F

RED is valid. F may implement against the authored tests. F must keep the `Args` fields as T landed them. F adds the
doc comments that AC 19 needs (`POS`, `SYNC`, `MISMATCH`, `"panes"`, `r2c1`, `--format=json`, `"pane"`, `"spec"`,
`"sync"`, `context`, `NDJSON`, `"cursor"`, `"change"`). F also adds the CHANGELOG entry (AC 23).
