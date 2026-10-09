# Handoff-F: Phase 6 - #669 hub plumbing for `pane/*` and `profile/*` (skeleton slice b)

**Date:** 2026-10-09
**Branch:** issue-669-implementation (on top of 3a7194e; nothing committed by F)
**Issue:** #669
**Architecture changed:** yes (four new hub modules, a changed public signature, a new dependency edge `holler-hub` -> `holler-pane`)

## What was done

All 10 tests in T's `crates/holler-hub/tests/pane_dispatch_test.rs` now pass, and so does the rest of the workspace. The only failing gate is `scripts/lint.sh`, because of one line in T's test file (see "Tests that look wrong").

New production files, all in `crates/holler-hub/src/`:
- `pane_dispatch.rs` (101 lines): the forwarding seam.
  - `PaneDeps { pub panes: Arc<PaneState>, pub profiles: Arc<ProfileState> }`, `#[derive(Clone)]`, with `PaneDeps::load(&HubState)` building both `Arc`s.
  - `forward(method, &cid, &obj, &PaneDeps) -> String` (`pub(crate)`, async): a `pane/*` method goes to `panes::dispatch(.., &deps.panes, &deps.profiles)`, a `profile/*` method to `profile::dispatch(.., &deps.profiles, &deps.panes)`, anything else to `MethodNotFound`.
  - `reply_line(&cid, &PaneReply) -> String` (`pub(crate)`): the one place a `PaneReply` becomes a JSON-RPC **result** line, through `encode_response`.
- `panes/mod.rs` (51 lines): `PaneState` (no fields, not `Clone`), `PaneState::load(&HubState)`, and async `dispatch(method, &cid, &obj, &PaneState, &ProfileState) -> String`, which answers `PaneReply::failure(&PaneError::NotImplemented)`.
- `profile/mod.rs` (69 lines): `ProfileState` (no fields, not `Clone`), `ProfileState::load`, async `dispatch(method, &cid, &obj, &ProfileState, &PaneState) -> String` (routes `profile/rename` to `rename::dispatch`, answers `not-implemented` for the rest), and `check_membership(&Pane, &ProfileState) -> Result<(), PaneError>`, which returns `Ok(())`.
- `profile/rename.rs` (25 lines): `rename::dispatch(&cid, &obj, &ProfileState, &PaneState)`, which answers `not-implemented`. See Deviations.
- `pane_wiring.rs` (6 lines): doc comment only; deliberately empty.

Edited files:
- `control_server.rs` (829 -> 837): two imports; `handle_control_conn` and `dispatch_control` gain one parameter (`pane_deps`); one new arm `Some(m) if is_pane_method(m) || is_profile_method(m) => crate::pane_dispatch::forward(..)`, placed before the `control/` arm; two doc lines.
- `serve.rs` (833 -> 838): `SharedState` gains one field `pane_deps: PaneDeps`; `build_shared_state` adds one call, `PaneDeps::load(state)` (with a two-line comment), right after the `Holds::load`/`with_holds` lines; the destructure and the `accept_loop` call pass it on; `accept_loop` gains one parameter and clones it into `handle_control_conn`; the `#184` allow note now says 6 handles.
- `lib.rs`: the four `pub mod` declarations (`pane_dispatch`, `pane_wiring`, `panes`, `profile`).
- `crates/holler-hub/Cargo.toml`: `holler-pane` (path) dependency; `holler-pane-testkit` (path) dev-dependency. `Cargo.lock`: exactly those two edges.
- `CHANGELOG.md`: an `## [Unreleased]` / Enhancements entry linking #669 (and the epic #633).

Nothing was changed in `holler-proto`, `holler-pane`, `holler-cli`, `control.rs`, `send_prompt`, any golden file or any ADR. `CATALOG` is still 22 rows.

## Design decisions

- **One bundle, `PaneDeps`** (A finding 4; T fixed the name). It is the single new `SharedState` field and the single new `accept_loop` parameter, so `serve.rs` grows by 5 lines. `PaneDeps::load` is what `build_shared_state` calls, so the two `Arc::new(..::load(state))` lines live in the new file and not in `serve.rs`. It follows the `AdminDeps` precedent (`#[derive(Clone)]`, `Arc` fields).
- **The state types are fieldless and not `Clone`** (brief Risks: no speculative fields). `load(_state)` ignores its argument and returns `Self`. #639 and #661 add their fields and their corrupt-file behaviour inside their own `load`.
- **`&PaneState` / `&ProfileState` signatures, with the `Arc` fields passed as they are** (A finding 1). `forward` writes `&deps.panes` and `&deps.profiles`, never `&*..` or `.as_ref()`. They deref-coerce to today's parameter types, and a story whose handler must move an owned handle into `spawn_blocking` can widen its own `dispatch` to `&Arc<_>` without touching `pane_dispatch.rs`.
- **Reply encoding is a `PaneReply` as the JSON-RPC result** (MO decision 3), implemented once in `reply_line`. All three stubs call it. A later handler does `reply_line(cid, &PaneReply::success(data))`, so no story re-implements the encoding. `serde_json::to_value(reply).unwrap_or_default()` follows the file's existing encoders; a `PaneReply` always serializes, and a `null` result would still fail the client's parse and never read as a success.
- **The defensive branch of `forward`** (a method in neither list) answers `MethodNotFound` through `encode_error`, as `dispatch_session_control` does. No `unreachable!` (denied), and no `Option` plus `unwrap_or_default` (it would write an empty line to the socket).
- **`rename.rs` is a one-function stub that `profile::dispatch` already routes to** (A finding 6, first option). See Deviations.
- **`pane_wiring.rs` is declared and empty**, as the issue says (A finding 7). Nothing is written to it. Removing it later needs a `lib.rs` edit, which is outside #649's blast radius; O decides whether to amend #649.
- **The `mod.rs` exception** (A finding 5). `src/panes/mod.rs` and `src/profile/mod.rs` are the first `mod.rs` files under any crate's `src/` (A found none on main). They are kept because each module root has to sit inside its owner's blast-radius glob: `panes/**` (#639) and `profile/**` (#661); a sibling `panes.rs` or `profile.rs` would be outside both. `pane_dispatch.rs`, `pane_wiring.rs` and `rename.rs` are plain files. The same reasoning is recorded in the module docs and in decisions.md.
- **`check_membership` keeps the issue's two-argument shape** (A finding 2 not applied). T's tests fix that signature, and widening it changes the issue text, which is O's call. The consequence A named stands: the hook cannot see the stored pane record, so #661's `pane-in-other-profile` rule either runs in #639's `cas_put` or widens this signature.
- **Live-hub check.** The integration test drives `handle_control_conn` directly and so bypasses the `serve.rs` wiring. I started a real `holler hub serve --listen 127.0.0.1:0` in an isolated state dir and sent requests over its real control socket: `pane/list`, `pane/watch`, `profile/list` and `profile/rename` each returned a JSON-RPC result `{"ok": false, "data": null, "error": {"code": "not-implemented", "message": "not implemented"}}`; `pane/frobnicate`, `profile/frobnicate`, `control/x` and `foo/bar` returned `-32601`; `control/status` returned a result; the hub exited 0 on SIGTERM. The script is not committed.

## Reuse / extend-vs-new

Extended, per the brief's Reuse map:
- The `control_hold.rs` precedent: a plain-function module that the control dispatcher calls from one arm (`pane_dispatch::forward`), so `dispatch_control` stays under the cognitive-complexity limit (clippy is clean with the new arm).
- `encode_response` and `encode_error` for every reply. No new reply encoder bypasses them; `reply_line` is a thin typed wrapper over `encode_response`.
- `Holds::load(&HubState)` and the `Arc<Roster>` / `Arc<Lockout>` sharing pattern for the state handles; the `AdminDeps` bundle shape for `PaneDeps`.
- `holler_proto::methods::{is_pane_method, is_profile_method}` for the guard; no method-name list is re-declared in the hub.

New objects (each named by the issue or the brief): the modules `pane_dispatch`, `panes`, `profile`, `profile::rename`, `pane_wiring`; `PaneState`, `ProfileState`, `PaneDeps`; `check_membership`. Beyond the issue: `PaneDeps::load`, `forward` and `reply_line`, which are the "forwarding helper" the issue asks for plus the one shared encoder (each has callers today, so none is dead code). No near-copy of `support::Hub` or `StateDir` was written; the test harness is T's and lives inside its test file.

## Architecture notes for A

- **Layers touched:** hub only. The control socket server (`control_server.rs`), hub startup (`serve.rs`), and four new modules. `holler-pane` gains its first consumer.
- **Dependency direction:** `holler-hub` -> `holler-pane` (new, correct direction: `holler-pane` has no runtime and no hub dependency). `holler-pane-testkit` is a dev-dependency only. No other edges changed.
- **Module cycles (as in existing code):** `pane_dispatch` -> `panes`/`profile` (dispatch) and `panes`/`profile` -> `pane_dispatch::reply_line`; the same shape as `control_server` <-> `control_hold`.
- **Public interface changes:** `handle_control_conn` (pub) gains a last parameter `pane_deps: PaneDeps`; its one caller is `accept_loop`. New pub items: `pane_dispatch::{PaneDeps, PaneDeps::load}`, `panes::{PaneState, PaneState::load, dispatch}`, `profile::{ProfileState, ProfileState::load, dispatch, check_membership}`, `profile::rename::dispatch`. No wire, protocol, file-format or config change.
- **Frozen after this slice:** `pane_dispatch.rs`, `control_server.rs`'s arm and `serve.rs`'s handles. #639 and #661 are expected to edit only `panes/**` and `profile/**`; #665 only `profile/rename.rs`.
- **`send_prompt` untouched** (MO decision 5); no store I/O is added, so `spawn_blocking` and `acquire_lock_retrying` do not apply yet.
- **Admin socket:** `pane/*` and `profile/*` stay unreachable over `admin/*` (the admin loop calls `dispatch_allowlisted`, not `dispatch_control`) and over a body socket (they are outside the closed `CATALOG`).
- **File sizes:** `control_server.rs` 837, `serve.rs` 838 (the fail line is 900).

How each A finding from Phase 3 was handled:

| A# | Handling |
|---|---|
| 1 | Applied: `&deps.panes` and `&deps.profiles` passed as they are; signatures are the issue's. |
| 2 | Not applied (changes the issue text); left open for O, see above. |
| 3 | T tested at the bundle seam; `serve.rs` gained no `pub` and no test hook. |
| 4 | Applied: `PaneDeps` in `pane_dispatch.rs`, one field, one parameter; the `#184` note updated; the fallback branch is `MethodNotFound`. |
| 5 | Recorded (decisions.md and the module docs). |
| 6 | Applied, first option: `profile::dispatch` routes `profile/rename` to a stub in `rename.rs`. |
| 7 | Kept as the issue says (declared, empty); recorded as a permanent placeholder; O decides on #649. |
| 8 | T's: the harness stays inside the test file. |

## Deviations from spec / wireframe

1. **`src/profile/rename.rs` is not empty.** The issue and the brief (AC 6) say "an empty module declared in `profile/mod.rs` (#665 fills it and edits no #661 file)". It holds one stub function, and `profile::dispatch` routes `"profile/rename"` to it. Reason: #661 owns `profile/**` including `profile/mod.rs`, and its scope lists six handlers that do not include `profile/rename`. With an empty `rename.rs` nothing would route the method into it, so #665 could not fill the module without editing a #661 file, which defeats the stated purpose. A raised this as finding 6 and suggested exactly this fix. It is an 8-line function plus one match arm; to get a literally empty file back, delete both. Observable behaviour is the same either way (`profile/rename` answers `not-implemented`).
2. The `#184` note on `accept_loop`'s `#[allow(clippy::too_many_arguments)]` now reads "6 shared hub-wide handles (the 6th is #669's)"; the `// #184` link stays on the line.

Nothing else deviates from the brief or the issue.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-hub --test pane_dispatch_test        (x4 runs, all identical)
running 10 tests
test testkit_links ... ok
test the_state_types_do_not_derive_clone_so_a_connection_cannot_fork_a_copy ... ok
test profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub ... ok
test check_membership_accepts_any_pane ... ok
test panes_dispatch_takes_both_handles_and_answers_not_implemented_as_a_result ... ok
test two_connections_share_the_one_pair_of_state_handles ... ok
test unknown_methods_still_answer_method_not_found ... ok
test every_pane_method_is_forwarded_to_the_stub_not_method_not_found ... ok
test every_profile_method_is_forwarded_to_the_stub_not_method_not_found ... ok
test an_existing_control_method_still_answers_through_the_new_dispatcher ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's command)
exit 0; 94 result blocks (test binaries and doc-tests), all "ok"; passed=937 failed=0 ignored=5 (the 5 are pre-existing #[ignore]s)
(The holler-cli suites start a real `holler hub serve` and bodies, so they run through the changed `serve_forever` and `accept_loop`. CI's `--ignored` interop step, `cargo test -p holler-cli --test body_run_test -- --ignored`, matches 0 tests: those cases are no longer ignored. The separately retried load-roster test was not run; it does not touch this change.)
$ cargo test -p holler-hub   (re-run after the last doc-only edit)
77 + 1 + 1 + 10 + 19 + 27 passed, 0 failed (1 pre-existing ignore)

$ cargo build --workspace
Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)      (clean, no warnings)

$ cargo machete
cargo-machete didn't find any unused dependencies in this directory. Good job!

$ bash scripts/changelog-check.sh
changelog-check: ok

$ bash scripts/lint.sh | grep -v '^warn:'
crates/holler-hub/tests/pane_dispatch_test.rs:1:#![allow(
lint: every #[allow] needs a trailing '// #NNN' issue link
(exit 1; the only complaint is T's test file, see below. Size warnings are the existing
 600-line ones; control_server.rs 837 and serve.rs 838 are under the 900-line fail line.)

$ rustfmt --check --edition 2021 crates/holler-hub/src/{pane_dispatch,pane_wiring,panes/mod,profile/mod}.rs
(clean; profile/mod.rs also covers profile/rename.rs, which it declares)

$ git diff --check
(clean)

$ git diff --name-only origin/main...HEAD, plus the working tree
only paths in the issue's Blast radius (hub src files above, Cargo.toml, Cargo.lock, CHANGELOG.md,
tests/pane_dispatch_test.rs, docs/handoffs/669*)
```

No existing file was reformatted: the diff of `control_server.rs`, `serve.rs` and `lib.rs` is the lines listed above and nothing else. No rustdoc warning comes from the new files (`cargo doc -p holler-hub --no-deps`; the existing warnings are all in other files).

## Evidence appendix

`docs/handoffs/669/evidence.md`: six entries for behaviour that lives in unchanged code and that this change relies on (`encode_response` and `encode_error`, `PaneReply::failure`, the `not-implemented` code, and the exact-list matching of `is_pane_method` / `is_profile_method`).

## Tests that look wrong (for T)

1. **`crates/holler-hub/tests/pane_dispatch_test.rs:1-6` fails `bash scripts/lint.sh` (AC 7).** The multi-line form

   ```rust
   #![allow(
       clippy::unwrap_used,
       ...
   )] // #669
   ```

   is flagged by lint check 1, which greps line by line: `#![allow(` is on line 1 and the `// #669` link is on line 6, so line 1 has no link. The one-line form that every other test file in the repo uses does not work for a new file either, because `rustfmt --check` rewrites it to the multi-line form (the arguments are wider than rustfmt's 60-column call width). Two layouts that satisfy both gates, both checked with `rustfmt --check --edition 2021` and with lint check 1's own grep:
   - two attributes, each with the link on its own line:
     `#![allow(clippy::unwrap_used, clippy::expect_used)] // #669` and `#![allow(clippy::panic, clippy::unreachable)] // #669`; or
   - the link after the opening parenthesis: `#![allow( // #669`, then the four lints on the following lines and `)]`.

   I did not edit the test. Nothing else in the file changes.
2. Not a defect, a note: `an_existing_control_method_still_answers_through_the_new_dispatcher` sends `control/status`, whose handler reads `listening.json` from the resolved state dir (`HOLLER_STATE_DIR`, else `~/.holler`). It reads, never writes, and the test only asserts the reply's shape, so it is correct as written. It is not hermetic about that one read.

## Known issues

- `bash scripts/lint.sh` exits 1 until T fixes the one line above. Everything else in AC 7 passes.
- No test drives `serve.rs`'s own wiring (`build_shared_state` -> `accept_loop` -> `handle_control_conn`): those items are private, and A finding 3 rules out adding `pub` or a test hook for it. It is covered by the build, the single call site, and the live-hub check above, not by an automated test.
- `forward`'s "method in neither list" branch is not reachable from `tests/` (the function is `pub(crate)` and the arm's guard keeps it out); it is a defensive `MethodNotFound`.
- Open for O (from A, unchanged): finding 2 (the membership hook cannot see the stored record, so #661's `pane-in-other-profile` rule needs a decision before #661), and finding 7 (`pane_wiring.rs` stays a placeholder; #649's text says "leave it empty or remove it" but removal needs `lib.rs`).

## Files changed

Production and manifest files (no test files; staged by path):
- `crates/holler-hub/src/pane_dispatch.rs` (new)
- `crates/holler-hub/src/pane_wiring.rs` (new)
- `crates/holler-hub/src/panes/mod.rs` (new)
- `crates/holler-hub/src/profile/mod.rs` (new)
- `crates/holler-hub/src/profile/rename.rs` (new)
- `crates/holler-hub/src/lib.rs`
- `crates/holler-hub/src/control_server.rs`
- `crates/holler-hub/src/serve.rs`
- `crates/holler-hub/Cargo.toml`
- `Cargo.lock`
- `CHANGELOG.md`

Pipeline artifacts: `docs/handoffs/669/handoff-F.md`, `docs/handoffs/669/evidence.md`, `docs/handoffs/669/decisions.md` (F entry appended).
