# Handoff-T-red: #847 remote-admin last_seen flake

**Date:** 2026-10-12
**Branch:** issue-847-implementation
**Brief / wireframe reviewed:** docs/handoffs/847-brief.md; no wireframe (no UI)

## A precondition
Confirmed: A returned PASS on the plan (`docs/handoffs/847/handoff-A.md`, four warns). A's W-1 (boundary wait before the admin calls), W-2 (keep the silent body's socket bound) and W-3 (one extracted presence helper in `support/remote_admin_rig.rs`) are applied.

## Tests authored
This story is test-only (no production change), so the authored artifact is the rewritten test plus three small support helpers. There is no feature for F to implement.

- `remote_admin_traffic_never_moves_the_bodys_last_seen` (`crates/holler-cli/tests/remote_admin_test.rs`), process level (real hub, real `holler` CLI, real admin socket), the cheapest tier that pins AC 4 as the brief requires (D4).
  - The "body" is `live_socket(.., "body")` that sends exactly one `session/presence` and then nothing; it is bound as `body_ws` to the end of the test.
  - Reads the baseline `last_seen`, then `wait_past_second(baseline)` (a `wait_for` on the wall clock's whole seconds; `hub status` exposes no `now` or uptime field, so the test process's clock is the observable, same `Clock::System`), THEN runs `roster --server` and `hub status --server` twice each.
  - Asserts `conn_state == "connected"` (not superseded) and `last_seen == baseline`.
  - No `thread::sleep`, no `Body::start` in the test.
  - Hub started with `HOLLER_HUB_LIVENESS_TIMEOUT_MS=120000`: see the finding under "RED confirmation".
- Support helpers in `crates/holler-cli/tests/support/remote_admin_rig.rs`: `joined_rig` (the first half of `rig_with_env`, which now calls it), `presence_frame` (the one `session/presence` literal, now also used by `remote_admin_hello_must_not_create_a_roster_row_from_presence`), `wait_past_second`.

## RED confirmation
1. **Old test, unmodified, reproduces the defect (3/3 RED).** `HOLLER_HEARTBEAT_INTERVAL_MS=300 cargo test -p holler-cli --test remote_admin_test remote_admin_traffic_never` panicked at `remote_admin_test.rs:380` all three times: `left: 1791825572 right: 1791825574`, `1791825574 vs 1791825576`, `1791825577 vs 1791825579` ("AC 4: admin/roster and admin/status traffic must never touch the body's last_seen"). Without the env var the old test passed (2.69 s).
2. **New test is green on the unmodified hub, including under the flake's trigger.** 1 run without env: ok (1.26 s). With `HOLLER_HEARTBEAT_INTERVAL_MS=300`: 15 of 15 ok (0.78 s to 2.09 s).
3. **New test is red against the AC 4 mutant (3/3).** Mutant: `deps.roster.touch(&record.token_id, "admin/mutant");` as the first statement of `circuit::admin::run` in `crates/holler-hub/src/circuit/admin.rs`. Panicked at `remote_admin_test.rs:392`, `left: 1791825688 right: 1791825687`, then `...692 vs ...690`, `...695 vs ...693` (left is the mutated value read back after the admin calls; right is the baseline). Restored with `git checkout -- crates/holler-hub`; `git diff --stat -- crates/holler-hub` is empty.

**Finding that changed the test (the brief's "green with HOLLER_HEARTBEAT_INTERVAL_MS=300" proof was not true as written).** The hub reads the same `HOLLER_HEARTBEAT_INTERVAL_MS` and derives its liveness timeout as 3 x that (`circuit.rs:103-109`), so at 300 ms the hub tears the silent body down after 900 ms. With the first version of the new test, 6 of 12 runs under that env failed at the `conn_state == "connected"` assertion with `conn_state: "reconnecting"` (the hub's correct handling of a silent body). Fix: the test pins `HOLLER_HUB_LIVENESS_TIMEOUT_MS=120000` on its own hub, via `joined_rig`'s `hub_env`. That is the hub's existing env override, not a new one (D3 holds), and it also makes the test independent of ambient env. The 15/15 above is with that fix.

## Ready for F
Confirmed: RED is valid for the defect (old test 3/3) and the new test is proven against the right mutant. There is no production work for F (AC 5: only `remote_admin_test.rs`, `support/remote_admin_rig.rs` and `docs/handoffs/` differ from main). Remaining for T-green: the 30 loaded runs, final `wc -l`, clippy and rustfmt reports.

## Interim facts for T-green
- `cargo test -p holler-cli --test remote_admin_test`: 21/21 on 5 of 6 runs with my change; the 6th failed in `remote_say_wait_and_hub_query_json_match_local_after_masking` (a different test). **Not caused by this change:** on the unmodified files the same test failed 1 of 14 full-target runs (`remote_admin_test.rs:706`); it passes 10 of 10 alone. Recorded as a follow-up under D5, not fixed here.
- `remote_admin_liveness_test`: 7/7.
- `cargo clippy -p holler-cli --test remote_admin_test --test remote_admin_liveness_test -- -D warnings`: clean. `cargo clippy -p holler-cli --all-targets` fails on `tests/token_load_test.rs:60` (`unneeded cloning of iterator items`), a file this story does not touch; confirm against main in T-green.
- `wc -l`: `remote_admin_test.rs` 827 (was 814, `lint.sh` limit 900), `remote_admin_rig.rs` 114.
- `rustfmt --check` on the two files reports pre-existing wide-line hunks across the whole file (the repo does not keep these files rustfmt-clean); my hunks follow the files' existing long-line style.
- `grep -n last_seen remote_admin_test.rs`: only the target test and the masking helper at line 643 (which masks the field). No other `last_seen` assertion. The only other time-sensitive test seen is the follow-up above.
- Longest observed run of the new test: 2.09 s, against the 45 s default (120 s pinned) liveness.
