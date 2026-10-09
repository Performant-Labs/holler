# Handoff-F: Phase 6 - #681 the JSON-envelope checker in `holler-pane-testkit` (slice b of #638)

**Date:** 2026-10-09
**Branch:** issue-681-implementation
**Issue:** #681

## What was done

- `crates/holler-pane-testkit/src/envelope.rs`: filled the stub.
  - **Module doc:** the stub text is gone. The new doc covers the two functions, the 14 rules in check order, the NDJSON
    rules, and how the checker parses (`serde_json` alone) and classifies (`class_of`, with no table of its own). It
    also has the key-set sentence from A's W-2. T's doctest (AC5) is kept byte for byte.
  - **Public API:** exactly the brief's five items. `Envelope`, `EnvelopeError`, and `EnvelopeFault` with its 17
    variants, each variant with a `///` line saying what triggers it. `Display` (one line per variant) and
    `std::error::Error`, plus `check_envelope` and `check_ndjson`.
  - **Private helpers:** `check_exit_code` (rule 0), `frame` and `starts_with_value` (rule 1), `exact_members`
    (rules 3, 4 and 10), `check_failure` (rules 8 to 13), `kebab_code` (11), `one_line_message` (12) and
    `check_line_before_last` (NDJSON rule 3). Each has a caller.
  - T's `#[cfg(test)] mod tests` is kept byte for byte.
- `crates/holler-pane-testkit/src/lib.rs`: the doc paragraph that starts at line 5 now says the crate depends on
  `holler-pane` and `serde_json` (for the envelope checker). The rest of that paragraph is re-wrapped, with its words
  unchanged. No item and no other line changed (AC7).
- `CHANGELOG.md`: an `[Unreleased]` / Enhancements entry after the #676 bullet, linking #681 (AC8).
- Not changed by F: `crates/holler-pane-testkit/Cargo.toml` and `Cargo.lock`. T committed the `serde_json` line and
  its comment at RED, and they already satisfy AC6.

## Design decisions

1. **The smallest extra key, in every build (A's W-1).** `exact_members` first finds the first missing key, in table
   order. It then removes the expected keys and names `object.keys().min()` of what is left. That is the sorted-first
   extra key whether `Map` is a `BTreeMap` (`-p holler-pane-testkit`) or an `IndexMap` (a `--workspace` build turns on
   `serde_json/preserve_order`). The suite passes both ways (Tier 1 below). Rejected: `keys().find(..)`, which names
   the first key in map order and fails T's `two-extra-keys-smallest-first` rows under `preserve_order`.
2. **The members come back as an array in key order:** `let [schema_version, ok, data, error] = exact_members(..)?`.
   The later rules read each member by name, and the success path moves `data` instead of cloning it. Inside
   `exact_members`, `remove(key).unwrap_or_default()` runs right after the presence check in the same function, so
   the `null` default is never used, and a comment says so. Rejected: `object.get(key).ok_or(MissingKey(key))?` at
   every use. It costs a `?` per read and a clone of `data`.
3. **One `(key, name)` table per object** (`ENVELOPE_KEYS`, `ERROR_KEYS`). `MissingKey` carries a `&'static str`, and
   the `error` keys are named `error.code` and `error.message`, so each name is stored next to its key. A path prefix
   (`""` or `"error."`) builds the `UnknownKey` text. Both objects share the one helper, so rules 3, 4 and 10 cannot
   drift apart.
4. **Framing uses `serde_json`'s `StreamDeserializer` only,** with no hand-written scanner (the Reuse map).
   - (a) A non-whitespace first byte, a first item that is `Ok`, and `byte_offset() < body.len()` give `TextAfter`.
     The first byte is checked because the deserializer skips leading whitespace (evidence.md).
   - (b) Text before is found with `match_indices('{')`, which yields char boundaries, and `str::get(at..)`, which
     cannot panic, so nothing is indexed.
   - (c) Anything else is `NotJson`.
   - `is_ascii_whitespace` differs from JSON whitespace only in form feed, which cannot start a JSON value, so both
     give the same fault for every input.
5. **NDJSON.** An empty body maps to no lines, and `split_last()` on no lines is `EmptyStream`, so there is no special
   case and no indexing. `collect::<Result<Vec<_>, _>>()` stops at the first faulty line. A line before the last goes
   through `check_envelope(line, 0)` with `OkDisagreesWithExit` renamed `NotLastFailure`. The last line is checked
   at the process's exit code.
6. **`Display` quotes every string payload with `{:?}`** (A's W-3): `UnknownKey`, `CodeNotKebab` and also
   `MissingKey`. Every value of every variant then prints on one line, as `InvalidCode` (`holler-cli/src/output.rs`)
   and `RefusalCodeError` (`holler-pane`) do. `holler_pane::error::excerpt` would also cut long text, but it is
   `pub(crate)`. Widening it would edit `holler-pane`, which is outside the blast radius.
7. **Not `#[non_exhaustive]`** (A's note), so the test crate's exhaustive `match` in `fault_name` keeps compiling.
8. **The exit set `0..=3`** is the ADR-0021 section 9 contract, written once in `check_exit_code`. It is not a code
   table: every code's class still comes from `class_of` (A confirmed this at Phase 3).

## Reuse / extend-vs-new

I extended the stub that slice a created for this checker, as the brief's Files section says ("extend the stub"). The
change reuses:
- `holler_pane::error::is_valid_code`, the only code validator, for rule 11.
- `holler_pane::error::class_of` with `ErrorClass::exit_code`, the only code-to-exit table, for rule 13.
- `serde_json::StreamDeserializer` for framing.

It adds no code list, no exit table per code, no JSON scanner, no `conformance/envelope.rs`, and no use of
`conformance::run_cases`, all per the brief's Reuse map. The three placeholder envelope parsers in the `holler-cli`
tests (A's W-5) are untouched. They are outside this blast radius and belong to #660 and the wave-3 verb stories.

## Architecture notes for A

- **Layers:** `holler-pane-testkit` only, which is a dev-dependency of `holler-cli` and `holler-hub`.
- **Public interface:** five new public items in the existing `envelope` module, matching the brief's "Public API"
  signatures. Nothing else in the crate's public surface changed: no new module, no re-export.
- **Dependencies:** the only new edge is `serde_json`, from T's manifest line. It is already in the graph through
  `holler-pane`. `cargo tree -p holler-pane-testkit -e normal --depth 1` lists only `holler-pane` and
  `serde_json`, and the `holler-(cli|hub)` count is `0`. The dependency direction is unchanged (ADR-0021 section 5).
- **Pattern followed:** an error enum with `Display` plus `std::error::Error`, quoting untrusted text with `{:?}`
  (`InvalidCode`, `RefusalCodeError`). I followed the house rule that library code never panics: no `unwrap`, `expect`,
  `panic!`, `unreachable!` or indexing.
- **`archChanged: true`** is a self-report under the role's definition, because the crate's public interface gained the
  checker's API. The API is the one the brief planned and A passed at Phase 3, and the implementation does not deviate
  from it. A later BLOCK most likely needs only an implementation fix.

## Deviations from spec / wireframe

- **`lib.rs` re-wrap.** AC7 asks for line 5. The new sentence is longer, so the rest of that one doc paragraph is
  re-wrapped (old lines 5 to 8 become 5 to 9). The words after the changed sentence are the same. A's W-6 note allows
  "any re-wrap kept inside that one doc paragraph". The minimal alternative leaves a seven-word line followed by a line
  that starts "(ADR-0021 section 5)".
- **`MissingKey` is quoted in `Display` too.** A's W-3 named `UnknownKey` and `CodeNotKebab`. This addition keeps the
  one-line property for every value of every variant.
- **The `Display` wording is mine.** The brief only requires one non-empty line per variant that names the rule.
- No other deviations. No wireframe applies, since there is no UI surface.

## Tier 1 self-check (incl. tests now GREEN)

All of these ran in the worktree on the final tree. Output is trimmed to the result lines.

```
$ cargo test -p holler-pane-testkit          # serde_json without preserve_order: Map is a BTreeMap
     Running unittests src/lib.rs
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out     <- AC4, envelope.rs mod tests
     Running tests/envelope_test.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out    <- AC1 to AC3
     Running tests/fake_pane_store_test.rs
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out    <- slice a, unchanged
     Running tests/pane_store_conformance_test.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out    <- slice a, unchanged
   Doc-tests holler_pane_testkit
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out     <- AC5
exit=0

$ cargo test -p holler-pane-testkit --features serde_json/preserve_order    # Map is an IndexMap, as under --workspace
test result: ok. 4 passed; 0 failed ...
test result: ok. 10 passed; 0 failed ...
test result: ok. 22 passed; 0 failed ...
test result: ok. 10 passed; 0 failed ...
test result: ok. 1 passed; 0 failed ...
exit=0

$ cargo test --workspace                     # serde_json/preserve_order is on in this build
106 test binaries: 1124 passed; 0 failed; 5 ignored. Among them, holler-pane-testkit:
  unittests src/lib.rs     4 passed; tests/envelope_test.rs 10 passed;
  doctest "src/envelope.rs - envelope (line 49)" ... ok
exit=0

$ cargo build --workspace                                                          exit=0
$ cargo clippy --workspace --all-targets -- -D warnings                            exit=0
$ rustfmt --check --edition 2021 crates/holler-pane-testkit/src/envelope.rs \
    crates/holler-pane-testkit/tests/envelope_test.rs crates/holler-pane-testkit/src/lib.rs   exit=0
$ cargo machete            "cargo-machete didn't find any unused dependencies"     exit=0
$ bash scripts/lint.sh     only warning on this slice's files: tests/envelope_test.rs is 671 lines   exit=0
$ bash scripts/changelog-check.sh      changelog-check: ok                         exit=0
$ bash scripts/test-hooks.sh           11 cases ok                                 exit=0
$ RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane-testkit --no-deps            exit=0
$ cargo tree -p holler-pane-testkit -e normal --depth 1
holler-pane-testkit v0.4.0
├── holler-pane v0.4.0
└── serde_json v1.0.151
$ cargo tree -p holler-pane-testkit -e normal | grep -cE "holler-(cli|hub)"
0
$ wc -l crates/holler-pane-testkit/src/envelope.rs
457
```

T's text is unchanged. `cmp` of the `#[cfg(test)] mod tests` block and of the doctest against the RED commit (HEAD)
shows both byte-identical, and `git diff HEAD -- crates/holler-pane-testkit/tests/` is empty.

**A probe against the real CLI.** This was a scratch crate outside the repo, depending on the testkit by path. It is
not a test, and nothing of it was kept.
- `holler pane list --format=json` exits 1 and writes
  `{"schema_version":1,"ok":false,"data":null,"error":{"code":"not-implemented","message":"not implemented (story #643)"}}`.
  `check_envelope` accepts it at exit 1 and gives `Err(ClassDisagreesWithExit)` at exit 3. `check_ndjson` accepts the
  same stdout at exit 1 as one envelope.
- `holler pane list --format=json --bogus` exits 2 with a `usage` envelope. It is accepted at exit 2 and gives
  `ClassDisagreesWithExit` at exit 1.

## Evidence appendix

`docs/handoffs/681/evidence.md`. It covers `is_valid_code`, `ErrorClass::exit_code`, `class_of`, the CLI's `Envelope`
and `emit_stream`, the `serde_json` stream deserializer's whitespace skip and `byte_offset`, the `Map` backend
selected by `preserve_order`, `Value: Eq`, and the ADR-0021 section 9 contract lines.

## Tests that look wrong (for T)

None. Every authored test passes with both `Map` backends. As T noted, `the_mutant_tables_cover_every_fault` passes
against any implementation that compiles. It is a guard at compile time and over the tables, not a behaviour test, and
that is by design.

## Known issues

- **Duplicate keys are not detected.** `serde_json::Value` keeps the last value of a repeated key with either map
  backend. A probe confirmed that `{"schema_version":1,"ok":false,"ok":true,"data":null,"error":null}` at exit 0 is
  accepted as `ok: true`. The CLI serializes a struct, so it cannot write a duplicate key. Detecting one would need a
  hand-written scanner, which the Reuse map rules out, and a fault variant, but the 17 are fixed. Neither the issue nor
  the brief has this rule. A follow-up should decide whether it is worth a variant.
- **Rule 1(b) can be quadratic.** It tries a parse at each `{` until one succeeds, so the worst case is many `{` that
  never close. That is only on output that is already faulty, in test-only code on small outputs.
- **Follow-ups A raised, not for this run:**
  - W-4: #660 owns the cross-check against the CLI's own output. The CLI's `one_line` keeps a lone `\r`, and turns an
    all-whitespace message into `""`. The checker rejects both with `MessageNotOneLine`, as it should.
  - W-5: the placeholder parsers in the CLI tests.
  - W-6: `lib.rs:27-28` still says the modules of slices b to e are empty stubs. That is for #684.
- `tests/envelope_test.rs` (T's file) is 671 lines, a `lint.sh` warning (600) and not a failure (900).

## Files changed

- `crates/holler-pane-testkit/src/envelope.rs`
- `crates/holler-pane-testkit/src/lib.rs`
- `CHANGELOG.md`

(Pipeline artifacts: `docs/handoffs/681/handoff-F.md`, `docs/handoffs/681/evidence.md` and the F entry in
`docs/handoffs/681/decisions.md`.)
