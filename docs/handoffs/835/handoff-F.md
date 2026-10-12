# Handoff-F: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 2: the guide and the records after the merge with `main`)

**Date:** 2026-10-11 (19:31 MDT)
**Branch:** `issue-835-implementation` at `526bbd3` (T-red's two-parent merge of `origin/main` `bda1069`), F's changes uncommitted
**Issue:** #835

This handoff replaces cycle 1's, which is at `9bd101b` and lists the code: the port method, the adapter, the fake, the
forwards, the import and the switch. This cycle is text only, as handoff-A pass 4 ("Notes for T and F", item 2) and
handoff-T-red cycle 2 ("Ready for F") say.

| Field | Value |
|---|---|
| GitHub issue | #835 |
| Working branch | `issue-835-implementation` |
| Phase | F, cycle 2: the rework after T-green's BLOCK (A pass 4 PASS, T-red cycle 2 PASS) |
| Input documents read | the brief (issue, amendment, rounds 2 and 3); handoff-A pass 4; handoff-T-red cycle 2; handoff-T-green cycle 1; handoff-F cycle 1; `decisions.md`; `evidence.md`; `docs/pane-control.md`; `docs_pane_control_test.rs`; `docs_cli_test.rs`; #654's options (`gh issue view 654`) |
| What this cycle must turn GREEN | `import_findings_are_named_in_section_3_or_10` and `section_3_names_every_flag_of_pane_import`, plus A's W2 to W4 text items (not pinned by a test) |
| Handoff document path | `docs/handoffs/835/handoff-F.md` |

## What was done

**The merge (A's W1)** was already made by T-red: `526bbd3` is a two-parent merge of `bda1069`. `git fetch` at 19:30 MDT
shows `origin/main` still at `bda1069`, so F merged nothing.

**`docs/pane-control.md`** (A's W4):

- **Section 1:** the import also creates no record, by default, for a pane whose tmux session runs a process named
  `claude` (`harness-claude-code`, sections 3 and 10).
- **Section 3, `pane import`:**
  - the synopsis gains `[--adopt-claude-code]`;
  - one sentence says what the switch would do, and that it is refused with `usage` until #831, in a dry run too;
  - two rows in the one findings table, in `ImportFinding::ALL` order after `harness-not-opencode`:
    `harness-claude-code` (info) and `harness-unreadable` (problem);
  - the `observe-failed` row is narrowed to the harness server's two calls, and says a failed tmux read is
    `harness-unreadable` instead;
  - the exit-code paragraph says `usage` comes before any file or port, so a dry run gets it too, and names its four
    causes, the switch among them.
- **Section 10:**
  - the fact is restated: why `harness-not-opencode` does not catch these panes, the rule (a process named exactly
    `claude`), what is still imported as OpenCode (so a merely-down server is still `server-down`), and
    `harness-unreadable`;
  - the rule's limits, with a link to ADR-0021 section 13 ("Decided (#835)") as the contract;
  - step 1's check, as its own paragraph: run the dry run while the Claude Code sessions run; each Claude Code pane
    should show `harness-claude-code`; one that does not (`record`, or `blocked` for another problem) was not
    recognised, and the fallback is #654's option (a); the real run of step 3 also runs while those sessions run;
  - the closing paragraph names `harness-claude-code` beside `harness-not-opencode`.
- **Section 12:** the #835 line is replaced by the adoption that is still later: `--adopt-claude-code` is refused until
  #831, and no story owns the adoption yet.

**The doc comments and the ADR:**

- `crates/holler-pane/src/import/findings.rs`: `ImportFinding::ObserveFailed`'s doc is narrowed to a call to the pane's
  harness server, and points at `harness-unreadable` for a failed tmux read (W4).
- `crates/holler-pane/src/unmanaged.rs` (#840's sentences, W2):
  - the docs of `unregistered` no longer name #835 as a caller; they say the import asks what a pane runs
    (`is_claude_code_command`, an intra-doc link), because no pane it observes has a record, and that a pane it leaves
    alone has no record, so this rule lists it;
  - the docs of `is_unmanaged` say "The question #831 asks".
- `crates/holler-pane/src/pane.rs`: `CLAUDE_CODE_COMMAND`'s doc names #831 as a story that can reuse the predicate, and
  no longer says #832 does (W3).
- `docs/adr/ADR-0021.md`:
  - section 13: "which #831 can reuse" (W3);
  - section 14: "that #831 calls instead of writing its own", plus one sentence that the import does not call the
    registry rule, pointing at section 13's "Not the registry's rule" (W2).
- `docs/handoffs/835/evidence.md`: three facts in unchanged code (section "F, cycle 2").

## Design decisions

1. **Section 10 states the rule and its limits, and links the contract.** A asked for the fact "restated with the rule and
   its limits", and lists "a guide section that restates ADR-0021 section 13 instead of linking to it" as a parallel path.
   - The guide keeps what an operator needs to read a dry run: the rule, what is imported as OpenCode, the problem and
     the limits.
   - It links section 13 for the rest: the port read, "never on a guess", the switch's reasons and "Not the registry's
     rule".
2. **The fact stays one bullet** under "One fact about the live fleet comes first". The rule is how the import meets that
   fact, so section 10's intro and structure stay as #652 wrote them.
3. **Step 1 also covers the real run.** Step 3's real run observes the panes again. A Claude Code session that exits
   between the dry run and the real run shows its shell and would be recorded as OpenCode. So step 1 says the real run
   also runs while those sessions run. W4 names only the dry-run check, but this sentence is inside step 1's text, which
   W4 names, and step 3 is not edited.
4. **The fallback is #654's option (a), in #654's words:** "a copy or a link farm" the operator makes, with the old files
   untouched (read with `gh issue view 654`).
5. **The exit-code paragraph lists all four `usage` causes, not only the switch.** Naming the switch alone would suggest it
   is the only one. The four are ADR-0021 section 9's row and `Input::read`'s checks (evidence, fact 2).
6. **The `observe-failed` row says where a failed tmux read went.** T-green's blocking issue 1 asks the guide to state that
   a failed tmux read is now `harness-unreadable`, a problem, rather than an `observe-failed` warning.
7. **Section 12 says no story owns the adoption yet**, as ADR-0021's "Deferred to named stories" does, so the guide does not
   imply that #831 builds it.

## Reuse / extend-vs-new

Every edit is in place, in the sentences, rows and sections that A's reuse map names for the rework ("The guide
(rework)" and "The records of #840 and of #832's reuse (rework)"):

- no new section, page, table or helper;
- the two findings go into the one findings table of section 3; there is no second table of import findings;
- section 10 links ADR-0021 section 13 rather than restating it (design decision 1);
- no edit to `main`'s files beyond the sentences W2 to W4 name (design decision 3's sentence is inside step 1).

## Architecture notes for A

None. Docs and doc comments only: no code, interface, dependency, protocol, error code or state change.
`archChanged: false`, as A's note 2 expects.

## Deviations from spec / wireframe

None. Design decision 3 adds one sentence beyond W4's list, inside the step W4 names.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test
  docs_pane_control_test: 17 passed, 0 failed
    (import_findings_are_named_in_section_3_or_10 ... ok; section_3_names_every_flag_of_pane_import ... ok)
  docs_cli_test: 3 passed (every_documented_holler_command_parses ... ok, so the new synopsis span parses)

$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
  exit 0. 154 result groups: 2398 passed, 0 failed, 32 ignored
  (T-red's run on the same merged tree: 2396 passed, 2 failed, the two tests above.)

$ cargo clippy --workspace --all-targets -- -D warnings     -> exit 0
$ cargo doc --no-deps -p holler-pane                        -> no warning (the new intra-doc link in unmanaged.rs resolves)
$ bash scripts/lint.sh                                      -> exit 0
$ bash scripts/changelog-check.sh                           -> changelog-check: ok
$ git diff --check                                          -> clean
```

Before the edits, `docs_pane_control_test` failed with exactly T-red's two failures (15 passed, 2 failed). The last edit
(step 1's paragraph in the guide, wording only) came after the workspace run. The only tests that read the guide,
`docs_pane_control_test` and `docs_cli_test`, were run again after it: 17 and 3 passed, and `git diff --check` was clean.

## Evidence appendix

`docs/handoffs/835/evidence.md`, section "F, cycle 2", three facts:

1. `observe-failed` comes only from the harness server's two calls (`observe.rs:177-187`);
2. every `usage` comes from `Input::read`, before the files are read and before `plan` (`pane/import.rs:145-146`,
   `:182-207`);
3. a whole-fleet reconcile pass reports a pane with no record as `unregistered-herdr-pane` (`reconcile.rs:362-372`).

## Tests that look wrong (for T)

None.

## Known issues

1. **`docs/pane-control.md` is 817 lines** (it was 794). `scripts/lint.sh` checks only `crates/**/*.rs`, so nothing
   fails. #652's A-dup named "about 800" as a soft line for the guide. That is S's or the operator's call, not this
   story's.
2. **The epic's contract block** (#633's issue body) is the operator's to edit, as in cycle 1.

## Files changed

This cycle's production files:

- `docs/pane-control.md`
- `docs/adr/ADR-0021.md`
- `crates/holler-pane/src/import/findings.rs`
- `crates/holler-pane/src/unmanaged.rs`
- `crates/holler-pane/src/pane.rs`

For S's scope review, the PR's production files across both cycles are cycle 1's list plus these five:

- `crates/holler-pane/src/{ports.rs, pane.rs, import.rs, unmanaged.rs}`
- `crates/holler-pane/src/import/{assess.rs, findings.rs, observe.rs}`
- `crates/holler-pane-testkit/src/host.rs`
- `crates/holler-adapter-host/src/{lib.rs, tmux.rs}`
- `crates/holler-cli/src/pane/{import.rs, wiring.rs, wiring/bind.rs}`
- `docs/adr/ADR-0021.md`, `docs/adr/ADR-0003.md`, `docs/pane-control.md`
- `CHANGELOG.md`

Run artifacts: `docs/handoffs/835/handoff-F.md`, `docs/handoffs/835/evidence.md` (appended),
`docs/handoffs/835/decisions.md` (appended).
