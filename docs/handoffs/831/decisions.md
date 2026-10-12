# Decision journal: #831 adopt, adopting the fleet's existing Claude Code panes

Run handoff directory `docs/handoffs/831` - branch `issue-831-implementation` - rigor in-session - uiSurface false. Append-only.

## A, up-front plan review: 2026-10-12

- **Decided:** BLOCK, 3 `block` and 7 `warn` (`docs/handoffs/831/handoff-A.md`). The plan extends the right objects and keeps the choke point.
  1. AC4 copies `pane.env`'s OpenCode values into a Claude record: `OC_DEFAULT_AGENT` makes the later `relaunch` refuse (`agent-unsupported`), and `OC_MODEL`/`OC_EFFORT` go to `claude --model --effort`.
  2. tmux has no foreground-process-group format, so `foreground_pids` needs `#{pane_tty}` plus a `ps` call: a new subprocess seam the brief does not name.
  3. Reusing the undo for an extension would tell the operator to `profile delete fleet --keep-panes`, which deletes the pre-existing profile.
- **Not blocking:** optional launcher port (where the check moves), the `PaneId` that `shown_session` needs, `Plan` carrying create-or-extend, final finding codes, a differing existing spec, `ports.rs` near 800 lines, call count per pane.
- **Read:** the brief; `import/{observe,assess,files,findings}.rs`, `import.rs`; `writes.rs`, `launcher.rs`, `pane_env.rs`; `ports.rs`, `pane.rs`, `profile_snapshot.rs`; the Claude and host adapters; `lint.sh`; ADR-0021 lines cited by the brief.
