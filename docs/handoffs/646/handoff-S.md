# Handoff-S: Phase 10 - #646 part 1 of 3 (646a) `holler pane park` and `holler pane unpark` (spec audit)

**Date:** 2026-10-09 (21:45 MDT)
**Branch:** issue-646-implementation (worktree `.claude/worktrees/0646-park-close-routing`, head `2098f7e`, merge base `ce12cdb`;
`origin/main` is `519947a`, six commits ahead, confirmed with `git ls-remote`). The branch is not pushed and has no PR yet.
**Issue:** #646 (epic #633). I read it live with `gh issue view 646`, and its GraphQL edit history. The last edit was at
03:30 MDT, before the brief was written (19:26 MDT), and it has no comments, so the brief matches the current issue.
**Brief:** `docs/handoffs/646-brief.md`, as amended in `e957a8d`.
**Handoffs reviewed:**
- `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` and `handoff-A-dup.md`;
- `decisions.md` and `evidence.md`;
- the outside diff review `646-diff-result-r1.md` (PASS, no block).

**Verdict:** PASS. F built what the brief asked for. Every AC has a test or check that tells the stub from the real verb,
and the quality guards hold. The advisories below go to O for the PR step: a CHANGELOG conflict with `origin/main`, and
wording that #643's merge made stale.

## A precondition

Met:
- `handoff-A.md`: PASS on the plan, with 0 blocks and 6 warns.
- `handoff-A-dup.md`: PASS on the diff (`ce12cdb..dca7abb`), with 0 blocks and 3 warns.

## T precondition

Met. `handoff-T-green.md` reports "Blocking issues: None".

- **RED** (`b996a8f`): `pane_verbs` gave `93 passed; 10 failed`. Each of the 10 new tests failed on an assertion against
  the stub's `not-implemented`, never on a compile error. T-red records the exact failing assertion of each.
- **GREEN** (`dca7abb`):
  - `pane_verbs`: 103 of 103 pass.
  - The workspace in CI's form passes 1448 tests with 0 failures.
  - Clippy, lint, changelog-check, `cargo machete`, the hook tests and rustfmt on the changed files are all clean.
- **Mutations:** 12 were made, and all 12 were caught. T found one hole and closed it: the AC 4 name order was being
  supplied by the fake, not the verb. The fix is the `ReversedScope` test, with no production change.
- **Phase boundaries** (`git show --stat` of each phase commit):
  - T-red touched the argument structs only, which the brief's test plan allows.
  - F changed no test file.
  - T-green changed only `tests/pane_verbs/park.rs`.
  - A and A-dup committed only handoff files.

## Acceptance criteria

Tests are in `crates/holler-cli/tests/pane_verbs/`. "S" marks a read-only check that I ran myself.

| Criterion | Proving test or evidence | Status |
|---|---|---|
| Issue: park and unpark round-trip | AC 1, AC 2 | Met |
| Issue: `park --profile P` parks every pane of P | AC 4 | Met |
| Issue: JSON passes the envelope helper; exit codes are equal across formats | AC 2, AC 10 | Met |
| Issue: close, `say --pane`, `say --queue`, `close --spec-only` | out of scope for 646a (brief C-2: 646b and 646c); the PR must say "Part of #646" | Deferred, as the brief says |
| AC 1 Park then unpark one pane, text | `park.rs::park_then_unpark_round_trips_one_pane`: exact lines; `Parked{R, W, since}` with `t0 <= since <= t1`; generations 2 then 3; every other field equal to the seed | Met |
| AC 2 The same in JSON | `park.rs::park_and_unpark_json_pass_the_envelope_helper`: `check_envelope(&out, 0)`, exact `data` with the stored `since` | Met |
| AC 3 Idempotent, nothing written | `park.rs::park_and_unpark_leave_a_pane_already_in_that_state`: (a) already parked keeps `old`/`later`/5; (b) not parked; (c) drained, for both verbs. Exact text and JSON (`changed: false`, generation 1), `cas_puts() == 0`, records as seeded | Met |
| AC 4 Every member, in name order | `park.rs::park_and_unpark_with_a_profile_take_every_member_in_name_order`: members seeded in reverse; one `since` per run; unpark on the same rig gives generation 3; non-members unchanged; the `ReversedScope` order check; JSON in order; `Demo Empty` | Met |
| AC 5 Membership and missing profile refused | `park.rs::park_and_unpark_refuse_a_pane_outside_the_profile_or_a_missing_profile` (both verbs × 3 cases × 2 formats, exit 3, nothing written; the named member parks only c1); `unpark.rs::unpark_of_a_member_named_with_its_profile_changes_only_that_pane` | Met |
| AC 6 A missing pane | `park.rs::park_and_unpark_refuse_a_pane_with_no_record`: exit 3, `error: pane not found: demo-c9r9\n`, the same JSON message | Met |
| AC 7 Usage before any store call | `park.rs::park_and_unpark_usage_errors_touch_no_store`: (a)-(d) on each flag, the exact message by equality, `assert_no_call_at_all` on both rigs; the check order; 200 `x` and 200 `é` accepted; trimmed storage; (e) clap's `MissingRequiredArgument` | Met |
| AC 8 Store failures name the pane and stop | `park/failures.rs::park_failures_name_the_pane_and_stop` and `unpark.rs::unpark_failures_name_the_pane_and_stop`: (a) and (b); (c) failing on the 1st, 2nd and 3rd write, with exact messages, exact records and the wrapper's `seen() == n`; the rerun; the edges (an unchanged pane is not listed; a one-member profile has no suffix); (d) through `Verb::Unpark` | Met |
| AC 9 No live act, no profile write | `rig.rs::assert_no_live_call_and_no_profile_write`, through `run_both` or called directly, in every test | Met |
| AC 10 Exit codes equal; every JSON output passes | `rig.rs::run_both` (equal codes, empty JSON `err`, `check_envelope(&out, code)`) on every AC 5-8 case | Met (advisory 6) |
| AC 11 The surface | S: ADR 0003 rows 54-55 match. Park has two spaces before `#646` (column 77); unpark keeps column 67; row 56 is untouched. The four fixture lines match AC 11b. In `stub.rs` the park and unpark entries are gone, and `// #646` and `close` stay. T-green: `pane_cli_process` 34, `cli_surface_test` 3, `docs_cli_test` 3, `pane_verbs` 103 (the `close` stub included) | Met |
| AC 12 ADR-0021 edited | S: the grep count is `1`. The inserted paragraph is identical to Decision 12's text (whitespace normalised, compared by `cmp`). It is inserted after the "Neither" bullet (base line 130) and its blank line. Row 341, now line 351, is unchanged, and no other ADR-0021 line changed (one hunk, +10/-0) | Met |
| AC 13 Hygiene | S: no manifest or lock change against `ce12cdb`; no `unsafe`, `unwrap`, `expect`, `panic!` or `#[allow]` in `park.rs`/`unpark.rs`; the largest file is 489 lines. The CHANGELOG entry is under `## [Unreleased]` / `### Enhancements` (line 196) and links #646. T-green: clippy `-D warnings`, rustfmt on the changed files, `scripts/lint.sh` exit 0, `changelog-check: ok` | Met |

## Spec compliance

Decisions 1-13 are built as the brief states them.

- **D1.** The only write is `cas_put(&Pane { hold, ..record.clone() }, record.generation)`, and there is no adapter call
  and no profile write.
- **D2.** `--reason` and `--release-when` are required plain `String`s with no `value_parser`, and PANE is optional.
  A run with neither PANE nor `--profile` is refused by the verb, and there is no `required_unless_present`.
- **D3.** PANE alone is `pane_store.get`, giving `PaneNotFound { what }` as reconcile does. With `--profile`, the scope
  is `scope.resolve(P, pane)`. The verb sorts by name itself. The Named arm never touches a profile.
- **D4.** `next_hold` writes only none to parked (park) and parked to none (unpark). Every other state, drained
  included, is left as it is.
- **D5.** The guard trims, then checks blank, then `char::is_control`, then `chars().count() > 200`. The six messages
  are exact and never hold the value. The typing order is neither, then the pane name, then the profile name, then
  `--reason`, then `--release-when`. A warn 1 is taken: the cap is a named constant that cites
  `holler_hub::holds::MAX_REASON_CHARS` without importing it.
- **D6.** `since` is taken once, in `park_request`, after the guards pass.
- **D7.** The whole scope is read before the first write. The run stops at the first failed write. The suffix appears
  only for two or more panes, both clauses are always present, an empty list is `none`, the changed list holds only
  the panes this run wrote, and nothing is retried.
- **D8.** The JSON is `{panes:[{name, changed, generation, hold}]}`, with `hold` the record's own `Hold` value and no
  mirror type. The text lines and the empty-profile line are exact, and every value is quoted by `findings::quoted`.
- **D9.** There is one engine in `park.rs`, and `unpark.rs` (38 lines) only types its target. The shared `pub(super)`
  entry is `run_hold_change`, which wraps the private engine `change_holds` (typed request plus `Ports`, returning the
  report or an `ErrorBody`) and the renderer. That is a narrower surface than Decision 9's wording, with the same
  behaviour. A and A-dup both reviewed it.
- **D10.** No prompt gate is added, and no file in `holler-hub` is touched.
- **D11.** The rig is test-local, AC 8c's failure comes from a test-local wrapper, and there is no production test
  switch.
- **D12.** The paragraph is verbatim and in place, and row 341 and ADR 0003 line 94 are untouched.
- **D13.** This can only be checked once the PR exists (advisory 3).

A warns 2 and 3 (the ADR wording) are not in the ADR text, because AC 12 pins the paragraph. F put the correct facts in
`park.rs`'s module doc (lines 8-14 and 35-38). This is not an ADVISORY-HOLD:
- The paragraph is accurate as written: "(`usage` otherwise)" describes what the verb refuses.
- Section 8's reconcile-step rule is about a conflict **after a live change**, which park does not have.
- Section 12's sentence is the only one that reads as universal. Both A reviews graded this a warn, and the follow-up
  is the fix (advisory 4).

## Quality audit

- **Correctness and failure handling:**
  - A `resolve` or `get` error (unavailable, store-corrupt, refusals) is answered as it is, before any write, so the
    verb fails closed.
  - A write error keeps the failed write's own code (`ErrorCode::from(&error)`) and therefore its exit class.
  - There are no lost writes: one CAS per pane at the generation it was read at, and the run stops on a conflict.
  - Pane names print bare. That is safe, because their grammar is `[0-9a-z-]` and serde parses them.
  - Stored texts and the profile name print through `quoted` (`{:?}`-escaped, cut at 64 with `...`).
- **Build guards:**
  - No `unwrap`, `expect`, `panic!` or `unreachable!` in production code.
  - No `#[allow]` anywhere in the diff.
  - Every touched `.rs` file is under 600 lines: `park.rs` 401, `unpark.rs` 38, and the tests 489, 407, 269 and 41.
  - No dead code (clippy `-D warnings` on all targets is clean).
- **Protocol:** no change. No error code is added, row 341 stands, no golden file is touched, and `docs/protocol/v2.md`
  needs no edit.
- **Tests:**
  - In-process over #638's fakes and two test-local delegating wrappers (`FailNthCasPut`, `ReversedScope`), as the
    issue and brief require.
  - No sleeps and no timing assertion except AC 1's `t0 <= since <= t1` bracket.
  - RED-first evidence is in T-red.
- **Docs:**
  - The CHANGELOG entry is present and links #646, "part 1 of 3".
  - The ADR 0003 rows and the ADR-0021 paragraph are updated.
  - No README or `docs/` page lists `holler pane` verbs, so nothing else is owed. No log event is added.
- **Public-repository privacy:** clean.
  - I grepped every added line of the diff, the committed handoffs included. No personal host, tailnet, account or
    machine name, IP, private domain, absolute home path or key is present. The hits were rule text ("no personal host,
    tailnet or account name") and `ProfileSecretRefused`.
  - `gitleaks git --log-opts=ce12cdb..HEAD`: 9 commits, no leaks.
  - The outside-review results are gitignored (`.gitignore:25`) and not in the diff.
- **Commit and PR hygiene:**
  - Every commit has a Conventional Commit subject and a `Co-Authored-By` trailer.
  - None carries a session link, which `CONTRIBUTING.md:20-21` describes. The recent squash merges on `main` (`ce12cdb`,
    `e612878`, `efd9a00`) lack it too, so this is standing drift, not this branch's doing.
  - The PR does not exist yet (advisory 3).

## Scope check

Delivered exactly the brief's scope:
- **Production:** `park.rs` and `unpark.rs`.
- **Tests:** `park.rs`, `park/rig.rs` and `unpark.rs`, plus `park/failures.rs`. That fourth file is the split A warn 6
  asked for, explained in T-red.
- **Surface:** ADR 0003 rows 54-55, ADR-0021 (one paragraph), `cli-surface.txt` (three lines changed; the fourth line
  is unchanged), the two `stub.rs` entries and the CHANGELOG.
- **Pipeline handoffs.**

No frozen or out-of-radius file is touched: `pane/mod.rs`, `args.rs`, `output.rs`, `cli.rs`, `close.rs`, the
prompt-target and prompt-verb files, `holler-pane`, the test kit, `holler-hub`, the manifests and `tests/pane_verbs/main.rs`.

Two size differences from the brief's estimates:
- `park.rs` is 401 lines, against "about 200". F explained it: the module doc and rustfmt.
- `rig.rs` is 407 lines, against "about 120". It mirrors #662's helpers, as A warn 5 asked.

There is no over-delivery. The brief's Files list put unpark's AC 3-10 cases in `unpark.rs`. They live in `park.rs`
under the brief's own `park_and_unpark_*` names, which cover both verbs. T noted this, and the coverage is complete.

## Verdict

**PASS.** All of AC 1-13 are met, Decisions 1-13 are built as stated, and the quality audit is clean. It is ready for O.

## Advisory notes (non-blocking, for O and the PR step)

1. **CHANGELOG.md conflicts with `origin/main`.** `git merge-tree --write-tree origin/main HEAD` reports
   `CONFLICT (content): Merge conflict in CHANGELOG.md`.
   - Cause: #642 part 1, #641 and #643 appended to the same `[Unreleased]` / `Enhancements` spot.
   - The conflict is purely additive: keep every entry.
   - `cli-surface.txt`, `stub.rs` and ADR 0003 merge cleanly. The merged result keeps each story's own lines, and only
     close's entry stays under `// #646`.
   - I found no semantic overlap: #643 touched only `list.rs`, `get.rs`, `watch.rs` and their tests, and #707 added a new
     test target.
   - Rebase before the PR can merge. A conflicting PR is not mergeable and its `pull_request` CI may not run.
2. **#643 (merged 20:38 MDT, after F's code) makes one sentence stale.** On current `main`, `pane list`, `get` and
   `watch` display the hold (`list.rs::hold_word`, `get.rs::hold_text`). Two places say otherwise:
   - `park.rs:40` says "Nothing in Holler reads the hold yet";
   - the CHANGELOG entry says "Nothing reads the hold yet".

   Both match the brief's base and Decision 10's own words, so this is not REWORK. When the conflict is resolved,
   reword both to say that nothing acts on the hold yet (the read verbs, #643, show it), and keep the prompt clause.
3. **Decision 13, at PR time:**
   - The title or squash subject is `feat(cli): holler pane park and unpark (#646 part 1 of 3)`.
   - The body says "Part of #646", with **no closing keyword**. #646 must stay open for 646b and 646c.
   - The body has an AI-assistance section per `CONTRIBUTING.md` (add it with `gh pr edit` if the script omits it).
   - The squash commit carries the `Co-Authored-By` trailer.
4. **Follow-ups still unfiled.** A-dup asked for issues with owners before merge:
   - **(a) ADR-0021 wording** (A warns 2 and 3). The text rule is the verb's guard, not the record's, so a reader still
     escapes. Record-only verbs print no reconcile step on `generation-conflict` or `timeout`. Section 12 (lines
     471-472) still reads as universal. A timed-out write may have landed, and the failure message does not say so.
     646b's brief edits ADR-0021 and can carry this.
   - **(b) One shared "panes in scope" helper** (A warn 4, A-dup warn 2). Until it exists, no other verb should import
     `HoldTarget` or `in_scope` from `park.rs`.
   - **(c) The rig consolidation** into `tests/verb_harness/` (A warn 5, A-dup warn 1).
   - **(d) Optionally, a guarded type** for the hold texts.
5. **The same stored hold now prints two ways.**
   - `pane get` prints `parked reason=<text_value> until=<text_value> since=<date>`. It is unquoted when the value is
     plain, it is never cut, and its label is `until`.
   - `pane park` prints `parked (reason <quoted>, release when <quoted>)`. It is always quoted, it is cut at 64 with
     `...`, and its label is `release when`.

   Both are terminal-safe, and the brief pins park's form. #643's `pane::list::text_value` ("every stored string the
   read verbs print in text mode goes through here") merged after the brief. A follow-up could align the write verbs'
   quoting and the `release_when` label.
6. **Small test gaps** (no REWORK):
   - AC 8c's rerun runs in text only. The JSON success shape is pinned by AC 2-5.
   - No test asserts zero profile-store reads for a PANE-alone run. Decision 3 is visibly met in `in_scope`, and AC 9's
     helper allows reads by design.
   - AC 2 checks `check_envelope(&out, 0)` without asserting `code == 0` itself. `run_both` asserts it for AC 3-5's
     successes.
