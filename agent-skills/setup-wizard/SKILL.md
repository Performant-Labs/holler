---
name: setup-wizard
description: Install Herdr (if not already present, asked as its own up-front yes/no) and, as a genuinely separate follow-up question, optionally set up a Herdr workspace — one or more local orchestrator panes plus one remote agent session pane per entry in a Holler session config, wired through a Holler hub (local) / body (remote) pair, using the safe `opencode attach` recipe (never herdr-mirror). Config-driven, not hardcoded — every orchestrator's name/command/directory, every session's harness, and the whole pane layout (including how many orchestrators there are and where each one sits) are all read from the config, never assumed. Use when the user asks to install Herdr, set up Herdr with a local orchestrator + remote agent sessions, recreate the alpha/beta (or N-agent, or multi-orchestrator) setup, or rebuild that workspace.
---

# Herdr: M local orchestrators + N remote agent sessions — wizard

Rebuilds a working Herdr + Holler setup verified live 2026-09-21/22 on a real two-machine
deployment. This repo's `docs/setup-wizard.md` carries the operational rules and the reasoning
behind each step's shape — read it if anything here looks surprising. The examples use one
Claude orchestrator and OpenCode session harnesses because that's what was verified live —
**don't read that as a hardcoded requirement.** Nothing about the Herdr/Holler mechanism itself
is Claude- or OpenCode-specific, and nothing about it requires exactly one orchestrator either —
those are just this pattern's most-exercised instance.

**The Holler session config file is the base model for everything this wizard builds.** It is
not just an artifact produced at the end — read it *first*, and every later stage's cardinality
(how many orchestrators to launch, how many backends to start, how many session IDs to
capture, how many panes to split and attach, and where every one of those panes sits) is
derived from the config you actually read. Never hardcode an orchestrator count, a session
count, any name, any command, any directory, or a harness anywhere in this run.

Target shape: one local Herdr workspace, arranged into one or more columns per the config's own
`layout` field — every orchestrator pane and every session pane is a real, explicit entry in
that grid (no assumed "there is exactly one orchestrator" or "it's always the fixed left pane"
— however many orchestrators the config's `[[orchestrator]]` list has, and wherever `layout`
puts each one, is what gets built), driven by a Holler hub locally and a Holler body on the
remote machine.

**Never use `herdr-mirror` for this.** Three separate confirmed incidents (in the handbook
doc's "Mirror gotchas" section) include one where the mirror daemon's own reconciliation
killed the real remote processes, not just their viewer panes. Everything below uses plain
local panes + `opencode attach`, which never touches the mirror daemon.

**What's genuinely generalized here vs. what's still OpenCode-specific:** every orchestrator
(command, working directory, and there can be more than one) is fully config-driven and can be
any CLI agent. The session side already carries a per-entry `harness` field (per
`docs/adr/ADR-0005.md`'s config shape) — but **Stage 4 (starting a backend) and Stage 9
(attaching a live view) only have real, implemented logic for `harness = "opencode"` today.** A
config entry with a different harness value is real config Holler itself may already support
for spawn-mode sessions, but this wizard's own backend-start and live-view mechanics haven't
been built for anything but OpenCode's attach-mode HTTP server yet — see the explicit gate on
this in Stages 4 and 9, don't silently assume another harness works the same way.

## Connections this wizard's setup actually makes

Holler has its own wire protocol — this is what Stage 2's checks (`tailscale status`, hub port
41807, the OpenCode model endpoint) are really probing, so it's worth stating explicitly rather
than leaving it implicit in scattered preflight commands. Four distinct connections, verified
against the real ADRs in the `holler` repo (`docs/adr/ADR-0004.md`, `ADR-0005.md`, `ADR-0006.md`)
rather than assumed:

1. **Hub ↔ body ("the circuit")** — the only connection that crosses machines. JSON-RPC 2.0 over
   a WebSocket (ADR 0004). The hub binds **loopback only** (`ws://127.0.0.1:41807`); the
   supported cross-machine path is `tailscale serve --bg 41807` proxying that to
   `wss://<hub-host>.<tailnet>.ts.net` (port 443, ADR 0006) — this is Stage 6's `hub serve` +
   `tailscale serve` and Stage 7's `body join`/`body run`. Authenticated by a minted token
   (`circuit/join`) plus a Noise XK handshake (`circuit/authenticate`/`circuit/prove`,
   ADR-0019) — a tailnet IP is **never** trusted as identity on its own (ADR 0006 point 4).
2. **Body ↔ its own backend agent ("the body hop")** — for `harness = "opencode"` in attach
   mode, plain HTTP on **loopback only**, `http://127.0.0.1:<port>` on the *body's own machine*
   (ADR 0005) — attach mode has no remote-endpoint form, which is exactly why Stage 4 starts
   `opencode` on the same remote host the body itself runs on, never elsewhere. Co-location is
   per body, not global — **different sessions can live on different remote hosts**, each with
   its own body process; that's exactly what each session's own `remote_host` field controls
   (see "The config file" below) and what Stage 7 groups by, rather than assuming one host for
   everything.
3. **The backend agent ↔ its model** — e.g. OpenCode ↔ a local or hosted model provider, as
   configured on each remote host (its `opencode.jsonc` provider block). This is the backend's
   own concern, not part of Holler's wire protocol at all — Stage 2 checks it only because a
   session can't do anything useful without it.
4. **This wizard's own Herdr control** — a **local Unix socket**
   (`~/.config/herdr/herdr.sock`), never a network connection, and entirely unrelated to
   Holler's wire. Stage 8 talks to it to build/inspect panes; it has nothing to do with how
   sessions actually communicate.

Only connection 1 ever needs a firewall/tailnet consideration; 2-4 are all loopback or a local
socket, on whichever single machine each one lives on.

## The config file

**One file, single source of truth — resolved in this order:**

1. **An explicit `--config <path>` flag passed to this skill's invocation** (e.g.
   `/setup-wizard --config /path/to/some/sessions.toml`, or `--config=<path>`) — if the
   invocation text contains this flag, `<path>` *is* the config file, full stop. It takes
   priority over both of the searches below, and neither of them runs at all in this case.
2. `./sessions.toml` (the current working directory the wizard is invoked from) — lets a
   project keep its own config, taking priority over the fallback when present.
3. `~/.config/setup-wizard/sessions.toml` — the global default/fallback.

Use whichever is found **first**, don't merge across tiers. An explicit `--config` path (tier 1)
that doesn't exist or fails to parse is a hard failure right there — it does **not** fall
through to tiers 2 or 3; see Stage 1's Gate. State which of the three you're actually using when
you report Stage 1's result — don't leave it ambiguous which config is in effect.

Everything this wizard needs to know — every orchestrator, every session, the whole layout, and
which machine everything remote actually runs on — is authored in whichever file that is. The
user edits one file, not several:

```toml
# LAYOUT — one field, a nested array: outer = columns, left to right; inner = each column's
# panes, top to bottom. This is the ENTIRE workspace layout — every `[[orchestrator]]` name and
# every `[[session]]` name is a real slot in this grid, nothing assumed. Every `[[orchestrator]]`
# name must appear in `layout` exactly once; every `[[session]]` name must appear exactly once;
# `[[orchestrator]]` and `[[session]]` names must not collide with each other (one flat
# namespace) — Stage 1 validates all of that. This example — o1 alone in its own column,
# alpha/beta stacked in a second column — is today's usual shape, but it's not assumed.
#
# MUST come before every table header below (`[[orchestrator]]`, `[[session]]`) — a bare
# top-level key placed after any table header is parsed as belonging to THAT table, not the
# file's top level. This bit the wizard's own real config once; don't repeat it.
layout = [["o1"], ["alpha", "beta"]]

# THIS machine's own tailnet FQDN — the hub advertises it so remote bodies can dial back.
# (Bare top-level key for the same "must come before any table header" reason as layout.)
hub_host = "hub.example.ts.net"

# Orchestrators — one or more. Each is real, named config: `name` is how `layout` below refers
# to it, `dir` is its working directory, `cmd` is commonly "claude" but can be any CLI agent (an
# opencode TUI, a plain shell script, whatever briefing that orchestrator needs). Each drives
# whichever sessions it's briefed to via Holler's own CLI (`holler say <name> "…"`, `holler
# roster`, `holler interrupt <name>`) — the wizard itself doesn't assign sessions to
# orchestrators; that's a matter of what each orchestrator is told to do, not layout.
[[orchestrator]]
name = "o1"
dir = "~/Projects/holler"
cmd = "claude"

# `harness` is real, per-session config (per docs/adr/ADR-0005.md) — "opencode" is what this
# wizard's Stage 4/Stage 9 actually know how to start and live-view today; a different value is
# valid Holler config but this wizard will flag it as unimplemented rather than guess a
# mechanism for it (see Stages 4 and 9's Gates).
#
# `remote_host`/`remote_tailnet_host` are PER SESSION, not a single global value — attach mode
# requires a session's body and backend to be co-located (ADR 0005), but that co-location can
# happen on a DIFFERENT host per session/body-group. `remote_host` is the SSH alias/hostname
# (matching your `~/.ssh/config` or a plain resolvable name); `remote_tailnet_host` is that same
# machine's tailnet FQDN, used to build the `wss://`/`http://` URLs the hub and `opencode
# attach` actually dial — not always identical to `remote_host` (an SSH alias and a MagicDNS
# name are different namespaces, even when they point at the same box). Stage 7 groups sessions
# by `remote_host` and runs one Holler body per distinct host, not one body for everything.

[[session]]
name = "alpha"
harness = "opencode"
mode = "attach"
endpoint = "http://127.0.0.1:47001"
remote_host = "remote-a"
remote_tailnet_host = "remote-a.example.ts.net"
# session_id filled in by Stage 5 — leave absent or blank in the template

[[session]]
name = "beta"
harness = "opencode"
mode = "attach"
endpoint = "http://127.0.0.1:47002"
remote_host = "remote-a"
remote_tailnet_host = "remote-a.example.ts.net"
```

**Two orchestrators, with sessions split across two different remote hosts, looks exactly like
more entries in the same lists — no different mechanism, and this is where per-session
`remote_host` actually earns its keep:**
```toml
layout = [["o1"], ["alpha", "beta"], ["o2"], ["gamma", "delta"]]   # must come first — see above
hub_host = "hub.example.ts.net"

[[orchestrator]]
name = "o1"
dir = "~/Projects/holler"
cmd = "claude"

[[orchestrator]]
name = "o2"
dir = "~/Projects/other-project"
cmd = "claude"

# alpha, beta: remote_host = "remote-a" (both on the same remote machine, one body)
# gamma, delta: remote_host = "remote-b" (both on a different machine, a second body)
# ... [[session]] entries for alpha, beta, gamma, delta, each with its own remote_host/
#     remote_tailnet_host ...
```
That's 4 columns: o1 alone, alpha/beta stacked, o2 alone, gamma/delta stacked — 6 panes total,
2 orchestrators + 4 sessions, all real data in one file — and **2 real Holler bodies**, one on
`remote-a` (hosting alpha/beta) and one on `remote-b` (hosting gamma/delta), because Stage 7 groups by
`remote_host`. Which sessions o1 vs. o2 actually *drive* is up to how each is briefed (its own
`AGENTS.md`/prompt, per the "MO" (multiple-orchestrator) pattern) — `layout` only controls where
panes sit on screen, not who talks to whom over Holler, and is independent of which host each
session's body actually runs on.

If any `[[orchestrator]]` entry is missing `name`/`dir`/`cmd`, ask the user once (in Stage 1)
for whatever's missing — don't default `cmd` to `"claude"` silently just because that's the
common case; state the default you're proposing and let them confirm or override it, and write
the answer back into this same file — don't ask again on every run.

**This file can be passed to `holler body run --config` as-is.** Holler's parser
(`crates/holler-body/src/config.rs`) still denies unknown keys at the top level and per session
(typos stay errors), but it knows `hub_host`, `layout`, `[[orchestrator]]` and each session's
`remote_host`/`remote_tailnet_host` and ignores them, validating only their types (strings; `layout`
an array of arrays of strings). Other tools may add their own data under a top-level
`[ext.<namespace>]` or per-session `[session.ext.<namespace>]` table, which the body also ignores.
Stripping is therefore no longer required, but the wizard still writes a derived copy per distinct
`remote_host` (Stage 7) so each remote host receives only its own `[[session]]` tables: for **each
distinct `remote_host`**, a *separate* copy containing only that host's `[[session]]` tables (the
wizard-only keys may be dropped), used for that host's own `scp`/`--config` (one file, one
`holler body run`, per distinct `remote_host`). The master file — whichever of the three sources
Stage 1 actually loaded (explicit `--config` flag, cwd, or the global fallback) — keeps every field, including
`hub_host`, every `[[orchestrator]]`, `layout`, every session's `remote_host`/
`remote_tailnet_host`, and the real `session_id`s once captured, for next time. Don't
hand-author separate files by hand — one master file in, the per-host files (one
per host) are *generated*, every run.

## How to run this: one stage at a time, gated on real verification

This is a **wizard, not a script to dump and run**. Work through the stages below **in order**,
starting at **Stage 0 — Install Herdr**, which stands apart from everything after it: it is
about getting the `herdr` binary onto this machine, not about building any particular workspace,
and it ends with its own explicit yes/no before Stage 1 (config + launch) is even offered.

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

1. State which stage you're starting (`Stage N/10: <name>`, N running 0 through 10 — 11 stages
   total, Stage 0 plus the 10 build-and-verify stages below it).
2. If the stage has an **Ask** line, ask the user that question and wait for the answer before
   doing anything else in the stage — don't guess a pane id, port, hostname, orchestrator
   name/command/directory, or harness-specific behavior that varies by session state or config.
3. Run the **Do** commands — for stages 4/5/8/9, this means looping over every session (and, in
   Stage 8, every orchestrator) in the config you read in Stage 1, not a fixed alpha/beta pair
   or a fixed single orchestrator, and using each entry's own `harness`, not assuming
   `opencode`.
4. Run the **Verify** command and read its real output — never assume success from a command's
   exit code alone if Verify gives you something more concrete to check (a session id, a
   `connected` row, a reply's content).
5. Report one line: what you did, what Verify actually showed.
6. **Gate:** if Verify passed, move to the next stage automatically. If it failed, don't retry
   blindly, don't skip ahead, and don't improvise a fix silently — but also don't just stop and
   hand the problem back unresolved. Look up the matching entry in "If a stage fails" below, tell
   the user plainly what broke, and ask them the one concrete question that unblocks it (a
   missing value, a yes/no on killing something, which of two real options they want) — then act
   on their answer and resume the stage sequence from wherever it left off. Only end the run
   early if the user's answer is itself "stop," or if a stage's own Gate is explicit that no
   answer can resolve it (an unimplemented harness, for instance).

**Stage 3 is a hard stop, not just a gate like the others: nothing that changes real state
(starting a process, killing one, joining, splitting a pane) may happen before the user has
explicitly said to proceed.** Stages 1 and 2 are read-only (loading a file, checking status) so
they're safe to run ahead of that; everything from Stage 4 onward is not.

Keep a running status checklist as you go — one line per stage, name plus symbol, not a dense
bracket-number line (a human reading it mid-run shouldn't have to cross-reference a legend to
know what's happening):

```
✅ 0. Install Herdr
✅ 1. Load the session config
✅ 2. Preflight
🔄 3. Present the plan, get assent
⬜ 4. Start remote agent backends (N=2: alpha, beta — harness: opencode)
⬜ 5. Capture session IDs
⬜ 6. Bring up the hub
⬜ 7. Join + run the body
⬜ 8. Build the Herdr workspace (M+N panes)
⬜ 9. Attach panes to sessions
⬜ 10. Verify end-to-end
```

✅ done · 🔄 in progress (the stage you just started) · ⬜ not started yet · ❌ failed/stopped
(pair with a one-line reason). Print the whole checklist after every stage transitions, not
just the numbers that changed. Include the real M (orchestrator count), N (session count), and
every name/harness once known (from Stage 1 onward), not a generic "N sessions" placeholder —
the point is a human can read it at a glance, including which harness is actually in play.

## Stage 0 — Install Herdr

**This stage is about the `herdr` binary only — installing it locally, on this machine.** It has
nothing to do with Holler, with any remote host, or with building a workspace; those are Stage 1
onward, and whether to even get there is a second, separate question asked only at the end of
this stage. Never fold this stage's install question and Stage 1's "configure and launch"
question into one combined ask — a yes to installing Herdr is not a yes to also setting up a
workspace in the same run.

**Ask:** check first, so the question is informed rather than asked blind:
```bash
command -v herdr >/dev/null 2>&1 && herdr --version || echo "herdr: not found"
```
If `herdr` is already on `PATH`, say so (name the version found) and skip straight to this
stage's closing question below — there's nothing to install. If it's not found, ask plainly:
**"Herdr isn't installed on this machine — do you want to install it?"** Wait for a real yes or
no; don't install anything before an explicit answer.

**Do**, only if the user said yes and `herdr` wasn't already found:
```bash
curl -fsSL https://herdr.dev/install.sh | sh
```
This is the canonical installer from the real `herdrdev/herdr` project (verified against its own
`README.md`, `https://github.com/herdrdev/herdr`) — it installs only the `herdr` binary itself
for the current user on this machine; it does not touch any remote host (that's Stage 2 onward,
and only for `holler`, a separate binary/project entirely — installing Herdr here says nothing
about whether `holler` is present anywhere, that's still checked fresh in Stage 2). If the user
says they'd rather use Homebrew (a real, equally valid path on a Homebrew-managed machine),
`brew install herdr` is the same install, just via a different channel — use whichever the user
asks for; don't default to one over the other without asking if they have a preference.

**Verify:**
```bash
command -v herdr >/dev/null 2>&1 && herdr --version || echo "herdr: still not found"
```
**A real Homebrew/Linuxbrew-installed `herdr` (or `holler`, same issue in Stage 2) can still
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
you're doing so (once, not on every command).

**Gate:** if the user said no to installing, stop this stage here — do not fall through to
asking about Stage 1 in the same breath; report plainly that Herdr isn't installed and the wizard
is stopping, and let the user re-invoke later if they change their mind. If the install command
itself fails, report the real error (network failure, permission issue, unsupported platform) and
don't proceed to the closing question until it's resolved or the user explicitly says to move on
without a working `herdr`.

**Then, only once `herdr` is confirmed present (already installed, or freshly installed and
verified) — ask a second, separate, explicit question, never assumed:** **"Do you want to
configure Herdr now and launch a workspace?"** A no (or "not right now," or any non-committal
answer) ends the wizard cleanly right here — installing Herdr does not commit the user to also
building a config/workspace in the same run. Only a clear yes moves on to Stage 1.

## Stage 1 — Load the session config

**Ask:** yes, if no config exists at any of the three tiers — but as a short scenario menu plus
whatever's genuinely undiscoverable, not a blank "give me the whole schema" ask (see the Do
step's own note on why, and the scenario menu it walks through). Also ask if any
`[[orchestrator]]` entry is missing `name`/`dir`/`cmd` in an existing file — that narrower case
stays a plain question, no menu needed.

**Do:**

First, check whether the invocation text contains a `--config <path>` flag (or `--config=<path>`)
— Claude Code skills receive their invocation arguments as free-form text after the command name
(whatever followed `/setup-wizard` when it was run), not pre-parsed shell argv, so this is
a text check against that string, not literal `$1`-style argument handling. **If `--config
<path>` (or `--config=<path>`) is present, treat `<path>` as the explicit config file and use it
directly, skipping the cwd/global search below entirely:**

```bash
CONFIG=<path parsed out of the --config flag>
cat "$CONFIG" 2>&1
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
elif [ -f ~/.config/setup-wizard/sessions.toml ]; then
  CONFIG=~/.config/setup-wizard/sessions.toml
else
  CONFIG=~/.config/setup-wizard/sessions.toml   # does not exist yet — see the hard stop below
fi
cat "$CONFIG" 2>&1
```
**If neither exists, this is a hard stop before writing anything — but it is NOT a four-category
interrogation either.** Two failure modes to avoid, both seen live: (1) auto-writing the
example's own values (`hub_host = "hub.example.ts.net"`, `remote_host = "remote-a"`) —
those are **placeholder values, not the user's real infrastructure**, copied verbatim from documentation, and on a
genuinely fresh setup the wizard would sail through validation and Stage 3 could end up trying
to actually SSH into a real host the new user never chose. (2) swinging the other way and asking
a brand-new user to compose the *entire* schema cold — orchestrators, every session's four
remote fields, `hub_host`, `layout` syntax — in one message. That's not a fix, it's punting the
whole interview onto someone who doesn't yet know the shape well enough to answer it.

Instead: **discover what's discoverable, default what's reasonable, and only ask for what is
genuinely unknowable** — a real remote machine's own address is the one thing that can't be
invented or guessed; everything else here can be.

1. **Check whether `tailscale` is even there before trying to use it for anything** — never
   assume it:
   ```bash
   command -v tailscale >/dev/null 2>&1; echo "tailscale: exit $?"
   ```
   If that's non-zero, this machine has no `tailscale` client (a real, unremarkable case —
   nothing about this wizard *requires* Tailscale specifically, it's just what this pattern's
   been verified against). Skip straight to asking for `hub_host` and any `remote_host`/
   `remote_tailnet_host` values directly — don't block on it, don't pretend it's there. It's
   fine to mention, once, in passing while asking (e.g. "if these machines are on a Tailscale
   tailnet, I can look values up live instead of you typing them — let me know if `tailscale`
   is installed somewhere I should check") — a suggestion offered while doing the real work,
   never a requirement gating it.

2. **If it is there, discover `hub_host` before asking anything** — it's a live fact about
   *this* machine, not a decision:
   ```bash
   tailscale status --self --json 2>&1
   ```
   Use `.Self.DNSName` (strip the trailing dot) if that succeeds. If the command itself fails
   (installed but not logged in / not running), fall back to asking — same as the "not
   installed" case above, just reached a different way.

3. **Offer a scenario menu instead of a blank-slate interview:**
   - **(A) Just a local orchestrator, nothing remote yet.** Needs only an orchestrator. Propose
     a default — `name = "o1"`, `dir` = the cwd this wizard is running from, `cmd = "claude"` —
     and ask one confirm-or-override question, not three separate ones. `layout` is then just
     `[["o1"]]`, computed, not asked. This is a fully valid, complete config: a real Herdr pane
     running a real orchestrator, with no SSH/tailnet knowledge required at all — and doesn't
     even touch the `tailscale` question above, since there's nothing remote to look up. Most
     useful as the on-ramp for someone who hasn't set up a remote body yet.
   - **(B) A local orchestrator plus one or more remote attach-mode sessions.** Same
     orchestrator default as (A), then — and only then — the genuinely unknowable part: for
     each remote session the user actually wants (ask how many, don't assume a count), get its
     `remote_host`. If `tailscale` is available (step 1), offer real, live candidates before
     asking them to type one blind:
     ```bash
     tailscale status 2>&1
     ```
     lets you show actual reachable peers by name, so the user can pick one instead of
     recalling/typing a hostname from memory — this is discovery of real machines, not
     invention of a fake one. `remote_tailnet_host` is that same peer's own tailnet DNSName from
     the same `tailscale status` output (again read, not asked, once they've picked the host).
     If `tailscale` isn't available, just ask for `remote_host`/`remote_tailnet_host` directly —
     that's a completely normal path, most machines aren't on a tailnet at all. Only the session
     `name` and the OpenCode port are genuinely up to the user either way — ask those together,
     one line per session, not as four separate prompts each.
   - **(C) I'll write or paste my own `sessions.toml`.** Tell them the path Stage 1 will look
     for next (`./sessions.toml` or the global fallback) and stop; re-invoke once it exists.
   - **(D) Something more complex** (multiple orchestrators, sessions split across hosts, an
     existing config to extend) — fall through to a real interview, but even then, use
     `tailscale` for discovery only when step 1 found it available, the same way (A)/(B) do;
     only the shape of *how many* orchestrators/sessions and what each does is genuinely the
     user's call.

   Present this as an actual choice (e.g. via whatever this session's own means of asking a
   multiple-choice question is), not as open-ended prose — a menu is faster to answer than a
   paragraph asking the same thing.

4. **Write the file from the answers, show it, and let the user confirm or edit before Stage 1
   re-validates it** — don't silently trust your own assembly of their answers into TOML;
   showing the real file is cheap and catches a transcription mistake before Stage 3 builds a
   plan from it.

Only once a config exists this way — assembled from the user's own real answers, plus live
`tailscale status` discovery when it's actually available, never from the doc's example values
— does Stage 1 proceed to Verify. If the file that *was* found (an existing, real config) has an `[[orchestrator]]` entry
missing `name`/`dir`/`cmd`, that narrower case is unaffected: ask the user once for what's
missing and write the answer back into that same file before moving on — don't defer that
question to Stage 8, where it's too late to have asked cheaply.

**Verify:** state plainly **which of the three paths/sources** you actually loaded (an explicit
`--config` flag, cwd, or the global fallback) — this is real, user-visible state that changes
what a later edit to either file would affect. The file parses as valid TOML with a top-level
`hub_host` (non-empty), at least one `[[orchestrator]]` table (every entry has a real `name`,
`dir`, and `cmd`), at least one `[[session]]` table (every entry has a real `name`, `harness`,
`endpoint` — a `http://127.0.0.1:<port>` URL — **and its own non-empty `remote_host` and
`remote_tailnet_host`**), and a top-level `layout` (a nested array of names). Cross-check
`layout` in every direction: every `[[orchestrator]]` name must appear in `layout` exactly
once, every `[[session]]` name must appear in `layout` exactly once, no orchestrator name and
session name may collide with each other, and `layout` must contain no name that isn't a real
orchestrator or session. Extract and state `hub_host`, the real M (orchestrator count) and N
(session count), every orchestrator's name/dir/cmd, every session's name/port/harness/
`remote_host`, **the set of distinct `remote_host` values across all sessions** (this is what
Stage 7 groups bodies by — state it explicitly, e.g. "2 distinct hosts: remote-a (alpha, beta),
remote-b (gamma, delta)"), and the resolved column/row layout **naming which pane holds which
orchestrator or session** — this is what every later stage builds from, not raw file order, an
assumed single fixed orchestrator pane, or a hardcoded/globally-shared hostname.

**Gate:** no config found at any of the three tiers (per the Do step above, this is a hard stop
with the user, never an auto-created file), a missing/empty `hub_host`, a missing `name`/`dir`/`cmd` on any orchestrator entry, a
missing `name`/`harness`/`mode`/`endpoint`/`remote_host`/`remote_tailnet_host` on any session
entry, a `mode` value other than `attach` (the only mode this wizard's Stage 4/9 implement —
flag it explicitly here rather than letting it fail silently three stages later, same as an
unimplemented `harness`), a port collision between two session entries **on the same
`remote_host`** (two different hosts may reuse the same port — that's not a collision), a name
collision between an orchestrator and a session, a `layout` that doesn't perfectly match every
orchestrator and session name exactly once (missing, extra, or duplicated), or a file that fails
to parse — stop and ask the user to fix the config rather than guessing what they meant. **If an
explicit `--config` flag was given and that file is missing or fails to parse, this is a hard
stop with no fallback** — do not silently substitute the cwd or global config; tell the user the
path they gave doesn't resolve and let them fix it or drop the flag.

## Stage 2 — Preflight

**Ask:** nothing yet — the no-`ssh` fallback below is a mode switch this wizard makes for
itself from a real, checkable fact (whether the `ssh` binary exists here), not a question to
put to the user.

**No local `ssh` binary — the manual-relay fallback, checked before anything else:**
```bash
command -v ssh >/dev/null 2>&1; echo "ssh: exit $?"
```
If that exits non-zero, this machine has no `ssh` client at all — a real, if unusual,
environment (a sandboxed Claude Code session, a locked-down container). **This does not mean
the wizard can't run; it means every remaining `ssh <host> "<remote command>"` in every later
stage switches to manual-relay mode for the rest of this run:** instead of running the command
yourself, print the exact command verbatim, ask the user to run it on their own machine (or
have them relay it to whichever remote host from a shell that does have `ssh`), and ask them to
paste back its output before you continue past that step. State plainly, once, right here in
Stage 2's report, that manual-relay mode is active and why (no local `ssh`) — don't silently
ask for one-off copy-pastes later without having said so. Every Gate/Verify step downstream
still applies the same way; the only thing that changes is who's holding the keyboard for the
remote half. If `ssh` *is* present locally, none of this applies — proceed with every `ssh`
call below exactly as written, run by you directly.

**Do (SSH reachability, first — looped over every DISTINCT `remote_host` from Stage 1, not just
one — everything after this depends on it, so check each host in isolation rather than letting
a bad one fail silently inside a later, more complicated command). Skip this block entirely in
manual-relay mode — there's nothing to gain by making the user run a bare reachability probe
for you; move straight to asking them to run each real command as it comes up in the stage that
needs it:**
```bash
for host in <distinct remote_host values>; do
  echo "=== $host ==="
  ssh -o ConnectTimeout=5 -o BatchMode=yes "$host" true
  echo "exit: $?"
done
```
`-o BatchMode=yes` forces a hard failure instead of hanging on a password prompt if key auth
isn't set up — each check should return in seconds, not sit waiting for input. A config with
sessions split across 2 hosts needs both checked here — don't stop at the first one that passes.

**Do (config-completeness checks that need the live local environment, not just a parse —
Stage 1 already validated every field is *present*; this is whether what it names actually
*resolves*, which only preflight can answer). Looped over every `[[orchestrator]]` entry:**
```bash
for each orchestrator: test -d "<its dir, ~-expanded>" && echo "<name>: dir ok" || echo "<name>: dir MISSING"
for each orchestrator: command -v "<first token of its cmd>" >/dev/null 2>&1 && echo "<name>: cmd ok ($(command -v <first token>))" || echo "<name>: cmd NOT on PATH"
```
An orchestrator whose `dir` doesn't exist or whose `cmd` doesn't resolve will fail silently deep
in Stage 8 (a pane launched into a broken directory, or a shell reporting "command not found"
with nothing watching for it) unless caught here, where it's cheap to catch and cheap to fix.

**Do (the rest of preflight, per distinct `remote_host` — never a hardcoded hostname, and never
assuming there's only one):**
```bash
tailscale status | grep -E "$(echo <hub_host> | cut -d. -f1)|<pipe-separated short names of every distinct remote_tailnet_host>"
which holler
for host in <distinct remote_host values>; do
  ssh "$host" "which holler"
done
# for every session on a host whose harness is opencode, resolve that host's model and check
# its endpoint. First read the host's OpenCode config to learn <provider>/<model> and the
# provider's baseURL (see "Resolving <provider>/<model>" below):
ssh <that session's remote_host> "cat ~/.config/opencode/opencode.jsonc" 2>&1
# then probe the provider's model-list endpoint at the baseURL that config names:
ssh <that session's remote_host> "curl -s <baseURL>/models" | head -c 200
holler hub status 2>&1
herdr status 2>&1
```
The last two calls are read-only status checks (they tell Stage 3 what already exists so its
plan is accurate) — neither starts, stops, nor changes anything. The model-endpoint checks
are driven by what each host's own OpenCode config says — never assume every host uses the same
provider or model.

**Resolving `<provider>/<model>` (do this once per distinct `remote_host`, here in Stage 2).**
Stages 4 and 5 need the OpenCode model a session should use, written `<provider>/<model>`
(e.g. the provider key from the config's `provider` block plus one of that provider's model
ids). Read it from the host's `~/.config/opencode/opencode.jsonc` — the top-level `model` key
if present, otherwise the single provider/model pair the `provider` block defines. If the config
doesn't name a default or defines several candidates, **ask the user once** which
`<provider>/<model>` to use for that host, and reuse the answer for every session on it. Never
invent one or copy a value from this document.

**Verify:** every orchestrator's `dir` exists and its `cmd`'s first token resolves on `$PATH`
(locally — orchestrators always run on the hub machine, never remote); every host's SSH check
exits 0 — a non-zero exit for any one of them is the *root cause* to report for that host
specifically, not a mysterious later failure three stages downstream (and not a reason to
silently skip that host's sessions — surface it). Every distinct `remote_tailnet_host` shows
`active`/`online` in `tailscale status` (not `offline`); every `which holler` call (local and
every remote host) resolves **to the current single-binary `holler` (hub/body subcommands) —
not the legacy `holler-client`/`holler-server` split** (check with `holler --help`: the new
binary lists `hub`/`body` subcommands, the legacy client lists flat `join`/`run`/`detach`/
`attach` verbs and reports a `0.1.x` version — and note that a plain non-login `ssh <host>
"which holler"` can miss a real Homebrew-installed binary if it's not on the non-login PATH;
retry with `ssh <host> "bash -lc 'which holler'"` before concluding it's actually missing — and
if it's *still* not found even in a login shell, check the common install locations directly
per Stage 0's note (the same brew-prefix-not-on-PATH issue applies to `holler` here, not just
`herdr` there) before concluding it's genuinely absent); for
every `opencode`-harness session, its own host's model endpoint returns real JSON (model list),
not a connection error, and the relevant provider block exists on that host's OpenCode config;
and you now know whether a hub and a Herdr server are already running, for Stage 3's summary.

**If the provider block is missing** (opencode-harness sessions only), don't proceed — the
session's model will fail to resolve. Stop and ask the user which provider/model that host
should use and where its endpoint lives, then add a block of this shape (values are the user's,
not yours to guess):
```json
"<provider>": {
  "name": "<display name>",
  "npm": "@ai-sdk/openai-compatible",
  "options": { "baseURL": "<endpoint base URL>", "apiKey": "<key, if the endpoint needs one>" },
  "models": { "<model>": { "tool_call": true, "reasoning": true } }
}
```

**If `holler` is missing, not on `PATH`, or is the legacy client**, that's a real gate failure
— stop and ask the user how to proceed (e.g. build/deploy the current release to both machines
and put it on `PATH`, or `brew install performant-labs/tap/holler` on each, matching how this
was resolved 2026-09-22) rather than substituting the legacy binary's verbs, which speak a
different, incompatible protocol.

**Gate:** any hard failure here (an orchestrator's `dir` missing or `cmd` not resolving, remote
host offline, `holler` missing/wrong version, an opencode-harness session's model endpoint
down) — stop and tell the user; every later stage depends on this one being real. A missing
`dir`/unresolved `cmd` is the user's config to fix (a typo, an uninstalled CLI, a directory that
moved) — don't guess a substitute path or auto-install anything on their behalf.

## Stage 3 — Present the plan, get assent

**Ask:** yes — this whole stage is the Ask. Nothing in Stage 4 onward runs until the user has
explicitly said to proceed.

**Do:** using what Stages 1 and 2 actually found (not a generic template), write out a concrete
plan and show it before touching anything:

- **Orchestrators (from the config):** the real M, each one's name/dir/cmd — e.g. "2
  orchestrators: o1 (`claude`, `~/Projects/holler`), o2 (`claude`,
  `~/Projects/other-project`)." Not assumed to be exactly one, not assumed to be Claude.
- **Sessions (from the config), grouped by their own `remote_host`** — the real N, names,
  ports, and **harnesses**, organized per host, not as one flat list — e.g. "2 hosts: remote-a
  (alpha opencode 127.0.0.1:47001, beta opencode 127.0.0.1:47002), remote-b (gamma opencode
  127.0.0.1:47001)." This grouping is what Stage 7 will actually build (one body per host) —
  get it right here, not there. If any session's harness isn't `opencode`, say so explicitly
  here and flag that Stages 4/9 don't have implemented logic for it yet — don't silently plan
  as if every entry were OpenCode.
- **What's already live vs. what will be started fresh, per host**, per Stage 2's findings:
  which backend ports already have a process listening (reused, not restarted) vs. which will
  be started new, on which host; whether a Holler hub is already running (reused, or a fresh
  one started — and if fresh, flag that any already-connected body process **on any host** will
  be orphaned and need a rejoin, per Stage 6's note); whether a Herdr server/workspace already
  exists (reused/extended) or will be started from scratch.
- **Anything that would be killed or replaced, named with which host it's on.** Be explicit and
  specific — "I'll kill the existing body process on remote-a (PID 12345) because the hub's
  identity key changed and it can no longer reconnect" is a real plan; "I'll clean things up as
  needed" is not. If nothing needs killing, say that plainly too.
- **The end state**: M orchestrator panes + N session panes in a Herdr workspace, in the
  columns/rows `layout` specifies, hub local, **one body per distinct remote host**, all N
  sessions across all hosts verified with a real round-trip reply.

**Also ask, as its own explicit yes/no, separate from "proceed with this plan":** whether to
install/update the Holler self-status briefing in each orchestrator's own `AGENTS.md` — the
fix for an orchestrator flailing on "what is `<session>` doing" (confirmed live 2026-09-21/22:
before this briefing existed, the orchestrator had no reason to know it could just run `holler
roster`; after, it answered correctly, checked in real time). It's a real edit to a file the
orchestrator's agent reads on every startup, so it's a genuine yes/no like any other file write
this wizard proposes, not a silent default. If yes, Stage 8 writes/updates the briefing block
right before launching that orchestrator's `cmd` — see Stage 8 for the exact text.

**This is orchestrator-side only — sessions get no such briefing, and no MCP either.** Verified
live 2026-09-22 via `holler --help` and a real failed call from a session's own remote host:
`roster`/`say`/`interrupt`/`wait`/`answer` are all **hub-only** — they only work on the machine
running the hub, which an orchestrator is (this AGENTS.md fix targets exactly that), but a
session's own remote host never is. A session has no working Holler self-query command at all,
CLI or otherwise, so there's nothing accurate to brief it on — it just answers "what are you
doing" from its own conversation context, same as without Holler in the picture. A dedicated MCP
server (session-side `whoami`/self-query tool) was tried first and reverted for the orchestrator
side specifically, since a plain `AGENTS.md` edit fixed that gap without a new binary — it
remains a real option worth revisiting **only** for the session side, where the CLI genuinely
has no answer, if that gap ever needs closing.

Then ask directly: **"Proceed with this plan?"** — and wait for a real yes before continuing.
A vague or non-committal reply is not assent; ask again or ask what to change.

**Verify:** the user's response is an explicit go-ahead (or an explicit no / a requested
change).

**Gate:** anything other than clear assent — STOP. If they want changes, revise the plan and
present it again; don't half-start based on an ambiguous answer. If they decline, stop the
whole run here — Stages 1 and 2 already ran (read-only, nothing to undo) but nothing else does.

## Stage 4 — Start the remote agent backends (one per config entry)

**Ask:** nothing further — Stage 3 already covered which ports get reused vs. started fresh; do
what the assented-to plan said, for every `[[session]]` entry the plan called "start fresh."

**Only implemented for `harness = "opencode"` today.** If a config entry's harness is anything
else, this is a real gate — stop and tell the user this wizard doesn't have a start mechanism
for that harness yet, rather than guessing one. Don't skip the entry silently and don't
improvise a plausible-looking command for an unfamiliar harness.

**In manual-relay mode** (per Stage 2), every `ssh <remote_host> "…"` below is a command to hand
the user, not one you run — print it verbatim, wait for them to paste back the real output, then
apply the matching Verify/Gate to what they pasted exactly as if you'd run it yourself.

**Do**, for every `opencode`-harness entry the plan marked as needing a fresh start (its own
`remote_host` — read per-entry from the config, not a single value shared by all — and its
`endpoint`'s port):
```bash
ssh <that entry's remote_host> "cd ~ && nohup opencode --port <port> --hostname 0.0.0.0 --model <provider>/<model> > /tmp/opencode-<name>.log 2>&1 &"
```
repeated once per entry, then:
```bash
sleep 2
```

**Verify**, for every entry (both reused and freshly started), against that entry's own
`remote_host`:
```bash
ssh <that entry's remote_host> "curl -s http://127.0.0.1:<port>/session >/dev/null && echo <name>-up || echo <name>-DOWN"
```

**Gate:** every entry must print `-up`. Any `-DOWN` — check that entry's own log file
(`/tmp/opencode-<name>.log`) before retrying — don't just re-run blind.

**No session-side `AGENTS.md` briefing is needed, and don't add one that tells a session to run
`roster`/`say`/`interrupt`/`wait`/`answer`.** Verified live 2026-09-22 via `holler --help`: those
five verbs are explicitly **hub-only** — they talk to the hub's own local control socket, which
doesn't exist on a session's `remote_host`. A session trying one gets a real, permanent failure
("no live hub reachable"), not a transient one; earlier drafts of this skill (and a live
`AGENTS.md` briefly deployed to a remote host) got this wrong before being corrected. The only
Holler-aware command that *does* work from a session's own host is `holler body status`, and it
reports the body process's own hub connection state (joined/stale/confirmed), not any one
session's task state — not useful for a session answering "what are you doing." A session
should just answer that from its own conversation context, the same way it would without Holler
in the picture at all; there's nothing to brief it on.

## Stage 5 — Create a real first session on each backend, capture its session_id

**Ask:** nothing — this is fully mechanical, looped over every `opencode`-harness config entry
(the same harness restriction as Stage 4 applies here). In manual-relay mode, the same rule from
Stage 4 applies: hand the user the exact `ssh` command, capture the `<NAME>_ID` they paste back.

`GET /session` is a **global, per-user list, not scoped by port**. Don't trust `session[0]` as
"the one just created" — create it with a real POST and read the id straight back from that
response, for every entry:

```bash
<NAME>_ID=$(ssh <that session's remote_host> "curl -s -X POST http://127.0.0.1:<port>/session -H 'Content-Type: application/json' \
  -d '{\"directory\":\"'\$HOME'\",\"model\":{\"id\":\"<model>\",\"providerID\":\"<provider>\"}}'" \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
```

**Verify:** every captured id is non-empty and starts with `ses_`. Write each one back into
your in-memory/working copy of the config's `session_id` field (you'll persist the completed
file in Stage 7) — every later stage that references a session needs the exact id, not a guess.

**Gate:** an empty id means that entry's POST failed (wrong provider/model name, the model endpoint down
for that host) — check the raw response body before moving on to the next entry.

## Stage 6 — Bring up the Holler hub locally

**Ask:** nothing further — Stage 3's plan already said whether the hub gets reused or started
fresh.

**Do**, only if the plan called for a fresh hub (skip entirely if reusing an already-running
one) — bringing up the hub itself is host-agnostic, done exactly once regardless of how many
remote hosts sessions are split across (token minting is per-host and happens in Stage 7):
```bash
holler hub serve --listen 127.0.0.1:41807 --advertise <hub_host> &
tailscale serve --bg 41807
```

**Verify:**
```bash
holler hub status
```
Shows `listening: 127.0.0.1:41807` and a real PID behind it — the hub process actually started.

**Gate:** if `hub serve` fails to start, stop and report the real error (port already bound by
something else, a stale lock, etc.) — don't proceed to Stage 7's per-host token minting against
a hub that isn't actually up.

**If a hub was already running and this is a resume** (its identity key didn't come from a
fresh `hub_identity_generated` log line): already-live remote body processes from an earlier
run may reconnect on their own without needing Stage 7's join again — check `holler roster`
before redoing Stage 7 for any given host. If the hub's identity key *is* fresh (new
`hub serve` invocation, no persisted key), every previously-running body process on every host
is orphaned (pinned to the old key) — this should already be called out in Stage 3's plan as
something that will be killed and rejoined, **per host**, not just once.

## Stage 7 — Migrate the config, join and run one body per distinct remote host

**Ask:** nothing — you have everything from stages 1, 5, and 6. In manual-relay mode, every
`scp`/`ssh` call in this stage (writing the derived per-host config out, `body join`, `body
run`) is another command to hand the user rather than run — same rule as Stage 4.

**Do, in three parts.** First, update the one master file — **`$CONFIG` from Stage 1**,
whichever of the three sources that actually was (explicit `--config` flag, cwd, or the global
fallback), not always the global fallback — with every entry's real `session_id` (and confirm
`hub_host`, every `[[orchestrator]]` entry, and every session's `remote_host`/
`remote_tailnet_host` are still intact); this is the file the user actually edits, so it keeps
every field:
```bash
# write the updated content, including hub_host, every [[orchestrator]], layout, and every
# session's remote_host/remote_tailnet_host, back to "$CONFIG" (Stage 1's path)
```

Second, **group every session by its own `remote_host`** — this is the real structural step,
not an afterthought: a config with sessions on 2 distinct hosts needs 2 separate derived
configs, 2 separate tokens, and 2 separate `holler body run` processes, not one of each.

Third, **for each distinct `remote_host`** (looping, not just doing this once):
1. **Derive** a per-host copy: `[[session]]` tables for *only this host's* sessions (the body
   ignores `remote_host`/`remote_tailnet_host`, `hub_host`, `[[orchestrator]]`, `layout` and `ext`
   tables, so they may stay or be dropped; dropping them keeps the copy small). Use *that* per-host
   copy for `scp`/`--config`, never a copy containing another host's sessions.
2. **Mint a token for this host specifically, with a label that names *this hub too*, not just
   the remote host** — never `<remote_host>-body` alone. Real incident, 2026-09-22: a token
   labeled plain `<remote_host>-body` (this exact pattern) was indistinguishable from a
   completely different, unrelated, already-live pairing to the same remote host from a
   *different* hub — a human reading the label had no way to tell which hub it belonged to,
   and nearly killed the wrong process. A remote host can be paired to more than one hub over its
   lifetime (or the same host runs bodies for two different setups at once, as happened here);
   the label is the only thing a human or another agent has to tell them apart later.

   First, check what — if anything — is already there, the same way Stage 2's SSH check does,
   so the label you choose is informed by real state, not guessed:
   ```bash
   ssh <remote_host> "bash -lc 'ps -ef | grep \"[h]oller body run\"'"
   ```
   If that shows an existing body (a different pairing, or a leftover from a prior run), your
   new label must be visibly distinct from whatever's already there — not just non-colliding as
   a string, but readable at a glance as "a different thing" by someone who didn't run this
   wizard. Then mint, with `hub_host`'s own short name folded into the label:
   ```bash
   holler hub token mint --label <hub_host's short name>-<remote_host>
   ```
   e.g. `hub1-remote-a`, not `remote-a-body` — so a roster entry, a token list, or a process
   inspected later on the remote host all carry which hub it's paired to, not just which remote
   host it runs on.

   **Labels are permanent, even for a token that's `revoke`d or `delete`d — a mint against an
   already-used label always fails.** Confirmed live 2026-09-23: minting `hub1-remote-a` a second
   time (after the first pairing's process was killed and its token `revoke`d, then `delete`d)
   still errored `label "hub1-remote-a" already in use` — `revoke` keeps the record by design (an
   audit trail), and `delete` only invalidates the secret, it does not free the label either.
   Don't spend time trying to reclaim a label. If the first mint attempt for a host you've
   confirmed is safe to reuse (step above) fails this way, just append a counter and re-mint —
   `hub1-remote-a-2`, then `-3`, etc. — rather than hunting for a way to delete the old label.

   **A minted token's join secret is valid for 24 hours by default (`--ttl`); the human-readable
   `expires` that `hub token mint` prints is one day early on holler releases before v0.3.0.** Confirmed
   2026-09-23: a fresh token printed `expires` equal to the mint time, but `--json` showed
   `expires - now = 86400s`, and `format_epoch` was missing a `+ 1` in its day term (fixed in
   holler v0.3.0). So a same-second `expires` is a display bug, not a zero-length window — there is
   no rush before `body join`; only a real `token expired`/`invalid token` error from `body join`
   means the token lapsed. (This skill previously called it a "redeem-immediately window" — wrong.)
   From holler #453 on (releases after v0.3.0), a joined body's token does not expire: the body
   keeps authenticating until `holler hub token revoke <token_id>` ends it, so a working pairing
   never needs a new mint just because a day has passed.
3. **Join and run the body on this host**, using *this host's own* derived config:
   ```bash
   scp <this-host-derived-sessions.toml> <remote_host>:~/sessions.toml
   ssh <remote_host> "holler body join --server wss://<hub_host> --token <token_id>:<secret> --hub-key <hub_key>"
   ssh <remote_host> "nohup holler body run --config ~/sessions.toml --debug quiet > /tmp/holler-body.log 2>&1 &"
   ```

**Verify:**
```bash
holler roster
```

**Gate:** must show **every** session, on **every** host, as `connected` — a partial roster (2
of 3 connected on host A, host B not connected at all, say) is a real failure for whichever
host/entry is missing, not a partial success; don't declare Stage 7 done until every distinct
`remote_host`'s body is confirmed connected with all of *its* sessions. A `no such token` error
for a given host means that host's mint (this stage, step 2) and its join ran against different
state/pepper — stop and check `HOLLER_STATE_DIR` on both sides for that host rather than
re-minting blindly, and don't let one host's token problem block diagnosing/fixing the others.

## Stage 8 — Build the Herdr workspace (M+N panes)

**Ask:** nothing further — Stage 3's plan already said whether the Herdr server/workspace gets
reused, extended, or built fresh.

**Do (server check, first, only if the plan called for a fresh server):**
```bash
herdr status
```
If `server: not running`, start one headlessly (`herdr` alone needs a real TTY and will fail
with "cannot attach without a usable terminal" when run non-interactively — use the headless
form):
```bash
nohup herdr server > /tmp/herdr-server.log 2>&1 &
sleep 3
herdr status   # should now show server: running
herdr pane list
```
A genuinely first-ever Herdr has exactly one pane and no way to know what it should become. This
wizard assumes the agent driving it is **not** itself running inside a Herdr pane (true for the
Claude Code desktop app and for a plain terminal), so there is nothing to "find" — you are building
the workspace from scratch. If you *are* inside a Herdr pane, don't build over your own pane: stop
and ask which workspace to use. Note whichever pane `herdr pane list` shows — that pane becomes the
**first slot** in the build below.

**But a Herdr that has been run before does not start blank — it restores its saved session,
and that restore is more than pane structure.** Confirmed live 2026-09-23: after killing the
server and starting a fresh one, `herdr pane list` came back with all 3 panes from the previous
run already present, the orchestrator pane's Claude conversation **auto-resumed** (Herdr
re-launched it via `claude --resume <id>` on its own), and the two session-viewer panes back as
**dead shells** showing `opencode --session <old id>` → `Session not found` (their old
`session_id`s no longer exist on the freshly started backends). So check `herdr pane list`
before splitting anything: if the pane count already equals M+N, reuse the restored structure —
don't split again (that produces M+N+… panes and a wrong layout) — skip re-launching an
orchestrator pane that resumed idle and healthy, and re-attach only the dead viewer panes in
Stage 9 using this run's *new* session ids. If the count doesn't match, that's the "config
changed" case in "If a stage fails" below.

If reusing an existing server (per Stage 3's plan), check whether a workspace matching this
run's target shape already exists (`herdr workspace list` / `herdr pane list`) before splitting
more — a persisted session can survive a server restart with its pane *structure* intact even
though the processes inside are gone (confirmed live 2026-09-22); reusing that structure is
fine, but verify pane **count matches (M+N)** for *this* config, not an old run's count. If it
doesn't match (the config has grown/shrunk since the layout was built), this should already
have been flagged in Stage 3's plan — don't silently under- or over-provision panes relative to
the current config.

**Do (build the layout)**, using Stage 1's resolved `layout` (the nested array, every
orchestrator name included as a real slot alongside every session name — **not** an assumed
single fixed orchestrator pane, and **not** raw `[[session]]`/`[[orchestrator]]` file order,
which says nothing about columns). Flatten `layout` column by column, top to bottom within each
column, into an ordered list of slots — each slot is either an orchestrator name or a session
name. **The first slot in that flattened list is the pre-existing base pane** — it needs no
split, it already exists; every other slot needs exactly one split to create its pane:
- The first slot of a *new* column: split **right** from the previous column's first-row pane.
- Any later slot within the *same* column: split **down** from the most-recently-created pane
  in that column.

```bash
# layout = [["o1"], ["alpha", "beta"]]  (this skill's example: o1 alone in column 1 — the
# pre-existing base pane, no split — alpha/beta stacked in column 2)
herdr pane split --pane <base-pane> --direction right   # -> alpha's pane (column 2, row 1), e.g. <p3>
herdr pane split --pane <p3> --direction down              # -> beta's pane (column 2, row 2), e.g. <p4>

# layout = [["o1"], ["alpha", "beta"], ["o2"], ["gamma", "delta"]]  (2 orchestrators, 4 sessions)
herdr pane split --pane <base-pane> --direction right   # -> alpha's pane (col 2), e.g. <p3>
herdr pane split --pane <p3> --direction down              # -> beta's pane (col 2), e.g. <p4>
herdr pane split --pane <p3> --direction right          # -> o2's pane (col 3), e.g. <p5>
herdr pane split --pane <p5> --direction right          # -> gamma's pane (col 4), e.g. <p6>
herdr pane split --pane <p6> --direction down              # -> delta's pane (col 4), e.g. <p7>
```
Keep a running map of pane id → slot name as you go (`<base-pane>` -> whatever slot 1 actually
is, `<p3>` -> slot 2, and so on) — this is what both the orchestrator-launch step below and
Stage 9's attach step key off, instead of assuming which pane is which.

**Verify:** `herdr pane list` shows exactly (M+N−1) new panes beyond the pre-existing base pane
(M+N total in the workspace, M of which map to `[[orchestrator]]` entries and N of which map to
real sessions), in the expected layout.

**Gate:** if `herdr pane split` errors, or the pane count doesn't match M+N, stop and confirm
the target workspace/pane with the user before retrying — don't chain more splits on a layout
you haven't confirmed, and don't silently build fewer/more panes than the config calls for.

**Do (launch every orchestrator in whichever pane the build above mapped it to)**, looped over
every `[[orchestrator]]` entry — don't leave any of these panes an unlabeled empty shell, a
human landing on one with no instructions will improvise (confirmed live 2026-09-22: typed a
bare `claude`, got a disconnected session with none of this setup's context, not "the Claude
that was set up"). For each orchestrator's mapped pane (call it `<orch-pane>` for that
orchestrator), first check it isn't already doing something real:
```bash
herdr pane read <orch-pane>
```
If it's already running something (a real shell session mid-task, an existing orchestrator
process, anything beyond an idle prompt), don't launch over it — flag this to the user instead
of overwriting active work, the same way Stage 3 would have.

**If Stage 3's agent-instructions question was answered yes:** brief this orchestrator on Holler
self-awareness before launching its `cmd` (so it's in context from the first prompt) — ensure an
`AGENTS.md` exists in that orchestrator's own `dir` (creating one if none exists, appending a
clearly-marked section if one does — never overwrite unrelated content) containing at least:
```markdown
## Holler self-status

You are the orchestrator "<name>" in a Holler-driven multi-agent setup. Every session you're
driving is a real, separate `holler` roster entry, but its **live roster name is namespaced**
(e.g. `<body-label>/<session-name>`), not just the bare config name — run `holler roster` with
no filter first to see the real names, then use `--prefix <that full name>` to narrow to one.
Use that same full name with `holler say <full name> "…"` to dispatch and `holler interrupt
<full name>` to redirect one that's stuck. Don't guess a session's status from memory or from
what you last told it to do — check `holler roster`.
```
Substitute that orchestrator's real `name`. If Stage 3's answer was no, or the orchestrator's
`AGENTS.md` already has this section from a prior run, skip this sub-step. Two things verified
live 2026-09-22, both worth getting right in the briefing rather than guessing: there
is no `holler status <name>` verb (`roster --prefix` is the real per-session filter), and
`--prefix` matches the roster's live namespaced session id, not the config file's bare `name` —
a bare-name `--prefix` silently matches nothing rather than erroring.

If it's genuinely idle, launch
using that orchestrator's own `cmd` from its own `dir` (both read from the config file back in
Stage 1 — that's where this is decided per orchestrator, never guessed here, and never assumed
to be `claude` even though that's today's common case):
```bash
herdr pane run <orch-pane> "cd <that orchestrator's dir> && <that orchestrator's cmd>"
herdr pane send-keys <orch-pane> enter    # `pane run` types the command; it does NOT press Enter
```
This is a **fresh session with equivalent working context** (the right directory, so any
project `CLAUDE.md`/MCP config in scope) — **not** a resume of whichever session is driving
this wizard right now. Don't claim it's "the same session" to the user; say plainly it's a new
one rooted in the same project. Repeat this whole sub-step for every orchestrator — one launch
per `[[orchestrator]]` entry, each in its own mapped pane, each with its own `dir`/`cmd`.

**Verify**, for every orchestrator's pane:
```bash
herdr pane read <orch-pane>
```
Shows a real startup banner/prompt for that orchestrator's `cmd`, not a bare shell prompt.

**Gate:** if a pane still shows a bare shell after a few seconds, check you sent `enter` (the
same common miss as the OpenCode attach panes) before assuming that orchestrator's `cmd` itself
failed to start. A partial launch (2 of 2 panes built but only 1 of 2 orchestrators actually
started) is a real gap for the un-launched one, not a partial success.

## Stage 9 — Attach each pane to its real remote session

**Ask:** nothing — you have the N session panes from Stage 8 (mapped to session names via
`layout`, not raw file order) and the N session ids from Stage 5.

**Only implemented for `harness = "opencode"` today.** If a session's harness isn't `opencode`,
this is a real gate — stop and tell the user this wizard doesn't have a live-view mechanism for
that harness yet (its Holler-side session may still be real and working; it just has no known
"watch it live in a pane" command here). Don't improvise an attach-like command for an
unfamiliar harness.

**Do**, for every (pane, `opencode`-harness session) pair in order — plain local panes running
`opencode attach` against *that session's own* `remote_tailnet_host` (not a single shared
value — a session on a different host uses a different URL here; `opencode attach --help`
confirms the positional URL argument is an example, not a hostname restriction):
```bash
herdr pane run <pane> "opencode attach http://<that session's remote_tailnet_host>:<port> -s <session_id>"
herdr pane send-keys <pane> enter    # `pane run` types the command; it does NOT press Enter
```

**Verify**, for every pane — use `herdr pane read <pane>` (not `herdr pane get`, which only
returns metadata/title, not actual scrollback content) and confirm the title is a distinctive,
real conversation identifier, not a generic shell prompt.

**Gate:** any pane still showing a generic title/prompt means the attach didn't land — check
you actually sent `enter` (the single most common miss) before assuming something deeper is
wrong.

## Stage 10 — End-to-end verification, don't just assume it worked

**Ask:** nothing.

**Do**, for every session in the config:
```bash
holler say <label>/<name> "reply with just: <name> ready"
```

**Verify:** the reply appears in the CLI output here for every session, regardless of harness.
For `opencode`-harness sessions, additionally confirm it appears in the matching pane's real
content: use `herdr pane read <pane>` and confirm your exact prompt text and its reply both
appear, with a "Last finished HH:MM:SS" timestamp matching when you ran `say`. That match is
what proves the pane is a genuine live view of the session Holler is driving, not a
disconnected second conversation. Repeat for every session — a partial pass (2 of 3 verified)
is a real gap for the untested one, not good enough. A non-`opencode` session that isn't Stage
9-attached to any pane is verified on the CLI half alone — say so plainly rather than treating
it the same as a fully pane-verified one. Also confirm every orchestrator pane from Stage 8 is
still showing a live prompt, not a crashed/exited shell.

**If Stage 3's AGENTS.md briefing was installed, prove it with a genuinely new session — a
resumed one proves nothing.** Herdr (and `claude --resume`) can bring back an orchestrator with
its old conversation history intact; that session may already "know" about Holler from its own
past turns, not from the briefing. Confirmed live 2026-09-23: the auto-resumed orchestrator's
recap already mentioned earlier `holler say` calls, so it would have passed any check regardless
of whether `AGENTS.md` worked. To actually test the briefing: exit that session (`/exit`), launch
a fresh `claude` (no `--resume`) in the same pane, and give it a task with **no Holler context
at all** — e.g. "Send a hello to alpha". Pass only if it discovers the roster on its own
(`holler roster`), finds the namespaced name (`<label>/alpha`), and gets a real reply.

**Gate:** if the CLI reply lands but `herdr pane read` never shows it for a given `opencode`
session, that pane's attach (Stage 9) is stale or pointed at the wrong session id — don't
declare success on the CLI half alone for a session that was supposed to be pane-attached, and
don't rely on `herdr pane get`'s `revision` field either (it does not reliably bump on new
content — confirmed live 2026-09-22, a real new turn landed with `revision` unchanged).

**All 10 stages green = done.** Report the final layout (which pane is which orchestrator or
session) and every session id, so the user has them on hand for a later "recover a dead pane"
run. Then give the user precise instructions to actually **see** the workspace, since
everything above ran headlessly over Herdr's socket API — nothing was visible on screen yet:

1. Focus the right workspace over the API first, so the human lands on it immediately instead
   of whatever workspace Herdr defaults to:
   ```bash
   herdr workspace focus <workspace_id>   # the workspace_id from herdr pane list/workspace list above
   ```
2. Tell the user exactly what to run and where: **"Open a terminal (Terminal.app, iTerm,
   whichever) and run `herdr`"** (bare, no flags — it attaches to the same running server and
   default session this wizard just used; it does not start a second server). If this wizard's
   server was started with an explicit `--session <name>`, say so and tell them to run
   `herdr --session <name>` instead, matching what was actually used.
3. Tell them what they should see immediately upon attaching: the workspace focused in step 1
   (M+N panes arranged in `layout`'s columns/rows — every orchestrator and every session where
   `layout` put it), not an empty default workspace. If they land somewhere else, the
   visible-in-TUI workspace switcher (not a socket command at that point, since they're now
   interacting directly) is how they navigate to it.

## If a stage fails

- **Stage 2's SSH check fails for one host among several:** don't let one bad host block
  diagnosing the others — finish checking every distinct `remote_host` first, then address
  failures. For the failing host, distinguish the real cause before doing anything else —
  `Connection timed out`/`No route to host` means that machine or tailnet is actually
  unreachable (check `tailscale status` for that host specifically); `Permission denied
  (publickey)` means the key isn't authorized for that account on that host (an `ssh-copy-id`
  or an agent-forwarding fix, not a retry); a hang despite `-o BatchMode=yes` still exiting
  cleanly (rather than instantly refusing) can mean the alias in `~/.ssh/config` points
  somewhere unexpected — `ssh -v <that host>` to see what it actually resolved to. Don't retry
  the same failing check hoping for a different result; diagnose, fix, then re-run Stage 2 from
  the top (all hosts, not just the one that failed — a fix might have side effects).
- **A viewer pane dies but `holler roster` still shows that session `connected`:** only the TUI
  pane died, the backend is fine. Reopen and reattach (Stage 9's commands, fresh pane) — no
  restart of `opencode` or Holler needed. Never use `--continue` in place of `-s <session_id>`
  if more than one session exists on that backend — it may resume the wrong one.
- **`holler roster` shows `connected`/`idle` for a session but its pane AND a direct `curl` to
  its endpoint both fail:** that backend process itself is dead (this happened once, from
  `herdr-mirror` reconciliation — not from anything in this recipe). `curl` the endpoint
  directly rather than trusting roster alone right after any disruption, then redo Stage 4 for
  that one entry (no need to restart the others) — and tell the user before doing so, the same
  way Stage 3 would have.
- **A join fails with `no such token` for one host:** that host's own token mint and its join
  must run against the exact same hub state dir and pepper — this is per-host only in the sense
  that each host has its own mint/join pair (Stage 7); the hub's state dir/pepper is still one
  shared thing. If the hub was started with a custom `HOLLER_STATE_DIR`, every command touching
  it (mint, serve, every host's join) needs the same value exported — check that before
  re-minting, and don't assume a token problem on one host means anything about the others.
- **The config changed (an orchestrator or session was added/removed) since the Herdr workspace
  was last built:** don't try to reconcile pane count implicitly — go back to Stage 3, present
  a fresh plan reflecting the new M/N, and get assent again before touching the workspace.
- **A config entry names a harness other than `opencode`:** this wizard's Stage 4/Stage 9 don't
  have implemented start/attach logic for it — don't improvise. Tell the user plainly which
  entry and harness, and that manual setup (or a wizard enhancement) is needed for that one.
- **An orchestrator's pane crashed or its process exited but the others are fine:** that's a
  real gap for that one orchestrator only — reopen its pane and relaunch just its `cmd`/`dir`
  (Stage 8's launch step), don't touch the others.
- **A live, working setup already exists and nothing is actually broken** (e.g. a peer session
  reports "I can't see alpha/beta" because it used `herdr agent list` instead of `herdr pane
  list`/`herdr pane read` — a plain `opencode attach` pane is never tracked as an agent by
  Herdr, by design, regardless of how well it's working): don't re-run this wizard against a
  working setup. Verify with `holler roster` and `herdr pane read` before concluding anything
  is actually wrong, and if it isn't, say so instead of rebuilding.

## Related

- Operational rules and the reasoning behind each step: this repo's `docs/setup-wizard.md`
- Holler repo: `Performant-Labs/holler` — `docs/adr/ADR-0005.md` (attach mode's normative design,
  the per-session `harness` field, and the session-config TOML shape this wizard's config file
  matches), `ADR-0006` (the `wss://`/TLS-proxy hub design this recipe relies on).
- Herdr itself (the workspace/pane tool Stage 0 installs): `https://herdr.dev` /
  `https://github.com/herdrdev/herdr` — a separate project from Holler; its own install script
  (`https://herdr.dev/install.sh`) is what Stage 0 runs.
