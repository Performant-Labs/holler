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

## F, implement against the RED: 2026-10-11T18:45-06:00 (MDT)

- **Decided:** implemented the brief (issue, amendment, rounds 2 and 3) as A's pass-3 notes and T's RED name it
  (`docs/handoffs/835/handoff-F.md`).
  - **The port:** `HostPort::foreground_commands` is required; `HerdrPort::observe_pane` lost its default. Every
    production implementation answers both: `TmuxHost`, `FakeHost`, `BindingHost`, `Wired<dyn HostPort>`, `Unwired`.
  - **The adapter:** `list-panes -s -t =NAME: -F "#{pane_dead} #{pane_current_command}"`, parsed as strictly as `ps`'s
    listing, errors through the one `refused()` path. No pid in the format: the method answers names only.
  - **The rule:** `CLAUDE_CODE_COMMAND` and `is_claude_code_command` in `pane.rs`, the one comparison with `claude`.
  - **The chain:** `ps`, then the names only once `ps` answered, then the server unless the session runs Claude Code. A
    pane whose session could not be read is not known to be Claude Code, so its server is still asked, as under #650.
  - **The findings:** `harness-claude-code` (info, the row `skipped`, one finding, no harness call) and
    `harness-unreadable` (problem). The latter replaces `observe-failed` for a failed `ps` and for a lost chain; it is
    not added beside it. The read's `pane-not-found` is `tmux-session-missing`.
  - **The switch:** `--adopt-claude-code` is `usage` at the top of `Input::read`, before any file or port, with
    `not_implemented_message(831)`.
  - **The docs:** ADR-0021 sections 2, 9, 13 ("Decided (#835)"), 14 (in place) and the two closing lists; ADR 0003's line;
    one CHANGELOG entry. Section 14's first bullet line and the CHANGELOG position were chosen so that the merge with
    `origin/main` (#840 edited section 14; #652 and #840 edited the CHANGELOG) does not overlap.
- **Assumed:**
  - tmux reports `#{pane_current_command}` with no space in the name (it cuts the name at the first space), so a line is
    `<0|1> <name>`. The real-tmux conformance run (case 10, tmux 3.7c, a private socket) agrees for `sleep`.
  - Every caller of `foreground_commands` is the import; nothing else reads it yet (#831 and #832 may).
- **Hedged:**
  - **T's `spec_names()` in `import/harness.rs` reads the key `name`; a profile spec's key is `pane`.** Two tests fail
    there after all their other assertions pass. Flagged for T, not edited. A temporary local fix (reverted, checked with
    `sha256sum -c`) showed all seven `import::harness` tests pass.
  - **This branch is behind `origin/main`** (`bda1069`: #840 and #652 merged after the base `2d217dd`). #840's two
    sentences that name #835 as a caller of `unmanaged::is_unmanaged`, and #652's guide, are not on this branch: F could
    not correct or extend them (A's W3; brief section 6). Listed in the handoff's Known issues for the merge.
  - Calling the server of a pane whose session could not be read is a reading of A's note 4 ("health and the session list
    only for a pane that is not `claude`"); S may prefer no server call for it. The pane is blocked either way.
- **Evidence:**
  - Commands: `cargo build --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings` (exit 0);
    `cargo test -p holler-pane --test import_test --test claude_code_rule_test --test ports_test`;
    `cargo test -p holler-pane --doc ports`; `cargo test -p holler-pane-testkit`; `cargo test -p holler-adapter-host`;
    `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored the_tmux_host_passes_the_host_conformance_suite`
    (a private scratch tmux; F's spot check of the parser on real output, not T's Tier 2);
    `cargo test -p holler-cli --test pane_verbs import::`;
    `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load`;
    `bash scripts/lint.sh` (exit 0); `bash scripts/changelog-check.sh` (ok); `rustfmt --check` on every changed source
    file (clean but `wiring.rs:76`, a long line from #832 that F left alone); `git diff --check` (clean).
  - Read: the brief, handoff-A (pass 3), handoff-T-red, this journal; every file listed in the handoff; `git show bda1069`
    and `git diff 2d217dd origin/main` for the overlap.
  - `docs/handoffs/835/evidence.md`: six facts in unchanged code.

## T, Phase 7 (verify / GREEN), 2026-10-11 18:56 MDT

- **Decided:**
  - I repaired my own Phase 4 bug in `pane_verbs/import/harness.rs`: `spec_names()` reads `spec["pane"]` (it read
    `name`). `import::harness` passes 7 of 7. The branch's workspace suite exits 0 (2377 passed).
  - I verified the branch against current `origin/main`, not only its base, because a PR's CI runs on the merge ref. A
    scratch merge commit, in a throwaway worktree with its own target dir (since removed), fails one test:
    `docs_pane_control_test::import_findings_are_named_in_section_3_or_10`. #652's guide names neither new code.
  - Verdict BLOCK, routed to F. The fix is the merge plus edits to `docs/pane-control.md`, which F owns, and no test
    change. In the same merge, F corrects #840's #835 claims (`unmanaged.rs:66`, `:84`; ADR-0021 line 972).
- **Assumed:**
  - The PR's CI runs `cargo test --workspace` on the merge with main, as GitHub's `pull_request` event does. So the
    failure on the merge is a real CI failure, not a hypothetical one.
- **Hedged:**
  - Nothing else fails on the merge (154 groups, 2396 passed). I did not run clippy on the merge, because main's three
    commits add no port implementor and no lint-relevant code beside these files.
- **Evidence:**
  - Commands:
    - `cargo test -p holler-cli --test pane_verbs import::` (46 passed)
    - the CI workspace test (exit 0)
    - `bash scripts/lint.sh` (exit 0) and `bash scripts/changelog-check.sh` (ok)
    - clippy `-D warnings` (exit 0)
    - `docs_cli_test` (3 passed) and `wire_selftest` (3 passed)
    - `cargo machete` (clean)
    - `real_tmux_test -- --ignored` (9 passed, tmux 3.7c)
    - `git merge-tree --write-tree HEAD origin/main` (clean), then the workspace suite on the merge (1 failed)
  - Mutations, each restored with `sha256sum -c` OK:
    - the rule always `false`: 4 tests fail
    - dead panes kept: 1 test fails
  - `evidence.md`: one T entry (`ProfileSpec.pane`).

## A, up-front plan review (fourth pass, re-entry after T-green's BLOCK): 2026-10-11T19:16-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 4 `warn` findings (`docs/handoffs/835/handoff-A.md`, which replaces pass 3's text;
  pass 3 is at `49e4508`).
  - **Why A ran again.** T-green returned BLOCK and F had reported `archChanged`, so the script sent the run back to A
    (`nextPhase`, case 7). The brief is unchanged since pass 3 (sha256 `bac2232a86462e4f...`), so this pass did two things:
    - checked F's cycle 1 for drift;
    - reviewed the rework.
  - **Cycle 1 has no drift.** F extended every object the plan named, in its layer. F's design decision 1 (the server of
    a pane whose tmux read failed is still asked) fits pass 3's note 4.
  - **The rework needs no contract decision.** Brief section 6 and R2-3 cover it.

  The warns:
  1. **The merge comes first, and it must be a real two-parent merge of `origin/main`.**
     - T-red makes it with `--no-commit`, and the script commits. Otherwise F makes it before its first edit.
     - Never a rebase, a squash or a copy.
  2. **#840 says in four places that #835 calls its registry rule.** T-green listed three. The fourth is the doc comment
     of #840's test `the_unmanaged_predicate_is_public_and_one_rule`.
  3. **This story's own text says #832 reuses the predicate.** The places are `pane.rs:146`, ADR-0021 section 13
     (`:966-967`) and `claude_code_rule_test.rs:3-4`. #832 closed without it.
  4. **The guide is stale in five places beyond its findings table:**
     - section 1;
     - section 3's synopsis, its `observe-failed` row and its exit codes;
     - section 10's fact and its step 1;
     - section 12.

     `ImportFinding::ObserveFailed`'s doc is stale too.
- **Assumed:**
  - `origin/main` stays at `bda1069` until the merge. If it moves, the merge takes the new tip and the same notes apply.
  - The script's phase commit (`git add -A`, then `git commit -m ... --allow-empty`) turns a merge left by
    `git merge --no-commit` into a two-parent commit, as git does while `MERGE_HEAD` exists. This was not run here,
    because the role is read-only.
  - T-red and F read this handoff's notes, as they read pass 3's.
- **Hedged:**
  - **W1 departs from T-green's routing only in who goes first.** Either order reaches the same tree. T-red goes first
    so that its RED is on the branch, which avoids a T-red retry loop (`T_RED_BLOCK_CAP` is 10).
  - **W4's flag pin is optional,** and the guide's wording is F's.
  - **"#832 closed without a second signal" rests on a grep** of every `src` for process or agent detection. #832's
    diff was not read again.
- **Evidence:**
  - **The branch and its state:**
    - `git log`, `git reflog` and `git diff --stat` of the branch (`1aeb42c..25ff495`);
    - `git fetch` at 18:57 and 19:05 MDT (`origin/main` is `bda1069`);
    - `gh issue view 832` (closed at 15:35 MDT) and `gh issue view 831` (open);
    - `gh pr list` (#852 and #673 are open, and neither shares a file with #835).
  - **The pipeline script:**
    - `nextPhase` case 7, `shouldEscalate`, `T_RED_BLOCK_CAP`, `decideArchReReview` and `buildPhaseCommitTaskPrompt`;
    - `feature.md` (`archChanged`), and what `tester.md` and `feature.md` say about commits.
  - **F's cycle 1:** `git diff 36b0de8 9bd101b` of every production file; handoff-F, handoff-T-red, handoff-T-green,
    `evidence.md` and this journal.
  - **`main`:**
    - `git show bda1069` (`unmanaged.rs`, ADR-0021, `mod.rs`, `CHANGELOG.md`) and `git show 0e62b51 --stat`;
    - `docs/pane-control.md` (sections 1, 3, 10, 11 and 12);
    - `docs_pane_control_test.rs` (acceptance 1, 8, 9, 14 and 17).
  - **The merge:** `git merge-tree --write-tree HEAD origin/main` gives tree `2ff3c92`, with no conflict. In that tree:
    - every `835` mention outside the handoffs;
    - `wc -l` of every touched `.rs` file (the largest is `launch/rig.rs`, 839 lines, under 900);
    - `unmanaged.rs:63-84` and `tests/pane_verbs/unmanaged.rs:467-486`.
  - **The commit hooks:**
    - `git diff HEAD 2ff3c92 | grep '^+' | gitleaks stdin` (no leaks);
    - `.githooks/{pre-commit,commit-msg,prepare-commit-msg}`;
    - `scripts/lint.sh` (the size gate covers `crates/**/*.rs` only).
  - **grep:**
    - `"claude"` in every non-test `.rs` file: only `pane.rs:147` in `holler-pane`;
    - `ObserveFailed`: now raised for the harness reads only.

## T, Phase 4 (AUTHOR / RED), cycle 2: 2026-10-11T19:17-06:00 (MDT)

- **Decided:** RED is valid (`docs/handoffs/835/handoff-T-red.md`, which replaces cycle 1's at `36b0de8`).
  - **The merge (A's W1).** I ran `git merge --no-ff --no-commit origin/main` (`bda1069`). It was clean, and `MERGE_HEAD`
    is left for the script's phase commit. There was no rebase or squash.
  - **The RED is two docs-conformance tests:**
    - #652's `import_findings_are_named_in_section_3_or_10` (unchanged). It wants `harness-claude-code` and
      `harness-unreadable` in section 3 or 10.
    - The new `section_3_names_every_flag_of_pane_import`. It wants `--adopt-claude-code` in section 3's `pane import`
      subsection.
  - **I fixed two test doc comments:** `pane_verbs/unmanaged.rs` (W2: #831 only) and `claude_code_rule_test.rs`
    (W3: "#831 can reuse").
- **Assumed:**
  - The script's phase commit turns the pending merge into a two-parent commit (A's assumption, not run here).
  - F's cycle is text only, so no Rust test beyond these two needs to change.
- **Hedged:**
  - **I narrowed the flag pin to `pane import`.** Its generic form, over every verb, fails on 25 flags of other verbs
    that #652's guide summarises rather than lists. That is #652's scope, so it is an operator follow-up and not part
    of this story.
  - **The guide's prose is not pinned.** This covers sections 1, 10 and 12 and the `observe-failed` row. S reads it,
    as A said.
- **Evidence:**
  - `cargo test -p holler-cli --test docs_pane_control_test`: 15 passed, 2 failed, both on their assertion.
  - CI's workspace step on the merged tree: 154 groups, 2396 passed, 2 failed (the same two), 32 ignored.
  - Clippy `-D warnings` clean; `lint.sh` exit 0; `changelog-check` ok; `docs_cli_test` 3 passed;
    `git diff --check -- crates` clean.

## F, implement against the RED (cycle 2, text only): 2026-10-11T19:31-06:00 (MDT)

- **Decided:** the rework A's pass 4 and T-red's cycle 2 name, in place, with no code change
  (`docs/handoffs/835/handoff-F.md`, which replaces cycle 1's at `9bd101b`). `archChanged: false`.
  - **The merge** was already T-red's (`526bbd3`, two parents). `origin/main` is still `bda1069` at 19:30 MDT, so F
    merged nothing.
  - **The guide (W4):**
    - section 1 names the second exclusion;
    - section 3 gains `[--adopt-claude-code]` and what it does, the rows `harness-claude-code` (info) and
      `harness-unreadable` (problem), the narrowed `observe-failed` row, and the four `usage` causes;
    - section 10 restates the fact with the rule and its limits, links ADR-0021 section 13 as the contract, adds step 1's
      dry-run check, and names `harness-claude-code` in the closing paragraph;
    - section 12 replaces the #835 line with the adoption after #831.
  - **The records:** `ObserveFailed`'s doc (W4); #840's sentences in `unmanaged.rs` and ADR-0021 section 14 (W2); the
    #832 reuse claims in `pane.rs` and ADR-0021 section 13 (W3).
  - **Step 1 also says the real run runs while the Claude Code sessions run,** since step 3 observes the panes again.
    This is one sentence beyond W4's list, inside the step W4 names.
- **Assumed:**
  - The guide has no size gate. `scripts/lint.sh` checks `crates/**/*.rs` only. It is now 817 lines, past #652's soft
    "about 800".
  - #654's option (a) stays the operator's fallback for a Claude Code pane the rule misses, as the issue's Pipeline
    section says.
- **Hedged:**
  - **The prose of sections 1, 10 and 12 is pinned by no test** (T-red: S reads it). Only the two codes in sections 3
    or 10 and the flag in section 3's subsection are pinned.
  - **The step-1 sentence about the real run** goes past W4's list. S may judge it scope; it is true of the code (the
    real run observes again).
- **Evidence:**
  - Commands:
    - `cargo test -p holler-cli --test docs_pane_control_test`: before, 15 passed and 2 failed (T-red's two); after,
      17 passed;
    - `docs_cli_test`: 3 passed;
    - CI's workspace step: exit 0, 154 groups, 2398 passed, 0 failed, 32 ignored;
    - clippy `-D warnings`: exit 0;
    - `cargo doc --no-deps -p holler-pane`: no warning;
    - `lint.sh`: exit 0; `changelog-check`: ok; `git diff --check`: clean;
    - `git fetch` and `git log HEAD..origin/main`: empty.
  - Read: the brief, handoff-A pass 4, handoff-T-red cycle 2, handoff-T-green, handoff-F cycle 1, this journal,
    `evidence.md`, `docs/pane-control.md`, `docs_pane_control_test.rs`, `docs_cli_test.rs`,
    `holler-pane/src/import/{observe,assess,findings}.rs`, `unmanaged.rs`, `pane.rs`, `reconcile.rs:362-372`,
    `holler-cli/src/pane/import.rs`, ADR-0021 sections 13 and 14 and the closing lists, and `gh issue view 654`.
  - `evidence.md`: three facts in unchanged code (section "F, cycle 2").

## T (Phase 7, GREEN), cycle 2: 2026-10-11T19:40-06:00 (MDT)

- **Decided:** PASS (`docs/handoffs/835/handoff-T-green.md`, which replaces cycle 1's BLOCK at `25ff495`).
  - CI's test step on `4604693` (the merge with `origin/main` `bda1069` plus F's text): 154 groups, 2398 passed,
    0 failed. Both cycle-2 RED tests now pass, and nothing else changed.
  - Clippy, `lint.sh`, `changelog-check.sh`, `docs_cli_test`, `wire_selftest`, `cargo machete`, rustdoc with
    `-D warnings` and `git diff --check` are clean. F's reported results match.
  - Cycle 1's blocking issues 1 (the guide) and 2 (#840's sentences, the #832 reuse claims) are resolved.
  - No test was repaired. F listed none.
- **Assumed:** `origin/main` is still `bda1069` at 19:40 MDT (`git fetch`), so the branch tree is the PR's merge ref.
- **Hedged:** the guide is 817 lines, past #652's soft 800, and no check enforces it. Left to S.
- **Evidence:**
  - a temporary rename of `harness-unreadable` and `--adopt-claude-code` in the guide made both doc tests fail, naming
    exactly that text; the file was restored and checked with `sha256sum -c`;
  - the guide's claims were read against `import/observe.rs:177-187` (the only `ObserveFailed` producers) and
    `pane/import.rs` `Input::read` (the four `usage` causes);
  - `grep` finds no "#831 and #832" or "#831 and #835" left in `crates` or ADR-0021.

## A, anti-duplication gate: 2026-10-11T19:48-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 1 `warn` (`docs/handoffs/835/handoff-A-dup.md`).
  - **The scope.** I reviewed the whole feature diff, `bda1069..b488a33`: 37 files outside `docs/handoffs`. This is the
    run's first A-dup, and cycle 1's code never reached this gate.
  - **No parallel path.** F extended every object the map named, in its layer:
    - the read: `HostPort` beside `ps`, and in the adapter `ps`'s shape, its listing pattern, a parser as strict as
      `parse_ps`, and `refused()`;
    - the rule: one constant and one predicate in `pane.rs`;
    - the chain: one more read on the same thread, through the one `tmux_finding` mapping and `failed_message`;
    - `Outcome::claude_code`, through `left_alone`;
    - the forwards in `BindingHost` and `Wired<dyn HostPort>`;
    - `FakeHost`: one `processes` list and one `each_process` helper;
    - the switch: in `Input::read`, with `usage()` and `not_implemented_message`.
  - **No drift from the rework.** F's cycle 2 changed only comment lines in the three Rust files it touched.
  - **The merge.** `526bbd3` is a clean merge, plus T-red's three test edits and the handoffs.
  - **The warn.** `ImportRig::run_with_host` copies `run_with_store`, and `text_row` is a twin of `text_line`. Both are
    test code, and folding them is optional.
- **Assumed:**
  - `origin/main` is still `bda1069` (`git fetch` at 19:42 MDT), so the diff base is the PR's base.
  - Brief R3-3 justifies, in writing, how much the two R3-3 `pane list` tests overlap `pane_verbs/unmanaged.rs`'s list
    tests. So the overlap is reviewed, not drift.
- **Hedged:**
  - **The three `list-panes` builders in `tmux.rs`** differ only in their format. They follow the module's "one pure
    function per command line" and the map's shape, so they are not flagged.
  - **The docs' "each process" wording** is narrower in the contract: the foreground process of each live pane. That is a
    question of what the docs say, so it is left to S.
  - **The "refused until #831" prose** spans the ADR, the guide, the help text and the CHANGELOG, and no test pins it. This
    is noted for #831's owner.
- **Evidence:**
  - **Git:**
    - `git fetch`;
    - `git diff --stat` and the full `git diff bda1069 HEAD` of every non-handoff file;
    - `git merge-tree --write-tree 529467f bda1069`, compared with `526bbd3`;
    - `git diff 526bbd3 4604693 -- crates`, filtered for lines that are not comments: none.
  - **Read in full:**
    - `import/{observe,assess}.rs`, `tmux.rs` and `testkit/src/host.rs`;
    - `pane_verbs/import/harness.rs` and `pane/import.rs` (lines 40-255);
    - `pane_integration/observe.rs` and `verb_harness/mod.rs`.
  - **Read in part:** `lib.rs` (330-480), `bind.rs` (520-600) and the conformance `Mutant`.
  - **grep:**
    - `"claude"` in every `src` (one command-name comparison);
    - every `impl HostPort for` (11 implementations);
    - every caller of `foreground_commands` (the import only, in production);
    - basename, poll and `text_line` helpers;
    - stale "default" wording about `observe_pane` in ADR-0021 (none);
    - stale story cross-references (none);
    - personal names in the diff (none).
  - **Sizes:** `wc -l` on every touched file. All are under 900.

## S, spec audit: 2026-10-11T20:05-06:00 (MDT)

- **Decided:** REWORK (production, docs only), one item (`docs/handoffs/835/handoff-S.md`).
  - **All 20 acceptance criteria have proving evidence.** The criteria come from the issue, the amendment and rounds
    2 and 3.
    - Every test was RED before F, failed under a mutation, or is a regression pin named as one (R3-3).
    - The adoption after #831 is out of scope by brief section 5.
  - **The item: the user-facing text misstates what the import reads.** It says the import reads "each process" of
    the pane's tmux session, and that it leaves alone "a pane with a process named exactly `claude`".
    - **Where:** the help text (`pane/import.rs:67-75`), the guide (sections 1, 3 and 10) and the CHANGELOG.
    - **What the brief says** (section 1; R2-5's preamble): the read is the foreground process of each tmux pane of
      the session. The ADR, `ports.rs` and `pane.rs` say this exactly.
    - **What the text gets wrong:** under the text's own model, a `claude` behind another foreground process would be
      found. That covers a `claude` started as a child of a wrapper script, and a suspended or backgrounded one. The
      import does not find it, and the text's limits leave it out.
    - A-dup (note 1) left this call to S.
  - **The departures from the brief's literal text are recorded, not silent:**
    - `observe_pane`'s change is recorded in sections 14 and 2 and in decision 23 (A pass 3, W2);
    - R3-2's second bullet is read as applying to wrappers (A pass 3, W5);
    - F's design decision 1;
    - R3-3's pins pass before F.
  - **Quality:**
    - No `unwrap`, `expect` or `panic` in any `src`.
    - The two `#![allow]` lines carry `// #835`.
    - The largest touched `.rs` file is 839 lines.
    - No protocol or golden change.
    - The privacy grep is clean.
    - The commits have the repo hook's trailer.
- **Assumed:**
  - T's and F's recorded runs stand. I re-ran no build or test, as the role says. My checks were read-only: `git diff`,
    `git log`, `grep`, `wc -l`, `rustfmt --check`, `gh issue view 835` and `gh issue view 633`.
  - The script commits this handoff and this entry at the phase commit.
  - The rework adds no lines to the guide (817) and needs no test change, so T-green re-verifies only.
- **Hedged:**
  - **Why REWORK and not PASS with an advisory.** No behaviour is wrong, and the dry-run check in the guide's
    section 10, step 1 catches a missed pane either way. But the text is the standing operator documentation for the
    cutover. It also blurs the distinction R2-5 asked to record. The precedent is #652's S: its hold also required
    one wrong sentence about the import to be corrected.
  - **R3-2's second bullet cannot hold as written.** The test-only Herdr wrappers still use `observe_panes`'s
    default. This is harmless, so it is an advisory, not an `ADVISORY-HOLD`.
- **Evidence:**
  - Live:
    - `gh issue view 835`: the body is identical to the brief's issue section;
    - `gh issue view 633`: the contract block's `HostPort` line lacks the new method (operator item);
    - `gh pr list --head issue-835-implementation`: no PR yet;
    - `git fetch`: `origin/main` is `bda1069`, an ancestor of `HEAD`.
  - Read: the brief; every handoff, with cycle 1's T-red and T-green from git (`36b0de8`, `25ff495`) and A pass 3
    (`49e4508`); this journal and `evidence.md`.
  - Diffs: the full `git diff origin/main...HEAD` of every non-handoff file.
  - Source checked:
    - `import/{observe,assess,findings}.rs`, `pane.rs`, `ports.rs` and `unmanaged.rs`;
    - the adapter's `lib.rs` and `tmux.rs`;
    - `pane/import.rs`, `wiring.rs` and `wiring/bind.rs`;
    - `reconcile.rs:362-372`;
    - `findings::{quoted, embedded}`;
    - every `impl HostPort` and `impl HerdrPort`, and every `observe_pane(s)` override;
    - `pane_verbs/doctor.rs:131` and `import/join.rs:334`.
  - Greps:
    - `"claude"` in every `src`;
    - the producers of `ObserveFailed`;
    - `unwrap`, `expect`, `panic` and `#[allow` in the added lines;
    - private names, IPs, paths and secrets in the 2,882 added lines;
    - "each process" and its variants in docs and help.
  - Sizes: `wc -l` on all 37 touched files.

## F, implement S's REWORK 1 (cycle 3, text only), and merge `main`: 2026-10-11T20:32-06:00 (MDT)

- **Decided:** S's REWORK 1, in place, with no behaviour or test change (`docs/handoffs/835/handoff-F.md`, which
  replaces cycle 2's at `4604693`). `archChanged: false`.
  - **The six places S names** now say the import reads the command name of the foreground process of each tmux pane
    in the pane's tmux session, and leaves alone a pane whose tmux session has a foreground process named exactly
    `claude`:
    - the help (`pane/import.rs`);
    - the guide's sections 1, 3 (the `harness-claude-code` row) and 10 (the fact);
    - the CHANGELOG;
    - `read_tmux`'s doc.
  - **One limits sentence** is used in the help, the guide and the CHANGELOG: "tmux names only the foreground process,
    so a Claude Code pane is taken for OpenCode when its `claude` has exited (it shows its shell), is not in the
    foreground (a wrapper script runs it, or it is suspended) or was started under another name."
  - **One place beyond S's table:** the `holler-pane` import module doc's "the command names of the pane's tmux
    session" gets the same foreground wording, for the reason S gives for `observe.rs`.
  - **The guide gains no line from this story** (817 on the branch). Section 10's bullet stays at 12 lines: "It is a
    default for this import, not a proof of what a pane runs; step 1 says how to check it" becomes "It is a default,
    not a proof; step 1 checks it".
  - **ADR-0021, `ports.rs` and `pane.rs` are unchanged.** S judged them exact.
  - **Merged `origin/main` `d6ae539`** (#851, PR #854, merged 20:07 MDT, after S's 20:03 fetch), with
    `--no-ff --no-commit`. The phase commit concludes it as a two-parent commit, as T-red's `526bbd3` was.
    - Reasons: S's audit rests on the branch tree being the PR's merge result; brief R2-1 asks for the current `main`;
      A's W1 precedent.
    - Clean: `git merge-tree` showed no conflict, and the merge auto-merged ADR-0003 and the guide.
- **Assumed:**
  - The script's phase commit (`git add -A`, then `git commit`) concludes an in-progress merge, as it did for T-red's
    `526bbd3`.
  - #851 is reviewed and merged on `main`, so its files are not this story's to review. They drop out of
    `git diff origin/main...HEAD` once the merge is committed.
- **Hedged:**
  - **Section 3's row and the limits' "not in the foreground" case** are worded differently from S's suggestions. S
    left the wording to F, and both say "foreground" and name a `claude` outside it (handoff-F, "Deviations").
  - **The merge is outside S's REWORK list.** It is branch upkeep, not production code. If the pipeline wants it done by
    T or by the run's agent, reverting is one `git merge --abort` before the phase commit, then `git apply` of the
    rework patch.
  - **The guide is 821 lines after the merge.** The 4 extra lines are #851's ("confirm the binary", before section 10's
    fact). S's advisory 1 is unchanged.
- **Evidence:**
  - Commands on the merged tree:
    - CI's workspace step: exit 0, 155 groups, 2406 passed, 0 failed, 32 ignored;
    - `docs_pane_control_test`: 17 passed; `docs_cli_test`: 3 passed;
    - clippy `-D warnings`: exit 0;
    - `lint.sh`: exit 0; `changelog-check`: ok;
    - rustdoc `-D warnings -p holler-pane`: exit 0;
    - `git diff --check` on the worktree, on the staged merge and against `origin/main`: clean.
  - Before the merge, the workspace step had 2 failures in `interrupt_test` ("body is not connected", "reconnecting") at
    load average 23. That binary then passed 11 of 11 alone.
  - The patch round trip:
    - the merged worktree diff equals the saved rework patch, line for line;
    - the three files #851 does not touch are byte-identical to the saved copies;
    - `CHANGELOG.md` and the guide differ from their copies only by #851's lines.
  - `holler-cli` rustdoc `-D warnings` fails on `main` in `cli.rs`, `launch.rs`, `park.rs`, `reset.rs` and `switch.rs`,
    none in this diff. There are 0 diagnostics in `pane/import.rs`, and CI has no doc step.
  - Read: `handoff-S.md`, the brief, handoff-F cycle 2, ADR-0021 section 13, `ports.rs`, `pane.rs`,
    `tmux.rs:35` (`FOREGROUND_FORMAT`), `docs_pane_control_test.rs`, and `git diff HEAD...origin/main`.

## T-green, cycle 3 (2026-10-11, 20:42 MDT)

- **Decided:** PASS. S's REWORK 1 is met: every place says "foreground process" and names a `claude` outside the
  foreground among the limits. The suite is GREEN on the merged tree (`e6d5fa0`, with `origin/main` `d6ae539` as an
  ancestor).
- **Assumed:** a text-only cycle needs no new test and no repeat of the mutation spot check. No production code changed
  (`git show --remerge-diff e6d5fa0`: 5 files, prose and doc comments only).
- **Hedged:** ADR-0021:952 still says "the command names of the pane's tmux session". It is left as it is, because its
  next sentence defines them as the foreground process of each pane.
- **Evidence:**
  - CI's workspace step: exit 0, 155 groups, 2406 passed, 0 failed, 32 ignored.
  - `docs_pane_control_test` 17 passed, `docs_cli_test` 3 passed, `wire_selftest` 3 passed.
  - clippy `-D warnings` exit 0; `lint.sh` exit 0; `changelog-check` ok; `cargo machete` clean.
  - rustdoc `-D warnings -p holler-pane` exit 0; `git diff --check` clean.
  - The built `holler pane import --help` shows the new paragraph.
  - `tmux.rs:35` `FOREGROUND_FORMAT` matches the wording.

## A, anti-duplication gate (second pass, after S's REWORK): 2026-10-11T20:46-06:00 (MDT)

- **Decided:** PASS, with 0 `block` and 2 `warn` (`docs/handoffs/835/handoff-A-dup.md`, which replaces the first
  A-dup's at `e5e35ee`).
  - **The scope.** This cycle's diff: `git show --remerge-diff e6d5fa0` (F's edits beyond the automatic merge of
    `ccc53a0` and `d6ae539`), and `d2922a9` (handoffs only).
  - **No parallel path.** The cycle is prose and doc comments, edited in place, in five files already in this story's
    diff. No object of the map changed: the rule, the chain, the switch, the port, the adapter, the forwards, the fake
    and the tests are as the first A-dup passed them.
  - **The merge is clean.** The automatic merge tree `e77e7a0` plus F's remerge diff is `e6d5fa0`, and no file of `main`
    was edited. #851 and #835 share only `CHANGELOG.md`, ADR-0003 and the guide, in separate places.
  - **Warn 1 (new, records).** The help, the guide's section 10 and the CHANGELOG now name three limits of the rule
    (exited; not in the foreground: a wrapper script, a suspended job; another name). ADR-0021 section 13 and
    `is_claude_code_command`'s doc name two, and put "a wrapper" under another `argv[0]`. Optional: one clause in each.
  - **Warn 2 (carried, test-only).** `ImportRig::run_with_host` copies `run_with_store`; `text_row` twins `text_line`.
- **Assumed:**
  - `origin/main` is still `d6ae539` (`git fetch` at 20:48 MDT), and an ancestor of `HEAD`, so `d6ae539` is the PR's
    base.
  - The first A-dup's object-by-object review of the production code holds, because the 31 feature files this cycle did
    not touch are byte-identical to `b488a33`.
- **Hedged:**
  - **Warn 1 is a warn, not a block.** Both lists are true, the ADR's definition of the read implies the missing case,
    and F's design decision 4 records leaving the ADR and `pane.rs` on S's reading. It is not a parallel path.
  - **F's merge of `main`** sits outside S's REWORK list. It is branch upkeep, and I verified it is the automatic merge
    plus F's prose, so it is not drift.
- **Evidence:**
  - **Git:**
    - `git fetch`; `git merge-base HEAD origin/main` is `d6ae539`;
    - the parents of `e6d5fa0` (`ccc53a0`, `d6ae539`) and of `d2922a9` (`e6d5fa0`);
    - `git merge-tree --write-tree ccc53a0 d6ae539` gives `e77e7a0`, no conflict; `git diff e77e7a0 e6d5fa0` equals the
      remerge diff (5 prose files and 2 handoffs);
    - `git diff b488a33 ccc53a0`: handoffs only; `git diff b488a33 HEAD` over the 31 other feature files: empty;
    - the files #851 and #835 share (`comm` of both name lists): `CHANGELOG.md`, ADR-0003 and the guide;
    - the remerge diff of the `.rs` files, filtered for lines that are not comments: none.
  - **Read:** the full remerge diff; `git diff origin/main...HEAD` of ADR-0003, the CHANGELOG and the guide; ADR-0021
    `:944-991`; `pane.rs:130-164`; `ports.rs:296-349`; `import/observe.rs:1-140`; handoff-A pass 4's reuse map;
    handoff-S, handoff-F cycle 3, handoff-T-green cycle 3 and this journal from the first A-dup on.
  - **grep:** the foreground wording and "each process" across the docs and `src`; `"claude"` in every `src` (one
    command-name comparison); the callers of `is_claude_code_command` (one in production); tests pinning the limits
    wording (none); personal names in the added lines (none).
  - **Sizes:** `wc -l` of the five files: 1007, 353, 220, 244 and 821 (the guide).

## S, spec audit (cycle 2, after S's REWORK 1 and the merge of #851): 2026-10-11T20:59-06:00 (MDT)

- **Decided:** REWORK (production, docs only), two items (`docs/handoffs/835/handoff-S.md`, which replaces cycle 1's at
  `ccc53a0`). `archChanged: false`.
  - **Cycle 1's REWORK 1 is resolved.** These places all say that the import reads the foreground process of each tmux pane,
    and each limits sentence names a `claude` outside the foreground:
    - the help;
    - the guide's sections 1, 3 and 10;
    - the CHANGELOG;
    - `read_tmux`'s doc;
    - the `holler-pane` import module doc (F's DD5).
  - **All 20 acceptance criteria have proving evidence.** The production code is the code I audited in cycle 1: the 32
    files this cycle did not touch are byte-identical to `b488a33`, except ADR-0003's change from #851. I re-read the full
    production diff anyway.
  - **Item 1: two sentences say that no story owns the adoption after #831.** They are ADR-0021 "Deferred to named
    stories" (`:1106`) and the guide's section 12 (`:816-817`).
    - #831's issue body has recorded since 15:50 MDT, as an operator decision of 2026-10-11, that #831 owns the step.
    - Brief §5's fact went stale when the operator did what §5 asked ("add a line to #831"). So no brief amendment is needed
      and this is not an `ADVISORY-HOLD`.
    - My cycle-1 advisory 6.2 repeated the stale fact.
  - **Item 2, in the same pass: A-dup's warn 1.** The limits in ADR-0021 section 13 and `is_claude_code_command`'s doc gain
    the case the user docs name: a `claude` that is not the foreground process (a wrapper script, a suspended job).
- **Assumed:**
  - T's and F's recorded runs stand. I re-ran no build or test, as the role says.
  - My checks were read-only:
    - `git diff`, `git show --remerge-diff`, `git merge-tree`, `git log -S`, `git blame`;
    - `grep`, `wc -l`, `rustfmt --check`, `changelog-check.sh`;
    - the built binary's `--help`;
    - `gh issue view 835` and `831`, `gh api` (#831's body edits), `gh pr list`.
  - The script commits this handoff and this entry at the phase commit.
- **Hedged:**
  - **Why REWORK and not PASS with an advisory.** No behaviour is wrong. But the PR would put two false statements into
    the contract and the operator guide. This run's precedent corrects such cross-story claims inside the PR (A pass 4, W2
    and W3). Item 2 rides along because F edits ADR-0021 in the same pass; on its own it would be an advisory.
  - **The constant's name differs from #831's text.** #835 spells it `CLAUDE_CODE_COMMAND`; #831's branch spells it
    `CLAUDE_COMMAND`. This needs no change here: #835's comment of 18:12 MDT says #831 adopts this story's name if told.
    Telling it is an outward action, so it is Advisory 1, for the run agent or the operator.
- **Evidence:**
  - Live:
    - `gh issue view 835`: the body equals the brief's issue section; the comments are at 15:51 and 18:12 MDT;
    - `gh issue view 831`: the section "Adopting the fleet's existing Claude Code panes";
    - `gh api graphql` `userContentEdits`: the ownership text is present from the 15:50 MDT revision;
    - `git fetch`: `origin/main` is `d6ae539`, an ancestor of `HEAD`;
    - `origin/issue-831-implementation` at `008a995`, read through git only: `CLAUDE_COMMAND` in ADR-0021 section 15,
      items 2 and 17, and in its brief.
  - Git:
    - `git merge-tree --write-tree ccc53a0 d6ae539` gives `e77e7a0`, and `git diff e77e7a0 e6d5fa0` is F's 5 prose
      files plus the handoffs;
    - `e6d5fa0..HEAD` changes handoffs only;
    - `git log -S` puts the two ownership sentences in `9bd101b` (18:42 MDT) and `4604693` (19:34 MDT).
  - Read: the brief; every handoff (cycle-1 T-red from `36b0de8`); `decisions.md`; `evidence.md`; the full production and
    docs diff; ADR-0021 `:950-986`, `:1013-1023`, `:1082-1107`; the guide's sections 1, 3, 10 and 12;
    `docs_pane_control_test.rs` (what it pins); `.github/workflows/ci.yml` (the steps; PR size is advisory).
  - Greps:
    - `"claude"` in every `src`: one constant and one comparison;
    - `unwrap`, `expect`, `panic` and `#[allow` in the added lines;
    - private names, IPs, paths and secrets in the 3,303 added lines;
    - every place that states the read or its limits.

## F, implement S's cycle-2 REWORK (cycle 4, text only): 2026-10-11T21:09-06:00 (MDT)

- **Decided:** S's two items, in place, with no behaviour or test change (`docs/handoffs/835/handoff-F.md`, which replaces
  cycle 3's at `e6d5fa0`). `archChanged: false`.
  - **Item 1, #831 owns the adoption.** I confirmed it live first: #831's issue body says "This story owns the step that
    adopts Claude Code panes that already exist." in its section "Adopting the fleet's existing Claude Code panes (added
    2026-10-11, operator decision)".
    - ADR-0021 "Deferred to named stories": the entry names #831 after the colon, by the operator's decision of 2026-10-11
      recorded in #831's issue. The switch's refusal, the pane left alone and "changes that step or takes another route"
      stay.
    - The guide's section 12: "(#831, the Claude Code harness; until it lands, `--adopt-claude-code` is refused)". Still
      two lines; the guide stays at 821.
    - S's optional row, ADR-0021 section 13 "The switch": "#831 owns it (operator, 2026-10-11)", and "#831 changes that
      step" for "its owner changes that step".
    - No route is described as #831's, as S said.
  - **Item 2, the same three limits in all five places.** ADR-0021 section 13's "The rule's limits" and
    `is_claude_code_command`'s doc now name the three cases of the help, the guide and the CHANGELOG: the `claude` has
    exited; it is not in the foreground (a wrapper script that stays in the foreground runs it, or it is suspended); it
    was started under another `argv[0]` (a wrapper that `exec`s it under another name, or `node` running the script). R2-5's
    two cases and its "a default, not a proof" sentence stay.
  - **S's advisory 2, folded in:** `ports.rs`'s module doc says "the foreground process of each pane of a tmux session"
    for "each pane process".
- **Assumed:**
  - `origin/main` stays at `d6ae539` until the phase commit (`git fetch` at 21:04 MDT: `HEAD..origin/main` is empty).
  - The script's phase commit stages these four files with the handoff and this entry.
- **Hedged:**
  - **Decision 22 of ADR-0021's decisions list is left as it is.** It records #835's own decision and is still exact; the
    ownership is recorded where S named it. If S wants it in the list too, it is one clause.
  - **The ADR's limits explain why each "not in the foreground" example hides `claude`** (the script's interpreter is
    named; the suspended job's shell has the foreground). The user docs do not, by design (the guide's line budget). Both
    list the same three cases, which is S's rule.
- **Evidence:**
  - Live: `git fetch` (`origin/main` `d6ae539`, ancestor of `HEAD`); `gh issue view 831` (open, last updated 16:00 MDT;
    the ownership section quoted above).
  - Commands: `import::the_adrs_name_the_switch_and_the_two_new_codes` (1 passed); `docs_pane_control_test` (17 passed)
    and `docs_cli_test` (3 passed); rustdoc `-D warnings -p holler-pane` (exit 0); `changelog-check` (ok);
    `git diff --check` (clean); `cargo test -p holler-pane --doc` (7 passed, both `compile_fail,E0046` included); clippy
    `-D warnings` on the workspace (exit 0); `lint.sh` (exit 0); CI's workspace step (exit 0, 155 groups, 2406 passed,
    0 failed, 32 ignored).
  - Greps: every "has exited", "suspended", "not recognised" and "under another" outside the handoffs (the five places,
    each with three cases); every "no story owns", "owns the adoption", "its owner" (none left about #831); the tests
    that read the guide or ADR-0021 (none pins section 12, "Deferred" or the limits).
  - Read: `handoff-S.md` (cycle 2), `handoff-A-dup.md` (warn 1), the brief (R2-5, section 5), handoff-F cycle 3, this
    journal; ADR-0021 `:950-989`, `:1082-1111`, `:1168-1175`; the guide `:697-708`, `:809-821`; `pane.rs:140-159`;
    `ports.rs:1-31`, `:293-349`; `pane/import.rs:55-120`; `CHANGELOG.md:76-91`; `scripts/pane-restart:74-88`.
  - Sizes: the guide 821 (unchanged), ADR-0021 1180, `pane.rs` 304, `ports.rs` 497.

## F, implement S's cycle-3 REWORK (cycle 5): merge `origin/main` after #857: 2026-10-12T05:04-06:00 (MDT)

- **Decided:** S's work list (`handoff-S.md` cycle 3, items 1a to 1e and 2a to 2i), as brief round 5 says
  (`docs/handoffs/835/handoff-F.md`, which replaces cycle 4's at `f1d1bd0`). `archChanged: false`.
  - **Merged `origin/main` `b76f4d2`** with `--no-commit --no-ff`. The 10 conflicts S listed are resolved per S's table and
    staged by path. `unmanaged.rs`, `wired.rs`, `ports_test.rs` and `claude_code_rule_test.rs` now equal `main`'s.
    `docs_pane_control_test.rs` keeps both sides' functions.
  - **The silent defects:**
    - 1a: one `CLAUDE_CODE_COMMAND` and one `is_claude_code_command`, at `main`'s place, with `main`'s use sentence and the
      branch's three-limit paragraph;
    - 1b: the branch's ADR decisions 22 and 23 are now 30 and 31, after `main`'s 29;
    - 1c: one "Deferred" adoption entry, the branch's, rewritten.
  - **Item 2:** every sentence of this story that tied the switch or the adoption to the Claude Code harness now names
    #831's adoption step (ADR-0021 section 15, item 17; decision 23), and none names a route. This covers S's places, plus
    three more with the same claim that S's line list and grep miss (`pane/import.rs`'s module doc and
    `CLAUDE_CODE_STORY`'s doc, `ImportFinding::HarnessClaudeCode`'s doc, ADR section 13's "The rule").
  - **The other item-2 edits:**
    - the guide's section 1, 10 and 12 resolutions follow S's 2c to 2e;
    - the CHANGELOG no longer calls `foreground_commands` new;
    - the limits statements are untouched.
- **Assumed:**
  - The script's phase commit concludes the in-progress merge as a two-parent commit, as it did for `526bbd3` and
    `e6d5fa0`.
  - `origin/main` stays `b76f4d2` until then (fetched at 04:43 MDT).
  - T-green reviews the four merged test files, as the brief says.
- **Hedged:**
  - **`unmanaged.rs` drops the branch's optional sentence.** S allowed it if it agrees with `main`'s section 14. It does,
    but `main`'s own paragraph says the same two lines below. The branch's hunk also says "and so does #831", which is
    false on the merged tree: the only callers are `unregistered` and `reconcile.rs:446`.
  - **The guide's section 1 says "will be a re-run"** where `main` said "is". With "which waits for #835 and is not built"
    dropped, the present tense would describe an adoption that section 12 lists as not available.
  - **Two of `main`'s sentences go stale when this PR merges, and are left alone** because they are not on S's list and
    are #831's records: ADR-0021 section 15 item 17's "until this harness exists", and the #831 CHANGELOG entry's "waits
    for #835". They are handoff-F Known issue 1, for S or #831's adoption step.
  - **The three places beyond S's line list** are the brief's rule ("Prose that says the adoption waits for the Claude
    Code harness must say it waits for #831's adoption step"), applied to this story's own sentences. They are not new
    scope.
- **Evidence:**
  - Commands on the merged tree (the staged index):
    - `cargo build --workspace --all-targets`: exit 0;
    - `lint.sh`: exit 0; `changelog-check`: ok; clippy `-D warnings`: exit 0;
    - CI's workspace step: exit 0, 167 groups, 2600 passed, 0 failed, 36 ignored. This includes
      `docs_pane_control_test` (19), `docs_cli_test` (3), `pane_verbs` (347) and the `holler-pane` doctests (7, both
      `compile_fail,E0046`);
    - rustdoc `-D warnings -p holler-pane`: exit 0; `cargo machete`: clean;
    - `git diff --check` and `git diff --check origin/main`: clean;
    - `git merge-tree --write-tree` of a throwaway two-parent object of the staged tree against `origin/main`: exit 0.
  - Git:
    - `git diff origin/main` of every resolved file: only this story's content is left;
    - `git diff --stat origin/main`: 21 files outside the handoffs;
    - `git grep` for decisions cited by number (none of 22 or 23 is cited);
    - S's `lands`/`831` grep: every hit names the adoption step, except `main`'s own ADR:1145, which is accurate.
  - Read:
    - the brief;
    - `handoff-S.md` (cycle 3);
    - handoff-F (cycle 4);
    - this journal;
    - `main`'s ADR-0021 sections 2, 9, 13, 14 and 15 (items 2 and 17), "Deferred" and decisions 21 to 29;
    - `docs_pane_control_test.rs`;
    - `.github/workflows/ci.yml`.
  - `evidence.md`, section "F, cycle 5": four quotes of `main`'s text, each checked line by line against its source.

## T-green, cycle 4 (after F cycle 5, the merge of `origin/main` after #857): 2026-10-12T05:12-06:00 (MDT)

- **Decided:** PASS. The suite is GREEN on the merged tree `59d8d59`, and nothing blocks.
- **Assumed:** none of the test files resolved in the merge needs repair. Three of them equal `main`'s, and
  `docs_pane_control_test.rs` keeps both sides' functions.
- **Hedged:** I repeated the behaviour-removal check on the predicate, because the import now calls `main`'s copy of
  `is_claude_code_command`. I did not repeat the cycle-1 and cycle-2 mutations of the other paths, because their code
  did not change.
- **Evidence:**
  - the workspace run (CI's step): exit 0, 167 groups, 2600 passed, 0 failed, 36 ignored;
  - clippy `-D warnings`, `lint.sh`, `changelog-check`, `docs_cli_test` (3), `wire_selftest` (3), `cargo machete`,
    rustdoc `-D warnings -p holler-pane`: all clean;
  - `git diff --check origin/main...HEAD` outside the handoffs: clean; `git merge-tree --write-tree HEAD origin/main`:
    exit 0;
  - `is_claude_code_command` set to `false`: 3 of 7 `import::harness` tests fail (the three Claude Code tests), the file
    restored after;
  - `git diff --quiet origin/main HEAD` holds for `wired.rs`, `ports_test.rs`, `claude_code_rule_test.rs` and
    `unmanaged.rs`.

## A, anti-duplication gate (third pass, after F cycle 5, the merge of `origin/main` after #857): 2026-10-12T05:25-06:00 (MDT)

- **Decided:** PASS, 0 `block` and 2 `warn` (`docs/handoffs/835/handoff-A-dup.md`, which replaces the second pass's at
  `2156b5f`).
  - **The scope.** This cycle's diff is `git show --remerge-diff 59d8d59`: F's resolutions beyond git's automatic merge of
    `cea1c17` and `b76f4d2`. I also read the whole PR, `git diff origin/main...HEAD`. `6787d99` changed only handoffs.
  - **No parallel path.** #857 put this story's port method, constant, rule, fake, case 10 and forwards on `main`. After
    the merge, the PR extends those objects and carries no second copy. The 16 files that left the PR's diff each equal
    `main`'s. Every line the branch had added in them is on `main`, except three that took `main`'s wording by S's table.
  - **S's silent duplicates are resolved:**
    - one `CLAUDE_CODE_COMMAND` and one `is_claude_code_command`;
    - the ADR decisions run 1 to 31, each number once;
    - one "Deferred" adoption entry.
  - **No duplicated text.** None of the PR's non-test files has a long line twice where `main`'s copy has it once.
  - **Warn 1 (new, about records).** The merge made one of this story's runtime messages false: `assess.rs:104-105`, "which
    Holler cannot drive yet". `main` drives Claude Code since #857. It is the same claim S's item 2 covers, and it is
    shown to the operator in the cutover's dry run. No test pins it.
  - **Warn 2 (carried, test-only).** `run_with_host` copies `run_with_store`, and `text_row` twins `text_line`.
- **Assumed:**
  - `origin/main` is still `b76f4d2` (`git fetch` at 05:21 MDT) and is an ancestor of `HEAD`, so it is the PR's base.
  - T-green's GREEN on `59d8d59` holds for `6787d99`, because that commit changed only handoffs. I ran no build or test.
- **Hedged:**
  - **W1 is a `warn`, not a `block`.** It is not a parallel path. It is a records defect of the kind S rules on, and fixing it
    means changing one string.
  - **`read_tmux` and `main`'s `refuse_in_use` both combine `foreground_commands` with the predicate.** This is not a
    finding. The two treat errors differently, and a shared helper would hold one `.any(...)` line. R2-7 is met, because
    neither one compares a command name with `"claude"` itself.
  - **ADR-0021 `:1023`, "which #831 can reuse", is left as optional.** It is still true; #831's launch now uses the
    predicate.
- **Evidence:**
  - **Git:**
    - `git fetch`; the merge base is `b76f4d2`; `59d8d59` has two parents;
    - `git merge-tree --write-tree HEAD origin/main` is clean (`9fd0f25`);
    - `git diff --stat 59d8d59 6787d99`: handoffs only;
    - the file lists of `git diff --name-only d6ae539 f1d1bd0` and of `origin/main HEAD`: 16 files left, none came in;
    - `git diff --quiet origin/main HEAD` holds for each of the 16;
    - the branch's added lines in those files, checked against `main`'s copies: only three are not there, and each took
      `main`'s wording;
    - `git diff f1d1bd0 HEAD` of the import files: only `main`'s `HarnessEndpoint` and `Some(port)` lines, plus F's prose.
  - **Read:**
    - the full remerge diff, and the PR's diff of every code and docs file;
    - ADR-0021: sections 2 (`:92-124`), 13 (`:982-1059`), 14 (`:1061-1116`), 15 items 2 and 17, "Deferred" and the
      decisions;
    - the guide: sections 1, 2, 3, 10 and 12;
    - `pane.rs:170-246`, `unmanaged.rs:60-90`, `tx_launch/process.rs:1-110`;
    - `import/observe.rs` in full, `import/assess.rs:50-139`, `import/harness.rs` in full;
    - the brief (round 5), `handoff-A.md` pass 4, `handoff-S.md` cycle 3, `handoff-F.md` cycle 5, `handoff-T-green.md`
      cycle 4, and this journal from the second A-dup on.
  - **grep:**
    - `CLAUDE_CODE_COMMAND`, `is_claude_code_command` and `"claude"` in `crates/`: one definition of each, two callers;
    - every `impl HostPort` (12) and `impl HerdrPort` (11), with their `foreground_commands` and `observe_pane`;
    - `compile_fail` on `main`: no `E0046` doctest;
    - decisions cited by number;
    - "cannot drive", "until that story lands" and their variants in the PR's added lines (W1);
    - personal names in the added lines: none.
  - **Sizes:** `wc -l` of every `.rs` file in the PR's diff: the largest is `ports.rs`, at 621.
