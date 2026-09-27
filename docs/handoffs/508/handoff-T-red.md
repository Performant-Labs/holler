# Handoff-T-red: Phase 4 - #508/#509 remote hub-admin client (epic #506)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Brief / wireframe reviewed:** `docs/handoffs/506-brief.md` (at 12a6681), `docs/handoffs/508/handoff-A.md` (PASS, 8 warns). No wireframe (`uiSurface: false`).

## A precondition

Confirmed: A returned **PASS** on the plan (Phase 3), `docs/handoffs/508/handoff-A.md`. None of A's 8 findings block T; W-1 (docs.rs line budget) and W-3 (`x25519_identity::ensure` side effect) are flagged there for F/T to handle during RED/GREEN — noted below, not yet actionable by T (they land in F's code, not in tests).

## Tests authored

All new tests compile against **current `main`** — none reference a not-yet-existing symbol (`HelloRole::Admin`, `admin_client`, `ControlCall`, `circuit::admin`, `--server`). Per this stack's own rule ("RED means the new test fails on an assertion about the missing behavior, not on a compile error"), every RED below is driven at the wire/CLI boundary (raw JSON frames, subprocess argv, a hand-rolled WS socket), never by calling a symbol that doesn't exist yet.

| Test | File | AC pinned | Tier | Why this tier |
|---|---|---|---|---|
| `admin_roster_method_decodes_once_catalogued` | `crates/holler-proto/tests/codec_test.rs` | AC 1 | unit (codec) | Pure decode of a hand-built frame; no process needed — cheapest tier for a catalog/decode fact. |
| `catalog_has_all_seven_admin_rows` | `crates/holler-proto/tests/codec_test.rs` | AC 1 | unit (codec) | Same reason; pins the full seven-name set the brief's MO 2 names, not just one. |
| `admin_role_wire_test::hello_role_admin_deserializes` | `crates/holler-proto/tests/docs_wire_test.rs` | AC 2 | unit (wire shape) | `Hello`'s hand-written round trip through `serde_json`, no socket needed. |
| `roster_remote_server_flag_is_not_yet_recognized` | `crates/holler-cli/tests/remote_admin_test.rs` | AC 9 | integration (CLI subprocess) | Needs the real `clap` parser via the built binary; a unit test can't observe argv parsing. |
| `every_pending_line_does_not_parse_yet` (existing test, new fixture lines) | `crates/holler-cli/tests/fixtures/cli-surface.pending.txt` | AC 9 (all 7 verbs) | integration (CLI subprocess) | Reused the repo's own pending/promoted-fixture mechanism (`cli_surface_test.rs`) instead of writing a second CLI-surface test — one `--server` line per verb (`roster`, `say`, `interrupt`, `answer`, `wait`, `hub status`, `hub query`), each currently failing to parse for the missing `--server` flag alone (verified the rest of each line's grammar against the existing `cli-surface.txt` entries for that verb). |
| `remote_admin_hello_must_not_supersede_the_connected_body` | `crates/holler-cli/tests/remote_admin_test.rs` | AC 3 | integration (real hub + body + hand-rolled admin socket) | The supersede hazard only exists across two real live connections sharing a token; nothing cheaper observes it. |
| `remote_admin_hello_must_not_create_a_roster_row_from_presence` | `crates/holler-cli/tests/remote_admin_test.rs` | AC 4 | integration (real hub + hand-rolled admin socket) | Roster-row creation is a hub-side side effect of a live WS frame; needs a real hub. |

**Test infra change (no behavior change):** moved `send_authenticate`/`run_hello`/`go_live` out of `hub_hygiene_test.rs` into `crates/holler-cli/tests/support/raw_ws.rs` as `pub`, giving `run_hello`/`go_live` a `role` parameter (`run_hello_as`/`go_live_as`) — the brief's own Clarifications ask for exactly this move ("do not write a third copy of the initiator"). `hub_hygiene_test.rs`'s 14 existing tests all still pass unmodified after the move (verified below). File sizes: `raw_ws.rs` 259 lines, `hub_hygiene_test.rs` 662 lines — both well under the 900-line guard.

## AC coverage in this RED pass, and what's deferred to a later RED addition

Covered now: AC 1, 2, 3, 4, 9 (the brief's own "Test plan: RED first" list, plus the wire-level catalog/role facts AC 1–2 add).

**Deliberately not attempted yet, and why each would not be a valid RED today:**
- **AC 5 (allowlist)** — sending `admin/revoke` or `control/test_drop` over the hub's WS *already* gets `-32601` today, for the trivial reason that no `admin/*` namespace and no WS-level `control/*` routing exist at all yet. A test asserting `-32601` here would pass immediately — not a valid RED (this file's role doc: "A test that passes before the feature exists... is not a valid RED"). This needs the seven `admin/*` methods to exist first (as an allowlist that *excludes* these names) to be a meaningful pin; writing it now would just be deleted and rewritten in Phase 6.
- **AC 6 (hold)**, **AC 7 (lockout parity)**, **AC 8 (concurrency)**, **AC 10 (JSON parity)**, **AC 11 (no hub state needed)**, **AC 12 (no-`--server` regression — already covered by the untouched existing suite passing)**, **AC 13 (failure words/exit codes)**, **AC 16 (admin-loop liveness a–e)**, **AC 17 (admin connection logs)** all exercise behavior of `--server`/`admin_client`/the admin loop that has no wire-level or CLI-argv-level substitute — every attempt to pin them without the transport/admin_client code compiling would either not compile (invalid RED per this stack's rule) or silently pass against the *local* path instead of the remote one.

**Recommendation to O:** re-enter T (a short RED-addition pass) once F's first commit lands the seven `admin/*` catalog rows, `HelloRole::Admin`, and the CLI `--server` flag scaffolding (even before the admin loop's full behavior exists) — at that point AC 5's allowlist test becomes writable as a real RED, and the AC 10 parity harness (JSON masking helper) and AC 16/17 liveness tests become at least compilable against `admin_client`. Flagging this now rather than silently shipping partial coverage as if it were complete.

## RED confirmation

```
cargo test -p holler-proto --test codec_test admin_roster_method_decodes_once_catalogued -- --nocapture
```
```
thread 'admin_roster_method_decodes_once_catalogued' panicked at crates/holler-proto/tests/codec_test.rs:...:
AC 1: `admin/roster` must decode as a known method once the seven `admin/*` catalog rows exist; on current main this fails with UnknownMethod("admin/roster") — that is this RED's failure: UnknownMethod("admin/roster")
```

```
cargo test -p holler-proto --test codec_test catalog_has_all_seven_admin_rows -- --nocapture
```
```
thread 'catalog_has_all_seven_admin_rows' panicked at crates/holler-proto/tests/codec_test.rs:...:
AC 1: CATALOG must contain `admin/status` (found: ["circuit/join", ... 15 existing rows ...]) — not catalogued yet on current main
```

```
cargo test -p holler-proto --test docs_wire_test hello_role_admin_deserializes -- --nocapture
```
```
thread 'admin_role_wire_test::hello_role_admin_deserializes' panicked at crates/holler-proto/tests/docs_wire_test.rs:...:
AC 2: `role: "admin"` must deserialize into `Hello` once `HelloRole::Admin` exists; on current main this fails because `admin` is not a known `HelloRole` variant: Error("unknown variant `admin`, expected `body` or `hub`", line: 0, column: 0)
```

```
cargo test -p holler-cli --test cli_surface_test every_pending_line_does_not_parse_yet -- --nocapture
```
```
test every_pending_line_does_not_parse_yet ... ok   (the 7 new --server lines correctly do NOT parse yet)
```

```
cargo test -p holler-cli --test remote_admin_test -- --test-threads=1
```
```
running 3 tests
test roster_remote_server_flag_is_not_yet_recognized ... ok

test remote_admin_hello_must_not_create_a_roster_row_from_presence ... FAILED
thread panicked at crates/holler-cli/tests/remote_admin_test.rs:199:
AC 4: an admin-role connection must never create a roster row from `session/presence`, but one appeared:
{"rows":[{"name":"admin-only/phantom", ..., "conn_state":"connected", ...}]}

test remote_admin_hello_must_not_supersede_the_connected_body ... FAILED
thread panicked at crates/holler-cli/tests/remote_admin_test.rs:122:
AC 3: an admin-role hello must never supersede the connected body, but the body's log shows `conn_superseded`:
... {"type":"conn_connected", ...}
    {"type":"session/presence"}
    {"type":"circuit/superseded"}
    {"type":"conn_superseded"}
    {"type":"mailbox_dequeue","name":"alpha","command":"shutdown"}

test result: FAILED. 1 passed; 2 failed
```

Each failure is on the **feature assertion** (the wrong-behavior symptom: a real `-32602`-free `UnknownMethod`, a real unknown-serde-variant error, a real `conn_superseded` log line, a real extra roster row) — never a compile error, an unresolved import, or a missing `[[test]]` entry. `remote_admin_test` **is** declared in `crates/holler-cli/Cargo.toml` (required since `holler-cli` sets `autotests = false`).

**Full-suite regression check** (nothing else broke from the `raw_ws.rs` move or the two new fixture/test files): `cargo test --workspace` — every pre-existing test still passes; only the 5 new RED assertions above fail (2 in `remote_admin_test`, 2 in `codec_test`, 1 in `docs_wire_test`). `bash scripts/lint.sh` exits 0 (only pre-existing size warnings, none new above the 900-line gate). `cargo clippy --workspace --all-targets -- -D warnings` is clean. `cargo machete` finds nothing unused. `cargo test -p holler-cli --test docs_cli_test`, `--test wire_selftest`, and `bash scripts/changelog-check.sh` all pass.

## Ready for F

**Confirmed RED is valid.** F may implement against these tests. F's first slice should prioritize the catalog rows + `HelloRole::Admin` + the branch point (making `codec_test`/`docs_wire_test`/AC 3–4 turn GREEN) and the `--server` CLI flags (turning the `cli-surface.pending.txt` lines into ones that must move to `cli-surface.txt`) — after which O should re-enter a short T pass to add the AC 5/10/16/17 tests this handoff deferred (see above).

## Notes for F (not architecture, just orientation)

- Reuse `support::raw_ws::go_live_as`/`run_hello_as` (role parameter) for any further hand-rolled-socket test you or T add — don't add a third copy of the handshake initiator.
- `remote_admin_test.rs` reads the body's persisted credential directly (`holler_body::x25519_identity::identity_path`), matching the brief's MO 7 ("the same x25519 identity + token, not a new credential") — your `admin_client` module should load it the same way.
