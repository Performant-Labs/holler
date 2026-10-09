# Handoff-S: Phase 10 - #669 hub plumbing for `pane/*` and `profile/*` (skeleton slice b), spec audit

**Date:** 2026-10-09
**Branch:** issue-669-implementation (at 8275e73)
**Issue:** #669 (slice b of epic #633)
**Brief:** `docs/handoffs/669-brief.md` (Revision 1)
**Handoffs reviewed:** `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`, `decisions.md`, `evidence.md`

**Diff audited:** `git diff origin/main...HEAD` (merge base f2602ba), 20 paths. I read every production file and the test file in full.

I also read the code that the change relies on:
- `control_server.rs:1-200, 461-475, 703-713`
- `serve.rs:338-590`
- `control_status.rs:23-63`
- `identity.rs:90-121`
- `state.rs:33-65`
- `roster.rs:536-555`
- `holler-pane/src/reply.rs:48-102`
- `holler-proto/src/methods.rs:112-143`

I read issue #669 with `gh issue view`, and also #661 and #665 for the rename seam.

## A precondition

Met.
- `handoff-A.md` (Phase 3) is **PASS**, with 8 warns and no block.
- `handoff-A-dup.md` (Phase 7) is **PASS**, with 3 warns. All three are about the test file or later stories.

## T precondition

Met.

**RED** (`handoff-T-red.md`): the test target did not build. There were 12 errors, all caused by the missing feature. A build failure cannot show that the assertions are right, so T also ran the file against a throwaway stub:
- 10 of 10 tests passed.
- With the forwarding arm disabled, the three forwarding tests failed on their assertions.

**GREEN** (`handoff-T-green.md`):
- 10 of 10 tests passed, three runs in a row.
- The workspace suite passed: 937 tests, no failure.
- T repeated the arm-disabled mutation on the real code. The same three tests failed, and T restored the file.
- No blocking issues.

**Tier 1:**
- T-green: `lint.sh` exit 0, `changelog-check.sh` ok, `clippy --workspace --all-targets -D warnings` clean, `cargo machete` clean.
- F recorded `cargo build --workspace`.

## Acceptance criteria

All eight criteria are met. The in-file test for AC 2 has a defect, which is the one REWORK item.

| AC | Criterion (short) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | Forwarding: every `pane/*` and `profile/*` method gets a JSON-RPC result that `into_result` reads as `Err(NotImplemented)` | Two tests at `pane_dispatch_test.rs:130` and `:147`. Mutation-checked by T-red and T-green: both fail with the arm disabled. Details below. | MET |
| 2 | Nothing else changes | `unknown_methods_still_answer_method_not_found` (`:164`), plus the unchanged workspace suites. Details below. | MET by the suites; the in-file probe is REWORK item 1 |
| 3 | State handles: built once, shared as `Arc`s, not `Clone` | Code reading of `serve.rs` and two tests (`:207`, `:262`). Details below. | MET |
| 4 | Dispatcher signatures and reply encoding | Two direct-call tests (`:283`, `:298`) and `reply_line`. Details below. | MET |
| 5 | `check_membership` returns `Ok(())` | `check_membership_accepts_any_pane` (`:339`) | MET |
| 6 | Manifest, reuse and module layout | `Cargo.toml`, `Cargo.lock`, `testkit_links` (`:354`), `lib.rs`. Details below. | MET |
| 7 | Gates, file sizes, changelog, formatting | T-green's Tier 1 table, `wc -l`, my own `rustfmt --check`. Details below. | MET |
| 8 | `git diff --name-only` stays inside the blast radius | 20 paths: the 12 code, manifest and changelog paths of the radius, plus `docs/handoffs/669-brief.md` and `docs/handoffs/669/*` | MET |

### AC 1: forwarding

The criterion: every method of both lists reaches the stub over `UnixStream::pair()`, and gets a JSON-RPC result that `PaneReply::into_result` reads as `Err(NotImplemented)`.

The proving tests are `every_pane_method_is_forwarded_to_the_stub_not_method_not_found` (`:130`) and `every_profile_method_is_forwarded_to_the_stub_not_method_not_found` (`:147`). Each one walks its `holler_proto` method list on one connection and asserts four things:
- the reply has no `error` key;
- the request id is echoed;
- `result` decodes as a `PaneReply`;
- `into_result()` gives `Err(PaneError::NotImplemented)`, which pins the wire code `not-implemented`.

No binary is spawned, and there is no copy of `support::Hub`.

### AC 2: nothing else changes

The criterion: unknown methods still answer `-32601`, existing `control/*` methods are unchanged, existing tests are unchanged, `CATALOG` still has 22 rows, and no golden file changes.

- `unknown_methods_still_answer_method_not_found` (`:164`) sends `control/x` and `foo/bar`. It also sends `pane/frobnicate` and `profile/frobnicate`, which would catch a guard that matches on the prefix.
- The existing `control/*` methods are covered by the unchanged suites (937 passed). The holler-cli suites drive `roster`, `say`, `wait`, `hold`, `status` and the rest through a real `holler hub serve`, so they run through the changed `accept_loop` and `dispatch_control`.
- No existing test, no `holler-proto` file and no golden file is in the diff.

The in-file probe `an_existing_control_method_still_answers_through_the_new_dispatcher` (`:185`) does not prove this criterion, and it has a side effect. See REWORK item 1.

### AC 3: state handles

The criterion: `load` returns empty state; the state is built once in `build_shared_state`, next to `Holds::load`; it is held as `Arc`s in `SharedState`; the types are not `Clone`; and two connections share the handle.

The types:
- Both are fieldless (`panes/mod.rs:27`, `profile/mod.rs:31`).
- Both `load` functions return `Self` (`:33`, `:37`).

The wiring, checked by reading:
- `PaneDeps::load(state)` is at `serve.rs:366`, right after `Holds::load` (`:360-363`).
- `build_shared_state` has one call (`:466`), which runs before the accept loop starts.
- The accept loop clones the bundle for each control connection (`:570`).

The tests:
- `two_connections_share_the_one_pair_of_state_handles` (`:207`) checks `ptr_eq`. It also checks `strong_count == 3` while both connections are live.
- `the_state_types_do_not_derive_clone_so_a_connection_cannot_fork_a_copy` (`:262`) uses a stable autoref probe. It shows that both state types are not `Clone` and that `PaneDeps` is.

The issue asks to compare "the `Arc`s the dispatcher receives". That is tested at the bundle seam, as A Phase 3 W3 recommended. `forward` passes `&deps.panes` and `&deps.profiles` unchanged (`pane_dispatch.rs:82, 84`).

### AC 4: dispatcher signatures

The criterion: async `panes::dispatch(.., &PaneState, &ProfileState)` and `profile::dispatch(.., &ProfileState, &PaneState)`; `PaneReply::failure(NotImplemented)` sent as a result through `encode_response`; and `watch` is long-poll.

- `panes_dispatch_takes_both_handles_and_answers_not_implemented_as_a_result` (`:283`) and `profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub` (`:298`) call both dispatchers directly, in the issue's argument order. That pins the signatures at compile time.
- Encoding: `reply_line` calls `encode_response` (`pane_dispatch.rs:98-100`).
- Watch: each dispatcher returns one `String`. In the profile table test, `profile/watch` is followed by two more requests on the same connection.

### AC 6: manifest, reuse and module layout

- `Cargo.toml` adds `holler-pane`, and `holler-pane-testkit` as a dev-dependency.
- `Cargo.lock` gains exactly those two edges.
- `testkit_links` (`:354`) uses `use holler_pane_testkit as _;`.
- `cargo machete` is clean (T-green).
- The forwarding helper is in `pane_dispatch.rs`, and `lib.rs` declares the four modules.
- `rename.rs` is declared at `profile/mod.rs:15`. It holds a stub function instead of being empty. That deviation is documented and accepted; see Spec compliance.

### AC 7: gates, sizes, changelog, formatting

The criterion: build, clippy, test, machete, lint and changelog all pass; no file is over 900 lines; there is a changelog entry; new files are rustfmt-clean; and no file is reformatted.

- Gates: the T-green table.
- Sizes (`wc -l`): `control_server.rs` is 837 lines and `serve.rs` is 838. The new source files are 6 to 101 lines, and the test file is 358.
- `CHANGELOG.md` has an `## [Unreleased]` → Enhancements entry linking #669 and #633.
- I re-ran `rustfmt --check --edition 2021` on the six new `.rs` files: exit 0.
- The hunks in `control_server.rs`, `serve.rs` and `lib.rs` are only the arm, the parameter, the field and handle lines, and doc lines, so nothing was reformatted.

## Spec compliance

### MO decisions

Each one is implemented as stated.

- **0. The hub is the store and wires no adapters.** The hub registers nothing, and `pane_wiring.rs` is empty.
- **1. Plain functions.** One arm (`control_server.rs:117`) calls `pane_dispatch::forward`, which calls `panes::dispatch` or `profile::dispatch`. There is no function-pointer registry.
- **2. Both handles go to both dispatchers.** See `pane_dispatch.rs:82, 84`. Each dispatcher gets them in the issue's order.
- **3. A `PaneReply` is the JSON-RPC result.** There is one encoder, `reply_line`, which goes through `encode_response`. `holler-proto` is untouched.
  - This departs from the existing convention of an error with `data.reason`. The reason is that the pane codes do not fit the closed wire `Code` table.
  - The departure and its reason are written in the module doc of `pane_dispatch.rs` and in the CHANGELOG.
  - `decisions.md` records the choice by reference to MO decision 3 (F entry).
- **4. State loads before the first connection, and corrupt-file handling lives inside `load`.** `serve.rs:366` runs before `accept_loop` is spawned (`:483-485`). `load` takes `&HubState` and is where #639 and #661 put their fail-closed handling.
- **5. No `send_prompt` change.** It is not in the diff.

### Files section

- The arm sits before the `control/` arm, as asked.
- `handle_control_conn` and `dispatch_control` each gain one bundled parameter.
- `SharedState` gains one bundle field. The brief allows this ("or one bundle struct").
- `accept_loop` gains exactly one parameter.
- The `#184` allow note was updated so it stays true (6 handles), and it keeps its link.

### Deviations

Both are recorded, and both are accepted.

**`profile/rename.rs` is a one-function stub, not an empty file.** It is recorded in four places: F's Deviations item 1 (A Phase 3 W6, option 1), the F entry in `decisions.md`, the module docs and the CHANGELOG.

The issue texts contradict each other:
- #665 says #669 "pre-creates the empty `rename.rs`, already declared in `profile/mod.rs`, so you edit none of #661's files".
- #661's scope lists six handlers, and `profile/rename` is not one of them.
- So with an empty `rename.rs`, nothing would route the method to it, and #665 would have to edit `profile/mod.rs`.

F's stub, routed at `profile/mod.rs:56`, is the smallest change that makes the issue's stated purpose achievable. Behaviour is the same either way: `profile/rename` answers `not-implemented`.

The deviation is not silent, and reverting it would bring the defect back, so it is neither REWORK nor a hold. The issue text should be amended; see Advisory notes.

**The first `mod.rs` files under any `src/`** (A Phase 3 W5). This is recorded in `decisions.md` and in both module docs. The reason is that each module root has to sit inside its owner's glob, `panes/**` or `profile/**`. The exception is limited to those two roots.

## Quality audit

### Correctness and failure handling

- `forward` has a branch for a method in neither list. It answers `MethodNotFound` through `encode_error`. It does not panic, and it does not write an empty line.
- `reply_line` uses `unwrap_or_default()`, which cannot fire: a `PaneReply` is a bool and two optional values. If it ever did fire, it would send a `null` result. A client cannot decode that, so it could never read as success.
- The new methods cannot be reached over `admin/*`, because `dispatch_allowlisted` does not list them. They cannot be reached over a body socket either, because they are outside `CATALOG`.
- `log_control` logs only the method and the id, never the params.
- Nothing writes to disk yet, so there are no writes to lose under concurrency.

### Build guards

- The added production lines have no `unwrap()`, `expect(`, `panic!`, `unreachable!`, `todo!` or `unsafe` (checked with grep).
- The only `#[allow]` change in production is the edited `#184` note on `accept_loop`, and it keeps its link.
- The test file's three `allow`s each carry `// #669`.
- No file is at or above 900 lines.
- There is no dead code. Every new item either has a caller or is public API that the issue requires: `check_membership`, and `pane_wiring.rs`, an empty module that the issue mandates (see Advisory notes).

### Protocol

No protocol change.
- The control socket is internal and is not on the wire.
- `CATALOG`, the golden files and `holler-proto` are untouched.
- `docs/protocol/v2.md` and ADR-0021 are out of scope by the brief; they belong to #634.

### Tests

- The tests run in process over `UnixStream::pair()`, with a real `Registry`, `Roster` and `Lockout`, as the issue prescribes.
- There are no sleeps. Each read is a bounded `timeout` of 10 s on the reply line.
- The RED-first evidence and the mutation checks are in T's handoffs.

One defect, which I verified, is described next.

### Finding: one test writes into the real `$HOME/.holler` and cannot fail because of the new arm

The test is `an_existing_control_method_still_answers_through_the_new_dispatcher` (`pane_dispatch_test.rs:184-202`). It has two problems.

**Problem 1: a side effect outside the test's sandbox.** The test sends `control/status`, and here is what the handler does:
1. The handler is `control_status::status_doc` (`control_status.rs:23-63`).
2. The test process sets no `HOLLER_STATE_DIR`, so the handler resolves the state dir from the environment: `$HOME/.holler` (`state.rs:38-58`).
3. It calls `identity::ensure` (`identity.rs:101-121`). That call creates `$HOME/.holler/hub/`. When no key exists, it **generates and saves a new hub X25519 private key** at `$HOME/.holler/hub/identity.key`, with mode 0600.
4. It also reads `listening.json` and `advertise.json` from that directory.

I confirmed this by running the built test binary for this one test with `HOME` pointed at an empty scratch directory. The test passed and left `.holler/hub/identity.key` (`-rw-------`) behind. I then deleted the scratch directory.

The consequences:
- On a developer machine with no hub identity, such as a body-only machine, `cargo test --workspace` now creates a hub private key in the operator's real state dir.
- A later `holler hub serve` on that machine with the default state dir would adopt that key as its long-lived identity.
- On a machine that runs a hub, the test reads the live hub's state dir.

This contradicts three things:
- the test file's own contract: "a connection never needs the state dir, so none is touched" (`:56-57`);
- A Phase 3 W8: "`tempfile::tempdir()` with `HubState::from_root` for state";
- the per-test state-dir isolation that `docs/testing.md` describes.

F described the test as "reads, never writes" (handoff-F, "Tests that look wrong" item 2), and T-green accepted it on that basis. That description is wrong.

**Problem 2: no proof value.** `control/status` has its own exact match arm at `control_server.rs:100`, which comes before the new arm at `:117`. This request can never reach the new arm, whatever its guard matches. So the assertion that the request was not "swallowed by the pane arm" cannot fail.

The methods the new arm could swallow are the ones matched after it:
- the `control/` prefix arm (`:118`, `dispatch_session_control`);
- the fallback.

### Documentation

- `CHANGELOG.md` has an `## [Unreleased]` → Enhancements entry linking #669 and #633. Its claims match the code.
- There is no new log event, CLI surface or protocol field, so no README or `docs/` change is due.

### Privacy

I grepped the added lines, including the handoffs, for:
- personal names and home paths;
- hostnames and tailnet names;
- IP addresses and e-mail addresses;
- key and secret patterns.

The only hits are harmless:
- `kiwi`, an established fixture label that appears on 94 lines in 27 files on `main`;
- `127.0.0.1`;
- the env var name `ANTHROPIC_API_KEY`, which is a name with no value.

There is nothing personal and no secret.

### Commit and PR hygiene (advisory)

- The commit subjects are Conventional Commits: `docs(#669): …` and `chore(#669): …`.
- The trailers are `Co-Authored-By: Claude <noreply@anthropic.com>`, and `Claude Sonnet 5.5` on the brief commit. None has the session link that `CONTRIBUTING.md` asks for.
- No PR exists yet.

## Scope check

The delivery matches the brief's scope. Nothing outside the plumbing changed:
- no handler logic, persistence, CLI or adapter;
- no change to `send_prompt`, `control.rs`, `holler-proto` or `holler-pane`.

F added two things the brief does not name:
- `PaneDeps::load`, `forward` and `reply_line`. These are the issue's "forwarding helper" plus the one shared encoder, and each has callers.
- The `rename.rs` stub, which is the accepted deviation described above.

Nothing is missing.

## Verdict

**REWORK**, test-only: no `src/` change is required.

1. **`crates/holler-hub/tests/pane_dispatch_test.rs:184-202`, `an_existing_control_method_still_answers_through_the_new_dispatcher`: send `control/roster` instead of `control/status`.** Use a request id such as `h-roster-1`.

   Why `control/roster`:
   - It reaches `dispatch_session_control` through the `control/` arm, which comes right after the new pane arm (`control_server.rs:118`, `:148`). The test then pins the arm order that the brief asked for.
   - It reads only the in-memory `Roster` (`roster_control`, `control_server.rs:461-475`; `Roster::rows_matching`), so nothing touches `$HOME`.

   Assert three things:
   - the reply has no `error` key;
   - `reply["result"]["rows"] == json!([])`, which is what a fresh roster returns;
   - the result does not decode as a `PaneReply`, as the test checks now.

   Do not switch to another method whose handler resolves the state dir from the environment. `control/status`, `control/caps`, `control/query_local`, `control/say` and `control/interrupt` all do.

   After this change, the `connect()` doc's "none is touched" (`:56-57`) becomes true. Update the test's comments and messages that name `control/status`.

Everything else complies. On the next S pass, this item is the only gate.

## Advisory notes

None of these block.

**Optional fold-ins for T while it edits the file:**
- A-dup W2: have `fresh_deps` (`:95-103`) call `PaneDeps::load(&state)`. Today no test calls the constructor that `build_shared_state` uses; its only caller is `serve.rs:366`.
- The comment at `:133-134` says that after `pane/watch` "the next request still gets its own reply". But `pane/watch` is the last entry of `PANE_METHODS`, so no request follows it in that loop. Either add one more call after the loop or cut the clause. The profile loop does show this behaviour.

**A test that cannot see the rename route:** `profile_dispatch_takes_both_handles_and_routes_rename_to_the_stub` cannot tell `rename::dispatch` apart from the `_` arm, because both answer `not-implemented`. I checked the route at `profile/mod.rs:56` by reading. #665's first behavioural test will pin it.

**For O, on the issue texts:**
- In #669 and #665, replace "pre-created empty" with "a `not-implemented` stub that `profile::dispatch` already routes to".
- Tell #661's brief to keep the `"profile/rename" => rename::dispatch(..)` arm when it rewrites `profile::dispatch`. Otherwise #665 has to edit #661's file after all.

**Still open for O from A Phase 3:**
- W2: `check_membership` cannot see the stored pane record. #661's `pane-in-other-profile` rule needs a home, decided before #639 and #661. Also, no release should ship #639 without #661 while the stub accepts every profile.
- W7: `pane_wiring.rs` is a permanent empty placeholder. #649 cannot remove it without editing `lib.rs`.

**A-dup W1 and W3:**
- Move `sample_pane` to `holler-pane-testkit` (#638).
- Move the in-process control harness to `crates/holler-hub/tests/common/` the first time #639 or #661 needs it.

**Coverage that has no automated test:**
- The `serve.rs` wiring has no automated test of its own; A W3 ruled out new `pub` items and test hooks. It is covered by the single call site, by the holler-cli suites through `holler hub serve`, and by F's manual live-hub check.
- `forward`'s branch for a method in neither list cannot be reached from `tests/`.

**PR:**
- When the script opens the PR, add the `CONTRIBUTING.md` AI disclosure with `gh pr edit`.
- The body should close #669 and reference #633.
- The squash commit should carry the `Co-Authored-By` trailer with a session link.
