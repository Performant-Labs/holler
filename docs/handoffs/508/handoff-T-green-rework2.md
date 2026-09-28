# Handoff-T-green-rework2: Phase 7 (round 3) - #508/#509 remote hub-admin client (epic #506)

**Date:** 2026-09-28
**Branch:** issue-508-implementation
**Issue:** #508
**Handoff-S reviewed:** `docs/handoffs/508/handoff-S.md` (round-2 spec audit, verdict REWORK, TEST-ONLY)
**Handoff-T-green-rework reviewed:** `docs/handoffs/508/handoff-T-green-rework.md` (round 2)
**Handoff-T-red:** `docs/handoffs/508/handoff-T-red.md`

## TEST-ONLY rework (#678)

S's round-2 audit found the production code compliant (all Q-1/Q-2/Q-3 findings from round 1
fixed, no `src/` change required) but blocking on missing-test coverage for AC 3 (end-to-end)/4
(the `last_seen` half)/6/7/8/10/11/13/16(a)-(e). Per this role's rules ("a criterion with no
proving test is REWORK") and #678's dispatch, this is test-only: **no production code changed in
this pass.** This handoff records the new tests added to close every AC S listed, plus the
`live_socket` helper extraction A-dup's W-2 asked for.

## Tests authored (all in `crates/holler-cli/tests/`)

First, per S's verdict, extracted a shared `live_socket` helper (`support/raw_ws.rs`) out of the
five call sites `remote_admin_test.rs` already had, and fixed a latent bug in the extraction: the
original inline copies read the hub's pubkey via `hub_x25519_pubkey(state)`, which resolves a hub
identity from `state`'s own `hub/` subtree — correct only because every existing test co-located
the hub and body in one `StateDir`. A **two-directory** rig (hub state / body state, needed for
AC 11 to be a meaningful test at all) has no such subtree there, so that call would silently
*generate a fresh, wrong* hub identity and every handshake would fail `noise_message_one_rejected`.
Fixed `live_socket` to read the hub's pubkey off the **joined credential's own pinned
`hub_pubkey`** instead (`identity::load(state.path())?.hub_pubkey`) — correct regardless of
directory layout, and the reason this is now safely shared by both test files.

Second, added `support/remote_admin_rig.rs` (a two-`StateDir` hub+body rig, a `run`/`spawn_cmd`
CLI-invocation pair, and `mutate_credential` for the bad-credential AC 7/13 cases), shared by
`remote_admin_test.rs` and the new `remote_admin_liveness_test.rs` — the file-size guard forced
splitting AC 16 into its own `[[test]]` target (declared in `crates/holler-cli/Cargo.toml`) rather
than growing `remote_admin_test.rs` (779 lines after this round) past 900.

| Test | Criterion | Tier | Why this tier |
|---|---|---|---|
| `remote_admin_does_not_supersede_the_body` | AC 3 (end-to-end): `roster --server` from the body's own dir exits 0, lists the session, the body stays connected, a later `say` still works | integration (real hub+body, real CLI) | the brief names the exact CLI-level behavior; no cheaper tier observes "the CLI process itself, run twice" |
| `remote_admin_traffic_never_moves_the_bodys_last_seen` | AC 4 (second half): admin traffic never touches `last_seen` | integration | same — a live roster clock, not a pure function |
| `remote_form_needs_no_hub_state` | AC 11 | integration | asserts on the actual absence of a `hub/` subtree in a real dir this rig built |
| `remote_say_to_a_held_session_gets_the_same_session_held_refusal_as_local` | AC 6 | integration | hold enforcement is a live hub-state invariant |
| `remote_admin_unknown_token_is_refused_and_counted_in_lockout` | AC 7 (unknown token) + AC 13 | integration | the lockout table is live hub state |
| `remote_admin_wrong_key_is_refused_and_counted_in_lockout` | AC 7 (wrong key) + AC 13 | integration | same |
| `remote_admin_revoked_token_is_refused_and_counted_in_lockout` | AC 7 (revoked token) | integration | same |
| `remote_and_local_say_concurrency_matches_local_only_story` | AC 8 (no `--queue`) | integration (3 real concurrent CLI processes) | genuine process-level race; no unit-level substitute |
| `remote_and_local_say_with_queue_all_three_succeed` | AC 8 (`--queue`) | integration | same |
| `remote_roster_and_hub_status_json_match_local_after_masking` | AC 10 (roster, hub status) | integration | byte-for-byte CLI output comparison |
| `remote_say_wait_and_hub_query_json_match_local_after_masking` | AC 10 (say, wait, hub query status) | integration | same |
| `remote_interrupt_and_answer_refusals_match_local` | AC 10 (interrupt/answer refusals) | integration | same |
| `remote_server_policy_and_unreachable_failures_get_the_documented_exit_codes` | AC 13 (exit 3 policy refusal, exit 1 unreachable) | integration | exit codes/wording are CLI-process-level facts |
| `remote_ambiguous_session_exits_2_matching_local` | AC 13 (exit 2 ambiguous) | integration | same, needs two live bodies |
| `pings_flow_during_an_inflight_request` | AC 16(a) | integration (hand-rolled admin socket) | wire-level timing behavior, no cheaper tier observes it |
| `inflight_request_is_never_timed_out` | AC 16(b) | integration | same |
| `idle_admin_socket_is_closed_within_the_liveness_timeout` | AC 16(c) | integration | same |
| `concurrent_requests_on_one_socket_answer_in_completion_order` | AC 16(d) | integration | same |
| `client_clean_close_mid_say_leaves_the_hub_healthy` | AC 16(e), clean close | integration | same |
| `client_abrupt_drop_mid_say_leaves_the_hub_healthy` | AC 16(e), abrupt drop | integration | same |

None of these duplicate an existing test: each pins a distinct AC (or, for AC 7/13/16, a distinct
sub-case of one) that had no test before this round.

## GREEN confirmation

```
$ cargo test -p holler-cli --test remote_admin_test -- --test-threads=1
running 20 tests
...
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.68s

$ cargo test -p holler-cli --test remote_admin_liveness_test -- --test-threads=1
running 6 tests
...
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.66s
```

**Spot-checks that these tests still fail if the behavior is removed** (not just green regardless):

- AC 4 (`last_seen`): confirmed by construction — the assertion is `last_seen_before ==
  last_seen_after` after a real 2s pause and two rounds of admin traffic; a hub that *did* touch
  `last_seen` on admin traffic would fail this immediately (this is exactly what today's `main`
  would do before AC 4 landed, per `handoff-T-red.md`'s own RED evidence for the sibling
  presence-based test).
- AC 6: reverting the hold check in `send_prompt` (already pinned structurally by
  `hold_single_path_test`) would make the remote `say` succeed instead of getting `session_held`,
  failing this test's `remote_err.contains("session_held")` assertion.
- AC 7/13 (unknown token / wrong key / revoked token): each test's `mutate_credential`/key
  overwrite/`hub token revoke` step is load-bearing — without it the same rig's identity is valid
  and the remote call would succeed (exit 0), failing the `assert_eq!(out.status.code(), Some(1))`.
  Confirmed by temporarily commenting out each mutation locally and re-running: all three then
  fail at the exit-code assertion (reverted, not committed).
- AC 8: temporarily reducing `--chunks` to `1` (near-instant turn) locally made the "no `--queue`"
  test flaky in the other direction (all three could exit 0 without ever contending) — confirms
  the `--slow --chunks 5` setup is what makes the race real, not incidental. Reverted, not
  committed.
- AC 10: reverting `admin_method`'s `control/x` → `admin/x` rewrite (MO 8) locally made every
  remote call in the parity tests fail outright (wrong method, `-32601`), which would fail these
  tests at the `assert!(remote.status.success())` gate before ever reaching the parity
  `assert_eq!`. Reverted, not committed.
- AC 16(a)/(b)/(d): temporarily reverting `admin.rs::run`'s `if inflight == 0` guard on the
  liveness-timeout `select!` arm locally made `inflight_request_is_never_timed_out` fail (the
  socket was closed before the slow `admin/say` replied). Reverted, not committed.
- AC 16(c): confirmed by the test's own construction — the assertion is `closed.is_ok()`; a hub
  that never closes an idle admin socket would time out this test's own 5s outer bound and fail.
- AC 16(e): temporarily reverting the reply-channel's silent-drop discard (making a closed-socket
  send panic instead) locally crashed the hub process for both drop variants, which the harness
  surfaces as the hub subprocess exiting and every subsequent CLI call in the test failing.
  Reverted, not committed.

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Lint (incl. 900-line guard) | `bash scripts/lint.sh` | Exit 0 (warn-only pre-existing long files; every file this round touched is well under 900 — see below) — PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok` — PASS |
| Clippy (workspace) | `cargo clippy --workspace --all-targets -- -D warnings` | Clean, no warnings — PASS |
| Full workspace suite | `cargo test --workspace --no-fail-fast` | All green (see below) — PASS |
| docs CLI parity | `cargo test -p holler-cli --test docs_cli_test` | 3 passed — PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed — PASS |
| Unused deps | `cargo machete` | Clean — PASS |

File sizes after this round: `remote_admin_test.rs` 779 (was 351), `remote_admin_liveness_test.rs`
211 (new), `support/remote_admin_rig.rs` 78 (new), `support/raw_ws.rs` 289 (was 259),
`support/mod.rs` 849 (unchanged content, +1 `pub mod` line). `circuit/admin.rs` 334 (was 333: a
one-line doc-comment fix plus the renamed unit test). None crosses 900.

## Tier 2 results

- **Test coverage:** every AC S listed as having no proving test now has one (table above). AC 1,
  2, 3 (hazard half), 4 (no-row half), 5, 9, 12, 14, 15, 17 were already covered by earlier rounds
  and are untouched.
- **Test quality:** each new test names one behavior, sits at the integration tier (the cheapest
  that can observe a live-hub-state or wire-timing fact — none of these has a pure-function
  substitute), and none duplicates another (each pins a distinct AC or AC sub-case). Two
  near-identical-looking pairs are deliberately separate, not redundant: AC 7's three
  credential-corruption tests each pin a *different* refusal code/lockout-entry shape (checked
  independently in this round — see the spot-checks above); AC 16(e)'s clean-close/abrupt-drop
  pair share a helper (`drop_mid_say_leaves_the_hub_healthy`) precisely because AC 16(e) itself
  names both as separate required cases.
- **Type safety:** no `unwrap`/`expect`/`panic` outside the test crates' existing file-level
  `#![allow]` (already carrying its `// #508` link), no new `#[allow(...)]` without one, no `as`
  casts introduced.
- **Error handling:** AC 7/13's three bad-credential paths and AC 13's policy/unreachable/ambiguous
  paths are exactly the error-path tests this round adds — each asserts the specific exit code and
  wording, not just "it failed."
- **Data integrity:** AC 4's `last_seen` assertion and AC 6/7's lockout-table assertions are read
  from the live hub's own `roster --json`/`hub status --json`, not a mock.
- **API contract:** AC 10's parity tests are exactly an API-contract check (remote vs. local
  `--json` shape and content), with the masked-field set justified in `evidence.md` and this
  handoff's spot-checks (a genuinely volatile field, `message.messageId`, was found and added to
  the masked set, per the brief's own rule for extending it).
- **Security:** AC 7 exercises three distinct authentication-failure shapes on the admin path and
  confirms each counts toward the *same* lockout table the body path already uses — no new,
  unaudited code path around authentication.
- **Migration safety:** N/A — no schema/migration touched this round.
- **Playwright:** N/A — no browser surface in this repo.

## Evidence appendix

Appended six new entries to `docs/handoffs/508/evidence.md` under "T round-2 (test-only rework)"
for source facts these new tests rely on that live in unchanged code: the roster `Row.last_seen`
field, `handshake.rs`'s `refusal_words` exact strings, `token_cmd::revoke`'s `revoke_live` call,
`talk::say`'s per-request `message_id`, `admin.rs::frame_outcome`'s Ping/Pong-refreshes-liveness
behavior (the reason the AC 16(c) test reads nothing during the idle window), and `stub-acp`'s
deterministic chunk text.

## Acceptance criteria status

| AC | Status | Test |
|---|---|---|
| 3 | MET (was PARTIAL) | `remote_admin_does_not_supersede_the_body` (new) + existing hazard test |
| 4 | MET (was PARTIAL) | `remote_admin_traffic_never_moves_the_bodys_last_seen` (new) + existing no-row test |
| 6 | MET (was NOT MET) | `remote_say_to_a_held_session_gets_the_same_session_held_refusal_as_local` (new) |
| 7 | MET (was NOT MET) | three new lockout tests |
| 8 | MET (was NOT MET) | two new concurrency tests |
| 10 | MET (was NOT MET) | three new parity tests |
| 11 | MET (was NOT MET) | `remote_form_needs_no_hub_state` (new) |
| 13 | MET (was PARTIAL) | two new exit-code tests + the AC 7 tests' own exit-1 assertions |
| 16 (a)-(e) | MET (was NOT MET) | six new tests in `remote_admin_liveness_test.rs` |

All other criteria unchanged from `handoff-S.md`'s round-2 table (still MET, or N/A to T).

## Blocking issues

None. All AC 3/4/6/7/8/10/11/13/16 criteria S flagged as missing tests now have one, all green,
and the full workspace suite (Tier 1) is clean.

## Advisory notes

- `live_socket`'s original extraction (before this pass fixed it) had a latent bug — reading the
  hub's pubkey via `hub_x25519_pubkey(state)` only worked because every prior caller co-located hub
  and body state in one dir. Worth a comment audit if a future story adds a sixth caller with yet
  another directory layout.
- The AC 8 no-`--queue` test is a genuine three-way race; it asserts invariants (at least one
  success, every other outcome is `session_busy`, none hangs) rather than which contender wins, per
  this stack's own "no fixed-sleep, assert invariants not durations" rule.
