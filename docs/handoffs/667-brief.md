# [test(pane): profile apply on scratch Herdr, tmux and OpenCode (the real-adapter scenario for apply)](https://github.com/Performant-Labs/holler/issues/667)

Part of epic #633. This is the complete statement of story #667, written as one text against `main` at `bda1069`: its facts were read from that tree and its evidence was cut from it (see "Evidence"). It supersedes the issue text of 2026-10-08, which was written before #649 merged, where the two differ (the differences are the numbered decisions below). The story is **test code, test support, one CHANGELOG entry and one deleted line of `docs/pane-control.md`: it changes no production code and no behaviour.**

What it is: an opt-in scenario, scratch only, that runs the real `holler` binary's `profile apply` (and the `pane` and `profile` verbs around it) against a real Herdr, a private tmux server and a real OpenCode, all started by the test, and checks the live result in Herdr's own snapshot, tmux and the registry. It backs step 0 (the scratch rehearsal) of the operator story #666. The standing spec of apply is ADR-0021 section 8, "Apply as built (#664)" (E58), and the operator guide `docs/pane-control.md` section 6 with its apply reference (E55); the scenario asserts what the guide says, and each row of its table is cited below where a step uses it. It is the only test that runs apply against real adapters; the default run already pins the whole row matrix with fakes (E61, E62, E63), so the scenario asserts only what a fake cannot: that a healthy pane's processes survive an apply, that a missing pane lands at its cell in Herdr, that the probe is a real process run before anything live changes, that the hub's registry takes the writes, and that the binary prints and exits as the guide says.

## Where this brief is the decision

1. **The scenario drives the real binary through a real hub, not an in-process harness.** The issue says to build an in-process harness from the real adapters and the test kit's in-memory `PaneStore` and `ProfileStore`; that was the only way before #649, and it is not the right one now.
   - The wiring exists and builds exactly that set from the environment (E29), but its constructor `from_env` is private. An in-process harness would assemble the ports by hand, as the in-process verb harness does for fakes (E27; E26 says it "never builds the real wiring"). That is a second assembly of the decorators, the bindings and the `tui_session` resolver that production owns (E76, E29), and it drifts the first time the wiring changes.
   - The refusal of real names is a property of the child's environment: guard 4 builds the binary's whole environment from nothing and refuses a session or a path outside the scratch directory before every verb (E9, E14). An in-process run inherits the developer's `HERDR_*`, `TMUX` and `HOLLER_*` and the wiring reads `std::env` (E28), so it would need the global-environment lock (E5) and would weaken the guard from "cannot reach a real session" to "is told not to".
   - What the guide says of apply is said of the binary: one envelope on stdout, the text form, exit codes 0, 1, 2 and 3 (E45, E47, E48, E54). Only a subprocess sees them.
   - The in-memory stores are already proved equal to the hub's by the conformance suites #649 runs over the hub clients (E5, module `clients`), and the hub's compare-and-swap is what `adopt` and `--take-over` write through (E34). Skipping the hub gains nothing.
   - **Consequences for the steps.** Every step is `Scratch::json(args, exit_code)` or `Scratch::holler(args)` of the shared scratch set, so each `--format=json` output is checked against `check_envelope` as it is read (E11, E53). The registries are the hub's; the test builds no store. Each test starts its own hub, tmux server, Herdr and OpenCode data directory, and the tests run with `--test-threads=1` because the scratch set plants a credential sentinel in the process environment (E10). The issue's "in-memory PaneStore and ProfileStore (#638)" is dropped, and with it the issue's sentence that the story "does not wait for the hub wiring".
   - *Rejected:* (a) the issue's in-process harness over in-memory stores (the second assembly above); (b) in-process over the hub's clients (still the private wiring, and the environment problem); (c) both, one per layer (two paths to keep equal).

2. **Reuse #649's scratch set whole; build only what is specific to apply.** The architecture gate of #649 already flagged the near-copies its rigs grew (E67), so #667 adds none.
   - *Reused unchanged:* the scratch Herdr guards and server (E12, included through `scratch/mod.rs`, E8); OpenCode's dead-end config, isolated environment, `free_port`, `refused`, the model guard and `raw` (E23, E24, E25); the private tmux server; the five guards and their default-run tests, `guards.rs` unchanged (E13, E14, E7); `Scratch` (`start`, `holler`, `json`, `launch`, `herdr_snapshot`, `harness`, `owned_pids`, `assert_no_credential_leaked`, and its `Drop` that kills what it started; E10, E11); `Hub`, `StateDir::at` (E22, E21); `check_envelope` (E53).
   - *Moved once, not copied (decision 3).*
   - *Not repeated:* #649's profile scenario already runs `create --from-current`, `launch --profile`, `close --profile`, `show`, `list` and `delete --keep-panes` on real adapters (E20); step 1 below uses `create --from-current` only as set-up.
   - *Built new:* the scenario's steps, a small reader of apply's `data.steps`, the "nothing changed" comparison, the probe test, and the default-run pins of decision 4.
   - **How it is included.** `profile_apply_scenario/main.rs` declares `#[path = "../support/mod.rs"] mod support;` and `#[path = "../pane_integration/scratch/mod.rs"] mod scratch;`, as #649 itself includes its three adapter-test modules; the inner `#[path]` attributes of `scratch/mod.rs` are relative to that file (E8). *Unverified (not built here): that the doubly nested include compiles; T proves it first.*
   - *Rejected:* (a) a second scratch module for #667 (the parallel path); (b) generalising the scratch set into a parameterised kit with its own prefixes (edits the guards and `guards.rs` and renames everything #649 merged, for a name); (c) making the scenario a module of the `pane_integration` target (the `[[test]]` placeholder for #667 already exists, E3, so the target costs no manifest edit, and a second target gives the operator a command that runs only the apply rehearsal; the 900-line gate also stays far away).

3. **The shared pieces #667 needs move out of #649's private spots, once, behaviour unchanged.** Today they are private functions of `scenario.rs` and a struct in `pane_integration/main.rs`, which a second target cannot call (E15, E16, E18, E19, E6). The moves, each a cut and paste with no change of assertion:
   - `Out` and `output_of` from `pane_integration/main.rs` to a new `pane_integration/out.rs`, which `main.rs` of both targets include; `scratch/mod.rs` needs them at its crate root (E8). (No third `Out`: `pane_verbs/process` has a second one already, a known duplication this story does not touch.)
   - Into `impl Scratch` in `scratch/mod.rs`: `record(pane)`, `session_of_record(pane)`, `findings()`, `assert_tmux_env_is_scratch()`, `kill_server(pane, port)` (the body of `break_both`'s server kill), and `assert_end_state(panes, workspaces)` (the parameters replace the constants `C1`, `C2` and `"hlr649"`); and `launch_args(...)`, the pure argv builder `Scratch::launch` already holds, so a spec-only `pane launch` uses the same flags. `scenario.rs` then calls them; its assertions do not change.
   - One added guard: `Scratch::holler` refuses any argument vector that carries `--herdr-session X` with X not the scratch session (a pure function in `scratch/mod.rs`, tested in `guards.rs`). It closes the one argument by which a profile verb could name a session; the adapter also refuses a name other than its configured session (E64 for the guards, E75 for the adapter).
   - Why move and not leave: `assert_end_state` is where a later story's change to what "a clean `doctor`" means (for instance #839) lands, and it should land once. Every move leaves #649's default run and its opt-in scenario passing unchanged; T runs both before and after.
   - *Rejected:* (a) leaving the private copies and writing equivalents in #667 (about 100 lines of near-copy, exactly what the architecture gate rejects); (b) a new shared crate or directory for the whole scratch set (a bigger move of merged files for no gain); (c) a prior refactor PR (the moves have no other reason to exist; one PR keeps the cause with the effect).

4. **Opt-in is #649's gate, unchanged: `HOLLER_TEST_PANE_INTEGRATION=1` exactly, and `#[ignore]` on every scratch test.** `Scratch::start` returns `None` unless the variable is exactly `1` (the pure `gate`, E12, E13), and every scenario test is `#[ignore = "opt-in: ...; HOLLER_TEST_PANE_INTEGRATION=1 (#667)"]` (E17), so `cargo test` skips it and shows it as ignored. Opted in, a missing `herdr`, `tmux` or `opencode` is a failure, not a skip (E23, E64); the probe tests also need `curl` on `PATH`, and its absence is a failure too (decision 9). The run line, in the module doc of `main.rs` and in the CHANGELOG entry:
   `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test profile_apply_scenario -- --ignored --test-threads=1`.
   - *Rejected:* (a) a variable of its own (a second knob and a second gate test for one decision; one variable opts in to the epic's scratch scenarios, and it only ever runs the target the operator names); (b) the variable alone with no `#[ignore]` (the tests would be listed as passing when they ran nothing); (c) `#[ignore]` alone (an `--ignored` run on a developer machine would then start real servers with no second confirmation).

5. **CI behaviour: compiled, never run.** (#649 split its tests the same way: a default run that needs at most a hub, and `#[ignore]` scenarios, E65, E5.) CI's test step is `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (E69, E70), which runs no `#[ignore]` test, and no workflow passes `--ignored` for this target. So the scenario is built on both CI platforms (the shared module already is, through `pane_integration`) and skipped. The target's own default run is the small pins A12 to A14 of Acceptance: pure, no hub, no process, no Herdr. No workflow changes. *Rejected:* a CI job that installs Herdr and OpenCode (out of scope, and #649 did not).

6. **Names stay #649's family.** The Herdr session is `holler649-<8 hex>`, every pane is `hlr649-...`, the workspaces are the four of `HOLLER_HERDR_WORKSPACES` in the scratch set, each 2 rows by 1 column (E8). #667's panes are `hlr649-ap1` to `hlr649-ap4`, and `hlr649-pz`, `hlr649-pw` and `hlr649-py`; its profiles are named `hlr649-apply-p`, `hlr649-apply-q` and `hlr649-probe`. The module doc says "649" is the scratch set's family name, the story that built it. *Rejected:* a `667` family (decision 2 (b)).

7. **Two tests, two scratch sets.** `the_apply_scenario_runs_in_order_on_scratch_instances` runs the issue's steps 1 to 5 and 7 and the end state; `a_failing_probe_refuses_the_create_and_the_relaunch_and_changes_nothing` runs step 6 on a scratch set of its own, so a failure in one does not hide the other and step 6's "nothing changed" has a quiet registry to compare against. The step table below is the contract of both. Timing: #649's four scenarios take about 60 to 80 seconds in all (E68); this story starts about eleven servers in two tests and should take about as long. A create or relaunch runs under launch's 10 s budget (E36); a server that cannot start inside it is a finding to report (#649 D9), not a reason to add a knob.

8. **"Nothing changed" and "never killed" are measured, and each has a twin that changes.** A snapshot value `Still` (new, in the scenario's `plan.rs`) holds, for a set of panes: `pane list`'s `data` (the registry), each record's `generation`, `profile` and `session_of_record`, `owned_pids(pane)` and `refused(port)`, `profile show`'s `profile.generation`, and Herdr's snapshot (pane ids and cells). A claim "apply left X alone" is `Still` before equal to `Still` after. Every such claim has a twin in the same run where the same measure must differ (a relaunch changes the owned pids; an adopt that joins changes the generation; a create adds a Herdr pane), so the measure is shown able to fail. A pane's processes are the pids `tmux` tags for its session, as the scratch set reads them (E11, `owned_pids`).

9. **The probe is a real process, and the scenario uses `curl` and `sh` for it.** The check is an argv run by the CLI's prober, never through a shell, looked up on the binary's `PATH`, in the CLI's environment (E39); a program that exits non-zero, or output without an expected string, is `probe-failed` (E38), and the probe is a plan step that runs before any live change (E36, E37). Guide section 8 gives the same shape with `curl -s` (E56). The bad endpoint is a port from `free_port()` that `refused(port)` proves nothing listens on: `check ["curl","-s","--max-time","3","http://127.0.0.1:<port>/global/health"]`, `expect ["healthy"]` (OpenCode answers `{"healthy": true}` on that path; the adapter's own health route). The control uses the same command against a live scratch server's port. A value that starts with `-` is given in the `=` form (`--check-arg=-s`, E52). A second check, `["sh","-c","touch <file in the scratch directory>; echo ok"]` with `expect ["ok"]`, leaves a mark, and is how the scenario shows a dry run runs no probe and a live create does (decision 8's twin). `curl` and `sh` are on the binary's `PATH` (`/usr/bin:/bin` is part of it, E10); T proves it, and a missing `curl` fails the opted-in run.

10. **The topology is rows of one pane, and one thing the scenario does not pin is reported.** Every scratch workspace is 2 rows by 1 column, so a missing pane at r2c1 is one `down` split of r1c1 (E40, E41). Apply creates in `(workspace, row, column)` order (E33) and the adapter plans one cell at a time (E43); the planner refuses "a row added below a row of more than one pane" as `grid-unreachable` (E42). By reading those three excerpts, a profile whose r1c2 and r2c1 are both missing in a workspace that holds only r1c1 cannot be created by one apply, because r1c2 is placed first. *Read, not run.* The scenario does not use that shape, and it does not pin it either (a test of a defect pins the defect). It is reported to the operator in the handoff as a candidate follow-up, with the excerpts; filing it is the operator's. "Checked against Herdr's own snapshot" means: after a create, the snapshot holds exactly the earlier panes (same ids and cells) plus one pane at the spec's cell. The scenario cannot feed `plan_splits` a snapshot: its `GridMap` is built only from Herdr's layout tree (E44), and `plan_splits` has its own tests and #649's grid test. *Rejected:* importing `plan_splits` into the scenario.

11. **What the scenario does not assert.** No `reset`, no `switch`, no `last_observed.driven`, no `shown-driven-mismatch`, no `reset --first`: #839 changes how DRIVEN follows the session of record and how `reset --first` is gated, and #649's own D18 (a) already leaves DRIVEN out of its scenario (E66). Apply's `relaunch` keeps the session of record (E36) and moves no DRIVEN. The scenario asserts SHOWN, where it must, only through `pane get`'s record (the binary reads the TUI with `LANG=C.UTF-8` from `binary_env`, which is #844's workaround, E9, E14); the test process never calls `shown_session` or `attach_tui` itself, because its own locale is not chosen. No `detail` of an `adopt` step is asserted: it may carry the "could not tell which session the TUI shows" note (E35), and a relaunched TUI is not ready at once (#845). No `switch` follows a `relaunch`, so #845's race cannot occur here. Row e (an unhealthy orchestrator), the `failed` and `skipped` outcomes and exit code 1 are pinned by the fakes in the default run and need no real adapter to differ; they stay out. No fake host is built here (#846's three `LinkedHost` copies grow by none). No step types into a TUI (I4).

12. **The scenario validates itself by controls and by mutants.** Controls are decision 8's twins and decision 9's good-and-bad pair. Mutants: Acceptance A16 lists them; T applies each by script to a throwaway copy of the production file, runs the opted-in scenario, and restores the file, as #649's T-green did for its mutants (E73). A mutant that the scenario does not fail is a gap in the scenario, to be closed before GREEN. A scenario that cannot fail is not evidence.

13. **Pipeline: there is no production code, so phase F has nothing to change.** The Feature role says "F never edits tests" (E74), and every file of this story is under `crates/*/tests/**` or is documentation (E70 lists the test paths). T authors the scenario, the moves of decision 3 and the default-run pins; RED is the default-run pins failing to compile or to pass before the helpers exist; GREEN is the pins, #649's default run, and one opted-in run of both scenarios. O records phase F as skipped by this decision, or runs it with the instruction to answer "nothing to change". *Unverified: how the Workflow script treats a story whose F phase leaves no diff; O checks at pre-flight.* `rigor: in-session`.

14. **CHANGELOG, the guide's last list, and the manifest.**
   - One entry under `## [Unreleased]` / `### Enhancements`, linking #667, naming the run line of decision 4, and saying **tests only: no behaviour change** (E71; E72 is #649's mention of its own scenario). Proposed text: "Pane control, the apply rehearsal (epic [#633](https://github.com/Performant-Labs/holler/issues/633)): an opt-in scenario runs `holler profile apply` and the pane verbs around it on a scratch Herdr, tmux and OpenCode that it starts itself ([#667](https://github.com/Performant-Labs/holler/issues/667); `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test profile_apply_scenario -- --ignored --test-threads=1`). It shares the scratch set of #649, whose helpers moved into one place. Tests only: no behaviour change."
   - `docs/pane-control.md` section 12 ("Later") lists "the real-adapter scenario for `profile apply` (#667)" as deferred (E57). The scenario will exist, so that one bullet is deleted. `docs_pane_control_test` does not pin it (it does not name #667). *Unverified by running the docs test; T runs it.* ADR-0021 needs no edit: it already says the opt-in scenarios of #649 and #667 refuse real session names (E60).
   - **`crates/holler-cli/Cargo.toml` is not edited.** `autotests = false` makes every target a hand-written `[[test]]` (E1), but #670 declared this story's target already (E3, E4), and #649 added the three adapter crates as `[dependencies]` (E2); `tempfile`, `serde_json`, `holler-pane-testkit` and the rest are there. If T or F finds an edit is needed, it stops and asks: that file is a hot spot (see "Clashes").

## Scope (final)

1. **The scenario target**, under `crates/holler-cli/tests/profile_apply_scenario/`, replacing the placeholder `main.rs` (E4): `main.rs` (module doc, run line, includes), `scenario.rs` (test A), `probe.rs` (test B), `plan.rs` (the `Applied`/`Step` reader, `Still`, the plan-against-show comparison), `pins.rs` (the default-run tests). Each file stays under about 450 lines (`scripts/lint.sh` fails at 900). Every crate-level `#![allow(...)]` of the target carries a trailing `// #667` (`scripts/lint.sh` rule 1), as `pane_integration/main.rs` does with `// #649` (E5).
2. **The moves and the one guard** of decision 3, in `crates/holler-cli/tests/pane_integration/` (`main.rs`, `scratch/mod.rs`, `scenario.rs`, `guards.rs`, new `out.rs`).
3. **`CHANGELOG.md`** and **one bullet of `docs/pane-control.md`** (decision 14).
4. **No** production source, no ADR, no manifest, no workflow, no `tests/support/mod.rs` (849 lines, past the overlay's line of about 800; #839 names it too).

## The scenario (what each step does and asserts)

Names: `A1`=`hlr649-ap1` (workspace `hlr649`, r1c1), `A2`=`hlr649-ap2` (`hlr649`, r2c1), `B1`=`hlr649-ap3` (`hlr649b`, r1c1), `X`=`hlr649-ap4` (`hlr649b`, r2c1; never launched by hand). Profiles `P`=`hlr649-apply-p`, `Q`=`hlr649-apply-q`. Ports come from `free_port()`; every server that a step starts is followed by the model guard on its port (`no_model_can_be_reached`), as in #649. `S` is the scratch Herdr session; every `--herdr-session` argument is `S` and nothing else (decision 3's guard). The commands are the contract; the exact flag lists are T's, built with `launch_args`.

| Step (issue) | What it does | What it asserts, and the control |
|---|---|---|
| 1 | Launch `A1`, `A2` (no profile). `profile create P --from-current`; `profile create Q --from P` (a detached copy: the known-good originals, no members). Then launch `B1` with no profile. `profile show P`. | `create`'s `members` is `[A1, A2]`; both records' `profile` is `P`; `B1`'s is `null`. `show P` JSON: every comparison row `matches`, every `differences` empty; the text form has no `differs` (E49, E50). Control: `show Q` lists its specs as `missing (no live pane)`, because a profile's live panes are its members; and `show P`, once `B1`'s spec is added in step 2, lists `B1` as `missing` for the same reason. |
| 2 | `pane relaunch A2 --profile P --spec-only --agent plan` (a spec change, nothing live); `pane launch B1 --profile P --spec-only` (B1's spec, same flags as its launch); `pane launch X --profile P --spec-only` (X's spec, with the marker check of decision 9, workspace `hlr649b`, `--grid r2c1`, a fixed port). Then `profile show P`, then `profile apply P --dry-run` without `--herdr-session`, then with it, then once in text. | `show P`: `A2` `differs` on the agent (spec `plan`, live `null`), `X` `missing` (no record), `A1` `matches`, and `B1` `missing` too, since `show` compares a profile's members and `B1` is not one (E50, E46, E51). Dry run without the flag: exit 2, error `usage`, before any Herdr call (E32). With it: exit 0, `dry_run: true`, one step per spec pane in `(workspace, row, col)` order, and **show's rows, with one stated exception**: for `A1`, `A2` and `X`, `matches` is `adopt`/`planned`, `differs` is `report`/`reported` with `diff` equal to show's `{pane, status, differences}` row, `missing` is `create`/`planned`, with the same assertion shape as the fakes' (E61); `B1` is the exception the scenario states, not hides: `show` says `missing` (not a member) and the plan says `adopt`/`planned`, because apply looks at every record a spec names, in any profile, and adopts it (E30, E31). Nothing changed: `Still` equal before and after, and the marker file does not exist (no probe in a dry run, E30). Text: the first line is `profile NAME: 4 steps, planned (dry run; nothing changed)` and each step is one `PANE: ACTION, OUTCOME` line (E48). |
| 3 | `profile apply P --herdr-session S`. Then the same again. Then `pane relaunch A2 --profile P` (the operator converges what apply only reported). | Exit 0; four steps: `A1` `adopt`/`adopted` (a member already: its `generation` unchanged), `A2` `report`/`reported` with its `diff` (live untouched), `B1` `adopt`/`adopted` (a non-member joins: `profile` becomes `P`, `generation` advances; **its owned pids, port health and session of record are the same**; E34), `X` `create`/`created` (record exists; `profile` `P`; `probe.last` is ok; **the marker file now exists**, so the probe ran in the live create and not in the dry run; **Herdr's snapshot holds the earlier panes at the same ids and cells plus one at `hlr649b` r2c1**, decision 10). `A1`'s pids unchanged (never killed); control: `X`'s are new. Second apply: no `created`, `Still` equal (apply is idempotent on real state, E63). The relaunch changes `A2`'s owned pids and its record's agent to `plan`, `show P` is `matches` again, and its session of record is kept. |
| 4 | Kill `B1`'s server (`kill_server`: SIGKILL to the process groups of its owned pids, then wait for the port to refuse). `profile apply P --dry-run`, then `profile apply P` without `--herdr-session`. | Dry run: `B1` `relaunch`/`planned`, the detail says its server did not answer. Live (no `--herdr-session`: a plan with no create needs none, E32): exit 0, `B1` `relaunch`/`relaunched`; its port answers again, **its session of record is the same**, its pids are new (control for step 3's "unchanged"); the other three panes' pids and `A1`'s `generation` are unchanged. The model guard runs on the new server. |
| 5 | `profile apply Q --herdr-session S`; then `profile apply Q --take-over --herdr-session S`. | Without `--take-over`: exit 0; `A1` and `A2` are `refuse`/`refused` with code `pane-in-other-profile`; **no record changes** (`Still` equal) (E33). With it: `A1` `adopt`/`adopted`, its `profile` is `Q` and its `generation` advanced by two (the leave and the join; T relaxes to "advanced" if the counter differs); `A2` `report`/`reported` (it differs from `Q`'s original: the agent `plan`), moved to `Q` with **nothing live touched** (pids unchanged); `B1` and `X` are not in `Q`'s specs, and stay in `P`. |
| 7 | `pane relaunch A2 --profile Q` (rollback of the changed pane to `Q`'s known-good spec), then `profile apply Q`. | The relaunch is allowed because `A2` is now `Q`'s member (a pane of another profile would be `pane-in-other-profile`, E45); `A2`'s record agent is `null` again and its pids are new. `apply Q`: exit 0, `A1` and `A2` `adopt`/`adopted`, `show Q` all `matches`, two live panes. |
| end | `assert_end_state([A1, A2, B1, X], ["hlr649", "hlr649b"])`; `assert_tmux_env_is_scratch`; `assert_no_credential_leaked`. | `doctor` reports no finding; each record's `(pane_id, grid)` equals Herdr's snapshot for the scratch panes; none is in `pane list`'s unmanaged table (E19). Also `profile list` live counts: `P` 2 (`B1`, `X`), `Q` 2 (`A1`, `A2`); `profile apply nonexistent`: exit 3, `profile-not-found`. |
| 6 (test B) | Own scratch set. Launch `Z`=`hlr649-pz` (`hlr649h`, r1c1) and `W`=`hlr649-pw` (`hlr649g`, r1c1; the live endpoint the good checks name). `profile create hlr649-probe --from-current`. Spec-only: `Y`=`hlr649-py` (`hlr649h`, r2c1, a fixed port) with the bad curl check. (i) `profile apply hlr649-probe --herdr-session S`. (ii) Kill `Z`'s server, spec-only give `Z` the bad check, `profile apply hlr649-probe`. (iii) Spec-only change both checks to the same command naming `W`'s port, `profile apply hlr649-probe`. | (i) Exit 0; `Z` and `W` `adopt`/`adopted`; `Y` `create`/`refused` with code `probe-failed` (the others proceed, E59). Nothing for `Y`: `pane get Y` exits 3 `pane-not-found`; Herdr's snapshot is `Still`-equal (no split); no tmux session named `Y`; its port refuses. (ii) `Z`: `relaunch`/`refused`, `probe-failed`, and `Y` refused again; the probe ran **before** the relaunch started anything: `Z`'s port still refuses, its owned pids are empty, its record is `Still`-equal. (iii) `Z` `relaunch`/`relaunched` and `Y` `create`/`created` at `hlr649h` r2c1 in Herdr's snapshot with `probe.last` ok. Each refusal and its fix differ only in the endpoint of the same curl command, so the refusal is the probe's. |

## Acceptance (each item is a test or a check)

The scenario items run only opted in; the pins run in the default run.

- **A1** (issue: every step passes on a developer machine with only scratch instances). `the_apply_scenario_runs_in_order_on_scratch_instances` and `a_failing_probe_refuses_the_create_and_the_relaunch_and_changes_nothing` pass opted in, twice in a row, leaving no scratch directory, process or Herdr session behind (T-green checks `ps` and `/tmp`, as #649's did).
- **A2** Step 1, as the table says; **A3** step 2; **A4** step 3 (adopt never kills, join writes, create lands at its cell, idempotent); **A5** step 4; **A6** step 5; **A7** step 7; **A8** step 6 (three phases); each with its control.
- **A9** (issue: the registry equals what the adapters observe at the end). The end row of the table, by the shared `assert_end_state`.
- **A10** (issue: every `--format=json` output parses against the envelope). Every JSON verb of both tests goes through `Scratch::json`, which runs `check_envelope` (E11); the text-mode runs are the only exceptions and are named in the table.
- **A11** (issue: nothing outside the scratch set is touched). The guards of #649 hold unchanged and pass in the default run (`guards.rs`, E13, E14), plus the new `--herdr-session` argument guard (A13); `assert_no_credential_leaked` runs at the end of both tests; the binary runs with `env_clear` and the environment `binary_env` builds (E10).
- **A12** (default run) A source pin: every `#[test]` in `profile_apply_scenario/*.rs` that starts a scratch set carries `#[ignore = "..."]` naming `HOLLER_TEST_PANE_INTEGRATION`. Fails when an `#[ignore]` is removed (T removes one to see it fail).
- **A13** (default run) `Scratch::holler`'s argument guard is a pure function: `--herdr-session <scratch session>` passes; `default`, another `holler649-` name that is not this scratch's, an absent value and a second flag in the value position are refused. Fails against a version of the guard that ignores the flag.
- **A14** (default run) The `Applied` reader parses a fixed envelope in the guide's shape (`{"profile","dry_run","steps":[{"pane","action","outcome","code","diff","detail"}]}`, absent keys `null`) and refuses one with a missing key or an unknown action or outcome name; and `launch_args` yields exactly the flag list `Scratch::launch` sent before the move.
- **A15** (default run) #649's whole default run still passes unchanged, and so does `docs_pane_control_test` and `docs_cli_test`; `cargo clippy --workspace --all-targets -- -D warnings` and `scripts/lint.sh` are clean.
- **A16** (mutants, by script, on the production files, restored after; each must fail the named step opted in). M1: in `tx_apply.rs` `choose`, make a healthy matching pane a `Row::Relaunch` instead of `Row::Adopt` (fails step 3's pids). M2: in `tx_launch.rs` `probe()`, return `Ok(None)` first (fails test B (i)). M3: in `choose`, drop the `&& !take_over` of row a so a pane of another profile is not refused (fails step 5). M4: in `tx_apply/observe.rs` `judge`, treat `health` as true (fails step 4: `B1` is not relaunched). M5: in `apply()`, run the live steps when `dry_run` is set (fails step 2's `Still` and the marker). M6: in `tx_apply/run.rs` `take_over`, skip the join (fails step 5's `profile`). M7: in `plan.rs` `below()`, `Direction::Down` becomes `Right` (fails step 3's create: the pane lands elsewhere or the adapter refuses). T reports each result in its handoff; a surviving mutant gets a stronger assertion before GREEN.
- **A17** The CHANGELOG entry and the deleted bullet (decision 14) are present, and `scripts/changelog-check.sh` passes.

## Blast radius

- `crates/holler-cli/tests/profile_apply_scenario/**` (new files; replaces the placeholder `main.rs`).
- `crates/holler-cli/tests/pane_integration/main.rs`, `scenario.rs`, `scratch/mod.rs`, `guards.rs` and the new `out.rs`: the moves and the one guard of decision 3 only; #649's default run and opt-in scenario pass unchanged.
- `CHANGELOG.md`; `docs/pane-control.md` (one bullet of section 12).
- `docs/handoffs/667-brief.md` and `docs/handoffs/667/**` (the pipeline's own).
- **Not edited:** `crates/holler-cli/Cargo.toml` (decision 14), `crates/*/src/**`, `docs/adr/ADR-0021.md`, `crates/holler-cli/tests/support/**`, `.github/**`, the adapter crates' test directories.

## Clashes with stories in flight

Read-only look at the issues #831, #835, #839 and #648 on `main` at `bda1069`; no worktree of those stories was read, except that the #839 worktree was listed and held no brief yet when this was checked.

| Shared file | #667 | #831 (Claude Code harness) | #835 (import leaves panes alone) | #839 (DRIVEN, absorbs #828) | #648 (roster) |
|---|---|---|---|---|---|
| `crates/holler-cli/Cargo.toml` | none | probably a `[[test]]` and a crate (from its text) | none expected | none expected | probably a `[[test]]` (its text adds test files; `autotests = false`) |
| `CHANGELOG.md` | one entry | one | one | one | one |
| `crates/holler-cli/tests/support/mod.rs` | none | none | none | named as a hot spot (849 lines) | none |
| `docs/adr/ADR-0021.md` | none | contract amendment | contract amendment | sections 8 and 11 | none |
| `docs/pane-control.md` section 12 | deletes the #667 bullet | may add or edit | deletes its bullet | deletes its bullet | deletes its bullet |
| `pane_integration/scenario.rs`, `scratch/mod.rs` | moves helpers (hunks around lines 29-52, 81-116, 207-274; `launch_args`) | its acceptance is a new scratch scenario, which should use `Scratch` | none | edits `reset_with_a_first_message` (about lines 190-205), "#828 will change it" | none |

- **Facts of the first row.** `Cargo.toml` is `paths.production` for the pipeline (E70), is edited by every story that adds a test target, and #667 needs no edit (decision 14), so #667 cannot conflict there and should not add a reason to.
- **#839 and DRIVEN.** #667 avoids DRIVEN altogether (decision 11): it uses no `reset` and no `switch`, and asserts no `last_observed.driven` and no `shown-driven-mismatch`. Its end state is the shared `assert_end_state`, so if #839 changes what a clean `doctor` means for a scratch pane, the change is made once, in that one function. The disjoint hunks in `scenario.rs` merge cleanly; if #839 lands first, #667 rebases and takes the new `reset_with_a_first_message` as it is.
- **Recommended merge order:** (1) **#667**: everything it needs has merged, it adds no production code and no manifest edit, and it creates the shared helpers that #839's and #831's scenario work should reuse rather than copy. (2) **#648** and **#835**, in either order, independent of #667 (fakes only). (3) **#839**, after the contract amendment, rebasing `scenario.rs`, `pane-control.md` section 12, ADR-0021 and the CHANGELOG. (4) **#831** last; it is blocked on the spike's amendment anyway. Every story after the first resolves the CHANGELOG by keeping all entries, as #649 did with #832 and #838 (E67), and `docs/pane-control.md` section 12 by keeping every bullet that is not its own.
- *Unverified:* the stories' own blast radii beyond their issue texts; #831 and #835 may touch files not listed.

## Depends on

All merged. Checked with `gh issue view` on 2026-10-11 (closed times in local time, MDT):

| Story | State | Closed | What #667 uses of it |
|---|---|---|---|
| #637 | closed | 2026-10-09 05:55 | `holler-pane` types, error codes, ports |
| #638 | closed | 2026-10-09 15:37 | the test kit, `check_envelope` |
| #644 | closed | 2026-10-11 11:45 | `pane launch`/`relaunch`, `--spec-only`, the launch transaction apply calls |
| #662 | closed | 2026-10-09 21:48 | `profile create`/`show`/`delete`/`list` |
| #664 | closed | 2026-10-11 12:14 | `profile apply` |
| #669 | closed | 2026-10-09 07:37 | the hub's `pane/*` and `profile/*` plumbing |
| #670 | closed | 2026-10-09 09:54 | the CLI skeleton, and the `[[test]]` placeholder of this story |
| #640 | closed | 2026-10-10 00:06 | the Herdr adapter (splits, snapshot) |
| #641 | closed | 2026-10-09 20:06 | the host adapter (tmux) |
| #642 | closed | 2026-10-11 11:56 | the OpenCode adapter |

Also merged and used: #649 (closed 2026-10-11 17:35; the scratch set and the wiring), #652 (closed 2026-10-11 18:13; the guide the scenario asserts). The issue's "does not depend on #649" is superseded (decision 1). Follow-ups that are not dependencies: #844 (the TUI query needs a UTF-8 locale; `binary_env` sets `LANG=C.UTF-8`), #845 (`pane switch` right after `pane relaunch`; #667 never switches), #846 (the three `LinkedHost` fakes; #667 adds none). Blocks: #666, whose step 0 is this rehearsal.

## Pipeline

`rigor: in-session` (the issue's). Fresh worktree, branch `issue-667-implementation`. Phases: O, A (this brief), T (author and RED), F skipped by decision 13, T (verify and GREEN, with the mutants of A16 and one opted-in run), A (anti-duplication), S. `uiSurface: false`.

## Rules for this story

See the epic (#633): the contract section there is fixed, so this story builds against it without waiting for another story's code. Issue first, tests first, Holler's pipeline and hooks apply (branch `issue-<N>-implementation`, fresh worktree, Conventional Commits, `cargo fmt` on the story's own files only, clippy clean, no new `unsafe`). Edit only the files in the Blast radius. No test touches a live fleet, a running pane or a real Herdr session; the scenario runs only on scratch instances it starts itself, and refuses real session names (guards 2 and 4, E7). The repository is public: no personal host, network or account name in any file. Never print or commit a credential.

## Questions only the operator can answer

1. **The creation-order hazard of decision 10.** Should it be filed as a follow-up (apply creates in row-then-column order, the planner wants rows before columns), and is its fix apply's order, the planner, or documentation? Read, not run; filing is outward.
2. **Phase F with no production code** (decision 13): skip it by journal decision (recommended), or run it empty? Whether the Workflow script accepts an empty F diff was not checked.
3. **Naming** (decision 6): this brief keeps `holler649`/`hlr649` for #667's scratch objects to avoid editing the merged guards; a rename is a separate, churn-only change.

## AI assistance

This brief was drafted by Claude (Sonnet 5.5) from the issue, the merged tree and #649's handoffs, at the operator's direction; the maintainer filing it is accountable for it, per `CONTRIBUTING.md`.

## Evidence (verbatim)

Each entry is cut from the file at `bda1069` (`main`) by the `sed -n` command printed under its heading; the awk adds the line numbers. Nothing is retyped. Quote an entry as E<n>. The excerpts of #649 brief and handoffs keep their own internal E-numbers and D-numbers; those are not entries of this brief.

### E1  `crates/holler-cli/Cargo.toml` lines 8-14: autotests is off: every test target is declared by hand

Command: `sed -n '8,14p' crates/holler-cli/Cargo.toml | awk -v s=8 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
8: # not an integration test. Cargo would otherwise auto-detect that file under
9: # tests/ as a *second* target named `stub-acp` (an integration test), which
10: # collides with the bin and emits a "present in multiple build targets" warning
11: # on every build. Disabling auto-detection and declaring the real test targets
12: # explicitly (below) keeps the bin the sole owner of the name while still
13: # building/running both integration tests.
14: autotests = false
```

### E2  `crates/holler-cli/Cargo.toml` lines 404-413: the three adapter crates are already holler-cli dependencies (#649)

Command: `sed -n '404,413p' crates/holler-cli/Cargo.toml | awk -v s=404 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
404: holler-proto = { path = "../holler-proto" }
405: # #670: the pane/profile verbs take every error code, guard and port type from
406: # the holler-pane crate (`Ports`, `PaneError`, `GridPos`, `Argv`, `EnvVarName`).
407: holler-pane = { path = "../holler-pane" }
408: # #649: `pane/wiring.rs` builds the real ports from the three adapter crates
409: # (ADR-0021 section 5: the verbs run CLI-side, so holler-cli depends on the
410: # adapters and holler-hub does not).
411: holler-adapter-herdr = { path = "../holler-adapter-herdr" }
412: holler-adapter-host = { path = "../holler-adapter-host" }
413: holler-adapter-opencode = { path = "../holler-adapter-opencode" }
```

### E3  `crates/holler-cli/Cargo.toml` lines 511-520: the placeholder target of #667 is already declared (#670)

Command: `sed -n '511,520p' crates/holler-cli/Cargo.toml | awk -v s=511 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
511: # Placeholder targets: #649 (hub wiring and integration scenario) and #667
512: # (profile apply on scratch instances) fill these in; declared here so neither
513: # story edits this manifest.
514: [[test]]
515: name = "pane_integration"
516: path = "tests/pane_integration/main.rs"
517: 
518: [[test]]
519: name = "profile_apply_scenario"
520: path = "tests/profile_apply_scenario/main.rs"
```

### E4  `crates/holler-cli/tests/profile_apply_scenario/main.rs` lines 1-2: the placeholder file

Command: `sed -n '1,2p' crates/holler-cli/tests/profile_apply_scenario/main.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! Placeholder target for story #667 (`holler profile apply` on scratch Herdr, tmux and
2: //! OpenCode). Declared by #670 so #667 adds a file here and edits no manifest.
```

### E5  `crates/holler-cli/tests/pane_integration/main.rs` lines 6-24: how #649 splits default-run tests from the opt-in scenario

Command: `sed -n '6,24p' crates/holler-cli/tests/pane_integration/main.rs | awk -v s=6 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
6: //! The pane wiring (#649) against a real hub, and the scripted scenario against scratch
7: //! Herdr, tmux and OpenCode instances (epic #633). One target, declared by #670.
8: //!
9: //! **Two kinds of test (D13).** The default run (CI) needs only a hub subprocess and a
10: //! state dir: the no-hub answer of every verb (`no_hub`), the verbs that need no adapter
11: //! (`store_only`), the store clients against a real hub (`clients`: the test kit's two
12: //! conformance suites) and against a hub that misbehaves on purpose (`fake_hub`), the
13: //! watch (`watch`), the environment the wiring reads (`env`, and `wired` for the ports
14: //! `connect` builds from it), the bindings, decorators and
15: //! lazy ports (`bind`), and the scenario's guards as pure functions (`guards`). The
16: //! scenario itself (`scenario`) is `#[ignore]` and runs only with
17: //! `HOLLER_TEST_PANE_INTEGRATION=1` exactly, on scratch instances it starts itself
18: //! (`scratch`).
19: //!
20: //! **The state-dir lock.** The store clients reach the hub through
21: //! `holler_hub::control::run`, which reads `HOLLER_STATE_DIR` from this process on every
22: //! call. Tests on parallel threads would race on it, so every in-process client call runs
23: //! inside [`with_state_dir`], which holds one lock while it sets the variable (A's W-10).
24: //! The process-level tests set the variable on the child only and need no lock.
```

### E6  `crates/holler-cli/tests/pane_integration/main.rs` lines 100-136: Out and output_of: what scratch/mod.rs needs from its crate root

Command: `sed -n '100,136p' crates/holler-cli/tests/pane_integration/main.rs | awk -v s=100 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
100: /// What one run of the `holler` binary produced.
101: #[derive(Debug)]
102: pub struct Out {
103:     pub code: i32,
104:     pub stdout: String,
105:     pub stderr: String,
106: }
107: 
108: impl Out {
109:     /// Stdout as exactly one envelope that agrees with the exit code (#638's checker).
110:     pub fn envelope(&self) -> Envelope {
111:         check_envelope(&self.stdout, self.code)
112:             .unwrap_or_else(|fault| panic!("stdout is one envelope: {fault}: {self:?}"))
113:     }
114: 
115:     /// The error code of the envelope on stdout.
116:     pub fn error_code(&self) -> Option<String> {
117:         self.envelope().error.map(|e| e.code)
118:     }
119: 
120:     /// The `data` of a successful envelope on stdout.
121:     pub fn data(&self) -> serde_json::Value {
122:         let envelope = self.envelope();
123:         assert!(envelope.ok, "a successful envelope: {self:?}");
124:         envelope.data
125:     }
126: }
127: 
128: /// Run `command` to its end and collect what it printed.
129: pub fn output_of(mut command: Command) -> Out {
130:     let output = command.output().expect("run the holler binary");
131:     Out {
132:         code: output.status.code().expect("holler exited, not killed"),
133:         stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
134:         stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
135:     }
136: }
```

### E7  `crates/holler-cli/tests/pane_integration/scratch/mod.rs` lines 1-36: the scratch set and its guards

Command: `sed -n '1,36p' crates/holler-cli/tests/pane_integration/scratch/mod.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! The scenario's scratch set (D10, D11, D12 and A's W-1): one fresh `/tmp/hlr649-*`
2: //! directory holding a hub's state dir, a scratch Herdr, a private tmux server and
3: //! OpenCode's whole `HOME`/`XDG_*`, every process started by the test itself, and five
4: //! guards, each a pure function the default run tests (`guards.rs`).
5: //!
6: //! **Reuse, never a copy (D11).** The Herdr guards and server come from #640's scratch module,
7: //! the dead-end config, the isolated environment, the port picker and the model guard from
8: //! #642's OpenCode rig, and the private tmux server from #641's tests, each included below with
9: //! `#[path]` and generalised in place where it had to be (A's W-3).
10: //!
11: //! **The guards.**
12: //!
13: //! 1. *Opt-in* ([`GATE_VAR`]): only `HOLLER_TEST_PANE_INTEGRATION=1` runs a scenario; opted in,
14: //!    a missing `herdr`, `tmux` or `opencode` is a failure, not a skip.
15: //! 2. *Unique names* ([`check_session`], [`check_pane_name`]): the Herdr session is
16: //!    `holler649-<8 hex>` (`default` refused by name) and every pane name is `hlr649-...`.
17: //! 3. *One directory* ([`check_inside`]): the state dir, the Herdr socket, the tmux socket and
18: //!    OpenCode's root all lie inside it; the guard removes it after killing what it started.
19: //! 4. *Refusal of real names* ([`binary_env`]): every name and path is checked before a verb
20: //!    runs, and the binary runs with exactly the environment built here, nothing inherited (no
21: //!    `HERDR_*`, no `TMUX`, no `TMUX_PANE`, no `HOLLER_*` the test did not set).
22: //! 5. *OpenCode's environment* ([`tmux_server_env`], W-1): the test starts the private tmux
23: //!    server itself, with exactly the rig's isolated pairs, before any verb runs. Under D7's
24: //!    `ProcessEnv::Inherit` every OpenCode process inherits that server's environment, so no
25: //!    OpenCode the scenario starts can read the operator's config, auth or session store. The
26: //!    model guard (D12) then runs on every new server right after its verb returns.
27: //! 6. *Killing what it started* (D10 (6)): the servers and TUIs are started by the private
28: //!    tmux server and Herdr, not by the test, so [`Scratch`]'s drop finds them by
29: //!    `tmux list-panes -a` on the scratch socket and `HostPort::ps` per scratch session, sends
30: //!    `TERM` then `KILL` to their process groups, kills the tmux server, stops Herdr, and only
31: //!    then removes the directory.
32: //!
33: //! **No credential leaks (D28).** [`Scratch::start`] plants a sentinel in this process's
34: //! environment under the names a provider credential goes by; [`Scratch::assert_no_credential_leaked`]
35: //! greps every verb's stdout and stderr, every file of the scratch directory and the argv of
36: //! every process (`ps`) for it.
```

### E8  `crates/holler-cli/tests/pane_integration/scratch/mod.rs` lines 38-110: the included modules, the names, the workspaces and the first two guards

Command: `sed -n '38,110p' crates/holler-cli/tests/pane_integration/scratch/mod.rs | awk -v s=38 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
38: #[path = "../../../../holler-adapter-herdr/tests/scratch_herdr/mod.rs"]
39: pub mod scratch_herdr;
40: 
41: #[path = "../../../../holler-adapter-opencode/tests/real_opencode/rig.rs"]
42: pub mod rig;
43: 
44: #[path = "../../../../holler-adapter-host/tests/real_tmux/mod.rs"]
45: pub mod real_tmux;
46: 
47: use std::path::{Path, PathBuf};
48: use std::process::{Command, Stdio};
49: use std::sync::{Arc, Mutex, PoisonError};
50: use std::time::Duration;
51: 
52: use holler_adapter_herdr::adapter::{HerdrAdapter, HerdrConfig};
53: use holler_adapter_herdr::plan::Extent;
54: use holler_adapter_host::TmuxHost;
55: use holler_adapter_opencode::{OpenCodeConfig, OpenCodeHarness, ProcessEnv, Timeouts, TmuxConfig};
56: use holler_pane::{HerdrPane, HerdrPort, HostPort, PaneError, PaneId, PaneName};
57: use holler_pane_testkit::envelope::check_envelope;
58: use tempfile::TempDir;
59: 
60: use crate::support::{holler_bin, wait_for, Hub, StateDir};
61: use crate::{output_of, Out};
62: use scratch_herdr::{check_name_as, check_socket, Flavor, ScratchHerdr};
63: 
64: /// Guard 1: the variable that opts in; only the exact value "1" runs a scenario.
65: pub const GATE_VAR: &str = "HOLLER_TEST_PANE_INTEGRATION";
66: 
67: /// Guard 2: this story's flavor of #640's scratch Herdr.
68: pub const FLAVOR: Flavor = Flavor {
69:     gate_var: GATE_VAR,
70:     name_prefix: "holler649-",
71:     root_prefix: "h649.",
72: };
73: 
74: /// Guard 2: every pane (and tmux session) the scenario names starts with this.
75: pub const PANE_PREFIX: &str = "hlr649-";
76: 
77: /// The tmux session that keeps the private server running; never a pane's.
78: pub const ANCHOR: &str = "hlr649-anchor";
79: 
80: /// The names a provider credential goes by; [`Scratch::start`] sets each to the sentinel in
81: /// this process's environment (D28).
82: pub const SENTINEL_VARS: &[&str] = &["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "OPENROUTER_API_KEY"];
83: 
84: /// Whether `pid` answers `kill -0` (D10 (6), D15).
85: pub fn alive(pid: u32) -> bool {
86:     Command::new("kill")
87:         .args(["-s", "0", "--", &pid.to_string()])
88:         .stderr(Stdio::null())
89:         .status()
90:         .is_ok_and(|s| s.success())
91: }
92: 
93: /// The workspaces the wiring is told about, each 2 rows by 1 column (`label=RxC`).
94: pub const WORKSPACES: &str = "hlr649=2x1,hlr649b=2x1,hlr649g=2x1,hlr649h=2x1";
95: 
96: /// Guard 2: the Herdr session must be `holler649-<8 hex>`; `default` is refused by name.
97: pub fn check_session(name: &str) -> Result<(), String> {
98:     check_name_as(FLAVOR.name_prefix, name)
99: }
100: 
101: /// Guard 2: a pane name the scenario may use: `hlr649-...` and a valid pane name.
102: pub fn check_pane_name(name: &str) -> Result<(), String> {
103:     if !name.starts_with(PANE_PREFIX) || name.len() == PANE_PREFIX.len() {
104:         return Err(format!("refusing the pane name {name:?}: a scratch pane is {PANE_PREFIX}..."));
105:     }
106:     PaneName::parse(name)
107:         .map(drop)
108:         .map_err(|e| format!("refusing the pane name {name:?}: {e}"))
109: }
110: 
```

### E9  `crates/holler-cli/tests/pane_integration/scratch/mod.rs` lines 116-176: Settings, the tmux server environment and binary_env (guard 4)

Command: `sed -n '116,176p' crates/holler-cli/tests/pane_integration/scratch/mod.rs | awk -v s=116 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
116: /// What the binary is pointed at. Every path lies inside `base`.
117: #[derive(Debug, Clone)]
118: pub struct Settings {
119:     pub base: PathBuf,
120:     pub state_dir: PathBuf,
121:     pub session: String,
122:     pub herdr_socket: PathBuf,
123:     pub tmux_socket: PathBuf,
124:     pub opencode: PathBuf,
125:     /// OpenCode's scratch root, laid out by the rig (`home`, `data`, `config`, ...).
126:     pub oc_root: PathBuf,
127:     /// The binary's `PATH`: where `tmux`, `kill` and `opencode` are found.
128:     pub path: String,
129: }
130: 
131: /// Guard 5: the private tmux server's whole environment, exactly the rig's isolated pairs.
132: pub fn tmux_server_env(opencode: &Path, oc_root: &Path) -> Vec<(String, String)> {
133:     rig::isolated_vars(opencode, oc_root)
134: }
135: 
136: /// Guard 4: the binary's whole environment, built from nothing inherited, after every name
137: /// and path is checked: OpenCode's scratch `HOME`/`XDG_*` (the rig's), the given `PATH`, a
138: /// UTF-8 `LANG`, deterministic logging, and the wiring's settings. Fails closed on any refusal.
139: pub fn binary_env(settings: &Settings) -> Result<Vec<(String, String)>, String> {
140:     check_session(&settings.session)?;
141:     for path in [
142:         &settings.state_dir,
143:         &settings.herdr_socket,
144:         &settings.tmux_socket,
145:         &settings.oc_root,
146:     ] {
147:         check_inside(&settings.base, path)?;
148:     }
149:     let mut env: Vec<(String, String)> = tmux_server_env(&settings.opencode, &settings.oc_root)
150:         .into_iter()
151:         .filter(|(name, _)| name == "HOME" || name.starts_with("XDG_"))
152:         .collect();
153:     let text = |path: &Path| path.display().to_string();
154:     env.extend(
155:         [
156:             ("PATH", settings.path.clone()),
157:             // Chosen by the test, not inherited: without a UTF-8 locale the tmux client
158:             // writes the tabs of the OpenCode adapter's `display-message` query as `_`, and
159:             // every `attach_tui` then reads its pane as gone (tmux 3.7c; the adapter runs tmux
160:             // without `-u`, a finding for #642's crate).
161:             ("LANG", "C.UTF-8".to_owned()),
162:             ("HOLLER_STATE_DIR", text(&settings.state_dir)),
163:             ("HOLLER_DEBUG", "quiet".to_owned()),
164:             ("HOLLER_LOG_FORMAT", "json".to_owned()),
165:             ("HOLLER_HERDR_SESSION", settings.session.clone()),
166:             ("HOLLER_HERDR_SOCKET", text(&settings.herdr_socket)),
167:             ("HOLLER_HERDR_WORKSPACES", WORKSPACES.to_owned()),
168:             ("HOLLER_TMUX_SOCKET", text(&settings.tmux_socket)),
169:             ("HOLLER_OPENCODE_BIN", text(&settings.opencode)),
170:             ("HOLLER_OPENCODE_SERVE_ARGS", r#"["--pure"]"#.to_owned()),
171:         ]
172:         .into_iter()
173:         .map(|(k, v)| (k.to_owned(), v)),
174:     );
175:     Ok(env)
176: }
```

### E10  `crates/holler-cli/tests/pane_integration/scratch/mod.rs` lines 183-279: Scratch, start (guard 1) and holler (the pane-name guard)

Command: `sed -n '183,279p' crates/holler-cli/tests/pane_integration/scratch/mod.rs | awk -v s=183 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
183: /// One scenario's scratch set. Its drop first signals every process the private tmux server
184: /// runs (guard 6); then the fields drop in order: the hub, then the tmux server (killed by
185: /// its socket), then Herdr (`server stop`), then the state dir, then the directory itself.
186: pub struct Scratch {
187:     pub hub: Option<Hub>,
188:     pub host: TmuxHost,
189:     pub tmux: real_tmux::Server,
190:     pub herdr: ScratchHerdr,
191:     pub state: StateDir,
192:     pub settings: Settings,
193:     pub tmux_bin: PathBuf,
194:     /// What every verb run printed, stdout then stderr (D28).
195:     outputs: Mutex<Vec<String>>,
196:     /// The planted credential (D28).
197:     sentinel: String,
198:     /// The sentinel variables as they were before the run, put back on drop (NV-6).
199:     saved_sentinels: Vec<(&'static str, Option<std::ffi::OsString>)>,
200:     base: TempDir,
201: }
202: 
203: impl Scratch {
204:     /// Guard 1, then the whole set; `None` when the run did not opt in.
205:     pub fn start() -> Option<Scratch> {
206:         let herdr_bin = scratch_herdr::opt_in_as(GATE_VAR)?;
207:         let (opencode, tmux_bin) = rig::binaries();
208:         assert!(
209:             rig::refused(9),
210:             "127.0.0.1:9 accepts connections; the dead-end provider would not be dead. Refusing."
211:         );
212:         let dir = tempfile::Builder::new()
213:             .prefix(PANE_PREFIX)
214:             .tempdir_in("/tmp")
215:             .unwrap();
216:         let base = std::fs::canonicalize(dir.path()).unwrap();
217:         let oc_root = base.join("oc");
218:         rig::lay_out(&oc_root);
219:         let state = StateDir::at(base.join("state"));
220:         let herdr = ScratchHerdr::start_in(&herdr_bin, FLAVOR, &base);
221:         let tmux = real_tmux::isolated(&base, &tmux_server_env(&opencode, &oc_root), ANCHOR);
222:         let host = TmuxHost::new(holler_adapter_host::TmuxSocket::Path(tmux.sock()))
223:             .with_config(tmux.config());
224:         let settings = Settings {
225:             base: base.clone(),
226:             state_dir: state.path().to_path_buf(),
227:             session: herdr.session().to_owned(),
228:             herdr_socket: herdr.socket().to_path_buf(),
229:             tmux_socket: tmux.sock(),
230:             path: format!("{}:{}:/usr/bin:/bin", dir_of(&tmux_bin), dir_of(&opencode)),
231:             opencode,
232:             oc_root,
233:         };
234:         binary_env(&settings).unwrap_or_else(|refusal| panic!("{refusal}"));
235:         let hub = Hub::start(&state);
236:         let sentinel = scratch_herdr::new_name_as("hlr649-sentinel-");
237:         let saved_sentinels = SENTINEL_VARS
238:             .iter()
239:             .map(|&name| (name, std::env::var_os(name)))
240:             .collect();
241:         for name in SENTINEL_VARS {
242:             std::env::set_var(name, &sentinel);
243:         }
244:         Some(Scratch {
245:             hub: Some(hub),
246:             host,
247:             tmux,
248:             herdr,
249:             state,
250:             settings,
251:             tmux_bin,
252:             outputs: Mutex::new(Vec::new()),
253:             sentinel,
254:             saved_sentinels,
255:             base: dir,
256:         })
257:     }
258: 
259:     /// `holler <args>` with exactly [`binary_env`], in the scratch directory. The pane a
260:     /// `pane` verb names (its first positional argument) is checked first (guard 2).
261:     pub fn holler(&self, args: &[&str]) -> Out {
262:         if let ["pane", _, pane, ..] = args {
263:             if !pane.starts_with("--") {
264:                 check_pane_name(pane).unwrap_or_else(|refusal| panic!("{refusal}"));
265:             }
266:         }
267:         let env = binary_env(&self.settings).unwrap_or_else(|refusal| panic!("{refusal}"));
268:         let mut command = Command::new(holler_bin());
269:         command
270:             .env_clear()
271:             .envs(env)
272:             .current_dir(&self.settings.base)
273:             .args(args);
274:         let out = output_of(command);
275:         let mut outputs = self.outputs.lock().unwrap_or_else(PoisonError::into_inner);
276:         outputs.push(out.stdout.clone());
277:         outputs.push(out.stderr.clone());
278:         out
279:     }
```

### E11  `crates/holler-cli/tests/pane_integration/scratch/mod.rs` lines 356-439: json, launch, restart_hub, herdr_snapshot, owned_pids

Command: `sed -n '356,439p' crates/holler-cli/tests/pane_integration/scratch/mod.rs | awk -v s=356 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
356:     /// [`Scratch::holler`] with `--format=json`, its stdout checked against the envelope
357:     /// (AC 9), and the exit code `code`.
358:     pub fn json(&self, args: &[&str], code: i32) -> Out {
359:         let mut argv = args.to_vec();
360:         argv.push("--format=json");
361:         let out = self.holler(&argv);
362:         check_envelope(&out.stdout, out.code)
363:             .unwrap_or_else(|fault| panic!("holler {argv:?}: not one envelope: {fault}: {out:?}"));
364:         assert_eq!(out.code, code, "holler {argv:?}: {out:?}");
365:         out
366:     }
367: 
368:     /// `pane launch` of `pane` in `workspace` at `grid` on `port`, with the dead-end model,
369:     /// then the model guard on its server (D12, W-1 (c)).
370:     pub fn launch(&self, pane: &str, workspace: &str, grid: &str, port: u16, extra: &[&str]) -> Out {
371:         let project = self.settings.oc_root.join("proj").display().to_string();
372:         let policy = format!("fixed:{port}");
373:         let mut argv = vec![
374:             "pane",
375:             "launch",
376:             pane,
377:             "--herdr-session",
378:             self.herdr.session(),
379:             "--project",
380:             &project,
381:             "--workspace",
382:             workspace,
383:             "--grid",
384:             grid,
385:             "--model",
386:             "deadend/none",
387:             "--effort",
388:             "medium",
389:             "--ctx-soft",
390:             "100000",
391:             "--ctx-hard",
392:             "150000",
393:             "--port-policy",
394:             &policy,
395:         ];
396:         argv.extend_from_slice(extra);
397:         let out = self.json(&argv, 0);
398:         rig::no_model_can_be_reached(port);
399:         out
400:     }
401: 
402:     /// Stop the hub and start a new one on the same state dir.
403:     pub fn restart_hub(&mut self) {
404:         if let Some(hub) = self.hub.take() {
405:             hub.stop(Duration::from_secs(5));
406:         }
407:         self.hub = Some(Hub::start(&self.state));
408:     }
409: 
410:     /// Herdr's own snapshot, read by the test through the adapter on the scratch socket.
411:     pub fn herdr_snapshot(&self) -> Vec<HerdrPane> {
412:         let extent = Extent { rows: 2, cols: 1 };
413:         let labels = WORKSPACES
414:             .split(',')
415:             .map(|row| row.split_once('=').map_or(row, |(label, _)| label));
416:         let config = labels.fold(
417:             HerdrConfig::new(self.herdr.session(), self.herdr.socket()),
418:             |config, label| config.with_workspace(label, extent),
419:         );
420:         let adapter = HerdrAdapter::connect(config).expect("connect to the scratch Herdr");
421:         adapter.snapshot().expect("Herdr's snapshot").panes
422:     }
423: 
424:     /// The private tmux server's global environment, as `NAME=value` lines.
425:     pub fn tmux_global_env(&self) -> Vec<String> {
426:         self.tmux.lines(&["show-environment", "-g"])
427:     }
428: 
429:     /// The pids `run` tagged in the tmux session `pane` (its owned processes, #641): what
430:     /// `tmux list-panes` lists for the session's tagged windows.
431:     pub fn owned_pids(&self, pane: &str) -> Vec<u32> {
432:         let target = format!("={pane}");
433:         let out = self.tmux.tmux(&["list-panes", "-s", "-t", &target, "-F", "#{@holler-pid}"]);
434:         String::from_utf8_lossy(&out.stdout)
435:             .lines()
436:             .filter_map(|line| line.trim().parse().ok())
437:             .collect()
438:     }
439: }
```

### E12  `crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs` lines 76-130: Flavor, the gate and the refusal of Herdr's default session

Command: `sed -n '76,130p' crates/holler-adapter-herdr/tests/scratch_herdr/mod.rs | awk -v s=76 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
76: /// The names one family of scratch servers uses: the gate variable that opts in, the
77: /// session-name prefix (followed by 8 lower-case hex digits) and the root's directory-name
78: /// prefix. Both prefixes keep #640's lengths, so [`SOCKET_TAIL`] bounds every flavor.
79: #[derive(Debug, Clone, Copy, PartialEq, Eq)]
80: pub struct Flavor {
81:     pub gate_var: &'static str,
82:     pub name_prefix: &'static str,
83:     pub root_prefix: &'static str,
84: }
85: 
86: /// #640's flavor: `HOLLER_HERDR_SCRATCH`, `holler640-`, `h640.`.
87: pub const HERDR_640: Flavor = Flavor {
88:     gate_var: GATE_VAR,
89:     name_prefix: NAME_PREFIX,
90:     root_prefix: ROOT_PREFIX,
91: };
92: 
93: #[derive(Debug, PartialEq, Eq)]
94: pub enum Gate {
95:     Run,
96:     Skip,
97: }
98: 
99: /// Pure: `Run` exactly when `value` is `Some("1")`.
100: pub fn gate(value: Option<&str>) -> Gate {
101:     if value == Some("1") {
102:         Gate::Run
103:     } else {
104:         Gate::Skip
105:     }
106: }
107: 
108: /// Pure: `Ok` only for `NAME_PREFIX` + 8 of `[0-9a-f]`; `default` is refused by name.
109: pub fn check_name(name: &str) -> Result<(), String> {
110:     check_name_as(NAME_PREFIX, name)
111: }
112: 
113: /// Pure: [`check_name`] for the session prefix `prefix`.
114: pub fn check_name_as(prefix: &str, name: &str) -> Result<(), String> {
115:     if name.eq_ignore_ascii_case("default") {
116:         return Err(format!(
117:             "refusing the Herdr session {name:?}: it is Herdr's default session, never a \
118:              scratch one"
119:         ));
120:     }
121:     let hex = name.strip_prefix(prefix).unwrap_or("");
122:     let is_hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
123:     if name.starts_with(prefix) && hex.len() == 8 && hex.chars().all(is_hex) {
124:         return Ok(());
125:     }
126:     Err(format!(
127:         "refusing the Herdr session {name:?}: a scratch session is {prefix} and 8 \
128:          lower-case hex digits"
129:     ))
130: }
```

### E13  `crates/holler-cli/tests/pane_integration/guards.rs` lines 39-66: the gate and session guards, tested in the default run

Command: `sed -n '39,66p' crates/holler-cli/tests/pane_integration/guards.rs | awk -v s=39 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
39: // --- guard 1: opt-in -----------------------------------------------------------------------
40: 
41: #[test]
42: fn the_gate_runs_only_on_exactly_1() {
43:     assert_eq!(gate(Some("1")), Gate::Run);
44:     for other in [None, Some(""), Some("0"), Some("true"), Some("yes"), Some(" 1"), Some("1 ")] {
45:         assert_eq!(gate(other), Gate::Skip, "{other:?}");
46:     }
47: }
48: 
49: // --- guard 2: unique names -----------------------------------------------------------------
50: 
51: #[test]
52: fn only_a_holler649_session_passes_and_default_is_refused_by_name() {
53:     assert_eq!(check_session("holler649-0123abcd"), Ok(()));
54:     for bad in [
55:         "default",
56:         "Default",
57:         "holler640-0123abcd",
58:         "holler649-0123ABCD",
59:         "holler649-0123abc",
60:         "holler649-0123abcde",
61:         "holler",
62:         "",
63:     ] {
64:         assert!(check_session(bad).is_err(), "{bad:?} is refused");
65:     }
66: }
```

### E14  `crates/holler-cli/tests/pane_integration/guards.rs` lines 97-130: the binary's environment, tested in the default run

Command: `sed -n '97,130p' crates/holler-cli/tests/pane_integration/guards.rs | awk -v s=97 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
97: // --- guard 4: the binary's environment ----------------------------------------------------
98: 
99: #[test]
100: fn the_binary_env_is_built_from_nothing_inherited() {
101:     let env = binary_env(&settings()).unwrap();
102:     let expected: BTreeSet<String> = [
103:         "HOME",
104:         "XDG_DATA_HOME",
105:         "XDG_CONFIG_HOME",
106:         "XDG_STATE_HOME",
107:         "XDG_CACHE_HOME",
108:         "PATH",
109:         "LANG",
110:         "HOLLER_STATE_DIR",
111:         "HOLLER_DEBUG",
112:         "HOLLER_LOG_FORMAT",
113:         "HOLLER_HERDR_SESSION",
114:         "HOLLER_HERDR_SOCKET",
115:         "HOLLER_HERDR_WORKSPACES",
116:         "HOLLER_TMUX_SOCKET",
117:         "HOLLER_OPENCODE_BIN",
118:         "HOLLER_OPENCODE_SERVE_ARGS",
119:     ]
120:     .into_iter()
121:     .map(str::to_owned)
122:     .collect();
123:     assert_eq!(names(&env), expected, "exactly these, no HERDR_*, TMUX or TMUX_PANE");
124:     for name in ["HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME"] {
125:         assert!(value(&env, name).starts_with(&format!("{BASE}/oc/")), "{name} is scratch");
126:     }
127:     assert_eq!(value(&env, "HOLLER_STATE_DIR"), format!("{BASE}/state"));
128:     assert_eq!(value(&env, "LANG"), "C.UTF-8", "a UTF-8 locale for the tmux client");
129:     assert_eq!(value(&env, "HOLLER_HERDR_SESSION"), "holler649-0123abcd");
130: }
```

### E15  `crates/holler-cli/tests/pane_integration/scenario.rs` lines 1-52: scenario.rs header and its private helpers record, session_of_record, delete_strays

Command: `sed -n '1,52p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! The scripted scenario (Scope 2, D10 to D15, D21, D23, D28): the real `holler` binary over
2: //! the real wiring, against a scratch Herdr, a private tmux server and OpenCode with the
3: //! dead-end provider only, all started by the test (`scratch`). Every test is `#[ignore]`
4: //! and runs only opted in:
5: //!
6: //! ```text
7: //! HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1
8: //! ```
9: //!
10: //! No step types into a TUI (I4): every verb acts through the ports, and the test reads
11: //! Herdr's snapshot, tmux's listings and OpenCode through a `HarnessPort` it builds itself
12: //! ([`Scratch::harness`]), never a screen. Raw HTTP is used only where no port can (A's W-G):
13: //! deleting a session, and the model guard's read of the providers. No model can be reached:
14: //! after every server a verb starts, the model guard runs before the next step.
15: 
16: use std::collections::BTreeSet;
17: use std::time::{Duration, Instant};
18: 
19: use holler_pane::ports::Activity;
20: use holler_pane::tx_launch::{wait_healthy, BOOT_POLL};
21: use holler_pane::{AgentKey, GridPos, HarnessPort, HostPort, PaneName, ServeSpec};
22: use holler_pane_testkit::envelope::check_envelope;
23: use serde_json::{json, Value};
24: 
25: use crate::scratch::rig::{free_port, no_model_can_be_reached, raw, refused, signal_group};
26: use crate::scratch::{alive, Scratch};
27: use crate::support::wait_for;
28: 
29: const C1: &str = "hlr649-c1r1";
30: const C2: &str = "hlr649-c2r1";
31: 
32: /// `pane get <pane>`'s record.
33: fn record(scratch: &Scratch, pane: &str) -> Value {
34:     scratch.json(&["pane", "get", pane], 0).data()["pane"].clone()
35: }
36: 
37: fn session_of_record(scratch: &Scratch, pane: &str) -> String {
38:     record(scratch, pane)["session_of_record"]
39:         .as_str()
40:         .unwrap_or_else(|| panic!("{pane} has a session of record"))
41:         .to_owned()
42: }
43: 
44: /// Delete every session on `port` that is no pane's session of record: a reset or a switch
45: /// leaves the previous one on the server as a stray, which doctor reports. The sessions are
46: /// listed through the port; no port deletes one, so the delete is raw HTTP (W-G).
47: fn delete_strays(scratch: &Scratch, port: u16, keep: &BTreeSet<String>) {
48:     let listed = scratch.harness().list_sessions(port).expect("list_sessions");
49:     for session in listed.into_iter().filter(|s| !keep.contains(s)) {
50:         raw(port, "DELETE", &format!("/session/{session}"), None);
51:     }
52: }
```

### E16  `crates/holler-cli/tests/pane_integration/scenario.rs` lines 54-116: switch_once_the_tui_takes_it (#845), findings, assert_tmux_env_is_scratch

Command: `sed -n '54,116p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=54 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
54: /// `pane switch <pane> <target>`, sent again while it answers `timeout`, for at most 20 s.
55: ///
56: /// A finding of this scenario, outside #649's Blast radius: right after `pane relaunch`, the
57: /// relaunched TUI shows its session (so `attach_tui` returns) before it takes
58: /// `/tui/select-session`, and a switch sent in the next few seconds times out in
59: /// `harness.select_session` (seen on OpenCode 1.19.0; one sent 8 s later succeeds). A failed
60: /// switch records nothing (`tx_switch.rs`, step 3), so sending it again is safe. Each retry is
61: /// printed, so the race stays visible in the run's output.
62: fn switch_once_the_tui_takes_it(scratch: &Scratch, pane: &str, target: &str) {
63:     let mut tries = 0;
64:     let done = wait_for(Duration::from_secs(20), || {
65:         tries += 1;
66:         let out = scratch.holler(&["pane", "switch", pane, target, "--format=json"]);
67:         let envelope = check_envelope(&out.stdout, out.code)
68:             .unwrap_or_else(|fault| panic!("pane switch: not one envelope: {fault}: {out:?}"));
69:         match envelope.error {
70:             None => Some(()),
71:             Some(error) if error.code == "timeout" => None,
72:             Some(error) => panic!("pane switch {pane} {target}: {error:?}: {out:?}"),
73:         }
74:     });
75:     assert!(done.is_some(), "pane switch {pane} {target} never succeeded in 20 s");
76:     if tries > 1 {
77:         eprintln!("pane switch {pane}: took {tries} tries (the TUI-readiness race after relaunch)");
78:     }
79: }
80: 
81: /// `pane doctor`'s finding kinds per pane (a finding about no pane is under `-`).
82: fn findings(scratch: &Scratch) -> Vec<(String, String)> {
83:     let data = scratch.json(&["pane", "doctor"], 0).data();
84:     data["findings"]
85:         .as_array()
86:         .cloned()
87:         .unwrap_or_default()
88:         .into_iter()
89:         .map(|f| {
90:             let pane = f["pane"].as_str().unwrap_or("-").to_owned();
91:             (pane, f["kind"].as_str().unwrap_or_default().to_owned())
92:         })
93:         .collect()
94: }
95: 
96: /// The private tmux server's environment is the rig's isolated pairs and nothing else (bar
97: /// the `PWD` tmux adds, the scratch directory), so nothing of the operator's reached any
98: /// OpenCode the scenario started (W-1).
99: fn assert_tmux_env_is_scratch(scratch: &Scratch) {
100:     let allowed: BTreeSet<String> = crate::scratch::tmux_server_env(
101:         &scratch.settings.opencode,
102:         &scratch.settings.oc_root,
103:     )
104:     .into_iter()
105:     .map(|(name, _)| name)
106:     .collect();
107:     let base = scratch.settings.base.display().to_string();
108:     for line in scratch.tmux_global_env() {
109:         let (name, value) = line.trim_start_matches('-').split_once('=').unwrap_or((&line, ""));
110:         if name == "PWD" {
111:             assert_eq!(value, base, "tmux's own PWD is the scratch directory");
112:             continue;
113:         }
114:         assert!(allowed.contains(name), "the tmux server carries {name}, which the test never set");
115:     }
116: }
```

### E17  `crates/holler-cli/tests/pane_integration/scenario.rs` lines 123-126: how a scratch test is marked and opted in

Command: `sed -n '123,126p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=123 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
123: #[test]
124: #[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
125: fn the_scenario_runs_in_order_on_scratch_instances() {
126:     let Some(mut scratch) = Scratch::start() else {
```

### E18  `crates/holler-cli/tests/pane_integration/scenario.rs` lines 207-232: break_both: killing a pane's server

Command: `sed -n '207,232p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=207 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
207: /// Steps 6 and 7 (D21). C2's server killed: SIGKILL to the process group of each pid tmux
208: /// lists as one `run` tagged in C2's session that `HostPort::ps` reports live there. Then C1's
209: /// session of record after the switch, `target`, deleted under its TUI (raw HTTP: no port
210: /// deletes a session). `doctor` finds both.
211: fn break_both(scratch: &Scratch, p1: u16, p2: u16, target: &str) {
212:     let tagged = scratch.owned_pids(C2);
213:     let live = scratch.host.ps(&PaneName::parse(C2).unwrap()).expect("ps");
214:     let server: Vec<u32> = tagged.into_iter().filter(|pid| live.contains(pid)).collect();
215:     assert!(!server.is_empty(), "the server of {C2} runs under its tmux session");
216:     for pid in &server {
217:         signal_group(*pid, "KILL");
218:     }
219:     assert!(wait_for(Duration::from_secs(10), || refused(p2).then_some(())).is_some());
220: 
221:     raw(p1, "DELETE", &format!("/session/{target}"), None);
222: 
223:     let found = findings(scratch);
224:     assert!(
225:         found.contains(&(C2.to_owned(), "server-down".to_owned())),
226:         "doctor finds C2's server down: {found:?}"
227:     );
228:     assert!(
229:         found.iter().any(|(pane, _)| pane == C1),
230:         "doctor reports C1's deleted session: {found:?}"
231:     );
232: }
```

### E19  `crates/holler-cli/tests/pane_integration/scenario.rs` lines 234-274: assert_end_state

Command: `sed -n '234,274p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=234 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
234: /// The end state after the repairs: `doctor` reports no finding, `pane list` equals Herdr's
235: /// snapshot for the scratch panes of the scenario's workspace, and none of them is in the
236: /// unmanaged table.
237: fn assert_end_state(scratch: &Scratch) {
238:     let report = scratch.json(&["pane", "doctor"], 0).data();
239:     assert_eq!(report["findings"], json!([]), "doctor reports no finding: {report}");
240:     let observed: BTreeSet<(String, String)> = scratch
241:         .herdr_snapshot()
242:         .into_iter()
243:         .filter(|p| p.workspace == "hlr649")
244:         .map(|p| (p.pane_id.as_str().to_owned(), json!(p.grid).to_string()))
245:         .collect();
246:     let recorded: BTreeSet<(String, String)> = [C1, C2]
247:         .into_iter()
248:         .map(|pane| {
249:             let herdr = record(scratch, pane)["herdr"].clone();
250:             (herdr["pane_id"].as_str().unwrap().to_owned(), herdr["grid"].to_string())
251:         })
252:         .collect();
253:     assert_eq!(recorded, observed, "pane list equals Herdr's snapshot for the scratch panes");
254: 
255:     // #832's unmanaged table, read through the real wiring (D33): Herdr is readable, and no
256:     // pane a record names is listed as unmanaged.
257:     let list = scratch.json(&["pane", "list"], 0).data();
258:     assert!(
259:         list.get("unmanaged_unavailable").is_none(),
260:         "Herdr is read: {list}"
261:     );
262:     let unmanaged: BTreeSet<String> = list["unmanaged"]
263:         .as_array()
264:         .into_iter()
265:         .flatten()
266:         .map(|pane| pane["pane_id"].as_str().unwrap().to_owned())
267:         .collect();
268:     for (pane_id, _) in &recorded {
269:         assert!(
270:             !unmanaged.contains(pane_id),
271:             "{pane_id} has a record, so is not unmanaged: {list}"
272:         );
273:     }
274: }
```

### E20  `crates/holler-cli/tests/pane_integration/scenario.rs` lines 276-326: #649's profile scenario

Command: `sed -n '276,326p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v s=276 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
276: /// The profile scenario. Create from current; edit through `launch --profile`; `close
277: /// --profile` closes the one named pane and removes its spec; `show` and `list`; `delete`
278: /// refused while other panes of the profile are live, then `--keep-panes`, after which the
279: /// panes keep running with no profile.
280: #[test]
281: #[ignore = "opt-in: scratch Herdr, tmux and OpenCode; HOLLER_TEST_PANE_INTEGRATION=1 (#649)"]
282: fn the_profile_scenario_runs_on_scratch_instances() {
283:     let Some(scratch) = Scratch::start() else {
284:         return;
285:     };
286:     let (p1, p2, p3) = (free_port(), free_port(), free_port());
287:     scratch.launch(C1, "hlr649", "r1c1", p1, &[]);
288:     scratch.launch(C2, "hlr649", "r2c1", p2, &[]);
289: 
290:     scratch.json(&["profile", "create", "hlr649p", "--from-current"], 0);
291:     let members = |scratch: &Scratch| -> Vec<String> {
292:         let shown = scratch.json(&["profile", "show", "hlr649p"], 0).data().to_string();
293:         [C1, C2, "hlr649-p1"]
294:             .into_iter()
295:             .filter(|pane| shown.contains(pane))
296:             .map(str::to_owned)
297:             .collect()
298:     };
299:     assert_eq!(members(&scratch), [C1, C2]);
300: 
301:     scratch.launch("hlr649-p1", "hlr649b", "r1c1", p3, &["--profile", "hlr649p"]);
302:     assert_eq!(members(&scratch), [C1, C2, "hlr649-p1"], "launch --profile adds its spec");
303: 
304:     scratch.json(&["pane", "close", "hlr649-p1", "--profile", "hlr649p"], 0);
305:     assert_eq!(members(&scratch), [C1, C2], "close --profile removes it");
306:     assert!(wait_for(Duration::from_secs(10), || refused(p3).then_some(())).is_some());
307: 
308:     let listed = scratch.json(&["profile", "list"], 0).data().to_string();
309:     assert!(listed.contains("hlr649p"), "{listed}");
310: 
311:     let refusal = scratch.json(&["profile", "delete", "hlr649p"], 3);
312:     assert_eq!(refusal.error_code().as_deref(), Some("profile-has-live-panes"));
313: 
314:     scratch.json(&["profile", "delete", "hlr649p", "--keep-panes"], 0);
315:     let panes = scratch.json(&["pane", "list"], 0).data();
316:     let rows = panes["panes"].as_array().expect("rows");
317:     assert_eq!(rows.len(), 2, "{panes}");
318:     for row in rows {
319:         assert_eq!(row["profile"], Value::Null, "no profile: {row}");
320:     }
321:     for port in [p1, p2] {
322:         assert!(!refused(port), "the server on {port} keeps running");
323:     }
324:     assert!(!scratch.json(&["profile", "list"], 0).data().to_string().contains("hlr649p"));
325:     scratch.assert_no_credential_leaked();
326: }
```

### E21  `crates/holler-cli/tests/support/mod.rs` lines 90-96: StateDir::at

Command: `sed -n '90,96p' crates/holler-cli/tests/support/mod.rs | awk -v s=90 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
90:     /// A state directory at `path`, created now and removed on drop like
91:     /// [`StateDir::new`]'s (#649: the scenario keeps its state dir inside its one scratch
92:     /// directory).
93:     pub fn at(path: PathBuf) -> Self {
94:         std::fs::create_dir_all(&path).expect("create state dir");
95:         Self { path }
96:     }
```

### E22  `crates/holler-cli/tests/support/mod.rs` lines 246-251: Hub::start

Command: `sed -n '246,251p' crates/holler-cli/tests/support/mod.rs | awk -v s=246 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
246: impl Hub {
247:     /// Spawn a hub bound to a free loopback port and wait (≤10 s) for it to
248:     /// report the bound port on stderr.
249:     pub fn start(state: &StateDir) -> Hub {
250:         Self::start_with_env(state, &[])
251:     }
```

### E23  `crates/holler-adapter-opencode/tests/real_opencode/rig.rs` lines 95-121: on_path, binaries, refused

Command: `sed -n '95,121p' crates/holler-adapter-opencode/tests/real_opencode/rig.rs | awk -v s=95 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
95: /// `name` on `PATH`, as an absolute path.
96: pub fn on_path(name: &str) -> Option<PathBuf> {
97:     let path = std::env::var_os("PATH")?;
98:     std::env::split_paths(&path)
99:         .map(|dir| dir.join(name))
100:         .find(|candidate| candidate.is_file())
101: }
102: 
103: /// The binaries: `OPENCODE_BIN` or `opencode` on `PATH`, and `tmux` on `PATH`. Opted in, a
104: /// missing one is a failure, not a skip.
105: pub fn binaries() -> (PathBuf, PathBuf) {
106:     let opencode = std::env::var_os("OPENCODE_BIN")
107:         .map(PathBuf::from)
108:         .or_else(|| on_path("opencode"))
109:         .unwrap_or_else(|| panic!("{OPT_IN}=1, but no opencode: set OPENCODE_BIN or PATH"));
110:     let tmux = on_path("tmux").unwrap_or_else(|| panic!("{OPT_IN}=1, but no tmux on PATH"));
111:     (opencode, tmux)
112: }
113: 
114: /// Whether a connect to `127.0.0.1:port` is refused (nothing listens).
115: pub fn refused(port: u16) -> bool {
116:     let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
117:     matches!(
118:         TcpStream::connect_timeout(&addr, Duration::from_millis(500)),
119:         Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused
120:     )
121: }
```

### E24  `crates/holler-adapter-opencode/tests/real_opencode/rig.rs` lines 131-150: free_port

Command: `sed -n '131,150p' crates/holler-adapter-opencode/tests/real_opencode/rig.rs | awk -v s=131 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
131: /// A port in 48100-48199 that refuses now, can be bound now, and that this process has
132: /// not handed out.
133: pub fn free_port() -> u16 {
134:     static TAKEN: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();
135:     let mut taken = TAKEN
136:         .get_or_init(Mutex::default)
137:         .lock()
138:         .unwrap_or_else(PoisonError::into_inner);
139:     let nanos = std::time::SystemTime::now()
140:         .duration_since(std::time::UNIX_EPOCH)
141:         .map_or(0, |d| d.subsec_nanos());
142:     let start = (nanos ^ std::process::id()) % u32::from(PORT_COUNT);
143:     for i in 0..u32::from(PORT_COUNT) {
144:         let port = FIRST_PORT + u16::try_from((start + i) % u32::from(PORT_COUNT)).unwrap();
145:         if !taken.contains(&port) && refused(port) && bindable(port) {
146:             taken.insert(port);
147:             return port;
148:         }
149:     }
150:     panic!("no free port in 48100-48199");
```

### E25  `crates/holler-adapter-opencode/tests/real_opencode/rig.rs` lines 313-337: no_model_can_be_reached and raw

Command: `sed -n '313,337p' crates/holler-adapter-opencode/tests/real_opencode/rig.rs | awk -v s=313 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
313: /// The model guard: the server on `port` lists exactly the dead-end provider, and
314: /// 127.0.0.1:9 still refuses. Otherwise the test stops here.
315: pub fn no_model_can_be_reached(port: u16) {
316:     let reply = raw(port, "GET", "/config/providers", None);
317:     let providers: Option<Vec<Value>> = serde_json::from_slice::<Value>(&reply.body)
318:         .ok()
319:         .and_then(|v| v.get("providers")?.as_array().cloned())
320:         .map(|list| {
321:             list.iter()
322:                 .map(|p| json!({ "id": p["id"], "url": p["options"]["baseURL"] }))
323:                 .collect()
324:         });
325:     let expected = vec![json!({ "id": "deadend", "url": "http://127.0.0.1:9/v1" })];
326:     assert!(
327:         providers.as_ref() == Some(&expected) && refused(DEAD_END),
328:         "model guard: the server on {port} must list only the dead end on a closed \
329:          127.0.0.1:9; got {providers:?}. Refusing to go on."
330:     );
331: }
332: 
333: /// One raw request to OpenCode (no adapter in between), which must get a reply.
334: pub fn raw(port: u16, method: &str, path: &str, body: Option<&Value>) -> Reply {
335:     http::request(port, method, path, body, Duration::from_secs(10))
336:         .unwrap_or_else(|e| panic!("raw {method} {path} on {port}: {e:?}"))
337: }
```

### E26  `crates/holler-cli/tests/verb_harness/mod.rs` lines 1-13: the in-process harness never builds the real wiring

Command: `sed -n '1,13p' crates/holler-cli/tests/verb_harness/mod.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! The in-process harness for the pane and profile verbs (story #670).
2: //!
3: //! Included by `pane_verbs` and `profile_verbs` through `#[path]`. It runs one verb
4: //! **in this process** over given `Ports` and two captured writers, so a verb story
5: //! tests its routing (text goes to `err`/`out`, JSON is one envelope on `out`) without
6: //! a subprocess. `run_verb_with` takes the ports (a verb story passes the fakes of
7: //! #638); `run_verb` is the same over [`Unwired`], the not-implemented port set, and is
8: //! what a stub verb's case uses. Neither builds the real wiring, so #649 never
9: //! changes what these tests run over. Subprocess helpers stay in `tests/support/`; the
10: //! envelope checks here are deliberately minimal (a `serde_json` parse plus
11: //! `schema_version`, `ok` and `error.code`), and the conformance helper of #638 is
12: //! what later verb stories use for full envelope validation.
13: 
```

### E27  `crates/holler-cli/tests/verb_harness/mod.rs` lines 23-37: unwired_ports: the Ports an in-process test hand-builds

Command: `sed -n '23,37p' crates/holler-cli/tests/verb_harness/mod.rs | awk -v s=23 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
23: static UNWIRED: Unwired = Unwired;
24: 
25: /// The ports every method of which answers `not-implemented` (see [`Unwired`]).
26: pub fn unwired_ports() -> Ports<'static> {
27:     Ports {
28:         pane_store: &UNWIRED,
29:         profile_store: &UNWIRED,
30:         herdr: &UNWIRED,
31:         host: &UNWIRED,
32:         harness: &UNWIRED,
33:         scope: &UNWIRED,
34:         prober: &UNWIRED,
35:     }
36: }
37: 
```

### E28  `crates/holler-cli/src/pane/wiring.rs` lines 30-58: the environment the wiring reads

Command: `sed -n '30,58p' crates/holler-cli/src/pane/wiring.rs | awk -v s=30 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
30: //!
31: //! | Variable | What it sets | When it is unset |
32: //! |---|---|---|
33: //! | `HOLLER_HERDR_SESSION` | the one Herdr session the panes live in (Herdr's `HERDR_SESSION` is not read) | each Herdr call answers `usage` naming it |
34: //! | `HOLLER_HERDR_SOCKET` | that session's socket, an absolute path | `HERDR_SOCKET_PATH`, else `$XDG_CONFIG_HOME/herdr/sessions/<session>/herdr.sock` (`$HOME/.config` for `XDG_CONFIG_HOME`) |
35: //! | `HOLLER_HERDR_WORKSPACES` | each workspace's rows by columns, `label=RxC[,label=RxC]...` (a launch into an unlisted workspace is `unavailable`) | no workspace |
36: //! | `HOLLER_TMUX_SOCKET` | the tmux server of the host and the harness, an absolute path (`tmux` and `kill` come from `PATH`) | tmux's default socket |
37: //! | `HOLLER_OPENCODE_BIN` | the `opencode` binary, an absolute path | `opencode` found on `PATH` (none: each harness call answers `unavailable` naming the variable) |
38: //! | `HOLLER_OPENCODE_SERVE_ARGS` | a JSON array of strings, appended to `opencode serve --port P --hostname 127.0.0.1` | none |
39: //!
40: //! A malformed value (a relative or empty path, a workspace list that does not parse, serve
41: //! arguments that are not a JSON array of strings) is `usage` naming its variable, at the first
42: //! use of the port it configures; a bad `HOLLER_TMUX_SOCKET` fails the host and the harness both,
43: //! and never falls back to the default tmux server. A variable of the table whose value is not
44: //! UTF-8 is read as the empty value, never as unset, so it takes no fallback: the empty value is
45: //! `usage` naming the variable, but for `HOLLER_HERDR_WORKSPACES`, where it is no workspace, so
46: //! the Herdr adapter refuses every launch as it refuses one into an unlisted workspace
47: //! ([`env::Lookup`]). The OpenCode processes run with `ProcessEnv::Inherit`: they inherit the
48: //! tmux server's environment, never the CLI's, so a provider credential reaches a harness server
49: //! only through that tmux server's environment. No variable here carries a credential, and
50: //! nothing of this module puts one on an argv, in a file or in a message.
51: //!
52: //! **The `tui_session` lookup (D8)** answers from the bindings of a launch in progress, then the
53: //! pane registry, then Herdr's labels ([`bind::tui_session`]).
54: //!
55: //! **Bounds (D9).** Each adapter and each registry request keeps its own 10 s bound (I5), and the
56: //! verbs keep their own budgets; there is no flag or variable for a bound.
57: //!
58: //! [`Unwired`] stays: it is the one not-implemented port set, which tests build `Ports` from.
```

### E29  `crates/holler-cli/src/pane/wiring.rs` lines 87-166: Wiring, connect, ports and the private from_env

Command: `sed -n '87,166p' crates/holler-cli/src/pane/wiring.rs | awk -v s=87 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
87: /// Who the profile scope logs every profile write of a wired run as (D1).
88: const ACTOR: &str = "holler pane";
89: 
90: /// The ports a verb runs against, owned (D1).
91: pub struct Wiring {
92:     panes: Arc<dyn PaneStore>,
93:     profiles: Arc<dyn ProfileStore>,
94:     herdr: BindingHerdr,
95:     host: BindingHost,
96:     harness: WiredHarness,
97:     scope: StoreScope,
98:     prober: SystemProber,
99: }
100: 
101: impl Wiring {
102:     /// Build the ports of a run from this process's environment, once the hub has answered.
103:     /// Fails `unavailable`, naming the hub, only when the hub cannot be reached (D2).
104:     ///
105:     /// A variable whose value is not UTF-8 is read as the empty value, never as unset
106:     /// ([`env::Lookup`]): unset, `HOLLER_TMUX_SOCKET` would be tmux's default server, the
107:     /// operator's live one (D6).
108:     pub fn connect() -> Result<Self, PaneError> {
109:         store::reach_hub()?;
110:         let actor = Actor::parse(ACTOR)?;
111:         let lookup = |name: &str| {
112:             std::env::var_os(name).map(|value| value.into_string().unwrap_or_default())
113:         };
114:         Ok(Self::from_env(&lookup, actor))
115:     }
116: 
117:     /// The ports, lent for one verb.
118:     pub fn ports(&self) -> Ports<'_> {
119:         Ports {
120:             pane_store: self.panes.as_ref(),
121:             profile_store: self.profiles.as_ref(),
122:             herdr: &self.herdr,
123:             host: &self.host,
124:             harness: &self.harness,
125:             scope: &self.scope,
126:             prober: &self.prober,
127:         }
128:     }
129: 
130:     /// The ports `env` configures, each configuration error kept for its port's first use. No
131:     /// I/O but the search of `PATH` for `opencode`.
132:     fn from_env(env: Lookup<'_>, actor: Actor) -> Self {
133:         let (panes, profiles) = store::stores();
134:         let herdr_config = env::herdr_config(env);
135:         let session = herdr_config
136:             .as_ref()
137:             .map(|config| config.session.clone())
138:             .unwrap_or_default();
139:         // `move`: the closure owns `herdr_config` from here on, so it is `'static`, as
140:         // `LazyHerdr::new` requires, and each build borrows the config from the closure.
141:         let lazy: Arc<dyn HerdrPort> =
142:             Arc::new(LazyHerdr::new(move || connect_herdr(&herdr_config)));
143:         let bindings = Bindings::new();
144:         let tmux = env::tmux_socket(env);
145:         let lookup = tui_session(
146:             &session,
147:             bindings.clone(),
148:             Arc::clone(&panes),
149:             Arc::clone(&lazy),
150:         );
151:         Self {
152:             herdr: BindingHerdr::new(lazy, bindings.clone()),
153:             host: BindingHost::new(Arc::new(tmux_host(&tmux)), bindings),
154:             harness: WiredHarness::new(opencode_harness(env, &tmux, lookup)),
155:             scope: StoreScope::new(Arc::clone(&profiles), Arc::clone(&panes), actor),
156:             panes,
157:             profiles,
158:             prober: SystemProber,
159:         }
160:     }
161: }
162: 
163: /// The Herdr adapter of `config` (D5), connected now: [`LazyHerdr`] calls this on its first use.
164: fn connect_herdr(config: &Result<HerdrConfig, PaneError>) -> Result<Arc<dyn HerdrPort>, PaneError> {
165:     let adapter = HerdrAdapter::connect(config.clone()?)?;
166:     Ok(Arc::new(adapter))
```

### E30  `crates/holler-pane/src/tx_apply.rs` lines 1-55: the apply transaction: order, health, the six rows, outcomes, bound

Command: `sed -n '1,55p' crates/holler-pane/src/tx_apply.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! The `holler profile apply` transaction (story #664): converge the live panes on a profile P,
2: //! declaratively and idempotently. It creates a missing pane through the launch transaction,
3: //! adopts a healthy pane that matches its spec, relaunches an agent that is not healthy, and
4: //! never kills, rewrites or moves a healthy pane: what it will not change, it reports. It works
5: //! only through the [`Ports`] (ADR-0021 section 5), calls [`tx_launch`] unchanged, and holds no
6: //! I/O of its own.
7: //!
8: //! - **What it looks at** (I6): the registry and live observation, never a file. A spec pane's
9: //!   record is the registry entry of its name, in any profile. A Herdr pane, tmux session or
10: //!   harness server that no record names is never adopted (`holler pane import` records those).
11: //! - **The order of a run.** Read P (`profile-not-found`) and the registry. Without
12: //!   `--herdr-session`, a spec pane with no record (a `create`) is `usage` before any Herdr or
13: //!   harness call, in a dry run too. Observe every recorded spec pane at once. Choose every
14: //!   pane's step: that is the plan, where a dry run stops. Otherwise run the steps one at a time.
15: //! - **Healthy** is observed, never the stored `harness.health`: `health` answers `Ok(true)`,
16: //!   Herdr's snapshot lists the record's Herdr session and pane id (that pair alone, never the
17: //!   cell or the label), and the TUI does not show a session other than the session of record.
18: //!   A shown session that cannot be read (`Ok(None)` or an error) cannot tell, so it does not
19: //!   make a pane unhealthy, and the step's detail says so. A `health` or `snapshot` call that
20: //!   fails makes the step of a recorded pane `failed`, unless row a refuses it.
21: //! - **The steps**, one per spec pane in `(workspace, row, col)` order; the first row that
22: //!   matches applies. (a) A record of another profile, without `--take-over`: `refused`
23: //!   (`pane-in-other-profile`). (b) No record: `create`, by `tx_launch::launch`. (c) Healthy and
24: //!   matching its spec: `adopt`, which records the pane as a member of P and touches nothing
25: //!   live. (d) Healthy and differing: `reported`, with its row of `diff_profile`. (e) Not
26: //!   healthy, the orchestrator: `refused` ([`ORCHESTRATOR_NOT_RELAUNCHED`]). (f) Not healthy, an
27: //!   agent: `relaunch` in place, by `tx_launch::relaunch`. Under `--take-over`, a record of
28: //!   another profile leaves it and then joins P, two writes, before anything else rows c, d
29: //!   and f do; row e never writes.
30: //! - **Outcomes.** A refusal from launch or relaunch is a `refused` step with its code, and a
31: //!   `usage` answer a `refused` step coded [`SPEC_NOT_LAUNCHABLE`]; the run goes on. A
32: //!   failure-class answer is a `failed` step, which stops the run: the steps after it are
33: //!   `skipped`. A `refused` or `failed` step whose launch or relaunch had started its act (a
34: //!   refusal can come from inside the act: the port's re-check, `verify_serving`) is marked
35: //!   [`Step::acted`], for the reconcile step the verb prints. In a dry run a step that would act
36: //!   is `planned`, and the run calls only its reads: no probe, no split, no write.
37: //! - **Each step re-reads** its record and P before it acts, so a stale spec is never written
38: //!   back: the spec handed to launch or relaunch is P's at that moment, and a membership write
39: //!   is a compare-and-swap at the generation re-read (a conflict is a `failed` step).
40: //! - **Bound (I5).** A run takes, in order: its two reads (P and the registry), each within I5's
41: //!   per-call bound; the up-front observation, whose calls (Herdr's snapshot, and each recorded
42: //!   spec pane's `health` and `shown_session`) all run at once, one thread each, so it takes at
43: //!   most one port call's own bound, I5's per-call bound (default 10 s); then the steps in
44: //!   turn, each under its own I5 bound. A `create` or `relaunch` step is one `tx_launch` run
45: //!   under the `TxOptions` budget its caller passes (the verb passes
46: //!   [`DEFAULT_BUDGET`](crate::tx_launch::DEFAULT_BUDGET)), which starts when that run starts,
47: //!   so it takes at most the budget plus one port call's own bound plus its rollback calls. An
48: //!   `adopt`, or a take-over's move, makes at most two record writes. Each of those steps first
49: //!   re-reads its record and P, two calls; a step refused or failed at planning calls nothing.
50: //!   There is no overall cap: a run of N steps takes at most 10 s for the observation plus N
51: //!   step bounds, beyond its reads.
52: //!
53: //! Untrusted text (a spec's pane, profile names, session and Herdr ids) reaches a message only
54: //! through `findings::quoted`. A [`PaneName`] is checked text (1 to 32 lowercase letters, digits
55: //! and inner `-`, `SessionName`'s grammar), so it is printed as it is.
```

### E31  `crates/holler-pane/src/tx_apply.rs` lines 207-235: apply()

Command: `sed -n '207,235p' crates/holler-pane/src/tx_apply.rs | awk -v s=207 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
207: /// Converge the live panes on `request.profile` (see the module docs). `Err` is an error with no
208: /// plan: `usage` for a create without `--herdr-session`, `profile-not-found`, or a read of P or
209: /// the registry that fails. Every step's outcome, `failed` included, is in `Ok`.
210: pub fn apply(
211:     ports: Ports<'_>,
212:     request: &ApplyRequest,
213:     options: &TxOptions,
214: ) -> Result<Applied, PaneError> {
215:     let profile = stored_profile(ports, &request.profile)?;
216:     let records = spec_records(&profile, ports.pane_store.list()?);
217:     require_herdr_session(request, &profile, &records)?;
218:     let observed = observe::observe(ports, &records);
219:     let planned = plan(&profile, &records, &observed, request.take_over);
220:     let steps = if request.dry_run {
221:         until_failed(&planned, Planned::dry)
222:     } else {
223:         let cx = run::Cx {
224:             ports,
225:             request,
226:             options,
227:         };
228:         until_failed(&planned, |planned| run::step(&cx, planned))
229:     };
230:     Ok(Applied {
231:         profile: profile.name,
232:         dry_run: request.dry_run,
233:         steps,
234:     })
235: }
```

### E32  `crates/holler-pane/src/tx_apply.rs` lines 257-283: a create without --herdr-session is usage, before any call

Command: `sed -n '257,283p' crates/holler-pane/src/tx_apply.rs | awk -v s=257 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
257: /// §4's gate, before any observation: a spec pane with no record is a `create`, which needs the
258: /// Herdr session, so without `--herdr-session` the run is `usage` and no step runs. A spec whose
259: /// pane is not a pane name is refused at planning, never created, so it needs no session.
260: fn require_herdr_session(
261:     request: &ApplyRequest,
262:     profile: &Profile,
263:     records: &[Pane],
264: ) -> Result<(), PaneError> {
265:     if request.herdr_session.is_some() {
266:         return Ok(());
267:     }
268:     let creates: Vec<String> = profile
269:         .panes
270:         .iter()
271:         .filter(|spec| PaneName::parse(&spec.pane).is_ok())
272:         .filter(|spec| !records.iter().any(|r| r.name.as_str() == spec.pane))
273:         .map(|spec| quoted(&spec.pane))
274:         .collect();
275:     if creates.is_empty() {
276:         return Ok(());
277:     }
278:     Err(PaneError::Usage {
279:         message: format!(
280:             "--herdr-session is required: apply creates the spec panes that have no record ({}), \
281:              and the flag names the Herdr session they go in",
282:             creates.join(", ")
283:         ),
```

### E33  `crates/holler-pane/src/tx_apply.rs` lines 329-396: plan, cell_key, choose, in_other_profile

Command: `sed -n '329,396p' crates/holler-pane/src/tx_apply.rs | awk -v s=329 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
329: /// Every spec's row (§3), in `(workspace, row, col)` order: P's order is not the steps' (C6).
330: /// The comparison is `diff_profile`'s, whose rows come one per spec in P's order, so they pair
331: /// with the specs; a sort by cell is stable, so specs of one cell keep P's order.
332: fn plan(profile: &Profile, records: &[Pane], observed: &Observed, take_over: bool) -> Vec<Planned> {
333:     let mut chosen: Vec<(&ProfileSpec, Planned)> = profile
334:         .panes
335:         .iter()
336:         .zip(diff_profile(profile, records))
337:         .map(|(spec, diff)| {
338:             let row = choose(spec, diff, records, observed, &profile.name, take_over);
339:             let pane = spec.pane.clone();
340:             (spec, Planned { pane, row })
341:         })
342:         .collect();
343:     chosen.sort_by(|(a, _), (b, _)| cell_key(a).cmp(&cell_key(b)));
344:     chosen.into_iter().map(|(_, planned)| planned).collect()
345: }
346: 
347: /// A spec's place in the run: its workspace, then its row and column.
348: fn cell_key(spec: &ProfileSpec) -> (&str, u16, u16) {
349:     let grid = spec.herdr.grid;
350:     (spec.herdr.workspace.as_str(), grid.row, grid.col)
351: }
352: 
353: /// The row of `spec`, whose row of `diff_profile` is `diff`: the first of the step table that
354: /// matches (§3). Row a is checked before any observation is looked at (C1, R5 B-3).
355: fn choose(
356:     spec: &ProfileSpec,
357:     diff: PaneDiff,
358:     records: &[Pane],
359:     observed: &Observed,
360:     profile: &ProfileName,
361:     take_over: bool,
362: ) -> Row {
363:     let name = match PaneName::parse(&spec.pane) {
364:         Ok(name) => name,
365:         Err(error) => return Row::Refuse(not_launchable(&error)),
366:     };
367:     let Some((record, seen)) = records
368:         .iter()
369:         .zip(&observed.panes)
370:         .find(|(record, _)| record.name == name)
371:     else {
372:         return Row::Create(name);
373:     };
374:     if in_other_profile(record, profile) && !take_over {
375:         return Row::Refuse(other_profile(record, profile));
376:     }
377:     match observe::judge(record, seen, &observed.snapshot) {
378:         Err(error) => Row::Unobserved(error),
379:         Ok(Judged::Healthy { note }) if diff.status == PaneStatus::Matches => {
380:             Row::Adopt { name, note }
381:         }
382:         Ok(Judged::Healthy { note }) => Row::Report { name, diff, note },
383:         Ok(Judged::Unhealthy(why)) if record.role == PaneRole::Orchestrator => {
384:             Row::Refuse(orchestrator(&name, &why))
385:         }
386:         Ok(Judged::Unhealthy(why)) => Row::Relaunch { name, why },
387:     }
388: }
389: 
390: /// Whether `record` belongs to a profile other than `profile` (slugs compared, as membership is).
391: fn in_other_profile(record: &Pane, profile: &ProfileName) -> bool {
392:     record.profile.is_some() && !is_member(record, profile)
393: }
394: 
395: /// Row a's refusal, and a step's when its pane moved to another profile during the run.
396: fn other_profile(record: &Pane, profile: &ProfileName) -> PaneError {
```

### E34  `crates/holler-pane/src/tx_apply/run.rs` lines 49-136: create, adopt, report, relaunch

Command: `sed -n '49,136p' crates/holler-pane/src/tx_apply/run.rs | awk -v s=49 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
49: /// Row b: launch the pane from P's spec as re-read, in the Herdr session the run was given. A
50: /// record that appeared since the plan is launch's to refuse (`pane-exists`).
51: fn create(cx: &Cx<'_>, planned: &Planned, name: &PaneName) -> Result<Step, Halt> {
52:     let fresh = Fresh::read(cx, name)?;
53:     let request = LaunchRequest {
54:         name: name.clone(),
55:         herdr_session: cx.request.herdr_session.clone(),
56:         spec: fresh.spec,
57:         profile: Some(fresh.profile),
58:         spec_only: false,
59:     };
60:     tx_launch::launch(cx.ports, &request, cx.options)?;
61:     Ok(planned.step(Action::Create, Outcome::Created))
62: }
63: 
64: /// Row c: set `Pane.profile` to P if it is not already, on the record as re-read (C10): no write
65: /// when it is in P, the join when it is in no profile, and under `--take-over` the leave and then
66: /// the join when it is in another. Nothing live is touched.
67: fn adopt(cx: &Cx<'_>, planned: &Planned, name: &PaneName) -> Result<Step, Halt> {
68:     let fresh = Fresh::read(cx, name)?;
69:     let record = fresh.record(name)?;
70:     let profile = &fresh.profile;
71:     if !is_member(&record, profile) {
72:         let pane_store = cx.ports.pane_store;
73:         match &record.profile {
74:             None => {
75:                 join(pane_store, &record, profile)?;
76:             }
77:             Some(_) if cx.request.take_over => {
78:                 take_over(pane_store, &record, profile)?;
79:             }
80:             Some(_) => return Err(other_profile(&record, profile).into()),
81:         }
82:     }
83:     Ok(planned.step(Action::Adopt, Outcome::Adopted))
84: }
85: 
86: /// Row d: report the pane's difference from its spec, its row of `diff_profile` from the plan.
87: /// Under `--take-over` a pane of another profile moves to P first (the leave, then the join);
88: /// otherwise nothing is written. Nothing live is touched.
89: fn report(cx: &Cx<'_>, planned: &Planned, name: &PaneName, diff: &PaneDiff) -> Result<Step, Halt> {
90:     let fresh = Fresh::read(cx, name)?;
91:     let record = fresh.record(name)?;
92:     if cx.request.take_over && in_other_profile(&record, &fresh.profile) {
93:         take_over(cx.ports.pane_store, &record, &fresh.profile)?;
94:     }
95:     Ok(Step {
96:         diff: Some(diff.clone()),
97:         ..planned.step(Action::Report, Outcome::Reported)
98:     })
99: }
100: 
101: /// Row f: relaunch the agent in place (`grid_given: false`) from P's spec as re-read, through
102: /// `tx_launch::relaunch` unchanged. Under `--take-over` a pane of another profile leaves it and
103: /// joins P first, so the relaunch's record is the one the join stored, whose generation its
104: /// record write expects (C11); if the relaunch then fails, the pane stays in P and nothing is
105: /// compensated (O1). Without a take-over the relaunch's own write sets `Pane.profile` to P.
106: fn relaunch(cx: &Cx<'_>, planned: &Planned, name: &PaneName) -> Result<Step, Halt> {
107:     let fresh = Fresh::read(cx, name)?;
108:     let record = fresh.record(name)?;
109:     let profile = fresh.profile;
110:     let (record, moved) = if cx.request.take_over && in_other_profile(&record, &profile) {
111:         let from = record.profile.clone();
112:         (take_over(cx.ports.pane_store, &record, &profile)?, from)
113:     } else {
114:         (record, None)
115:     };
116:     let request = RelaunchRequest {
117:         record,
118:         spec: fresh.spec,
119:         grid_given: false,
120:         profile: Some(profile.clone()),
121:         spec_only: false,
122:     };
123:     if let Err(failure) = tx_launch::relaunch(cx.ports, &request, cx.options) {
124:         let note = moved.map(|from| {
125:             let (from, to) = (quoted(from.as_str()), quoted(profile.as_str()));
126:             format!(
127:                 "{name} left profile {from} and joined {to} before the relaunch, and stays in {to}"
128:             )
129:         });
130:         return Err(Halt {
131:             note,
132:             ..Halt::from(failure)
133:         });
134:     }
135:     Ok(planned.step(Action::Relaunch, Outcome::Relaunched))
136: }
```

### E35  `crates/holler-pane/src/tx_apply/observe.rs` lines 106-162: judge and shown: what healthy means

Command: `sed -n '106,162p' crates/holler-pane/src/tx_apply/observe.rs | awk -v s=106 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
106: /// What `seen` and the snapshot say of `record`'s health (§2, C9, C12). `Err` when a call its
107: /// row needs failed (a `failed` step, C2 and C3): its `health`, or Herdr's snapshot, which every
108: /// recorded pane that row a does not refuse needs.
109: pub(super) fn judge(
110:     record: &Pane,
111:     seen: &Seen,
112:     snapshot: &Result<HerdrSnapshot, PaneError>,
113: ) -> Result<Judged, PaneError> {
114:     let answers = seen
115:         .health
116:         .clone()
117:         .map_err(|error| as_failure(HARNESS_HEALTH, record, error))?;
118:     let snapshot = snapshot
119:         .as_ref()
120:         .map_err(|error| as_failure(HERDR_SNAPSHOT, record, error.clone()))?;
121:     if !answers {
122:         let port = record.harness.port;
123:         return Ok(Judged::Unhealthy(format!(
124:             "its harness server on port {port} did not answer"
125:         )));
126:     }
127:     if !listed(snapshot, record) {
128:         return Ok(Judged::Unhealthy(format!(
129:             "Herdr did not list its pane {} in session {}",
130:             quoted(record.herdr.pane_id.as_str()),
131:             quoted(&record.herdr.session)
132:         )));
133:     }
134:     Ok(shown(record, &seen.shown))
135: }
136: 
137: /// §2's third condition, from the shown session alone (C12, the decision on O2): only a session
138: /// other than the session of record makes the pane unhealthy; `Ok(None)` and an error cannot
139: /// tell, which the step's detail says. Every reason is in the past tense: it is what the
140: /// observation saw, before the step ran.
141: fn shown(record: &Pane, shown: &Result<Option<String>, PaneError>) -> Judged {
142:     let of_record = record.session_of_record.as_deref();
143:     let untold = match shown {
144:         Ok(Some(session)) if shown_differs(of_record, Some(session.as_str())) => {
145:             let record_text = of_record.map_or_else(|| "none".to_owned(), quoted);
146:             return Judged::Unhealthy(format!(
147:                 "its TUI showed session {}, not its session of record {record_text}",
148:                 quoted(session)
149:             ));
150:         }
151:         Ok(Some(_)) => None,
152:         Ok(None) => Some("the TUI showed no session: its home screen, or no TUI".to_owned()),
153:         Err(error) => Some(format!("{HARNESS_SHOWN_SESSION} failed ({})", error.code())),
154:     };
155:     let note = untold.zip(of_record).map(|(why, of_record)| {
156:         format!(
157:             "could not tell whether its TUI shows its session of record {} ({why}); not acted on",
158:             quoted(of_record)
159:         )
160:     });
161:     Judged::Healthy { note }
162: }
```

### E36  `crates/holler-pane/src/tx_launch.rs` lines 1-20: launch plan: the probe is a plan step

Command: `sed -n '1,20p' crates/holler-pane/src/tx_launch.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! The launch/relaunch transaction (plan, act, observe, record; I3 and I8), story #644: the
2: //! engine `holler pane launch` and `holler pane relaunch` run, and `profile apply` (#664) will.
3: //! It works only through the [`Ports`] (ADR-0021 section 5) and holds no I/O of its own.
4: //!
5: //! - **Plan**, writing nothing and changing nothing live: the spec names the pane; a launched
6: //!   name has no record (`pane-exists`) and its Herdr session is given; the port policy names a
7: //!   port ([`port_of_policy`]); the harness builds the server's argv (`serve_command`, pure, so
8: //!   its refusal, `agent-unsupported` say, writes nothing); the probe passes when the spec has a
9: //!   check (a failing result is not stored); Herdr's version is supported; the target cell holds
10: //!   no Herdr pane (`grid-occupied`, decided by Herdr's snapshot alone, recorded or not: a pane is
11: //!   never adopted); and no server answers on the port (`port-in-use`, never adopted).
12: //! - **Act**, inside `ProfileScope::edit_spec` (I8: with a profile, P is written first and its
13: //!   specs are put back if the act fails): ensure the Herdr pane and the tmux session, run the
14: //!   spec's command there, then start the server from the planned argv and check it (both runs
15: //!   in the spec's directory; the child module `server`, #802), create the session of record
16: //!   through the harness API (never a "ping" session, never "the most recent"), attach the TUI to
17: //!   exactly that session, and observe that the TUI shows it and Herdr still lists the pane (a
18: //!   mismatch is `unavailable`). Only then is the record written, by one compare-and-swap, with no
19: //!   server pid: the server is a process of the pane's tmux session.
20: //! - **Rollback.** A failed step stops the tmux session's processes, the harness server among
```

### E37  `crates/holler-pane/src/tx_launch.rs` lines 336-358: observe: the probe comes first, before any live change

Command: `sed -n '336,358p' crates/holler-pane/src/tx_launch.rs | awk -v s=336 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
336:     /// The plan's reads, none of which writes or changes anything live: first the server's argv
337:     /// (pure, so its refusal stays in the plan; #802), then steps 3 to 6: the probe, Herdr's
338:     /// version, the target cell (where a relaunch's own pane is no occupant) and the port, which
339:     /// for a relaunch is checked only when it changes (its own server answers there until B1).
340:     fn observe(&self, ports: Ports<'_>, probe_timeout: Duration) -> Result<Seen, PaneError> {
341:         let server = ports
342:             .harness
343:             .serve_command(self.name, self.port, &self.serve)?;
344:         let probe = probe(ports, self.spec, probe_timeout)?;
345:         let herdr_api_version = ports.herdr.version()?;
346:         let listed = ports.herdr.snapshot()?.panes;
347:         let own = self.old.map(|old| &old.herdr);
348:         refuse_occupied(ports, &listed, &self.target, own)?;
349:         let must_be_free = self.old.is_none_or(|old| old.harness.port != self.port);
350:         if must_be_free && ports.harness.health(self.port)? {
351:             return Err(port_in_use(self.port));
352:         }
353:         Ok(Seen {
354:             server,
355:             probe,
356:             herdr_api_version,
357:             listed,
358:         })
```

### E38  `crates/holler-pane/src/tx_launch.rs` lines 474-492: probe(): a failing check is probe-failed

Command: `sed -n '474,492p' crates/holler-pane/src/tx_launch.rs | awk -v s=474 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
474: /// Step 3: the spec's health check, when it has one. A pass is kept for the record; a failure
475: /// refuses the run, quoting only the expected strings or the prober's reason, never the output.
476: fn probe(
477:     ports: Ports<'_>,
478:     spec: &ProfileSpec,
479:     timeout: Duration,
480: ) -> Result<Option<ProbeResult>, PaneError> {
481:     let Some(check) = &spec.check else {
482:         return Ok(None);
483:     };
484:     let message = match ports.prober.run_probe(check, &spec.expect, timeout) {
485:         ProbeResult::Ok => return Ok(Some(ProbeResult::Ok)),
486:         ProbeResult::Failed { missing } => {
487:             let missing: Vec<String> = missing.iter().map(String::as_str).map(quoted).collect();
488:             format!("missing {}", missing.join(", "))
489:         }
490:         ProbeResult::Error(reason) => embedded(&reason),
491:     };
492:     Err(PaneError::ProbeFailed { message })
```

### E39  `crates/holler-pane/src/probe.rs` lines 58-92: run_probe: argv never a shell, PATH, the caller's environment, the verdict

Command: `sed -n '58,92p' crates/holler-pane/src/probe.rs | awk -v s=58 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
58: /// Run the health probe `argv` (never through a shell) and look for every string of
59: /// `expect` in its output, giving up after `timeout`.
60: ///
61: /// - **How it runs.** `argv[0]` is the program, looked up on `PATH`, and every other element
62: ///   is one argument, never joined or split. stdin is null, stdout is read, stderr is
63: ///   discarded, and the environment and working directory are the caller's. On Unix the
64: ///   process gets a process group of its own.
65: /// - **The verdict.** [`ProbeResult::Ok`] only when the program exited 0 and every `expect`
66: ///   string occurs in its stdout (as bytes, case-sensitive, so output that is not UTF-8
67: ///   still matches; an empty string always does). [`ProbeResult::Failed`] when it exited 0
68: ///   without some of them, listed in `expect` order. Everything else is
69: ///   [`ProbeResult::Error`], whatever the output: an empty argv, a zero timeout or one too
70: ///   large for the clock (no process is started), a program that cannot be started, a
71: ///   non-zero exit or an end by a signal, the deadline passing, or more than 1 MiB of
72: ///   stdout.
73: /// - **The reasons** are fixed texts. None echoes an argv element (a check may carry a
74: ///   token) or a byte of output; a spawn failure is named by its `io::ErrorKind` alone.
75: /// - **The bound.** One deadline, `timeout` after the call. When it passes, or the cap is
76: ///   hit, the runner kills the probe's whole process group with `KILL`, then the leader,
77: ///   and reaps the leader, within one cleanup budget of 1 s: it returns at the deadline
78: ///   plus at most 1 s of cleanup (normally a few ms). A probe whose background child keeps
79: ///   stdout open after the leader exits is a timeout, never a verdict.
80: /// - **The group kill is contingent on the `kill` program.** It runs `kill` from `PATH`:
81: ///   if it cannot be spawned, only the leader is ended (`Child::kill`), its children are
82: ///   left, and the runner still returns within the same bound. Off Unix only the leader is
83: ///   ended.
84: /// - **No signal once the leader may have been reaped.** The runner signals only while the
85: ///   leader is unreaped, so its pid cannot belong to another process. An exit status that
86: ///   cannot be read (a `try_wait` error: something else in this process reaped the leader)
87: ///   is `Error`, no signal is sent, and any child still in the group is left.
88: /// - **Long-lived callers.** A probe whose child escapes the group (`setsid`, a double fork)
89: ///   and keeps stdout open leaves one reader thread and its pipe blocked after the runner
90: ///   returns, until that child closes the pipe. A long-lived caller that reuses
91: ///   [`crate::SystemProber`] from `spawn_blocking` or a thread can so leak one thread per
92: ///   such run, without bound; today's callers are short-lived CLI verbs.
```

### E40  `crates/holler-adapter-herdr/src/plan.rs` lines 1-36: the split planner

Command: `sed -n '1,36p' crates/holler-adapter-herdr/src/plan.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! The split planner (epic #633; spike section 7): the `right` and `down` splits that
2: //! take a workspace to one holding every cell asked for, or the reason Herdr cannot.
3: //!
4: //! Herdr places no pane at a cell. A pane is made by splitting one, and a split divides
5: //! only the split pane's own cell (the nesting rule). So the plan is built the way the
6: //! spike built its 2x4 grid (`scripts/spikes/herdr-grid.sh`), one new cell per step and
7: //! never a pane that was not asked for:
8: //!
9: //! 1. An empty workspace starts at `r1c1`, the root pane of `workspace.create`.
10: //! 2. New rows come first, top to bottom. Row `r` is a `down` split of `r(r-1)c1`, the
11: //!    first pane of the last row, and that row must hold one pane: a `down` split of a
12: //!    pane in a wider row nests inside its cell, which is not a grid.
13: //! 3. Then each row's new columns, left to right. `rRcC` is a `right` split of
14: //!    `rRc(C-1)`, which must be the last pane of its row.
15: //!
16: //! A split's `ratio` is the share of its cell that the split pane keeps (Herdr's
17: //! `first` child): `1/(rows - r + 2)` when it makes row `r`, and `1/(cols - c + 2)` when
18: //! it makes column `c`, where `rows` and `cols` are the extent's. A whole extent built
19: //! this way has equal rows and equal columns (the spike's recipe). Ratios never change a
20: //! position (`crate::layout`).
21: //!
22: //! The checks, in order:
23: //!
24: //! - A cell outside the extent is `grid-out-of-range`, before anything else.
25: //! - A cell that exists is answered with no step, even in a workspace that is not a grid.
26: //!   A cell asked for twice is planned once.
27: //! - Any other cell is [`GRID_UNREACHABLE`] when the workspace is not a grid (it has an
28: //!   unplaced pane), or when no single split makes it: it needs a pane that is neither
29: //!   there nor asked for, or it is in a new row below a row of more than one pane.
30: //!
31: //! A refusal returns no plan at all, never the steps before it.
32: 
33: use holler_pane::error::RefusalCode;
34: use holler_pane::{GridPos, PaneError};
35: 
36: use crate::layout::{Direction, GridMap};
```

### E41  `crates/holler-adapter-herdr/src/plan.rs` lines 75-99: plan_splits

Command: `sed -n '75,99p' crates/holler-adapter-herdr/src/plan.rs | awk -v s=75 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
75: pub fn plan_splits(existing: &GridMap, target: &Target) -> Result<Vec<Step>, PaneError> {
76:     let extent = target.extent;
77:     let mut cells = target.cells.clone();
78:     cells.sort_unstable_by_key(|cell| (cell.row, cell.col));
79:     cells.dedup();
80:     if let Some(&outside) = cells.iter().find(|cell| !contains(extent, **cell)) {
81:         return Err(out_of_range(outside, extent));
82:     }
83:     cells.retain(|cell| existing.at(*cell).is_none());
84:     let Some(&first) = cells.first() else {
85:         return Ok(Vec::new());
86:     };
87:     let unplaced = existing.unplaced().len();
88:     if unplaced > 0 {
89:         return Err(refuse(
90:             first,
91:             &format!(
92:                 "the workspace is not a grid ({} in a split nested inside one cell), and no \
93:                  pane is added to a workspace that is not one",
94:                 count(unplaced, "pane")
95:             ),
96:         ));
97:     }
98:     // A new cell in column 1 is a new row; the rest are new columns. Both stay in order.
99:     let (rows, cols): (Vec<GridPos>, Vec<GridPos>) =
```

### E42  `crates/holler-adapter-herdr/src/plan.rs` lines 168-205: below(): a new row only below a row of one pane

Command: `sed -n '168,205p' crates/holler-adapter-herdr/src/plan.rs | awk -v s=168 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
168:     /// A cell of a row that does not exist yet. Only the first cell of the next row is
169:     /// one split away: a `down` split of the first pane of the last row, which must hold
170:     /// one pane.
171:     fn below(&self, cell: GridPos, extent: Extent) -> Result<Step, PaneError> {
172:         let last = self.rows();
173:         let above = GridPos {
174:             row: cell.row.saturating_sub(1),
175:             col: 1,
176:         };
177:         if above.row != last {
178:             return Err(gap(cell, above));
179:         }
180:         let width = self.width(last).unwrap_or(0);
181:         if width != 1 {
182:             return Err(refuse(
183:                 cell,
184:                 &format!(
185:                     "a row can be added only below a row of one pane, and row {last} has {} \
186:                      (a down split there nests inside one cell, which is not a grid); place \
187:                      rows before columns",
188:                     count(usize::from(width), "pane")
189:                 ),
190:             ));
191:         }
192:         if cell.col != 1 {
193:             let left = GridPos {
194:                 row: cell.row,
195:                 col: cell.col.saturating_sub(1),
196:             };
197:             return Err(gap(cell, left));
198:         }
199:         Ok(Step::Split {
200:             from: above,
201:             direction: Direction::Down,
202:             ratio: share(extent.rows, cell.row),
203:             creates: cell,
204:         })
205:     }
```

### E43  `crates/holler-adapter-herdr/src/adapter.rs` lines 224-232: ensure plans one cell at a time

Command: `sed -n '224,232p' crates/holler-adapter-herdr/src/adapter.rs | awk -v s=224 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
224:     fn ensure(&self, spec: &HerdrSpec, deadline: Instant) -> Result<HerdrPane, PaneError> {
225:         let extent = self.extent_of(spec)?;
226:         let workspace = self.workspace_grid(&spec.workspace, deadline)?;
227:         let target = Target {
228:             extent,
229:             cells: vec![spec.grid],
230:         };
231:         let (pane_id, label) = match plan_splits(&workspace.map, &target)?.as_slice() {
232:             // An occupied cell: its pane, and no request that changes anything.
```

### E44  `crates/holler-adapter-herdr/src/layout.rs` lines 71-84: GridMap, built only from Herdr's layout tree

Command: `sed -n '71,84p' crates/holler-adapter-herdr/src/layout.rs | awk -v s=71 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
71: /// The rows of slots read off a split tree by [`grid_of`]. The default is an empty
72: /// workspace: no tree and no pane.
73: #[derive(Debug, Clone, Default, PartialEq)]
74: pub struct GridMap {
75:     /// The rows in tree order. A row holds at least one slot, and a slot at least one
76:     /// pane, so the map holds a pane exactly when it holds a row.
77:     rows: Vec<Vec<Slot>>,
78:     /// The panes of every unplaced slot, in tree order.
79:     unplaced: Vec<PaneId>,
80: }
81: 
82: impl GridMap {
83:     /// The pane at `cell`; `None` for a free cell or an unplaced slot.
84:     pub fn at(&self, cell: GridPos) -> Option<&PaneId> {
```

### E45  `crates/holler-cli/src/profile/apply.rs` lines 1-22: the verb: output and exit rules

Command: `sed -n '1,22p' crates/holler-cli/src/profile/apply.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! `holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]` (story #664, epic
2: //! #633): converge the live panes on a profile, declaratively and idempotently.
3: //!
4: //! The verb types the profile's name (`usage`), runs the engine
5: //! (`holler_pane::tx_apply::apply`, with `TxOptions::default()`, as `pane launch` does) and
6: //! prints what it answered, by its exit code:
7: //!
8: //! - **0**: the steps. In JSON the envelope's `data` is the engine's `{"profile", "dry_run",
9: //!   "steps"}` itself; for a dry run those steps are the plan.
10: //! - **1, a `failed` step** (the decision on O6): only the failure, coded as that step's error,
11: //!   with the message `apply stopped at <pane> (<action>): <step error>; <N> later step(s)
12: //!   skipped`. No plan and no `skipped` list is printed, for a dry run too; the engine's result
13: //!   holds them.
14: //! - **An error before any plan** (a bad name, P not found, a registry that cannot be read, a
15: //!   `create` without `--herdr-session`): that error alone.
16: //!
17: //! **The reconcile step.** A step whose launch or relaunch had started its act before it was
18: //! refused or failed (since #802 the act can refuse: the port's re-check, `verify_serving`) may
19: //! leave the records and the live panes apart. So the verb ends that step's detail with the
20: //! reconcile step of P, as `pane launch` ends an acted failure: a `refused` step prints it with
21: //! its detail, and a `failed` step's message above carries it (ADR-0021 section 8, step 6).
22: 
```

### E46  `crates/holler-cli/src/profile/apply.rs` lines 52-65: the flags

Command: `sed -n '52,65p' crates/holler-cli/src/profile/apply.rs | awk -v s=52 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
52: #[derive(Args, Debug)]
53: pub struct ProfileApply {
54:     /// The profile's name (spaces allowed).
55:     pub name: String,
56:     /// Print the plan and change nothing: no probe, no split, no write.
57:     #[arg(long)]
58:     pub dry_run: bool,
59:     /// Take over a pane that belongs to another profile.
60:     #[arg(long)]
61:     pub take_over: bool,
62:     /// The Herdr session a spec pane with no record is created in. Required when there is one.
63:     #[arg(long, value_name = "NAME")]
64:     pub herdr_session: Option<String>,
65: }
```

### E47  `crates/holler-cli/src/profile/apply.rs` lines 110-134: a failed step is the only thing printed

Command: `sed -n '110,134p' crates/holler-cli/src/profile/apply.rs | awk -v s=110 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
110: /// The failure a run with a `failed` step prints, and nothing else (the decision on O6):
111: /// `apply stopped at <pane> (<action>): <step error>; <N> later step(s) skipped`, coded as the
112: /// step's error, whose text is the step's detail (with the reconcile step when it had acted).
113: /// `None` when no step failed.
114: fn stopped(applied: &Applied) -> Option<ErrorBody> {
115:     let (failed, error) = applied
116:         .steps
117:         .iter()
118:         .find(|step| step.outcome == Outcome::Failed)
119:         .and_then(|step| Some((step, step.error.as_ref()?)))?;
120:     let skipped = applied
121:         .steps
122:         .iter()
123:         .filter(|step| step.outcome == Outcome::Skipped)
124:         .count();
125:     let message = failed.detail.clone().unwrap_or_else(|| error.to_string());
126:     Some(ErrorBody {
127:         code: ErrorCode::from(error),
128:         message: format!(
129:             "apply stopped at {} ({}): {message}; {skipped} later step(s) skipped",
130:             quoted(&failed.pane),
131:             failed.action.as_str()
132:         ),
133:     })
134: }
```

### E48  `crates/holler-cli/src/profile/apply.rs` lines 136-173: the text form: header and one PANE: ACTION, OUTCOME line per step

Command: `sed -n '136,173p' crates/holler-cli/src/profile/apply.rs | awk -v s=136 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
136: /// The text form: the header, then one line per step, with a reported step's differences.
137: fn render(applied: &Applied) -> String {
138:     let done = if applied.dry_run {
139:         "planned (dry run; nothing changed)"
140:     } else {
141:         "applied"
142:     };
143:     let mut lines = vec![format!(
144:         "profile {}: {}, {done}",
145:         applied.profile,
146:         count(applied.steps.len(), "step")
147:     )];
148:     for step in &applied.steps {
149:         lines.push(step_line(step));
150:         if let Some(diff) = &step.diff {
151:             lines.extend(diff.differences.iter().map(difference_line));
152:         }
153:     }
154:     lines.join("\n")
155: }
156: 
157: /// `PANE: ACTION, OUTCOME`, then ` (CODE)` and `: DETAIL` when the step has them. The pane and
158: /// the detail are escaped as `profile show` escapes a stored string.
159: fn step_line(step: &Step) -> String {
160:     let mut line = format!(
161:         "{}: {}, {}",
162:         text(&step.pane),
163:         step.action.as_str(),
164:         step.outcome.as_str()
165:     );
166:     if let Some(code) = &step.code {
167:         line.push_str(&format!(" ({code})"));
168:     }
169:     if let Some(detail) = &step.detail {
170:         line.push_str(&format!(": {}", text(detail)));
171:     }
172:     line
173: }
```

### E49  `crates/holler-cli/src/profile/create.rs` lines 1-24: create --from-current and its refusal

Command: `sed -n '1,24p' crates/holler-cli/src/profile/create.rs | awk -v s=1 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1: //! `holler profile create NAME [--from-current | --from PROFILE]` (story #662, epic #633): make a
2: //! profile that is empty, a detached copy of another profile's specs (`--from`), or a snapshot
3: //! of every pane in the pane registry that makes each pane a member (`--from-current`).
4: //!
5: //! - **Checks first.** Every check runs before the first write (#662 B3): the names parse
6: //!   (`usage`), NAME is not taken by name or slug (`profile-exists`), PROFILE exists
7: //!   (`profile-not-found`), and with `--from-current` no pane belongs to another profile
8: //!   (`pane-in-other-profile`, every such pane at once).
9: //! - **One create write.** Every form stores its record through `insert_profile`, the one
10: //!   place that reports the create race as `profile-exists` (B7). Every new record is built by
11: //!   `holler_pane::profile_snapshot::profile_from_panes`, so no `Profile` is built by hand, and
12: //!   `--from-current`'s specs come from each pane's record (its position from `herdr.grid`,
13: //!   never from its name), with no Herdr call.
14: //! - **Write order.** No transaction spans the two registries (ADR-0021 section 8), so
15: //!   `--from-current` writes the profile first (a pane can name only a profile that exists),
16: //!   then each pane joins in `list()` order, its `Pane.profile` set by compare-and-swap. A
17: //!   failed join undoes everything, since a failure records nothing (I3, #662 B2): the pane
18: //!   whose join failed leaves if its join landed after all (a `timeout` may have), then each
19: //!   joined pane leaves, newest first, then the profile is deleted. The undo stops at its first
20: //!   failed step and reports `profile-conflict` with the commands that reconcile, so the
21: //!   profile still exists whenever a member may remain.
22: //!
23: //! The verb reads and writes the two registries and nothing else: no adapter, no probe, and
24: //! nothing from the process environment.
```

### E50  `crates/holler-cli/src/profile/show.rs` lines 28-60: show: the comparison apply must agree with

Command: `sed -n '28,60p' crates/holler-cli/src/profile/show.rs | awk -v s=28 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
28: /// Show a profile and where live panes differ from it.
29: ///
30: /// The text form starts with `profile NAME (SLUG): generation G, N specs, L live`, where
31: /// the live panes are the profile's members (the panes whose profile is this one). Then
32: /// one block per spec, `spec PANE` and one `FIELD: VALUE` line per field. Then one line
33: /// per pane: `matches` or `differs` (a spec and a live pane of its name), `missing (no
34: /// live pane)` (a spec alone) or `extra (no spec)` (a live pane alone). Under a pane that
35: /// differs, one `FIELD: spec VALUE, live VALUE` line per field that differs; under every
36: /// live pane, its last probe result (`ok`, `failed (missing "...")`, `error ("...")` or
37: /// `none`), which this verb reads and never runs. A position is `r<row>c<col>`, a command
38: /// or a check a JSON array, and a control character in a stored string is escaped.
39: ///
40: /// `--format=json` prints one envelope whose data is `{"profile": PROFILE, "comparison":
41: /// [ROW, ...]}`: the profile as stored, and per pane `{"pane", "status", "differences",
42: /// "probe"}`, where each difference is `{"field", "spec", "live"}` and `probe` is the
43: /// last probe result or null.
44: #[derive(Args, Debug)]
45: pub struct ProfileShow {
46:     /// The profile's name (spaces allowed).
47:     pub name: String,
48: }
49: 
50: /// Run `holler profile show`: read the profile and its members, compare them, and print
51: /// the comparison.
52: pub fn run(args: &ProfileShow, ctx: &mut VerbCtx<'_>) -> i32 {
53:     let result = view(args, ctx.ports).map_err(|e| ErrorBody::from(&e));
54:     emit(&mut ctx.sink, ctx.format, result, render)
55: }
56: 
57: /// The data of `profile show`.
58: #[derive(Debug, Serialize)]
59: struct ProfileView {
60:     /// The profile as stored.
```

### E51  `crates/holler-cli/src/pane/args.rs` lines 20-33: --profile and --spec-only

Command: `sed -n '20,33p' crates/holler-cli/src/pane/args.rs | awk -v s=20 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
20: #[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
21: pub struct ProfileOpt {
22:     /// Act on the panes of this profile; a named pane must belong to it.
23:     #[arg(long, value_name = "NAME")]
24:     pub profile: Option<String>,
25: }
26: 
27: /// `--spec-only`: edit a profile's spec and change nothing live.
28: #[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
29: pub struct SpecOnly {
30:     /// Edit the profile's spec for the pane and change nothing live.
31:     #[arg(long, requires = "profile")]
32:     pub spec_only: bool,
33: }
```

### E52  `crates/holler-cli/src/pane/args.rs` lines 83-91: --check-arg, --check-json, --expect

Command: `sed -n '83,91p' crates/holler-cli/src/pane/args.rs | awk -v s=83 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
83:     #[arg(long, value_name = "ARG", conflicts_with = "check_json")]
84:     pub check_arg: Vec<String>,
85:     /// The health check as a JSON array of strings; not with --check-arg.
86:     #[arg(long, value_name = "JSON")]
87:     pub check_json: Option<String>,
88:     /// A string the health check's output must contain. Repeatable.
89:     #[arg(long, value_name = "STR")]
90:     pub expect: Vec<String>,
91: }
```

### E53  `crates/holler-pane-testkit/src/envelope.rs` lines 198-233: check_envelope

Command: `sed -n '198,233p' crates/holler-pane-testkit/src/envelope.rs | awk -v s=198 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
198: /// code of the verb that wrote it.
199: ///
200: /// Returns the envelope, or the fault of the first rule it breaks, in the order the
201: /// module doc lists.
202: pub fn check_envelope(stdout: &str, exit_code: i32) -> Result<Envelope, EnvelopeFault> {
203:     check_exit_code(exit_code)?;
204:     let Value::Object(object) = frame(stdout)? else {
205:         return Err(EnvelopeFault::NotAnObject);
206:     };
207:     let [schema_version, ok, data, error] = exact_members(object, ENVELOPE_KEYS, "")?;
208:     if schema_version.as_u64() != Some(SCHEMA_VERSION) {
209:         return Err(EnvelopeFault::SchemaVersion);
210:     }
211:     let succeeded = exit_code == 0;
212:     if ok.as_bool() != Some(succeeded) {
213:         return Err(EnvelopeFault::OkDisagreesWithExit);
214:     }
215:     if succeeded {
216:         return if error.is_null() {
217:             Ok(Envelope {
218:                 schema_version: SCHEMA_VERSION,
219:                 ok: true,
220:                 data,
221:                 error: None,
222:             })
223:         } else {
224:             Err(EnvelopeFault::ErrorWhileOk)
225:         };
226:     }
227:     let error = check_failure(error, &data, exit_code)?;
228:     Ok(Envelope {
229:         schema_version: SCHEMA_VERSION,
230:         ok: false,
231:         data: Value::Null,
232:         error: Some(error),
233:     })
```

### E54  `docs/pane-control.md` lines 359-377: the guide: holler profile apply

Command: `sed -n '359,377p' docs/pane-control.md | awk -v s=359 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
359: ### `holler profile apply NAME`
360: 
361: Converge the live panes on a profile: create, adopt or relaunch, and never kill a healthy pane. Its flags are
362: `--dry-run` (print the plan, change nothing), `--take-over` (move a pane that belongs to another profile) and
363: `--herdr-session NAME` (needed when the plan creates a pane). The full reference is in section 6, which ADR-0021
364: section 8 ("Apply as built") and its section 9 apply row state as the contract.
365: 
366: ```bash
367: holler profile apply fleet --dry-run
368: holler profile apply fleet --herdr-session demo
369: ```
370: 
371: Each step is `{pane, action, outcome, code, diff, detail}`. The actions are `create`, `adopt`, `report`, `relaunch`,
372: `refuse` and `observe`; the outcomes are `created`, `relaunched`, `adopted`, `reported`, `refused`, `failed`, `skipped`
373: (after a failed step) and `planned` (a dry run's step that would act). A `refused` or `reported` step does not stop the
374: run or change the exit code; a `failed` step stops the run and exits 1. Its own open codes are
375: `orchestrator-not-relaunched` and `spec-not-launchable`. It exits 2 (`usage`) when the plan has a create and no
376: `--herdr-session`, 3 for `profile-not-found`, 1 for a failed step, and 0 otherwise.
377: 
```

### E55  `docs/pane-control.md` lines 499-558: the guide: section 6 and the apply reference

Command: `sed -n '499,558p' docs/pane-control.md | awk -v s=499 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
499: ## 6. Profiles
500: 
501: A profile is a named set of pane specs, kept in the registry beside the panes. A pane belongs to at most one profile
502: (`pane-in-other-profile`). A profile holds names and non-secret values, never a secret: an environment variable is a
503: NAME only, and a spec that carries a value is refused with `profile-secret-refused` (I7). A profile's **live panes** are
504: the records whose `profile` names it.
505: 
506: - `create` makes an empty profile, a detached copy (`--from PROFILE`), or a snapshot of every pane (`--from-current`,
507:   each pane becomes a member).
508: - `delete` is refused with `profile-has-live-panes` while panes belong to it; with `--keep-panes` each member is
509:   detached first and keeps running.
510: - `list` and `show` read only. `show` prints the specs, the live panes and, per pane, every field that differs from its
511:   spec.
512: - `apply` converges the live panes on the profile, as the reference below says.
513: - A spec-editing verb with `--profile P` (`pane launch`, `pane relaunch`, `pane close`) edits P's spec for the pane and the
514:   live pane in one transaction: a failed run puts P's specs back (I8).
515: 
516: ### The `profile apply` reference
517: 
518: The contract is ADR-0021 section 8, "Apply as built (#664)", with its definition of "adopt", and section 9's apply row.
519: **To adopt** a pane is to record that a pane which already has a record, is healthy and matches its spec belongs to P,
520: and to touch nothing live. Apply adopts records, never processes: a Herdr pane, a tmux session or a server that no
521: record names is never adopted (`pane import` is what records existing panes).
522: 
523: Apply reads P and the registry, then observes every recorded spec pane. **Healthy** is a live observation, never the
524: stored health: the harness answers its health call, Herdr's snapshot lists the record's pane, and the TUI does not show
525: a session other than the session of record (a shown session that cannot be read does not count against it).
526: 
527: One step per spec pane, in workspace, row and column order; the first matching row applies:
528: 
529: | Row | Situation | Step |
530: |---|---|---|
531: | a | The pane's record belongs to another profile, and no `--take-over`. | `refuse`, outcome `refused` (`pane-in-other-profile`). |
532: | b | No record. | `create` through the launch transaction, outcome `created`. |
533: | c | Healthy and matches its spec. | `adopt`, outcome `adopted`: the record's profile becomes P (with `--take-over` from another profile, leave then join, two writes). |
534: | d | Healthy and differs from its spec. | `report`, outcome `reported`, with the pane's differences; nothing live is touched. |
535: | e | Not healthy, role orchestrator. | `refuse`, outcome `refused` (`orchestrator-not-relaunched`); nothing is written, even under `--take-over`. |
536: | f | Not healthy, role agent. | `relaunch`, outcome `relaunched`; under `--take-over` from another profile the leave and join happen first, then the relaunch. |
537: 
538: - **A step** is `{pane, action, outcome, code, diff, detail}`; an absent `code`, `diff` or `detail` prints `null`. The
539:   actions are `create`, `adopt`, `report`, `relaunch`, `refuse` (rows a and e, and a spec whose pane is not a pane name)
540:   and `observe` (a recorded pane whose health or snapshot call failed). The outcomes are `created`, `relaunched`,
541:   `adopted`, `reported`, `refused`, `failed`, `skipped` (a step after a failed one) and `planned` (a dry run's step that
542:   would act; row d's stays `reported`). `detail` is the step's own line, inside `data`.
543: - **Refusals do not stop the run.** A `refused` or `reported` step leaves the exit code alone. A refusal from the launch
544:   transaction is a `refused` step with that code; a `usage` answer of the launch for a step's spec is refused with the
545:   open code `spec-not-launchable`.
546: - **A failure stops it.** A `failed` step (`unavailable`, `timeout`, a failed health call, a store conflict) stops the
547:   run, and the later steps are `skipped`. A run that ends with a `failed` step prints **only the failure**: in JSON the
548:   failure envelope, whose `error.code` is the failed step's code and whose message is
549:   `apply stopped at <pane> (<action>): <step error message>; <N> later step(s) skipped`; in text one `error:` line on
550:   stderr and nothing on stdout. So `skipped` appears only in that count. A dry run with a failed step prints no plan.
551: - A step refused or failed after its launch or relaunch had begun carries the command that reconciles.
552: - **`--dry-run`** prints the plan as the same table (for a run that does not fail), runs no probe, issues no split and
553:   writes nothing. `data` is `{"profile": NAME, "dry_run": bool, "steps": [...]}`, every key always present.
554: - **Exit codes:** 0 for a run with no failed step, 1 for a failed step, 2 for `usage` (a plan with a create and no
555:   `--herdr-session`, checked before any step, in a dry run too), 3 for `profile-not-found`.
556: - `orchestrator-not-relaunched` and `spec-not-launchable` are open codes of apply: refusals, exit 3, not in the closed
557:   list of section 9.
558: 
```

### E56  `docs/pane-control.md` lines 588-597: the guide: the health probe

Command: `sed -n '588,597p' docs/pane-control.md | awk -v s=588 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
588: ## 8. Probe, argv, layout, Herdr version
589: 
590: **The health probe.** A pane spec can carry a health check, an argv run before a launch or relaunch acts, and strings its
591: output must contain. A check that fails refuses the run with `probe-failed` (exit 3) and changes nothing. For example,
592: `check ["curl","-s","http://127.0.0.1:8095/v1/models"]` with `expect ["qwen38"]`:
593: 
594: ```bash
595: holler pane relaunch demo-alpha --check-arg curl --check-arg=-s --check-arg http://127.0.0.1:8095/v1/models --expect qwen38
596: ```
597: 
```

### E57  `docs/pane-control.md` lines 783-792: the guide: section 12, Later

Command: `sed -n '783,792p' docs/pane-control.md | awk -v s=783 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
783: ## 12. Later
784: 
785: Deferred, so not documented here as existing; none of them is available:
786: 
787: - the roster extension: the pane columns on `holler roster` (#648);
788: - applying a pane's model, effort and environment to its harness server (#829);
789: - DRIVEN following each pane's session of record, and `reset --first` through the hub's gate (#839);
790: - an import that leaves the panes Holler does not drive alone (#835);
791: - the shared subprocess runner of the adapters and the probe (#696);
792: - the real-adapter scenario for `profile apply` (#667);
```

### E58  `docs/adr/ADR-0021.md` lines 654-689: ADR-0021: Apply as built (#664)

Command: `sed -n '654,689p' docs/adr/ADR-0021.md | awk -v s=654 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
654: **Apply as built (#664).** `holler profile apply NAME [--dry-run] [--take-over] [--herdr-session NAME]` runs one engine
655: over the ports, `holler_pane::tx_apply`, which calls `tx_launch` unchanged. **To adopt** a pane is to record that a pane
656: which already has a record, is healthy and matches its spec belongs to P, and to touch nothing live: its `Pane.profile`
657: becomes P by one compare-and-swap (the join), or under `--take-over` by two (the leave from its profile, then the join),
658: and nothing is written when it is in P already. That is the only adoption in this ADR, and it adopts a record, never a
659: process: a Herdr pane, a tmux session or a harness server that no record names is still never adopted (launch refuses
660: `grid-occupied` and `port-in-use`, and `pane import` (#650) is what records existing panes). Apply reads P and the
661: registry; without `--herdr-session`, a spec pane with no record (a create) is `usage` before any Herdr or harness call,
662: in a dry run too. It then observes every recorded spec pane at once, one thread per call: a pane is healthy when `health`
663: answers true, Herdr's snapshot lists the record's Herdr session and pane id (never the cell or the label), and its TUI
664: does not show a session other than its session of record (a shown session that cannot be read does not count against
665: it). Each spec pane, in workspace, row and column order, takes the first row that matches: a record of another profile
666: without `--take-over` is refused (`pane-in-other-profile`); a spec pane with no record is created through launch; a
667: healthy one is adopted when it matches its spec and reported with its `profile_diff` row when it differs, never
668: rewritten, moved or killed; one that is not healthy is relaunched in place when it is an agent, and refused
669: (`orchestrator-not-relaunched`) when it is the orchestrator. A refusal from the launch transaction is a refused step
670: with its code (`usage` for the spec is `spec-not-launchable`); a failure-class answer, a failed `health` or snapshot call
671: included, is a failed step that stops the run, and the later steps are skipped. A take-over moves the pane to P before
672: the step's live change: if the relaunch then fails the pane stays in P, and if the join fails after the leave landed the
673: pane is left in no profile, as the step's error says, and no re-join is written. Each step re-reads its record and P
674: before it acts, so the spec it hands to launch or relaunch is P's at that moment. `--dry-run` chooses every step from
675: the same reads and observations and calls nothing else (no probe, no split, no write); its steps that would act are
676: `planned`. The exit codes are the doctor form (section 9): 0 for a run with no failed step, which prints its steps
677: (`{"profile", "dry_run", "steps"}` in JSON, the plan for a dry run); 1 for a failed step, which prints only
678: `apply stopped at <pane> (<action>): <step error>; <N> later step(s) skipped`, for a dry run too, since an envelope
679: cannot carry data with a failure. A step is the object `{pane, action, outcome, code, diff, detail}` in
680: `data.steps[]`, whose absent `code`, `diff` or `detail` prints `null`: its `action` is one of `create`, `adopt`,
681: `report`, `relaunch`, `refuse` (rows a and e, and a spec whose pane is not a pane name) and `observe` (a recorded pane
682: whose `health` or snapshot call failed); its `outcome` is one of `created`, `relaunched`, `adopted`, `reported`,
683: `refused` and `failed`, or `skipped` after a failed step, or `planned` for a dry run's step that would act; its
684: `detail` is the step's own line, inside `data`, so it does not conflict with section 9's "the envelope has no
685: `detail`"; and a step whose launch or relaunch had started its act before it was refused or failed (since #802 the act
686: can refuse: the port's re-check, `verify_serving`) carries step 6's reconcile step, a `refused` step at the end of its
687: `detail` and a `failed` step in that failure's message, as `pane launch` does. There is no budget over the whole run
688: (decided 2026-10-11): each create or relaunch runs under launch's own budget, so a run is bounded per step, as the
689: module doc of `tx_apply.rs` records.
```

### E59  `docs/adr/ADR-0021.md` lines 763-763: ADR-0021 section 9: the apply row

Command: `sed -n '763,763p' docs/adr/ADR-0021.md | awk -v s=763 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
763: | `profile apply` | In the doctor form (#664): the verb fails only before it has a plan, with `profile-not-found`, or `usage` when a spec pane with no record needs `--herdr-session`; every other code is a step's. A `refused` step leaves the exit code 0, and a `failed` step stops the run and exits 1 with that step's code. Per step: `pane-in-other-profile` (without `--take-over`), `probe-failed` (the others proceed), `herdr-version-unsupported`, `grid-out-of-range`, `pane-not-found` and `pane-not-in-profile` (the record or the spec gone during the run), `generation-conflict`, `profile-conflict`; open (#664): `orchestrator-not-relaunched` (an orchestrator that is not healthy) and `spec-not-launchable` (launch or relaunch answered `usage` for the spec); open (#644), from a create or a relaunch: `pane-exists`, `grid-occupied`, `port-in-use`; open (#802), through the launch engine, `agent-unsupported`, and (#642) the OpenCode adapter's `agent-unusable`; open (#640) for a cell that no single split reaches, `grid-unreachable` |
```

### E60  `docs/adr/ADR-0021.md` lines 1030-1031: ADR-0021: the opt-in scenarios refuse real session names

Command: `sed -n '1030,1031p' docs/adr/ADR-0021.md | awk -v s=1030 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
1030:   fakes, never a real Herdr, tmux or OpenCode; the opt-in scenarios (#649, and #667 for `profile apply`) run only on
1031:   scratch Herdr, tmux and OpenCode instances they start themselves, and refuse real session names.
```

### E61  `crates/holler-cli/tests/profile_verbs/apply/dry_run.rs` lines 118-184: default-run: a dry run and show agree

Command: `sed -n '118,184p' crates/holler-cli/tests/profile_verbs/apply/dry_run.rs | awk -v s=118 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
118: #[test]
119: fn a_dry_run_and_profile_show_agree() {
120:     let seed = || {
121:         let rig = Rig::new();
122:         let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100));
123:         let d = rig.seed_live(&Seed::at("pane-d", cell(1, 2), 48101));
124:         let g = rig.seed_live(&Seed::at("pane-g", cell(3, 1), 48103));
125:         let mut moved = spec_of(&d);
126:         moved.herdr.grid = cell(3, 3);
127:         let mut other_agent = spec_of(&g);
128:         other_agent.opencode_agent = Some(AgentKey::parse("review").unwrap());
129:         assert_ne!(other_agent.opencode_agent, g.opencode_agent);
130:         rig.set_specs(
131:             P,
132:             vec![
133:                 spec_of(&c),
134:                 moved,
135:                 new_spec("pane-b", cell(2, 1), 48102),
136:                 other_agent,
137:             ],
138:         );
139:         rig
140:     };
141:     let shown = data_of(&seed().run(&["profile", "show", P], Format::Json));
142:     let planned = data_of(&seed().run(&apply_argv(&["--dry-run", HS[0], HS[1]]), Format::Json));
143:     let rows = shown["comparison"].as_array().expect("show's comparison");
144:     let mut checked = 0;
145:     for row in rows.iter().filter(|r| r["status"] != "extra") {
146:         let pane = row["pane"].as_str().unwrap();
147:         let planned_step = json_step(&planned, pane);
148:         match row["status"].as_str().unwrap() {
149:             "differs" => {
150:                 let want = json!({
151:                     "pane": row["pane"],
152:                     "status": row["status"],
153:                     "differences": row["differences"],
154:                 });
155:                 assert_eq!(planned_step["diff"], want, "{pane}");
156:             }
157:             "matches" => {
158:                 assert_eq!(planned_step["diff"], Value::Null, "{pane}");
159:                 assert_eq!(planned_step["action"], json!(wire(Action::Adopt)), "{pane}");
160:             }
161:             "missing" => {
162:                 assert_eq!(planned_step["diff"], Value::Null, "{pane}");
163:                 assert_eq!(
164:                     planned_step["action"],
165:                     json!(wire(Action::Create)),
166:                     "{pane}"
167:                 );
168:             }
169:             other => panic!("{pane}: {other}"),
170:         }
171:         checked += 1;
172:     }
173:     assert_eq!(checked, 4, "every spec pane: {shown}");
174:     assert_eq!(
175:         planned["steps"].as_array().unwrap().len(),
176:         4,
177:         "nothing beyond show's rows"
178:     );
179:     let d = json_step(&planned, "pane-d");
180:     assert_eq!(
181:         d["diff"]["differences"][0]["spec"],
182:         json!({"row": 3, "col": 3, "pos": "r3c3"})
183:     );
184: }
```

### E62  `crates/holler-cli/tests/profile_verbs/apply/rows.rs` lines 254-291: default-run: a failing probe refuses that pane, the others proceed

Command: `sed -n '254,291p' crates/holler-cli/tests/profile_verbs/apply/rows.rs | awk -v s=254 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
254: #[test]
255: fn a_failing_probe_refuses_that_pane_and_the_others_proceed() {
256:     let seed = || {
257:         let rig = Rig::new();
258:         let mut probed = new_spec("pane-b", cell(1, 1), 48100);
259:         probed.check = Some(argv(&["curl", "http://127.0.0.1:8095/v1/models"]));
260:         probed.expect = vec!["qwen38".to_owned()];
261:         rig.prober.script(
262:             probed.check.clone().unwrap(),
263:             ProbeResult::Failed {
264:                 missing: vec!["qwen38".to_owned()],
265:             },
266:         );
267:         rig.set_specs(P, vec![probed, new_spec("pane-z", cell(2, 1), 48101)]);
268:         rig
269:     };
270:     let rig = seed();
271:     let (answer, _) = rig.engine(&request(false, false, true), &options());
272:     let applied = answer.unwrap();
273:     assert_eq!(
274:         outcomes(&applied),
275:         vec![
276:             ("pane-b", StepOutcome::Refused),
277:             ("pane-z", StepOutcome::Created)
278:         ]
279:     );
280:     assert_eq!(
281:         step(&applied, "pane-b").code.as_deref(),
282:         Some("probe-failed")
283:     );
284:     assert_eq!(rig.prober.calls().len(), 1, "pane-b's probe ran once");
285:     assert_eq!(rig.record("pane-b"), None);
286:     assert!(
287:         rig.herdr_panes().iter().all(|p| p.grid != cell(1, 1)),
288:         "nothing made"
289:     );
290:     assert_eq!(seed().run(&apply_argv(&HS), Format::Json).code, 0);
291: }
```

### E63  `crates/holler-cli/tests/profile_verbs/apply/rows.rs` lines 446-481: default-run: apply twice changes nothing

Command: `sed -n '446,481p' crates/holler-cli/tests/profile_verbs/apply/rows.rs | awk -v s=446 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
446: #[test]
447: fn apply_twice_changes_nothing_the_second_time() {
448:     let rig = Rig::new();
449:     let c = rig.seed_live(&Seed::at("pane-c", cell(1, 1), 48100));
450:     let n = rig.seed_live(&Seed::at("pane-n", cell(1, 2), 48101).in_profile(None));
451:     let f = rig.seed_live(&Seed::at("pane-f", cell(1, 3), 48102));
452:     rig.kill(&f);
453:     let b = new_spec("pane-b", cell(2, 1), 48103);
454:     rig.set_specs(P, vec![spec_of(&c), spec_of(&n), spec_of(&f), b]);
455:     let (first, _) = rig.engine(&request(false, false, true), &options());
456:     assert_eq!(
457:         outcomes(&first.unwrap()),
458:         vec![
459:             ("pane-c", StepOutcome::Adopted),
460:             ("pane-n", StepOutcome::Adopted),
461:             ("pane-f", StepOutcome::Relaunched),
462:             ("pane-b", StepOutcome::Created),
463:         ]
464:     );
465:     let before = rig.cursor();
466:     let (second, calls) = rig.engine(&request(false, false, true), &options());
467:     let second = second.unwrap();
468:     assert!(
469:         second
470:             .steps
471:             .iter()
472:             .all(|s| s.outcome == StepOutcome::Adopted),
473:         "{second:?}"
474:     );
475:     assert_only_observed(&calls);
476:     assert_eq!(
477:         rig.writes_since(before),
478:         vec![],
479:         "the second run writes nothing"
480:     );
481: }
```

### E64  `docs/handoffs/649-brief.md` lines 38-42: #649 D10 to D12

Command: `sed -n '38,42p' docs/handoffs/649-brief.md | awk -v s=38 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
38: D10. **Scratch-only, four guards.** (1) *Opt-in:* every scratch test is `#[ignore]` and runs only with `HOLLER_TEST_PANE_INTEGRATION=1` exactly (E43, E49); opted in, a missing `herdr`, `tmux` or `opencode` is a failure, not a skip (E48). (2) *Unique prefix:* the Herdr session is `holler649-<8 lower-case hex>`, every pane name is `hlr649-...`, and `default` is refused by name (E44). (3) *Temp state dir:* the hub, its registries, the tmux socket and Herdr's whole `HOME`/`XDG_*` live in one fresh `/tmp` directory, removed by a guard that kills only the processes it started (E42, E45, E49). (4) *Refusal of real names:* the test checks every name and path before it runs a verb and fails closed on a Herdr session that is not the scratch one, a tmux socket outside the scratch directory, a state dir outside it, or a pane name without the prefix; the binary is run with `HERDR_*`, `TMUX` and `TMUX_PANE` removed and `HOLLER_STATE_DIR` set (E53). Each guard is a pure function the default run tests. (audit) (5) *OpenCode's and tmux's environment:* the test starts the private tmux server itself, before any verb runs, with exactly the isolated pairs of the adapter rig (`HOME` and the `XDG_*` directories inside the scratch directory, `PATH`, `TERM`, the five `OPENCODE_DISABLE_*` flags) (tmux reads no user config: it is started by the test under the scratch `HOME`), and has written the dead-end `opencode.json` under the scratch `XDG_CONFIG_HOME/opencode/` first; under D7's `ProcessEnv::Inherit` every OpenCode the verbs start then inherits that server's environment and cannot read the operator's config, auth or session store. The binary under test gets exactly `HOLLER_STATE_DIR`, `HOLLER_HERDR_SESSION`, `HOLLER_HERDR_SOCKET`, `HOLLER_HERDR_WORKSPACES`, `HOLLER_TMUX_SOCKET`, `HOLLER_OPENCODE_BIN`, `HOLLER_OPENCODE_SERVE_ARGS=["--pure"]` and the logging variables, plus (A review, W-D) a `PATH` the test chooses (the directories of `tmux` and `opencode`, then `/usr/bin:/bin`: the binary's `TmuxHost` and the OpenCode `TmuxConfig` find `tmux` and `kill` on `PATH`, and with none they fall back to libc's default search path) and the scratch `HOME` and `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME` (a child that wants a home would otherwise fall back to the account's real one) and (diff gate, round 3, W-1) `LANG=C.UTF-8` (a UTF-8 locale for the tmux client; the guard pins it), nothing else inherited; the guard `the_binary_env_is_built_from_nothing_inherited` pins the exact set. A guard check (a pure function) fails closed when any of `HOME`, `XDG_*`, a socket or the state dir lies outside the scratch directory. (6) *Killing what it started (W16):* the servers and TUIs are started by the private tmux server and Herdr, not by the test, so the guard finds them by `tmux list-panes -a -F '#{pane_pid}'` on the scratch socket and by `HostPort::ps` per scratch session, sends `TERM` then `KILL` to their process groups, runs `tmux kill-server` on the scratch socket, runs `herdr server stop` for the scratch session, and only then removes the directory; "no server process of the test is left" (D15) means none of those pids answers `kill -0` afterwards.
39: 
40: D11. **Reuse, do not copy.** (audit, rewritten) The scratch Herdr guard (`holler-adapter-herdr/tests/scratch_herdr/mod.rs`, E42 to E44), the OpenCode rig's dead-end config, isolated environment, binary lookup, port picker and model guard (`holler-adapter-opencode/tests/real_opencode/rig.rs`, E45, E47) and the private tmux pattern (`holler-adapter-host/tests/real_tmux_test.rs`, E49) are included from their crates with `#[path = "..."] mod ...;`, not copied. They cannot be included as they are, so each gets one small generalisation in place, and the three adapter test directories are in the Blast radius for exactly these edits and no others, with every default-run test of those directories passing unchanged: (a) *scratch_herdr:* a `Flavor { gate_var, name_prefix, root_prefix }` with #640's own flavor (`HOLLER_HERDR_SCRATCH`, `holler640-`, `h640.`) as the default of the unsuffixed functions, and `_as`/`_in` variants (`check_name_as`, `new_name_as`, `prove_as`, `opt_in_as`, `ScratchHerdr::start_in(herdr, flavor, base)`) taking the flavor and the scratch base; this story's flavor is `HOLLER_TEST_PANE_INTEGRATION`, `holler649-`, `h649.`. (b) *real_opencode/rig.rs:* `CONFIG`, `on_path`, `binaries`, `no_model_can_be_reached`, and the new `lay_out(root)` and `isolated_vars(opencode, root)` (the pieces the rig's own constructor now calls) become `pub`. (c) *real_tmux:* the private tmux server moves out of the test target `real_tmux_test.rs` into the includable `tests/real_tmux/mod.rs`, which the test target and this story include. An included module whose items this target does not all use carries `#![allow(dead_code)] // #649` (the workspace denies `dead_code`; `pane_integration/main.rs` has it). A second copy of a guard is a defect.
41: 
42: D12. **No model call.** The only provider is the dead-end one of E47 (a closed local port): no model can be reached and no prompt reaches one. After every server is healthy the test checks it reports exactly that provider, or fails without going on. `reset --first TEXT` sends its message through `HarnessPort::send_prompt` (ADR-0021, #642 part 3); with the dead-end provider the server accepts it and no model is called. No step types into a TUI (I4). (audit) The first message goes to pane C2 (the one reset), and the test asserts `session_activity` after it; the activity reads `Busy` and that is expected, not a D9 finding: OpenCode 1.19.0 retries a refused dead-end provider (status `retry`) until the turn is aborted (`crates/holler-adapter-opencode/tests/real_opencode/rig.rs` lines 9-12; #642's `handoff-S.md` row 11), and the adapter reads `retry` as `Busy` (`lib.rs` `running`; `prompt.rs`). The later steps on C2 (park, kill, unpark, relaunch) do not need an idle session; killing the server ends the retry.
```

### E65  `docs/handoffs/649-brief.md` lines 44-44: #649 D13

Command: `sed -n '44,44p' docs/handoffs/649-brief.md | awk -v s=44 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
44: D13. **Two kinds of test, one target.** Tests that need only a hub subprocess and a state dir (the `Hub` and `StateDir` harness, E53, E54) run in the default run (CI): the store clients' round trips, the no-hub failure (`unavailable`, exit 1, D2), a lost write reply (D3), a hub restart with the registry intact, and the watch test of D14. Tests that need Herdr, tmux and OpenCode are the scenario and are `#[ignore]` (D10). (audit) The lost reply of D3 is tested with an in-test fake hub: a `UnixListener` at `<state>/hub/control.sock` (`pane_integration/fake_hub.rs`) that answers `control/status` and, for a write, reads the request and closes the connection without replying; `send_over` then answers `BadReply` at once, so the test does not wait out the 10 s bound (D9 admits no knob). Both live in `tests/pane_integration/main.rs`'s modules, the one `[[test]]` that exists (E50); no manifest edit beyond the dependencies (D16).
```

### E66  `docs/handoffs/649-brief.md` lines 54-54: #649 D18

Command: `sed -n '54,54p' docs/handoffs/649-brief.md | awk -v s=54 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
54: D18. **Not done here, and not decided here.** (a) *DRIVEN:* how the hub's DRIVEN session follows `session_of_record` stays deferred to #654 (E41); this story wires nothing for it, `last_observed.driven` stays as stored, and the scenario asserts SHOWN against `session_of_record` only. (b) The whole-fleet defaults of ADR-0021 line 522, including the `--herdr-session` default: the flag stays required on every verb that reaches Herdr and #654 names the session in the cutover; no code story is planned for the default (A third pass, W-K). (c) Moving the first message to the hub's prompt path: owned by #828 (`pane reset --first` bypasses the hub's park and hold gate), not by #646 and not by this story. (d) Importing the live fleet (#650) and the cutover (#654). (f) (A third pass, W-K) The pane's model, effort and environment reaching the harness server (recorded today, not applied): #829; how a provider credential reaches the tmux session in production is #654's. The DRIVEN wiring of (a) is recorded on issue #654 by a comment. (e) (A review, W-J) Every `--profile` edit made through a pane verb is logged under the single actor `holler pane` (D1), so the profile log cannot tell `launch --profile` from `close --profile`, where each profile verb names its own actor; a per-verb actor needs a seam other than the argument-less `connect()`. Accepted for this story; follow-up #825.
```

### E67  `docs/handoffs/649/handoff-A-dup.md` lines 40-42: A-dup's duplication warns W-7, W-1 to W-4 and W-6, W-O

Command: `sed -n '40,42p' docs/handoffs/649/handoff-A-dup.md | awk -v s=40 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
40: | W-7 | warn (nit, T; not binding) | `crates/holler-cli/tests/pane_integration/observe.rs:27-55`, `:62-81`, `:198-205` | **`observe.rs` copies three pieces of its sibling `bind.rs`.**<ul><li>**The conformance run.** `the_lazy_herdr_passes_the_herdr_conformance_suite` (`:27-55`) repeats `bind.rs:293-316`: the same fixture (`FakeHerdr::new("scratch").with_workspace("scratch", 2, 1)`), the same label hook and the same assertion. Only the wrapper's constructor differs. The house pattern folds repeated runs of a suite through `scratch_fake()` and `fixture(port)` (`holler-pane-testkit/tests/herdr_conformance_test.rs:50-62`).</li><li>**The call log.** `Observing`'s `calls: Mutex<Vec<&'static str>>` and `note` (`:62-81`) repeat `Recording`'s (`bind.rs:164-178`). The recording port itself is justified (Checked and clean); only its log is a second copy.</li><li>**The unbuildable `LazyHerdr`.** The counting closure that answers `usage` naming `HOLLER_HERDR_SESSION` (`:198-205`) repeats `bind.rs:135-142`.</li><li>`main.rs`'s module doc (`:9-18`) lists every module but `observe`.</li></ul>**Why not a block.** This is test glue local to one target. It is not one of the overlay's named helpers, and not a guard (D11). It is the same class as W-1 to W-4 and W-6. T's reason for the new module is sound: `bind.rs` is 580 lines, and lint warns at 600. | <ul><li>One generic helper called by both suite runs, e.g. `fn assert_passes_the_herdr_suite<H: HerdrPort>(wrap: impl Fn(Arc<dyn HerdrPort>) -> H)` in `observe.rs`. This also shortens `bind.rs`.</li><li>One `CallLog` in `main.rs`, used by `Recording` and `Observing`.</li><li>One `unbuildable_lazy_herdr() -> (LazyHerdr, Arc<AtomicUsize>)`.</li><li>Add `observe` to `main.rs`'s module doc.</li><li>It can join the test-glue consolidation (W-O).</li></ul> |
41: | W-1 to W-4, W-6 | warn (low or nit; not binding), carried | as in the first and second passes | **Unchanged by this cycle:**<ul><li>W-1: a third `alive`;</li><li>W-2: `tmux_on_path`;</li><li>W-3: `trap_dir`, and the inline `sessions_for(&[])`;</li><li>W-4: the third `Feed` shape;</li><li>W-6: the no-hub text spelled three ways.</li></ul> | As in the earlier passes. |
42: | W-O | warn (carried from phase 3) | `crates/holler-cli/tests/pane_integration/bind.rs:423` | **Still three `struct LinkedHost`s, and no fourth.** The consolidation is still unfiled: at 17:04 MDT, `gh issue list --search LinkedHost --state all` finds only #649 and the closed #802. | O files it (Notes for O 2). It can absorb W-1 to W-3, W-6 and W-7. |
```

### E68  `docs/handoffs/649/handoff-S.md` lines 69-69: how long #649's four scenarios take

Command: `sed -n '69,69p' docs/handoffs/649/handoff-S.md | awk -v s=69 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
69: - **The opt-in scenario ran 4 of 4 twice on the merged tree:** F in 81.56 s, and T-green in 59.05 s with the new
```

### E69  `.github/workflows/ci.yml` lines 186-193: CI's test step

Command: `sed -n '186,193p' .github/workflows/ci.yml | awk -v s=186 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
186:       # sequentially (this test's own binary was never observed overlapping
187:       # another test binary's run), so intra-job contention was never the
188:       # mechanism. It is skipped here so its own dedicated retry (below)
189:       # re-runs only itself in ~4 minutes worst case, not the whole
190:       # multi-minute Workspace suite.
191:       - name: Workspace suite
192:         if: env.SKIP_THIS_LEG != 'true'
193:         run: cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
```

### E70  `.aftersight/pipeline.config.json` lines 5-13: the pipeline's test command and paths

Command: `sed -n '5,13p' .aftersight/pipeline.config.json | awk -v s=5 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
5:   "test": {
6:     "unit": { "command": ["cargo", "test", "--workspace", "--", "--skip", "roster_stays_accurate_under_concurrent_body_load"], "timeoutMs": 900000 },
7:     "e2e": null,
8:     "pluginSuite": null,
9:     "pluginTypecheck": null
10:   },
11:   "paths": {
12:     "test": ["crates/*/tests/**", "plugins/*/tests/**"],
13:     "production": ["crates/*/src/**", "crates/*/Cargo.toml", "plugins/*/src/**", "plugins/*/Cargo.toml", "plugins/*/herdr-plugin.toml", "Cargo.toml", "Cargo.lock", "scripts/**"]
```

### E71  `CHANGELOG.md` lines 8-10: CHANGELOG head

Command: `sed -n '8,10p' CHANGELOG.md | awk -v s=8 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
8: ## [Unreleased]
9: 
10: ### Enhancements
```

### E72  `CHANGELOG.md` lines 36-37: CHANGELOG: #649's mention of the opt-in scenario

Command: `sed -n '36,37p' CHANGELOG.md | awk -v s=36 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
36:   An opt-in scenario (`HOLLER_TEST_PANE_INTEGRATION=1`) runs the verbs end to end against scratch Herdr, tmux and OpenCode
37:   instances it starts itself, and refuses any real session. [ADR 0021](docs/adr/ADR-0021.md) records the wiring.
```

### E73  `docs/handoffs/649/handoff-T-green.md` lines 19-30: how #649's T-green applied and reported mutants

Command: `sed -n '19,30p' docs/handoffs/649/handoff-T-green.md | awk -v s=19 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
19: | `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1` (merged tree, with the new end-state check) | `ok. 4 passed; 0 failed; ... finished in 59.05s`. No new `/tmp/hlr649-*` directory and no `hlr649`/`holler649` process left (the two older dirs F named are unchanged) |
20: 
21: **The tests fail when the behaviour is removed.** Three mutants of F's `bind.rs`, each applied by a script, run with
22: `-- observe:: bind::`, and restored from a scratchpad copy (`git status` shows no change under `src/` after each):
23: 
24: | Mutant | Failing rows |
25: |---|---|
26: | M11: `BindingHerdr::observe_panes` loops over the inner `observe_pane` (correct values, wrong call shape) | `observe::the_binding_herdr_delegates_…`, `observe::the_wirings_herdr_chain_…`: `left: ["observe_pane", "observe_pane", "observe_pane"]`, `right: ["observe_panes"]` |
27: | M12: `LazyHerdr::observe_pane` answers `Ok(default)` when the build failed | `bind::a_lazy_herdr_that_cannot_be_built_answers_that_error_from_every_method`, `observe::the_lazy_herdr_passes_the_herdr_conformance_suite` |
28: | M13: `LazyHerdr::observe_panes` answers `[]` on a failed build | `observe::a_lazy_herdr_that_cannot_be_built_observes_nothing_for_each_pane` |
29: 
30: T-red already showed the no-forward state fails all seven rows. So each line F added is pinned.
```

### E74  `.aftersight/releases/claude/workflow/feature.md` lines 159-161: the Feature role: F never edits tests

Command: `sed -n '159,161p' .aftersight/releases/claude/workflow/feature.md | awk -v s=159 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
159: ## Tests that look wrong (for T)
160: [Any authored test you believe is incorrect — do NOT edit it; flag it here for T to fix in Phase 6. "None" if none.]
161: [F never edits tests. If a rework finding you were handed is test-only (nothing in production code to change), say so here instead of reporting `done` with an unchanged diff: the Workflow script routes self-reported test-only S REWORKs and classified test-only diff-gate BLOCKs to T ([#678](https://github.com/Performant-Labs/playbook/issues/678)), but a finding that reached you is one it judged production-shaped.]
```

### E75  `docs/handoffs/649-brief.md` lines 26-26: #649 D5: the Herdr session and socket, and the adapter's refusal of another session

Command: `sed -n '26,26p' docs/handoffs/649-brief.md | awk -v s=26 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
26: D5. **Herdr** is `HerdrAdapter::connect(HerdrConfig)` (E22, E23) from the environment of the process: the session is `HOLLER_HERDR_SESSION` only (audit: no `HERDR_SESSION` fallback; no source verifies that Herdr sets it in its panes, and the setup wizard marks it UNVERIFIED until #734, E64); the socket is `HOLLER_HERDR_SOCKET`, else `HERDR_SOCKET_PATH`, else `$XDG_CONFIG_HOME/herdr/sessions/<session>/herdr.sock` (`~/.config` when `XDG_CONFIG_HOME` is unset, E25); the workspaces are `HOLLER_HERDR_WORKSPACES=label=RxC[,label=RxC]...` (rows by columns, E24, E37, E38). (audit) A malformed value (an item without `=` or `x`, an empty label, a label containing `=` or `,`, a zero or non-numeric extent, a repeated label) is `usage` naming the variable, raised when the Herdr port is first used, never at `connect`. No session configured: the Herdr port answers `usage` naming `HOLLER_HERDR_SESSION` when it is first used (store-only verbs are unaffected, D2). `--herdr-session` stays required by `launch` as ADR-0021 records it (E37); the adapter refuses a name other than its configured session as `unavailable` (E24). Defaulting the flag from the wiring (ADR-0021 line 522) is **not** done here: it edits `launch.rs`, outside the Blast radius, and stays recorded as deferred.
```

### E76  `docs/handoffs/649-brief.md` lines 32-32: #649 D8: the tui_session resolver and its bindings

Command: `sed -n '32,32p' docs/handoffs/649-brief.md | awk -v s=32 '{printf "%d: %s\n", s+NR-1, $0}'`

```text
32: D8. **The `tui_session` resolver** (`Resolver<PaneId, PaneName>`, E31, E32) answers from, in this order: (a) the **bindings** of a launch in progress; (b) the pane registry (a record whose `herdr.pane_id` is the id); (c) the pane's label in Herdr's snapshot (E26: only a pane someone labelled, e.g. an imported one; `launch` never writes one); otherwise `pane-not-found` (E32). The bindings exist because the engine calls `attach_tui` with the Herdr pane id before the record exists (E34): a `HerdrPort` decorator remembers the `pane_id` that `ensure_pane` returned, and a `HostPort` decorator binds it to the name when `ensure_session(name, ..)` follows (the engine's order, E34). The bindings are per run and in memory. (A review, W-5) (b) matches a record only when `herdr.session` is the adapter's configured Herdr session and `pane_id` is the id, because Herdr numbers pane ids within a session; more than one match is `unavailable`, never a pick; (a) is a map from pane id to name, filled by `ensure_pane` and claimed by the next `ensure_session`, so a run that launches several panes stays correct. (A review, W-C) The adapter calls the resolver after it has fixed its method's deadline (`attach.rs:160-176`), so when (a) misses, (b) is one `pane/list` (10 s) and (c) one Herdr snapshot (10 s plus the lazy adapter's connect ping on a first use): a TUI method that resolves through (b) or (c) can run past its own 10 s bound when the hub or Herdr is slow. Keep (a) first so a launch never nests a port call, add no knob (D9), and record this third case beside ADR section 2's two methods that are "bounded otherwise" (`edit_spec`, `run_probe`, line 96) in D17's paragraph. This couples the wiring to the engine's call order; a test pins it, and carrying the name in `attach_tui` is a later change to the port, not this story's.
```
