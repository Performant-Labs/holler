# Decision journal: #835, `holler pane import` leaves panes Holler does not drive alone, with `--adopt-claude-code`

Run handoff directory `docs/handoffs/835` · branch `issue-835-implementation` · rigor in-session · uiSurface false
(D and U are N/A). Append-only; every phase adds its entry.

## A, up-front plan review: 2026-10-11T15:11-06:00 (MDT)

- **Decided:** BLOCK, with 2 `block` and 5 `warn` findings (`docs/handoffs/835/handoff-A.md`). The plan extends the
  right object: #650's skip path for a pane that is not an OpenCode pane (`harness-not-opencode`, `Action::Skipped`,
  `Outcome::left_alone`), and the verb's own argument checks.

  The two blocks:
  1. The contract amendment the brief puts first is undecided: no port, method, signature, default, fake or
     conformance case. Recommended: one read-only `HostPort` method beside `ps` that reads tmux's
     `#{pane_current_command}` for the session's panes, names only.
  2. The rule is undecided, and "a pane whose harness is not OpenCode", matched by command name, would leave every
     real OpenCode pane alone. The old launcher's `oc-holler` pane shows `bash`. The test kit's world runs no process
     in any tmux session. Recommended: a positive `claude` match is left alone; anything else is imported as today;
     an unreadable signal is decided explicitly.

  The warns:
  3. The finding level should be `skipped`, as `harness-not-opencode` is, not `info`.
  4. `usage` departs from `output::not_implemented(story)`, so the brief should record the reason.
  5. No story owns the adoption after #831, and `profile-exists` would refuse it after the cutover creates `fleet`.
  6. The plan names no blast radius: owned files (#649's `wiring.rs` if the method is required), files near the size
     gate, the test kit's mutant pins and the import's read allow-list.
  7. #832 should reuse this signal rather than add a second one.
- **Assumed:**
  - The fleet's panes have the shapes `scripts/pane-restart` gives them. An OpenCode pane is `oc-holler` under bash,
    typed into the pane's tmux shell (`:247`, `:301-306`, `:348`). A Claude Code pane is `claude`, typed the same way
    (`:153`, `:179`), and its argv[0] is `claude`: the script's own check is `(^|/)claude$` on `/proc/<pid>/cmdline`
    (`:86`). The live fleet was not looked at (out of bounds); the probe reproduced these shapes on a scratch tmux.
  - In the automated path there is no O, so this BLOCK stops the run (`needs-amendment`). Recovery is an amended brief
    and a fresh run.
- **Hedged:**
  - Findings 1 and 2 recommend a signal and a rule; the brief decides. What an unreadable signal does (a new
    `problem` kind, or the existing `observe-failed` warning) is the operator's call.
  - Finding 4 defers to the issue's explicit `usage` once the brief records the reason.
  - Finding 7 depends on how #832 reads "whose harness Holler cannot drive"; it may need no signal at all.
- **Evidence:**
  - Read: the brief, issues #835, #633 (epic, in full), #654 (with the operator's direction of 2026-10-11), #831,
    #832 and #828; `git log` and `git worktree list` (no other worktree's files were opened).
  - Code: `holler-pane/src/{import.rs, import/{assess,observe,findings,files}.rs, ports.rs, pane.rs, error.rs}`,
    `holler-cli/src/pane/{import.rs, import/writes.rs, wiring.rs}`, `holler-cli/src/output.rs`,
    `holler-adapter-host/src/{lib,tmux}.rs`, `holler-pane-testkit/src/host.rs`,
    `holler-pane-testkit/tests/host_conformance_test.rs`, `holler-cli/tests/pane_verbs/import/{rig,seams}.rs`,
    `holler-cli/tests/pane_verbs/doctor/rig.rs`, `holler-proto/src/vocab.rs`, `scripts/pane-restart`,
    `scripts/lint.sh`; `wc -l` on every file the change would touch.
  - Docs: ADR-0021 §2, §9 (`:698`), §13 and the closing lists; ADR 0003 (`:63`); `cli-surface.txt` (`:166-167`);
    `docs/research/claude-code-pane-spike.md` (summary, §6, contract changes, decisions);
    `docs/research/herdr-api-spike.md` (operations table, §9).
  - Probe: a scratch tmux 3.7c server on a private `-L` socket, killed afterwards, with its socket file removed. It
    printed `#{pane_current_command}` for three panes:
    - a typed bash script whose child is `sleep 600`: `bash`;
    - `exec -a claude sleep 600`: `claude`;
    - `new-window -- env -- sleep 600`: `sleep` (its session's shell window: `bash`).

## A, up-front plan review (second pass, the amended brief): 2026-10-11T15:45-06:00 (MDT)

- **Decided:** BLOCK, with 1 `block` and 7 `warn` findings (`docs/handoffs/835/handoff-A.md`, which replaces pass 1's
  text; pass 1 is at `33416fa`).

  The amendment settles both of pass 1's blocks (the port contract in §1, the rule in §2) and settles or records all
  five of its warns. The plan still extends the right objects.

  The new block is §1's default-armed shape together with §6's "do not touch `wiring.rs`". #649's branch (a-dup PASS at
  15:27 MDT, not merged) wires the production host as `BindingHost` over `WiredHost` over `TmuxHost`, and both decorators
  forward only the four existing methods. So the real import's read would answer the default's refusal for every pane:
  every pane `command-unreadable`, every real run `import-blocked`. No compile error and no test catches it.
  - Recommended: a required method (decision 12's "compile error at every call site").
  - Acceptable: default-armed, with forwards in both decorators and a `WiredHost` conformance pin.
  - Either way: sequence the run after #649 merges.

  This retracts pass 1's finding 6, which recommended default-armed without reading #649's branch.

  The warns:
  - §13 has no finding table, and `no-tmux-session` is not a code;
  - `fake_tmux_test.rs` must change, and #832 (PR #836) edits the same files;
  - the two codes should name the condition, not the action or "command";
  - the rule's limits are unrecorded, and pass 1 misdescribed the old launcher's test as argv[0]-only;
  - a lost chain and a skipped read still record a pane on a guess;
  - the constant and its predicate need one public home (`pane.rs`);
  - the conformance case must hold on a real tmux (a shell window is listed; `env` may still show right after `run`).
- **Assumed:**
  - #649 merges with the decorators as they are on its branch at `9cafe0d`. If its wiring changes before merging, the
    finding still applies to whatever decorates the host.
  - tmux on Linux names a pane's foreground process by its process group leader's argv[0] (`/proc/<pgrp>/cmdline`),
    cut at the first space; a name that starts with `/` is reduced to its last component. Pass 1's probe agrees:
    `exec -a claude` changes argv[0] only, and tmux reported `claude`.
  - The fleet's Claude Code processes have argv[0] `claude`, as the old launcher types the command (`pane-restart:153`, `:179`). The
    live fleet was not looked at (out of bounds); the dry run is the check, and finding 5 asks for that to be recorded.
- **Hedged:**
  - Finding 1 offers A or B. Both are existing patterns: decision 12 for a required change, #645 and #832 for
    default-armed methods. The block is the missing forward, not the choice.
  - Findings 4 and 8 are guidance. The ADR's spelling and T's case design are the owners' decisions.
- **Evidence:**
  - Read: the amended brief (sha256 `18e5debe773dff24...`), pass 1's handoff and this journal; issues #831 and #832;
    `git fetch` at 15:33 MDT (`origin/main` still `4366fb8`); `gh pr list` (#832 is PR #836, open; #649 has no PR yet).
  - Read from other branches through git only, without opening their worktrees:
    - #649's `crates/holler-cli/src/pane/wiring.rs`, `wiring/bind.rs`, `tests/pane_integration/bind.rs` and
      `docs/handoffs/649/handoff-A.md`;
    - #832's diff to `ports.rs`, `wiring.rs` and ADR-0021.
  - Code on this branch:
    - `holler-pane/src/{ports.rs, pane.rs, import.rs, import/{assess,observe,findings}.rs}`;
    - `holler-adapter-host/src/{lib,tmux}.rs` and `tests/{fake_tmux_test.rs, real_tmux_test.rs}`;
    - `holler-pane-testkit/src/{host.rs, conformance/host.rs}` and `tests/host_conformance_test.rs`;
    - `holler-cli/src/pane/{import.rs, wiring.rs}` and `holler-cli/src/output.rs`;
    - `holler-cli/tests/pane_verbs/{import.rs, import/{rig,seams}.rs, doctor/rig.rs}`;
    - `holler-pane/tests/import_test.rs`, `holler-proto/src/vocab.rs` and `scripts/pane-restart`.
  - Docs: ADR-0021 §2, §9 (`:698`, `:755`), §13 and the decisions list (`:976-978`); ADR 0003 (`:63`);
    `cli-surface.txt`; `docs/research/claude-code-pane-spike.md` (§6, the contract changes, the decisions).
  - Sizes: `wc -l` on every file the change would touch.

## A, up-front plan review (third pass, rounds 2 and 3, on the merged #649): 2026-10-11T17:59:50-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 6 `warn` findings (`docs/handoffs/835/handoff-A.md`, which replaces pass 2's text;
  pass 2 is at `cc3d366`).
  - Round 2 settles pass 2's block: `foreground_commands` is required, the `wiring.rs` rule is lifted for exactly the
    forwards, and the story runs after #649. The worktree is built on it (base `2d217dd`, #649 merged at 17:35 MDT).
  - Round 2 settles or records all seven of pass 2's warns.
  - The plan extends the right objects, and no contract decision is left open.

  The warns:
  1. Round 3 was overtaken. It was written at 15:54 MDT, and #649's D33 (merged at 17:35) already forwards `observe_pane`
     and `observe_panes` in both Herdr wrappers and pins them (`tests/pane_integration/observe.rs`, `bind.rs`).
     - R3-2 is already met: T adds no copies.
     - R3-3's test passes before any code: T writes it as a regression pin in `observe.rs`.
     - "Required" is pinned with a `compile_fail,E0046` doctest.
  2. ADR-0021 §14's `observe_pane` bullet ("has a default ... keeps compiling") becomes false and is not in the brief. Amend
     it in place, keep §13's "Decided (#835)" paragraph to the import, and update `ports.rs`'s stale list of amendments.
  3. Open PR #840 makes `unmanaged::is_unmanaged` "the one rule of 'a pane Holler does not drive'" and says #835 calls it.
     - The import cannot use it: it is true of every import candidate.
     - R2-7's process rule stays in `pane.rs`. If #840 merges first, F corrects #840's two sentences that name #835.
     - Name #840 among the coexistence notes.
  4. R2-6 reverses `import/join.rs:392-404`, a merged #650 test. Two corners are left open: a failed `ps` (read names
     only after `ps` answers) and a read's `pane-not-found` (`tmux-session-missing`).
  5. The implementation lists are short: `pane_integration/bind.rs`'s `LinkedHost` and `ports_test.rs`'s `TestHerdr`.
     R3-2's "no test double relies on a default" holds for forwarding wrappers only.
  6. `WiredHost` has no forwarding pin. One host-suite run over it, plus one line in `wired.rs`; `BindingHost`'s existing
     run covers its forward.
- **Assumed:**
  - #840 keeps its text as it is at `8e39fcc`. If it merges first, the correction in warn 3 is F's; if it changes, the warn
    still applies to whatever it says about #835.
  - The pipeline's T reads handoff-A. With no O in the automated path, the warns reach T and F through "Notes for T and F".
  - `origin/main`'s one commit after the base (`7fc00f1`, a #655 journal file) changes nothing #835 reads or edits.
- **Hedged:**
  - Warn 1 suggests a `compile_fail` doctest as the RED for "required". The test design is T's.
  - Accepting a test that passes before F starts as R3-3's evidence is T's call and S's. This review says only that the
    behaviour already exists.
  - Warn 3's correction of #840's wording depends on the merge order. The operator can settle it more cheaply on #840.
  - R2-5's limit (`node` running Claude Code is not recognised) is unverified on the real fleet, which is out of bounds. The
    first real import's dry run is the check.
- **Evidence:**
  - Read: the brief (sha256 `bac2232a86462e4f...`), passes 1 and 2 and this journal; `gh issue view 835` (the body is the
    same as the brief's issue section); `gh pr list` (#840 open; #649 merged as PR #843).
  - `git fetch` at 17:44 and 17:57 MDT; `git log` and `git diff --stat` for #649 (`2d217dd`), #832 (`652841b`), #838 and
    `7fc00f1`.
  - Through git only, with no other worktree opened: #840's diff, and the other branches' changed files (#647, #648, #652,
    #655, #828 and #842; none overlaps #835's code).
  - Code: `holler-pane/src/{ports.rs, pane.rs, import.rs, import/{assess,observe,findings}.rs, findings.rs}`;
    `holler-cli/src/pane/{wiring.rs, wiring/bind.rs, import.rs}`, `output.rs` and `prompt_target.rs`;
    `holler-adapter-host/src/{lib,tmux}.rs`; `holler-pane-testkit/src/{host.rs, conformance/host.rs}`.
  - Tests: `holler-pane-testkit/tests/host_conformance_test.rs`; `holler-cli/tests/pane_integration/{bind,observe,wired}.rs`;
    `holler-cli/tests/pane_verbs/{import.rs, import/{rig,join}.rs, unmanaged.rs}`;
    `holler-pane/tests/{import_test,ports_test}.rs`; `holler-adapter-host/tests/fake_tmux_test.rs`.
  - Every implementation of `HostPort` and `HerdrPort` in the workspace, by grep, checked for `observe_pane`.
  - Docs: ADR-0021 §2, §9, §13, §14 and the decisions list; `docs/handoffs/649-brief.md` (D33) and 649's journal;
    `docs/research/claude-code-pane-spike.md` (§6, the contract changes, the decisions); `scripts/pane-restart:74-88`;
    `.aftersight/pipeline.config.json` (`cargo test --workspace` runs doctests); the T rules in `tester.md`.
  - Sizes: `wc -l` on every file the change would touch; all are under 900 (`launch/rig.rs` about 844 with #840 and #835).

## T, author tests (RED): 2026-10-11T18:20-06:00 (MDT)

- **Decided:** **RED valid** (`docs/handoffs/835/handoff-T-red.md`). The tests cover the issue, the amendment and rounds 2 and 3, following A's pass-3 notes for T.
  - **The import's behaviour:** seven tests in a new `pane_verbs/import/harness.rs`:
    - a pane running `claude` is left alone; a pane running `bash` and one running nothing are imported as OpenCode;
    - the dry run shows the same finding and writes nothing;
    - a pane left alone has one finding and blocks nothing;
    - the switch is `usage` naming `not_implemented_message(831)`, with no port call;
    - a failed `ps`, a failed read and a lost chain are each `harness-unreadable`, and a read that answers `pane-not-found` is `tmux-session-missing`.
  - **#650's `a_failed_ps_is_observe_failed_and_the_pane_imports`** is removed from `join.rs` and replaced with its R2-6 opposite (A's W4).
  - **The code table:** `import_test.rs` goes to 12 codes, with the two levels.
  - **The rule:** a unit test in `claude_code_rule_test.rs`.
  - **"Required":** two `compile_fail,E0046` doctests in `ports.rs`'s trait docs (A's W1).
  - **The conformance suite:** host case 10 (R2-8), with two `Mutant` breaks.
  - **The fake and the adapter:** `FakeHost` tests; the adapter's `METHODS` (818 to 820 lines) and a new `foreground_test.rs`.
  - **The forwards:** a `WiredHost` conformance run and a line in `wired.rs` (A's W6).
  - **The docs pins:** the ADRs and the `cli-surface.txt` line.
  - **R3-3:** written as two regression pins in `pane_integration/observe.rs`, as A's W1 says. They pass before F starts. R3-2's tests are not copied.
- **Decided:** **the names F must use**, which the brief left open:
  - `HostOp::ForegroundCommands`;
  - `holler_pane::pane::CLAUDE_CODE_COMMAND` and `is_claude_code_command(&str) -> bool`.
- **Decided:** **T, not F, adds the forwards that live in test files.** These are `TestHerdr::observe_pane`, `TestHost`, the three `LinkedHost`s and the `Mutant`. A's note 6 assigned some of them to F, but F writes no test files.
- **Decided:** **T edits two files under `src/`, and both are test content.**
  - `holler-pane-testkit/src/conformance/host.rs`: case 10 of the conformance suite, which A's note 3 gives to T.
  - `holler-pane/src/ports.rs`: two doctests, which A's W1 gives to T. The diff adds doc-comment lines only.
- **Assumed:**
  - **The listing format.** The adapter's listing carries `#{pane_dead}` beside `#{pane_current_command}`. `a_malformed_listing_is_unavailable` relies on it, following A's "strict parser beside `parse_ps` that drops dead panes". The other adapter tests do not depend on how the format is spelled, because the fake tmux fills in whatever format it is sent.
  - **A real `env -- sleep 30` settles to `sleep`** within 2 s. A's pass-1 probe saw `sleep` for that shape.
- **Hedged:**
  - **The final RED is a compile RED.** The new required method is called from the test-kit lib, so nothing downstream builds until F adds it.
  - **The assertion-level RED is shown in two steps** in the handoff:
    1. the tests that use only today's API, run on unchanged code;
    2. a temporary bare-API stub, reverted. The six production files were restored and checked with `sha256sum`, and touched so that cargo rebuilds them.
  - **Under the stub,** 28 tests fail, each on an assertion and each in this story. Clippy with `-D warnings` is clean.
- **Evidence:**
  - Commands:
    - `cargo test -p holler-pane --test import_test`;
    - `cargo test -p holler-pane --doc ports`;
    - `cargo test -p holler-cli --test cli_surface_test`;
    - `cargo test -p holler-cli --test pane_verbs import::`;
    - `cargo test -p holler-cli --test pane_integration observe::`;
    - `cargo test --workspace --no-run`;
    - `bash scripts/lint.sh` (exit 0);
    - under the stub: `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` and `cargo clippy --workspace --all-targets -- -D warnings` (exit 0).
  - The target dir was seeded from `../0832-review-fixes/target` with `scripts/seed-target-dir.sh`.
