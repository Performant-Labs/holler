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

## A, anti-duplication gate: 2026-10-11T21:17-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 9 `warn` findings (`docs/handoffs/831/handoff-A-dup.md`). Diff `d6ae539..d137a1d`.
  - **None of the ten parallel paths that handoff-A's Reuse map listed exists:**
    - one router (`HarnessRouter`, built by the wiring and wrapped by `FakeHarnesses`);
    - one relaunch sequence (`relaunch_on` and `relaunch_recorded`, called by switch, reset, interrupt and doctor's
      repair), and reconcile does not import `tx_launch`;
    - one wait loop (`wait_until`);
    - one rule each for the port policy (`port_for`), the harness check (`check_harness_unchanged`), the login variables
      and the command name;
    - the `Recording` forwarding pin, extended;
    - no per-rig `FakeClaude` linker;
    - each code declared where it is raised.
  - **The warns:**
    1. `parse_harness` repeats the import's `harness_kind`, and `HarnessKind::as_str` is not pinned to the serde names.
    2. The single-process reconcile chain copies the server chain's mismatch and `--fix` skeleton, the orchestrator skip
       predicate included.
    3. `relaunch_recorded`'s `Launched.pane` is unwrapped three times, twice with the same message.
    4. `interrupt_cmd::route` copies `route_target_by`'s `--pane` arm.
    5. `holler-adapter-claude/src/exec.rs` is a third runner copy, by the documented #696 pattern.
    6. The three old `LinkedHost`s keep their own linking, and two docs overstate "no rig writes a linker".
    7. `wiring/env.rs` spells `"claude"` beside `CLAUDE_CODE_COMMAND`.
    8. The #835 copy is not exactly "identical hunks" (two doc lines and the constant's position), and `origin/main`
       moved to `0f5835e`, whose trial merge conflicts only in `CHANGELOG.md`.
    9. `switch.rs` is at 893 lines and `launch/rig.rs` at 852.
- **Assumed:**
  - The three un-migrated OpenCode rigs follow the brief's no-edited-assertion, no-changed-rig-shape rule, so they are a
    doc fix or a follow-up, not drift.
  - `ClaudeOp`'s repeat of `HarnessOp` is by design: the map said "`HarnessOp`-style ops", and T's RED named it.
- **Hedged:**
  - W2 is close to the line between a sibling chain and a near-copy. I rated it `warn` because the plan's W3 asked for
    a distinct single-process chain, and F's chain reuses the parent's helpers (`observe_screen`,
    `check_session_of_record`, `record`, `summary`).
- **Evidence:**
  - Read in full:
    - `ports.rs` (diff), `ports/{process,router}.rs` and `pane.rs` (diff);
    - `tx_launch.rs` and `tx_launch/{process,rules,undo,budget}.rs`, `tx_launch/server.rs` (diff) and `tx_switch.rs`
      (diff);
    - `reconcile.rs` (diff), `reconcile/observe.rs` and `reconcile/observe/process.rs`;
    - `tx_apply` (diff), `profile_snapshot.rs`, `findings.rs`, `unmanaged.rs` and `import/*` (diff);
    - the adapter's `lib.rs`, `argv.rs`, `agents.rs`, `inbox.rs`, `seams.rs`, `exec.rs` and `Cargo.toml`;
    - the CLI's `wiring.rs`, `wiring/{bind,env}.rs`, `say_cmd.rs`, `interrupt_cmd.rs`, `answer_cmd.rs` and
      `pane/{args,doctor,get,launch}.rs` (diffs), and `pane/relaunch.rs`;
    - the test kit's `harnesses.rs`, `harness.rs` (diff), `claude.rs` (head) and `conformance/claude.rs`;
    - the rigs `pane_claude/rig.rs`, `launch/rig.rs`, `apply/rig.rs` and `pane_integration/bind.rs`.
  - Greps:
    - every `impl HarnessPort` and `impl HostPort`;
    - the string literals `"claude"` and `"opencode"`;
    - every `HarnessEndpoint` construction, and the raw `harness.port` and `pid` reads;
    - `thread::sleep` in `holler-pane` and the adapters;
    - `port_of_policy` and `port_for` callers, `relaunch_recorded` callers and "stored no record".
  - Diffed against `issue-835-implementation`: the host adapter, `FakeHost`, the host suite and the forwards are
    identical; the doc lines of `pane.rs` and `ports.rs` differ.
  - Ran:
    - `diff` of the OpenCode and Claude `exec.rs` files;
    - `gh issue view 696`;
    - `git merge-tree` of HEAD with `origin/main` `0f5835e`: a conflict in `CHANGELOG.md` only;
    - `wc -l` of every touched `.rs` file.

## S, spec audit: 2026-10-11T21:40-06:00 (MDT)

- **Decided:** REWORK (`reworkKind: production`), three items (`docs/handoffs/831/handoff-S.md`). They are one production
  literal and two statements in the contract. No behaviour changes.
  - **The acceptance lines are covered.** Every line has proving evidence, or the brief's own rule defers it:
    - the `say` step waits for #839 and #850, and its ignored test names both;
    - the roster line waits for #648, since no roster or watchdog reads the registry yet;
    - item 17 waits for #835, which is not on `main`.
  - **Section 15 is built as recorded,** and so are the operator's abort decision, A's W1 to W14 and the relayed request
    (amend first, rigor, worktree, the answer about `say` and #839, and the #835 coordination in code).
  - **Item 1:** section 15 item 1 says that the doc comment of `HARNESS_IDS` records the `claude` id's two uses (pane
    and body). `holler-proto/src/vocab.rs:269-271` was not changed, and no as-built note records why. A silent deviation.
  - **Item 2:** `wiring/env.rs:107` spells `"claude"` a second time. Item 2 makes `CLAUDE_CODE_COMMAND` the one place the
    name is written (A-dup W7).
  - **Item 3:** the as-built sentence "text for text ... identical hunks" in item 2 is not true. The API and the host-side
    code match #835's branch; the doc comments of the constant, the rule and `foreground_commands` do not (A-dup W8).
- **Assumed:**
  - T's and F's recorded runs stand. I re-ran no build or test, as the role says.
  - My checks were read-only: `git diff`, `show`, `log`, `diff --check` and `merge-base`; `grep` and `wc -l`;
    `changelog-check.sh`; `gh issue view` of 831, 633, 828, 835, 839, 850 and 648.
  - The script commits this handoff and this entry at the phase commit.
- **Hedged:**
  - **Why REWORK and not PASS with advisories.** No behaviour is wrong, but the PR would put three false statements
    into the contract, one of them about the #835 coordination that the operator asked for. #835's S held its own
    contract statements to the same bar: its REWORK 1 and 2, docs only.
  - **The test gaps are advisory, not REWORK.** Two of them are documented codes: `harness-dialog-open` has no test at
    all, and `session-holds-permission` is asserted only by its exit code. Each is a decided behaviour that is
    implemented, not an acceptance line. F's and T-green's handoffs say the adapter's hermetic mapping pins
    `harness-dialog-open`; no test does. I asked T-green to add the rows in this loop (Advisory 4).
  - **The opt-in real test is not a merge condition.** The issue makes it opt-in and `#[ignore]`. It should run before
    #831 closes or a Claude Code pane is launched on the fleet. The hook's `$PPID` key and the frame stay unverified, and
    both fail closed.
- **Evidence:**
  - Live:
    - `gh issue view 831`: the body equals the brief's issue section, with no comments;
    - #633's amendment comment, 18:12 MDT;
    - #828 is closed as not planned (15:56 MDT), absorbed by #839;
    - #835, #839, #850 and #648 are open;
    - `git fetch`: `origin/main` is `0f5835e`; the merge base is `d6ae539`.
  - Diff:
    - `git diff origin/main...HEAD`: 119 files, +12019/-1302;
    - `issue-835-implementation` at `f1d1bd0`, diffed: the host code is identical, and the doc comments of `pane.rs:225-244`
      and `ports.rs:330-339` differ.
  - Read:
    - the brief, every handoff, `decisions.md` and `evidence.md`;
    - the full production diff (`holler-pane`, `holler-adapter-claude`, `holler-cli`, the test kit's `harnesses.rs` and
      `conformance/claude.rs`);
    - ADR-0021 sections 1, 2, 8, 9, 14 and 15, "Deferred" and decisions 22 to 28, ADR-0003, `docs/pane-control.md`,
      `docs/orchestrating.md`, the CHANGELOG and the research note;
    - the tests `pane_claude/{mixed,lifecycle,doctor,rig}.rs`, `hermetic_test.rs`, `real_claude_test.rs`, the forwarding
      pin in `pane_integration/bind.rs`, `rework_test.rs` and `claude_kind_test.rs`.
  - Greps:
    - `unwrap`, `expect`, `panic` and `#[allow` in the added lines;
    - every `"claude"` literal in `src`;
    - private names, IPs, home paths and secrets in about 12,000 added lines: none;
    - every test of `harness-dialog-open`, `session-holds-permission`, `server-unsupported` and `foreground_commands`;
    - `wc -l` of every touched file (the largest `.rs` is 893).

## F, rework after S's REWORK: 2026-10-11T22:05-06:00 (MDT)

- **Decided:** done (`docs/handoffs/831/handoff-F.md`, "Rework 1"). The three REWORK items, the merge of `origin/main`,
  and the advisories that are this PR's own doc statements. No behaviour changes.
  - **REWORK 1:** the doc comment of `HARNESS_IDS` says what section 15 item 1 says (the pane and the body uses of
    `claude`, `HarnessKind`, `holler-body` and ADR 0012, the pinning test), and no longer calls the other ids reserved.
  - **REWORK 2:** `claude_bin` passes `CLAUDE_CODE_COMMAND`; its doc names the `HOLLER_CLAUDE_BIN` basename limit.
  - **REWORK 3:** section 15 item 2's "As built" states what holds against #835's branch at `f1d1bd0`: the API and the
    host-side code are identical, the doc comments and the constant's place are not, so the second to land resolves a
    doc-comment conflict. Re-copying #835's doc comments was rejected: #835 rewords them again at each of its reworks.
  - **The merge:** `origin/main` at `037405f` (#855, #856) merged with `--no-commit`. It conflicted in `CHANGELOG.md`,
    ADR-0003 and ADR-0021, each resolved by keeping both sides. No code changed for it.
  - **The roster:** with #648 merged, a Claude Code pane has no roster row until a body serves its session (#850). It is
    recorded in section 15 item 18, `docs/pane-control.md` section 4 and the CHANGELOG. The roster's code is not changed:
    listing a pane with no hub row would change #648's decided output.
  - **Advisories done:** 5 (the "one linker" and the subscription-login wordings), 6 (item 18's #828 sentence), 7, 8 and
    9. Not done: A-dup W1 and W3, which S lists as follow-ups (Advisory 10).
- **Assumed:**
  - The driver's phase commit concludes the merge, as it did for `881ebe1` (two parents). The three conflicted files are
    staged by path, so the index holds no unmerged entry.
  - #839 edits the older #828 sentences of ADR-0021 (sections 5 and 8, "Deferred") and of #649's CHANGELOG entry, as its
    issue says it will ("whichever lands second edits the other's words"), so they are left as they are.
- **Hedged:**
  - **S's acceptance row 8 moves.** It said the roster waits for #648. #648 is now on the branch, and the roster still
    shows no Claude Code pane until #850. S judges whether the line is met as the brief reads it ("where they read the
    registry"); the gap is recorded either way.
  - **Merging now widens what T-green, A-dup and S see** (#855's and #856's files arrive through the merge). They are
    `main`'s, reviewed there, and `git diff origin/main...HEAD` leaves them out once `origin/main` is the merge parent.
- **Evidence:**
  - Read: `handoff-S.md`, `handoff-A-dup.md`, `handoff-F.md`, the brief, ADR-0021 section 15; the roster code
    (`roster_cmd.rs` on `037405f`), `pane/list.rs`'s `PaneRow`, the hub's `panes/{persist,store}.rs`, the wiring's
    `bind.rs` and `wiring.rs`, `reconcile.rs`'s `observe_all`; ADR 0012, `holler-body/src/config.rs`, `docs/protocol/v2.md`
    section 9 and the spike note's item 1.
  - `gh issue view`: 835 (OPEN), 828 (CLOSED, not planned, 15:56 MDT), 839 (OPEN; "absorbs #828").
  - `git diff issue-835-implementation HEAD` (`f1d1bd0`): `holler-adapter-host/src`, the test kit's `host.rs` and
    `conformance/host.rs`, `fake_tmux_test.rs`, `foreground_test.rs`, `fake_host_test.rs` and
    `host_conformance_test.rs` identical; the forwards' bodies identical; the doc comments of the constant, the rule and
    `foreground_commands`, and the module doc of `claude_code_rule_test.rs`, differ.
  - Ran on the merged tree: `cargo build --workspace --tests`, `cargo clippy --workspace --all-targets -- -D warnings`
    (exit 0), the CI test step (2523 passed, 0 failed, 36 ignored), `docs_cli_test`, `docs_pane_control_test`,
    `cargo test -p holler-proto`, `scripts/lint.sh` (exit 0), `scripts/changelog-check.sh` (ok), `cargo machete`
    (clean), `rustfmt --check` on the four edited `.rs` files.

## T, verify GREEN after the rework: 2026-10-11T22:20-06:00 (MDT)

- **Decided:**
  - GREEN with no blocking issue on `7260b5e` plus two test-only edits. S's REWORK items 1 to 3 are fixed as S asked.
  - Closed S Advisory 4's two main gaps, as T's work: `hermetic_test::a_dialog_and_an_unknown_status_are_never_an_activity`
    (`harness-dialog-open` and `unavailable`), and the `session-holds-permission` code in
    `lifecycle::switch_refuses_a_busy_pane_and_one_waiting_on_a_permission`. The first is its own test because the existing
    table returns `Activity` and these return errors.
  - The lower-value Advisory 4 gaps (`pane get`/`pane watch` over a Claude record, `interrupt --pane` output,
    `server-unsupported`) are left as advisories. They do not gate.
- **Assumed:**
  - `cargo fmt --check`'s workspace-wide failure is not a gate: neither `ci.yml` nor `lint.sh` runs fmt, and the drift is
    in files the rework did not touch.
- **Hedged:**
  - The roster line is still for S to judge: a Claude Code pane has no roster row until #850.
- **Evidence:**
  - CI test step: 2524 passed, 0 failed, 36 ignored (165 binaries). Clippy `-D warnings`, `lint.sh`, `changelog-check.sh`,
    `cargo machete`, `docs_cli_test` + `docs_pane_control_test`, `wire_selftest`, `cargo test -p holler-proto` and
    `test-hooks.sh` all pass.
  - Mutant: an `if false` guard on the `dialog open` arm of `agents::activity` fails the new test; reverted.
  - `grep '"claude"' crates/*/src`: no second spelling outside `as_str`, the constant, `HARNESS_IDS`, a hub test and
    `holler-body`.
