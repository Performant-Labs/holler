# #655: the Herdr server on the hub's machine, decisions and rollback

Operator-run, no code. Run on 2026-10-11 (America/Denver), from the hub's machine. The machines
are called "the hub's machine" and "the workstation" here; no hostnames are recorded.

## State before

- The Herdr server for the fleet (session `jupiter`) ran on the workstation. Each of its eight panes
  attached to a tmux session on the hub's machine over ssh.
- The hub's machine ran only a `default` Herdr session, with two unrelated test panes.
- The eight tmux sessions `hj-c1r1` to `hj-c4r2` (the pane names; the name lists the column first) were
  attached by the workstation's Herdr, 61 by 31 or 32 cells each.

## Steps taken

1. Started a second Herdr server on the hub's machine, session `jupiter`, in a detached tmux session
   (`tmux new-session -d -s herdr-session-jupiter 'herdr --session jupiter'`), the way the existing
   `default` session is kept. Herdr needs a terminal to start, which tmux provides; the server then
   runs headless. Socket: `~/.config/herdr/sessions/jupiter/herdr.sock`, a local path on the hub's machine.
2. Created the workspace `jupiter` and built a 2 row by 4 column grid by splitting right three times
   and each column down once (row 1: `hj-c1r1`, `hj-c2r1`, `hj-c3r1`, `hj-c4r1`; row 2: `hj-c1r2` to
   `hj-c4r2`). Each pane is labelled with its pane name (`herdr pane rename`), so `pane import` can
   join by label.
3. Attached each pane to its live tmux session with `tmux attach-session -f ignore-size -t NAME`.
   `ignore-size` keeps the new client out of tmux's size negotiation, so the workstation's panes
   were not resized (checked with `tmux list-clients`: the old clients stayed at 61 by 31 or 32).
4. Checked from the workstation, over ssh, that the new server lists the panes: 9 panes (the eight
   and the default `~` pane that every new session starts with).

## Decisions

- **Same session name, `jupiter`, on the hub's machine.** The issue names it. A session name is local to
  a machine, so the old server and the new one do not collide.
- **tmux stays under every pane.** The panes are the existing fleet sessions, and the harness servers run
  inside those tmux sessions. Herdr's server persists panes too, but the harness lifecycle (ADR-0021
  section 2) is a tmux session, and the move changes only who shows it. Recorded here for the ADR (#634).
- **Both servers show the same tmux sessions for now.** Nothing was moved or stopped. The old server
  and layout are untouched until the soak ends.
- **The default `~` pane stays.** It is Herdr's own first pane; the cutover decides about it.

## Not verified by this run

- That a pane survives a reboot of the workstation: not tested (a reboot is the operator's call). It
  should, because the server and the tmux sessions are on the hub's machine, not the workstation.
- That the server survives a reboot of the hub's machine: it does not. Like the existing `default`
  session, it is a tmux session started by hand. Autostart is not part of this story.
- `herdr --remote` from the workstation needs a terminal; the pane list over ssh was checked instead.

## Rollback

Nothing else was changed, so rollback is removing the new server:

```bash
herdr session stop jupiter
herdr session delete jupiter
tmux kill-session -t herdr-session-jupiter
```

The tmux sessions `hj-*` are not touched by it (a detached client leaves them running). The
workstation's server and layout are as they were.
