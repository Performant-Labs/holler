# Handoff-S: Phase 10 - #649 hub wiring and the scratch integration scenario  (spec audit, second pass)

**Date:** 2026-10-11 (16:21 MDT)
**Branch:** `issue-649-implementation` (worktree `.claude/worktrees/0649-live-harness`, head `c001eeb`, merge base `5d366b9`,
#650).
`origin/main` is `652841b` (#832, merged 15:35 MDT); `git ls-remote` agrees. A trial merge
(`git merge-tree --write-tree origin/main HEAD`, tree `2e527e2`) conflicts in `CHANGELOG.md` only.
**Issue:** #649 (epic #633), read live with `gh issue view 649` (last edited 12:16 MDT).
- It still differs from the brief only by D10 (5)'s `LANG=C.UTF-8` clause (`c13400a`).
- It names neither #832 nor `observe_pane`.
**Brief:** `docs/handoffs/649-brief.md` at `c13400a`, unchanged since the first pass.
**Handoffs reviewed (under `docs/handoffs/649/`):**
- `handoff-A.md` (fifth pass, `2280399`) and `handoff-A-dup.md` (second pass, `c001eeb`);
- `handoff-T-red.md` (second cycle, `ce5da11`);
- `handoff-F.md` (all three cycles);
- `handoff-T-green.md` (fifth pass, `98280ad`: S's test-only rework);
- the first S pass (`d387625`), which this handoff replaces. Git history keeps it;
- `decisions.md` and `evidence.md`.

**What changed since the first pass.**
- In `crates/`, one file: `crates/holler-cli/tests/pane_integration/fake_hub.rs` (T's fifth pass).
- No production line, ADR, CHANGELOG or manifest changed.
- `origin/main` moved: #832 merged at 15:35 MDT.

**Verdict:** ADVISORY-HOLD.
- F's code and T's tests meet the brief on the branch's base, and the first pass's REWORK is closed.
- The branch cannot land as the brief leaves it, and each fix is the operator's, not F's or T's:
  - the merge of `main` that the PR needs breaks D31 W-4 in two wrappers (A-dup's W-5);
  - the brief file itself fails CI's rename scan.
- The details are under Verdict.

## A precondition

Met.
- `handoff-A.md` (fifth pass): **PASS**.
- `handoff-A-dup.md` (second pass, diff `c13400a..98280ad`): **PASS**, no block.
  - W-5 binds at the merge with `origin/main`. W-6 is a nit.
  - W-1 to W-4 and W-O carry over as follow-up material.

## T precondition

Met.
- **Verdict.** `handoff-T-green.md` (fifth pass) says "Blocking issues: None in F's code."
- **RED, then GREEN, for the rework.**
  - Each of two mutants fails the new assertions when its behavior is removed:
    - M9: `NoLiveHub` added to `timed_out`'s write arm;
    - M10: `hub_unavailable`'s second arm with the hub dropped.
  - Then `cargo test -p holler-cli --test pane_integration fake_hub`: 15 passed.
  - Clippy `-D warnings` (workspace, all targets), `lint.sh`, `changelog-check` and `machete` are clean.
- **The workspace run was still running when T returned** (57 of 151 binaries, 0 failures).
  - The rework is one file of one test binary, and that binary compiled and passed its changed module.
  - So the fourth pass's result stands: 2312 passed, 1 failed. The failure is the rename scan (Quality audit, CI
    readiness 3).
- **Opt-in scenario:** 4 of 4 at `22aa539` (82.48 s).
  - No scenario or production code has changed since, on this branch.
  - It has never run on a tree that includes #832.

## Acceptance criteria

- Paths are under `crates/holler-cli/tests/pane_integration/` unless given in full.
- Each status is for the branch on its base, `5d366b9`. The Verdict covers the tree that would land.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | No hub: every `pane` and `profile` verb exits 1 `unavailable`, naming the missing hub, and touches no adapter (D28). The process stub tests run against a started hub (D22) | **No hub.** `no_hub::every_verb_with_no_hub_is_unavailable_naming_the_hub_in_json` and `…_in_text` run all 20 verbs of `EVERY_VERB`. Each run checks:<br>- one envelope, exit 1, `unavailable`;<br>- "no live holler hub reachable" and the state dir in the message;<br>- untouched traps: a held Herdr listener, a tmux socket path, an `opencode` that leaves a mark, and an empty `PATH`.<br>**D22.** `pane_verbs/process/stub.rs`'s three stub tests run through `LiveHub`, their assertions unchanged | Met |
| 2 | `pane list`, `get` and `watch` and `profile list` work with no Herdr, tmux or OpenCode (D2) | `store_only::the_store_only_verbs_work_with_no_herdr_tmux_or_opencode`: an empty `PATH`, no adapter setting, `profile show` too. Also `env::malformed_adapter_settings_do_not_stop_the_store_only_verbs` | Met |
| 3 | A read the hub took and never answered is `timeout` naming the store method, within 10 s. Every other read failure is `unavailable` naming the hub. A JSON-RPC error is `unavailable` for reads and writes | **Timeout:**<br>- `fake_hub::a_read_the_hub_never_answers_is_timeout_within_the_bound` (`pane_store.get`, under 13 s);<br>- `a_watch_request_the_hub_never_answers_is_timeout_naming_watch_next`;<br>- from below, `a_read_and_a_write_answered_after_6_s_succeed`.<br>**Unavailable, naming the hub by its state dir:**<br>- `a_read_whose_reply_is_lost_is_unavailable_naming_the_hub` (`BadReply`, five reads);<br>- `a_json_rpc_error_is_unavailable_naming_the_hub_for_reads_and_writes` (`Refused`, ten methods);<br>- `every_read_and_write_with_no_hub_is_unavailable_naming_the_hub` (`NoLiveHub`, ten methods) | **Met.** It was partly met at the first pass. Its REWORK items 1 and 2 are closed |
| 4 | A malformed `HOLLER_TMUX_SOCKET` is `usage` naming it, from the host and the harness, never the default socket (D6) | `wired::a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server` and `wired::a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server`. Both run through `Wiring::connect` against a real hub, with a `TMUX_TMPDIR` trap. The reader's own row is `env::a_relative_tmux_socket_is_usage_never_the_default_server` | Met |
| 5 | A write whose reply is lost is `timeout` (D3) | `fake_hub::a_write_whose_reply_is_lost_is_timeout`: all five writes, each naming its `op`. Also `a_write_the_hub_never_answers_is_timeout_within_the_bound`. The converse, a write with no hub is `unavailable`, is now pinned too (row 3) | Met |
| 6 | The scenario passes on scratch instances only. Nothing outside them is touched, real names are refused, and the guards are tested in the default run | The recorded opt-in 4 of 4. `guards::*` has 7 tests:<br>- the gate on exactly `1`;<br>- the session, with `default` refused;<br>- pane names;<br>- paths inside the base;<br>- the binary's exact environment, `LANG` included;<br>- its refusals;<br>- the tmux server's exact environment | Met (recorded run; not on #832) |
| 7 | The scenario's order, then the repairs of D21 | `scenario::the_scenario_runs_in_order_on_scratch_instances`: two launches, relaunch, reset `--first`, switch, park, killed server and deleted session with one `doctor` finding both, hub restart with `pane list` unchanged, then `reset C1`, `unpark C2`, `relaunch C2` and the stray deletes | Met (advisory 3) |
| 8 | No step types into a TUI. At the end, `doctor` reports no finding and the registry equals Herdr's snapshot | No `send_text`, `send_keys` or `send-keys` in `scenario.rs` or `scratch/mod.rs`. `assert_end_state` checks that `doctor` reports no finding, and compares each record's `(pane_id, grid)` with Herdr's snapshot | Met (advisory 4) |
| 9 | The profile scenario | `scenario::the_profile_scenario_runs_on_scratch_instances` | Met |
| 10 | Each grid launch sits at row 2, column 1 in Herdr's own snapshot, in a workspace whose r1c1 a Holler launch made first (D23) | `scenario::the_two_grid_spellings_land_at_row_2_column_1_in_herdrs_snapshot` | Met |
| 11 | Every `--format=json` output parses against the envelope, each `watch` line too | `Scratch::json` runs #638's `check_envelope` (`scratch/mod.rs:358-362`). The scenario checks each line of its final `pane watch`. In the default run: `watch::pane_watch_prints_the_put_of_a_client_write_each_line_an_envelope` | Met |
| 12 | The scenario's launch resolves `attach_tui` before the record exists, pinned by a test | Indirect in the scenario: launch writes no record before `attach_tui` and no label (E26). The direct pin is row 14 | Met |
| 13 | The server argv runs under a real tmux pty (D15) | `scenario::the_server_argv_runs_under_a_real_tmux_pty_and_stops`, with the default spec and an agent spec | Met |
| 14 | `attach_tui` resolves before the record exists, with the decorators over fakes, in the default run (D8, D24) | `bind::launch_resolves_attach_tuis_pane_from_the_bindings_before_the_record_exists`. Supporting:<br>- the binding-order rows;<br>- `bind::the_binding_herdr_passes_the_herdr_conformance_suite`;<br>- `bind::the_binding_host_passes_the_host_conformance_suite` | Met. **On the merged tree, the Herdr suite fails at #832's case 13** (Spec compliance) |
| 15 | No credential is printed, written to a file or put on an argv (D28) | By construction: the wiring reads only its `HOLLER_*` table, `HERDR_SOCKET_PATH`, `XDG_CONFIG_HOME`, `HOME` and `PATH`, and the harness runs with `ProcessEnv::Inherit`. Guards 4 and 5 pin both environments. `Scratch::assert_no_credential_leaked` runs at `scenario.rs:138`, `:187` and `:304` | Met (advisory 1) |

The tests assert outcomes, not the implementation: codes, messages, call logs, files, sockets, processes and Herdr's own
snapshot. Mutants M1 to M10, the 5 s pin and the `move` check each fail when their behavior is removed.

## Spec compliance

**On the branch's base, D1 to D32 are implemented as stated.**
- No production line has changed since the first pass, which checked each decision.
- I re-read `wiring.rs`, `wiring/bind.rs` and `wiring/store.rs`, and re-ran the guard greps.
- The deviations A reviewed stand:
  - the `Effect` enum;
  - `stores()` returning trait objects;
  - `Wired<P>`;
  - `env::Lookup`.

**The first pass's one gap is closed.** D31 W-9 says "T pins each row of D3's mapping". The table in `fake_hub.rs`'s module
doc (lines 10 to 18) is now true as written:

| What the exchange did | A read | A write |
|---|---|---|
| no socket (`NoLiveHub`) | `every_read_and_write_with_no_hub_is_unavailable_naming_the_hub` | the same test |
| a JSON-RPC error (`Refused`) | `a_json_rpc_error_is_unavailable_naming_the_hub_for_reads_and_writes` | the same test |
| closed before the reply (`BadReply`) | `a_read_whose_reply_is_lost_is_unavailable_naming_the_hub` | `a_write_whose_reply_is_lost_is_timeout` |
| no reply within the bound (`Io`) | `a_read_the_hub_never_answers_…` (`pane_store.get`) and `a_watch_request_…` (`pane_store.watch_next`) | `a_write_the_hub_never_answers_…` (`pane_store.cas_put`) |
| a `data` that does not decode | `data_that_does_not_decode_is_unavailable_naming_the_method` | none: the row is for reads only |

**Against `main`, D31 W-4 does not survive the merge.** This is A-dup's W-5, which I checked against the code.
- `HerdrPort` on `main` has two new methods with default bodies, `observe_pane` and `observe_panes`
  (`crates/holler-pane/src/ports.rs:196-220`, #832).
  - ADR-0021 section 14 on `main` says: "A wrapper port must delegate it."
  - #832 changed no other port.
- **The trial merge.** In tree `2e527e2`:
  - `BindingHerdr` (`bind.rs:97-128`) and `LazyHerdr` (`bind.rs:201-229`) implement the seven methods of the brief's
    base, and neither new one;
  - their own docs say they forward every method (`bind.rs:11`, `:84`);
  - `main`'s `Unwired`, by contrast, overrides `observe_pane`.
- **The production path.** The chain is `BindingHerdr` over `LazyHerdr` over `HerdrAdapter` (`wiring.rs:141-152`).
  - On `main`, `unmanaged::observe` calls `ports.herdr.observe_panes` (`crates/holler-pane/src/unmanaged.rs:108`).
  - Through the wrappers it gets the trait default, an empty observation per pane.
  - It never reaches the adapter's own two methods (`crates/holler-adapter-herdr/src/adapter.rs:415`, `:431` on `main`).
- **Effect.** Under the real wiring, `pane list` and `pane get` would show no working directory for any unmanaged pane.
  - That contradicts #832's CHANGELOG entry.
  - #832's view only goes live in production when #649 lands, because `main`'s wiring is still the `Unwired` stub.
- **Not F's deviation.** On the brief's base, both wrappers forward every method of the port. The brief was cut from a
  base that `main` has since moved past (Verdict, defect 1).

## Quality audit

- **Correctness and failure handling.** Unchanged since the first pass, which found no lost-write or race defect:
  - each port fails closed on a bad configuration;
  - a lost write reply is `timeout` and is never retried;
  - `LazyHerdr` keeps a failed build;
  - the bindings tolerate a poisoned lock.
- **The rework's tests.**
  - **One runner.** `against` runs over two tables, `reads()` and `writes()`. Each call gets a fresh fake hub on its
    own temporary state dir, and the runner still checks that the client sent its method.
  - **`Outcome::assert_names_the_hub`.** It requires the text `holler hub` and the fake hub's own dir. The dir is a fresh
    path, so neither the transport's text nor the fake's refusal text can supply it.
  - **The no-hub row.** It uses a fresh `StateDir` per call and requires `no live holler hub reachable at <dir>`.
  - **Parallel safety.** Every in-process call runs under `with_state_dir`'s lock (`main.rs:50-66`), so parallel
    tests cannot race on `HOLLER_STATE_DIR`.
  - **No captured state.** `type Call` holds `fn` pointers, so a row captures nothing.
  - **Nit.** The no-hub text is spelled three ways in the target (A-dup W-6).
- **Build guards.**
  - The added `src` lines have no `unwrap`, `expect`, `panic!`, `unreachable!`, print or `unsafe`.
  - The six added `#[allow]`s are all in tests, and each carries `// #649`.
  - Every touched `.rs` file is under 900 lines:
    - `tests/support/mod.rs` is 857 (+8, only `StateDir::at`, as D22 and W-I allow);
    - `pane_integration/bind.rs` is 579;
    - `fake_hub.rs` is 454.
- **Protocol.** No change: no new method, field, error code or golden file. No `docs/protocol/v2.md` edit is due.
- **Tests.**
  - Cross-process behavior runs against real hub subprocesses, and the fake hub is used only for the failure rows
    (D13).
  - No fixed sleep is used to synchronize. The fake's sleeps play a slow hub, and the scenario polls.
  - The RED-first evidence is in T's handoffs.
  - The target has 61 default-run tests and 4 opt-in ones.
- **Documentation.**
  - The CHANGELOG `## [Unreleased]` entry links #649, #633, #825 and #828.
  - ADR-0021 has "Wiring as built (#649)".
  - `wiring.rs`'s module doc tables the variables.
  - See advisories 6 and 7.
- **Public-repository privacy.**
  - I grepped the whole branch diff, and separately the lines added since the first pass. I looked for IPs, hostnames,
    tailnet names, home paths, account names, e-mail addresses and key shapes.
  - The only hits are the generic `/home/u` in the `env` and guard rows, `127.0.0.1`, and the first pass's own
    description of the check. Clean.
- **Commit hygiene.**
  - The three commits since the first pass have `chore(#649): …` subjects and the `Co-Authored-By` trailer, as the
    rest of the run does.
  - The squash title and the PR body's AI disclosure belong to the PR phase (Notes for O).
- **CI readiness: the branch cannot merge as it stands.**
  1. **The PR would conflict with `main` in `CHANGELOG.md`.** Both sides add the top entry of `### Enhancements`.
     - The PR phase pushes the branch as it is; it does not merge `main` (`coding-pipeline.workflow.mjs:4700-4800`).
     - GitHub runs no `pull_request` workflow on a PR with a merge conflict, so CI (`ci.yml`) would first run after
       `main` is merged in.
  2. **On the merged tree, CI fails `bind::the_binding_herdr_passes_the_herdr_conformance_suite`.**
     - #832's case 13 (`crates/holler-pane-testkit/src/conformance/herdr.rs:424-447` on `main`) wants `pane-not-found`
       from `observe_pane` of an unknown id. `BindingHerdr` answers `Ok` from the default.
     - No test runs over `LazyHerdr`'s or `observe_panes`' forwards. Its every-method test
       (`bind::a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method`, `bind.rs:134-159`) lists the
       seven methods by hand.
     - So a fix to `BindingHerdr` alone would pass CI and still mask the adapter.
  3. **CI's `wizard_scripts` rename scan fails on two tracked lines.** Both cite the setup wizard's skill directory as it
     was before #820:
     - `docs/handoffs/649-brief.md:1197` (E64's Source);
     - `docs/handoffs/649/decisions.md:66` (A's first entry).

     Details:
     - The script is now `agent-skills/herdr-setup/lib/herdr.sh`, and E64's excerpt is at the same lines, 8 to 11.
     - T rewrote its own hit.
     - The release keeps the handoffs unless the run passes `removeHandoffsBeforePr: true`
       (`coding-pipeline.workflow.mjs:4753`).
     - `main` keeps every story's handoffs (`docs/handoffs/650`, `664`, `802`, `808`).

## Scope check

- **Delivered.**
  - F delivered the brief's scope, and T delivered its tests.
  - Every changed file is inside the Blast radius (27 files outside `docs/handoffs/`).
- **The extras are the ones A reviewed:**
  - `Wired<P>`;
  - F's environment edges;
  - the ADR's section-2 pointer.
- **Nothing is under-delivered** on the brief's base.
- **W-5's work is outside the brief.** It is two forwards in each wrapper, plus their pins. The brief predates #832, so
  it does not name them.

## Verdict

**ADVISORY-HOLD.**
- This is not a REWORK of F's or T's work. On the brief's base, the implementation is complete and spec-compliant.
- The hold is for two defects in the source of truth, which only the operator can fix.

**Defect 1. The brief is cut from a base that `main` has moved past, and the merge the PR needs breaks its D31 W-4.**
- **The defect.**
  - The brief's base is `5d366b9`. Its Depends-on stops at #644, and its evidence quotes `HerdrPort` before #832.
  - #832 (merged 15:35 MDT) gave `HerdrPort` two default-armed methods.
  - It also made delegating them a rule for wrapper ports (ADR-0021 section 14).
- **The convention violated:** D31 W-4 ("Every delegating impl overrides every method of its port") and ADR-0021 section
  14 on `main`. A story is built against the base it lands on.
- **Why F cannot fix it inside the pipeline:**
  - the fix does not compile before the merge, because the methods do not exist on the branch;
  - F's role has no merge step, and F stages only production files it wrote;
  - this run's three merges of `main` were the operator's (`3dff970`, `f59ba21`, `d0c1531`);
  - on an S REWORK, the driver gives F only "Perform Phase 6's work." (`coding-pipeline.workflow.mjs:5650`), with no
    finding.

**Defect 2. The brief file fails CI.**
- **The defect.**
  - E64's Source (`docs/handoffs/649-brief.md:1197`) cites the setup wizard's skill directory as it was before #820.
  - The brief is a tracked file, so it fails `wizard_scripts`'
    `skilltext_w5_rename::no_old_command_or_directory_outside_the_rename_note`, which CI's workspace run includes.
  - A's first journal entry carries the same citation (`decisions.md:66`).
- **The convention violated:**
  - #820's CI-enforced rule that the old skill path appears in no tracked file outside the rename note;
  - the brief's own rule that each Evidence entry cites its current source.
- **Neither F nor T may edit the brief or A's journal.**

**Why not PASS.**
- Phase 11 would push a branch that conflicts with `main`, so CI would not run.
- The merge after the PR would then add production code (W-5) outside the gates: T-red, F, T-green, the diff gate,
  A-dup and S.
- CI catches only half of that code's defect.

**Proposed fix (operator), in order:**
1. **Merge `origin/main` into the branch.**
   - In `CHANGELOG.md`, keep both entries at the top of `## [Unreleased]` / `### Enhancements`.
   - `ADR-0021.md` and `wiring.rs` auto-merge.
   - Do not run a workspace `cargo fmt` in the merge (this run's own lesson).
2. **Amend the brief, and the issue with it.**
   - (a) **A decision for #832**, as A-dup's W-5 gives it:
     - `BindingHerdr` and `LazyHerdr` override `observe_pane` and `observe_panes`;
     - `observe_pane` forwards. In `LazyHerdr` it is `self.port()?.observe_pane(pane)`;
     - `observe_panes` forwards to the inner port's own `observe_panes`, which keeps section 14's one call;
     - when `LazyHerdr`'s build has failed, `observe_panes` answers one empty observation per pane. That is the
       trait's contract for a failed call.

     The pins, for T:
     - the Herdr conformance suite runs over `LazyHerdr` too, so case 13 covers it;
     - `observe_pane` joins the method list of
       `bind::a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method`;
     - one recording check per wrapper: `observe_panes` reaches the inner `observe_panes` once, not `observe_pane` once
       per pane.
   - (b) #832 joins Depends on, with evidence re-cut from `main` for `ports.rs:196-220` and ADR-0021 section 14.
   - (c) E64's Source names the `herdr-setup` directory, at lines 8 to 11. The excerpt is unchanged.
   - (d) The issue body gets D10 (5)'s `LANG=C.UTF-8` clause, as the first pass asked.
   - (e) A nit: D17's "(#642 only)" for ADR line 511 becomes #829, as D18 (f) and the ADR already say.
3. **Correct the citation in `decisions.md:66` the same way.**
   - Record the correction in a new journal entry, since the journal is otherwise append-only.
   - Running the PR phase with `removeHandoffsBeforePr: true` would also clear the scan. But this repo keeps its
     handoffs on `main`, so I do not recommend it.
4. **Relaunch at phase 4**, as this run did after each earlier merge of `main`:
   - A reviews the amendment;
   - T-red pins W-5 (RED on the merged tree);
   - F adds the forwards;
   - T-green verifies and **re-runs the opt-in scenario on the merged tree**. #832 changes what `pane list`,
     `pane get` and `pane doctor` read;
   - the diff gate, A-dup and S then review the tree that lands.

## Notes for O

1. **For the PR.**
   - The PR body needs CONTRIBUTING's AI disclosure (`gh pr edit`).
   - The squash title must be a Conventional Commit subject. The issue's title already is one.
   - It should also say what the merge makes visible:
     - with no Herdr configured, an unscoped `pane list` still exits 0 with the registry;
     - in JSON it adds `"unmanaged_unavailable": "usage"`;
     - in text it prints a warning naming `HOLLER_HERDR_SESSION`.
   - `store_only.rs`'s assertions hold on the merged tree, by my reading (A-dup's Notes for O 1).
2. **Still unfiled.** Checked with `gh issue list --search` at 16:15 MDT:
   - **The test-glue consolidation (W-O).** There are three `LinkedHost`s. It can absorb A-dup's W-1 to W-3 and W-6.
   - **T's two product findings:**
     - the OpenCode adapter's TUI query fails without a UTF-8 locale;
     - a `pane switch` right after `pane relaunch` times out until the TUI accepts its select-session call.
   - **The wizard's rename scan could skip `docs/handoffs/`,** as `docs_cli_test` does (#462). Then defect 2 cannot
     recur.
   - **For Aftersight:**
     - W-U, which A noted for diff-gate BLOCKs. The same gap applies to a production S REWORK: F gets no finding;
     - F's two diff-gate input gaps: the 12,000-byte evidence cap, and no `--handoff`;
     - `pipeline-conventions.md` section 1 still says the handoffs are always removed, which is false since #558.
   - **Carried from A:**
     - the #825 comment;
     - #653 and #828 (`pfleet o reset`);
     - #803 does not list #649.

## Advisory notes

1. **D28's sentinel cannot reach the code under test**, given D10's exact environments. AC 15 holds by construction.
   The first pass recommended, for #829, planting the sentinel in the private tmux server's global environment.
2. **Read timeouts under a real hang name only `pane_store.get` and `pane_store.watch_next`.** The other read `op`s are
   constants, checked by grep against the test kit's names. Each extra pin would cost a 10 s wait.
3. **Scenario step 7 asserts that `doctor` finds something for C1** (`scenario.rs:228-231`), not which finding.
   Asserting the kind would pin what the delete caused.
4. **The end state reads the registry with `pane get` of C1 and C2**, not with `pane list` as AC 8 words it.
   - After the merge, an unscoped `pane list` also carries #832's unmanaged table.
   - So the re-run on the merged tree should check that the scenario's own panes never appear in it.
5. **A-dup's W-1 to W-4 and W-6** are follow-up material.
6. **`CHANGELOG.md` says the variables are "each read when its adapter is first used".**
   - In fact `connect` reads them all at once, and a bad value is answered at the port's first use.
   - The effect described is right; the verb is not.
7. **The operator needs a page for the six variables.** D17 leaves `docs/pane-control.md` to #652, which should carry
   the table.
8. **A non-UTF-8 `HOLLER_HERDR_WORKSPACES` reads as no workspace** (F's Design decision 9).
   - It fails closed: every launch is refused `unavailable`.
   - The refusal names the workspace, not the variable.
