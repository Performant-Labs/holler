# Handoff-S: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`  (spec audit, cycle 2: after S's REWORK 1 and the merge of #851)

**Date:** 2026-10-11, 20:59 MDT
**Branch:** `issue-835-implementation` at `2156b5f`. `origin/main` is `d6ae539` (#851, PR #854; `git fetch` at 20:59 MDT). It is
an ancestor of the branch through F's two-parent commit `e6d5fa0`, so the branch tree is the PR's merge result.
**Issue:** [#835](https://github.com/Performant-Labs/holler/issues/835), OPEN. I read it live with `gh issue view 835`:
- its body matches the brief's issue section (I diffed them; only a trailing blank line differs);
- it has two comments, at 15:51 and 18:12 MDT, on coordination with #832 and #831 (Advisory 1).

**Brief:** `docs/handoffs/835-brief.md` (the issue, the amendment and rounds 2 and 3), unchanged since `f8a1864`.
**Handoffs reviewed:**
- `handoff-A.md`: pass 4, PASS. Passes 1 to 3 are in git.
- `handoff-T-red.md`: cycle 2. Cycle 1 is at `36b0de8`.
- `handoff-F.md`: cycle 3. Cycles 1 and 2 are at `9bd101b` and `4604693`.
- `handoff-T-green.md`: cycle 3, PASS. Cycle 2 is at `b488a33`; cycle 1's BLOCK is at `25ff495`.
- `handoff-A-dup.md`: second pass, PASS. The first pass is at `e5e35ee`.
- My cycle-1 handoff (REWORK 1), at `ccc53a0`, which this handoff replaces.
- `decisions.md` and `evidence.md`.
- Also read: #831's issue (`gh issue view 831`) and, through git only, its branch `origin/issue-831-implementation` at `008a995`.

**Diff audited:**
- **This cycle.** `git show --remerge-diff e6d5fa0` touches 5 files, all prose or doc comments, plus handoffs. I checked that
  it equals `git diff e77e7a0 e6d5fa0`, where `e77e7a0` is the automatic merge `git merge-tree --write-tree ccc53a0 d6ae539`
  (no conflict). `e6d5fa0..HEAD` changes handoffs only.
- **The whole PR.** `git diff origin/main...HEAD` outside `docs/handoffs/` is 37 files, +1452/-159. The 32 files this cycle
  did not touch are byte-identical to `b488a33`, the tree of my cycle-1 audit. The one exception is ADR-0003, whose only
  change since is #851's, from `main`. I re-read the full production diff anyway.

**Verdict:** **REWORK** (production, docs only), two items. Cycle 1's REWORK 1 (the "foreground" wording) is resolved
everywhere.
1. Two sentences say that no story owns the adoption after #831. Since 15:50 MDT, #831's issue body records, as an operator
   decision, that #831 owns it.
2. In the same pass: the contract's list of the rule's limits (ADR-0021 and `pane.rs`) gains the case that the help, the
   guide and the CHANGELOG now name. This is A-dup's warn 1, which was left to the gate.

## A precondition

Met.
- `handoff-A.md` (pass 4) is **PASS**, with 0 block and 4 warn. All four warns were carried out in cycle 2, which my cycle-1
  audit checked.
- `handoff-A-dup.md` (second pass) is **PASS**, with 0 block and 2 warn:
  - warn 1 (the records list the limits differently) is REWORK item 2 below;
  - warn 2 is test-only and optional (Advisory 6).

## T precondition

Met.
- **RED, cycle 1** (`36b0de8`). It was a compile RED on the new required method, also proved at assertion level:
  - on unchanged code, `import_test`, both `compile_fail` doctests, `cli_surface_test` and six `import::` tests failed;
  - under a bare-API stub, reverted and checked with `sha256sum`, 28 tests failed, each on an assertion and each in this
    story.
- **RED, cycle 2** (`526bbd3`). The guide's two docs tests failed on their assertions.
- **Cycle 3** is text only. No RED is due: I asked for no test, and no test pins this prose.
- **GREEN** (T-green cycle 3, on `e6d5fa0`, which has the same code as `HEAD`):
  - CI's test step: 155 groups, 2406 passed, 0 failed, 32 ignored;
  - clippy with `-D warnings`, `lint.sh`, `changelog-check.sh`, `docs_cli_test`, `docs_pane_control_test` (17),
    `wire_selftest`, `cargo machete`, rustdoc with `-D warnings` for `holler-pane`, and `git diff --check`: all clean;
  - 0 blocking issues.
- **Mutations** (T-green cycle 2). These were not repeated, because no code changed since:
  - the rule always `false`: 4 tests fail;
  - dead panes kept: 1 test fails;
  - the guide's codes and the flag renamed: 2 tests fail.
- **Real tmux.** Case 10 passed on tmux 3.7c.
- **My own check.** The built `target/debug/holler` (20:36 MDT, after the last source edit at 20:25 MDT) prints the
  reworked `pane import --help`.

## Acceptance criteria

Paths are under `crates/`. `harness.rs` is `holler-cli/tests/pane_verbs/import/harness.rs`.

| # | Criterion (source) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | A pane whose tmux session has a foreground process `claude` is not registered. It gets one `info` finding, `harness-claude-code`, and nothing is written for it (issue Work 1; brief §2, §3 and R2-4). | `harness.rs::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode` (:93), a real run:<br>- B has exactly one finding, `harness-claude-code` at `info`;<br>- B's row is `skipped`, with null role, position and session;<br>- B has no record, and `fleet` holds A and C only.<br>The mutation "rule always `false`" fails it. | Met |
| 2 | Not a block: the import goes on for the other panes. One finding and no others, and no harness call for the pane (Work 1; §3). | - The same test: `recorded` is 2, with 2 `health` and 2 `list_sessions` calls for 3 panes.<br>- `a_pane_left_alone_blocks_nothing_even_unlabelled` (:172). | Met |
| 3 | A dry run shows the same finding and writes nothing (Work 1; the issue's fixture). | `a_dry_run_reports_the_pane_left_alone_and_writes_nothing` (:143):<br>- JSON, and text (`B skipped - - -`, `  info harness-claude-code: `);<br>- no store write, no file changed, no record and no profile. | Met |
| 4 | A pane left alone shows as `unregistered-herdr-pane` in a whole-fleet `doctor` pass, which is expected (Work 1). | Criterion 1 (no record), together with `holler-cli/tests/pane_verbs/doctor.rs::unregistered_herdr_pane_is_reported` (:131) and `reconcile.rs:362-372` (`evidence.md`). This is a consequence, not new behaviour. | Met |
| 5 | `--adopt-claude-code` is refused as `usage` (exit 2), naming #831, with nothing written; a dry run is refused too, before any file or port is read (Work 2; §4). | `adopt_claude_code_is_usage_naming_831_and_writes_nothing` (:193), alone and with `--dry-run`, in JSON and text:<br>- exit 2, `usage`, and the text contains `not_implemented_message(831)`;<br>- one stderr line;<br>- no Herdr, host or harness call, no store write and no file changed.<br>The check is the first one in `Input::read` (`holler-cli/src/pane/import.rs:176-182`). | Met |
| 6 | "After #831 it registers them with that harness" (Work 2). | Not built here. Brief §5 puts it out of scope, and `HarnessKind` has only `Opencode`. #831 owns it (#831's issue body, operator decision of 2026-10-11). | Out of scope by a recorded decision. Two sentences that say no story owns it are REWORK item 1. |
| 7 | The signal is a read-only `HostPort::foreground_commands`. It is required, it reads names only and never a command line, and a missing session answers `pane-not-found` (Work 3; §1; R2-1). | - `holler-pane/src/ports.rs:303`: a `compile_fail,E0046` doctest (`PidsOnly`), RED before F.<br>- Conformance case 10 (`holler-pane-testkit/src/conformance/host.rs:226`), with two `Mutant` breaks (`host_conformance_test.rs:241`, `:249`).<br>- The fake: `fake_host_test.rs:316`, `:337` and `:352`.<br>- The adapter, `holler-adapter-host/tests/foreground_test.rs:35`, `:62`, `:78` and `:92`: live panes in tmux order with the dead one dropped; one exact-target `list-panes -s`; no `start_command` in the format (I7); `pane-not-found`; `unavailable` with stderr's first line only; one bad line fails the whole read.<br>- `fake_tmux_test.rs` `METHODS`. | Met |
| 8 | A pane whose OpenCode server is merely down is still adopted and flagged `server-down` (Work 3). | `holler-cli/tests/pane_verbs/import/join.rs::an_unhealthy_server_is_down_or_wedged_as_reconcile_says` (:334, #650). It now runs through the new chain. | Met |
| 9 | Only an exact `claude` is left alone. `bash`, `opencode` and an empty session are imported as OpenCode. There is one constant and one predicate (§2; R2-7). | - `holler-pane/tests/claude_code_rule_test.rs:14` and `:20`.<br>- `harness.rs:93`: A (`bash`) and C (no process) are recorded as `expected_record`.<br>- grep: the constant is `pane.rs:147` and the only comparison is `:158`. The one production caller is `import/observe.rs:131`. The other `"claude"` literals in any `src` are protocol harness ids or test code. | Met |
| 10 | An unreadable signal fails closed. A read error other than `pane-not-found`, a failed `ps` and a lost chain are each `harness-unreadable` (a problem): no record, and a real run is `import-blocked`. A `pane-not-found` from either read is `tmux-session-missing` (§2; R2-6; R2-2). | - `a_failed_ps_is_harness_unreadable_and_blocks` (:227). It replaces #650's opposite test, with a pointer at `join.rs:391-393`.<br>- `a_failed_command_name_read_is_a_problem_and_blocks` (:261), both cases.<br>- `a_lost_chain_is_harness_unreadable_and_blocks` (:327). | Met |
| 11 | The surface: the `cli-surface.txt` line, ADR 0003's row, ADR-0021 section 9's row and the help. `--format=json` shows the finding like every other (Work 4; §3, §4). | - `holler-cli/tests/fixtures/cli-surface.txt:171`, through `cli_surface_test` (RED before F).<br>- `holler-cli/tests/pane_verbs/import.rs::the_adrs_name_the_switch_and_the_two_new_codes` (:123).<br>- The JSON assertions of criteria 1, 3 and 10.<br>- The help, read in the source (`pane/import.rs:67-79`, `:116-119`) and from the built binary. | Met. Cycle 1's REWORK 1 is resolved. |
| 12 | The code table has two new codes, at `info` and `problem`, and the prose counts are updated (R2-2; R2-4). | - `holler-pane/tests/import_test.rs::import_finding_codes_are_the_pinned_set` (:99, RED before F) and `each_import_finding_has_its_level` (:147).<br>- `findings.rs` says "eighteen kinds", "twelve" of them the import's own. That matches `ALL`: 18 kinds, 6 shared. | Met |
| 13 | ADR-0021 records the change (§1 records; R2-2; R2-5): section 2's table and its "amended since" sentence; section 13's "Decided (#835)" (the method, the rule, its limits, the codes, the switch); section 9's row; the closing lists. | The test of criterion 11 pins the row, the paragraph and both codes. I read `:108-109`, `:121-124`, `:760`, `:950-986`, `:1013-1023`, `:1105-1107` and `:1168-1174`. | Met, except REWORK item 1 (`:1106`) and item 2 (`:973-976`) |
| 14 | Every host decorator forwards `foreground_commands`, and each is pinned (R2-1). | - `holler-cli/tests/pane_integration/bind.rs::the_binding_host_passes_the_host_conformance_suite` (:319), now with case 10.<br>- `the_wired_host_passes_the_host_conformance_suite` (:330).<br>- `wired.rs:111`.<br>- `Unwired` answers `NotImplemented` (`holler-cli/src/pane/wiring.rs:328`).<br>The production chain is `BindingHost` over `WiredHost` over `TmuxHost` (`wiring.rs:153`, `:172`). | Met |
| 15 | `HerdrPort::observe_pane` is required, `observe_panes` keeps its default, and `BindingHerdr` and `LazyHerdr` forward both (R3-1; R3-2). | - `holler-pane/src/ports.rs:180`: a `compile_fail,E0046` doctest (`NoObservation`), RED before F.<br>- D33's pins, which were not copied: `pane_integration/observe.rs:52`, `:184`, `:192` and `:202`, and `bind.rs:294`.<br>- The overrides are at `wiring/bind.rs:138-142` and `:253-257`. | Met |
| 16 | Over the real wiring, `pane list` shows an unmanaged pane's working directory, and its last screen change when the adapter reports one; otherwise the time is null in JSON and `-` in text (R3-3). | `pane_integration/observe.rs:290` and `:308`, in JSON and text. They run over `BindingHerdr(LazyHerdr(FakeHerdr))`, which is how `Wiring` builds it (`wiring.rs:140-152`). | Met, as regression pins: D33 made this hold before F |
| 17 | #652's guide is told of the new default and the flag (§6). | - `holler-cli/tests/docs_pane_control_test.rs::import_findings_are_named_in_section_3_or_10` (:451) and `section_3_names_every_flag_of_pane_import` (:531), both RED in cycle 2.<br>- I read sections 1, 3, 10 and 12 against the code. | Met, except REWORK item 1 (section 12, `:816-817`) |
| 18 | A `CHANGELOG.md` `[Unreleased]` entry links #835 and names the follow-up to #832 (§6; R3-4). | `CHANGELOG.md:76-91`, under `## [Unreleased]` and `### Enhancements`, links #633, #835, #831 and #832. `changelog-check: ok`: I ran it. | Met |
| 19 | Fixture addition: a pane left alone writes nothing, in a real run and in a dry run. | Criteria 1 and 3. | Met |
| 20 | Case 10 holds on a real tmux (R2-8): no empty-list assertion, a poll within a bound, the name the process settles to, and a `Mutant` break that fails it. | `holler-pane-testkit/src/conformance/host.rs:226-252` polls every 20 ms for up to 2 s (`:50-51`) and asserts only that `sleep` is named. Both breaks fail it, and it passed on tmux 3.7c. | Met |

No test is a tautology. Each criterion's test either was RED before F, failed under a mutation, or is a regression pin that
is named as one (criterion 16).

## Spec compliance

**Cycle 1's REWORK 1 is resolved, in every place I named and one more.** Each place now says that the import reads the
command name of the foreground process of each tmux pane in the pane's tmux session. Each limits sentence also names a
`claude` outside the foreground. The places:
- the help (`pane/import.rs:67-79`);
- the guide's section 1 (`:29`), section 3's `harness-claude-code` row (`:299`) and section 10's fact (`:697-708`);
- the CHANGELOG (`:79-88`);
- `read_tmux`'s doc (`import/observe.rs:114-116`);
- beyond my table, the `holler-pane` import module doc (`import.rs:24-32`), which is F's DD5 and consistent with the rest.

F reworded two places differently from my suggestions. That was within the rule I gave, so it is not a deviation.

I checked the wording against the adapter: `FOREGROUND_FORMAT` is `#{pane_dead} #{pane_current_command}` (`tmux.rs:35`),
read once per live tmux pane. I also checked it against how tmux names a pane's command: the argv[0] of the leader of the
pane's foreground process group. Under that model, each case of the limits sentence is true:
- an exited `claude` shows its shell;
- a wrapper script that stays in the foreground shows its interpreter, and a suspended `claude` gives the terminal back to
  the shell;
- a `claude` started under another name shows that name.

**As stated, unchanged since cycle 1:**
- the signature, required;
- names only, from the `list-panes` listing that `ps` makes, with a parser as strict as `parse_ps`;
- errors through `Call` and `refused()`, and `ps` left unchanged;
- `HostOp::ForegroundCommands`, whose `as_str` is the adapter's `OP_*`;
- the rule and its one home;
- fail closed, with no record on a guess;
- the two codes and their levels;
- `Outcome::claude_code` through `left_alone`, with no new `Action` or `Level`;
- the switch at one site, with `not_implemented_message(831)`;
- the records.

**The recorded decisions I treat as such:**
- The level is `info` where `harness-not-opencode`'s is `skipped` (brief §3).
- The switch answers `usage`, not `not-implemented` (brief §4).

**Departures from the brief's literal text, each recorded and none silent** (unchanged from cycle 1):
1. `observe_pane`'s change is recorded in section 14, section 2 and decision 23 (A pass 3, W2).
2. R3-2's second bullet is read as applying to forwarding wrappers (A pass 3, W5; Advisory 5).
3. F's design decision 1: a pane whose tmux read failed still has its server asked. It is blocked either way.
4. R3-3's two tests passed before F. They are regression pins of the acceptance item; "no default" is proved by the
   doctest of criterion 15.

**New this cycle: one fact in the docs is out of date (REWORK item 1).** Brief §5 (round 1, written between A's passes 1 and
2, 15:11 to 15:45 MDT) says "No story owns the post-#831 adoption" and asks the operator to "add a line to #831". The
operator did:
- #831's issue body has had a section "Adopting the fleet's existing Claude Code panes (added 2026-10-11, operator decision)"
  since its revision of 15:50 MDT (last edited 16:00 MDT). The section says: "This story owns the step that adopts Claude
  Code panes that already exist."
- #831's branch, `008a995`, records the route too, in its ADR-0021 section 15, item 17. It is not merged.

F wrote "no story owns the adoption yet" into the ADR's "Deferred to named stories" list (cycle 1, committed in `9bd101b` at
18:42 MDT) and into the guide's section 12 (cycle 2, `4604693`, 19:34 MDT). By then #831 already owned it. A, T, A-dup and my cycle-1 audit all repeated the
brief's fact; my cycle-1 advisory 6.2 was wrong on this point.

This is not an `ADVISORY-HOLD`: the brief needs no amendment. Its own contingency ("The operator should add a line to
#831") has happened, and naming #831 changes no decision. This story still builds the refusal, not the adoption.

## Quality audit

- **Correctness and failure handling.** I read the full production diff again.
  - `ps` runs first, and the names are read only after it answers (`import/observe.rs:117-132`).
  - A `pane-not-found` from either read is `tmux-session-missing`.
  - Any other error, and a lost chain, is `harness-unreadable` (`:203-234`). Both are problems, so the pane gets no record and
    a real run is refused before any write (`Plan::refusal`; `writes.rs:38-42`).
  - The Claude Code path returns before the server is asked, and `assess_all` routes it to `Outcome::claude_code`.
  - The messages quote no observed name. Errors go through `failed_message` and `embedded`.
  - The adapter adds no new subprocess path: `Call::start`, `run_tmux`, a strict parse, then `refused()`.
  - Everything is read only, on the pane's own scoped thread.
- **Build guards.**
  - No `unwrap`, `expect`, `panic`, `todo`, `unimplemented` or `unreachable` is added in any `src`. I grepped the added
    lines.
  - The two `#![allow]` lines are in new test files and carry `// #835`.
  - The largest touched `.rs` file is `pane_verbs/launch/rig.rs`, at 839 lines.
  - Clippy with `dead_code = "deny"` is clean (T-green).
  - `rustfmt --check` passes on every touched production file except `wiring.rs:73-77`. That drift is `main`'s line 76, from
    #832 (`git blame` `652841b0`), outside this diff, whose only `wiring.rs` hunk is `:324-330`.
- **Protocol.** Nothing changed: no file in `holler-proto`, no `docs/protocol/v2.md` change and no golden file.
  - `usage` already exists, and the two new codes are import findings.
  - The only fixture change is the additive `cli-surface.txt` line.
  - ADR-0021 section 9's row is updated.
- **Tests.**
  - The import is a CLI verb over ports, so the tests run in-process over the test kit's fakes. The real tmux runs in the
    `--ignored` suite on a scratch server.
  - No fixed sleep is used for synchronisation. Case 10 polls within a bound, as R2-8 asks.
  - The RED-first evidence is in both T-red handoffs.
- **Documentation.**
  - These are updated: `CHANGELOG.md`, ADR 0003, ADR-0021, the guide and the help.
  - No other doc names `pane import`. I grepped `README.md`, `docs/*.md` and the ADRs.
  - The wording of cycle 1's REWORK 1 is correct now.
  - **Two defects remain:** REWORK items 1 and 2.
- **Public-repository privacy.**
  - I grepped all 3,303 added lines, handoffs included. There is no IP, no tailnet or `ts.net` name, no personal host or
    account name, no home path and no secret.
  - Every path is neutral (`/srv/demo`, `/srv/state`, `/opt/demo/bin/claude`, `/usr/bin/sleep`, `/work/seeding`).
  - I7 holds: the format asks for `#{pane_current_command}` only, and `foreground_test.rs` pins that it has no
    `start_command`.
- **Commits and PR.**
  - All 19 commits since `main` have a Conventional subject and a `Co-Authored-By` trailer. That is the form the repo hook
    writes, with no session link, as on `main`.
  - No PR is open yet (`gh pr list --head issue-835-implementation`). Its AI disclosure and coexistence notes are the run
    agent's (Advisory 7).

## Scope check

- **Delivered:**
  - the issue, as amended by rounds 1 to 3;
  - A's pass-4 warns;
  - the guide's update that brief §6 asks for;
  - cycle 1's REWORK 1.
- **This cycle's extras:**
  - **The `holler-pane` import module doc (F's DD5).** It is one sentence, consistent with the rest.
  - **F's merge of `origin/main` (#851).** This is branch upkeep that R2-1 ("it also needs the current `main`") and A's W1
    precedent support.
    - I verified it is the automatic merge plus F's prose.
    - #851 and #835 share only `CHANGELOG.md`, ADR-0003 and the guide, in separate places.
- **Changes to #649's files** are the forwards and the module doc that describes them only, as R2-1 and R3-1 allow.
- **Under-delivery:** only the adoption after #831, which brief §5 puts out of scope and #831 owns.
- **No over-delivery:** there is no unrelated refactor and no new rig, page, table or helper.

## Verdict

**REWORK** (production, docs and one doc comment). There is no test change, so the rework goes to F. **Report
`archChanged: false`.**

1. **Two sentences say no story owns the adoption after #831, and #831 owns it.**

   The source is #831's issue body, section "Adopting the fleet's existing Claude Code panes (added 2026-10-11, operator
   decision)": "This story owns the step that adopts Claude Code panes that already exist."

   | Where | Lines | Change |
   |---|---|---|
   | `docs/adr/ADR-0021.md`, "Deferred to named stories" | 1105-1107 | "No story owns the adoption yet." becomes a sentence that names #831 as the owner, by the operator's decision of 2026-10-11 recorded in #831's issue. Keep the rest: the switch is refused until #831; the import leaves such a pane alone; the write step refuses a `fleet` that exists, so the owner changes that step or takes another route. |
   | `docs/pane-control.md`, section 12 | 816-817 | "and no story owns the adoption yet" becomes a statement that #831 builds the adoption, for example "adopting the Claude Code panes the import leaves alone (#831; until it lands, `--adopt-claude-code` is refused)". |
   | `docs/adr/ADR-0021.md`, section 13 "The switch" (optional) | 980-982 | "so its owner changes that step" may name #831. The sentence is accurate as it is. |

   Do not describe #831's route (a re-run of the import, or another verb). It is undecided on `main`, and #831's own
   amendment records it.

2. **In the same pass: the contract's list of limits gets the case the user docs now name** (A-dup's warn 1). The help
   (`crates/holler-cli/src/pane/import.rs:74-76`), the guide (`docs/pane-control.md:705-707`) and the CHANGELOG
   (`CHANGELOG.md:86-88`) name three ways a Claude Code pane is missed:
   - its `claude` has exited;
   - it is not in the foreground (a wrapper script runs it, or it is suspended);
   - it was started under another name.

   ADR-0021 section 13's "The rule's limits" (`docs/adr/ADR-0021.md:973-976`) and `is_claude_code_command`'s doc
   (`crates/holler-pane/src/pane.rs:153-156`) name two of these. The guide links section 13 as "the contract".
   - Add the missing case to both, for example: "nor is one whose `claude` is not the pane's foreground process (a wrapper
     script that stays in the foreground runs it, or it is suspended)".
   - Keep R2-5's two cases and its "a default, not a proof" sentence. If "(a wrapper, ...)" stays under "another
     `argv[0]`", make it say a wrapper that starts `claude` under another name, so that "wrapper" is not ambiguous.
   - The wording is F's, as long as every place lists the same three cases.

**Change no behaviour and no test.** Do not grow the guide (821 lines: 817 from this branch plus #851's 4).

**Then run:**
- `cargo test -p holler-cli --test pane_verbs import::the_adrs_name_the_switch_and_the_two_new_codes`. It reads ADR-0021:
  keep "Decided (#835)", both codes, and the flag in the `pane import` row.
- `cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test`.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane`, for the `pane.rs` doc and its links.
- `bash scripts/changelog-check.sh`.
- `git diff --check`.

## Advisory notes (non-blocking)

1. **Coordinating with #831 on the constant's name** (for the run agent or the operator; an outward action).
   - #835's comment of 18:12 MDT gives #831's spelling, `pub const CLAUDE_COMMAND`, and asks: "If you have already named the
     constant differently, say so here and #831 adopts your name".
   - This PR names it `holler_pane::pane::CLAUDE_CODE_COMMAND`, with the predicate `is_claude_code_command`.
     `foreground_commands`' signature matches #831's text exactly.
   - So reply on #835 with these two names. #831's branch (`008a995`) still spells `CLAUDE_COMMAND`, in its ADR-0021
     section 15, items 2 and 17, and in its brief. Its brief already says "if #835 has merged, adopt its constant".
   - #831's branch also edits ADR-0021 section 2's `HostPort` row (`:109`), as this PR does. Whichever merges second
     resolves that conflict.
2. **Two internal docs say "pane process"** where the rest says "the foreground process of each tmux pane". They are the
   `ports.rs` module doc (`:27-28`) and the module doc of `harness.rs` (`:1-2`, a test file, T's).
   - Neither is user-facing, and both read as "each pane's process".
   - F may fold `ports.rs` into the rework. This is optional.
3. **The guide is 821 lines,** past #652's soft "about 800". No check enforces that. This story added 23 lines; #851 added 4.
4. **`rustfmt` drift in this story's new test code** (carried from cycle 1). It is test-only and optional. CI has no format
   step, and `main` already drifts.
5. **The test-only Herdr wrappers still use `observe_panes`'s default** (carried). This is harmless; the brief's R3-1 lists
   them as fine.
6. **A-dup's warn 2** (test-only, optional). `ImportRig::run_with_host` copies `run_with_store`. `text_row` is a twin of the
   import rig's `text_line`.
7. **For the run's agent, after the script opens the PR:**
   - add the AI disclosure that `CONTRIBUTING.md` asks for, with `gh pr edit`;
   - add coexistence notes for #832 (PR #836), #840, #652 and #851 (PR #854). They edited `ports.rs`, `wiring.rs`,
     `bind.rs`, ADR-0021 sections 2, 9, 13 and 14, ADR-0003, the guide and the CHANGELOG. Also note #831's open branch
     (Advisory 1);
   - write times in MDT;
   - CI's "PR size" step will warn, because the PR adds more than 800 lines. The step is advisory and does not fail.
8. **For the operator (outside the pipeline):**
   1. **The epic's contract block.** #633's body (the `HostPort ensure_session / run / stop_owned / ps` line) does not name
      `foreground_commands`. Its `HerdrPort` line does not name `observe_pane` or `observe_panes`.
   2. **#831 replaces the refusal at one code site,** `Input::read` (`pane/import.rs:176`). "Refused until #831" is also
      prose in:
      - ADR-0021: section 9's row, section 13's "The switch" and "Deferred";
      - the guide: sections 3 and 12;
      - the help;
      - the CHANGELOG.

      The limits of the rule are also written in five places: the help, the guide, the CHANGELOG, ADR-0021 and `pane.rs`.
   3. **R2-5's real-fleet check.** Run the first real import's dry run while the fleet's Claude Code sessions run. The
      guide's section 10, step 1 says so.
