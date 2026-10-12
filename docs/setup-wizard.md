# Building a Herdr workspace of Holler registry panes

The setup wizard builds one [Herdr](https://herdr.dev) workspace whose every pane is a Holler
**registry pane**: a pane that Holler launches, records and keeps in shape with its pane control
verbs (`holler pane` and `holler profile`; see [the operator guide](pane-control.md) and
[ADR-0021](adr/ADR-0021.md)). A pane's harness is OpenCode or Claude Code, one or more of the panes are
orchestrators that drive the others through Holler, and a Holler hub and one Holler body on the same
machine carry the prompts. This page holds the operational rules of that setup and the reasoning
behind each step of the wizard that builds it, a Claude Code skill (see "Automated setup").

Holler drives the panes; Herdr shows them. The wizard never drives a Herdr pane itself: every pane is
built, changed and read through Holler.

## Shape

```
This machine
├─ Herdr server (started by herdr.sh server-start; the instance's own session when it has one)
│  └─ workspace <workspace>: one registry pane per [[pane]], at its cell of `layout`
│     ├─ r1c1  OpenCode, the orchestrator   (its server on 127.0.0.1:47001)
│     ├─ r2c1  OpenCode                     (its server on 127.0.0.1:47002)
│     └─ r1c2  Claude Code                  (one process, no port)
├─ tmux: one session per pane, named as the pane, holding its harness
├─ HOLLER_STATE_DIR=<state_dir> holler hub serve --listen 127.0.0.1:<hub_port>
│     (the pane registry and the profile live in the hub's state directory)
└─ HOLLER_STATE_DIR=<state_dir> holler body run --config <body_config>
      (one attach row per OpenCode pane: its port and session of record, from the registry)
```

The ports in the diagram are the defaults (hub 41807, OpenCode panes from 47001); an `[instance]`
table changes them (see "Running an instance beside another").

Everything is on one machine: the hub binds loopback only and the body dials it there
(`ws://127.0.0.1:<hub_port>`), Holler reaches Herdr over its local socket, and the body reaches each
OpenCode pane's server over loopback HTTP. The config decides how many panes there are, which
harness each runs and which are orchestrators; none of that is hardcoded. The previous wizard built
something else — OpenCode backends on other machines, each shown here in a pane through an attached
TUI — and that is no longer built: registry panes are local to the hub (epic #633). See "If you
built a workspace with the previous wizard".

## The config file

One TOML file is the source of truth for a run — every pane, its harness and role, and the whole
layout. The wizard resolves it in this order, using whichever it finds first (no merging across
tiers):

1. An explicit `--config <path>` passed to the wizard's invocation.
2. `./sessions.toml` — a project-local config, when the wizard is run from a project directory.
3. `~/.holler/herdr/setup/sessions.toml` — the global fallback.

An existing global `sessions.toml` skips the interview. To start clean, move it aside; the wizard
never deletes it. When no config is found, the wizard's message names the path it looked for.

When you do not name the panes, the default names are the grid cells, row then column, like a
spreadsheet cell: `r1c1`, `r1c2`, `r1c3`, `r1c4` for the first row, `r2c1` for the first cell of the
second row, and so on. With a pane count and no grid, the wizard proposes a grid by one rule: two
rows for more than four panes, up to 8 panes, and above 8 panes the squarest grid. It asks one confirm. Your own names always win. A count that does not
fill the last row leaves its short columns on the right.

An optional top-level `workspace = "<name>"` (before every table header) names the Herdr workspace:
1 to 40 characters, a letter or digit first, then letters, digits, spaces, `.`, `_` or `-`
(`instance.sh` refuses anything else, naming the key). Stage 1 suggests two lower-case words joined
with a hyphen, such as `herald-relay`. With no `workspace` the workspace is `holler`. The same name is
the profile the wizard builds: `panes.sh grid` prints it, so the agent never chooses it.

Shape:

```toml
# The defaults every pane takes unless it sets its own (Stage 1 proposes them).
model = "provider/model-name"
effort = "medium"
ctx_soft = 800000
ctx_hard = 900000

# LAYOUT — a nested array: outer = columns left to right, inner = each column's panes top to
# bottom. Every [[pane]] name appears exactly once, and no column is taller than the column to
# its left. Must come before any table header below (a bare key after a [table]/[[array]] header
# silently becomes a member of that table in TOML, not a top-level key), and on one line:
# instance.sh reads a line that starts with [ as a table header, so panes.sh check refuses a
# layout spread over several lines, naming layout.
layout = [["r1c1", "r2c1"], ["r1c2"]]

workspace = "herald-relay"

[[pane]]
name = "r1c1"
dir = "~/Projects/demo"
role = "orchestrator"                   # at least one pane is an orchestrator

[[pane]]
name = "r2c1"
dir = "~/Projects/demo"                 # harness "opencode" and role "agent" are the defaults

[[pane]]
name = "r1c2"
dir = "~/Projects/demo"
harness = "claude"
model = "anthropic/claude-sonnet-5-5"   # a Claude Code pane sets its own model
```

Each `[[pane]]` has a `name` (lower-case letters and digits, `-` between them, 1 to 32 characters:
it is also the pane's tmux session name), a `dir` (absolute, or starting with `~/`), a `harness`
(`opencode`, the default, or `claude`) and a `role` (`agent`, the default, or `orchestrator`). It may
set `agent` (the OpenCode agent its messages run as; refused on a Claude Code pane, whose launch answers
`agent-unsupported`), `backend_port` (an OpenCode pane's own port; a Claude Code pane has none) and
its own `model`, `effort`, `ctx_soft` and `ctx_hard`. A new pane spec needs all four of those, so
the top level holds them as the defaults; a Claude Code pane never takes the top-level `model`,
which is read from OpenCode's configuration and names an OpenCode model. At least one pane is an
orchestrator, and several are allowed (the MO pattern): an orchestrator is a role on a pane of
either harness, not a command of its own. Which panes an orchestrator drives is a matter of how it
is briefed, not something `layout` encodes.

**The staircase rule.** `layout` keeps its meaning — columns of names, each top to bottom — and
gains one rule: no column may be taller than the column to its left. Herdr makes every pane by
splitting one, and adds a row only below a row of one pane, so that is exactly the set of grids
`profile apply` can build from an empty workspace (its creation order is in
[pane-control.md section 6](pane-control.md#6-profiles)). Every rectangular default grid is a
staircase; `[["r1c1"], ["r1c2", "r2c2"]]` (one pane, then two stacked beside it) is not, while the
same panes as `[["r1c2", "r2c2"], ["r1c1"]]` are. `panes.sh check` refuses a taller column, naming
`layout` and the column.

**The retired tables and keys.** The previous wizard's config had `[[orchestrator]]` tables (with a
free-form `cmd`) and `[[session]]` tables, and the keys `endpoint`, `mode`, `session_id`,
`remote_host`, `remote_tailnet_host` and `cmd`. `panes.sh check` refuses each by name (exit 1, each
message naming the key and saying the wizard no longer builds attach-mode or remote sessions), and
Stage 1 offers a conversion as text (see "If you built a workspace with the previous wizard"); it
never rewrites your file. `hub_host` is accepted and ignored, with one warning line: the hub listens
on loopback only.

**The master file is not the body's `--config`.** Holler's config parser
(`crates/holler-body/src/config.rs`) denies unknown keys, so a typo like `harnes` is an error, and it
does not know the `[instance]` table, a pane's `backend_port`, a `[[pane]]` table or the top-level
`workspace` key: a master file that has any of them makes `holler body run` fail. The wizard
therefore writes a **derived body config**, and the body reads only that: it drops the `[instance]`
table, every `[[pane]]` table with its `backend_port`, and the top-level `workspace` key with the
other top-level keys, and holds one `[[session]]` row per OpenCode pane, whose `endpoint` and
`session_id` the wizard reads from the registry (`holler pane get NAME --format=json`: the pane's
port and its session of record), never from the master file. Other tools can keep their own data in
the master file under a top-level `[ext.<namespace>]` table, which the wizard ignores. The derived
config is `<state_dir>/herdr/setup/<prefix>-sessions.toml`; the master file (whichever of the three
sources above was actually loaded) keeps every field for the next run.

## How the wizard builds panes

Every pane is built, changed and read through Holler's pane control; the wizard runs no Herdr pane or
workspace command of its own (`herdr.sh run` refuses every word but `status`, `session list` and
`workspace list`). The design, in the order the stages run it:

1. **The hub first** (Stage 4). Every pane and profile verb needs the hub — with none it answers
   `unavailable` — so the hub starts before anything of the registry is read or written: on loopback,
   with no advertised address, because nothing on another machine dials it.
2. **The Herdr server** (Stage 5), through `herdr.sh server-start`, recorded in the ledger. A
   `holler pane doctor` pass then prints Herdr's version first: the one read that proves Holler's
   adapter reaches this instance's Herdr session.
3. **The profile** (Stage 6). `panes.sh plan` prints every call, in order, so the agent orders
   nothing: `holler profile create NAME`, then one spec-only launch per pane in layout order
   (`holler pane launch PANE --profile NAME --spec-only` with the pane's workspace, cell, project,
   model, effort, role, ceilings and port policy: `fixed:PORT` for an OpenCode pane, `none` and
   `--harness claude` for a Claude Code pane). A spec-only launch writes the pane's spec into the
   profile and touches nothing live. The profile's name is the workspace's.
4. **The dry run, then `profile apply` alone** (Stage 7). The dry run prints the steps apply would
   take; the apply then creates every pane of the grid on its own, in its own creation order, adopts
   a healthy pane that matches its spec and relaunches an unhealthy agent. It never kills a healthy
   pane and never relaunches an orchestrator. A rerun is the same sequence without `profile create`.
   The wizard launches no pane outside that one apply. Apply's creation order, its exit codes and
   what it prints (0, 1 for a failed step, 3 for a refused one) are in
   [pane-control.md section 6](pane-control.md#6-profiles); the wizard stops and
   asks on a refused, failed or reported step, and gives the apply a timeout above its expected
   duration (up to 10 s to observe, plus about 15 s per create).
5. **One body** (Stage 8) on the same machine, joined over `ws://127.0.0.1:<hub_port>`, whose attach
   rows are derived from the registry. A Claude Code pane has no body row until
   [#850](https://github.com/Performant-Labs/holler/issues/850), so it has no roster row either, and
   a `say` round trip reaches the OpenCode panes only.
6. **The registry check** (Stage 9): `holler pane doctor`, `holler pane list` and
   `holler profile show NAME`, gated on any finding except `unregistered-herdr-pane`. DRIVEN prints
   `-` until [#839](https://github.com/Performant-Labs/holler/issues/839).

Where the rest is documented, not copied here: every verb in
[pane-control.md section 3](pane-control.md#3-the-verbs), profiles and `profile apply` in
[section 6](pane-control.md#6-profiles), grid notation in [section 7](pane-control.md#7-grid-notation),
and the health probe, the argv rule and Herdr's version in
[section 8](pane-control.md#8-probe-argv-layout-herdr-version).

What a pane's harness can and cannot do yet is the guide's too: an OpenCode pane's model is recorded,
not applied, until [#829](https://github.com/Performant-Labs/holler/issues/829) (its server runs what
its OpenCode configuration selects); a Claude Code pane's model and effort are applied, and its login
is the one Claude Code itself holds — the wizard never reads, copies or sets it, and a
`claude-login-missing` finding is answered by logging in once by hand in a plain terminal.

**Not `herdr-mirror`.** `herdr-mirror` (a plugin for live-streaming a remote *workspace*, not just a
session) is a different, heavier mechanism with real documented failure modes — including the mirror
daemon's own reconciliation killing the real processes it was only supposed to be showing, not just
their local viewer panes. Registry panes never touch it.

## Never kill a process you didn't identify first

**Before stopping, killing, or reusing any process, port, or credential — whether it's this
pattern's own hub/body or something else entirely — identify what it actually is first.** If it
wasn't started by *this* run, treat it as someone's live production until proven otherwise, and get
explicit, named confirmation ("kill PID 822098, `holler body run`") before touching it. Never kill
something just to free up a port or "be able to run a test" — that phrasing is the failure mode
itself, not a justification for it.

This isn't hypothetical: a real incident did exactly this. An agent testing an earlier version of
this setup against a real host found an existing `holler body run` process using a resource it
wanted, killed it "to be able to run the test," and moved on — without checking what that process
actually was. It turned out to be an unrelated, already-live Herdr session (a different hub, on a
different machine, serving real work), which went down with no warning and no way for anyone to have
known in advance it was safe to kill. It was recoverable in this case (an attach-mode session
survives independently of the Holler body wrapping it, so rejoining a fresh body restored it without
data loss), but that was luck: a body killed mid-turn would not have been recoverable the same way.

`ps -ef` and reading the process's own config/cwd before acting on it costs seconds. Guessing, or
asking forgiveness after, does not undo a killed production process. The wizard's ledger (see
"Instance state and the ledger") is what makes this mechanical.

## Label every token by its instance

**A token's label names the hub and the instance its body belongs to**, so a roster entry, a
`hub token list` row, or a process someone inspects later says which instance it is on sight. A plain
`<host>-body` label names only one side of a pairing: two hubs paired to the same machine end up with
tokens and bodies that look identical to a human — or another agent — reading a process list or a
token store later. That ambiguity is exactly what made the incident above possible.

The wizard runs one body per instance, on the hub's own machine, and labels its token
`<prefix>-local` (the instance's prefix). Before minting, it checks what's already there
(`HOLLER_STATE_DIR=<state_dir> holler hub token list`), and a new label must be visibly distinct from
anything already there, not merely a different string.

**Mechanical gotchas worth knowing before minting, confirmed live 2026-09-23:**

- **A label stays taken until its token's record is deleted**
  ([#454](https://github.com/Performant-Labs/holler/issues/454)). `revoke` deliberately keeps
  the record (it's an audit trail), so a revoked token still holds its label; `delete` removes
  an `unused` or `revoked` token's record and frees the label. A mint over a taken label fails
  with `label "..." already in use by <state> token <token_id>`, followed by the exact commands
  that free it. If you're confident the old pairing is genuinely done (per the "never kill"
  section above), revoke its token if it is still `bound`
  (`HOLLER_STATE_DIR=<state_dir> holler hub token revoke <token_id>`), delete it
  (`HOLLER_STATE_DIR=<state_dir> holler hub token delete <token_id>`), and
  mint the label again; the new token has a new id, so the body joins with the new join line.
  On holler v0.3.0 and earlier nothing frees a label (`delete` only invalidated the secret):
  there, append a counter and mint `<prefix>-local-2`, `-3`, and so on.
- **A minted token's join secret is valid for 24 hours by default (`--ttl`), even though releases before v0.3.0
  printed its `expires` a day early.** A freshly minted token showed `expires` equal to "now", which
  looked like a zero-length window; it was a display bug in `format_epoch` (fixed in v0.3.0), and the
  real expiry — visible in `--json` — was always `now + ttl`. There is no rush between `mint` and
  `body join`; only a real `token expired`/`invalid token` error *from `body join` itself* means the
  token actually lapsed. (An earlier version of this page called it a "redeem-immediately window";
  that was wrong.)
- **Once a body has joined, its token does not expire**
  ([#453](https://github.com/Performant-Labs/holler/issues/453)). `expires` bounds only the join
  window, so `hub token list` shows `-` in EXPIRES for a bound token, and a long-running body keeps
  authenticating on every reconnect. `HOLLER_STATE_DIR=<state_dir> holler hub token revoke <token_id>`
  is what ends it. Releases
  before #453 refused a joined body's reconnect once `expires` had passed (hub log reason
  `token_expired`, now retired); against an upgraded hub such a body authenticates again with no new
  join. Before upgrading, revoke any such token whose body must stay cut off: find them with
  `HOLLER_STATE_DIR=<state_dir> holler hub token list --json` (`bound` rows with a past `expires`; the text output shows `-`).
- **A real Homebrew/Linuxbrew-installed `herdr` or `holler` can still report `not found` even
  in a login shell** — some machines never add the brew prefix's `bin` dir to `$PATH` at all,
  not just a login-vs-non-login gap. Before concluding either binary is genuinely missing, check
  `~/.local/bin/<binary>`, `/home/linuxbrew/.linuxbrew/bin/<binary>`, and
  `/opt/homebrew/bin/<binary>` directly; if one resolves, use that absolute path for the rest of
  the run (`HERDR_BIN` for Herdr, `WIZARD_HOLLER_BIN` for the Holler calls `herdr.sh holler` runs)
  rather than fixing the shell profile as part of setup.
- **A restarted Herdr server does not start blank — it restores its saved session, including
  resuming agents.** After killing a server and starting a fresh one, all the previous run's panes
  were already there and an orchestrator's Claude conversation had auto-resumed
  (`claude --resume <id>`). Those restored panes have no record: Holler shows them as unmanaged
  (the second table of `holler pane list`, and `unregistered-herdr-pane` in a doctor pass), and no
  Holler verb adopts them. The wizard never reuses, splits or closes them: Stage 5 lists them after
  the server starts, and Stage 7 builds the wizard's own workspace beside them, with your yes. Stage 3
  says this in the plan when `herdr session list` shows a stopped session of the instance's name,
  names the new workspace and lists the restored panes beside it, and the "Proceed with this plan?" yes covers creating it (a "no" means stop). A restored
  workspace that already has the config's `workspace` name is refused (Holler would take it for
  the wizard's): pick another name.
- **Stage 7 builds the grid with `profile apply` alone.** The panes are created in apply's own
  creation order, which is the order Herdr can build in, so the wizard splits nothing and labels
  nothing: a pane's name is its record's, and its cell is what `holler pane list` prints in its POS
  column. Stage 5's status check reads Herdr 0.8.x's `status: not running` / `status: running` line
  under `server:` and still accepts the older `server: not running`.
- **A resumed orchestrator can't prove the `AGENTS.md` briefing works.** It may already know about
  Holler from its own past turns. A pane the run just created is a fresh session; on a rerun that
  adopted the orchestrator, the wizard gives it a fresh one first, with your yes, since it drops the
  conversation: it runs `holler pane reset PANE --as-operator` through its door (the orchestrator's
  pane changes only as the operator's deliberate act; without the flag the reset is refused as
  `orchestrator-pane`). An OpenCode orchestrator then has a new session of record, so the wizard
  derives the body config again and restarts the body, and its old session stays on its server,
  where a doctor pass reports it as a `stray-session` (no Holler verb deletes a session). A Claude
  Code orchestrator is relaunched on a fresh session id: it has no body row to derive again and no
  server to keep a stray session. Then ask it something with no Holler context, like "Send a hello
  to r2c1" — it should find the roster, resolve the namespaced session name, and get a real reply.

## Running an instance beside another

By default the wizard builds the only Holler instance: hub port 41807, Holler's default state
directory and an unnamed Herdr server. To build a second instance next to a running one, add an
`[instance]` table to `sessions.toml`. It is a table header, so it goes after the bare top-level keys
(`layout`, `workspace` and the defaults) and before every `[[pane]]`. Leave it out and nothing
changes.

| Key | Type | Default | Rule |
|---|---|---|---|
| `name` | string | `default` | `^[a-z][a-z0-9-]{0,23}$` |
| `prefix` | string | the value of `name` | prefixes every log, config and process name; same pattern as `name` |
| `hub_port` | integer | 41807 | 1024 to 65535 |
| `serve_https_port` | integer | 443 | 1 to 65535; still validated, and unused since the hub is served on loopback only |
| `state_dir` | string | empty (Holler's own default) | absolute path or one starting with `~/` |
| `herdr_session` | string | empty (no named session) | same pattern as `name` |
| `backend_port_base` | integer | 47001 | OpenCode pane `i` (0-based, in layout order) listens on `base + i` unless it sets its own `backend_port` |

A non-default instance (any key differing from its default) must set `name`, `state_dir` and
`herdr_session`. It also may not use the first instance's ports: `instance.sh` refuses a non-default
instance with a `hub_port` of 41807 or a `serve_https_port` of 443, and `panes.sh` refuses OpenCode
ports in 47001 to 47001 plus the OpenCode pane count unless every OpenCode pane sets its own
`backend_port` (exit 1, one line naming the key). Stage 1 runs
`agent-skills/herdr-setup/lib/instance.sh <config>` and `panes.sh check <config>`, which refuse a
malformed value or a missing required key naming the key, a `state_dir` that is the default state
directory, and two panes that resolve to one port, naming both. A non-default instance is driven
from a terminal outside any Herdr pane (see "Herdr session"), and its panes run on its own tmux
server (`<state_dir>/tmux.sock`), so two instances' `r1c1` never meet in one tmux server.

```toml
layout = [["r1c1", "r2c1"], ["r1c2"]]
workspace = "second-relay"

[instance]
name = "second"
hub_port = 41808
serve_https_port = 8443
state_dir = "~/.holler-second"
herdr_session = "second"
backend_port_base = 47101
```

### Choosing the Herdr session

When Stage 1 builds a new config, it asks one question after the panes and before the workspace
name: which Herdr session to build in. Herdr has no session rename, so the choice is made when the
workspace is first built. An existing config is not re-asked; Stage 1's report and Stage 3's plan
state the session it targets.

- **The default, unnamed session** (recommended when there is no reason to separate this
  workspace from other Herdr work). It writes no `[instance]` table and nothing changes: the
  wizard builds in Herdr's unnamed `default` server. The plan says "Herdr session: default
  (unnamed)".
- **A new named session.** The wizard suggests a name (pattern `^[a-z][a-z0-9-]{0,23}$`, not
  `default`) and refuses one already in `herdr session list`, running or stopped, naming that
  session and offering another name or the default: it never builds in an existing session. For
  a free name it proposes a full second instance, written after the bare top-level keys and before
  `[[pane]]`: `name` and `herdr_session` set to the name, `state_dir = "~/.holler-<name>"`,
  `hub_port = 41808`, `serve_https_port = 8443` and `backend_port_base = 47101`. The ports are
  defaults only; Stage 3's collision preflight refuses a taken one. The file is shown, and you
  confirm or edit it before `instance.sh` validates it. The plan says "Herdr session: `<name>`
  (new, own state directory and ports)".

A named session is driven from a terminal outside any Herdr pane (see "Herdr session"), and
`holler` calls in your own shells need `HOLLER_STATE_DIR=<state_dir>` (for example
`HOLLER_STATE_DIR=$HOME/.holler-work holler hub status`); without it they reach the default
instance. A `holler pane` or `holler profile` call needs more than that: it goes through the
wizard's door (see "Basic Herdr commands").

## Instance state and the ledger

Stages 4 to 8 start the hub, the Herdr server and the body on the instance's own ports and in its
own state directory (the `[instance]` table of `sessions.toml`; an absent table means today's
defaults). Stage 1 runs `instance.sh` for the instance and `panes.sh` for the panes: `panes.sh grid`
prints every pane's cell and each OpenCode pane's port, and every later stage uses those ports and
never recomputes one. The hub listens on `hub_port`, on loopback only, and the body on the same
machine joins it there. The hub, the body and every pane call run with `HOLLER_STATE_DIR` set to the
instance's `state_dir`, **resolved to an absolute path once** (a `~/` value is expanded by the shell;
with no `state_dir` the one spelling of the default is `$HOME/.holler`, resolved). A second instance
beside a running one therefore shares no token store, pepper, roster, pane registry, profile or
config file with it. Because the body refuses the wizard's keys, it never reads the master file: it
reads a derived config built from the registry (see "The config file").

The agent runs every command in a fresh shell, so the skill carries each value (state directory,
ports, pids, the Herdr settings) as a literal in the command itself and never relies on a
variable surviving to the next command. A command that lost its `HOLLER_STATE_DIR` would reach
the default instance's hub; the skill therefore sets it inline on every `holler hub`, `holler
roster`, `holler say` and `ledger.sh` call, and every pane and profile call goes through
`herdr.sh holler`, which refuses to run without it (see "Herdr session").

Every process the wizard starts is recorded in `<state_dir>/herdr/setup/wizard-ledger.toml` (mode
0600, written atomically; with no `state_dir`, in Holler's default state directory). One
`[[process]]` table per process: `pid`, `started` (`LC_ALL=C ps -o lstart= -p <pid>`), `cmd`
(`ps -o command= -p <pid>`), `role` (`hub`, `herdr` and `body` for this wizard; `backend` and `serve`
rows come from the previous wizard), `stage` (4 to 9) and `session`. A row is **live** while a
process with that pid exists and its start time and command still match; a pid that was reused by
another program is **stale**, and a process in no ledger is **foreign**. The wizard signals only live
rows. On a rerun "reused" means exactly one thing, a live row in this instance's ledger whose command
still matches; it starts and records a new process when the old one died, and never adopts one it did
not record. `agent-skills/herdr-setup/lib/ledger.sh` (`record`, `list`, `owns <pid>`) maintains the
file and reads the state directory from `HOLLER_STATE_DIR`, which every call must set, the default
instance's included (an unset value is refused, exit 64, so a lost variable cannot reach another
instance's ledger). Each start command prints the pid of the program itself (`{ nohup ... & echo $!; }`,
because `cmd && nohup X & echo $!` can print the pid of a subshell on some login shells), and after
recording the agent confirms with `list` that the recorded `cmd` starts with the program. The Herdr
server's pid is the first line `herdr.sh server-start` prints. **The panes are not ledger processes**:
Holler launched them and its registry records them, so they stop through `holler pane close` (see
"Stopping and tearing down").

**The first instance has no ledger, and must never be given one by hand.** The Holler hub, bodies,
backends and Herdr that were running before the wizard first built a second instance were not
started by the wizard, so no ledger records them: they are foreign to every run, always reported
and never signalled. Do not "fix" that by recording their pids with `ledger.sh record`: a
hand-written row would make the wizard willing to stop the first instance, which is the one thing
this design exists to prevent. (Once the wizard itself builds an instance with the default state
directory, that instance does get its own ledger, because the wizard started its processes.)

A setup built before ledgers existed (by an earlier version of the wizard, or by hand) is
refused as foreign for the same reason: nothing records its processes, so the wizard reports them
and never stops them. See "The first instance has no ledger" above.

**The `AGENTS.md` briefing.** If the operator says yes in Stage 3, Stage 7 adds a `## Holler
self-status` section to each orchestrator's `AGENTS.md`, before the apply launches the orchestrators
(an agent reads the file when it starts, and nothing is ever typed into a pane). It is offered when
the grid has a pane besides the orchestrators. The text names no orchestrator, because orchestrators
that share a `dir` share one file. It never writes the instance's absolute `HOLLER_STATE_DIR` into the
file (the file may be shared with another instance's orchestrator, and an "already has this section"
rule would then leave the wrong path in place). Instead it says the pane's environment already sets
`HOLLER_STATE_DIR` and the orchestrator must never change or unset it (every pane starts with it, from
the tmux server `herdr.sh holler` started). When the section has to be instance-specific, it is
headed `## Holler self-status (instance <name>)`, and the wizard stops and asks when another
instance's section already exists.

The logs of the processes Stages 4 to 9 start go under the instance's state directory too,
`<state_dir>/logs/<prefix>-...` (`$HOME/.holler/logs` for the default), never `/tmp`, which is
RAM-backed on the target hosts and must not be filled.

## Stopping and tearing down

The wizard keeps a per-instance ledger of every process it starts (`<state_dir>/herdr/setup/wizard-ledger.toml`,
one `[[process]]` with pid, start time, command, role, stage and session). A process is the wizard's
to stop only if that ledger recorded it and its current start time and command still match.

Every file the wizard keeps for itself lives in `<state_dir>/herdr/setup/` (for the default instance,
`~/.holler/herdr/setup/`): the ledger (`<state_dir>/herdr/setup/wizard-ledger.toml`) and the derived
body config. The global master config `sessions.toml` is always `~/.holler/herdr/setup/sessions.toml`, even for a non-default instance. Logs stay in `<state_dir>/logs` and scratch files in `<state_dir>/wizard-scratch`.
**After upgrading**, a rerun finds no ledger at the new path. A ledger at the old path
(`<state_dir>/wizard-ledger.toml`) is not read, and there is no automatic fallback or migration: if its
processes are still running they appear as foreign and are never signalled. Move the file into
`<state_dir>/herdr/setup/` by hand to keep ownership.

- **Stale** (the pid was recorded but now belongs to another command): never signalled, reported.
- **Foreign** (in no ledger, for example something else holding a planned port): never signalled;
  the wizard reports the pid, owner and command, then stops and asks.
- **Live**: stopped with SIGTERM, nothing stronger. If it is still running after the grace
  period the wizard says so and asks.

`agent-skills/herdr-setup/lib/stop-owned.sh` does this (`stop`, `restart`, `check-port`,
`teardown`; exit 0 done, 1 stale, 2 foreign, 3 still running, 4 usage). `restart` stops the
recorded process and prints `RESTART-CMD <recorded command>`; it does not run it, and that
recorded command has no environment, no `nohup` and no log redirect (run alone it would use the
default state directory). It only identifies the entry: the wizard re-runs the stage's own start
command (with `HOLLER_STATE_DIR`, `nohup` and the log path) and records the new pid.

**A teardown closes the panes through Holler first, then stops the ledger's processes.** With the
hub still live, the wizard closes each pane with `holler pane close PANE --profile NAME` (in reverse
layout order; `panes.sh teardown <config> <profile>` prints the lines), then deletes the profile with
`holler profile delete NAME`, each through `herdr.sh holler`. A close stops the pane's harness,
closes its Herdr pane and removes its record. Only then does `stop-owned.sh teardown` stop the
instance's live ledger processes in reverse start order (the body, the instance's own Herdr server —
through `stop-owned.sh`, never through `herdr.sh`, which refuses a server stop — and the hub), remove
only that instance's ledger, and print what it left: stale entries, foreign processes, the rest of the
state directory (removed only with `--purge-state`, which is refused for the default state directory,
`$HOME/.holler`) and every other instance's processes, ports and state. If the hub is gone, the
panes cannot be closed through Holler: the wizard says so and asks, and never falls back to a Herdr
command.

**The instance's tmux server keeps running.** A pane's close ends its processes but not its tmux
session, and no ledger row records the tmux server, so the wizard never stops it: a named instance's
server stays on `<state_dir>/tmux.sock` (end it by hand with `tmux -S <state_dir>/tmux.sock
kill-server` once you no longer need it), and for the default instance the sessions stay in your own
tmux server. `--purge-state` is not used while that server runs, because its socket is inside the
state directory. An old serve entry of the instance, left by the previous wizard, is not the
wizard's to remove; `tailscale serve reset` is never used: it wipes every serve entry, the first
instance's included.

**Teardown also stops the restored workspace.** The Herdr server the wizard started holds any
restored workspace of the user (with an auto-resumed agent) too, and teardown stops that server.
Stage 3's plan and the teardown entry list those panes and ask before teardown; for the default
instance the wizard offers to leave the server running and stop only the hub.

## If you built a workspace with the previous wizard

The previous wizard built a fleet of attach-mode sessions: OpenCode backends it started on other
machines, a session it created on each, one Holler body per remote machine, and Herdr panes it split,
labelled and attached a TUI to by typing into them. Nothing can adopt that layout as registry panes:
Holler's import (`pane import`) reads the old launcher's per-pane files, which no wizard file
provides, and `holler profile create NAME --from-current` snapshots only panes that already have a
record. So the wizard:

- **refuses the old config, and offers a conversion as text.** `panes.sh check` names every retired
  table and key. The conversion turns each `[[orchestrator]]` into a `[[pane]]` with
  `role = "orchestrator"` and the harness its `cmd` names when that is `opencode` or `claude` (for any
  other command it asks which of the two the pane gets, since what a pane holds is its harness, never
  a free-form command), drops each `[[session]]` and names it, and drops `hub_host`. The converted file
  is written next to the old one; the old one is never overwritten, and you confirm the new one before
  Stage 1 loads it.
- **leaves the old layout alone.** When the instance's ledger has `backend` rows, or Herdr panes named
  by the config show as unmanaged in `holler pane list`, Stage 3 offers a second instance beside the
  old layout (its own `[instance]` name, state directory, ports and Herdr session) or stopping. It
  offers no conversion in place and no teardown there; tearing the old layout down is the ledger
  teardown of "Stopping and tearing down", which you ask for by name.

A remote, headless OpenCode session is still possible by hand: attach mode
([ADR 0014](adr/ADR-0014.md)) and `holler body` serve it, and ADR-0021 section 11 says how that
relates to registry panes.

## Automated setup

A Claude Code skill drives this end to end — `herdr-setup`, a 12-stage wizard (Stage 0 through
Stage 11). The command you type is `/herdr-setup`. Stage 0 is the preflight. The skill ships in this repo at
[`agent-skills/herdr-setup/SKILL.md`](../agent-skills/herdr-setup/SKILL.md), which is its source
of truth, and its stages call the helper scripts in `agent-skills/herdr-setup/lib/`, so the
**whole directory** is installed, not just `SKILL.md`. On a machine with no checkout, one command
does it (no `sudo`):

```bash
mkdir -p ~/.claude/skills && curl -fsSL https://github.com/Performant-Labs/holler/archive/refs/heads/main.tar.gz | tar -xz -C ~/.claude/skills --strip-components=2 holler-main/agent-skills/herdr-setup
```

No release tag contains the whole skill directory yet, so the command fetches `main`; pin it to a
tag once one does. A skill installed as `~/.claude/skills/setup-wizard` before the rename is a different skill name now: remove or move it by hand (the wizard never deletes it). (The README has the same line, and its agent prompt fetches the directory the same way on a
machine that doesn't have it). Stage
0 comes first and needs no config: it is read-only and machine-level, prints one status table of what
this machine has (`herdr`, `holler` and its pane verbs, `tmux`, `opencode`, `claude`, the skill's
`lib/` scripts, the Herdr server and saved sessions, a hub running here, an existing ledger and
config), then prints the stage checklist as a vertical table. If `herdr` is missing it asks, as its
own yes/no, whether to install it (via [herdr.dev's install script](https://herdr.dev/install.sh) or
`brew install herdr`) and re-checks. It ends with a second, separate yes/no — "All systems go. Proceed?" — before anything past it runs; a yes to installing Herdr is never treated
as a yes to also building a workspace in the same run, and a no ends the wizard cleanly. Only once that second question gets a
yes does Stage 1 onward run: load the config described above and check it (`instance.sh` and
`panes.sh`), the config checks of Stage 2 (each pane's directory and harness, the OpenCode model,
this instance's ledger and registry, the collision inventory), present the plan and get explicit
assent before touching anything, then bring up the hub (Stage 4), start the Herdr server (Stage 5),
create the profile with one spec-only launch per pane (Stage 6), build the panes with the dry run and
one `profile apply` (Stage 7), join and run the one body (Stage 8), check the registry (Stage 9),
and verify every OpenCode pane end to end with a real round-trip reply (Stage 10). It's config-driven
throughout — however many panes, orchestrators and harnesses the config lists is however many get
built, never a hardcoded pair.

**A failed check is a prompt to ask, not a reason to stop.** The wizard is meant to push through
to a fully working install — when a stage's Gate fails or something is genuinely ambiguous, it
asks the operator one concrete, answerable question about exactly what's blocking it, then acts
on the answer and keeps going through the rest of the stages. Reporting a broken stage and
leaving the run there isn't the goal; a complete, verified workspace is.

## Collision preflight

Stage 2 takes a read-only inventory of this machine (`agent-skills/herdr-setup/lib/inventory.sh`):
listening ports, running `holler hub`, `holler body`, `opencode` and `herdr` processes (a body's row
carries its `--config` path), any serve configuration and the Herdr sessions; a tool it needs and
cannot find on `PATH` is reported as a `warn` line, never skipped silently. It only runs `ss`/`lsof`,
`ps`, the serve status and `herdr session list` (the Herdr section runs only with
`WIZARD_INVENTORY_HERDR=1`), and it never writes, signals, starts or stops anything. The first line of
an inventory is `home<TAB><this machine's $HOME>`, the home that `-` and `~/` state directories are
resolved against. Stage 3 feeds the inventory and the instance's plan to `lib/collide.sh`, in one
call that sets `WIZARD_HERDR_HOST=1`, `WIZARD_HUB_PORT` and `WIZARD_HERDR_SESSION`, the OpenCode
panes' ports as `WIZARD_BACKEND_PORTS`, and `WIZARD_SERVE_HTTPS_PORT` empty (the hub is served on
loopback only). It prints the plan beside the inventory (other instances' items are "present, not
touched", and so is an old serve entry of the instance) and **refuses** the plan, rather than warning,
when anything collides with something this instance did not create: a port already in use, a hub
already listening on `hub_port`, a state directory already in use (a hub or body row whose state
directory is not visible counts as the default one), or a Herdr session of the same name. For the
default instance, a running unnamed Herdr server with no live `herdr` ledger row is foreign and
refused too; the operator may answer yes to "build in it" in Stage 3 (which means a new workspace in
that server, which Holler's adapter creates, never anything done to the panes it already has), and the
wizard still never stops it. It does not check bodies per session, because a body's session is not on
its command line. Each refusal names the colliding item and the config key to change (`hub_port`,
`backend_port_base` or a pane's `backend_port`, `state_dir` or `herdr_session`). Stage 3 passes the
instance's own ledger so this instance's own processes are not refused on a rerun. A process counts as
this instance's own only if the instance's ledger (`<state_dir>/herdr/setup/wizard-ledger.toml`)
records its pid with the same start time and command; a reused pid is foreign.

Stage 3 also judges which case the build is: fresh, a rerun of this wizard (its profile exists:
the dry run shows what apply would do), panes in the registry outside the wizard's profile (the wizard
touches none, and refuses a config name that one of them holds, since apply would adopt, report or
relaunch such a record), a layout the previous wizard built (left alone: a second instance, or stop),
or restored panes (the new workspace is built beside them). Panes outside the profile are never put
under it unless you ask (`holler profile create NAME --from-current`, or apply's `--take-over`).

## Herdr session

Stage 5 starts the instance's Herdr server through `agent-skills/herdr-setup/lib/herdr.sh`, which adds
`--session <herdr_session>` (from the `[instance]` table) to every Herdr command it runs. The wrapper
reaches Herdr only for the server's start and three read-only words — `herdr.sh run status`,
`herdr.sh run session list` and `herdr.sh run workspace list` — and refuses every other `run` word (a
pane or workspace change, `server stop`, `session attach`) with exit 2, naming `holler pane`: the
panes are Holler's. A bare `herdr server stop` is never issued: which server it stops is not
established, and it could stop another instance's session. The wrapper refuses to run when
`WIZARD_INSTANCE_NAME` is unset (the default instance is the literal `default`, which may start and
use an unnamed server) and when a non-default instance sets no `herdr_session`. The skill sets
`WIZARD_INSTANCE_NAME`, `WIZARD_HERDR_SESSION`, `WIZARD_INSTANCE_PREFIX`, `WIZARD_STATE_DIR`,
`WIZARD_LOG_DIR` and the literals `panes.sh env` prints inline on every `herdr.sh` command (an agent's
shell does not keep exports between commands, and a lost export must not become a bare `herdr`
command on the default workspace). `herdr.sh server-start` prints the server's pid on its first line;
the skill records it with `ledger.sh record --pid <pid> --role herdr --stage 5 --session <herdr_session>`,
and that live `herdr` row is what later makes the session "created by the wizard". The instance's own
server stops through `stop-owned.sh`. Before building, the stage lists the machine's Herdr sessions;
if the named session exists and the instance's ledger did not record creating it (a live
`role = herdr` row), it stops and names the session, and never builds in a session it did not create.
If the run is inside a pane of a different session than the instance's, it stops; if the driving agent
is inside a Herdr pane at all and cannot establish the pane's session, `check-pane` stops and names the
fix (run the wizard from a terminal outside any Herdr pane). Stage 2 runs `check-pane` and
`check-session`, and a stop there is a Stage 3 refusal before "Proceed?". For a refused session that
exists and is stopped, the skill offers, with the operator's yes, `herdr.sh session-delete` (the
instance's own session, stopped only, refused when the ledger has a live `herdr` row); for a running
one the answer is another `herdr_session`. The same verb removes the stopped session after a teardown.
The server log is named for the instance and lives in the instance's logs directory
(`<state_dir>/logs/<prefix>-herdr-server.log`), never under `/tmp`.

**The one door for Holler.** Every pane and profile call runs as `herdr.sh holler <words>`, which runs
Holler with those words, unchanged, and the instance's literals: `HOLLER_STATE_DIR`, `HOLLER_HERDR_SESSION`
(`default` for the default instance), `HOLLER_HERDR_WORKSPACES` (`<workspace>=<rows>x<columns>`,
printed by `panes.sh env`), and `HOLLER_HERDR_SOCKET` for the default instance or `HOLLER_TMUX_SOCKET`
(`<state_dir>/tmux.sock`) for a named one. It refuses, with exit 2 and nothing run, an unset
`WIZARD_STATE_DIR`, a named instance without an absolute `WIZARD_TMUX_SOCKET`, and the default
instance without an absolute `WIZARD_HERDR_SOCKET`. Holler's verbs are configured from the environment
of the process that runs them, and the first call that creates a pane starts the instance's tmux
server with that environment, which every pane then inherits (tmux's own rule, unverified here): a
call from the driving agent's own shell would carry the agent's variables, its credentials and its
own Herdr pane included, into every pane — the leak #771 fixed for the Herdr server.

`herdr.sh server-start` and `herdr.sh holler` scrub the environment with one allow-list, so only it
reaches the server, `holler`, and so every pane: `PATH`, `HOME`, `USER`, `LOGNAME`, `SHELL`, `TERM`,
`LANG`, `LC_*`, `TMPDIR`, `XDG_*`, plus what an interactive pane needs: `SSH_AUTH_SOCK`, `DISPLAY`,
`WAYLAND_DISPLAY`, `DBUS_SESSION_BUS_ADDRESS`, `COLORTERM`, `TZ`, `EDITOR`, `VISUAL`, `LANGUAGE`, the
proxy variables (`HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`, `ALL_PROXY`, in both cases) and
`SSL_CERT_FILE`, `SSL_CERT_DIR`, `NODE_EXTRA_CA_CERTS`. The server also keeps `HERDR_*`; `holler`
gets the instance's `HOLLER_*` literals instead and no `HERDR_*`. The driving agent's own variables
(`CLAUDE_*`, `ANTHROPIC_*`, tokens, keys, `TMUX`, its own `HOLLER_*`) stay out, so Holler finds
`opencode` and `claude` as executables on `PATH`, never through a shell alias. A plain shell you open
in Herdr reads your shell rc as usual, but a registry pane's harness starts with no shell, so a
variable it needs (a provider credential for OpenCode, say) belongs in that harness's own
configuration, not in your shell rc (unverified until the operator's repeat of the new-user run).

Unverified until story #734 checks it with the real binary: how a pane's own session is learned
(`HERDR_PANE_ID` and `HERDR_SESSION` in the environment, in one function, `pane_session`, in
`herdr.sh`), and that `herdr session list` prints the session name first on each line. The tests
use a fake `herdr` and a fake `holler`.

## Basic Herdr commands

The wizard ends with a final step (Stage 11) that prints the handful of Herdr commands a new user
needs. It changes nothing; it is output only. The bindings are Herdr's defaults (confirm with
`prefix+?` if yours differ): attach with `herdr` (or `herdr --session <name>`); the prefix is
`ctrl+b`, released, then the key; `prefix+b` shows or hides the sidebar; `prefix+w` switches
workspace; `prefix+shift+n` / `prefix+shift+w` / `prefix+shift+d` create, rename and close a
workspace (also `herdr workspace close <id>`); `prefix+v` / `prefix+minus` split right / down;
`prefix+h/j/k/l` move between panes; `prefix+z` / `prefix+x` zoom and close a pane; `prefix+q`
detaches and leaves everything running. The wizard's panes are Holler's: to close, restart or
replace one, use Holler through the wizard's door, not `prefix+x`, or the registry keeps a record of
a pane that is gone. The wizard's final report prints what the door needs, the instance's
`<herdr_env>` settings and the skill's absolute `lib` path, so from your own shell a close is
`<herdr_env> bash <lib>/herdr.sh holler pane close PANE --profile <profile>` and a relaunch is
`<herdr_env> bash <lib>/herdr.sh holler pane relaunch PANE`. Never a bare `holler pane` verb, with
only `HOLLER_STATE_DIR` or with no environment: without the instance's Herdr session and tmux socket,
a close reads the record, then stops the processes of the tmux session of that name on tmux's default
server, which may be another instance's pane, and only then fails on the missing Herdr session,
leaving the record and the Herdr pane in place.

It also names any workspace the wizard did not create that sits beside the new one (a restored
saved session, or a fresh server's own first workspace, which holds one pane that is not the
wizard's), with its id and pane count, and says how to close it; the wizard never closes it. The
wizard's own teardown closes its panes through Holler, then runs `stop-owned.sh teardown`, which
closes the restored workspace too.

## Related

- [pane-control.md](pane-control.md) — the operator guide for every pane and profile verb the wizard
  runs, their exit codes and findings.
- [ADR-0021](adr/ADR-0021.md) — the pane control contract; its section 11 says how attach mode relates
  to registry panes.
- [ADR 0014](adr/ADR-0014.md) — attach mode, which the body uses to drive each OpenCode pane's session.
- [ADR 0005](adr/ADR-0005.md) — the session config's normative design.
