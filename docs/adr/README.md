# Architecture Decision Records

ADR is the mechanism Holler uses to record decisions that matter and to point later work at them. This README is the index: one row per ADR, numbered in order, with the issue that carries it.

## How an ADR becomes real

Do not open a new issue for an ADR. Claim the next reserved slot in this table, retitle it, and write the ADR here:

```
ADR NNNN  ↔  issue #(NNNN + 1)
```

`ADR 0001` lives in [#2](https://github.com/Performant-Labs/holler/issues/2), `ADR 0002` in [#3](https://github.com/Performant-Labs/holler/issues/3), and so on through `ADR 0025` in [#26](https://github.com/Performant-Labs/holler/issues/26). The reserved slots are the 25 issues between the project brief (#1) and the first test-case slot (#27).

Claiming a slot:

1. Pick the lowest-numbered unclaimed slot in the index below.
2. Retitle that issue `ADR NNNN — <title>` and write the ADR in the issue body.
3. Keep the `adr` label. Fill the row in the index here.
4. Link the issue from the doc: `docs/adr/ADR-NNNN.md`.

The issue is the canonical record; the markdown file is a copy for browsing and for later stories that need a stable file to link.

## Index

| # | Title | Status | Issue |
|---|-------|--------|-------|
| 0001 | One binary, two roles: hub and body | accepted | [#2](https://github.com/Performant-Labs/holler/issues/2) |
| 0002 | Decisions retired from holler-server and holler-client | accepted | [#3](https://github.com/Performant-Labs/holler/issues/3) |
| 0003 | CLI surface of the single binary; one version | accepted | [#4](https://github.com/Performant-Labs/holler/issues/4) |
| 0004 | Holler protocol v2 is JSON-RPC 2.0 over WebSocket | accepted | [#5](https://github.com/Performant-Labs/holler/issues/5) |
| 0005 | Hierarchical session names; name is not locator | accepted | [#6](https://github.com/Performant-Labs/holler/issues/6) |
| 0006 | v1 deployment: loopback `ws` behind a TLS-terminating proxy; a tailnet is the supported path | accepted | [#7](https://github.com/Performant-Labs/holler/issues/7) |
| 0007 | Join token, then a body credential | accepted | [#8](https://github.com/Performant-Labs/holler/issues/8) |
| 0008 | Tokens hashed at rest with a pepper; TLS is the proxy's job | accepted | [#9](https://github.com/Performant-Labs/holler/issues/9) |
| 0009 | Interrupt is `session/cancel`; the session survives; the response is the ack | accepted | [#10](https://github.com/Performant-Labs/holler/issues/10) |
| 0010 | Presence is status; the roster is a TTL tri-state driven by periodic presence | accepted | [#11](https://github.com/Performant-Labs/holler/issues/11) |
| 0011 | Address sessions, not hosts — via routable names | accepted | [#12](https://github.com/Performant-Labs/holler/issues/12) |
| 0012 | Bodies are config + a subprocess (or an attached endpoint), never plugins | accepted | [#13](https://github.com/Performant-Labs/holler/issues/13) |
| 0013 | Body protocol is ACP v2 via the official Rust SDK (2.1.0) | accepted | [#14](https://github.com/Performant-Labs/holler/issues/14) |
| 0014 | Attach mode: the body is not the parent of the harness | accepted | [#15](https://github.com/Performant-Labs/holler/issues/15) |
| 0015 | License AGPL-3.0-or-later; outside PRs welcome; AI-disclosure policy | accepted | [#16](https://github.com/Performant-Labs/holler/issues/16) |
| 0016 | Hub-enforced turn and spend caps | draft | [#17](https://github.com/Performant-Labs/holler/issues/17) |
| 0017 | A2A bridge at the hub: agent cards, task mapping, artifacts/data plane | draft | [#18](https://github.com/Performant-Labs/holler/issues/18) |
| 0018 | | reserved | [#19](https://github.com/Performant-Labs/holler/issues/19) |
| 0019 | | reserved | [#20](https://github.com/Performant-Labs/holler/issues/20) |
| 0020 | | reserved | [#21](https://github.com/Performant-Labs/holler/issues/21) |
| 0021 | | reserved | [#22](https://github.com/Performant-Labs/holler/issues/22) |
| 0022 | | reserved | [#23](https://github.com/Performant-Labs/holler/issues/23) |
| 0023 | | reserved | [#24](https://github.com/Performant-Labs/holler/issues/24) |
| 0024 | | reserved | [#25](https://github.com/Performant-Labs/holler/issues/25) |
| 0025 | | reserved | [#26](https://github.com/Performant-Labs/holler/issues/26) |

Reserved test-case slots follow the ADRs: [#27](https://github.com/Performant-Labs/holler/issues/27)–[#126](https://github.com/Performant-Labs/holler/issues/126).

## Superseded

The ADRs that existed under `holler-server` and `holler-client` are retired by this scheme; their decisions are folded into the slots above as they are rewritten. [ADR 0001](ADR-0001.md) through [ADR 0015](ADR-0015.md) are folded in as of this writing; each new ADR's own "Supersedes" line names exactly which legacy ADR(s) it replaces.
