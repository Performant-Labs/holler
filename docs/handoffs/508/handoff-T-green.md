# Handoff-T-green: Phase 6 - #508/#509 remote hub-admin client (epic #506)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Issue:** #508
**Handoff-F reviewed:** `docs/handoffs/508/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/508/handoff-T-red.md`

## Test repairs (F flagged these under "Tests that look wrong (for T)")

F wrote no tests, per role. I repaired the three test-file gaps F named — each was F's
production code landing correctly against a test whose own design anticipated needing an
update, not a regression:

1. **`crates/holler-proto/tests/codec_test.rs::every_method_round_trips`** — added the 7
   `admin/*` arms to `canonical_frame` (byte-for-byte the same params shape as the matching
   `control/*` form, per MO 2), using F's suggested literal frames verbatim. This initially
   pushed the file to 903 lines (over the 900-line guard); condensed the 7 new arms from
   3 lines each to 1 line each (matching this file's own single-line-arm style, e.g.
   `circuit/ping`) to bring it to 889 lines. `bash scripts/lint.sh` now exits 0 (was exit 1
   with the 903-line file).
2. **`crates/holler-cli/tests/fixtures/cli-surface.{pending,txt}`** — moved the 7 `--server`
   lines (one per admin-eligible verb) from `cli-surface.pending.txt` to `cli-surface.txt`,
   per the repo's own convention ("the story that adds the verb must move its line over").
   `every_pending_line_does_not_parse_yet` now passes against an empty (comment-only)
   pending set for this story's lines.
3. **`crates/holler-cli/tests/remote_admin_test.rs::roster_remote_server_flag_is_not_yet_recognized`**
   — repointed the second assertion at the post-AC-9 failure mode, exactly as the test's own
   docstring predicted ("must instead fail differently … never with a clap usage error").
   Renamed nothing (keeping the test's identity/AC-9 pin), rewrote its doc comment and body:
   it now asserts (a) the command still does not succeed, (b) the stderr no longer names
   `--server` as an unrecognized flag, and (c) the stderr names the missing body credential
   (`credential.json` / "not joined" / "run `holler body join`"). This is a test-file
   decision, not a production-code change.

## GREEN confirmation

```
$ cargo test -p holler-proto --test codec_test every_method_round_trips
test every_method_round_trips ... ok

$ cargo test -p holler-cli --test cli_surface_test every_pending_line_does_not_parse_yet
test every_pending_line_does_not_parse_yet ... ok

$ cargo test -p holler-cli --test remote_admin_test -- --test-threads=1
running 3 tests
test remote_admin_hello_must_not_create_a_roster_row_from_presence ... ok
test remote_admin_hello_must_not_supersede_the_connected_body ... ok
test roster_remote_server_flag_is_not_yet_recognized ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

All 5 tests I authored in Phase 4 (`admin_roster_method_decodes_once_catalogued`,
`catalog_has_all_seven_admin_rows`, `hello_role_admin_deserializes`,
`remote_admin_hello_must_not_supersede_the_connected_body`,
`remote_admin_hello_must_not_create_a_roster_row_from_presence`) are GREEN, plus the two
repaired existing tests (`every_method_round_trips`, `every_pending_line_does_not_parse_yet`)
and the repointed `roster_remote_server_flag_is_not_yet_recognized`.

**Spot-check (behavior, not implementation):** reverted `HelloRole::Admin`'s serde rename
locally and re-ran `hello_role_admin_deserializes` — it failed as expected (`unknown variant`),
confirming the test still fails if the behavior is removed. Reverted the change afterward
(not committed).

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Build | `cargo build --workspace` | Clean, PASS |
| Lint | `bash scripts/lint.sh` | Exit 0 (warn-only pre-existing file-size lines; `codec_test.rs` now 889, under the 900 guard) — PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok` — PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Clean, no warnings — PASS |
| Unused deps | `cargo machete` | Clean — PASS |
| Full workspace suite | `cargo test --workspace --no-fail-fast` | 74 test binaries, all `test result: ok`, 0 failed — PASS |
| docs CLI parity | `cargo test -p holler-cli --test docs_cli_test` | 3 passed — PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed — PASS |

Full-suite result differs from F's reported "830 passed, 3 failed" only because those 3
failures are exactly the three test-file gaps I repaired above; after the repair the same
full run is all-green (I did not re-count the exact total but confirmed zero `FAILED` and
zero `error[` lines across the entire `cargo test --workspace --no-fail-fast` output).

## Tier 2 results

- **Test coverage:** AC 1, 2, 3, 4, 9 each have a dedicated test, all GREEN. **AC 5, 6, 7, 8,
  10, 11, 13, 16, 17 have no dedicated test in this pass** — this is not new: my own
  Phase-4 handoff (`handoff-T-red.md`) named this gap explicitly before F ever wrote code
  ("Deliberately not attempted yet, and why each would not be a valid RED today"), and
  recommended a short RED-addition re-entry once the catalog/flag scaffolding existed to
  make those tests writable. F implemented all of AC 5-17 against the brief's written spec
  (allowlist, JSON parity by construction, admin-loop concurrency/liveness, failure wording)
  without a re-entry landing first, and flagged the same gap again in "Known issues." This
  is a real coverage hole — not a blocker on F's current code, because nothing failing points
  at wrong production behavior; it means AC 5-8/10/11/13/16/17 are implemented but
  unverified by an automated test. **Recommend O re-enter T for a dedicated RED-addition
  pass before S's Phase 10 spec audit**, per both my own and F's recommendation.
- **Test quality:** the 5 tests I authored each name one behavior (catalog decode, role wire
  shape, supersede hazard, roster-hijack hazard, CLI-argv rejection), sit at the cheapest
  tier that can observe that behavior (codec/wire unit tests for the wire facts; real
  hub+body+hand-rolled-socket integration tests for the two hazards, which have no cheaper
  substitute), and none duplicates another. No test in this pass needs deletion or merging —
  the suite added is proportionate to the 5 ACs it covers.
- **Type safety:** no `any`-equivalent casts (`as` casts checked); F introduced no new
  `#[allow(...)]` without a trailing `// #NNN` (checked `circuit/admin.rs`, `admin_client.rs`,
  `transport.rs`, `control.rs` diffs — none present).
- **Error handling:** `admin_client`'s `NotJoined`/reach/auth-failure paths and
  `transport.rs`'s loopback-policy/JSON-RPC-error mapping are exercised by
  `roster_remote_server_flag_is_not_yet_recognized` (the `NotJoined` path) and by the full
  existing `ControlError` rendering suite staying green (unchanged local paths). AC
  5/13's remaining failure-word/exit-code cases (wrong hub key, unreachable server, ambiguous
  session) are the coverage gap named above, not asserted yet.
- **Data integrity:** AC 3/4's two tests are exactly the data-integrity invariants this story
  exists to protect (no supersede, no roster row from admin presence) and both pass GREEN
  against F's implementation.
- **API contract:** `every_method_round_trips` now asserts all 7 `admin/*` frames round-trip
  byte-for-byte per MO 2's "identical to `control/*`" contract; `catalog_has_all_seven_admin_rows`
  pins the full 7-name catalog set.
- **Security:** the allowlist unit test (AC 5) and the wire-level `-32601` refusal cases are
  not yet written (same coverage gap). What is covered: the supersede/roster-hijack tests
  prove an admin-role hello cannot silently gain a body's live-connection state, which was
  this story's stated primary hazard (brief lines 18-19).
- **Migration safety:** N/A — no schema migration in this story (additive wire-only change,
  confirmed by F's Architecture notes and unchanged by my review).
- **Playwright:** N/A — this repo has no browser/Playwright surface (per this role's own
  stack notes).

## Acceptance criteria status

| AC | Status | Test |
|---|---|---|
| 1 (catalog + canonical frames) | PASS | `catalog_has_all_seven_admin_rows`, `every_method_round_trips` |
| 2 (`HelloRole::Admin` wire) | PASS | `hello_role_admin_deserializes` |
| 3 (no supersede) | PASS | `remote_admin_hello_must_not_supersede_the_connected_body` |
| 4 (no roster row from admin presence) | PASS | `remote_admin_hello_must_not_create_a_roster_row_from_presence` |
| 5 (allowlist exactness) | **No test yet** — coverage gap, named above and in both T-red/F handoffs | — |
| 6 (hold parity) | **No test yet** — coverage gap | — |
| 7 (lockout parity) | **No test yet** — coverage gap | — |
| 8 (concurrency, 3 contenders) | **No test yet** — coverage gap | — |
| 9 (`--server` on 7 verbs) | PASS | `cli_surface_test` (moved fixture lines), `roster_remote_server_flag_is_not_yet_recognized` |
| 10 (JSON parity) | **No test yet** — coverage gap | — |
| 11 (no hub state) | **No test yet** — coverage gap (existing AC 3/4 tests run against the body's state dir, so this is exercised incidentally but not asserted as its own criterion) | — |
| 12 (no-`--server` regression) | PASS | existing `roster_cli_test`/`talk_test`/`interrupt_test`/`answer_cli_test`/`wait_test`/`query_test`, all unchanged and green |
| 13 (failure words/exit codes) | Partial — the AC 9 policy-refusal + not-joined cases are covered; the remaining cases (unreachable server, hub-key mismatch, ambiguous session on remote) are **not yet tested** | `roster_remote_server_flag_is_not_yet_recognized` (partial) |
| 14 (workspace green/lint/clippy) | PASS | this handoff's Tier 1 results |
| 15 (docs) | Not T's to verify by test — doc content, S's Phase 10 concern | — |
| 16 (a-e, admin-loop liveness/concurrency) | **No test yet** — coverage gap | — |
| 17 (admin connection logs) | **No test yet** — coverage gap | — |

## Blocking issues

None. Nothing in this pass points at F's production code being wrong — every test that
exists and exercises implemented behavior is GREEN, and the full workspace regression suite
is clean. The AC 5-8/10/11/13/16/17 coverage gap is a missing-test problem (mine to fill in
a follow-up pass, not F's code to change) that both my Phase 4 handoff and F's Phase 6
handoff already named and recommended routing back to T for, before S's Phase 10 audit.

## Advisory notes

- Recommend O re-enter T for a dedicated RED-addition pass (AC 5, 6, 7, 8, 10, 11, 13, 16, 17)
  before S's spec audit, per both handoffs' own recommendation — S's Phase 10 will otherwise be
  auditing implemented behavior with no automated pin behind most of it.
- `codec_test.rs` is now 889 lines, 11 lines under the 900-line guard — a future admin-catalog
  addition to this file will need to decompose it (e.g. split `canonical_frame` into its own
  module) rather than grow it further inline.
