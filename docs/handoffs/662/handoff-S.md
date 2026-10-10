# Handoff-S: Phase 8 - #662a profile verbs: the pure core and the read verbs  (spec audit)

**Date:** 2026-10-09, 18:53 MDT
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `06a27f5`; diff base `origin/main`
`3bdd129`, re-fetched at audit time and unchanged; this run is 662a only)
**Issue:** #662 (run 662a, so the PR must say `Part of #662`). Issue body re-read with `gh issue view 662`: last updated
2026-10-08 18:56 MDT, before the brief, and identical to the brief's E1.
**Handoffs reviewed:**
- the brief, `docs/handoffs/662-brief.md`, in full;
- `docs/handoffs/662/`: `handoff-A.md` (round 2), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`,
  `handoff-A-dup.md`, `decisions.md` and `evidence.md`;
- the outside model's two gates (`662-brief-result-r1.md`, `662-diff-result-r1.md`; gitignored).

**Verdict:** PASS

## A precondition

Met.
- `handoff-A.md` (round 2, on the amended brief `31062ce`) is **PASS**, with five warns and no block. Round 1's BLOCK
  (`ca42de7`) was fixed by the amendment.
- `handoff-A-dup.md` (diff `3bdd129..80b59c3`) is **PASS**, with five warns and no block.

## T precondition

Met.
- **RED.** `handoff-T-red.md` shows a valid RED at `d175825` over T's signature stubs (the brief's "allowed approach"):
  25 tests fail on behaviour assertions, with representative failing lines quoted. None fails on compile or setup.
  2c passes vacuously, as the brief predicts.
- **GREEN.** `handoff-T-green.md` shows GREEN on F's `03ed19d`. F's commit touches no test file, which S confirmed with
  `git show --stat`. T-green lists **no blocking issues**, and a mutation table shows each test fails when the behaviour
  it pins is removed.

## Acceptance criteria

Only the 662a criteria apply (Decision 1). "S re-ran" means a read-only grep or `wc` that S ran at `06a27f5`. Test
results are T's recorded runs, as the role doc requires.

| AC | Criterion (662a) | Proving test or evidence | Status |
|---|---|---|---|
| 1a | The snapshot copies every spec field | `crates/holler-pane/tests/profile_snapshot_test.rs::snapshot_copies_every_spec_field_from_the_pane_record` checks against a spec written out by hand, including `"fixed:48100"`. `snapshot_ignores_the_fields_a_spec_does_not_hold` pins the "Not copied" list. | Met |
| 1b | The position comes from the record, not the name | `snapshot_takes_the_position_from_the_record_not_the_name` (`demo-c1r2`@r2c1 gives r2c1; `demo-c2r1`@r1c1 gives r1c1) | Met |
| 1c | `profile_from_panes` keeps order and starts at generation 0 | `profile_from_panes_keeps_order_and_starts_at_generation_zero` (slug, generation 0, both stamps 0, order, empty input) | Met |
| 2a | Each differing field once, in `ALL` order, raw grid JSON | `crates/holler-pane/tests/profile_diff_test.rs::diff_reports_each_differing_field_once`. The fields are changed in reverse `ALL` order, and the raw string is the brief's, exactly. | Met |
| 2b | `env` and `expect` compare as sets; argv in order | `diff_compares_env_and_expect_as_sets_and_argv_in_order` (T's ordered-compare mutation fails it) | Met |
| 2c | The round trip gives no difference | `snapshot_round_trips_to_no_difference` (`common::pane()` and a sparse orchestrator pane; Advisory 6) | Met |
| 2d | Matches, differs, missing and extra, in order | `diff_profile_classifies_matches_differs_missing_extra` | Met |
| 2e | Values serialize like the spec; no hand-written serde name | `spec_field_values_serialize_like_the_spec` (the 15 paths, `to_value(field)` equal to its path, values equal to the spec's JSON). The grep `'^[^/]*"(opencode\|agent\|orchestrator)"'` on `profile_diff.rs` prints nothing (S re-ran). | Met |
| 2f | Control characters are escaped; argv prints as a JSON array | `field_text_escapes_control_characters` (ESC and newline; also DEL and C1 in Text, List and Argv) | Met |
| 2g | `port_policy` compares against the live port | `diff_compares_port_policy_as_the_live_port` (`fixed:48100` against 48101; the bare `fixed`) | Met |
| 2h | `is_member` compares slugs | `is_member_compares_slugs` (T's raw-name mutation fails it) | Met |
| 3 | No adapter, probe or environment | All 13 verb tests run through `rig.rs::run_both`, which calls `assert_no_adapter_call()` on both rigs. The rig's `scope` is `Unwired`. The AC 3 grep over the six files prints nothing (S re-ran). | Met |
| 4a | `list` in both formats | `crates/holler-cli/tests/profile_verbs/list.rs::list_reports_name_slug_panes_live_generation_in_both_formats` (data, text lines, raw key order) | Met (Advisory 4) |
| 4b | `list` counts members by slug | `list_counts_members_by_slug` (`SOME-PROFILE` counts for `Some Profile`; singular `1 pane`) | Met |
| 4c | `list` of no profiles | `list_of_no_profiles` (`no profiles\n`, `[]`, exit 0) | Met |
| 5a | `show` reports differences field by field | `crates/holler-cli/tests/profile_verbs/show.rs::show_reports_differences_field_by_field_in_both_formats` (the exact block, the JSON row, the profile as stored, the raw `"live":{"row":2,"col":1,"pos":"r2c1"}`) | Met |
| 5b | `show` reports nothing for a matching pane | `show_reports_nothing_for_a_matching_pane` | Met |
| 5c | The last probe result, never run | `show_reports_the_last_probe_result_without_running_one`. Its members carry a stored `check`, so a probe run would show in `FakeProber::calls()`, which `run_both` asserts empty. | Met |
| 5d | `command` and `check` print as argv arrays | `show_prints_command_and_check_as_argv_arrays` (spec block, difference line, JSON) | Met |
| 5e | Missing and extra panes | `show_lists_missing_and_extra_panes`: a detached spec naming a pane of `Other` is `missing`; A warn 5's spec pane with ESC prints escaped, with no raw ESC on `out` | Met |
| 5f | A missing profile is `profile-not-found` | `show_of_a_missing_profile_is_profile_not_found_in_both_formats` (exit 3, nothing on text `out`) | Met |
| 5g | A store failure passes through | `show_passes_a_store_failure_through` (wedged pane store: `timeout`, exit 1). Also `show_passes_a_profile_store_failure_through` (`unavailable`), `show_with_a_bad_name_is_usage_in_both_formats` (exit 2) and `list_passes_a_store_failure_through`. | Met |
| 8 (a) | The stub cases are gone | No `assert_stub_routes` in `profile_verbs/{list,show}.rs`. `grep -c '662),' stub.rs` prints `2`, and `// #662` is kept at line 36 (S re-ran). | Met |
| 9 (a) | Surface | `docs/adr/ADR-0003.md:67-68` read `holler profile list` and `holler profile show NAME`, with `#662` at column 67 like every neighbour (S measured). The fixture has `profile show \| Demo` and `profile show \| "Some Profile" --json`. T-green: `cli_surface_test` 3/3, `docs_cli_test` 3/3 (the RED failure on ADR-0003:68 is fixed), `pane_cli_process` 34/34. | Met |
| 10 (a) | ADR-0021 | `Deferred to #662` is absent. `fixed:<port>` is at lines 154-155, which is Decision 14 (i) verbatim. Both #662 bullets are gone from "Deferred to named stories", per (iii). (S re-ran.) | Met |
| 11 (a) | Crate tests | T-green: `profile_verbs` 19/19 and every `holler-pane` target pass. The workspace run has 1404 passed and 4 failed, all 4 in `logging_test`. | Met locally; CI confirms (Advisory 2) |
| 12 (a) | Lints | T-green: `clippy --workspace --all-targets -D warnings` is clean and `lint.sh` exits 0. S found two `#[allow]`s, both `// #662`. `rig.rs` relies on #670's crate-root allows in `profile_verbs/main.rs`, which settles the outside model's NV-2. The largest touched file is 336 lines (S, `wc -l`). | Met |
| 13 (a) | No `unsafe`, no dependency | No `+unsafe` line, and no `Cargo.toml` or `Cargo.lock` diff (S re-ran). `cargo machete` is clean (T). | Met |
| 14 (a) | Formatting | T-green: `rustfmt --check --edition 2021` exits 0 on all 10 touched `.rs` files | Met |
| 15 (a) | CHANGELOG | One entry under `## [Unreleased]` / `### Enhancements` (lines 8 and 10; the entry starts at 167, before `## [0.4.0]` at 181), linking #662. `changelog-check: ok` (T). | Met |
| 6, 7; the (b) halves of 3 and 8-15 | `create`, `delete` | Out of this run (Decision 1); 662b | N/A |

**The issue's own acceptance list, mapped to these criteria:**
- **Happy path and every refusal, in both formats; the envelope helper passes; exit codes are equal.** AC 4 and 5. `run_both`
  asserts equal codes and `check_envelope` on every verb test.
- **`ok` and `failed (missing "qwen38")`.** AC 5c.
- **Field-by-field differences in rowcol, and nothing for a match.** AC 5a and 5b.
- **`create --from-current`, `delete --keep-panes` and I7.** These are 662b. In 662a, `spec_from_pane` copies `env` as
  `Vec<EnvVarName>`, which holds names by type.

## Spec compliance

The decisions are implemented as stated:
- **Decision 1.** Only 662a's files change. The `create` and `delete` stubs, STUBS entries, fixture lines and ADR rows are
  untouched.
- **Decision 3.** `fixed_port_policy` gives `fixed:<port>`, and `PortPolicy` is in `ALL`. The live side is
  `fixed_port_policy(live.harness.port)`, through `spec_from_pane`.
- **Decision 4.** `is_member` (slug on both sides) is the one definition. `list`'s live count and `show`'s member filter
  both call it, and neither filters inline.
- **Decision 5.** `diff_spec` compares `spec` with `spec_from_pane(live)` over `ALL`. `env` and `expect` compare as
  `BTreeSet`s, and the values keep stored order. Argv compares in order.
- **Decision 6.** `show` reads `probe.last` and never calls the prober.
- **Decision 7 (662a's part).** The codes are `usage` (2), `profile-not-found` (3), and store errors passed through.
  There is no new code, no `Refused`, and `error.rs` is untouched.
- **Decision 10.**
  - The JSON comes only from derived structs, `FieldValue`, and `SpecField`'s `as_str` serializer.
  - `emit` serializes straight to a string (`output.rs:304-315`), so struct and `#[serde(flatten)]` key order is fixed
    in every build.
  - `list` sorts by slug itself (`list.rs:53`).
- **Decision 11.** The pure tests are in `holler-pane/tests/`, with the `// #662` header and `mod common;`.
- **Decision 12.** The rig is in `profile_verbs/rig.rs`, declared from `list.rs` by `#[path]`. Its adapters are the four
  fakes, and its `scope` is `Unwired`.
- **Decision 14 (i) and (iii).** Both edits are verbatim.
- **Contradictions C3-C6.** No Herdr call. The probe is read, not run. `rustfmt --check` is run per file. The blast
  radius matches the Files list.

**Public API.** It is exactly the brief's.
- The signatures #644 pins are byte-identical. S grepped `profile_snapshot.rs:18`, `:21` and `:46`, and `:72` for
  `profile_from_panes`.
- One difference in form is **journaled, not silent**. `SpecField` serializes through `impl Serialize` with `as_str`,
  not 15 `#[serde(rename)]`s. A recommended it (round 2, warn 1); T-red and F (design decision 1) recorded it. The JSON
  is identical, and AC 2e pins the 15 paths and that serde equals `as_str`.

**Text forms.** They match "What each verb prints": the header, the spec blocks, the pane rows, the difference lines,
the probe forms, no probe line under `missing`, and `probe: none` under `extra`. F refined three things the brief leaves
open, listed under "Deviations" in `handoff-F.md`, and T-green pinned each:
- a count of 1 is singular;
- a `failed` probe with no missing strings prints `failed`;
- DEL and C1 controls inside a JSON array print as `\u` escapes, which is still the JSON of the same strings.

**The outside model's diff gate** (deepseek-v4-pro, round 1) was PASS, and its three NV items are settled:
- **NV-1.** `as_set` and stored-order values are the brief's rule.
- **NV-2.** The crate-root allows cover `rig.rs`.
- **NV-3.** The `serde_text` fallback is unreachable for today's unit variants, as the brief specifies.

Both gates made a real completion: their `usage.json` files show `finish_reason: stop`.

## Quality audit

- **Correctness and failure handling.**
  - Both verbs are read-only, so no write can be lost.
  - Every store error passes through with its own code and is never read as `profile-not-found` (tested for `timeout`
    and `unavailable`).
  - A blank NAME is `usage` (exit 2), raised by the verb, not by clap.
  - The verb's two reads (`get`, then `list`) are not one transaction. A pane that joins between them shows on the next
    run, which is acceptable for a read verb.
  - **Terminal safety.** Every stored string in text mode is escaped: `Text` values, spec pane names (A warn 5), `List`
    and `Argv` (DEL and C1 included), and probe reasons (`{:?}`). Profile names refuse control characters, and slugs
    are ASCII.
- **Build guards.**
  - Production code has no `unwrap`, `expect`, `panic`, `unreachable` or `todo`. The one fallback is
    `unwrap_or_default` on an encode that cannot fail (`profile_diff.rs:213`).
  - Every `#[allow]` carries `// #662`.
  - No file is near 900 lines: production files are at most 333 lines and tests at most 336.
  - There is no dead code. Every helper is used, and the stubs' `STORY` and `not_implemented` imports are gone.
- **Protocol.** None changes: `holler-proto` is untouched, and there is no golden file and no wire field, so
  `docs/protocol/v2.md` is N/A. ADR-0021's failure-mode rows for `profile list` (none) and `profile show`
  (`profile-not-found`) agree with the code.
- **Tests.**
  - The verbs run in-process over the test kit's fakes. There is no cross-process behaviour to cover yet: the binary's
    ports stay `Unwired` until #649.
  - No test uses a sleep (S grepped).
  - RED-first evidence and a mutation table are in T's handoffs.
- **Documentation.**
  - CHANGELOG, the ADR-0003 CLI rows and the ADR-0021 decisions are all updated, and both verbs have a long `--help`.
  - No README or `docs/` page names these verbs (S grepped), and there is no new log event.
- **Public-repository privacy.**
  - S grepped every added line for host, tailnet, account and machine names, private IP ranges, personal paths, email
    domains and secrets. There is no hit beyond the loopback `127.0.0.1` in test argv.
  - Test data uses the kit's neutral names.
  - The values in `tests/common` that predate this diff are reused, not added. One of them is asserted (Advisory 7).
  - The review-model sidecars (`*-result-r*.md`, `*.prompt.txt`, `*.usage.json`) are gitignored (`git check-ignore`).
    The handoffs are removed before push (pipeline conventions §1).
- **Commit and PR hygiene.**
  - Every subject is a Conventional Commit. The `.githooks/commit-msg` hook is active, and S ran it on two of the
    subjects: exit 0.
  - Every commit has a `Co-Authored-By` trailer. None has a session link (Advisory 8).
  - The PR is not open yet. The Workflow script's PR title and body need editing (Advisory 1).

## Scope check

Exact.
- **F changed seven files,** exactly the 662a row of the Size check: `profile_snapshot.rs`, `profile_diff.rs`,
  `profile/list.rs`, `profile/show.rs`, ADR-0003, ADR-0021 and CHANGELOG.
- **T changed seven files,** exactly the brief's 662a "Files (T)": the two pure test files, `rig.rs`, `list.rs`,
  `show.rs`, `stub.rs` and `cli-surface.txt`. Its signature stubs in `src` are the Test plan's allowed approach, and F
  replaced every body.
- **Nothing outside the list changed.** No frozen file, manifest, test-kit file, `tests/common`, `verb_harness` or
  `profile_verbs/main.rs`, and no 662b file.
- **The extras are small and journaled,** with no unrelated refactor:
  - `list::count` (`pub(crate)`, shared with `show`);
  - the long `--help` on both verbs;
  - the three text refinements.

## Verdict

**PASS.** Every 662a acceptance criterion has a proving test or a re-run check. Every decision is implemented as stated,
and the one difference in form is journaled. Quality is acceptable, and the scope is exact. Ready for O.

## Advisory notes (non-blocking)

1. **PR title and body: must be done before merge, by the run's agent, not F.**
   - The Workflow script opens the PR with the title `Implements #662` and the body `Closes #662.`
     (`coding-pipeline.workflow.mjs:4822`).
   - The brief requires `Part of #662`, because 662b closes the issue. A squash merge with `Closes` would close #662
     early.
   - After the PR opens, edit the body to `Part of #662.` and add the AI disclosure per `CONTRIBUTING.md` (repo
     CLAUDE.md). Give the PR a Conventional Commit title like the squash commits on `main`, for example
     `feat(cli): holler profile list and show, the profile snapshot and the spec-versus-live diff (#662 part 1 of 2)`.
2. **CI must be green.** Locally, 4 of 11 `logging_test` cases fail, each `Unexpected success` on `holler roster`,
   because a hub is reachable from this machine.
   - `logging_test.rs` never names a profile verb, and this diff does not touch the roster path.
   - The same 4 failed at RED, and with an empty `HOLLER_STATE_DIR` all 11 pass (F, T-green).
3. **Rebase onto #647 (PR #701, still open)** before merging: A-dup warn 4 applies.
   - In ADR-0021's "Deferred" list, keep both deletions.
   - Under CHANGELOG's Enhancements, keep both entries.
   - Then re-run `bash scripts/changelog-check.sh` and AC 10's greps.
4. **`list`'s own sort is not pinned** (T-green advisory 1). Decision 10 is implemented at `list.rs:53`, but every
   existing store already lists in slug order. AC 4a pins the sorted output, so it is met.
   - Optional test-only follow-up: a delegating `ProfileStore` in `rig.rs` whose `list()` answers reversed. That is the
     pattern of 662b's Decision 13 seam.
5. **`SpecField::ALL` has no completeness guard** against `ProfileSpec` (A-dup warn 2). `ALL` is complete today: 15
   leaves, with AC 2e pinning the paths. A guard test would protect #664, #665 and #650 from a field added later to
   `ModelSpec` or `ContextCeilings`. It is one optional test and needs no production change.
6. **AC 2c's second case is a sparse orchestrator pane,** not "the fully populated pane of 1a". T journaled the
   substitution.
   - The property holds by reflexivity through `spec_from_pane`, so the 1a pane would add no discriminating power.
   - The sparse pane covers the `None` and empty side, which `common::pane()` does not.
7. **`diff_reports_each_differing_field_once` asserts `common::pane()`'s cwd `/work/holler`** as its spec value, where
   Decision 11 asks for a neutral name wherever a test asserts one. The value is a generic placeholder already on
   `main`, not a private name. The fix would be cosmetic.
8. **No commit has a session link.** `CONTRIBUTING.md` says agent commits carry `Co-Authored-By` and a session link.
   The Workflow script's phase commits carry only the trailer, and so do `main`'s recent squash commits. This is a
   repo-wide practice gap, not this diff's. The squash commit's message is written at merge.
9. **For the MO to relay** (A-dup warns 1, 3 and 5):
   - #643's `names_profile` should switch to `profile_diff::is_member`;
   - #644's brief should drop its stale `COMPARED` quote;
   - the escape helper and #647's `findings::embedded` should be settled into one copy, under the brief's text-forms
     Follow-up;
   - later profile verbs should reuse `super::list::count`.
