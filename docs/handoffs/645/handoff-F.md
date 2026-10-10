# Handoff-F: Phase 5 (Workflow script Phase 6), rework round 5 - #645a `pane switch` and `pane reset` (the outside diff gate's r4 BLOCK)

**Date:** 2026-10-10 (00:50 MDT)
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`). This round starts at `48f2395`, T-green
round 4. The changes are not committed: the Workflow script's phase commit records them. Round 4's handoff is in git at
`29fa0ff`, round 3's at `62aae72`, round 2's at `37f2101` and round 1's at `fa59fe7`. Their content that still holds is kept
below.
**Issue:** #645 (part 1 of 2, 645a)
**Rework of:** `docs/handoffs/645-diff-result-r4.md` (gitignored), the outside diff gate's BLOCK at 00:32 MDT on `48f2395`.
The gate reports "2 blocking finding(s)". B-1 stands. The reviewer also wrote B-2 to B-4 under its BLOCK heading, then
withdrew B-2, called B-3 not a block, and found no contradiction in B-4. So this round answers all four, as well as W-1,
NIT-1, NV-1 and NV-3.

**Confirmation table** (the role doc asks for one before implementing; a Workflow run has no human to wait for, so it is
recorded here):

| Field | Value |
|-------|-------|
| GitHub issue | #645 (645a) |
| Working branch | `issue-645-implementation` |
| Build plan phase | epic #633, wave 3 (the brief's split, part 1 of 2) |
| Input documents read | the brief (all of it); `645-diff-result-r1.md` to `-r4.md`; handoffs A, T-red, F (round 4), T-green (round 4), A-dup (round 2) and S; `decisions.md`; `evidence.md` |
| Acceptance criteria count | 26 (brief ACs 1-26) |
| Handoff document path | `docs/handoffs/645/handoff-F.md` |

## What was done (round 5)

- **`crates/holler-pane/src/tx_switch.rs`: B-1, the `acted` flag is now tied to the `select_session` call.**
  - `switch` calls `ports.harness.select_session(&record.herdr.pane_id, &target)` itself, right where the `acted` mapping
    starts, and maps that call's error with `acted`.
  - The observation is a function of its own, `observe` (`shown_session`, then `shown_differs`). It runs only after
    `select_session` has returned `Ok`, and its errors are mapped with `acted` too.
  - `select_and_observe`, which merged the act with the observation, is gone.
  - A comment at the call says which failures are `acted` and why the call's own failure is one of them (W-1). The
    `acted` field doc says the same thing in one sentence, after the brief's sentence, which is kept.
  - Behaviour is unchanged. The calls are the same and in the same order (AC 2's sequences hold), and so are the errors
    and the messages.
- **`tx_switch.rs` `read`: B-3 and NIT-1, a doc comment only.** It says why P1 searches the scope's answer by the pane's
  name, as `pane get` does (`get.rs:101-105`), instead of taking the first pane. The two are the same pane while
  `resolve` keeps its contract. A scope that broke the contract cannot make the run act on another pane.
- **`crates/holler-cli/src/pane/reset.rs`: B-2 (withdrawn), a doc comment only.** `run`'s doc says that it takes
  switch's own path, `execute`, which prints through `emit_outcome`, so none of switch's code is copied.
- **`docs/handoffs/645/evidence.md`: three entries for NV-1, NV-3, B-1 and W-1.**
  - A failed `select_session` may already have moved the screen: `ports.rs:198-199`, and the spike's `select_session`
    plan at `opencode-pane-spike.md:237-239`.
  - OpenCode's id shape: spike `:149-151`, the OpenCode adapter's `hermetic_test.rs:25-26` and the fake's
    `harness.rs:122-124`.
  - The rig's call log covers every port, keystrokes included: `doctor/rig.rs:249-272` and `:159-170`, the test kit's
    `fault.rs:85-97`, and `herdr.rs:259-260` and `:270-271`.
  - The header records that there is no round-5 merge.
- **`docs/handoffs/645/decisions.md`:** two entries. One records the r4 gate round, since the gate writes none (S's
  advisory 4). The other is this F round.
- **No merge.** Since `abdcbb6`, `origin/main` has moved only by #715 (`cec1f82`), which touches
  `docs/handoffs/0660-output/decisions.md` alone. It conflicts with nothing here and changes no AC-24 or AC-25 input.

## Design decisions (round 5)

1. **B-1 is fixed in code, not argued.** Before this round `acted` was already right on every path, because every
   failure of `select_and_observe` came after `select_session` was called. But only the order of lines inside a merged
   helper made it right. The reviewer's case is a later step added inside that helper before the select: the flag would
   then say something false, and nothing would catch it. Now the boundary is in `switch`, at the call itself:
   - every failure above it converts through `From<PaneError>` (`acted: false`);
   - every failure from it on goes through the `acted` closure.

   Considered and rejected:
   - A type-state token that only the select can produce. It is too heavy for one function's boundary.
   - Separate error variants for a failed select and a failed observation. They would change the public `SwitchFailure`,
     whose fields the brief fixes, and both cases need the same message anyway.
2. **The select's own failure stays `acted` (W-1).** The brief's A2 row says so (`e`, `acted: true`). It is also the safe
   direction:
   - The engine cannot tell how far a failed call got. The port answers only `Ok(())` or an error, and the planned real
     adapter fails after it has sent the request that moves the screen (spike `:237-239`).
   - A reconcile step printed when the TUI did not move is harmless: doctor `--fix` finds nothing to repair.
   - A step left out when the TUI did move would leave the operator without the repair.
3. **P1 keeps `find` (B-3, NIT-1), now explained where the code is.** It is S's accepted deviation (rounds 1-4), and it
   is how the merged `pane get` reads the same answer. Switching to `next()` would satisfy the brief's wording but drop
   the guard against acting on a pane the run was not asked about.
4. **Doc lines, not code, for B-2 and B-4.** Both were withdrawn or found not to be contradictions. The reviewer reads
   only the diff, so an explanation in the code's own docs is where the next round will see it.

## What was done (rounds 1-4, still standing)

- **Round 4** (S's REWORK). Merged `origin/main` (`abdcbb6`: #713 for #660, and #714 for #640 part 3). The ADR-0021
  conflict was resolved by keeping both sides: the section 9 rows, and the "Deferred" bullets. Five `evidence.md`
  citations or excerpts were corrected for #714's three doc-comment lines in `error.rs`.
- **Round 3** (the outside diff gate's r2 B-1). `check_health`'s remedy fallback became an explicit `match`. The
  `SESSION_ID_MAX` doc says why the two limits agree. Two evidence entries settled r2's NV-1.
- **Round 2** (A-dup round 1's BLOCK). Merged `origin/main` (`d9eabbb`). Four sentences of the "Switch and reset as built
  (#645)" paragraph say when the reconcile step is printed and why it names the pane with `--fix`.
- **Round 1.** `tx_switch.rs` is filled as the brief's P0-R table. Both verbs share one after-clap path
  (`switch::execute`). The ADR-0021 edits follow Decision 17, and there is a CHANGELOG entry.

## Reuse / extend-vs-new

Unchanged, and round 5 extends nothing new and copies nothing. A-dup round 2's table still holds:
- The objects extended are the `tx_switch.rs` stub and the two verb stubs, built on doctor's `--fix` repair: select,
  observe, record. As in reconcile's `select` (`reconcile/observe.rs:318-321`), the port's `select_session` is now
  called directly, not through a helper.
- Reused as they are: `findings::doctor_command(Some(pane), true)`, `findings::quoted`, `reconcile::shown_differs`,
  `FindingKind::ServerDown.remedy(..)` (by `match`), `ports.scope.resolve`, `output::{emit, emit_error, ErrorBody,
  ErrorCode, VerbCtx}`, `ProfileOpt` and `holler_proto::clock::now_millis`.
- The one justified copy is `screen_text` (O1).

## Architecture notes for A

- **Round 5:** one private function of `tx_switch.rs` (`select_and_observe`) became a direct port call in `switch` plus a
  private `observe`. There is no new module, public item, dependency or layer, and no frozen file or #647 file changed.
  No public item changed in this round, apart from one added sentence in the `acted` field doc. `archChanged: false`.
- **Rounds 1-4:** the engine is in `holler-pane`, pure over `Ports`, with no I/O, no async and no new dependency.
  `tx_switch` uses `findings` and `reconcile::shown_differs`, `pane/switch.rs` uses `holler_pane::tx_switch`, and
  `pane/reset.rs` uses `pane/switch.rs` (`execute`, `Verb`). No frozen file and no #647 file is touched, apart from
  `doctor.rs:10` (Decision 16, T's).

## Deviations from spec / wireframe

Round 5: none new. The brief's A2 and O1 rows hold as written. A2 is `select_session`, whose error is `acted: true` with
`created` as A1's. O1 is `shown_session` and `shown_differs`, whose failures are `acted: true` too.

Rounds 1-4, still standing and accepted by S:
- P1 uses `find` by name, where the brief says `into_iter().next()`. `read`'s doc now says why.
- The "Deferred" bullet takes A's round-2 form, "#644 to follow". #644's paragraph is still not on `origin/main` at
  `cec1f82`.
- `reset.rs` reuses `switch::{execute, Verb}`, and `execute` prints through `emit_outcome`. `run`'s doc now says so.
- The #645 paragraph has round 2's four sentences.

No wireframe applies (no UI surface).

## Tier 1 self-check (incl. tests now GREEN), round 5

Every build ran with `CARGO_BUILD_JOBS=4`. `origin/main` was fetched at 00:34 MDT, and it is `cec1f82`.

```
$ rustfmt --check --edition 2021 tx_switch.rs pane/switch.rs pane/reset.rs \
    tests/pane_verbs/switch.rs tests/pane_verbs/reset.rs                       -> exit 0
$ cargo clippy --workspace --all-targets -- -D warnings                        -> exit 0 (0 warnings)
$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test
   cli_surface_test 3 passed; docs_cli_test 3 passed; pane_cli_process 35 passed; pane_verbs 161 passed; 0 failed
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::               -> 21 passed, 0 failed
$ cargo test -p holler-pane                                                    -> 98 passed, 0 failed
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's form)
   -> on the finished tree, 00:47-00:51 MDT: exit 0; 137 result lines; 1663 passed, 0 failed, 16 ignored
      (round 4's count; an earlier run at 00:39-00:43, before the last doc-only edit, gave the same)
$ bash scripts/lint.sh                    -> exit 0 (size warnings only: error.rs 713, tests/pane_verbs/switch.rs 613)
$ bash scripts/changelog-check.sh         -> changelog-check: ok
$ gitleaks protect --redact --no-banner   -> no leaks found
$ git diff origin/main -- '*Cargo.toml' Cargo.lock | wc -l                     -> 0
$ git diff origin/main -- docs/adr/ADR-0021.md | grep -c '^@@'                  -> 4 (AC 24, unchanged)
$ grep 'unwrap(|expect(|panic!|unreachable!|todo!|[0]' on the three production files -> none
$ python3 -I check_evidence.py <worktree> docs/handoffs/645/evidence.md      -> entries: 27, mismatches: 0
```

**Self-check mutations** (F's own sanity check, each restored with `cmp` against a scratch copy; T owns the mutation
table):
- Map the select's error with `?` (so `acted: false`) instead of `acted`: `switch_select_failure_names_the_reconcile_step`
  (AC 8) and `reset_failure_after_create_names_the_unrecorded_session` (AC 18) fail.
- Do the same for `observe`'s error: `switch_mismatch_after_select_records_nothing` (AC 7) and
  `reset_mismatch_records_nothing` (AC 19) fail.

Every command above ran on the finished production tree, the three production files as they are handed off.

## Evidence appendix

`docs/handoffs/645/evidence.md` holds 27 facts in unchanged code or text. 15 are from F round 1, 2 from T, 4 from F round
2, 1 from T round 2, 2 from F round 3 and 3 from F round 5. Every excerpt matches its cited lines on this tree.

## Tests that look wrong (for T)

None. No test changed, and every test passes.

A coverage note for T, not a wrong test: no case has `shown_session` itself fail (an `Err`, not a mismatch) after a good
select. The r4 gate's NV-2 points this out. A case with `fail_next(HarnessOp::ShownSession, Timeout { .. })` would pin
`acted` on that path: exit 1 `timeout`, the message ending with the reconcile step, the record unchanged, and for reset
the created id named. This round's code maps that path with `acted`, as AC 7's mismatch path is mapped. Adding the case is
T's call.

## Known issues

- **The next diff-gate round may raise B-2 or B-3 again.** The reviewer saw both in two rounds (r3 NV-1, r4 B-2 and B-3).
  Each is now answered in the docs next to the code, where the reviewer looks.
- **Carried from round 4, unchanged:**
  - ADR-0021 `:465` says "#645's and #646's, planned". Once 645a lands, that is out of date for #645's three codes. It is
    for O, at the next ADR-0021 edit, because AC 24 allows no fifth hunk.
  - More ADR-0021 conflicts may come from #644 or #642 part b (R-1). Whichever lands second resolves the conflict by
    anchor text.
  - A-dup's warns and follow-ups F-1 to F-4 are for O to file. Once the `screen_text` fold is filed, the comment in
    `tx_switch.rs` should name its issue.
  - A timed-out `create_session` in reset may leave a session that the message cannot name. That is a 645b question.
  - The accepted risks remain: R-3, R-5 and F-1.

## Files changed

Production (this story, all rounds):
- `crates/holler-pane/src/tx_switch.rs` (round 5: the act at the `acted` boundary, `observe`, and two doc comments)
- `crates/holler-cli/src/pane/switch.rs` (unchanged in round 5)
- `crates/holler-cli/src/pane/reset.rs` (round 5: `run`'s doc only)

Docs (this story; none changed in round 5):
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Pipeline records:
- `docs/handoffs/645/handoff-F.md` (this file)
- `docs/handoffs/645/evidence.md` (round 5: three entries, and the header)
- `docs/handoffs/645/decisions.md` (round 5: the r4 gate round, and the F round-5 entry)
