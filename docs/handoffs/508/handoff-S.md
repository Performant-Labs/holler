# Handoff-S: Phase 10, round 4 (final manual audit) - #508/#509 remote hub-admin client (epic #506), spec audit

**Date:** 2026-09-28
**Branch:** issue-508-implementation (at acdd158)
**Issue:** #508 (together with #509, epic #506)
**Brief:** `docs/handoffs/506-brief.md`
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` (now carries the round-3 test-only rework detail), `handoff-F-rework.md`, `handoff-T-green-rework.md`, `handoff-T-green-rework2.md`, `handoff-A-dup.md` (round 4), the round-3 `handoff-S.md` (6cd1653, superseded by this file), `decisions.md`, `evidence.md`
**Diff audited:** `git diff origin/main...HEAD` in full. The production diff (`crates/*/src`, 1,769 diff lines) was re-read end to end. The round-3 rework delta `032588c..acdd158` was read line by line. It is test-only: `git diff 032588c HEAD -- crates/` touches only `crates/holler-cli/tests/remote_admin_test.rs` (`git show 237b444`). `remote_admin_test.rs` (813 lines) and `remote_admin_liveness_test.rs` (211 lines) were read in full, along with `support/remote_admin_rig.rs`.

This is a manual, ad hoc re-run. The automated pipeline stopped at its round-budget cap after the round-3 REWORK. The round-3 fixes were committed at 237b444, and the round-4 A-dup gate passed on them. Until this file, no S audit had covered that state.

## A precondition

Met. `handoff-A.md` is **PASS** (8 warns). `handoff-A-dup.md` round 4 is **PASS** with 1 new warn: `remote_admin_test.rs` is at 813 lines, past the ~800-line flag. Earlier warns are carried. Neither handoff has a block.

## T precondition

Met. The round-3 T pass (`decisions.md`, "T (Phase 7 re-entry ... round-3 REWORK)", with detail in `handoff-T-green.md`) reports no blocking issues.
- Test counts: `remote_admin_test` has 21 passing tests (up from 20) and `remote_admin_liveness_test` has 6.
- Tier 1 was green immediately before this audit (the operator's statement).
- I re-ran `cargo test -p holler-cli --test remote_admin_test -- --test-threads=1` myself: **21 passed, 0 failed** (23.4 s).
- RED/failure evidence: the original RED for AC 1-4 and 9 is in `handoff-T-red.md`. Revert spot-checks for AC 4/6/7/8/10/16 are in `handoff-T-green-rework2.md`.

## Acceptance criteria

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Seven `admin/*` catalog rows | `codec_test.rs::catalog_has_all_seven_admin_rows`, `::admin_roster_method_decodes_once_catalogued`, `::every_method_round_trips` (7 new `canonical_frame` arms) | MET |
| 2 | `HelloRole::Admin` is `"admin"` and round-trips | `docs_wire_test.rs::admin_role_wire_test::hello_role_admin_deserializes` (deserializes `role:"admin"`, then asserts re-serialization gives `"admin"`) | MET |
| 3 | No supersede, no roster hijack | Hazard test: `remote_admin_hello_must_not_supersede_the_connected_body`. End-to-end test: `remote_admin_does_not_supersede_the_body` runs the real CLI `roster --server` from the body's own dir and asserts: exit 0; the session is listed; the row is `connected`; a later `say` exits 0; and, **new at 237b444**, `!body.log_text().contains("conn_superseded")`. That is the body's own event for an inbound `circuit/superseded` (`holler-body/src/connection.rs:707`). `Body::log_text` reads the body's log file (`support/mod.rs:703`). The check runs after the `say`, so a supersede has had time to land. | MET (round-3 item 2 closed) |
| 4 | Admin never creates or refreshes a roster row | `remote_admin_hello_must_not_create_a_roster_row_from_presence` (sends `session/presence` on an admin socket; no row appears within 3 s). `remote_admin_traffic_never_moves_the_bodys_last_seen` (compares `last_seen` before and after a 2 s gap plus `admin/roster` and `admin/status`). | MET |
| 5 | Allowlist and `-32601` | `admin_socket_refuses_every_non_admin_method_with_method_not_found` sends all six named methods on the wire. It asserts `-32601` and that the id is echoed, with a 10 s timeout. `body_socket_sending_admin_roster_gets_method_not_found`. Unit test `circuit::admin::tests::allowlist_is_exactly_the_six_mapped_verbs_plus_query`. | MET |
| 6 | Hold parity; `hold_single_path_test` unmodified | `remote_say_to_a_held_session_gets_the_same_session_held_refusal_as_local` checks equal exit codes, `session_held` in both stderr streams, and the reason carried through. `hold_single_path_test.rs` is not in the diff. | MET |
| 7 | `-32002` and lockout on the admin path | Unknown token: asserts exit 1, the `token_unknown` words and a `token_unknown` lockout reason. Revoked token: asserts exit 1, the `token_not_bound` words and a lockout entry naming the token. Wrong key: **tightened at 237b444**. It now asserts only `"does not match the key the hub registered at join"` (`handshake.rs:190`, `KEY_MISMATCH`/`NO_PUBLIC_KEY`), and the lockout table is non-empty. The disjunction that let this case pass as a hub-key mismatch is gone. | MET (round-3 item 1b closed) |
| 8 | 2 remote + 1 local `say`, with and without `--queue` | `remote_and_local_say_concurrency_matches_local_only_story`: each contender exits 0 or exits 1 with `session_busy`; at least one exits 0; the session returns to idle; a fourth `say` exits 0. `remote_and_local_say_with_queue_all_three_succeed`: all three exit 0. | MET |
| 9 | `--server` on 7 verbs; cli-surface fixture | 7 new lines in `cli-surface.txt` (roster, say, interrupt, answer, wait, hub status, hub query). The pending file is empty. `cli_surface_test` is green in Tier 1. | MET |
| 10 | `--json` byte parity, masked by one helper | Three tests: `remote_roster_and_hub_status_json_match_local_after_masking`, `remote_say_wait_and_hub_query_json_match_local_after_masking`, and `remote_interrupt_and_answer_refusals_match_local`. **Changed at 237b444:** `masked()` (`remote_admin_test.rs:618`) gained a `"say"` arm that masks `elapsed_ms` and `message.messageId`, and the inline loop is gone. `masked` is now the only place the masked set is defined: roster `last_seen`/`last_update_at`, wait `age_secs`, hub status `lockout.peers[*].retry_after_secs`, say `elapsed_ms` plus `message.messageId`, hub query status none. `message.messageId` was added under the brief's extension rule; the evidence is `talk.rs:458`. A field missing on one side still fails, because `mask_field` only replaces keys that are present. | MET (round-3 item 4 closed) |
| 11 | Remote form needs no hub state | `remote_form_needs_no_hub_state` asserts `!body_state.hub().exists()` and exit 0. Every rig test also runs the remote CLI from the separate body `StateDir`. | MET |
| 12 | No-`--server` behaviour unchanged | The existing `roster_cli_test`, `talk_test`, `interrupt_test`, `answer_cli_test`, `wait_test` and `query_test` are not in the diff. They are green in the Tier 1 full workspace run. | MET |
| 13 | Failure words and exit codes | Exit 3 and exit 1 unreachable: `remote_server_policy_and_unreachable_failures_get_the_documented_exit_codes` (asserts exit 3; exit 1 plus `could not reach the hub at`). Missing credential: **tightened at 237b444**. `roster_remote_server_flag_is_not_yet_recognized` now asserts `code() == Some(1)` and that stderr contains both `credential.json` **and** `holler body join`, matching `AdminClientError::NotJoined`'s display (`admin_client.rs:61`). Hub-key mismatch: **new at 237b444**. `remote_admin_hub_key_mismatch_is_refused` corrupts the pinned `hub_pubkey` and asserts exit 1 and `"hub public key mismatch"`. That string is the body's own `refusal_attempt` wording for `noise_message_one_rejected` (`handshake.rs:171-172`), the same text `body run` prints. It does not overlap the wrong-key test. Ambiguous `say`: `remote_ambiguous_session_exits_2_matching_local` checks exit 2 on both paths and the same `error:` line. | MET (round-3 items 1a and 3 closed; ambiguous `hub query`, see Advisory) |
| 14 | Workspace tests, lint, clippy green; 900-line guard | Tier 1 was green before this audit. `wc -l`: circuit.rs 898, docs.rs 899, codec_test.rs 889, support/mod.rs 849, control_server.rs 829, remote_admin_test.rs 813, admin.rs 334, liveness test 211. All are below 900. | MET |
| 15 | v2.md: admin role, methods, posture, lockout shared fate | §3.2 covers the admin role, the permissive posture, allowlist and role separation, and liveness. It has a "Shared fate" paragraph covering the lockout bucket and `roster.clear`, and a revocation-reach paragraph. §4 has the seven rows, all "identical to `control/*`". §4's hello note says an admin hello has no harnesses or sessions. | MET |
| 16 | Admin-loop liveness (a) to (e) | `remote_admin_liveness_test.rs` has six tests (a, b, c, d, and e as clean close and abrupt drop). Each is read in full and each asserts the named behaviour: a Ping seen before the response; the response arrives after a silent 1.5 s against a 1 s timeout; an idle socket is closed and `admin_dropped` is logged; the response order is `[roster, say]` with ids; after a drop the session returns to idle with `last_turn`, and remote `roster` and `say` both exit 0. Unchanged since round 3. | MET |
| 17 | `admin_connected`/`admin_dropped` 4 fields; no `conn_connected` | `admin_connected_and_dropped_log_lines_carry_all_four_fields` asserts exactly one line of each, all four fields on both, and no `conn_connected`. | MET |

All four round-3 REWORK items are closed. For each one I checked the actual assertions at HEAD, not the T handoff's description of them.

## Spec compliance

- **Production code is unchanged since round 2**, which found it compliant. `git diff 032588c HEAD -- crates/` touches only the test file. I re-read the full production diff for this final audit.
  - MO 1: `hello_exchange` returns the role, and an unparsable hello defaults to `Body`. The branch comes before supersede, `registry.insert` and roster writes (`circuit.rs` +7 net lines, within the W-4 budget).
  - MO 2 and 3: the allowlist is `allowlisted_verb` plus the `admin/query` target split. It dispatches through one `dispatch_allowlisted`, which the Unix socket also uses. `revoke`, `test_drop`, `hold`, `release`, `caps` and `support` are unreachable.
  - MO 4: non-admin methods get `-32601`, including on decode failure with the id echoed. Notifications are dropped without touching the roster.
  - MO 5: requests are spawned; pings keep flowing; the liveness arm is gated on `inflight == 0`; a reply to a dropped client is discarded; `admin_connected`/`admin_dropped` carry 4 fields.
  - MO 6: the hub hello is unchanged.
  - MO 7: `admin_client` reuses `handshake::authenticate` unchanged. `hello_exchange` gained a role parameter, with one copy of the pinning. The identity is cloned and only `server_url` is overridden; nothing is written to disk.
  - MO 8: `ControlCall` exists, the public fns are thin wrappers, `transport::call` is the single dispatch point, and holler-hub has no holler-body dependency.
- **MO 9 deviation:** `ControlError` gained two variants (`RemotePolicyRefused`, which gives exit 3, and `RemoteUnavailable`) where MO 9 says one. This stays accepted. AC 13 needs exit 3 for the policy case separately from exit 1, and A-dup carries it as a warn. The `NoLiveHub` wording is unreachable in remote mode.
- **Flag shape:** `--server` only, with no `--token` or `--hub-key`, matching ADR 0020 at `b7a517c`. `docs/adr/ADR-0020.md` is not in the diff.
- **AC 10 refusal parity** compares only the trailing `error:` line. That exclusion is justified: the remote WS path's debug-tier `component: wire` lines have no local counterpart. Accepted in round 3 and still accepted.

## Quality audit

**Build guards:** the production `+` lines have no `unwrap()`, `expect(`, `panic!`, `unreachable!` or new `#[allow]` (checked with grep). The test files' `#![allow]` attributes carry `// #508` (and `// #149` / `// #185…` on the existing proto test files). No file is at or above 900 lines. The largest are docs.rs at 899 and circuit.rs at 898, which have no headroom (carried warn).

**Correctness / failure handling:** every remote failure maps to an existing rendering. A hub JSON-RPC error goes through `Refused`, so the busy, held and ambiguous handling is shared. Policy refusals exit 3. Connect, auth, identity and dropped failures exit 1 with plain words. `load_identity` refuses a missing X25519 key before dialling, so a half-joined state dir is never written. The admin loop's `unwrap_or_default` on encode fails closed: it sends an empty frame, it does not panic.

**Protocol:** the only change is additive: 7 catalog rows, `HelloRole::Admin`, and doc comments. There are no new error codes. The golden files are unchanged (not in the diff). `docs/protocol/v2.md` §3.2 and §4 are updated.

**Tests:**
- They run against a real hub and body, with separate `StateDir`s for each, as the Reuse map requires. The full-handshake initiator was moved into `raw_ws.rs` (`live_socket`, `go_live_as`), not copied.
- Synchronization uses `wait_for` on roster and log state.
- The fixed sleeps are the ones round 3 judged intrinsic to the behaviour under test: 2 s in AC 4, 1.5 s in 16(b) and 2 s in 16(c). The one synchronization sleep, 100 ms in 16(e), fails closed rather than passing falsely (advisory, carried).
- The round-3 additions assert specific exit codes and specific wording. None of them is a bare "it failed".

**Documentation:** `CHANGELOG.md` has an `## [Unreleased]` → Enhancements entry that links #508, #509, #506 and ADR 0020. Its "byte-identical `--json`" claim is backed by AC 10.

**Privacy:** I grepped the added lines for tailnet and host names, personal names, RFC 1918 and CGNAT ranges, and key or secret patterns. PR-Agent's review flagged a literal RFC 1918 address (`10.0.0.5`) in `remote_admin_test.rs`, which the round-3/round-4 audits had wrongly excused as "the brief's own placeholder" — a policy carve-out this repo's privacy rule does not grant. Fixed post-audit: both the test and the brief's matching prose now use `192.0.2.7` (IANA TEST-NET-1, RFC 5737 — reserved for documentation, never routable to a real host), consistent with the placeholder already used elsewhere in this diff. The only remaining hit is `hub.example.ts.net`, a non-resolving example hostname. Nothing is personal and there are no secrets.

**Commit and PR hygiene (advisory):**
- The subjects follow Conventional Commits (`chore(#508): …`, `test(#508): …`).
- The trailers are `Co-Authored-By: Claude <noreply@anthropic.com>` or `Claude Sonnet 5 <…>`, with no session link.
- No PR exists yet. When O opens it, it must add the `CONTRIBUTING.md` AI disclosure with `gh pr edit`, and the body must close #508 and #509 and reference #506.

## Scope check

In scope, with no over- or under-delivery:
- The seven verbs get `--server` and nothing else new.
- There is no hold/release, caps, support or token over the network.
- `body join`, `body run`, the Noise handshake and the token store are unchanged.
- ADR 0020 is untouched, and there is no deploy or migration doc (#510).

Accepted decompositions:
- `control_status.rs`: the brief's pre-agreed W-4 fallback move.
- `remote_admin_liveness_test.rs` and `support/remote_admin_rig.rs`: forced by the 900-line guard. The brief said "all in `remote_admin_test.rs`".

The round-3 delta is strictly the four named test fixes.

## Verdict

**PASS**

All 17 acceptance criteria have a proving test that asserts the behaviour. The four round-3 REWORK items are closed at 237b444, and I verified each against the code at HEAD:
- A hub-key-mismatch test now exists, and the wrong-key assertion is tightened. The two cases no longer overlap.
- The end-to-end AC 3 test asserts no `conn_superseded`.
- The missing-credential test asserts exit 1, `credential.json` and `holler body join`.
- `say` masking lives in the single `masked()` helper.

Production code is unchanged since round 2's compliant finding. Build, protocol, docs and privacy checks are clean, and scope matches the brief.

## Advisory notes

None of these block the merge.

- **AC 13, ambiguous `hub query` target:** there is still no remote end-to-end test. It is not required. #184's hostname-uniqueness enforcement prevents the collision end to end, as `query_test.rs:423-445` pins on the local side. The remote path reaches the same `is_ambiguous` arm through `Refused` (`hub_cmd.rs:190-193`). O may record this in `decisions.md`.
- **`body status --server` / `body query --server` parse and are silently ignored.** `Status` and `Query` are clap structs shared between `hub` and `body`, and F journalled this. It widens the accepted surface slightly beyond "only the seven verbs". A follow-up could reject `--server` on the `body` leaves.
- **Stale test prose:** `roster_remote_server_flag_is_not_yet_recognized` is still named for its RED-era failure mode, and the `remote_admin_test.rs` module doc still says "authored before any admin/`--server` production code exists". This is cosmetic; a rename is optional.
- **AC 7 wrong-key lockout** asserts `!peers.is_empty()`, not a specific reason key. That is sufficient in a fresh rig where this is the only refusal, but weaker than the unknown-token and revoked-token assertions.
- **AC 10:** only the refusals are compared for `interrupt`/`answer`. A success-path comparison would strengthen the byte-identical claim.
- **AC 8:** `wait_with_output` has no outer timeout, so a hang would stall the test instead of failing it. The `--queue` variant does not re-check "a fourth `say` succeeds".
- **AC 16(e):** the 100 ms pre-drop sleep is a synchronization sleep. A `wait_for` on the session's roster `state` leaving `idle` would be deterministic.
- **File headroom:** `remote_admin_test.rs` is at 813 lines (A-dup round-4 W-1), and circuit.rs (898) and docs.rs (899) have none. Further tests should go into the liveness file or a new file.
- **Carried from earlier rounds:**
  - A-dup W-1: `reply_decode_error` repeats `wire.rs`'s frame build.
  - `transport.rs`: `resolve_state_dir().unwrap_or_default()` falls back to an empty path.
  - The brief's "Handoff locations" line names `docs/handoffs/506-handoff-*.md`, but the run used `docs/handoffs/508/`. This is cosmetic.
  - Commit trailers have no session link. The PR's AI disclosure is still to be added.
- **Operator items outside this PR (from the brief):** the #509 wording edit and the #506/#511 acceptance-text edits (the deferred credential-separation and scope split) are GitHub issue edits for the operator.
