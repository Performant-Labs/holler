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
