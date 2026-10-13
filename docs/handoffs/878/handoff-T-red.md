# Handoff-T-red: #878 pane compact (Part A), phase 4, author the tests (RED)

**Date:** 2026-10-12
**Branch:** issue-878-implementation
**Brief / wireframe reviewed:** `docs/handoffs/878-brief.md` (as amended after A round one); no wireframe (`uiSurface: false`)

## A precondition

Confirmed: A returned **PASS** on the plan in round two (`docs/handoffs/878/handoff-A.md`, 0 block, 5 warn). Its warns 1, 2
and 5 are taken here: the engine and verb tests use `output::emit`'s shape and no `switch::emit_outcome`; the bound tests
use `Call { request: timeouts.compact, ..call_until(..) }`'s observable effect; `HarnessOp` was grepped (see decisions).

## What "RED" means for this change, and how it was proven

A new trait method, new types and a new engine cannot be called from a test without compiling against them, so in Rust
the first RED is a compile error on exactly the feature's symbols. That is not by itself a valid RED, so it was checked in
three ways. The tests were run against throwaway scaffolds that were **reverted and never staged** (see "Scaffold leftovers").

1. **As staged (committed state).** `cargo build --workspace --all-targets --keep-going` stops at `holler-pane-testkit`
   (lib), every error on a feature symbol, none on a typo or a setup mistake, and everything that depends on the kit (all
   of `holler-cli`, `holler-adapter-opencode`, `holler-adapter-claude` tests) is blocked behind it:

   ```
   error[E0432]: unresolved import `holler_pane::ports::COMPACT_UNSUPPORTED`           conformance/claude.rs:52
   error[E0432]: unresolved imports `...::COMPACT_MODEL_UNPINNED`, `...::COMPACT_UNSUPPORTED`  conformance/harness.rs:55
   error[E0599]: no method named `compact` found for reference `&dyn HarnessPort`      claude.rs:467, harness.rs:581,597,617
   error[E0282]: type annotations needed   harness.rs:624,633   (the same missing method)
   error: could not compile `holler-pane-testkit` (lib) due to 8 previous errors
   ```

2. **Stub scaffold (the minimum that makes the names exist, no behaviour).** Types and consts; `HarnessPort::compact` with
   the `compact-unsupported` default; `HarnessOp::Compact`; no-op fake seeds; an engine returning `unavailable`; a verb
   that parses and answers `unavailable`; `Timeouts.compact` defaulting to 10 s. Everything compiles, and every new test
   fails **on an assertion about the missing behaviour**, not on setup:

   | Target | Result with the stub |
   |---|---|
   | `holler-pane-testkit` `fake_harness_compact_test` | 9 failed, 1 passed (the op name) |
   | `holler-pane-testkit` `fake_harnesses_test` | 3 failed (router, `FakeHarnesses`, and the server suite) |
   | `holler-pane-testkit` `harness_conformance_test` | 3 failed (fake, unbroken wrapper, the no-shrink mutant) |
   | `holler-pane-testkit` `claude_conformance_test` | 4 passed: the default already refuses (case 14 is a guard; see Advisory) |
   | `holler-adapter-opencode` `compact_test` | 17 failed, all `compact-unsupported` where an outcome or another code was expected |
   | `holler-adapter-opencode` `hermetic_test` | 1 failed: `ac11_default_timeouts_are_10s_5s_2s_5s_120s` (10 s, not 120 s) |
   | `holler-cli` `pane_verbs` | 18 failed of 424 (9 verb, 7 engine, `unwired_ports_...`, `unmanaged`: e.g. `left: 1, right: 3` exit; `left: "unavailable", right: "session-busy"`; `left: "", right: "compacted demo-c1r1: 90000 -> 12000 tokens\n"`) |
   | `holler-cli` `pane_claude` | 4 failed of 50 |
   | `holler-cli` `pane_integration` | 2 failed of 77 (`compact_forwarding`) |
   | `holler-cli` `pane_cli_process` | 1 failed: `docs_rows::adr_0003_has_one_row_per_pane_and_profile_verb` (a docs row only F can add) |

3. **Oracle scaffold (a complete throwaway implementation, to prove the tests are satisfiable and not contradictory).**
   Written by me for this purpose alone and discarded. Against it: `holler-pane-testkit`, `holler-adapter-opencode`,
   `holler-adapter-claude` and `holler-pane` pass in full (643 tests, 0 failed); `pane_verbs` 424/424, `pane_claude`
   50/50, `pane_integration` 77/77, `profile_verbs` 118/118, `cli_surface_test` and `docs_cli_test` green; the one failure
   left is the ADR-0003 row above. `cargo clippy --workspace --all-targets -- -D warnings` is clean on every file touched
   (it stops on a pre-existing `unneeded cloning` in `crates/holler-cli/tests/token_load_test.rs:60`, untouched by this
   story), and `bash scripts/lint.sh` exits 0. **Mutation spot checks against the oracle** (each broke one rule and the
   named tests failed, then was restored): POST and token reads on the ordinary `call` (3 bound tests); `unexpected`
   instead of `unexpected_status` for the config (the leak test); `rsplit_once` for the pin (the slash test); no activity
   check in the adapter (busy and hang tests); no `check_idle` in the engine (the busy engine and verb tests); no shrink
   rule (the no-shrink tests); `Wired`, router and `FakeHarnesses` each without `compact` (their own forwarding tests,
   independently); `Unwired` without `compact` (`unwired_ports_answer_not_implemented`); the verb without the
   `unmanaged::missing` mapping (the unmanaged test). The adapter's `compact_test` ran 6 times plus 3 under 12 busy loops,
   17/17 each time (about 2.6 s, no fixed sleep as a readiness wait).

## Tests authored

All are behaviour tests at the cheapest tier that can see the behaviour. No test asserts a duration as a result: bounds
are asserted as "ended near the bound, not at the reply" with 300 ms to 2 s margins, and the replies are held by the stub
(`Stub::delay`, `Stub::hang`), never slept for.

**`holler-pane-testkit`** (the contract of the fake and of the port)
- `tests/fake_harness_compact_test.rs` (new, 10): the op is `harness.compact`; a compaction runs on the pin and shrinks the
  seeded context; an explicit model wins over the pin and needs no pin; no model and no pin is `compact-model-unpinned`
  with nothing run; a busy, question-holding or permission-waiting session is refused with its code and nothing run;
  activity is per session and a refused call changes nothing; `session_activity` answers the seed and is not logged;
  the check order (unserved port, unknown session, busy, unpinned, unreadable context); a frozen or killed server and a
  process endpoint; faults, wedge and the log.
- `src/conformance/harness.rs`: 3 new cases (17 becomes 20): `compact-of-unserved-port-is-unavailable`,
  `compact-of-unknown-session-is-session-not-found`, `compact-shrinks-the-context-or-is-refused`.
  `src/conformance/claude.rs`: 1 new case (13 becomes 14): `compact-is-unsupported`.
- `tests/harness_conformance_test.rs`: `DOCUMENTED_CASES` is 20, plus 3 mutants (`CompactAcksAnUnservedPort`,
  `CompactAcksAnUnknownSession`, `CompactDoesNotShrink`) and the `Mutant` wrapper's `compact`.
  `tests/claude_conformance_test.rs`: `DOCUMENTED_CASES` is 14, plus the `AcksCompact` mutant.
- `tests/fake_harnesses_test.rs` (+2): the bare `HarnessRouter` and `FakeHarnesses` each forward `compact` by endpoint
  (a port reaches the OpenCode fake and runs; a process endpoint reaches the Claude fake and is `compact-unsupported`).

**`holler-adapter-opencode`** (the stand-in replies follow the OpenCode 1.19.0 OpenAPI document; labelled in the file)
- `tests/compact_test.rs` (new, 17): one summarize on the server's pin with the exact body `{providerID, modelID}`; the
  request order and the whole route set; the pin is split at the first `/`; an explicit model is sent as given and the
  config is not read; seven unusable pins and four unusable `agent.compaction` shapes are `compact-model-unpinned` with
  nothing posted; an unknown session sends only the existence check; busy, retry, a question and a permission request send
  no POST; no reply body (config, message list, summarize; 500, 400, 204, garbled) ever reaches an error, with a marker
  that leads every body; unreadable counts before and after are `unavailable`, never zero; a summarize and a message read
  slower than `call` and `request` succeed inside `compact`; one that outlasts `compact` is `timeout` of `harness.compact`
  near that bound; hangs on the existence check, the activity reads and the config end at `call`, not at `compact`; a
  spent call; a refused port.
- `tests/hermetic_test.rs`: `ac11_default_timeouts_are_10s_5s_2s_5s_120s` (renamed; `compact: 120 s`).
- `tests/support/stub.rs`: `Stub::delay(method, path, by)`. `tests/support/fixtures.rs`, `attach_test.rs` (x2),
  `tmux_capture_test.rs`: `compact:` added to the five `Timeouts` literals (mechanical, A5).

**`holler-adapter-claude`**
- `tests/hermetic_test.rs` (+1): `compact_is_refused_compact_unsupported_and_sends_nothing`: the adapter is untouched in
  Part A, inherits the default, and writes nothing to the inbox. Green on the default alone, by design.

**`holler-cli`**
- `tests/pane_verbs/compact.rs` (new, 10) and `tests/pane_verbs/compact/engine.rs` (new, 7), registered in
  `pane_verbs/main.rs`. Verb, both formats, on `switch::both` (so each run also proves no Herdr or host call, I4, and a
  valid envelope): AC1 `compact_prints_before_and_after_on_the_harness_pinned_model`; AC2
  `compact_refuses_a_busy_session_and_sends_nothing` (busy, question, permission); AC3 OpenCode half
  `compact_without_a_pinned_model_is_refused_and_sends_nothing`; AC4
  `compact_that_does_not_shrink_exits_1_naming_both_counts` (equal and larger); `compact_refuses_an_unhealthy_server_first`;
  `compact_applies_the_pane_state_refusals_before_the_harness` (parked, unhealthy record, SHOWN/DRIVEN mismatch: the harness
  is not reached); `compact_of_a_pane_without_a_session_of_record_is_session_not_found`; `compact_in_a_profile`;
  `compact_of_an_unknown_pane_and_a_bad_name`; `compact_has_no_model_flag_and_takes_the_target_flags_only`. Engine,
  `tx_compact::compact(ports, &Pane)` called directly: compacts once naming no model and logs `[Health, Compact]` with no
  registry write and no Herdr, host or hub call; unhealthy first (kill, freeze, a failing check passed on); not idle before
  the act; no shrink is `unavailable` naming both counts; a harness refusal or failure passes through unchanged; no
  session of record is `session-not-found` and calls nothing; the engine applies **no** pane-state predicate.
- `tests/pane_claude/compact.rs` (new, 4): AC3 Claude half (an idle healthy Claude fake is `compact-unsupported`, exit 3,
  after `[Health, SessionActivity]`, nothing live acted); a busy or waiting Claude pane is told so first; a stopped process
  is `server-unhealthy` first; an OpenCode pane compacts through the router beside a Claude pane.
- `tests/pane_integration/compact_forwarding.rs` (new, 2): `WiredHarness` forwards `compact` and a misconfigured one
  answers its error from it.
- `pane_verbs/main.rs` (`unwired_ports_answer_not_implemented` gains `compact`), `pane_verbs/unmanaged.rs` (both
  every-acting-verb loops gain `compact`), `pane_verbs/process/{main.rs,docs_rows.rs}` (`PANE_VERBS` gains `compact`,
  `STORY_GROUPS` gains `(878, ["pane compact"])`, which also puts the new verb through the flag-matrix tests), and
  `tests/fixtures/cli-surface.txt` (a `# #878` block of three `pane compact` lines).

Acceptance criteria to tests: AC1 `compact_prints_before_and_after_on_the_harness_pinned_model`; AC2
`compact_refuses_a_busy_session_and_sends_nothing`; AC3 `compact_without_a_pinned_model_is_refused_and_sends_nothing` and
`an_idle_claude_pane_is_compact_unsupported`; AC4 `compact_that_does_not_shrink_exits_1_naming_both_counts` (and the engine's);
AC5 `switch::both` in every verb case (no Herdr/host call) and `assert_inert` in `pane_claude/compact.rs`; AC6 the
conformance cases, the mutants, and `grep -c compact crates/holler-pane-testkit/src/conformance/harness.rs` is now 25;
AC7 `compact_test.rs`; AC8 `the_router_forwards_compact_by_endpoint`, `fake_harnesses_forwards_compact_to_the_fake_its_endpoint_names`,
`the_wired_harness_forwards_compact_and_not_the_default` (and `Unwired`); AC9 is F's and S's to run.

## The contract F implements against (names are mine; the brief gave none for the fake)

`holler_pane::ports` (re-exported from `ports.rs`): `CompactOutcome { pub tokens_before: u64, pub tokens_after: u64 }`
(`Debug, Clone, Copy, PartialEq, Eq`; a `const` of it is built in a test); `CompactModel { pub provider: String, pub
model_id: String }` (`Debug, Clone, PartialEq, Eq`); `COMPACT_UNSUPPORTED`, `COMPACT_MODEL_UNPINNED` (`RefusalCode`);
`HarnessPort::compact(&self, at: &HarnessEndpoint, session: &str, model: Option<&CompactModel>) -> Result<CompactOutcome,
PaneError>`, default `compact-unsupported`. `holler_pane::tx_compact::compact(ports: Ports<'_>, record: &Pane) ->
Result<CompactOutcome, PaneError>`: endpoint, then `session-not-found` for no session of record (before any port call),
then `check_health`, then `check_idle`, then `ports.harness.compact(&at, &session, None)`, then the shrink rule.

`holler-pane-testkit` (`harness.rs`): `HarnessOp::Compact` (`"harness.compact"`); `FakeHarness::seed_context(port, session,
tokens_before, compacts_to)`, `set_compaction_pin(port, Option<CompactModel>)`, `set_activity(port, session, Activity)`,
`compact_runs() -> Vec<CompactRun>`; `CompactRun { port, session, asked: Option<CompactModel>, ran_on: CompactModel }`
(`Debug, Clone, PartialEq, Eq`). The fake's `compact` checks, in order, after `faults.enter(Compact)`: `at.port()`, the server
is reached (frozen is `timeout` with the op, unserved or killed is `unavailable`), the session is known, the activity
(`activity_refusal`'s codes), the model (the given one, else the pin, else `compact-model-unpinned`), the seeded context
(none is `unavailable`); then it logs a `CompactRun` and answers the seeded pair. Seeding and `session_activity` are
**not** in the call log; `session_activity` answers the seed (default `Idle`). A refused call changes nothing.

Verb: `PaneCmd::Compact`, `holler pane compact PANE [--profile NAME]` and no other flag; `acting_pane` arm; it calls
`say_cmd::resolve_pane_record`, and **a `PaneNotFound` from it must be re-asked as `unmanaged::missing(ports, pane)`**
(#832: no acting verb accepts an unmanaged pane; `unmanaged.rs` pins it and the brief does not say it). Text `compacted
<pane>: <before> -> <after> tokens` (plain decimal integers, one line, no thousands separators); JSON `data` exactly
`{"pane","session","tokens_before","tokens_after"}`; an error line for no shrink contains `did not shrink` and both counts.

`Timeouts.compact` (default 120 s) and the adapter's `compact`, with the stand-in conventions the tests pin and Part B.1
must confirm: success is `200` only; the token count is the sum of the last assistant message's `info.tokens`
(`input` and `output` required, `reasoning`, `cache.read`, `cache.write` optional); the pin is `agent.compaction.model`,
split at the first `/`, both halves non-empty; with a model given, `GET /config` is not read; every reply in `compact` goes
through `unexpected_status`; the summarize and the token reads use the compact `Call` and nothing before them does.

## RED confirmation

Commands and their output are in the three tables above. To reproduce: staged state `cargo build --workspace
--all-targets --keep-going`; then the per-target runs named in the table, each `cargo test -p <crate> --test <target>
--no-fail-fast`. With the stub, `cargo test -p holler-cli --test pane_verbs compact` shows the 16 compact failures quoted above (the flag-matrix test passes on the stub's own clap tree).

## Ready for F

**Confirmed: RED is valid. F may implement against these tests.** The ask of F beyond the brief: the four named forwards,
the ADR-0003 row (and its blank-line separation from the neighbouring stories' rows, `adr_0003_rows_of_different_stories_are_never_adjacent`),
the `unmanaged::missing` mapping above, and the docs (`docs_cli_test`).

## Scaffold leftovers (action needed)

The scaffolds touched tracked production files and created four untracked ones. The tracked files are restored
(`git checkout`). The four untracked files could not be removed by me (a safety check blocked `rm`), so they are emptied
(0 bytes), unstaged and unreferenced by any `mod`:
`crates/holler-pane/src/ports/compact.rs`, `crates/holler-pane/src/tx_compact.rs`, `crates/holler-cli/src/pane/compact.rs`,
`crates/holler-adapter-opencode/src/compact.rs`. These are exactly the files the brief has F create, so F's `Write`
replaces them; if anyone prefers, delete them first. No production code of mine remains anywhere. The oracle's patch
(for reference only, not for F) is outside the repo in the session scratchpad.

## Advisory notes

- Case 14 of the Claude suite and the Claude adapter guard test are green on the trait default alone; their value is as
  guards (a mutant fails the case; an adapter override would fail the guard), not as RED.
- The leftover from the brief's file list: `crates/holler-pane/tests/` has no kit to build `Ports` from (the kit depends on
  `holler-pane`), so the engine tests are in `holler-cli/tests/pane_verbs/compact/engine.rs`, where every other engine's are.
- `fake_harness_compact_test.rs` and the adapter tests use plain-decimal counts; if F's text formats counts differently
  the tests, not the format, are to be raised with T.
- `crates/holler-pane-testkit/src/harness.rs` is 692 lines now and the fake's additions will take it near 780; still under 900.
- `cargo clippy` fails today in `token_load_test.rs:60` (pre-existing `unneeded cloning of iterator items`), so the
  "clippy clean" gate of AC9 is "clean on the files touched" as the brief words it.
