# Handoff-S: Phase 10 - #508/#509 remote hub-admin client (epic #506), spec audit

**Date:** 2026-09-27
**Branch:** issue-508-implementation (at ac4e002)
**Issue:** #508 (together with #509, epic #506)
**Brief:** `docs/handoffs/506-brief.md`
**Handoffs reviewed:** `handoff-A.md` (Phase 3), `handoff-T-red.md` (Phase 4), `handoff-F.md` (Phase 6), `handoff-T-green.md` (Phase 6), `handoff-A-dup.md` (Phase 7), `decisions.md`, `evidence.md`
**Diff audited:** `git diff origin/main...HEAD` (40 files, +2201/-277)

## A precondition

Met. `handoff-A.md` (plan review) is **PASS** with 8 warns, and `handoff-A-dup.md` (anti-duplication gate) is **PASS** with 4 warns. Neither has a block.

## T precondition

Met, formally. `handoff-T-red.md` shows a valid RED for AC 1, 2, 3, 4 and 9: each test failed on the feature assertion, not on a compile error. `handoff-T-green.md` shows the same tests GREEN, the full workspace green and "Blocking issues: None". Both T handoffs and F's handoff also say plainly that AC 5-8, 10, 11, 13 (partly), 16 and 17 have **no test**. They recommended a T re-entry before this audit, and that re-entry did not happen. This audit treats that gap as a finding, not as a precondition failure.

## Acceptance criteria

| AC | Criterion (short) | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Seven `admin/*` catalog rows with canonical frames | `holler-proto/tests/codec_test.rs::catalog_has_all_seven_admin_rows`, `::admin_roster_method_decodes_once_catalogued`, `::every_method_round_trips` (7 new arms) | MET |
| 2 | `HelloRole::Admin` is `"admin"` and round-trips | `holler-proto/tests/docs_wire_test.rs::admin_role_wire_test::hello_role_admin_deserializes`. It covers deserialize only. T's spot-check reverted the rename and the test failed. | MET (serialize half only implied) |
| 3 | No supersede, no roster hijack | `holler-cli/tests/remote_admin_test.rs::remote_admin_hello_must_not_supersede_the_connected_body`. This uses a hand-rolled admin socket. The brief's own test (`remote_admin_does_not_supersede_the_body`, which runs `roster --server` from the body's state dir, expects exit 0, and then has a later `say` succeed) is not written. | PARTIAL |
| 4 | Admin never creates or refreshes a roster row | `remote_admin_test.rs::remote_admin_hello_must_not_create_a_roster_row_from_presence`. The "`last_seen` not moved by admin traffic" half is not asserted. | PARTIAL |
| 5 | Allowlist: `-32601` for non-admin methods on an admin socket; body socket `admin/roster` gets `-32601`; unit test that the allowlist is exactly 7 names | **No test.** The code is also wrong here (see Quality audit Q-1). | NOT MET |
| 6 | Hold parity (remote `say` to a held session) | `hold_single_path_test` is unmodified and green. The remote-`say`-to-held-session parity has **no test**. | PARTIAL |
| 7 | `-32002` and lockout parity on the admin path | **No test** | NOT MET |
| 8 | Concurrency: 2 remote + 1 local `say` | **No test** | NOT MET |
| 9 | `--server` on 7 verbs; cli-surface fixture | `cli_surface_test` (7 lines moved to `cli-surface.txt`), `remote_admin_test.rs::roster_remote_server_flag_is_not_yet_recognized` (repointed) | MET |
| 10 | `--json` byte parity with a masking helper | **No test.** F's claim of "parity by construction" is not evidence. | NOT MET |
| 11 | Remote form needs no hub state | **No test** as its own criterion | NOT MET |
| 12 | No-`--server` behaviour unchanged | Existing `roster_cli_test`, `talk_test`, `interrupt_test`, `answer_cli_test`, `wait_test`, `query_test` are unchanged and green (T-green Tier 1) | MET |
| 13 | Failure words and exit codes (exit 3 policy; not-joined; unreachable; hub-key mismatch; ambiguous = exit 2) | Only the not-joined case, via the repointed AC 9 test. The exit 3 policy refusal, unreachable server, key mismatch and ambiguous session have **no test**. | PARTIAL |
| 14 | Workspace tests, lint (900-line guard) and clippy green | T-green Tier 1 table. `wc -l`: the largest touched files are docs.rs 899 and circuit.rs 898, both under 900. | MET |
| 15 | v2.md §3/§4 documents the admin role, the 7 methods, the permissive posture and the lockout shared fate | v2.md §3.2 plus 7 catalog rows plus the hello note. The content is present, but the section is placed so that it breaks §3.1, and it states behaviour the code does not have (see Q-3). | PARTIAL |
| 16 | Admin-loop liveness (a) to (e) | **No test** | NOT MET |
| 17 | `admin_connected`/`admin_dropped` each carry `token_id, label, peer, sas`; no `conn_connected` | **No test.** The code is also wrong here (see Q-2). | NOT MET |

The brief assigns AC 3-8, 10-13, 16 and 17 to `remote_admin_test.rs`, and assigns "unit tests in `circuit/admin.rs` (allowlist)". `circuit/admin.rs` has no `#[cfg(test)]` module.

## Spec compliance

- **MO 1-4 (role, catalog, allowlist, branch point):** the branch point is implemented as stated. `circuit.rs` `handle_authenticated` returns into `admin::run` before `conn_connected`, supersede, `registry.insert` and any roster write. The allowlist dispatches only to the existing handlers through `dispatch_allowlisted`, so `send_prompt` remains the single prompt path. `admin/query` is split by `params.target` as MO 2 and A's W-2 require.
- **MO 4 "anything else is -32601":** **deviation.** See Q-1.
- **MO 5 (liveness, concurrency, drop mid-say):** the design matches. Each request runs on its own spawned task, the liveness timeout is gated on `inflight == 0`, and replies go through an mpsc channel. Nothing verifies it (AC 16).
- **MO 7 (same identity, never writes the state dir):** matches. `admin_client::load_identity` checks the X25519 key file before dialling (A W-3).
- **MO 8 (`ControlCall` + one transport switch):** matches. The public `control::*` signatures are unchanged.
- **MO 9 (one `ControlError` variant):** F added **two** (`RemotePolicyRefused` for exit 3 and `RemoteUnavailable` for exit 1). A-dup W-3 flagged this. It is a small, justified deviation because AC 13 needs a separate exit-3 arm. I accept it and do not count it as a finding. O should record it in decisions.md.
- **ADR 0020 not edited:** complied.
- **v2.md (MO docs, AC 15):** see Q-3.

## Quality audit

**Q-1 (correctness, production): non-catalogued methods on an admin socket are silently dropped, so there is no `-32601` and the client hangs.** In `crates/holler-hub/src/circuit/admin.rs:178`, `let Ok(env) = holler_proto::decode(text) else { return false };` discards any frame that does not decode. `control/*` names are **not** in `CATALOG` (methods.rs:52-83), and neither are `admin/revoke`, `admin/hold` or `admin/release`. So five of the six methods AC 5 names (`control/revoke`, `control/test_drop`, `admin/revoke`, `admin/hold`, `admin/release`) fail to decode and get **no reply at all**. A client waits until its own timeout. The same frame also refreshes `last_frame_at`, so it keeps the connection alive. The body loop in the same file family handles this case by sending `send_error(sink, None, e.code(), …)` (circuit.rs:812-817), and `EnvelopeError::UnknownMethod` maps to `Code::MethodNotFound` (envelope.rs:184). That reply is the "decode `UnknownMethod`" AC 5 allows. v2.md §3.2 promises "anything else … is `-32601`". Only `session/prompt` (catalogued) currently gets the `-32601`.

**Q-2 (correctness, production): `admin_dropped` is missing `label` and `sas`.** At `crates/holler-hub/src/circuit/admin.rs:119` it logs only `token_id` and `peer`. AC 17 and v2.md §3.2 both require `{token_id, label, peer, sas}` on **each** of `admin_connected`/`admin_dropped`.

**Q-3 (documentation): v2.md §3.2 is inserted in the middle of §3.1.** The new `### 3.2` heading (v2.md:187) comes before the paragraph "It also carries the **live** lockout state …" (v2.md:231) and the rest of §3.1's lockout text. "It" referred to `control/status`'s reply (v2.md:185). It now reads as part of "Remote admin connections", so the lockout subsection is split in two. v2.md §3.2 also states "anything else … is `-32601`" and describes the `admin_dropped` fields, and neither is true until Q-1 and Q-2 are fixed.

**Q-4 (test coverage): the ACs with no proving test**, listed in the table above: AC 5, 7, 8, 10, 11, 16 and 17 in full, and the missing halves of AC 3, 4, 6 and 13. Under this role's rules, a criterion with no proving test is REWORK. The two correctness defects above (Q-1, Q-2) are exactly what the missing AC 5 and AC 17 tests would have caught.

**Build guards:** no new `unwrap()`, `expect(` or `panic!` in production code, and no new `#[allow]` (checked with a diff grep). No touched file is at or above 900 lines (docs.rs 899, circuit.rs 898, codec_test.rs 889, control_server.rs 829). No dead code found. `dispatch_allowlisted`'s `None` arm is reachable only in principle, and its doc says so.

**Protocol:** the changes are additive (a new enum variant and 7 new catalog rows). There are no new error codes and existing golden files are unchanged. The v2.md catalog table is updated.

**Tests:** the tests that exist use the real hub and body harness (`support::{StateDir, Hub}`, `raw_ws::go_live_as`). The initiator was moved into `raw_ws.rs`, not copied. RED-first evidence is recorded for the tests that exist.

**CHANGELOG:** there is an `## [Unreleased]` entry that links #508, #509 and #506. It claims "`--json` output is byte-identical to the local form", which is untested (AC 10).

**Privacy:** the diff grep found only `hub.example.ts.net` (a placeholder in the cli-surface fixtures, taken from the brief). There are no personal hostnames, tailnet names, IPs, account names or secrets.

**Commit and PR hygiene:** the subjects follow Conventional Commits (`chore(#508): …`). The `Co-Authored-By` trailers have **no session link**, and one trailer names a different model. There is no PR yet, so AI disclosure cannot be checked. O should fix the trailers and add the disclosure when it squashes or opens the PR. This is advisory and does not block.

## Scope check

Scope is right. The `dispatch_allowlisted` refactor of the Unix-socket arms is what the brief's Files entry asks for. The `control_status.rs` verbatim move is the pre-agreed W-4 fallback. The `--server` flag also parsing, inert, on `body status`/`body query` comes from the shared clap structs the brief counts, and F explained it. F added no unrelated refactors or features. The shortfall is test delivery and the two defects above.

## Verdict

**REWORK** (production, with test additions)

1. `crates/holler-hub/src/circuit/admin.rs:178`: when `holler_proto::decode(text)` fails, reply with an error frame instead of returning silently. Use `Envelope::error_frame` with the decode error's `e.code()` (`-32601` for `UnknownMethod`), carrying the request `id` when the raw JSON has one, the same way the body loop does at circuit.rs:812-817. The admin socket can stay open. Do not refresh liveness for an undecodable frame, or document why it does. This makes AC 5's `control/revoke`, `control/test_drop`, `admin/revoke`, `admin/hold` and `admin/release` cases observable.
2. `crates/holler-hub/src/circuit/admin.rs:119`: add `("label", record.label.clone())` and `("sas", sas.to_string())` to the `admin_dropped` log line (AC 17).
3. `docs/protocol/v2.md:187-229`: move `### 3.2 Remote admin connections` so it comes after the whole of §3.1. The paragraph that starts "It also carries the **live** lockout state" (line 231) and the rest of §3.1 must stay together under §3.1. Also add A's W-6 note: a revoked credential's already-open admin request runs to completion, because `control/revoke` does not close admin sockets.
4. Tests, all in `crates/holler-cli/tests/remote_admin_test.rs` unless noted, per the brief's Files and Test plan (T authors them; they must fail without items 1 and 2):
   - AC 5: wire refusals for all six named methods on an admin socket, a body-role socket sending `admin/roster` gets `-32601`, and a `#[cfg(test)]` unit test in `circuit/admin.rs` that the allowlist is exactly the seven names.
   - AC 3 end-to-end (`remote_admin_does_not_supersede_the_body`: `roster --server` exits 0 from the body state dir and a later `say` still replies). AC 4's `last_seen` not moved.
   - AC 6 remote `say` to a held session matches local; AC 7 `-32002` plus the lockout count for an unknown token, a wrong key and a revoked token; AC 8 the three-contender concurrency case, with and without `--queue`.
   - AC 10 JSON parity with the single masking helper and the exact masked-field set; AC 11 (run with `HOLLER_STATE_DIR` set to the body dir, asserted).
   - AC 13 exit 3 for `ws://10.0.0.5:1`, unreachable exit 1 with its words, hub-key mismatch exit 1, ambiguous-session exit 2 matching local.
   - AC 16 (a) to (e), and AC 17 (exactly one `admin_connected` and one `admin_dropped` with all four fields, no `conn_connected`).

## Advisory notes

- `crates/holler-cli/src/transport.rs:222`: `resolve_state_dir().unwrap_or_default()` falls back to an empty path, which is relative to the current directory. The failure is still closed, because it reports "not joined" with a relative path. A clear "cannot resolve state dir" error would be cleaner.
- AC 2: consider also asserting that `HelloRole::Admin` serializes to `"admin"`. The current test covers deserialize only.
- docs.rs (899 lines) and circuit.rs (898 lines) have no headroom. The next story that touches either file should do the pre-agreed moves first (A-dup W-4).
- MO 9's two-variant deviation (A-dup W-3) is accepted here. O should record it in decisions.md.
- The CHANGELOG's byte-identical `--json` claim should stay only once the AC 10 tests exist.
