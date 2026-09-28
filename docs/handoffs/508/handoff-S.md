# Handoff-S: Phase 10 (round 3, after the test-only REWORK) - #508/#509 remote hub-admin client (epic #506), spec audit

**Date:** 2026-09-27
**Branch:** issue-508-implementation (at 032588c)
**Issue:** #508 (together with #509, epic #506)
**Brief:** `docs/handoffs/506-brief.md`
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-F-rework.md`, `handoff-T-green-rework.md`, `handoff-T-green-rework2.md` (round 3), `handoff-A-dup.md` (round 3), `decisions.md`, `evidence.md`
**Diff audited:** `git diff origin/main...HEAD` in full. I read the round-3 delta `7f8e39f..HEAD` line by line: `remote_admin_test.rs`, the new `remote_admin_liveness_test.rs`, `support/remote_admin_rig.rs`, `support/raw_ws.rs`, `Cargo.toml`, and the `circuit/admin.rs` test rename.

This supersedes the round-2 S handoff (REWORK at ef24c71), which is still in git history.

## A precondition

Met. `handoff-A.md` is **PASS** (8 warns). `handoff-A-dup.md` round 3 is **PASS** (2 new warns, with earlier warns carried). Neither has a block.

## T precondition

Met. `handoff-T-green-rework2.md` reports "Blocking issues: None". `remote_admin_test` has 20 passing tests and `remote_admin_liveness_test` has 6. The full workspace suite and Tier 1 are clean. RED/failure evidence comes from revert spot-checks: the credential mutations, the `inflight == 0` guard and the `admin_method` rewrite were each reverted, and the matching tests failed. The original RED for AC 1-4 and 9 is in `handoff-T-red.md`.

## Acceptance criteria

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Seven `admin/*` catalog rows | `codec_test.rs::catalog_has_all_seven_admin_rows`, `::every_method_round_trips` | MET |
| 2 | `HelloRole::Admin` is `"admin"` and round-trips | `docs_wire_test.rs::admin_role_wire_test::hello_role_admin_deserializes` (deserializes, then asserts that re-serialization gives `"admin"`) | MET |
| 3 | No supersede, no roster hijack | `remote_admin_hello_must_not_supersede_the_connected_body` (hand-rolled socket, asserts no `conn_superseded`) and `remote_admin_does_not_supersede_the_body` (real CLI: exit 0, lists the session, row `connected`, a later `say` succeeds). The end-to-end test does **not** assert the brief's "body log has no superseded" clause. | PARTIAL (item 2) |
| 4 | Admin never creates or refreshes a roster row | `remote_admin_hello_must_not_create_a_roster_row_from_presence` and `remote_admin_traffic_never_moves_the_bodys_last_seen` | MET |
| 5 | Allowlist and `-32601` | `admin_socket_refuses_every_non_admin_method_with_method_not_found` (all six named methods on the wire, id echoed, 10 s timeout), `body_socket_sending_admin_roster_gets_method_not_found`, `circuit::admin::tests::allowlist_is_exactly_the_six_mapped_verbs_plus_query` | MET |
| 6 | Hold parity | `remote_say_to_a_held_session_gets_the_same_session_held_refusal_as_local`. `hold_single_path_test` is unmodified (not in the diff). | MET |
| 7 | `-32002` and lockout on the admin path | `remote_admin_unknown_token_is_refused_and_counted_in_lockout`, `remote_admin_wrong_key_is_refused_and_counted_in_lockout`, `remote_admin_revoked_token_is_refused_and_counted_in_lockout` | MET (see item 1 on the wrong-key assertion) |
| 8 | 2 remote + 1 local `say`, with and without `--queue` | `remote_and_local_say_concurrency_matches_local_only_story`, `remote_and_local_say_with_queue_all_three_succeed` | MET |
| 9 | `--server` on 7 verbs; cli-surface fixture | `cli_surface_test`, fixture lines | MET |
| 10 | `--json` byte parity, masked by one helper | `remote_roster_and_hub_status_json_match_local_after_masking`, `remote_say_wait_and_hub_query_json_match_local_after_masking`, `remote_interrupt_and_answer_refusals_match_local`. The `say` masking (`elapsed_ms`, `message.messageId`) is done inline at lines 670-675 instead of in the `masked` helper. | MET in substance; helper deviation (item 4) |
| 11 | Remote form needs no hub state | `remote_form_needs_no_hub_state`. Every rig test also runs the remote CLI from the separate body dir. | MET |
| 12 | No-`--server` behaviour unchanged | Existing suites unchanged and green (T Tier 1) | MET |
| 13 | Failure words and exit codes | Exit 3 policy refusal and exit 1 unreachable: `remote_server_policy_and_unreachable_failures_get_the_documented_exit_codes`. Ambiguous `say` exit 2: `remote_ambiguous_session_exits_2_matching_local`. **Hub-key mismatch: no test.** The wrong-key test accepts `"does not match the key the hub registered at join" \|\| "hub public key mismatch"`, so neither case is pinned. **Missing credential:** `roster_remote_server_flag_is_not_yet_recognized` asserts only `!success` and an OR of three phrases, not exit 1 with both the path and `holler body join`. | PARTIAL (items 1, 3) |
| 14 | Workspace tests, lint, clippy green; 900-line guard | T-green-rework2 Tier 1. `wc -l`: circuit.rs 898, docs.rs 899, codec_test.rs 889, support/mod.rs 849, remote_admin_test.rs 779. All are under 900. | MET |
| 15 | v2.md: admin role, methods, posture, lockout shared fate | Unchanged since round 2, when it was MET | MET |
| 16 | Admin-loop liveness (a) to (e) | `remote_admin_liveness_test.rs`: `pings_flow_during_an_inflight_request`, `inflight_request_is_never_timed_out`, `idle_admin_socket_is_closed_within_the_liveness_timeout`, `concurrent_requests_on_one_socket_answer_in_completion_order`, `client_clean_close_mid_say_leaves_the_hub_healthy`, `client_abrupt_drop_mid_say_leaves_the_hub_healthy` | MET |
| 17 | `admin_connected`/`admin_dropped` 4 fields; no `conn_connected` | `admin_connected_and_dropped_log_lines_carry_all_four_fields` (now asserts exactly one line of each) | MET |

## Spec compliance

- Production code is unchanged since round 2, which found it compliant. The only `src/` change this round is the `#[cfg(test)]` allowlist test rename and its doc comment in `circuit/admin.rs`. MO 1-8 are compliant. The MO 9 two-variant deviation stays accepted (A-dup W-3).
- AC 10's extension of the masked set with `message.messageId` follows the brief's own rule. The field is justified in `evidence.md` (`talk.rs:458`). T recorded it in T-green-rework2 rather than the T-red handoff. That is a procedural difference only.
- The comparison of only the trailing `error:` line for refusal parity is justified. The extra lines on the remote stderr are the WS transport's own debug-tier frame logging, not part of the rendered refusal. I accept it.
- AC 10's "one helper" rule is not followed for `say` (item 4).

## Quality audit

**Production:** no change this round. There is no `unwrap`/`expect`/`panic` outside tests and no new `#[allow]`. Both test-file `#![allow]`s carry `// #508`. No file is at or above 900 lines.

**Protocol:** there is no wire change beyond the round-1 additive rows. The error-code table and golden files are unchanged.

**Tests:**
- They use the real hub and body, and a separate `StateDir` for each, as the brief's Reuse map requires.
- `live_socket` was extracted, and T found and fixed its latent hub-pubkey bug.
- Synchronization is done with `wait_for` on roster and log state. Three sleeps are intrinsic to the behaviour under test rather than synchronization: the 2 s gap in AC 4 lets a moved `last_seen` show; the idle windows in AC 16(b) and 16(c) must outlast the liveness timeout.
- One sleep is synchronization: the 100 ms in `drop_mid_say_leaves_the_hub_healthy`, which waits "for the request to reach dispatch". If it is too short, the test fails and does not pass falsely, because it requires `last_turn` to be set. It is advisory.
- The tests assert behaviour: exit codes, refusal words, lockout table entries and wire ids.

**Documentation:** the `CHANGELOG.md` `## [Unreleased]` entry links #508, #509 and #506. Its "byte-identical `--json`" claim is now backed by the AC 10 tests.

**Privacy:** I grepped the diff for tailnet and host names, private IP ranges and key or secret patterns. The only hits are the brief's placeholders (`hub.example.ts.net`, `10.0.0.5`) and the TEST-NET `192.0.2.7`. Nothing is personal.

**Commit and PR hygiene (advisory, unchanged):** the `chore(#508): …` subjects follow Conventional Commits. The trailers are `Co-Authored-By: Claude <noreply@anthropic.com>` with no session link. There is no PR yet, so AI disclosure cannot be checked. O must add the disclosure per `CONTRIBUTING.md` when it opens the PR.

## Scope check

In scope. This round is test-only apart from the admin.rs test rename. Splitting AC 16 into `remote_admin_liveness_test.rs` departs from the brief's "all in `remote_admin_test.rs`". T explained that the 900-line guard forced it, and I accept it as a necessary decomposition. There is no over-delivery.

## Verdict

**REWORK** (TEST-ONLY: no `src/` change required)

A small closing list. Every item is in `crates/holler-cli/tests/remote_admin_test.rs`, which is at 779 lines and has room.

1. **AC 13 hub-key mismatch (no test) and AC 7 wrong-key precision.**
   - Add `remote_admin_hub_key_mismatch_exits_1_with_body_run_words`. Use `mutate_credential(&body_state, |v| v["hub_pubkey"] = json!("<any other valid 64-hex key>"))`. Run `roster --server <ws> --json`. Assert `status.code() == Some(1)` and that stderr contains `"hub public key mismatch"` (the `body run` words, `handshake.rs:172/285`).
   - In `remote_admin_wrong_key_is_refused_and_counted_in_lockout` (lines 469-472), drop the `|| "hub public key mismatch"` alternative. Assert only the `-32002` key-mismatch words, `"does not match the key the hub registered at join"` (`handshake.rs:190`). Today the disjunction lets each case pass as the other.
2. **AC 3 end-to-end (lines 341-345).** In `remote_admin_does_not_supersede_the_body`, after the remote roster, add `assert!(!body.log_text().contains("conn_superseded"), …)`. The brief's AC 3 explicitly says "the body log has no superseded".
3. **AC 13 missing credential (lines 60-71).** In `roster_remote_server_flag_is_not_yet_recognized`, assert `out.status.code() == Some(1)`. Replace the three-way OR with two assertions: stderr contains `credential.json` (the path) **and** `holler body join`. `AdminClientError::NotJoined` prints both (`admin_client.rs:55`).
4. **AC 10 single masking helper (lines 670-675).** Add a `"say"` arm to `masked` that masks `elapsed_ms` and `message.messageId`. Use `masked("say", …)` in `remote_say_wait_and_hub_query_json_match_local_after_masking` instead of the inline loop, so the masked set lives in exactly one helper, as the brief states.

## Advisory notes

- **AC 13 ambiguous `hub query` target:** there is no remote end-to-end test, and I do not require one. A hub-query target ambiguity is a body-hostname collision, which #184's uniqueness enforcement prevents end to end (see `query_test.rs:423-445`, which pins the local side the same way). The remote path reaches the same `is_ambiguous` branch through `ControlError::Refused` (`hub_cmd.rs:194`). The remote ambiguous-`say` test proves that remote errors map to `Refused`. The brief's wording asks for more than the system can produce. O may note this in decisions.md.
- **AC 10 `interrupt`/`answer` success-path stdout parity:** only the refusals are compared. The AC text ("including one refusal each") is satisfied, but a success-path comparison would strengthen the byte-identical claim.
- **AC 8:** `wait_with_output` has no outer timeout, so a hang would stall the test instead of failing it. The `--queue` variant does not re-check "body still connected, a fourth `say` succeeds" (the no-queue variant does).
- **AC 16(e):** the 100 ms pre-drop sleep is a synchronization sleep. A `wait_for` on the session's roster `state` leaving `idle` would be deterministic.
- **Carried from earlier rounds:**
  - A-dup W-1: `reply_decode_error` repeats `wire.rs`'s frame build.
  - `transport.rs`: `resolve_state_dir().unwrap_or_default()` falls back to an empty path.
  - circuit.rs (898) and docs.rs (899) have no headroom.
  - Commit trailers have no session link. The PR's AI disclosure is still to be added.
