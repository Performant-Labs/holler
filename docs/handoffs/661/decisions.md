# Decisions — #661 the hub profile registry

## A (Phase 3, up-front plan review) — 2026-10-09T14:41:53-06:00
- **Decided:** PASS on docs/handoffs/661-brief.md at 075a83a, with 8 warns and no blocks (see handoff-A.md). The plan extends #639's pane registry through the seam #639 documented for #661 (`persist`, `feed`, `handlers::run` via `RegistryEntry`, plus `PaneStoreOptions`, `WATCH_WAIT`, `log_fault` and `next_generation`). The one new object, `profile::store::Store`, is justified in writing by the issue's "one comparison" limit on `panes/**`. D7 sits under the pane lock after `next_generation`, as ADR "Decisions taken" 2 and the fake require. D8 reads the profile registry before the pane lock, so no lock nests. No wire, golden, error-code or ADR change. No size block: #639 landed the same shape in one run (12 files, +2,342 lines), and the 661a/661b seam is clean if F invokes the scope cap.
- **Assumed:** T and F read handoff-A.md (the run context points them at it; T's role doc does not say so explicitly), so findings 3-6 are addressed to them directly. #639's registry passes the 18 non-membership pane conformance cases; it has never run the suite. If it does not, the brief's escalate rule applies; I did not run it.
- **Hedged:**
  - Finding 1: the mirrored store is about 120 lines across ten items, not "about 60". It is a warn, not a block, because the brief justifies the new object in writing and generalising #639's store would exceed the issue's blast radius. It needs a tracked follow-up, not "a follow-up may".
  - Finding 3: the brief lists `drain_profiles`, a near-copy of `pane_support::drain`, while its own Reuse map calls such a copy an A-dup block. I made it a warn for T to fix rather than stop the run for a one-line brief edit.
  - Finding 8: D9 makes "reconcile is the net" depend on #647 handling `profile-not-found`. That is a product consequence for the operator, not drift.
- **Evidence:**
  - Read in full: panes/{mod,store,persist,feed,handlers}.rs, profile/{mod,rename}.rs, pane_dispatch.rs, holler-pane/src/{profile,reply,generation,pane}.rs and error.rs:380-710.
  - Test kit: profile_store.rs, pane_store.rs, conformance/{mod,profile_store,profile_store/log,profile_store/watch}.rs, conformance/pane_store.rs:1-120 and :370-534, and fixture.rs.
  - Tests: tests/pane_support/mod.rs, pane_dispatch_test.rs, pane_handlers_test.rs:1-200 and pane_registry_test.rs:60-194.
  - Docs and issues: ADR-0021 §1-2 and §6-8 plus "Decisions taken"; issue #661; epic #633 "Skeleton split" and "Decisions 2026-10-09".
  - Commands: `git show --stat 2a6f349` (the #639 size), `wc -l` on every touched file, `rustfmt --check` on panes/, profile/ and tests/pane_* (clean), and clippy.toml (complexity 15, too_many_lines 100). Grepped tests/pane_* for an existing pane moved between profiles; none exists.

## T-red (Phase 4, author the RED suite) — 2026-10-09T14:52:00-06:00
- **Decided:** RED is valid; handoff-T-red.md written. Five test files (four new profile/membership files plus a fifth, `profile_persistence_test.rs`, split from the registry file to stay under lint's 600-line warn) and amended `pane_dispatch_test.rs`, `pane_handlers_test.rs`, `pane_support/mod.rs`. AC 34's assertion RED was recorded before `pane_support` was extended: exactly one conformance case fails (`pane-in-other-profile`), so the brief's escalate rule does not fire. Applied A findings 3 (generic `drain`, `head_by`, no copied helper) and 5 (tenth row in the corrupt table: live event name != record name).
- **Assumed:** the D2 file layout and the serde forms of `ProfileChange` are as the brief and the merged `holler-pane` types say; the hand-built corrupt documents are mutations of a file the registry itself wrote, so they stay valid except for the one edited member. `profile/rename` stays `not-implemented` on a corrupt registry (it is routed to `rename.rs`, which never touches the store).
- **Hedged:**
  - The compile-RED of the whole `holler-hub` test tree is transient and inherited from the brief's own design (shared `pane_support` calls `load_with`); A finding 4 flagged it. I did not duplicate helpers to avoid it.
  - I verified compilation and non-vacuous failure with a throwaway stub (reverted; `git status` shows no production file changed). That does not prove each assertion's expected value: the stricter tests (exact summary wording, `what` prefix, file layout) are checked for the first time when F lands code, and any wrong expectation there is a test bug I fix in the GREEN round.
  - Time assertions (AC 6) use real `now_millis()` windows, per the brief; they are only flaky if the system clock steps backwards mid-test.
- **Evidence:** `cargo test -p holler-hub --test pane_membership_test` before the helper extension (3 passed, 3 failed, one conformance case `pane-in-other-profile`); `cargo test -p holler-hub --no-run` after (E0599 `load_with`/trait methods and one E0277 only); stubbed run 0/12, 0/8, 0/6, 0/11 on the four new profile binaries, pane_registry_test 14/14, pane_feed_test 7/7; `cargo clippy -p holler-hub --all-targets -- -D warnings -A clippy::drop_non_drop` clean with the stub; `bash scripts/lint.sh` rc 0; `rustfmt --edition 2021 --check` on every new and touched test file.

## F (Phase 5, implement against the RED) — 2026-10-09T15:05:38-06:00
- **Decided:**
  - Implemented the brief as written:
    - `profile/{entry,store,handlers}.rs` are new; `profile/mod.rs` is filled.
    - D7 is `refuse_profile_move` in `panes::store::Store::cas_put`, after `next_generation` (A finding 6).
    - D8 is `check_membership`.
    - Two `pub(crate)` openings in #639's code: `NoParams`, and `log_fault` re-exported as `crate::panes::log_fault`. `mod store` stays private (A finding 2, a departure from D10's literal wording).
  - D4 is one stamp per write, `max(now, last log at, live updated)`. It is used for `at`, for `updated`, and for a create's `created`.
  - D5 pairs specs by pane name, in order, so the counts stay consistent even with duplicate pane names.
  - The D2 load checks plus A finding 5, with one extra condition: a live record's log must not end with `Deleted`.
  - The name rule applies against the live record only (D6).
  - Every mirrored store item carries a "Twin of `panes::store::...`" doc line (A finding 1).
  - Doc lines fixed per A finding 7 in `panes/mod.rs` and `panes/store.rs`.
  - CHANGELOG entry as the brief gives it, plus the two new `error` events, following #639's entry.
  - All of T's tests are GREEN with no test edited.
- **Assumed:**
  - `Table::from_doc`, the 10-line load-side twin that A finding 1's enumeration omits, is within the accepted mirror. It is the half of `load_table` that builds the table, as in the pane store.
  - The README "Debug output" table stays out of scope. It is outside AC 45's paths, and #639 left its own events out of it too.
- **Hedged:**
  - The A finding 1 follow-up issue does not exist yet, so the "Twin of" lines carry no issue number. That issue should also fix `panes/persist.rs:17-21`, which still says `ProfileEvent` entries.
  - A finding 8's cross-story notes and the operator-review flags (D4, D9, D10, D12) remain for O and the operator.
  - archChanged is reported `true`: there are new module boundaries and a new public `load_with` and `ProfileStore` impl, all as planned and reviewed by A.
- **Evidence:**
  - `cargo build -p holler-hub` clean.
  - Nine hub binaries GREEN: 12, 8, 6, 11, 6, 10, 8, 14 and 7 passed; both conformance suites pass in full, 23 and 19 cases.
  - `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`: rc 0, 1297 passed, 0 failed.
  - `cargo clippy --workspace --all-targets -- -D warnings` clean; `rustfmt --edition 2021 --check` rc 0; `bash scripts/lint.sh` rc 0; `changelog-check` ok; `cargo machete` clean.
  - The AC 41 and AC 28 greps and the AC 45 path check hold.
  - Disabling each of `entry.rs`'s five load checks in turn made `a_corrupt_file_fails_closed_and_is_never_rewritten` fail every time; the file was restored byte-identical.
  - Handoff: `handoff-F.md`. Facts: `evidence.md`.

## T (Phase 7, verify GREEN + Tier 2)
- **Decided:** No test repair needed; suite is GREEN and Tier 2 is clean. Verdict PASS.
- **Assumed:** CI's own `--skip roster_stays_accurate_under_concurrent_body_load` is the right workspace invocation.
- **Hedged:** AC 6 timing windows depend on a non-stepping wall clock (accepted in RED).
- **Evidence:** workspace 1297 passed / 0 failed; five new binaries green over 5 repeats; mutation of `refuse_profile_move` and `check_name` each turned tests RED (production restored, tree clean); clippy, lint, changelog-check, docs_cli_test, wire_selftest, machete all clean. Handoff: `handoff-T-green.md`.

## A (Phase 7, anti-duplication gate) — 2026-10-09T15:22:36-06:00
- **Decided:** PASS on diff 9d61c9f..9026762, with 4 warns and no blocks (see handoff-A-dup.md).
  - F extended the #639 pane registry through its documented seam and copied none of its shared parts (`persist`, `feed`, `run`/`NoParams`, the options, `log_fault`, `next_generation`, `reply_line`).
  - The `profile::store::Store` mirror is the brief's written justification (D10). Its twins differ from `panes::store` only in the key, the entry type and `poll`'s event projection.
  - `Table::from_doc` is ruled inside the accepted mirror (finding 1), although my Phase 3 list omitted it. It is the half of `load_table` that builds the table, and `load_table` was on the list.
- **Assumed:**
  - The testkit fake's near-verbatim twins of `refuse_profile_move`, `check_name` and `log_entry` are by design: the fake is dev-only, and #637 left these rules out of the port, so each implementation keeps its own and one conformance suite checks both.
  - The per-binary local `Rig` follows #639's test convention.
- **Hedged:**
  - The A-dup gate has no channel to open an issue. The generalisation follow-up (Phase 3 finding 1, now finding 2) still does not exist, and a BLOCK would send F to fold the twin past the issue's `panes/**` limit. So it stays a warn, addressed to the run's merging agent or the operator before merge.
  - The test-side copies (`pname` x3, `who` x2, the handler `Rig`) are warns, not blocks: none copies an existing `pane_support` helper, which is the brief's block rule.
- **Evidence:**
  - Read in full: `profile/{mod,store,entry,handlers}.rs`, `panes/{mod,store,persist,feed,handlers}.rs`, `tests/pane_support/mod.rs`, the five new test files and `pane_handlers_test.rs`.
  - Diffs: `pane_dispatch_test.rs`, `pane_handlers_test.rs`, `CHANGELOG.md` and the three `panes/` files.
  - Testkit `profile_store.rs:170-361` and `pane_store.rs` (the fake's `put` and `check_membership`).
  - Normalised diff of `panes/store.rs` against `profile/store.rs`.
  - Grep sweep of `profile/` for re-implemented pipeline, persist or feed pieces and for direct logging (none found).
  - Survey of local helpers across the hub test binaries; `wc -l` on every touched file (max 415 production, 577 test).
