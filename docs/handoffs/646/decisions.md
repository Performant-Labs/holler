# Decision journal: #646 part 1 of 3 (646a), `holler pane park` and `holler pane unpark`

## A (Phase 3, up-front plan review) — 2026-10-09T19:57:46-06:00
- **Decided:** PASS, with no block and six warns (`docs/handoffs/646/handoff-A.md`).
  - The plan extends `PanePark` and `PaneUnpark` in their own files, copies doctor's `run`/`pass`/`emit` shape, scopes
    through `ProfileScope::resolve`, and records by one `cas_put` at the read generation, as reconcile does.
  - Its engine in `park.rs`, shared with `unpark.rs`, follows the `profile/list.rs` `count` precedent. ADR-0021 section 5's
    closed list of `holler-pane` engines leaves park in the CLI.
  - The warns:
    1. A second hold-reason guard beside `holler_hub::holds::sanitize_reason` and `MAX_REASON_CHARS`: same 200 cap,
       opposite policy (refuse rather than cut), not named in the brief.
    2. The ADR paragraph states as a record property a guard only the verb enforces (the hub stores any `hold`).
    3. The paragraph omits the exception to the reconcile-step rule of sections 8 and 12 for a verb with no live act.
    4. The scoping arms copy the private `reconcile::resolve`, with no shared helper.
    5. The Reuse map misses #662's `profile_verbs/rig.rs`, the closest rig on main.
    6. No size estimate or split point for `tests/pane_verbs/park.rs`, against the lint's 600/900 gate.
- **Assumed:**
  - The Reuse map in the brief is the survey (there is no `survey.md` for this run).
  - On PASS the automated path goes to T without amending the brief, so the warns are advisory. Warns 2 and 3 can then
    land only through a later brief amendment or a follow-up, because AC 12 pins the ADR text.
  - #643's and #644's plans (`origin/issue-643-implementation` at `2e7f221`, `origin/issue-644-implementation` at
    `7195993`) and #663's (local `issue-663-implementation` at `137c00f`) are current enough to judge cross-story overlap
    and ADR merge hygiene.
- **Hedged:**
  - Warn 1 sits between warn and block. It stays a warn because the codebase is split by domain: the pane domain's
    `ProfileName` and `Actor` guards refuse, and the prompt hold's `sanitize_reason` cuts. ADR-0021 line 44 also keeps the
    two holds apart.
  - Warn 3 sits between warn and block too. It stays a warn because the brief does edit ADR-0021 in the same change, and
    the behaviour (no reconcile step for a record-only verb) is right. #663's second plan review graded the same kind of
    ADR gap a warn (its W-8).
- **Evidence:**
  - The brief in full, and the three brief-gate results (`646-brief-result-r{1,2,3}.md`).
  - Issue #646 (`gh issue view 646`).
  - `docs/adr/ADR-0021.md` in full, and `docs/adr/ADR-0003.md` lines 30-99.
  - `crates/holler-cli/src/pane/{doctor,mod,args,wiring,park,unpark}.rs`, `src/output.rs`, `src/profile/{list,show}.rs`.
  - `crates/holler-pane/src/{lib,ports,pane}.rs`, `profile.rs:240-404`, `reconcile.rs:1-250`, `findings.rs:326-336`, the
    `error.rs` `excerpt` and `Display`.
  - `crates/holler-pane-testkit/src/{pane_store,profile_scope,fixture}.rs` (the parts cited).
  - `crates/holler-hub/src/holds.rs:55-234`, `panes/store.rs:150-230`, `panes/handlers.rs:95-133`.
  - `crates/holler-cli/tests/{verb_harness/mod.rs,profile_verbs/{rig,main}.rs,pane_verbs/main.rs}`, the `doctor/rig.rs`
    outline, `pane_verbs/process/{stub,flags,docs_rows}.rs`, `tests/fixtures/cli-surface.txt`, `tests/docs_cli_test.rs`
    (the scan and normalisation), and `scripts/lint.sh:43-52`.
  - #644's plan review, #663's second plan review, the #643 brief, the #663 `StoreScope` (`resolve`, `member`), and
    `git log ce12cdb..origin/main` (two adapter-only commits).

## T (Phase 4, author tests / RED): 2026-10-09T20:06:40-06:00
- **Decided:** RED is valid. 10 new in-process tests over a new rig (`tests/pane_verbs/park/rig.rs`): 8 in `park.rs`
  and `park/failures.rs`, 2 in `unpark.rs`. All 10 fail on the stub's `not-implemented` (exit 1, or the envelope's code
  against `generation-conflict`); the other 93 `pane_verbs` cases pass (`docs/handoffs/646/handoff-T-red.md`).
  - The brief's test plan has T land the argument structs (`PANE`, required `--reason`/`--release-when`) with `run`
    still a stub. The surface edits had to land in the same step: ADR 0003 rows 54-55, the four `cli-surface.txt` lines,
    and removing park/unpark from `process/stub.rs`. Without them `holler pane park` exits 2 from clap and breaks
    `stub_verb_not_implemented`, `cli_surface_test` and `docs_cli_test`.
  - AC 8 lives in `park/failures.rs` as a check that takes the verb, which `unpark.rs` calls for AC 8d (A warn 6). The
    rig mirrors #662's names (A warn 5).
- **Assumed:**
  - For an unchanged pane, AC 3's JSON `data` carries `generation` 1 and the stored hold (Decision 8).
  - AC 7b and 7c's messages are `PaneName::parse` and `ProfileName::parse`'s own, compared against those functions'
    output rather than a hard-coded string.
  - ADR-0021's paragraph (AC 12) and the CHANGELOG entry (AC 13) are F's, checked by grep and script at S, so no
    test pins them.
- **Hedged:**
  - The brief puts "unpark's cases of AC 3-10" in `unpark.rs`. They live in `park.rs` under the brief's own
    `park_and_unpark_*` test names, which cover both verbs; `unpark.rs` keeps AC 8d and the AC 5 named-member case.
  - A throwaway prototype of the verb was used only to prove the tests can pass and catch mutations. It was restored to
    the stub, never staged, and is not F's design.
- **Evidence:**
  - RED: `cargo test -p holler-cli --test pane_verbs` gives `93 passed; 10 failed`, each failure the stub's answer at the
    first behavior assertion.
  - Against the prototype: `10 passed`, and 4 of 4 mutations caught (untrimmed store, overwrite any hold, continue after
    a failure, suffix on one pane).
  - `pane_cli_process` 34 passed, `cli_surface_test` 3 passed, `docs_cli_test` 3 passed.
  - `cargo clippy -p holler-cli --all-targets -D warnings` clean, `rustfmt --check` clean, `scripts/lint.sh` exit 0, and
    `git diff HEAD -- '*Cargo.toml'` empty. The `origin/main` manifest diff is upstream #702/#705 drift (noted for AC 13).

## F (Phase 5, implement): 2026-10-09T20:20:29-06:00
- **Decided:** the verbs are real record changes. `park.rs` holds the one engine (`run_hold_change`, `change_holds`), the
  scope rule (`in_scope`), the text guards (`checked_text`, `MAX_TEXT_CHARS`) and the renderer; `unpark.rs` types its
  target and calls `run_hold_change` with `HoldChange::Unpark` (Decision 9, the `profile/list.rs` `count` precedent).
  The 10 RED tests pass, 103 of 103 in `pane_verbs` (`docs/handoffs/646/handoff-F.md`).
  - The typed request is `HoldRequest { target: HoldTarget, change: HoldChange }`. `HoldTarget` is `Named(PaneName)` or
    `Profile { profile, pane }`, so "neither PANE nor `--profile`" cannot be represented after typing.
  - The text line is derived from the report entry (`changed` and the record's `Hold`) plus the change, only for a
    drained pane (park: `drained, left as it is`; unpark: `not parked`). JSON stays exactly `{name, changed, generation,
    hold}`, `hold` being the record's own `Hold` value.
  - A warns 1 and 4 taken in code: the cap is a named constant whose doc cites `holler_hub::holds::MAX_REASON_CHARS`
    without importing it, and `in_scope` copies reconcile's `resolve` arms, the same `PaneNotFound { what: name }`.
  - A warn 3 taken in the module doc only: no reconcile step is printed (no live act), and a timed-out write may have
    landed, which the rerun reports as already in the asked state.
  - ADR-0021: Decision 12's paragraph inserted verbatim after line 130, line breaks as in the brief (AC 12). CHANGELOG:
    one entry under `[Unreleased]` / `Enhancements`, "part 1 of 3" of #646.
- **Assumed:**
  - The empty-profile text names the profile as typed (`--profile` after `ProfileName::parse`), not the stored display
    name; they are equal in every test. Doctor's report also echoes the requested profile.
  - The phase this run calls "Phase 6" is F (the script's own numbering); the role doc's numbering calls it Phase 5.
- **Hedged:**
  - A warns 2 and 3 are not in the ADR text: AC 12 pins the paragraph to Decision 12's words, and A's own notes say F
    cannot take them without departing from the brief. Both are listed as a follow-up for O (646b's brief also edits
    ADR-0021 and can carry the two sentences).
  - `park.rs` is 401 lines, against the brief's estimate of about 200: the module doc and rustfmt's layout. It is under
    lint's 600-line warning.
- **Evidence:**
  - `cargo test -p holler-cli --test pane_verbs`: 103 passed (the `park` filter 10, the `unpark` filter 9).
  - `pane_cli_process` 34, `cli_surface_test` 3, `docs_cli_test` 3, `profile_verbs` 19 and the CLI's lib tests 9, all
    passed.
  - `cargo clippy -p holler-cli --all-targets -- -D warnings` and `cargo clippy --workspace --all-targets -- -D
    warnings` clean; `rustfmt --check --edition 2021` on both files clean; `bash scripts/lint.sh` exit 0 (only the
    pre-existing size warnings); `bash scripts/changelog-check.sh` ok; `git diff --stat ce12cdb -- '*Cargo.toml'` empty.
  - The source facts relied on are in `docs/handoffs/646/evidence.md`.

## T (Phase 7, verify GREEN): 2026-10-09T20:31:10-06:00
- **Decided:**
  - GREEN is valid. `pane_verbs` passes 103 of 103, F changed no test, and 12 mutations of F's real `park.rs` were each
    caught, one of them only after a test repair.
  - The repair is T's: `ProfileScope::resolve` promises no order and the fake scope sorts, so the AC 4 test could not see
    the verb's own sort. `park.rs`'s test now runs park and unpark over a test-local `ReversedScope` and asserts the
    lines are still in name order. No production change was needed, so PASS.
- **Assumed:**
  - The 4 `logging_test` failures in the first workspace run are the session's `HOLLER_STATE_DIR`, which points at a live
    hub. With an empty state dir, as on CI, the workspace passes 1448 of 1448.
  - The repo-wide `cargo fmt --check` drift in untouched crates is pre-existing and out of scope (CI does not run it;
    epic ruling 4).
- **Hedged:**
  - AC 13's literal `git diff --stat origin/main -- '*Cargo.toml'` is non-empty from upstream drift. It is empty against
    the merge base `ce12cdb`. Noted for S and for the rebase before the PR.
- **Evidence:**
  - `docs/handoffs/646/handoff-T-green.md`: the Tier 1 and Tier 2 tables and the mutation table.
  - A new T entry in `docs/handoffs/646/evidence.md` (`profile.rs:380-404`).
