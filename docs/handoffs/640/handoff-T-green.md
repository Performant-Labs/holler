# Handoff-T-green: Phase 7 - #640 part 3 of 3: the opt-in scratch-Herdr test, the contract docs and the ADR-0021 rows

**Date:** 2026-10-09 (11:25 PM MDT)
**Branch:** issue-640-implementation (F's commit `8993137`)
**Issue:** #640 (epic #633), part 3 of 3; the PR closes #640
**Handoff-F reviewed:** `docs/handoffs/640/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/640/handoff-T-red.md`

This file replaces part 2's `handoff-T-green.md`, as the brief's Handoffs line says. Part 2's is in git at `0ad2d8a`.

Every build ran with `CARGO_BUILD_JOBS=4`. T changed no file at GREEN except this handoff and `decisions.md`: F listed
no test as wrong ("Tests that look wrong (for T)": none), F changed no test file (`git diff --stat 5cd4687 8993137`
touches no `tests/` path), and T's review found nothing to repair. No production code was touched.

## GREEN confirmation

`cargo test -p holler-adapter-herdr --no-fail-fast` (default run, no variable set):

```
adapter_conformance_test  ok. 4 passed; 0 failed
adapter_messages_test     ok. 6 passed; 0 failed     (at RED: 1 passed, 5 failed)
adapter_test              ok. 18 passed; 0 failed
layout_test               ok. 10 passed; 0 failed
plan_splits_test          ok. 19 passed; 0 failed
protocol_test             ok. 33 passed; 0 failed
scratch_herdr_test        ok. 7 passed; 0 failed; 2 ignored
transport_test            ok. 13 passed; 0 failed   (still asserts `herdr.ping` at the transport level, AC 20)
```

**The two mutants of the Test plan** (each applied to the working tree, run, then reverted with `git checkout`):

| Mutant | Required | Result |
|---|---|---|
| (a) `protocol.rs:63` `EXCERPT_LIMIT` 64 becomes 65 | all four AC 18 tests fail | **Killed as required.** `a_misplaced_new_panes_id_is_cut_to_64`, `a_new_pane_with_no_cell_has_its_id_cut_to_64`, `a_vanished_split_targets_id_is_cut_to_64`, `a_tabless_workspaces_label_is_cut_to_64` FAILED; `adapter_messages_test` 2 passed, 4 failed. Every other target green. |
| (b) the `op` rename removed (`run_as` keeps the transport's `op`: `Timeout { op: o } => Timeout { op: o }`) | AC 16 fails, no other new test | **Killed as required.** Only `a_timeout_names_the_port_method` FAILED (5 passed, 1 failed). Every other target green. |

One extra spot-check, beyond the Test plan, for AC 17's guard: `run_as` mapping *every* error to `Timeout { op }`
(`other => other` became `_ => Timeout { op }`) fails `every_other_error_passes_through_unchanged` and the four AC 18
tests (1 passed, 5 failed). So AC 17 is not vacuous: it bites a rename that overreaches.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Crate tests | `cargo test -p holler-adapter-herdr --no-fail-fast` | all pass, 2 ignored | above | PASS |
| Contract crate | `cargo test -p holler-pane` | all pass | 14 targets, 86 passed, 0 failed | PASS |
| Workspace | `cargo test --workspace --no-fail-fast` | exit 0 | exit 0; 134 targets, 1519 passed, 0 failed, 7 ignored (no environmental failure this time, so no isolated `HOLLER_STATE_DIR` was needed) | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | exit 0, no warning | PASS |
| Format | `cargo fmt --check -p holler-adapter-herdr -p holler-pane` | clean | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0; warns only (none at 900): `error.rs` 713, `adapter_test.rs` 645, `wire_herdr/mod.rs` 654, `protocol_test.rs` 679, and four files outside this crate | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused deps | `cargo machete` | none | none found | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed (scans the new `docs/testing.md` section) | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-adapter-herdr -p holler-pane --no-deps` | clean | generated, no warning | PASS |
| Merge with `main` | `git merge-tree --write-tree HEAD origin/main` (`d9eabbb`) | clean | exit 0, no conflict | PASS |

**Cross-check against F's handoff:** every number F reported matches T's re-run (crate targets 4/6/18/10/19/33/7+2/13,
`holler-pane` 86, `docs_cli_test` 3, clippy, fmt, doc, lint warns, changelog, machete). No discrepancy.

## Tier 2 results

### The gating, without a server (AC 9-11), re-run by T

- **AC 9:** `cargo test -p holler-adapter-herdr --test scratch_herdr_test -- --list --ignored` lists exactly
  `scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane` and `scratch_herdr_passes_the_conformance_suite`.
  `grep -rn '#\[ignore' crates/holler-adapter-herdr/` prints exactly `scratch_herdr_test.rs:214` and `:268`, each with the
  pinned reason. The default run reports `2 ignored`. **PASS**
- **AC 10:** `env -i target/debug/deps/scratch_herdr_test-<hash> --ignored --nocapture`: 2 passed, each with one line
  `skipped (HOLLER_HERDR_SCRATCH is not 1): no scratch Herdr server was started`. **PASS**
- **AC 11:** `env -i HOLLER_HERDR_SCRATCH=1 <binary> --ignored`: both FAILED, each with
  ``HOLLER_HERDR_SCRATCH=1 asks for a scratch Herdr, but no executable `herdr` is on PATH``. **PASS**

### Against a real scratch Herdr (AC 12-14), against F's adapter

Command: `HOLLER_HERDR_SCRATCH=1 cargo test -p holler-adapter-herdr --test scratch_herdr_test -- --ignored --test-threads=1 --nocapture`,
then the built binary three more times with the same arguments.

```
test scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane ... Herdr reports version 0.9.1-preview.2026-09-21-0ff0f27e2226
the pane pass took 120.625936ms
ok
test scratch_herdr_passes_the_conformance_suite ... the conformance suite took 3.093355519s
ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 3.32s
```

Repeats: pane pass 97.8 ms, 80.0 ms, 82.1 ms; conformance suite 3.42 s, 2.50 s, 2.71 s; all `ok` (2 passed each).
AC 12 (all 11 conformance cases) and AC 13 (steps 1-6) **PASS** against real Herdr with F's `op` and `excerpt` changes in.

AC 14 record:

- Herdr version: `0.9.1-preview.2026-09-21-0ff0f27e2226`, as the scratch server reports it through `version()` (the
  build spike #636 tested, protocol 22). It is the same build `handoff-T-red.md` recorded from `herdr --version`. T
  did not run `herdr` by hand at GREEN (the brief's Operating rules: only the harness runs it), so this line is the
  server's own report rather than the CLI's `--version`.
- Durations: above (pane pass 80-121 ms including server start and proof; conformance suite 2.5-3.4 s for 11 servers).
  The opt-in tests assert only upper bounds.
- After the runs, `pgrep -af 'h640[.]'` (a pattern that cannot match the checking shell) printed nothing, and
  `pgrep -a herdr | grep -c holler640` printed `0`.
- `find /tmp -maxdepth 1 -name 'h640.*'` found `0` entries (the scratch base on this machine is `/tmp`, as at RED).

**PASS.** No real-Herdr gap, so AC 15 adds nothing.

### Structural checks

| Check | Method | Result |
|---|---|---|
| Coverage of each AC | every AC of sections A-D has a named test (table below); E and F are greps and gates, run below | PASS |
| Test quality | Each AC 16-18 test names a behaviour, reaches its site (AC 16 asserts each trap fired; AC 18's tests assert the rewrite or split happened), and is killed by the matching mutant only: (a) kills exactly the four AC 18 tests, (b) exactly AC 16. Section A's tests are pure and test the harness (Decision 15). No test duplicates another: part 2's `adapter_test.rs` still pins the split `pane-not-found` → `unavailable` mapping, AC 17 pins pass-through. The suite is proportionate: one file per topic, 6 new adapter tests for 3 criteria. | PASS |
| Type safety | clippy `-D warnings` clean; `run_as` is generic over the return type, no cast | PASS |
| Error handling | AC 16 (14 timeout rows), AC 17 (14 pass-through rows), AC 18 (4 message sites) | PASS |
| API contract | Public interface unchanged (F's architecture notes, confirmed by reading the diff: `run_as`, `ensure` and the `OP_*` constants are private, `excerpt` is `pub(crate)`). The seven port-method `op`s equal `HerdrOp::as_str` (AC 16 takes them from it); `herdr.connect` is documented in the adapter. | PASS |
| Security / isolation | AC 7's env guard, AC 2-6's name and socket guards pass by default; AC 8: one `Command::new` (`tests/scratch_herdr/mod.rs:247`), `std::env`/`env::var` under `tests/` only in `scratch_herdr/mod.rs`; AC 29: the narrowed pattern prints nothing over `src/` and `Cargo.toml`, and matches only `scratch_herdr/mod.rs` and `scratch_herdr_test.rs` under `tests/`. No secret is involved. | PASS |
| Migration safety | N/A (no schema, no store) | N/A |
| Protocol / goldens | N/A: no wire change to Holler's protocol; `docs/protocol/v2.md` untouched | N/A |
| Platform (AC 32) | no socket option, no `io::ErrorKind` assertion, no fixed sleep for readiness in the new tests (unchanged since RED) | PASS |
| Evidence appendix | `evidence.md` (F's twelve entries) covers every unchanged-code fact the tests rely on: the transport as the only `Timeout` producer, `HerdrOp`'s strings (verbatim `herdr.rs` excerpt), `excerpt`/`EXCERPT_LIMIT` (verbatim), where the four quoted values come from, `connect` → `connect_with`. T found no gap to append. | PASS |
| Playwright / browser | none in this repo | N/A |

### The doc and shape greps (AC 19-27, 29-31), re-run by T

- **AC 19:** `grep -n 'fn excerpt' crates/holler-adapter-herdr/src/*.rs` prints only
  `crates/holler-adapter-herdr/src/protocol.rs:580:pub(crate) fn excerpt(text: &str) -> String {`. The second grep prints
  `adapter.rs:312`, `:318`, `:422`, `:436` (each inside `excerpt(..)`) and `:383` (`workspace: workspace.label.clone()`,
  the returned value A finding 5 exempts). **PASS**
- **AC 20:** `git diff origin/main -- crates/holler-adapter-herdr/src/transport.rs` is empty; `transport_test` 13 passed.
  **PASS**
- **AC 21:** `grep -nE 'C-c|Enter' crates/holler-pane/src/ports.rs` prints nothing. **PASS**
- **AC 22:** the `ensure_pane` doc names `grid-out-of-range` and `grid-unreachable`. **PASS**
- **AC 23:** `error.rs:416` has `outside the Herdr workspace's extent`; `:457` has `<port>.<method>`. **PASS**
- **AC 24:** `grep -rnE '#640 records it|recorded by the Herdr adapter' crates/ docs/adr/` prints nothing. **PASS**
- **AC 25:** `grid-unreachable` count `4`; the two old sentences print nothing; `<port>.<method>` prints only
  `ADR-0021.md:396`, the §9 `timeout` row; `git diff --stat dc300ab -- docs/adr/ADR-0021.md` is `18 insertions, 11
  deletions`. Whether those are exactly A1-A9's lines is S's read. **PASS** (greps)
- **AC 26:** `docs/testing.md` names `HOLLER_HERDR_SCRATCH` (2), the section C command (`:444`) and "CI never runs them"
  (`:449`); `ci.yml`'s only `--ignored` run is `holler-cli --test body_run_test`. **PASS**
- **AC 27:** one entry right after the part-2 entry, linking #633, #649 and #640; `changelog-check: ok`. **PASS**
- **AC 30:** `cargo tree -p holler-adapter-herdr -e normal --depth 1` lists only `holler-pane` and `serde_json`; the only
  manifest change since `dc300ab` is the `tempfile` comment. **PASS**
- **AC 31:** every touched file is under 900 lines (largest: `error.rs` 713, `wire_herdr/mod.rs` 654,
  `ADR-0021.md` 569, `adapter.rs` 493); clippy (with the workspace's `too_many_lines` / `cognitive_complexity` lints)
  is clean. **PASS**

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 | PASS | `the_gate_runs_only_on_exactly_1` |
| 2 | PASS | `the_name_guard_refuses_the_default_session_by_name` |
| 3 | PASS | `the_name_guard_accepts_only_holler640_and_8_hex` |
| 4 | PASS | `generated_names_pass_the_guard_and_differ` |
| 5 | PASS | `the_socket_guard_keeps_the_socket_inside_the_root` |
| 6 | PASS | `the_server_is_proven_by_its_own_status` |
| 7 | PASS | `the_scratch_env_carries_nothing_of_the_live_herdr` |
| 8 | PASS | greps above (S reads the function) |
| 9-11 | PASS | the gate runs above |
| 12 | PASS | `scratch_herdr_passes_the_conformance_suite` (opt-in, 4 runs) |
| 13 | PASS | `scratch_herdr_creates_runs_sends_reads_closes_and_snapshots_a_pane` (opt-in, 4 runs) |
| 14 | PASS | the AC 14 record above |
| 15 | N/A | no real-Herdr gap |
| 16 | PASS | `a_timeout_names_the_port_method` (mutant (b) kills it) |
| 17 | PASS | `every_other_error_passes_through_unchanged`; part 2's split `pane-not-found` test in `adapter_test.rs` |
| 18 | PASS | the four `..._is_cut_to_64` tests (mutant (a) kills all four) |
| 19-27 | PASS | greps above (AC 25's "only A1-A9's lines" is S's read) |
| 28 | PASS | Tier 1 table |
| 29-31 | PASS | greps above |
| 32 | PASS | read of the new tests (S re-reads) |

## Blocking issues

None.

## Advisory notes

- `run_as` takes the deadline with `self.deadline()?` before the body runs, so an error from `deadline()` itself
  (a `usage` for a timeout too long to add to the clock) is not passed through the `Timeout` rename. That is correct:
  `deadline_after` returns no `Timeout`, and part 2's `config_is_validated_before_any_request` still pins it.
- AC 14's version line comes from the server's `version()` reply, not from a hand-run `herdr --version` (Operating rules);
  the build is the one RED recorded from `--version`.
- F's three left-overs for the operator (the `snapshot` port doc, A7's PROPOSED owner line, the #638 test-kit
  `ASSUMPTION (#640)` comments) are documentation follow-ups, outside T's criteria.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
