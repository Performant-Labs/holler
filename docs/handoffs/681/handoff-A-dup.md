# Handoff-A-dup: Phase 7 - #681 the JSON-envelope checker in `holler-pane-testkit`  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-681-implementation
**Diff base:** e410e9d (origin/main, the merge base)   **Diff head:** b2c9f21
**Reuse map:** docs/handoffs/681-brief.md §Files, "Reuse map (extend, do not duplicate)"
**Verdict:** PASS

## Summary

PASS. F extended the object the map named. It filled the `envelope.rs` stub that slice a created, and added no new
module, no `conformance/envelope.rs` and no re-export. Each seam the map named is the one the code calls:
`is_valid_code` for code validation, `class_of(..).exit_code()` for the code-to-exit mapping, and `serde_json`'s stream
deserializer for framing. The testkit has no code list, no exit table per code and no JSON scanner. Inside the module no
rule is checked twice: `check_ndjson` runs `check_envelope` on each line, and one `exact_members` serves both the
envelope and its `error` object (rules 3, 4 and 10). No drift came in during GREEN, because T's verify-phase repair
touched the test file only. The two `warn` rows below carry forward Phase 3 findings that this diff correctly leaves
alone. Neither is new.

## Findings

No duplication; extension is clean.

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/verb_harness/mod.rs:82`, `crates/holler-cli/tests/pane_verbs/process/main.rs:79`, `crates/holler-cli/tests/pane_verbs/process/usage.rs:17` | Carried forward from Phase 3 W-5, not introduced here. Now that `check_envelope` and `check_ndjson` exist, the three minimal envelope parsers `one_envelope`, `Out::envelope` and `usage_envelope` are partial copies of a helper that ships. Their own doc (`verb_harness/mod.rs:9-12`) says the #638 helper "is what later verb stories use for full envelope validation". This diff does not touch them, which is correct: they are outside the blast radius, and `holler-cli` is read only here. | The first story to touch each one (#660 or the first wave-3 verb story) makes it call `holler_pane_testkit::envelope` or removes it. At Phase 7 of those stories, a new or extended envelope parser in `holler-cli` or `holler-hub` tests is a `block`. |
| 2 | warn | `crates/holler-pane-testkit/src/lib.rs:28-29` | Carried forward from Phase 3 W-6. "The modules of slices b to e are empty stubs" is no longer true of `envelope`. As Phase 3 advised, the diff keeps AC7's edit to one paragraph (lines 5-9). | #684, the last slice, rewrites the sentence once every stub is filled. The A Phase 3 entry in `decisions.md` and handoff-F's "Known issues" already record this. |

Checked and found clean (evidence for the PASS):

- **Placement and scope.** `git diff --name-only e410e9d...b2c9f21` lists only the brief's blast radius: `envelope.rs`,
  `tests/envelope_test.rs` (new), the testkit `Cargo.toml`, one `Cargo.lock` line, the `lib.rs` paragraph at line 5 (doc
  only), `CHANGELOG.md` and `docs/handoffs/681*`. Nothing in `holler-pane`, `holler-cli`, `holler-hub`, `holler-proto`
  or `conformance/` changes, and no ADR or golden file changes. The public surface is exactly the brief's five items.
  The three consts and the eight helpers are all private.
- **Code validation and classification.** `kebab_code` calls `is_valid_code` (`envelope.rs:355`), and `check_failure`
  calls `class_of(&code).exit_code()` (`:345`). A grep of the workspace finds no other code validator or exit table in
  the diff. `0..=3` in `check_exit_code` (`:262`) is the ADR-0021 section 9 contract ("0 ok, 1 runtime failure, 2
  usage, 3 refusal"), not a code table. `holler-pane` has no exit-code set and no `ErrorClass::ALL` that the range could
  come from, and adding one would edit `holler-pane`, which is outside the blast radius.
- **Framing.** `frame` and `starts_with_value` use `Deserializer::from_str(..).into_iter::<Value>()` with
  `byte_offset()`. `match_indices('{')` only finds the places where a value might start, and `serde_json` parses each
  one. No other code in the workspace uses a stream deserializer. F declined to hand-write a scanner to detect
  duplicate keys (handoff-F, "Known issues"), which is the choice the map asked for. Whether a duplicate key should be a
  fault is a spec question for S or a follow-up issue, not drift.
- **Within the module.** `check_ndjson` reuses `check_envelope` for every line. `check_line_before_last` only renames
  `OkDisagreesWithExit` to `NotLastFailure`, so there is no second per-line validator. One generic `exact_members` covers
  both the envelope (`ENVELOPE_KEYS`) and `error` (`ERROR_KEYS`). Its rule of naming the smallest extra key
  (`keys().min()`) settles Phase 3 W-1 with both `Map` backends.
- **Deliberate pins on the producer's values.** `SCHEMA_VERSION` (`envelope.rs:66`, compare
  `holler-cli/src/output.rs:34`) and `ENVELOPE_KEYS` (compare the fields of the CLI's `Envelope<T>`) repeat what the
  producer writes. That is required, because ADR-0021 section 5 lets the testkit depend only on `holler-pane` and
  `serde_json`. It is also correct for a contract checker: one that shared the producer's constant would pass a version
  bump silently. The module doc says a new envelope field goes into the key list in the same change (Phase 3 W-2).
  `one_line_message` checks that a message is one line, while the CLI's `one_line` rewrites a message into one line
  before writing it. They sit on opposite sides of the contract and do not duplicate each other.
- **Patterns.** `Display` quotes untrusted payloads with `{:?}`, as `InvalidCode` does
  (`holler-cli/src/output.rs:94-98`) and as `excerpt` does (`holler-pane/src/error.rs:688-695`). `excerpt` also cuts
  text to 64 characters, but it is `pub(crate)`. For test-only code the uncapped `{:?}` is consistent enough, so this is
  not a finding. The test file follows the conformance pattern: one table of named rows, with an assertion that names
  the row. It does not use `run_cases`, which the brief ruled out. It adds no `#[allow]`. The CHANGELOG entry follows
  the form of slice a's "Pane control, the test kit's ..." entry.
- **Dependency direction.** `cargo tree --offline -p holler-pane-testkit -e normal --depth 1` lists only `holler-pane`
  and `serde_json v1.0.151`, and there are 0 `holler-(cli|hub)` lines. `serde_json` has no feature list and is already
  in the graph through `holler-pane`. `holler-cli` (`Cargo.toml:435`) and `holler-hub` (`Cargo.toml:58`) still take the
  testkit as a dev-dependency only.
- **Size and repository hygiene.** `envelope.rs` is 457 lines and `tests/envelope_test.rs` is 696 lines. `lint.sh` warns
  at 600 and fails at 900, and both files are under the overlay's flag at about 800. Slices c to e do not add to this
  test file. The added lines name no personal infrastructure and contain no absolute home path.

## Notes for F

None. The verdict is PASS, so F has nothing to fold.

## Patterns referenced

- `crates/holler-pane/src/error.rs:140-303` (`ALL_CODES`, `is_valid_code`, `ErrorClass::exit_code`, `class_of`) and `:363-390, 686-695` (`RefusalCodeError`, `excerpt`)
- `crates/holler-cli/src/output.rs:34, 84-98, 219-350` (`SCHEMA_VERSION`, `InvalidCode`, `emit_stream`, `emit_json`, `one_line`)
- `crates/holler-pane-testkit/src/{lib.rs, conformance/mod.rs, fault.rs}` and `tests/pane_store_conformance_test.rs`
- `crates/holler-cli/tests/verb_harness/mod.rs:1-12, 82-96`, `tests/pane_verbs/process/{main.rs:79-91, usage.rs:17-30}`
- `docs/adr/ADR-0021.md` section 5 and section 9 (the envelope, the exit codes and the `class_of` table)
