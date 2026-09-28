# Handoff-F: Phase 6 (rework round 1) - #508/#509 remote hub-admin client (epic #506)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Issue:** #508

## What was done

This is a rework round, not a fresh implementation pass: it addresses the three
production/doc items from `handoff-S.md`'s **REWORK** verdict (S's Phase 10 spec audit).
S's item 4 (new tests for AC 5/6/7/8/10/11/13/16/17) is not mine — it is test-only and
belongs to T's own re-entry (see "Tests that look wrong (for T)" below).

- `crates/holler-hub/src/circuit/admin.rs` — **Q-1**: `handle_request` no longer silently
  drops a frame `holler_proto::decode` fails to parse. It now returns a 3-way
  `RequestOutcome` (`Spawned`/`Inline`/`Rejected`) instead of `bool`; on a decode failure, a
  new `reply_decode_error` helper replies with an error frame carrying the decode error's own
  `e.code()` (`-32601` for the `UnknownMethod` case S's five named methods hit) and the
  request `id` when the raw JSON has one. `FrameOutcome` gained a matching `Rejected`
  variant so `run`'s own `select!` loop does not refresh `last_frame_at` for an undecodable
  frame. **Q-2**: the `admin_dropped` log line now carries `label` and `sas`, matching
  `admin_connected`'s own four fields two lines above.
- `docs/protocol/v2.md` — **Q-3**: moved `### 3.2 Remote admin connections` so it comes
  after the whole of §3.1 (the lockout paragraph and its JSON example no longer sit inside
  §3.2), and added a new paragraph at the end of §3.1 — A's Phase 3 finding #6 ("Revocation
  reach"), which recommended stating it next to the lockout text — that a revoked
  credential's already-open `admin/wait`/`admin/say` runs to completion, because `control/revoke`
  never reaches an admin socket.
- `docs/handoffs/508/decisions.md` — appended this round's phase entry.
- `docs/handoffs/508/evidence.md` — appended two facts this fix relies on that live in
  unchanged source (see "Evidence appendix").

## Design decisions

- **Liveness: chose "do not refresh" over "document why it does."** S's item 1 offered
  either option. Not refreshing is the simpler, more conservative choice — it does not let
  malformed/untrusted input count as proof a legitimate client is still there — and needs no
  persuasive doc comment justifying the alternative.
- **`reply_decode_error` recovers the request `id` itself, rather than always sending an
  unkeyed error like `circuit.rs:812-817` does.** The body loop's own decode-failure path
  (`send_error(self.sink, None, e.code(), &e.to_string())`) always passes `None`. I read S's
  "carrying the request id when the raw JSON has one" as a real requirement, not just "mirror
  circuit.rs literally" — a well-formed request naming an uncatalogued method (e.g.
  `control/revoke`) still carries a valid id, and an admin client has no other way to
  correlate that reply with its pending request. `decode` already failed by this point, so
  there is no `Envelope` to read an id off; the fix does a plain best-effort `serde_json`
  read of the raw text for an `id` string, and only builds a keyed `Envelope::error_frame`
  when that string also parses as a valid `CorrelationId` — otherwise it falls back to an
  unkeyed `Envelope::Error`, the same fallback `wire.rs::send_error_with_reason` already uses
  (see Evidence appendix).
- **`Envelope::Error { id, error }` constructed directly, not through a new constructor.**
  `Envelope`'s doc comment says its variants have "private fields on purpose," but
  `holler-hub`'s own `wire.rs` (a different crate than `holler-proto`) already builds this
  exact struct literal for the same unkeyed-error case. I reused that proven pattern rather
  than adding a new `Envelope` constructor for a one-call need, or reaching into
  `control_server.rs`'s private `unkeyed_error_line` helper (which is `fn`, not `pub(crate)`,
  and lives in an unrelated module).
- **A new `RequestOutcome` enum, not a widened `bool`.** `handle_request` needed a third
  outcome (`Rejected`) distinct from the existing `Spawned`/`false`-means-inline split, and
  `FrameOutcome` (the caller's own outcome type, one level up) already uses the same
  small-enum-per-decision-point style in this file, so `RequestOutcome` matches the local
  convention rather than introducing a different shape.

## Reuse / extend-vs-new

No new production object. This rework extends the same `crates/holler-hub/src/circuit/admin.rs`
module F's first pass created for issue #508 (per the brief's Reuse map, admin.rs is itself the
one new object the brief called for) and the same `docs/protocol/v2.md` section that pass added.
The only new items are two small private helpers inside that existing module
(`RequestOutcome`, `reply_decode_error`) and one enum variant (`FrameOutcome::Rejected`) — not
new modules, not new public surface.

## Architecture notes for A

None. No new module boundaries, no changed public interfaces (`RequestOutcome` and
`reply_decode_error` are both private to `circuit/admin.rs`; `FrameOutcome`'s new `Rejected`
variant is `enum` without visibility modifiers, same as its existing variants, already private
to this module), no dependency-direction change, no schema/contract change (the wire behavior
change — an undecodable frame now gets a reply instead of silence — is a bug fix to match the
spec `v2.md` already stated, "anything else … is `-32601`," not a new contract). Reporting
`archChanged: false` below on that basis.

## Deviations from spec / wireframe

None. All three fixes match handoff-S.md's verdict items 1-3 as written (file, line, and
required behavior).

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo build --workspace
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.52s

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.28s
(clean — no warnings; one doc_lazy_continuation lint hit during development from a doc-comment
 line shaped like a markdown list item ("1) or spawn…") — fixed by rewording the comment, not
 by adding an #[allow])

$ cargo machete
cargo-machete didn't find any unused dependencies in this directory. Good job!

$ bash scripts/lint.sh; echo "EXIT: $?"
(warn-only file-size lines, unchanged from F's first pass — e.g. "warn: crates/holler-hub/src/circuit.rs
 is 898 lines" — admin.rs itself is not in the warn list: 222 lines before this rework, 288 after,
 well under 900)
EXIT: 0

$ bash scripts/changelog-check.sh
changelog-check: ok
```

**Existing suite, now exercising the fixed path (no dedicated RED for Q-1/Q-2 yet — that is
S's item 4, routed to T):**
```
$ cargo test -p holler-cli --test remote_admin_test -- --test-threads=1
running 3 tests
test remote_admin_hello_must_not_create_a_roster_row_from_presence ... ok
test remote_admin_hello_must_not_supersede_the_connected_body ... ok
test roster_remote_server_flag_is_not_yet_recognized ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.87s
```

**Full-workspace regression check** (`cargo test --workspace --no-fail-fast`): 74
`test result: ok` lines, zero `FAILED`, zero `error[` — no regression from this rework
(matches T-green's prior 74-binary-green baseline).

## Evidence appendix

`docs/handoffs/508/evidence.md`, section "Rework round 1 (F, Phase 6 redo — handoff-S REWORK
item 1)" — 2 new facts, each `file:line` + verbatim excerpt: `wire.rs::send_error_with_reason`'s
existing `Envelope::Error { id, error }` struct-literal construction (proves this is legal from
`holler-hub`, a different crate than `holler-proto`, before I relied on it in
`reply_decode_error`); `EnvelopeError::code()`'s `UnknownMethod -> Code::MethodNotFound` mapping
(the `-32601` the fix's reply now carries is this unchanged mapping, not something the diff
defines).

## Tests that look wrong (for T)

None. S's item 4 (new tests for AC 5, 6, 7, 8, 10, 11, 13, 16, 17) is not a wrong test — it is
tests that do not exist yet. Per the routing rule this role doc and pipeline-conventions.md §3
both state (`reworkKind: test-only` goes to T, `production` to F), S's REWORK verdict is mixed:
items 1-2 are production-code fixes and item 3 is a doc fix (mine, done above); item 4 is
test-authoring (T's). I made no test-file changes and staged none.

## Known issues

Same known gaps handoff-F.md and handoff-T-green.md already named and this rework does not
close: AC 5 (allowlist unit test), AC 6 (hold parity), AC 7 (lockout parity), AC 8
(concurrency), AC 10 (JSON parity), AC 11 (no-hub-state), AC 13 (remaining exit-code cases),
AC 16 (a-e liveness/concurrency), AC 17 (connection-log field assertions) have no dedicated
test yet — this is exactly S's item 4, recommended for a T re-entry before the next S/diff-gate
audit. The two correctness defects Q-1/Q-2 those missing tests would have caught are now fixed
in production code by this rework; only the assertions themselves are still missing.

## Files changed

- `crates/holler-hub/src/circuit/admin.rs`
- `docs/protocol/v2.md`
