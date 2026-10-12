# Decision journal: #831, a Claude Code harness for panes

Run handoff directory `docs/handoffs/831` · branch `issue-831-implementation` · rigor in-session · uiSurface false (D and U
are N/A). Append-only; every phase adds its entry. The contract amendment is commit `008a995` (ADR-0021 section 15, the
brief, the research note's "Decisions").

## A, up-front plan review: 2026-10-11T18:43-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 15 `warn` findings (`docs/handoffs/831/handoff-A.md`).
  - **The plan extends the right objects:**
    - the port is changed in place (`HarnessEndpoint`, the #802 decision 2 precedent);
    - `HarnessInfo.port` becomes optional;
    - the facts about a kind are methods of `HarnessKind`;
    - `Ports` keeps one `harness`, behind a router;
    - there is one new adapter crate, a dependency of `holler-cli` only;
    - the test kit's fake and conformance pattern;
    - the engines, not a parallel launcher.
  - **The choke point holds.** A Claude pane gets no CLI-side `say`. The acceptance's `say` step is honestly marked as blocked by #839 and #850.
  - **The warns:**
    1. A SingleProcess `switch` loses its `session-not-found` check. Items 5 and 6 would start and record a new conversation for any unknown id.
    2. `doctor --fix` becomes a relaunch with no idle gate. It drops a turn, its record write conflicts with the relaunch's, and it contradicts reconcile's contract and ADR §8.
    3. Reconcile's chain is undefined for a single process (`list_sessions`), and doctor-clean is an acceptance line.
    4. The stop, run, wait and record sequence needs one entry point (`tx_launch::relaunch` plus a session choice). `reset --first` would send to the stopped pid.
    5. The router must be one type in `holler-pane`, because the test kit is a dev-dependency of `holler-cli`. It also needs a routing table.
    6. #835 pinned `CLAUDE_CODE_COMMAND` and `is_claude_code_command`, not `CLAUDE_COMMAND`.
    7. Adding `HarnessKind::Claude` flips the import's `harness-not-opencode` rule (`import/files.rs`). A merged test catches it.
    8. The socket-path file needs a writer, a location, a key, an atomic write and a fail-closed read.
    9. `login_status`'s `NotApplicable` default fails open.
    10. `ServeSpec`'s new fields, `process_command` in the plan, and where each open code is declared.
    11. A launch has no stand-in for `port-in-use` (two `claude` processes in one session).
    12. `reset --first` is the existing carve-out around the hub's prompt gate, which #828 is moving.
    13. `--harness` belongs in `pane/args.rs`; one port-policy rule; the `HarnessInfo` `try_from` pattern.
    14. Sizes: `tx_launch.rs` 831, and on `origin/main` `launch/rig.rs` 835 and `switch.rs` 868.
    15. Coexistence with #840, #849, #835 and #828; the issue's "by the recorded pid" against `stop_owned`.
- **Assumed:**
  - T reads `handoff-A.md` before authoring RED, as #808's T did, and pins W1 to W4, W11 and W12 as its "Notes for T" say.
  - There is no O in this automated path, so F records the section 15 corrections "as built" (the brief allows F "as built" facts), and the PR body lists each edit to the amendment.
  - #835 will merge before this story reaches F's adoption step. Its F is in progress in its worktree: `pane.rs` and `ports.rs` are modified and uncommitted, and its T-RED is committed at `36b0de8`.
- **Hedged:**
  - **W1 and W2 correct normative text of section 15.** The amend-first rule would prefer those corrections before T. I rated them `warn` and not `block` because each fix keeps an existing contract (switch's refusal; reconcile never drops work), and none needs an operator decision. "Notes for O" tells the operator how to amend before T instead.
  - **W2's mechanism** (the caller-supplied act, or `pane doctor` after the pass) is left to F. Both keep reconcile from importing `tx_launch`.
  - **W8's key** (pid or session id) is left to F. Both meet the property that matters, no stale socket.
  - **No watchdog code exists in the repository.** The acceptance's "the watchdog sees the Claude pane" can only be judged by S.
- **Evidence:**
  - Read:
    - the brief;
    - the amendment's diff (`git show 008a995`);
    - ADR-0021 §§1, 2, 8, 12, 13, 14 and 15, "Deferred" and "Decisions taken";
    - `docs/research/claude-code-pane-spike.md` in full;
    - #835's brief and handoff-A in its worktree, its `claude_code_rule_test.rs` at `36b0de8`, and its uncommitted `pane.rs` and `ports.rs`;
    - #828's brief.
  - Code read: `holler-pane` `ports.rs`, `pane.rs`, `tx_launch.rs`, `tx_launch/server.rs`, `tx_switch.rs`, `reconcile.rs`, `reconcile/observe.rs`, `tx_apply/observe.rs`, `profile_snapshot.rs`, `findings.rs`, `error.rs` (head), `import/files.rs` and `lib.rs`; `holler-adapter-opencode/src/lib.rs`; the test kit's `harness.rs`, `conformance/mod.rs` and `conformance/harness.rs`; `holler-cli` `wiring.rs`, `wiring/bind.rs`, `pane/args.rs`, `pane/launch.rs`, `pane/relaunch.rs`, `pane/import/writes.rs`, `say_cmd.rs`, `interrupt_cmd.rs` and `prompt_target.rs`; `holler-hub/src/circuit/dispatch.rs` (`pane_gate`, `send_prompt`); `holler-proto/src/vocab.rs` (`HARNESS_IDS`).
  - Greps:
    - every `impl HarnessPort`;
    - every production use of `harness.port`, `HarnessKind` and `Activity::`;
    - `LinkedHost` and `start_server` in tests;
    - `harness-not-opencode` fixtures;
    - `#[ignore` conventions.
  - Checked: `holler-cli/Cargo.toml` (the test kit is under `[dev-dependencies]`), `scripts/lint.sh` (warns at 600 and fails at 900, test files included), and `.aftersight/pipeline.config.json` (paths).
  - `git fetch`: `origin/main` is `bda1069`, and #840 and #849 merged after the base. `wc -l` of the hot files on the branch and on `origin/main`.

## T, author RED: 2026-10-11T19:10-06:00 (MDT)

- **Decided:** RED is valid (`docs/handoffs/831/handoff-T-red.md`). It is compile-level for the contract, as #802's round 1 was, plus one assertion-level RED: the CLI surface fixture's two `--harness` lines.
  - **The suite:** 96 new tests in 11 new test files, 2 of them ignored (the opt-in real-binary test and the `say` step). Four existing test files were also changed: `rework_test.rs`, `pane_verbs/main.rs`, `pane_integration/bind.rs` and the CLI surface fixture.
  - **New targets:** `pane_claude` is a new holler-cli `[[test]]`, so `pane_verbs` and `profile_verbs` compile again once the endpoint step lands alone.
  - **What it pins:**
    - A's W1, W2, W3, W4, W11 and W12 as asked, and W5, W9, W10 and W13 as A suggested.
    - The operator's abort decision: interrupt is a relaunch on the session of record, `abort` is `abort-unsupported`, and nothing is typed.
    - The acceptance scenario on the fakes, with its `say` step `#[ignore]` naming #839 and #850.
  - **The contract's names** are fixed in handoff-T-red, as #808's T did. Among them: `HarnessEndpoint`, `SessionChoice::{Fresh, Keep, Resume}` (W1's choice), `LoginStatus`, `PROCESS_UNSUPPORTED`, `Shape` and `AbortStyle`, `HarnessInfo::endpoint -> Result`, `ServeSpec::{with_model, with_cwd}`, `FakeClaude`, `FakeHarnesses`, `conformance::claude` (12 cases, one mutant each), `ClaudeHarness::with_seams`, the three raw-I/O seams, `interrupt_pane` and `PaneInterrupt`, `answer_pane_target`, and `SpecValues.harness`.
- **Assumed:**
  - The mechanical endpoint migration of the existing tests is F's endpoint step: about 280 call sites, plus the `HarnessInfo { port }` literals. A's note 3 and the brief say "the whole existing suite stays green with no assertion edited". T migrated only the three files it edited for other reasons: `pane_verbs/main.rs`, `pane_integration/bind.rs` and `rework_test.rs`.
  - The `claude agents --json` fixture shape (a JSON array of the spike's entries) and the hook's `$PPID` or stdin `session_id` are as the spike describes. The opt-in real test is the guard.
  - #835's `HostPort::foreground_commands` and `FakeHost`'s basename naming land before F, or F builds them to #835's committed API, per the brief.
- **Hedged:**
  - **The crate skeleton.** T wrote `crates/holler-adapter-claude/Cargo.toml`, with test dev-dependencies only, and an empty `src/lib.rs`. Without them, the workspace's `crates/*` glob fails every build. They are the bare minimum, and F owns both.
  - **Not authored:**
    - Item 17 (adoption) needs #835 on `main`. T authors it at a re-entry when F reaches it.
    - `harness-dialog-open` has no fake state in item 15. It is left as an advisory.
    - The adapter's `inbound-held` window is pinned on the fake only.
    - The roster and the watchdog: neither reads the registry on `main`. S judges that line.
  - `reset_first_reaches_the_new_process` depends on #828 not having moved `--first` yet. If #828 lands first, the test goes back to T.
  - No trial implementation proves assertion-level RED beyond the fixture. T-green spot-checks that the key pins fail with their behaviour removed.
- **Evidence:**
  - Read: the brief; ADR-0021 section 15; handoff-A; the spike note (sections 1 to 6, the recommended design and the test kit); #835's diff on its branch (`pane.rs`, `ports.rs`, `host.rs`).
  - Code read: `ports.rs`, `pane.rs`, `tx_launch.rs` (head), `tx_switch.rs` (checks), `reconcile.rs` and `findings.rs`; the test kit's `harness.rs`, `conformance/{mod,harness}.rs` and `fixture.rs`; `pane/{args,launch,list,doctor,close,park}.rs`; `say_cmd.rs` and `interrupt_cmd.rs`; the rigs `launch/rig.rs`, `doctor/rig.rs`, `apply/rig.rs` and `list.rs`; `pane_integration/bind.rs`; `verb_harness`; `rework_test.rs`.
  - Ran:
    - `cargo test -p holler-cli --test cli_surface_test`: 1 failed, the 2 lines.
    - `--no-run` of every new or changed target: only contract items unresolved.
    - `scripts/lint.sh`: exit 0.
    - `rustfmt --check` on the new files.

## F, implement: 2026-10-11T20:54-06:00 (MDT)

- **Decided:** done (`docs/handoffs/831/handoff-F.md`). The contract of `handoff-T-red.md` is built in A's order. Every
  authored test is GREEN except `findings_test.rs`'s stable-codes list, which T must extend with the two new kinds.
  - **W1:** `SessionChoice::{Fresh, Keep, Resume}`. A switch's resume-only choice is `session-not-found` in the plan,
    before anything stops.
  - **W2 and W3:** `reconcile_with(ports, request, Option<Relaunch>)`. `holler pane doctor` passes
    `relaunch_recorded(Keep)`. The repair runs only while the process is idle, and the chain writes no record after a
    relaunch. A's open choice of mechanism was taken this way so the chain knows it relaunched.
  - **W4 and W10:** one entry point, `tx_launch::relaunch_on` and `relaunch_recorded`. `process_command` runs first in
    the plan. `reset --first` goes to the stored record's endpoint.
  - **W5:** `holler_pane::HarnessRouter<S, P>`, with a routing table by endpoint variant and by call.
  - **W6:** #835's `CLAUDE_CODE_COMMAND` and `is_claude_code_command`, and its required `HostPort::foreground_commands`,
    were copied verbatim from its branch (`9bd101b`, `36b0de8`), because #835 is not on `main`.
  - **W7:** the import's rule is `!= Some(Opencode)`.
  - **W8:** the hook is fixed text and writes `<state>/claude-inbox/$PPID` atomically under `umask 077`. The read fails
    closed and trusts a file only for a pid listed now. A's open choice of key was taken as the pid, so a `/clear`
    rewrites the same file.
  - **W9:** `LoginStatus {Ok, Missing}`. Every default refuses `process-unsupported`.
  - **W11:** `process-in-use` through `foreground_commands`.
  - **W13:** `--harness` in `SpecFlags`, with one policy rule, `port_for`. `HarnessInfo` uses `try_from`.
  - **W14:** `tx_launch.rs` is split into five submodules (747 lines).
  - **Under the standing rules:**
    - a new port code, `server-unsupported`, for a single-process harness asked for a server's or a TUI's step;
    - the login variables are a fact of the kind, refused by the engine's plan (`claude-login-env-refused`);
    - the login check runs once per pass, only with a single process in scope.
  - **Docs:**
    - ADR-0021: "as built" in sections 8, 14 and 15, the section 9 rows, "Deferred", and decisions 26 to 28;
    - `docs/pane-control.md`, `docs/orchestrating.md`, ADR-0003 and `CHANGELOG.md`.
  - **Merge:** `origin/main` (`d6ae539`: #849, #840, #851) was merged with `--no-commit`, and `MERGE_HEAD` is left for
    the driver's commit. A rebase was impossible: `008a995` is pushed, and the driver does not force-push.
- **Assumed:**
  - The endpoint migration of the existing tests is F's endpoint step, as T's handoff and A's note 3 assign it. It was
    done with no assertion edited, and Phase 6 checks that.
  - Claude Code runs a hook's shell as a direct child, so `$PPID` is the pid that `claude agents --json` lists. This is
    not verified live. If it is wrong, `send_prompt` fails closed (`unavailable`).
- **Hedged:**
  - **Item 17 is not built:** #835 is not on `main` (checked at F's start and end). The PR says "Part of #831".
  - **The opt-in real test was not run by F:** it uses the real login and a model turn. Its case 9 is the live guard for
    the frame and the hook.
  - **Two test-only findings for T:**
    - `findings_test.rs`'s `CODES` gains `pane-waiting` and `claude-login-missing`;
    - `pane_claude/mixed.rs:242` trips `clippy::never_loop`, a CI step.
  - **#835's verbatim files** keep their own formatting, so the two branches merge identical hunks.
- **Evidence:**
  - Ran:
    - `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`: 2487 passed,
      1 failed (above), 34 ignored;
    - the T targets: `pane_claude` 42 passed and 1 ignored, `claude_conformance_test` 4, `fake_claude_test` 13,
      `fake_harnesses_test` 5, `claude_kind_test` 15, the adapter's `hermetic_test` 15, `cli_surface_test` 3,
      `pane_integration` 67;
    - `cargo clippy --workspace --all-targets -- -D warnings`: only `mixed.rs:242`;
    - `scripts/lint.sh` (exit 0), `cargo machete` (clean) and `scripts/changelog-check.sh` (ok);
    - the docs tests (`docs_cli_test`, `docs_pane_control_test`, `docs_errors_test`, `docs_wire_test`): all pass.
  - Read:
    - `handoff-A.md`, `handoff-T-red.md` and the brief;
    - ADR-0021 sections 2, 8, 9, 14 and 15;
    - the spike note;
    - `docs/pane-control.md` in full;
    - #835's branch diff.
  - `git fetch` at the end: `origin/main` is `d6ae539`, and #835 is not merged.
  - Evidence of unchanged code: `docs/handoffs/831/evidence.md`.

## T, verify GREEN: 2026-10-11T21:10-06:00 (MDT)

- **Decided:** GREEN, with no blocking issue (`docs/handoffs/831/handoff-T-green.md`). PASS.
  - I repaired the two tests F flagged, both test-only:
    - `findings_test.rs`: `CODES` gains `pane-waiting` and `claude-login-missing`;
    - `pane_claude/mixed.rs`: the ignored `say` test's `panic!` moved after the loop (`clippy::never_loop`).
  - No production code was changed.
- **Assumed:**
  - `findings_test`'s list is a stable-codes pin from `main` (#832) that the contract's append (item 13) must extend.
    It is not a regression in F's code.
  - Whether to run the opt-in real Claude Code test is the operator's or S's decision. It needs the real login and a
    model turn.
- **Hedged:**
  - The mutation spot-check covers six pins (W1, W2, W4, W11, the frame, and the W9 default), not every test.
  - My first W1 and W2 mutants did not compile. I replaced them with compiling mutants and re-ran both, so no "kill"
    rests on a compile error.
- **Evidence:**
  - `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`: 2488 passed,
    0 failed, 34 ignored.
  - Exit 0 for each of:
    - `cargo clippy --workspace --all-targets -- -D warnings`;
    - `scripts/lint.sh`, `scripts/changelog-check.sh` and `cargo machete`;
    - `docs_cli_test`, `wire_selftest` and `cargo test -p holler-proto`;
    - `scripts/test-hooks.sh` and `scripts/tests/pane-restart-test.sh`.
  - The six mutants were each killed by their pinning test (the table in `handoff-T-green.md`).
  - Assertion counts are equal between `d6ae539` and HEAD in eight migrated test files.
  - `FakeHarnesses` wraps `holler_pane::HarnessRouter` (`harnesses.rs:22`).
  - `evidence.md` gained one T entry (`error.rs:239-245`).
