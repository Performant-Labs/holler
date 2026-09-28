# Handoff-A-dup: Phase 7 - #508/#509 remote hub-admin client  (anti-duplication gate, round 3 after S round-2 REWORK)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Diff base:** daf6633 (full feature); rework delta 7f8e39f..09313a1   **Diff head:** 09313a1
**Reuse map:** docs/handoffs/506-brief.md §Files "Reuse map"
**Verdict:** PASS

This supersedes the round-2 gate, which passed at 7f8e39f and is still in git history. The round-1 warns still stand, and the rework touched none of them: `read_reply` compared with `next_envelope`, `send_ping` and the `circuit/ping` answer, the two `ControlError` variants, and the 898/899-line headroom in circuit.rs and docs.rs. Round-2 W-1 (`reply_decode_error` copying `wire.rs`'s frame build) also stands untouched, as an optional follow-up. Round-2 W-2 (identity-read setup repeated five times) is **resolved**. This pass reviews the S round-2 rework delta. That delta is test-only apart from one test rename and a doc-comment fix in `circuit/admin.rs`.

## Summary

PASS. The rework adds no production code. The only `src/` change is a rename of the `#[cfg(test)]` allowlist test, plus its doc comment, in `circuit/admin.rs`. T extracted `live_socket` into `support/raw_ws.rs`, as round-2 W-2 asked, and all 10 call sites in both test files now use it. There is no copy left. The new `support/remote_admin_rig.rs` builds on the shared harness (`Hub::start_with_env`, `mint_token`, `join`, `write_sessions_toml`, `Body::start`, `wait_for`, `roster_json`, `holler_cmd`) and copies none of them. It also reuses the production loaders `holler_body::identity::load`/`BodyIdentity::path` and `x25519_identity::identity_path`. The AC 16 liveness tests went into a second test file because of the 900-line guard, and that file shares the same rig instead of copying it. One new warn: the rig has the same shape as `hold_rig::Rig::start_with`, and sharing `hold_rig::Rig` was not possible (details in finding 1).

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/support/remote_admin_rig.rs:26-43` (`rig_with_env`) | It follows the same setup sequence as `support/hold_rig.rs:133-149` (`Rig::start_with_args`): two `StateDir`s, start the hub, `mint_token(.., "b")`, `join`, `write_sessions_toml`, start the body, wait for each session to be idle. Both call the same shared primitives, so neither is a near-copy of a listed harness helper. Reusing `hold_rig::Rig` was not possible. It keeps `addr` and `hub` (a raw `Child`, not a `support::Hub`) private, and the new tests need a `Hub` for `ws_url()`, `log_text()`/`log_events` and `live_socket`. The two rigs now run side by side, so this is a warn and not a block. | No change needed for this issue. Optional follow-up: have `hold_rig::Rig` wrap a `support::Hub`, or expose `ws_url()` on `Rig`, and then implement `two_dir_rig` on top of it. |
| 2 | warn | `crates/holler-cli/tests/support/remote_admin_rig.rs:46-67` (`run`, `spawn_cmd`) | `run` is the 8th spawn-and-`wait_with_output` wrapper around `holler_cmd` in this crate's tests. Six test files already have their own local `fn run`. There is no single dominant shared helper here (`support::cmds` has only verb-specific wrappers), so this follows the existing pattern and is not drift. It is at least shared between the two new files and not copied. | None required. A crate-wide `support::run` would be a separate cleanup, outside this scope. |

No parallel path, and the extension is clean. The rework introduced no architectural drift:
- `send_prompt`, `dispatch_allowlisted`, `circuit.rs` (898), `docs.rs` (899) and every other `src/` file are unchanged apart from the admin.rs test rename.
- `live_socket` now reads the pinned `hub_pubkey` from the joined credential instead of `hub_x25519_pubkey(state)`, which fixes a latent bug that appears when hub and body state live in separate directories. It reuses the production `BodyIdentity` loader and does not parse the file itself.
- `mutate_credential` changes the real joined credential in place. It adds no second join path.
- AC 7 checks lockout through the existing `hub status --json` `lockout.peers` table. It does not copy `Lockout`.
- The only masking helper is `masked` in `remote_admin_test.rs`, as the brief requires.
- The new `[[test]]` entry in `Cargo.toml` follows the crate's `autotests = false` convention.
- File sizes: `remote_admin_test.rs` 779, `remote_admin_liveness_test.rs` 211, `remote_admin_rig.rs` 78, `raw_ws.rs` 289, `admin.rs` 334. All are under 900. `remote_admin_test.rs` is close to the ~800 line mark, so any further AC tests should go into the liveness file or a third file.

## Notes for F

None (PASS).

## Patterns referenced

- `crates/holler-cli/tests/support/hold_rig.rs:23-35, 123-149` (`Rig`, `Rig::start_with_args`)
- `crates/holler-cli/tests/support/mod.rs` (`StateDir`, `Hub`, `Body`, `holler_cmd`, `wait_for`, `write_sessions_toml`)
- `crates/holler-cli/tests/support/onboard.rs` (`mint_token`, `join`)
- `crates/holler-cli/tests/support/raw_ws.rs` (`connect_ws`, `go_live_as`, `live_socket`, `log_events`)
- Round-2 baseline: this file at 7f8e39f
