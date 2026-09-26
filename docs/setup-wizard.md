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
│  ├─ pane: opencode attach ...47001 -s ...  ├─ holler body run --config sessions.toml
│  └─ pane: opencode attach ...47002 -s ...  │    (attach mode, one entry per backend)
├─ holler hub serve --listen 127.0.0.1:41807 │
│  --advertise <host>.<tailnet>.ts.net       │
└─ tailscale serve --bg 41807                │
```

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

**The body now accepts this file directly as its `--config`.** Holler's config parser
(`crates/holler-body/src/config.rs`) still denies unknown keys, so a typo like `harnes` is an
error, but it knows the wizard's master-file keys (`hub_host`, `layout`, `[[orchestrator]]`, and each
session's `remote_host`/`remote_tailnet_host`) and ignores them, checking only their types. Other tools
can keep their own data in the same file under an explicit namespace: a top-level `[ext.<namespace>]`
table and a per-session `[session.ext.<namespace>]` table, each an arbitrary TOML table the body never
interprets (an `ext` or `ext.<namespace>` that is not a table is refused). The wizard may still
write a derived, stripped copy per remote host (only that host's `[[session]]` tables) to `scp`, so a
remote host's copy carries only its own sessions; the body no longer requires the stripping. The master file
(whichever of the three sources above was actually loaded) keeps every field, including the real
captured `session_id`s, for the next run.

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
scenario the section above describes. `holler hub token mint --label <remote_host>-body` (e.g.
`remote-a-body`) names only the remote side of the pairing, so two different hubs both talking to
the same remote host end up with tokens/bodies that look identical to a human — or another
agent — reading a process list or a token store later. That ambiguity is exactly what made the
incident above possible in the first place: nobody could tell which `remote-a`-something belonged
to which hub without digging.

Fold the hub's own identity into the label instead: `<hub_host's short name>-<remote_host>` —
`hub1-remote-a`, not `remote-a-body`. Before minting, check what's already on the remote host
(`ssh <remote_host> "ps -ef | grep holler"` — the same check "Never kill a process you didn't
identify first" already has you running) and make sure your new label is visibly distinct from
anything already there, not merely a different string. The goal is that a roster entry, a
`hub token list` row, or a process someone inspects six months from now on a shared host names
*which hub* it belongs to on sight — not just that it's "the remote-a one," when there may be
more than one.

**Two mechanical gotchas worth knowing before minting, confirmed live 2026-09-23:**

- **Labels are permanent, even for a `revoke`d or `delete`d token.** `revoke` deliberately keeps
  the record (it's an audit trail); `delete` only invalidates the secret. Neither frees the
  label — minting against a label you've used before, ever, fails with `label "..." already in
  use`, even if that token is long gone. If you're confident the old pairing is genuinely done
  (per the "never kill" section above), don't try to reclaim the label: just append a counter
  and mint `hub1-remote-a-2`, `-3`, and so on.
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
  authenticating on every reconnect. `holler hub token revoke <token_id>` is what ends it. Releases
  before #453 refused a joined body's reconnect once `expires` had passed (hub log reason
  `token_expired`, now retired); against an upgraded hub such a body authenticates again with no new
  join. Before upgrading, revoke any such token whose body must stay cut off: find them with
  `holler hub token list --json` (`bound` rows with a past `expires`; the text output shows `-`).
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

## Automated setup

A Claude Code skill drives this end to end — `setup-wizard`, an 11-stage wizard (Stage 0 through
Stage 10). The skill ships in this repo at
[`agent-skills/setup-wizard/SKILL.md`](../agent-skills/setup-wizard/SKILL.md), which is its source
of truth: install it by copying that file to `~/.claude/skills/setup-wizard/SKILL.md` (the README
has a one-line `curl` for it), or let the README's agent prompt fetch it on a machine that doesn't
have it. Stage
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

## Related

- [ADR 0005](adr/ADR-0005.md) — attach mode's normative design.
- [ADR 0006](adr/ADR-0006.md) — the `wss://`/TLS-proxy hub design this pattern's `tailscale
  serve` step relies on.
