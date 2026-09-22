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
Local machine                              Remote machine (e.g. Jupiter)
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
no remote-OpenCode-endpoint mode.

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
viewing, not just their local viewer panes. Full incident history:
[pl-ops-handbook's Holler+Herdr page](https://github.com/Performant-Labs/pl-ops-handbook/blob/main/src/content/docs/infrastructure/services/holler-herdr-io-jupiter.md).
The tradeoff for the simpler `opencode attach` approach: you lose Herdr's own agent-status
tracking (`idle`/`working`/`blocked`) for the remote session, since Herdr only classifies panes
it manages directly — a plain `opencode attach` pane is just a terminal running a program from
Herdr's point of view, invisible to `herdr agent list` even while fully live and working. Use
`herdr pane list` / `herdr pane read` to see it instead.

## Automated setup

A Claude Code skill drives this end to end — `herdr-workspace`
(`~/.claude/skills/herdr-workspace/SKILL.md`), a 10-stage wizard: load a session config,
preflight, present the plan and get explicit assent before touching anything, start the remote
OpenCode backends, capture real session IDs, bring up the hub, join and run the body, build the
Herdr workspace, attach each pane, and verify every session end to end with a real round-trip
reply. It is config-driven — however many `[[orchestrator]]` and `[[session]]` entries the
config lists is however many panes get built (one or more orchestrators, each any CLI agent,
not just Claude; any number of sessions), never a hardcoded pair — and the session shape is the
same one `holler body run
--config` consumes (Holler's own config parser is `#[serde(deny_unknown_fields)]`, so any
wizard-only settings are migrated out of a derived copy before that file reaches Holler, never
sent to it directly).

## Related

- [ADR 0005](adr/ADR-0005.md) — attach mode's normative design.
- [ADR 0006](adr/ADR-0006.md) — the `wss://`/TLS-proxy hub design this pattern's `tailscale
  serve` step relies on.
- [pl-ops-handbook: Holler + Herdr on a real two-machine setup](https://github.com/Performant-Labs/pl-ops-handbook/blob/main/src/content/docs/infrastructure/services/holler-herdr-io-jupiter.md) —
  the full incident history and every gotcha hit building this, kept up to date as the primary
  source of truth for *why* each step is shaped the way it is.
