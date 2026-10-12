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
