# Handoff-F: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 4: S's REWORK, cycle 2: #831 owns the adoption, and the same three limits everywhere)

**Date:** 2026-10-11 (21:09 MDT)
**Branch:** `issue-835-implementation` at `d71b091` (S's REWORK commit). F's rework is unstaged on top of it.
`origin/main` is still `d6ae539` (`git fetch` at 21:04 MDT), an ancestor of `HEAD`, so no merge was needed this cycle.
**Issue:** #835

This handoff replaces cycle 3's, which is at `e6d5fa0` (cycles 1 and 2 are at `9bd101b` and `4604693`). This cycle is
text only: the two items of S's cycle-2 REWORK (`handoff-S.md`, "Verdict"), plus S's optional advisory 2.

| Field | Value |
|---|---|
| GitHub issue | #835 |
| Working branch | `issue-835-implementation` |
| Phase | F, cycle 4: the rework after S's second REWORK (A pass 4 PASS, T-green cycle 3 PASS, A-dup second pass PASS) |
| Input documents read | `handoff-S.md` (cycle 2); the brief (issue, amendment, rounds 2 and 3); handoff-F cycle 3; `decisions.md`; `handoff-A-dup.md` (warn 1); ADR-0021 section 13, "Deferred to named stories" and the decisions list; the guide's sections 10 and 12; `pane.rs` and `ports.rs` (the rule's and the port's docs); the help in `pane/import.rs`; the CHANGELOG entry; `scripts/pane-restart:74-88`; #831's issue body, live (`gh issue view 831`) |
| What this cycle must change | Item 1: the two sentences that say no story owns the adoption after #831 name #831 as the owner. Item 2: ADR-0021 section 13's limits and `is_claude_code_command`'s doc list the same three cases as the help, the guide and the CHANGELOG. No behaviour change, no test change, and the guide no longer than 821 lines |
| Handoff document path | `docs/handoffs/835/handoff-F.md` |

## What was done

**Item 1: #831 owns the adoption.** I first confirmed the fact live: #831's issue body (last edited 16:00 MDT) has the
section "Adopting the fleet's existing Claude Code panes (added 2026-10-11, operator decision)", which says, verbatim:
"This story owns the step that adopts Claude Code panes that already exist."

| Where | Change |
|---|---|
| `docs/adr/ADR-0021.md`, "Deferred to named stories" (now `:1108-1111`) | "No story owns the adoption yet." is gone. The entry now names #831 after the colon, as every other entry of the list names its story: "#831, the Claude Code harness, by the operator's decision of 2026-10-11 recorded in #831's issue." The rest stays: the switch is refused until #831 lands, the import leaves such a pane alone, and the write step refuses a `fleet` that exists, "so #831 changes that step or takes another route". |
| `docs/pane-control.md`, section 12 (`:816-817`) | "and no story owns the adoption yet" is gone: "adopting the Claude Code panes the import leaves alone (#831, the Claude Code harness; until it lands, `--adopt-claude-code` is refused);". Still two lines, so the guide stays at 821. |
| `docs/adr/ADR-0021.md`, section 13, "The switch" (S's optional row, now `:982-985`) | "The adoption itself is not built here: #831 owns it (operator, 2026-10-11)." Then "so its owner changes that step" becomes "so #831 changes that step". The example route (`pane launch --profile fleet`) is unchanged, as S said it is accurate. |

No route is described as #831's (S's instruction): both ADR sentences keep "changes that step or takes another route",
which is the same choice #831's own issue leaves open ("through `--adopt-claude-code` on a re-run, or a separate verb").

**Item 2: the same three limits in all five places.** The help, the guide's section 10 and the CHANGELOG already name
three cases: the `claude` has exited, it is not in the foreground (a wrapper script runs it, or it is suspended), or it
was started under another name. The two contract places now name the same three:

| Where | Change |
|---|---|
| `docs/adr/ADR-0021.md`, section 13, "The rule's limits" (now `:973-978`) | One sentence with the three cases: "a Claude Code pane is imported as OpenCode when its `claude` has exited (the pane shows its shell), is not in the foreground (a wrapper script that stays in the foreground runs it, so tmux names the script's interpreter; or it is suspended, so its shell has the foreground) or was started under another `argv[0]` (a wrapper that `exec`s it under another name, or `node` running Claude Code's script)". The first sentence (tmux and `argv[0]` against the old launcher's every argument) and the "a default, not a proof" sentence are unchanged. |
| `crates/holler-pane/src/pane.rs`, `is_claude_code_command`'s doc (`:153-158`) | The same three cases, in the doc's shorter form, then the unchanged "It is a default for an import, not a proof of what a pane runs." |

**S's advisory 2 (optional), folded in.** `crates/holler-pane/src/ports.rs`'s module doc said "the command name of each
pane process of a tmux session". It now says "the command name of the foreground process of each pane of a tmux
session", as the method's own doc does. The bullet is re-wrapped in place (5 lines before and after).

## Design decisions

1. **The limits are one sentence with three parallel cases**, with "its `claude`" as the subject of all three, so the
   list reads the same as the user docs' sentence, and a reader can count three cases in every place.
   - **"Not in the foreground"** keeps the user docs' words and adds, in the ADR only, why each example hides `claude`:
     tmux names the wrapper script's interpreter, or the suspended job's shell has the foreground. These are what S
     checked against tmux's model in cycle 2 (`handoff-S.md`, "Spec compliance").
   - **"Another `argv[0]`"** keeps R2-5's case and its example. Its "a wrapper" is now "a wrapper that `exec`s it under
     another name", which is S's required disambiguation: a wrapper that runs `claude` as a child is the second case, and
     one that `exec`s it under another name is the third. The help, the guide and the CHANGELOG say "another name" for
     the same case; the contract keeps the precise `argv[0]`.
   - **R2-5's two cases and its "a default, not a proof" sentence are kept,** as S asked.
2. **The ADR's "Deferred" entry names #831 after the colon,** because that is the list's pattern ("- The roster's
   `--json` shape beside the envelope: #648."). Before, the entry read "after the Claude Code harness (#831)" and then
   said nobody owned it.
3. **The guide's section 12 line follows its list's pattern too:** the story in parentheses, here with the switch's state
   after a semicolon. It keeps two lines, so the guide stays at 821 lines (S: "Do not grow the guide").
4. **Section 13's optional change is made.** With #831 named as the owner in "Deferred", the bare "its owner" in section
   13 would have left the reader to look it up. Naming #831 adds one short line to the bullet.
5. **The decisions list (decision 22) is not changed.** It records #835's own decision (the default and the switch) and
   is still exact. Who owns the adoption is recorded where S named it, and #831 records its route in its own amendment.

## Reuse / extend-vs-new

All edits are in place, in sentences this story already wrote, in four files already in this story's diff. There is no
new section, page, table, helper or test.

## Architecture notes for A

None. Prose and doc comments only: no code, interface, dependency, protocol, error code or state change.
`archChanged: false`, as S says.

## Deviations from spec / wireframe

None.
- S's two items and S's optional section-13 row are done as S's table asks. The wording is F's, which S allowed "as long
  as every place lists the same three cases".
- One place beyond S's REWORK list: `ports.rs`'s module doc, which is S's optional advisory 2.

## Tier 1 self-check (incl. tests now GREEN)

There is no RED for this cycle: S asked for no test, and no test pins this prose (I checked `docs_pane_control_test.rs`
and `pane_verbs/import.rs`: neither reads section 12, "Deferred" or the limits). S's five commands, then the rest:

```
$ cargo test -p holler-cli --test pane_verbs import::the_adrs_name_the_switch_and_the_two_new_codes
  test import::the_adrs_name_the_switch_and_the_two_new_codes ... ok      (1 passed, 0 failed)
$ cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test
  docs_cli_test: 3 passed, 0 failed       docs_pane_control_test: 17 passed, 0 failed
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane      -> exit 0 (pane.rs and ports.rs docs re-documented)
$ bash scripts/changelog-check.sh                                    -> changelog-check: ok
$ git diff --check                                                   -> clean

$ cargo test -p holler-pane --doc                                    -> 7 passed, incl. both compile_fail,E0046 (ports.rs)
$ cargo clippy --workspace --all-targets -- -D warnings              -> exit 0
$ bash scripts/lint.sh                                               -> exit 0 (size warns only, all pre-existing)
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
  exit 0. 155 result groups: 2406 passed, 0 failed, 32 ignored (the same as cycle 3's, as a docs-only change should give)
$ wc -l docs/pane-control.md                                         -> 821 (unchanged)
```

## Evidence appendix

None: no behaviour outside the diff is relied on. The one outside fact is not in source: #831's issue body, quoted
verbatim under "What was done" (`gh issue view 831`, read at 21:05 MDT; the issue was last updated at 16:00 MDT).

## Tests that look wrong (for T)

None.

## Known issues

1. **S's advisories for the run agent and the operator are unchanged** (`handoff-S.md`, advisories 1, 7 and 8): the reply
   on #835 naming `CLAUDE_CODE_COMMAND` and `is_claude_code_command` for #831, the PR's AI disclosure and coexistence
   notes, and the epic's contract block. None is F's.
2. **The guide is still 821 lines** (817 from this branch plus #851's 4). This cycle adds none. S's advisory 3 applies.

## Files changed

This cycle's production files (4), all already in this story's diff:

- `crates/holler-pane/src/pane.rs` (doc comment only)
- `crates/holler-pane/src/ports.rs` (module doc only)
- `docs/adr/ADR-0021.md`
- `docs/pane-control.md`

The PR's production files across all cycles are cycle 2's list, unchanged.

Run artifacts: `docs/handoffs/835/handoff-F.md` (this file) and `docs/handoffs/835/decisions.md` (appended).
