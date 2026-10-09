# Herdr socket API spike: pane operations, positions, layout, versions and plugins

**Status:** research spike for issue #636 (part of epic #633), **not a decision record**. ADR-0021 and the
stories that build on it (#640 the Herdr adapter, #651 the display plugin, #655 the server move) decide.
**Date:** 2026-10-09.
**Herdr tested:** `0.9.1-preview.2026-09-21-0ff0f27e2226`, socket protocol `22`, schema version `1`, on Linux.
It is the only Herdr build on the test machine.
**Scripts:** [`scripts/spikes/herdr-ops.sh`](../../scripts/spikes/herdr-ops.sh) (operations, events, agent status,
metadata), [`herdr-grid.sh`](../../scripts/spikes/herdr-grid.sh) (positions, layout model, nesting),
[`herdr-restart.sh`](../../scripts/spikes/herdr-restart.sh) (id stability, attach, restart),
[`herdr-plugin.sh`](../../scripts/spikes/herdr-plugin.sh) (plugins, sidebar), and the shared harness
[`herdr-lib.sh`](../../scripts/spikes/herdr-lib.sh). Each needs `herdr`, `jq` and `python3`; the attach and
sidebar steps also need `tmux`. Run any of them with `bash scripts/spikes/herdr-<name>.sh`.

**Evidence labels used below.** **VERIFIED**: a spike script ran it against a scratch Herdr server and the
output is quoted. **SCHEMA**: read in `herdr api schema --json`, not exercised. **DOCS**: read in `herdr --help`,
`herdr --skill` or `herdr --default-config`. **BINARY**: read in strings of the Herdr binary. **INFERRED**:
reasoning from the above. **UNVERIFIED**: not checked; what remains to check is said each time.

## 1. How the spike stayed off real sessions

Every script sources `herdr-lib.sh`, which:

- creates a fresh root with `mktemp -d` and runs every `herdr` call with `HOME` and all `XDG_*` directories inside
  it and every inherited `HERDR_*` variable removed, so even Herdr's *default* session resolves to a socket inside
  the scratch root, where no real server listens;
- also names the session on every call: `--session spike636-<8 random hex>`;
- refuses to continue unless `herdr status server --json` reports that session and a socket inside the scratch
  root (`target proven: session=<scratch-session> socket=<scratch-root>/home/.config/herdr/sessions/<scratch-session>/herdr.sock`);
- writes a scratch `config.toml` with update and manifest checks off (no network), `/bin/sh` as the pane shell and a
  neutral prompt;
- uses a private tmux server (`tmux -L <scratch-session>`) for the TUI client checks, never the default tmux server;
- on exit stops only the server PID it started, kills only its private tmux server, checks that no process still
  names the scratch root and deletes the root.

Pane commands were `echo`, `sleep`, `cat` and `/bin/sh` only. No plugin was installed into a real Herdr config;
the plugin probe links a plugin into the scratch config home.

## 2. Summary

- **Coverage.** The socket API covers every operation Holler needs: create (workspace, tab, split, or a whole
  layout), run, send text, send keys, read, wait for output, close, snapshot, and what each pane runs
  (`pane.process_info`, with argv). VERIFIED.
- **Addressing.** A pane is `w<N>:p<M>` (for example `w1:p7`). Ids are opaque, never reused after close, survive
  resize, zoom, swap, close of a sibling, client attach and detach, and a full server restart. A move into another
  workspace gives a new id. VERIFIED.
- **Positions.** Herdr has no grid cell and no row or column index. It reports each pane's rectangle in terminal
  cells as `{x, y, width, height}`: **x (column axis) first, then y (row axis); 0-based; origin at the top left of
  the tab area**. Numbers change with the attached client's terminal size; their order does not. #640 converts a
  `GridPos` by ranking (section 6). VERIFIED.
- **Layout model.** Splits only (`right` or `down`), as a binary tree. A split divides only the target pane's own
  cell. `layout.apply` can build a whole tree in one call and start argv commands without a shell, but applied to
  an existing tab it **replaces the tab and kills its panes**. VERIFIED.
- **Restore.** A server restart restores workspaces, tabs, labels, the split tree and pane ids, but every pane comes
  back as a **fresh default shell**: running commands, argv launches, metadata and agent reports are gone. VERIFIED.
- **Events.** `events.subscribe` streams NDJSON on the open connection: pane created, closed, exited, layout
  updated, agent status changed and more. VERIFIED.
- **Plugins.** A plugin is a `herdr-plugin.toml` with actions, panes, event hooks and startup commands. Its process
  gets the session socket and can call every method, including mutating ones; Herdr does not enforce "display
  only". The sidebar shows custom `$tokens` reported through `pane.report_metadata` and
  `workspace.report_metadata`. VERIFIED.
- **Recommendation.** The adapter talks to the socket directly; the display plugin only reports metadata.
  Section 12.

## 3. The schema and the wire

**How to read it** (VERIFIED):

```
$ herdr --session <scratch-session> api schema
Herdr API schema
protocol: 22
schema_version: 1
schemas: error_response, event, request, subscription_event, success_response
$ herdr --session <scratch-session> api schema --json     # full JSON Schema (draft 2020-12), about 270 KB
{"protocol":22,"schema_version":1,"schemas":["error_response","event","request","subscription_event","success_response"],"request_methods":104}
```

`herdr api schema` prints the schema **bundled in the CLI binary**. It needs no running server (VERIFIED: it works
with no server up), so it describes the client binary, not necessarily the running server. The server's own
version comes from `ping` (section 11). `--output PATH` writes the JSON to a file.

Structure: `request` is a `oneOf` over 104 methods, each `{"id": string, "method": const, "params": {...}}`;
`success_response` is `{"id", "result": {"type": <result kind>, ...}}` with 65 result kinds; `error_response` is
`{"id", "error": {"code", "message"}}`; `event` and `subscription_event` describe the event stream.

The pane, layout, events and topology methods (VERIFIED list from the schema):

```
session.snapshot workspace.create workspace.list workspace.get workspace.focus workspace.rename workspace.move
workspace.move_block workspace.report_metadata workspace.close tab.create tab.list tab.get tab.focus tab.rename
tab.move tab.close pane.split pane.swap pane.move pane.zoom pane.layout pane.process_info layout.export
layout.apply layout.set_split_ratio pane.neighbor pane.edges pane.focus_direction pane.resize pane.scroll
pane.clear pane.edit_scrollback pane.selection.read pane.copy_motion pane.copy_search pane.list pane.current
pane.get pane.focus pane.input.set pane.link.activate pane.link.resolve pane.rename pane.send_text pane.send_keys
pane.send_input pane.read pane.graphics.set pane.graphics.clear pane.graphics.info pane.report_agent
pane.report_agent_session pane.report_metadata pane.clear_agent_authority pane.release_agent pane.close
events.subscribe events.wait pane.wait_for_output
```

The rest are `ping`, `server.*` (including `server.stop`), `notification.show`, `agent.*`, `worktree.*`,
`integration.*`, `plugin.*` and a few client-only methods.

**Wire** (VERIFIED, `herdr-ops.sh`):

- A Unix stream socket, owner-only (`srw-------`). Default session: `$XDG_CONFIG_HOME/herdr/herdr.sock` (on Linux
  `~/.config/herdr/herdr.sock`); a named session: `$XDG_CONFIG_HOME/herdr/sessions/<name>/herdr.sock`.
  `herdr session list` prints each session's socket path. Inside a Herdr pane, `HERDR_SOCKET_PATH` names it.
- One JSON request per line, and **one request per connection**: a second request on the same connection is
  refused (`ConnectionResetError` / broken pipe). `events.subscribe` is the exception: the connection stays open
  and streams events.
- An unknown method is `{"error":{"code":"invalid_request","message":"invalid request: unknown variant `pane.nope`, expected one of ..."}}`.
- The CLI sends the same requests with ids like `cli:pane:close`. It prints the JSON response on stdout, or the
  error JSON on stderr with exit 1; a CLI syntax error exits 2. Exceptions: `pane read` prints plain text, and
  `pane run`, `pane send-text`, `pane send-keys` and the report commands print nothing on success.

## 4. Operations

Socket calls are `{"id": ..., "method": ..., "params": {...}}`; only the method and params are shown. "CLI" is the
equivalent `herdr --session <s> ...` command. All rows are VERIFIED by `herdr-ops.sh` unless marked.

| Operation | Supported | Call (socket; CLI) | Result shape | Stability |
|---|---|---|---|---|
| Create a workspace | yes | `workspace.create {cwd?, label?, env?, focus?}`; `workspace create --cwd D --label L --no-focus` | `{type: "workspace_created", workspace: WorkspaceInfo, tab: TabInfo, root_pane: PaneInfo}` | ids `w<N>`, `w<N>:t<M>`, `w<N>:p<M>`, never reused |
| Create a tab | yes (SCHEMA) | `tab.create {workspace_id?, cwd?, label?, env?, focus?}`; `tab create` | `{type: "tab_created", tab, root_pane}` | as above |
| Split | yes | `pane.split {target_pane_id, direction: "right"\|"down", ratio?, cwd?, env?, focus?}`; `pane split ID --direction right --ratio 0.25 --no-focus` | `{type: "pane_info", pane: PaneInfo}`; new pane id in `.result.pane.pane_id` | no `command` param: the new pane runs the default shell. `left`/`up` are `invalid_request` |
| Create a layout | yes | `layout.apply {workspace_id?, tab_id?, tab_label?, focus?, root: LayoutNode}` (socket only) | `{type: "layout_apply", layout: {workspace_id, tab_id, zoomed, focused_pane_id, root}}` with the new pane ids | without `tab_id`: a new tab. **With `tab_id`: the tab is replaced, its panes and processes are killed, pane ids in the input are ignored** |
| Run a command | yes, two ways | (a) `layout.apply` with `{"type":"pane","command":["sleep","601"]}`: the argv runs directly as the pane process, no shell; (b) `pane.send_input {pane_id, text, keys: ["enter"]}`; `pane run ID 'cmd'`: typed into the pane's shell | (a) as above; (b) `{type: "ok"}`, CLI prints nothing | (a) the argv is not re-run after a server restart (section 8) |
| Send text | yes | `pane.send_text {pane_id, text}`; `pane send-text ID TEXT` | `{type: "ok"}` | literal text, no Enter |
| Send keys | yes | `pane.send_keys {pane_id, keys: ["enter"]}`; `pane send-keys ID enter` | `{type: "ok"}` | logical key names (`enter`, `esc`, `ctrl+c`, ...) |
| Read | yes | `pane.read {pane_id, source: "visible"\|"recent"\|"recent_unwrapped"\|"detection", lines?, format?: "text"\|"ansi", strip_ansi?}`; `pane read ID --source recent --lines 4` (plain text) | `{type: "pane_read", read: {pane_id, workspace_id, tab_id, source, format, text, revision, truncated}}` | `revision` increases with output |
| Wait for output | yes | `pane.wait_for_output {pane_id, source, match: {...}, timeout_ms?}`; `pane wait-output ID --match TEXT --timeout MS` | `{type: "output_matched", matched_line, pane_id, revision, read: {...}}` | searches existing output first, then polls |
| Close | yes | `pane.close {pane_id}`; `pane close ID` | `{type: "ok"}`; again: `{"error":{"code":"pane_not_found","message":"pane w1:p5 not found"}}`, exit 1 | the sibling takes the space; a shell that exits closes its pane too (`pane_exited`) |
| Snapshot | yes | `session.snapshot {}`; `api snapshot` | `{type: "session_snapshot", snapshot: {version, protocol, workspaces[], tabs[], panes[PaneInfo], layouts[{workspace_id, tab_id, zoomed, area, focused_pane_id, panes[{pane_id, focused, rect{x,y,width,height}}], splits[{id, direction, ratio, rect}]}], agents[], focused_*}}` | split `id`s (`split_3_011`) encode a tree path and are renumbered on relayout: not stable |
| Export the tree | yes | `layout.export {tab_id? \| pane_id?}` (socket only) | `{type: "layout_export", layout: {..., root: LayoutNode}}`; nodes `{"type":"split", direction, ratio, first, second}` or `{"type":"pane", pane_id, label?, cwd?, command?}` | `command` shows the argv launch until the server restarts |
| What a pane runs | yes | `pane.process_info {pane_id}`; `pane process-info --pane ID` | `{type: "pane_process_info", process_info: {pane_id, shell_pid, foreground_process_group_id, foreground_processes: [{pid, name, argv, cmdline, cwd}]}}` | live OS state. Example: `{"name":"sleep","argv":["sleep","600"],"cwd":"<scratch-root>/work"}`. `tty` is in the schema but was never returned |
| Pane info | yes | `pane.get {pane_id}`, `pane.list {workspace_id?}`; `pane get ID` | `PaneInfo`: `pane_id, terminal_id, workspace_id, tab_id, focused, cwd, foreground_cwd, label?, title?, agent?, display_agent?, agent_status, agent_session?, state_labels?, tokens?, scroll, revision, restore_error?` | absent optional fields are omitted, not null |
| Move | yes | `pane.move {pane_id, destination: {"type":"tab", tab_id, split, target_pane_id?, ratio?} \| {"type":"new_tab"} \| {"type":"new_workspace"}}` (socket only; the CLI has `pane move`) | `{type: "pane_move", move_result: {changed, previous_pane_id, pane: PaneInfo, source_layout...}}` | same workspace: id and process kept; other workspace: **new id** (`w1:pE` became `w2:p1`) |
| Swap, resize, zoom | yes | `pane.swap {source_pane_id, target_pane_id}`, `pane.resize {pane_id, direction, amount}`, `pane.zoom {pane_id, mode: "on"\|"off"\|"toggle"}` | each returns the new layout | ids unchanged |
| Rename (label) | yes | `pane.rename {pane_id, label}`; `pane rename ID LABEL` | `pane_info` | label persists across restart |
| Agent status | yes | in `PaneInfo.agent_status`; `agent.list`, `agent.get`; `pane.report_agent {pane_id, source, agent, state}`; `pane report-agent` | `idle\|working\|blocked\|done\|unknown` | section 9 |
| Display metadata | yes | `pane.report_metadata {pane_id, source, title?, display_agent?, state_labels?, tokens?, ttl_ms?}`, `workspace.report_metadata {workspace_id, source, tokens}` | `{type: "ok"}`; shows in `PaneInfo`/`WorkspaceInfo` | lost on server restart |
| Subscribe to events | yes | `events.subscribe {subscriptions: [{"type":"pane.created"}, ...]}` (socket only) | `{"id":..,"result":{"type":"subscription_started"}}` then one event per line | section 10 |

## 5. Addressing and id stability

A pane is addressed by its public id `w<N>:p<M>`. A tab is `w<N>:t<M>` and a workspace `w<N>`. Also present:
`terminal_id` (`term_<hex>`), the pane's label, an agent name (agent commands only) and, inside a pane,
`HERDR_PANE_ID`. VERIFIED:

| Event | `pane_id` | `terminal_id` | Evidence |
|---|---|---|---|
| resize, zoom on and off, swap, a sibling closed | unchanged | unchanged | `herdr-restart.sh`: "ids of the surviving panes unchanged: true" |
| TUI client attached, then detached | unchanged | unchanged | "ids and tree identical before attach / attached / after detach: yes" |
| full server restart | **unchanged** | **changed** | "ids, labels and tree identical across the restart: yes"; "terminal ids changed: true" |
| move to another tab in the same workspace | unchanged (process kept) | unchanged | `herdr-ops.sh` move step: "same process after the move (pid unchanged)" |
| move to another workspace | **new id** | unchanged | exploration run (same calls, before the scripts were written): `w1:pE` became `w2:p1` (`move_result.previous_pane_id` keeps the old one) |
| close | gone, never reused | gone | after `w1:p3` closed, the next new pane was `w1:p5` |
| workspace closed | gone, never reused | gone | after `w2` closed the next workspace was `w3` |

The number after `p` is **not decimal**: after `w1:p9` came `w1:pA` ... `w1:pF`, `w1:pG` (base 36 on the saved
counter `next_public_pane_number`). Treat ids as opaque strings; never parse or predict them, read them from
responses. A workspace's `number` field is its display ordinal (it was `2` for `w3`), not its id.

**A pane id is not a position.** After `pane.swap` of the marker (`w1:p7`, at row 2 column 3) with `w1:p1`, the
marker is reported at row 1 column 1 and `w1:p1` at row 2 column 3 (`herdr-grid.sh`, last table). The id follows
the process; the position belongs to the slot in the layout.

## 6. Position: Herdr's native axis order and base

**Herdr exposes no grid position.** It exposes a rectangle per pane in terminal cells, `rect: {x, y, width,
height}`, in `pane.layout`, `session.snapshot.layouts[].panes[]`, and the `layout_updated` event, and the split
tree in `layout.export`. In that rectangle **x is the column axis and comes first, y is the row axis; both are
0-based cell offsets from the top-left corner of the tab's area** (the area excludes the sidebar; a headless
server uses a virtual 120x40, configurable as `[server] headless_cols/headless_rows`). VERIFIED.

**The probe** (`herdr-grid.sh`). It builds 2 rows by 4 columns by splits only, places the marker pane at row 2,
column 3 (`split r1c1 down 0.5`, then in each row `right 0.25`, `right 0.3333`, `right 0.5`), and reads it back:

```
$ herdr pane layout --pane w1:p7
{"focused":false,"pane_id":"w1:p7","rect":{"height":20,"width":30,"x":60,"y":20}}
pane    label        herdr rect {x,y,w,h}     rank (row,col) tree (row,col)
w1:p1   r1c1         {0,0,30,20}              r1c1           r1c1
w1:p3   r1c2         {30,0,30,20}             r1c2           r1c2
w1:p4   r1c3         {60,0,30,20}             r1c3           r1c3
w1:p5   r1c4         {90,0,30,20}             r1c4           r1c4
w1:p2   r2c1         {0,20,30,20}             r2c1           r2c1
w1:p6   r2c2         {30,20,30,20}            r2c2           r2c2
w1:p7   marker       {60,20,30,20}            r2c3           r2c3
w1:p8   r2c4         {90,20,30,20}            r2c4           r2c4
area {'height': 40, 'width': 120, 'x': 0, 'y': 0}  distinct x: [0, 30, 60, 90]  distinct y: [0, 20]
```

The pane placed at row 2, column 3 is reported as `x=60, y=20`: column-axis value first, 0-based (the top-left
pane is `x=0, y=0`). `pane.edges` adds which outer edges a pane touches (`{"left":false,"right":false,"up":false,"down":true}`).

**The numbers are not stable, their order is.** With a TUI client attached, the area follows the client's
terminal minus the sidebar: a 150x45 client gave a 124x44 area (`herdr-restart.sh`), a 160x45 client 134x44 with
rects such as `{x:34, y:22}` (exploration run). The rank of a rect among the tab's distinct `x` and `y` values did
not change.

**Conversion for #640.** `GridPos { row, col }` is 1-based and row-first. Herdr is (x, y), 0-based, in cells, and
column-first. Do not convert a `GridPos` to cell numbers. Read a pane's `GridPos` as:

- `col = 1 + index of rect.x in the sorted distinct x values of the tab's panes`,
- `row = 1 + index of rect.y in the sorted distinct y values`,

or, more robustly, walk `layout.export`: the root's chain of `down` splits gives the rows in order, and each row's
chain of `right` splits gives its columns. Both give the same answer for an aligned grid (the probe's `rank` and
`tree` columns). They disagree when it is not a grid, and that disagreement is the signal:

- **Unaligned rows** (each row split with different ratios): row 1 split at 0.5 and row 2 at 0.3 give distinct
  x `[0, 36, 60]`, and ranking puts row 1's second pane in column 3. The tree walk gives `r1c2` correctly.
  **Prefer the tree walk.**
- **A nested split** inside one cell makes the tab not a rows-of-columns tree. The tree walk reports
  "not a rows-of-columns tree" for those panes, and ranking shifts every lower row (`r2c1` became `r3c1`).
  The adapter should refuse to assign a `GridPos` there (a finding, not a guess).

**Fallback if no position is exposed.** This Herdr exposes rects and the tree, so the fallback is for a future
version that drops them, or for a layout that is not a grid. Derive the position from `layout.export` if that
remains. Otherwise record the `GridPos` only in the registry, as the position Holler asked for when it placed
the pane, and mark it unobserved, since I6 forbids inferring it. `profile create --from-current` and the `fleet`
import (#650) then report the pane as having no observed position rather than invent one.

## 7. Layout model: splits only, and the nesting rule

- **Splits only, no absolute cells.** A tab's layout is a binary tree. An inner node is a split with `direction`
  `right` (side by side) or `down` (stacked) and a `ratio`; a leaf is a pane. No call places a pane at a cell.
  `pane.split` accepts only `right` and `down`: `--direction left` exits 2, and a socket `"left"` is
  `invalid_request: unknown variant 'left', expected 'right' or 'down'`. The new pane is always the `second`
  child (right of or below the target). VERIFIED.
- **Ratio** is the **first** child's share: splitting a 120-wide pane `right` with `ratio 0.25` leaves the target
  30 wide and gives the new pane 90. It is stored as a 32-bit float (`0.3333` comes back as
  `0.33329999446868896` in events). VERIFIED.
- **The nesting rule: a split divides only the target pane's own cell.** After the 2x4 grid, `split r1c2 down`
  changed only `r1c2` (`{x:30,y:0,w:30,h:20}` became `{x:30,y:0,w:30,h:10}`) and added the new pane at
  `{x:30,y:10,w:30,h:10}`. No other rect changed. In the same way, splitting the top pane `right` left the full-width
  bottom pane alone. VERIFIED.
- **Consequence for `plan_splits`** (#640): to reach an R-row by C-column grid, split the first pane `down` R-1 times
  with ratios `1/R, 1/(R-1), ..., 1/2` (each time on the newest pane), then each row's first pane `right` with
  `1/C, 1/(C-1), ..., 1/2`. The probe built 2x4 exactly this way. Rows built first make the columns of different
  rows independent: they line up only because the ratios match. Building columns first would instead make the
  rows of different columns independent. Choose one order and keep it (the epic stores rows of columns:
  `r<row>c<col>`). To add a pane to an existing grid without moving healthy panes, split the cell's neighbour that
  the tree walk names, never `layout.apply` on the tab.
- **Close** gives the closed pane's space to its sibling subtree (closing `r2c4` made `r2c3` 60 wide). It can turn
  a regular grid into an irregular one.
- **`layout.apply`** builds a whole tree in one call. A pane node may carry `command` (argv), `cwd`, `env` and
  `label`. Into a new tab it is the only way to start an argv **without a shell** (VERIFIED: `process_info` shows
  `sleep 601` as the foreground process and `shell_pid == foreground_process_group_id`). Onto an existing
  `tab_id` it **replaced the tab** (new tab id, new pane ids, the input's `pane_id`s ignored, `sleep 601` killed).
  VERIFIED. `layout.set_split_ratio {tab_id|pane_id, path: [bool], ratio}` changes one ratio (SCHEMA).
- **Argv launch at a chosen place, without a shell** (VERIFIED, `herdr-ops.sh`): `layout.apply` a one-pane tree with
  the argv into a staging tab, then `pane.move {pane_id, destination: {type: "tab", tab_id, split: "right"|"down",
  target_pane_id, ratio}}`. The pane id and the process (pid) are unchanged, and the emptied staging tab closes by
  itself.

## 8. How the saved layout is restored

The server saves `session.json` (and timestamped copies under `session-snapshots/`) in the session directory. It
stores, per workspace, the label, cwd, the saved id counters (`public_pane_numbers`, `next_public_pane_number`),
and per tab the split tree and per pane `cwd`, `label` and, for argv panes, `launch_argv`. It does not store
metadata, agent state or scrollback (`[experimental] pane_history` is off by default, DOCS). VERIFIED by reading
the scratch file after `server stop`.

On restart (`herdr-restart.sh`):

- Restored: workspaces, tabs, tab labels, pane labels, the split tree and ratios, and pane ids. Rects follow the
  new area. Later ids continue from the saved counter. VERIFIED.
- **Not restored: the processes.** Every pane comes back as a new default shell (`argv: ["/bin/sh"]`) with a new
  `terminal_id`. A pane started from `layout.apply` with `command: ["sleep","700"]` comes back as a shell too, and
  `layout.export` then shows `command: null`, although `session.json` still holds `launch_argv`. All pane processes
  end when the server stops ("pid ... ended with the server"). VERIFIED.
- Not restored: `title`, `tokens`, `state_labels`, `display_agent`, workspace tokens, agent reports (`agent: null`,
  `agent_status: unknown`). VERIFIED.
- `[session] resume_agents_on_restore = true` (the default) resumes supported agents into their own sessions after
  a restart, but "requires official integrations that report session refs" (DOCS). UNVERIFIED: running an agent
  was out of scope. Holler should not rely on it; a restart is what the epic calls "a reboot that returns the
  layout as empty shells", and `holler profile apply` (#664) relaunches them.

## 9. What `agent_status` reports

`agent_status` is in `PaneInfo`, `AgentInfo`, `TabInfo` and `WorkspaceInfo` (the last two aggregate), with values
`idle | working | blocked | done | unknown` (SCHEMA). `pane.report_agent` accepts only `idle | working | blocked |
unknown`; `done` is derived by the server.

- A plain shell: `{"agent": null, "agent_status": "unknown"}` (the script prints the omitted `agent` as null). VERIFIED.
- An agent is known to Herdr in two ways: screen detection by bundled agent manifests (`opencode` is among the
  known kinds), or a report through `pane.report_agent {pane_id, source, agent, state}`. The spike used reports
  (no agent was run). VERIFIED: reported `working`, then `blocked`, then `idle` were each reflected in `pane.get`
  and `agent.list` (`state_change_seq` counts the changes). `pane.release_agent` returned the pane to `unknown`.
- `idle` and `done` both mean "ready for input"; the server reports `done` until the completion is "seen" (DOCS:
  `herdr --skill`). In the spike, a working-to-idle report showed `done` in `agent get`, and a blocked-to-idle
  report showed `idle`. VERIFIED, but the seen rule itself is DOCS.
- `pane.report_agent_session {agent_session_id}` was accepted (exit 0), but no `agent_session` appeared in
  `pane.get` while no real agent ran. UNVERIFIED: whether Herdr exposes the session id of a real OpenCode pane.
  If it does, it is a candidate source for SHOWN in the roster, but I6 still asks for OpenCode's own answer (#635).
- `agent explain ID --json` shows the detection rules evaluated against the pane's screen. VERIFIED.

For Holler, `agent_status` is Herdr's guess from the screen or from whoever reports. It is useful for display, but
the hub should take an OpenCode session's state from OpenCode (#635, #642), not from Herdr.

## 10. Events

`events.subscribe {subscriptions: [...]}` keeps the connection open and writes one JSON object per line. VERIFIED
(`herdr-ops.sh`):

```
{"event":"subscription_started","pane":null}
{"event":"pane_created","pane":"w1:p3"}
{"event":"layout_updated","pane":null}
{"event":"pane_created","pane":"w1:p4"}
{"event":"pane_closed","pane":"w1:p4"}
{"event":"pane_exited","pane":"w1:p3"}
{"event":"pane.agent_status_changed","data":{"agent":"opencode","agent_status":"working","pane_id":"w1:p1","workspace_id":"w1"}}
```

(The script reduces each line to its event name and pane.) Each event line is `{"event": <name>, "data":
{...}}`. Subscription types (SCHEMA): `workspace.created|updated|metadata_updated|renamed|moved|reordered|closed|focused`,
`worktree.created|opened|removed`, `tab.created|closed|focused|renamed|moved`,
`pane.created|closed|updated|focused|moved|exited|agent_detected|scroll_changed`,
`pane.output_matched {pane_id, source, match}`, `pane.agent_status_changed {pane_id}`, `layout.updated`.
`layout_updated` carries the whole tab layout with rects, so a subscriber does not need a follow-up snapshot.

Watch for a naming inconsistency within protocol 22: most event names use underscores (`pane_created`), but
`pane.agent_status_changed` arrives dotted. VERIFIED. A consumer must accept both forms.
`events.wait {match_event, timeout_ms}` is a one-shot wait. The binary says "events.wait currently supports pane
agent status matches" (BINARY). Not exercised.

## 11. The server on the hub's machine (decision 3), and remote attach

- **Local server, local panes: VERIFIED.** `herdr --session <s> server` runs headless with no client attached.
  Every pane process is a child of that server process (in an exploration run, a `layout.apply` argv pane's parent
  pid was the server's pid), so panes and their processes are local to the machine the server runs on. The API is a local
  Unix socket, owner-only. A TUI client attaching and detaching changes only the rect sizes (section 5). So the
  adapter (#640) can live on the hub's machine and talk to a local socket, as decision 3 says.
- **Ownership.** Stopping the server ends every pane process, and a restart brings back shells (section 8).
  The hub must treat a Herdr server restart as "every pane needs relaunch".
- **`herdr --remote <ssh-target> [--session <name>]`: UNVERIFIED.** The spike was not allowed to SSH anywhere.
  What the docs and binary say: it attaches a TUI client through SSH to a Herdr server on the target (DOCS). It
  can manage its own SSH config and control socket (`[remote] manage_ssh_config`, DOCS). Strings in the binary say
  it can download and install a Herdr binary on the remote, and may offer to stop the remote server to complete an
  update ("To complete the remote update, Herdr must stop the running remote server after installing", a `[y/N]`
  prompt) (BINARY). `herdr --machine <label> <command>` forwards API commands to a saved machine; `herdr machine
  add` "prepares the remote Herdr server" (DOCS).
- **INFERRED:** pane ids, the tree and `session.json` live on the server, and the client only renders, so an
  attach from another machine should not change them. The local attach and detach did not change them.

**What the operator still has to verify (for #655), and how.** Use a scratch session on the hub's machine, never
the live one:

1. On the hub's machine: `herdr --version`, then start `herdr --session spike636-remote server`, build a few
   panes, and record `herdr --session spike636-remote api snapshot | jq -c '[.result.snapshot.panes[] | {pane_id, tab_id, label}]'`
   and the tree of each tab (`layout.export`, see `herdr-restart.sh`'s `ids` function).
2. On the other machine: confirm the same `herdr --version`. A different version may trigger the remote install or
   update path; answer **No** to any prompt to stop a remote server. Then run
   `herdr --remote <hub-ssh-target> --session spike636-remote`.
3. While attached, and again after detaching (`prefix+q`), repeat step 1's records on the hub and diff them.
   Expected: identical ids, labels and tree; only rects change with the client's size.
4. Stop only that session: `herdr session stop spike636-remote`, then `herdr session delete spike636-remote`.

## 12. Recommendation

**Adapter (#640): talk to the socket directly; do not shell out to the CLI.**

1. Several operations exist only on the socket: `layout.export`, `layout.apply`, `events.subscribe`, `pane.move`
   with a split destination, and `pane.send_input`. Positions need `layout.export`, and argv launch without a
   shell (B2) needs `layout.apply` plus `pane.move`.
2. The socket's input and output are typed and versioned by a JSON Schema the adapter can check against.
   Errors are `{code, message}`. The CLI's output is not uniform (`pane read` is plain text, several commands
   print nothing).
3. The protocol is simple: connect, write one JSON line, read one JSON line, close. Subscriptions are one
   long-lived connection. Per call it costs no process spawn.
4. The socket path is the only configuration: `<config-home>/herdr/sessions/<name>/herdr.sock` or the default.
   Take it from configuration or `herdr session list`.

Mapping the provisional `HerdrPort`:

- `ensure_pane`: look the pane up in the snapshot; to create one, place it by splits as `plan_splits` decides
  (section 7). To start an argv, `layout.apply` it into a staging tab and `pane.move` it into the target split.
  Never send a shell command line; never `layout.apply` onto an existing tab.
- `send_text`, `send_keys`, `read`, `close`: `pane.send_text`, `pane.send_keys`, `pane.read`
  (`recent_unwrapped` for transcripts), `pane.close`.
- `snapshot`: `session.snapshot` plus `layout.export` per tab, with `GridPos` derived by the tree walk (section 6),
  and `pane.process_info` for what each pane runs.
- `version`: `ping` (section 13). Watch with `events.subscribe` (`pane.exited`, `pane.closed`, `layout.updated`)
  rather than polling.

The adapter must never call `server.stop`, `server.live_handoff`, `layout.apply` with a `tab_id`, or the
`integration.*` and `plugin.*` mutators.

**Display plugin (#651): it may show; it must not change panes.**

1. It may show what `pane.report_metadata` and `workspace.report_metadata` carry: a pane `title`, a
   `display_agent`, `state_labels` and up to 16 `tokens` per report (names `^[A-Za-z0-9_-]{1,32}$`, 32 kept per
   pane; these limits are SCHEMA). The sidebar renders them where the operator's `[ui.sidebar.agents] rows` and `[ui.sidebar.spaces] rows`
   name them as `$token`. Examples: the pane's `r2c1` position, SHOWN versus DRIVEN, the profile name.
2. It must not change panes, and Herdr will not stop it. VERIFIED: a plugin action's process receives
   `HERDR_SOCKET_PATH` (with `HERDR_BIN_PATH`, `HERDR_SESSION`, `HERDR_PANE_ID`, `HERDR_TAB_ID`,
   `HERDR_WORKSPACE_ID` and the `HERDR_PLUGIN_*` set), and its own `herdr pane split` created a pane. So "display
   only" must be Holler's rule, enforced in #651 by code and test: the plugin calls only the two
   `report_metadata` methods and read-only methods, and a test fails if its source names any other mutating
   method.
3. Its metadata is lost on a server restart, and `ttl_ms` can expire it. The plugin (or the hub) re-reports on
   `layout.updated`, on `pane.created` and after a restart. Use one `source` id (for example `holler`) so its
   tokens can be cleared together.
4. Agent rows appear in the sidebar only for panes Herdr considers to have an agent (detected or reported). For
   a pane without one, only the workspace row and the pane's `title` (shown by the `pane` built-in) are available.

## 13. API versions

**How to read the version** (VERIFIED):

- From the running server: `ping` returns `{"type":"pong","version":"0.9.1-preview.2026-09-21-0ff0f27e2226",
  "protocol":22,"capabilities":{"live_handoff":true,"detached_server_daemon":false,"endpoint_protocol_generation":1,"surface_interest":true,"health_check":true}}`.
  `herdr status server --json` reports the same fields, plus `compatible`, `endpoint_compatible` and
  `restart_needed`, judged against the CLI binary.
- From the snapshot: `session.snapshot.version` and `.protocol`.
- From a binary, offline: `herdr api schema` gives `protocol` and `schema_version` of the bundled schema (the
  client, not the server).

The integer `protocol` is the compatibility key. The version string carries a channel and build date and changes
more often. Use `protocol` (and `schema_version`) to decide support, and record the version string as
`host.herdr_api_version` for display.

**Differences between versions: UNVERIFIED.** One build is installed on the test machine, the live server runs
the same binary, and installing another (`herdr update`) would replace the binary that the live server uses, so
no second version was compared. Facts that hold within protocol 22 and that the adapter must tolerate anyway
(VERIFIED):

- Optional fields are omitted rather than null (`agent`, `label`, `title`, `tokens`, `tty`).
- Event names mix forms (`pane_created` versus `pane.agent_status_changed`).
- An unknown method fails as `invalid_request` with "unknown variant", which is how a call missing from an older
  server will look.
- Plugin manifests need `min_herdr_version`, and an unknown event-hook name is a warning, not an error
  (`unknown event 'pane_created'`; the valid form is `pane.created`).

**How the operator (or #640) can list the differences** when a second build is at hand, without touching any
server: put the other binary in a scratch directory (not on `PATH`), then

```
<other-herdr> api schema --json --output other.json
herdr api schema --json --output current.json
diff <(jq -r '[.schemas.request.oneOf[].properties.method.const] | sort[]' current.json) \
     <(jq -r '[.schemas.request.oneOf[].properties.method.const] | sort[]' other.json)
diff <(jq -S '.schemas.success_response["$defs"]' current.json) <(jq -S '.schemas.success_response["$defs"]' other.json)
```

**Which versions #640 should support:** exactly **protocol 22 with schema version 1** (Herdr 0.9.1). INFERRED to
be what the live fleet runs: the live server's executable is the same file as the tested binary, which predates
the server's start; the live server itself was not queried. Read `ping` on connect and record the version string. Refuse any other protocol with
`herdr-version-unsupported`, and have the message name "Herdr protocol 22 (0.9.1)". Widen the set only after
the schema diff above has been run against the new build and the adapter's tests pass on it.

## 14. Gotchas found on the way

- **A trailing `--help` is data for some CLI verbs.** `herdr pane rename <id> --help` renamed the pane to `--help`
  in the scratch session. Never probe a mutating CLI verb this way (the Herdr skill also warns against omitting
  arguments). VERIFIED.
- Some CLI commands print nothing on success (`pane run`, `send-text`, `send-keys`, `report-*`), and `pane read`
  prints plain text. VERIFIED.
- A shell that exits closes its pane (`pane_exited`); `pane_closed` is sent only for an explicit close. VERIFIED.
- `layout.apply` with a `tab_id` replaces the tab (section 7). VERIFIED.
- The plugin event-hook names are the dotted subscription names (`pane.created`), not the underscored event
  names. VERIFIED.
- Without `[update] version_check = false` and `manifest_check = false`, a server contacts herdr.dev in the
  background (DOCS); the scratch config turns both off.

## 15. Everything left UNVERIFIED

- `herdr --remote` and `herdr --machine`: attaching from another machine, and whether ids and the saved layout
  survive it. The steps to check are in section 11.
- The differences between Herdr versions. Only one build was available; the method to list them is in section 13.
- `resume_agents_on_restore`, and whether Herdr reports a real OpenCode pane's session id (`agent_session`).
  Both need a real agent in a scratch pane.
- Screen-based agent detection for OpenCode (only reported states were tested).
- `events.wait`, `tab.create`, `layout.set_split_ratio`, `pane.wait_for_output` over the socket (the CLI
  `wait-output` was used), plugin panes (`plugin.pane.open` with `placement` `overlay|popup|split|tab|zoomed`),
  and plugin `startup` and `link_handlers` entries. These were read in the schema only.
- Behaviour on macOS (`shell_mode = "auto"` uses login shells there, DOCS). Everything here ran on Linux.
