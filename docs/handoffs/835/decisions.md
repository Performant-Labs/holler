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
