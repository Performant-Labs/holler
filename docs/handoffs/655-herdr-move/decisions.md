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

## Correction (2026-10-11, evening): the grid was built column-major, and was rebuilt row-first

The first build (steps 2 and 3 above) split right three times and each column down once. The Holler adapter reads rows as the
root's chain of `down` splits (`crates/holler-adapter-herdr/src/layout.rs`, `grid_of`; ADR-0021 section 10), so that tree gave
no pane a cell: the adapter listed none of the eight, and `holler pane import` found no pane with a label (`pane-unlabelled`).
The earlier check (the pane list over ssh) proved only that the panes existed, not that the adapter could place them.

The workspace was rebuilt in the new session only, beside the old one, following `scripts/spikes/herdr-grid.sh` and the
adapter's own recipe (`plan_splits`): a `down` split for the second row (ratio 0.5), then each row's `right` splits (ratios
0.25, 0.3333, 0.5). `herdr-relayout` does not exist, and `plan_splits` cannot be called on its own (it runs inside
`holler pane launch`), so the calls were made by hand.

Order and checks:

1. The old workspace was renamed `jupiter-colmajor`; the new one was created as `jupiter`.
2. The tree was read back: 2 rows of 4, each pane at the cell it should have.
3. Each new pane ran `tmux attach-session -f ignore-size -t <name>` (3 clients on each tmux session meanwhile).
4. The old labels were cleared, then the new panes were labelled (`hj-c<col>r<row>`, the name that sits at `r<row>c<col>`).
5. `holler pane import --dry-run` against a scratch hub, with `HOLLER_HERDR_SESSION=jupiter` and
   `HOLLER_HERDR_WORKSPACES=jupiter=2x4`, joined all eight panes by label at `r1c1` .. `r2c4`, with no `pane-unlabelled`,
   `pane-label-ambiguous` or `name-cell-differs`. It wrote nothing; the state root's files were unchanged by hash.
6. Only then was the old workspace closed. That ended its eight `tmux attach` clients and nothing else: the eight tmux
   sessions and their agents were untouched, and every session kept two clients (the workstation's and the new one).

The workstation's server and layout were not touched at any point. The import's other findings (`source-missing`,
`source-unreadable`, `sources-disagree` on one pane) are state-root work, not layout.

Lesson for the guide and for any rebuild: build rows first (outer splits `down`), then split each row `right`, and check the
layout with the adapter (`pane import --dry-run`) before trusting it.
