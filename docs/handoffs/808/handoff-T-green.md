# Handoff-T-green: Phase 7 (verify) - #808 a Herdr snapshot pane carries its label

**Date:** 2026-10-11
**Branch:** `issue-808-implementation` (F at `db59fc8`, T-red at `bc96f89`)
**Issue:** #808
**Handoff-F reviewed:** `docs/handoffs/808/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/808/handoff-T-red.md`

Phase numbering is the Workflow script's (verify is its Phase 7); the role doc calls it Phase 6.

## GREEN confirmation

F changed no test file: `git diff --stat bc96f89 db59fc8 -- 'crates/*/tests/**'` is empty. F's "Tests that look wrong"
is "None", and I found none, so no test was repaired in this phase.

The full suite, `test.unit.command`:

```text
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
  exit 0; 148 test targets, every one "test result: ok"; 2068 passed, 0 failed, 25 ignored (the opt-in suites)
```

The targets this story's tests sit in:

| Target | Result |
|---|---|
| `holler-pane --test records_test` | 15 passed |
| `holler-pane-testkit --test fake_herdr_test` | 24 passed |
| `holler-pane-testkit --test herdr_conformance_test` | 21 passed |
| `holler-adapter-herdr --test protocol_test` | 37 passed |
| `holler-adapter-herdr --test adapter_test` | 22 passed |
| `holler-adapter-herdr --test adapter_conformance_test` | 4 passed |
| `holler-cli --test pane_verbs` | 252 passed |
| `holler-cli --test profile_verbs` | 44 passed |

**The first full run (without `--no-fail-fast`) failed one test outside this story:** `holler-cli --test remote_admin_test`
`remote_admin_traffic_never_moves_the_bodys_last_seen` (`last_seen` 1791720219 became 1791720220). The diff touches
nothing in `holler-hub`, `holler-cli/src`, `holler-body` or `holler-proto`. The test passed in three isolated reruns and
in the full `--no-fail-fast` rerun above. It is a pre-existing timing flake (see Advisory notes), not a #808 regression.

**Spot-checks: the tests fail when the behaviour is removed.** Each mutant was applied to `src/`, run, and reverted with
`git checkout`. `src/` is clean afterwards.

| Mutant | Caught by |
|---|---|
| `Plan::record` writes `live.herdr` (keeps the label) | `pane_verbs` 251/1 (`relaunch_in_place_of_a_labelled_pane_records_no_label`) |
| O2 also requires whole-value equality (`same_place(..) && pane == herdr`) | `pane_verbs` 251/1 (`launch_passes_o2_when_only_the_panes_label_changed`) |
| adapter `label_of` keeps `""` | `adapter_conformance_test` 1/3 (all three adapter runs fail the label case) |
| adapter `snapshot()` drops the join (every label `None`) | `adapter_conformance_test` 1/3 |

T-red's mutants (stub stages 1-3, the serde, O2 and reconcile guards) cover the rest; I did not repeat them.

## Tier 1 results

| Check | Command | Expected | Actual | |
|---|---|---|---|---|
| Build + lint | `cargo clippy --workspace --all-targets -- -D warnings` | clean | `Finished`, no warning | PASS |
| Repo lint | `bash scripts/lint.sh` | exit 0 | exit 0 (size warnings only, none at 900) | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Workspace tests | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0 on the `--no-fail-fast` rerun; one unrelated flake on the first pass | PASS |
| Docs CLI | `docs_cli_test` (in the workspace run) | ok | 3 passed | PASS |
| Canary | `wire_selftest` (in the workspace run) | ok | 3 passed | PASS |
| Unused deps | `cargo machete` | none | "didn't find any unused dependencies" | PASS |
| Format | `rustfmt --edition 2021 --check` on every `.rs` the branch touches | clean | exit 0 | PASS |

**Cross-check of F's reported numbers.** They match: 148 targets, 2068 passed, 0 failed, 25 ignored; `pane_verbs` 252,
`profile_verbs` 44, fake 24, conformance 21, adapter 22, protocol 37, adapter conformance 4, records 15; clippy, lint
and the changelog check are clean. The only difference is the flake on my first pass, which F did not hit.

## Tier 2 results

| Check | Method | |
|---|---|---|
| Coverage per criterion | Every acceptance bullet maps to a test (table below) | PASS |
| Test quality | Each test names one behaviour and sits at its cheapest tier. The four conformance mutants each fail only the label case (`a_label_break_fails_no_other_case`). The three guards are labelled as guards. No test is redundant: the adapter-level tests pin the join and the reply labels that the conformance case does not reach | PASS |
| Proportionality | About 680 test lines for a contract field that touches four crates and both fakes. That is proportionate to the surface | PASS |
| Type safety | No `unsafe`, no `#[allow]` added (`git diff 92fbc6e HEAD` has no `+#[allow`). `LabelHook` is a type alias for E12's exact type | PASS |
| Error paths | `rename` of an unknown pane is `pane-not-found` (test 5). A non-string or `null` label parses as `None` and no reply fails on it (tests 12, 13). A pane made between the two reads is listed with `None` and the call succeeds (test 18) | PASS |
| Data integrity | The pre-#808 record bytes are pinned by the hard-coded literal (`record_bytes.rs`). An unlabelled record has no `label` key, and an older `panes.json` reads (`#[serde(default)]` under `deny_unknown_fields`) | PASS |
| API contract | The diff matches the shapes T-red's table pinned. `PaneRef.label`, the parser tuples, `label_of`, `FakeHerdr::rename` and `run_herdr_conformance(fresh, label)` all compile against T's call sites unchanged | PASS |
| Protocol / goldens | No change in `holler-proto`, `docs/protocol/v2.md` or the hub (`git diff --stat 92fbc6e HEAD` on those paths is empty). No new Herdr method (`METHOD_COUNT` 9) | PASS (N/A) |
| Security | A pane label is Herdr-sent text and is never logged or put in a message by this diff. No secret is involved | PASS |
| Migration safety | Additive optional field with `skip_serializing_if`, byte-identical when absent; a downgrade reads an unlabelled record unchanged | PASS |
| File sizes | Largest touched: `adapter_test.rs` 810, `tx_launch.rs` 798, `protocol_test.rs` 750 (fail is 900) | PASS |
| Browser / Playwright | None in this repo | N/A |

**Evidence appendix.** I checked F's five excerpts verbatim against the source at the lines given, and they match. I
appended two facts under "T (Phase 7, verify)" in `docs/handoffs/808/evidence.md`, because the doctor guard
(`labels_change_nothing_doctor_reports`) relies on unchanged reconcile code:

- `reconcile.rs:328-335` and `:351-358` key live and recorded Herdr panes by `(session, pane_id)` only.
- `findings.rs:270-276`'s `with_herdr_pane` carries only the pane id and cell, never the label.

## Acceptance criteria status

| Criterion | | Backed by |
|---|---|---|
| A snapshot entry reports `Some("x")` for a pane labelled `x`, and `None` for no label and for `""` (adapter against the wire fake, and the fake) | PASS | `adapter_test` `snapshot_reports_each_placed_panes_own_label`, `ensure_pane_of_an_occupied_cell_answers_its_panes_label`, `ensure_pane_of_a_new_pane_answers_the_label_its_reply_carries`, `a_pane_made_between_the_two_reads_is_listed_with_no_label`; `fake_herdr_test` `a_labelled_pane_is_reported_with_its_label`; `protocol_test` label parsers and `label_of_folds_no_label_and_the_empty_string_to_none` |
| A pre-change `panes.json` deserializes unchanged, and a `launch` record serializes to the same bytes | PASS | `record_bytes.rs` `a_launch_writes_the_record_bytes_it_wrote_before_808`, `the_pre_808_record_and_a_labelled_one_both_read_and_round_trip`; `records_test` `herdr_label_round_trips_set_and_absent`; `relaunch_in_place_of_a_labelled_pane_records_no_label` |
| Doctor / reconcile output is identical with and without labels | PASS | `doctor.rs` `labels_change_nothing_doctor_reports` (read-only and `--fix`, `Report`, records, JSON and text) |
| The conformance case passes for `FakeHerdr` and the wire fake | PASS | `herdr_conformance_test` (fake, split-only and the four mutants); `adapter_conformance_test` `run_in_process` and `run_over_a_socket` (three adapter runs over `WireHerdr`) |
| ADR-0021 edits; `CHANGELOG.md` `[Unreleased]` entry linking #808 | PASS | Read in the diff: clarification 5's four places plus section 1's row (A warn 2, which the PR body must name). `changelog-check: ok`; the entry links #808 |

O2's place check (E11): `launch_passes_o2_when_only_the_panes_label_changed`, `launch_fails_o2_when_its_pane_is_listed_elsewhere`
and `launch_fails_when_its_herdr_pane_vanishes` all PASS.

## Blocking issues

None.

## Advisory notes

- **Unrelated flake, not filed:** `crates/holler-cli/tests/remote_admin_test.rs:365`
  `remote_admin_traffic_never_moves_the_bodys_last_seen`. It reads `last_seen`, sleeps a fixed 2 s, makes admin calls and
  asserts `last_seen` is unchanged. On one run it moved by 1 s. The likely cause is the body's own traffic landing in the
  window, which is legitimate and not admin traffic. It is the fixed-sleep pattern the repo warns about. No issue
  matching the test name exists (`gh issue list --search` was empty). It is worth a follow-up issue, outside #808.
- `protocol.rs` is now 636 lines (the warning is 600 and the fail is 900), as F noted.
- The merge-time items from F stand: renumber ADR "Decisions taken" item 10 if #802 lands first. The PR body must name
  the section-1 ADR row edit.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
