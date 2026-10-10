# Handoff-T-red: Phase 4 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (amendment 1)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on e3be3c1, A's PASS on amendment 1; production code unchanged since 06320ad)
**Brief / wireframe reviewed:** docs/handoffs/643-brief.md (amendment 1: Decision 3, AC 3, AC 19, Decision 15, Test plan
"RED for amendment 1"); docs/handoffs/643/handoff-A.md (amendment 1, W-12 to W-16, notes for T). Wireframe: N/A (no UI
surface).

Round 2 of this handoff is in git: `git show 3140f35:docs/handoffs/643/handoff-T-red.md`. This file replaces it.

## A precondition

Confirmed: A returned PASS on amendment 1 (handoff-A.md: 0 blocks, warns W-12 to W-16). A's note 1 for T was applied
as written: AC 3 extends the existing `observed()` helper in place (no second helper), and `sync_rig()` is still the one
AC 3 fixture, shared by the `list`, `get` and `watch` tests. W-12 has no RED. W-13's optional `at: -1` case was taken,
as its own test (below).

## Tests authored

All are in-process over the `Rig` (`run_verb_with`, test-kit fakes). This is the cheapest tier that runs the real verb
through clap, dispatch and `output.rs`.

| Test | File | Pins | Change |
|---|---|---|---|
| `observed(name, record, shown, driven, at)` (helper) | `tests/pane_verbs/list.rs:112-131` | AC 3's fixture: sets `session_of_record` and `at` as well as `shown`/`driven` | extended in place (was `observed(name, shown, driven)` with `at: 0`) |
| `sync_rig()` + `SYNC_WANT` | `list.rs:312-341` | AC 3's six-row table (c1 to c6), and the expected SYNC per pane, in text and JSON. One table, used by all three tests | rewritten |
| `list_flags_a_pane_whose_shown_differs_from_its_session_of_record` | `list.rs:343` | AC 3 for `list`: PANE, SHOWN, DRIVEN and SYNC cells per row, and JSON `shown`/`driven` (string or `null`) and `sync` | replaces `list_flags_a_pane_whose_shown_and_driven_differ` |
| `get_flags_a_mismatch` | `get.rs:158` | AC 3 for `get`: the six `sync:` lines and `data.sync` | re-authored over `SYNC_WANT` |
| `watch_flags_a_mismatch` | `watch.rs:284` | AC 3 for `watch`: `sync=` per text line, and `[data.name, data.pane.sync]` per NDJSON line | re-authored over `SYNC_WANT` |
| `list_help_documents_the_columns_and_the_json_shape` | `list.rs:562` | AC 19: also `session of record`, `home screen`, `#649` | three needles added |
| `watch_help_documents_the_stream` | `watch.rs:304` | AC 19 (W-4): also `pane get` | one needle added |
| `a_negative_observed_at_is_never_observed` | `get.rs:172` | A's W-13: a record with `at: -1` prints `observed-at: never`, `sync: -` and JSON `"unobserved"`, although SHOWN (`ses-b`) differs from the record (`ses-a`) | new. It is a guard, not RED (see below) |
| `a_stored_dash_prints_apart_from_the_empty_value` | `get.rs:375` | unchanged behaviour. Only its fixture call moves to the new `observed` signature (`record: Some("-")` replaces its separate assignment) | call-site update |

The W-13 case is a separate `get` test, not a seventh `sync_rig` row, so AC 3 keeps the six rows the brief pins. One
verb is enough for it: `observed-at` and `sync` sit side by side only in `get`, and `PaneRow::from` calls the same
`SessionSync::of`.

## RED confirmation

Command: `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::` on the unchanged production code (which
still compares SHOWN with DRIVEN).

```
test result: FAILED. 31 passed; 5 failed; 0 ignored; 0 measured; 91 filtered out
```

Each failure is an assertion about the behaviour amendment 1 adds. None is a compile, setup or harness failure:

| Test | Failing assertion (verbatim) | Matches the brief's "RED for amendment 1" |
|---|---|---|
| `list_flags_a_pane_whose_shown_differs_from_its_session_of_record` | `list.rs:357`: `left: ["demo-c1r1", "ses-b", "-", "-"]` / `right: ["demo-c1r1", "ses-b", "-", "MISMATCH"]` | c1: the code gives `-`, the rule gives MISMATCH |
| `get_flags_a_mismatch` | `get.rs:162`: `demo-c1r1: left: Some("-")` / `right: Some("MISMATCH")` | c1, same |
| `watch_flags_a_mismatch` | `watch.rs:290`: `left: [Some("-"), Some("-"), Some("-"), Some("-"), Some("-"), Some("MISMATCH")]` / `right: [Some("MISMATCH"), Some("ok"), Some("MISMATCH"), Some("-"), Some("-"), Some("ok")]` | the whole table at once: c1, c2, c3 and c6 differ, and c4 and c5 agree, exactly the four cases the brief lists |
| `list_help_documents_the_columns_and_the_json_shape` | `list.rs:582`: `` `pane list --help` names "session of record" `` | AC 19 |
| `watch_help_documents_the_stream` | `watch.rs:315`: `` `pane watch --help` names "pane get" `` | AC 19 (W-4) |

`a_negative_observed_at_is_never_observed` passes at RED. That is expected: the current rule answers `unobserved` for
any record with `driven: None`. It guards against F writing the guard as `at == 0`.

**The tests can pass, and they pin the rule.** T applied the brief's Decision 3 body to `SessionSync::of` (taking
`&Pane` and calling `holler_pane::reconcile::shown_differs`, at both call sites) with a scratch script. T ran the same
command and then restored both files with `git checkout`:

| Variant | Result |
|---|---|
| Guard `at <= 0` (Decision 3 with W-13) | 34 passed, 2 failed: only the two help tests, which are F's doc work. All three AC 3 tests and the W-13 test pass |
| Guard `at == 0` (Decision 3 to the letter) | 33 passed, 3 failed: the two help tests, plus `a_negative_observed_at_is_never_observed` at `get.rs:187` |

After the restore, `git status --short` lists only the three test files, so no production file changed.

The rest of the binary still holds (`cargo test -p holler-cli --test pane_verbs`: 122 passed, and the same 5 failed).

Static checks on the touched files:

| Check | Result |
|---|---|
| `rustfmt --check --edition 2021` on `tests/pane_verbs/{list,get,watch}.rs` | exit 0 |
| `cargo clippy -p holler-cli --all-targets -- -D warnings` | exit 0 |
| `bash scripts/lint.sh` | exit 0 |
| Test file sizes | list.rs 587, get.rs 389, watch.rs 320, all under 900 |
| Non-ASCII bytes, new `#[allow]` | none |
| Names | neutral only (`demo-*`, `ses-a`, `ses-b`, `/srv/demo`), and no secret-shaped value |

## Ready for F

RED is valid. F may implement against these tests:
- `SessionSync::of(pane: &Pane)`, with the guard `at <= 0` (W-13), returning `Unobserved` with no session of record,
  and otherwise calling `shown_differs`;
- the two call sites, and the docs and help of Decision 15 (AC 19's needles: `session of record`, `home screen` and
  `#649` in `pane list --help`, and `pane get` in `pane watch --help`);
- W-12 (`is_member` in `watch.rs`), which the existing `watch_profile_*` tests pin;
- AC 23's CHANGELOG sentences, and the `evidence.md` entries for W-12, W-13 and W-16.

The tests rely on nothing outside the diff that `evidence.md` lacks: `shown_differs` (`reconcile.rs:171-181`) and the
test kit's `sample_pane` (`at: 0`, no session of record, already cited for AC 1 and AC 2) are both quoted in the brief.
