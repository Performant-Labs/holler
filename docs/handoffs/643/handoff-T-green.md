# Handoff-T-green: Phase 7 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (round 2)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (on f922dd8, F's round-2 commit)
**Issue:** #643
**Handoff-F reviewed:** docs/handoffs/643/handoff-F.md (round 2)
**Handoff-T-red:** docs/handoffs/643/handoff-T-red.md (round 2)

Round 1 of this handoff is in git: `git show 50bcc83:docs/handoffs/643/handoff-T-green.md`. This file replaces it.
Its AC table, Tier 2 analysis and mutation table (M1 to M11) still hold, because no production file has changed since
06320ad (`git diff 06320ad..HEAD -- crates CHANGELOG.md` lists only T's three test files).

## GREEN confirmation

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::` | 35 passed, 0 failed (60 filtered out) |
| `cargo test -p holler-cli --test pane_verbs` | 95 passed, 0 failed |

That is round 1's 33 read-verb tests plus T-red round 2's two new tests
(`text_output_escapes_each_hidden_class_and_keeps_plain_unicode` and `a_stored_dash_prints_apart_from_the_empty_value`),
with `read_verbs_call_no_adapter_or_probe` extended to the profile store.

F listed no tests under "Tests that look wrong (for T)", and T found none. No test was changed this phase.

**Spot-check (behaviour removed, test fails):** T re-ran T-red's mutation Mc on the current tree. It drops
`&& value != NO_VALUE` from `text_value` (`src/pane/list.rs:263`). Result: 34 passed, 1 failed,
`get::a_stored_dash_prints_apart_from_the_empty_value` panicked at `get.rs:362`. T restored the file with
`git checkout`, and `git status --short` was empty afterwards. Ma and Mb (T-red round 2) and M1 to M11 (round 1) were
run on the same production code and are not repeated here.

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0 | exit 0 | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| `cargo test --workspace` (as `HOLLER_STATE_DIR=<empty scratch dir> cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`) | 0 failed | 125 result lines: 1414 passed, 0 failed, 5 ignored; exit 0 | PASS |
| `cargo test -p holler-cli --test pane_cli_process` | 0 failed | 34 passed | PASS |
| `cargo test -p holler-cli --test cli_surface_test` | 0 failed | 3 passed | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | 0 failed | 3 passed | PASS |
| `cargo test -p holler-cli --test wire_selftest` (canary) | 0 failed | 3 passed | PASS |
| `cargo machete` | no unused deps | none found | PASS |
| `rustfmt --check --edition 2021` on the 7 touched `.rs` files | exit 0 | exit 0 | PASS |
| `cargo test -p holler-cli --test pane_verbs watch`, 3 runs (AC 14 flake check) | 0 failed each | 11, 11, 11 passed | PASS |

Server start and API smoke do not apply: until #649 the binary wires `Unwired`, and the verbs run in-process over the
fakes.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage: a test per AC | Round 1's table stands (AC 1 to 19 each have a named test; AC 20 to 24 are command checks, re-run below). AC 18 now also has the per-class test | PASS |
| Test quality | The two new tests each name one behaviour, assert T-written literals (not `{:?}` output), run at the in-process tier, and do not duplicate `text_output_escapes_c1_and_bidi_characters`: that one pins C1 and RLO and JSON argv; the new one pins one character per hidden class and that plain Unicode is not escaped. Mutations show each fails when its behaviour is removed. The suite stays proportionate (35 tests for three verbs) | PASS |
| Type safety | No production change; clippy clean under `-D warnings` | PASS |
| Error paths, data integrity, API contract | Unchanged from round 1. AC 17's guard now also asserts no `ProfileStoreOp::{CasPut, Delete, Rename}`, and that at least one profile-store read happened, so it is not vacuous | PASS |
| Security | Text mode escapes every hidden class the gate named (B-3); no secret-shaped value in tests or output (`demo-*`, `/srv/...` only) | PASS |
| Migration safety, protocol and goldens | No schema, store, `holler-proto` or golden change | N/A |
| AC 17, 21, 22 greps | `ports\.(herdr\|host\|harness\|prober)`, `not_implemented\|const STORY`, `unsafe` in the touched files | nothing printed; PASS |
| Manifests (AC 22) | `git diff --stat origin/main -- Cargo.toml Cargo.lock 'crates/*/Cargo.toml'` | empty; PASS |
| File sizes | `wc -l`: src 316, 275, 227; tests 370, 545, 311; all under 900. Test files are ASCII-only | PASS |
| Blast radius (AC 24) | `git diff --name-only origin/main...HEAD` lists the brief's files plus `docs/handoffs/643*`, the same set as round 1 | PASS |
| Playwright / browser | No such surface in this repo | N/A |

**Evidence appendix:** T added nothing this phase. The round-2 tests rely on unchanged-code facts that T-red already
entered (`fault.rs:90-98`, `profile_store.rs:27-37` and `:250-251`, `feed.rs:263-269`), and F kept them in the rebuilt
file. `wc -c docs/handoffs/643/evidence.md` is 11,696 bytes, under the gate's 12,000-byte cap, so the whole file reaches
the gate.

**F's commands cross-checked:** every command in handoff-F.md's Tier 1 self-check was re-run, with the same results
(35 / 95 / 34 / 3 / 3; clippy, lint, changelog, machete and rustfmt exit 0; greps empty; 316/275/227 lines), with one
difference: F's workspace run had 1 failure (`body_run_test::fresh_hello_and_presence_on_every_reconnect`, "hub did not
report listening within 10s"). T's run of the same command had 0 failures (1414 passed). That supports F's reading of
it as a flake in a file this branch does not touch.

## Acceptance criteria status

All 24 PASS, backed by the tests and commands in round 1's table (`git show 50bcc83:docs/handoffs/643/handoff-T-green.md`,
"Acceptance criteria status"). This round adds:

| AC | Status | New backing |
|---|---|---|
| 17 Observe nothing | PASS | `read_verbs_call_no_adapter_or_probe`, now also over the profile store |
| 18 No terminal injection | PASS | `text_output_escapes_each_hidden_class_and_keeps_plain_unicode` |
| Decision 11 (`-` for no value) | PASS | `a_stored_dash_prints_apart_from_the_empty_value` |

## Blocking issues

None.

## Advisory notes

- **The `body_run_test` reconnect flake** (F's run only) is outside this blast radius. If it recurs on `main`, it
  deserves its own issue. T did not file one from a single sighting.
- **M8 (the verb's own sort) still survives by construction**, as in round 1: every kit fake returns name order.
- **evidence.md has 304 bytes of headroom.** Any later entry has to fit, or the gate drops the tail.
- JSON mode writing DEL, C1 and bidi raw (#660, A's W-11) and A's open warns are unchanged and are the MO's.
