# Handoff-A: #835 - `holler pane import` leaves panes Holler does not drive alone, with `--adopt-claude-code`  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-835-implementation` at `7265a9f` (base `origin/main` at `4366fb8`; the branch's one commit adds only `docs/handoffs/835-brief.md`)
**Brief reviewed:** `docs/handoffs/835-brief.md` (the issue, copied verbatim)   **Reuse map:** none (no `survey.md`); this review builds one, below   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK: 2 `block`, 5 `warn`.

The plan extends the right object. #650 already has a path for a pane that is not an OpenCode pane: `harness-not-opencode`. That pane gets `Action::Skipped`, one finding at `Level::Skipped` and no record, and it blocks nothing (`holler-pane/src/import.rs:173-174`, `import/assess.rs:85-95`, `import/findings.rs:37`, `:159`). The new default extends that path, and the switch extends the verb's own argument checks. Neither needs a new module.

What blocks is the contract amendment. The brief puts it first and leaves it open ("How the import tells the harness apart is NOT decided here"):

1. **No port contract.** The plan names no port, method, signature, default, fake or conformance case for the read-only signal. `ports.rs` is frozen and changes only by amendment. Left to the story's first step, T would design the port contract while writing RED tests.
2. **No rule.** The brief's own words ("a pane whose harness is not OpenCode") point the wrong way, for two reasons:
   - **The real fleet.** On a scratch tmux 3.7c, the shape of the old launcher's OpenCode pane reports `bash`, not `opencode`, while a Claude Code pane reports `claude`.
   - **The test world.** The world the test kit builds runs no process in any pane's tmux session.

   So a rule that needs a positive OpenCode match would leave every real OpenCode pane alone and break #650's fixtures.

Both are decisions for the amended brief. Each has a recommendation below, grounded in the code.

The five warns:

- **Finding level.** The new finding should be `skipped`, as `harness-not-opencode` is, not `info`.
- **Refusal code.** `usage` departs from the one "story not landed" code, `not-implemented`, so the brief should say why.
- **After #831.** No story owns the post-#831 adoption. As built, `profile-exists` would refuse it in the operator's own sequence.
- **Blast radius.** The plan names no files. The changes touch owned and near-limit files, including #649's `wiring.rs` if the new method is required.
- **#832.** That story, in the same wave, should reuse this signal rather than add a second one.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | Work item 3 (brief `:11`): "the contract amendment ... adds the read-only signal, for example the pane's foreground command name" | contract; dependency direction (ports) | The amendment is left to the story's first step. It names no port, method, signature, default, fake or conformance case.<br>- **The port.** `HostPort` is frozen (`ports.rs:12-24`). Each amendment under the rule decided the method's shape before any test: #645 added default-armed methods, #802 changed a signature (ADR-0021 `:115-119`, `:127-128`).<br>- **Its users.** Seven types implement `HostPort`. The test kit's conformance meta-test requires its `Mutant` wrapper to pass every case (`holler-pane-testkit/tests/host_conformance_test.rs:167-171`).<br>- **The seam.** `HostPort` is "tmux sessions and the processes in them" (`ports.rs:197`). The #830 spike found a Claude Code pane to be a normal child of the pane's tmux session (`claude-code-pane-spike.md:64`, `:252-255`). `ps` answers pids only (`ports.rs:217-218`, `holler-adapter-host/src/tmux.rs:29`).<br>- **Herdr's `pane.process_info`** returns each foreground process's full `argv` and `cmdline` (`herdr-api-spike.md:136`). A full argv can carry a secret: `NAME=value` is the shape the host adapter never echoes (`holler-adapter-host/src/lib.rs:451-465`).<br>- **Claude Code's per-process records** under its config directory are what the old launcher reads (`scripts/pane-restart:78-87`). They are off limits (spike decision 4, `claude-code-pane-spike.md:351`; I7).<br>- **Changing `ps` itself** would reach every implementation and #647's frozen `reconcile/observe.rs:134`, the only other caller. | Name the amendment in the brief. Recommended:<br>- **The method.** One new read-only `HostPort` method beside `ps`, for example `foreground_commands(name) -> Result<Vec<String>, PaneError>`: the command **name** of every live pane of the session. A missing session is `pane-not-found`, as for `ps`.<br>- **The adapter.** The tmux adapter reads `#{pane_current_command}` from the same `list-panes -s -t =NAME:` listing `ps` uses (`tmux.rs:112-124`). It never reads an argv, a cmdline or the config directory.<br>- **The shape.** State whether the method is default-armed or required (finding 6).<br>- **The records.** Add it to ADR-0021 §2's port table (`:107`) and its "amended since" sentence (`:118-119`), and to the epic's contract block (`HostPort` line). |
| 2 | block | Work item 1: "does not register a pane whose harness is not OpenCode"; item 3: "and the rule that uses it"; the Fixture | contract (the rule); consistency with the existing fixtures | The rule is unwritten. Its obvious reading fails on two things already in the repo.<br>- **The real fleet.** The old launcher starts an OpenCode pane through `oc-holler`, a bash script. Its own process match is `^bash [^ ]*/oc-holler` (`scripts/pane-restart:247`, `:301-306`). A Claude Code pane is `claude`, typed into the shell (`:153`, `:179`). A scratch probe on tmux 3.7c, on a private server, gave these values of `#{pane_current_command}`:<br>  - a typed bash script with a long-running child: `bash`;<br>  - a typed process whose argv[0] is `claude`: `claude`;<br>  - `new-window -- env -- sleep 600` (the shape of `HostPort::run`): `sleep`.<br>  So by command name, an old-fleet OpenCode pane looks like an idle shell, and a Claude Code pane can be identified.<br>- **The test world.** The world the import rig builds starts no process in any tmux session: `ensure_session`, then the server through `FakeHarness::start_server` (`tests/pane_verbs/doctor/rig.rs:380-385`). "Only a pane seen running OpenCode is imported" would turn every #650 import test's `record` into "left alone".<br>- **An unreadable signal.** The brief does not say what a port error, a timeout, or a host without the read does. When the import cannot be sure, it blocks rather than guesses (`sources-disagree`, ADR-0021 `:892-894`). An unreadable signal that silently means "OpenCode" would bring back the bug this story fixes. | State the rule for every answer in the brief:<br>- **Recommended.** A pane whose session has a live pane whose command name is `claude` is left alone, or, with the switch after #831, adopted. Any other answer (`bash`, `opencode`, an empty session) is imported as OpenCode, as today. This keeps a server-down OpenCode pane adopted and flagged `server-down`, as item 3 requires. It keeps #650's fixtures green, and it matches the old launcher's own test, `(^\|/)claude$` on the process's argv[0] (`pane-restart:86`).<br>- **Other harnesses.** Say whether any other command name (another `HARNESS_IDS` entry, `vocab.rs:272-275`) is also left alone.<br>- **Unreadable.** Choose a new `problem` kind (no record; a real run is refused) or the existing `observe-failed` warning (recorded as OpenCode), and say why. A `problem` follows the import's own fail-closed stance. |
| 3 | warn | Work item 1: "It reports an info finding for it (a new finding code ...)" | pattern consistency: finding levels in a versioned JSON contract | The import gives a pane that is left out because it is not an OpenCode pane the level `skipped`, not `info`:<br>- `harness-not-opencode` is `Level::Skipped` (`findings.rs:37`, `:159`).<br>- Its row is `Action::Skipped`.<br>- `Outcome::left_alone` gives it exactly one finding, and no position, role or session (`assess.rs:59-95`).<br>`info` is for a finding that changes nothing: `name-cell-differs`, `ignored-entry`, `already-imported`. The JSON `level` and `action` are part of the verb's versioned output (ADR-0021 `:918-919`; the help lists the values, `holler-cli/src/pane/import.rs:75-78`). A new kind at `info` with action `skipped` would give two "left out, not OpenCode" outcomes different levels. A new **code** is justified, because the switch adopts these panes and not `harness-not-opencode`'s. A code is stable once merged (`findings.rs:10`). | Make the new kind `Level::Skipped`:<br>- one constructor beside `Outcome::not_opencode`, through `Outcome::left_alone`;<br>- `Action::Skipped`, one finding and no others; no new `Action` or `Level`;<br>- the import does not ask such a pane's port for harness health or sessions.<br>Name the code in the amendment. Its message names the matched command from a constant, or quotes observed text through `findings::quoted` (`findings.rs:18-21`). Keep `info` only if the brief records why. |
| 4 | warn | Work item 2: "the switch is refused as `usage` (exit 2), naming #831" | pattern consistency: errors | The CLI has one code and one wording for "this story has not landed": `output::not_implemented(story)`. Its code is `not-implemented` (exit 1) and its message "not implemented (story #N)" (`holler-cli/src/output.rs:251-264`). Every stub uses it, and so do the legacy verbs' `--pane` and `--profile` refusals (epic ruling 5). `usage` means "a missing or conflicting argument" (`holler-pane/src/error.rs:407-409`). The issue chose `usage` explicitly, so this is the operator's call, but the brief records no reason. | Keep `usage` and record why, or use the helper. Either way:<br>- raise it in `Input::read`, step 1, before any file is read or any port is called, beside the three existing usage checks (`holler-cli/src/pane/import.rs:150-166`), so a dry run is refused too;<br>- build the "story #831" text with `not_implemented_message(831)`, so it has one spelling;<br>- keep it at that one site, so #831 replaces one place;<br>- add the case to ADR-0021 §9's `pane import` row (`:698`). |
| 5 | warn | Work item 2: "After #831 it registers them with that harness" | scope and ownership | **No owner.** No story owns the after-#831 behaviour. #831's body does not mention the switch. #835 cannot build it: `HarnessKind` has only `Opencode` (`holler-pane/src/pane.rs:136-138`), and the spike's plan replaces `HarnessInfo.port` with an endpoint (`claude-code-pane-spike.md:295-299`).<br>**Unreachable as built.** #654's direction makes the first real import create `fleet` (option (a)). A later `pane import --adopt-claude-code` would then have records to write while `fleet` exists. The import refuses that with `profile-exists`: "it adds none to a profile that exists" (`holler-cli/src/pane/import/writes.rs:47-54`, `:75-86`). | Name the owner: a line on #831, or a follow-up issue. In that place, say that adopting into an existing `fleet` needs one of two things:<br>- a change to the import's write step; or<br>- another route, such as `pane launch --profile fleet` with #831's harness.<br>This is not #835's to build. |
| 6 | warn | The plan as a whole: no blast radius | size and structure; coexistence with stories in flight | The plan names no files, and the amendment touches shared, owned and large ones:<br>- **The port.** `ports.rs` (frozen, #637's).<br>- **The test kit.** `FakeHost`. `HostOp`, whose `as_str` must equal the adapter's `OP_*` constant (`lib.rs:137-142`). The conformance suite, and the `Mutant` meta-test.<br>- **The host adapter.** `lib.rs` (499 lines) and `tmux.rs`. Its command-line test file, `tests/fake_tmux_test.rs`, is at 818 lines, past the overlay's ~800.<br>- **If the method is required**, four more types need it: `wiring.rs`'s `Unwired`, owned by #649, which is running now (`wiring.rs:8-13`, `:151-167`); `tests/pane_verbs/launch/rig.rs` (814 lines); `tests/profile_verbs/apply/rig.rs` (759); and `holler-pane/tests/ports_test.rs` (596). A **default-armed** method, #645's precedent, touches none of them. Its default must fail loudly (a refusal, as `send_prompt`'s does, `ports.rs:291-299`), never "nothing runs": an implementation that does not override it would then import a Claude pane as OpenCode.<br>- **The import tests.** The read allow-list admits `HostOp::Ps` only (`tests/pane_verbs/import/rig.rs:474-482`). `import/observe.rs:11-12` says a chain makes at most three port calls. Both change on purpose.<br>- **Coexistence.** #828, running alongside, edits ADR-0021 §8 and `CHANGELOG.md`. #652's guide (`docs/pane-control.md`, not on main) describes the import. | List the blast radius in the brief, by owner:<br>- Choose default-armed or required. If required, coordinate `wiring.rs` with #649.<br>- Put the new adapter tests in a new file, not `fake_tmux_test.rs`.<br>- Keep ADR-0021 edits to §2, §9, §13 and the closing lists.<br>- Note the new default on #652.<br>- The epic's contract block (a GitHub issue body) is for O or the operator to edit, not F. |
| 7 | warn | None: a story in the same amendment wave | duplication across concurrent stories | #832 (same wave; worktree open, no commits) lists "a pane whose harness Holler cannot drive" as `unmanaged`, with a fixture "with a harness the adapters do not know". If it needs to know what an unregistered pane runs, it could add a second signal beside this story's `HostPort` read. Herdr's screen-detected `agent` is one candidate (`herdr-api-spike.md:137`, `:289-302`). | Note on #832 that the `HostPort` read and the rule exist (or will) here, so it reuses them rather than adding another. Keep the rule in one place in `holler-pane`. |

## Notes for O

The automated path stops on this BLOCK (`needs-amendment`). Amend `docs/handoffs/835-brief.md` with these decisions, then start a **fresh** run, not `resumeFromRunId`.

1. **The signal (finding 1).**
   - Name the port and the method, with its signature. Recommended: one read-only `HostPort` method beside `ps`.
   - Say what it answers for a missing session. Recommended: `pane-not-found`, as `ps` does.
   - Say what tmux format it reads. Recommended: `#{pane_current_command}` from `ps`'s own `list-panes -s -t =NAME:` listing.
   - State that it reads command names only: never an argv, a cmdline, or Claude Code's config directory.
   - Say whether it is default-armed or required. If default-armed, say what the default answers (a loud refusal).
2. **The fake and its tests (finding 1).**
   - `FakeHost` answers the basename of argv[0] for each live process that `run` started, as tmux reports `sleep` for `env -- sleep`. A session with no process answers an empty list.
   - Add one conformance case and its `Mutant` break. Have the `Mutant` wrapper delegate the new method, so `the_unbroken_host_wrapper_passes` still passes.
   - The issue's fixture then needs no new scenario method: `host.run(name, cwd, ["claude", ...])` in the left-alone pane's session.
3. **The rule, for every answer (finding 2).**
   - A live pane whose command name is `claude`: left alone. With the switch, after #831: adopted.
   - Anything else, an empty session included: imported as OpenCode, as today.
   - An unreadable signal: a new `problem` kind or the existing `observe-failed` warning, with the reason.
   - Whether other harness names are also left alone.
   - Put the rule's names (`claude`) in one constant in `holler-pane`, so #832 and #831 can reuse it.
4. **The finding (finding 3).**
   - The code name.
   - `Level::Skipped`, `Action::Skipped`, one finding through `Outcome::left_alone`.
   - No harness calls for such a pane.
   - A message that names a constant, or quotes observed text through `findings::quoted`.
5. **The switch (finding 4).**
   - The code, `usage` or `not-implemented`, with its reason.
   - The single site in `Input::read` step 1, so a dry run is refused too, and the wording from `not_implemented_message(831)`.
   - The documentation: ADR-0021 §9's `pane import` row (`:698`), the `PaneImport` help text (`import.rs:49-78`), ADR 0003's row (`:63`), and a `cli-surface.txt` line with the flag (`:166-167`).
6. **After #831 (finding 5).** Name the owner of the adoption. Record the `profile-exists` interaction where that owner will see it.
7. **The blast radius (finding 6).** List it by owner, with the coexistence notes:
   - #649 owns `wiring.rs`, if the method is required.
   - #828 edits ADR-0021 §8 and `CHANGELOG.md`.
   - #652's guide describes the import.
   - #832 should reuse the signal (finding 7).

## Reuse map (built by this review; for the amended brief and the Phase 7 gate)

| Need | Extend, do not copy | Where |
|---|---|---|
| A pane left alone | `Outcome::left_alone`, plus a constructor beside `not_opencode`. `Action::Skipped`. `ImportFinding`: one new kind in `ALL`, `code` and `level`. | `holler-pane/src/import/assess.rs:59-95`; `import.rs:66-89`; `import/findings.rs:62-165` |
| The observation | `observe()`'s chain: one more read on the same per-pane thread. `tmux_finding` and `failed_message` for its errors. | `import/observe.rs:79-84`, `:155-181` |
| The port read | `HostPort`, beside `ps`. In the tmux adapter: the `panes()` listing, a strict parser beside `parse_ps`, and `Call` and `refused()` for its errors. | `holler-pane/src/ports.rs:203-219`; `holler-adapter-host/src/tmux.rs:112-124`, `:214-228`; `src/lib.rs:383-392`, `:439-449` |
| The fake and its pins | `FakeHost` and a new `HostOp` variant; a case in `conformance/host.rs`; a `Break` in the `Mutant`. | `holler-pane-testkit/src/host.rs`; `src/conformance/host.rs`; `tests/host_conformance_test.rs` |
| The switch | The `PaneImport` Args, `Input::read` step 1 and `usage()`. `output::not_implemented_message`. | `holler-cli/src/pane/import.rs:79-96`, `:149-166`, `:218-222`; `output.rs:251-255` |
| The docs | ADR-0021 §2 (`:107`, `:115-119`), §9 (`:698`), §13 (`:863-899`); ADR 0003 (`:63`); `cli-surface.txt` (`:166-167`); `CHANGELOG.md` | as listed |

At Phase 7, flag any of these as a parallel path:

- a second tmux runner or listing in the adapter;
- a second per-pane thread scaffold in the import;
- an import-private copy of `ps`'s error mapping;
- a new `Action` or `Level`;
- a second classification of command names outside the one constant.

## Patterns referenced

- `crates/holler-pane/src/import.rs`, `import/{assess,observe,findings,files}.rs`: the #650 plan, its skip path and its level taxonomy.
- `crates/holler-pane/src/ports.rs` and ADR-0021 §2: the frozen ports, and the #645 and #802 amendment precedents.
- `crates/holler-adapter-host/src/{lib,tmux}.rs`: how `ps` lists a session's panes.
- `crates/holler-pane-testkit/src/{host.rs,conformance/host.rs}` and `tests/host_conformance_test.rs`: the fake, the conformance suite and its mutant pins.
- `docs/research/claude-code-pane-spike.md` §6 and decision 4; `docs/research/herdr-api-spike.md` (`pane.process_info`); `scripts/pane-restart` (how the old fleet runs each kind of pane).
