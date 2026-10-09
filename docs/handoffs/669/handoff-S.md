# Handoff-S: Phase 10 - #669 hub plumbing for `pane/*` and `profile/*` (skeleton slice b), spec audit, pass 2

**Date:** 2026-10-09 (07:24 MDT)
**Branch:** issue-669-implementation (at 31fe2bd)
**Issue:** #669 (slice b of epic #633)
**Brief:** `docs/handoffs/669-brief.md` (Revision 1)
**Handoffs reviewed:**
- `handoff-A.md`
- `handoff-T-red.md`
- `handoff-F.md`
- `handoff-T-green.md`, including its "Test-only rework" section
- `handoff-A-dup.md` (pass 2)
- `decisions.md` and `evidence.md`
- this file's pass 1 (commit cc674a0)

**Diff audited:** `git diff origin/main...HEAD`, 21 paths. The merge base is f2602ba, which is still `origin/main`.

Since pass 1 (at 8275e73), only one code file has changed: `crates/holler-hub/tests/pane_dispatch_test.rs` (+16/-14, commit 820e385). `git diff 8275e73 HEAD -- crates Cargo.lock CHANGELOG.md` shows nothing else.

I re-read every production file and the test file in full at HEAD. I read issue #669 with `gh issue view`, and also #661 and #665 for the rename seam.

## A precondition

Met.
- `handoff-A.md` (Phase 3) is **PASS**, with 8 warns and no block.
- `handoff-A-dup.md` (Phase 7, pass 2, diff f2602ba...820e385) is **PASS**, with 2 warns.
  - The warns are W1 (a copied fixture) and W3 (where the harness lives). Both are about the test file and later stories.
  - Pass 1's W2 is resolved.

## T precondition

Met.

**RED** (`handoff-T-red.md`): the test target did not build. There were 12 errors, all caused by the missing feature: the missing modules, the missing manifest lines and the `handle_control_conn` arity. T also checked the assertions against a throwaway stub:
- 10 of 10 tests passed.
- With the forwarding arm disabled, the three forwarding tests failed on their assertions.

**GREEN** (`handoff-T-green.md`):
- 10 of 10 tests passed, three runs in a row.
- The workspace suite passed with CI's command: 937 tests.
- The arm-disabled mutation fails the same three tests. T restored the file afterwards.
- Blocking issues: none.

**Test-only rework** (`handoff-T-green.md`, "Test-only rework"):
- The probe now sends `control/roster`.
- With `HOME` set to an empty scratch directory, 10 of 10 tests passed and no `$HOME/.holler` was created.
- `rustfmt --check` is clean, and `lint.sh` and clippy are clean.

**Tier 1** (T-green):
- `lint.sh` exit 0
- `changelog-check.sh` ok
- `clippy --workspace --all-targets -D warnings` clean
- `cargo machete` clean
- workspace tests pass
- F recorded `cargo build --workspace`.

## Pass 1's REWORK item is resolved

Pass 1 found two problems with `an_existing_control_method_still_answers_through_the_new_dispatcher`:
- It sent `control/status`, which wrote `$HOME/.holler/hub/identity.key`.
- It could not fail because of the new arm.

### The new probe

The probe is at `pane_dispatch_test.rs:181-204`. It sends `control/roster` with the id `h-roster-1` and asserts three things, which is what pass 1 asked for:
- the reply has no `error` key;
- `result.rows` is `[]`;
- the result does not decode as a `PaneReply`.

### The route

I traced the route through the code:
1. `dispatch_control` has no exact arm for `control/roster`, so the request passes the new arm's guard (`control_server.rs:117`).
2. It reaches the `control/` prefix arm (`:118`), then `dispatch_session_control` (`:148`), then `roster_control` (`:461-475`).
3. That handler reads only `Roster::rows_matching` and `Registry::holds()`.
4. `Registry::new()` carries `Holds::in_memory()` (`holds.rs:282-284`, `path: None`).

The other handles do no file I/O either:
- `Lockout::new` and `Roster::with_system_clock` read only environment variables and memory.
- `log_control` writes only to stderr (`holler-proto/src/log.rs:548-554`).

### The probe can now fail because of the change

- If the new arm's guard matched a `control/` method, `forward` would take its branch for a method in neither list. That branch answers `-32601` (`pane_dispatch.rs:85-93`), which fails the first assertion.
- If that branch answered with a `PaneReply` result instead, the other two assertions would fail.

### Verified: the side effect is gone

- I built the test binary with `cargo test -p holler-hub --test pane_dispatch_test --no-run`. It was already up to date at HEAD.
- I ran it with `HOME` set to an empty scratch directory and `HOLLER_STATE_DIR` unset.
- Result: 10 passed, and the scratch directory was still empty afterwards. I then deleted it.

### The `connect()` doc is now true

The `connect()` doc (`:56-57`) says "the methods these tests send never resolve the state dir, so none is touched". That now holds for every request in the file:
- The pane and profile stubs do no I/O.
- `control/x` ends at the fallback of `dispatch_session_control` (`control_server.rs:171`).
- `foo/bar`, `pane/frobnicate` and `profile/frobnicate` end at the fallback of `dispatch_control` (`:121`).
- `fresh_deps` and `check_membership_accepts_any_pane` use `tempfile::tempdir()`.

### Fold-ins

- `fresh_deps` now calls `PaneDeps::load(&state)` (`:95-99`). The tests therefore run the constructor that `build_shared_state` uses (`serve.rs:366`).
- The `pane/watch` comment (`:129-131`) no longer claims that a request follows `pane/watch` in that loop.

## Acceptance criteria

All eight criteria are met.

| AC | Criterion (short) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Forwarding: every method in `PANE_METHODS` and `PROFILE_METHODS` gets a JSON-RPC result that `into_result` reads as `Err(NotImplemented)`, over `UnixStream::pair()`, with no binary and no copy of `support::Hub` | `every_pane_method_is_forwarded_to_the_stub_not_method_not_found` (`:126`) and `every_profile_method_is_forwarded_to_the_stub_not_method_not_found` (`:144`). Details below. | MET |
| 2 | Nothing else changes | `unknown_methods_still_answer_method_not_found` (`:161`), the reworked probe (`:182`) and the unchanged suites. Details below. | MET |
| 3 | State handles: empty `load`, built once next to `Holds::load`, held as `Arc`s, not `Clone`, and shared by two connections | Code reading of `serve.rs` and two tests (`:209`, `:264`). Details below. | MET, at the bundle seam (A Phase 3 W3) |
| 4 | Dispatcher signatures, reply encoding and long-poll `watch` | Two direct-call tests (`:285`, `:300`) and `reply_line`. Details below. | MET |
| 5 | `check_membership` returns `Ok(())` for any pane | `check_membership_accepts_any_pane` (`:341`): a pane with no profile, and one that names a missing profile | MET |
| 6 | Manifest, reuse and module layout | `Cargo.toml`, `Cargo.lock`, `testkit_links` (`:356`), `lib.rs`. Details below. | MET |
| 7 | Gates, file sizes, changelog, formatting | T-green's Tier 1 table, `wc -l`, my own `rustfmt --check`. Details below. | MET |
| 8 | `git diff --name-only origin/main...HEAD` stays inside the blast radius | 21 paths: the 12 product paths of the radius, `docs/handoffs/669-brief.md`, and 8 files under `docs/handoffs/669/` | MET |

### AC 1: forwarding

Each of the two tests walks its `holler_proto` method list on one connection and asserts four things:
- the reply has no `error` key;
- the request id is echoed;
- `result` decodes as a `PaneReply`;
- `into_result()` gives `Err(PaneError::NotImplemented)`.

T's mutation check showed that both tests fail with the arm disabled.

### AC 2: nothing else changes

- `unknown_methods_still_answer_method_not_found` sends `control/x` and `foo/bar`. It also sends `pane/frobnicate` and `profile/frobnicate`, which would catch a guard that matches on the prefix.
- The probe sends `control/roster` through the `control/` arm that follows the new arm.
- The unchanged suites pass (937 tests).
- No existing test, no `holler-proto` file and no golden file is in the diff.
- `CATALOG.len() == 22` is asserted at `holler-proto/src/methods.rs:154`, which is untouched.

### AC 3: state handles

The types:
- Both are fieldless and do not derive `Clone` (`panes/mod.rs:27`, `profile/mod.rs:31`).
- Both `load` functions return `Self` (`:33`, `:37`).

The wiring, checked by reading `serve.rs`:
- `PaneDeps::load(state)` is at `:366`, right after `Holds::load` (`:360-363`).
- `build_shared_state` has one call (`:466`). It runs before `accept_loop` is spawned (`:483-484`).
- The accept loop clones the bundle for each control connection (`:570`).

The tests:
- `two_connections_share_the_one_pair_of_state_handles` (`:209`) checks `Arc::ptr_eq`. It also checks `strong_count == 3` while both connections are live.
- `the_state_types_do_not_derive_clone_so_a_connection_cannot_fork_a_copy` (`:264`) shows that both state types are not `Clone` and that `PaneDeps` is.

### AC 4: dispatcher signatures

- `panes_dispatch_takes_both_handles_and_answers_not_implemented_as_a_result` (`:285`) and `profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub` (`:300`) call both dispatchers directly, in the issue's argument order. That pins the signatures at compile time.
- Encoding: `reply_line` calls `encode_response` (`pane_dispatch.rs:98-101`).
- Watch: each dispatcher returns one `String`. In the profile table test, `profile/watch` is followed by `profile/log` and `profile/rename` on the same connection.

### AC 6: manifest, reuse and module layout

- `Cargo.toml` adds `holler-pane`, and `holler-pane-testkit` as a dev-dependency. `Cargo.lock` gains exactly those two edges.
- `testkit_links` (`:356`) relies on `use holler_pane_testkit as _;` (`:40`).
- `cargo machete` is clean (T-green).
- `forward` is in `pane_dispatch.rs`, and `lib.rs` declares the four modules.
- `rename.rs` is declared at `profile/mod.rs:15`. It is a routed stub, not an empty file; that deviation is accepted (see Spec compliance).

### AC 7: gates, sizes, changelog, formatting

- Gates: T-green's Tier 1 table.
- Sizes (`wc -l`): `control_server.rs` is 837 lines and `serve.rs` is 838. The new source files are 6 to 101 lines, and the test file is 360.
- `CHANGELOG.md` has an `## [Unreleased]` → `### Enhancements` entry linking #669 and #633.
- I re-ran `rustfmt --check --edition 2021` on the six new `.rs` files, including the test file: exit 0.
- The hunks in `control_server.rs`, `serve.rs` and `lib.rs` are only the arm, the parameter, the field, the handle lines and doc lines, so nothing was reformatted.

## Spec compliance

Production code is unchanged since pass 1. I re-checked each point against HEAD.

### MO decisions

Each one is implemented as stated.

- **0. The hub is the store and wires no adapters.** The hub registers nothing, and `pane_wiring.rs` is empty.
- **1. Plain functions.** One arm (`control_server.rs:117`) calls `pane_dispatch::forward`, which calls `panes::dispatch` or `profile::dispatch`. There is no function-pointer registry.
- **2. Both handles go to both dispatchers.** See `pane_dispatch.rs:82, 84`. Each dispatcher gets them in the issue's order.
- **3. A `PaneReply` is the JSON-RPC result.** It goes through `reply_line` and then `encode_response`.
  - This departs from the existing convention of an error with `data.reason`.
  - The departure and its reason are written in the module doc of `pane_dispatch.rs`, in the CHANGELOG, and in `decisions.md` (the F entry).
  - `holler-proto` is untouched.
- **4. State loads before the first connection.** `serve.rs:366` runs before `:483`. Both `load` docs give the corrupt-file handling to #639 and #661.
- **5. No `send_prompt` change.** It is not in the diff.

### Files section

- The arm sits before the `control/` arm, as asked.
- `handle_control_conn` and `dispatch_control` each gain one bundled parameter.
- `SharedState` gains one bundle field. The brief allows this ("or one bundle struct").
- `accept_loop` gains exactly one parameter.
- The `#184` allow note now says 6 handles: registry, roster, hygiene, lockout, the pre-auth semaphore and `pane_deps`. It keeps its link.

### Deviations

Both are recorded, and both are accepted.

**`profile/rename.rs` is a routed `not-implemented` stub, not an empty file.** I re-checked the issue texts:
- #665 says "#669 pre-creates the empty `holler-hub/src/profile/rename.rs`, already declared in `profile/mod.rs`, so you edit none of #661's files".
- #665's blast radius lists only `rename.rs` on the hub side.
- #661's handler list has six methods, and `profile/rename` is not one of them.

So with an empty file, #665 would have to add the route in #661's `profile/mod.rs`.

The deviation is recorded in four places: F's Deviations, `decisions.md`, the module docs and the CHANGELOG. It is not silent, it does not change behaviour, and it is the smallest fix, so it is not REWORK. It is not an ADVISORY-HOLD either: F did not implement the defective text; F fixed it in the way A Phase 3 W6 proposed. The issue text needs an amendment; see Advisory notes.

**The first `mod.rs` files under any `src/`** (A Phase 3 W5). This is recorded in `decisions.md` and in both module docs. The exception is limited to the two module roots, which have to sit inside their owners' globs.

## Quality audit

### Correctness and failure handling

- `forward`'s branch for a method in neither list answers `MethodNotFound` through `encode_error`. It does not panic, and it does not write an empty line.
- `reply_line` uses `unwrap_or_default()`, which cannot fire. If it ever did fire, it would send a `null` result. A client cannot decode that, so it could never read as success.
- The new methods are reachable only over the control socket:
  - `dispatch_control`'s only caller is `handle_control_conn` (`control_server.rs:44`).
  - The admin loop goes through `dispatch_allowlisted` (`circuit/admin.rs:276`), which does not list them.
  - Body sockets are bound by the closed `CATALOG`.
- `log_control` logs only the method and the id, and only to stderr.
- Nothing writes to disk yet, so there are no writes to lose under concurrency.

### Build guards

- The added `src/` lines have no `unwrap()`, `expect(`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` or `unsafe` (checked with grep).
- The only `#[allow]` in `src/` is the edited `#184` note, and it keeps its link. The test file's three `allow`s each carry `// #669`.
- No file is at or above 900 lines. The largest touched file is `serve.rs` at 838.
- There is no dead code. Every new item has a caller or is public API that the issue requires: `check_membership`, and the empty `pane_wiring` module.

### Protocol

No protocol change.
- `CATALOG`, the golden files and `holler-proto` are untouched.
- `docs/protocol/v2.md` and ADR-0021 are out of scope by the brief; they belong to #634.

### Tests

- The tests run in process over `UnixStream::pair()`, with a real `Registry`, `Roster` and `Lockout`, as the issue prescribes.
- There are no sleeps. Each reply line is read with a bounded `timeout` of 10 s.
- The RED-first evidence and the mutation checks are in T's handoffs.
- The file no longer touches `$HOME` (verified above).

### Documentation

- `CHANGELOG.md` has an entry under `## [Unreleased]` → `### Enhancements` that links #669 and #633. Its claims match the code.
- There is no new log event, CLI surface or protocol field, so no README or `docs/` change is due.

### Privacy

I grepped all 1,804 added lines, including the handoffs, for:
- home paths;
- user, account and machine names;
- tailnet names and private domains;
- IPv4 addresses and e-mail addresses;
- secret patterns.

The only hits are harmless:
- `127.0.0.1` (3 hits);
- `ANTHROPIC_API_KEY`, a variable name with no value (2 hits);
- `noreply@anthropic.com` (1 hit, in a description of commit trailers);
- `github.com` issue links;
- the fixture host `kiwi`, which `main` already uses in 27 files.

There is nothing personal and no secret.

### Commit and PR hygiene (advisory)

- The commit subjects are Conventional Commits: `docs(#669): …` and `chore(#669): …`.
- The trailers are `Co-Authored-By: Claude <noreply@anthropic.com>`, and `Claude Sonnet 5.5` on the brief commit. None has the session link that `CONTRIBUTING.md` describes.
  - These are the workflow script's own commits. This repo's `CLAUDE.md` has the disclosure added after the PR opens.
  - The squash commit is what lands on `main`.
- No PR exists yet.

## Scope check

The delivery matches the brief's scope. Nothing outside the plumbing changed:
- no handler logic, persistence, CLI or adapter;
- no change to `send_prompt`, `control.rs`, `holler-proto` or `holler-pane`.

F added two things the brief does not name:
- `PaneDeps::load`, `forward` and `reply_line`. These are the issue's "forwarding helper" plus the one shared encoder, and each has callers.
- The `rename.rs` stub, which is the accepted deviation described above.

The rework changed only the test file and added no surface. Nothing is missing.

## Verdict

**PASS.** Ready for O.
- All eight acceptance criteria are met, each proven by a named test or by evidence.
- MO decisions 0 to 5 are implemented as stated.
- The two deviations are recorded and justified.
- Pass 1's REWORK item is resolved. I confirmed it by running the test binary with an empty scratch `HOME`.

## Advisory notes

None of these block.

### New this pass

**The probe comment overstates which handlers touch the state dir.** The comment at `pane_dispatch_test.rs:186-188` says "The other control handlers resolve the state dir from the environment". Only five do:

| Handler | Where it resolves the state dir |
|---|---|
| `control/status` | `control_status.rs:26` |
| `control/caps` | `control_server.rs:103`, through `read_listening_here` (`:623-625`) |
| `control/query_local` | `control_server.rs:226, 230`, through `read_listening_here` |
| `control/say` | `control_server.rs:303` |
| `control/interrupt` | `control_server.rs:370` |

The overstatement errs on the safe side. Reword it when the harness moves to `tests/common/` (A-dup W3).

**`#678` is the playbook's issue, not Holler's.**
- It appears at `handoff-T-green.md:60` and `decisions.md:120`.
- It means the playbook's test-only routing issue (cited in `coding-pipeline-logic.mjs`). Holler has no #678 yet.
- These files are pipeline scratch (`pipeline-conventions.md` §1). Write `playbook#678` if they are kept.

**The post-rework workspace test count is not in a file.** T-green's rework section leaves it to T's return message. Only the self-contained `pane_dispatch_test` binary changed, so the earlier 937-test pass plus the 10/10 run cover it.

### Carried from pass 1, unchanged, for O

**The rename route is checked by reading only.** `profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub` cannot tell `rename::dispatch` apart from the `_` arm, because both answer `not-implemented`. #665's first behavioural test will pin the route.

**Issue texts:**
- In #669 and #665, replace "pre-created empty" with "a `not-implemented` stub that `profile::dispatch` already routes to".
- Tell #661's brief to keep the `"profile/rename" => rename::dispatch(..)` arm when it rewrites `profile::dispatch`.

**Still open from A Phase 3:**
- W2: `check_membership` cannot see the stored pane record.
  - Decide where `pane-in-other-profile` lives before #639 and #661 start.
  - No release should ship #639 without #661 while the stub accepts every profile.
- W7: `pane_wiring.rs` is a permanent empty placeholder. #649 cannot remove it without editing `lib.rs`.

**A-dup W1 and W3:**
- Move `sample_pane` to `holler-pane-testkit` (#638).
- Move the harness to `crates/holler-hub/tests/common/` the first time #639 or #661 needs it.

**Coverage that has no automated test:**
- The `serve.rs` wiring has no automated test of its own; A W3 ruled out new `pub` items. It is covered by the single call site, by the holler-cli suites through `holler hub serve`, and by F's manual live-hub check.
- `forward`'s branch for a method in neither list cannot be reached from `tests/`.

**PR:**
- When the PR opens, add the `CONTRIBUTING.md` AI disclosure with `gh pr edit`.
- The body should close #669 and reference #633.
- The squash commit should carry the `Co-Authored-By` trailer with a session link.

## Pass history

- **Pass 1** (2026-10-09 06:56 MDT, head 8275e73): REWORK, test-only. The `control/status` probe could write `$HOME/.holler/hub/identity.key`, and it could not fail because of the new arm. The full text is in commit cc674a0.
- **T rework** (commit 820e385, 07:13 MDT):
  - the probe now sends `control/roster`;
  - `fresh_deps` now calls `PaneDeps::load`;
  - one comment is corrected.

  There is no `src/` change.
- **A-dup pass 2** (commit 31fe2bd, 07:18 MDT): PASS, with 2 warns.
- **Pass 2** (this pass, head 31fe2bd): PASS.
