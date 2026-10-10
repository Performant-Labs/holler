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
