# Handoff-S: Phase 10 (round 2, after REWORK) - #508/#509 remote hub-admin client (epic #506), spec audit

**Date:** 2026-09-27
**Branch:** issue-508-implementation (at 7f8e39f)
**Issue:** #508 (together with #509, epic #506)
**Brief:** `docs/handoffs/506-brief.md`
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-F-rework.md`, `handoff-T-green-rework.md`, `handoff-A-dup.md` (round 2), `decisions.md`, `evidence.md`
**Diff audited:** `git diff origin/main...HEAD` in full. I read the rework delta `ac4e002..HEAD` line by line.

Supersedes the round-1 S handoff (REWORK at 75c411f, still in git history).

## A precondition

Met. `handoff-A.md` is **PASS** (8 warns) and `handoff-A-dup.md` round 2 is **PASS** (2 new warns plus the 4 round-1 warns). Neither has a block.

## T precondition

Met. `handoff-T-green-rework.md` shows "Blocking issues: None", all 6 `remote_admin_test` tests plus the new unit test GREEN, and the full workspace suite green (74 binaries). RED evidence for the new AC 5 wire test is a revert spot-check: with Q-1's fix reverted, the test hangs rather than failing. That is valid failure evidence, and there is an advisory note about it below. T states plainly that it did **not** do the full item-4 sweep.

## Acceptance criteria

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Seven `admin/*` catalog rows | `holler-proto/tests/codec_test.rs::catalog_has_all_seven_admin_rows`, `::admin_roster_method_decodes_once_catalogued`, `::every_method_round_trips` | MET |
| 2 | `HelloRole::Admin` is `"admin"` | `docs_wire_test.rs::admin_role_wire_test::hello_role_admin_deserializes` | MET (deserialize only, advisory) |
| 3 | No supersede, no roster hijack | `remote_admin_test.rs::remote_admin_hello_must_not_supersede_the_connected_body` (hand-rolled socket). The brief's end-to-end `remote_admin_does_not_supersede_the_body` (`roster --server` from the body state dir exits 0, then a later `say` succeeds) is still missing. | PARTIAL |
| 4 | Admin never creates or refreshes a roster row | `remote_admin_hello_must_not_create_a_roster_row_from_presence`. The "`last_seen` not moved" half is still not asserted. | PARTIAL |
| 5 | Allowlist and `-32601` | `circuit::admin::tests::allowlist_is_exactly_the_six_delegated_admin_verbs` (unit), `remote_admin_test.rs::admin_socket_refuses_every_non_admin_method_with_method_not_found` (`control/revoke`, `control/test_drop`, `session/prompt` on the wire, id echoed), and `::body_socket_sending_admin_roster_gets_method_not_found`. `admin/revoke`, `admin/hold` and `admin/release` are only unit-checked against the map, not sent on the wire, but they take the same decode-failure path as `control/revoke`. | MET |
| 6 | Hold parity (remote `say` to a held session) | **No test** | NOT MET |
| 7 | `-32002` and lockout parity on the admin path | **No test** | NOT MET |
| 8 | Concurrency: 2 remote + 1 local `say`, with and without `--queue` | **No test** | NOT MET |
| 9 | `--server` on 7 verbs; cli-surface fixture | `cli_surface_test`, `remote_admin_test.rs::roster_remote_server_flag_is_not_yet_recognized` | MET |
| 10 | `--json` byte parity (masking helper) | **No test.** The CHANGELOG still claims byte-identical output. | NOT MET |
| 11 | Remote form needs no hub state | **No test** | NOT MET |
| 12 | No-`--server` behaviour unchanged | Existing CLI suites unchanged and green | MET |
| 13 | Failure words and exit codes | Only not-joined, via the AC 9 test. Exit 3 policy refusal, unreachable exit 1, hub-key mismatch exit 1 and ambiguous exit 2 still have **no test**. | PARTIAL |
| 14 | Workspace tests, lint and clippy green | T-green-rework Tier 1 table. `wc -l`: admin.rs 333, circuit.rs 898, docs.rs 899, remote_admin_test.rs 351, all under 900. | MET |
| 15 | v2.md documents the admin role, the methods, the posture and the lockout shared fate | v2.md §3.1 is now contiguous (the lockout paragraph and its JSON example are back in §3.1). §3.2 follows. The W-6 revocation-reach paragraph is present, closing §3.2. §3.2's "-32601" and `admin_dropped` field claims are now true. | MET |
| 16 | Admin-loop liveness (a) to (e) | **No test** | NOT MET |
| 17 | `admin_connected`/`admin_dropped` carry 4 fields; no `conn_connected` | `remote_admin_test.rs::admin_connected_and_dropped_log_lines_carry_all_four_fields`. It checks "at least one" line of each, not "exactly one" (advisory). | MET |

## Spec compliance

- **Round-1 production findings are resolved.** Q-1: `circuit/admin.rs::handle_request` now calls `reply_decode_error` when a frame does not decode. The reply carries `e.code()` (`-32601` for `UnknownMethod`) and echoes the id when the id parses. `FrameOutcome::Rejected` does not refresh `last_frame_at`, which is the conservative option the round-1 verdict offered. Q-2: `admin_dropped` now logs `token_id, label, peer, sas`. Q-3: the v2.md sections are fixed.
- MO 1-5 and MO 7-8 are unchanged from round 1 and still compliant. The MO 9 two-variant deviation is accepted, as in round 1.
- The rework adds no new deviations. F-rework's handoff says the W-6 note went "at the end of §3.1". It actually closes §3.2, which is the right place because it describes admin sockets. The handoff's wording is inaccurate, but the spec is not affected.

## Quality audit

**Production code:** correct. `reply_decode_error` fails safe: a missing or invalid id falls back to an unkeyed `Envelope::Error`, and `encode` failure yields an empty send with no panic. The rework adds no `unwrap`, `expect` or `panic!` outside tests and no `#[allow]`. There is no dead code: all three `RequestOutcome` variants are used. No touched file is at or above 900 lines.

**Protocol:** no wire change beyond the round-1 additive rows. No error codes or golden files changed.

**Tests (new this round):** they use the real hub and the `raw_ws` harness, with `wait_for` polling and no fixed sleeps. They assert behaviour, not implementation: the wire code, the echoed id and the log fields. Each would fail if its fix were removed.

**Test coverage (the blocking item):** AC 6, 7, 8, 10, 11 and 16 have no test, and neither do the missing halves of AC 3, 4 and 13. This is exactly round-1 item 4, which the T re-entry scoped down on purpose and did not close. Under this role's rules, a criterion with no proving test is REWORK. The operator's "downgrade and proceed" covered the review-rigor tier (the outside-model brief gate). It does not waive acceptance criteria, and I do not read it that way.

**Documentation:** the `CHANGELOG.md` `## [Unreleased]` entry links #508, #509 and #506. Its "byte-identical `--json`" claim still has no AC 10 test.

**Privacy:** the rework delta adds no hostnames, tailnet names, private IPs or secrets. `192.0.2.7` is a TEST-NET documentation address in moved text. `hub.example.ts.net` and `10.0.0.5` are placeholders taken from the brief.

**Commit and PR hygiene:** same as round 1 (advisory). The `chore(#508): …` subjects follow Conventional Commits. The trailers lack a session link. There is no PR yet, so AI disclosure cannot be checked. O should settle both when it opens the PR.

## Scope check

In scope. The rework touched only `circuit/admin.rs` and `v2.md` (F) and the test files (T), with no unrelated changes. Under-delivery remains, and it is in tests only.

## Verdict

**REWORK** (TEST-ONLY: no `src/` change required)

All remaining items are test authoring. Round-1 items 1-3 are closed. As A-dup W-2 recommends, first extract a local `async fn live_socket(state, hub, token_id, label, role)` helper in `crates/holler-cli/tests/remote_admin_test.rs` (the identity-read and `go_live_as` setup now appears 5 times). Then add the following to `crates/holler-cli/tests/remote_admin_test.rs`:

1. **AC 3 end-to-end:** `remote_admin_does_not_supersede_the_body`. Run `holler roster --server <ws>` from the body's state dir and assert exit 0. Then assert that a later `say` to that body still gets a reply.
2. **AC 4:** assert that the body's roster `last_seen` does not move across admin traffic (take the value before and after an `admin/roster` and `admin/status`).
3. **AC 6:** a remote `say --server` to a held session gets the same outcome as the local `say` (use `hold_single_path_test`'s setup).
4. **AC 7:** an admin hello with an unknown token, a wrong key and a revoked token each gets `-32002`, and the lockout count increments exactly as on the body path.
5. **AC 8:** two remote plus one local `say` contend for one session, with and without `--queue`. Assert the brief's stated winner and loser outcomes. Synchronize on log or roster state, not sleeps.
6. **AC 10:** `--json` byte parity between local and `--server` for the 7 verbs, through one masking helper that masks exactly the brief's named fields. If this test fails, the CHANGELOG claim is false and F owns the fix.
7. **AC 11:** run the `--server` form with `HOLLER_STATE_DIR` set to the body dir, which has no hub state, and assert success.
8. **AC 13:** assert each of these exit codes and its stderr words:
   - exit 3 for `--server ws://10.0.0.5:1`
   - exit 1 for an unreachable server
   - exit 1 for a hub-key mismatch
   - exit 2 for an ambiguous session, matching the local form
9. **AC 16 (a) to (e):** admin-loop liveness per the brief, covering idle timeout, timeout held off while a request is in flight, and ping keepalive. Use each sub-case exactly as the brief lists it.

## Advisory notes

- `admin_socket_refuses_every_non_admin_method_with_method_not_found`: when the fix is reverted, the test hangs forever instead of failing. Wrap `decode_next` in `tokio::time::timeout` so a regression fails fast. Consider also sending `admin/revoke`, `admin/hold` and `admin/release` on the wire, since AC 5 names them.
- `allowlist_is_exactly_the_six_delegated_admin_verbs`: the name says "six" and the doc says "seven". Settle the wording: six mapped verbs plus the `admin/query` special case.
- AC 17 test: assert exactly one `admin_connected` and one `admin_dropped` line, as the brief says.
- AC 2: also assert that `HelloRole::Admin` serializes to `"admin"`.
- A-dup W-1: `reply_decode_error` duplicates `wire.rs::send_error_with_reason`'s frame build. A shared `error_frame_text` in wire.rs would remove the copy (optional follow-up).
- `transport.rs`: `resolve_state_dir().unwrap_or_default()` falls back to an empty path, carried over from round 1.
- docs.rs (899) and circuit.rs (898) have no headroom left.
