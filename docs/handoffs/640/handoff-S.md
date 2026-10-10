# Handoff-S: Phase 10 - #640 part 2 of 3: the socket transport, `HerdrAdapter` and a simulated Herdr (spec audit)

**Date:** 2026-10-09
**Branch:** issue-640-implementation (head `0fbe0e5`; diff `origin/main...HEAD` = `3bdd129...0fbe0e5`, excluding `docs/handoffs`)
**Issue:** #640 (epic #633), part 2 of 3. The PR says `Part of #640`.
**Source of truth:** `docs/handoffs/640-brief.md` (all of it), `gh issue view 640`
**Handoffs reviewed:** `docs/handoffs/640/handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`,
`handoff-A-dup.md`, `evidence.md`, `decisions.md`; the outside diff gate `docs/handoffs/640-diff-result-r2.md`
(DeepSeek V4 Pro, PASS, git-ignored)
**Read in full:** `src/adapter.rs`, `src/transport.rs`, the `lib.rs`/`Cargo.toml`/`Cargo.lock`/`CHANGELOG.md` hunks,
`tests/adapter_test.rs`, `tests/transport_test.rs`, `tests/adapter_conformance_test.rs`; `tests/wire_herdr/` by its
public surface and the helpers the gate questioned.

## A precondition

**Met.** `handoff-A.md` (Phase 3, plan) is `PASS` (0 block, 7 warn). `handoff-A-dup.md` (Phase 7, anti-duplication) is
`PASS` (0 block, 4 warn).

## T precondition

**Met.** `handoff-T-red.md`: 34 new tests, all failing on `NotImplemented` assertions (not compile errors), the 62
part-1 tests green. `handoff-T-green.md`: 97/97 in `cargo test -p holler-adapter-herdr`, "Blocking issues: None",
the AC 14 mutant table recorded, and every Tier 1 gate clean (the 4 `holler-cli --test logging_test` failures are
environmental, a live hub on this machine, and pass 11/11 with an isolated `HOLLER_STATE_DIR`; `holler-cli` does not
depend on this crate). RED then GREEN is confirmed. I did not re-run Tier 1 or Tier 2; the greps below are read-only.

## Acceptance criteria

The issue's lines covered by this part (brief, Scope): the conformance suite, the grid read-back with the swapped
conversion failing, and the version line at adapter level. The scratch-session line is part 3's.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| Issue 1 | Passes the conformance suite from #638 | `adapter_conformance_test.rs`: `the_adapter_passes_the_suite_from_an_empty_herdr`, `..._with_a_root_pane`, `..._over_a_real_socket` each assert `run_herdr_conformance(..) == Ok(())` | met |
| Issue 2 | r2c1 / r1c2 land in their cells, read back, swapped conversion fails | `adapter_test.rs::r2c1_and_r1c2_land_in_their_cells_and_read_back` (literal trees, snapshot order, `to_string`, split params) + AC 14 mutants in `handoff-T-green.md` | met |
| Issue 3 | Each supported version works against the fake's two versions; unknown refused with `herdr-version-unsupported` | `version_and_the_gate` (22 is `Ok`; 99 and `None` refused at `connect_with`; `version()` re-gates after `set_protocol(Some(99))`) | met (as the brief reads it, operator items 4/5) |
| 1 | Writes the request line, returns the reply line as text | `transport_test.rs::exchange_writes_the_request_line_and_returns_the_reply_line` | met |
| 2 | One connection per request | `each_request_opens_its_own_connection` (two accepts, one line each, then EOF) | met |
| 3 | Missing / non-socket path is `Unavailable` naming it | `a_missing_or_non_socket_path_is_unavailable_naming_it` | met |
| 4 | Too-long path is `Unavailable`, no panic | `a_socket_path_too_long_for_the_os_is_unavailable` (asserts a 200-byte path) | met |
| 5 | Silent server is `Timeout{herdr.ping}` by deadline+2s | `a_silent_server_is_timeout_by_the_deadline` | met |
| 6 | Dripping server is `Timeout` (deadline bounds the whole reply) | `a_dripping_server_is_timeout_by_the_deadline` | met |
| 7 | Close without reply is `Unavailable` | `a_server_that_closes_without_replying_is_unavailable` | met |
| 8 | Over-limit reply is `Unavailable` naming the limit | `a_reply_over_the_limit_is_unavailable` (connection held open, so only the cap ends it; spot-check with no cap gives `Timeout`) | met |
| 9 | EOF without newline returns the bytes | `a_reply_ended_by_eof_without_a_newline_is_returned` | met |
| 10 | Non-UTF-8 reply is `Unavailable` | `a_non_utf8_reply_is_unavailable` | met |
| 11 | Passed deadline is `Timeout`, no connect | `a_passed_deadline_is_timeout_without_connecting` (non-blocking `accept` is `WouldBlock`) | met |
| 12 | No error echoes typed text | `no_transport_error_echoes_typed_text` (silent, closing, missing; `Display` and `Debug`) | met |
| 12a | One answer per wire condition; workers end by themselves | `one_wire_condition_gives_one_answer_at_the_deadline` (20 x exactly `Timeout`, <=20 connections, each reaches EOF within 2s, no worker panic) | met (see advisory 5) |
| 13 | Grid acceptance (above) | `r2c1_and_r1c2_land_in_their_cells_and_read_back` | met |
| 14 | Mutants (a), (b), (c) fail as derived | `handoff-T-green.md` table: (a) fails 13+16, (b) fails 13(+16)+20, (c) fails 13 only; observed values match the oracle | met |
| 15 | Closed pane's space goes to its sibling | `a_closed_panes_space_goes_to_its_sibling` | met |
| 16 | Misplaced pane is `Unavailable`, left in place, no close | `a_pane_that_lands_elsewhere_is_unavailable_and_left_in_place` (spot-check: ignoring the read-back fails it) | met |
| 17 | Split target closed under the adapter is `Unavailable`, not `PaneNotFound` | `a_split_target_closed_under_the_adapter_is_unavailable_not_pane_not_found` (spot-check: no rewrite gives `PaneNotFound`) | met |
| 18 | Occupied cell sends no mutating request | `an_occupied_cell_sends_no_mutating_request` (exactly `session.snapshot`, `layout.export`) | met |
| 19 | Nesting refused before any split, (a) and (b) | `nesting_is_refused_before_any_split` | met |
| 20 | Missing workspace created only for r1c1, with `{label, focus:false}` | `a_missing_workspace_is_created_only_for_r1c1` | met |
| 21 | Another tab neither lists nor places | `a_pane_in_another_tab_neither_lists_nor_places` (also asserts every `layout.export` is the grid tab) | met |
| 22 | Snapshot lists every workspace by label, duplicates included | `snapshot_lists_every_workspace_by_label` | met |
| 23 | Session / workspace errors send nothing; duplicate label is `Unavailable` | `session_and_workspace_errors_send_nothing` | met |
| 24 | Keys go out verbatim | `keys_go_out_verbatim` | met |
| 25 | `read` asks for recent text, trims; `read(p,0)` sends `lines:1`, returns `""` | `read_asks_for_recent_text_and_trims` | met |
| 26 | Version and the gate | `version_and_the_gate` | met |
| 27 | Config validated before any request; order and message | `config_is_validated_before_any_request` (9 cases incl. `Duration::MAX`, label order, `session` before `socket`) | met |
| 28 | One deadline covers every exchange of a call | `one_deadline_covers_every_exchange_of_a_call` (exact `Instant` equality, `before+t <= d <= after+t`) | met |
| 29 | Garbled reply is `Unavailable` everywhere | `a_garbled_reply_is_unavailable_everywhere` (connect + all 7 methods) | met |
| 30 | Base-36 pane numbering, returned verbatim | `the_fake_numbers_panes_in_base_36` | met |
| 31-33 | Conformance: empty, root pane, real socket | the three conformance tests; the socket one follows the brief's construction order | met |
| 34 | No method off the allow-list | `no_case_calls_a_method_off_the_allow_list` (also asserts each fake saw requests) | met |
| 35-39 | Independence and shape greps | re-run by S: all five print nothing | met |
| 40 | Gates | `handoff-T-green.md` Tier 1 table (workspace run green except the 4 environmental `logging_test` cases) | met (CI is the final word on the workspace run) |
| 41 | Dependencies | `Cargo.toml` hunk: only `tempfile = { workspace = true }` as a dev-dep with a consumer comment; `Cargo.lock` +1 line; no `holler-hub`; T's `cargo tree` output | met |
| 42 | Every touched file < 900 lines; clippy size/complexity | `wc -l`: `adapter.rs` 445, `transport.rs` 286, `lib.rs` 41, `adapter_test.rs` 645, `transport_test.rs` 421, `adapter_conformance_test.rs` 98, `wire_herdr/mod.rs` 650, `serve.rs` 84, `CHANGELOG.md` 638; clippy `-D warnings` clean | met |
| 43 | CHANGELOG entry after the part-1 entry, linking #640 and #633 | `CHANGELOG.md` hunk at line 139 (also links #649) | met |

Every test asserts observable behaviour (wire params, returned values, error variants and message substrings, the
fake's tree), not implementation details, and T's spot-checks show the key ones fail when the behaviour is removed.

## Spec compliance

Each "Decision already made" checked against the code:

- **D2/D3/D4 (transport).** New connection per exchange, no `shutdown`, bytes after `\n` ignored, EOF-with-bytes
  returned, empty EOF `unavailable`, 16 MiB cap with reading stopped (`transport.rs:178-208`). One deadline; worker
  thread with per-syscall socket timeouts from `saturating_duration_since`, `Interrupted` retried, zero time left is
  `Timeout` without the syscall (`:236-248`); caller `recv_timeout`, `Disconnected` is `unavailable`, spawn failure is
  `unavailable` (`:97-117`). `op` is `herdr.<wire method>` as Decision 3 pins. Messages name socket, method and
  `ErrorKind`, never request or reply bytes.
- **D2 "with `write_all`".** F uses its own write loop so each `write` syscall gets the time left (Decision 3's
  "before each syscall"). Declared in `handoff-F.md` Deviations 2; identical bytes on the wire. Not silent, and it
  serves the stricter rule. Accepted.
- **D5.** Socket is configuration only; relative is `usage`; no env read, no default path (AC 38 grep empty).
- **D6.** Order `session`, `socket`, `timeout`, workspaces by label, `rows` before `cols` (`adapter.rs:400-430`). One
  addition: a timeout that overflows `Instant` is `usage` (A finding 3), declared as F Deviation 1 and pinned by T's
  ninth AC 27 case. It only refuses configs that would otherwise panic. Accepted.
- **D7/D8.** `connect_with` validates, pings, `parse_pong`, `check_supported`, keeps nothing (`:95-100`). `version()`
  pings and re-gates under its own deadline and refuses an empty or control-character version as `unavailable`
  (`:346-355`), pinned by T's added test. The other methods do not re-check.
- **D9/D10.** Session mismatch and unconfigured workspace are `unavailable` with no request (`:135-155`); a missing
  workspace is an empty map; a duplicate label surfaces `SessionState::workspace`'s error; a tab-less workspace is
  `unavailable` via `grid_tab` (`:367-377`).
- **D11.** Plan, act, observe exactly as listed: snapshot then `layout.export`, one-cell `plan_splits` with errors
  passed through, empty plan returns the occupant, one step runs `CreateRoot` or `Split`, `PaneNotFound` from the split
  only becomes `unavailable` (`changed_under`, `:382-395`), more than one step is `unavailable` with no step run, then
  the read-back; a misplaced pane is left in place and nothing is closed.
- **D12.** One `session.snapshot`, then `layout.export` per workspace with a grid tab in Herdr's order, `cells()` order,
  label as `workspace`, `config.session` on every pane; tab-less workspaces skipped.
- **D13.** Single-request methods via `expect_ok`; `read` clamps `lines` to `max(1)` saturated at `u32::MAX` and passes
  the unclamped `max_lines` to `parse_read` (`:309-318`).
- **D14.** `exchange` returns the raw `String`; `decode_reply` is in the adapter's `call` (`:113-115`).
- **D17.** No `// stub (#640 part 2)` marker remains in `src/`; T's `#[allow(dead_code)]` on `transport` is removed.
- **D18/D19.** No new normal dependency, no `unsafe`, no lock or cell in `adapter.rs` (AC 37 grep empty).
- **Pinned API.** Every pinned item is present with the pinned signature. Additions are private helpers,
  `#[derive(Debug)]` on `HerdrAdapter` (allowed: "F may derive traits"), and one doc-only fix to the `Transport` doc
  (backticks around `herdr.<method>` for rustdoc). No rename or drop.

No silent deviation found. The brief is not defective: the brief-gate block (operator item 4, the version line) was
overridden by the operator on 2026-10-09 and journaled in `decisions.md`, so Decision 8 stands.

## Quality audit

- **Correctness and failure handling.** Every socket fault maps to `timeout` or `unavailable`; a passed deadline never
  connects; an abandoned worker drops its answer silently; nothing in the worker can panic (no indexing, no
  `Instant + Duration`, `.get(..).unwrap_or_default()` on slices). The adapter keeps no state, so no lost write under
  concurrency is possible inside it; the cross-call race is the verbs' (Decision 19).
- **Build guards.** No `unwrap`/`expect`/`panic!`/`unreachable!` in `src/adapter.rs` or `src/transport.rs` (grep). No
  `#[allow]` in `src/`; every test-file `#![allow]` carries `// #640`. All files under 900 lines. The three
  can't-happen branches in `ensure_pane` return `unavailable` instead of a denied `unreachable!`, each backed by an
  `evidence.md` entry; they are defensive code, not dead code.
- **Protocol.** No Holler wire change: no golden, `holler-proto` or `docs/protocol/v2.md` touch, and none is needed
  (the adapter speaks Herdr's protocol, not the hub's).
- **Tests.** Cross-process behaviour is tested over real Unix sockets in `tempfile` directories (transport tests and
  the AC 33 conformance run); no hub/body harness is needed. No fixed sleep is used for synchronization: the one
  `thread::sleep` (AC 6's drip server) simulates a slow peer, and `serve()` binds before returning. RED-first evidence
  is in `handoff-T-red.md`.
- **Documentation.** `CHANGELOG.md` `[Unreleased]` entry present and linked. No new log event, CLI surface or protocol
  field, so no README or `docs/` change is due. Module docs in `adapter.rs`, `transport.rs` and `lib.rs` describe the
  behaviour accurately.
- **Public-repository privacy.** The diff (excluding `docs/handoffs`) has no home path, user, host, tailnet, IP or
  credential. The only "secret" hits are AC 12's sentinel `typed-secret-text`. Test paths are tempdirs and
  `/unused/h.sock`.
- **Commit and PR hygiene.** See advisory 1: no PR exists yet, and the branch's commits are the pipeline's `chore(#640)`
  checkpoints.

**The outside diff gate's needs-verification findings** (`640-diff-result-r2.md`), settled by reading source:

- **NV-1** (spawn failure): Decision 3 says "Failing to spawn the worker is `unavailable`", and
  `transport.rs:101-110` maps any `spawn` error so. Nothing more to distinguish. Settled.
- **NV-2** (a blocked `connect`): the claim the reviewer tests is not the code's. The socket timeouts are set only
  after `connect` returns, so they do not bound it; the caller's `recv_timeout` bounds the call, and a worker stuck in
  `connect` lingers until the server accepts or dies. The module doc (`transport.rs:12-17`) and the brief's Risks
  ("covered by design only") say exactly this. Settled as designed.
- **NV-3** (`base36`): `tests/wire_herdr/mod.rs:639-650` emits no leading zero: 1 is `1`, 10 is `A`, 35 is `Z`, 36 is
  `10`. The reviewer's `01`/`09`/`0A` reading is wrong. AC 30 pins `w1:p9` then `w1:pA`. Settled.
- **NV-4** (`Arc` forwarding): `transport.rs:62-66` is the only `Transport for Arc<T>`, and it forwards to
  `(**self).exchange`; `WireHerdr` and `Tap` each have one impl. No conflict. Settled.
- **NV-5** (overflowing timeout): `validate` calls `deadline_after` (`adapter.rs:417`), which uses `checked_add`
  (`:434-440`); `adapter_test.rs:483-496` pins `Duration::MAX` as `usage`. Settled.
- **W-3** (an `Ok(0)` read at the deadline): not a real race. A socket timeout surfaces as a `WouldBlock`/`TimedOut`
  error, never `Ok(0)`, and zero time left is `Timeout` before the syscall (`left()`). `Ok(0)` is only a true EOF.

## Scope check

Exactly the brief's Files table: `Cargo.toml`, `src/lib.rs`, `src/transport.rs`, `src/adapter.rs`, the five test files,
`CHANGELOG.md`, and one `Cargo.lock` line. `holler-pane`, `holler-pane-testkit`, part-1 sources, `tests/common/mod.rs`,
`holler-cli` and `docs/adr/` are untouched. Part 3's items (scratch test, `holler-pane` doc edits, ADR rows) are not
delivered, correctly. Over-delivery is limited to the overflow `usage` case and the version-form test, both closing
gaps the reviews named. No under-delivery.

## Verdict

**PASS.** All issue and brief criteria covered by part 2 are met by named tests that assert behaviour; every MO
decision is implemented as stated, with two declared and justified refinements; quality, privacy and scope are clean.
Ready for O.

## Advisory notes (non-blocking)

1. **PR and merge hygiene, still to do.** No PR is open yet. When the script opens it: the body must say `Part of #640`
   (not `Closes`) and carry the `CONTRIBUTING.md` AI disclosure (add it with `gh pr edit`); the squash subject should be
   a Conventional Commit such as `feat(adapter-herdr): ...`. The pipeline's commit trailers are
   `Co-Authored-By: Claude <noreply@anthropic.com>` with no session link. `git log origin/main..HEAD` also lists part
   1's pre-squash commits (the branch name was reused and merged with `origin/main`); the tree diff is clean and a
   squash merge drops them.
2. **`Timeout.op` vocabulary diverges from `FakeHerdr`** (wire method `herdr.layout.export` vs port method
   `herdr.ensure_pane`). Decision 3 as written; carry it with Decision 20 into part 3's ADR-0021 §9/§10 rows and the
   operator's list (A finding 1, A-dup warn 2).
3. **Herdr-sent text is quoted with `{:?}`, not cut to 64 characters** (`adapter.rs:230-241`, `:372-374`, `:385-391`). One line always; length unbounded. Follow-up: make `protocol::excerpt` `pub(crate)` in part 3 or later
   (A-dup warn 1).
4. **Part 3 must write the ADR rows** for extent-from-config, the gate at `connect`/`version()`, and the `op` rule
   (A-dup warn 3); #642 should reuse this bounded-exchange design via a shared home, not a copy (A-dup warn 4).
5. **AC 12a's server leaves connections in the listen backlog** during the 20 exchanges rather than accepting them
   (T-red assumption 4). The client cannot tell the two apart, and the later drain still proves each worker dropped
   its socket, so the criterion holds; noted because it is not the brief's literal set-up.
6. **Workspace test run on CI.** The 4 `logging_test` failures are environmental here; CI is the confirmation.
