# Handoff-A-dup: Phase 7 - #508/#509 remote hub-admin client  (anti-duplication gate, round 4 after S round-3 REWORK)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Diff base:** daf6633 (full feature); rework delta 032588c..b8adc87   **Diff head:** b8adc87
**Reuse map:** docs/handoffs/506-brief.md §Files "Reuse map"
**Verdict:** PASS

This supersedes the round-3 gate, which passed at 09313a1/032588c and is still in git history. All earlier warns still stand, and this rework touched none of them. From round 1: `read_reply` compared with `next_envelope`, `send_ping` and the `circuit/ping` answer, the two `ControlError` variants, and the 898/899-line headroom in circuit.rs and docs.rs. From round 2: W-1, `reply_decode_error` copying `wire.rs`'s frame build. From round 3: W-1, `rig_with_env` having the same shape as `hold_rig::Rig`, and W-2, the per-file `run` wrapper. This pass reviews the S round-3 rework delta. That delta changes only one file, `crates/holler-cli/tests/remote_admin_test.rs`, and `git diff 032588c HEAD -- crates/*/src` is empty.

## Summary

PASS. The rework adds no production code and no new helpers. The new test `remote_admin_hub_key_mismatch_is_refused` is built from the existing shared rig (`two_dir_rig`, `mutate_credential`, `run`). It corrupts the real joined credential in place and does not add a second join path. The AC 3 body-log assertion uses the existing `Body::log_text()`. The AC 10 change removes a duplicate: the inline masking loop in `remote_say_wait_and_hub_query_json_match_local_after_masking` is gone, and the brief's single `masked` helper now has a `"say"` arm. That helper is still the only masking helper, as the brief requires. There is one new warn. `remote_admin_test.rs` is now 813 lines, past the ~800 line mark.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/remote_admin_test.rs` (813 lines) | The file grew from 779 to 813 lines. It is now above this stack's ~800-line flag, with 87 lines left before the `scripts/lint.sh` 900-line failure. | No change for this issue. Any further AC tests go into `remote_admin_liveness_test.rs` (211 lines) or a third file that uses `support/remote_admin_rig.rs`. They do not go into this file. |

No duplication, and the extension is clean. The rework introduced no architectural drift. Evidence:
- `masked` (`remote_admin_test.rs:618`) gains a `"say"` arm that reuses its existing `mask_field` closure. The masked set is `elapsed_ms` plus `message.messageId`, the same as before. The inline copy was deleted, so there are fewer masking sites than before.
- `remote_admin_hub_key_mismatch_is_refused` reuses `two_dir_rig` and `mutate_credential` (`support/remote_admin_rig.rs:19, 72`). It asserts the production wording `hub public key mismatch` from `handshake.rs` and does not re-implement the check.
- The tightened assertions in `roster_remote_server_flag_is_not_yet_recognized` and `remote_admin_wrong_key_is_refused_and_counted_in_lockout` only narrow existing assertions. They add no structure.
- `send_prompt`, `dispatch_allowlisted`, `circuit/admin.rs`, `circuit.rs` (898 lines) and `docs.rs` (899 lines) are all unchanged.

## Notes for F

None (PASS).

## Patterns referenced

- `crates/holler-cli/tests/support/remote_admin_rig.rs` (`two_dir_rig`, `mutate_credential`, `run`)
- `crates/holler-cli/tests/support/mod.rs` (`Body::log_text`, `StateDir`, `Hub`)
- `crates/holler-cli/tests/remote_admin_test.rs:618` (`masked`, the brief's single masking helper)
- Round-3 baseline: this file at 032588c
