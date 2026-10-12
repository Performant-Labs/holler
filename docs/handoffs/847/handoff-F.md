# Handoff-F: #847 - remote-admin last_seen flake

**Date:** 2026-10-12
**Branch:** issue-847-implementation
**Issue:** #847

## What was done
- No production code. The brief marks this story as test-only (root cause is in the test; Files: "Production: none"; AC 5: no production file changes). T-red already authored the whole fix (the rewritten test plus the `joined_rig`, `presence_frame` and `wait_past_second` support helpers), and that fix is green against the unmodified hub. F had nothing to implement.
- F's work here was to confirm that the hand-off build is GREEN and that nothing in production is waiting on F.

## Design decisions
- Production stays unchanged, per the brief's D3 (no hub clock seam, no new env var) and AC 5. Adding any production code would break AC 5.

## Reuse / extend-vs-new
Not applicable to F: no object was created or extended. T-red extended `support/remote_admin_rig.rs` (`rig_with_env` now calls the extracted `joined_rig`) per A's W-3.

## Architecture notes for A
None. No production layer was touched.

## Deviations from spec / wireframe
None.

## Tier 1 self-check (incl. tests now GREEN)
```
$ cargo test -p holler-cli --test remote_admin_test remote_admin_traffic_never
test remote_admin_traffic_never_moves_the_bodys_last_seen ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 1.32s

$ HOLLER_HEARTBEAT_INTERVAL_MS=300 cargo test -p holler-cli --test remote_admin_test remote_admin_traffic_never
test remote_admin_traffic_never_moves_the_bodys_last_seen ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.90s

$ cargo test -p holler-cli --test remote_admin_test
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s

$ cargo clippy -p holler-cli --test remote_admin_test --test remote_admin_liveness_test -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.98s

$ git diff origin/main --stat -- crates/
 crates/holler-cli/tests/remote_admin_test.rs       | 63 +++++++++++++---------
 .../holler-cli/tests/support/remote_admin_rig.rs   | 46 ++++++++++++++--
 2 files changed, 79 insertions(+), 30 deletions(-)
```

## Evidence appendix
None: F changed nothing, so the diff relies on no new behavior outside it. T-red's handoff already cites the hub facts the test relies on (the liveness timeout is derived from `HOLLER_HEARTBEAT_INTERVAL_MS`; there is no roster touch in `admin.rs`).

## Tests that look wrong (for T)
None.

## Known issues
- Left for T-green, as T-red listed them: the 30 loaded runs (AC 3), clippy across `--all-targets`, which T-red saw fail at `tests/token_load_test.rs:60` in a file this story does not touch (confirm against main), and the pre-existing flake in `remote_say_wait_and_hub_query_json_match_local_after_masking` (out of scope under D5).
- No `CHANGELOG.md` entry was added. AC 5 limits the diff to the two test files and `docs/handoffs/`, and `scripts/changelog-check.sh` only checks that the `## [Unreleased]` heading exists. O or S should decide whether a test-only flake fix needs an entry.

## Files changed
None (no production files).
