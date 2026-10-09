# Brief: #669 the hub plumbing for `pane/*` and `profile/*` control methods

Repo: Performant-Labs/holler. Issue: #669 (slice b of epic #633's skeleton; slice a #637 is merged at `f2602ba`, slice c is
#670). Rigor: in-session. UI surface: no. Kind: feature.

**Branch:** `issue-669-implementation`. **Design (D):** N/A (no UI). **Decision record:** the contract section of epic #633
and the "Skeleton split" section of the epic are fixed; ADR-0021 (#634) ratifies them later. The issue text of #669 is the
source of truth with the epic; where they differ from this brief, the issue wins.

**Revision 1.** Written from the merged `holler-pane` crate (not from the earlier design), after the single-story brief
drew three architecture BLOCKs and was split (2026-10-09). The earlier combined brief's hub half is the design basis; its
public-API claims were re-checked against the merged code below.

## Problem

A `pane/*` or `profile/*` request on the hub's control socket falls through to MethodNotFound. Stories #639 (pane registry),
#661 (profile registry) and #649 (the CLI's control-socket client) need a place to put their handlers and shared state without
editing `control_server.rs` (829 lines) or `serve.rs` (833 lines), both past the 800-line flag. This slice adds only the
plumbing: a forwarding arm, two shared-state handles, stub dispatchers that answer "not implemented", and a membership-check
hook. It adds no logic, no persistence and no adapter.

## Evidence (verbatim, as of `f2602ba`)

The control socket answers every non-`control/` method with MethodNotFound; state reaches handlers through four arguments:
```
crates/holler-hub/src/control_server.rs:76
async fn dispatch_control(line: &str, registry: &Registry, roster: &Roster, lockout: &Lockout) -> String {
crates/holler-hub/src/control_server.rs:108-111 (the tail of the match)
        Some(other) if other.starts_with("control/") => {
            dispatch_session_control(other, &cid, &obj, registry, roster, lockout).await
        }
        Some(other) => encode_error(&cid, Code::MethodNotFound, format!("unknown control method: {other}")),
crates/holler-hub/src/control_server.rs:24-29
pub async fn handle_control_conn(stream: UnixStream, registry: Registry, roster: std::sync::Arc<Roster>, lockout: std::sync::Arc<Lockout>)
crates/holler-hub/src/serve.rs:565   (its only caller)
tokio::spawn(crate::control_server::handle_control_conn(stream, registry.clone(), roster.clone(), lockout.clone()));
```
Shared hub state is built once, before the first connection, and handles are `Arc`s cloned per connection:
```
crates/holler-hub/src/serve.rs:343-349 (SharedState: registry, hygiene, lockout (Arc), preauth_semaphore (Arc), roster (Arc))
crates/holler-hub/src/serve.rs:351-362 (build_shared_state(state: &HubState, ..); `Holds::load(state)` loads before the first connection)
crates/holler-hub/src/serve.rs:545 (accept_loop, already many parameters, under `#[allow(clippy::too_many_arguments)] // #184`)
```
`encode_response` is `pub(crate)` (`control_server.rs:697`), the helper existing control modules (`control_hold.rs`) already call.
`HubState` is `{ root, hub_dir }` with `HubState::from_root(PathBuf)` (`state.rs:15-25`). The merged `holler-pane` crate (slice
a) gives: `PaneReply::{success, failure, into_result}`, `decode_params`, `PaneError::NotImplemented` (code `not-implemented`), the
params structs in `holler_pane::reply`, and `holler_proto::methods::{PANE_METHODS, PROFILE_METHODS, is_pane_method,
is_profile_method}`. `PANE_METHODS` is `pane/get|list|cas_put|delete|watch`; `PROFILE_METHODS` is
`profile/get|list|cas_put|delete|watch|log|rename`. They are outside the closed 22-row `CATALOG`, which this slice does not touch.

Build guards: `cargo machete` fails CI on an unused dependency; clippy denies `unwrap`, `expect`, `panic`, `too_many_lines`
(100), `cognitive_complexity` (15); `dead_code = "deny"`; `scripts/lint.sh` warns at 600 lines and **fails at 900**. The tree is
not rustfmt-clean and CI has no fmt step: every **new** `.rs` file passes `rustfmt --check --edition 2021`, existing files are
**not reformatted** (formatting `control_server.rs` or `serve.rs` would push them past 900).

## Acceptance criteria

1. **Forwarding.** A `pane/list` and a `profile/list` request over the control socket reach the stub and return a JSON-RPC
   **result** that parses back with `holler_pane::PaneReply::into_result` into `Err(PaneError::NotImplemented)` (code
   `not-implemented`), not MethodNotFound. Every method in `PANE_METHODS` and `PROFILE_METHODS` is forwarded (a table test over
   both lists). Test: `crates/holler-hub/tests/pane_dispatch_test.rs`, driving `handle_control_conn` over
   `tokio::net::UnixStream::pair()` (`Registry::new`, `Roster::with_system_clock`, `Lockout::new` are `pub`); it does not spawn the
   `holler` binary and does not copy `support::Hub`.
2. **Nothing else changes.** An unknown `control/x` and an unknown `foo/bar` still return MethodNotFound; every existing
   `control/*` method behaves as before; the existing hub tests pass unchanged; `CATALOG.len()` is still 22; no golden file changes.
3. **State handles.** `PaneState::load(&HubState) -> PaneState` and `ProfileState::load(&HubState) -> ProfileState` return empty
   state (stubs; #639 and #661 fill them, including corrupt-file behaviour). They are built once in `build_shared_state` next to
   `Holds::load`, carried in `SharedState` as `Arc<PaneState>` and `Arc<ProfileState>`, and the types do **not** derive `Clone`
   (so a later inline-state change cannot fork a copy per connection and defeat the compare-and-swap). A test shows two
   connections share the same handle (pointer equality of the `Arc`s the dispatcher receives).
4. **Dispatch signatures.** `panes::dispatch(method, &cid, &obj, &PaneState, &ProfileState) -> String` and
   `profile::dispatch(method, &cid, &obj, &ProfileState, &PaneState) -> String` (async), each receiving both handles, so a
   membership check cannot fail open and `profile/rename` can reach the pane store. Both answer
   `PaneReply::failure(&PaneError::NotImplemented)` as a JSON-RPC result through `encode_response`. `*/watch` is long-poll (one
   `{events, cursor}` reply per request): the stub does not stream.
5. **Membership hook.** `check_membership(&holler_pane::Pane, &ProfileState) -> Result<(), PaneError>` is a plain function
   returning `Ok(())` for any pane (`check_membership_accepts_any_pane`); #639's `cas_put` will call it and #661 will fill it.
6. **Manifest and reuse.** `holler-hub` depends on `holler-pane` (consumed by the above) and dev-depends on `holler-pane-testkit`
   with a one-line consumer test (`testkit_links`), so #638, #639 and #661 add no manifest line. `cargo machete` is clean. The
   forwarding helper lives in the new `src/pane_dispatch.rs`; `lib.rs` declares `panes`, `profile`, `pane_wiring` (empty),
   `pane_dispatch`; `src/profile/rename.rs` is an empty module declared in `profile/mod.rs` (#665 fills it without editing a #661
   file).
7. `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo machete`,
   `bash scripts/lint.sh` and `bash scripts/changelog-check.sh` pass; no file over 900 lines (`control_server.rs` and `serve.rs`
   grow by only the arm, the handles and one parameter); `CHANGELOG.md` has an `## [Unreleased]` entry linking #669; every new
   `.rs` file passes `rustfmt --check --edition 2021` and no existing file is reformatted.
8. `git diff --name-only origin/main...HEAD` lists only paths in the Blast radius below (grep-able check for S).

## Files

- `crates/holler-hub/src/control_server.rs` (edit, small): in `dispatch_control` one new arm
  `Some(m) if is_pane_method(m) || is_profile_method(m)` that calls a helper in `pane_dispatch.rs`; `handle_control_conn` and
  `dispatch_control` gain the one bundled shared-state parameter. Place the arm before the `control/` arm so ordering is explicit.
- `crates/holler-hub/src/serve.rs` (edit, small): `SharedState` gains the two `Arc` handles (or one bundle struct holding them),
  `build_shared_state` builds them next to `Holds::load`, and the one caller at `serve.rs:565` passes the clone. `accept_loop`
  grows by at most one parameter.
- New: `src/pane_dispatch.rs` (the forwarding helper, keeping cognitive complexity of `dispatch_control` under 15),
  `src/panes/mod.rs` (`PaneState`, `dispatch`), `src/profile/mod.rs` (`ProfileState`, `dispatch`, `check_membership`),
  `src/profile/rename.rs` (empty), `src/pane_wiring.rs` (empty, declared; no job under the CLI-side ruling, left for #649 to drop or
  fill). `src/lib.rs` (edit): the four declarations.
- `crates/holler-hub/Cargo.toml`: `holler-pane` (path) dependency; `holler-pane-testkit` (path) dev-dependency.
- Tests: `crates/holler-hub/tests/pane_dispatch_test.rs`.
- `CHANGELOG.md`: an `## [Unreleased]` entry linking #669. `Cargo.lock`: the two new edges only.

Reuse map (extend, do not duplicate): the `control_hold.rs` precedent of a plain-function module the control dispatcher calls;
`encode_response` and `encode_error` for the reply; `Holds::load(&HubState)` and the `Arc<Roster>`/`Arc<Lockout>` sharing
pattern for the state handles; the existing `handle_control_conn` test style (a raw `UnixStream` pair) for the dispatch test. A new
near-copy of `support::Hub` or `StateDir` is a rejection.

## Decisions already made (MO)

0. **Verbs run CLI-side against the ports; the hub is the store only.** The hub registers no adapters.
1. **Plain functions, no function-pointer registry.** The arm calls a helper; the helper matches on the method list and calls
   `panes::dispatch` or `profile::dispatch`.
2. **Both handles go to both dispatchers** (decision on shared state, 2026-10-09): `panes::dispatch` gets the `ProfileState` so
   `cas_put` can call `check_membership`; `profile::dispatch` gets the `PaneState` so `profile/rename` can update panes.
3. **Reply encoding:** a `PaneReply` as the JSON-RPC **result** (not a JSON-RPC error). This deviates from the existing control
   methods (JSON-RPC errors with a `data.reason` sub-code, `control_server.rs:316,697-713`) because the domain codes do not fit
   the closed wire `Code` table; record it in decisions.md. `holler-proto` is not touched.
4. **State loads before the first connection** and corrupt-file behaviour stays inside `load` (fail closed, owned by #639/#661).
5. **No `send_prompt` change.** If #646 needs a pane-state gate at `send_prompt`, #646 adds it and widens its own radius.

## Out of scope

Any handler logic, persistence, change feed, CAS, membership rules (#639, #661); adapters (#649 wiring is CLI-side); the CLI
(#670); `docs/protocol/v2.md` and ADR-0021 (#634); `control.rs` (no edit: #649's client uses `holler_hub::control::run` as is).

## Test plan

RED first (T): `pane_dispatch_test.rs` (forwarding table over both method lists, the unchanged-MethodNotFound cases, the shared
`Arc` handle, `check_membership_accepts_any_pane`, `testkit_links`); it cannot compile until the modules exist, which is the RED.
Confirm RED by `cargo test -p holler-hub --test pane_dispatch_test` failing to build or on named tests. GREEN: F adds the modules
and plumbing. Then the Tier 1 commands in AC 7, `rustfmt --check --edition 2021` on each new `.rs` file, and the full workspace
test run (the existing hub tests must pass unchanged).

## Risks

- `control_server.rs` (829) and `serve.rs` (833) are past the 800-line flag: add only the arm, the handles and one parameter; no
  logic. Do not run rustfmt over them.
- `dispatch_control` is already split to stay under the cognitive-complexity limit: the new arm calls a helper.
- Changing `handle_control_conn`'s signature touches its one caller (`serve.rs:565`); no test calls it directly today.
- `dead_code = "deny"` and `cargo machete`: the empty `pane_wiring` and `rename` modules must be declared `pub mod` and the new
  dependencies consumed.
- The `PaneState`/`ProfileState` types carry no data yet: do not add fields speculatively; #639 and #661 add theirs.

## Blast radius

`crates/holler-hub/src/lib.rs`, `control_server.rs`, `serve.rs`, `pane_dispatch.rs`, `pane_wiring.rs`, `panes/mod.rs`,
`profile/mod.rs`, `profile/rename.rs`; `crates/holler-hub/Cargo.toml`; `crates/holler-hub/tests/pane_dispatch_test.rs`; `Cargo.lock`;
`CHANGELOG.md`; `docs/handoffs/669*` (pipeline artifacts). Not changed: `holler-proto`, `holler-pane`, `holler-cli`, `control.rs`,
`vocab.rs`, any golden file, ADRs.
