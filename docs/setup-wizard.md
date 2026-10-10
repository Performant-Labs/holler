# Watching multiple attach-mode sessions in one Herdr workspace

A pattern for driving several Holler-connected OpenCode sessions from one local terminal
workspace: one local orchestrator pane, plus one live view pane per remote session, all
watchable and interactable in one place — without giving up the fail-closed guarantees attach
mode already has ([ADR 0005](adr/ADR-0005.md)).

This is not a Holler feature — Holler's CLI (`hub`, `body`, `say`, `roster`, …) is unaware
Herdr exists. It is a way of arranging terminal panes around a real Holler hub/body pair, using
[Herdr](https://herdr.dev), a terminal workspace manager. If you don't use Herdr, the hub/body
setup itself is unaffected; this page only concerns the *viewing* layer on top of it.

## Shape

```
Local machine                              Remote machine
├─ Herdr workspace                          ├─ opencode --port 47001 --hostname 0.0.0.0
│  ├─ pane: local orchestrator (Claude/      │    (session "alpha" backend)
│  │        opencode, driving `holler`       ├─ opencode --port 47002 --hostname 0.0.0.0
│  │        itself via its own shell tool)   │    (session "beta" backend)
│  ├─ pane: opencode attach ...47001 -s ...  ├─ HOLLER_STATE_DIR=<state_dir> holler body run
│  └─ pane: opencode attach ...47002 -s ...  │    --config <body_config>  (attach mode, one
├─ HOLLER_STATE_DIR=<state_dir> holler hub   │    entry per backend)
│  serve --listen 127.0.0.1:<hub_port>       │
│  --advertise <host>.<tailnet>.ts.net:<serve_https_port>
└─ tailscale serve --bg --https <serve_https_port> <hub_port>
```

The ports in the diagram are the defaults (hub 41807, serve 443, backends from 47001); an
`[instance]` table changes them (see "Running an instance beside another").

The hub sits on whichever machine should be reachable from anywhere with tailnet access; the
body must be co-located with the real `opencode serve`/`opencode --port` processes it attaches
to, since attach mode talks to them over `127.0.0.1` loopback HTTP by construction — there is
no remote-OpenCode-endpoint mode. The diagram above shows the common one-orchestrator,
one-remote-host case; the config below is what actually decides how many orchestrators, how
many sessions, and how many distinct remote hosts a given run has — none of that is hardcoded.

## The config file

One TOML file is the source of truth for a run — every orchestrator, every session, and the
whole pane layout. The wizard resolves it in this order, using whichever it finds first (no
merging across tiers):

1. An explicit `--config <path>` passed to the wizard's invocation.
2. `./sessions.toml` — a project-local config, when the wizard is run from a project directory.
3. `~/.config/setup-wizard/sessions.toml` — the global fallback.

Shape:

```toml
# LAYOUT — a nested array: outer = columns left to right, inner = each column's panes top to
# bottom. Every [[orchestrator]] name and every [[session]] name must appear exactly once. Must
# come before any table header below (a bare key after a [table]/[[array]] header silently
# becomes a member of that table in TOML, not a top-level key).
layout = [["o1"], ["alpha", "beta"]]

# This machine's own tailnet FQDN — where the hub advertises itself.
hub_host = "hub.example.ts.net"

# One or more orchestrators — each a real CLI agent (commonly "claude", but not required to
# be), with its own working directory. Which sessions an orchestrator actually drives is a
# matter of how it's briefed, not something layout or this table encodes.
[[orchestrator]]
name = "o1"
dir = "~/Projects/holler"
cmd = "claude"

# One or more sessions. remote_host/remote_tailnet_host are PER-SESSION, not global — different
# sessions can live on different remote machines, so nothing here assumes a single shared host.
[[session]]
name = "alpha"
harness = "opencode"
mode = "attach"
endpoint = "http://127.0.0.1:47001"
remote_host = "remote-a"                              # for ssh — an entry in ~/.ssh/config
remote_tailnet_host = "remote-a.example.ts.net"     # for the attach pane's URL
# session_id filled in by the wizard once a real session exists — leave absent in a template

[[session]]
name = "beta"
harness = "opencode"
mode = "attach"
endpoint = "http://127.0.0.1:47002"
remote_host = "remote-a"
remote_tailnet_host = "remote-a.example.ts.net"
```

Sessions on different hosts are just more entries with a different `remote_host`/
`remote_tailnet_host` — the wizard groups sessions by host and runs one Holler body per distinct
host, rather than assuming everything lives on one remote machine. Multiple orchestrators work
the same way: more `[[orchestrator]]` entries, each with its own `dir`/`cmd`, each a real slot
in `layout`.

**The master file is not the body's `--config`.** Holler's config parser
(`crates/holler-body/src/config.rs`) denies unknown keys, so a typo like `harnes` is an error. It
knows the wizard's master-file keys (`hub_host`, `layout`, `[[orchestrator]]`, and each
session's `remote_host`/`remote_tailnet_host`) and ignores them, checking only their types. It does
not know the `[instance]` table or a session's `backend_port`, so a master file that has either
makes `holler body run` fail. The wizard therefore writes a **derived body config** per remote host
(only that host's `[[session]]` tables) and sends that one: it drops the `[instance]` table and every
session's `backend_port`, and sets each session's `endpoint` to
`http://127.0.0.1:<resolved backend port>`, the port `instance.sh` printed for that session (the master
file's own `endpoint` may name the first instance's backend). Other tools can keep their own data in
the file under an explicit namespace: a top-level `[ext.<namespace>]` table and a per-session
`[session.ext.<namespace>]` table, each an arbitrary TOML table the body never interprets (an `ext`
or `ext.<namespace>` that is not a table is refused). The master file (whichever of the three
sources above was actually loaded) keeps every field, including `[instance]` and the real
captured `session_id`s, for the next run. On the remote host the derived config is `~/sessions.toml`
when the master file has no `[instance]` table, else `<state_dir>/<prefix>-sessions.toml`.

**No local `ssh` client?** The wizard detects this and switches into a manual-relay mode: every
command it would otherwise run over `ssh` is printed for you to run yourself (or relay to a
shell that has `ssh`), with your pasted-back output driving the same verification it would do
directly. This isn't a config setting — it's a fallback based on whether `ssh` is actually
present, so no config change is required either way.

## The viewing mechanism: plain panes, not `herdr-mirror`

**Use plain local Herdr panes running `opencode attach <url> -s <session_id>`, not
`herdr-mirror`.** `opencode attach` accepts any URL, not just `localhost` — bind the remote
`opencode` process to `--hostname 0.0.0.0` (safe on a tailnet-only host; loopback access for
the co-located Holler body keeps working too) and point `attach` at
`http://<remote-host>.<tailnet>.ts.net:<port>` over the tailnet. The result is an ordinary
local pane as far as Herdr is concerned — closing it, moving it, or mixing it with other local
panes carries no special risk, because there is no cross-machine mirror relationship to break.

`herdr-mirror` (a plugin for live-streaming a remote *workspace*, not just a session) is a
different, heavier mechanism with real documented failure modes — including the mirror
daemon's own reconciliation killing the real remote processes it was only supposed to be
viewing, not just their local viewer panes.
The tradeoff for the simpler `opencode attach` approach: you lose Herdr's own agent-status
tracking (`idle`/`working`/`blocked`) for the remote session, since Herdr only classifies panes
it manages directly — a plain `opencode attach` pane is just a terminal running a program from
Herdr's point of view, invisible to `herdr agent list` even while fully live and working. Use
`herdr pane list` / `herdr pane read` to see it instead.

## Never kill a process you didn't identify first

**Before stopping, killing, or reusing any process, port, or credential on a remote host —
whether it's this pattern's own hub/body or something else entirely — identify what it actually
is first.** If it wasn't started by *this* run, treat it as someone's live production until
proven otherwise, and get explicit, named confirmation ("kill PID 822098, `holler body run` on
the remote host") before touching it. Never kill something just to free up a port or "be able to run a
test" — that phrasing is the failure mode itself, not a justification for it.

This isn't hypothetical: a real incident did exactly this. An agent testing this
setup pattern against a real remote host found an existing `holler body run` process using a
resource it wanted, killed it "to be able to run the test," and moved on — without checking
what that process actually was. It turned out to be an unrelated,
already-live Herdr session (a different hub, on a different machine, serving real
work), which went down with no warning and no way for anyone to have known in advance it was
safe to kill. It was recoverable in this case (attach-mode sessions survive independently of the
Holler body wrapping them — see "The viewing mechanism" above — so rejoining a fresh body
restored it without data loss), but that was luck: a spawn-mode session, or a body killed
mid-turn, would not have been recoverable the same way.

`ps -ef` (or the remote-host equivalent) and reading the process's own config/cwd before acting
on it costs seconds. Guessing, or asking forgiveness after, does not undo a killed production
process.

## Label every token by its hub, not just its remote host

**A remote host can be paired to more than one hub at once, or over time** — the same real
scenario the section above describes. A label of the form `<remote_host>-body` (e.g.
`remote-a-body`) names only the remote side of the pairing, so two different hubs both talking to
the same remote host end up with tokens/bodies that look identical to a human — or another
agent — reading a process list or a token store later. That ambiguity is exactly what made the
incident above possible in the first place: nobody could tell which `remote-a`-something belonged
to which hub without digging.

Fold the hub's own identity and the instance's `<prefix>` into the label instead:
`<hub_host's short name>-<prefix>-<remote_host>` — `hub1-second-remote-a`, not `remote-a-body`. Before minting, check what's already on the remote host
(`ssh <remote_host> "ps -ef | grep holler"` — the same check "Never kill a process you didn't
identify first" already has you running) and make sure your new label is visibly distinct from
anything already there, not merely a different string. The goal is that a roster entry, a
`hub token list` row, or a process someone inspects six months from now on a shared host names
*which hub* it belongs to on sight — not just that it's "the remote-a one," when there may be
more than one.

**Two mechanical gotchas worth knowing before minting, confirmed live 2026-09-23:**

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
  there, append a counter and mint `hub1-second-remote-a-2`, `-3`, and so on.
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
  the run rather than fixing the shell profile as part of setup.
- **A restarted Herdr server does not start blank — it restores its saved session, including
  resuming agents.** After killing the server and starting a fresh one, all the previous run's
  panes were already there, the orchestrator pane's Claude conversation had auto-resumed
  (`claude --resume <id>`), and the two viewer panes had come back as dead shells running
  `opencode --session <old id>` → `Session not found`. Check `herdr pane list` before splitting:
  if the pane count already equals orchestrators + sessions, reuse the structure, leave a
  healthy resumed orchestrator alone, and re-attach only the dead viewer panes using the new run's
  session ids.
- **A resumed orchestrator can't prove the `AGENTS.md` briefing works.** It may already know about
  Holler from its own past turns. To test the briefing, exit that session, launch a fresh
  `claude` (no `--resume`), and ask it something with no Holler context, like "Send a hello to
  alpha" — it should find the roster, resolve the namespaced session name, and get a real reply.

## Running an instance beside another

By default the wizard builds the only Holler instance: hub port 41807, Holler's default state
directory, `tailscale serve --bg 41807` and an unnamed Herdr server. To build a second instance
next to a running one, add an `[instance]` table to `sessions.toml`. It is a table header, so it
goes after the bare top-level keys (`layout`, `hub_host`) and before `[[orchestrator]]` and
`[[session]]`. Leave it out and nothing changes.

| Key | Type | Default | Rule |
|---|---|---|---|
| `name` | string | `default` | `^[a-z][a-z0-9-]{0,23}$` |
| `prefix` | string | the value of `name` | prefixes every log, config and process name; same pattern as `name` |
| `hub_port` | integer | 41807 | 1024 to 65535 |
| `serve_https_port` | integer | 443 | 1 to 65535 |
| `state_dir` | string | empty (Holler's own default) | absolute path or one starting with `~/`; the same value on every host the run touches |
| `herdr_session` | string | empty (no named session) | same pattern as `name` |
| `backend_port_base` | integer | 47001 | session `i` (0-based, in config order) listens on `base + i` unless it sets its own `backend_port` |

A non-default instance (any key differing from its default) must set `name`, `state_dir` and
`herdr_session`. Stage 1 runs `agent-skills/setup-wizard/lib/instance.sh <config>`, which refuses a
malformed value or a missing required key naming the key, and refuses two sessions on the same
`remote_host` that resolve to one backend port, naming both. A session may set `backend_port`
to override its `base + i` port.

```toml
layout = [["o1"], ["alpha", "beta"]]
hub_host = "hub.example.ts.net"

[instance]
name = "second"
hub_port = 41808
serve_https_port = 8443
state_dir = "~/.holler-second"
herdr_session = "second"
backend_port_base = 47101
```

## Instance state and the ledger

Stages 4 to 7 start everything on the instance's own ports and in its own state directory (the
`[instance]` table of `sessions.toml`; an absent table means today's defaults). Stage 1 runs
`instance.sh`, which prints each session's resolved backend port and endpoint; every later stage
uses those ports and never recomputes one (without an `[instance]` table a session's port is its
`endpoint`'s port; with one it is `backend_port_base + i` or the session's `backend_port`). The
hub listens on `hub_port` behind `tailscale serve --bg --https <serve_https_port> <hub_port>`, and
advertises `<hub_host>:<serve_https_port>`: bodies join with `wss://<hub_host>:<serve_https_port>`,
so a non-443 serve port reaches the instance's own serve endpoint and not the first instance's.
The hub and every body run with `HOLLER_STATE_DIR` set to the instance's `state_dir`, **resolved
to an absolute path once per host** (a `~/` value is expanded by that host's own shell; with no
`state_dir` the one spelling of the default is `$HOME/.holler`, resolved). A second instance
beside a running one therefore shares no token store, pepper, roster or config file with it, and
its bodies cannot join the other hub. Because the body refuses an `[instance]` table and a
`backend_port`, it never reads the master file: it reads a derived copy per host (see "The config
file").

The agent runs every command in a fresh shell, so the skill carries each value (state directory,
ports, pids, the Herdr settings) as a literal in the command itself and never relies on a
variable surviving to the next command. A command that lost its `HOLLER_STATE_DIR` would reach
the default instance's hub; the skill therefore sets it inline on every `holler hub`, `holler
roster`, `holler say` and `ledger.sh` call.

Every process the wizard starts is recorded in `<state_dir>/wizard-ledger.toml` on the host it
runs on (mode 0600, written atomically; with no `state_dir`, in Holler's default state
directory). One `[[process]]` table per process: `pid`, `started` (`LC_ALL=C ps -o lstart= -p
<pid>`), `cmd` (`ps -o command= -p <pid>`), `role` (`backend`, `hub`, `serve`, `body`, `herdr`),
`stage` (4 to 9) and `session`. A row is **live** while a process with that pid exists and its
start time and command still match; a pid that was reused by another program is **stale**, and
a process in no ledger is **foreign**. The wizard signals only live rows. On a rerun "reused"
means exactly one thing, a live row in this instance's ledger whose command still matches; it
starts and records a new process when the old one died, and never adopts one it did not record.
`agent-skills/setup-wizard/lib/ledger.sh` (`record`, `list`, `owns <pid>`) maintains the file and
reads the state directory from `HOLLER_STATE_DIR`. Each start command prints the pid of the
program itself (`{ nohup ... & echo $!; }`, because `cmd && nohup X & echo $!` can print the pid
of a subshell on some login shells), and after recording the agent confirms with `list` that the
recorded `cmd` starts with the program. The Herdr server's pid is the first line `herdr.sh
server-start` prints.

**The first instance has no ledger, and must never be given one by hand.** The Holler hub, bodies,
backends and Herdr that were running before the wizard first built a second instance were not
started by the wizard, so no ledger records them: they are foreign to every run, always reported
and never signalled. Do not "fix" that by recording their pids with `ledger.sh record`: a
hand-written row would make the wizard willing to stop the first instance, which is the one thing
this design exists to prevent. (Once the wizard itself builds an instance with the default state
directory, that instance does get its own ledger, because the wizard started its processes.)

On a rerun the wizard fetches each remote host's ledger next to its inventory (`ssh <remote_host>
"cat <state_dir>/wizard-ledger.toml"` into a named scratch directory under this machine's state
directory, `<state_dir>/wizard-scratch`) and passes it to the collision check as `WIZARD_LEDGER`;
without it every remote process would look foreign and the instance's own processes would be
refused.

The logs of the processes Stages 4 to 9 start go under the instance's state directory too,
`<state_dir>/logs/<prefix>-...` (`$HOME/.holler/logs` for the default), never `/tmp`, which is
RAM-backed on the target hosts and must not be filled.

## Stopping and tearing down

The wizard keeps a per-instance ledger of every process it starts (`<state_dir>/wizard-ledger.toml`,
one `[[process]]` with pid, start time, command, role, stage and session). A process is the wizard's
to stop only if that ledger recorded it and its current start time and command still match.

- **Stale** (the pid was recorded but now belongs to another command): never signalled, reported.
- **Foreign** (in no ledger, for example something else holding a planned port): never signalled;
  the wizard reports the pid, owner and command, then stops and asks.
- **Live**: stopped with SIGTERM, nothing stronger. If it is still running after the grace
  period the wizard says so and asks.

`agent-skills/setup-wizard/lib/stop-owned.sh` does this (`stop`, `restart`, `check-port`,
`teardown`; exit 0 done, 1 stale, 2 foreign, 3 still running, 4 usage). `restart` stops the
recorded process and prints `RESTART-CMD <recorded command>`; it does not run it, and that
recorded command has no environment, no `nohup` and no log redirect (run alone it would use the
default state directory). It only identifies the entry: the wizard re-runs the stage's own start
command (with `HOLLER_STATE_DIR`, `nohup` and the log path) and records the new pid. `teardown`
stops the instance's live ledger processes in reverse start order (the instance's own Herdr
server included, through `stop-owned.sh`, never `herdr.sh run server stop`), removes only that
instance's ledger, and prints what it left: stale entries, foreign processes, the rest of the
state directory (removed only with `--purge-state`, which is refused for the default state
directory, `$HOME/.holler`) and every other instance's processes, ports and state. It runs on
**every host the run touched**, each with that host's own state directory (for a remote host the
skill sends `stop-owned.sh` and `ledger.sh` together over `ssh`). The instance's `tailscale
serve` entry is not a process of the wizard, so teardown leaves it; with the user's yes,
`tailscale serve --https=<serve_https_port> off` turns off that one entry. `tailscale serve reset`
is never used: it wipes every serve entry, the first instance's included.

## Automated setup

A Claude Code skill drives this end to end — `setup-wizard`, an 11-stage wizard (Stage 0 through
Stage 10). The skill ships in this repo at
[`agent-skills/setup-wizard/SKILL.md`](../agent-skills/setup-wizard/SKILL.md), which is its source
of truth, and its stages call the helper scripts in `agent-skills/setup-wizard/lib/`, so the
**whole directory** is installed, not just `SKILL.md`. On a machine with no checkout, one command
does it (no `sudo`):

```bash
mkdir -p ~/.claude/skills && curl -fsSL https://github.com/Performant-Labs/holler/archive/refs/heads/main.tar.gz | tar -xz -C ~/.claude/skills --strip-components=2 holler-main/agent-skills/setup-wizard
```

(the README has the same line, and its agent prompt fetches the directory the same way on a
machine that doesn't have it). Stage
0 stands apart from the rest: it only installs the `herdr` binary itself (via
[herdr.dev's install script](https://herdr.dev/install.sh) or `brew install herdr`), asked as
its own up-front yes/no, and ends with a second, separate yes/no — "configure Herdr now and
launch a workspace?" — before anything past it runs; a yes to installing Herdr is never treated
as a yes to also building a workspace in the same run. Only once that second question gets a
yes does Stage 1 onward run: load the config described above, preflight (including per-host SSH
reachability, with a manual-relay fallback when no local `ssh` client exists), present the plan
and get explicit assent before touching anything, start the remote OpenCode backends, capture
real session IDs, bring up the hub, migrate the config and join/run one body per distinct remote
host, build the Herdr workspace, attach each pane, and verify every session end to end with a
real round-trip reply. It's config-driven throughout — however many orchestrators, sessions, and
distinct remote hosts the config lists is however many panes and bodies get built, never a
hardcoded pair.

**A failed check is a prompt to ask, not a reason to stop.** The wizard is meant to push through
to a fully working install — when a stage's Gate fails or something is genuinely ambiguous, it
asks the operator one concrete, answerable question about exactly what's blocking it, then acts
on the answer and keeps going through the rest of the stages. Reporting a broken stage and
leaving the run there isn't the goal; a complete, verified workspace is.

## Collision preflight

Stage 2 takes a read-only inventory of each host the run touches
(`agent-skills/setup-wizard/lib/inventory.sh`): listening ports, running `holler hub`,
`holler body`, `opencode` and `herdr` processes (a body's row carries its `--config` path), the `tailscale serve`
configuration and the Herdr sessions; a tool it needs and cannot find on `PATH` is reported as a
`warn` line, never skipped silently. It only runs `ss`/`lsof`, `ps`, `tailscale serve status` and
`herdr session list`; it never writes, signals, starts or stops anything. Stage 3 feeds each
inventory and the instance's plan to `lib/collide.sh`, which prints the plan beside the
inventory (other instances' items are "present, not touched", and so is this instance's own
`tailscale serve` entry when the ledger has a live `hub` row) and **refuses** the plan, rather
than warning, when anything collides with something this instance did not create: a port
already in use, a hub already listening on `hub_port`, a state directory already in use (a hub
or body row whose state directory is not visible counts as the default one), or a Herdr session
of the same name. It does not check bodies per session, because a body's session is not on its
command line. Each
refusal names the colliding item and the config key to change (`hub_port`, `serve_https_port`,
`backend_port_base` or a session's `backend_port`, `state_dir`, `herdr_session`, or the
`[[session]]` name). Stage 3 passes each host's own ledger (fetched next to its inventory) so
this instance's own processes are not refused on a rerun. A process counts as this instance's own only if the instance's ledger
(`<state_dir>/wizard-ledger.toml`) records its pid with the same start time and command; a
reused pid is foreign.

## Herdr session

Stage 8 and Stage 9 run every Herdr command through `agent-skills/setup-wizard/lib/herdr.sh`,
which adds `--session <herdr_session>` (from the `[instance]` table) to each one, `server stop`
and `session attach` included. A bare `herdr server stop` is never issued: which server it stops
is not established, and it could stop another instance's session. The wrapper refuses to run when
`WIZARD_INSTANCE_NAME` is unset (the default instance is the literal `default`, which may start
and use an unnamed server) and when a non-default instance sets no `herdr_session`. The skill
sets `WIZARD_INSTANCE_NAME`, `WIZARD_HERDR_SESSION`, `WIZARD_INSTANCE_PREFIX`, `WIZARD_STATE_DIR`
and `WIZARD_LOG_DIR` inline on every `herdr.sh` command (an agent's shell does not keep exports
between commands, and a lost export must not become a bare `herdr` command on the default
workspace). `herdr.sh server-start` prints the server's pid on its first line; the skill records
it with `ledger.sh record --pid <pid> --role herdr --stage 8 --session <herdr_session>`, and that
live `herdr` row is what later makes the session "created by the wizard". The instance's own
server stops through `stop-owned.sh`, not `herdr.sh run server stop`. Before building, the stage lists the machine's
Herdr sessions; if the named session exists and the instance's ledger did not record creating it
(a live `role = herdr` row), it stops and names the session, and never splits panes of a session it did not
create. If the run is inside a pane of a different session than the instance's, it stops before
any split. The server log is named for the instance and lives in the instance's logs directory
(`<state_dir>/logs/<prefix>-herdr-server.log`), never under `/tmp`.

Unverified until story #734 checks it with the real binary: how a pane's own session is learned
(`HERDR_PANE_ID` and `HERDR_SESSION` in the environment, in one function, `pane_session`, in
`herdr.sh`), and that `herdr session list` prints the session name first on each line. The tests
use a fake `herdr`.

## Related

- [ADR 0005](adr/ADR-0005.md) — attach mode's normative design.
- [ADR 0006](adr/ADR-0006.md) — the `wss://`/TLS-proxy hub design this pattern's `tailscale
  serve` step relies on.
