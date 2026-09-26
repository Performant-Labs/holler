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
| OpenAI Codex, via `@agentclientprotocol/codex-acp@1.13.1` | spawn, ACP v1 fallback | Partial. Say, interrupt (turn ended cancelled, a later say worked) and detach (adapter torn down) passed once, on a small model with API-key auth, but only after a manual Codex login in the adapter's own home directory: Holler does not answer the adapter's authenticate request ([#439](https://github.com/Performant-Labs/holler/issues/439)). Not tried: ChatGPT-login auth, Codex's default model, longer sessions. | [#303](https://github.com/Performant-Labs/holler/issues/303) |
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

Holler asks each agent for ACP v2 first ([ADR 0013](adr/ADR-0013.md)). The Claude Code adapter
and the newest published `@agentclientprotocol/sdk` do not yet implement past v1, so the body
falls back to v1 on its own. Features that need v2, such as the answerable `blocked` state, do
not apply to an agent that negotiates v1.

## Adding a row

Run the agent through a real say round trip, and also interrupt and detach. Record the
run in an acceptance-gate issue, then add the agent here with a link to it. Do not add a row from
a design argument or an untested config.
