# Handoff-S: Phase 8 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (spec audit, amendment 1)

**Date:** 2026-10-09 (20:26 MDT)
**Branch:** issue-643-implementation, at `f8df56a` (19 commits ahead of `origin/main`, 3 behind; `origin/main` is
`e327569`, merge base `ce12cdb`)
**Issue:** #643 (epic #633). Brief: `docs/handoffs/643-brief.md` (amendment 1). Rigor: second-opinion.
**Handoffs reviewed:**
- `handoff-A.md` (amendment 1): PASS, warns W-12 to W-16.
- `handoff-T-red.md` (amendment 1): RED, 31 passed and 5 failed.
- `handoff-F.md` (amendment 1): `done: true`, `archChanged: false`.
- `handoff-T-green.md` (amendment 1): GREEN, 36/36 and 127/127.
- `handoff-A-dup.md` (amendment 1): PASS.
- `decisions.md` and `evidence.md`.
- The outside gates, both PASS: the brief gate's `643-brief-result-r2.md` and this run's diff gate, `643-diff-result-r1.md`
  (20:12 MDT).

**Verdict:** PASS

## A precondition

Met. `handoff-A.md` (amendment 1, at `d525032`) shows **PASS** with 0 blocks. Its two act-now warns are in the code:
- W-12: `watch.rs:13` imports `holler_pane::profile_diff::is_member`, and `watch.rs:215` calls it.
- W-13: the guard is `at <= 0`, at `list.rs:233`.

W-14 to W-16 are follow-ups or evidence entries. `handoff-A-dup.md` (at `c65993b`) also shows **PASS**, with no block and
no new warn. It confirms the D-1 fold: one call to `shown_differs`, and `last_observed.driven` is never compared.

## T precondition

Met.
- **RED.** `handoff-T-red.md` ran `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::` on the
  unchanged round-1 production code: 31 passed, 5 failed. Each failure is an assertion about amendment 1's behaviour:
  - the three AC 3 tests (c1, c2, c3 and c6 differ, exactly the brief's "RED for amendment 1");
  - the two AC 19 help tests.

  Round 1's RED (`decisions.md`, T Phase 4) was 28 failures against the stubs.
- **GREEN.** `handoff-T-green.md`: 36 of 36 read-verb tests pass, and 127 of 127 in `pane_verbs`. The isolated
  workspace suite has 1472 passed and 0 failed. Its "Blocking issues" section says "None".
- **Mutations.** Mc1 to Mc5 each turn the tests that pin them red. Rounds 1 and 2 recorded M1 to M11 and Ma to Mc.

## Acceptance criteria

Paths: `L` = `crates/holler-cli/tests/pane_verbs/list.rs`, `G` = `.../get.rs`, `W` = `.../watch.rs`. Every test runs
the real verb in-process (clap, dispatch, `output.rs`) over the test kit's fakes. Each asserts literal output written
by T, not values computed by the code under test.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | List, text: header, rows sorted by name, cells, no trailing space | `list_prints_a_header_and_one_row_per_pane_sorted_by_name` (L:238). It seeds c2 before c1 and asserts the 3 lines, every cell and no trailing space. See advisory 2(c) on the sort | PASS |
| 2 | List, JSON: one envelope, nine keys, null and enum values | `list_json_is_one_envelope_with_a_row_per_pane` (L:281): `check_envelope(out, 0)`, exact key set, `null`, `"unknown"`, `"none"`, `"unobserved"` | PASS |
| 3 | SYNC is `shown_differs`'s answer; DRIVEN is printed, never compared (amendment 1) | `sync_rig` (L:315) is AC 3's six rows exactly, and `SYNC_WANT` (L:333) is their expected values. Tests: `list_flags_a_pane_whose_shown_differs_from_its_session_of_record` (L:342) for PANE, SHOWN, DRIVEN and SYNC cells and the JSON; `get_flags_a_mismatch` (G:157); `watch_flags_a_mismatch` (W:283). RED on the old rule; Mc2 to Mc4 | PASS |
| 4 | Unhealthy server shown | `list_shows_an_unhealthy_server` (L:383): the HEALTH cell, the JSON `{"unhealthy": ...}`, `get`'s `health: unhealthy "server wedged"` line and `data.pane.harness.health` | PASS |
| 5 | Positions row first | `every_verb_prints_positions_row_first` (L:410): pane `demo-c1r2` at row 2, col 1. It checks `r2c1` in all three text forms, the raw compact JSON in all three, and that nothing prints `c1r2` | PASS |
| 6 | `--profile` scopes each verb; refusals exit 3 in both formats | `list_profile_lists_only_the_profile_s_members` (L:447), `get_profile_member_is_shown` (G:146), `watch_profile_prints_only_member_changes` (W:220) and `profile_refusals_exit_3_in_both_formats` (L:467), all over `scoped_rig` (L:134), which is AC 6's store. The refusal test covers 3 `profile-not-found` and 4 `pane-not-in-profile` cases, each through `assert_fails`: exit 3, the same exit in both formats, one `error:` line, nothing on `out`, `check_envelope` | PASS |
| 7 | `get` shows the whole record, ceilings included | `get_shows_every_field_of_the_record` (G:67): the 12 whole lines; `data.pane == to_value(stored)` at generation 1; `data.profile`, `data.spec` (with `context.soft` and `.hard`) and `data.sync`, with and without `--profile` | PASS |
| 8 | `get` of a pane with no profile shows its ceilings | `get_pane_without_a_profile_shows_its_ceilings` (G:119): `profile: -`, `spec: -`, the `context:` line, `null` keys present, `data.pane.context` | PASS |
| 9 | Missing pane is `pane-not-found`; PANE is required | `get_missing_pane_is_pane_not_found_in_both_formats` (G:192), with the exact `err` line; `get_requires_a_pane_name` (G:206) gets `MissingRequiredArgument` | PASS |
| 10 | Bad names are usage (exit 2) | `bad_names_are_usage_in_both_formats` (L:496), the three argv | PASS |
| 11 | Store failures exit 1 | `store_failures_exit_1_in_both_formats` (L:509): `Fail(Unavailable)` gives `unavailable` and `Wedged` gives `timeout` for list, get and watch; `check_ndjson` for watch | PASS |
| 12 | `watch` from the start prints the current state once | `watch_from_the_start_prints_each_live_pane_once` (W:70) over `history_rig` (W:26): exactly `put 4 c2`, `put 6 c3` | PASS |
| 13 | `--since` prints each later change once, in order | `watch_since_prints_each_later_change_once` (W:81): cursors 3 to 6, the delete line's exact `data`, 4 text lines, `cursor=5 delete demo-c1r1`, and `--since 6` empty with exit 0 in both formats | PASS |
| 14 | A change made while `watch` waits prints exactly once | `watch_prints_a_concurrent_change_exactly_once` (W:150), five fresh rigs. Idle wait 2 s; it polls the call log for `WatchNext` with a 5 s bound, writes, and expects exactly cursor 7. This is a bounded poll on an observed condition, not a fixed sleep | PASS |
| 15 | Named pane; a store error ends the stream; `--since` ahead is usage | `watch_named_pane_follows_only_that_pane` (W:157), `watch_ends_at_a_store_error` (W:167) and `watch_since_ahead_of_the_head_is_usage` (W:195) | PASS |
| 16 | A watch that owes nothing prints nothing | `watch_with_nothing_owed_prints_nothing` (W:207): exit 0, empty `out` and `err`, both formats | PASS |
| 17 | The verbs observe nothing | `read_verbs_call_no_adapter_or_probe` (L:527): 5 runs times 2 formats; reads of both stores are present, writes absent, and `assert_nothing_observed` holds. The AC 17 grep prints nothing (re-run by S) | PASS |
| 18 | Text output cannot inject terminal sequences | `text_output_escapes_control_characters` (G:232): no ESC and no CR in list, get or watch, the newline count, the literal escaped cwd, and JSON passing the envelope helper. Also `text_output_escapes_c1_and_bidi_characters` (G:277) and `text_output_escapes_each_hidden_class_and_keeps_plain_unicode` (G:308) | PASS |
| 19 | Output documented in `--help` | `list_help_documents_the_columns_and_the_json_shape` (L:561, all 15 needles, `session of record`, `home screen` and `#649` among them), `get_help_documents_the_json_shape` (G:213) and `watch_help_documents_the_stream` (W:303, including `pane get`). Each renders clap's `DisplayHelp` (G:20) | PASS |
| 20 | Surface rows | `docs/adr/ADR-0003.md:44-46` matches AC 20 byte for byte, and `cli-surface.txt:105-113` matches Decision 13's nine lines (S ran `diff` of each against the brief: identical). `stub.rs:18` keeps `// #643`, and the row grep prints nothing (re-run by S). `cli_surface_test` (3), `docs_cli_test` (3) and `pane_cli_process` (34) pass (T-green) | PASS |
| 21 | No stub left | `grep -nE 'not_implemented\|const STORY'` on the three verb files prints nothing (re-run by S) | PASS |
| 22 | Quality gates | T-green's Tier 1: `lint.sh`, `changelog-check.sh`, clippy `-D warnings`, `pane_verbs`, the workspace (isolated, CI's skip), `cargo machete` and `rustfmt --check --edition 2021`, all passing. Re-run by S: the `unsafe` grep on the seven files prints nothing, the manifest diff is empty, and `wc -l` gives src 335, 277 and 223 and tests 587, 389 and 320 (all under 900). Clippy's `too_many_lines` is denied and passes | PASS |
| 23 | CHANGELOG | `CHANGELOG.md:167`, under `## [Unreleased]` (:8) and `### Enhancements` (:10). Both of amendment 1's sentences are there (the second verbatim), it ends with the `[#643]` link and names no host. The diff only adds lines, so the `doctor` and `profile` entries are untouched | PASS |
| 24 | Blast radius | `git diff --name-only origin/main...HEAD`: the brief's ten files plus `docs/handoffs/643*`, nothing else. No frozen file, manifest, `holler-pane`, test kit or ADR-0021 is touched (diff checked by S) | PASS |

The issue's own criteria map onto these:
- the SHOWN/DRIVEN flag: AC 3, under ruling (a1); see advisory 1;
- an unhealthy server: AC 4;
- each change once: AC 12 to 14;
- stable, documented output: AC 1, 2, 7, 13 and 19;
- `--profile`, the envelope helper and equal exit codes: AC 6, 9 to 11 and 15;
- the ceilings: AC 7 and 8;
- `r2c1`: AC 5.

## Spec compliance

Each decision is implemented as stated. The departures from the letter are recorded, and A asked for them; none is silent.

- **D1, arguments.** The positionals, `--since: Option<u64>` and `--until-idle: bool` are in place. Names are typed in
  `run` with `PaneName::parse` and `ProfileName::parse`, so a bad name is `usage`. `list PANE` with no record gives an
  empty table (`list.rs:100`; `list_named_pane_lists_only_that_pane`, L:266).
- **D2, shared code.** It is `pub` in `list.rs`, one copy each. `get` and `watch` import it through `super::list`.
- **D3, SYNC.**
  - `SessionSync::of(&Pane)` (`list.rs:230-240`) answers `Unobserved` when `at <= 0` or there is no session of record,
    and otherwise gives `shown_differs`'s answer.
  - It has two call sites, `list.rs:180` and `get.rs:88`, and `watch` reads SYNC through `PaneRow`.
  - `last_observed.driven` is only printed (`list.rs:179`, `get.rs:178`).
  - **Deviation:** `at <= 0` instead of the letter `at == 0`. A asked for it (W-13). It matches `shown_differs`'s own
    proviso, "provided `at > 0`" (`reconcile.rs:178`), and `observed_at`. It is recorded in `evidence.md` and
    handoff-F, and pinned by `a_negative_observed_at_is_never_observed` (G:171).
- **D4, list output.** The verb sorts (`list.rs:103`). The table has character widths, a two-space gap, an unpadded last
  column and a header-only form. The JSON is `{"panes": [...]}` with the nine keys in order, and every `null` is
  present.
- **D5, get output.**
  - The four keys are always present.
  - `profile` is the record's (`get.rs:86`).
  - The spec lookup follows both paths.
  - A `profile_store.get` error fails the verb (`get.rs:125`, `?`).
  - The text field order is exactly Decision 5's list (`get.rs:155-195`).
  - A scope answer without the named pane is `pane-not-in-profile` (`get.rs:101-108`). That is a superset of "empty
    `panes`", journalled as F's round-1 deviation 4 and accepted by A (round 2, B-1).
- **D6, argv in text.** Arrays print as JSON through `json_text`, as built (W-9, now in the brief).
- **D7, watch.**
  - `resolve` runs before `watch` opens, and errors go out through `emit_error`.
  - The idle, until-idle and error arms are in `watch.rs:161-180`.
  - Membership: `Members` (`watch.rs:198-222`) starts from `resolve` and is updated per event.
  - **Deviation:** `is_member` replaces the inline `ProfileName::slug` comparison. A asked for it (W-12). It is the same
    comparison, recorded in `evidence.md` and pinned by Mc5.
  - W-4's help sentence is present (`watch.rs:34-36`).
- **D8 and D9.** There is no verb-side dedupe. The port calls are only the ones Decision 9 lists.
- **D10 to D13.** All four are as specified:
  - the `Rig`;
  - `text_value`, with the W-9 trigger set now in the brief;
  - `observed_at`, which prints `never` for `ms <= 0`, otherwise the time plus ` UTC`;
  - the fixture block.
- **D14 to D16.** Exit codes go through `emit`. The verbs raise only the closed `PaneNotFound` and `PaneNotInProfile`.
  The help matches Decision 15's content and AC 19's needles. `list` makes at most one port call and `get` at most two.

## Quality audit

- **Correctness and failure handling.**
  - Every result and error goes out through `emit`, `emit_error` or `emit_stream`, so the exit codes are the same in
    both formats.
  - Store failures fail closed (exit 1, AC 11). `get` does not turn a profile-store error into `spec: null` (D5).
  - `watch` ends at the first error and does not reconnect.
  - The verbs write nothing, so no write can be lost.
  - AC 14 pins the exactly-once property on a concurrent write.
  - `json_text`'s `unwrap_or_default()` (`list.rs:306`) cannot be reached for the types it gets (evidence NV-1).
- **Build guards.**
  - The three source files contain no `unwrap`, `expect`, `panic!`, `unreachable!` or `todo!`.
  - The diff adds no `#[allow]` under `crates/`.
  - No touched file is near 900 lines; the largest is `tests/pane_verbs/list.rs` at 587.
  - There is no dead code: rustc denies `dead_code`, and every `pub` helper has a caller.
- **Protocol.** None. No change to `holler-proto`, the wire, a golden file or the error-code table. `docs/protocol/v2.md`
  does not apply. The new CLI JSON shapes are additive: new verbs, and objects that can take a field later.
- **Tests.**
  - All in-process over the kit's fakes, as the issue requires ("Tests use only #638's fakes and envelope helper").
    Until #649 the binary is `Unwired`, so a cross-process harness would only show `not implemented`.
  - The surface is tested at process level (`pane_cli_process`).
  - No fixed sleep is used as synchronization. AC 14 polls an observed condition with a bound.
  - RED-first evidence is in both T-red handoffs.
- **Documentation.**
  - The CHANGELOG entry is under `[Unreleased]` and links #643.
  - The CLI surface is in ADR 0003's rows and in `--help`.
  - No new log event or protocol field.
  - `README.md` lists no `holler pane` or `holler profile` verb, the same as #647's and #662's entries. Operator docs
    belong to #652.
- **Public-repository privacy.** S grepped all 5,229 added lines of the branch, handoff documents included. It searched
  for personal and account names, home paths, host and tailnet names, IPv4 addresses and secret-shaped values. The only
  hits are neutral:
  - the brief's own privacy statement;
  - the mutation labels `M1 to M11`;
  - the loopback `http://127.0.0.1:48100/health` that AC 7 calls for.

  Fixtures use `demo-*`, `ses-a`, `ses-b` and `/srv/demo`. The commit author is the GitHub no-reply address.
- **Commit and PR hygiene.** Every branch commit has a Conventional Commit subject (`chore(#643): ...`, `docs(#643):
  ...`, `docs(handoffs): ...`) and a `Co-Authored-By` trailer. No PR exists yet; see advisory 3.

## Scope check

Exactly the brief's scope:
- F edited the three verb files and `CHANGELOG.md`.
- T edited the three test files, the fixture block, ADR 0003's three rows and the `STUBS` rows.
- W-12 (`is_member`) and W-13 (`<= 0`) are in-file changes A asked for. They do not over-deliver.
- T-green round 1 added three tests (`list_named_pane_lists_only_that_pane`, `text_output_escapes_c1_and_bidi_characters`
  and `watch_profile_prints_a_pane_leaving_the_profile_once`). Each pins a stated decision (D1, D11, D7), inside #643's
  own test files.

Nothing is under-delivered. There is no unrelated refactor and no new file, and the 900-line gate needed no split.

## Verdict

**PASS.** All 24 acceptance criteria have proving tests that assert behaviour. Every brief decision is implemented, and
the two departures from the letter (W-12, W-13) are recorded and were asked for by A. Code, test and documentation
quality is acceptable, and the privacy sweep is clean. Ready for the run's agent to open the PR, verify CI and merge
(advisories 3 and 4 first).

## Advisory notes (non-blocking)

1. **Record ruling (a1) on the issue before merge.** #643's body still says "A pane whose SHOWN and DRIVEN differ is
   flagged". The shipped rule compares SHOWN with `session_of_record` (the MO's ruling (a1); the brief's Decision 3 and
   C8; ADR-0021 I2 and §11). AC 3's row c6 shows the visible consequence: the DRIVEN column reads `ses-b`, SHOWN reads
   `ses-a`, and SYNC reads `ok`. The ruling lives only in the brief and the handoffs, and the issue is the source of
   truth. One `gh issue comment 643` naming the ruling would make the issue match what ships. Until #649, no real
   record holds a `driven` value, so no user can see c6 yet.
2. **Implemented exactly as stated, but no test pins it** (none is an AC):
   - (a) **D5's fail-closed rule:** a `profile_store.get` error exits 1. No test sets a profile-store fault, so a
     change that swallowed the error into `spec: null` would pass every test. A guard test: the profile store
     `Fault::Fail(Unavailable)`, then `pane get <member>` exits 1 with `unavailable` in both formats.
   - (b) **D12's timestamp form for `at > 0`, and D5's quoted non-zero `since`** in the `hold:` line (the outside diff
     gate's W-3). Every test uses `at` or `since` of 0, or 1000 without reading `observed-at`.
   - (c) **D4's sort in the verb** (`list.rs:103`). T-green's M8 survived because every kit path already returns name
     order, and the issue limits tests to the kit.

   (a) and (b) fit in the next change to `tests/pane_verbs/get.rs`; D-5 already plans one. The JSON that scripts use is
   pinned verbatim by AC 7 (`data.pane == to_value(stored)`), which is why these do not block.
3. **PR hygiene for the run's agent.** After the script opens the PR:
   - add the AI disclosure to the body (`CONTRIBUTING.md`, `gh pr edit`);
   - make sure the title is a Conventional Commit `feat(cli): ...` subject, since it becomes the squash commit on
     `main`.

   No branch commit carries a session link. That matches `main` (none in its last 200 commits) and the
   `prepare-commit-msg` hook, so CONTRIBUTING's "and a session link" is not this branch's to fix.
4. **Merge `origin/main` (`e327569`, 3 commits) before the PR merges.** The three commits touch adapter crates,
   `Cargo.lock` and `CHANGELOG.md`. A-dup's `git merge-tree --write-tree origin/main HEAD` exited 0 at that head, and
   it was still the head after S's own `git fetch` this phase.
5. **File the follow-ups at merge.** None has an issue yet (A's Notes for O, 2):
   - D-2, widened by W-16;
   - D-3, D-4, D-5 and D-6;
   - #647's unseen first observation, which can produce a false MISMATCH;
   - W-14, before #646c;
   - W-15, for #648 or #649;
   - W-11, for #660.
6. **Cosmetic or informational:**
   - (a) T-green's note: `list.rs:6-7` and `:206` call `shown_differs` "the one SHOWN/DRIVEN rule" next to "DRIVEN ...
     never compared". That uses the word DRIVEN in two senses, the ADR's (the session of record) and the column's.
   - (b) T-green's Tier 1 row lists `rustfmt` on six files, but AC 22 names seven. `stub.rs` was in F's seven-file run,
     which T-green re-ran, and its diff is a three-line deletion.
   - (c) Once AC 20 removes the `STUBS` rows, no process-level test runs these three verbs on the real binary. The
     CHANGELOG line "the installed binary still answers `not implemented`" holds because of `Unwired` (`wiring.rs`,
     #649), so #649 should add that coverage when it wires the hub.
