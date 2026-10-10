# Handoff-T-green: Phase 6 - #640 part 2 of 3: the socket transport, `HerdrAdapter` and a simulated Herdr

**Date:** 2026-10-09
**Branch:** issue-640-implementation (F's commit `d3b1eba`)
**Issue:** #640 (epic #633), part 2 of 3
**Handoff-F reviewed:** `docs/handoffs/640/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/640/handoff-T-red.md`

## What T changed at GREEN (test code only)

F changed no test file. F listed no test as wrong, and named one optional gap. T closed it and one more gap found in
review. No production code was touched.

1. **`config_is_validated_before_any_request` (AC 27) has a ninth case:** `HerdrConfig { timeout: Duration::MAX, .. }`
   must be `Usage` naming `timeout`, with no request recorded. This pins A finding 3 / F deviation 1, a timeout too long
   to add to the clock. The mutant `Some(Instant::now() + timeout)` in `deadline_after` panics at `adapter.rs:435`, and
   the test fails.
2. **New test `a_version_that_is_empty_or_holds_a_control_character_is_unavailable`** (Decision 8, which had no test).
   A `Tap` answers `ping` after connect with a supported pong (`protocol: 22`) whose version is `""`, `"0.7\n1"` or
   `"0.7\u{7}"`, and `version()` must be `Unavailable`. Spot-check: with the form check disabled
   (`if false && (...)`), each of the three cases on its own fails with `Ok(<that string>)`. So `parse_pong` accepts all
   three, and only the adapter's check refuses them.
3. **`evidence.md`:** T appended one entry, the compact `to_line` serialization and `Direction::as_str`. AC 16's
   substring rewrite depends on both. Excerpts were copied from source (`protocol.rs:148-151`, `:175-184`;
   `layout.rs:38-43`).

`adapter_test.rs` is now 645 lines (under 900), with 18 tests.

## GREEN confirmation

`cargo test -p holler-adapter-herdr --no-fail-fast`, final run after T's edits:

```
adapter_conformance_test  ok. 4 passed; 0 failed
adapter_test              ok. 18 passed; 0 failed
layout_test               ok. 10 passed; 0 failed
plan_splits_test          ok. 19 passed; 0 failed
protocol_test             ok. 33 passed; 0 failed
transport_test            ok. 13 passed; 0 failed; finished in 1.00s
```

That is 97/97. All 34 RED tests are GREEN, plus the new version test. Repeats looking for flakes: before T's new test,
the three new targets ran 15 times in a row (45 runs) while the workspace suite ran in parallel, and all were green.
After the edits, `adapter_test` ran 5 more times, all green.

### AC 14: the three mutants (applied locally, restored with `git checkout`, never committed)

| Mutant | AC 13 | AC 16 | AC 20 | Brief requirement |
|---|---|---|---|---|
| (a) `grid_of` with `Down`/`Right` swapped (`layout.rs:147-148`) | **FAILS**: `Err(Unavailable { "Herdr put the new pane \"w1:p2\" at r1c2, not at r2c1; ..." })` at `adapter_test.rs:131` (bullet 2) | **FAILS**: `expected Unavailable, got Ok(HerdrPane { pane_id: "w1:p2", grid: r2c1 })` | passes | AC 13 + AC 16: **met** |
| (b) `number` returns the 0-based index (`layout.rs:210`) | **FAILS**: `Err(Unavailable { "... \"w1:p1\" at r0c0, not at r1c1 ..." })` at `adapter_test.rs:127` (bullet 1) | FAILS (its `ensure r1c1` setup gets the same `r0c0` refusal) | **FAILS**: the same `r0c0` read-back at `adapter_test.rs:295` | AC 13 + AC 20: **met** |
| (c) the adapter sends `Right` for every split (`adapter.rs:208`) | **FAILS**: `Err(Unavailable { "... \"w1:p2\" at r1c2, not at r2c1 ..." })` at `adapter_test.rs:131` | passes (as derived) | passes | AC 13: **met** |

Each observed value matches the brief's oracle table exactly.

### Spot-checks: do the tests pin behaviour?

| Behaviour removed | Test | Result |
|---|---|---|
| The read-back is ignored (`let _ = self.confirm(..)`) | AC 16 | FAILS: `expected Unavailable, got Ok(.. w1:p2 ..)` |
| No `changed_under` rewrite | AC 17 | FAILS: `expected Unavailable, got Err(PaneNotFound { what: "w1:p1" })` |
| No reply cap (`READ_LIMIT = usize::MAX`) | AC 8 | FAILS: `expected Unavailable, got Err(Timeout { op: "herdr.ping" })` |
| Unchecked `Instant + timeout` | AC 27 (new case) | FAILS: panic at `adapter.rs:435` |
| No version form check | new Decision 8 test | FAILS: `"": Ok("")` (and each other case on its own) |

Every `src/` file was restored after each mutant. `git status` shows only T's test and evidence edits.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 (only the >600-line warnings, none of them at 900) | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean, re-checked after T's edits | PASS |
| Fmt (AC 40) | `cargo fmt --check -p holler-adapter-herdr` | exit 0 | exit 0 | PASS |
| Machete | `cargo machete` | no unused deps | none | PASS |
| Crate tests | `cargo test -p holler-adapter-herdr` | all pass | 97/97 | PASS |
| Workspace tests | `cargo test --workspace --no-fail-fast` | all pass | every target passes except 4 in `holler-cli --test logging_test` (see below) | PASS (environmental) |
| `logging_test` isolated | `HOLLER_STATE_DIR=$(mktemp -d) cargo test -p holler-cli --test logging_test` | all pass | 11/11 | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3/3 | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3/3 | PASS |
| Deps (AC 41) | `cargo tree -p holler-adapter-herdr -e normal --depth 1`; `grep holler-hub Cargo.toml` | `holler-pane`, `serde_json`; nothing | as expected | PASS |

**About `logging_test`:** the 4 failures are `banner_names_resolved_level_and_format`, `debug_flag_beats_env`,
`env_none_loses_to_flag_noisy` and `log_output_stays_off_stdout`. They are the same 4 F reported, for the same reason:
these tests expect `holler roster` to fail because no hub is reachable, and this machine runs a live hub. With an
isolated `HOLLER_STATE_DIR` all 11 pass. `holler-cli` does not depend on `holler-adapter-herdr`. Not a blocker.

**About F's report:** T's results match it. F reported 96/96 for the crate, and T got the same before adding test 2.
F's clippy, lint, changelog and machete results and the same 4 `logging_test` failures all reproduce.
`cargo fmt --all --check` shows thousands of diffs in other crates (`holler-body`, `holler-proto`, ...), none in
`holler-adapter-herdr`. CI does not run fmt, and AC 40 asks for `-p holler-adapter-herdr` only, so this is
pre-existing drift (likely a rustfmt version difference) and outside this diff.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per AC | Each of AC 1-34 has its named test (see `handoff-T-red.md`). AC 14 is the mutant table above. AC 35-43 are greps and gates, re-run here | PASS |
| Test quality | Each test names one behaviour. The spot-checks above show the key tests fail when that behaviour is removed. All tests are integration tier, at the public seam (`Transport`, `connect_with`). The two additions extend existing tests where that fits (one AC 27 case), and the one new test pins a stated decision that had no test. No redundant test was found to prune | PASS |
| Type safety | No `as` casts, `unwrap` or indexing in `src/adapter.rs`/`src/transport.rs`. `u32::try_from(..).unwrap_or(u32::MAX)` for read lines, and `checked_add` for every deadline | PASS |
| Error paths | `timeout` (silent, dripping, passed deadline), `unavailable` (missing path, too-long path, EOF, oversize, non-UTF-8, garbled on all 7 methods, moved pane, closed split target, bad version string), `usage` (9 configs) and the version gate are each tested | PASS |
| Concurrency / timing | AC 12a holds as an invariant ("exactly one answer at the deadline, no worker panic"), and AC 28 has one deadline per call with no tolerance. No fixed sleeps in `src/` (AC 39 grep is empty). Green over 45+ repeats under load | PASS |
| API contract | Request params are pinned by AC 13, 20, 24 and 25. AC 34 confirms every recorded method is in `ALLOWED_METHODS` | PASS |
| Security / secrets | AC 12: no `Display`/`Debug` of a transport error contains the typed text. No message carries request or reply bytes (`transport.rs` messages name only the method, the socket and an `ErrorKind`) | PASS |
| Greps AC 35-39 | All five re-run, and each printed nothing. There is no `#[allow` anywhere in the crate | PASS |
| AC 42 | All files under 900 lines. `too_many_lines`/`cognitive_complexity` are workspace denies, and clippy is clean | PASS |
| AC 43 | The `CHANGELOG.md` entry sits right after the part-1 entry and links #633, #649 and #640 | PASS |
| Protocol goldens / `docs/protocol/v2.md` | No Holler wire change: the adapter talks to Herdr, not to the hub protocol | N/A |
| Migrations | none | N/A |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1-12, 12a | PASS | `transport_test.rs` (13 tests) |
| 13 | PASS | `r2c1_and_r1c2_land_in_their_cells_and_read_back` |
| 14 | PASS | the mutant table: (a) fails 13+16, (b) fails 13+20, (c) fails 13 |
| 15-30 | PASS | `adapter_test.rs`, one named test each. AC 27 now has 9 cases, and the Decision 8 version test is added |
| 31-34 | PASS | `adapter_conformance_test.rs` (4 tests) |
| 35-39 | PASS | greps print nothing |
| 40 | PASS | every listed gate (the workspace run is green except the 4 environmental `logging_test` cases, which pass when isolated) |
| 41 | PASS | `cargo tree`, manifest grep |
| 42 | PASS | `wc -l`, clippy denies |
| 43 | PASS | `CHANGELOG.md` diff |

## Blocking issues

None.

## Advisory notes

- **Timeout `op` names the wire method** (`herdr.layout.export`, ...), while `FakeHerdr` names the port method
  (`herdr.ensure_pane`). This is A finding 1, and F kept Decision 3. No test pins the port-level `op`. It is listed for
  part 3's ADR rows, not a GREEN issue.
- **A worker stuck in `connect`** on a wedged server's full backlog outlives its call until the server accepts or dies.
  The module docs say so, and the brief accepted it. The caller still returns by its deadline (AC 5, 6, 12a).
- **Herdr-sent text is quoted with `{:?}` and not cut to a length** (F known issue 3, under the part-1 freeze). Messages
  stay on one line, which AC 26 and 27 assert.
- **`cargo fmt --all --check` drift in other crates** is pre-existing and outside this diff (see Tier 1).

T-green complete, no blocking issues. No UI surface: U is N/A, ready for S.
