# Handoff-F: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 5: S's cycle-3 REWORK, the merge of `origin/main` after #857, and the prose brought in line with it)

**Date:** 2026-10-12 (05:00 MDT)
**Branch:** `issue-835-implementation` at `cea1c17` (brief round 5). The merge of `origin/main` `b76f4d2` is staged and
uncommitted (`MERGE_HEAD` is `b76f4d2`), so the phase commit records it as a two-parent commit, as the brief asks.
**Issue:** #835

This handoff replaces cycle 4's, which is at `f1d1bd0` (cycles 1 to 3 are at `9bd101b`, `4604693` and `e6d5fa0`). The work
list is S's cycle-3 REWORK (`handoff-S.md`, "Verdict", items 1a to 1e and 2a to 2i) and nothing else, per brief round 5.

| Field | Value |
|---|---|
| GitHub issue | #835 |
| Working branch | `issue-835-implementation` |
| Phase | F, cycle 5: the rework after S's third REWORK (A pass 4 PASS, T-green cycle 3 PASS, A-dup second pass PASS) |
| Input documents read | the brief (issue, rounds 1 to 3, round 5); `handoff-S.md` (cycle 3); handoff-F cycle 4; `decisions.md` (F cycle 3's merge, S cycle 2, F cycle 4); `evidence.md`; `main`'s ADR-0021 sections 2, 9, 13, 14 and 15 (items 2 and 17), "Deferred" and decisions 21 to 29; both sides of every conflict; `docs_pane_control_test.rs` (what it pins in sections 1, 10 and 12); `.github/workflows/ci.yml` (the steps) |
| What this cycle must change | Merge `origin/main` `b76f4d2` and resolve its 10 conflicts and 3 silent defects; make the prose that says the switch or the adoption waits for the Claude Code harness say it waits for #831's adoption step (ADR-0021 section 15, item 17; decision 23); keep every limits statement exactly as it is |
| Handoff document path | `docs/handoffs/835/handoff-F.md` |

## What was done

**The merge.** `git fetch origin` at 04:43 MDT: `origin/main` is still `b76f4d2` (S's 03:40 fetch). Then
`git merge --no-commit --no-ff origin/main`: 10 conflicts, the 10 S listed. Each is resolved as S's table says, and staged
by explicit path:

| File | Resolution |
|---|---|
| `holler-pane/src/import/observe.rs` | Both imports: `crate::pane::{is_claude_code_command, Health}` and `crate::{HarnessEndpoint, PaneError, PaneName, Ports}`. The rest auto-merged into `main`'s `HarnessEndpoint::Port` harness calls plus the branch's `foreground_commands` read. |
| `holler-pane/src/ports.rs`, module doc | `main`'s "Amended since:" header, its #831 bullet and its `mod process; mod router;`, with the branch's #835 bullet (the foreground process of each pane; `observe_pane` lost its default), which now ends in `;` before #831's. |
| `holler-pane/src/ports.rs:395-396` | `main`'s `process-in-use` clause. |
| `holler-cli/src/pane/wiring/bind.rs` | `main`'s four process methods in the list of defaulted methods; `observe_pane` leaves that list (it is required on the merged tree), and the branch's sentence that `observe_pane` and `foreground_commands` have no default closes it. |
| `holler-pane/src/unmanaged.rs` | `main`'s text in both hunks, so the file equals `main`'s (design decision 1). |
| `holler-cli/tests/pane_integration/wired.rs` | `main`'s `HarnessEndpoint` form; the file equals `main`'s. |
| `holler-pane/tests/ports_test.rs` | `main`'s import list; the file equals `main`'s. |
| `holler-pane/tests/claude_code_rule_test.rs` (add/add) | One file, with `main`'s module doc (#831 and #832 reuse the rule); it equals `main`'s. |
| `holler-cli/tests/docs_pane_control_test.rs` | Both: the branch's `section_3_names_every_flag_of_pane_import`, then `main`'s `built_lines_marked_proposed` and its two tests. |
| `docs/adr/ADR-0021.md`, `docs/pane-control.md` | Item 2, below. |

**S's three silent defects** (files git merged without a conflict):

- **1a, `holler-pane/src/pane.rs`:** one `CLAUDE_CODE_COMMAND` and one `is_claude_code_command`, at `main`'s place after
  `impl HarnessKind`. The constant's doc keeps `main`'s use sentence ("the import (#835) and the stories that adopt or show
  such a pane (#831, #832)"); the predicate's doc keeps the branch's three-limit paragraph, byte for byte. The file
  differs from `main`'s only by that paragraph.
- **1b, ADR-0021 "Decisions taken":** the branch's decisions 22 and 23 move after `main`'s 29 as **30** and **31**
  (moved, not copied). `main`'s layout, a blank line after 21, is kept. Nothing cites either by number (`git grep`).
- **1c, ADR-0021 "Deferred to named stories":** one adoption entry. The branch's bullet stays, rewritten per 2a and 2b, and
  `main`'s sentence ("#831 itself, once #835 is on `main`") leaves the #850 bullet (design decision 2).

**Item 2, the prose after #857.** The Claude Code harness is on `main`; what has not landed is #831's adoption step.

| S's item | Where (merged tree) | Now |
|---|---|---|
| 2a | the help `pane/import.rs:79-80` and the flag doc `:117-118` | "until #831's adoption step lands". The runtime text `not implemented (story #831)` is unchanged. |
| 2a | the guide `:335-338` (section 3), `:379` (section 3, exit codes) and `:981-982` (section 12) | "until the adoption step of #831 lands (ADR-0021 section 15, item 17)", "until #831's adoption step lands", and section 12's bullet below. |
| 2a, 2h | `CHANGELOG.md:135`, `:139-140` | "through `HostPort::foreground_commands`" (no longer "the new"; the sentence says the import now reads through it), and "until the adoption step of #831 lands". |
| 2a | ADR-0021 `:814` (section 9 row), `:1035-1040` ("The switch"), `:1455-1457` ("Deferred"), `:1544-1546` (decision 30) | "until #831's adoption step lands (section 15, item 17)". |
| 2b | ADR-0021 "The switch" and "Deferred" | No route is named: "#831 owns it (operator, 2026-10-11), by the route that section 15, item 17 and decision 23 decide", and the "Deferred" entry names "#831's adoption step (section 15, item 17 and decision 23)". The `profile-exists` / `pane launch --profile fleet` sentences are gone. |
| 2c | the guide's section 1 boundary paragraph | `main`'s two harness kinds; no "closed set with one member"; the branch's `harness-claude-code` sentence; no "which waits for #835 and is not built" (design decision 3). |
| 2d | the guide's section 10 fact | The branch's text, byte for byte. |
| 2e | the guide's section 12 | `main`'s #835 bullet is gone; one adoption bullet: "(#831's last step, ADR-0021 section 15, item 17; until it lands, `--adopt-claude-code` is refused)"; `main`'s #850 and permission-prompt bullets are kept. |
| 2f | ADR-0021 section 2's port table | The branch's `HerdrPort` row ("required since #835"); `HostPort` cites both, "(#835, section 13; #831, section 15)", with `main`'s "the read-only"; `main`'s `HarnessPort` row (#831's amendment). |
| 2g | ADR-0021 section 14's unmanaged bullet | `main`'s text. |
| 2i | every limits statement | Unchanged: help, guide sections 1, 3, 10 and 12, the CHANGELOG, ADR-0021 sections 9 and 13, `pane.rs`, `ports.rs`, `import.rs`, `read_tmux`'s doc. |

**Three more places with the same false claim** (the brief's rule: "Prose that says the adoption waits for the Claude Code
harness must say it waits for #831's adoption step"). All are this story's own sentences, and two are in a file S's 2a
names, but S's line list and grep miss them, because the claim is split across lines or phrased "can be driven":

- `holler-cli/src/pane/import.rs`, the module doc's step 1 (`:9-10`) and `CLAUDE_CODE_STORY`'s doc (`:48-50`);
- `holler-pane/src/import/findings.rs:72`, `ImportFinding::HarnessClaudeCode`'s doc: "once Claude Code can be driven
  (#831)" becomes "once #831's adoption step lands";
- ADR-0021 section 13, "The rule" (`:1018`): the same words, the same change (the bullet is re-wrapped in place, still 6
  lines).

## Design decisions

1. **`unmanaged.rs` takes `main`'s text in both hunks, and the branch's optional sentence is not kept.** S allowed "The
   import (#835) does not call it" to stay if it agrees with `main`'s section 14. It does, but `main`'s own paragraph two
   lines below already says it ("The import tells such a pane by its foreground command (#835,
   `pane::is_claude_code_command`), not by this rule", evidence), and the branch's hunk also says "and so does #831",
   which is false on the merged tree: the only callers of the rule are `unregistered` itself and reconcile
   (`reconcile.rs:446`). So the file equals `main`'s and leaves the PR's diff.
2. **"Deferred" keeps the branch's bullet and drops `main`'s sentence.** S's 2a and 2b give fixes for the branch's bullet,
   it names the refusal in force today and points to section 13, and `main`'s "once #835 is on `main`" is met by this PR.
   The bullet follows the list's pattern, the owner after the colon.
3. **The guide's section 1 says "will be a re-run of the import".** With "which waits for #835 and is not built" dropped,
   `main`'s present tense ("is a re-run") would describe the adoption as available, while section 12, which the sentence
   cites, says none of its items is available. One word keeps the sentence true; the rest is `main`'s.
4. **Where the adoption step is named.** ADR places say "(section 15, item 17)", as S's 2a words it. The guide says
   "ADR-0021 section 15, item 17" at its first mention in a section, since its own "section 15" does not exist. The help
   names no ADR section: it is user text, and the issue number already points there.
5. **Section 2's "amended since" sentence** auto-merged `main`'s "Amended again by #831" with the branch's "and by #835,
   which adds the required `foreground_commands`". It is kept: it agrees with the `HostPort` row (#835, section 13; #831,
   section 15), with `main`'s `ports.rs` doc ("#835's: the required `HostPort::foreground_commands`") and with `main`'s
   section 15 item 2 ("the method ... that #835's amendment decides").

## Reuse / extend-vs-new

No new object. #857's API is kept exactly, as `main`'s section 15 item 2 asks of the story that lands second (evidence):
one constant, one predicate, one required port method, the conformance case and the forwards, all `main`'s. The
branch's copies of those files (the tmux adapter, `FakeHost`, the conformance suite, the forwards in `wiring.rs` and
`pane_integration/bind.rs`) are byte-identical to `main`'s after the merge and leave the PR's diff, which falls to 21
files outside the handoffs.

## Architecture notes for A

None. A merge plus prose and doc comments. The merged code is `main`'s code plus this story's, unchanged: the only code
lines in any resolution are `observe.rs`'s two `use` lines (both sides' names) and `ports.rs`'s `mod process; mod router;`
(`main`'s). No interface, dependency, protocol, error code or state change. `archChanged: false`, as S says.

## Deviations from spec / wireframe

- **Three places beyond S's line list** (above), each the same false claim, under the brief's rule for item 2.
- **The guide's section 1: "will be"** for `main`'s "is" (design decision 3).
- Otherwise none.

## Tier 1 self-check (incl. tests now GREEN)

There is no RED for this cycle: no test was authored. The four test files touched are merge resolutions the brief
authorises, three of them now equal to `main`'s; T-green reviews them. On the merged tree (the staged index):

```
$ cargo build --workspace --all-targets                              -> exit 0
$ bash scripts/lint.sh                                               -> exit 0 (size warns only; every .rs of the
                                                                        PR's diff is under 900, the largest ports.rs 621)
$ bash scripts/changelog-check.sh                                    -> changelog-check: ok
$ cargo clippy --workspace --all-targets -- -D warnings              -> exit 0
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
  exit 0. 167 result groups: 2600 passed, 0 failed, 36 ignored. Among them:
    docs_pane_control_test 19 passed (both sides' functions)   docs_cli_test 3   cli_surface_test 3
    pane_verbs 347 (import::harness::*, import::the_adrs_name_the_switch_and_the_two_new_codes)
    pane_integration 71 (4 ignored)   claude_code_rule_test 2   import_test 5   ports_test 8
    Doc-tests holler_pane 7, incl. ports::HostPort (line 351) and ports::HerdrPort (line 228), compile_fail E0046
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane      -> exit 0
$ cargo machete                                                      -> no unused dependencies
$ git diff --check; git diff --check origin/main                     -> clean, clean
  (git diff --cached --check: only main's own handoff files, Known issue 2)
$ git merge-tree --write-tree <staged merge> origin/main             -> exit 0, the staged tree (no conflict)
  (<staged merge> is a throwaway two-parent object from `git write-tree` and `git commit-tree`, on no ref;
   before the merge, `git merge-tree --write-tree HEAD origin/main` exits 1)
$ git grep -n "lands" -- crates/holler-cli/src/pane/import.rs docs/pane-control.md CHANGELOG.md docs/adr/ADR-0021.md | grep 831
  every hit names #831's adoption (or last) step, except ADR-0021:1145, main's own "whichever of #835 and #831
  lands second" (accurate)
$ target/debug/holler pane import --help                             -> "... refused (usage) until #831's adoption step lands"
```

## Evidence appendix

`docs/handoffs/835/evidence.md`, section "F, cycle 5": four entries, each `main`'s unchanged text that this cycle's prose
points to or follows (section 15 item 2's rule for the second story, item 17, decision 23, and `unmanaged.rs:68-73`). Each
quote was checked line by line against its cited lines on the merged tree.

## Tests that look wrong (for T)

None. For T-green: the merge touched `pane_integration/wired.rs`, `ports_test.rs` and `claude_code_rule_test.rs` (each now
`main`'s) and `docs_pane_control_test.rs` (both sides' functions).

## Known issues

1. **Two sentences of `main` go stale when this PR merges, and are not on S's list,** so they are left for S or #831's
   adoption step:
   - ADR-0021 section 15 item 17 (`:1361`): "#835 makes the flag a refused argument (`usage`, naming #831) until this
     harness exists". The harness exists; the refusal waits for item 17 itself. It is #831's record, and item 17's "As
     built: not yet" says the same, so it misleads no one.
   - `CHANGELOG.md:14-15`, #831's entry: the adoption "waits for [#835]". True until this PR merges.
2. **`git diff --cached --check` flags trailing whitespace** in `main`'s own `docs/handoffs/665-brief.md`,
   `667-brief.md` and `667/evidence.md`, which the merge brings in. This PR adds none: `git diff --check origin/main` is
   clean.
3. **The guide is 986 lines** (`main`'s 964, plus this story's 22). Markdown, so the 900 guard does not apply.
4. **S's advisories are unchanged** (the PR body and its AI disclosure, the coexistence notes, `harness.rs:2`, the
   operator's items). None is F's.

## Files changed

This cycle's production files, all already in this story's diff:

- `crates/holler-pane/src/pane.rs` (1a)
- `crates/holler-pane/src/ports.rs` (merge)
- `crates/holler-pane/src/unmanaged.rs` (merge; now equal to `main`'s)
- `crates/holler-pane/src/import/observe.rs` (merge)
- `crates/holler-pane/src/import/findings.rs` (doc comment)
- `crates/holler-cli/src/pane/wiring/bind.rs` (merge)
- `crates/holler-cli/src/pane/import.rs` (doc comments and help)
- `docs/adr/ADR-0021.md`
- `docs/pane-control.md`
- `CHANGELOG.md`

Test files resolved in the merge, not authored: `crates/holler-cli/tests/pane_integration/wired.rs`,
`crates/holler-pane/tests/ports_test.rs`, `crates/holler-pane/tests/claude_code_rule_test.rs`,
`crates/holler-cli/tests/docs_pane_control_test.rs`.

Run artifacts: `docs/handoffs/835/handoff-F.md` (this file), `docs/handoffs/835/decisions.md` and
`docs/handoffs/835/evidence.md` (appended).
