# Handoff-S: Phase 8 - #662b profile write verbs (`holler profile create`, `holler profile delete`)  (spec audit)

**Date:** 2026-10-09 (audited about 21:35 MDT)
**Branch:** issue-662-implementation (worktree `.claude/worktrees/0662-profile-verbs`, head `7452ee3`, base `ce12cdb`)
**Issue:** #662 (epic #633). 662b closes the issue. I read it live with `gh issue view 662` and checked its edit history
(GraphQL `userContentEdits`). The last edit was 2026-10-08 18:56 MDT, and there are no comments. Epic #633's last edit was
2026-10-09 17:39 MDT. The brief was committed at 19:28 MDT, after both edits, and its E2 quotes that edit. Neither text
moved during the run.
**Brief:** `docs/handoffs/662-brief.md` (`79fc998`)
**Handoffs reviewed:**
- `handoff-A.md`, `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md` and `handoff-A-dup.md`;
- `decisions.md` and `evidence.md`;
- the outside model's diff review `662-diff-result-r1.md` (deepseek-v4-pro, round 1, PASS; gitignored).

**Verdict:** PASS

## A precondition

Met.
- `handoff-A.md` returned PASS on the plan (0 blocks, 4 warns).
- `handoff-A-dup.md` returned PASS on the diff `ce12cdb..5396f54` (0 blocks, 5 warns).

## T precondition

Met. `handoff-T-green.md` reports "Blocking issues: None".

- **RED** (T-red, head `795717e`): `profile_verbs` gave 18 passed and 25 failed, the same in 6 runs.
  - Each of the 25 new verb tests failed on its first behavioural assertion, against the stub's `not-implemented`.
  - None failed on a compile error or a clap panic.
  - `create_flags_conflict` passed early. It is a clap property, and T journalled it.
- **GREEN** (T-green):
  - `profile_verbs`: 44 of 44, in each of 10 runs.
  - The workspace, in CI's form (`--skip roster_stays_accurate_under_concurrent_body_load`, with an isolated
    `HOLLER_STATE_DIR` on this machine): 1464 passed, 0 failed.
  - Clippy, lint, fmt, changelog, machete, the canary and the hooks are all clean.
- **Mutation check:** T made 9 mutations. 8 were caught at once. The 9th (the undo runs oldest first) survived, and T's
  added test `create_from_current_undoes_the_joined_panes_newest_first` now catches it.
- **Since T-green:** only docs changed. `git diff --stat 5396f54..HEAD` lists only `decisions.md` and `handoff-A-dup.md`.

## Acceptance criteria

The tests are in `crates/holler-cli/tests/profile_verbs/` unless a row says otherwise. "S" means I re-ran the read-only
check myself. Every verb test runs through `run_both_with` (or `run_both` or `run_both_seamed`, which call it). That runs
both formats on fresh rigs, checks that the exit codes match, checks the JSON with `check_envelope`, and calls
`assert_no_adapter_call()` after each run.

| Criterion | Proving test or evidence | Status |
|---|---|---|
| Issue: each verb's happy path and every refusal, in both formats; the JSON passes the envelope helper; exit codes match | Happy paths: AC 2a, 2d, 2f, 3a, 3c. Refusals: `profile-exists` 2b and 2c; `profile-not-found` 2e and 3d; `pane-in-other-profile` 2g; `profile-has-live-panes` 3b and 3h; `profile-conflict` 2i and 3f. All run through `run_both_with`. | Met |
| Issue: `create --from P` is a detached copy (no `Pane.profile` changes; a spec naming a pane of another profile is kept) | AC 2d | Met |
| Issue: `--from-current` gives one spec per pane with the right GridPos, cwd, harness, model and effort, role, env names and ceilings | AC 2f: the specs equal `profile_from_panes(NAME, seeded)`, and the r2c1 grid is checked in the JSON. 662a's field mapping is proven by `crates/holler-pane/tests/profile_snapshot_test.rs::snapshot_copies_every_spec_field_from_the_pane_record` and `::snapshot_takes_the_position_from_the_record_not_the_name`. `PaneRole` has two variants, so "distinct role" can only mean "not all the same". | Met |
| Issue: `delete --keep-panes` leaves every pane running with `profile` None | AC 3c: each record is kept, with only `profile` and `generation` changed. No adapter is called, so nothing is closed. | Met |
| Issue: no profile ever holds an env value (I7) | AC 2j | Met |
| Issue: `--format=text|json` through `output::emit()`, with stable codes | `create.rs:65-71` and `delete.rs:50-56` call `emit` once. The codes come from the closed `PaneError` set, and `error.rs` is not edited. | Met |
| AC 1: no adapter, no probe, no environment | S: the grep prints nothing. `run_both_with` (`rig.rs:133`) calls `assert_no_adapter_call` after each run (`rig.rs:140`, `:143`). `create_flags_conflict` runs no verb. | Met |
| AC 2a | `create.rs::create_makes_an_empty_profile_in_both_formats` | Met |
| AC 2b | `create.rs::create_refuses_a_taken_name_or_slug` | Met |
| AC 2c | `create.rs::create_reports_a_create_race_as_profile_exists` (caught mutation M2) | Met |
| AC 2d | `create.rs::create_from_makes_a_detached_copy`, also run with the source spelled `ALPHA` | Met |
| AC 2e | `create.rs::create_from_a_missing_profile_is_profile_not_found` | Met |
| AC 2f | `create.rs::create_from_current_snapshots_every_pane_and_joins_it` | Met |
| AC 2g | `create.rs::create_from_current_refuses_a_pane_in_another_profile_and_writes_nothing`: two panes, `; `-joined, no write in either call log | Met |
| AC 2h | `create.rs::create_from_current_undoes_everything_when_a_join_fails` | Met |
| AC 2i | `create.rs::create_from_current_reports_profile_conflict_when_the_undo_fails`: checks the one-line `err` | Met |
| AC 2j | `create.rs::create_holds_env_names_never_values` | Met |
| AC 2k | `create.rs::create_flags_conflict` (`ErrorKind::ArgumentConflict`) | Met |
| AC 2l | `create.rs::create_with_a_bad_name_is_usage_in_both_formats`. Its `--from` case runs `create Fresh --from "   "`, not the AC's `Demo` (line 554). The AC does not say Demo is seeded, so the criterion is proven (advisory 2). | Met |
| AC 2m | `create.rs::create_from_current_undoes_a_join_that_landed_but_timed_out` (caught mutation M1) | Met |
| AC 3a | `delete.rs::delete_removes_a_profile_without_members_in_both_formats`: run with and without `--keep-panes` (B6), and checks the compact key order | Met |
| AC 3b | `delete.rs::delete_refuses_while_panes_are_live` | Met |
| AC 3c | `delete.rs::delete_keep_panes_detaches_then_deletes` | Met |
| AC 3d | `delete.rs::delete_of_a_missing_profile_is_profile_not_found` | Met |
| AC 3e | `delete.rs::delete_conflict_without_members_is_generation_conflict`: run with and without `--keep-panes` | Met |
| AC 3f | `delete.rs::delete_conflict_after_a_detach_is_profile_conflict` (caught mutation M3) | Met |
| AC 3g | `delete.rs::delete_stops_at_a_failed_detach` | Met |
| AC 3h | `delete.rs::delete_counts_members_by_slug` | Met |
| AC 4: the stub cases are gone | S: `assert_stub_routes` is in neither file. `grep -c '662),' stub.rs` prints `0`, and `// #662` is kept at `stub.rs:35`. | Met |
| AC 5: the surface | S: in ADR-0003, lines 65-66 match the AC exactly, and `#662` is at column 67 on lines 65-68. The `# #662` fixture group is exactly the nine AC lines. T-green: `cli_surface_test`, `docs_cli_test` and `pane_cli_process` pass. | Met |
| AC 6: ADR-0021, Decision 14 (ii) | S: each grep prints one line (346 and 347). The ADR-0021 diff is those two rows only, and each gains `profile-conflict` as its last code. | Met |
| AC 7: the crate tests | T-green: `profile_verbs` 44 of 44, which includes every AC 2-3 name. The workspace gave 1464 passed and 0 failed. | Met |
| AC 8: lints | T-green: clippy `-D warnings` and `lint.sh` pass. S: no `#[allow]` is added. The touched files have 346, 219, 564, 299, 306 and 215 lines. Clippy's `too_many_lines` is set to deny. | Met |
| AC 9: no new `unsafe`, no new dependency | S: no added line has `unsafe`, and the manifest and lock diff is 0 lines | Met |
| AC 10: formatting | T-green: `rustfmt --check --edition 2021` exits 0 on all 7 touched `.rs` files | Met |
| AC 11: the CHANGELOG | S: one entry under `## [Unreleased]` / `### Enhancements`, after 662a's. It links #662 and names no host. Its claim that the binary answers `not-implemented` until #649 holds: `pane/wiring.rs`'s `Unwired` stores answer `PaneError::NotImplemented`. T-green: `changelog-check: ok`. | Met |
| AC 12: the blast radius | S: `git diff --name-only origin/main...HEAD` lists exactly the brief's Files plus `docs/handoffs/662*` | Met |

Four of T's tests prove decisions that have no numbered AC:
- `create_from_current_of_no_panes_is_an_empty_profile_with_no_members` (Decision 2, `members: none`);
- `create_from_current_joins_a_pane_that_already_names_the_profile` (Decision 2; caught M5);
- `delete_stops_at_a_failed_second_detach_naming_the_first` (B4's `detached so far: a`, the gap A's W-4 named);
- `delete_quotes_a_name_with_a_quote_in_its_suggested_command` (B1's `'\''` branch; caught M4).

`delete_with_a_bad_name_is_usage_in_both_formats` also pins B3's parse-first rule. With T-green's newest-first test, that
makes 27 verb tests.

The tests assert behaviour: exit codes, envelope codes, message parts, and the stores' state on both rigs. They check
call logs only for the brief's "no write" claims. Each one fails when the change is removed: T-red shows them all red
against the stub, and the mutations show each branch is caught.

## Spec compliance

I checked each decision against `src/profile/create.rs` and `src/profile/delete.rs`.

- **The carried structs and names.** `ProfileCreate { name, from_current (conflicts_with = "from"), from }` and
  `ProfileDelete { name, keep_panes }` match the brief. The names are `String` at parse time and go through
  `ProfileName::parse` in the verb, so a bad name is a `usage` envelope in both formats.
- **B3, the order of the checks:** done. `create`'s plan (`create.rs:109-126`) runs:
  1. parse NAME;
  2. parse PROFILE;
  3. build the actor;
  4. `get(NAME)` (`profile-exists`);
  5. `get(PROFILE)` (`copy_of`, `profile-not-found`), or `list()` and the other-profile check (`snapshot`).

  `delete`'s plan (`delete.rs:80-103`) parses NAME, builds the actor, then runs `get` and `list()`. Every check runs
  before the first write.
- **B4, the exact messages:** each of the 11 rows matches the code word for word:
  - `taken`;
  - the `PaneInOtherProfile` join;
  - `JoinFailed::body`, both arms; the `\` line continuations join the text into one line;
  - `has_live_panes`;
  - `detach_all`;
  - `delete_failed`.

  In each message, `<name>` is the stored display name when a record exists, and NAME as parsed when none does.
- **B2, the undo:** done (`create.rs:309-327`), in the brief's order:
  1. re-read the failed pane, and detach it only if `is_member`;
  2. detach the joined panes, newest first (`joined.iter().rev()`), each at the generation `cas_put` returned;
  3. delete the profile at the generation it was stored at.

  The `?` on each step stops the undo at the first failure, so the profile is kept whenever a member may remain.
- **B5, the data comes from the stored records:** done.
  - `data.profile` is the record `insert_profile` returned. 2a and 2f compare it with the stored record.
  - `members` is in join order.
  - `copied from` uses the source's stored name. 2d runs with `ALPHA`.
  - `delete`'s `name` and `slug` come from the record `get` returned.
- **B6 and B7:** done. `insert_profile` has B7's signature, maps only `Conflict` to `profile-exists`, and is the one
  create write of all three forms.
- **B8 and C3:** no call to `ProfileScope`, an adapter or the prober (AC 1).
- **Carried Decisions 2, 4, 7, 8, 9, 10, 12 and 14 (ii), and C2 and C7:** done as stated. The actors are the literal
  verbs. Membership is `is_member` in all three places, with no inline slug filter. The membership writes change only
  `profile` (2f and 3c check every other field).
- **Documented changes, none silent.** F's "Deviations" and T's handoffs record each one:
  - B1's helper is named `single_quoted`, not `shell_word` (A's W-3). Its body, home and visibility are as B1 says.
  - `detach` and `pane_list` are added as shared `pub(crate)` helpers in `delete.rs`, so that neither verb copies the
    other's code.
  - Decision 13's seam is in `rig.rs`, not in `create.rs` (A's W-4), and `delete.rs` uses it too.
  - The actor's parse error goes through the one `emit` call. That is `emit_error`'s own body, so the output is the same.

## Quality audit

- **Correctness and failure handling.**
  - Every write is a compare-and-swap at the generation the verb read, so no concurrent write is lost. A race fails
    loudly instead:
    - a join or detach answers `generation-conflict`;
    - the create race is `profile-exists`;
    - a profile that moved after a detach is `profile-conflict`.
  - A store error (`unavailable`, `timeout`, `store-corrupt`) passes through with its own code.
  - A refused create writes nothing, and the call-log checks in 2b, 2e and 2g prove it. A failed create leaves no profile
    and no member, or else reports `profile-conflict` with the profile kept, so the remedy it prints can run.
    `delete` never re-attaches.
  - Two ambiguous answers stay unmapped, as the brief says (advisory 3). Re-running the verb converges in both cases.
- **Build guards.**
  - The two source files have no `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or print macro (S grep).
  - No `#[allow]` is added.
  - The largest touched file has 564 lines, against the limit of 900.
  - There is no dead code. Every `Form` variant, `JoinFailed` field and new rig helper has a user, and clippy `-D
    warnings` is clean on all targets.
- **Protocol.** None. No wire method, field, golden file or `docs/protocol/v2.md` changes.
- **Tests.**
  - The verbs run in-process over the test kit's fakes, which is what the issue asks for ("test kit only"). The real
    stores are #649's scenario.
  - No test sleeps. The `NthCasPut` seam is deterministic, and its only state is its counter.
  - The RED-first evidence is in T-red.
- **Documentation.**
  - The CHANGELOG entry is accurate.
  - The CLI surface is in ADR-0003, and the codes are in ADR-0021's section 9 rows.
  - No README or `docs/` page lists the profile verbs yet. The operator guide is #652's.
  - The verbs add no log event.
- **Public-repository privacy.**
  - I grepped the whole diff, handoff docs included, for personal and account names, hosts, tailnet names, private IPs,
    home paths and key names. There is no hit. The only IP is `127.0.0.1`, in test data.
  - `gitleaks git --log-opts=origin/main..HEAD` scanned 6 commits and found no leaks.
  - The test data is neutral: `demo-*`, `/srv/demo`, `demo-provider`, `Some Profile`, `Alpha`, `Other`.
  - The commit identity is the GitHub no-reply address.
- **Commit and PR hygiene.**
  - The six commits use Conventional Commit subjects: the pipeline's own `chore(#662): <phase> -- ...` and
    `docs(handoffs): ...`. Each has a `Co-Authored-By` trailer. Like `main`'s squash commits and the #647 precedent, they
    have no session link.
  - There is no PR yet (advisory 5).

## Scope check

- **Delivered:** the brief's whole scope. F edited exactly the brief's production files: the two verbs, the two
  ADR-0003 rows, the two ADR-0021 rows and one CHANGELOG entry.
- **Over-delivery,** each small and explained:
  - `detach` and `pane_list` (one copy each, used by both verbs);
  - help-text doc comments on the two `Args` structs;
  - T's four extra tests and T-green's ordering test;
  - additive helpers in `rig.rs`. The brief allows these, and `run_both`'s behaviour is unchanged: it now delegates to
    `run_both_with(.., Rig::run)`.
- **Under-delivery against the brief:** none.
- **Open against project convention, not against the brief:** A's Phase 3 W-2, which Phase 7 repeats as W-4 (advisory
  1).

## Verdict

**PASS.** Every issue and brief criterion has a test that proves its behaviour, and every brief decision is built as
stated. The four changes from the brief are documented, and the quality gates hold. Ready for O.

## Advisory notes (non-blocking)

1. **ADR-0021 still lags this change, and no follow-up is filed** (A's W-2 at Phase 3 and W-4 at Phase 7). I searched
   GitHub's issues for `profile-conflict` and `keep-panes`. The only hits are the feature issues and the epic, and none of
   them is this follow-up.
   - The section 9 rows now list `profile-conflict` for `profile create` and `profile delete`. But the code table at
     line 395 still defines it as "The profile moved after the live change (the I8 write order); a race."
   - `create`'s undo-failed case fits neither part. In AC 2i the undo fails with `unavailable`, and the profile did not
     move.
   - Section 3 (line 149) records `--from-current`'s `Pane.profile` writes, but not `delete --keep-panes`'s.
   - The architecture overlay asks for the ADR to be updated "in the same change". AC 6 forbids that edit.

   I accept the follow-up route, not a hold, for three reasons:
   - A rated it a warn twice and named the follow-up as acceptable;
   - the issue itself lists `profile-conflict` among these verbs' codes;
   - until #649, the real binary answers `not-implemented` for both verbs, so no script can see either code yet.

   **Before merge, the MO files the issue, or relaxes AC 6 and adds the two sentences:**
   - (i) widen line 395's Reason to "a later write of a multi-write verb, or its compensation, lost after an earlier
     write landed; the message names the reconcile step";
   - (ii) after the `--from-current` sentence in section 3, record that `--keep-panes` sets each member's
     `Pane.profile` to none by compare-and-swap, and then deletes the profile.
2. **Not every B3 tie is pinned.** The code follows B3, but no test has two errors that both apply, where B3 chooses the
   one reported.
   - `create_with_a_bad_name_is_usage_in_both_formats` seeds `Demo` but runs `create Fresh --from "   "`
     (`create.rs:554`). Running the AC's `Demo` would pin "parse PROFILE before `get(NAME)`" (usage, not
     `profile-exists`).
   - Nothing pins `get(NAME)` before `get(PROFILE)` (`create Demo --from Missing` with Demo stored is `profile-exists`).
   - Nothing pins `get(NAME)` before the other-profile check (`create Demo --from-current` with Demo stored and a pane in
     another profile is `profile-exists`).

   These are cheap test-only additions for a later run. They are not required here.
3. **Two ambiguous answers stay unmapped, as the brief specifies** (A's Phase 3 note; the outside model's W-1).
   - **(a)** `insert_profile` answers `timeout` or `unavailable` after its write landed. The verb reports that code, and a
     profile with specs and no members may remain. A re-run answers `profile-exists`, and `profile show` shows the state.
   - **(b)** A non-`Conflict` error from `delete`'s last step after some detaches passes through without the detached
     list. A re-run converges.

   If #649's scenario or #650 needs more, re-reading NAME after an ambiguous `insert_profile` answer is the extension.
4. **Follow-ups from A-dup, unchanged:**
   - W-1: a shared profile lookup and members helper, in #664's brief;
   - W-2 and W-3: profile-command builders, and the `single_quoted` fold with #663's private copy, owned by whichever of
     #663 and 662b merges second, or by a filed issue;
   - W-5: move `stored` and one write-count helper into `rig.rs` when #664 extends it;
   - Phase 3 W-1: #650 moves `insert_profile` (and `join`, `undo` and `detach` if it reuses them) down into
     `holler-pane` when its engine needs them.
5. **The PR.**
   - The body says `Closes #662`, as the brief requires.
   - Add the AI disclosure (`CONTRIBUTING.md`) with `gh pr edit`, as CLAUDE.md says. The script's own body lacks it.
   - Mirror #703's title, for example "... (#662 part 2 of 2)".
6. **Merging.** `origin/main` is six commits past the base:
   - `0ad2d8a` (#640 part 2), `dc300ab` (#642 part 1), `e327569` (#641);
   - `3107487` (#708), `efd9a00` (#643), `519947a` (#707).

   A read-only `git merge-tree ce12cdb origin/main HEAD` shows one textual conflict, in `CHANGELOG.md`. Keep both
   entries. `cli-surface.txt`, `stub.rs` and `ADR-0003.md` change on both sides but merge cleanly. After the rebase, let
   CI re-run the surface targets (#643 edited the same files).
7. **The outside model's diff review** (deepseek-v4-pro, round 1) is PASS with no blocks.
   - Three of its needs-verification items are now settled:
     - NV-4 by 662a's `profile_snapshot_test.rs`;
     - NV-5 by T's evidence;
     - NV-9 by S's column check (67).
   - Its NIT-2 and NIT-3 describe doc text that is not in the files. Both are misreads, and no action is needed.
