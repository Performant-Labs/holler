# Agent compatibility

What Holler has actually been run against, and what it has not. Holler is designed so that a new
agent is a config row plus an ACP-speaking adapter ([ADR 0012](adr/ADR-0012.md)), but "designed
for" is not "tested with". This page records only tested results, each linked to its evidence.
An agent that is not in the table has not been tried.

## Agents

| Agent | How Holler reaches it | Result | Evidence |
|---|---|---|---|
| `stub-acp` (Holler's own deterministic test agent) | spawn, ACP v2 | Pass. Runs in CI. | [#130](https://github.com/Performant-Labs/holler/issues/130) |
| OpenCode, two real sessions on a second machine | attach mode, over a tailnet, no SSH tunnel | Pass, all 8 steps, real hardware | [#317](https://github.com/Performant-Labs/holler/issues/317) |
| OpenCode in a Herdr pane | attach mode | Pass, all 6 steps, real hardware | [#318](https://github.com/Performant-Labs/holler/issues/318) |
| Claude Code, via `@agentclientprotocol/claude-agent-acp@0.79.0` | spawn, ACP v1 fallback | Partial. A real `say` round trip passes. `interrupt` and detach have not been run against it. | [#294](https://github.com/Performant-Labs/holler/issues/294) |
| OpenAI Codex, via `@agentclientprotocol/codex-acp@1.13.1` | spawn, ACP v1 fallback | Pass, for the scope tested. With an API key (`auth_method`, no prior login) and with a prior ChatGPT login (an account with no active plan), on Codex's default model and a small pinned one: say, interrupt (turn ended cancelled, the next say worked) and detach (adapter processes torn down). Permission requests show as `input-required` and are answered by approve, reject, or interrupt while pending. A 16-turn session with a 25 s tool turn, an interrupted long turn and a hub restart in the middle (the body reconnected and the session kept its context) also passed. Limits: no single turn longer than about 30 s of model time, no session of hundreds of turns, and a ChatGPT login has to be done once outside Holler. See the [Codex recipe](../README.md#codex). | [#303](https://github.com/Performant-Labs/holler/issues/303), [#439](https://github.com/Performant-Labs/holler/issues/439), [#470](https://github.com/Performant-Labs/holler/issues/470), [#471](https://github.com/Performant-Labs/holler/issues/471), [#472](https://github.com/Performant-Labs/holler/issues/472), [#473](https://github.com/Performant-Labs/holler/issues/473) |
| Any other ACP agent | spawn | Not tried. | none |

## Transport and platforms

| Setup | Result | Evidence |
|---|---|---|
| Hub on Linux, body on macOS, over a real tailnet | Pass (checkpoints hlr-1405 and hlr-1406). The agent in these runs was `stub-acp`, so this proves the transport, not a real agent. | [#316](https://github.com/Performant-Labs/holler/issues/316), [#193](https://github.com/Performant-Labs/holler/issues/193) |
| Linux arm64 | Real binary built and version-checked on hosted arm64 hardware. Not part of the CI test matrix. | [Releasing](releasing.md) |

## Not yet run

- A multi-hour soak of a real session: [#302](https://github.com/Performant-Labs/holler/issues/302).
- Load against a real model backend rather than `stub-acp`: [#374](https://github.com/Performant-Labs/holler/issues/374).

## ACP versions

Holler asks each agent for ACP v2 first ([ADR 0013](adr/ADR-0013.md)). The Claude Code and
Codex adapters, and the newest published `@agentclientprotocol/sdk`, do not yet implement past
v1, so the body falls back to v1 on its own. What works over v1 is recorded per row: for example,
a Codex permission request shows as `input-required` and can be answered over v1. The v2
authentication flow (`auth/login`, [#459](https://github.com/Performant-Labs/holler/issues/459))
has been tested only against Holler's own stub, because no real adapter negotiates v2 yet.

## Adding a row

Run the agent through a real say round trip, and also interrupt and detach. Record the
run in an acceptance-gate issue, then add the agent here with a link to it. Do not add a row from
a design argument or an untested config.
