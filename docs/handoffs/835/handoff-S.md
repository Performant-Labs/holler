[S]

# Handoff-S: #835, `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (spec audit, cycle 4: after F cycle 5, the merge of `origin/main` after #857)

**Date:** 2026-10-12, 05:35 MDT
**Branch:** `issue-835-implementation` at `ecb04db` (A-dup's commit). F cycle 5's merge commit is `59d8d59`, with parents
`cea1c17` (brief round 5) and `b76f4d2` (`origin/main`, PR #857).
- `git fetch` at 05:23 MDT: `origin/main` is still `b76f4d2`, and it is an ancestor of `HEAD`.
- `git merge-tree --write-tree HEAD origin/main` exits 0.
- The PR's diff, `git diff origin/main...HEAD` outside `docs/handoffs/`, is 21 files: 950 lines added, 124 removed.

**Issue:** [#835](https://github.com/Performant-Labs/holler/issues/835), OPEN, read live. The body equals the brief's issue
section. One comment is new since cycle 3: 03:49 MDT, "Parked again". The operator allowed one more run: F merges per
cycle 3's list, then T-green, then S once more.

**#831:** OPEN, read live. Its comment of 00:50 MDT still lists "Adopt the existing Claude Code panes
(`pane import --adopt-claude-code`): after #835 is on `main`" as still to do.

**Brief:** `docs/handoffs/835-brief.md`, round 5 (`cea1c17`).

**Handoffs reviewed:**
- `handoff-A.md`: pass 4, PASS.
- `handoff-T-red.md`: cycle 2. Cycle 1 is at `36b0de8`.
- `handoff-F.md`: cycle 5.
- `handoff-T-green.md`: cycle 4, PASS.
- `handoff-A-dup.md`: third pass, PASS.
- My cycle-3 handoff (`dc64e11`, REWORK), which this file replaces.
- `decisions.md`, and `evidence.md` (section "F, cycle 5").

**Diff audited:**
- The whole PR, every file: `git diff origin/main...HEAD`.
- The merged files in full: the import (`import/observe.rs`, `import/assess.rs`, `import/findings.rs`, `import.rs`), the
  CLI verb, `pane.rs`, `ports.rs` and `wiring/bind.rs`.
- The merged ADR-0021 (sections 2, 9, 13, 14, 15 items 2 and 17, "Deferred", decisions), the guide (sections 1, 3, 10
  and 12) and the CHANGELOG.

**Verdict:** **REWORK** (production), **one item**. The merge left one runtime message of this story false. Everything
else is done and accepted, including all of cycle 3's work list.

## A precondition

Met.
- `handoff-A.md` (pass 4) is **PASS**, with 0 block.
- `handoff-A-dup.md` (third pass) is **PASS**, with 0 block and 2 warns.
  - W1 is this audit's REWORK item.
  - W2 is test-only and carried (Advisory 6).

## T precondition

Met.
- **RED.**
  - Cycle 1 (`36b0de8`): a compile RED on the missing required method. Then an assertion-level RED with a stub that was
    reverted: 6 `import::harness` tests, both `compile_fail` doctests, `every_surface_line_parses` and
    `import_finding_codes_are_the_pinned_set` failed.
  - Cycle 2 (`526bbd3`): the two guide tests.
  - F cycle 5 authored no test, so no RED is due.
- **GREEN.** T-green cycle 4 is PASS, with 0 blocking issues, on `59d8d59`.
  - CI's workspace step: 167 groups, 2600 passed, 0 failed, 36 ignored.
  - Clippy `-D warnings`, `lint.sh`, `changelog-check`, `docs_cli_test`, `wire_selftest`, `cargo machete`, rustdoc
    `-D warnings`, `git diff --check` and `merge-tree`: all clean.
  - The behaviour-removal check was repeated on the merged tree: with the predicate set to `false`, the three Claude
    Code tests fail.
  - `6787d99` and `ecb04db` change handoffs only.

## Acceptance criteria

All are on the merged tree at `ecb04db`.
- Paths are under `crates/`.
- `harness.rs` means `holler-cli/tests/pane_verbs/import/harness.rs`.
- `observe.rs` in the table means `holler-cli/tests/pane_integration/observe.rs`.

| # | Criterion (source) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | A pane whose tmux session has a foreground `claude` is not registered. It gets one `info` finding, `harness-claude-code`, and nothing is written for it (Work 1; §2, §3, R2-4). | `harness.rs::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode`. T-green's mutation (predicate set to `false`) fails it. | Met. Its message is the REWORK item. |
| 2 | Not a block, and no harness call for that pane (Work 1; §3). | The same test (2 `health` and 2 `list_sessions` calls for 3 panes), and `a_pane_left_alone_blocks_nothing_even_unlabelled`. | Met |
| 3 | A dry run shows the same finding and writes nothing (Work 1; the fixture). | `a_dry_run_reports_the_pane_left_alone_and_writes_nothing`, in JSON and text. | Met |
| 4 | A pane left alone is `unregistered-herdr-pane` in a whole-fleet `doctor` (Work 1). | Criterion 1, together with `pane_verbs/doctor.rs:133` `unregistered_herdr_pane_is_reported`. | Met (a consequence) |
| 5 | `--adopt-claude-code` is `usage` (exit 2) and names #831. It writes nothing and is refused in a dry run too, before any read (Work 2; §4). | `adopt_claude_code_is_usage_naming_831_and_writes_nothing`: no Herdr, host or harness call. It is the first check in `Input::read` (`holler-cli/src/pane/import.rs:177-183`). | Met |
| 6 | "After #831 it registers them" (Work 2). | Out of scope by brief §5. #831 owns it: ADR-0021 section 15, item 17 and decision 23 decide the route, and #831's issue lists it as still to do. | Out of scope, recorded |
| 7 | `HostPort::foreground_commands` is required, reads names only, and answers `pane-not-found` for a missing session (Work 3; §1; R2-1). | The `compile_fail,E0046` doctest at `holler-pane/src/ports.rs:351`, case 10 (`holler-pane-testkit/src/conformance/host.rs:226`), `fake_host_test.rs:316-352` and `holler-adapter-host/tests/foreground_test.rs`. All but the doctest are `main`'s since #857. | Met |
| 8 | A pane whose server is merely down is still adopted and flagged `server-down` (Work 3). | `pane_verbs/import/join.rs:334` `an_unhealthy_server_is_down_or_wedged_as_reconcile_says`. Criterion 1's test records the `bash` pane and the pane with no process. | Met |
| 9 | Only an exact `claude` is left alone. There is one constant and one predicate (§2; R2-7). | `holler-pane/tests/claude_code_rule_test.rs` (`main`'s). The constant is at `pane.rs:232` and the predicate at `:244`. Its only callers are the import (`import/observe.rs:131`) and `main`'s launch check (`tx_launch/process.rs:65`). | Met |
| 10 | An unreadable signal fails closed: a failed read, a failed `ps` or a lost chain is `harness-unreadable`, and a real run is `import-blocked` (§2; R2-6). | `a_failed_ps_is_harness_unreadable_and_blocks`; `a_failed_command_name_read_is_a_problem_and_blocks` (where `pane-not-found` is `tmux-session-missing`); `a_lost_chain_is_harness_unreadable_and_blocks`. | Met |
| 11 | The surface: the `cli-surface.txt` line, ADR 0003, ADR-0021 section 9, the help, and JSON (Work 4). | `cli_surface_test.rs:54`, `pane_verbs/import.rs::the_adrs_name_the_switch_and_the_two_new_codes`, and the JSON assertions in `harness.rs`. | Met |
| 12 | Two new codes with their levels, and the prose counts updated (R2-2; R2-4). | `holler-pane/tests/import_test.rs:99` and `:147`. The `findings.rs` module doc says "eighteen kinds". | Met |
| 13 | ADR-0021 records it: section 2, "amended since", section 13's "Decided (#835)", section 9 and the closing lists (R2-2; R2-5). | `:108-109`, `:118-124`, `:814`, `:1006-1044`, `:1455-1457`, `:1544-1550`. | Met |
| 14 | Every host decorator forwards `foreground_commands`, and each is pinned (R2-1). | `pane_integration/bind.rs:406` and `:417` run the host suite over `BindingHost` and `WiredHost`. `Unwired` answers the method. | Met |
| 15 | `observe_pane` is required, `observe_panes` keeps its default, and both decorators forward both (R3-1; R3-2). | The `compile_fail,E0046` doctest at `ports.rs:228`; `ports.rs:295`; `observe.rs:184`, `:192` and `:202`; `holler-cli/src/pane/wiring/bind.rs:143-147` and `:258-262`. | Met |
| 16 | Over the real wiring, `pane list` shows the working directory and the last change, or null in JSON and `-` in text (R3-3). | `observe.rs:290` and `:308`, in JSON and text, over `BindingHerdr(LazyHerdr(FakeHerdr))`. | Met (regression pins) |
| 17 | #652's guide is told of the default and the flag (§6). | `holler-cli/tests/docs_pane_control_test.rs:458` and `:538`. I read sections 1, 3, 10 and 12. | Met |
| 18 | A CHANGELOG `[Unreleased]` entry links #835 and names the follow-up to #832 (§6; R3-4). | `CHANGELOG.md:131-146`; `changelog-check: ok`. | Met |
| 19 | A pane left alone writes nothing, in a real run and a dry run (fixture additions). | Criteria 1 and 3. | Met |
| 20 | Case 10 holds on a real tmux: no empty-list assertion, and a bounded poll (R2-8). | `conformance/host.rs:226-252`: it polls for `sleep` for up to 2 s and asserts only the name. | Met |

## Spec compliance

**Cycle 3's work list is done.** I checked each item on the merged tree.
- **1a.** One constant (`pane.rs:232`) and one predicate (`:244`), at `main`'s place. The constant's doc has `main`'s use
  sentence, and the predicate's doc has the branch's three-limit paragraph. `pane.rs` differs from `main` only by that
  paragraph.
- **1b.** ADR-0021's decisions run 1 to 31, each number once. #835's two decisions are 30 and 31.
- **1c.** "Deferred to named stories" has one adoption entry (`:1455-1457`).
- **1d.** The 10 conflicts are resolved as tabled.
  - `unmanaged.rs`, `wired.rs`, `ports_test.rs` and `claude_code_rule_test.rs` equal `main`'s.
  - `docs_pane_control_test.rs` keeps both sides' functions.
- **1e.** The build and Tier 1 are green: F's run, cross-checked by T-green.
- **2a and 2b.** No "until the Claude Code harness" remains, and no route is named. Every hit of `lands` with `831` names
  the adoption step, apart from `main`'s accurate ADR `:1145`.
- **2c to 2e.** The guide's sections 1, 10 and 12 are as listed.
- **2f to 2h.** The ADR section 2 rows, the section 14 bullet (`main`'s) and the CHANGELOG wording are as listed.
- **2i.** Every limits statement is unchanged from `f1d1bd0`.
  - In each of the six places, the counts of "has exited", "suspended", "another name" and "a default, not a proof" are
    the same as before.
  - The help's paragraph differs only by 2a's phrase.
  - The guide's one extra "another name" is `main`'s section 2 (the launch check).

**F's additions are accepted.**
- **Three places beyond the list:**
  - the CLI module doc and `CLAUDE_CODE_STORY`'s doc;
  - `ImportFinding::HarnessClaudeCode`'s doc;
  - ADR section 13, "The rule".

  Each is round 5's rule applied to this story's own text.
- **Design decisions 1 to 5:**
  - `unmanaged.rs` takes `main`'s text;
  - "Deferred" keeps the branch's bullet;
  - the guide's "will be";
  - how the adoption step is cited;
  - the "amended since" sentence.

**The recorded decisions still hold.**
- The level is `info`, where `harness-not-opencode`'s is `skipped` (§3).
- The switch is `usage`, not `not-implemented` (§4).
- ADR section 13 records the rule, its limits, the fail-closed paths and the switch.

**The gap: #857 made a runtime message of this story false.** `Outcome::claude_code` (`holler-pane/src/import/assess.rs:104-105`)
gives every pane the import leaves alone this message:

> its tmux session runs Claude Code, which Holler cannot drive yet, so the import leaves the pane alone

On the merged tree, Holler drives Claude Code:
- the CHANGELOG's #831 entry (`:11-13`): `pane launch PANE --harness claude` launches such a pane "and the pane verbs drive
  it";
- `holler-pane/src/unmanaged.rs:68-71`: "Holler drives every `HarnessKind` a record can hold, OpenCode and, since #831,
  Claude Code", and #832's "or whose harness Holler cannot drive" "has no case";
- ADR-0021 section 14 (`:1063-1064`) and the guide's section 1 (`:25-27`).

What still waits is the import's adoption step. Round 5 tells F to correct exactly this claim: "Prose that says the adoption
waits for the Claude Code harness must say it waits for #831's adoption step". Here it is in this story's own runtime
output.
- **It reaches the operator.** The operator reads it in the cutover's dry run, which the guide's section 10, step 1
  (`docs/pane-control.md:899`) tells them to check pane by pane. It appears in the text form (`  info harness-claude-code:
  ...`) and in JSON (`message`).
- **Earlier gates missed it.** My cycle-3 list missed it. F's grep missed it too, because it looked for "lands". A-dup's
  third pass found it (W1).
- **Nothing else depends on the wording.** No test pins it beyond the line's prefix (`harness.rs:163`), and no document
  quotes it.

**F's Known issue 1, ruled: not required in this PR.** Neither sentence is this story's text.
- **ADR-0021 section 15, item 17 (`:1361`):** "#835 makes the flag a refused argument (`usage`, naming #831) until this
  harness exists". It has been stale since #857. But:
  - it is #831's section, outside this brief's ADR sections (sections 2, 9, 13 and 14 and the closing lists: §6, R2-2,
    R3-4);
  - its own "As built: not yet" (`:1378-1379`) says that the adoption is not built;
  - #831's adoption step rewrites this item when it builds it.

  So it is #831's (Advisory 2).
- **`CHANGELOG.md:14-15`, #831's entry, "waits for [#835]".** This repo keeps an `[Unreleased]` entry's forward reference
  as written when a later story lands. Five entries still say that #649 wires the adapters in (for example `:130`), and
  #649's own entry is at `:85`. It is not this PR's to rewrite.

## Quality audit

- **Correctness and failure handling.**
  - The feature's code is cycle 2's, plus `main`'s `HarnessEndpoint::Port` lines from the merge.
  - I read the chain in full (`import/observe.rs`):
    - `ps` runs first, then the name read;
    - a `claude` pane ends the chain before any harness call;
    - `pane-not-found` from either read is `tmux-session-missing`;
    - any other read error, and a lost chain, is `harness-unreadable`, a problem, so the pane gets no record and a real
      run is `import-blocked`.
  - Messages carry observed error text only through `failed_message`'s `embedded`, which is #650's form. The
    `harness-claude-code` message is a constant.
  - The write step is unchanged. A pane left alone is in no spec of `fleet` (`spec_names` in `harness.rs`).
- **Build guards.**
  - The PR's added `src` lines have no `unwrap`, `expect`, `panic!`, `todo!` or `unreachable!`. The test-only
    `unwrap`s are in tests and doctests.
  - The PR adds no `#[allow]`.
  - Every touched `.rs` file is under 900 lines. The largest is `ports.rs`, at 621.
  - The guide (986 lines), ADR-0021 and the CHANGELOG are markdown, which `lint.sh`'s size gate does not cover (it reads
    `*.rs` only).
  - There is no dead code: `HOST_PS` and the other call names are all used.
- **Protocol.** No change. No hub, body, protocol or golden file is in the diff.
- **Tests.**
  - Fakes, plus a real tmux in `--ignored`. No fixed sleep is used for synchronization: case 10's 20 ms poll is bounded
    by `SETTLE`.
  - RED-first evidence is in both T-red cycles.
- **Documentation.**
  - The CLI surface is in the help, `cli-surface.txt`, ADR 0003, ADR-0021 section 9 and the guide's section 3.
  - The two codes are in the guide's table and ADR section 13.
  - The CHANGELOG entry links #835.
  - The README documents no `pane` verb, so it needs nothing.
- **Public-repository privacy.**
  - I grepped all 3,154 added lines, handoffs included, for:
    - IPs, `ts.net`, tailnet and tailscale;
    - home paths, and personal host and account names;
    - e-mail addresses, and keys or tokens.
  - **One hit, in my own role's file:** my cycle-3 handoff, line 32, cited the orchestrator's Tier-1 log by an absolute
    home-directory path that names the local machine account. `main` has no such path. Its handoffs carry only the
    generic `/home/u` and `/home/demo`.
    - **Removed from the tree** by this rewrite of `handoff-S.md`.
    - **History.** It stays in the trees of five unpushed commits: `dc64e11`, `cea1c17`, `59d8d59`, `6787d99` and
      `ecb04db`. The squash merge keeps it off `main`, but pushing the branch publishes those commits in the PR.
    - **Not F's to fix.** This pipeline does not rewrite history. Whether to rewrite it before the first push is the
      operator's decision (Advisory 1).
  - The rest is clean. The fixture paths are generic: `/home/demo` and `/srv/demo`, as on `main`, and the new test's
    `/work/seeding`.
- **Commits and PR.**
  - All 26 commits after `origin/main` have Conventional subjects and the hook's `Co-Authored-By` trailer, as on `main`.
  - No PR exists (`gh pr list --head issue-835-implementation --state all` is empty). The script will title it
    "Implements #835". The AI disclosure and the real title are the run agent's job (Advisory 4).

## Scope check

- **Delivered:**
  - the issue as amended by rounds 1 to 3;
  - round 5's merge and prose;
  - A's warns and the guide update;
  - the three REWORKs before this one.
- **No over-delivery.** F's three extra places fall under round 5's rule.
- **Under-delivery** is only the adoption, which brief §5 puts out of scope and #831 owns.
- **#857 carries part of this story.** The port method, constant, rule, case 10 and forwards are on `main` through #857,
  so the PR's diff fell from 37 files to 21. That is expected, and the PR body should say so.

## Verdict

**REWORK** (production). It goes to F. Report `archChanged: false`: no interface, behaviour or test changes.

1. **`crates/holler-pane/src/import/assess.rs:104-105`, the message of `Outcome::claude_code`.** Replace it with a message
   that meets four conditions:
   - It does not say or imply that Holler cannot drive Claude Code. Since #857, it can.
   - It says why the import leaves the pane alone, in terms that are true on the merged tree: the import does not adopt
     such a pane until #831's adoption step lands (`--adopt-claude-code`).
   - It uses the help's words "until #831's adoption step lands", so that one grep finds every place #831's step must
     update.
   - It stays a constant, with no observed text in it.

   For example: "its tmux session runs Claude Code, which the import does not adopt until #831's adoption step lands, so
   it leaves the pane alone".

   No test changes: `harness.rs:163` pins only the line's prefix, `  info harness-claude-code: `. No document quotes the
   message. No other file changes.

**Then run:**
- `git grep -n "cannot drive" -- crates/holler-pane/src/import`, which must print nothing;
- `cargo test -p holler-cli --test pane_verbs -- import::harness` (7 tests), then the full Tier 1;
- `git fetch`, then `git merge-tree --write-tree HEAD origin/main`. If `origin/main` has moved and this conflicts, merge it
  as in cycle 5. If it is clean, do not merge.

**For the next S.** With this item done and nothing else changed, every criterion and every finding of cycles 1 to 4 is
closed. The notes below are advisory: none of them is a REWORK item, and the rulings above on F's Known issue 1 stand.

## Advisory notes (non-blocking)

1. **Privacy in history (operator, before the first push).** Five unpushed commits carry an absolute home-directory path
   that names the local machine account, in `docs/handoffs/835/handoff-S.md:32` (Quality audit). This file no longer has
   it, and the squash merge keeps it off `main`. Pushing the branch as it stands publishes it in the PR's commits.
2. **For #831's adoption step**, which replaces the refusal:
   - ADR-0021 `:1361`, "until this harness exists";
   - `CHANGELOG.md:14-15`, "waits for #835";
   - every "until #831's adoption step lands": the help, the flag's doc, the two module docs, the guide's sections 3 and
     12, ADR-0021 section 9, section 13, "Deferred" and decision 30, the CHANGELOG, and, after item 1, the message.
3. **Test-only and optional (T).**
   - `harness.rs:2` says "every pane process". It could say "the foreground process of each tmux pane".
   - `harness.rs:9` says "until that story lands". It could say "until #831's adoption step lands".
   - Neither blocks. #831 is still open, so "that story" has not landed.
4. **The PR (run agent).**
   - **Title.** The script titles it "Implements #835", and the squash subject becomes `main`'s record. The issue's title
     ends "and --adopt-claude-code adopts them", which this PR does not do. A title such as "feat(pane): holler pane
     import leaves Claude Code panes alone; --adopt-claude-code is refused until #831's adoption step (#835)" matches the
     CHANGELOG.
   - **Body.**
     - Say that #857 already carries this story's port method, constant, rule, case 10 and forwards, which is why the
       diff is 21 files.
     - Add the coexistence notes: #832/#836, #840, #652, #851/#854, #667/#855, #648/#856, #665/#858 and #831/#857.
     - Add the AI disclosure from `CONTRIBUTING.md` with `gh pr edit`.
     - Write times in MDT.
   - CI's "PR size" step will warn. It is advisory.
5. **For the operator, carried.**
   - #633's contract block still omits `foreground_commands`, `observe_pane` and `observe_panes`.
   - R2-5's real-fleet dry run (the guide's section 10, step 1) is still the only check of the rule's limits.
6. **Carried, test-only.**
   - `rustfmt` drift in this story's new test code. CI has no format gate.
   - The test-only Herdr wrappers use `observe_panes`'s default.
   - A-dup's W2: the `run_with_host` and `text_row` twins.
