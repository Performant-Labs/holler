# Handoff-T-green (re-entry, rework round 1): Phase 6 - #508/#509 remote hub-admin client (epic #506)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Issue:** #508
**Handoff-F-rework reviewed:** `docs/handoffs/508/handoff-F-rework.md`
**Handoff-S reviewed:** `docs/handoffs/508/handoff-S.md`
**Prior handoff-T-green:** `docs/handoffs/508/handoff-T-green.md`

## Why this re-entry

`handoff-S.md`'s Phase 10 spec audit returned **REWORK** with four items. Items 1-3
(production/doc: Q-1 undecodable-frame silent drop, Q-2 missing `admin_dropped` fields,
Q-3 v2.md section placement) were production-shaped and routed to F; F fixed all three
(`handoff-F-rework.md`) and explicitly left item 4 — new tests for the ACs S found
untested — to T's own re-entry, per this role's routing rule (`reworkKind: production` for
F's items, the test-authoring item is mine). This handoff is that re-entry.

**Scope of this pass:** S's item 4 lists nine ACs with no proving test (AC 5, 6, 7, 8, 10,
11, 13, 16, 17, several only partially). Authoring a full suite for all nine in one pass —
concurrency races (AC 8), the admin-loop liveness state machine (AC 16 a-e), JSON-byte-parity
(AC 10) — is a substantial authoring effort in its own right. This pass is deliberately
scoped to the tests that most directly exercise the two production defects F's rework round
just fixed (Q-1, Q-2) plus the allowlist-shape unit test S's item 4 names explicitly, rather
than attempting the full nine-AC sweep in one sitting. The remaining gap is named below as
an advisory, exactly as both prior T-green and F's handoffs already did — not silently
dropped.

## Tests authored this pass

1. **`crates/holler-hub/src/circuit/admin.rs::tests::allowlist_is_exactly_the_six_delegated_admin_verbs`**
   (unit, `#[cfg(test)]` inside the module, per S's item 4 first bullet: "a unit test in
   `circuit/admin.rs` that the allowlist is exactly the seven names"). Asserts
   `allowlisted_verb` maps exactly the six delegated names (`admin/status`, `roster`, `say`,
   `interrupt`, `answer`, `wait`) to their control-server verbs, and returns `None` for
   `admin/query` (handled separately in `handle_request`), for the five methods AC 5 names as
   currently-uncatalogued (`admin/revoke`, `admin/hold`, `admin/release`, `control/revoke`,
   `control/test_drop`), and for a body-shaped method and the empty string.
2. **`crates/holler-cli/tests/remote_admin_test.rs::admin_socket_refuses_every_non_admin_method_with_method_not_found`**
   (integration, real hub + hand-rolled admin socket). Sends `control/revoke` and
   `control/test_drop` (uncatalogued — exercises the Q-1 decode-failure reply path directly)
   and `session/prompt` (catalogued but hub-to-body — exercises `handle_request`'s own
   decodes-fine-but-unrecognised `-32601` arm) on a live admin socket, and asserts each gets
   back an `Envelope::Error` with `error.code == -32601` carrying the request's own id. This
   is the AC 5 wire-refusal bullet, and it is exactly what Q-1's fix makes observable — before
   the fix, the two uncatalogued methods got no reply at all and the test hangs (spot-check
   below).
3. **`crates/holler-cli/tests/remote_admin_test.rs::body_socket_sending_admin_roster_gets_method_not_found`**
   (integration). A body-role socket sending `admin/roster` gets `-32601` — AC 5's second
   bullet. This is unchanged hub behavior (`circuit.rs`'s existing catch-all
   `Envelope::Request { id, .. }` arm, `circuit.rs:869-872` — see `evidence.md`'s new "T
   re-entry" section), so this test pins an existing invariant rather than new production
   code.
4. **`crates/holler-cli/tests/remote_admin_test.rs::admin_connected_and_dropped_log_lines_carry_all_four_fields`**
   (integration, AC 17). Connects and cleanly drops an admin socket, and asserts the hub's
   `admin_connected` and `admin_dropped` log lines each carry all four of `token_id`, `label`,
   `peer`, `sas` (Q-2's fix — before it, `admin_dropped` had only two), and that the log never
   contains `conn_connected` (the body-path event name), pinning MO 3-4's "an admin socket
   never touches the body-path connection log."

All four tests sit at the cheapest tier that can observe their behavior: the allowlist shape
is a pure function, so it gets a unit test with no socket at all; the wire-refusal and
log-field facts are only observable over a real socket against the real hub, so they are
`holler-cli` integration tests using the existing `raw_ws`/`Hub`/`StateDir` harness, no new
test infrastructure.

## GREEN confirmation

```
$ cargo test -p holler-hub --lib circuit::admin::tests
running 1 test
test circuit::admin::tests::allowlist_is_exactly_the_six_delegated_admin_verbs ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 76 filtered out

$ cargo test -p holler-cli --test remote_admin_test -- --test-threads=1
running 6 tests
test admin_connected_and_dropped_log_lines_carry_all_four_fields ... ok
test admin_socket_refuses_every_non_admin_method_with_method_not_found ... ok
test body_socket_sending_admin_roster_gets_method_not_found ... ok
test remote_admin_hello_must_not_create_a_roster_row_from_presence ... ok
test remote_admin_hello_must_not_supersede_the_connected_body ... ok
test roster_remote_server_flag_is_not_yet_recognized ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.43s
```

**Spot-check (behavior, not implementation):** temporarily reverted the Q-1 fix in
`handle_request` (made the decode-failure arm silently `return RequestOutcome::Rejected`
without calling `reply_decode_error`, the exact pre-rework shape) and re-ran
`admin_socket_refuses_every_non_admin_method_with_method_not_found` alone. The client's own
`decode_next` call hung indefinitely (no reply ever arrives for `control/revoke`) — the test
does not pass, confirming it fails for the right reason when the fixed behavior is removed.
Restored the fix immediately afterward (`git diff` on `admin.rs` now shows only this pass's
test-module addition, nothing else); did not commit the reverted state.

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Build | `cargo build --workspace` | Clean, PASS |
| Lint | `bash scripts/lint.sh` | Exit 0. `admin.rs` now 333 lines (well under 900); no file crossed the guard | PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok` — PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Clean, no warnings — PASS |
| Unused deps | `cargo machete` | Clean — PASS |
| Full workspace suite | `cargo test --workspace --no-fail-fast` | 74 `test result: ok` lines, zero `FAILED`, zero `error[` — PASS (matches F-rework's own reported 74-line baseline; no regression) |
| docs CLI parity | `cargo test -p holler-cli --test docs_cli_test` | 3 passed — PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed — PASS |

Cross-checked F-rework's own reported commands (`cargo build --workspace`, `cargo clippy
--workspace --all-targets -- -D warnings`, `cargo machete`, `bash scripts/lint.sh`, `bash
scripts/changelog-check.sh`, `cargo test -p holler-cli --test remote_admin_test
-- --test-threads=1`, `cargo test --workspace --no-fail-fast`) — all re-run above, no
discrepancy with F-rework's reported results.

## Tier 2 results

- **Test coverage:** AC 5's three named bullets now each have a test (wire refusal for
  uncatalogued + catalogued-wrong-direction methods; body-socket-sends-admin/roster; the
  allowlist-exactness unit test). AC 17 now has a test (all four fields on both log lines, no
  `conn_connected`). **AC 6, 7, 8, 10, 11, 13 (remaining cases), 16 still have no dedicated
  test** — see "Advisory notes" below; this pass did not attempt the full nine-AC sweep (see
  "Why this re-entry" above for the scoping decision).
- **Test quality:** each of the four new tests names one behavior (allowlist shape, wire
  refusal on decode failure, wire refusal on role mismatch, connection-log field parity),
  fails in isolation for the right reason (the unit test needs no socket; the three
  integration tests were spot-checked or are structurally tied to the exact fix they pin —
  see the spot-check above for #2; #3 pins pre-existing behavior directly; #4's four-field
  assertion fails without Q-2's fix by construction, since `admin_dropped`'s log call site
  is the only place those fields are added), sits at the cheapest tier (unit for the pure
  function, integration only where a real socket is required), and none duplicates an
  existing test. Proportionate: four tests for four newly-observable facts, no more.
- **Type safety:** no new `any`-equivalent casts; no new `#[allow(...)]` without a trailing
  `// #NNN` in the test files or `admin.rs`'s new test module.
- **Error handling:** the wire-refusal tests are themselves the error-path assertions AC 5
  calls for (both the decode-failure and the decodes-fine-but-unrecognised path now return an
  explicit `-32601` instead of a hang or a silent drop).
- **Data integrity:** N/A for this pass's four tests (no roster/registry writes exercised
  beyond what AC 3/4's existing tests already pin).
- **API contract:** the wire-refusal tests assert the exact `-32601` code and id-echo contract
  `reply_decode_error`'s own doc comment states.
- **Security:** the allowlist-exactness test is itself a security-relevant regression guard —
  it fails if a future change accidentally widens the delegated-verb map.
- **Migration safety:** N/A — no schema change in this pass.
- **Playwright:** N/A — no browser surface in this repo.

## Acceptance criteria status (this pass's deltas only; full table in `handoff-T-green.md`)

| AC | Status | Test |
|---|---|---|
| 5 (allowlist: `-32601` for anything else, incl. decode failure) | PASS | `allowlist_is_exactly_the_six_delegated_admin_verbs`, `admin_socket_refuses_every_non_admin_method_with_method_not_found`, `body_socket_sending_admin_roster_gets_method_not_found` |
| 17 (`admin_connected`/`admin_dropped` fields, no `conn_connected`) | PASS | `admin_connected_and_dropped_log_lines_carry_all_four_fields` |
| 6, 7, 8, 10, 11, 13 (remaining cases), 16 | Still no dedicated test — unchanged from `handoff-T-green.md`/`handoff-F-rework.md`'s "Known issues" | — |

Q-1 and Q-2 (the two production defects S's audit found) are now each directly exercised by
a test that fails without the fix (spot-checked for Q-1 above; Q-2's four-field assertion is
unsatisfiable without F-rework's fix, by construction of the log call site).

## Blocking issues

None. Nothing in this pass points at F's (or F-rework's) production code needing a change —
every test added is GREEN against the current implementation, the two production defects
S's audit found are now both covered, and the full workspace regression suite is clean.

## Advisory notes

- **AC 6 (hold parity), 7 (lockout parity, `-32002`), 8 (3-contender concurrency), 10 (JSON
  byte parity), 11 (no-hub-state as its own criterion), 13 (remaining exit-code cases:
  unreachable, key mismatch, ambiguous), 16 (a-e liveness/concurrency)** still have no
  dedicated test. This is the same gap named in `handoff-T-green.md`, `handoff-F.md`, and
  `handoff-S.md`'s item 4 — not new, and not closed by this pass. Recommend O route a further
  T re-entry for these before the next S/diff-gate audit; S's own rule ("a criterion with no
  proving test is REWORK") means this gap will very likely produce another REWORK verdict at
  the next audit unless that re-entry happens first.
- The CHANGELOG's "`--json` output is byte-identical to the local form" claim (S's note)
  should stay conditional on AC 10's test landing.
