# Handoff-T-green: Phase 7 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on 06320ad, F's commit)
**Issue:** #643
**Handoff-F reviewed:** docs/handoffs/643/handoff-F.md
**Handoff-T-red:** docs/handoffs/643/handoff-T-red.md

## GREEN confirmation

`cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::`: on F's commit **30 passed, 0 failed**. These are all
28 RED tests plus the two that passed at RED by design. After the three tests T added this phase (below), **33 passed,
0 failed**. The whole `pane_verbs` target gives 93 passed, 0 failed.

F listed no tests under "Tests that look wrong (for T)", and T found no wrong test. No authored test was changed.

**Tests added this phase** (T owns test authorship; each one closes a gap the mutation check below exposed, and no
production code was touched):

| Test | File | Pins |
|---|---|---|
| `list_named_pane_lists_only_that_pane` | `tests/pane_verbs/list.rs` | Decision 1: `list PANE` with no `--profile` lists that pane alone; a pane with no record gives the header alone, exit 0, and JSON `"panes": []` |
| `text_output_escapes_c1_and_bidi_characters` | `tests/pane_verbs/get.rs` | F's deviations 1 and 2 (the security claim in `--help` and the CHANGELOG). U+009B and U+202E never reach the terminal raw. A cwd with U+202E prints as `"/srv/demo\u{202e}x"`, and a `command` argv prints as the still-valid JSON `["opencode","a\u009bb","x\u202ey"]` |
| `watch_profile_prints_a_pane_leaving_the_profile_once` | `tests/pane_verbs/watch.rs` | Decision 7's membership rule, which T hedged at RED. A pane that leaves the profile prints that one change, and so does a pane deleted while a member. A later change to the pane outside the profile prints nothing. The test replays from `--since 1`, with no thread |

The test files stay under 900 lines (536, 280 and 311), with no `#[allow]`, no `unsafe` and no non-ASCII byte. The
bidi and C1 characters are written as `\u{..}` escapes, because rustc's `text_direction_codepoint_in_literal` lint
denies a raw U+202E in source.

**Spot-check: do the tests fail when the behavior is removed?** For each mutation, T applied a `perl` substitution
to F's code, ran the 33 tests, and restored the file with `git checkout`. Afterwards `git status` shows only the three
test files.

| Mutation | Caught by |
|---|---|
| M1 an unobserved side counts as a mismatch | 5 tests (AC 1, 2, 3 in list/get/watch) |
| M2 `text_value` never escapes | 4 tests (AC 4, 7, 18) |
| M3 `watch --profile` admits every event | `watch_profile_prints_only_member_changes`, the new leave test |
| M4 `watch PANE` filter off | `watch_named_pane_follows_only_that_pane` |
| M5 `list PANE` (no profile) lists every pane | **survived on F's commit**; caught by the new `list_named_pane_lists_only_that_pane` |
| M6 `get`'s `profile` always null | `get_profile_member_is_shown`, `get_shows_every_field_of_the_record` |
| M7 `get` without `--profile` never finds the spec | `get_shows_every_field_of_the_record` |
| M8 `list` does not sort | **survives**: the fake's `list` and `resolve` already return name order (evidence.md, T entry). Advisory |
| M9 `acts_on_terminal` reduced to `is_control` (drops bidi) | **survived on F's commit**; caught by the new C1/bidi test |
| M10 `json_text` leaves terminal characters raw | **survived on F's commit**; caught by the new C1/bidi test |
| M11 `watch --profile` ignores prior membership | **would have survived**; caught by the new leave test |

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0 | exit 0 (warnings only, on files outside this story) | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 (re-run after T's test edits) | PASS |
| `cargo test --workspace` (as `HOLLER_STATE_DIR=<empty scratch dir> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`, see note) | 0 failed | 125 result lines: 1409 passed, 0 failed, 5 ignored; exit 0 | PASS |
| `cargo test -p holler-cli --test pane_verbs` | 0 failed | 93 passed (after T's additions) | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | 0 failed | 3 passed | PASS |
| `cargo test -p holler-cli --test cli_surface_test` | 0 failed | 3 passed | PASS |
| `cargo test -p holler-cli --test pane_cli_process` | 0 failed | 34 passed | PASS |
| `cargo test -p holler-cli --test wire_selftest` (canary) | 0 failed | 3 passed | PASS |
| `cargo machete` | no unused deps | none found, exit 0 | PASS |
| `rustfmt --check --edition 2021` on the 7 touched `.rs` files | exit 0 | exit 0 (re-run after T's edits) | PASS |
| `cargo test -p holler-cli --test pane_verbs watch`, 3 runs (brief's flake check for AC 14) | 0 failed each | 11 passed, 11 passed, 11 passed | PASS |

Note on the workspace run: the workspace suite was isolated from this host's live hub with `HOLLER_STATE_DIR` and run
with `--no-fail-fast`, as F's handoff advised. The `--skip` is CI's own form. T's numbers match F's exactly (1409 / 0 / 5).
Server start and API smoke do not apply: until #649, the binary wires `Unwired`, and the verbs are exercised in-process
over the fakes.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage: a test per AC | AC 1-19 each map to a named test (table below); AC 20-24 are command checks, run here | PASS |
| Test quality | Each test names one behavior, fails in isolation on an assertion, and runs at the in-process tier (the cheapest one that exercises clap, dispatch and `output.rs`). Proportionate: 33 tests for three verbs and 19 behavioral ACs, with no duplicates found. The mutation table shows the tests pin behavior, not implementation | PASS |
| Type safety | No `as` sign cast (`observed_at` uses `u64::try_from`); exhaustive `match` for role, kind, health and hold; clippy clean under the workspace's deny set | PASS |
| Error paths | `usage` (AC 10, AC 15 `--since 99`), `profile-not-found` and `pane-not-in-profile` (AC 6), `pane-not-found` (AC 9), `unavailable` and `timeout` (AC 11), an error mid-stream (AC 15). All checked in both formats with matching exit codes | PASS |
| Data integrity | Read-only. AC 17's test also asserts no `CasPut` or `Delete` call. Each change appears once, in order, including a concurrent write (AC 12-14, 5 in-test repeats, plus 3 reruns) | PASS |
| API contract | JSON shapes pinned key by key (AC 2, 7, 8, 13). Every JSON check goes through `check_envelope` or `check_ndjson`. Positions print row first (AC 5) | PASS |
| Security | Text output is terminal-safe for C0 (AC 18) and, newly pinned, for C1 and bidi characters, in stored strings and in JSON-rendered argv. No secret-shaped value in any test or output (neutral `demo-*`, `/srv/demo`, `localhost`). Env prints names only (`EnvVarName`) | PASS |
| Migration safety | No schema, store or golden change | N/A |
| Protocol / goldens | No `holler-proto` change, no golden drift (`git diff --name-only` lists none) | N/A |
| `docs_cli_test` for documented commands | ADR 0003 rows parse (3 passed) | PASS |
| Blast radius (AC 24) | `git diff --name-only origin/main...HEAD` lists exactly the brief's files plus `docs/handoffs/643*` | PASS |
| Playwright / browser | No such surface in this repo | N/A |

**Evidence appendix:** F listed 11 facts. T appended 6 test-kit facts the tests rely on (sample-pane cwd and ceilings,
seed and cursor order, the `Cursor(0)` vs `--since` rule, the fake `list`'s sort, `check_ndjson`'s empty-stream
refusal). Each has a `file:line` and an excerpt T copied from source.

**F's commands cross-checked:** every command in handoff-F.md's Tier 1 self-check was re-run. The results match: 30 passed on
the read-verb filter, 90 on `pane_verbs` before T's additions, and 3/3/34 on the other CLI targets. Clippy, lint,
changelog, machete and rustfmt all exit 0. The AC 17/21/22 greps print nothing, the line counts are 316/275/227, and
the workspace numbers are identical. No discrepancy.

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 List, text | PASS | `list_prints_a_header_and_one_row_per_pane_sorted_by_name` |
| 2 List, JSON | PASS | `list_json_is_one_envelope_with_a_row_per_pane` |
| 3 SHOWN/DRIVEN flagged | PASS | `list_flags_a_pane_whose_shown_and_driven_differ`, `get_flags_a_mismatch`, `watch_flags_a_mismatch` |
| 4 Unhealthy shown | PASS | `list_shows_an_unhealthy_server` |
| 5 Row first | PASS | `every_verb_prints_positions_row_first` |
| 6 `--profile` scopes | PASS | `list_profile_lists_only_the_profile_s_members`, `get_profile_member_is_shown`, `watch_profile_prints_only_member_changes`, `profile_refusals_exit_3_in_both_formats` (+ `watch_profile_prints_a_pane_leaving_the_profile_once`, Decision 7) |
| 7 `get` whole record | PASS | `get_shows_every_field_of_the_record` |
| 8 `get` no profile, ceilings | PASS | `get_pane_without_a_profile_shows_its_ceilings` |
| 9 `get` missing pane | PASS | `get_missing_pane_is_pane_not_found_in_both_formats`, `get_requires_a_pane_name` |
| 10 Bad names usage | PASS | `bad_names_are_usage_in_both_formats` |
| 11 Store failures exit 1 | PASS | `store_failures_exit_1_in_both_formats` |
| 12 `watch` from start | PASS | `watch_from_the_start_prints_each_live_pane_once` |
| 13 `watch --since` | PASS | `watch_since_prints_each_later_change_once` |
| 14 Concurrent change once | PASS | `watch_prints_a_concurrent_change_exactly_once` (5 in-test repeats, 3 reruns) |
| 15 Named pane; store error; `--since` ahead | PASS | `watch_named_pane_follows_only_that_pane`, `watch_ends_at_a_store_error`, `watch_since_ahead_of_the_head_is_usage` |
| 16 Nothing owed | PASS | `watch_with_nothing_owed_prints_nothing` |
| 17 Observe nothing | PASS | `read_verbs_call_no_adapter_or_probe`; the grep prints nothing |
| 18 No terminal injection | PASS | `text_output_escapes_control_characters` (+ `text_output_escapes_c1_and_bidi_characters`) |
| 19 `--help` documents output | PASS | `list_help_…`, `get_help_…`, `watch_help_documents_the_stream` |
| 20 Surface rows | PASS | ADR 0003 lines 44-46 match exactly; the fixture's `# #643` block is the nine lines of Decision 13; `// #643` kept (one line) and the three `STUBS` rows gone; the three test targets pass |
| 21 No stub left | PASS | the `not_implemented\|const STORY` grep prints nothing |
| 22 Quality gates | PASS | Tier 1 table above; no `unsafe`; no manifest or lock diff; files under 900 lines; clippy's `too_many_lines` clean |
| 23 CHANGELOG | PASS | one `### Enhancements` entry linking #643, naming no host or account; `changelog-check: ok` |
| 24 Blast radius | PASS | `git diff --name-only origin/main...HEAD` |

(Decision 1's `list PANE` is now backed by `list_named_pane_lists_only_that_pane`.)

## Blocking issues

None.

## Advisory notes

- **M8 survives by construction.** Decision 4 says the verb sorts rather than relying on the store's order. Every
  test-kit fake already returns name order, so no fake-backed test can pin the verb's own `sort_by`. It matters only
  for a real store (#649) that returns a different order. A test would need a store double outside the kit, which the
  issue rules out.
- **`watch --profile --since N` replay edge (accepted in the brief's Risks and documented in `--help`).** Membership
  starts as the profile's panes *now*. A current member whose replayed history includes a change from before it joined
  prints that pre-join change once, because "was a member" is seeded from `resolve`. The rule as written implies this.
  It is noted so S can confirm it is the intended reading.
- **JSON mode passes DEL, C1 and format characters raw** (F's known issue, in `output.rs` under #660). This is outside
  this blast radius. It is for the MO.
- A's open warns W-1 to W-6 are unchanged by this phase. They are the MO's to settle.
- The three test files are ASCII-only. A raw U+202E in a Rust literal is a compile error, so future
  edits must keep using `\u{..}` escapes.
