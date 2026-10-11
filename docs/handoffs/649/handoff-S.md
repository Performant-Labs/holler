# Handoff-S: Phase 10 - #649 hub wiring and the scratch integration scenario  (spec audit, third pass: the D33 cycle)

**Date:** 2026-10-11 (17:15 MDT)
**Branch:** `issue-649-implementation` (worktree `.claude/worktrees/0649-live-harness`, head `fc58026`). Its merge base
with `origin/main` is `652841b` (#832).
- `origin/main` is now `853c1e1` (#838, merged 16:33 MDT). `main`'s CI passed on it.
- A trial merge (`git merge-tree --write-tree origin/main HEAD`) conflicts in `CHANGELOG.md` only.

**Issue:** #649 (epic #633), read live with `gh issue view 649` (last edited 16:23 MDT).
- Above the Evidence section, its body equals the brief's. The issue carries only the Evidence index.
- It now has D33, D10 (5)'s `LANG=C.UTF-8` clause and D17's #829 wording. The last two are the ones the second pass asked
  for.

**Brief:** `docs/handoffs/649-brief.md` at `6e20075`. Since the second pass it adds D33, renames E64's Source and adds #832
to Depends on.
**Handoffs reviewed (under `docs/handoffs/649/`):**
- `handoff-A.md` (fifth pass, `2280399`) and `handoff-A-dup.md` (third pass, `fc58026`);
- `handoff-T-red.md` (D33 cycle, `0cc840e`);
- `handoff-F.md` (all four cycles);
- `handoff-T-green.md` (D33 cycle, `c35916b`);
- the second S pass (`c68e34e`, ADVISORY-HOLD), which this handoff replaces. Git history keeps it;
- `decisions.md`: O's amendment entry and every entry after it;
- `evidence.md`: the three D33-cycle sections;
- `docs/handoffs/649-diff-result-r1.md`: the outside model's review of this cycle's diff, at 16:58 MDT. It returned PASS
  with no BLOCK, and its prompt held D33 and the `bind.rs` change.

**What changed since the second pass.**
- **The merge.** O merged `origin/main` at `652841b` (#832) in `63b4afa`. `--remerge-diff` shows one conflict,
  `CHANGELOG.md`, which kept both entries.
- **The brief and the issue.** O amended both: D33, E64's Source and Depends on (`a87dac3`, `6e20075`).
- **Production.** `src/pane/wiring/bind.rs` (+35, D33). One clause of this story's CHANGELOG entry (the second pass's
  advisory 6).
- **Tests.**
  - the new `pane_integration/observe.rs`, with 5 tests;
  - one row in `bind.rs`;
  - D33's rows in `store_only.rs`;
  - an end-state check in `scenario.rs` (the second pass's advisory 4).

**Verdict:** PASS.
- D33 is implemented as written, on the tree that includes #832. Tests pin it, and each fails when its behaviour is
  removed.
- The second pass's two defects are closed:
  - #832 is merged in, and its observe methods now reach the Herdr adapter through both wrappers;
  - no tracked file cites the setup wizard's pre-#820 directory, so CI's rename scan passes.
- What remains is PR-phase work and follow-ups for O. None of it is F's or T's:
  - a second merge of `main`, for #838: `CHANGELOG.md` only, and no code to re-gate;
  - #828 is now closed and folded into #839. The operator's own #839 already plans to replace the wording that names it.

## A precondition

Met.
- `handoff-A.md` (fifth pass): **PASS**.
- `handoff-A-dup.md` (third pass, diff `c68e34e..c35916b`): **PASS**, no block. W-7 is new: a nit for T, not binding.
- **No phase-3 pass ran on the amendment.** O's journal entry resumed the run at T-red.
  - A-dup reviewed D33 as a plan and found no drift.
  - D33 is A-dup's own W-5 word for word, plus the second S pass's pins.
  - I take that as A's review of D33. T-red reached the same reading.

## T precondition

Met.
- **Verdict.** `handoff-T-green.md` says "Blocking issues: None."
- **RED, then GREEN, for D33.**
  - **T-red, on the merged tree:** 59 passed, 7 failed. Each failed on its own assertion: the five `observe::*` rows, the
    new every-method row, and `bind::the_binding_herdr_passes_the_herdr_conformance_suite` at #832's case 13.
  - **T-green:** 66 passed, 0 failed, 4 ignored.
  - **Mutants.** M11 to M13 each fail their rows. T-red's mutant forwarded `observe_pane` only, and it fails the three
    recording rows.
- **The opt-in scenario ran 4 of 4 twice on the merged tree:** F in 81.56 s, and T-green in 59.05 s with the new
  end-state check. Neither run left a scratch directory or a process.
- **The workspace run** (CI's command):
  - F: 2345 passed, 0 failed.
  - T-green: 2344 passed, 1 failed. The failure is `remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen`,
    which is outside the branch and passed alone three times. It is a load flake.
- **The rename scan passes.** I re-ran its two needles and its filters over `git ls-files` at `fc58026` and got no hit.
- **Also clean:**
  - clippy `-D warnings` (workspace, all targets);
  - `lint.sh`, `changelog-check` and `machete`;
  - `docs_cli_test` and `wire_selftest`.

## Acceptance criteria

- Paths are under `crates/holler-cli/tests/pane_integration/` unless given in full.
- Each status is for the branch at `fc58026`, which includes #832.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | No hub: every `pane` and `profile` verb exits 1 `unavailable`, naming the missing hub, and touches no adapter (D28). The process stub tests run against a started hub (D22) | **No hub.** `no_hub::every_verb_with_no_hub_is_unavailable_naming_the_hub_in_json` and `…_in_text` run the 20 verbs of `EVERY_VERB`. Each run checks:<br>- one envelope, exit 1, `unavailable`;<br>- "no live holler hub reachable" and the state dir in the message;<br>- untouched traps: a held Herdr listener, a tmux socket path, an `opencode` that leaves a mark, and an empty `PATH`.<br>**D22.** `pane_verbs/process/stub.rs`'s three stub tests run through `LiveHub`. Their assertions are unchanged. F's workspace run has 36 passing in `pane_cli_process` | Met |
| 2 | `pane list`, `get` and `watch` and `profile list` work with no Herdr, tmux or OpenCode (D2) | `store_only::the_store_only_verbs_work_with_no_herdr_tmux_or_opencode`, with an empty `PATH` and no adapter setting. It runs `profile show` too.<br>It now also pins D33's last sentence (`:68-77`): `pane list` exits 0 with the record, adds `"unmanaged_unavailable": "usage"`, and warns in text, naming `HOLLER_HERDR_SESSION`.<br>Also `env::malformed_adapter_settings_do_not_stop_the_store_only_verbs` | Met |
| 3 | A read the hub took and never answered is `timeout` naming the store method, within 10 s. Every other read failure is `unavailable` naming the hub. A JSON-RPC error is `unavailable` for reads and writes | **Timeout:**<br>- `fake_hub::a_read_the_hub_never_answers_is_timeout_within_the_bound` (`pane_store.get`, under 13 s);<br>- `a_watch_request_the_hub_never_answers_is_timeout_naming_watch_next`;<br>- from below, `a_read_and_a_write_answered_after_6_s_succeed`.<br>**Unavailable, naming the hub by its state dir:**<br>- `a_read_whose_reply_is_lost_is_unavailable_naming_the_hub`;<br>- `a_json_rpc_error_is_unavailable_naming_the_hub_for_reads_and_writes`;<br>- `every_read_and_write_with_no_hub_is_unavailable_naming_the_hub`.<br>`fake_hub.rs` is unchanged since the second pass | Met |
| 4 | A malformed `HOLLER_TMUX_SOCKET` is `usage` naming it, from the host and the harness, never the default socket (D6) | `wired::a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server` and `wired::a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server`, through `Wiring::connect` against a real hub, with a `TMUX_TMPDIR` trap. The reader's own row is `env::a_relative_tmux_socket_is_usage_never_the_default_server` | Met |
| 5 | A write whose reply is lost is `timeout` (D3) | `fake_hub::a_write_whose_reply_is_lost_is_timeout` covers all five writes, each naming its `op`. Also `a_write_the_hub_never_answers_is_timeout_within_the_bound` | Met |
| 6 | The scenario passes on scratch instances only. Nothing outside them is touched, real names are refused, and the guards are tested in the default run | **The opt-in run:** 4 of 4, twice, on the merged tree.<br>**`guards::*`** has 7 tests:<br>- the gate on exactly `1`;<br>- the session, with `default` refused;<br>- pane names;<br>- paths inside the base;<br>- the binary's exact environment, `LANG` included;<br>- its refusals;<br>- the tmux server's exact environment | Met |
| 7 | The scenario's order, then the repairs of D21 | `scenario::the_scenario_runs_in_order_on_scratch_instances`, in order:<br>- two launches, a relaunch, `reset --first`, a switch and a park;<br>- a killed server and a deleted session, with one `doctor` finding both;<br>- a hub restart, with `pane list` unchanged;<br>- then `reset C1`, `unpark C2`, `relaunch C2` and the stray deletes | Met (advisory 3) |
| 8 | No step types into a TUI. At the end, `doctor` reports no finding and the registry equals Herdr's snapshot | There is no `send_text`, `send_keys` or `send-keys` in `scenario.rs` or `scratch/mod.rs`.<br>`assert_end_state` (`scenario.rs:237-274`) checks three things:<br>- `doctor` reports no finding;<br>- each record's `(pane_id, grid)` equals Herdr's snapshot;<br>- new: an unscoped `pane list` has no `unmanaged_unavailable`, and no recorded pane is in its `unmanaged` table | Met (advisory 4) |
| 9 | The profile scenario | `scenario::the_profile_scenario_runs_on_scratch_instances` | Met |
| 10 | Each grid launch sits at row 2, column 1 in Herdr's own snapshot, in a workspace whose r1c1 a Holler launch made first (D23) | `scenario::the_two_grid_spellings_land_at_row_2_column_1_in_herdrs_snapshot` | Met |
| 11 | Every `--format=json` output parses against the envelope, each `watch` line too | `Scratch::json` runs #638's `check_envelope` (`scratch/mod.rs:358-362`). The scenario checks each line of its final `pane watch` (`scenario.rs:180-185`). The default run has `watch::pane_watch_prints_the_put_of_a_client_write_each_line_an_envelope` | Met |
| 12 | The scenario's launch resolves `attach_tui` before the record exists, pinned by a test | Indirect in the scenario: a launch writes no record before `attach_tui`, and no label (E26). The direct pin is row 14 | Met |
| 13 | The server argv runs under a real tmux pty (D15) | `scenario::the_server_argv_runs_under_a_real_tmux_pty_and_stops`, with the default spec and an agent spec | Met |
| 14 | `attach_tui` resolves before the record exists, with the decorators over fakes, in the default run (D8, D24) | `bind::launch_resolves_attach_tuis_pane_from_the_bindings_before_the_record_exists`.<br>Supporting:<br>- the binding-order rows;<br>- `bind::the_binding_herdr_passes_the_herdr_conformance_suite`, green again on the merged tree, case 13 included;<br>- `bind::the_binding_host_passes_the_host_conformance_suite` | Met. The second pass's merged-tree failure is closed |
| 15 | No credential is printed, written to a file or put on an argv (D28) | **By construction:**<br>- the wiring reads only its `HOLLER_*` table, `HERDR_SOCKET_PATH`, `XDG_CONFIG_HOME`, `HOME` and `PATH`;<br>- the harness runs with `ProcessEnv::Inherit`;<br>- guards 4 and 5 pin both environments.<br>`Scratch::assert_no_credential_leaked` runs at `scenario.rs:138`, `:187` and `:325` | Met (advisory 1) |
| D33 | (a decision, not an AC bullet) Both Herdr wrappers forward #832's two observe methods | The table under Spec compliance | Met |

**The tests assert outcomes, not the implementation.**
- The outcomes they check are codes, messages, call logs, files, sockets, processes and Herdr's own snapshot.
- D33's recording rows assert the answers and the call shape that ADR-0021 section 14 requires: one `observe_panes`. They
  do not assert F's code.
- Mutants M1 to M13 and the `move` check each fail when their behaviour is removed. The two 6 s rows pin the store's
  bound from below.

## Spec compliance

**D1 to D32 hold.**
- No production line outside `bind.rs` has changed since the second pass, which checked each decision.
- The merge touched `wiring.rs` only with #832's `Unwired::observe_pane` and its import.
- The deviations A reviewed stand:
  - the `Effect` enum;
  - `stores()` returning trait objects;
  - `Wired<P>`;
  - `env::Lookup`.

**D33, clause by clause.** `bind.rs` here is `src/pane/wiring/bind.rs`.

| D33 says | Code | Pinned by |
|---|---|---|
| `BindingHerdr` overrides `observe_pane` and `observe_panes` | `bind.rs:137-143` | `observe::the_binding_herdr_delegates_observe_pane_and_observe_panes`; case 13 in `bind::the_binding_herdr_passes_the_herdr_conformance_suite` |
| `LazyHerdr::observe_pane` is `self.port()?.observe_pane(pane)` | `bind.rs:248-250` | `observe::the_lazy_herdr_delegates_observe_pane_and_observe_panes`; case 13 in `observe::the_lazy_herdr_passes_the_herdr_conformance_suite` |
| `observe_panes` goes to the inner port's own `observe_panes`: one call, never one `observe_pane` per pane | `bind.rs:141-143`, `:252-257` | The three recording rows: the inner log is `["observe_panes"]` and nothing else. One row runs production's chain, `BindingHerdr` over `LazyHerdr` (`observe.rs:186-192`) |
| Failed build: `observe_panes` answers one empty observation per pane | `bind.rs:253-256` | `observe::a_lazy_herdr_that_cannot_be_built_observes_nothing_for_each_pane`: three panes give three defaults, no pane gives `[]`, and the build ran once |
| Failed build: `observe_pane` answers the stored error | `bind.rs:249`, through `port()` | `bind::a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method` (row `:152`): `usage` naming `HOLLER_HERDR_SESSION` |
| The opt-in scenario is re-run on the merged tree | none | F's and T-green's 4 of 4 |
| With no Herdr configured, an unscoped `pane list` exits 0 with the registry, adds `"unmanaged_unavailable": "usage"` in JSON, and warns in text naming `HOLLER_HERDR_SESSION` | #832's `pane/list.rs:86-149`, through `LazyHerdr`'s failed build | `store_only.rs:68-77` |

**W-4 holds on the merged tree.**
- The ports have exactly four default-armed methods: `HerdrPort::observe_pane` and `observe_panes`, and
  `HarnessPort::send_prompt` and `session_activity`.
- Every wrapper overrides every method of its port:
  - `BindingHerdr` and `LazyHerdr`: 9 of 9;
  - `BindingHost` and the `Wired` host: 4 of 4;
  - the `Wired` harness: 11 of 11.
- The adapter's `observe_panes` reads one `session.snapshot` (`holler-adapter-herdr/src/adapter.rs:431-447`). The
  production chain now reaches it.

**No ADR change is owed.** I agree with F's design decision 17.
- Section 14 already says "A wrapper port must delegate it" (`ADR-0021.md:974`).
- "Wiring as built (#649)" lists no port methods.

**`Unwired` overrides `observe_pane` but not `observe_panes`.** This is #832's code, merged from `main`.
- `Unwired` is not a wrapper.
- Its default asks `observe_pane` for each pane. That answers `not-implemented`, so every pane gets an empty observation.
- It is not this story's to change.

## Quality audit

- **Correctness and failure handling.**
  - **One build path.** `LazyHerdr`'s two new methods go through `port()`. So the build runs once, and a failure is kept
    and never retried.
  - **The operator still sees a failed build's error.** In a wired run, `unmanaged::observe` calls `snapshot` before
    `observe_panes` (`holler-pane/src/unmanaged.rs:89-92`, `:104-108`). `snapshot` answers the error, and `pane list`
    reports it as `unmanaged_unavailable`.
  - **The bindings are untouched.** `BindingHerdr`'s observe methods do not touch them. Only `ensure_pane` offers to the
    bindings.
  - The rest is unchanged since the second pass, which found no lost-write or race defect.
- **Build guards.**
  - The added `src` lines have no `unwrap`, `expect`, `panic!`, `unreachable!`, print or `unsafe`.
  - The six added `#[allow]`s are all in tests, and each carries `// #649`. This cycle added none.
  - Every touched `.rs` file is under 900 lines:
    - the largest is `tests/support/mod.rs` at 857;
    - `src/pane/wiring/bind.rs` is 439;
    - `pane_integration/bind.rs` is 580;
    - `observe.rs` is 221;
    - `scenario.rs` is 394.
  - **Dead code.** The workspace denies it, and clippy is clean.
- **Formatting.**
  - The production wiring files are rustfmt-clean, except #832's import line in `wiring.rs`, which `main` has too.
  - Several of the story's own test files have rustfmt drift (advisory 6).
  - No CI step, hook or `lint.sh` check enforces rustfmt.
- **Protocol.** No change: no new method, field, error code or golden file. No `docs/protocol/v2.md` edit is due.
- **Tests.**
  - **Tiers.** Cross-process behaviour runs against real hub subprocesses. D33's forwarding is in-process behaviour, so
    its rows are in-process over a recording port.
  - **Why `Observing` exists.** `FakeHerdr` overrides only `observe_pane`, and `HerdrOp` has no `ObservePanes`. So the
    fake's own log cannot tell one `observe_panes` from N `observe_pane`s (A-dup, Checked and clean).
  - **No fixed sleep is used to synchronize.** The fake hub's sleeps play a slow hub, and the rigs poll to a deadline.
  - The RED-first evidence is in `handoff-T-red.md`.
  - The target has 66 default-run tests and 4 opt-in ones. The only `#[ignore]`s are the four in `scenario.rs`.
- **Documentation.**
  - **The CHANGELOG.** The `## [Unreleased]` entry links #649, #633, #825 and #828.
  - **Its new clause is accurate.** `connect` reads every variable at once (`wiring.rs:108-115`, `:132-160`), and a bad
    value is answered at its port's first use.
  - **`bind.rs`'s docs.** The module and type docs name the four default-armed methods and `LazyHerdr`'s one exception.
  - **The ADR.** "Wiring as built (#649)" is unchanged. For its #828 and #654 pointers, see Notes for O 2.
- **Public-repository privacy.**
  - I grepped the whole branch diff: 9,393 added lines. I looked for IPs, home paths, hostnames, tailnet names, account
    names, e-mail addresses and key shapes.
  - The only hits are:
    - `127.0.0.1`;
    - the generic `/home/u` in the `env` and guard rows;
    - a scratch-relative `h649.ab/home/.config` in `guards.rs`;
    - the second pass's own description of this check.
  - Clean.
- **Commit hygiene.**
  - Since the second pass there are seven commits. All have Conventional subjects (`chore(#649): …`) and a
    `Co-Authored-By` trailer.
  - None carries a session link. Neither does any other commit of this run, nor `main`'s recent squash commits.
  - The PR's own items are in Notes for O 3.
- **CI readiness.**
  1. **The rename scan passes:** no hit.
  2. **The branch conflicts with `main` in `CHANGELOG.md`** (#838). A PR in this state gets no CI:
     - the PR phase runs only `git push -u origin <branch>` (`coding-pipeline.workflow.mjs:4168-4177`, `:4765`);
     - `ci.yml` runs on `pull_request` and on pushes to `main` only (`ci.yml:3-9`);
     - GitHub runs no `pull_request` workflow on a PR that has a merge conflict.

     See Notes for O 1.
  3. **#838 needs no code fix.**
     - It touches no file of this branch and no `holler-cli/src` file.
     - Its new test, `wizard/skilltext_838.rs`, reads only `agent-skills/herdr-setup/SKILL.md` and
       `docs/setup-wizard.md` (I read it).
     - `main`'s CI passed at `853c1e1`.

## Scope check

- **Delivered:**
  - D33 in production (`bind.rs`, +35);
  - the tests D33 names;
  - one extra recording row over the production chain. It is T's, and it catches a fix applied to only one link;
  - the second pass's advisory 4, as a scenario check.
- **Beyond D33:** one clause of this story's own CHANGELOG entry. That was the second pass's advisory 6 (F's design
  decision 20).
- **The Blast radius.** Every changed file is inside it: 28 files outside `docs/handoffs/` (27 at the second pass, plus
  `observe.rs`). I checked the list against the brief's Blast radius patterns.
- **Nothing is under-delivered.**

## Verdict

**PASS.**
- All 15 acceptance criteria and D33 are met on the tree that includes #832.
- D1 to D33 are implemented as stated, and the quality audit finds nothing blocking.
- The second pass's two defects are closed:
  - **Defect 1:** the forwards are in place, with all three pins plus a check over the production chain. Case 13 is
    green over both wrappers.
  - **Defect 2:** E64's Source and A's journal entry name the `herdr-setup` directory, and the scan passes.
- **Two things the operator should know, neither a hold:**
  - **#838 has to be merged** before CI can run. This is PR-phase work: `CHANGELOG.md` only, and no code to re-gate.
  - **#828 is closed** (Notes for O 2). D17 and D18 (c) name it as the owner of the `reset --first` bypass, and F wrote
    that into the CHANGELOG and the ADR, as the brief says. But at 15:56 MDT the operator closed #828 as not planned and
    folded it into #839, which also takes DRIVEN from #654.
    - #839's body records the merge order: "Prefer #649 first; this story then supersedes its DRIVEN wording". Its work
      item 6 rewrites these ADR sentences.
    - So landing the text as written is the operator's own plan, and the guard it states ("do not use `reset --first` on a
      parked or held pane") is still the right one.
    - Holding the run for a pointer that #839 owns would cost a full gate cycle and buy nothing.

## Notes for O

1. **Merge `origin/main` (`853c1e1`, #838) into the branch, so that CI can run.**
   - In `CHANGELOG.md`, keep all three entries at the top of `## [Unreleased]` / `### Enhancements`: #838, #649 and #832.
   - Do not run a workspace `cargo fmt` in the merge (this run's lesson).
   - No re-gate is owed: #838 changes no file this story touches, and no Rust source.
   - Per `CLAUDE.md`, this is the run's own agent's work after the PR opens, before it checks CI.
2. **#828 is closed. Tell #839 which of this story's lines to replace.**
   - **Lines that name #828 as the owner of closing `reset --first`'s bypass:**
     - `CHANGELOG.md:26-28` ("until #828 lands, do not use `reset --first` on a parked or held pane");
     - `docs/adr/ADR-0021.md:367`, `:582` and `:1043`.
   - **Lines that give DRIVEN to #654:**
     - `ADR-0021.md:369-370`, `:580`, `:883`, `:935` and `:1035`.
   - **What #839 already covers, and what it does not.** Its item 6 names the ADR sentences, and its Depends-on names
     "Wiring as built (#649)". Nothing in #839 names the CHANGELOG clause.
   - **The fix.** A comment on #839 that lists these lines is enough. The release's hand-written CHANGELOG pass is a
     second chance (`docs/releasing.md`, step 7).
   - **The PR body** should say that #839 now owns both the bypass and DRIVEN.
   - The carried follow-up "#653 and #828: the automated `pfleet o reset` caller" now reads "#653 and #839".
3. **For the PR.**
   - **The AI disclosure.** The body needs `CONTRIBUTING.md`'s disclosure, added with `gh pr edit`.
   - **The squash title** must be a Conventional Commit subject. The issue's title is one (`test(pane): …`).
     - `feat(pane): …` would describe the change better: it makes every pane and profile verb live.
   - **What the merge of #832 makes visible.** Say this in the body. With no Herdr configured, an unscoped `pane list`
     still exits 0 with the registry:
     - in JSON it adds `"unmanaged_unavailable": "usage"`;
     - in text it prints `warning: unmanaged panes not listed`, naming `HOLLER_HERDR_SESSION`.
4. **Still unfiled.** I checked with `gh issue list --search` at about 17:10 MDT.
   - **The test-glue consolidation (W-O).** There are three `LinkedHost`s. It can absorb A-dup's W-1 to W-4, W-6 and W-7.
   - **T's two product findings:**
     - the OpenCode adapter's TUI query fails without a UTF-8 locale;
     - a `pane switch` right after `pane relaunch` times out until the TUI accepts its select-session call.
   - **The workspace flake** `remote_admin_traffic_never_moves_the_bodys_last_seen` (#508's test), from T-green.
   - **The wizard's rename scan could skip `docs/handoffs/`,** as `docs_cli_test` does (#462).
   - **For Aftersight:**
     - W-U. It applies to a production S REWORK too: F gets no finding;
     - F's two diff-gate input gaps: the 12,000-byte evidence cap, and no `--handoff`;
     - `pipeline-conventions.md` section 1 still says Phase 11 removes the handoffs, which has been false since #558.
   - **Carried from A:**
     - comment on #825 that `profile apply` logs as `holler pane`. #825 still has no comments;
     - #653 (see item 2);
     - #803 still does not list #649.
   - **From A-dup's Notes for O 3,** for #652 or #654: #838's wizard writes `[instance] herdr_session`, but the wiring
     reads `HOLLER_HERDR_SESSION`.

## Advisory notes

1. **D28's sentinel cannot reach the code under test,** given D10's exact environments. AC 15 holds by construction. The
   earlier passes suggested that #829 plant the sentinel in the private tmux server's global environment.
2. **Under a real hang, the read timeouts name only `pane_store.get` and `pane_store.watch_next`.**
   - The other read `op`s are constants, which I checked by grep against the test kit's names.
   - Each extra pin would cost a 10 s wait.
3. **Scenario step 7 asserts that `doctor` finds something for C1** (`scenario.rs:228-231`), but not which finding.
   Asserting the kind would pin what the delete caused.
4. **AC 8's "`pane list` equals Herdr's snapshot" is still checked through `pane get` of C1 and C2.**
   - The unscoped `pane list` is now read too, but only for the unmanaged table.
   - Its `panes` rows are not compared with the snapshot. The data is the same records.
5. **No opt-in check shows D33 against a real Herdr.** No unmanaged pane's `cwd` is checked through the real chain.
   The in-process chain row (`observe.rs:186-192`) pins the forwarding.
6. **Several of the story's own test files have rustfmt drift**, counted with `rustfmt --edition 2021 --check` per file:
   - `scenario.rs` 23 hunks, `fake_hub.rs` 19, `bind.rs` 15 and `env.rs` 13;
   - `scratch/mod.rs` 7 and `guards.rs` 6;
   - `wired.rs`, `store_only.rs` and `no_hub.rs` 4 each;
   - `watch.rs` and `main.rs` 1 each.

   The brief's Rules list `cargo fmt`, but this repo does not enforce it, and half of the existing test files drift too.
   - The fix is to format file by file. Not the workspace, and not `main.rs` as a root, because its `#[path]` children are
     other stories' files.
   - It fits the test-glue follow-up.
7. **A-dup's W-1 to W-4, W-6 and W-7** are follow-up material. `pane_integration/main.rs`'s module doc does not list
   `observe`.
8. **The operator needs a page for the six variables.** D17 leaves `docs/pane-control.md` to #652, which should carry
   the table.
9. **A non-UTF-8 `HOLLER_HERDR_WORKSPACES` reads as no workspace** (F's design decision 9).
   - It fails closed: every launch is refused `unavailable`.
   - The refusal names the workspace, not the variable.
10. **The brief's Evidence section has no entry for #832's source.**
    - D33 states the two signatures inline, and T-red's section of `evidence.md` quotes them, together with ADR section 14
      and case 13.
    - The issue's Evidence index still gives E64's Source as the setup wizard's pre-#820 directory. The brief has the new
      one. The issue is not a tracked file, so CI does not see it.
