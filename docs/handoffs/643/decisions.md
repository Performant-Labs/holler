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
