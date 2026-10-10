# Decisions — #643 the read verbs (`holler pane list`, `get` and `watch`, with SHOWN and DRIVEN)

## A (Phase 3, up-front plan review) — 2026-10-09T17:19:10-06:00
- **Decided:** PASS on docs/handoffs/643-brief.md at 2e7f221 (sha256 62fce781...), with 0 blocks and 8 warns (see
  handoff-A.md).
  - The plan's objects and layers are right. The verbs read only through the frozen ports and print only through `output.rs`.
    They raise only closed codes and type names in `run` as `SpecFlags::validate` does. They reuse the record's serde forms
    and `GridPos`, and edit no frozen file. The shared view code has to live in `list.rs`, because `pane/mod.rs` is frozen.
  - W-1 (the most consequential): the SHOWN/DRIVEN rule. `SessionSync` in `holler-cli` cannot serve #646's gate at
    `send_prompt` (`holler-hub`) or #647's engine (`holler-pane`). #647, and #644 as amended after its own A, read the check
    as SHOWN vs `session_of_record` and keep `last_observed.driven` empty until #649. Under those plans #643's SYNC would be
    `-` for every real pane while `pane doctor` reports mismatches. The MO picks one reading.
  - W-2 and W-3: the parallel briefs #647 and #662 define their own terminal-escaping helpers, and #644, #647 and #662 their
    own fakes-backed test rigs. The MO designates one of each.
  - W-4: `watch`'s `data.pane` is a `PaneRow`, while `get`'s `data.pane` and the port's `PaneEvent.pane` are the full record.
    One key has two shapes, and it becomes permanent public JSON.
  - W-5: an empty `watch --until-idle` stream fails `check_ndjson` (`EmptyStream`).
  - W-6: I5 is read per port call for `watch`, with no ADR line.
  - W-7: T writes the `Args` fields (production code) before RED.
  - W-8: bidi characters pass `text_value` unquoted.
- **Assumed:** The parallel briefs in `.claude/worktrees/{0644-launch-relaunch,0647-reconcile-doctor,0662-profile-verbs}` are the
  current plans of those stories. #644's `driven: None` amendment was uncommitted in its working tree when read, after its A
  BLOCK at d2636ba. None of these has passed its own A, so their readings may still change. I used them as evidence of what
  is being planned, not as merged code. The issue (#643), the epic (#633), ADR-0021, ADR 0003 and the merged code are the
  authority.
- **Hedged:**
  - W-1 is a warn, not a block. The documented contract supports both readings: #643's issue says DRIVEN comes from
    `last_observed`, while I2 makes `session_of_record` the session the hub drives. The frozen port (`None` = "if it can
    tell") and the merged fake (`None` = home screen or no TUI) disagree on what `shown: None` means. The role rule is: no
    dominant pattern, no block. Because a warn does not stop the run, T will pin AC 3 as written. If the MO picks the other
    reading, AC 3, Decision 3 and the `--help` text change: one match arm and its tests, with no change to the JSON value set.
  - W-4 is a warn because precedent is mixed: `hub token list --json` puts summaries under `tokens`.
  - W-7 is a process point more than an architecture one. #644 and #647 plan the same thing.
- **Evidence:**
  - Read in full: the brief; `holler-cli/src/{output,roster_cmd,time_fmt,lib}.rs`, `pane/{mod,args,profile_scope}.rs`, the
    `pane/{list,get,watch}.rs` stubs; `holler-pane/src/{pane,profile}.rs`, `ports.rs:150-235`, `reply.rs`, `error.rs:686-695`;
    `holler-pane-testkit/src/envelope.rs`, `harness.rs:125-150, 340-356`, `lib.rs`; `holler-cli/tests/verb_harness/mod.rs`,
    `tests/pane_verbs/{main,list,get,launch}.rs`, `tests/pane_verbs/process/{main,flags,stub}.rs`, `usage.rs:100-238`,
    `docs_rows.rs:1-120`, `tests/cli_surface_test.rs:1-80`; `docs/adr/ADR-0021.md` (all) and ADR-0003.md:30-110.
  - Also read: `hub_cmd.rs:95-140`, `token_cmd.rs:100-163`, `holler-proto/src/log.rs:440-520`,
    `holler-hub/src/holds.rs:210-219`, `lockout.rs:265-295`, `panes/store.rs:330-360`, `panes/feed.rs:1-60`; the overlays
    `docs/agent-overlays/{tester,architecture-reviewer}.md`; the playbook's `tester.md:1-70`, `pipeline-conventions.md` §1
    and the workflow script's commit and arch-gate code; the 642 A handoff and journal (format precedent); the outside
    reviewer's 643-brief-result-r1.md (PASS).
  - Issues read with `gh issue view`: 643, 633, 646, 647, 648, 660, 662, and the states of 644, 645, 649, 652, 653, 663,
    670, 676 and 681.
  - Parallel briefs: 644-brief.md (git diff e7064e9 against its working tree, lines 40-58, 118-122, 1721-1722, 1806), 647-brief.md
    (lines 911, 940-1010, 1016-1019, 1105-1120, 1146), 662-brief.md (lines 1510-1530, 1630-1645, 1750-1762).
  - Greps: SHOWN/DRIVEN logic across crates, control-character escaping helpers, `slug()` comparisons, `Ports {` builders,
    `#643` in docs, and the brief for personal infrastructure names (none).

## T (Phase 4, author tests / RED) — 2026-10-09T17:27:57-06:00
- **Decided:** RED is valid. 28 new in-process tests fail on assertions about the missing behaviour. The only two that
  pass are `testkit_links` (the kept link marker) and `get_requires_a_pane_name` (it pins the `get PANE` surface T
  landed). T landed the surface first, as the brief's Test plan says: the `Args` fields with one-line docs and the stub
  `run` untouched (A's W-7), the ADR 0003 rows, the fixture block and the `STUBS` deletions. The surface tests pass
  (3 + 3 + 34).
  - AC 3 is pinned as written (`last_observed.shown` vs `.driven`; `None` on either side is `-`/`unobserved`), per A's
    W-1 note.
  - The no-write check of Decision 9 is folded into the AC 17 test rather than added as its own test.
- **Assumed:** The fake feed's first cursor is 1 (`feed.rs:87`, `head + 1` from `Cursor(0)`), so seeding two panes
  gives cursors 1 and 2, as AC 12 states.
- **Hedged:**
  - The `watch` refusal cases (AC 6, AC 10) add `--until-idle` to the AC's argv. A wrong implementation then fails
    instead of hanging, because the fake's idle wait is zero, and the refusal is pinned the same way.
  - AC 18's `get` line count is compared with the same pane holding clean strings, not a hard-coded 27.
  - Decision 7's "a pane leaving P (or deleted while in P) prints that one event" has no AC and no test. Exercising it
    needs a write after `resolve`, like AC 14's thread. Left for S and A to judge.
- **Evidence:** `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::` gives 2 passed, 28 failed. The
  whole target gives 62 passed, 28 failed, with no other module regressed. The failing assertions are quoted in
  handoff-T-red.md. Also: `rustfmt --check --edition 2021` on the 7 touched `.rs` files exits 0;
  `cargo clippy -p holler-cli --all-targets -- -D warnings` exits 0; `bash scripts/lint.sh` exits 0.

## F (Phase 5, implement) — 2026-10-09T17:50:31-06:00
- **Decided:** GREEN. The three `run`s are implemented and the help and the CHANGELOG entry are written. T's 30 cases
  pass, and so do the whole `pane_verbs` (90), `pane_cli_process` (34), `cli_surface_test` and `docs_cli_test`. T's
  `Args` fields are unchanged.
  - The shared view code is in `list.rs`, `pub`, one copy each: `PaneRow`, `SessionSync`, `text_value`,
    `observed_at`, plus `json_text`, `profile_name`, `optional_text`, `health_word`, `hold_word`, `COLUMNS` and
    `NO_VALUE`. A `watch` line is built from `COLUMNS` and `PaneRow::cells()`, so it cannot drift from the table.
  - SYNC follows Decision 3 as AC 3 pins it (`shown` vs `driven`, either `None` is `unobserved`). A's W-1 stays with
    the MO, and `SessionSync::of` is the one place to change.
  - `text_value` adopts A's W-8. It quotes and escapes whatever Rust's `{:?}` would write as a `\u{..}` escape
    (bidi, zero-width, combining, line separators), as well as Decision 11's set and the literal `-`.
  - The JSON-rendered text fields go through `json_text`: `serde_json`, plus `\u` escapes for DEL, C1 and format
    characters, which `serde_json` leaves raw. They stay valid JSON, and every AC string is unchanged.
  - `get`'s `profile` is the record's, not the `--profile` argument's. With `--profile`, a scope answer without the
    named pane is `pane-not-in-profile`.
- **Assumed:** `serde_json::to_string` cannot fail for `Argv`, `Vec<String>` or `ProfileSpec`: there is no
  non-string map key and no `Serialize` that errors. On that basis `json_text`'s `unwrap_or_default()` is
  unreachable. A production `PaneStore::watch` blocks in `next()` up to its bound before it answers idle (the port's
  documented rule), so `watch` without `--until-idle` does not spin. Only the test fake's idle wait is zero.
- **Hedged:**
  - Deviations 1 to 4 of handoff-F.md are additive hardening or wider error cases, not shape changes. A or S may still
    prefer the letter of Decisions 2, 5 and 11.
  - JSON mode (`output.rs`, #660) writes DEL, C1 and format characters raw. That is outside this blast radius, and it
    is raised for the MO in handoff-F.md.
  - Decision 7's leave-the-profile event is implemented but untested, as T noted.
- **Evidence:**
  - Commands, all exit 0: `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::` (30 passed); the four
    `holler-cli` targets above; `cargo clippy --workspace --all-targets -- -D warnings`; `bash scripts/lint.sh`;
    `bash scripts/changelog-check.sh`; `cargo machete`; `rustfmt --check --edition 2021` on the 7 files.
  - The AC 17, 21 and 22 greps print nothing.
  - The workspace suite was run with `HOLLER_STATE_DIR` set to an empty dir and `--no-fail-fast`: 1409 passed,
    0 failed. Unisolated, `logging_test`'s 4 `roster` cases fail on this host, because the host's live hub answers.
    The diff does not touch them.
  - A scratch program outside the repo printed the real text and JSON over the fakes. It also checked rustc 1.98's
    `{:?}` on bidi, C1, DEL, NBSP, combining and CJK input.
  - Facts in unchanged code are in `evidence.md`.

## T (Phase 7, verify tests / GREEN) — 2026-10-09T18:00:22-06:00
- **Decided:** GREEN, no blocking issue. All 30 authored tests pass on F's commit, and F flagged no wrong test. T
  mutated F's code eleven ways, ran the suite each time, and restored each file from git. Three of those changes
  survived, and the behaviour each one broke is documented. So T added three tests, with no production edit:
  `list_named_pane_lists_only_that_pane` (Decision 1), `text_output_escapes_c1_and_bidi_characters` (F's deviations
  1 and 2, the security claim in `--help` and the CHANGELOG), and `watch_profile_prints_a_pane_leaving_the_profile_once`
  (Decision 7, closing T's RED hedge without a thread: a `--since 1` replay shows the pane joining first). 33 pass.
- **Assumed:** The fake seeds in order and each change takes head + 1 (`pane_store.rs:90-99`, `feed.rs:87`). So the
  leave test's cursors are 1 (c4), 2 (c1), 3 (c2), 4 (c1 leaves), 5 (c2 deleted), 6 (c1 outside). Recorded in
  evidence.md.
- **Hedged:**
  - The `list` sort mutation (M8) survives, because every fake already returns name order. It is left advisory rather
    than tested with a store double outside the kit.
  - The `--profile --since` replay edge (a current member's pre-join change prints once) follows the rule as written.
    It is flagged for S.
- **Evidence:** `cargo test -p holler-cli --test pane_verbs` gives 93 passed. The workspace suite (isolated
  `HOLLER_STATE_DIR`, `--no-fail-fast`, CI's skip) gives 1409 passed, 0 failed, 5 ignored, exit 0. Clippy (workspace,
  all targets, `-D warnings`), `lint.sh`, `changelog-check.sh`, `cargo machete` and `rustfmt --check --edition 2021`
  on the 7 files all exit 0. `docs_cli_test`, `cli_surface_test`, `pane_cli_process` and `wire_selftest` pass. The
  `watch` filter passes 3 of 3 reruns. The AC 17/21/22 greps print nothing. The mutation table is in
  handoff-T-green.md.

## A (Phase 3, up-front plan review, round 2) — 2026-10-09T18:14:28-06:00
- **Decided:** PASS, with 0 blocks and 9 open warns (see handoff-A.md, which replaces round 1's; round 1 is at fafd138).
  The pass covers the unchanged brief (sha256 62fce781...) and the architecture of F's code at 50bcc83. It ran because
  the outside diff gate BLOCKed round 1 (4 BLOCKs) while F had reported `archChanged`.
  - F's code keeps the plan's architecture. It touches `holler-cli` only, in the three verb files, reads through the
    frozen ports, prints through `output.rs` and edits no frozen file. `get` and `watch` depend on `list`, one way. The
    seven extra `pub` helpers are in the file and layer Decision 2 chose. No re-plan is needed.
  - The gate's B-1 to B-3 are not defects, and no production change is needed for them. B-1's `what` form is
    byte-identical to `FakeProfileScope::resolve`'s and to #663's planned scope. B-2's quoted `-` is what keeps a stored
    `-` apart from the empty value, and no profile or pane name can be `-`. B-3's predicate catches every character the
    gate lists, and `é` and CJK text pass unchanged (rustc 1.98.1). The gate's NV-1, NV-5, NV-6, W-1 and W-3 are
    unreachable or false.
  - New warns. W-9: the brief no longer describes the code on four points (Decisions 2, 6 and 11 and one Risks bullet),
    and the gate reads the brief and `evidence.md`, so F should record the facts in `evidence.md`. W-10:
    `profile_name` is a shared-flag guard kept in a verb file (`args.rs` is frozen). W-11: JSON mode writes C1 and bidi
    characters raw (#660). W-1 to W-6 stay open and W-7 and W-8 are closed.
- **Assumed:** `origin/main` is still 3bdd129 (`git ls-remote`; nothing merged since round 1, and no #644, #647 or #662
  PR open). #647's code (9500955) and #644's brief (7195993) are unmerged, read at about 18:05 MDT as evidence of
  plans. With `archChanged` set, the driver's rework classifier is skipped, so this handoff is the only carrier of the
  gate's findings to T-red and F.
- **Hedged:**
  - W-9 is a warn, not a block. The four deviations go the way round 1's W-8 asked, handoff-F.md records them, and a
    BLOCK would stop the run for a brief edit that `evidence.md` can carry.
  - W-1 is still a warn. ADR-0021 supports both readings (I2 :161, :45, :344), and every verb is `Unwired` until #649,
    which is therefore the deadline.
- **Evidence:**
  - Read in full: `pane/{list,get,watch}.rs` at 50bcc83, handoff-F.md, the round-1 handoff-A.md, decisions.md, the gate's
    `643-diff-result-r1.md`, `pane/args.rs` and `prompt_target.rs`.
  - Also read: `output.rs:296-330`, `roster_cmd.rs:55-110`, `verb_harness/mod.rs:76-100`, `tests/pane_verbs/list.rs`
    (the `Rig` and its helpers), testkit `feed.rs:244-300` and `profile_scope.rs:122-123`, hub `panes/store.rs:296-330`,
    `profile.rs:40-92, 150-200`, `vocab.rs:202-216`, and the ADR-0021 lines on SHOWN, DRIVEN and `session_of_record`.
  - #647's `reconcile/observe.rs:195-391`, `findings.rs:322-354` and the `doctor/rig.rs` API; #644's brief (lines 121,
    1577, 1631-1632, 2077-2079).
  - The driver's phase routing and its diff-gate prompt builder (the gate passes `evidence.md` when it is not empty).
    Issues #643 and #633: no MO ruling on any warn since round 1.
  - A scratch program outside the repo checked the predicate on 25 characters with rustc 1.98.1.

## T (Phase 4, author tests / RED, round 2) — 2026-10-09T18:22:00-06:00
- **Decided:** PASS: the round-2 tests are valid. With no new behaviour this cycle (A's note 4) there is no RED. Each new
  test passes on F's code and fails when the behaviour it pins is removed (mutations Ma, Mb and Mc in handoff-T-red.md).
  - Two new tests in `tests/pane_verbs/get.rs`. `text_output_escapes_each_hidden_class_and_keeps_plain_unicode` covers
    the gate's B-3, NV-6 and W-1: one character per class is escaped, and `é`, `ï` and CJK text are unchanged.
    `a_stored_dash_prints_apart_from_the_empty_value` covers B-2 and W-3: a stored `-` prints as `"-"`, an absent value
    as `-`.
  - `read_verbs_call_no_adapter_or_probe` now also asserts no profile-store write (the gate's W-5).
  - No change for W-4: the exactly-once invariant holds on both interleavings (feed.rs:263-269). No test for B-1: only a
    scope that answers success without the pane reaches `get`'s own construction, and the fake never does. NIT-6 is
    wrong: `Value`'s `Index` gives `Null` for a missing key, so the `get().is_some()` check is the one that pins
    "always present".
- **Assumed:** rustc 1.98.1's `escape_debug` and `{:?}` table, probed in a scratch program outside the repo, is the one
  CI builds with. If a later toolchain changes which characters `{:?}` escapes, the class test fails loudly. It does
  not drift silently.
- **Hedged:**
  - The W-5 extension can only be a guard: no current code path writes, so no mutation turns it red. Its read half keeps it
    from being vacuous.
  - The plain-Unicode pin fixes the current reading ("`é` is plain"). If the MO later picks the gate's
    ASCII-only predicate, this test is the one to change, on purpose.
- **Evidence:** `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::` gives 35 passed (95 for the whole
  target). Mutations Ma, Mb and Mc each turn the new test red, and production was restored with `git checkout`
  (`git status` lists only the two test files). `rustfmt --check`, `cargo clippy -p holler-cli --all-targets -- -D warnings`
  and `bash scripts/lint.sh` all exit 0. The test files are 370, 545 and 311 lines, ASCII-only. Three evidence.md entries
  were added, with excerpts copied from source.

## F (Phase 5, implement, round 2) — 2026-10-09T18:44:55-06:00
- **Decided:** done, with no production change (A's note 3). T-red's round-2 tests pass on round 1's code: 35 of 35,
  and 95 for the whole target. `git diff 06320ad..HEAD -- crates CHANGELOG.md` lists only T's test files. The cycle's
  work is the evidence the round-2 gate reads. `evidence.md` now opens with A's four W-9 deviations and the round-1
  findings, each settled from source:
  - B-1: a profile name inside a sentence-shaped `what` is `{:?}`-quoted in all five merged constructions, the fake
    scope's identical `pane-not-in-profile` among them.
  - B-2 and W-3: no pane or profile name can be `-`.
  - B-3, NV-6 and W-1: rustc 1.98.1's `escape_debug_ext` names its classes explicitly, and the predicate is that list
    plus `is_control`.
  - NV-1, W-2 and NV-5: those paths cannot be reached.
  - `archChanged` is false.
- **Assumed:** The gate keeps only the first 12,000 bytes of the appendix (`dual-review.sh`'s
  `DUAL_REVIEW_EVIDENCE_MAX_BYTES` default, not set in `.env`), and it shows the handoff as `(no handoff file)`, as in
  round 1's prompt. I took rust-lang/rust's sources at tag `1.98.1` to be the sources of this host's `rustc 1.98.1
  (48a229cea)`. A probe on the installed compiler matches them on 39 characters.
- **Hedged:**
  - The appendix had reached 12,744 bytes, so a section appended at the end would have been cut before the gate saw
    it. To make it fit (11,696 bytes now), the round-2 section goes first. F's own entries that duplicate the brief's
    verbatim quotes are now pointers. So are T-green's three such entries (`feed.rs:22-26`, `pane_store.rs:54`,
    `envelope.rs:243-249`), with T's wording kept. That edits T's section. No fact is dropped, the excerpts stay
    readable in the brief, and every other T entry is byte-identical.
  - Only 304 bytes of headroom are left for T-green (flagged in handoff-F.md, "Known issues").
  - Round 1's deviation 4 (`get --profile` with a wrong-name scope answer) has no evidence entry, to save bytes. It
    is the B-1 entry's code path, and the round-1 gate did not question it.
- **Evidence:**
  - Commands, all exit 0: the read-verb filter (35 passed); `pane_verbs` (95), `pane_cli_process` (34),
    `cli_surface_test` (3) and `docs_cli_test` (3); `cargo clippy --workspace --all-targets -- -D warnings`;
    `bash scripts/lint.sh`; `bash scripts/changelog-check.sh`; `cargo machete`; `rustfmt --check --edition 2021` on
    the 7 files. The AC 17, 21 and 22 greps print nothing.
  - The isolated workspace suite gave 1413 passed, 1 failed, 5 ignored. The failure was
    `body_run_test::fresh_hello_and_presence_on_every_reconnect`, in a file this branch does not touch. That target
    then passed 10 of 10 on three reruns, so it is a flake.
  - Read: `dual-review.sh` (the evidence cap, ref resolution, prompt assembly, the 64,000-token ceiling) and round 1's
    gate prompt and usage. This round's estimate is about 51k tokens, under the ceiling.
  - Source for the excerpts: `profile_scope.rs:100-135`, the hub's `panes/store.rs:293-351` and
    `profile/store.rs:395-400`, the fake `pane_store.rs:222-240`, `profile.rs:28-100, 150-240`, `argv.rs:18-92`,
    `feed.rs:236-286`, `vocab.rs:185-223`, and serde_json 1.0.151's `src/ser.rs:2135-2165`. Library files fetched into
    the scratchpad: rust-lang/rust 1.98.1's `char/methods.rs:478-505, 545-546, 2428-2445` and `fmt/mod.rs:2916-2956`.

## T (Phase 7, verify GREEN, round 2) — 2026-10-09T18:52:09-06:00
- **Decided:** GREEN, no blocking issue. The read-verb filter is 35 of 35 and `pane_verbs` 95 of 95. No production file
  has changed since 06320ad, F flagged no wrong test, and T changed no test. T added nothing to `evidence.md`: the
  round-2 tests rely only on the facts T-red already entered, and the file stays at 11,696 bytes, under the gate's cap.
- **Assumed:** Round 1's mutation table (M1 to M11) and T-red round 2's Ma and Mb still hold, because they ran on the
  same production code. Only Mc was re-run.
- **Hedged:** F's one workspace failure (`body_run_test::fresh_hello_and_presence_on_every_reconnect`) did not recur in
  T's run. T treats it as a flake outside this branch and filed nothing from a single sighting.
- **Evidence:** isolated workspace run, 125 result lines, 1414 passed, 0 failed, 5 ignored, exit 0. `pane_cli_process`
  34, `cli_surface_test` 3, `docs_cli_test` 3, `wire_selftest` 3. Clippy (`-D warnings`), `lint.sh`,
  `changelog-check.sh`, `cargo machete` and `rustfmt --check` exit 0. The AC 17, 21 and 22 greps and the manifest diff
  are empty. `watch` ran 3 times, 11 passed each. Mutation Mc fails `a_stored_dash_prints_apart_from_the_empty_value` at
  get.rs:362, and `git status --short` was empty after the restore.

## A (Phase 7, anti-duplication) — 2026-10-09T19:08:52-06:00
- **Decided:** BLOCK, with 1 block (D-1) and 5 warns (D-2 to D-6); see handoff-A-dup.md. F extended every object the
  Reuse map named and copied none of them. The block comes from `main`. `origin/main` moved to `e612878` (#647 part 1,
  merged at 18:56:22 MDT) during this run. That commit put `holler_pane::reconcile::shown_differs` on `main`: it is
  `pub`, calls itself "the one form of this comparison" and names `pane get` and the roster as its readers. ADR-0021
  §11 (:454-455) records it.
  - `SessionSync::of` (`list.rs:195-221`) is a second public rule over other fields (`last_observed.shown` against
    `.driven`). Nothing on `main` writes `.driven`, so after merge the read verbs would print SYNC `-` for every
    observed pane while `pane doctor` reports `shown-driven-mismatch`.
  - #647's A-dup (on `main`, :111-115) tells each later story's A-dup gate to reject its own copy.
  - The fold needs an MO ruling and a brief amendment, because AC 3, Decision 3 and the issue text pin the current
    reading. Notes for F tell F to make no production change and return `done: false`, so the run stops instead of
    looping to the three-BLOCK limit.
  - D-2 to D-6: `text_value` against `findings::quoted` (#647's D-2 deadline is now; no drop-in); the second fakes rig
    against the private doctor rig (#647's D-1 makes #643 the folder, but the doctor rig cannot be reached); `profile_name`
    against doctor's inline guard; `help()` against `verb_harness::parse::try_parse`; and AC 14's inline poll against
    `support::wait_for`, which cannot be reached here and is not copied.
- **Assumed:** The A-dup gate judges the diff against `main` as the PR will merge into it, not only against the merge
  base. The run's own agent merges the PR after CI and the gates, and CI cannot see this conflict, because AC 3's
  fixtures set `driven` directly. The shared ref `origin/main` was updated by another session's fetch, and this
  worktree's HEAD (`279a1fb`) is unchanged.
- **Hedged:**
  - D-1 is a block, not a warn. The role's "no dominant pattern" rule held it at warn in both Phase 3 rounds (W-1), but
    the ambiguity it relied on is gone: `main` now has exactly one rule, recorded in an ADR, which names this story's
    verb. The brief's exemption ("justified in writing") was written before that rule existed, and its Forward-compat
    row expected #647 to reuse `SessionSync`.
  - The issue text supports #643's reading, so D-1 offers both (a) adopt `shown_differs` and (b) keep #643's reading
    with an ADR and doc edit. This review does not pick between them; it only says that (a) matches `main` as it stands.
  - D-2 stays a warn. The two policies differ for a reason (bare plain cells, no cut in `get`), and the fix crosses
    crates.
- **Evidence:**
  - Read in full: the brief, `handoff-A.md`, `handoff-F.md`, `decisions.md`, the three verb files and the three test
    files at `279a1fb`, `verb_harness/{mod,parse}.rs` and `pane_verbs/main.rs`.
  - Read on `e612878`: `reconcile.rs` (module doc and :112-181), `reconcile/observe.rs` (fn list and :348-391),
    `findings.rs:320-351`, `pane/doctor.rs`, `doctor.rs:1-40`, the `doctor/rig.rs` API (:121-200, :326-398),
    `doctor/surface.rs`'s helper list, the ADR-0021 and ADR 0003 diffs, and #647's `handoff-A-dup.md`, plus the parts
    of `handoff-S.md` that mention the hold and #643.
  - Greps on `origin/main` for escaping helpers, table renderers, time formatters, enum words, spec lookups, slug
    comparisons, change kinds, `Ports {` builders, polling loops, `DisplayHelp` and writes of `driven`.
  - `git merge-tree --write-tree origin/main HEAD` reports a conflict in `CHANGELOG.md` only.
  - `gh issue view 643` (body, no comments) and `gh issue view 633` (latest comments): no MO ruling on W-1 or W-2.
  - The driver's A-dup routing (BLOCK goes to F, phase 6, with no extra note) and the stop on F's `done: false`
    (`coding-pipeline.workflow.mjs:1186-1194`, :4775-4800, logic `nextPhase` case 6).

## F (Phase 5, implement, round 3: after A-dup's BLOCK) — 2026-10-09T19:17:11-06:00
- **Decided:** `done: false` and `archChanged: false`, with no production change. This stops the run so the MO can
  rule on D-1 and amend the brief, as A-dup's Notes for F 1 and 2 ask. F cannot fold D-1 inside the brief:
  - every fold changes SYNC, which AC 3 and its three tests pin;
  - (a) needs `origin/main` merged into the branch, which is a commit, and F makes none;
  - (b) edits `holler-pane` and ADR-0021, outside the blast radius;
  - `done: true` would only re-run T-green, the paid diff gate and A-dup to the same BLOCK.
  The stop reads `gate-unavailable:6` (`coding-pipeline.workflow.mjs:4798`, `:1107-1108`), not
  `unrecognized-verdict:unavailable` as A-dup's note 2 says. F recommends (a), with DRIVEN still printing
  `last_observed.driven` until #649 ("a1"). The details are in handoff-F.md, "For the MO".
- **Assumed:** The MO's fresh run starts after the brief is amended, and it takes in `origin/main` first, whatever the
  ruling. Only `CHANGELOG.md` conflicts. No MO ruling exists yet: #643 has no comments, and #633's latest comments say
  nothing on SHOWN/DRIVEN.
- **Hedged:**
  - The lean to (a1) over (a2) is a judgment, not a finding. Both are honest. (a1) keeps the issue's wording and commits
    #649 to nothing. (a2) makes a flagged row show both sessions.
  - The false-MISMATCH caveat under (a) was traced in the source, not run. When `shown_session` fails on a first
    observation, `at` is stamped while `shown` stays `None`. It belongs to #647's rule.
- **Evidence:**
  - Read on `origin/main` (`e612878`): `reconcile.rs:29-36, 153-161, 171-181`, `reconcile/observe.rs:78-84, 96-108,
    186-195, 230-234, 350-386`, the ADR-0021 diff from `3bdd129`, #647's `handoff-A-dup.md:95-130`, and
    `findings.rs:318-352`.
  - `git grep` for writes of `driven` on `origin/main`: only the test kit's fixture sets it, to `None`.
  - Read on this branch: `list.rs` (whole), `get.rs:1-95`, the `observed` fixture and AC 3's test in `pane_verbs/list.rs`,
    `CHANGELOG.md`'s entry, the brief (whole), and handoffs A-dup, F (round 2) and T-green (round 2).
  - The driver: `runImplementPhase`, the F branch of the phase loop, `commitPhaseWork` (`git add -A`) and `nextPhase`.
  - Commands: `cargo test -p holler-cli --test pane_verbs` gave 95 passed. `cargo clippy -p holler-cli --all-targets --
    -D warnings` and `bash scripts/lint.sh` exited 0. `git diff --stat 279a1fb..HEAD -- crates CHANGELOG.md` was empty.
    `git merge-tree --write-tree origin/main HEAD` conflicted in `CHANGELOG.md` only.

## A (Phase 3, up-front plan review, amendment 1) — 2026-10-09T19:41:26-06:00
- **Decided:** PASS on docs/handoffs/643-brief.md at d525032 (amendment 1, sha256 53058e77...), with 0 blocks and 5 new
  warns (W-12 to W-16; see handoff-A.md, which replaces round 2's, kept at 04e6af2).
  - Amendment 1 resolves D-1 the way `main` and ADR-0021 stand. `SessionSync` stays the CLI's three-state view, the
    comparison is a call to `shown_differs`, and DRIVEN is printed and never compared. It touches no frozen file, no
    `holler-pane` file and no ADR, and none needs an edit.
  - W-12 (act on it this cycle): `watch.rs`'s `Members::names_profile` restates `holler_pane::profile_diff::is_member`.
    #662 part 1 made that function public (ce12cdb, in this branch through 05e7337), and its doc calls it the
    definition of membership. The Reuse map does not name it. F folds it inside `watch.rs`.
  - W-13 (act on it this cycle): Decision 3's guard `at == 0` should be `at <= 0`, which is `shown_differs`'s "provided
    `at > 0`". Then it agrees with `observed_at`.
  - W-14: the record-reader form (the `at` guard, a session of record, then `shown_differs`) will have one copy per
    reader, and #646c's gate in `holler-hub` cannot use `holler-cli`'s. Follow-up: the roster uses `SessionSync::of`,
    and the form moves into `holler-pane` before #646c.
  - W-15 (low): `PaneRow` cannot explain its own SYNC, because it has no `session_of_record` and `driven` is null until
    #649. A key can be added later without breaking anything.
  - W-16: #662 added a third rule for printing stored strings (`FieldValue`'s `Display`), and a second rendering of
    `ProbeResult`. It is a non-fold for the same reasons as `findings::quoted`. F records it in `evidence.md`, and D-2
    widens after merge.
  - D-1, W-1, W-4 and W-9 are closed. D-2 to D-6, W-5, W-6 and W-11 stay as the brief lists them.
- **Assumed:**
  - F reads this handoff before it codes, as the driver's run context tells every phase to (`buildRolePhaseTaskPrompt`),
    so W-12 and W-13 reach F without a brief edit. Round 1's W-8 reached F the same way.
  - Folding W-12 is not a drive-by edit: it changes F's own file and calls a `pub` function without editing it.
  - The worktrees of #646 (876f87e) and #663 (1d6a5ab) are plans, not merged code. #663's A BLOCKed at 71f9ae2.
- **Hedged:**
  - W-12 is a warn, not a block. The copy agrees with `is_member`. The Reuse map does not name it, so a Phase 7 block
    rule would not fire on the map. And a Phase 3 BLOCK would stop this automated run, for a brief edit, over a
    three-line fold that F can make in its own file with no AC change. If F does not fold it, A-dup should rate it a
    warn, not a block.
  - W-13 asks F to depart from the letter of Decision 3 for a case no writer produces. It is worth it only because the
    change costs one character and makes the reader agree with the doc it cites.
  - W-15 restates a trade-off the MO already made in ruling (a1). It is recorded for #648 and #649, not reopened.
- **Evidence:**
  - Read in full: the brief (1,689 lines); this directory's handoff-A (round 2), handoff-A-dup, handoff-F (round 3),
    decisions.md and evidence.md; the outside brief reviews 643-brief-result-r1.md (BLOCK) and -r2.md (PASS).
  - Read in full, at d525032: `pane/{list,get,watch}.rs`, `holler-pane/src/reconcile.rs` and `reconcile/observe.rs`.
  - Also read: `profile_diff.rs:150-300`, `profile/show.rs:100-198`, `profile_snapshot.rs:30-60`, `fixture.rs:41-75`,
    the helper list of `tests/pane_verbs/list.rs`, the builders of `doctor/rig.rs`, `pane.rs:183-190`, `lib.rs`, and
    the ADR-0021 lines on SHOWN, DRIVEN and `session_of_record`.
  - From the playbook: the feature-implementor role, the driver's `buildRolePhaseTaskPrompt` and its Phase 3 stop path,
    and `pipeline-conventions.md` §1.
  - The parallel plans: #646's brief (its gate is deferred, :32) and #663's `profile_scope.rs` (`belongs`).
  - Greps, on origin/main at 0ad2d8a and in the branch: writes of `driven`, readers of `last_observed` and of
    `shown_differs`, other SHOWN/DRIVEN rules, slug comparisons, `is_member`'s callers, spec lookups, and word forms on
    `Health`, `Hold` and `ProbeResult`. Also the brief, for #662's helpers (none) and for personal names (none).
  - Commands: `git fetch`. `git merge-tree --write-tree origin/main HEAD` exited 0.
    `git diff --name-only origin/main...HEAD` listed the brief's Files only. `gh issue view` on 646, 648, 649, 662 and
    663, and `gh issue list` found no follow-up filed.

## T (Phase 4, author tests / RED, amendment 1) — 2026-10-09T19:47:47-06:00

- **Decided:**
  - AC 3 is re-authored over one fixture: `observed()` was extended in place to take `record` and `at` (A's note 1). The
    expected SYNC per pane is one `SYNC_WANT` table in `list.rs`, so `list`, `get` and `watch` cannot drift apart.
  - The list test also asserts each row's PANE cell, so a sort change cannot pair a row with the wrong expectation.
  - W-13's `at: -1` case is a separate `get` test (`a_negative_observed_at_is_never_observed`), not a seventh
    `sync_rig` row, so AC 3 keeps the six rows the brief pins. It is a guard against an `at == 0` guard, not a RED.
  - AC 19 gains the four needles the brief names, and nothing more.
  - D-5 (`verb_harness::parse::try_parse`) was not taken. It is optional, and `get.rs`'s `help` helper is not part of
    this rework.
- **Assumed:**
  - `sample_pane` has no session of record and `at: 0` (`fixture.rs:63-70`, as A cited), so AC 1 and AC 2 still expect
    `unobserved` under the new rule. Confirmed by running them against the temporary rule (they pass).
  - The watch feed from cursor 0 yields puts in seed order, as the round-1 test already relied on.
- **Hedged:**
  - A temporary edit to production code, used to show the new tests can pass, was restored with `git checkout`, and
    `git status` confirms that only the test files changed. F still writes the real change.
- **Evidence:**
  - RED: `cargo test -p holler-cli --test pane_verbs -- list:: get:: watch::` gave 31 passed and 5 failed (the three
    AC 3 tests, the two AC 19 help tests), each on an assertion, matching the brief's "RED for amendment 1" (c1, c2, c3,
    c6).
  - Target rule applied temporarily: with `at <= 0`, only the two help tests fail. With `at == 0`, the W-13 test fails
    too (`get.rs:187`).
  - The full `pane_verbs` binary gave 122 passed and the same 5 failed. `rustfmt --check`, `cargo clippy -p holler-cli
    --all-targets -D warnings` and `scripts/lint.sh` all exit 0. Test files are 587, 389 and 320 lines.
