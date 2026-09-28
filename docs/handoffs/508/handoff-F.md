# Handoff-F: Phase 6 - #508/#509 remote hub-admin client (epic #506)

**Date:** 2026-09-27
**Branch:** issue-508-implementation
**Issue:** #508

## What was done

Production files created/modified (grouped by crate):

**holler-proto** (wire types/catalog):
- `crates/holler-proto/src/docs.rs` — `HelloRole::Admin` variant (wire `"admin"`); updated the two-role doc comments (the enum's own doc, the `Hello.role` field doc, and `Status`'s doc) to reflect the third role.
- `crates/holler-proto/src/methods.rs` — the seven `admin/*` catalog rows (`admin/status`, `admin/roster`, `admin/say`, `admin/interrupt`, `admin/answer`, `admin/wait`, `admin/query`), each `Request`/`BodyToHub`; updated the module doc's row/direction count and `Direction::BodyToHub`'s doc line.

**holler-body** (the admin client):
- `crates/holler-body/src/connection/handshake.rs` — `hello_exchange` gains a `role: HelloRole` parameter (visibility widened `pub(super)` → `pub(crate)` so `admin_client` can call it); an admin hello sends `harnesses: None, sessions: None` (MO 7) instead of the body's harness list. `authenticate` is unchanged, per the brief.
- `crates/holler-body/src/connection.rs` — updated `hello_exchange`'s one call site to pass `HelloRole::Body` explicitly.
- new `crates/holler-body/src/admin_client.rs` — the one-shot admin client: loads this process's own `BodyIdentity` (never mints/persists anything), checks the X25519 identity key exists *before* dialling (A's Phase 3 finding W-3 — `handshake::authenticate` would otherwise generate and persist a fresh key into a half-joined state dir), dials `--server`, runs `handshake::authenticate` unchanged, runs `handshake::hello_exchange` with `HelloRole::Admin`, sends one `admin/*` request, reads its reply (or the hub's JSON-RPC error), closes cleanly.
- `crates/holler-body/src/lib.rs` — `pub mod admin_client;`.

**holler-hub** (the admin loop + allowlist):
- `crates/holler-hub/src/circuit.rs` — `hello_exchange` now also returns the peer's claimed `HelloRole` (defaulting to `Body` if the hello didn't parse, preserving today's fallthrough); `handle_authenticated` branches to the new admin loop immediately after the pre-auth-permit release and *before* `conn_connected`, the supersede check, `registry.insert`, or any roster write (AC 3-4) — `mod admin;` declared alongside the existing `auth`/`dispatch` submodules.
- new `crates/holler-hub/src/circuit/admin.rs` — the admin session loop (MO 5): WS pings keep flowing and the liveness timeout stays armed but is disabled while any request is in flight (AC 16 a/b/c); each `admin/*` request is dispatched on its own spawned task (A's Phase 3 finding #4) so a 600s `admin/say` never blocks a concurrent `admin/roster` (AC 16d) and a client drop mid-request is harmless (AC 16e — the task keeps running, its reply is dropped once the loop's own receiver is gone); `circuit/ping` is answered directly; anything else is `-32601`; logs `admin_connected`/`admin_dropped` with `{token_id, label, peer, sas}`. Never touches `Registry`'s body-slot bookkeeping or `Roster` (A's Phase 3 finding #6).
- `crates/holler-hub/src/control_server.rs` — new `pub(crate) dispatch_allowlisted(verb, cid, obj, registry, roster, lockout)`: the one allowlisted-verb entry (`status`/`roster`/`say`/`interrupt`/`answer`/`wait`/`query_local`/`query_remote`) both the admin loop and the Unix control socket now call — `dispatch_control`'s and `dispatch_session_control`'s matching `control/*` arms were refactored to delegate through it (behaviour-preserving: each arm now calls the exact same handler fn it always called, just through the shared entry) rather than staying a second, parallel implementation. `dispatch_session_control` gained a `lockout` parameter to reach the `status` arm through the shared entry (only relevant to that one verb; the others ignore it). `status_doc`/`read_listening` moved out to `control_status.rs` (see below) to stay under the 900-line file guard once this addition landed.
- new `crates/holler-hub/src/control_status.rs` — `status_doc`/`read_listening`, moved verbatim out of `control_server.rs` (the brief's own pre-agreed W-4 fallback), now `pub(crate)`.
- `crates/holler-hub/src/control.rs` — `ControlCall { method, params, timeout }` (MO 8) with one constructor per remote-eligible verb (`status`/`roster`/`say_with`/`interrupt`/`answer`/`wait`/`query_local`/`query_remote`); the matching public fns are now thin wrappers around a call site's own `ControlCall`, unchanged signatures/behaviour. Two new `ControlError` variants: `RemotePolicyRefused` (exit 3, a `--server` loopback-policy refusal) and `RemoteUnavailable` (exit 1, MO 9 — a remote reach/auth failure that isn't a hub-side JSON-RPC error). New `pub fn run(&ControlCall)` — the generic local-exchange entry `holler-cli`'s `transport` module calls.
- `crates/holler-hub/src/lib.rs` — `pub mod control_status;`.

**holler-cli** (the `--server` flag + transport switch):
- `crates/holler-cli/src/cli.rs` — `--server: Option<String>` added to `Roster`, `Say`, `Interrupt`, `Answer`, `Wait`, `Status`, `Query` (the brief's "7 structs" — `Status`/`Query` are shared between `hub`/`body`, so `body status`/`body query` also parse the flag; `body_cmd.rs` never reads it, so it is inert there, matching "extend the shared struct, don't fork a hub-only copy").
- new `crates/holler-cli/src/transport.rs` — the one dispatch point (MO 8): `call(server: Option<&str>, &ControlCall)` runs `holler_hub::control::run` locally when `server` is `None`, or (checking the loopback policy first, exit-3 on a plaintext non-loopback address — the same rule `body join` applies) rewrites `control/x` → `admin/x` (the two query forms collapsing onto `admin/query`, MO 2) and calls `holler_body::admin_client::call` on a throwaway current-thread runtime (mirrors `holler_body::join::join`'s own "the CLI is not a tokio runtime" pattern). Maps a hub-side JSON-RPC error back onto `ControlError::Refused` unchanged (MO 9) so every existing refusal rendering below is shared between local and remote.
- `crates/holler-cli/src/{roster,say,interrupt,answer,wait,hub}_cmd.rs` — each routed through `crate::transport::call` with its own `ControlCall` constructor and the caller's `.server` field, plus one added match arm for `ControlError::RemotePolicyRefused` (exit 3). `hub_cmd::status` gained a `server: Option<&str>` parameter.
- `crates/holler-cli/src/main.rs` — `HubCommand::Status(_)` → `HubCommand::Status(status)`, passing `status.server.as_deref()` through (previously ignored the whole struct, since `Status` was empty).
- `crates/holler-cli/src/lib.rs` — `pub mod transport;`.

**Docs:**
- `docs/protocol/v2.md` — new §3.2 "Remote admin connections" (the admin role, the branch point, the shared-fate lockout/roster risk statement AC 15 requires); seven new catalog rows in §4 (`admin/*`, "identical to `control/x`"); one sentence added to the Hello-document subsection noting the admin hello's shape.
- `CHANGELOG.md` — `## [Unreleased]` → `### Enhancements` entry linking #508/#509/#506.

## Design decisions

- **`dispatch_allowlisted` refactors the Unix socket's own dispatch, not just the admin loop's.** The brief's Files entry for `control_server.rs` explicitly asks for one entry "used by both the Unix socket and the admin loop" — I read this as a spec'd refactor-to-extend (not a drive-by), and did it as a pure, behaviour-preserving delegation (each `control/*` arm now calls the exact fn it always called, through one more layer) rather than leaving a second, parallel implementation for A's anti-duplication gate to catch.
- **`admin/query`'s local-vs-remote split lives in `circuit/admin.rs`, not `dispatch_allowlisted`.** The Unix socket already has two separate wire methods (`control/query_local`/`control/query_remote`); `dispatch_allowlisted` keeps that same two-verb shape (`"query_local"`/`"query_remote"`), and only the admin loop (which has one wire method, `admin/query`, per MO 2) decides which of the two to call, by checking `params.target`'s presence — the same check `transport::admin_method`'s doc explains from the CLI side. This avoids adding target-sniffing logic to the shared entry point that the Unix socket path would never exercise.
- **Every request on an admin socket is its own spawned task**, per A's Phase 3 finding #4 (`registry`/`roster`/`lockout` are cheap to clone — see evidence.md). I considered a simpler sequential loop (award each request to completion before reading the next frame) but that fails AC 16(a)/(d) directly: a `600s admin/say` would block the WS ping arm and a concurrent `admin/roster` on the same socket.
- **The admin loop's liveness timeout is gated on an `inflight` counter, not a fixed grace window.** MO 5 says "never timed out while a request is in flight"; a fixed grace window would still time out a slow `say`. Tracking how many spawned tasks haven't replied yet (incremented on spawn, decremented when a reply arrives) is the minimal state that satisfies "never" literally.
- **`ControlError` gained two variants inside `holler-hub`, even though only `holler-cli` ever constructs them.** This keeps every `*_cmd.rs` refusal rendering keyed on one error type (per MO 9's own framing) without `holler-hub` depending on `holler-body` — the variants only ever carry a `String`, never a `holler-body` type.
- **`Status`/`Query`'s `--server` field is shared with `body status`/`body query`, unused there.** Splitting them into hub-only/body-only struct pairs would be a bigger, unplanned refactor of the clap tree the brief did not ask for; the brief's own Files entry counts "7 structs" including `Status`/`Query`, which are these exact shared types.

## Reuse / extend-vs-new

Per the brief's Reuse map: extended `control.rs`'s client fns and `control_server.rs`'s dispatcher (no new roster/say/wait handlers — the admin loop calls `dispatch_allowlisted`, which calls the *existing* `say`/`interrupt`/`answer`/`roster_control`/`wait`/`hub_query_local`/`hub_query_remote`/`status_doc` verbatim); extended `hello_exchange` on both sides with a role parameter (no second hub-key-pinning copy); extended `server_address::{parse, loopback_only_check}` (called from `transport.rs`, not re-implemented); extended the existing `*_cmd.rs` render/exit-code logic (no new rendering path). `authenticate_and_hello`/`connection::handshake::authenticate` are unchanged, as the brief required. No new object was created where the brief named an existing one to extend.

## Architecture notes for A

- **Layers touched:** proto (catalog + wire enum), hub (branch point + new admin submodule + dispatch refactor + client-facing `ControlCall`/`ControlError` additions), body (hello role param + new `admin_client` module), cli (new flag + new `transport` dispatch module + 7 routed leaves).
- **Dependency direction:** unchanged — `holler-hub` still does not depend on `holler-body`; `transport.rs` (in `holler-cli`, which already depends on both) is the only place that imports both `holler_hub::control` and `holler_body::admin_client`.
- **New public surface:** `holler_proto::docs::HelloRole::Admin`; 7 new `holler_proto::methods::CATALOG` rows; `holler_hub::control::{ControlCall, ControlError::{RemotePolicyRefused, RemoteUnavailable}, run}`; `holler_hub::control_server::dispatch_allowlisted` (`pub(crate)`); `holler_hub::control_status::{status_doc, read_listening}` (`pub(crate)`); `holler_body::admin_client::{call, AdminClientError}`; `holler_body::connection::handshake::hello_exchange`'s widened visibility and new `role` parameter; `holler_cli::transport::call`.
- **No schema/contract break:** every new wire field/method is additive (docs §3-4), no protocol version bump, matching the ADR 0014 precedent A's own Phase 3 review already checked.
- **Local pattern followed:** `AdminDeps` (in `circuit/admin.rs`) bundles `Registry`/`Arc<Roster>`/`Arc<Lockout>` the same way `AuthDeps`/`CommandChannels` already bundle shared connection state elsewhere in this same module family, purely to stay under clippy's argument-count gate.

## Deviations from spec / wireframe

None from the brief's decisions (MO 1-9) or the Clarifications. One mechanical accommodation: `crates/holler-cli/src/cli.rs`'s own inline `#[cfg(test)] mod tests` constructs a `Query { rest: ... }` literal; adding `Query::server` required adding `server: None` to that one struct literal so the crate still compiles under `cfg(test)` — no test assertion changed, this is not a `tests/*.rs` file. Flagged here rather than silently folded into "what was done" since it is technically a change inside a `#[cfg(test)]` block.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo build --workspace
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.46s

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.44s
(clean — no warnings)

$ cargo machete
cargo-machete didn't find any unused dependencies in this directory. Good job!

$ bash scripts/lint.sh; echo "EXIT: $?"
(warn-only file-size lines, e.g. "warn: crates/holler-hub/src/circuit.rs is 898 lines",
 "warn: crates/holler-proto/src/docs.rs is 899 lines" — both under the brief's own tight
 W-4 budget, neither >= 900)
EXIT: 0

$ bash scripts/changelog-check.sh
changelog-check: ok
```

**T's authored RED tests, now GREEN:**
```
$ cargo test -p holler-proto --test codec_test admin_roster_method_decodes_once_catalogued -- --nocapture
test admin_roster_method_decodes_once_catalogued ... ok

$ cargo test -p holler-proto --test codec_test catalog_has_all_seven_admin_rows -- --nocapture
test catalog_has_all_seven_admin_rows ... ok

$ cargo test -p holler-proto --test docs_wire_test hello_role_admin_deserializes -- --nocapture
test admin_role_wire_test::hello_role_admin_deserializes ... ok

$ cargo test -p holler-cli --test remote_admin_test -- --test-threads=1
running 3 tests
test remote_admin_hello_must_not_create_a_roster_row_from_presence ... ok
test remote_admin_hello_must_not_supersede_the_connected_body ... ok
test roster_remote_server_flag_is_not_yet_recognized ... FAILED   (expected — see "Tests that look wrong" below)
```

**Full-workspace regression check** (`cargo test --workspace --no-fail-fast`): **830 tests passed**; exactly 3 failed, one each in `holler-cli --test cli_surface_test` (`every_pending_line_does_not_parse_yet`), `holler-cli --test remote_admin_test` (`roster_remote_server_flag_is_not_yet_recognized`), and `holler-proto --test codec_test` (`every_method_round_trips`) — all three are test-file gaps this implementation surfaces, not regressions (see "Tests that look wrong (for T)"). Every other test binary in every crate (`holler_hygiene_test`, `hold_hub_test`, `hold_cli_test`, `join_held_test`, `grant_cli_test`, `hold_single_path_test`, `query_test`, `talk_test`, `roster_cli_test`, `wait_test`, `interrupt_test`, `reconnect_contract_test`, `hub_serve_test`, `hub_status_lockout_test`, `platform_test`, `docs_cli_test`, `golden_test`, `docs_errors_test`, plus every crate's own lib unit tests) is unchanged and green.

## Evidence appendix

`docs/handoffs/508/evidence.md` — 6 facts, each `file:line` + verbatim excerpt: `Record`'s `token_id`/`label` fields; `Registry`'s `Clone`/`Default` derive; `AuthDeps`'s exact 4 fields; `hub_query_local`'s "reads the whole request object" shape; `handshake::authenticate`'s existing `pub(crate)` visibility; `BodyIdentity::path`'s signature.

## Tests that look wrong (for T)

None of these are wrong — each is a test whose own design (per its docstring, in two cases) anticipated needing an update once this story's production code landed. Flagging so a real RED isn't mistaken for a regression:

1. **`crates/holler-proto/tests/codec_test.rs`'s `every_method_round_trips`** (existing test, not new RED) fails: `no canonical frame for "admin/status"`. Its `canonical_frame(method)` helper has no arm for the 7 new `admin/*` names; AC 1 explicitly requires this test to pass "with a canonical frame for each" of them. Suggested arms (params/results are byte-for-byte the same as the matching `control/*` form, per MO 2 — these mirror the existing arms' own style):
   ```rust
   "admin/status" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/status","params":{{}}}}"#),
   "admin/roster" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/roster","params":{{"all":false,"prefix":null}}}}"#),
   "admin/say" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/say","params":{{"session":"io/alpha","text":"hi","queue":false,"grant":null,"timeout_ms":600000}}}}"#),
   "admin/interrupt" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/interrupt","params":{{"session":"io/alpha","text":null}}}}"#),
   "admin/answer" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/answer","params":{{"session":"io/alpha","choice":"0"}}}}"#),
   "admin/wait" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/wait","params":{{"sessions":"io/alpha","prefix":null,"until":null,"after":null,"timeout_ms":600000}}}}"#),
   "admin/query" => format!(r#"{{"jsonrpc":"2.0","id":"{id}","method":"admin/query","params":{{"target":null,"method":"query/status","params":null}}}}"#),
   ```

2. **`crates/holler-cli/tests/cli_surface_test.rs`'s `every_pending_line_does_not_parse_yet`** fails exactly as its own assertion message says: `these pending lines now parse — the owning story landed; move them to cli-surface.txt`, naming the same 7 lines `docs/handoffs/506-brief.md`'s AC 9 lists. This is `cli-surface.pending.txt`/`cli-surface.txt` — fixture files, T's to move per this repo's own convention (`cli_surface_test.rs`'s module doc: "the story that adds a verb must move its line over").

3. **`crates/holler-cli/tests/remote_admin_test.rs`'s `roster_remote_server_flag_is_not_yet_recognized`** (AC 9's RED) now fails on its second assertion only: the command still does not succeed (`--server` against an *unjoined* state dir correctly refuses), but the stderr is now `.../body/credential.json not found; run \`holler body join\` first` — exit 1, not a clap usage error, so it no longer contains `"--server"`/`"unexpected argument"`. The test's own doc comment predicts this exactly: "Once AC 9 lands this exact invocation must instead fail differently (dial a real hub, or a clean 'could not reach') … never with a clap usage error — this test's failure mode is the pin." T's options: retire this test (its purpose — "clap doesn't know `--server` yet" — is gone) or repoint its second assertion at the new `NotJoined` wording; either is a test-file decision.

## Known issues

- AC 5 (allowlist unit test + wire refusals), AC 6 (hold), AC 7 (lockout parity), AC 8 (concurrency), AC 10 (JSON parity), AC 11 (no hub state), AC 13 (the remaining failure-word/exit-code cases beyond the policy check), AC 16 (a-e, the liveness/concurrency behaviors), AC 17 (connection logs) have **no dedicated RED test yet** — T's own handoff-T-red.md named this gap explicitly ("Deliberately not attempted yet") and recommended a short RED-addition pass before F's "behavior-heavy work" landed. That pass did not happen before this Phase 6 run reached me. I implemented all of this behavior against the brief/MO's written spec (admin loop concurrency+liveness per MO 5, the allowlist per MO 2-3, `--server` failure wording per AC 13, JSON parity by construction — the remote path calls the exact same handler fns as the local path, byte for byte, so there is no second rendering to drift) and it is exercised end-to-end by the two AC 3/4 tests plus the full existing suite staying green, but none of AC 5-8/10/11/13/16/17 has its own assertion yet. Recommend O re-enter T for that deferred pass before S's spec audit (Phase 10).
- The three test-file gaps under "Tests that look wrong" above are otherwise the only known gaps.

## Files changed

- `crates/holler-proto/src/docs.rs`
- `crates/holler-proto/src/methods.rs`
- `crates/holler-body/src/connection/handshake.rs`
- `crates/holler-body/src/connection.rs`
- `crates/holler-body/src/admin_client.rs` (new)
- `crates/holler-body/src/lib.rs`
- `crates/holler-hub/src/circuit.rs`
- `crates/holler-hub/src/circuit/admin.rs` (new)
- `crates/holler-hub/src/control_server.rs`
- `crates/holler-hub/src/control_status.rs` (new)
- `crates/holler-hub/src/control.rs`
- `crates/holler-hub/src/lib.rs`
- `crates/holler-cli/src/cli.rs`
- `crates/holler-cli/src/transport.rs` (new)
- `crates/holler-cli/src/roster_cmd.rs`
- `crates/holler-cli/src/say_cmd.rs`
- `crates/holler-cli/src/interrupt_cmd.rs`
- `crates/holler-cli/src/answer_cmd.rs`
- `crates/holler-cli/src/wait_cmd.rs`
- `crates/holler-cli/src/hub_cmd.rs`
- `crates/holler-cli/src/main.rs`
- `crates/holler-cli/src/lib.rs`
- `docs/protocol/v2.md`
- `CHANGELOG.md`
