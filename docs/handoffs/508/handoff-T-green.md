# Handoff-T-green: Phase 7 (re-entry, test-only rework #678) - remote hub-admin client

**Date:** 2026-09-28
**Branch:** issue-508-implementation
**Issue:** #508
**Handoff-F reviewed:** `docs/handoffs/508/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/508/handoff-T-red.md`
**Prior rounds reviewed:** `handoff-T-green-rework.md`, `handoff-T-green-rework2.md`, `handoff-S.md` (round 3)

## Context

Both preconditions were already met (A and A-dup round 3 PASS). Production code has not changed
since round 2, which A/S already found compliant. This round is **test-only rework** per #678:
S's round-3 audit (`handoff-S.md`) found four small test gaps, all in
`crates/holler-cli/tests/remote_admin_test.rs`, and no `src/` change required. Per this role's
rules for a test-only rework, I repaired/added the named tests myself and made **no** production
code change.

## Test-only changes made

All in `crates/holler-cli/tests/remote_admin_test.rs` (779 → 813 lines, still well under the
900-line guard):

1. **AC 13 hub-key mismatch (new test).** Added
   `remote_admin_hub_key_mismatch_is_refused`: uses `mutate_credential` to overwrite the joined
   credential's `hub_pubkey` with a value the real hub never presented, runs
   `roster --server <ws> --json`, and asserts exit code `Some(1)` and stderr containing
   `"hub public key mismatch"` — the words the body's own handshake code emits when the hub's
   advertised key does not match the credential's pinned one
   (`crates/holler-body/src/connection/handshake.rs:283-286`, unchanged code).
   In `remote_admin_wrong_key_is_refused_and_counted_in_lockout` (the *body's own* key corrupted,
   not the pinned hub key), removed the `|| stderr.contains("hub public key mismatch")`
   alternative so it now asserts only
   `"does not match the key the hub registered at join"` (`handshake.rs:190`) — the disjunction
   previously let each test pass as the other, so neither pinned its own distinct failure mode.
2. **AC 3 end-to-end.** `remote_admin_does_not_supersede_the_body` now also asserts
   `!body.log_text().contains("conn_superseded")` after the remote roster call, per the brief's
   own "the body log has no superseded" wording for this AC — previously only the roster/say
   assertions ran, never the body log.
3. **AC 13 missing credential.** `roster_remote_server_flag_is_not_yet_recognized` now asserts
   `out.status.code() == Some(1)` (was: any non-success), and that stderr contains **both**
   `"credential.json"` and `"holler body join"` (was: a three-way OR that let any one of three
   phrases pass). `AdminClientError::NotJoined`'s `Display` impl prints both substrings in one
   message (`crates/holler-body/src/admin_client.rs:55`), so the tightened assertion is not a
   stricter requirement on the implementation — it is the same behavior, pinned precisely.
4. **AC 10 single masking helper.** Added a `"say"` arm to the `masked` helper (masks
   `elapsed_ms` and `message.messageId`), and `remote_say_wait_and_hub_query_json_match_local_
   after_masking` now calls `masked("say", …)` instead of the inline per-field loop it had — the
   masked set now lives in exactly one helper, as the brief states.

No production (`src/`) file was touched.

## GREEN confirmation

```
$ cargo test -p holler-cli --test remote_admin_test 2>&1 | tail -30
running 21 tests
test roster_remote_server_flag_is_not_yet_recognized ... ok
test body_socket_sending_admin_roster_gets_method_not_found ... ok
test admin_socket_refuses_every_non_admin_method_with_method_not_found ... ok
test remote_admin_revoked_token_is_refused_and_counted_in_lockout ... ok
test admin_connected_and_dropped_log_lines_carry_all_four_fields ... ok
test remote_server_policy_and_unreachable_failures_get_the_documented_exit_codes ... ok
test remote_say_to_a_held_session_gets_the_same_session_held_refusal_as_local ... ok
test remote_form_needs_no_hub_state ... ok
test remote_interrupt_and_answer_refusals_match_local ... ok
test remote_admin_hub_key_mismatch_is_refused ... ok
test remote_admin_unknown_token_is_refused_and_counted_in_lockout ... ok
test remote_roster_and_hub_status_json_match_local_after_masking ... ok
test remote_admin_wrong_key_is_refused_and_counted_in_lockout ... ok
test remote_admin_does_not_supersede_the_body ... ok
test remote_say_wait_and_hub_query_json_match_local_after_masking ... ok
test remote_ambiguous_session_exits_2_matching_local ... ok
test remote_and_local_say_with_queue_all_three_succeed ... ok
test remote_and_local_say_concurrency_matches_local_only_story ... ok
test remote_admin_hello_must_not_supersede_the_connected_body ... ok
test remote_admin_traffic_never_moves_the_bodys_last_seen ... ok
test remote_admin_hello_must_not_create_a_roster_row_from_presence ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
```

21 tests (was 20) — the one new test (`remote_admin_hub_key_mismatch_is_refused`) plus the three
tightened assertions all green.

```
$ cargo test -p holler-cli --test remote_admin_liveness_test 2>&1 | tail -10
running 6 tests
test inflight_request_is_never_timed_out ... ok
test pings_flow_during_an_inflight_request ... ok
test concurrent_requests_on_one_socket_answer_in_completion_order ... ok
test idle_admin_socket_is_closed_within_the_liveness_timeout ... ok
test client_clean_close_mid_say_leaves_the_hub_healthy ... ok
test client_abrupt_drop_mid_say_leaves_the_hub_healthy ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
```

**Spot-check (test pins behavior, not implementation):** the new `remote_admin_hub_key_mismatch_
is_refused` fails if the pinned-hub-key check is removed — I confirmed the assertion is on
handshake-refusal text that only appears when `hub_hello.hub_pubkey != identity.hub_pubkey`
(`handshake.rs:281-286`); an implementation that skipped this check would either hang the noise
handshake or fail a different assertion, never pass this one silently. The tightened
`roster_remote_server_flag_is_not_yet_recognized` assertion (both substrings, not an OR) fails if
either half of `NotJoined`'s message is dropped, closing the exact loophole S named.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0, no file ≥900 lines | exit 0; largest touched file is `remote_admin_test.rs` at 813 lines (warn-only threshold, guard is 900) | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean, no warnings | PASS |
| Full workspace tests | `cargo test --workspace` | all green | all green, no `FAILED`, no `error[` | PASS |
| docs_cli_test | `cargo test -p holler-cli --test docs_cli_test` | 3 passed | 3 passed | PASS |
| cargo machete | `cargo machete` | no unused deps | "didn't find any unused dependencies" | PASS |
| wire_selftest canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed | 3 passed | PASS |

## Tier 2 results

- **Test coverage:** all 4 gaps S's round-3 audit named now have a proving test or tightened
  assertion; no new gap introduced. See "Acceptance criteria status" below.
- **Test quality:** each touched/added test names one behavior (hub-key mismatch; body-log
  no-supersede; exit-1 + both-substrings on missing credential; single masking helper), fails in
  isolation for the right reason (verified by the spot-check above), sits at the cheapest
  sufficient tier (process-level CLI test, matching its siblings — no cheaper tier proves an
  end-to-end refusal message or an exit code), and does not duplicate another test. The suite
  stays proportionate: no test was added beyond the four S named, and the inline masking loop
  removed in favor of `masked("say", …)` is a reduction, not an addition.
- **Type safety:** no `unwrap`/`expect`/`panic` outside the file's existing `#![allow(...)]`
  (already carrying its `// #508` link); no new `#[allow(...)]`; no `as` casts.
- **Error handling:** the new/tightened tests assert exact exit codes (`Some(1)`) and exact
  wording, not just "it failed" — closing exactly the either-or loopholes S flagged.
- **Data integrity:** N/A to this round — no roster/store code touched.
- **API contract:** N/A to this round — no wire-shape change; the `masked("say", …)` refactor
  keeps the same masked field set (`elapsed_ms`, `message.messageId`).
- **Security:** the new hub-key-mismatch test exercises a distinct authentication-failure shape
  (hub-side key, not body-side) with no new, unaudited code path — it reuses the same
  `mutate_credential` + `roster --server` rig as the sibling AC 7/13 tests.
- **Migration safety:** N/A.
- **Playwright:** N/A — no browser/visual surface in this repo.

### Evidence appendix

Two entries were missing for source facts the new/tightened tests rely on in unchanged code (F's
appendix and T's prior rounds did not need them, since no prior test isolated the hub-key-mismatch
wording or the `NotJoined` message's two substrings). Appended to `docs/handoffs/508/evidence.md`
under "T round-3 (test-only rework)":

- The `hub_hello.hub_pubkey != identity.hub_pubkey` branch in `handshake.rs:280-287`, whose exact
  words (`"hub public key mismatch: pinned … but this hub presented …"`) are distinct from the
  body-side `KEY_MISMATCH_REASON` wording — the fact `remote_admin_hub_key_mismatch_is_refused`
  depends on to assert the right message for the right corruption.
- `AdminClientError::NotJoined`'s `Display` impl (`admin_client.rs:55`), which prints both the
  credential path (ending `credential.json`) and `` run `holler body join` first `` in one
  message — the fact the tightened `roster_remote_server_flag_is_not_yet_recognized` assertion
  relies on.

## Cross-check of prior round's verification

Re-ran the commands `handoff-T-green-rework2.md` reported (full workspace suite, clippy, lint,
machete, changelog-check, docs_cli_test, wire_selftest) — all reproduce green, consistent with
that handoff. No discrepancy found.

## Acceptance criteria status

| AC | Status | Test |
|---|---|---|
| 13 (hub-key mismatch) | MET (was: no test) | `remote_admin_hub_key_mismatch_is_refused` (new) |
| 13 (wrong body-key precision) | MET (was: either-or let each pass as the other) | `remote_admin_wrong_key_is_refused_and_counted_in_lockout` (tightened) |
| 3 (end-to-end, body log) | MET (was: body log unchecked) | `remote_admin_does_not_supersede_the_body` (tightened) |
| 13 (missing credential) | MET (was: any one of three phrases accepted) | `roster_remote_server_flag_is_not_yet_recognized` (tightened) |
| 10 (single masking helper) | MET (was: inline duplicate masking) | `masked("say", …)` + `remote_say_wait_and_hub_query_json_match_local_after_masking` |

All other criteria unchanged from `handoff-T-green-rework2.md`'s table (still MET or N/A).

## Blocking issues

None. All four gaps S's round-3 audit named are closed, all green, and the full workspace suite
(Tier 1) is clean. No production code changed this round.

## Advisory notes

Carried forward, unchanged (S's own advisory list — none of these are blocking and none is
test-only, so none was actioned this round): the ambiguous `hub query` target cannot be produced
end to end (#184 uniqueness); interrupt/answer parity is compared only on refusals; AC 8 tests
have no outer timeout on `wait_with_output`; AC 16(e) uses a 100ms sleep instead of `wait_for`;
`reply_decode_error`'s partial duplication of `wire.rs`; the empty-path fallback in `transport.rs`;
`circuit.rs`/`docs.rs` headroom; commit trailers with no session link; the PR's AI disclosure
still to be added by O.
