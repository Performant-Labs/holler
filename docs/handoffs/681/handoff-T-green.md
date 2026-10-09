# Handoff-T-green: #681 the JSON-envelope checker in `holler-pane-testkit` (slice b of #638)

**Date:** 2026-10-09
**Branch:** issue-681-implementation
**Issue:** #681
**Handoff-F reviewed:** docs/handoffs/681/handoff-F.md
**Handoff-T-red:** docs/handoffs/681/handoff-T-red.md

## GREEN confirmation

The authored suite passes against F's implementation with both `serde_json::Map` backends.

```
$ cargo test -p holler-pane-testkit                                         (Map = BTreeMap)
unittests src/lib.rs 4 passed; tests/envelope_test.rs 10 passed; fake_pane_store_test 22 passed;
pane_store_conformance_test 10 passed; doctest 1 passed
$ cargo test -p holler-pane-testkit --features serde_json/preserve_order    (Map = IndexMap, as under --workspace)
same counts, all ok
$ cargo test --workspace
106 test binaries: 1124 passed; 0 failed; 5 ignored
```

These match F's reported numbers exactly (F: 106 binaries, 1124 passed, 5 ignored).

**Mutation spot-checks** (a temporary edit to `src/envelope.rs`, reverted with `git checkout` after each run; the
tree holds no production change):
- drop the `class_of(..).exit_code()` comparison: 3 integration tests fail.
- skip the `data` is `null` check on failure: `every_mutant_is_rejected_with_its_fault` fails.
- widen the exit range to `0..=4`: fails.
- drop the blank-message check: fails.
- accept any `schema_version`: fails.
- `keys().min()` -> `keys().next_back()` (BTreeMap build): fails.
- `keys().min()` -> `keys().next()` under `preserve_order`: **survived** the Phase-5 suite (see below). Killed after the repair.

## Test repair made in this phase (T owns tests; no production code changed)

The surviving mutant above showed a weak spot in A's W-1 rows. Under `preserve_order`, `Map::remove` is a
`swap_remove`. For the old row `..."error":null,"zz":1,"aa":2}` the removals happen to leave `aa` first, so an
implementation that took `keys().next()` after removing the known keys passed. I added a helper
`extra_key_order_rows()` in `crates/holler-pane-testkit/tests/envelope_test.rs` with two rows, wired into
`mutant_rows()`:
- `extra-keys-among-known-keys-smallest-first`: `{"zz":1,"mm":3,"schema_version":1,...,"aa":2}` -> `UnknownKey("aa")`.
- `error-extra-keys-among-known-keys-smallest-first`: the same shape inside `error` -> `UnknownKey("error.aa")`.

The rows are in their own function because `key_rows` went over clippy's `too_many_lines` (100) with them. After the
repair: the `next()` mutant fails under `preserve_order`; F's code passes both backends. The test file is 690 lines
(lint warns at 600, fails at 900; it was 671).

## Tier 1 results

| Command | Result |
| --- | --- |
| `cargo test --workspace` | PASS (1124 passed, 0 failed, 5 ignored) |
| `cargo test -p holler-pane-testkit` (BTreeMap and `preserve_order`) | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (re-run after the test repair) |
| `rustfmt --check` on the changed files / `cargo fmt --check -p holler-pane-testkit` | PASS |
| `bash scripts/lint.sh` | PASS (exit 0; warns only for the 690-line test file, as before) |
| `bash scripts/changelog-check.sh` | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3) |
| `cargo machete` | PASS |

## Tier 2 results

- **Coverage per criterion:** PASS, see the next section.
- **Test quality:** PASS. The 10 integration tests are table-driven pure-function checks at the cheapest tier. The mutant
  tables each name the rule and the fault they expect. `the_mutant_tables_cover_every_fault` is a compile-time and table
  guard, not a behavior test, and F and T both state that. No timing, no sleeps, no I/O, so there is no flake risk to repeat.
- **Type safety / error handling:** PASS. No `unwrap`, `expect`, `panic!` or indexing in production code
  (`unwrap_or_default` is guarded by the presence check just above it). Clippy is clean at `-D warnings`.
- **Dependency direction:** PASS. `cargo tree -p holler-pane-testkit -e normal --depth 1` shows `holler-pane` and `serde_json` only.
- **Reuse:** PASS. `is_valid_code` and `class_of(..).exit_code()` are the only code and exit logic; the checker has no code table.
- **Secrets in logs/errors:** PASS. `Display` quotes untrusted text with `{:?}`, and the checker never reads secrets.
- **Protocol/golden files:** N/A (no wire change). **Docs CLI commands:** no change; `docs_cli_test` passes.
- **Allow attributes:** the new test rows add none.
- **Evidence appendix:** `docs/handoffs/681/evidence.md` (135 lines) exists and covers the unchanged-code facts the tests rely
  on (`is_valid_code`, `class_of`, `ErrorClass::exit_code`, the CLI's envelope, the stream-deserializer whitespace and
  `byte_offset` facts, the `Map` backend selected by `preserve_order`). I added one fact below.
- **No browser/Playwright surface** in this repo; none run.

## Acceptance criteria status

All pass; the tests are in `crates/holler-pane-testkit/tests/envelope_test.rs` unless stated.
- AC1 (valid envelopes accepted: success, null data, ADR lines, missing final newline, every closed code at its class exit): PASS.
- AC2 (every rule's mutant rejected with the expected first fault; NDJSON stream rules): PASS.
- AC3 (rule order: output that breaks two rules gets the same fault): PASS.
- AC4 (`Display` one non-empty line per variant, newline payloads included; framing unit tests in `src/envelope.rs`): PASS.
- AC5 (module doctest): PASS.
- AC6 (`serde_json` dependency, dependency direction): PASS.
- AC7 (`lib.rs` doc change only, no item change): PASS (F's re-wrap stays inside the one paragraph).
- AC8 (CHANGELOG entry linking #681): PASS (`changelog-check: ok`).

## Blocking issues

None.

## Advisory notes

- Duplicate keys in one object are not detected (F's known issue). `serde_json::Value` keeps the last value. A follow-up
  could decide whether to add a variant; out of scope here (17 variants are fixed by the brief).
- Rule 1(b) is quadratic in the worst case, only on already-faulty, small test outputs.
- `Map::remove` is a `swap_remove` under `preserve_order`. This is why the swap-order rows above were needed; any future
  change to `exact_members` should keep those rows.
