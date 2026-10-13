---
name: herdr-setup
description: Run a read-only preflight first (Stage 0: what this machine has, with Herdr installed on a yes if it is missing), end it with the question "All systems go. Proceed?", and only then optionally build a Herdr workspace of Holler registry panes — a grid of OpenCode and Claude Code panes, one or more of them orchestrators, each launched, recorded and kept in shape by `holler pane` and `holler profile apply`, and driven through a Holler hub and body on this machine (never herdr-mirror). Config-driven, not hardcoded — every pane's name, directory, harness and role, and the whole grid (how many panes, how many orchestrators, and where each one sits), are read from the config, never assumed. Use when the user asks to install Herdr, set up a Herdr workspace of agent panes driven by Holler, recreate the orchestrator-and-agents grid, or rebuild that workspace.
---

# Herdr: a grid of OpenCode and Claude Code panes, built through Holler — wizard

Builds a Herdr workspace whose every pane is a Holler **registry pane**: a record in the hub's pane
registry, launched, recorded and reconciled by Holler's own verbs (`holler pane`,
`holler profile apply`; epic #633, ADR-0021), running OpenCode or Claude Code, with one or more of
the panes the orchestrators that drive the others. This repo's `docs/setup-wizard.md` carries the operational rules
and the reasoning behind each step's shape (its section "How the wizard builds panes" is the design),
and `docs/pane-control.md` is the operator guide for every pane verb the stages run — read them if
anything here looks surprising.

**The Holler session config file is the base model for everything this wizard builds.** It is
not just an artifact produced at the end — read it *first*, and every later stage's cardinality
(how many panes, which of them are orchestrators, which harness each one runs, and where every one
of them sits) is derived from the config you actually read. Never hardcode a pane count, an
orchestrator count, any name, any directory, or a harness anywhere in this run.

Target shape: one Herdr workspace on this machine, arranged into one or more columns per the
config's own `layout` field — every pane is a real, explicit entry in that grid (no assumed
"there is exactly one orchestrator" or "it's always the fixed left pane": however many `[[pane]]`
tables the config has, and wherever `layout` puts each one, is what gets built), recorded in the
registry under one profile, and driven by one Holler hub and one Holler body, both on this machine.

**Never use `herdr-mirror` for this.** Three separate confirmed incidents (in the handbook
doc's "Mirror gotchas" section) include one where the mirror daemon's own reconciliation
killed the real processes it was only meant to show, not just their viewer panes. Everything below
builds registry panes through Holler's pane verbs, which never touch the mirror daemon.

**Never drive a Herdr pane yourself.** Every pane is built, changed and read through Holler, and
every Holler pane call goes through one door, `herdr.sh holler` (Stage 6 says why). The wizard runs no
Herdr pane or workspace command of its own: `herdr.sh run` reaches Herdr only for `status`,
`session list` and `workspace list`, and refuses every other word (exit 2, naming `holler pane`).

**Shell variables do not persist between commands.** Every command you run starts a fresh
shell: nothing you assigned or `export`ed in an earlier command exists in the next one. So every
value this skill uses (the skill's `lib` path, the instance's state directory, a pid, a port, the
Herdr settings) is carried as a **literal**: write the real value into each command, or set it
inline in that same command (`HOLLER_STATE_DIR=<state_dir> holler ...`). Where the text below
shows `$WIZARD_LIB` or an angle-bracket placeholder such as `<state_dir>`, substitute the literal
value before you run it. Keep the literals (Stage 1 lists them) in your own notes and repeat the
ones a human needs in each stage's report. A lost value must never turn into a command that
reaches the default hub or the default Herdr: if you are not sure of a value, stop and re-derive
it from Stage 1.

**The ownership rule (the one rule every stop, restart, rejoin and teardown step points at).**
The wizard signals a process only if this instance's own ledger
(`<state_dir>/herdr/setup/wizard-ledger.toml`) recorded it **and** the process's current start time and
command still equal the recorded ones. A recorded pid whose start time or command differ is
**stale** (a reused pid): never signal it, report it. A process in no ledger is **foreign**:
never signal it, report its pid, owner and command, then stop and ask the user. This holds
even when a foreign process holds a port or a name the plan needs. Do it with
`stop-owned.sh` in the skill's `lib` directory (it asks `ledger.sh` and signals nothing by name
or pattern): `bash $WIZARD_LIB/stop-owned.sh stop <state_dir> <pid>`, `... restart <state_dir> <pid>`,
`... check-port <state_dir> <port>`, `... teardown <state_dir>` (with `--purge-state` as a third
argument only when the user asked for it). Exit 0 is done, 1 is stale, 2 is foreign (both
unsignalled: report and ask), 3 is still running, 4 is a usage error. Do not substitute a
hand-written `kill`, `pkill` or `killall`. `<state_dir>` is the instance's resolved state directory
(Stage 1). The Herdr server the wizard started for the instance is a ledger process like any other:
it stops through `stop-owned.sh` (a `herdr` row), never `herdr.sh run
server stop` (which `herdr.sh` refuses anyway). **A pane is not a ledger process**: Holler started it
and records it, so it stops only through `holler pane close` (the teardown in "If a stage fails"),
never through `stop-owned.sh` or a signal.

A ledger left at the old path (`<state_dir>/wizard-ledger.toml`, before the wizard moved its files to
`<state_dir>/herdr/setup/`) is not read: its processes look foreign. The operator may move the file by hand.

**Two harnesses, one kind of pane.** A pane's harness is OpenCode (`harness = "opencode"`, the default)
or Claude Code (`harness = "claude"`, #831), and an orchestrator is a role on a pane of either kind, never
a free-form command: Holler starts what the harness adapter builds, so what a pane holds is its harness
and nothing else. Where the two differ, the stage says so: an OpenCode pane has a server on a fixed port,
and its model is recorded, not applied, until #829 (its server runs what its OpenCode configuration
selects); a Claude Code pane has no server and no port, its model and effort are applied, and its body
row is not derived by this wizard: Stage 8 derives OpenCode rows only, and the operator writes a Claude
Code pane's row by hand from the README's "A Claude Code pane" (#850). So Stage 10's `holler say` round
trip covers a Claude Code pane only when that row exists.

## Connections this wizard's setup actually makes

Every literal port and path in this section is one of the default instance's **defaults** (hub port
41807, the OpenCode panes' ports from 47001, the unnamed Herdr session's socket); an `[instance]` table
changes them (`hub_port`, `backend_port_base`, `herdr_session`, and a pane's own `backend_port`), and the
stages below always use the instance's values. Everything runs on this one machine, so every
connection is loopback or a local socket, verified against the ADRs in the `holler` repo
(`docs/adr/ADR-0004.md`, `ADR-0014.md`, `ADR-0021.md`) rather than assumed:

1. **Hub ↔ body ("the circuit").** JSON-RPC 2.0 over a WebSocket (ADR 0004) on loopback,
   `ws://127.0.0.1:41807`: the hub binds loopback only, and the body on this machine dials it there (a
   plaintext `ws://` to a loopback host is allowed, so there is no TLS proxy and no tailnet in the
   path). Authenticated by a minted token (`circuit/join`) plus a Noise XK handshake (ADR-0019). Stage 4
   starts the hub; Stage 8 joins and runs the body.
2. **Body ↔ each OpenCode pane's server ("the body hop").** Attach mode (ADR 0014): plain HTTP on
   loopback, `http://127.0.0.1:<port>`, to the server Holler started inside that pane's tmux session.
   The body is not that server's parent, and its rows take each pane's port and session of record from
   the registry (Stage 8). A Claude Code pane has no server: its body row, which the operator writes by
   hand (#850), attaches to its process, and this wizard derives none.
3. **Holler ↔ Herdr, tmux and the harnesses.** Holler's pane verbs reach Herdr through its adapter over
   Herdr's **local Unix socket** (for the default instance `~/.config/herdr/herdr.sock`), run each pane's
   harness in a tmux session named as the pane (the default instance on tmux's default server, a named
   instance on its own socket), and reach OpenCode over loopback HTTP and Claude Code as a process. The
   wizard itself reaches Herdr only for the server's start and its read-only status words.
4. **A harness ↔ its model.** OpenCode's provider block in `~/.config/opencode/opencode.jsonc`; Claude
   Code's own login. Not part of Holler's wire: Stage 2 checks the OpenCode model only because a pane
   can't do anything useful without it, and the wizard never touches the Claude Code login (Stage 9).

None of them crosses machines, so nothing here needs a firewall or tailnet consideration.

## The config file

**One file, single source of truth — resolved in this order:**

1. **An explicit `--config <path>` flag passed to this skill's invocation** (e.g.
   `/herdr-setup --config /path/to/some/sessions.toml`, or `--config=<path>`) — if the
   invocation text contains this flag, `<path>` *is* the config file, full stop. It takes
   priority over both of the searches below, and neither of them runs at all in this case.
2. `./sessions.toml` (the current working directory the wizard is invoked from) — lets a
   project keep its own config, taking priority over the fallback when present.
3. `~/.holler/herdr/setup/sessions.toml` — the global default/fallback.

Use whichever is found **first**, don't merge across tiers. An explicit `--config` path (tier 1)
that doesn't exist or fails to parse is a hard failure right there — it does **not** fall
through to tiers 2 or 3; see Stage 1's Gate. State which of the three you're actually using when
you report Stage 1's result — don't leave it ambiguous which config is in effect.

**Default pane names.** When the user does not name the panes, each one is named for its grid cell,
row then column, like a spreadsheet cell: `r1c1`, `r1c2`, `r1c3`, `r1c4` for the first row, `r2c1`
for the first cell of the second row, and so on. `layout` is built as columns of those names.

Everything this wizard needs to know — every pane, its directory, harness and role, the whole
layout, and the defaults the panes share — is authored in whichever file that is. The user edits one
file, not several:

```toml
# THE DEFAULTS every pane takes unless it sets its own: the model (provider/id, read from this
# machine's OpenCode configuration in Stage 1), the effort, and the two context ceilings a watchdog
# reads. A new pane spec needs all four. A Claude Code pane never takes this `model` (it names an
# OpenCode model): it sets its own. Bare top-level keys, before every table header.
model = "provider/model-name"
effort = "medium"
ctx_soft = 800000
ctx_hard = 900000

# LAYOUT — one field, a nested array: outer = columns, left to right; inner = each column's
# panes, top to bottom. This is the ENTIRE workspace layout: every `[[pane]]` name is a real slot in
# this grid and appears in it exactly once. No column may be taller than the column to its left (the
# first column is the tallest): Herdr adds a row only below a row of one pane, so that is the set of
# grids Holler can build. Stage 1's check refuses a taller column, naming `layout` and the column.
# Written on ONE line: `instance.sh` reads a line that starts with `[` as a table header, so the
# check refuses a layout spread over several lines, naming `layout`.
#
# MUST come before every table header below (`[instance]`, `[[pane]]`) — a bare top-level key
# placed after any table header is parsed as belonging to THAT table, not the file's top level.
# This bit the wizard's own real config once; don't repeat it.
layout = [["r1c1", "r2c1"], ["r1c2", "r2c2"]]

# OPTIONAL — the name of the Herdr workspace: 1 to 40 characters, a letter or digit first, then
# letters, digits, spaces, `.`, `_` or `-`. Bare top-level key, before every table header. With no
# `workspace` it is `holler`. The same name is the profile the wizard builds (`<profile>`).
workspace = "herald-relay"

# OPTIONAL — an `[instance]` table, to build a second Holler instance beside a running one.
# Leave it out and everything below behaves exactly as before. It is a table header, so it goes
# AFTER the bare top-level keys above and BEFORE every `[[pane]]`. Every value shown is the
# default. A non-default instance must set `name`, `state_dir` and `herdr_session`. Stage 1
# validates it with `lib/instance.sh`. An OpenCode pane may also set its own `backend_port`.
# See docs/setup-wizard.md.
#
# [instance]
# name = "default"             # ^[a-z][a-z0-9-]{0,23}$
# prefix = "default"           # prefixes every log, config and process name; defaults to `name`
# hub_port = 41807             # 1024 to 65535
# serve_https_port = 443       # 1 to 65535; still validated, and unused: the hub is served on loopback only
# state_dir = ""               # absolute or ~/ path; empty = Holler's default ($HOME/.holler)
# herdr_session = ""           # empty = no named Herdr session
# backend_port_base = 47001    # OpenCode pane i (0-based, layout order) listens on base + i unless it sets backend_port

# One `[[pane]]` per grid cell. `name` is how `layout` refers to it (lower-case letters and digits,
# `-` between them, 1 to 32 characters: it is also the pane's tmux session name); `dir` is its
# working directory (absolute, or starting with ~/); `harness` is "opencode" (the default) or
# "claude"; `role` is "agent" (the default) or "orchestrator" — at least one pane is an orchestrator,
# and there may be several (the MO pattern). Optional: `agent` (the OpenCode agent the pane's
# messages run as), `backend_port` (an OpenCode pane's own port), and `model`, `effort`,
# `ctx_soft`, `ctx_hard` to override the defaults above for this pane.
[[pane]]
name = "r1c1"
dir = "~/Projects/demo"
role = "orchestrator"

[[pane]]
name = "r2c1"
dir = "~/Projects/demo"

[[pane]]
name = "r1c2"
dir = "~/Projects/demo"
harness = "claude"
model = "anthropic/claude-sonnet-5-5"   # a Claude Code pane sets its own model

[[pane]]
name = "r2c2"
dir = "~/Projects/demo"
agent = "build"
```

That is a 2 x 2 grid of four panes: an OpenCode orchestrator at `r1c1`, two OpenCode agents and one
Claude Code agent. Which panes an orchestrator actually *drives* is up to how it is briefed (its own
`AGENTS.md`, per the "MO" (multiple-orchestrator) pattern) — `layout` only controls where panes sit on
screen, not who talks to whom over Holler. With a pane count that does not fill the last row, the short
columns go on the right: five panes are `[["r1c1", "r2c1"], ["r1c2", "r2c2"], ["r1c3"]]`.

If a `[[pane]]` is missing `name` or `dir`, ask the user once (in Stage 1) for what's missing, state
the value you propose and let them confirm or override it, and write the answer back into this same
file — don't ask again on every run.

**The retired tables and keys.** A config of the previous wizard has `[[orchestrator]]` and
`[[session]]` tables and the keys `endpoint`, `mode`, `session_id`, `cmd` and the two remote-host keys:
the wizard no longer builds attach-mode sessions on other machines, and an orchestrator is a role on a
pane. `bash $WIZARD_LIB/panes.sh check <config>` refuses each by name, and Stage 1 offers the conversion.
`hub_host` is accepted and ignored (`check` prints one warning line): nothing on another machine dials
the hub any more.

**The master file is not the body's config.** Holler's parser (`crates/holler-body/src/config.rs`)
denies unknown keys at the top level and per session (typos stay errors), and it does not know the
wizard's `[instance]` table, a pane's `backend_port`, a `[[pane]]` table or the top-level `workspace`
key: a file holding any of them makes `holler body run` fail to start. So Stage 8 writes a
**derived body config**, built from the registry rather than copied from this file: it **drops** the `[instance]`
table and every `[[pane]]` table with its `backend_port`, it **drops** the top-level `workspace` key with
`layout` and the defaults, and it **sets** one `[[session]]` row per OpenCode pane whose `endpoint` and
`session_id` are that pane's port and session of record as `holler pane get NAME --format=json` prints
them, never values from this file. The master file — whichever of the three sources Stage 1 actually
loaded (explicit `--config` flag, cwd, or the global fallback) — keeps every field for next time; the
derived file is *generated*, every run, at `<body_config>` (Stage 8). Don't hand-author it.

## How to run this: one stage at a time, gated on real verification

This is a **wizard, not a script to dump and run**. Work through the stages below **in order**,
starting at **Stage 0 — Preflight**, which comes before everything else: it is machine-level and
read-only, needs no config, prints one status table of what this machine has (installing Herdr on
its own yes if `herdr` is missing), and ends with an explicit yes/no, "All systems go. Proceed?",
before Stage 1 (config + launch) is even offered.

**The whole point of this wizard is to push through to a complete, working install — a Gate
failure is a prompt to ask the user a concrete question, not a reason to stop and hand the run
back unfinished.** Never end a run by reporting a broken stage and leaving it there. When a Gate
fails or something is genuinely ambiguous: identify exactly what's blocking progress, ask the
user one specific, answerable question about it (not a vague "what should I do?"), and — once
answered — act on that answer and continue the stage sequence, all the way through Stage 10.
"I've told the user what's wrong" is not the same as "this is done" — keep going until the
workspace is actually up and verified, or until the user gives an explicit, deliberate reason to
stop the whole run (declining Stage 3's plan, or a hard technical wall nothing in "If a stage
fails" resolves). A half-finished run with a diagnosis but no working workspace is not success.
For every stage:

1. State which stage you're starting (`Stage N/11: <name>`, N running 0 through 11 — 12 stages
   total: Stage 0 (the Preflight), the build-and-verify stages 1 to 10, and the final Stage 11).
2. If the stage has an **Ask** line, ask the user that question and wait for the answer before
   doing anything else in the stage — don't guess a directory, a harness, a port, a model or a name.
3. Run the **Do** commands — in Stages 6 to 9 this means every line `panes.sh` printed and every pane
   in the config you read in Stage 1, not a fixed pair or a fixed single orchestrator, and each pane's
   own `harness`, not an assumed `opencode`.
4. Run the **Verify** command and read its real output — never assume success from a command's
   exit code alone if Verify gives you something more concrete to check (a step's outcome, a pane's
   cell, a `connected` row, a reply's content).
5. Report one line: what you did, what Verify actually showed.
6. **Gate:** if Verify passed, move to the next stage automatically. If it failed, don't retry
   blindly, don't skip ahead, and don't improvise a fix silently — but also don't just stop and
   hand the problem back unresolved. Look up the matching entry in "If a stage fails" below, tell
   the user plainly what broke, and ask them the one concrete question that unblocks it (a
   missing value, a yes/no on stopping something, which of two real options they want) — then act
   on their answer and resume the stage sequence from wherever it left off. Only end the run
   early if the user's answer is itself "stop," or if a stage's own Gate is explicit that no
   answer can resolve it.

**Every Holler call is bounded, and a failure is a question.** Each pane and profile verb carries its
own 10 s per-call bound; `profile apply` adds those bounds up over its creates (Stage 7 states the
sum and the timeout to give it). A call that fails is a Gate with a question: never a retry loop,
and never a fallback to a Herdr command.

**Stage 3 is a hard stop, not just a gate like the others: nothing that changes real state
(starting a process, stopping one, joining, creating a profile or a pane) may happen before the user
has explicitly said to proceed.** Stage 0 (apart from installing Herdr or the skill, each on its own yes) and Stages 1 and 2 are read-only except the scratch directory and the config file you write from the user's answers (loading a file,
checking status; the only things they write are `<scratch>` and that config file, and installing the skill needs its own
yes, see Stage 0) so they're safe to run ahead of that; everything from Stage 4 onward is not.

Keep a running status checklist as you go, as a **vertical table** (one row per stage, columns
Stage, Name, Status), not an inline list and not a dense bracket-number line (a human reading it
mid-run shouldn't have to cross-reference a legend to know what's happening). Stage 0 prints it
directly after the preflight status table, and every stage transition reprints the whole table:

```
| Stage | Name                                                     | Status |
|-------|----------------------------------------------------------|--------|
| 0     | Preflight                                                | ✅     |
| 1     | Load the session config                                  | ✅     |
| 2     | Config checks                                            | ✅     |
| 3     | Present the plan, get assent                             | 🔄     |
| 4     | Bring up the hub                                         | ⬜     |
| 5     | Start the Herdr server                                   | ⬜     |
| 6     | Create the profile                                       | ⬜     |
| 7     | Build the panes                                          | ⬜     |
| 8     | Join and run the body                                    | ⬜     |
| 9     | Check the registry                                       | ⬜     |
| 10    | Verify end-to-end                                        | ⬜     |
| 11    | Basic Herdr commands                                     | ⬜     |
```

✅ done · 🔄 in progress (the stage you just started) · ⬜ not started yet · ⏭️ skipped (nothing to
do, for example Stage 8 with no OpenCode pane: there is no body row) · ❌ failed/stopped (pair with a
one-line reason). The legend stays one line under the first table. From Stage 1 onward add the real
counts to the Name column where they help (the pane count, how many run OpenCode and how many Claude
Code, the orchestrators' names), not a generic "N panes" placeholder — the point is a human can read it
at a glance, including which harnesses are actually in play.

## Stage 0 — Preflight

**Stage 0 is the Preflight: machine-level and read-only, and it needs no config.** It looks at what this
machine has, prints it as one status table, and ends with the question "All systems go.
Proceed?". Its one write is installing `herdr` (or the skill's own directory), each only on an
explicit yes. It has nothing to do with building a workspace: the checks that need the config (each
pane's directory and harness, the OpenCode model) are Stage 2's. Never fold the install question and
the closing question into one ask: a yes to installing Herdr is not a yes to also setting up a
workspace in the same run.

**First, locate the skill's scripts and define `WIZARD_LIB` (once, here, in Stage 0).** Every helper script
this wizard runs lives in the `lib/` directory of the **whole** installed skill directory, not
just `SKILL.md`. The one definition, an absolute path (`<HOME>` is the output of `printf %s
"$HOME"`; if the skill is installed elsewhere, use that directory's absolute `lib` path):
```
WIZARD_LIB=<HOME>/.claude/skills/herdr-setup/lib
```
Check it before going on: `ls <that path>/ledger.sh <that path>/instance.sh <that path>/herdr.sh <that path>/panes.sh`.
If the files are not there, the skill was installed as a single file, or by a version of this skill
from before `panes.sh` existed. Installing writes into
`~/.claude/skills`, so **ask the user before installing** (Stage 0 changes nothing else but Herdr, on its own yes), then
install the whole directory (works on a machine with no checkout of this repository, and needs no
`sudo`):
```bash
mkdir -p ~/.claude/skills && curl -fsSL https://github.com/Performant-Labs/holler/archive/refs/heads/main.tar.gz | tar -xz -C ~/.claude/skills --strip-components=2 holler-main/agent-skills/herdr-setup
```
No release tag contains the whole skill directory yet, so the command fetches `main`; pin it to a
tag once one does. From here on the text writes `$WIZARD_LIB`; as the shell-state rule says, write the literal path
into each command (it is never set for you). Run the scripts with `bash`, never by path alone.

**Do (every command is local, none changes anything, and each runs as its own command).** The
Herdr settings here are the default instance's (the config is not read yet); a config with an
`[instance]` table gets its own scoped checks in Stage 2, from its ledger and inventory.
```bash
command -v herdr >/dev/null 2>&1 && herdr --version || echo "herdr: not found"
command -v holler >/dev/null 2>&1 && holler --version && holler --help || echo "holler: not found"
command -v tmux >/dev/null 2>&1 && tmux -V || echo "tmux: not found"
command -v opencode >/dev/null 2>&1 && echo "opencode: $(command -v opencode)" || echo "opencode: not found"
command -v claude >/dev/null 2>&1 && echo "claude: $(command -v claude)" || echo "claude: not found"
for f in ledger instance panes herdr; do test -f <WIZARD_LIB>/$f.sh && echo "$f.sh ok" || echo "$f.sh MISSING"; done
WIZARD_INSTANCE_NAME=default bash $WIZARD_LIB/herdr.sh check-pane
WIZARD_INSTANCE_NAME=default bash $WIZARD_LIB/herdr.sh run status 2>&1
herdr session list 2>&1   # default instance only
HOLLER_STATE_DIR="$HOME/.holler" holler hub status 2>&1   # default instance only
cat "$HOME/.holler/herdr/setup/wizard-ledger.toml" 2>&1   # default instance only
ls ./sessions.toml "$HOME/.holler/herdr/setup/sessions.toml" 2>&1
```
When `holler` is found, also run `holler pane --help` on its own: it must list the pane verbs
(`launch`, `list`, `get`, `doctor`, `close`), and `holler --help` must list `pane` and `profile` beside
`hub` and `body`. A `holler` without them predates pane control (epic #633). Each of `tmux`, `opencode`
and `claude` counts as present only when `command -v` prints an absolute path: Holler finds `opencode`
and `claude` as executables on `PATH`, never through a shell alias, so an alias (which `command -v`
prints as `alias ...`) is not an alias Holler can use — it is missing. A `--config <path>` flag in the
invocation text is the first of the three config tiers: report whether that file exists. When `herdr`
or `holler` is off `PATH`, use its absolute path (see the note after Verify). A missing tool is a row of
the table, not a failure of the stage.

**Print one vertical status table**, columns Check, Result and Detail, one row per machine fact
the wizard can establish without a config:

| Check | Result | Detail |
|---|---|---|
| `herdr` installed | ok / missing | the version; the absolute path when it is off `PATH` |
| `holler` is the current single binary | ok / missing / legacy | the version; whether `hub`, `body`, `pane` and `profile` subcommands exist (`holler pane --help` lists the pane verbs; the legacy client lists flat `join`/`run`/`detach`/`attach` verbs and a `0.1.x` version) |
| `tmux` present | ok / absent | the version: every pane's harness runs in a tmux session |
| `opencode` present | ok / absent | the absolute path (an executable on `PATH`, not an alias); needed by an OpenCode pane |
| `claude` present | ok / absent | the absolute path (an executable on `PATH`, not an alias); needed by a Claude Code pane |
| The skill's `lib/` scripts present (`WIZARD_LIB`) | ok / missing | the absolute `lib` path |
| Driving agent not inside a Herdr pane (`check-pane`) | ok / stop | a stop is a Stage 3 refusal, carried forward |
| Herdr server state and saved sessions | info | the server running or stopped, and every session from `herdr session list` with its status (running or stopped) |
| A Holler hub running here | info | `holler hub status`, default state directory |
| An existing wizard ledger | info | its live rows |
| An existing config at the three tiers | info | which tier, as an absolute path |

**Directly after that table, print the stage checklist as a vertical table** (columns `| Stage |
Name | Status |`, one row per stage, as defined under "How to run") and reprint it after every
stage transition. The legend stays one line under the first table. Never print the checklist
before this table: before it the machine's state is not known.

**Ask:** only if `herdr` is missing. Check first, so the question is informed rather than asked
blind. If `herdr` is already on `PATH`, say so (name the version) and go straight to the closing
question below: there is nothing to install. If it is not found, ask plainly: **"Herdr isn't
installed on this machine — do you want to install it?"** Wait for a real yes or no; don't
install anything before an explicit answer.

**Do**, only if the user said yes and `herdr` wasn't already found:
```bash
curl -fsSL https://herdr.dev/install.sh | sh
```
This is the canonical installer from the real `herdrdev/herdr` project (verified against its own
`README.md`, `https://github.com/herdrdev/herdr`) — it installs only the `herdr` binary itself
for the current user on this machine, and it says nothing about whether `holler`, `tmux`,
`opencode` or `claude` is present (each is its own row). If the user would rather use Homebrew (a
real, equally valid path on a Homebrew-managed machine), `brew install herdr` is the same install,
just via a different channel — use whichever the user asks for; don't default to one over the other
without asking if they have a preference.

**Verify:** the stage re-checks `herdr` after an install, then prints the status table again with the new result:
```bash
command -v herdr >/dev/null 2>&1 && herdr --version || echo "herdr: still not found"
```
**A real Homebrew/Linuxbrew-installed `herdr` (or `holler`, same issue for its row) can still
report `not found` here even in a login shell (`bash -lc`)** — this isn't just the
login-vs-non-login PATH gap noted elsewhere; some machines simply never added the brew prefix's
`bin` dir to `$PATH` at all. Confirmed live 2026-09-23: `herdr` was a real, working
Linuxbrew-managed install (a valid symlink into `../Cellar/herdr/...` under the Linuxbrew
prefix) on a machine whose `bash -lc 'which herdr'` still came back empty. Before
concluding it's genuinely missing, check the common install locations directly:
```bash
ls -la ~/.local/bin/herdr "$(brew --prefix 2>/dev/null)/bin/herdr" /opt/homebrew/bin/herdr /home/linuxbrew/.linuxbrew/bin/herdr 2>/dev/null
```
If one of those resolves to a real binary, it *is* installed — just not on this shell's `$PATH`.
Don't fix the shell profile as part of this wizard; simplest is to use that absolute path for
every `herdr`/`holler` invocation on this host for the rest of the run, and say plainly that
you're doing so (once, not on every command). For Herdr that means `HERDR_BIN=<that absolute
path>` among the Herdr settings Stage 1 defines (`herdr.sh` runs that binary); for Holler it means
`WIZARD_HOLLER_BIN=<that absolute path>` among the same settings (`herdr.sh holler` runs that binary).

**Gate:** A no to installing ends the wizard cleanly right here: report plainly that Herdr
isn't installed and the wizard is stopping, and let the user re-invoke later; do not fall through
to the closing question. If the install command itself fails, report the real error (network
failure, permission issue, unsupported platform) and don't proceed to the closing question until
it is resolved or the user explicitly says to move on without a working `herdr`. A `holler` that
is missing, the legacy client or one without the pane verbs, a missing `tmux`, a missing `lib/`
directory (after the install offer above), and a `check-pane` stop are reported as rows of the table
and carried forward: the first ones are Stage 1 and Stage 2 gates, the pane stop a Stage 3 refusal. A
missing `opencode` or `claude` matters only when the config has a pane of that harness (Stage 2).

**Then, only once `herdr` is confirmed present (already installed, or freshly installed and
verified) — ask a second, separate, explicit question, never assumed:** **"All systems go. Proceed?"** (yes/no) A no (or "not right now," or any non-committal
answer) ends the wizard cleanly right here — installing Herdr does not commit the user to also
building a config/workspace in the same run. Only a clear yes moves on to Stage 1.

## Stage 1 — Load the session config

**First run.** A first run has no skill directory: ask, then install (Stage 0's install paragraph). An
existing global `sessions.toml` skips the interview; to start clean, the user moves it aside (for
example `mv ~/.holler/herdr/setup/sessions.toml ~/.holler/herdr/setup/sessions.toml.old`). The
wizard never deletes it. The "no config found" message names the config path it looked for.

**Ask:** yes, if no config exists at any of the three tiers — but as a short scenario menu plus
whatever's genuinely undiscoverable, not a blank "give me the whole schema" ask (see the Do
step's own note on why, and the scenario menu it walks through). Also ask if any `[[pane]]` in an
existing file is missing `name` or `dir` — that narrower case stays a plain question, no menu needed.

**Do:**

Stage 0 already checked the binaries and the `lib/` scripts; use its table. First, check whether the invocation text contains a `--config <path>` flag (or `--config=<path>`)
— Claude Code skills receive their invocation arguments as free-form text after the command name
(whatever followed `/herdr-setup` when it was run), not pre-parsed shell argv, so this is
a text check against that string, not literal `$1`-style argument handling. **If `--config
<path>` (or `--config=<path>`) is present, treat `<path>` as the explicit config file and use it
directly, skipping the cwd/global search below entirely:**

```bash
cat <path parsed out of the --config flag> 2>&1
```

If that file doesn't exist, or `cat`/TOML parsing fails, this is a **hard Gate failure right
here** — see Gate below. Do not fall back to the cwd or global search in that case; the user
named a specific file via `--config`, and silently substituting a different one would be worse
than stopping.

**If no `--config` flag was given**, fall back to the cwd-then-global search, unchanged from
before (the literal `$1`-style bash below is fine here since this branch never depended on
skill-invocation text parsing):
```bash
if [ -f ./sessions.toml ]; then
  CONFIG=./sessions.toml
elif [ -f ~/.holler/herdr/setup/sessions.toml ]; then
  CONFIG=~/.holler/herdr/setup/sessions.toml
else
  CONFIG=~/.holler/herdr/setup/sessions.toml   # does not exist yet — see the hard stop below
fi
echo "config: $CONFIG"; cat "$CONFIG" 2>&1
```
From here on carry the config's path as a literal (`<config>`); the variable above lives only
inside that one command. **If neither exists, this is a hard stop before writing anything — but it is
NOT a full-schema interrogation either.** Two failure modes to avoid, both seen live: (1) auto-writing
the example's own values (`dir = "~/Projects/demo"`, the example model `provider/model-name`) — those
are **placeholder values, not the user's real projects or models**, copied verbatim from
documentation, and on a fresh setup the wizard would sail through validation and build panes in a
directory the new user never chose. (2) swinging the other way and asking a brand-new user to
compose the *entire* schema cold — every pane, its harness, the defaults, `layout` syntax — in one
message. That's not a fix, it's punting the whole interview onto someone who doesn't yet know the
shape well enough to answer it.

Instead: **discover what's discoverable, default what's reasonable, and only ask for what is
genuinely unknowable** — which project directory each pane works in, and which harness it runs;
everything else here can be read or proposed.

1. **Read this machine's OpenCode model before proposing one** (read-only; it is the default every
   OpenCode pane takes):
   ```bash
   cat ~/.config/opencode/opencode.jsonc 2>&1
   ```
   The model is written `<provider>/<model>`: the top-level `model` key if present, otherwise the
   single provider/model pair the `provider` block defines. If the file names no default or defines
   several candidates, **ask the user once** which `<provider>/<model>` to use. Never invent one or
   copy a value from this document.

2. **Offer a scenario menu instead of a blank-slate interview:**
   - **(A) One orchestrator pane, nothing else yet.** Propose a default — `name = "r1c1"`, `dir` =
     the cwd this wizard is running from, `role = "orchestrator"` — and ask one confirm-or-override
     question plus its harness (OpenCode or Claude Code), not separate questions. `layout` is then just
     `[["r1c1"]]`, computed, not asked. When the user wants several panes and does not name them,
     the default names are the grid cells, row then column: `r1c1`, `r1c2`, `r1c3`, `r1c4` for
     the first row, `r2c1` for the first cell of the second row, and so on, with `layout` built
     as columns of those names. If they give a pane count and no grid, propose a grid by one rule
     (two rows for more than four panes, up to 8 panes, for example 8 panes as 2 rows of 4; above 8 panes the squarest grid) and ask one confirm.
     A count that does not fill the last row leaves its short columns on the right, because no
     column may be taller than the column to its left. The user's own names always win. This is a
     fully valid, complete config: one registry pane holding a real orchestrator. Most useful as the
     on-ramp for someone trying the wizard for the first time.
   - **(B) A grid of agent panes with an orchestrator** — the typical shape: ask how many panes (or
     the grid itself), then, in one line per pane, its directory and its harness, then which pane or
     panes are orchestrators (the default orchestrator cell is `r1c1`). One `ls -1 ~/Projects` is a
     fair read-only hint for the directories; it pins nothing.
   - **(C) I'll write or paste my own `sessions.toml`.** Tell them the path Stage 1 will look
     for next (`./sessions.toml` or the global fallback) and stop; re-invoke once it exists.
   - **(D) Something more complex** (several orchestrators in chosen cells, one project per column, an
     OpenCode agent key per pane, a named Herdr session) — fall through to a real interview; only the
     shape of the grid and what each pane does is genuinely the user's call.

   Present this as an actual choice (e.g. via whatever this session's own means of asking a
   multiple-choice question is), not as open-ended prose — a menu is faster to answer than a
   paragraph asking the same thing.

   **For every pane, its harness, asked plainly:** OpenCode or Claude Code, no third choice. For an
   orchestrator say once why there is no free-form command to give it: what a pane holds is its
   harness, which Holler launches, records and reconciles, so a CLI agent or a script of the user's own
   cannot be a pane. **Then the defaults, asked once** as one confirm-or-override: the `model` read in step 1,
   `effort = "medium"`, and the ceilings `ctx_soft = 800000` and `ctx_hard = 900000` (the operator
   guide's own launch example). None of them is a default of Holler, so this is asked, never assumed.
   A Claude Code pane's `model` is asked on its own (`provider/id`, for example
   `anthropic/claude-sonnet-5-5`): the proposal above names an OpenCode model, and a Claude Code pane
   never takes it.

   **Next, once the panes are settled, ask the Herdr session question** (skip it for (C);
   an existing config is not re-asked: its `[instance]` table, or the lack of one, stands, and
   Stage 1's report states which session it targets). Herdr has no session rename, so this is
   decided now, before the workspace is first built. Offer two answers, again as an actual choice:
   - **The default, unnamed session**, recommended when the user has no reason to separate this
     workspace from their other Herdr work. It writes no `[instance]` table, and the behaviour is
     unchanged: the wizard builds in Herdr's unnamed `default` server, and Stage 1's report and
     Stage 3's plan say "default (unnamed)".
   - **A new named session**: a full second instance with its own state directory and ports.
     Suggest a name (for example `work`); it must match `^[a-z][a-z0-9-]{0,23}$` and must not be
     `default`. Check it against every session Stage 0 listed from `herdr session list`, running
     or stopped: building in an existing session is not offered, so refuse a name already listed,
     name the session it matches and its status, and offer the choice of another name or the
     default. For a free name, propose this `[instance]` table:
     ```toml
     [instance]
     name = "<name>"
     herdr_session = "<name>"
     state_dir = "~/.holler-<name>"
     hub_port = 41808
     serve_https_port = 8443
     backend_port_base = 47101
     ```
     The ports are defaults only (nothing is probed here): Stage 3's `collide.sh` stays the
     authority and refuses a taken port, and the user may change any value. Step 3 writes the
     table after the bare top-level keys and before `[[pane]]`, shows the file, and the
     user confirms or edits it before `instance.sh` validates it (Verify, below). Say this once:
     a named session is driven from a terminal outside any Herdr pane (rule (2) of "Two rules for
     a non-default instance", below), and `holler` calls in the user's own shells need
     `HOLLER_STATE_DIR=<state_dir>` (for example `HOLLER_STATE_DIR=$HOME/.holler-work holler hub
     status`), or they reach the default instance instead. A `holler pane` or `holler profile` call
     needs the door, not that variable alone (Stage 11's first note).

   **Then, once the panes and the session are settled, ask the last question: the
   workspace name.**
   Suggest **two dictionary words related to the task**, lower case, joined with a hyphen
   (Discord-style), for example `herald-relay`, `lantern-hub`, `quiet-tower`. Offer one
   suggestion (three alternatives on request); the user accepts, edits or types their own, and
   the answer is written into the config as the top-level `workspace = "<name>"`, before every
   table header. Pick nouns or adjectives that fit what is being built (a message hub, relays,
   agents), never a project's real name, never a person's name. The key is optional: with no
   `workspace` the workspace is named `holler`. It is also the name of the profile the wizard builds.

3. **Write the file from the answers** (on a first run create the directory first:
   `mkdir -p ~/.holler/herdr/setup`), **show it, and let the user confirm or edit before Stage 1
   re-validates it** — don't silently trust your own assembly of their answers into TOML;
   showing the real file is cheap and catches a transcription mistake before Stage 3 builds a
   plan from it. A new named session's `[instance]` table goes after the bare top-level keys
   (the defaults, `layout`, `workspace`) and before the first `[[pane]]`.

Only once a config exists this way — assembled from the user's own real answers, never from the doc's
example values — does Stage 1 proceed to Verify. If the file that *was* found (an existing, real
config) has a `[[pane]]` missing `name` or `dir`, that narrower case is unaffected: ask the user once
for what's missing and write the answer back into that same file before moving on.

**A config of the previous wizard is converted, never rewritten silently.** When `panes.sh check`
(Verify, below) refuses `[[orchestrator]]`, `[[session]]` or one of their keys, the file is the
previous wizard's. Show the user the conversion **as text** first (the confirm step above applies),
and never write over the old file: each `[[orchestrator]]` becomes a `[[pane]]` with
`role = "orchestrator"`, its `name` and `dir`, and its `harness` taken from the first word of its `cmd`
when that word is `opencode` or `claude`; for any other command the `cmd` is not carried, and you ask
which of the two harnesses the pane gets. Each `[[session]]` is dropped — it was a backend on another
machine, and registry panes are local to the hub — and the conversion text names every one. `layout`
keeps the orchestrators' names in their columns, with the dropped names and any column left empty
removed, and must then pass the rule that no column is taller than the one to its left (offer the
reordering when it does not). `hub_host` goes. The defaults (`model`, `effort` and the ceilings) are
asked as above. The converted file is written next to the old one (for example
`sessions.pane.toml`), shown and confirmed, and Stage 1 then loads it as an explicit `--config`.

**Remote sessions are not built.** A user who needs a remote, headless OpenCode session is told,
once, that this wizard does not build one: attach mode (ADR 0014) and `holler body` still serve such a
session, set up by hand (ADR-0021 section 11 says how the two relate).

**Verify:** state plainly **which of the three paths/sources** you actually loaded (an explicit
`--config` flag, cwd, or the global fallback), as an absolute path — this is real, user-visible state that changes
what a later edit to either file would affect. State the Herdr session the config targets: the
`[instance]` table's `herdr_session` ("Herdr session: `<name>`"), or, with no `herdr_session`,
"Herdr session: default (unnamed)". Then run the two validators on the loaded config (its absolute
path) and state their output:
```bash
bash $WIZARD_LIB/instance.sh <config>
bash $WIZARD_LIB/panes.sh check <config>
bash $WIZARD_LIB/panes.sh grid <config>
bash $WIZARD_LIB/panes.sh env <config>
```
`instance.sh` validates the `[instance]` table and the `workspace` key. It prints the resolved instance
(name, prefix, hub port, serve port, state directory, Herdr session, backend port base) and, when the
config has one, `workspace: <name>`; it exits 1 with one message per problem, each naming the key. A
config with no `[instance]` table resolves to all defaults. It refuses a malformed value, a non-default
instance missing `name`, `state_dir` or `herdr_session`, and a non-default instance whose `state_dir` is
the default state directory (`~/.holler`: it would share the first instance's state).

`panes.sh check` validates the panes, and only the panes: it reads the `[instance]` table and the
workspace from what `instance.sh` prints, never again from the file. It exits 1 with one message per
problem, each naming the key: a name outside the grammar or given twice, a name missing from `layout` or
extra in it, a column taller than the column to its left (naming `layout` and the column), a `layout`
spread over several lines (it goes on one line, so that `instance.sh` reads every top-level key after
it, `workspace` included), a relative `dir`, a `harness` or `role` outside its set, no orchestrator, a
pane with no `model`, `effort`, `ctx_soft` or `ctx_hard` of its own or from the defaults, `ctx_soft`
above `ctx_hard`, an `agent` or a `backend_port` on a Claude Code pane, a Claude Code pane with no
`model` of its own, the retired tables and keys (with the word "attach"), and the port rules below.
On success it prints one summary line, and an ignored `hub_host` is one warning line. `panes.sh grid`
prints the workspace, its extent (`RxC`: the first column's height by the number of columns), **the
profile** and one `cell` line per pane in layout order: its cell, name, harness, role, directory (with
`~/` expanded) and, for an OpenCode pane, its port. **Those ports are the only ports every later stage
uses** (the collision plan, the specs, the body): never recompute one from `backend_port_base`.
`panes.sh env` prints the environment literals below.

**Two rules for a non-default instance, stated here because Stage 3 refuses them.** (1) It never
uses the first instance's ports: `instance.sh` refuses a `hub_port` of 41807 and a `serve_https_port`
of 443, and `panes.sh` refuses OpenCode ports that fall in 47001 to 47001 plus the OpenCode pane count
unless every OpenCode pane sets its own `backend_port`, two panes on one port, and a port above 65535
(exit 1, one line naming the key). (2) It is driven from a terminal outside any Herdr pane: a driving
agent that runs inside a Herdr pane cannot establish which session it is in, and Stage 2's `check-pane`
stops it (the fix is to start the wizard from a plain terminal, or from the desktop app, not from a pane).

**Resolve the instance's literals once, and carry them.** After both validators pass, write down
these values; every later command uses them as literals (shell-state rule, top of this file):
- `<name>`, `<prefix>` (default: the name), `<hub_port>` (41807) and `<herdr_session>` (empty for the
  default instance), from the `instance.sh` output above.
- `<profile>`, the profile the wizard builds: the `profile` line of `panes.sh grid`, which is the
  workspace's name (`holler` when the config has no `workspace`). It is computed by `panes.sh`, never
  chosen by you. `<workspace>` is the same name, from the `workspace` line.
- `<apply_session>`, the `--herdr-session` value of the apply lines: `default` for the default
  instance, else `<herdr_session>` (`panes.sh plan` prints it).
- Each OpenCode pane's `<port>`, from its `cell` line.
- `<state_dir>`, **resolved to an absolute path once** and used unchanged everywhere (the ledger,
  `HOLLER_STATE_DIR`, logs, the derived config). With no `state_dir` the one spelling is
  `$HOME/.holler`, resolved: `printf %s "$HOME/.holler"`, or for a `~/` value,
  `printf %s ~/.holler-second`. An absolute `state_dir` is used as given.
- `<logs_dir>` = `<state_dir>/logs`. Create it first with `mkdir -p`.
- `<scratch>` = `<state_dir>/wizard-scratch` on this machine (create it with `mkdir -p`): Stage 2's
  inventory and Stage 7's apply output live there. Never `/tmp`.
- `<herdr_env>`, the Herdr and Holler settings, which go in front of every `herdr.sh` command:
  `WIZARD_INSTANCE_NAME=<name> WIZARD_HERDR_SESSION=<herdr_session> WIZARD_INSTANCE_PREFIX=<prefix> WIZARD_STATE_DIR=<state_dir> WIZARD_LOG_DIR=<logs_dir>`
  followed by the lines `panes.sh env` printed, exactly as printed:
  `WIZARD_HERDR_WORKSPACES='<workspace>=<R>x<C>'` and, for the default instance,
  `WIZARD_HERDR_SOCKET='<socket path>'`, or, for a named instance, `WIZARD_TMUX_SOCKET='<state_dir>/tmux.sock'`
  (add `HERDR_BIN=<absolute path>` when `herdr` is not on `PATH`, and `WIZARD_HOLLER_BIN=<absolute path>`
  when `holler` is not). `WIZARD_INSTANCE_NAME` is mandatory: `herdr.sh` refuses to run without it,
  and the default instance's name is the literal `default`. `herdr.sh holler` also refuses (exit 2,
  nothing run) without an absolute `WIZARD_STATE_DIR`, a named instance without an absolute
  `WIZARD_TMUX_SOCKET`, and the default instance without an absolute `WIZARD_HERDR_SOCKET`: a lost value
  must never reach the default hub, another Herdr session, or tmux's default server. The default
  instance's socket is the default session's path in Herdr's configuration directory (UNVERIFIED against
  a real Herdr until the operator's repeat of the new-user run, #778); a named instance's panes run on
  their own tmux server, so two instances' `r1c1` never meet on one.

**Gate:** Stop on: the files under `$WIZARD_LIB` are missing and the install command did not fix it
(stop and say so); `instance.sh` or `panes.sh check` exits non-zero (stop and show the user its
messages, never guess a fix) — except the retired tables and keys, for which you offer the conversion
above; no config found at any of the three tiers (per the Do step above, this is a hard stop with the
user, never an auto-created file); a missing `name` or `dir` on any pane; or a file that fails to parse
— stop and ask the user to fix the config rather than guessing what they meant. **If an explicit
`--config` flag was given and that file is missing or fails to parse, this is a hard stop with no
fallback** — do not silently substitute the cwd or global config; tell the user the path they gave
doesn't resolve and let them fix it or drop the flag.

## Stage 2 — Config checks

**Ask:** nothing yet.

This stage holds only the checks that need the config. Stage 0 already ran the machine-level
ones (`herdr`, `holler`, `tmux`, `opencode`, `claude`, the `lib/` scripts, the Herdr server, the default
hub and ledger), and they are not repeated here. Every check below is read-only and runs on this
machine: there is no other host.

**Do (each pane's directory and harness).** For every pane of `panes.sh grid`, its `dir` (with `~/`
already expanded in the `cell` line) exists, and its harness's binary is an executable on `PATH`:
```bash
test -d "<the pane's dir, from its cell line>" && echo "<name>: dir ok" || echo "<name>: dir MISSING"
command -v opencode >/dev/null 2>&1 && echo "opencode: $(command -v opencode)" || echo "opencode: NOT on PATH"
command -v claude >/dev/null 2>&1 && echo "claude: $(command -v claude)" || echo "claude: NOT on PATH"
```
(the first line once per pane; the second when the grid has an OpenCode pane, the third when it has a
Claude Code pane). A pane whose `dir` doesn't exist fails its launch deep in Stage 7, and a Claude Code
pane in a directory where Claude Code has never run may stop at its first-run or trust dialog there
(`harness-dialog-open`): say so now, so the user can answer that dialog once by hand, in a plain
terminal in that directory, before Stage 7.

**Do (the OpenCode model, read-only, when the grid has an OpenCode pane).** Re-read this machine's
OpenCode configuration (Stage 1 proposed the model from it) and check that its provider's endpoint
answers:
```bash
cat ~/.config/opencode/opencode.jsonc 2>&1
curl -s <baseURL>/models | head -c 200
```
The model endpoint is the `baseURL` of the provider the config's `model` names. The check is driven by
what the OpenCode config says — never assume a provider or a model. Until #829 an OpenCode pane's server
runs the model its OpenCode configuration selects, whatever the spec records, so this is the model that
matters.

**If the provider block is missing**, don't proceed — the pane's model will fail to resolve. Stop and
ask the user which provider/model to use and where its endpoint lives, then add a block of this shape
(values are the user's, not yours to guess):
```json
"<provider>": {
  "name": "<display name>",
  "npm": "@ai-sdk/openai-compatible",
  "options": { "baseURL": "<endpoint base URL>", "apiKey": "<key, if the endpoint needs one>" },
  "models": { "<model>": { "tool_call": true, "reasoning": true } }
}
```
A provider credential belongs in that OpenCode configuration: a pane's server starts with no shell, so
it never reads a shell rc, and the scrubbed environment of Stage 6 drops the agent's own variables
(UNVERIFIED against a real OpenCode inside tmux until the operator's repeat of the new-user run, #778).

**Do (this instance's own processes and panes).** Stage 0's hub and Herdr lines report the
*default* instance's, which is a different thing when this is a second instance. This instance's own
hub, Herdr server and body are read from its ledger (a live row) and the inventory below:
```bash
HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh list
<herdr_env> bash $WIZARD_LIB/herdr.sh run session list
```
and, **only when the ledger has a live `hub` row** (the instance's hub is up, from an earlier run), the
registry, which needs that hub:
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane list --format=json
<herdr_env> bash $WIZARD_LIB/herdr.sh holler profile list --format=json
```
`data.panes` are the registry's panes, each with its `name`, `pos`, `profile` and `sync`;
`data.unmanaged` (left out when empty) are the Herdr panes no record names, each with its `workspace`,
`pos` and `label`; the profile list says whether `<profile>` exists. Stage 3 judges the build's case
from these.

**Do (the collision inventory — read-only).** Run `inventory.sh` (in `$WIZARD_LIB`) into the scratch
directory (`mkdir -p <scratch>` first):
```bash
WIZARD_INVENTORY_HERDR=1 bash $WIZARD_LIB/inventory.sh > <scratch>/inventory-<host>.tsv
```
(`<host>` names the file, for example this machine's short name). `WIZARD_INVENTORY_HERDR=1` switches
on the Herdr section (the Herdr sessions that exist, and a `missing tool herdr` warning). The first line
of the inventory is `home<TAB><this machine's $HOME>`; `collide.sh` resolves `-` and `~/` state
directories against it. `inventory.sh` lists listening TCP ports, running `holler hub`, `holler body`,
`opencode` and `herdr` processes (pid, owner, state directory where visible — Linux only —, start time,
and for a body its `--config` path), any serve configuration, and the Herdr sessions that exist. It
only reads: it never writes, signals, starts or stops anything. If the inventory cannot be taken, that
is a hard failure for this stage, not something to skip — the plan cannot be checked without it. A line
`warn<TAB>missing tool <name>` means the tool is not on this shell's `PATH`: put it on `PATH`, then take
the inventory again; never plan from an inventory with such a line unless the user says to.

**Do (the Herdr session checks for this instance, before Stage 3):**
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh check-pane      # stops if this run is inside a Herdr pane it cannot place in the instance's session
<herdr_env> bash $WIZARD_LIB/herdr.sh check-session   # stops if the named session exists and the ledger did not create it
```
Both only read, and both take this instance's own Herdr settings (Stage 0's `check-pane` ran with the default instance's). A stop from either is a **Stage 3 refusal**: carry its message into Stage 3, and
do not ask "Proceed with this plan?" (Stage 3 says what to offer). `check-pane` stops when the
driving agent is inside a Herdr pane (the fix is to run the wizard from a terminal outside any
Herdr pane) and when that pane belongs to a different session than the instance's. For the
default instance, with no `herdr_session`, both pass.

The checks above are read-only (they tell Stage 3 what already exists so its plan is
accurate) — none starts, stops, nor changes anything.

**Verify:** every pane's `dir` exists and its harness's binary resolves on `$PATH` (an absolute path,
not an alias); when the grid has an OpenCode pane, the model endpoint returns real JSON (a model list),
not a connection error, and the provider block exists; and you now know whether this instance's own
hub, Herdr server and body are already running (a live row in its ledger and the inventory), and, when
its hub is up, which panes and profiles its registry holds — for Stage 3's summary.

**If `holler` is missing, not on `PATH`, the legacy client, or lacks the pane verbs**, that's a real
gate failure — stop and ask the user how to proceed (e.g. install the current release with
`brew install performant-labs/tap/holler`, or build and put it on `PATH`) rather than substituting
another binary's verbs, which speak a different, incompatible protocol.

**Gate:** any hard failure here (a pane's `dir` missing, its harness's binary not resolving, the model
endpoint down for an OpenCode pane, `holler` without the pane verbs, a missing `tmux`) — stop and tell
the user; every later stage depends on this one being real. A missing `dir` is the user's config to fix
(a typo, a directory that moved) — don't guess a substitute path or auto-install anything on their
behalf.

## Stage 3 — Present the plan, get assent

**Ask:** yes — this whole stage is the Ask. Nothing in Stage 4 onward runs until the user has
explicitly said to proceed.

**Do:** using what Stages 1 and 2 actually found (not a generic template), write out a concrete
plan and show it before touching anything:

- **The grid, from the config.** Print it, read-only:
  ```bash
  bash $WIZARD_LIB/panes.sh grid <config>
  bash $WIZARD_LIB/panes.sh plan <config> <profile>
  ```
  Show the grid as a table: cell, name, harness, role, directory, port (`-` for a Claude Code pane),
  plus the model of each pane; then the workspace, its extent `RxC`, the profile `<profile>` and the pane
  count (how many OpenCode, how many Claude Code, which are orchestrators). The ports are the ones
  `panes.sh grid` printed, never recomputed. Say, in one line each: the model, effort and environment
  of an OpenCode pane are recorded, not applied, until #829 (its server runs what its OpenCode
  configuration selects); the model and effort of a Claude Code pane are applied; and a Claude Code
  pane's body row is written by the operator by hand, not by this wizard (#850), so its roster row and
  Stage 10's round trip exist only when that row does.
  The `plan` lines are the calls Stages 6 and 7 run, in that order: the profile, one `--spec-only`
  launch per pane (which touches nothing live), the dry run, then one apply.
- **Which case this is.** Name which of these holds, from Stage 2's ledger, session list and
  registry; the user's yes covers only that case:
  1. **Fresh**: no profile `<profile>` and no pane in the registry (with no live `hub` row in the
     ledger there is no registry yet; a live one, left by an earlier run that stopped early, is
     reused). Stages 4 to 9 build everything.
  2. **A rerun of this wizard**: a live `hub` row, and the profile `<profile>` exists. Show the
     registry's own plan, read-only:
     ```bash
     <herdr_env> bash $WIZARD_LIB/herdr.sh holler profile apply <profile> --dry-run --herdr-session <apply_session> --format=json
     ```
     Read `data.steps` (each `{pane, action, outcome, code}`), never the exit code: a dry run exits 0
     whatever it plans. Apply is idempotent: it adopts a healthy pane that matches its spec, relaunches
     an unhealthy agent, creates a pane the config added, reports one that differs, and never kills a
     healthy pane; it never relaunches an orchestrator (`orchestrator-not-relaunched`), and it refuses a
     recorded pane whose spec now names another harness (`harness-change-refused`). Each such refusal is
     reported and asked about, as Stage 7 says. A spec in `<profile>` that the config no longer names
     keeps its spec (a `--spec-only` launch never removes one): name it, and offer, with the user's yes,
     `holler pane close NAME --profile <profile> --spec-only` through the door. Stage 6 then runs the
     `--rerun` plan, which leaves out `profile create`.
  3. **The registry already has panes outside `<profile>`** (a live `hub` row, whose registry holds
     panes from an import or launched by hand). The wizard touches none of them. Compare the config's
     names with the registry before anything changes (Stage 2 ran this already; run it again when time
     has passed):
     ```bash
     <herdr_env> bash $WIZARD_LIB/herdr.sh holler pane list --format=json
     ```
     A config name that a record outside `<profile>` already holds is refused here, by this check: apply
     would adopt such a record if it is healthy and matches, report it if it differs, or relaunch it if
     it is an unhealthy agent, so no refusal of Holler's guards this case — the wizard's own pre-check
     does. A cell of the workspace that a pane Holler does not drive already occupies is a create apply
     refuses as `grid-occupied`. The choices: other names in the config, another workspace, another
     instance, or stop. `holler profile create <profile> --from-current`, which puts the un-profiled
     records under the wizard's profile, is offered only if the user asks for it, and so is
     `--take-over` (apply takes a pane from another profile).
  4. **A layout the previous wizard built**: the ledger has `backend` rows (the previous wizard's
     OpenCode backends), or the registry lists unmanaged Herdr panes whose labels are the config's pane
     names. Nothing can adopt it: Holler's import reads the old launcher's files, which no wizard file
     provides. The wizard leaves that layout alone and offers a **second instance** beside it (a new
     `[instance]` name, state directory, ports and Herdr session: Stage 1's session question), or
     stopping. Those are the only two choices: no conversion in place, and no teardown is offered here
     (the ledger teardown of "If a stage fails" stays a separate action the user asks for by name). This
     case is judged before case 5, so a restored saved session that holds the previous wizard's layout
     is this case.
  5. **Restored panes.** A restarted Herdr server restores its saved session, and its panes have no
     record: they are unmanaged, and no Holler verb adopts them. They are never reused, split or
     closed; the wizard builds its workspace beside them with the user's yes (Holler's adapter creates
     it), and refuses a `workspace` name that a restored workspace already uses (the adapter finds a
     workspace by its name, so a restored workspace of that name would be taken for ours, and its
     occupied cells refused as `grid-occupied`): the user then picks another `workspace`.
- **The collision preflight — a refusal, not a warning.** Before anything else in the plan is shown
  as final, run `collide.sh` with the instance's contract values set inline on that command:
  `WIZARD_INSTANCE_NAME`, `WIZARD_STATE_DIR`, `WIZARD_BACKEND_PORTS` (the OpenCode panes' ports from
  `panes.sh grid`), `WIZARD_LEDGER` (this instance's ledger) and the Herdr and hub values:
  ```bash
  WIZARD_HERDR_HOST=1 WIZARD_INSTANCE_NAME=<name> WIZARD_HUB_PORT=<hub_port> WIZARD_SERVE_HTTPS_PORT= WIZARD_HERDR_SESSION=<herdr_session> WIZARD_STATE_DIR=<state_dir> WIZARD_BACKEND_PORTS="<the OpenCode panes' ports>" WIZARD_LEDGER=<state_dir>/herdr/setup/wizard-ledger.toml bash $WIZARD_LIB/collide.sh <host-label> <scratch>/inventory-<host>.tsv
  ```
  `WIZARD_HERDR_HOST=1` marks this machine as the one that runs the Herdr server (the unnamed-server
  refusal below applies there). The call leaves `WIZARD_SERVE_HTTPS_PORT` empty: the hub is served on
  loopback only, so no serve port of the instance can collide. An old serve entry of the instance (the
  previous wizard served the hub to bodies on other machines) is listed as present, not touched: with
  the port empty `collide.sh` no longer calls it "this instance's port pair, no live hub", and the
  wizard never turns it off — removing it is the user's own step. Its output is the inventory printed
  beside what the run will create: everything that runs and was not created by this instance is listed
  as "present, not touched". It exits 1 and prints a `REFUSED:` line — naming the colliding item and the
  config key to change — when the plan needs a port that is in use, a hub port a hub already listens on,
  a state directory another process uses, or a Herdr session of the same name. A planned Herdr session
  counts only when `inventory.sh` ran with `WIZARD_INVENTORY_HERDR=1` (Stage 2). On any refusal, show the
  refusals, **do not ask "Proceed with this plan?"**, and stop until the config is changed and Stage 1
  onward is re-run. Never start, stop or kill the colliding thing to make room: it was not created by
  this run. A stop from Stage 2's `check-pane` or `check-session` is a refusal of the same kind: show
  it, do not ask "Proceed with this plan?". What to offer for each case:
  - **A Herdr session of the instance's name exists and the ledger did not create it**
    (`check-session`, or `collide.sh`'s `REFUSED: a herdr session named ... already exists`). Never
    build in it. If `herdr session list` shows it `stopped`, offer, **with the user's yes**, to delete
    it: `<herdr_env> bash $WIZARD_LIB/herdr.sh session-delete` (it deletes only this instance's own
    session, only when it is stopped and the ledger has no live `herdr` row for it, and refuses
    otherwise), then re-run Stage 2's `check-session`. If it is running, or the user says no, the
    answer is another `herdr_session` in the config, and Stage 1 onward is re-run.
  - **The default instance and a running unnamed Herdr server with no live `herdr` row in the
    ledger** (`REFUSED: an unnamed herdr server is already running`): that unnamed Herdr server is
    foreign. Ask one question: build in it (the user's own yes to using the server; "build in it" means
    a new workspace in that server, which Holler's adapter creates when Stage 7's apply makes its first
    pane, never anything done to the panes it already has; Stage 5 records nothing for it), or stop
    (set a `herdr_session` in an `[instance]` table, which makes the instance non-default). Never stop
    that server.
- **The Herdr session.** The plan names it in both cases: "Herdr session: default (unnamed)" for
  the default instance (no `herdr_session`), or "Herdr session: `<name>` (new, own state directory
  and ports)" for a named one, followed by its `state_dir` and ports.
- **The workspace's name.** As for the session, the plan names it ("workspace `<workspace>`", `holler`
  when the config has no `workspace`), and the profile `<profile>`, which has the same name.
- **A new workspace beside restored panes** (a restored saved session, or "build in it" on a foreign
  server): name the new workspace in this plan and list the restored panes beside it (they stay
  untouched). The plan's one yes ("Proceed with this plan?") is the user's yes to that: the
  "Proceed with this plan?" yes covers creating it, and a "no" means stop.
- **Teardown and the restored workspace.** The Herdr server the wizard starts also holds any
  restored workspace of the user (with any auto-resumed agent), and teardown stops that server:
  list those panes here and ask before any teardown. For the default instance, offer to leave the
  server running and stop only the hub.
- **What's already running vs. what will be started fresh.** "Reused" means exactly one thing for a
  process: a **live row in this instance's own ledger** (its pid, start time and command still match;
  see Stage 4, "Reuse on a rerun"). List which of the hub, the Herdr server and the body are reused
  that way and which will be started new; the panes are the registry's, and case 2's dry run says which
  are adopted. When `herdr session list` shows a stopped session of the instance's name (for the
  default instance (no `herdr_session`) the entry named `default`), say so here: starting the Herdr
  server restores that saved session (its workspaces and panes, and an agent in a pane may resume), so
  Stage 5 lists them after `server-start`, and Stage 7 builds the new workspace beside them. A listener
  on a planned port that has no live row is not reused: it is a collision (above), however much it
  looks like ours.
- **Anything that would be stopped.** Only ledger-recorded processes (ownership rule, top of this
  file) may be planned for stopping, and only panes of `<profile>` may ever be closed (through Holler);
  anything else holding a planned port or name is reported under this heading as a conflict to ask
  about. Be explicit and specific — "I'll stop this instance's recorded body (PID 12345, live in its
  ledger) because the hub's identity key changed and it can no longer reconnect" is a real plan; "I'll
  clean things up as needed" is not. If nothing needs stopping, say that plainly too.
- **The end state**: the grid of panes in the workspace `<workspace>`, in the columns and rows `layout`
  specifies, every pane recorded in the registry under `<profile>`, the hub and one body on this
  machine, and every OpenCode pane verified with a real round-trip reply.

**Also ask, as its own explicit yes/no, separate from "proceed with this plan":** whether to
install/update the Holler self-status briefing in each orchestrator's own `AGENTS.md` — the
fix for an orchestrator flailing on "what is `<pane>` doing" (confirmed live 2026-09-21/22:
before this briefing existed, the orchestrator had no reason to know it could just run `holler
roster`; after, it answered correctly, checked in real time). It's a real edit to a file the
orchestrator's agent reads on every startup, so it's a genuine yes/no like any other file write
this wizard proposes, not a silent default. Offer it when the grid has a pane besides the orchestrators
(something to drive). If yes, Stage 7 writes/updates the briefing block before the apply launches that
orchestrator — see Stage 7 for the exact text (the briefing never embeds the instance's state directory
path; a second instance gets its own headed section). Only orchestrators are briefed: an agent pane is
driven, it does not drive, and it answers "what are you doing" from its own conversation.

Then ask directly: **"Proceed with this plan?"** — and wait for a real yes before continuing.
A vague or non-committal reply is not assent; ask again or ask what to change.

**Verify:** the user's response is an explicit go-ahead (or an explicit no / a requested
change).

**Gate:** anything other than clear assent — STOP. If they want changes, revise the plan and
present it again; don't half-start based on an ambiguous answer. If they decline, stop the
whole run here — Stages 1 and 2 already ran (read-only, nothing to undo) but nothing else does.

## Stage 4 — Bring up the hub

**Ask:** nothing further — Stage 3 already covered whether the hub is reused (a live ledger row) or
started fresh.

**The instance's values.** Stages 4 to 9 use the literals Stage 1 resolved (an absent `[instance]`
table means every default): `<state_dir>` (resolved, absolute; for the default instance
`$HOME/.holler`, which is Holler's own default), `<prefix>`, `<hub_port>`, `<profile>`, `<herdr_env>`,
and each OpenCode pane's `<port>` **exactly as `panes.sh grid` printed it**. Never pick a port, and
never derive one from `backend_port_base`: `panes.sh` already applied the rule for this config.

**Logs.** Never write a log under `/tmp` (a RAM-backed tmpfs on these hosts; filling it has
taken a fleet down). Every log Stages 4 to 9 create goes in `<logs_dir>` = `<state_dir>/logs`,
named `<prefix>-...` (Stage 5 passes it to `herdr.sh` as `WIZARD_LOG_DIR`). Create it first, before
the first use: `mkdir -p <logs_dir>`.

**The ledger.** Every process this wizard starts in Stages 4 to 8 — the hub, the Herdr server and the
body — is recorded, once it is up, in `<state_dir>/herdr/setup/wizard-ledger.toml` (contract: epic
#726; the file is the only source for "what this run created"). `ledger.sh` has three verbs and reads
the state directory from `HOLLER_STATE_DIR` in its environment (unset means `$HOME/.holler`, and then it
would silently use the default instance's ledger), so **every** call sets it inline:
```bash
HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh record --pid <pid> --role <hub|herdr|body> --stage <4-9> [--session <name>]
HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh list        # pid, live|stale, role, stage, session, cmd (tab separated)
HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh owns <pid>  # exit 0 live, 1 stale (pid reused or process gone), 2 not recorded
```
Record **after** the stage's Verify passes for that process, never while it is still starting (its
command line changes when the shell hands over to the program). Never record a pid you did not start,
and never signal a pid whose `owns` is not 0. **Every start command below prints the pid of the program
itself** (a `{ nohup ... & echo $!; }` group, not `cmd && nohup X & echo $!`, which on some login shells
prints the pid of a subshell): record that pid, then run `list` and confirm the recorded `cmd` starts
with the program (`holler`, `herdr`). A row whose `cmd` is `bash` or `sh` is a wrapper: it would be
stale as soon as the real program runs. Delete nothing by hand; stop, find the real pid and record that
one. The panes are never recorded here: Holler launched them, its registry records them, and they stop
only through `holler pane close`.

**Reuse on a rerun.** Before starting anything in Stages 4 to 8, look the process up in the ledger by
role and session (`list`). A row that is `live` and whose command still has the port or arguments you
are about to use is **reused**: do not start another one, and say so in the stage's report. A `stale`
row, or no row, means start a new process and record it. Never adopt a process that is not in the
ledger, even if its command line looks identical (it may belong to another instance) — if the port is
taken by an unrecorded process, that is Stage 3's collision gate, stop and report it.

**A restart re-runs the stage's own start command.** `stop-owned.sh restart <state_dir> <pid>`
stops the recorded process and prints `RESTART-CMD <recorded command>`. That line only
identifies the entry: the recorded command has no environment (`HOLLER_STATE_DIR`), no `nohup`
and no log redirect, and run alone it would use the default state directory. So run the stage's
own start command again (the one below for the hub; Stage 8's for the body), with its
`HOLLER_STATE_DIR=<state_dir>`, `nohup` and log path, then record the **new** pid and run `list` to
confirm it.

**`serve_https_port` is unused.** It stays a key of the `[instance]` table that `instance.sh` validates
(a named session's proposal still sets one), and the wizard no longer uses it: the hub listens on
loopback only, advertises no address and gets no serve entry, because nothing on another machine dials
it. `<hub_port>` and `<serve_https_port>` are the instance's (defaults 41807 and 443); only the first is
used.

**Never stop or replace a hub that the ledger did not record** (ownership rule, top of this file):
run `bash $WIZARD_LIB/stop-owned.sh check-port <state_dir> <hub_port>` before the start block below;
free, or held by this instance's live `hub` row, means go on, and a foreign or stale holder is
reported and the run stops and asks.

**Do**, only if the plan called for a fresh hub (skip entirely if the ledger has a live `hub` row,
which means the hub is reused). `mkdir -p <logs_dir>` first, then start it, printing the program's own
pid:
```bash
{ HOLLER_STATE_DIR=<state_dir> nohup holler hub serve --listen 127.0.0.1:<hub_port> > <logs_dir>/<prefix>-hub.log 2>&1 & echo $!; }
```
Note the printed pid as a literal (it is not kept anywhere else). The hub's state — token store,
pepper, identity key, control socket, and the pane registry and profiles — is the instance's own
`<state_dir>`, so a second instance beside this one shares none of it.

**Verify:**
```bash
HOLLER_STATE_DIR=<state_dir> holler hub status
```
Shows `listening: 127.0.0.1:<hub_port>` and a real PID behind it — the hub process actually
started. Then **record** it with the pid printed above, and confirm with `list` that the recorded
`cmd` starts with `holler`:
```bash
HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh record --pid <pid> --role hub --stage 4
```

**Gate:** if `hub serve` fails to start, stop and report the real error (port already bound by
something else, a stale lock, etc.) — every pane and profile verb answers `unavailable` without a
live hub, so nothing past this stage can run against a hub that isn't actually up.

**If a recorded hub was already running and this is a resume** (its identity key didn't come from
a fresh `hub_identity_generated` log line): this instance's already-live body from an earlier
run may reconnect on its own without needing Stage 8's join again — check
`HOLLER_STATE_DIR=<state_dir> holler roster` before redoing Stage 8. If the hub's identity key *is*
fresh (new `hub serve` invocation, no persisted key), the body **this instance's ledger recorded** is
orphaned (pinned to the old key) — this should already be called out in Stage 3's plan as something
that will be stopped (through `stop-owned.sh`) and rejoined. A body that is in no ledger of this
instance is not this run's to stop or rejoin, whatever it looks like.

## Stage 5 — Start the Herdr server

**Ask:** nothing further — Stage 3's plan already said whether the Herdr server is reused, built in
(a foreign unnamed server the user said yes to), or started fresh.

**Every Herdr command in this stage goes through `herdr.sh`** (in `$WIZARD_LIB`), written in full as
`<herdr_env> bash $WIZARD_LIB/herdr.sh <verb> ...`, where `<herdr_env>` is the literal settings from
Stage 1 (`WIZARD_INSTANCE_NAME`, `WIZARD_HERDR_SESSION`, `WIZARD_INSTANCE_PREFIX`, `WIZARD_STATE_DIR`,
`WIZARD_LOG_DIR=<logs_dir>`, the `panes.sh env` lines, plus `HERDR_BIN` when `herdr` is off `PATH`).
They are set inline on **every** command, never exported: a lost export would send a bare `herdr`
command to the default session. `herdr.sh run` puts `--session <herdr_session>` in front of the words
after it, and only for `status`, `session list` and `workspace list`: every other word exits 2, names
`holler pane`, and reaches no Herdr. **A bare `herdr server stop` is never issued, by this skill or by
the wrapper**: which server it stops is not established, and it could stop another instance's. The
wrapper refuses to run when `WIZARD_INSTANCE_NAME` is unset, and when the instance is not the default
one and sets no `herdr_session`.

**Do (session check, before anything starts):**
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh check-pane      # stops if this run is inside a pane of a DIFFERENT Herdr session
<herdr_env> bash $WIZARD_LIB/herdr.sh check-session   # stops if the named session exists and the ledger did not create it
```
These ran in Stage 2 and passed or were refused in Stage 3; run them again here, since time has
passed. If `check-session` stops, tell the operator the session's name and ask (Stage 3 lists what
to offer, including `session-delete` for a stopped session); never build in a session this wizard did
not create (it counts as created only when the ledger has a **live** `herdr` row for it). If
`check-pane` stops, build nothing. **For the default instance, a running unnamed Herdr server with no
live `herdr` row is foreign** (Stage 3's question "build in it, or stop"): only with the user's yes to
building in it does the wizard use that unnamed Herdr server, and the wizard never stops it; without
that yes, stop here and ask. "Build in it" means a new workspace in that server, which Holler's adapter
creates in Stage 7: the panes it already has are never touched.

**Do (server check, first, only if the plan called for a fresh server):**
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh run status
```
Herdr 0.8.x prints the line `status: not running` or `status: running` under `server:`; the
older `server: not running` text stays accepted. If it is not running, start one headlessly (`herdr`
alone needs a real TTY and will fail with "cannot attach without a usable terminal" when run
non-interactively — use the headless form; its log is `<logs_dir>/<prefix>-herdr-server.log`, see
`herdr.sh log-path`). The default instance starts an unnamed server; a named session starts a server
for that session, in a scrubbed environment (the agent's own variables never reach it). The **first
line** of `server-start`'s output is the server's pid:
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh server-start
sleep 3
<herdr_env> bash $WIZARD_LIB/herdr.sh run status   # should now show `status: running` under `server:`
```
Then record the server in the ledger, with that pid, so a later run knows this wizard created the
session (for the default instance, leave out `--session`):
```bash
HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh record --pid <pid> --role herdr --stage 5 --session <herdr_session>
```
and confirm with `list` that the recorded `cmd` starts with `herdr`. A live `herdr` row for this
instance means the server is reused: do not start another.

**Do (prove that Holler reaches this Herdr).** This is the first Holler pane call; it goes through the
door like every other (Stage 6 says why):
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane doctor
```
Its first line prints Herdr's version: that is the one read that proves Holler's adapter reaches *this*
instance's Herdr session through the socket `<herdr_env>` names. A `herdr-version-unsupported` finding
is a Gate: no Holler verb upgrades Herdr; report it and ask. An `unregistered-herdr-pane` finding is
expected here (the server's own first pane, and any restored pane).

**Restored saved session.** Starting the server restores the previous saved session of that name (its
workspaces and panes, and an agent in a pane may resume). List them through Holler:
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane list --format=json
```
They are `data.unmanaged`, each with its `workspace` and `label`. Never reuse or close them, and
report them to the user. When their labels are the config's pane names they are the previous wizard's
layout — Stage 3's case 4: stop and ask (a second instance, or stop). When a restored workspace has the
config's `workspace` name, stop and ask for another name (Stage 3's case 5). Otherwise Stage 7 builds
the workspace beside them.

**What a fresh server holds is not established** (UNVERIFIED until the operator's repeat of the
new-user run, #778): it may already show one workspace with one pane. That pane is the server's, not the
wizard's: the wizard's workspace is a new one that Holler's adapter creates on the first create, Stage 9
counts that pane as an expected `unregistered-herdr-pane`, and Stage 11 says how to close it.

**Gate:** if the server does not start, read its log (`herdr.sh log-path`) before anything else. If
`pane doctor` answers `unavailable`, the hub (Stage 4) or the socket in `<herdr_env>` is wrong: check
both, never fall back to a Herdr command.

## Stage 6 — Create the profile

**Ask:** nothing further — Stage 3's plan already listed every call of this stage.

**Every Holler pane and profile call goes through one door**, written in full as
`<herdr_env> bash $WIZARD_LIB/herdr.sh holler <words>`: it runs `holler <words>` unchanged in a
scrubbed environment — the same allow-list as the Herdr server's, without any `HERDR_*`, so none of the
driving agent's own variables (its credentials, its own Herdr pane, its tmux client) passes — with the
instance's literals set: `HOLLER_STATE_DIR`, `HOLLER_HERDR_SESSION` (`default` for the default
instance), `HOLLER_HERDR_WORKSPACES`, and `HOLLER_HERDR_SOCKET` or `HOLLER_TMUX_SOCKET`. Holler's verbs
are configured from the environment of the process that runs them, and the first call that creates a
pane starts the instance's tmux server with that environment, which every pane then inherits
(UNVERIFIED: tmux's own rule, checked by the operator's repeat of #778). A bare `holler pane ...` from
your own shell would carry your variables into every pane: it is never run.

**Do:** print the calls (the agent orders nothing; `panes.sh` does):
```bash
bash $WIZARD_LIB/panes.sh plan <config> <profile>            # a first run
bash $WIZARD_LIB/panes.sh plan <config> <profile> --rerun    # case 2: <profile> exists
```
Each printed line is `TAG ARGS`: `profile-create`, then one `spec` per pane in layout order, then
`dry-run` and `apply` (those two are Stage 7's). The ARGS are already shell-quoted: run each line of
this stage, in order, as `<herdr_env> bash $WIZARD_LIB/herdr.sh holler ARGS`, copying the ARGS as
printed. For the example grid of "The config file" the first lines are:
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler profile create <profile>
<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane launch r1c1 --profile <profile> --spec-only --workspace <workspace> --grid r1c1 --project <dir> --role orchestrator --model <model> --effort medium --ctx-soft 800000 --ctx-hard 900000 --port-policy fixed:<port>
```
A `spec` line writes the pane's spec into `<profile>` and touches nothing live: no Herdr call, no tmux
session, no server, so it needs no `--herdr-session`. A Claude Code pane's line carries
`--harness claude --port-policy none` and no `--agent`; an OpenCode pane's carries
`--port-policy fixed:<port>` and its `--agent` when it has one. The wizard passes no `--env` at all.
On a rerun, `profile create` would refuse `profile-exists`: the `--rerun` plan leaves that line out, and
the `spec` lines rewrite the specs in place. Each call answers within its own bound.

**Verify:**
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler profile show <profile>
```
Lists every pane of the config, at its cell, with its harness and role.

**Gate:** any line that exits non-zero — show its message and the code it names (for example
`port-policy-unsupported` or `agent-unsupported`, which `panes.sh check` should already have caught),
then stop and ask. Never edit a printed line to get past a refusal.

## Stage 7 — Build the panes

**Ask:** nothing further — Stage 3's plan named the workspace and listed any restored panes beside it,
and the "Proceed with this plan?" yes covers creating it; a "no" means stop. The workspace is always a
new workspace in that server: Holler's adapter creates it when the apply makes its first pane, whether
the server is fresh, holds restored panes, or is a foreign unnamed server the user said to build in; the
wizard never touches a pane it did not create.

**First, if Stage 3's agent-instructions question was answered yes, the briefing.** An orchestrator's
agent reads its `AGENTS.md` when it starts, and no command is ever typed into a pane, so the briefing is
written before the apply launches the orchestrators: ensure an `AGENTS.md` exists in each orchestrator's
own `dir` (creating one if none exists, appending a clearly-marked section if one does — never overwrite
unrelated content) containing at least:
```markdown
## Holler self-status

You are an orchestrator in a Holler-driven multi-agent setup. Every session you're
driving is a real, separate `holler` roster entry, but its **live roster name is namespaced**
(e.g. `<body-label>/<session-name>`), not just the bare config name. Your hub is the one whose
state directory is not yours to pass: this pane's environment already sets `HOLLER_STATE_DIR`,
and you must never change or unset it, or `holler` reaches a different hub (or none).
Run `holler roster` with no filter first to see the real names,
then use `--prefix <that full name>` to narrow to one.
Use that same full name with `holler say <full name> "…"` to
dispatch and `holler interrupt <full name>` to redirect one that's
stuck. Don't guess a session's status from memory or from what you last told it to do — check
`holler roster`.
```
The text names no orchestrator: orchestrators that share a `dir` share one file, so a per-orchestrator
name would be wrong for all but one. **The briefing never carries the state directory's
path**: `AGENTS.md` may be shared with another instance's orchestrator (the same project directory),
and a path written into it would send that orchestrator to this hub. The pane's own environment
carries the value: every pane starts with the instance's `HOLLER_STATE_DIR`, from the tmux server that
`herdr.sh holler` started (UNVERIFIED until #778's repeat; Stage 10's fresh-session check is the proof).

- **Only one instance uses this `AGENTS.md`** (the default instance, or a directory no other
  instance's orchestrator reads): if it already has the plain self-status section, skip this
  sub-step; if Stage 3's answer was no, skip it too.
- **The section must be instance-specific** (another instance's orchestrator reads this file too):
  head it `## Holler self-status (instance <name>)` with the instance's own `<name>`, and add it
  alongside the other section. If another instance's section already exists (a different
  `(instance ...)` heading, or the plain heading written for the first instance), do not edit it:
  stop and ask the user which sections to keep. If a section with this instance's own heading
  already exists, skip this sub-step. Two things verified live 2026-09-22, both worth getting right in
  the briefing rather than guessing: there is no `holler status <name>` verb (`roster --prefix` is the
  real per-session filter), and `--prefix` matches the roster's live namespaced session id, not the
  config file's bare `name` — a bare-name `--prefix` silently matches nothing rather than erroring.

**Then the dry run, once more, before the first live change** (the `dry-run` line Stage 6 printed):
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler profile apply <profile> --dry-run --herdr-session <apply_session> --format=json
```
Read `data.steps`. On a fresh build every step is `create`, in creation order: the workspace's column 1
by row, then every other cell by row and column, which is the order Herdr can build in. Stop and ask
if the steps differ from the table Stage 3 showed (a pane added or missing; on a fresh build an
`adopt`, `report` or `relaunch` step means a record of that name exists, which is Stage 3's case 3), or
if a step is `refused` or `reported`: a dry run exits 0 whatever it plans, so its steps are the Gate
here. A dry run issues no split, so it cannot show `grid-unreachable`: only the real run finds that.

**Then the apply, once:**
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler profile apply <profile> --herdr-session <apply_session>
```
It creates every pane of the grid on its own, in that creation order, adopts a healthy pane that already
exists and matches, and relaunches an unhealthy agent. The wizard launches no pane outside this one
call.

**How long it takes, and its timeout.** Apply has no overall cap: reading the profile and observing its
panes takes up to 10 s, and each `create` adds up to the launch's own 10 s budget (a cold `opencode serve`
may need all of it), plus a port check and, on a refusal, a rollback — call it 15 s per create. State
the expected duration before running it (10 s plus 15 s per `create` step of the dry run: a 16-pane
build is about four minutes), and give the command a timeout above it, or run it in the background with
its output in `<scratch>/<prefix>-apply.out` and read that when it ends. Your shell tool's own timeout
(two minutes by default in Claude Code) must never end an apply: a killed apply can leave a step that
had acted unreconciled.

**The Gate reads the exit code and what the run printed:**
- **Exit 0**: it printed `profile <profile>: N steps, applied` and one `PANE: ACTION, OUTCOME` line
  per step. A step whose outcome is `reported` (a healthy pane that differs from its spec, left alone)
  is a Gate: show its differences, name `holler profile show <profile>` and
  `holler pane relaunch PANE --profile <profile>`, and ask. Every other step is `created`, `adopted` or
  `relaunched`; a `refused` or `failed` step never appears in an exit-0 run.
- **Exit 3**: at least one `refused` step and no failed one. Every step ran, nothing was rolled back,
  and the run printed only one line, `error: apply refused <N> step(s): <pane> (<code>), ...; <M> other
  step(s) ran`. Each refused code is a Gate: `orchestrator-not-relaunched` (apply never relaunches an
  unhealthy orchestrator: report it and ask; with the user's yes it is relaunched with
  `holler pane relaunch PANE`, through the door), `pane-in-other-profile` (Stage 3's case 3; `--take-over` only if the
  user asks), `harness-change-refused` (a recorded pane whose spec now names another harness: close it
  and launch it again, with the user's yes), `grid-unreachable` (a new row below a live row of two or
  more panes: apply moves nothing live to make room, so ask — the pane can go in a new column, or the
  user may close and rebuild that column), `grid-occupied` (the cell holds a pane Holler does not
  drive), and a launch's own refusals, such as `port-in-use` or `process-in-use` (a tmux session of
  that name already runs Claude Code: the wizard adopts or doubles nothing). The panes created around a
  refusal stay, and a rerun adopts them; `holler pane list` shows what was built.
- **Exit 1**: a `failed` step stopped the run, the later steps were skipped, and only that failure was
  printed (`apply stopped at <pane> (<action>): ...`). `harness-dialog-open` means a Claude Code pane
  stopped at its first-run or trust dialog and its launch was rolled back: the answer is by hand —
  answer the dialog once in that directory in a plain terminal, then re-run Stage 7. A `timeout` on a
  cold server is a Gate with a question too. Once the user fixed the cause, a rerun of Stage 7
  continues where it stopped.
- **Exit 2**: a usage error — the line was not typed as `panes.sh` printed it.

A Gate: show it, name the command it names, ask; never work around it, and never fall back to a Herdr
command. Apply's own creation order builds every grid `panes.sh check` accepts, so no other build order
exists to fall back to.

**Verify:** the run exited 0 and every step is `created`, `adopted` or `relaunched`.

## Stage 8 — Join and run the body

**Ask:** nothing — you have everything from Stages 1, 4 and 7.

**One body on this machine drives every OpenCode pane.** A body is how the hub drives a session: it
attaches to the session each pane's server holds (attach mode, ADR 0014; it is not that server's
parent). **This stage derives no row for a Claude Code pane** (#850: that row attaches to the pane's
process, and the operator writes it by hand from the README's "A Claude Code pane"): Stage 3 said so,
and Stage 10's round trip covers a Claude Code pane only when that row exists. One body serves all the OpenCode panes, so restarting it reconnects every
one of them at once. With no OpenCode pane in the grid, this stage has nothing to do: report it skipped
(⏭️), never failed.

**Do, in three parts.** First, **derive the body config from the registry**, never from the config
file: for each OpenCode pane,
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane get <name> --format=json
```
and read `data.pane.harness.port` and `data.pane.session_of_record`. A pane with no port or no session
of record is a Gate: no body is started without one. Write one `[[session]]` row per OpenCode pane, and
nothing else, to `<body_config>`:
```toml
[[session]]
name = "<name>"
harness = "opencode"
mode = "attach"
endpoint = "http://127.0.0.1:<data.pane.harness.port>"
session_id = "<data.pane.session_of_record>"
```
`<body_config>`, the body config's path, is defined once, here:
`<state_dir>/herdr/setup/<prefix>-sessions.toml`, in the wizard's own directory (every file the wizard
keeps lives there). Re-read it before using it: only `[[session]]` rows, each `endpoint` ending in its
own pane's port and each `session_id` its pane's session of record.

Second, **mint one token for this body**, with a label that names the instance, `<prefix>-local`, so a
roster entry, a token list row or a process inspected later says which instance it belongs to. Check
what is there first (`HOLLER_STATE_DIR=<state_dir> holler hub token list`), then:
```bash
HOLLER_STATE_DIR=<state_dir> holler hub token mint --label <label>
```
`<label>` is the Stage 8 token label: `<prefix>-local`, written as a literal in the command (never a
bare `body` or a host name, which says nothing about which hub and instance the body belongs to).

**A label stays taken until its token's record is deleted.** `revoke` keeps the record by design (an
audit trail of what was cut off), so a revoked token still holds its label; `hub token delete` removes
an `unused` or `revoked` token's record and frees the label (holler #454, releases after v0.3.0). A
mint over a taken label fails with `label "<prefix>-local" already in use by <state> token <token_id>;
free it with: ...`, naming the exact commands. To reuse the label of a body you've confirmed is done,
revoke the old token if `HOLLER_STATE_DIR=<state_dir> holler hub token list` still shows it `bound`,
delete it, then mint again:
```bash
HOLLER_STATE_DIR=<state_dir> holler hub token revoke <old_token_id>   # only if it is still bound
HOLLER_STATE_DIR=<state_dir> holler hub token delete <old_token_id>
HOLLER_STATE_DIR=<state_dir> holler hub token mint --label <label>
```
The new token has a new token id, so the body joins again with the new join line. On holler v0.3.0
and earlier nothing frees a label: there, append a counter and mint `<prefix>-local-2`, then `-3`.
A minted token's join secret is valid for 24 hours by default (`--ttl`), and once a body has joined its
token does not expire (holler #453): the body keeps authenticating until
`HOLLER_STATE_DIR=<state_dir> holler hub token revoke <token_id>` ends it.

Third, **join and run the body** on this machine, with the hub's own `<state_dir>` (`mkdir -p
<logs_dir>` first):
```bash
HOLLER_STATE_DIR=<state_dir> holler body join --server ws://127.0.0.1:<hub_port> --token <token_id>:<secret> --hub-key <hub_key>
{ HOLLER_STATE_DIR=<state_dir> nohup holler body run --config <body_config> --debug quiet > <logs_dir>/<prefix>-holler-body.log 2>&1 & echo $!; }
```
`<token_id>:<secret>` and `<hub_key>` are what the mint printed. The join URL is loopback `ws://` to the
hub's own port: the body runs on the hub's machine. `body join` and `body run` get the **same**
`HOLLER_STATE_DIR`: the token's pairing is stored under it, and a body started with another directory
finds no pairing. Note the pid the last command prints, and after the body shows up in
`HOLLER_STATE_DIR=<state_dir> holler roster` (Verify, below), **record** it and confirm with `list` that
the recorded `cmd` starts with `holler`:
```bash
HOLLER_STATE_DIR=<state_dir> bash $WIZARD_LIB/ledger.sh record --pid <pid> --role body --stage 8 --session <the first OpenCode pane's name>
```
Reuse rule: a `live` `body` row whose recorded `cmd` names the expanded path of `<body_config>` (the
recorded command line holds the absolute path) means the body is reused — but only when the body config
it reads still matches the registry. A pane added or removed, or relaunched onto a new session of record,
needs a new body config: derive it again, stop the recorded body through `stop-owned.sh`, and run it
again. A restarted wizard that finds the row `stale` runs it again and records the new pid.

**Verify:**
```bash
HOLLER_STATE_DIR=<state_dir> holler roster
```
Shows `<label>/<name>` `connected` for every OpenCode pane; since #648 a roster row joined to a pane
also prints the pane's cells. The hub lists only the bodies paired to its own state directory: a body
of another instance can neither appear here nor join, its token does not exist in this store.

**Gate:** every OpenCode pane must show as `connected` — a partial roster is a real failure for the
missing panes, not a partial success. A `no such token` error means the mint and the join ran against
different state: stop and check `HOLLER_STATE_DIR` on both commands rather than re-minting blindly.

## Stage 9 — Check the registry

**Ask:** nothing. This stage reads the registry through Holler; it types into no pane and reads no
pane's screen.

**Do:**
```bash
<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane doctor --format=json
<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane list --format=json
<herdr_env> bash $WIZARD_LIB/herdr.sh holler profile show <profile>
```
`pane doctor` exits 0 with or without findings: read `data.findings`. The gate is **no finding except
`unregistered-herdr-pane`**: that kind is expected for the Herdr server's own first workspace pane and
for restored panes (Stage 11 says how to close them). Any other finding — `shown-driven-mismatch`,
`profile-drift`, `pane-waiting`, `server-down`, `claude-login-missing`, and any kind a later Holler adds
— is a Gate: show it, name the command it names, ask. `pane list` shows every pane of the config, at its
cell (`pos.pos`), with `profile` = `<profile>` and SYNC `ok` (SHOWN, the session the pane's screen
shows, equals its session of record). DRIVEN prints `-` until #839 wires it: a `-` there is not a
failure. `profile show <profile>` lists no differing field. Each call answers within its own bound.

**The login is never touched.** The skill never runs `claude auth` or `claude login`, never reads,
prints or copies a login, and never names a credential variable's value: a Claude Code pane's process is
started without the variables that would change its login, so it uses the login Claude Code itself holds.
Whether that login is usable comes only from `holler pane doctor`'s `claude-login-missing` (asked once
per pass when a Claude Code pane is in scope). The answer is: log in once, by hand, in a plain terminal
outside the wizard, then re-run Stage 9.

**Verify:** the three reads above agree: every pane present at its cell in `<profile>`, SYNC `ok`, and
no finding but `unregistered-herdr-pane`.

**Gate:** any other finding, a missing pane, a pane at another cell, or SYNC not `ok` — show it and ask.
Never repair a pane by typing into it, and never fall back to a Herdr command: the command a finding
names, run through the door with the user's yes, is the repair.

## Stage 10 — End-to-end verification, don't just assume it worked

**Ask:** nothing.

**First, verify what exists:** the hub answers (`HOLLER_STATE_DIR=<state_dir> holler hub status`
shows `listening: 127.0.0.1:<hub_port>`) and every orchestrator pane is alive (Stage 9's `pane list`
shows it healthy at its cell, and `pane doctor` found nothing about it).

**Do**, for every OpenCode pane:
(`<label>` is the Stage 8 token label.)
```bash
HOLLER_STATE_DIR=<state_dir> holler say <label>/<name> "reply with just: <name> ready"
```

**Verify:** the reply appears in the CLI output here for every OpenCode pane, a real round trip through
the hub, the body and the pane's own server. Repeat for every OpenCode pane — a partial pass (2 of 3
verified) is a real gap for the untested one, not good enough. A Claude Code pane has a round trip only
when the operator wrote its body row by hand (#850; this wizard derives none): run the same `holler say`
for it then; otherwise say so plainly, and count it verified by Stage 9 alone.

**If Stage 3's AGENTS.md briefing was installed, prove it with a genuinely new session — a
resumed one proves nothing.** An orchestrator with old conversation history may already "know" about
Holler from its own past turns, not from the briefing. Confirmed live 2026-09-23: an auto-resumed
orchestrator's recap already mentioned earlier `holler say` calls, so it would have passed any check
regardless of whether `AGENTS.md` worked. A pane this run created is a fresh session; on a rerun that
adopted the orchestrator, ask before giving it one, since it drops the conversation, and with the
user's yes run `holler pane reset PANE --as-operator` through the door
(`<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane reset PANE --as-operator`): the orchestrator's pane
changes only as the operator's deliberate act, and without `--as-operator` the reset is refused as
`orchestrator-pane`. Then say what the reset leaves. An OpenCode orchestrator has a new session of
record, so its body row is stale: derive Stage 8's body config again and restart the body (Stage 8's
reuse rule) before the test below. Its old session stays on its server (no Holler verb deletes a
session), and a doctor pass reports it as `stray-session` from then on: when Stage 9's gate shows it,
say it is the session this reset left. A Claude Code orchestrator is relaunched on a fresh session id:
it has no body row and no server to keep a stray session. Then ask the user to attach (Stage 11's
first row) and give that orchestrator a task with **no Holler context at all** — e.g. "Send a hello to
`<an agent pane's name>`". Pass only if it discovers the roster on its own (`holler roster` in its own
pane), finds the namespaced name (`<label>/<name>`), and gets a real reply.

**Gate:** a CLI reply that never lands for a given OpenCode pane means its body row is stale (the pane
was relaunched onto a new session of record, or the body was not restarted after a change): derive the
body config again and restart the body (Stage 8) before anything else.

**Every stage green or skipped = done.** Report the final layout (which cell holds which pane, its
harness and role) and every pane's session of record (from `pane list`), so the user has them on hand
for a later repair. Also print, with every value written out, the two literals that a pane verb needs
in the user's own shell: the instance's `<herdr_env>` (Stage 1) and the skill's absolute `lib` path
(the value of `$WIZARD_LIB`). A pane verb the user runs goes through the same door as the wizard's
own (`<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane ...`, Stage 11's first note). Never give the
user a pane verb with `HOLLER_STATE_DIR` alone or with no environment: without the instance's Herdr
session and tmux socket, a `holler pane close` stops the processes of the tmux session of that name on
tmux's default server, which may be another instance's pane, and only then fails on the missing
Herdr session, leaving the record and the Herdr pane in place. Then give the user precise
instructions to actually **see** the workspace, since everything above ran headlessly through Holler
and Herdr's socket — nothing was visible on screen yet:

1. Tell the user to open a terminal and attach (Stage 11's first row has the exact command, with
   the instance's `herdr_session` when it has one), then switch to the workspace `<workspace>`
   (`prefix+w`, or click it in the sidebar), where the grid is: every pane in `layout`'s columns and
   rows. A fresh server may also hold its own first workspace (one pane, not the wizard's).
2. Then continue to Stage 11, which holds every command for looking around and cleaning up.

## Stage 11 (final): Basic Herdr commands

**Ask:** nothing. This step changes nothing: it is output only, shown after the verification
report. Print this table (the bindings are Herdr's defaults, checked against `herdr --default-config`
and Herdr's keyboard docs; if a binding differs on the user's install, `prefix+?` shows every active
binding):

| Task | Command |
|---|---|
| Attach | `herdr` (or `herdr --session <name>`, the instance's `herdr_session`) |
| The prefix key | `ctrl+b`, release, then the key |
| Show or hide the sidebar | `prefix+b` (or click the `»` at the bottom-left) |
| Switch workspace | `prefix+w`, or click the sidebar |
| New / rename / close workspace | `prefix+shift+n` / `prefix+shift+w` / `prefix+shift+d` (also `herdr workspace close <id>`) |
| Split right / down | `prefix+v` / `prefix+minus` |
| Move between panes | `prefix+h/j/k/l` |
| Zoom / close pane | `prefix+z` / `prefix+x` |
| Detach, leave everything running | `prefix+q` |
| Every binding | `prefix+?` |

Then add three notes:

- **The wizard's panes are Holler's.** Moving between them, zooming one and detaching are fine; to
  close, restart or replace one, use Holler through the wizard's door, with the `<herdr_env>` and the
  `lib` path Stage 10's report printed (write both out here too):
  `<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane close NAME --profile <profile>` to close one, and
  `<herdr_env> bash $WIZARD_LIB/herdr.sh holler pane relaunch NAME` to restart one. Never `prefix+x`,
  or the registry keeps a record of a pane that is gone (a doctor pass then reports it), and never a
  bare `holler pane` verb in your own shell, with only `HOLLER_STATE_DIR` or with no environment: a
  close then stops the processes of the tmux session of that name on tmux's default server, which may
  be another instance's pane, and fails, leaving the record and the Herdr pane in place.
- **A restored workspace:** if `herdr.sh run workspace list` shows any workspace the wizard did
  **not** create beside the new one (a restored saved session, or the fresh server's own first
  workspace), name it with its id and pane count and say how to close it if the user wants
  (`prefix+shift+d` on it, or `herdr workspace close <id>`). Never close it for them.
- **Teardown:** the wizard's own teardown closes the panes through Holler, then runs
  `stop-owned.sh teardown`; warn again that it closes the restored workspace too (the server it
  stops holds it).

## If a stage fails

- **A pane died, or `holler pane doctor` names one, and the others are fine:** that is a real gap for
  that one pane only. Doctor names the finding (`server-down`, `herdr-pane-missing`,
  `tmux-session-missing`, ...) and the command to run (usually `holler pane relaunch PANE`): run it
  through the door, with the user's yes, and touch nothing else. Apply never relaunches an orchestrator,
  so an orchestrator's relaunch is always that one command. A relaunched OpenCode pane may hold a new
  session of record: derive Stage 8's body config again and restart the body.
- **`HOLLER_STATE_DIR=<state_dir> holler roster` shows a pane `connected` but its server does not
  answer:** the pane's server itself is down. `holler pane doctor` confirms it (`server-down` or
  `server-wedged`); relaunch it as above, never by name and never with a hand-written `kill`.
- **Tearing the instance down:** first, the Herdr server the wizard started also holds any restored
  workspace of the user (with any auto-resumed agent), and teardown stops that server: list those
  panes (`holler pane list` shows them as unmanaged) and ask before teardown. For the default instance,
  offer to leave the server running and stop only the hub, with the body that dials it
  (`stop-owned.sh stop <state_dir> <pid>` for the body, then for the hub, instead of a whole teardown,
  after the panes are closed). Then, **with the hub still
  live**, close the panes through Holler: `bash $WIZARD_LIB/panes.sh teardown <config> <profile>` prints
  one `pane close NAME --profile <profile>` per pane (reverse layout order), then `profile delete
  <profile>`; run each line, in order, through the door (`<herdr_env> bash $WIZARD_LIB/herdr.sh holler
  ARGS`). A close stops the pane's harness, closes its Herdr pane and removes its record; a
  `pane-not-found` for a pane this run never built is not a failure. Only then run
  `bash $WIZARD_LIB/stop-owned.sh teardown <state_dir>`: it stops only ledger processes, in reverse
  start order (the body, the Herdr server — so the named Herdr session stops with it; what Herdr keeps
  of that session's saved layout is Herdr's own, report it and do not delete it by hand — and the
  hub), removes only the ledger, and prints what it left (stale entries, foreign processes, other files,
  other instances). **If the hub is gone** (a stale `hub` row), the panes cannot be closed through
  Holler: say so and ask; never fall back to a Herdr command, and never close a pane by hand. **The
  instance's tmux server keeps running:** a close ends a pane's processes but not its tmux session, and no
  ledger row records the tmux server, so the wizard never stops it. Say what stays and how the user ends
  it: for a named instance, its own server on `<state_dir>/tmux.sock` (`tmux -S <state_dir>/tmux.sock ls`
  shows its sessions, and the user may end it by hand with `tmux -S <state_dir>/tmux.sock kill-server`);
  for the default instance, the sessions named as the panes in the user's own tmux server, which stays
  theirs. Pass `--purge-state` only when the user asked to remove the instance's whole state directory,
  and not while that tmux server still runs (its socket is inside it); it is refused for the default
  state directory. Once the Herdr server has stopped, the named session stays listed as `stopped`: with
  the user's yes, delete it with `<herdr_env> bash $WIZARD_LIB/herdr.sh session-delete` (own session,
  stopped only), never with a bare `herdr session delete`. An old serve entry of the instance (from the
  previous wizard) is not the wizard's: never `tailscale serve reset`, which wipes every serve entry.
- **A join fails with `no such token`:** the token mint and the join must run against the same hub,
  with the same `<state_dir>` (Stage 8). Every command touching the hub (mint, token list, join, run)
  carries `HOLLER_STATE_DIR=<state_dir>` inline — check that before re-minting.
- **The config changed (a pane added, removed or moved) since the panes were built:** don't reconcile
  implicitly — go back to Stage 3: the `--rerun` plan and the dry run show what changes, a removed pane
  keeps its spec in `<profile>` until `holler pane close NAME --profile <profile> --spec-only` (with the
  user's yes), and a pane added below a live row of two or more panes is `grid-unreachable` (Stage 7).
  A pane whose harness changed is closed first (`holler pane close NAME --profile <profile>`, with the
  user's yes): a recorded pane never changes its harness (`harness-change-refused`), and its stored spec
  refuses the other harness's port policy. Get assent again before touching the workspace.
- **A Claude Code pane stops at its first-run or trust dialog** (`harness-dialog-open`): answer the
  dialog once in that directory in a plain terminal, by hand, then re-run Stage 7. The wizard types into
  no pane.
- **A live, working setup already exists and nothing is actually broken:** don't re-run this wizard
  against a working setup. Verify with `HOLLER_STATE_DIR=<state_dir> holler roster` and Stage 9's reads
  before concluding anything is actually wrong, and if it isn't, say so instead of rebuilding.

## Related

- Operational rules and the reasoning behind each step: this repo's `docs/setup-wizard.md`
  (its "How the wizard builds panes" is the design)
- The pane and profile verbs, their exit codes and findings: `docs/pane-control.md`; the contract:
  `docs/adr/ADR-0021.md` (section 11 says how attach mode relates to registry panes)
- `docs/adr/ADR-0014.md` (attach mode, which the body uses to drive each OpenCode pane's session)
- Herdr itself (the workspace/pane tool Stage 0 can install): `https://herdr.dev` /
  `https://github.com/herdrdev/herdr` — a separate project from Holler; its own install script
  (`https://herdr.dev/install.sh`) is what Stage 0 runs on a yes.
