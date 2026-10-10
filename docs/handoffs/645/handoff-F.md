# Handoff-F: Phase 5 (Workflow script Phase 6), rework round 6 - #645a `pane switch` and `pane reset` (T-green round 5's BLOCK: merge `origin/main`)

**Date:** 2026-10-10 (01:07 MDT)
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`). This round starts at `7435610`, T-green
round 5. The merge of `origin/main` is staged, not committed (`MERGE_HEAD` is `bd5e825`): the Workflow script's phase commit
records it with both parents, as it did in rounds 2 and 4. Round 5's handoff is in git at `a1f01aa`, round 4's at `29fa0ff`,
round 3's at `62aae72`, round 2's at `37f2101` and round 1's at `fa59fe7`. Their content that still holds is kept below.
**Issue:** #645 (part 1 of 2, 645a)
**Rework of:** `docs/handoffs/645/handoff-T-green.md` (round 5), blocking issue 1: `origin/main` moved to `bd5e825` (#642
part 2, merged 00:50 MDT) and the branch no longer merged, with a conflict in `docs/adr/ADR-0021.md` ("Deferred to named
stories"), so AC 24 and AC 25 failed on the tree that will land.

**Confirmation table** (the role doc asks for one before implementing; a Workflow run has no human to wait for, so it is
recorded here):

| Field | Value |
|-------|-------|
| GitHub issue | #645 (645a) |
| Working branch | `issue-645-implementation` |
| Build plan phase | epic #633, wave 3 (the brief's split, part 1 of 2) |
| Input documents read | the brief (all of it); handoffs T-green (round 5) and F (round 5); `decisions.md`; `evidence.md`; `git diff HEAD...origin/main` over `holler-pane`, ADR-0021, the CHANGELOG and `Cargo.lock`; the OpenCode adapter's `attach.rs` |
| Acceptance criteria count | 26 (brief ACs 1-26) |
| Handoff document path | `docs/handoffs/645/handoff-F.md` |

## What was done (round 6)

- **Merged `origin/main` (`bd5e825`) with `git merge --no-commit --no-ff`.** Since the branch's last merge (`abdcbb6`),
  `origin/main` gained #715 (`cec1f82`, a handoff file of #660's) and #642 part 2 (`bd5e825`, the OpenCode adapter's TUI
  side). `CHANGELOG.md` merged cleanly, and so did every file outside ADR-0021. Nothing in #642 part 2 touches a file of
  this story apart from ADR-0021: in `holler-pane` it changes doc comments only (`lib.rs:32-33`, `ports.rs:13-15` and
  `:174-175`).
- **`docs/adr/ADR-0021.md`: the one conflict, resolved by keeping both sides,** as T-green's blocking issue says. In
  "Deferred to named stories" the list now reads, in order:
  - this branch's mismatch-code bullet ("decided, `unavailable` (exit 1), section 8: #645 for switch and reset; #644 to
    follow for launch and relaunch"), in place of `origin/main`'s still-open form ("... : #644 and #645");
  - this branch's **PROPOSED (#645, pending the operator)** bullet;
  - `origin/main`'s `HerdrPort` bullet, which drops "`HarnessPort` in its final form: #635, then #642" now that #642 has
    made it final, in place of the old combined bullet;
  - `origin/main`'s two new bullets: applying `Pane.opencode_agent` (#642's last part, after #700) and stopping the
    harness server (#695).
  The resolved file is staged (`git add docs/adr/ADR-0021.md`), and no conflict marker is left in any file.
- **`docs/handoffs/645/evidence.md`: four citations moved, and one entry now cites built code.**
  - #642 part 2's new section 2 paragraph ("`HarnessPort` as built (#642)") moved ADR-0021 down by 20 lines: `:294-299`
    is now `:314-319` and `:331-337` is now `:351-357`. The round-2 note on where the #645 paragraph sits now says
    `:365-390`.
  - `ports.rs` gained one doc line in `HarnessPort`'s doc, so `select_session` moved from `:198-199` to `:199-200`.
  - `hermetic_test.rs` gained a line above its `ID` constant, so `:25-26` is now `:26-27`.
  - Round 5's entry on why a failed `select_session` is `acted` cited the spike's plan for the real adapter. That adapter
    is now on `origin/main`, so the entry cites its built `select_session` (`attach.rs:7-8`, `:74-95`, `:202-204`): the
    switch request is sent, then the title is watched, so the watch's `timeout`, or a TUI found gone during it, comes after
    the request that moves the screen, while the same `no TUI in pane P` or a `timeout` can also come before anything is
    sent. That is the W-1 reasoning, now shown in source rather than in a plan.
  - The header records round 6.
- **`docs/handoffs/645/decisions.md`:** the F round-6 entry.
- **No production code and no test changed.** `tx_switch.rs`, `pane/switch.rs` and `pane/reset.rs` are as round 5 left them.

## Design decisions (round 6)

1. **Both sides kept, in the order of the two lists.** This branch's two bullets stay where the mismatch bullet was, and
   `origin/main`'s three follow in `origin/main`'s order, so `git diff origin/main -- docs/adr/ADR-0021.md` shows the
   "Deferred" hunk as exactly the mismatch bullet rewritten and the PROPOSED bullet added (AC 24's fourth hunk). Taking
   either side whole was rejected: `origin/main`'s side drops Decision 17(c) and (d), and this branch's side would bring
   back "`HarnessPort` in its final form: #635, then #642", which is now false, and drop two of #642's bullets.
2. **No edit to #642's new ADR-0021 text.** AC 24 allows no fifth hunk, and nothing in "`HarnessPort` as built (#642)"
   contradicts this story. Its item 6 says the adapter's pane lookup answers `pane-not-found` for a `PaneId` it does not
   know. For switch and reset that would be a `select_session` failure, so under the brief's A2 row it is `acted` and its
   message carries the reconcile step, which is harmless. Its item 3 (a `None` from `shown_session` means "cannot tell") is
   the case the brief's R-2 already covers: a mismatch, `unavailable`, which fails safe.
3. **Evidence entry 25 cites the built adapter instead of the plan.** The fact is unchanged. The source is now code on the
   PR's base, which the diff gate can attach beside the excerpt, so a reviewer no longer has to trust a plan.

## What was done (rounds 1-5, still standing)

- **Round 5** (the outside diff gate's r4 BLOCK, B-1). `switch` calls `select_session` itself at the `acted` boundary and
  maps that call's error with `acted`; the observation is a private `observe`, also mapped with `acted`. Doc comments on
  `acted`, `read` and `reset::run`. Three `evidence.md` entries for NV-1, NV-3, B-1 and W-1.
- **Round 4** (S's REWORK). Merged `origin/main` (`abdcbb6`: #713 for #660, and #714 for #640 part 3). The ADR-0021
  conflict was resolved by keeping both sides. Five `evidence.md` citations were corrected for #714's lines in `error.rs`.
- **Round 3** (the outside diff gate's r2 B-1). `check_health`'s remedy fallback became an explicit `match`. The
  `SESSION_ID_MAX` doc says why the two limits agree. Two evidence entries settled r2's NV-1.
- **Round 2** (A-dup round 1's BLOCK). Merged `origin/main` (`d9eabbb`). Four sentences of the "Switch and reset as built
  (#645)" paragraph say when the reconcile step is printed and why it names the pane with `--fix`.
- **Round 1.** `tx_switch.rs` is filled as the brief's P0-R table. Both verbs share one after-clap path
  (`switch::execute`). The ADR-0021 edits follow Decision 17, and there is a CHANGELOG entry.

## Reuse / extend-vs-new

Unchanged; round 6 adds no code. A-dup round 2's table still holds:
- The objects extended are the `tx_switch.rs` stub and the two verb stubs, built on doctor's `--fix` repair: select,
  observe, record. As in reconcile's `select` (`reconcile/observe.rs:318-321`), the port's `select_session` is called
  directly.
- Reused as they are: `findings::doctor_command(Some(pane), true)`, `findings::quoted`, `reconcile::shown_differs`,
  `FindingKind::ServerDown.remedy(..)` (by `match`), `ports.scope.resolve`, `output::{emit, emit_error, ErrorBody,
  ErrorCode, VerbCtx}`, `ProfileOpt` and `holler_proto::clock::now_millis`.
- The one justified copy is `screen_text` (O1).

## Architecture notes for A

- **Round 6:** none. A merge of `origin/main` and a docs conflict resolved by keeping both sides. No module, public item,
  dependency, layer or frozen file of this story changed. `archChanged: false`.
- **Rounds 1-5:** the engine is in `holler-pane`, pure over `Ports`, with no I/O, no async and no new dependency.
  `tx_switch` uses `findings` and `reconcile::shown_differs`, `pane/switch.rs` uses `holler_pane::tx_switch`, and
  `pane/reset.rs` uses `pane/switch.rs` (`execute`, `Verb`). No frozen file and no #647 file is touched, apart from
  `doctor.rs:10` (Decision 16, T's).

## Deviations from spec / wireframe

Round 6: none new.

Rounds 1-5, still standing and accepted by S:
- P1 uses `find` by name, where the brief says `into_iter().next()`. `read`'s doc says why.
- The "Deferred" bullet takes A's round-2 form, "#644 to follow". #644's paragraph is still not on `origin/main` at
  `bd5e825`.
- `reset.rs` reuses `switch::{execute, Verb}`, and `execute` prints through `emit_outcome`. `run`'s doc says so.
- The #645 paragraph has round 2's four sentences.

No wireframe applies (no UI surface).

## Tier 1 self-check (incl. tests now GREEN), round 6

Every build ran with `CARGO_BUILD_JOBS=4`, on the merged tree (HEAD `7435610` plus the staged merge of `bd5e825`).
`origin/main` was fetched at 00:58 MDT, and it is `bd5e825`.

```
$ git merge --no-commit --no-ff origin/main     -> CONFLICT (content) in docs/adr/ADR-0021.md only; resolved and staged
$ git ls-files -u | wc -l                       -> 0 (no unmerged path); git diff --cached --check -> clean
$ rustfmt --check --edition 2021 tx_switch.rs pane/switch.rs pane/reset.rs \
    tests/pane_verbs/switch.rs tests/pane_verbs/reset.rs                       -> exit 0
$ cargo clippy --workspace --all-targets -- -D warnings                        -> exit 0
$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test \
    --test wire_selftest
   cli_surface_test 3 passed; docs_cli_test 3; pane_cli_process 35; pane_verbs 162; wire_selftest 3; 0 failed
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::               -> 22 passed, 0 failed
$ cargo test -p holler-pane                                                    -> 98 passed, 0 failed
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's form)
   -> 01:02-01:06 MDT: exit 0; 140 result lines; 1700 passed, 0 failed, 25 ignored
      (round 5's 1664 + #642 part 2's attach_test 23 and tui_test 13; its real_opencode_test adds 9 ignored)
$ bash scripts/lint.sh                    -> exit 0 (size warnings only; tests/pane_verbs/switch.rs 649)
$ bash scripts/changelog-check.sh         -> changelog-check: ok
$ cargo machete                           -> no unused dependencies
$ gitleaks protect --staged / unstaged    -> no leaks found (both)
$ git diff origin/main -- '*Cargo.toml' Cargo.lock | wc -l                     -> 0 (AC 25; was 26 before the merge)
$ git diff origin/main -- docs/adr/ADR-0021.md | grep -c '^@@'                  -> 4 (AC 24; was 5 before the merge)
$ grep -n 'Switch and reset as built (#645)' docs/adr/ADR-0021.md               -> 365 (between ### 8. :305 and ### 9. :392)
$ grep -c '#645' docs/adr/ADR-0021.md                                           -> 9 (origin/main: 6)
$ the `unavailable` decision in section 8 (:305-391)                            -> stated once (:372)
$ python3 -I check_evidence.py <worktree> docs/handoffs/645/evidence.md         -> entries: 27, mismatches: 0
```

Every command above ran on the finished tree as it is handed off. No production file changed in this round, so no
self-check mutation was run; T's round-5 mutation table covers the code unchanged.

## Evidence appendix

`docs/handoffs/645/evidence.md` holds 27 facts in unchanged code or text: 15 from F round 1, 2 from T, 4 from F round 2,
1 from T round 2, 2 from F round 3 and 3 from F round 5 (one of them re-sourced in round 6). Every excerpt matches its
cited lines on the merged tree, and none of the cited files differs from `origin/main`.

## Tests that look wrong (for T)

None. No test changed, and every test passes on the merged tree, T's round-5 test
`switch_observation_failure_names_the_reconcile_step` included.

## Known issues

- **ADR-0021 `:485`** (was `:465` before this merge) says "#645's and #646's, planned". Once 645a lands that is out of date
  for #645's three codes. It is for O, at the next ADR-0021 edit, because AC 24 allows no fifth hunk.
- **"its home screen" for a `None` that means "cannot tell".** With the built adapter, `shown_session` answers `None` for
  the home screen and also whenever it cannot tell (ADR-0021 section 2, "`HarnessPort` as built (#642)", item 3). O1's
  message then says "shows its home screen", as reconcile's private `screen_text` does, word for word as the brief requires.
  The run still fails safe (`unavailable`, nothing recorded). The wording belongs to the F-4 fold, which A-dup round 2's
  warn 3 widened to `screen_text`, so doctor and switch would change together. For O.
- **More ADR-0021 conflicts may come** from #644 or #642's last part (R-1). Whichever lands second resolves the conflict by
  anchor text.
- **Carried from earlier rounds, unchanged:**
  - The next diff-gate round may raise B-2 or B-3 again; each is answered in the docs next to the code.
  - A-dup's warns and follow-ups F-1 to F-4 are for O to file. Once the `screen_text` fold is filed, the comment in
    `tx_switch.rs` should name its issue.
  - A timed-out `create_session` in reset may leave a session that the message cannot name. That is a 645b question.
  - The accepted risks remain: R-3, R-5 and F-1.

## Files changed

Production (this story, all rounds; none changed in round 6):
- `crates/holler-pane/src/tx_switch.rs`
- `crates/holler-cli/src/pane/switch.rs`
- `crates/holler-cli/src/pane/reset.rs`

Docs (this story):
- `docs/adr/ADR-0021.md` (round 6: the merge conflict in "Deferred to named stories", both sides kept)
- `CHANGELOG.md` (unchanged in round 6; it merged cleanly)

Merged from `origin/main` (`bd5e825`, staged with the merge, not this story's change): #642 part 2's files and #715's.

Pipeline records:
- `docs/handoffs/645/handoff-F.md` (this file)
- `docs/handoffs/645/evidence.md` (round 6: four moved citations, entry 25's source, the header)
- `docs/handoffs/645/decisions.md` (round 6: the F entry)
