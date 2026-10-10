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
