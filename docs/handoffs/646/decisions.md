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
