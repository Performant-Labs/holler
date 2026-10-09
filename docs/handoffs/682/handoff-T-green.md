# Handoff-T-green: Phase 7 - #682 part 1, `FakeProfileStore` and the `ProfileStore` conformance suite

**Date:** 2026-10-09
**Branch:** issue-682-implementation (at 117b861, plus T's one test repair, uncommitted)
**Issue:** #682 (part 1 of 2; the PR is "Part of #682")
**Handoff-F reviewed:** docs/handoffs/682/handoff-F.md
**Handoff-T-red:** docs/handoffs/682/handoff-T-red.md

## GREEN confirmation
`cargo test -p holler-pane-testkit`: fake_pane_store_test 22, fake_profile_store_test 23, pane_store_conformance_test 10,
profile_store_conformance_test 9, all ok. Repeated 15 times with no failure (no flake).
`cargo test --workspace --no-fail-fast`: exit 0, 107 result lines, 1141 passed, 0 failed.

Spot-check that the tests pin behavior: in `FakeProfileStore::cas_put`, replacing the generation check with `current + 1`
makes `a_concurrent_put_makes_the_next_cas_stale` and `seeding_the_same_name_twice_is_a_conflict` fail. Source restored
(`git diff --quiet HEAD -- src` is clean). The six wrapper mutants in `profile_store_conformance_test.rs` pin the
suite's own cases.

## Test repair (the one item F flagged)
`sample_spec_is_deterministic_harmless_and_agrees_with_sample_pane` failed `clippy::cognitive_complexity` (16/15). Split
into `sample_spec_is_deterministic_and_harmless` and `sample_spec_agrees_with_sample_pane`, same assertions, no test
dropped. File is now 597 lines (under the 600 warn), rustfmt-clean. Only the test file changed; no production code.

## Tier 1 results
| Command | Result |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (was FAIL on the test above) |
| `cargo build --workspace` | PASS, no warnings |
| `cargo test --workspace --no-fail-fast` | PASS (see note on `body_run_test`) |
| `bash scripts/lint.sh` | PASS |
| `bash scripts/changelog-check.sh` | PASS (`changelog-check: ok`) |
| `bash scripts/test-hooks.sh` | PASS |
| `cargo machete` | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3) |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane-testkit --no-deps` | PASS |
| `rustfmt --check --edition 2021` on every changed `.rs` | PASS |

F's reported results reproduce, apart from the clippy failure that this phase fixed.

Note on `holler-cli` `body_run_test::fresh_hello_and_presence_on_every_reconnect`: it failed once in the first full run
and in 3 isolated reruns ("hub did not report listening within 10s: Disconnected"), then passed alone and in the next full
run. This diff touches nothing in `holler-cli`; the test restarts a hub on a fixed port and the host has many listeners.
It is a pre-existing environmental flake, not caused by #682. Not blocking.

## Tier 2 results
- AC1 to AC4: PASS, backed by the two new test files (list in handoff-T-red.md).
- AC5: PASS. `git diff origin/main` on both slice-a test files is empty, and they pass.
- AC6: PASS. Manifest untouched; no `holler-(cli|hub|adapter)` dependency.
- AC7: PASS. The three greps print nothing.
- AC8: PASS with the sanctioned deviation. `lib.rs`, `conformance/mod.rs` and both `profile_scope` stubs are unchanged. The
  diff adds `conformance/profile_store/log.rs`, which the brief's Risks section prescribes to stay under 600 lines.
- AC9: PASS. One entry at the end of `[Unreleased]` / Enhancements, links #682, and `changelog-check` passes.
- AC10: PASS. Largest `.rs` in the diff is 597 lines; clippy's `too_many_lines` and `cognitive_complexity` pass.
- Test quality: each test names a behavior; timing tests assert lower bounds or generous upper bounds, never an exact
  duration; waits use the feed, not sleeps; the suite is proportionate. No secret is involved.
- Protocol-visible change: none (test kit only), so no goldens or `v2.md` change.
- Playwright/browser: N/A (none in this repo).
- Evidence appendix: `evidence.md` has 11 entries with `file:line` and verbatim excerpts; the tests rely on no unchanged
  source fact outside them.

## Acceptance criteria status
AC1 to AC10: PASS (see above).

## Blocking issues
None.

## Advisory notes
- W-6 for the run's agent: use "Part of #682", not "Closes #682", and add the CONTRIBUTING.md AI disclosure with
  `gh pr edit`; confirm #682 is still open after the merge.
- W-2: O's follow-up to move the five shared helpers into `conformance/mod.rs` after slices b, d and e merge.
- `lib.rs`'s one-line summary of `fixture` is stale (names only `sample_pane`); AC8 forbids editing `lib.rs`.
- The test repair is uncommitted; the script's commit step should include `fake_profile_store_test.rs` and this handoff.
