# Handoff-S: Phase 8 - #682 part 1, `FakeProfileStore` and the `ProfileStore` conformance suite  (spec audit)

**Date:** 2026-10-09
**Branch:** issue-682-implementation (at ab27aea, on origin/main e410e9d; working tree clean)
**Issue:** #682 (slice c of #638, part 1 of 2; part 2 is #688). Rigor: in-session. UI surface: none.
**Handoffs reviewed:** docs/handoffs/682-brief.md, docs/handoffs/682/{handoff-A.md, handoff-T-red.md, handoff-F.md,
handoff-T-green.md, handoff-A-dup.md, decisions.md, evidence.md}
**Diff audited:** `git diff origin/main...HEAD` (11 code and changelog files, plus pipeline artifacts). No `src/` change
after F (117b861). T-green changed only `tests/fake_profile_store_test.rs`. After ad84b41 only handoffs changed.

## A precondition

Met. handoff-A.md (Phase 3) is **PASS** with six warns and no block. handoff-A-dup.md (Phase 7, at ad84b41) is **PASS** with
four warns and no block.

## T precondition

Met. handoff-T-green.md lists **no blocking issues**.

- **RED** (handoff-T-red.md): both new test targets failed to build with E0432, and only on the six missing public items.
  That is the RED shape the brief names for an API that does not exist yet. T also compiled both files against a stub that
  was then thrown away: 21 of the 22 fake tests failed on behaviour, so no typo was hidden.
- **GREEN**: `fake_profile_store_test` passes 23 tests and `profile_store_conformance_test` 9. Slice a's tests pass 22 + 10.
  T ran the kit 15 times with no flake. The workspace run is exit 0 (1141 passed). T also hand-mutated the fake's CAS, and
  two tests failed.
- **Clippy**: the one clippy failure F flagged (cognitive complexity 16/15 in a T test) was fixed by splitting that test,
  with no assertion dropped.

The test counts in the files match: 23 and 9 `#[test]`s.

## Acceptance criteria

**The issue's acceptance (part 1's share):**

| Criterion | Proving test or evidence | Status |
|---|---|---|
| The fake passes its suite, and the suite rejects mutants (a mutation check per suite) | `tests/profile_store_conformance_test.rs`: `the_fake_passes_the_profile_store_conformance_suite`, `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone`, and six `a_store_that_*_fails`. Each mutant breaks one rule, and the test asserts that its named case is among the failures. | Met (store). The scope half is #688 |
| Nothing depends on `holler-cli` or `holler-hub`; the kit depends on `holler-pane` only | `cargo tree -p holler-pane-testkit -e normal`: only `holler-pane` (and its own `holler-proto`). The forbidden-crate grep prints nothing. The manifest and `Cargo.lock` are unchanged. | Met |
| Workspace gates pass, with no unwrap, expect or panic in library code | T-green's Tier 1 table: clippy `-D warnings`, build, test, `lint.sh`, `changelog-check`, `test-hooks.sh`, machete, rustfmt and `cargo doc` all PASS. My grep of the added `src/` lines finds only a doc example inside a `text` fence. | Met |

**The issue's scope clauses, mapped to suite cases:**

| Scope clause | Case(s) | Status |
|---|---|---|
| Create at 0 is stored at 1 with one entry `{1, actor, Created}` | 3 `create-at-zero-stored-at-one` | Met |
| An update bumps by one and logs `Updated { summary }` | 7 `update-bumps-by-one`. `history` also checks every summary is one non-empty line | Met |
| Stale and ahead generations: `generation-conflict`, no log entry, no event | 8 and 9 (conflict, with get/list/log unchanged). 9 also covers a never-stored name (no record, `log` not found). 23 checks for no event on the stale path | Met. Ahead with no event is not observed directly (advisory N-4) |
| Another name with the same slug at 0 is `profile-exists` | 6 `same-slug-other-name-is-profile-exists`, and 23 | Met |
| Delete of a missing name is `profile-not-found` at any generation | 13 (at 0 and at 7) | Met |
| A stale delete is `generation-conflict` | 12 `delete-stale-conflicts` | Met |
| A good delete logs `Deleted` and publishes `profile: None` | 11 (the log) and 20 (the `None` event) | Met |
| The log is append-only, oldest first, and readable after a delete | 15, plus 11 and 14 | Met |
| `log` of a never-created name is `profile-not-found` | 16 | Met |
| `rename` is `not-implemented` (#665) | 17, and 23 | Met |
| The pane suite's watch cases 14 to 18, for profiles | 19 to 23. They have the same ids as pane cases 14 to 18 and use the shared generic helpers | Met |
| The env guard through JSON (`["TOKEN=x"]` and `[" "]`), with no second scan | 18 pins the `EnvVarName` codes and a verbatim round trip of env names through the store. AC7's grep shows no `'='` scan. The JSON leg is pinned in `holler-pane/tests/argv_env_test.rs` (`env_var_name_serde_is_a_plain_string_with_the_same_refusals`, `no_profile_spec_or_pane_field_can_hold_an_environment_value`) | Met, via brief decision 8 (see Spec compliance) |
| Faults: wedged, `store-corrupt` everywhere, one-shot, slow | AC3's tests (below) | Met |

**The brief's AC1 to AC10:**

| AC | Proving test or evidence | Status |
|---|---|---|
| AC1 the fake passes; 23 documented ids | `the_fake_passes_the_profile_store_conformance_suite`; `the_suite_runs_the_documented_cases` (all 23 ids, in order, equal to the brief's table) | Met |
| AC2 mutation check | `Break::Nothing` gives `Ok(())`. `NoCas` fails `stale-generation-conflicts`, `KeepsSubmittedGeneration` fails `submitted-generation-ignored`, `GenerationBeforeExistence` fails `delete-missing-is-profile-not-found`, `SameSlugOverwrites` fails `same-slug-other-name-is-profile-exists`, `ForgetsLogOnDelete` fails `log-is-append-only-and-oldest-first`, and `RepeatsOnResume` fails `watch-resumes-without-gap-or-repeat`. I traced each mutant through its named case. | Met |
| AC3 faults | `a_wedged_store_times_out_every_method` (7 methods, nothing written, then works), `a_wedged_store_ends_an_open_watch`, `a_corrupt_store_fails_closed_everywhere` (7 methods; records and log intact after), `fail_next_is_one_shot_and_per_op`, `a_slow_call_takes_at_least_the_delay` (lower bound only), `calls_are_recorded_in_order` (seeding and `concurrent_*` not recorded), `port_op_names_are_port_dot_method` (all 8) | Met |
| AC4 the fake's own rules | `the_clock_stamps_created_updated_and_at` (1000, 2000, then 3000 with `Deleted` at generation 3, and 0 by default), `an_update_summarises_the_spec_count`, `the_submitted_slug_is_replaced_by_the_names_slug`, `a_same_slug_other_name_is_profile_exists_at_any_generation`, `a_concurrent_put_makes_the_next_cas_stale`, `a_concurrent_put_creates_a_profile_at_one`, `a_concurrent_delete_makes_the_profile_vanish`, `a_watch_ahead_of_the_head_is_usage`, `the_idle_wait_wakes_on_a_write`, the three seeding tests, `sample_profile_is_valid_and_deterministic`, `sample_spec_is_deterministic_and_harmless`, `sample_spec_agrees_with_sample_pane`, `the_fake_is_send_and_sync_and_its_watch_is_send` | Met |
| AC5 slice a still holds | `git diff --quiet origin/main...HEAD` on both slice-a test files is empty, and they pass (T-green). The `PaneEvent` `Change` impl gives `key = name` and `record = pane.as_deref()`, so the generic helpers behave as before | Met |
| AC6 dependencies | The kit's `Cargo.toml`, the workspace `Cargo.toml` and `Cargo.lock` are unchanged. The `cargo tree` grep prints nothing | Met |
| AC7 one guard, one feed, one writer enum | All three greps print nothing (run by me) | Met |
| AC8 layout | `lib.rs`, `conformance/mod.rs` and both `profile_scope` stubs are unchanged. The diff touches only the test kit, `CHANGELOG.md` and `docs/handoffs/682*`. It adds one file beyond the list: `conformance/profile_store/log.rs`, the split the brief's Risks section prescribes by name (F's Deviation 1) | Met (sanctioned split) |
| AC9 CHANGELOG | One entry, last under `[Unreleased]` / `### Enhancements` and after #676. It covers the faults, the clock, the append-only log, the 23-case suite and its topics, "the fake profile scope follows" and "test code only". It links #682 (and epic #633). `changelog-check` is ok | Met |
| AC10 guards | T-green's Tier 1 table is all PASS. The largest `.rs` in the diff is 597 lines (`wc -l`), under the 600 warn | Met |

## Spec compliance

- **Public API**: exactly as the brief states. `ProfileStoreOp` has the brief's 8 variants, derives and `profile_store.*`
  strings. `FakeProfileStore` has `new`, `seeded`, `set_idle_wait`, `set_now`, `faults`, `concurrent_put` and
  `concurrent_delete`, all with the brief's signatures, plus `Default`. It is not `Clone`. There is one
  `impl Change for ProfileEvent` (key = the slug). The suite has `profile_store_cases` and `run_profile_store_conformance`
  with the brief's bounds, and the fixtures `sample_spec` and `sample_profile`. There are no flat re-exports.
- **Reuse refactors**: `Writer` and `expected` moved into `feed.rs` as `pub(crate)`, and `lock` is `pub(crate)`.
  `pane_store.rs` imports `Writer` and keeps no copy. The five pane-suite helpers are `pub(super)` and documented as
  reused. `expect_change`, `changes` and `cursors` are generic over `Change`, and `increasing` stays over `&[Cursor]`, as
  the brief's signature block has it. Each change is behaviour-neutral, and AC5 confirms it.
- **Fake behaviour**: every port method enters the fault switch first, `rename` included. `cas_put` follows the brief's
  order: the name rule (port writers only, at any generation, before the CAS), then `next_generation`, then the record
  (the name's slug, `created` kept, `updated = now`), then `log.append(event)?`, and only then the log entry. `delete`
  checks existence first (`ProfileNotFound { what: name }`), then the CAS, then the `None` event under the stored name,
  then `Deleted` at the deleted generation + 1. `log` is per slug, never cut, and `profile-not-found` only for a name never
  created. `rename` is `NotImplemented` after the switch. There is no env scan. `seeded` and `concurrent_*` use
  `Writer::Port(0)` and `Writer::Other`. Only closed `PaneError` variants are returned.
- **Decisions already made** (ADR-0021 and the amendments): all six are implemented as stated.
- **Decisions made in the brief**: 1 to 10 are implemented as stated. That covers the split, the name rule before the
  generation, filing by slug, `Deleted` at g + 1, the never-cut per-slug log, timestamp and summary neutrality with the
  fake's clock and `pane specs: a -> b`, an `&Actor` on `seeded` and `concurrent_*`, the env leg without JSON, the
  unpinned `list` order (the suite sorts by slug), and reuse through slice a's files. The suite's module doc states the
  four rules it adds to the port, with their reasons, for #661 (A's W-1).
- **The issue's JSON env case**: brief decision 8 deviates from it on the record and with a reason. The kit cannot take
  `serde_json` before #681 lands it in the same manifest block. The port takes a typed `Profile`, so JSON decoding is a
  property of `holler-pane`'s types, and `holler-pane` already pins the serde refusals. This is a documented, reasoned
  choice, not a silent deviation.
- **F's stated deviations, all accepted**:
  1. `log.rs` exists (the brief's Risks split; the one-file suite was 640 lines).
  2. `expect_change` adds an `E: Debug` bound (`pub(super)`, needed to print the unexpected event).
  3. The pane-suite failure detail now reads `PaneName("x")` (no test pins detail text).
  4. `lib.rs`'s summaries are stale (AC8 forbids the edit).
  5. The private field is named `change_logs`, not `history` (A's W-4).

## Quality audit

- **Correctness and failure handling.**
  - Each write runs in one `feed.write` critical section. Every refusal (name rule, CAS, missing profile) and every
    overflow fails before anything changes. The log entry is appended only after the event is published, so a refused
    or unpublishable write leaves no entry and no event. `Feed::write` wakes watchers only on `Ok`.
  - The lock order is documented and holds: the feed lock first, then `change_logs`, never the reverse, and `log()` takes
    `change_logs` alone. So no deadlock is possible. A `log()` read that runs during a write is linearizable before or
    after that write.
  - With `Fail(StoreCorrupt)` set, all seven methods fail closed, and the records and log are intact afterwards (tested).
- **Build guards.**
  - No `unwrap`, `expect`, `panic`, `assert` or `unsafe` in the added `src/` lines.
  - Both new test files start with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #682`, and no
    other `#[allow]` was added.
  - Sizes: max 597 (`fake_profile_store_test.rs`) and 534 (`conformance/pane_store.rs`), so nothing is near 900 and
    nothing reaches 600.
  - `dead_code` is denied and clippy is clean, so there is no dead code.
  - No `process::exit`, no runtime `CARGO_BIN_EXE` read, no feature change.
- **Protocol**: none. The wire, the error-code table, the golden files and `docs/protocol/v2.md` are unchanged. The only
  codes returned are closed codes.
- **Tests.**
  - All of this behaviour is in-process (a test kit), so there is no cross-process harness to use.
  - Timing tests assert a lower bound, or a generous upper bound (4 s against a 5 s wait). The 50 ms delay in the
    idle-wait test's writer thread is not used to synchronize: a write that lands first is yielded at once. The same
    pattern is in slice a's `fake_pane_store_test.rs:321-329`.
  - RED-first evidence is recorded in handoff-T-red.md.
  - Each test asserts behaviour: removing the rule it covers fails it. The mutants show this for the suite, and T's
    hand-mutation shows it for the fake.
- **Documentation.**
  - The `CHANGELOG.md` entry is correct (AC9).
  - There is no new log event, CLI surface or protocol field, so no README or `docs/` change is owed. Slice a changed
    none either.
  - The module docs are current: the fake's rules, the suite's four added rules, the fixtures, and the shared
    `Writer`/`lock` in `feed.rs`.
- **Public-repository privacy**: grepped every added line, the handoffs included. There are no home paths, personal or
  account names, emails, IPs, hostnames, tailnet names or private domains.
  - The secret-like strings are refusal fixtures (`TOKEN=x`; `TOKEN=hunter2` is quoted from an existing `holler-pane`
    test). `ANTHROPIC_API_KEY` appears only as a name.
  - `$WORKFLOW_ROOT/...` already appears in five tracked files on main.
  - The pipeline artifacts are removed before the push anyway.
- **Commit hygiene**: all six commits have Conventional Commit subjects (`docs(handoffs):`, `chore(#682):`). Each is
  authored with the GitHub no-reply identity and carries a `Co-Authored-By` trailer. That matches the merged pipeline
  runs #685, #686 and #687. The PR is not open yet, so the PR body (disclosure, "Part of") is for the run's agent (N-1).

## Scope check

Part 1 is delivered exactly as scoped, and part 2 (`FakeProfileScope`, the scope suite, the I8 cases and the two scope
mutants) is deferred to #688 under brief decision 1. That is why the PR must say "Part of #682".

- **Over-delivery, all small and justified:**
  - `log.rs`, the split the brief prescribes.
  - T's fixture-agreement test (A's W-5).
  - `SAMPLE_GRID`, `SAMPLE_CWD`, `SAMPLE_PORT_POLICY`, `sample_model()` and `sample_context()`, shared by `sample_pane`
    and `sample_spec` (A's W-5, behaviour-neutral, guarded by AC5).
  - The doc updates A asked for (W-3, W-4).
- **Under-delivery:** none for part 1.
- **No unrelated refactor.** Every other crate, every manifest, `lib.rs` and `conformance/mod.rs` are untouched.

## Verdict

**PASS.** Every acceptance criterion of the issue (for part 1) and of the brief has a proving test or a check I verified.
Every recorded decision is implemented as stated, the two deviations from the issue are documented with reasons, and the
quality guards hold. It is ready for O.

## Advisory notes (non-blocking)

- **N-1 (run's agent, before the merge): the PR body.** The script opens the PR with `Closes #682.`. Change it to
  `Part of #682.` with `gh pr edit`, and in the same edit add the `CONTRIBUTING.md` AI disclosure. After the merge, check
  that #682 is still open, and reopen it if GitHub closed it. This is A's W-6, carried forward.
- **N-2 (run's agent or O, before the merge): #688 points at a file that will not reach main.**
  - #688 says its 14 cases and 2 mutants are "fixed in the appendix of `docs/handoffs/682-brief.md` (on the
    `issue-682-implementation` branch until part 1 merges)".
  - The script's handoff cleanup runs `git rm -r docs/handoffs/682 docs/handoffs/682-*` before the push, and the merge
    deletes the branch. So that appendix never reaches main.
  - Fix: copy the brief's section "Appendix: fixed for part 2" into #688's body (the issue is the source of truth), or
    link it with a commit-SHA permalink into this PR.
- **N-3 (O): file the consolidation follow-up** that A raised twice (Phase 3 W-2, Phase 7 W-1 and W-2). It is still
  unfiled. Once #681, #683, #684 and #688 have merged:
  - move the five shared helpers and the duplicated `CONFLICT`, `C1`, `C2` and `C3` into `conformance/mod.rs`;
  - start `tests/common/mod.rs`;
  - refresh `lib.rs`'s stale summaries.
- **N-4 (#688 or N-3): pin "ahead, no event" literally.** Add an ahead write (`cas_put(&revised(&p, 2), 5)`, expecting
  `generation-conflict`) to case 23's refused writes, a one-line change. This is not blocking: ADR-0021 sections 7 and 8
  put the shared `next_generation` check before any publish, and case 23 already pins that branch through the stale write.
- **N-5 (#688 or N-3): the JSON env round trip.** Brief decision 8 deferred it until #681 adds `serde_json`, but no issue
  records it.
- **N-6 (#661): ratify the suite's added rules.** #661 inherits these rules:
  - the name rule before the generation;
  - `Deleted` at the deleted generation + 1;
  - the per-slug log that is never cut.

  A noted (W-1) that the frozen field doc ("the generation the profile had after the write") also admits 0 for a
  delete. #661's brief should accept these rules explicitly, or amend this suite first, as the brief's Risks section
  says. A line in ADR-0021 section 7 would settle it.
- **N-7 (#688):** A-dup's W-3. Add `SAMPLE_HARNESS` and `SAMPLE_ROLE` consts, and extend `sample_spec_agrees_with_sample_pane`
  to harness, role, workspace and grid.
- **N-8:** T saw `holler-cli` `body_run_test::fresh_hello_and_presence_on_every_reconnect` fail locally ("hub did not
  report listening within 10s"). It is unrelated to this diff. It looks like the fixed-port rebind race of #259 (closed).
  If it appears in CI, reopen #259 or file a new issue.
