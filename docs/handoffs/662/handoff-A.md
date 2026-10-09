# Handoff-A: Phase 3 - #662a profile verbs: the pure core and the read verbs  (up-front plan review, round 2)

**Date:** 2026-10-09
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `31062ce`; this run is 662a only)
**Brief reviewed:** `docs/handoffs/662-brief.md` as amended in `31062ce`   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)" (lines 1937-1952; there is no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This replaces round 1's BLOCK (`ca42de7`, still in git history). It is a fresh review of the whole amended brief, not only of
the amendment.

## Summary

PASS. Round 1's one block is fixed, and its six warns are addressed. The pure tests now sit in `crates/holler-pane/tests/`
over `tests/common`, the convention of `docs/testing.md:45` and of the crate's nine neighbours (#647 is already doing the
same in flight with `findings_test.rs`). The production plan still fits:

- placement per ADR-0021 section 5;
- the reuse of `emit`, `class_of`, the closed codes, the name types and the records' serde forms;
- no adapter, scope or prober call;
- the ADR edited in the same change;
- the three signatures #644 pins, unchanged.

Five warns remain. None blocks. Four of them line the plan up with in-flight siblings (#643, #644, #647, #663) or with the
crate's own conventions, and one closes a gap in the brief's own escaping rule.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | `SpecField`: 15 `#[serde(rename = "...")]` on a derived `Serialize`, plus `const fn as_str` "equal to the serde name" (lines 1474-1499) | pattern consistency (single source) | Every dotted path is written twice, and only AC 2e's test keeps the two copies equal. The crate's rule for a closed set of names is one table that the rest derives from: `PaneCode::ALL` plus one `const fn as_str` ("so the three cannot drift (the same single-source rule as `holler_proto::Code`)", `error.rs:41-43`). A string-form type serializes through `as_str` by hand (`PaneName`, `pane.rs:62-66`). No type in `holler-pane/src` or `holler-cli/src` has a per-variant `#[serde(rename = "...")]` (grep: none). Round 1's warn 4 fixed this for `kind` and `role`, but not for the paths themselves. | Derive `Debug, Clone, Copy, PartialEq, Eq, Hash` without `Serialize`. Write `impl Serialize for SpecField` as `serializer.serialize_str(self.as_str())` and drop the 15 renames. The JSON, the public API and AC 2e are unchanged, and AC 2e becomes true by construction. No sibling plan pins `SpecField`. |
| 2 | warn | Forward-compat rows for #643, #663 and #644 (lines 1962-1964); Follow-up "One base rig" (lines 1980-1982); Decision 12's last sentence | cross-story contract (parallel paths) | **(a)** `is_member` reuse covers only "662a merges first". #643 (T-red PASS, `837718b`) plans an inline slug filter for `pane watch --profile` (643-brief Decision 7, line 1069). #663 (`89b611f`) filters `PaneStore::list` by slug inline in `StoreScope::resolve` (663-brief Decision 2, line 1540). Neither brief names `is_member`, so if either merges first, its copy stays beside `is_member` and nothing records the switch. **(b)** The rig follow-up leaves out #647's `pane_verbs/doctor/rig.rs` (409 lines at `8d1b199`), a fourth rig over the same fakes; #644's own follow-up (its decision 25) names it. #643's `Rig` (`pane_verbs/list.rs:39-98`) has Decision 12's shape (the seven fakes, `new`, `ports`, `run(argv, format)`), but names the no-adapter check `assert_nothing_observed`. **(c)** #644's evidence I-2 (644-brief lines 1408-1416) quotes the superseded Decision 3 ("`port_policy` is **not compared** (`SpecField::COMPARED` omits it)"). Nothing in #644 depends on it: its pre-flight grep checks only the three unchanged signatures. | **(a)** Add a Follow-up: "whichever of #643 and #663 merges before 662a switches its inline slug filter to `profile_diff::is_member` once both are on `main`". The MO relays the two rows to those runs. **(b)** Add #647's rig to the follow-up and to Decision 12. Name the rig's methods as #643 does (`assert_nothing_observed`), so the later fold is a move, not a rename. **(c)** Add to the #644 row: "Decision 3 now compares `port_policy`; `COMPARED` is gone". #644's O refreshes I-2 at its rebase. |
| 3 | warn | Decision 4; Behaviour, **show** (lines 1654-1656) and **delete** (662b) | pattern consistency (two seams for one query) | `ProfileScope::resolve(P, None)` is documented as "every pane of the profile ... A missing profile is `profile-not-found`" (`profile.rs:381-384`). That is the query `show` and `delete` build from `profile_store.get` + `pane_store.list()` + `is_member`. The bypass is justified: the issue limits #662 to `ProfileStore`, `PaneStore` and `HerdrPort`; `ProfileScope` is "the helper every `--profile` verb uses", and the profile verbs take NAME as a positional; `list` needs one `pane_store.list()` for every profile; the rig asserts `scope` is never called. But the brief never says so, so a later reader sees two unexplained ways to ask for P's members. | Add one sentence to Decision 4 giving those three reasons. Together with finding 2(a), `StoreScope::resolve` (#663) and `show`/`delete` then share `is_member`, so the two paths cannot diverge. |
| 4 | warn | Decision 14 (iii): delete the "Deferred" bullets at ADR-0021 lines 525 and 530 | parallel-agent coexistence | #647 (T-red PASS, `8d1b199`; 647-brief line 951) deletes the bullet at line 524, the line just above 662a's 525. Whichever of #647 and 662a merges second gets a git conflict. `stub.rs:12-16` documents the same hazard for its own list ("delete adjacent lines, and git reports that as a conflict"). Marking the bullet decided in place does not help, because any edit to line 525 touches the same hunk. #644 already schedules its ADR edits around both stories (644-brief decision 20). | Add one Risks line: "#647 deletes ADR-0021 line 524 and 662a deletes line 525: the second to merge resolves a one-line conflict by keeping both deletions". The run's own agent can then resolve it at merge instead of stopping. |
| 5 | warn | Text rule ("stored strings printed through `FieldValue`'s `Display`", line 1618); Risks, "Terminal injection" (lines 2024-2026) | cross-cutting (output escaping) | `ProfileSpec.pane` is plain text with no grammar check (`profile.rs:265-269`). It is not validated in the hub's registry either (grep finds no check). `show`'s text prints it in `spec <pane>` and in a `Missing` row's `pane <pane>: missing`, where `PaneDiff.pane` comes from the spec. A stored spec whose pane carries an ESC sequence reaches the terminal raw. The brief's Risks list names `cwd`, `workspace`, model strings and the probe reason, but not this one. `ProfileName` and `PaneName` refuse control characters, so the other printed names are safe. | Print a spec's `pane` through the same escape, for example through `FieldValue::Text(..)`'s `Display`, which keeps one copy with no API change. Name it in Risks, and pin one case: a spec pane with `\u{1b}` in AC 5e's `Missing` row. |

### Checked and consistent (the evidence behind the PASS)

- **Round 1's block is fixed.**
  - Decision 11, AC 1 and 2 (locations), Files (T), AC 11, 12 and 14, the Size check and the RED note all now say
    `crates/holler-pane/tests/profile_{snapshot,diff}_test.rs` over `tests/common`.
  - That matches `docs/testing.md:45` and the nine neighbours. Seven of them start with the
    `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #637` header and `mod common;`.
  - `common/mod.rs` carries `#![allow(dead_code)] // #637`, so a file that uses only some of its helpers lints clean.
  - `common::pane()` is fully populated: grid, cwd, workspace, model, role, env, context, command, check, expect, `last`
    and `profile`.
  - No manifest change is needed: there is no `[[test]]` or `autotests`, `serde_json` is a normal dependency, and no
    dev-dependency is required.
  - No `holler-pane/src` file has an inline `#[cfg(test)]`, so AC 2e's grep fits the crate's convention.
  - In flight, #647's `crates/holler-pane/tests/findings_test.rs` follows the same pattern.
- **Round 1's warns are addressed.**
  - `is_member(&Pane, &ProfileName)` mirrors the kit's `belongs` (`holler-pane-testkit/src/profile_scope.rs:234-239`).
  - `harness.kind` and `role` take their text from serde, as `parse_role` does (`holler-cli/src/pane/args.rs:142-150`).
  - The Forward-compat rows are in place, and so is the port-0 sentence in `spec_from_pane`'s doc.
  - The non-existent `auto` is dropped.
  - The rig is in `profile_verbs/rig.rs` through `#[path]`.
  - The text-form divergence from #643 is recorded in Follow-ups and Risks.
- **Decision 3 now compares `port_policy`, and that is consistent with #644.**
  - #644's `port_of_policy` accepts only canonical decimal, and "an accepted policy equals `fixed_port_policy` of its port"
    (644-brief lines 1483-1486). So the string comparison is a port comparison for every launchable spec.
  - `diff_spec(&spec_from_pane(p), p)` is still empty.
  - ADR-0021 is updated in the same change (Decision 14 (i)).
  - The bare `fixed` of `sample_spec` differing is stated, and the verb tests override it.
- **The rig mechanics hold.**
  - A `#[path]` on a non-inline `mod` in the non-mod-rs file `list.rs` resolves against `list.rs`'s own directory.
  - `crate::list::rig` is reachable from the sibling modules.
  - `holler_cli::pane::wiring::Unwired` is public, so the rig's `scope` needs no frozen file.
  - `assert_stub_routes` keeps callers in `profile_verbs` (apply, rename, export, import), so no dead code follows.
- **The production plan is unchanged and fits.**
  - Both modules are pure and live where ADR-0021 section 5 puts them. `profile_diff` depends on `profile_snapshot` inside
    one crate.
  - The verbs print only through `emit`/`emit_error` and use the closed codes.
  - `list` makes one `pane_store.list()` call and `show` makes two store calls, so there is no N+1.
  - `show` reads `probe.last` and never runs a probe.
  - Text escaping in holler-pane is the one copy of the rule: `holler_proto::log::escape_field_value` is private to the wire
    crate, and `holler_pane::error::excerpt` is a different rule (it truncates and quotes).
- **The pins and surfaces are safe.**
  - The three `profile_snapshot` signatures are byte-identical to the ones #644's pre-flight grep expects.
  - `SpecField::COMPARED` appears in #644 only inside quoted evidence.
  - `flags.rs:6-7` already counts "only a required positional is missing" as accepted, so `show`'s new NAME does not break
    `pane_cli_process`.
  - The `stub.rs` deletions keep `// #662`, and an unchanged line separates them from `// #664`.
- **No outside change since `3bdd129`.** `origin/main` is still at `3bdd129`, and no sibling has merged.

## Notes for O

PASS: no amendment is required before T. The warns are cheap to take in the same edit, or the MO can take them as follow-ups:

1. **Finding 1.** Make one line of the API block `impl Serialize for SpecField` through `as_str()`, and drop the 15 renames.
   AC 2e stays as written.
2. **Findings 2 and 3.**
   - Add the Follow-up for the other merge order of `is_member`.
   - Add #647's rig, and use #643's method names in the rig.
   - Add the `COMPARED` note to the #644 row.
   - Add one sentence to Decision 4 on why the verbs do not call `ProfileScope::resolve`.
   - The MO relays the `is_member` rows to #643's and #663's runs: neither brief mentions it today.
3. **Findings 4 and 5.** Each is one Risks line.
   - The ADR-0021 line 524/525 conflict with #647.
   - `ProfileSpec.pane` is escaped like the other stored strings; T can pin it in AC 5e.

## Patterns referenced

- `docs/testing.md:45`; `crates/holler-pane/tests/{common/mod.rs,records_test.rs,argv_env_test.rs}`.
- `crates/holler-pane/src/error.rs:41-160` (`PaneCode`, the single-source table) and `crates/holler-pane/src/pane.rs:62-66`
  (`PaneName`'s `Serialize` through `as_str`).
- `crates/holler-pane/src/profile.rs:265-269` (`ProfileSpec.pane` is plain text) and `profile.rs:373-404` (`ProfileScope::resolve`).
- `docs/adr/ADR-0021.md`: section 3 (lines 149-154), section 5 (lines 169-193) and "Deferred to named stories" (lines 521-533).
- The in-flight sibling plans, read from their worktrees at about 17:45-17:55 MDT. They are evidence of intent, not merged code:
  - 643-brief and `pane_verbs/list.rs` (`837718b`);
  - 644-brief (`7195993`);
  - 647-brief and `pane_verbs/doctor/rig.rs` (`8d1b199`);
  - 663-brief (`89b611f`).
