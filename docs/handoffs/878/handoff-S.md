# Handoff-S: #878 pane compact (Part A), spec audit

**Date:** 2026-10-12
**Branch:** issue-878-implementation
**Issue:** #878
**Handoffs reviewed:** docs/handoffs/878/handoff-A.md (round two), handoff-T-red.md, handoff-F.md, handoff-T-green.md, handoff-A-dup.md, decisions.md, evidence.md; brief docs/handoffs/878-brief.md (Part A only; Part B is the operator's later step)
**Diff audited:** `git diff origin/main...HEAD` (51 files)

## A precondition
Met. handoff-A.md verdict PASS (round two); handoff-A-dup.md verdict PASS.

## T precondition
Met. handoff-T-green.md: RED recorded in handoff-T-red.md, then GREEN; "Blocking issues: None". Two load-timing flakes in untouched code (`holler-body` rebind tests, `holler-cli` `interrupt_test`) pass alone and are outside the diff.

## Acceptance criteria

| # | Criterion (brief A4) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | exit 0, both counts printed, exactly one `compact` call with `model: None`, on the fake's own pin | `compact_prints_before_and_after_on_the_harness_pinned_model` (`crates/holler-cli/tests/pane_verbs/compact.rs:70`); engine: `the_engine_compacts_once_naming_no_model` (`pane_verbs/compact/engine.rs:48`) | met |
| 2 | busy session: exit 3 `session-busy`, no `compact` logged | `compact_refuses_a_busy_session_and_sends_nothing` (`compact.rs:125`); `a_session_that_is_not_idle_is_refused_before_the_act` (engine.rs:117) | met |
| 3a | no pinned model: exit 3 `compact-model-unpinned`, no summarize | `compact_without_a_pinned_model_is_refused_and_sends_nothing` (`compact.rs:158`); adapter `a_missing_or_unusable_pin_is_compact_model_unpinned_and_nothing_is_posted` (`opencode/tests/compact_test.rs:296`) | met |
| 3b | idle healthy Claude fake: exit 3 `compact-unsupported`; busy/unhealthy Claude is refused first | `an_idle_claude_pane_is_compact_unsupported`, `a_busy_or_waiting_claude_pane_is_told_what_it_is_doing_first`, `a_stopped_claude_process_is_server_unhealthy_first` (`pane_claude/compact.rs:38,72,96`) | met |
| 4 | no shrink: exit 1, message names both counts, `--json` `ok:false` | `compact_that_does_not_shrink_exits_1_naming_both_counts` (`compact.rs:182`); engine `a_context_that_did_not_shrink_is_unavailable_naming_both_counts` (engine.rs:147). T-green mutated the shrink rule and both failed | met |
| 5 | no keystroke (I4) | `switch::both` / `ran_nothing` in every verb case; `assert_inert` in `pane_claude/compact.rs:27` | met |
| 6 | conformance cases 18-20 (harness) and the Claude case, each with a mutant | `conformance/harness.rs` cases `compact-of-unserved-port-is-unavailable`, `compact-of-unknown-session-is-session-not-found`, `compact-shrinks-the-context-or-is-refused`; Claude case in `conformance/claude.rs`; mutants in `harness_conformance_test.rs` and `claude_conformance_test.rs`; `fake_harness_compact_test.rs` (10 tests) pins the busy case the rig cannot make | met |
| 7 | OpenCode adapter stub tests: bounds, no pin no POST, request body names the pin, first-slash split, no reply quoted, activity not given the compact bound | `compact_test.rs` (17 tests): `compact_posts_one_summarize_on_the_servers_pin_and_reads_both_counts`, `the_pin_is_split_at_the_first_slash` (mutated to `rsplit_once` in T-green, fails), `a_summarize_slower_than_a_request_but_inside_compact_succeeds`, `a_summarize_outlasting_compact_is_a_timeout_of_the_compact_op`, `a_hang_before_the_act_ends_at_call_not_at_compact`, `no_reply_ever_reaches_an_error_message`, `the_unpinned_refusal_does_not_carry_the_config`. Stub replies are labelled as 1.19.0 OpenAPI stand-ins in the module doc | met |
| 8 | forwarding test per wrapper | router: `the_router_forwards_compact_by_endpoint`; `FakeHarnesses`: `fake_harnesses_forwards_compact_to_the_fake_its_endpoint_names` (`fake_harnesses_test.rs:201,236`); `Wired` and `Unwired`: `compact_forwarding.rs:43,72` | met |
| 9 | workspace tests, clippy, docs carry the amendment | T-green Tier 1 table (one flake in untouched code; clippy error only at pre-existing `token_load_test.rs:60`, a file this branch does not touch); ADR-0021, ADR-0003, `docs/pane-control.md`, `CHANGELOG.md` all carry it (diff read) | met (`cargo machete` is for CI) |

## Spec compliance
- **N1 (model is the server's own pin, no flag, no spec field):** `Args` has only `PANE` and `--profile` (`crates/holler-cli/src/pane/compact.rs:32`); the engine passes `None` (`tx_compact.rs:39`); the adapter reads `agent.compaction.model`, splits at the first `/`, refuses `compact-model-unpinned` with nothing posted (`opencode/src/compact.rs:98-124`). Test `compact_has_no_model_flag_and_takes_the_target_flags_only`.
- **N3 (no generic ACP agent):** nothing added. Correct.
- **N4 (120 s bound only on the summarize POST and the token reads):** `Timeouts::compact` default 120 s; `known`, the activity read and `GET /config` run on the ordinary 10 s call; the long `Call` is a struct-update of `call_until` with `request: bound` (`compact.rs:77-81`), as A's round-two warn 2 asked. `session_activity` keeps its own 10 s call.
- **Engine/CLI split (A2):** `tx_compact::compact(ports, &Pane)` owns endpoint, session-of-record defence, health, idle, act, shrink rule, in the brief's order; no import from `holler-cli`; the CLI calls `resolve_pane_record` and re-asks `PaneNotFound` as `unmanaged::missing` (the `acting_pane` contract). `check_health` / `check_idle` are called through a `pub(crate)` re-export, not copied.
- **Codes:** `session-busy` reuses `activity_refusal`; the new open codes are `compact-unsupported` and `compact-model-unpinned` only; "did not shrink" is `unavailable` (exit 1).
- **Secrets rule:** every reply in `compact.rs` is reported with `unexpected_status`; only the provider/model name pair is carried.
- **Claude Code:** untouched adapter source, inherits the default (`compact-unsupported`); ADR and `pane-control.md` say so and name the spike as the decider.
- **Wrappers:** router, `FakeHarnesses`, `Wired` forward; `Unwired` answers `not-implemented`.
- **Token source and `200`:** implemented as the brief's stand-in (sum of `info.tokens` of the last assistant message; a count that cannot be read is `unavailable`, never zero). Stated as unrun in the module doc, ADR, CHANGELOG and `pane-control.md`.

## Quality audit
- **Correctness and failure handling:** all refusal and failure paths are tested (unhealthy, busy, question, permission, unpinned, unknown session, bad status, garbled and unreadable counts, timeouts, refused port). Counts use `checked_add`; no zero is invented. No registry write, so no lost-write concern.
- **Build guards:** the `+` lines of `crates/*/src` contain no `unwrap(`, `expect(`, `panic!`, `todo!`, `unimplemented!` or `#[allow`. Largest touched files: `holler-pane-testkit/src/harness.rs` 830, `holler-adapter-opencode/tests/attach_test.rs` 843, `opencode/src/lib.rs` 793, `ports.rs` 785; all under 900. `bash scripts/lint.sh` exit 0 per T-green.
- **Protocol:** no wire, hub, registry or log-event change; `schema_version` stays 1; JSON `data` is additive; golden `cli-surface.txt` gains only the new verb rows; open codes use `RefusalCode::from_static`, not the closed `holler-proto` table. `docs/protocol/v2.md` needs no edit as the v2 wire is unchanged.
- **Tests:** no fixed-sleep synchronisation (stub hold/hang and `wait_for`); mutation checks recorded for the shrink rule and the first-slash split; every conformance case has a mutant. Engine tests sit in `holler-cli/tests/pane_verbs/compact/engine.rs` rather than the brief's `holler-pane/tests/`, because `holler-pane` has no dev-dependency on the kit; T recorded the reason and the tests call `tx_compact::compact(ports, &Pane)` as specified.
- **Documentation:** CHANGELOG `[Unreleased]` / Enhancements entry links #878 and #633; ADR-0021 (ports row, section 2 bound, verb list, "Compaction (#878)" paragraph, failure-modes row, Deferred entry, decision 50), ADR-0003 verb row, `docs/pane-control.md` (verb list, acting-verb list, section, open codes, section 12) all updated.
- **Public-repository privacy:** grep of the whole diff (code, docs, handoffs) for tailnet names, personal paths, account names, private IPs and key patterns: no hits. Git identity is the no-reply address.
- **Commit hygiene:** subjects are Conventional Commits and carry the `Co-Authored-By` trailer (see advisory 3).

## Scope check
Delivered the brief's Part A, and only that. Two small, justified additions: the ADR-0003 row (T's request, required by `docs_cli_test` / `cli-surface`) and `unmanaged.rs` / `process` test-table rows. No unrelated refactors. Part B (spike) was correctly not run.

## Verdict
**PASS.** All nine criteria are proved by tests that fail without the change; every decision in the brief is implemented as stated; no blocking quality finding.

## Advisory notes (non-blocking)
1. `crates/holler-pane-testkit/src/harness.rs` is now 830 lines (was 692) and `opencode/src/lib.rs` is 793; the next change to either needs a split first. Follow-up issue suggested for the fake's compaction/activity state (also A-dup warn 1).
2. The OpenCode request, the `200`, `agent.compaction.model` in `GET /config` and the token source are unverified against a real server; the PR body must say so (brief A "Launch condition") and Part B.1 must replace the stand-in fixtures. The conformance case 20 would report `unavailable` on an opt-in real run with a pin and an empty session (T-red hedge).
3. Commit trailers read `Co-Authored-By: Claude <noreply@anthropic.com>` with no session link; CONTRIBUTING.md says commits carry "a session link". After the script opens the PR, check the PR body for the AI disclosure (`gh pr edit`), and confirm the `commit-msg` hook's trailer satisfies the convention.
4. The brief's "commit 1 is docs only" ordering was not kept in history (phase commits, docs landed with F); the PR is squash-merged, so this has no effect unless a reviewer asks for a split.
5. Before merge: rebase onto `origin/main` and keep both sides in `CHANGELOG.md` and `ADR-0021.md`; the decision number 50 is "next free at merge time" (brief A9); CI runs `cargo machete`.
