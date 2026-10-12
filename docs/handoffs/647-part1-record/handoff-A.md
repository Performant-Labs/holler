# Handoff-A: Phase 3 - #647 the reconcile engine and `holler pane doctor`  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-647-implementation (worktree `.claude/worktrees/0647-reconcile-doctor`, head `a98258a`, on `3bdd129`)
**Brief reviewed:** `docs/handoffs/647-brief.md` (as amended in `a98258a`, sha256 `0f98bbc7b52e5c78...`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)", lines 924-939 (there is no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS: no blocks, seven warns. The plan puts each piece where ADR-0021 says it goes:

- a pure engine in `holler-pane` (`reconcile.rs`, `findings.rs`, section 5) that reaches the world only through `Ports`;
- a thin verb in its own file that types its arguments in `run` and prints only through `output::emit`;
- closed error codes only, with exit codes from `class_of`;
- scoping through `ProfileScope::resolve`, with no membership check of its own;
- record writes that follow the hub's skip-unchanged rule (`panes/mod.rs:27-28`).

It edits no frozen file. It copies nothing that is on `main`, with one exception: the clock read (W-3).

Most of the warns are seams with the wave-3 briefs being planned in parallel (#643, #644, #663), plus two items for the
standing spec. The most consequential is W-1. #647 is the writer of `last_observed` and the new writer of `harness.health`.
Other verbs gate on those fields, but what doctor writes into them would be written down only in this brief.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn | Decision 6 (`harness.health` written; `at` = "when the current value was first observed"); Decisions 3 and 6 (`shown: None` after `Ok(None)` is a home screen and a mismatch); AC 28 ("No other ADR-0021 line changes"); Decision 1(c)'s sentence; Forward-compat (no #643, #645 or #646 row) | ADRs; cross-story contract | ADR-0021 §1 names each field's writer, but its `harness` row (line 41) names none. Its `last_observed` row (line 45) says only "Written by reconcile". The verbs that read these fields already read them differently. Doctor's writes gate `say --pane` (#646) and switch/reset (#645). | Record the two field meanings in §1's `harness` and `last_observed` rows. Do it in this change (widen Decision 1 and AC 28) or through the amend channel. F also states them in `reconcile.rs`'s module doc. Add Forward-compat rows for #643, #645 and #646, and reword 1(c). Detail below. |
| W-2 | warn | Decision 3's `shown-driven-mismatch` rule; Decision 10 (the rule is private to the engine) | duplication (forward); dependency direction | Three other layers need the same SHOWN-vs-record comparison: #643's view (CLI), #646's gate at `send_prompt` (`holler-hub`) and #648's roster. `reconcile.rs` and `findings.rs` are the only unfrozen files all of them can import. | Expose the comparison as one small `pub` pure function in `reconcile.rs`. The engine uses it too. Name it in Forward-compat for #643, #646 and #648. |
| W-3 | warn | Decision 10: "the CLI passes `SystemTime::now()` in milliseconds" (brief:1067); Files: "the clock read" (brief:905-906); no clock row in the Reuse map | duplication | `holler_proto::clock::now_millis()` (`clock.rs:33-40`) is the workspace's one wall-clock reader. #207 folded more than five copies into it, and `holler-cli` already depends on `holler-proto`. A hand-written `SystemTime::now()` read in `doctor.rs` would be a near-copy, and the Phase 7 gate rejects it. #644's A raised the same point as its W-5. | **F:** pass `holler_proto::clock::now_millis()` as `now_ms`. Add a Reuse-map row. A plain value is the right seam for a single-pass engine. |
| W-4 | warn | "the remedy builder" in `findings.rs` (Files, brief:900-901; Decision 8(a)), absent from Decision 10's public API; C-8; R-4 | duplication (cross-story); layering | ADR-0021 §8 (lines 273-275, 299-301) and §12 (458-460) make "the pane doctor command line for that pane" the reconcile step that other verbs print. Two other briefs are building that command line right now: #663's `profile_scope::reconcile_step` (profile form) and #644, CLI-side (pane form). #647 owns the doctor verb's grammar (`[PANE]`, `--fix`), yet keeps its builder private. | Make the doctor form one `pub fn` in `findings.rs`, for example `doctor_command(pane: Option<&PaneName>, fix: bool) -> String`. Keep every remedy string in one table. On C-8: keep the remedies naming the verbs that own each repair. Never print raw OpenCode, Herdr or tmux commands. |
| W-5 | warn | Decision 10: `FindingKind` derives `Serialize` with `rename_all` and also has a hand-written `code()`; `ObservedHealth { Healthy, Wedged, Down, Unknown }`. Decisions 3 and 6: `observe-failed` also covers a `generation-conflict` on the record write. Forward-compat #650 row | pattern consistency; naming | (a) The finding kinds have two sources of truth, kept equal by a test, where the code tables have one (#145). (b) One observation would be spelled three ways in the same JSON document. (c) `observe-failed` has two meanings from its first day, and kind codes are stable once merged. (d) #650 also reports findings, and the plan does not say whether it reuses `Finding`. | (a) Implement `Serialize` from `code()`. (b) Align `ObservedHealth` with the finding codes, or reuse `Health`. (c) Split the write conflict into its own kind, or define `observe-failed` to cover both. (d) Add a Forward-compat sentence for #650. Detail below. |
| W-6 | warn | Decision 12; AC 25; AC 26; Files (T); the rig; Decision 8(b)'s `{:?}` text rendering | file structure; duplication (test helpers) | (a) `holler-pane`'s pure vocabulary is tested in `crates/holler-pane/tests/*_test.rs`, and AC 25 needs no fake. (b) #643 and #644 plan rigs over the same seven fakes in the same `pane_verbs` binary. (c) `verb_harness::parse::try_parse` already exists. (d) #643's `text_value` uses a different escaping rule for stored text. | (a) Put AC 25 in `crates/holler-pane/tests/findings_test.rs`. (b) Reuse whichever rig is on `main`. (c) Have AC 26 call `try_parse`. (d) Use `text_value` if it has merged. Detail below. |
| W-7 | warn | Decision 3 (`tui-foreign-session`, `stray-session`); AC 3; R-3 | what the ports can observe; contract | (a) `shown_session` does not say which server the TUI is attached to. On the live fleet's shared data directory, incident 3 can go unseen. (b) Under ADR-0021:481 every abandoned session is a stray, and no verb removes one. Every reset, and every pre-cutover session, becomes a stray that every run reports, with no remedy. | Add both to Risks. (a) needs an amend-first change to `HarnessPort` if it must be caught live, not a heuristic in reconcile. (b) is for O: file a follow-up or not. This story's scope does not change. |

### Finding detail

**W-1. The meaning of what reconcile writes is not in the standing spec.**

- ADR-0021 §1 records each field's writer in its Notes column:
  - `host.herdr_api_version`: "recorded by the Herdr adapter (#640)" (line 40);
  - `last_observed`: "Written by reconcile, never inferred" (line 45);
  - `model`: "Recorded by launch, relaunch (#644) and import (#650)" (line 47).
  The `harness` row (line 41) names no writer for `health`.
- The brief decides three things these rows do not say:
  - reconcile writes `harness.health` (`{"unhealthy": "server-wedged"}` or `{"unhealthy": "server-down"}`);
  - `last_observed.at` changes only when a value changes, so it means "first observed", not "last checked";
  - `shown: None` after an `Ok(None)` observation is a home screen (or no TUI), and counts as a mismatch.
- The frozen docs are ambiguous on the last two:
  - `pane.rs:188` says `at` is "When it was observed";
  - `ports.rs:198` says `shown_session` answers "if it can tell";
  - the fake says `None` is "on its home screen or with no TUI" (`harness.rs:135-136`).
- The readers already disagree:
  - #643's in-flight AC 3 renders `{shown: None, ...}` as "unobserved" (its handoff-A, W-1);
  - #646's `say --pane` refuses "when the record says the pane is unhealthy or the shown and driven sessions differ"
    (issue #646; ADR-0021:344);
  - #645 refuses a pane "whose server is unhealthy" (ADR-0021:339);
  - #648's roster shows server health and SHOWN against DRIVEN.

  So a doctor run changes what other verbs do. After merge, the meaning of what it writes would live only in this brief, and
  CLAUDE.md names the ADRs, not briefs, as the standing spec.
- Because `at` moves only with a value, no reader can tell "doctor confirmed this a minute ago" from "doctor last ran three
  days ago". The hub's rule (`panes/mod.rs:27-28`) forces this, and the choice is right; readers still need to know it.
  An edge case shows the same thing: a first observation that equals the defaults (`shown: None`, health not observed)
  writes nothing, so `at` can stay `0` after an observation.
- Decision 1(c)'s sentence says SHOWN is compared with `session_of_record` "which the hub drives under I2". Before #649,
  nothing points DRIVEN at it (ADR-0021:452-453), so as written the sentence claims the invariant already holds.

Suggested text:

- `harness` row: "`health` is also written by reconcile (#647) from what it observed this run."
- `last_observed` row: "`shown: None` with `at > 0` is an observed home screen or no TUI; reconcile writes only on change,
  so `at` is when the current values were first observed, not a freshness stamp."
- 1(c): "...compares SHOWN with `session_of_record`, which I2 makes the session the hub drives, and leaves
  `last_observed.driven` as stored."

**W-4. Remedies, C-8 and R-4.**

- The doctor form is the one remedy that other stories must print too (ADR-0021 §8 step 6 and §12).
- #663's `reconcile_step(&ProfileName)` names `holler pane doctor --profile '<P>'` (663-brief.md:1596-1597).
- #644's in-flight brief prints `holler pane doctor <name>` from the CLI. #644's own A told it to use #663's function
  (644 handoff-A, finding 4).
- `holler-pane` is the one crate that the engines (`tx_*`, in `holler-pane`) and the CLI verbs can both reach. A `pub`
  builder in `findings.rs` is therefore the natural single owner of the pane form, since #647 owns the grammar it spells.
  If #644 or #663 merges first with its own pane-form string, O files a follow-up to fold it onto #647's.
- On C-8, the architecture view is narrow. Naming `relaunch` and `reset` keeps each repair with its one owner, so option (a)
  or the documented window are both consistent. What would be drift is option (b) done by printing OpenCode, Herdr or tmux
  commands: that brings back the second control path the epic exists to remove.
- On R-4: #644's in-flight brief declares `PaneRelaunch { name: String (positional PANE), .. }`, so the planned
  `holler pane relaunch <pane>` is likely to parse once #644 merges.

**W-5. Vocabulary: one source, one spelling, one meaning.**

- (a) `PaneCode` is "the one table that `ALL_CODES`, `PaneError::code` and the wire parse-back all derive from, so the three
  cannot drift (the same single-source rule as `holler_proto::Code`)" (`holler-pane/src/error.rs:41-43`; and
  `holler-proto/src/error.rs:12-16`, #145).
  - With `#[serde(rename_all)]` plus a hand-written `code()`, the finding kinds have two sources.
  - Fix: implement `Serialize for FindingKind` as `serializer.serialize_str(self.code())`, the way `PaneName` (`pane.rs:62-66`)
    and `GridPos` (`grid.rs:129-138`) implement theirs. Then AC 25's equality holds by construction. Keep the test.
- (b) The brief gives no reason for a new enum beside `Health`. If `ObservedHealth` stays (a closed JSON set is a fair
  reason), serialize it as `healthy | server-wedged | server-down | unknown`. Otherwise one run's document says
  `panes[].health: "wedged"`, `findings[].kind: "server-wedged"`, and the record says `{"unhealthy": "server-wedged"}`.
- (c) Kind codes are stable once merged (ADR-0021:328-329). `observe-failed` covers both "a read call failed" and "the record
  write lost a race". Either give the write conflict its own kind (for example `record-conflict`, a 13th entry), or define
  `observe-failed` as "a port call of the pass failed (a read or the record write)" in its doc.
- Also say in `findings.rs`'s module doc that the report types serialize every absent value as `null`. That is the
  envelope's convention, unlike the records' "left out when absent" policy (`lib.rs:38`); the brief makes this choice on
  purpose, so a reader of the crate should see it stated.
- (d) `pane import` reports findings too (ADR-0021:343; issue #650: disagreements, `command-not-argv`, "imported with no
  session of record and a finding"). Say in Forward-compat whether #650 extends `FindingKind` and `Finding` or keeps its
  own report type, so it does not build a parallel one by default.

**W-6. Tests.**

- (a) `crates/holler-pane/tests/error_test.rs:67-87` pins `ALL_CODES` as unique, kebab-case and the exact closed set. AC 25 is
  the same test for `FindingKind::ALL`, and Decision 12's reason (the test kit is not a dev-dependency of `holler-pane`)
  does not apply to it. Move AC 25, and any test of the remedy builder or sanitizer, to
  `crates/holler-pane/tests/findings_test.rs`, and list that file in Files and AC 33. Cargo finds `tests/*.rs`
  automatically, so no manifest edit is needed.
- (b) #643's brief has `pub(crate) struct Rig` in `tests/pane_verbs/list.rs` (643-brief.md:1081-1089). #644's has
  `pub(crate) mod rig` in `tests/pane_verbs/launch.rs`. #643's A (W-3) asks every A-dup gate to reject a second copy of
  whichever rig merges first.
  - **T:** if one is on `main` when T starts, add only the live-world seeding this story needs (`ensure_pane`,
    `ensure_session`, `serve`, `create_session`, `attach_tui`). At Phase 7 I will check this.
  - If neither is on `main`, say in Forward-compat that #645 and #646 reuse doctor's rig.
- (c) `crates/holler-cli/tests/verb_harness/parse.rs:24` (`try_parse`) already wraps `Cli::try_parse_from`. AC 26 should
  call it after dropping the remedy's leading `holler`.
- (d) #643's `text_value` (643-brief.md:1090-1097) quotes a stored string only when it needs quoting, while #647 always uses
  `{:?}`. #643's A (W-2) asks for one helper before the second story merges. **F:** if `pane::list::text_value` is on
  `main`, `doctor.rs` uses it. If not, `{:?}` matches `error::excerpt`, and O records which form is the shared one.

**W-7. What the frozen ports cannot see.**

- (a) `HarnessPort::shown_session` returns a session id only.
  - The live fleet's servers share one data directory, as the fake does by default (`harness.rs:104-108`).
  - So a bare OpenCode TUI attached to another server shows a session that the pane's own server also lists. Doctor then
    reports it as `shown-driven-mismatch` at most, or as nothing when it happens to show `session_of_record`.
  - AC 3 detects the incident only because it gives the foreign server its own directory (`set_data_dir`).
  - If incident 3 must be caught on the live layout, the port has to report which server the TUI is on. That is a
    change to the provisional `HarnessPort` under the epic's amend-first rule, with #642. It is not a heuristic for
    reconcile. Record it as a risk.
- (b) ADR-0021:481 makes "any other session on the pane's server" a stray, and C-5 says no Holler verb removes one.
  - Every `reset` (and every switch away) leaves its old session behind as a stray.
  - On cutover, every session in the shared directory's history becomes one.
  - Each is reported on every run with `remedy: null` and no suppression (Decision 2), so they can drown out the findings
    that matter. R-3 covers the same kind of noise for Herdr panes but not this.
  - O decides on a follow-up: a verb that retires a session, or a known-sessions set. Nothing in this story changes.

### Checked and consistent with existing patterns (no finding)

- **Placement and layering.**
  - The engine and the finding types fill the stubs ADR-0021 §5 assigns to #647 (lines 178-181). They work only through
    `Ports` and add no dependency or async runtime.
  - The verb stays one file with its own `Args` (ruling 2).
  - `lib.rs`, `ports.rs`, `error.rs`, `pane.rs`, `output.rs`, `pane/mod.rs`, the test kit and every manifest stay
    untouched.
  - The two fallback submodules (`reconcile/observe.rs`, `doctor/rig.rs`) follow the test kit's
    `conformance/profile_scope.rs` plus `profile_scope/act.rs` layout.
- **Dependency direction.** `holler-pane` keeps serde, `serde_json` and `holler-proto` only. The report types derive
  `Serialize` there, and the CLI only renders them. Engine tests live in the `pane_verbs` target that already links the
  test kit, as `crates/holler-cli/Cargo.toml:432-434` planned. No fake is copied, and `holler-pane/tests/common`'s
  `MemPaneStore` is not extended.
- **Scope.**
  - `--profile` goes through `ProfileScope::resolve`, and a named pane through `pane_store.get`, so there is no membership
    check of doctor's own.
  - Strays are judged against all records, so another profile's session of record is not a stray.
  - `unregistered-herdr-pane` appears only in whole-fleet runs. All of this matches ADR-0021 §3 (lines 126-129).
- **Errors and exit codes.**
  - Only closed codes, through `ErrorBody::from(&PaneError)`, `emit` and `class_of`.
  - The failure set matches ADR-0021 §9's `pane doctor` row (line 342).
  - Exit 0 with findings follows from that row ("findings are kinds, not errors") and from the envelope's rules 6 and 9:
    data cannot travel with a failure.
  - A bad `PANE` or `--profile` value is `usage`, typed in `run` the way `SpecFlags::validate` types its flags.
- **The record write.**
  - Only on change (`panes/mod.rs:27-28`).
  - A `generation-conflict` is reported, not retried (ADR-0021 §8).
  - `driven` is left as stored, which is the outcome #644's A asked of #644 (its finding 3).
  - The CLI supplies the timestamp, since the hub "stamps nothing" (`panes/mod.rs:9`).
- **I2, I3 and I4.**
  - `--fix` is plan, act, observe, record: `select_session`, then `shown_session` again.
  - It never writes `session_of_record` and never calls a `HostPort` or `HerdrPort` writer.
  - It skips the orchestrator unless the run names that pane, matching the orchestrator rule of ADR-0021:339.
- **Bounded time (I5).** The engine uses `std::thread::scope` over the `Copy` `Ports`. That needs no dependency because every
  port is `Send + Sync` (`ports.rs:8-10, 224-226`). Keeping the fan-out inside `reconcile()` gives every caller the bound,
  including #648 and a later loop.
- **Secrets and the terminal.**
  - No `command`, `env`, `probe`, `cwd` or `model` in the report.
  - Remedies are built only from constant words and the `PaneName` grammar.
  - Untrusted single values are quoted through `error::excerpt`.
  - Op names follow the test kit's `<port>.<method>` (`fault.rs:20`).
- **The ADR-0021 edits for the timer** (Decision 1(a) and (b)): consistent with ruling 1 and with the overlay's rule that a
  decision settling an ADR item edits the ADR in the same change. The comment on #634 is correctly left to the MO.

## Notes for O

Not required for a PASS. These are the decisions that only O or the MO can make:

1. **W-1 and W-2 (before T pins AC 1, 6 and 7).**
   - Decide whether this run records the `harness.health` writer and the `last_observed` meaning in ADR-0021 §1 (amend
     AC 28) or through the amend channel.
   - Pick the epic's single reading of `shown: None` and of the SHOWN comparison. #647's reading matches I2 and #644's
     amended plan; #643's AC 3 is the outlier.
2. **W-4.** Tell #644 and #663 that the pane form of the reconcile step will be `findings.rs`'s builder. If either merges
   first, file a follow-up to fold it.
3. **W-6.** Designate one rig and one terminal-text helper for the `pane_verbs` stories (the same ask #643's A made).
   Amend Files and AC 33 if AC 25 moves to `holler-pane/tests/`.
4. **W-7(b).** Decide whether to file a follow-up for stray accumulation.

## Patterns referenced

- `crates/holler-pane/src/error.rs` (the single-source code table, `excerpt`) and `crates/holler-pane/tests/error_test.rs`
- `docs/adr/ADR-0021.md` §1 (records and their writers), §5 (crate layout), §8 and §12 (the reconcile step), §9 (codes and exits)
- `crates/holler-proto/src/clock.rs` (`now_millis`, #207)
- `crates/holler-hub/src/panes/mod.rs:27-28` (the periodic-writer rule)
- Cross-story evidence (plans, not merged code): `.claude/worktrees/0643-read-verbs/docs/handoffs/643/handoff-A.md`,
  `.claude/worktrees/0644-launch-relaunch/docs/handoffs/644/handoff-A.md`, and the #643, #644 and #663 briefs in their
  worktrees
