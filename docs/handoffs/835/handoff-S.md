[S]

# Handoff-S: #835, `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (spec audit, cycle 3: after F's third pass `f1d1bd0`)

**Date:** 2026-10-12, 03:47 MDT
**Branch:** `issue-835-implementation` at `f1d1bd0`. Its merge base is `d6ae539` (#851).
- **`origin/main` has moved to `b76f4d2`.** I ran `git fetch` at 03:40 MDT. Four PRs merged since the merge base:
  - `0f5835e`: #667, PR #855, 21:11 MDT on 10-11;
  - `037405f`: #648, PR #856, 21:25 MDT;
  - `8462c9e`: #665, PR #858, 00:02 MDT on 10-12;
  - `b76f4d2`: PR #857, "part of #831", 00:50 MDT.
- **The branch conflicts with `origin/main`.** `git merge-tree --write-tree HEAD origin/main` exits 1, with conflicts in 10 files. The automatic merge also leaves three silent defects in files git reports as merged cleanly (see the Verdict).

**Issue:** [#835](https://github.com/Performant-Labs/holler/issues/835), OPEN. I read it live:
- the body is unchanged since cycle 2;
- there are two new comments:
  - 21:21 MDT: the run was parked after two REWORKs;
  - 23:09 MDT: #831 (PR #857) took #835's own names, `CLAUDE_CODE_COMMAND`, `is_claude_code_command` and the required `HostPort::foreground_commands`, and "whichever PR lands second merges main in".

**#831:** I read it live. It is OPEN and was reopened at 00:50 MDT. Its comment says PR #857 is part of #831 and lists what is still to do: "Adopt the existing Claude Code panes (`pane import --adopt-claude-code`): after #835 is on `main`."

**Brief:** `docs/handoffs/835-brief.md`, unchanged since `f8a1864`.

**Handoffs reviewed:**
- `handoff-A.md`: pass 4, PASS.
- `handoff-T-red.md`: cycle 2.
- `handoff-F.md`: the third pass of S's gate (F's header says cycle 4).
- `handoff-T-green.md`: cycle 3, PASS, on `e6d5fa0`.
- `handoff-A-dup.md`: second pass, PASS.
- My cycle-2 handoff at `d71b091` (REWORK) and my cycle-1 handoff at `ccc53a0` (REWORK).
- `decisions.md` and `evidence.md`.
- The orchestrator's Tier-1 log for `f1d1bd0`, `/home/andre-angelantoni/.cache/tmp/835-tier1.log` (exit 0).

**Diff audited:**
- `git diff d71b091 f1d1bd0`. It changes `pane.rs`, `ports.rs`, ADR-0021 and the guide (prose and doc comments only), plus handoffs.
- The whole PR, `git diff origin/main...HEAD` outside `docs/handoffs/`.
- The trial merge tree `3264623` (`git merge-tree HEAD origin/main`).

**Verdict:** **REWORK** (production). Cycle 2's two items are resolved, and on its own base the branch is consistent everywhere. But `main` now contains PR #857. That PR took this story's port method, constant and rule, and it rewrote the same ADR, guide and doc places. The branch must merge `origin/main` and bring its prose in line with it. Without that it cannot merge, and the automatic merge would not compile.

## A precondition

Met.
- `handoff-A.md` (pass 4) is **PASS**, with 0 block.
- `handoff-A-dup.md` (second pass) is **PASS**, with 0 block. Its warn 1 was my cycle-2 item 2, which is now resolved.

## T precondition

Met.
- **RED.** Cycle 1 (`36b0de8`) and cycle 2 (`526bbd3`) are as recorded in cycle 2. Cycle 3 (`e6d5fa0`) and this pass (`f1d1bd0`) are prose only, and no test pins that prose, so no RED is due.
- **GREEN.**
  - T-green cycle 3 is PASS with 0 blocking issues, on `e6d5fa0`. That is the same code as `f1d1bd0`: I read `git diff e6d5fa0 f1d1bd0`, and it touches doc comments and markdown only.
  - The orchestrator's Tier 1 on `f1d1bd0`, which I spot-checked in the log:
    - `changelog-check: ok`;
    - `lint.sh` gives warnings only, for files under 900 lines;
    - 156 `test result: ok` groups, and no `FAILED` and no non-zero `failed` count anywhere;
    - clippy, rustdoc `-D warnings`, `docs_cli_test` and `git diff --check` are recorded as clean.

## Acceptance criteria

These are on the branch as it stands at `f1d1bd0`. Paths are under `crates/`. `harness.rs` is `holler-cli/tests/pane_verbs/import/harness.rs`. Every proving test is as in cycle 2, because no test or code changed since.

| # | Criterion (source) | Proving test or evidence | Status |
|---|---|---|---|
| 1 | A pane whose tmux session has a foreground `claude` is not registered. It gets one `info` finding, `harness-claude-code`, and nothing is written for it (Work 1; §2, §3, R2-4). | `harness.rs::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode`. The mutation "rule always false" fails it. | Met |
| 2 | Not a block, and no harness call for that pane (Work 1; §3). | The same test (2 `health` and 2 `list_sessions` calls for 3 panes), and `a_pane_left_alone_blocks_nothing_even_unlabelled`. | Met |
| 3 | A dry run shows the same finding and writes nothing (Work 1; the fixture). | `a_dry_run_reports_the_pane_left_alone_and_writes_nothing`, in JSON and text. | Met |
| 4 | A pane left alone is `unregistered-herdr-pane` in a whole-fleet `doctor` (Work 1). | Criterion 1, together with `pane_verbs/doctor.rs::unregistered_herdr_pane_is_reported`. | Met (a consequence, not new behaviour) |
| 5 | `--adopt-claude-code` is `usage` (exit 2), names #831, writes nothing, and is refused in a dry run too, before any read (Work 2; §4). | `adopt_claude_code_is_usage_naming_831_and_writes_nothing`. The check is the first one in `Input::read` (`holler-cli/src/pane/import.rs:176-182`). | Met |
| 6 | "After #831 it registers them" (Work 2). | Out of scope (brief §5). The branch now names #831 as the owner: ADR `:982-985` and `:1108-1111`, guide `:816-817`. After the merge, see REWORK item 2. | Out of scope by a recorded decision |
| 7 | `HostPort::foreground_commands` is required, reads names only, and answers `pane-not-found` for a missing session (Work 3; §1; R2-1). | The `ports.rs` `compile_fail,E0046` doctest, conformance case 10, the `Mutant` breaks, `fake_host_test.rs` and `holler-adapter-host/tests/foreground_test.rs`. | Met |
| 8 | A pane whose server is merely down is still adopted and flagged `server-down` (Work 3). | `join.rs::an_unhealthy_server_is_down_or_wedged_as_reconcile_says`. | Met |
| 9 | Only an exact `claude` is left alone. There is one constant and one predicate (§2; R2-7). | `holler-pane/tests/claude_code_rule_test.rs`, and `harness.rs` (`bash` and no process are recorded). The constant is `pane.rs:147` and the only comparison is `:160`. | Met on the branch. The merged tree defines both twice (REWORK 1a). |
| 10 | Unreadable means fail closed: a read error, a failed `ps` or a lost chain is `harness-unreadable` and `import-blocked` (§2; R2-6). | `a_failed_ps_is_harness_unreadable_and_blocks`, `a_failed_command_name_read_is_a_problem_and_blocks`, `a_lost_chain_is_harness_unreadable_and_blocks`. | Met |
| 11 | The surface: the `cli-surface.txt` line, ADR 0003, ADR-0021 section 9, the help, and JSON (Work 4). | `cli_surface_test`, `pane_verbs/import.rs::the_adrs_name_the_switch_and_the_two_new_codes`, and the JSON assertions. | Met |
| 12 | Two new codes with their levels, and the prose counts updated (R2-2; R2-4). | `import_test.rs::import_finding_codes_are_the_pinned_set`, `each_import_finding_has_its_level`. | Met |
| 13 | ADR-0021 records the change: section 2, "amended since", section 13 "Decided (#835)", section 9, the closing lists (R2-2; R2-5). | I read `:108-124`, `:760`, `:950-990`, `:1013-1025`, `:1108-1111`, `:1172-1178`. | Met on the branch. The merge must reconcile it (REWORK 1 and 2). |
| 14 | Every host decorator forwards `foreground_commands`, and each is pinned (R2-1). | `pane_integration/bind.rs` (both conformance runs), `wired.rs:111`, `Unwired`. | Met |
| 15 | `observe_pane` is required, `observe_panes` keeps its default, and both decorators forward both (R3-1; R3-2). | The `ports.rs` `compile_fail,E0046` doctest, the `pane_integration/observe.rs` pins, `wiring/bind.rs` overrides. | Met |
| 16 | Over the real wiring, `pane list` shows the working directory and the last change (or null / `-`) (R3-3). | `pane_integration/observe.rs` (JSON and text, over `BindingHerdr(LazyHerdr(FakeHerdr))`). | Met (regression pins) |
| 17 | #652's guide is told of the default and the flag (§6). | `docs_pane_control_test.rs::import_findings_are_named_in_section_3_or_10` and `section_3_names_every_flag_of_pane_import`. | Met on the branch. The merge must reconcile it (REWORK 2). |
| 18 | A CHANGELOG `[Unreleased]` entry links #835 and names the follow-up to #832 (§6; R3-4). | `CHANGELOG.md:76-91`; `changelog-check: ok`. | Met |
| 19 | A pane left alone writes nothing, in a real run and a dry run. | Criteria 1 and 3. | Met |
| 20 | Case 10 holds on a real tmux: no empty-list assertion and a bounded poll (R2-8). | `holler-pane-testkit/src/conformance/host.rs:226-252`; it passed on tmux 3.7c. | Met |

## Spec compliance

**Cycle 2's REWORK is resolved on the branch.**

1. **#831 owns the adoption.**
   - ADR-0021 "The switch" (`:982-985`): "The adoption itself is not built here: #831 owns it (operator, 2026-10-11)".
   - The "Deferred" entry (`:1108-1111`) names #831 by the operator's decision.
   - The guide's section 12 (`:816-817`) names #831.
2. **The same three limits everywhere.** On the branch, every place that states the rule agrees on each point:
   - the foreground process;
   - the command name (a name only, never a command line);
   - an exact `claude`;
   - three limits: the `claude` has exited, it is not in the foreground (a wrapper script, or suspended), or it runs under another `argv[0]` or name;
   - "a default, not a proof".

   The places I read:
   - the help (`holler-cli/src/pane/import.rs:67-79`);
   - the guide's sections 1 (`:28-30`), 3 (`:299-300`), 10 (`:697-708`, `:775-778`) and 12 (`:816-817`);
   - the CHANGELOG (`:76-88`);
   - ADR-0021 section 9 (`:760`) and section 13 (`:950-990`);
   - `pane.rs:140-160`;
   - `ports.rs:27-31` and `:340-348`;
   - the `holler-pane` import module doc (`import.rs:24-32`);
   - `read_tmux`'s doc (`import/observe.rs:114-116`).
3. **Grep.** There is no "no story owns" anywhere. "each process" and "every process" occur only in correct descriptions of the fake, `stop_owned` and test harnesses. "Pane process" remains only in the test module doc `harness.rs:2` (Advisory 2).

**The recorded decisions still hold:**
- the `info` level, as against `harness-not-opencode`'s `skipped`;
- `usage` rather than `not-implemented`;
- the four recorded departures listed in cycle 2.

**New: `main` has moved under the brief, as #831's body did in cycle 2.** PR #857 (part of #831) put on `main`:
- `HarnessKind::Claude`;
- harness calls keyed by `HarnessEndpoint`;
- copies of this branch's `foreground_commands`, `CLAUDE_CODE_COMMAND`, `is_claude_code_command`, case 10 and the forwards, taken from `9bd101b` and `36b0de8`;
- an ADR-0021 section 15 whose item 17 and decision 23 **decide** the adoption route: a re-run of `pane import --adopt-claude-code` that extends the existing profile, with `already-registered`.

This is not an `ADVISORY-HOLD`. The brief needs no change: the issue's "until #831" switch is still a refusal that #831's last step replaces, and the issue comment of 23:09 MDT says "whichever PR lands second merges main in". It is branch upkeep plus prose that must match the new `main`, which is F's work.

## Quality audit

- **Correctness and failure handling.** There is no code change since cycle 2; my cycle-2 reading stands:
  - `ps` runs first;
  - `pane-not-found` is `tmux-session-missing`;
  - any other error, and a lost chain, is `harness-unreadable`, so the pane gets no record and a real run is `import-blocked`;
  - the Claude Code path makes no harness call;
  - the messages quote no observed text.
- **Build guards.**
  - No new `unwrap`, `expect` or `panic` in `src`. The two `#![allow]` lines carry `// #835`.
  - The largest touched `.rs` file is `pane_verbs/launch/rig.rs`, at 839 lines, and 852 in the trial merge. Every touched `.rs` stays under 900 after the merge too: `pane.rs` is 486 and `apply/rig.rs` is 775.
  - The guide is 821 lines on the branch and `main`'s is 964. It is markdown, so the 900 guard does not apply (`lint.sh` passed).
- **Protocol.** No change.
- **Tests.** Fakes, plus a real tmux in `--ignored`. No fixed sleeps. RED-first evidence is in both T-red handoffs.
- **Documentation.** Consistent on the branch. The merge must reconcile it (REWORK 2).
- **Public-repository privacy.** I grepped every added line of `git diff origin/main...HEAD`, handoffs included, for IPs, `ts.net`, tailnet and tailscale, personal host and account names, home paths, e-mail addresses and keys or tokens. The only hits are rule text that names the categories ("no personal host, tailnet or account names"). Clean.
- **Commits and PR.**
  - All 21 commits since the base have Conventional subjects and a `Co-Authored-By` trailer, the hook's form, as on `main`.
  - No PR exists (`gh pr list --head issue-835-implementation --state all` is empty). The AI disclosure is the run agent's job, after the script opens the PR.

## Scope check

- **Delivered:** the issue as amended by rounds 1 to 3, A's warns, the guide update, and both earlier REWORKs.
- **This pass:** prose only, within my cycle-2 items, plus the optional `ports.rs` wording (my Advisory 2 of cycle 2).
- **No over-delivery.** Under-delivery is only the adoption, which brief §5 puts out of scope and #831 owns.
- **After the merge, part of this PR's diff will already be on `main`** (the port method, constant, rule, case 10 and forwards from #857). That is expected, and the PR body should say so.

## Verdict

**REWORK** (production, with some test files in the merge). It goes to F. Report `archChanged: false`: the contract and behaviour do not change.

1. **Merge `origin/main` (`b76f4d2`) into the branch and resolve it fully.** Line numbers below refer to the trial merge tree `3264623` unless they name `HEAD`.

   a. **Silent duplicate, a compile error.** `crates/holler-pane/src/pane.rs` defines `CLAUDE_CODE_COMMAND` and `is_claude_code_command` twice: `main`'s copy at `:232`/`:242` and the branch's at `:253`/`:265`. git reports this file as merged cleanly.
      - Keep one, at `main`'s position.
      - Its doc keeps `main`'s use sentence ("for the import (#835) and the stories that adopt or show such a pane (#831, #832)") and this branch's three-limit paragraph (HEAD `pane.rs:153-158`).

   b. **Silent duplicate numbering.** In ADR-0021's "Decisions taken", the branch's decisions 22 and 23 (`:1540`, `:1543`) sit beside `main`'s 22 to 29. Renumber the branch's two as 30 and 31.

   c. **Silent duplicate entry.** ADR-0021's "Deferred to named stories" holds both `main`'s adoption sentence (`:1453-1454`, "#831 itself, once #835 is on `main`") and the branch's bullet (`:1476-1479`). Keep one.

   d. **The 10 conflicts.** Take both sides' meaning in each:

      | File | Resolution |
      |---|---|
      | `holler-pane/src/import/observe.rs:20-26` | Import both `is_claude_code_command` and `HarnessEndpoint`. |
      | `holler-pane/src/ports.rs` module doc (`:15-55`) | `main`'s "Amended since:" header and its #831 bullet, plus the branch's #835 bullet, which uses the "foreground process of each pane" wording and says that `observe_pane` lost its default. |
      | `holler-pane/src/ports.rs:406-411` | `main`'s added `process-in-use` clause. |
      | `holler-cli/src/pane/wiring/bind.rs:17-31` | `main`'s list of the four process methods, plus the branch's sentence that `observe_pane` and `foreground_commands` have no default. |
      | `holler-pane/src/unmanaged.rs:66-95` | `main`'s text. The branch's "The import (#835) does not call it" may stay if it agrees with `main`'s section 14. |
      | `holler-cli/tests/pane_integration/wired.rs:112-118` | `main`'s `HarnessEndpoint` form. |
      | `holler-pane/tests/ports_test.rs:18-30` | `main`'s import list. |
      | `holler-pane/tests/claude_code_rule_test.rs` (add/add) | One file. The module doc should say #831 and #832 reuse the rule (`main`). |
      | `holler-cli/tests/docs_pane_control_test.rs:532-571` | Keep both functions (`section_3_names_every_flag_of_pane_import` and `main`'s `built_lines_marked_proposed`). |
      | `docs/adr/ADR-0021.md`, `docs/pane-control.md` | See item 2. |

   e. Then `cargo build --workspace --all-targets` and the full Tier 1. T-green must confirm GREEN on the merged tree, because test files change in the merge.

2. **Bring the prose in line with `main` after #857.** The Claude Code harness has landed; what has not is #831's adoption step (`main`'s ADR-0021 section 15 item 17, decision 23). After the merge these statements are false:

   a. **"until the Claude Code harness (#831) lands".** It should say: until #831's adoption step lands (section 15, item 17). The runtime text `not implemented (story #831)` stays. The places, by HEAD lines:
      - help: `holler-cli/src/pane/import.rs:78-79` and the flag doc at `:116-117`;
      - guide: `docs/pane-control.md:283-284`, `:326`, `:816-817`;
      - `CHANGELOG.md:84-85`;
      - ADR-0021: `:760` (section 9 row), `:979` ("The switch"), `:1109` ("Deferred"), `:1174` (decision 22, to be renumbered).

   b. **"#831 changes that step or takes another route, such as `pane launch --profile fleet`".** This is at HEAD ADR-0021 `:983-985` and `:1110-1111`, and it contradicts `main`'s decision 23. The route is decided: a re-run with `--adopt-claude-code` that extends the existing profile. Refer to section 15, item 17 and decision 23 instead of naming a route.

   c. **The guide's section 1 boundary paragraph** (conflict at the trial merge's `:25-43`):
      - keep `main`'s two harness kinds;
      - drop the branch's "closed set with one member, `opencode`";
      - keep the branch's `harness-claude-code` sentence;
      - drop `main`'s "which waits for #835 and is not built".

   d. **The guide's section 10 fact** (`:867-887`). Take the branch's text, since `main`'s says #835 is "not built".

   e. **The guide's section 12** (`:999-1007`). Drop `main`'s "#835" bullet. Keep one adoption bullet, naming #831's last step, and `main`'s other two bullets.

   f. **ADR-0021 section 2's `HostPort` row** (`:108-116`). Keep the branch's `HerdrPort` row ("required since #835"). The `HostPort` row cites both: "#835, section 13; #831, section 15".

   g. **ADR-0021 section 14's unmanaged bullet** (`:1085-1095`). Take `main`'s text: #831's Claude Code is drivable, and the import uses `is_claude_code_command`. The branch's "#831 adds a second kind and is added to this function then" is out of date.

   h. **The CHANGELOG #835 entry.** "through the new `HostPort::foreground_commands`" should not claim the method as new, because `main`'s #831 entry already announces it, "built to #835's brief". Say that the import now uses it.

   i. **Keep every limits statement exactly as it is on the branch:** the three cases, foreground, name only, exact `claude`. It is consistent now; do not reword it during the merge.

**Then run:**
- the full Tier 1 (`lint.sh`, `changelog-check.sh`, clippy `-D warnings`, `cargo test --workspace`, `docs_cli_test`, `docs_pane_control_test`, rustdoc `-D warnings` for `holler-pane`, `git diff --check`);
- `git merge-tree --write-tree HEAD origin/main` once more, which should now be clean;
- `git grep -n "lands" -- crates/holler-cli/src/pane/import.rs docs/pane-control.md CHANGELOG.md docs/adr/ADR-0021.md | grep 831`, to check item 2a.

## Advisory notes (non-blocking)

1. **The PR body** (for the run agent):
   - say that #857 already carries this story's port method, constant, rule, case 10 and forwards;
   - give the coexistence notes: #832/#836, #840, #652, #851/#854, #667/#855, #648/#856, #665/#858, #831/#857;
   - add the AI disclosure from `CONTRIBUTING.md` with `gh pr edit`;
   - write times in MDT. CI's "PR size" step will warn; it is advisory.
2. **`harness.rs:2`, a test module doc, says "every pane process".** The others say "the foreground process of each tmux pane". TEST-ONLY and optional.
3. **For the operator:**
   - #633's contract block still omits `foreground_commands`, `observe_pane` and `observe_panes`;
   - the refusal "until #831" is prose in about eight places that #831's adoption step must update with its one code change at `Input::read`;
   - R2-5's real-fleet dry run (guide section 10, step 1) is still the only check of the rule's limits.
4. **Carried from cycle 2:**
   - `rustfmt` drift in this story's new test code (test-only);
   - the test-only Herdr wrappers use `observe_panes`'s default;
   - A-dup's warn 2 (`run_with_host` and `text_row` twins).
