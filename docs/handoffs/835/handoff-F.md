# Handoff-F: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 6: S's cycle-4 REWORK, the `harness-claude-code` message)

**Date:** 2026-10-12 (05:50 MDT)
**Branch:** `issue-835-implementation` at `bb233f0` (S cycle 4's commit). This cycle's edit is uncommitted, for the
phase commit.
**Issue:** #835
**Architecture changed:** no

This handoff replaces cycle 5's, which is at `59d8d59` (cycles 1 to 4 are at `9bd101b`, `4604693`, `e6d5fa0` and
`f1d1bd0`). The work list is S's cycle-4 REWORK (`handoff-S.md`, "Verdict", item 1) and nothing else.

| Field | Value |
|---|---|
| GitHub issue | #835 |
| Working branch | `issue-835-implementation` |
| Phase | F, cycle 6: the rework after S's fourth REWORK (A pass 4 PASS, T-green cycle 4 PASS, A-dup third pass PASS with W1) |
| Input documents read | the brief (issue, rounds 1 to 3, round 5); `handoff-S.md` (cycle 4); `handoff-A-dup.md` (W1); handoff-F cycle 5; `decisions.md` (F cycle 5 to S cycle 4); `evidence.md`; `import/assess.rs`; the CLI `pane/import.rs` (help, flag doc, `Input::read`); the tests that read the finding (`pane_verbs/import/harness.rs`, `rig.rs`'s `one_finding` and `message`) |
| What this cycle must change | `crates/holler-pane/src/import/assess.rs:104-105`, the message of `Outcome::claude_code`, per S's four conditions; then S's checks |
| Handoff document path | `docs/handoffs/835/handoff-F.md` |

## What was done

**S's item 1, `crates/holler-pane/src/import/assess.rs:104-106`.** The message of `Outcome::claude_code`, which every
pane the import leaves alone carries (text `  info harness-claude-code: ...`, JSON `message`):

- Before: "its tmux session runs Claude Code, which Holler cannot drive yet, so the import leaves the pane alone"
- After: "its tmux session runs Claude Code, and the import does not adopt such a pane until #831's adoption step lands
  (--adopt-claude-code), so it leaves the pane alone"

S's four conditions, one by one:

1. **No claim that Holler cannot drive Claude Code.** The sentence is about the import only.
   `git grep -n "cannot drive" -- crates/holler-pane/src/import` prints nothing.
2. **Why the pane is left alone, in terms true on the merged tree.** The import does not adopt such a pane until #831's
   adoption step lands, and that step is the `--adopt-claude-code` switch (ADR-0021 section 15, item 17; evidence).
3. **The help's words.** "until #831's adoption step lands" is on one source line (`assess.rs:105`), so
   `git grep -n "until #831's adoption step lands"` now finds the message beside the help (`pane/import.rs:80`), the
   CLI module doc (`:9`), ADR-0021 (`:814`, `:1546`) and the guide (`:379`). The places that wrap the phrase across two
   lines are unchanged, and S's Advisory 2 lists them.
4. **A constant.** It is still a `&str` literal with no observed text, passed to `Outcome::left_alone` as before.

**No merge.** `git fetch` at 05:38 MDT: `origin/main` is still `b76f4d2`, an ancestor of `HEAD`, and
`git merge-tree --write-tree HEAD origin/main` exits 0. S's rule: "If it is clean, do not merge."

## Design decisions

1. **The wording is not S's example, word for word.** S's example ("Claude Code, which the import does not adopt")
   makes "which" refer to the harness, but the import adopts panes. "such a pane" fixes the referent. The parenthetical
   names the switch, as S's condition 2 does: "until #831's adoption step lands (`--adopt-claude-code`)". S gave its
   wording "for example", and all four conditions hold.
2. **The flag is written bare, with no backticks.** The import's other runtime text writes a flag the same way:
   `holler-pane/src/import.rs:164` ("run with --dry-run for the report") and the CLI's refusal of this flag
   ("--adopt-claude-code: not implemented (story #831); ...", `pane/import.rs:178-182`).
3. **"so it leaves the pane alone"**, where "it" is the import, already named in the same sentence. It keeps the old
   message's "leaves the pane alone": the issue's words are "leave ... alone", and the constructor
   (`Outcome::left_alone`), both module docs and the guide's code table (`docs/pane-control.md:352`) say "left alone".
4. **An issue number in a `holler-pane` runtime string.** It is the first one in that crate. S's condition 3 asks for
   it, so one grep finds every place #831's step must update. The CLI's refusal of the same flag already names #831 at
   run time (`not_implemented_message(831)`).

## Reuse / extend-vs-new

Nothing new: one string literal, changed in place. `Outcome::left_alone` and the finding kind are unchanged.

## Architecture notes for A

None. One `&str` literal in `holler-pane`. No interface, behaviour, error code, test, protocol or state change.
`archChanged: false`, as S says.

## Deviations from spec / wireframe

None. The wording differs from S's example only as design decision 1 says.

## Tier 1 self-check (incl. tests now GREEN)

There is no RED for this cycle: no test was authored, and no test pins the message. `harness.rs:163` checks only the
text line's prefix, `  info harness-claude-code: `. On `bb233f0` plus this edit:

```
$ git grep -n "cannot drive" -- crates/holler-pane/src/import       -> prints nothing (exit 1)
$ cargo test -p holler-cli --test pane_verbs -- import::harness      -> 7 passed, 0 failed
$ cargo build --workspace --all-targets                              -> exit 0
$ cargo clippy --workspace --all-targets -- -D warnings              -> exit 0
$ bash scripts/lint.sh                                               -> exit 0 (size warns only, none for assess.rs,
                                                                        435 lines)
$ bash scripts/changelog-check.sh                                    -> changelog-check: ok
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
  exit 0. 167 result groups: 2600 passed, 0 failed, 36 ignored, as in cycle 5. Among them:
    pane_verbs 347   pane_integration 71 (4 ignored)   docs_pane_control_test 19   docs_cli_test 3
    cli_surface_test 3   wire_selftest 3   import_test 5   claude_code_rule_test 2   ports_test 8
    Doc-tests holler_pane 7, incl. ports::HostPort (line 351) and ports::HerdrPort (line 228), compile_fail
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane      -> exit 0
$ cargo machete                                                      -> no unused dependencies
$ rustfmt --check --edition 2021 crates/holler-pane/src/import/assess.rs   -> exit 0
$ git diff --check; git diff --check origin/main -- . ':!docs/handoffs'    -> clean, clean
$ git fetch origin (05:38 and 05:48 MDT); git merge-tree --write-tree HEAD origin/main
  origin/main is b76f4d2, an ancestor of HEAD; exit 0, so no merge
$ strings on the built pane_verbs test binary, for the message       -> the new text, whole:
  "its tmux session runs Claude Code, and the import does not adopt such a pane until #831's adoption step lands
  (--adopt-claude-code), so it leaves the pane alone"
```

## Evidence appendix

`docs/handoffs/835/evidence.md`, section "F, cycle 6": one entry (`CHANGELOG.md:11-15`: on the merged tree the pane
verbs drive a Claude Code pane, and the adoption of the existing panes is #831's last step). The section also points to
cycle 5's entry for ADR-0021 section 15, item 17 (`:1359-1368`, unchanged since), and notes that this edit moves every
later `assess.rs` line down by one (cycle 1's `:163-174` is now `:164-175`).

## Tests that look wrong (for T)

None. S's Advisory 3 (`harness.rs:2` and `:9`, wording only) is T's and optional, and this cycle does not touch it.

## Known issues

1. **#831's adoption step has one more place to update.** The message now carries "until #831's adoption step lands",
   as S's Advisory 2 expects ("and, after item 1, the message"). When that step lands, this message changes with it.
2. **S's advisories are unchanged.** History privacy before the first push (1), the PR title and body and the AI
   disclosure (4), the operator's items (5) and the test-only items (6). None is F's.

## Files changed

- `crates/holler-pane/src/import/assess.rs` (the message of `Outcome::claude_code`)

Run artifacts: `docs/handoffs/835/handoff-F.md` (this file), `docs/handoffs/835/decisions.md` and
`docs/handoffs/835/evidence.md` (appended).
