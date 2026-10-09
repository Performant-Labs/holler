# Handoff-T-green: Phase 7 - #637 slice a, the `holler-pane` crate (VERIFY / GREEN + Tier 2)

**Date:** 2026-10-09
**Branch:** issue-637-implementation (on top of 37c1f32)
**Issue:** #637
**Handoff-F reviewed:** docs/handoffs/637/handoff-F.md
**Handoff-T-red:** docs/handoffs/637/handoff-T-red.md

## GREEN confirmation

`cargo test -p holler-pane --no-fail-fast`: all six RED test files pass (argv_env 6, error 12, grid 5, names 8, ports 6, records 11 = 48) and both `compile_fail` doctests pass. `cargo test -p holler-proto --lib methods`: 2 passed. F's numbers reproduce exactly: `cargo test --workspace --no-fail-fast` gives 91 result lines, 911 passed, 0 failed, 5 ignored (before T's additions).

F flagged no wrong test. T added one file, `crates/holler-pane/tests/adopted_test.rs` (9 tests, 207 lines), for the behaviors F adopted from A's rows and the RED handoff said T would cover. No production file was touched. The holler-pane suite is now 57 tests plus 2 doctests, run 3 more times with identical results (no flake; the tests use no clock, thread or process).

Spot-check that the tests still fail when the behavior is removed (production line mutated, suite run, `git checkout` restored):

| Mutation | Killed by |
|---|---|
| slug-equals-name check on `Profile` read disabled | `a_profile_whose_stored_slug_disagrees_with_its_name_does_not_load` |
| `next_generation` compare inverted | `next_generation_is_the_one_compare_and_swap_rule` (and the RED CAS tests) |
| `deny_unknown_fields` removed from `HostInfo` | `records_and_params_refuse_an_unknown_field` |
| `EnvVarName` control-character check removed | `grid_accepts_an_absent_pos_..._env_refuses_control_characters` |
| slug made Unicode-aware | `the_slug_is_ascii_only_and_a_non_ascii_letter_is_a_separator` |

The doctest "remove the privacy" check (making the `RefusalCode` field `pub`) is the one that did **not** fail: both `compile_fail` doctests still pass, because they fail on the missing `From<&str>` (E0277) and on the `from_static` const assertion (E0080), not on field privacy. See Advisory notes.

## Tier 1 results

| Command | Result |
|---|---|
| `cargo build --workspace` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` (with the new test file) | PASS, no warning |
| `cargo test --workspace --no-fail-fast` | PASS: 911 passed, 0 failed, 5 ignored (matches F) |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3) |
| `cargo machete` | PASS, no unused dependency |
| `bash scripts/lint.sh` | PASS (exit 0); only new warn is `crates/holler-pane/src/error.rs` at 608 lines (warn 600, fail 900) |
| `bash scripts/changelog-check.sh` | PASS |
| `bash scripts/golden-diff-summary.sh` | PASS, no output, no golden drift |
| `rustfmt --check --edition 2021` on every new `.rs` file, `methods.rs` and `adopted_test.rs` | PASS |
| `git diff --name-only origin/main` plus untracked | PASS: only Blast-radius paths (`docs/handoffs/637*` covers `637-split-proposal.md`) |

## Tier 2 results

- **Coverage per criterion:** PASS, see the table below. F's adopted behaviors (A rows 2, 4, 6-9) now each have a named test.
- **Test quality:** PASS. Each test names a behavior and fails in isolation for the right reason; every test is a unit test at the cheapest tier, none duplicates another; the suite is proportionate (largest file `ports_test.rs`, 538 lines, all under 900). No `#[allow]` without a `// #637` link. `adopted_test.rs` overlaps no RED test (checked against `argv_env_test`, `error_test`, `records_test`).
- **Type safety:** PASS. `cargo clippy -D warnings` under the workspace lint set (unwrap/expect/panic denied); no `unsafe`.
- **Error handling:** PASS. Every closed code is reached; invalid wire code gives `unavailable`; a malformed reply gives `unavailable`; guard failures keep their own code through `decode_params`; anything else is `usage`.
- **Data integrity (CAS, unknown fields):** PASS. Stale `cas_put` and stale `delete` give `generation-conflict`; unknown fields on records, nested records, params and events are refused; a counter at `u64::MAX` gives `store-corrupt`, not a wrap.
- **API contract:** PASS. Reply and record JSON pinned (`{ok, data, error:{code, message, detail?}}`, grid key order, hold/health arms); `PANE_METHODS` (5) and `PROFILE_METHODS` (7) outside the 22-row `CATALOG`; no golden drift.
- **Security (I7, secret absence):** PASS. `NAME=value` is `profile-secret-refused` and the refusal text never echoes the value (asserted again through `decode_params`); no field can hold an env value.
- **Migration safety:** N/A (no schema or stored data yet).
- **Playwright / browser:** N/A (none in this repo).
- **Evidence appendix:** nothing to add. T's new tests rely only on facts in the diff.
- **Cross-check of F's results:** PASS, no discrepancy.

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 gates (build, clippy, test, machete, lint, changelog, rustfmt, CHANGELOG #637, no file over 900) | PASS | Tier 1 table |
| 2 grid | PASS | `grid_test.rs` (5), `adopted_test::grid_accepts_an_absent_pos_...` |
| 3 `Argv` / `EnvVarName` | PASS | `argv_env_test.rs` (6), `adopted_test::{argv_from_json_..., decode_params_...}` |
| 4 record round trips, stale CAS | PASS | `records_test.rs` (11), `adopted_test::{records_and_params_refuse_..., a_profile_whose_stored_slug_...}` |
| 5 error taxonomy, `RefusalCode`, reply round trip, `compile_fail` | PASS | `error_test.rs` (12), 2 doctests, `adopted_test::a_reply_whose_ok_and_error_disagree_...` |
| 6 ports, `Ports`, name grammar tables | PASS | `ports_test.rs` (6), `names_test.rs` (8) |
| 7 methods outside the catalog, no golden change | PASS | `methods::tests` (2), `golden-diff-summary.sh` |
| 8 blast radius | PASS | `git diff --name-only origin/main` plus untracked |

## Blocking issues

None.

## Advisory notes

1. **`RefusalCode` field privacy is not pinned by a doctest.** Making the field `pub` leaves the suite green. The brief and the RED handoff both asked for a closed-code constructor route only; a public field would let a caller build `RefusalCode(Cow::Borrowed("Not A Code"))` or a closed code and bypass the validation. F confirmed the field blocks direct construction (E0423) in a scratch crate, so the production code is right today. T cannot add the missing `compile_fail` (doctests live in `src/`, which T may not edit). Suggested third doctest on `RefusalCode`, for O or F at their discretion, not a blocker: ` ```compile_fail,E0423 ` over `use holler_pane::error::RefusalCode; let _ = RefusalCode(std::borrow::Cow::Borrowed("Not A Code"));`.
2. `HarnessPort::health` returns `bool` while `Pane.harness.health` can carry `Unhealthy(reason)`; provisional until #635 (F's note, agreed).
3. `error.rs` is 608 lines (lint warn); fine, three exhaustive matches.
4. `# for` consumer markers on the workspace `serde` / `serde_json` lines do not list `holler-pane`; the workspace `Cargo.toml` is outside the radius and `lint.sh` check 5 passes.
5. A closed variant without a payload is rebuilt from its code alone on parse-back (peer's message text not kept); documented by F.

---

# Rework pass: Phase 7 re-entry after S's REWORK (2026-10-09, 05:14 MDT)

**Handoff-F reviewed:** docs/handoffs/637/handoff-F.md ("Rework pass" section). F flagged no wrong test; it listed six test jobs for T and said it edited two test type-annotation lines (`tests/common/mod.rs` `empty_watch`, `tests/ports_test.rs` `MemProfileStore::watch`) at the coordinator's instruction. T reviewed both: type annotations only, no assertion changed.

## What T changed (tests only; no production file touched)

New `crates/holler-pane/tests/rework_test.rs` (7 tests, ~190 lines, `// #637` on the one `#[allow]`):

| Test | Pins |
|---|---|
| `a_bad_env_in_a_spec_or_a_pane_is_refused_by_code_and_never_echoed` | S REWORK 2. 11 bad `env` shapes (`"TOKEN=hunter2"`, `"=hunter2"`, `["A","B=hunter2"]`, bare name, `""`, non-string element, object element, map, number, null, `["A b"]`), each in a `ProfileSpec` and a `Pane`, directly and through `decode_params`: the code (`profile-secret-refused` or `env-name-invalid`), and `hunter2` absent from the serde error, `Display`, `Debug`, the serialized `PaneReply::failure`, with the reply parsing back to the same code |
| `a_good_env_is_still_a_list_of_names_and_an_absent_env_is_empty` | the guard did not break the accepted shapes (`[]`, two names, absent) |
| `an_idle_watch_item_is_not_an_error_and_the_stream_stays_usable` | D1: `Ok(None)` is skipped, later `Ok(Some)` still arrive, no error |
| `a_timeout_from_the_store_is_a_failure_that_ends_the_stream` | D1: `Err(Timeout)` is a failure, never reported as idle |
| `watch_params_decode_from_an_empty_object_as_from_the_beginning` | A-dup row 3: `decode_params::<WatchParams>({})` gives `Cursor(0)` |
| `every_harness_kind_serde_name_is_in_the_protocol_vocabulary` | D2: each `HarnessKind` serde name is in `holler_proto::HARNESS_IDS` (exhaustive match, so a new variant forces an update); an unknown id does not load |
| `deleting_a_missing_pane_is_pane_not_found_at_any_generation` | A-dup row 2: `pane-not-found` for 0, 1, 7, `u64::MAX`; a present record keeps conflict and not-found apart |

Edited test doubles (my files): `tests/common/mod.rs` `MemPaneStore::cas_put` and `delete`, and `tests/ports_test.rs` `MemProfileStore::cas_put` and `delete`, now go through `holler_pane::next_generation` (A-dup row 4); both `delete`s check a missing record first and return `pane-not-found` / `profile-not-found` (A-dup row 2). The older empty-iterator assertions (`ports_test.rs` `watch.next().is_none()`) still mean "the stream ended" and stay valid under the new item type; idle is now covered by the new Watch tests.

## GREEN confirmation

```
$ cargo test -p holler-pane --no-fail-fast
  adopted 9, argv_env 6, error 12, grid 5, names 8, ports 6, records 11, rework 7  (64 tests), all ok
  Doc-tests holler_pane: 3 passed (all compile_fail)
$ cargo test -p holler-proto --lib methods                      2 passed
$ cargo test --workspace --no-fail-fast                         93 result lines: 928 passed, 0 failed, 5 ignored
```

Mutation spot-check (production line changed, suite run, file restored with `cp`; `git status` confirmed clean afterwards): removing `deserialize_with = "crate::argv::deserialize_env_names"` from `Pane.env` makes `a_bad_env_in_a_spec_or_a_pane_is_refused_by_code_and_never_echoed` fail (the other 7 rework tests pass), so the test pins the behavior. The D1 tests are type-and-contract pins (the production side is a type alias), so they have no production mutation to kill; they fail to compile if the item type reverts to `Result<T, _>`.

## Tier 1 results

| Command | Result |
|---|---|
| `cargo build --workspace` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (one `single_element_loop` in my new test was fixed first) |
| `cargo test --workspace --no-fail-fast` | PASS, 928 passed / 0 failed / 5 ignored (F reported 921; the difference is my 7 new tests) |
| `cargo test -p holler-cli --test docs_cli_test` / `--test wire_selftest` | PASS (3 / 3) |
| `cargo machete` | PASS |
| `bash scripts/lint.sh` | PASS (exit 0); warn only: `error.rs` 617 lines (limit 900) and two pre-existing files |
| `bash scripts/changelog-check.sh` | PASS |
| `bash scripts/golden-diff-summary.sh` | PASS, no output |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane --no-deps` | PASS |
| `rustfmt --check --edition 2021` on every `holler-pane/tests` file | PASS |
| `git diff --check`; `gitleaks dir crates/holler-pane/tests` | clean; no leaks |

## Tier 2 results

| Check | Result |
|---|---|
| Test quality: each new test names a behavior, fails in isolation, sits at unit tier, no duplicate | PASS. The `next_generation` double check was dropped as redundant with `next_generation_is_the_one_compare_and_swap_rule` |
| Suite proportion | PASS: test files 1910 lines in all; largest `ports_test.rs` ~540; none near 900 |
| Secret never in logs/errors (S REWORK 2) | PASS: asserted absent in 4 renderings |
| Error paths and API contract | PASS: closed-code table, reply round trip, params JSON |
| No sleeps, clocks, threads, or processes | PASS: all unit tests |
| Every `#[allow]` carries `// #NNN` | PASS |
| Protocol/golden/docs impact | none (control-socket names only) |
| S's REWORK 1 (trait docs in `ports.rs`) | doc-only; `cargo doc -D warnings` clean; not testable beyond that |

## Acceptance criteria status

AC 1 PASS (gates above). AC 2 PASS (`grid_test.rs`). AC 3 PASS now: the bare-string and `=`-bearing `env` gap S found is closed and pinned by `rework_test.rs`. AC 4 PASS (`records_test.rs`). AC 5 PASS (`error_test.rs`, doctests). AC 6 PASS (`ports_test.rs`, `names_test.rs`). AC 7 PASS (`methods.rs` tests). AC 8 PASS (diff is inside the blast radius; T added only `tests/rework_test.rs` and edited two existing test files).

## Blocking issues

None.

## Advisory notes

1. `HarnessPort::health` still returns `bool` while `Pane.harness.health` can hold `Unhealthy(reason)` (F noted, provisional until #635).
2. The brief (`docs/handoffs/637-brief.md:147`) still spells the old `Watch<T>` alias; decisions.md says the new one supersedes it. O may want a revision note.
3. F's Process note says two interleaved executions touched the tree. T re-ran every number from scratch, and they agree except for the +7 tests.
4. `compile_fail,E0423` is not code-checked by stable rustdoc (S advisory 1); it proves only that the line does not compile, which F showed by making the field `pub`.
