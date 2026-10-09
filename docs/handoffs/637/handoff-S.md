# Handoff-S: Phase 9 (re-entry) - #637 slice a, the `holler-pane` crate (SPEC AUDIT after the rework)

**Date:** 2026-10-09, 05:35 MDT
**Branch:** issue-637-implementation at 127914e (base origin/main 27da2da). The code tree is the one T verified at 19d5817; 127914e changed handoff files only.
**Issue:** #637 and the contract section of epic #633 (both last edited 2026-10-09 03:30 MDT, so neither reflects D1 or D2)
**Brief:** docs/handoffs/637-brief.md, revision 6
**Handoffs reviewed:** handoff-A.md (PASS), handoff-T-red.md, handoff-F.md (both passes), handoff-T-green.md (both passes, PASS), handoff-A-dup.md (PASS, after the rework), decisions.md, evidence.md
**Replaces:** the first audit (REWORK, two items), kept in git at 5f69488.
**Verdict:** **PASS.** Three operator confirmations are listed below; none blocks the merge, and each has a default that stands without an answer.

## A precondition

Met.
- handoff-A.md reviews brief revision 5 and returns **PASS** (0 blocks, 13 warns). Revision 6 folds in the warns the operator decided.
- The anti-duplication gate after the rework (handoff-A-dup.md) returns **PASS** (0 blocks, 3 warns), on the diff 27da2da..19d5817.

## T precondition

Met. handoff-T-green.md lists **no blocking issues** in either pass.
- **RED** is in handoff-T-red.md: all six `holler-pane` targets fail with E0432 on the API the brief defines, and the two `methods.rs` tests fail on assertions against a stub.
- **GREEN, first pass:** 57 tests and 2 doctests.
- **GREEN, rework pass:**
  - 64 `holler-pane` tests and 3 doctests;
  - 2 `holler-proto` `methods` tests;
  - workspace: 928 passed, 0 failed, 5 ignored;
  - every Tier 1 gate passes.
- **The rework tests were written after the fix,** as a REWORK loop requires. Instead of RED:
  - the env test fails when the `deserialize_with` on `Pane.env` is removed (T's mutation);
  - the D1 tests are compile-time pins of a type alias.

## Acceptance criteria

| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| 1 | build, clippy `-D warnings`, test, machete, lint.sh and changelog-check pass; no file over 900 lines; `[Unreleased]` entry links #637; new `.rs` files rustfmt-clean; no existing file reformatted | T-green's rework Tier 1 table (at 19d5817; the code has not changed since). `wc -l`: the largest touched `.rs` file is `error.rs` at 617 (lint.sh warns at 600 and fails at 900). The CHANGELOG `### Enhancements` entry links #637 and #633. The `methods.rs`, `CHANGELOG.md` and `Cargo.lock` diffs only add lines. | PASS |
| 2 | Grid table, case and whitespace rules, a bare pair is row,col, the ambiguous and out-of-range lists, the 12 x 12 round trip, serde key order, a disagreeing `pos` refused | `grid_test.rs`: `grid_parse_table`, `grid_refuses_interior_whitespace_and_junk`, `grid_display_is_row_first_and_round_trips_every_cell_of_a_12_by_12_grid`, `grid_serde_writes_row_col_pos_in_that_order_and_reads_it_back` (exact bytes), `grid_serde_refuses_a_pos_that_disagrees_and_a_zero_cell` | PASS |
| 3 | `Argv` is a JSON array, and a bare string is `command-not-argv`; `EnvVarName` is the one env guard (`=` gives `profile-secret-refused`; empty or whitespace gives `env-name-invalid`); no `ProfileSpec` field can hold an env value | `argv_env_test.rs` (6 tests), plus `rework_test.rs::a_bad_env_in_a_spec_or_a_pane_is_refused_by_code_and_never_echoed` (details below) and `a_good_env_is_still_a_list_of_names_and_an_absent_env_is_empty`. | **PASS** (was PARTIAL; first-round REWORK 2 is fixed). |
| 4 | Round trips of `Pane` (full and without `profile`), `Profile` and `ProfileSpec`; a stale `cas_put` is `Conflict` / `generation-conflict` | `records_test.rs`: `pane_full_round_trips`, `pane_without_profile_loads_as_none`, `profile_round_trips`, `profile_spec_round_trips`, `cas_put_with_a_stale_generation_is_a_generation_conflict` (the record is also unchanged), and `delete_with_a_stale_generation_...`. The doubles now call `next_generation`. | PASS |
| 5 | `ALL_CODES` unique and kebab-case through `is_valid_code`; every variant's `code()` and `Display`; `RefusalCode::parse` rejects invalid and closed codes; a `compile_fail` doctest; reply round trip; an unknown code parses to `Refused` | `error_test.rs` (12 tests). Doctests: `error.rs:231` (E0080), `:385` (E0277), `:394` (E0423, new). The E0423 doctest pins field privacy: F made the field `pub` in a detached copy and only that doctest failed. | PASS |
| 6 | Every port implemented in-test and `Send + Sync`; `resolve` and `edit_spec` with and without a profile or pane; `run_probe`, `ProbeResult` and `version()`; `Ports` driven by a verb-shaped function; the `pane_name_grammar` and `profile_name_slug` tables | `ports_test.rs`: `ports_are_send_sync_trait_objects`, `ports_compile_in_test_impl`, `scope_resolve_with_and_without_a_pane`, `scope_edit_spec_with_and_without_a_profile`, `ports_bundle_drives_a_verb_shaped_function`, `run_probe_stub_never_reports_success`. `names_test.rs` (8 tests). | PASS |
| 7 | `find("pane/list")` and `find("profile/list")` are `None`; `CATALOG.len()` is 22; the two lists exactly; the test is inside `methods.rs`; no golden change | `methods.rs` `mod tests`: `pane_methods_not_in_wire_catalog`, `is_pane_method_and_is_profile_method_classify_by_list`. `golden-diff-summary.sh` reports no drift, and the diff touches no golden file. | PASS |
| 8 | `git diff --name-only origin/main...HEAD` stays inside the Blast radius | I filtered all 49 paths against the radius and none falls outside it. The working tree is clean. | PASS |

**How the AC 3 rework test works.** It sends 11 bad `env` shapes, including `"TOKEN=hunter2"`, `"=hunter2"`, `["A","B=hunter2"]`, a map, a number and `null`.
- Each shape goes into a `ProfileSpec` and into a `Pane`, both directly and through `decode_params`.
- The test asserts the code, and asserts that `hunter2` appears in none of these: the serde error, `Display`, `Debug`, or the serialized `PaneReply::failure`.
- The reply must also parse back to the same code.

## Spec compliance

**The first audit's REWORK items are both done:**

1. **Decision 7's per-trait doc rule is on all seven ports.** The five that lacked it now carry it:
   - `PaneStore` (`ports.rs:56-58`)
   - `HerdrPort` (`:121-123`)
   - `HostPort` (`:153-155`)
   - `HarnessPort` (`:173-175`)
   - `Prober` (`:204-207`), worded for `ProbeResult`, as requested

   `ProfileStore` and `ProfileScope` already had it.
2. **The env echo is closed.** `argv::deserialize_env_names` (`argv.rs:119-149`) reads a JSON value first, as `Argv` does.
   - It is used on `Pane.env` (`pane.rs:247`) and `ProfileSpec.env` (`profile.rs:188`), which are the only env fields in the crate.
   - A bare string with `=` is `profile-secret-refused`. Every other wrong shape is `env-name-invalid`.
   - Both are unit variants with fixed text, so no refusal can carry its input.
   - The field type is still `Vec<EnvVarName>`, so the frozen contract does not change.

**The rest of the brief is unchanged from the first audit's PASS:**
- decisions 2, 3, 8 and 9;
- the Files list and the Reuse map;
- the root re-exports;
- the 22 closed codes, each with its owning story on its variant.

**One recorded deviation from the brief: D1.**
- **What changed:** `Watch<T>`'s item is now `Result<Option<T>, PaneError>` (`ports.rs:52`). `Ok(None)` means idle; `Err(Timeout)` now means only a store that did not answer.
- **Why it is not a silent deviation:** decisions.md records it (F rework entry, line 238), and so does handoff-F.md under Deviations. Decision 7 lets the implementer adjust a provisional signature with a record. Neither the issue nor the epic spells the item type.
- **What is stale:** the brief still shows the old alias (`637-brief.md:147`; advisory 3).
- **Who chose it:** the coordinator, citing the operator's delegation. It was not my first-round default (b). See the operator decisions below.

**D2 is implemented as recommended.** `HarnessKind` stays a closed enum; its doc (`pane.rs:117-124`) and a test tie it to `holler_proto::HARNESS_IDS`.

**Also done in the rework:**
- the not-found-first rule for `delete` (`ports.rs:73-77`, `profile.rs:344-347`);
- the absent-`params` rule for `decode_params` (`reply.rs:111-114`).

Both match #639's amended text and `holler_proto::typed_params`.

## Quality audit

- **Correctness and failure handling: no new defect found.**
  - Every refusal on the env path is a fixed-text unit variant. I traced each in-crate env decode, and each goes through the new reader: `Pane`, `ProfileSpec`, `Profile.panes`, `PaneEvent`, `ProfileEvent` and the params structs.
  - Fail-closed paths are unchanged from the first audit:
    - CAS goes through `next_generation`, and an overflow is `store-corrupt`;
    - `deny_unknown_fields` is on records, params and events;
    - a profile whose slug does not match its name is refused on read;
    - a malformed reply is `unavailable`.
  - This crate has no concurrency code.
- **Build guards: pass.**
  - Nothing in `src/` calls `unwrap`, `expect`, `panic!`, `unreachable!` or `todo!`. The one `assert!` is the brief's const assertion in `RefusalCode::from_static` (`error.rs:239`).
  - There is no `unsafe`.
  - All 9 `#[allow]` lines are in test files, and each carries a `// #637` link.
  - No `.rs` file is at or above 900 lines: `error.rs` is 617 and `ports_test.rs` is 539.
  - rustc denies dead code, and clippy `-D warnings` is clean (T).
- **Protocol:**
  - There is no v2 wire change. `CATALOG` stays at 22 rows, there is no golden drift, and `docs/protocol/v2.md` is untouched by design (ruling 6; ADR-0021/#634 documents these names).
  - The `Watch` item change is library-only. The wire form (`WatchReply { events, cursor }`) is unchanged, and an idle long-poll answers `{events: [], cursor}`, as `control/wait` does.
- **Tests:**
  - All are unit tests, with no process, clock, thread or sleep.
  - RED, GREEN and mutation evidence is in T's handoffs.
  - Test-file naming drifts from `docs/testing.md` (advisory 2).
- **Documentation:**
  - The CHANGELOG `[Unreleased]` entry links #637 and is still accurate after the rework.
  - There is no new log event, CLI surface or v2 field, so no README or `docs/` update is needed. No doc in the repo lists the workspace crates.
- **Public-repository privacy: clean.**
  - I grepped all 6,100 added lines for IPs, emails, home paths, tailnet names, private domains, machine names and token shapes:
    - the only IP is `127.0.0.1`;
    - the host `kiwi` is the repo's existing placeholder (`holler-body/src/x25519_identity.rs:282`, `holler-cli/tests/attach_mode_test.rs`);
    - `io` appears only as `std::io`.
  - `git diff origin/main...HEAD | gitleaks stdin` reports no leaks.
- **Commit and PR hygiene:**
  - All 20 commits have a Conventional Commit subject, the GitHub no-reply author address and a `Co-Authored-By` trailer.
  - None has a session link. Neither does any of the last 200 commits on main, so this is the repo's practice, not this run's defect.
  - The PR is not open yet. Once the script opens it, the run's agent must add the AI-disclosure line that `CONTRIBUTING.md` requires (`gh pr edit`).

## Scope check

- **Delivered exactly the brief's scope.** The rework touched:
  - the six `src` files named in handoff-F.md (doc lines, the env reader and the `Watch` alias);
  - the test doubles;
  - one new test file.

  All of it is inside the radius.
- **No unrelated refactor.** A-dup row 6 (the `Spec*` rename) was skipped on the coordinator's instruction, and it stays optional.
- **Process deviations, both recorded in the handoffs:**
  - F edited two test lines, type annotations that the alias change forced, at the coordinator's instruction. T reviewed them, and no assertion changed.
  - F's rework ran as two interleaved executions. T re-ran every number from scratch, and they agree.

## Verdict

**PASS.** All eight acceptance criteria are met by named tests or recorded gates, and both first-round REWORK items are fixed and pinned. The code is spec-compliant, with one recorded deviation (D1), and quality is acceptable. Ready for O.

## Operator decisions (relayed request: "Ask for decisions now before I go back to sleep")

None blocks the merge, and each default stands without an answer. After merge, D1 or D2 can still be overruled, but only through the epic's amend-first rule, and #638/#639 start building on these types right away.

- **D1. Confirm that `Watch` signals idle with `Ok(None)`.**
  - **Status:** implemented.
  - **Who chose it:** the coordinator, citing the delegation "drive this issue to completion". F could not confirm that delegation.
  - **Recommendation:** confirm. A-dup recommends it, and so do I. It follows `control/wait` and keeps a wedged store's `Err(Timeout)` distinct from idle.
  - **Cost of overruling:** the alias, its doc and two tests.
- **D2. Confirm the closed `HarnessKind { Opencode }`.**
  - **Status:** implemented, and a test ties it to `HARNESS_IDS`.
  - **Recommendation:** confirm. It was also my first-round default.
- **D3. When to do the pre-freeze polish in advisories 1 and 2.**
  - **The options:** fold it into this PR with one more short F and T pass, or merge now and file a follow-up issue.
  - **Default: merge now.** The issue says #637 gates waves 2 and 3 ("keep it small and finish it first"), and nothing in the polish is a defect. O files the follow-up.

## Advisory notes (non-blocking)

1. **Pre-freeze polish for F (A-dup rows 1 and 2). It is under 10 lines and adds to the frozen surface without changing it.**
   - **(a) `argv.rs:130`: make `deserialize_env_names` `pub`.** Also add a line to `EnvVarName`'s doc (`argv.rs:83-87`) and to the module doc: every `Vec<EnvVarName>` field reads through it.
     - **Today:** nothing leaks, because the crate's only two env fields use the reader.
     - **The risk:** a struct in another crate with the epic's field type (`env: Vec<EnvVarName>`) would bring back the echo of a bare-string `env`. A-dup reproduced this in a scratch crate.
   - **(b) `ports.rs:36-52`: say what the end of a `Watch` means** (the iterator returns `None`). The store closed the stream, after an `Err` or because a fake ran out of scripted changes; a consumer that wants more calls `watch(last_cursor)` again. #638, #639, #643, #649 and #661 implement or consume it in parallel.
   - **(c) `generation.rs:10-11`: add one line** saying that `delete` answers a missing record with not-found before it compares generations. Read alone, "a record that does not exist yet is at generation 0" invites `next_generation(0, 0)` on a delete.
2. **Test layout for T (A-dup row 3).**
   - **Naming:** `tests/rework_test.rs` and `tests/adopted_test.rs` are named after pipeline passes. `docs/testing.md:43-45` sets "one file per behavior", and every other test file in the workspace is named for its subject.
   - **Module docs:** both cite pipeline artifacts ("S's REWORK 2", "A-dup rows 2 and 3", "#637 rows 2, 4, 6-9"). `rework_test.rs:2` also calls D1 and D2 "the operator's", but the coordinator chose them.
   - **Fix:** move each test to its subject's file. Move the duplicate helpers into `tests/common/mod.rs`: `error_test.rs:49-58` `from_wire` (the same function as `ports_test.rs:23-30` `coded`), and the wire round trip `rework_test.rs:19-34` repeats (`error_test.rs:60-65` `over_the_wire`).
3. **The brief is stale on `Watch<T>`.** `637-brief.md:147` still shows `Result<T, PaneError>`, and decisions.md supersedes it. The brief ships in this PR, so O may want to add a one-line revision-7 note before the PR opens.
4. **I7 beyond the `env` field (for the epic, not #637).**
   - **What holds:** AC 3's "no `ProfileSpec` field can hold an environment value" holds for `env`.
   - **What does not:** `command`, `check` and `expect` are free strings, so they can carry a secret, for example `["env","TOKEN=...","opencode"]` or a `curl -H "Authorization: Bearer ..."` check.
   - **Why no type can refuse it:** a check for `=` would also refuse `--port=8095`. I7 for those fields depends on the writers: #644, #650, and #662's `--from-current`.
5. **First-audit advisories that still stand:**
   - stable rustdoc does not check `compile_fail` error codes, so the E0277 and E0080 doctests prove only that the line does not compile (E0423 is pinned by F's mutation);
   - `code()` returns `&str`, the only workable reading of the brief, which contradicts itself here;
   - `RefusalCode::from_static` panics if called at run time with a bad literal (documented);
   - a grid cell above `u16::MAX` read through serde is `usage`;
   - `HarnessPort::health` returns `bool`, which stays provisional until #635;
   - the workspace `Cargo.toml` `# for` markers do not list `holler-pane` (outside the radius).
