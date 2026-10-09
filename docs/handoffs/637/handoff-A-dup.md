# Handoff-A-dup: Phase 7 - #637 slice a, the `holler-pane` crate  (anti-duplication gate, after S's REWORK)

**Date:** 2026-10-09, 05:30 MDT
**Branch:** issue-637-implementation
**Diff base:** 27da2da (merge base with origin/main)   **Diff head:** 19d5817
**This cycle's code delta:** 40b722b..19d5817. It covers F's rework (407c9b5) and T's GREEN pass (19d5817); the commits between them change handoff files only.
**Reuse map:** docs/handoffs/637-brief.md §Files "Reuse map" (no separate survey.md exists), plus S's REWORK 2 "Suggested shape" (handoff-S.md) for the env reader
**Verdict:** PASS

This replaces the first-pass review (diff 27da2da..40b722b), which is kept in git at af72b6d. The status of its six rows is below, under their old numbers.

## Summary

PASS, with 0 blocks and 3 warns.

The rework extended existing objects and built no parallel path:
- **The new env reader extends the existing guard.** `deserialize_env_names` sits in `argv.rs` beside `EnvVarName`. It sends every element through `EnvVarName::parse`, the one guard, and raises only the two existing fixed-text variants, through `coded_message()`. It reads a JSON value first, as `Argv` does. This is the shape S's REWORK 2 asked for.
- **Both test doubles use the shared CAS rule.** `cas_put` and `delete` now call `next_generation` (first-pass row 4).
- **`HarnessKind` stays one enum** (D2). A test ties it to `holler_proto::HARNESS_IDS`.
- **`Watch<T>` changed its item type in place** (D1). No second stream type was added.
- **Everything else is doc lines and one doctest.**

None of the stack's Phase 7 candidates is copied: the token store, `Lockout`, `Roster`, the `log` helper and the test harness. The blast radius holds, there is no golden drift, and no touched file is above 617 lines.

The three warns:
- **Row 1 (F, before the freeze):** the env reader cannot be reached from outside the crate.
- **Row 2 (F, before the freeze):** two doc lines are missing.
- **Row 3 (T, optional):** test files are named after pipeline passes, and three reply helpers are near-copies.

None needs an operator decision. The relayed operator request is "Ask for decisions now before I go back to sleep". The two open questions are D1 and D2: the coordinator answered them, not the operator. See Notes for O.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-pane/src/argv.rs:130` (`deserialize_env_names`, `pub(crate)`), `argv.rs:113-117` (`EnvVarName`'s own `Deserialize`), `argv.rs:7-12` (module doc) | **The env guard moved from the type to two fields, and the field reader is private to the crate.**<br>**The new pattern:** this is the crate's first field-level guard. Every other guard is type-level: `Argv`, `GridPos`, the three names, and `Profile`'s slug check. F kept it field-level so the epic's field type `env: Vec<EnvVarName>` does not change.<br>**Why it matters:** #661 and #665 were told the guard is "raised by the type's decode", so they must not pre-scan; #638 was told to use "the one `EnvVarName` guard". A struct in another crate with the epic's field type cannot reach the `pub(crate)` reader.<br>**Evidence:** I built such a struct in a scratch crate with a path dependency on this head. Through `decode_params`, `"env": "TOKEN=hunter2"` comes back as `usage`, with the value in both `message` and `detail`. That is the defect S's REWORK 2 fixed. `[12345]` echoes the number the same way, because `EnvVarName`'s own `Deserialize` goes through `deserialize_parsed` and `String::deserialize`.<br>**Today:** no such struct exists. `Pane.env` and `ProfileSpec.env` are the only env fields, and #665's import file can hold `ProfileSpec`s. | **F, before the freeze; no operator decision needed:**<br>- Make `deserialize_env_names` `pub`. It is then reachable as `holler_pane::argv::deserialize_env_names`, with no `lib.rs` edit.<br>- Add one line to `EnvVarName`'s doc and to the `argv` module doc: every `Vec<EnvVarName>` field reads through it.<br>**Not recommended now:** a type-level container, `EnvVarNames(Vec<EnvVarName>)`, the way `Argv` wraps `Vec<String>`. It would be more robust, but it changes the epic's field type, so the epic would need amending. |
| 2 | warn | `crates/holler-pane/src/ports.rs:36-52` (the `Watch` doc), `crates/holler-pane/src/generation.rs:10-11` | **Two doc gaps in contracts that freeze at merge.**<br>**(a) The end of a `Watch` is undefined.** The doc defines the three items and says `Ok(None)` is "not the end of the iterator", but not what an end (`next()` is `None`) means. Five stories implement or consume it in parallel: #638's fakes, the stores of #639 and #661, and #643 and #649. The doubles return `std::iter::empty()`, so today "the stream ended" is set only by example.<br>**(b) `generation.rs` can be misread for `delete`.** It says "a record that does not exist yet is at generation 0". Read alone, that lets a delete of a missing record pass `next_generation(0, 0)` and succeed. That was the old `MemPaneStore` behaviour, which the new `delete` docs forbid. | **F, doc only:**<br>(a) Add one line to the `Watch` doc, for example: "The iterator ends only after an `Err`, or when the store closes the stream (a hub shutting down, a fake that has yielded its scripted changes); a consumer that wants more calls `watch(last_cursor)` again."<br>(b) Add one line to `generation.rs`: "`delete` answers a missing record with not-found before it compares generations (see `PaneStore::delete`)." |
| 3 | warn | `crates/holler-pane/tests/rework_test.rs:1-5`, `tests/adopted_test.rs:1-5`; helpers at `tests/error_test.rs:50-65`, `tests/ports_test.rs:24-30` and `tests/rework_test.rs:19-33` | **Test files named after pipeline passes, and three reply helpers that are near-copies.**<br>**The convention:** `docs/testing.md` (Layout, `:43-45`) says one file per behaviour, in every crate. No other test file in the workspace is named after a pipeline phase.<br>**`rework_test.rs`** bundles five unrelated behaviours: the env echo, watch idle, empty-object params, the harness vocabulary, and deleting a missing record. Each has a subject file already.<br>**`adopted_test.rs`**, from the first pass, does the same. I passed it last cycle without flagging it.<br>**The helpers:** `error_test.rs::from_wire` and `ports_test.rs::coded` are the same function, and `rework_test.rs` repeats `error_test.rs::over_the_wire` inline.<br>**Coverage gap:** no test reaches the new not-found branch of `MemProfileStore::delete` (`ports_test.rs:73-78`). | **T, optional now or as a follow-up; nothing here freezes:**<br>- Move each test to its subject's file: env tests to `argv_env_test.rs`; watch tests to `ports_test.rs` (or a new `watch_test.rs`); the harness vocabulary and the missing-record delete to `records_test.rs`. Move the `adopted_test.rs` tests the same way.<br>- Move `from_wire` and `over_the_wire` into `tests/common/mod.rs` and delete the copies.<br>- Add one case that calls `MemProfileStore::delete` with a name the double does not hold. |

### Status of the first-pass rows (af72b6d)

| Old row | Subject | Status now |
|---|---|---|
| 1 | `Watch` idle reused `timeout` | **Done as option (a).** The item is `Result<Option<T>, PaneError>`, where `Ok(None)` means idle and `Err(Timeout)` means only that the store did not answer. Two tests in `rework_test.rs` pin it. Neither the issue nor the epic needs amending: #637's text names `Watch<T>` without its item type, and the epic does not spell it. Only the brief (`637-brief.md:146`) still spells the old alias, and decisions.md supersedes it. The end of the iterator is still undefined (row 2a). |
| 2 | Deleting a missing record | **Done.** Both `delete` docs now say a missing record is not-found first, whatever the generation. Both doubles follow the rule, and the `MemPaneStore` case is pinned. This matches #639's amended text ("a missing record `pane-not-found`"). |
| 3 | `decode_params` and absent params | **Done as a doc line**, an option I allowed. It is pinned by `watch_params_decode_from_an_empty_object_as_from_the_beginning`. It matches `holler_proto::typed_params` (`envelope/dispatch.rs:27-32`). |
| 4 | Doubles re-implemented the CAS rule | **Done.** Both doubles call `next_generation`. |
| 5 | `HarnessKind` beside `HARNESS_IDS` | **Done as option (a).** The enum's doc records the choice, and `every_harness_kind_serde_name_is_in_the_protocol_vocabulary` links the two lists. |
| 6 | `HerdrSpec` and `SpecHerdr` differ only in word order | **Skipped** on the coordinator's instruction. It stays optional, and becomes a breaking change after merge. |

Also done: T-green's advisory 1. A `compile_fail,E0423` doctest at `error.rs:394` pins `RefusalCode`'s field privacy.

### Checked, no duplication

- **`deserialize_env_names` beside `EnvVarName::deserialize`:** both call `EnvVarName::parse`, so the grammar has one source. They differ only on a non-string element (row 1).
- **`de::Error::custom(e.coded_message())`** now appears three times: in `Argv`, `deserialize_parsed` and the env reader. It is a one-line idiom over the single-source `coded_message()`, so a helper would add nothing.
- **The "Blocking." paragraph is on all seven port traits and in the module doc.** The issue says each port is "documented" with it, and S's REWORK 1 required it. This is not drift.
- **The not-found-first rule of `delete`** is a two-branch check in each store. A helper for it would be a one-`if` function, so the trait docs carrying the rule is the right level.
- **`Watch`:** no second stream type was added. `WatchReply` and `WatchParams` are unchanged, and the idle reply stays `{events: [], cursor}`, as `control/wait` does.

### Structure and gates

- `git diff --name-only 27da2da...HEAD`: every path is inside the brief's blast radius.
- `bash scripts/golden-diff-summary.sh`: no output, exit 0.
- The largest touched files are `error.rs` (617 lines) and `ports_test.rs` (539); none is near 800.
- The rework adds no dependency.

## Notes for O

The verdict is PASS, so the run continues to S.

**Operator decisions to ask now.** Each is reversible only until the run's agent merges the PR. With no answer, the implemented choice stands.
- **D1: confirm that `Watch` signals idle with `Ok(None)` (implemented as option a).**
  - **Who chose it:** the coordinator, citing the operator's delegation "drive this issue to completion". F could not confirm that delegation (decisions.md, F rework entry).
  - **A recommends confirming.** It follows `control/wait` (`control_server.rs:477-480`), and it keeps the wedged and slow calls #638 injects (`Err(Timeout)`) distinct from idle.
  - **Confirming needs no issue or epic amendment.** The brief's line 146 is stale; O may add a revision note.
  - **Overruling costs** the alias, its doc and two tests.
- **D2: confirm the closed `HarnessKind` enum (implemented as option a).**
  - **Who chose it:** the coordinator, as for D1.
  - **A recommends confirming.** The serde-name test keeps the enum and `HARNESS_IDS` linked.

**No decision needed for the rest:**
- **Rows 1 and 2 are F's:** one visibility keyword and three doc lines. Each costs least before the freeze.
- **Row 3 is T's and optional.**

## Patterns referenced

- `crates/holler-pane/src/argv.rs:76-81` (`Argv`'s value-first `Deserialize`) and `error.rs:521-540` and `:604-617` (`coded_message`, `from_decode`, `deserialize_parsed`).
- `crates/holler-proto/src/envelope/dispatch.rs:13-34` (`typed_params`), `crates/holler-proto/src/vocab.rs:269-275` (`HARNESS_IDS`).
- `crates/holler-hub/src/control_server.rs:469-481` (`control/wait`, idle as success).
- `docs/testing.md:37-46` (Layout: one test file per behaviour).
- Issues #633, #634, #637, #638, #639, #643, #647, #649, #661, #665, #669 and #670, fetched with `gh issue view` on 2026-10-09 at about 05:18 MDT. The latest edit to any of them was at 03:40 MDT, before S's REWORK.
