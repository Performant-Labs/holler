# Handoff-A-dup: Phase 7 - #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)  (anti-duplication gate)

**Date:** 2026-10-09
**Branch:** issue-643-implementation (worktree `.claude/worktrees/0643-read-verbs`)
**Diff base:** `3bdd129` (the merge base)   **Diff head:** `279a1fb`
**`origin/main` now:** `e612878`, #647 part 1 (PR #701), merged at 18:56:22 MDT, four minutes after this run's T-green
(18:52:09 MDT)
**Reuse map:** `docs/handoffs/643-brief.md`, "Reuse map (extend, do not duplicate)" (lines 986-1001), with the Phase 3
warns in `handoff-A.md`
**Verdict:** BLOCK

## Summary

BLOCK, for one finding, D-1. It is not F's drift: F extended every object the Reuse map named and copied none of them.
It comes from `main`, which moved during this run. #647 merged its own public SHOWN/DRIVEN rule,
`holler_pane::reconcile::shown_differs`. That rule calls itself "the one form of this comparison" and names `pane get`
and the roster as its readers, and ADR-0021:454-455 records it. #643's `SessionSync::of` is a second public rule, over a
different pair of fields. Merged as it stands, `pane doctor` and the three read verbs would disagree about the same
record. The read verbs would never print MISMATCH for a pane that reconcile has observed, because nothing on `main`
writes `last_observed.driven`. #647's A-dup handoff, which is on `main`, tells this gate to reject #643's copy.

F cannot fold this alone. AC 3, Decision 3 and the issue's "SHOWN and DRIVEN come from the record's `last_observed`" pin
the current reading, so the fold needs an MO ruling and a brief amendment first (Notes for F, Notes for O).

D-2 to D-6 are warns. They are smaller overlaps with what #647 merged, or with the test harness, and this branch cannot
fold any of them inside its blast radius.

This cycle's own diff (`04e6af2..279a1fb`) is test-only: two tests in `tests/pane_verbs/get.rs` and one import in
`tests/pane_verbs/list.rs`. It adds no helper and no rework drift. No production file has changed since `06320ad`.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| D-1 | block | `crates/holler-cli/src/pane/list.rs:195-221` (`SessionSync`, `SessionSync::of`) | **A second SHOWN/DRIVEN rule beside the one on `main`, and the two disagree.** `SessionSync::of` compares `last_observed.shown` with `last_observed.driven`, and `None` on either side is `unobserved`. Its doc says "This is the one copy of the rule the read verbs print" (:195-196). On `e612878`, `holler_pane::reconcile::shown_differs` (`crates/holler-pane/src/reconcile.rs:171-181`, `pub`, with `pub mod reconcile` at `lib.rs:52`) compares `session_of_record` with `shown`, and `shown: None` (the home screen) is a mismatch. Its doc: "The one form of this comparison: a pass calls it with what the TUI shows now, and a reader of a record (the roster, `pane get`, the gate at `send_prompt`) with `last_observed.shown`, provided `last_observed.at > 0`" (:176-178). ADR-0021 §11 (:454-455): "Until #649 wires DRIVEN, reconcile (#647) compares SHOWN with `session_of_record`, which I2 makes the session the hub drives, and leaves `last_observed.driven` as stored." Nothing on `main` writes `last_observed.driven`: reconcile's `record` writes only `harness.health`, `last_observed.shown` and `at` (`reconcile/observe.rs:354-386`). **Failure:** a pane whose `session_of_record` is `ses-a` and whose TUI shows `ses-b`. `pane doctor` reports `shown-driven-mismatch` and records `last_observed {shown: "ses-b", driven: None, at > 0}`. `pane list` then prints SHOWN `ses-b`, DRIVEN `-`, SYNC `-` (JSON `"sync": "unobserved"`), and `get` and `watch` say the same. The brief calls these verbs the place where the mismatch "becomes visible" (brief:22-23), yet they never flag it. **Why the brief's exemption no longer holds:** Decision 3 was written when no rule existed, and the brief's Forward-compat row expected the opposite order ("#647 doctor \| the same SHOWN/DRIVEN rule if it reports one \| `SessionSync` reusable", brief:1136). #647's A-dup (`docs/handoffs/647/handoff-A-dup.md:111-115`, on `main`) names `shown_differs` and says "Each later story's A-dup gate should then reject its own copy." The help text and `PaneRow`'s doc also say DRIVEN is "as reconcile last recorded it" (list.rs:45-48, 153-154), which ADR-0021:455 now says reconcile does not do. | The MO rules, the brief is amended, then a fresh run (Notes for O). **(a) Adopt `shown_differs`.** This matches `main` and ADR-0021 as they stand. Bring `origin/main` into the branch. `SessionSync::of` takes the record and calls `shown_differs(session_of_record, last_observed.shown)` when `last_observed.at > 0`. It is `unobserved` otherwise, or when there is no session of record. It stays the one place `list`, `get` and `watch` read SYNC. Amend Decision 3 (including what the DRIVEN column prints until #649), AC 3, the `--help` lines and the CHANGELOG sentence. AC 3 changes because its fixtures set `driven` with `at: 0` and no session of record, and its third case, `shown: None`, becomes a mismatch. The three JSON values and `PaneRow`'s keys do not change. **(b) Keep #643's reading on purpose.** Then the same change amends `shown_differs`'s doc and ADR-0021 §11 to say which rule each reader uses (the read verbs, the roster #648, the gate #646). That edit is outside this blast radius. Either way, the rule ends up written down once. |
| D-2 | warn | `crates/holler-cli/src/pane/list.rs:251-272` (`text_value`) | **Two public rules for printing an untrusted string, one per crate.** On `main`, `holler_pane::findings::quoted` (`findings.rs:328-334`, `pub`) is "the quoting of every untrusted value doctor prints, its text output included". It always quotes with `{:?}` and cuts at 64 characters (`error::excerpt`). `text_value` leaves a plain value bare and never cuts. #647's D-2 set the deadline "before the second of #643 and #647 merges". #647 merged first, so the deadline is now. This is not a near-copy, and `quoted` is no drop-in: folding onto it would quote every plain table cell (AC 1) and cut a long cwd in `get` (AC 7). Once they quote, both escape the same characters, because both use `{:?}`. After merge, `pane doctor` prints a session id as `"ses-a"` and `pane get` prints it as `ses-a`. | The MO picks one rule in the same ruling as D-1. The smallest single rule is one escaper in `holler-pane` with two faces: always quoted and cut, for messages; bare when plain and uncut, for fields. It would be added amend-first. Nothing changes in this branch until then. |
| D-3 | warn | `crates/holler-cli/tests/pane_verbs/list.rs:39-95` (`Rig`), `:143-147` (`ok_envelope`), `:166-197` (`assert_fails`) | **A second fakes rig in the `pane_verbs` binary.** The doctor rig is now on `main` in the same binary. It has `ports` and `ports_with` (`doctor/rig.rs:155-170`) and `json_data` (`:333-343`, the twin of `ok_envelope`). Next to it, `doctor/surface.rs:131-156` has `both_formats` and `assert_refused`, which are close to `assert_fails`. #647's D-1 says the story that merges second "folds its general helpers onto the first one's, and that story's A-dup gate checks it". #643 is second, but the fold cannot happen here. The doctor rig is private (`mod rig;`, `doctor.rs:10`; the `pub(crate)` that D-1 suggested was not applied), so reaching it means editing #647's file. It also does a different job. It builds a live world from `Seed`s, gives each record a session of record, and changes a seeded record only by a store write (`Rig::rewrite`, a `concurrent_put`). #643's cases seed hand-edited records directly, and AC 12-15 count cursors from that seed. | No change here. Follow-up for the MO: one shared `pane_verbs` rig, with a store-only and a live-world constructor and the shared JSON and refusal checks. It goes through the frozen-file amend channel (`pane_verbs/main.rs`, or `pub(crate) mod rig;` in `doctor.rs`). |
| D-4 | warn | `crates/holler-cli/src/pane/list.rs:132-136` (`profile_name`) | **The `--profile` guard has two spellings after merge.** `pane doctor` on `main` types the flag inline with the same expression (`doctor.rs:49-54`). Phase 3's W-10 stands: the repo's home for a guard of a shared flag group is `args.rs`, which is frozen. | Keep `pane::list::profile_name` as the one guard. Through the amend channel, move it onto `ProfileOpt` in `args.rs` and have `doctor` call it. |
| D-5 | warn | `crates/holler-cli/tests/pane_verbs/get.rs:20-29` (`help`), `:188` | **`verb_harness::parse`'s argv builder is rebuilt.** `help` and `get_requires_a_pane_name` build `["holler", ...]` themselves and call `Cli::try_parse_from`. `verb_harness::parse::try_parse` (`tests/verb_harness/parse.rs:16-26`) already does this, it can be reached as `crate::verb_harness::parse::try_parse`, and `doctor/surface.rs:19` uses it. It is three lines. | T: use `try_parse` the next time `get.rs` changes. It is not worth a cycle alone. |
| D-6 | warn (low) | `crates/holler-cli/tests/pane_verbs/watch.rs:118-127` (AC 14's poll) | **Checked because the overlay names `wait_for`; it is not a copy.** It is an inline loop, not a new helper. `support::wait_for` (`tests/support/mod.rs:151`) cannot be reached from here: the frozen `pane_verbs/main.rs` does not include `support`, which `verb_harness/mod.rs:9` keeps apart on purpose. More than a dozen test sites write the same inline bounded poll, the in-process `holler-hub/src/holds/tests.rs:297` among them. This poll waits on an observed value rather than sleeping blind, as `wait_for`'s rule asks, and it also stops when the verb ends. The kit's own test of the same race sleeps blind (`fake_pane_store_test.rs:328-329`). | Follow-up only: move `wait_for`, which needs only std, into `verb_harness`, so in-process tests share one wait. |

### Carried from Phase 3 (not re-rated here)

- W-1 is now D-1, W-2 is now D-2, W-3 is now D-3 and W-10 is now D-4.
- W-4: `PaneChange.pane` is a `PaneRow`, while `get`'s `data.pane` is the record. This is still the last cheap moment to
  rename it. Put it in the same amendment.
- W-5 and W-6 are open: the empty `watch --until-idle` stream against `check_ndjson`, and I5 read per port call.
- W-9: the brief no longer describes the code on four points, and `evidence.md` carries them. Fold them into the same
  amendment, since one is needed anyway.
- W-11 is open: JSON mode writes DEL, C1 and bidi characters raw (`write_envelope`, #660).

### Checked and consistent (no finding)

- **The Reuse map, row by row.** F reused every object it names and copied none:
  - output goes only through `emit`, `emit_stream`, `emit_error` and `ErrorBody::from` (`list.rs:73-76`,
    `get.rs:58-61`, `watch.rs:61-66`), with no printing or exit-code table of the verbs' own;
  - `ProfileOpt` is flattened in all three structs;
  - the only port calls are `PaneStore::{get, list, watch}`, `ProfileStore::get` and `ProfileScope::resolve`, and the
    AC 17 grep prints nothing;
  - names are typed with `PaneName::parse` and `ProfileName::parse`;
  - positions go through `GridPos`'s `Display` and `Serialize`, with no second formatter;
  - JSON uses the record's own serde forms (`Pane`, `Health`, `Hold`, `ProfileSpec`, `Cursor`);
  - the verbs raise only the closed `PaneNotFound` and `PaneNotInProfile`;
  - `observed_at` calls `time_fmt::format_epoch`;
  - the tests use `run_verb_with`, `check_envelope`, `check_ndjson`, `sample_pane`, `sample_profile` and the kit's
    fakes. `ok_envelope` and `ok_stream` build on the kit's checks, not on a copy of `verb_harness::one_envelope`.
- **The other new objects, against `main` at `e612878`.**
  - Every CLI table on `main` renders its own (`roster_cmd.rs:65`, `token_cmd.rs:138`, `attach_cmd.rs:186`), and
    there is no shared table helper. `COLUMNS` and `PaneRow::cells()` are the one definition behind both the `list`
    table and the `watch` line.
  - `hub_cmd.rs:132`'s `printable` is private and lossy (it writes `?` for anything but printable ASCII). It is a
    different policy and cannot be reached from here.
  - `holler-pane` gives `PaneRole`, `HarnessKind`, `Health` and `Hold` no word form or `Display`, so `role_word`,
    `health_word` and `hold_word` copy nothing. `Profile` has no lookup of a spec by pane, so `spec_for` copies
    nothing.
  - Every other place on `main` writes the slug comparison inline, as `Members::names_profile` does
    (`holler-hub/src/panes/store.rs:347`, `holler-hub/src/profile/entry.rs:91`, and the fake store and scope).
  - The pane feed has no change-kind type to reuse for `ChangeKind`. `ProfileChange` is a profile log-entry kind.
  - `PaneRow` summarizes a record. #647's `PaneSummary` is what one pass observed, which is a different contract.
- **The overlay's stack checks.** No prompt path, wire format, golden file, persisted state or hub code is touched.
  The hub-side candidates (`token.rs`, `Lockout`, `Roster`, `log(Severity, ...)` and the `Hub`, `Body`, `mint_token`,
  `join` and `StateDir` harness) are neither touched nor copied. The largest touched file has 545 lines
  (`tests/pane_verbs/list.rs`). The ADR 0003 rows edited are this story's own.
- **Merging with `main`.** `git merge-tree --write-tree origin/main HEAD` conflicts only in `CHANGELOG.md`, where both
  stories add an entry at the same place under `## [Unreleased]`. `cli-surface.txt`, `process/stub.rs` and ADR 0003
  merge cleanly.

## Notes for F

1. **Do not change SYNC this cycle.** AC 3, Decision 3 and the issue text pin the current reading, and only the MO can
   choose between D-1's (a) and (b). A change of your own would fail AC 3's tests and S's audit, and (b) is outside your
   blast radius.
2. **Make no production change.** Say in `handoff-F.md` that D-1 needs the MO's ruling and a brief amendment, and
   return `done: false`. The driver then stops (`unrecognized-verdict:unavailable`). Otherwise it would run F, T-green,
   the diff gate and A-dup again, with the same result, until the three-BLOCK limit.
3. **After the ruling and the amendment, in the fresh run:** for (a), bring `origin/main` (`e612878` or later) into
   the branch first, because `shown_differs` is not on it. Then `SessionSync::of` delegates the comparison to
   `holler_pane::reconcile::shown_differs`, keeps `SessionSync`'s three values and serde names, and stays the only
   place `list`, `get` and `watch` read SYNC. T re-authors AC 3 first, test-first.
4. D-2 to D-6 need nothing from you.

## Notes for O

There is no O on this automated path. These notes are for the operator or the MO when the run stops.

1. **D-1: rule (a) or (b).** (a) matches `main` and ADR-0021 as they stand. (b) needs an amend-first edit to #647's
   `shown_differs` doc and to ADR-0021 §11. Then amend the brief: Decision 3, AC 3, and the help and CHANGELOG lines,
   plus the Files list for (b). Start a fresh run, as after a Phase 3 amendment.
2. **D-2 in the same ruling:** one terminal-text rule for the pane verbs. #647's D-2 deadline is now.
3. **Fold W-9's four points and W-4 into the same amendment.** The brief needs a change anyway.
4. **The branch is behind `main`, whatever the ruling.** Bring `origin/main` in before the PR merges. Only
   `CHANGELOG.md` conflicts.
5. **Follow-ups, not blocking:** D-3 (one `pane_verbs` rig), D-4 (the `ProfileOpt` guard in `args.rs`), D-6
   (`wait_for` in `verb_harness`) and D-5 (T's next change to `get.rs`).

## Patterns referenced

- On `e612878`: `crates/holler-pane/src/reconcile.rs` (module doc :1-43, `shown_differs` :171-181),
  `reconcile/observe.rs:354-386`, `findings.rs:328-334` and `lib.rs:42, 52`.
- On `e612878`: `docs/adr/ADR-0021.md:161` (I2) and `:451-455` (§11), and `docs/handoffs/647/handoff-A-dup.md` (D-1,
  D-2, Notes for O).
- On `e612878`: `crates/holler-cli/src/pane/doctor.rs` and `crates/holler-cli/tests/pane_verbs/doctor/{rig,surface}.rs`.
- `crates/holler-cli/tests/verb_harness/{mod,parse}.rs`, `crates/holler-cli/tests/support/mod.rs:140-170`, and
  `crates/holler-cli/src/{output,roster_cmd,token_cmd,attach_cmd,hub_cmd,time_fmt}.rs`.
