# Decision journal: #831 adopt, adopting the fleet's existing Claude Code panes

Run handoff directory `docs/handoffs/831` - branch `issue-831-implementation` - rigor in-session - uiSurface false. Append-only.

## A, up-front plan review: 2026-10-12

- **Decided:** BLOCK, 3 `block` and 7 `warn` (`docs/handoffs/831/handoff-A.md`). The plan extends the right objects and keeps the choke point.
  1. AC4 copies `pane.env`'s OpenCode values into a Claude record: `OC_DEFAULT_AGENT` makes the later `relaunch` refuse (`agent-unsupported`), and `OC_MODEL`/`OC_EFFORT` go to `claude --model --effort`.
  2. tmux has no foreground-process-group format, so `foreground_pids` needs `#{pane_tty}` plus a `ps` call: a new subprocess seam the brief does not name.
  3. Reusing the undo for an extension would tell the operator to `profile delete fleet --keep-panes`, which deletes the pre-existing profile.
- **Not blocking:** optional launcher port (where the check moves), the `PaneId` that `shown_session` needs, `Plan` carrying create-or-extend, final finding codes, a differing existing spec, `ports.rs` near 800 lines, call count per pane.
- **Read:** the brief; `import/{observe,assess,files,findings}.rs`, `import.rs`; `writes.rs`, `launcher.rs`, `pane_env.rs`; `ports.rs`, `pane.rs`, `profile_snapshot.rs`; the Claude and host adapters; `lint.sh`; ADR-0021 lines cited by the brief.

## A, up-front plan review (second pass, amended brief): 2026-10-12

- **Decided:** BLOCK, 1 `block` and 6 `warn` (`docs/handoffs/831/handoff-A.md`). The amendment resolves the first pass's 3 blocks (D3 model/effort/agent rule; the `ps -t` mechanism of `foreground_pids`; a record-delete-only undo for an extension). The 10 `impl HostPort for` sites and the cited lines match the tree.
  1. "Launcher" has `assess` emit `source-missing` for a port-less entry "or a Claude pane with the flag off". Today a flag-off Claude pane is `Outcome::claude_code(name)`, which drops `files.problems`, so it is `skipped`; the sentence would make the operator's existing import `import-blocked`, against AC3.
- **Not blocking:** `Target.adopt` cannot encode off / on-labelled / on-unlabelled; the stale "until #831 lands" message and help; the undo's failed-record check would delete a concurrent `fleet` member in an extension; `fake_tmux_test.rs` at 822 lines; the OpenCode helpers the Claude record must skip; partial `ps` answers.
- **Read:** the brief; `import/{observe,assess,files,findings}.rs`, `import.rs`; `writes.rs`, `launcher.rs`, `sources.rs`; `ports.rs`, `ports/router.rs`, `profile_snapshot.rs`; the host and Claude adapters; `lint.sh`; ADR-0021 lines cited by the brief.

## A, up-front plan review (third pass, brief amended twice): 2026-10-12

- **Decided:** PASS, 0 `block` and 5 `warn` (`docs/handoffs/831/handoff-A.md`). The second pass's block is resolved: the port-less `source-missing` is emitted on the OpenCode path only, a flag-off Claude pane keeps `Outcome::claude_code`, and a named test pins it. The second pass's six warns are folded in. The cited tree (12 `HostPort for` hits, `checked`, `write`/`undo`, store behaviour, file sizes) matches.
  1. Warns for F and Phase 7: a second `Pane { .. }` literal and a copied `assess` tail in `adopt.rs` would be a parallel path; share `record()` and the join tail.
  2. Port 0 and a relative cwd are `source-unreadable` in the tree, not `source-missing`; T pins the real codes.
  3. Decide the test file split (`adopt.rs`, `adopt_undo.rs`) up front.
- **Read:** the brief; `import/{assess,observe,files}.rs`, `import.rs`; `writes.rs`, `launcher.rs`; the testkit and hub pane stores; `profile_diff.rs`; `lint.sh`; ADR-0021 lines 110, 1577, 1743.
