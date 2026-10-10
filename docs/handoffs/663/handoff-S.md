# Handoff-S: Phase 10 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)  (spec audit)

**Date:** 2026-10-09 (21:55 MDT)
**Branch:** issue-663-implementation (worktree `.claude/worktrees/0663-profile-scope-probe`, head `d9e6c3d`; merge base
`0ad2d8a`; `origin/main` is at `519947a`)
**Issue:** #663 (epic #633). Read live with `gh issue view 663`. Its GraphQL `userContentEdits` show the last edit at 08:40 MDT
on 2026-10-09, before the brief was first written (16:38 MDT, `aaf8fb5`), and the issue has no comments. The epic was last
edited at 17:39 MDT. Every epic line the brief quotes is in the live epic, and the epic has no ruling for #663 that the brief
lacks.
**Brief:** `docs/handoffs/663-brief.md`, as amended in `9762a97`
**Handoffs reviewed:**
- `handoff-A.md` (fourth pass), `handoff-T-red.md` (re-entry), `handoff-F.md` (re-entry), `handoff-T-green.md` (re-entry) and
  `handoff-A-dup.md`;
- `decisions.md` and `evidence.md`;
- from git history, the previous run's from-scratch RED (`8fe683b`) and GREEN (`93fb653`);
- the outside diff review `663-diff-result-r1.md` (PASS, no block).

**Verdict:** PASS. Every acceptance criterion of the issue and the brief has a test or check that asserts behaviour.
Decisions 1-23 are built as stated. The two small deviations F reported are documented, and both are doc-only. The quality
checks are clean.

## A precondition

Met.
- `handoff-A.md` (Phase 3, fourth pass, `e46b427`) is **PASS**, with no block and warns W-16 to W-18. None of them asks F for
  a change.
- `handoff-A-dup.md` (Phase 7, `d9e6c3d`) is **PASS**, with warns D-1 and D-2. Neither is a block (see Advisory notes 1 and 8).

## T precondition

Met.
- **RED was confirmed twice:**
  - from scratch, at `8fe683b`: the scope tests were 0 passed and 7 failed, and the probe tests 0 passed and 12 failed, each
    on a feature assertion; 8m was green by design;
  - for the re-entry, at `f79cd05`: 3 passed and 4 failed, all four on the reconcile step's text.
- **GREEN, then:** `handoff-T-green.md` reports 7 of 7 scope tests, 12 of 12 probe tests and 8m passing. Its "Blocking
  issues" section says None.
- **No test was edited** after RED. Both test modules hash the same as at `f79cd05`.
- **The tests were mutation-checked:** 10 mutations in the first run and 4 in the re-entry, and every one was killed.
- **Only `.md` files changed after T-green.** The one commit since `d9210bc` is A-dup's handoff and journal entry.

## Acceptance criteria

I did not re-run the test suites (that is T's job). I read each test against its criterion, and re-ran the source and
document checks myself (AC 5's greps and AC 9-14).

**The issue's acceptance, point by point.** AC 1 runs all 15 suite cases against `StoreScope`. The suite's own docs map
the issue's points onto its cases (`holler-pane-testkit/src/conformance/profile_scope.rs:58-64`).

| Issue criterion | Proving test or evidence | Status |
|---|---|---|
| Membership refusal | Cases 3 (`resolve-non-member-is-pane-not-in-profile`: c3 in Beta and c4 in no profile are refused) and 14 (`pane-in-other-profile-before-any-write`: the act never runs and nothing is written), through `store_scope_passes_the_profile_scope_conformance_suite` (`profile_scope.rs:449`) | PASS |
| Every-pane scope | Case 1: `resolve(Alpha, None)` is exactly c1 and c2, each equal to its record. Alpha's detached spec c3 and the unaffiliated c4 are left out. Case 2 covers a named pane. | PASS |
| A failed act leaves the specs equal (generation +2) | Case 9: the specs equal what they were before, the generation is g + 2, the log gained two `updated` entries, and the pane store is unchanged | PASS |
| A successful act bumps the generation once | Case 5: g + 1, one log entry, the entry replaced in place, the pane store unchanged. Cases 6, 7 and 15 are the same for an append, a remove and a detached remove. | PASS |
| A stale generation gives `profile-conflict` | A stale *restore* is case 11, and AC 3's test adds the message checks. A stale *first write* is `generation-conflict` (case 10), per ADR-0021 section 8 step 2 and the issue's own amended Scope ("a conflict ... after the act ... `profile-conflict`"). The brief records this reading (C2). | PASS |
| No profile is touched without `--profile` | Case 12: `edit_spec(None, ..)` runs only the act, both when it succeeds and when it fails, and the profile store's call log is empty | PASS |
| The profile must exist | Cases 4 and 13 (`profile-not-found`, before the act and before `pane-in-other-profile`) | PASS |
| `run_probe`: all expected strings give `Ok` | 8a `all_expected_strings_present_is_ok` (`probe.rs:429`) | PASS |
| `run_probe`: one missing string gives `Failed`, naming it | 8b `one_missing_string_is_failed_naming_it` (`probe.rs:440`), an exact `Failed { missing: ["gamma"] }` | PASS |
| `run_probe`: a hung command gives `Error` within the timeout | 8e `hung_command_is_error_at_the_timeout` (`probe.rs:470`): at least 300 ms and under 2,300 ms. The 1 s cleanup is a documented narrowing (C7, ADR-0021 section 2). | PASS |
| `run_probe`: no shell (an argv with `;` is passed literally) | 8h `argv_is_never_given_to_a_shell` (`probe.rs:500`): `;` and `$(...)` arrive literally, no marker file is created, and a one-element `printf hello` "could not be started". 8j's spawn failure also holds. AC 9's greps (below) back both. | PASS |

**The brief's ACs:**

| AC | Proving test or evidence | Status |
|---|---|---|
| 1 Conformance | `store_scope_passes_the_profile_scope_conformance_suite` asserts `run_profile_scope_conformance(..StoreScope::new(..)) == Ok(())` with the actor `conformance` | PASS |
| 2 A restore that fails without a conflict | `restore_failure_keeps_its_code_and_names_the_unrestored_edit` (`:460`). It runs once for each of `timeout`, `unavailable` and `store-corrupt`, and checks: the code is the injected error's; the five substrings and no `\n`; the act ran once; the calls are `[Get, CasPut, CasPut]`; and generation 2 holds `s'`. | PASS |
| 3 The restore conflict carries the step | `restore_conflict_names_the_act_error_and_the_reconcile_step` (`:502`) | PASS |
| 4 A first write that times out | `first_write_timeout_says_the_edit_may_have_landed` (`:518`): `timeout`, the act ran 0 times, the message holds `may hold the edit` and the doctor step, and `unavailable` passes through exactly | PASS |
| 5 The step is quoted and built on `doctor_command` | `reconcile_step_single_quotes_the_profile_name` (`:549`): the exact scoped text, `'It'\''s $(id) Demo'` with no `\n`, the exact unscoped text, and equality with `doctor_command(None, false)`. S re-ran the three greps: **0 / 1 / 0**. | PASS |
| 6 A spec filed under another pane | `set_of_a_spec_for_another_pane_is_usage_before_any_write` (`:568`): `usage`, the act ran 0 times, no `CasPut` | PASS |
| 7 A pane-store fault fails before the profile write | `pane_store_fault_fails_a_remove_before_the_profile_write` (`:586`): an exact `Unavailable`, the act ran 0 times, no `CasPut`, generation 1 | PASS |
| 8a-8l The probe runner | `probe.rs:429-576`, one test per sub-criterion, each with the brief's argv, expectations and order. 8f and 8g build the pid file's path, assert it absent, and assert it present only after the return. They poll `ps` at most 40 times, 50 ms apart. The module runs in 1.39 s serially (the budget is 30 s). | PASS |
| 8m The stub's regression guard | `ports_test.rs:513` `run_probe_stub_never_reports_success`, green (T-green) | PASS |
| 9 No shell, no broad kill, two spawns | S re-ran: the forbidden tokens print nothing; `Command::new` counts **2** (the program and `"kill"`); the shell shapes print nothing; `#[cfg(test)]` appears once, and its module is the file's last item | PASS |
| 10 Quality gates | T-green's Tier 1 table: rustfmt on both files, `lint.sh`, `changelog-check.sh`, clippy `-D warnings`, the workspace tests (1,495 passed), the `holler-cli` lib tests, `docs_cli_test`, the test kit (202) and `cargo machete` all pass. S re-checked: 605 and 577 lines (the gate is 900; the 600-line warning is accepted by AC 10 and journalled), and both `#[allow]` lines carry `// #663` | PASS |
| 11 No new `unsafe`, no new dependency | S re-ran both commands: they print nothing | PASS |
| 12 Blast radius | S re-ran: the two `.rs` files, `CHANGELOG.md`, `docs/adr/ADR-0021.md` and `docs/handoffs/663*` only | PASS |
| 13 CHANGELOG | One entry under `## [Unreleased]` (line 8) / `### Enhancements` (line 10), linking #663 (and the epic). It names no host or account. | PASS |
| 14 ADR-0021 amended in place | S re-ran the checks: no table row or heading changed; `RECONCILE_STEP_UNSCOPED\|takes one \(#647\)\|for now the` counts **0**; the added text holds the exact step, `doctor_command`, `try_wait` and `1 MiB`; section 12's first paragraph holds the rule, `section 1` and `(#663)`; there are five hunks, at a, b, the fence bullet, steps 2/5/6 and f, and each cites `(#663)` | PASS |

## Spec compliance

Each of the brief's "Decisions already made" was checked against the code, by line.

**The scope (`crates/holler-cli/src/pane/profile_scope.rs`):**

- **D1 Type.** `StoreScope { profiles, panes, actor }` and `new` are the fake's shape (`:59-74`). It is `Send + Sync` by
  the auto traits; the trait's supertrait makes the compiler check that.
- **D2 `resolve`.** Read P first. Then, for no pane, the slug-filtered `list` in name order (`:85-90`). For a named pane,
  `get` plus membership, and otherwise `pane-not-in-profile` with the briefed `what` text (`:93-100`).
- **D3 The `edit_spec` order** (`:180-200`):
  - get P;
  - `check_filed_under`, before any pane-store call;
  - the pane record, for every edit;
  - `check_joins`, for a `Set` only;
  - the edit (in place, or appended; a `Remove` with no entry still writes);
  - one first write at g;
  - the act, once;
  - one restore at `written.generation`, as `Profile { panes: stored.panes, ..written }`.

  `None` runs the act only.
- **D4.** Every ASSUMPTION the suite pins is confirmed by AC 1. The three message-only differences from the fake are the
  briefed ones (D5-D7).
- **D5 A restore that fails without a conflict** keeps its own code. It appends, in exactly the briefed words, "`profile
  "<P>" still holds|may still hold the edit of <pane>, but the live change or its record failed (<failure>); <step>`"
  (`:146-155`).
  - `with_context` (`:279-309`) names all 23 variants, with no `_` arm.
  - Its rustdoc states the `Timeout.op` stretch.
- **D6 The first write.** A `timeout` gets the briefed "may have landed" text plus the step, and every other error passes
  through unchanged (`:205-216`).
- **D7 The restore conflict.** The `profile-conflict` text is exactly as briefed, with the step (`:137-145`).
- **D8 `reconcile_step(Option<&ProfileName>)`** (`:49-56`):
  - both forms come from `doctor_command(None, false)`;
  - the name is POSIX-single-quoted;
  - the const is gone;
  - the production lines never spell `holler pane doctor`;
  - the rustdoc gives C9's reason for naming no pane, and W-17's limit.
- **D9-D10.** The `usage` text is as briefed (`:225-232`). The pane record is read before the first write, for every edit
  (`:113`).
- **D11 The bound narrowing** is in the module docs (`:24-27`) and in the first paragraph of `edit_spec`'s own rustdoc
  (`:176-179`).
- **D12.** No retry, no pane-record write and no `catch_unwind`. The module docs say a panicking act leaves the first write
  in place (`:27`).

**The probe runner (`crates/holler-pane/src/probe.rs`):**

- **D13 Process setup.** `Command::new(argv[0]).args(rest)`. stdin is null, stdout piped, stderr null. `process_group(0)`
  sits under `#[cfg(unix)]` (`:203-216`). The environment and the working directory are inherited.
- **D14 The verdict order** (`:93-104`, `:135-165`):
  - an empty argv, then a zero or too-large timeout, are each refused before any process starts;
  - then a spawn failure, the cap and the deadline (both kill the group), an unreadable status (no signal) and a non-zero
    exit or a signal are each `Error`;
  - an exit of 0 gives `Ok` or `Failed`.
- **D15 Timing.** One `checked_add` deadline, taken before the spawn.
  - The runner waits on `recv_timeout` first, and only then polls `try_wait` every 10 ms (`POLL`, `:43`).
  - `kill_and_reap` (`:259-267`) runs, in order: the group kill, then `Child::kill`, then the reap, on one 1 s budget.
  - Nothing reaps the leader before the kill.
  - A `try_wait` error returns `WaitFailed` and sends no signal (`:252`).
  - The rustdoc states the `kill` contingency, the long-lived-caller leak and the no-signal rule (`:75-92`).
  - The module docs state the pid-reuse warning (`:19-24`).
- **D16 The group kill.** `kill -s KILL -- -<pid>`, with stdin, stdout and stderr null (`:272-293`). Its outcome is ignored,
  and it shares the budget. A `kill` that outlasts the budget is killed, then reaped with one non-blocking `try_wait`. It is
  not reaped by a blocking wait, which keeps the bound. This is a refinement of "killed and reaped itself", and the function
  rustdoc (`:269-271`) and F's journal entry document it.
- **D17 The cap.** `MAX_OUTPUT = 1 << 20` (`:37`). The read takes `MAX + 1` and reports `Overflow` past it (`:233-240`).
  stderr is never read.
- **D18 Matching** is on bytes and case-sensitive, with no trimming. `missing` keeps the `expect` order and duplicates, and
  an empty string always matches (`:138-147`, `:173-178`).
- **D19 The reasons.** All eleven fixed texts are word for word the brief's. A spawn error carries only its `ErrorKind`
  (`:185`, `:158-160`).
- **D20 The runner stays private.** No new public item. The mechanics (`run_bounded`, `Outcome`) are kept apart from
  `verdict`, and the module docs (`:10-17`) name #696's three points.
- **D21 The tests** are inline, each module opening with the `// #663` allow. `Scratch` makes `hlr-probe-663-<pid>-<n>`
  under the temp dir and removes it on `Drop`. No test sends a signal.

**Process:**

- **D22** is met: ADR-0021 has the six edits a-f (AC 14), and `decisions.md` journals every phase.
- **D23** (the PR) comes after this phase: see Advisory note 7.

**F's two reported deviations** are both documented, and neither is silent:

- a 4-line rustdoc bullet on `run_probe` for the `try_wait` rule, which makes the public doc name all three exceptions the
  ADR calls documented;
- AC 14e's `reconcile.rs:222-243`, cited as the file and its two codes, without line numbers that would drift.

No decision is implemented other than as stated. The brief is not self-contradictory in any way that reached the code:
C1-C10 are resolved in writing, and A's stale-premise warns (W-16, D-1, D-2) leave the implementation correct.

## Quality audit

- **Correctness and failure handling.**
  - **The scope fails closed.** A store that cannot be read or is corrupt fails before P is written (D10). A first-write
    error leaves nothing live moved.
  - **No write is lost.** Every write is a CAS. A restore conflict leaves the other writer's version in place, and
    `profile-conflict` says so.
  - **A failed restore keeps the act's error.** It is inside the message, so it is not lost (D5).
  - **The probe never answers `Ok` on any error path.** It sends a signal only while the leader is unreaped. It bounds
    memory (1 MiB) and time (deadline + 1 s), and it has no `Instant` overflow panic (`checked_add` on both instants).
- **Build guards.**
  - No `unwrap`, `expect`, `panic!`, `unreachable!` or `todo!` on a production line of either file (grepped).
    `unwrap_or_else(Instant::now)` is not `unwrap`.
  - Both `#[allow]`s carry `// #663`.
  - The files are 605 and 577 lines, under 900.
  - No dead code: `dead_code = "deny"` and clippy `-D warnings` pass, and `pub mod pane` → `pub mod profile_scope` makes the
    public items reachable.
  - No new `unsafe` and no manifest change.
- **Protocol.** No wire change. `ProbeResult`, the closed code list, `class_of`, the golden files and
  `docs/protocol/v2.md` are all untouched, and none needs a change.
- **Tests.**
  - The issue scopes acceptance to the test kit, so the scope is tested over the kit's fakes, which is right for an
    in-process helper.
  - The probe tests run real child processes of harmless commands.
  - No test synchronises on a fixed sleep. The only sleeps are the 50 ms ticks of a 40-iteration bounded poll.
  - The RED-first evidence is in T's handoffs (both runs).
  - Every test asserts behaviour: codes, messages, call logs, generations, files on disk and process liveness. Each
    failed on the stub or under a mutation.
- **Documentation.**
  - The `CHANGELOG.md` entry is under `[Unreleased]` / `Enhancements` and links #663.
  - ADR-0021 is amended in place, in sections 1, 2, 8 and 12. The prose reads coherently, with no rewritten paragraph and no
    changed table row or heading.
  - The change adds no CLI surface, log event or protocol field, so no README or other `docs/` change is due.
- **Public-repository privacy.** I grepped every added line of the diff (4,995 lines) for hostnames (including the
  operator's machine names), tailnet and `.ts.net` names, `.local` and `.lan` hosts, IPv4 addresses, home paths, the Unix
  account, email addresses, private domains and secret-shaped tokens.
  - **No hit.** The only matches are the test markers `m1` and `m2` (scratch file names in 8h).
  - **One personal name is in the diff, and it is not a hit.** The operator's name appears in `decisions.md:1`, the
    driver's brief-gate override line. A personal name is not one of the rule's categories, it is already public as the
    author of every commit, and main has the same line (`docs/handoffs/640/decisions.md:1`).
  - The brief-gate and diff-gate artifacts (`663-*-result-r*.md`, `*.prompt.txt`) are gitignored (`git check-ignore -v`).
- **Commit and PR hygiene.**
  - The branch's commits have Conventional Commit subjects (`chore(#663): ...`, `docs(handoffs): ...`), each with a
    `Co-Authored-By:` trailer.
  - The PR does not exist yet: the script opens it after S, and the run's agent adds the disclosure. See Advisory note 7.
- **The outside diff gate (round 1, DeepSeek V4 Pro)** passed with no block. Its warns are all already decided in the
  brief:
  - W-1 is A's W-17;
  - W-2 is D5's documented `op` stretch (F2);
  - W-3 is D16's ignored `kill` outcome, a documented exception in ADR-0021 section 1;
  - W-4 is not an issue.

## Scope check

**Exact.** F delivered `StoreScope`, `reconcile_step`, `run_probe`, one CHANGELOG entry and the ADR edits a-f, and
nothing else:
- no refactor outside the two files;
- no extra feature, and no public runner API (#696);
- no change to `wiring.rs`, the test kit or `holler-pane`'s frozen files.

The issue's blast radius named only the two `.rs` files. The brief widened it to `CHANGELOG.md` (a repo rule) and
`docs/adr/ADR-0021.md` (A's B-1, under the stack rule), in writing (AC 12). Nothing in the brief is under-delivered.
Follow-ups F1-F6 are filing work for O at Phase 11, not F's.

## Verdict

**PASS.** Ready for O.

## Advisory notes (non-blocking)

1. **D-1's fold goes to F2.** No rework runs, so O adds A-dup's 5-line fold to F2: `belongs` (`profile_scope.rs:219-222`)
   becomes `holler_pane::profile_diff::is_member`. O also adds the fake's identical `belongs` to F1. The two functions are
   identical today, and the fold also trims `profile_scope.rs` toward 600 lines.
2. **D5's split wording has no test.** The change from "still holds" to "may still hold" after a timed-out restore is built
   (`profile_scope.rs:147-150`), but no test pins it, because AC 2's substring list leaves it out. One assertion inside AC
   2's loop would pin it. Fold it into F1 or F2, which touch this behaviour anyway.
3. **Five of D19's eleven reasons have no test:** the timeout too large, a probe ended by a signal, an output reader that
   cannot start, output that cannot be read, and an exit status that cannot be read. No AC asks for them. Two are cheap to
   pin when #696 lifts the runner:
   - `Duration::MAX` gives "too large";
   - `["sh", "-c", "kill -KILL $$"]` gives "ended by a signal". That is the probe program signalling itself, so T should
     confirm it fits D21's "the tests signal nothing themselves".
4. **W-17 stands, documented but not fixed.** The printed step does not parse for a profile name that starts with `-`. The
   fix is O's follow-up in `ProfileName::parse`, filed before #662b's `create` merges.
5. **The frozen trait docs still overstate the bounds.** `Prober` says it "returns within the `timeout`", and
   `ProfileScope` that it returns "within I5's bound". ADR-0021 section 2 and both rustdocs state the real bounds, and F4
   amends the trait docs.
6. **The merge.** `git merge-tree --write-tree HEAD origin/main` (`519947a`) conflicts in `CHANGELOG.md` only: keep both
   entries.
   - None of the contract files has moved on main since `0ad2d8a`: `profile.rs`, `ports.rs`, `error.rs`, `findings.rs`
     and the conformance suite.
   - So T-green's result at the merge base stands. CI re-checks it on the merged head.
7. **The PR step (the run's agent, after the script opens the PR):**
   - a Conventional Commit title, `feat(pane): ...` (D23);
   - `Closes #663`;
   - an "AI assistance" section in the PR body, per `CONTRIBUTING.md`, as #709's PR body has.

   `CONTRIBUTING.md` also says agent commits carry `Co-Authored-By:` and a session link. The branch's phase commits carry
   the trailer but no session link, and main's recent squash commits (for example `efd9a00`) carry neither. That gap is
   pipeline-wide, not this story's. The squash message for #663 should carry the trailer.
8. **Follow-ups for O to file (Phase 11):**
   - F1, the fake and the suite to D5-D7, plus `belongs`;
   - F2, the hoists, plus D-1's fold;
   - F4, the trait docs, the crate's "no I/O" wording and the stub test's name;
   - F5, re-quoting #644's and #646's briefs: cite #646 at `ec2b6b3:521-526`;
   - F6, the three shell-word copies: `single_quoted`, #662b's `shell_word` and the test-only `sh_quote`;
   - W-17's grammar change;
   - a #696 comment listing the two merged private runners (#705, #706) and differences (a) to (i), keeping D15's order and
     its no-signal rule.
