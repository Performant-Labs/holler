# Where Holler fits

> This is the honest "where does Holler actually sit" page the top-level [README](../../README.md)
> and [docs index](../README.md) link to. It is a synthesis, not a research memo and not an ADR —
> it draws on [`prior-art-2026-09-05.md`](prior-art-2026-09-05.md),
> [`competitive-landscape-2026-09-05.md`](competitive-landscape-2026-09-05.md), and
> [`landscape-2026-09-07.md`](landscape-2026-09-07.md), each dated and cited on its own. Where
> those three memos disagree with each other in emphasis, this page states the position; where a
> claim below needs a source, it points back at the memo that has one.

## The claim, stated narrowly

**Holler is a composition, not a green-field protocol.** It does not invent agent-to-agent
messaging, a blocked/waiting-on-input state, or interrupt semantics — A2A, ACP, and decades of
RPC design already solved those (see [`landscape-2026-09-07.md`](landscape-2026-09-07.md) §1, §7).
What Holler actually is:

> A self-hosted, outbound-only circuit for interactive coding sessions on machines you own, with
> per-machine minted, revocable identity, and a hub that can supervise — an audit log, turn/spend
> caps — if the operator wants that.

Every word in that sentence is load-bearing and excludes something a reader might otherwise assume:

- **Self-hosted** — not a vendor's cloud relay (excludes Warp Remote Control, Cursor My Machines,
  Copilot `/remote`, Devin Outposts, Antigravity Remote Control — all reviewed in
  [`competitive-landscape-2026-09-05.md`](competitive-landscape-2026-09-05.md)).
- **Outbound-only** — the body dials the hub; the hub never needs an inbound listener reachable
  from the body's network. This is the opposite assumption from A2A's Agent Card model and AWS
  AgentCore Gateway, both of which route to agents that are themselves reachable endpoints (see
  [`landscape-2026-09-07.md`](landscape-2026-09-07.md) §1, §3).
- **Circuit, not a mux** — no PTY attach, no typing into someone else's terminal. See
  [holler-server ADR-0001](https://github.com/Performant-Labs/holler-server/blob/main/docs/adr/ADR-0001.md),
  ported as [`prior-art-2026-09-05.md`](prior-art-2026-09-05.md).
- **Interactive coding sessions** — not a general agent bus, not a workflow engine, not an
  MCP-tools plane.
- **Machines you own** — the threat model is a single operator's own fleet, not a multi-tenant
  public service (this framing recurs through [`security-hijack-dos.md`](security-hijack-dos.md)
  and [`message-integrity.md`](message-integrity.md)).
- **Per-machine minted, revocable identity** — a join token, then a bound credential
  ([ADR 0007](../adr/ADR-0007.md)), not an account, not a shared secret, not network-trust
  (a tailnet or VPN is underlay, never identity — [ADR 0006](../adr/ADR-0006.md)).
- **A hub that can supervise, if the operator wants that** — this is the part SLIM's design
  explicitly argues against (§ below). Holler chooses the supervisable side of that tradeoff on
  purpose, not by oversight.

## The three-layer table

| Layer | Owns | Example primitives | Holler's relationship to it |
| --- | --- | --- | --- |
| **A2A semantics** | Cross-agent discovery, task lifecycle, blocked state | Agent Cards, `Task`, `Part` (`text\|file\|data`), `INPUT_REQUIRED` | Not adopted as the wire protocol (Holler's own [protocol v2](../protocol/v2.md) is JSON-RPC 2.0, deliberately simpler than A2A's discovery/task model for a single-operator scope). Referenced as the semantics other agent-interop tooling expects — see the reserved A2A-bridge ADR slot ([holler-server#376](https://github.com/Performant-Labs/holler-server/issues/376)) for whether/how Holler ever speaks A2A at the hub boundary. |
| **Holler transport + identity** | Hub↔body circuit, join tokens, credentials, presence/roster | `circuit/join`, `session/prompt`, `session/presence`, [ADR 0004](../adr/ADR-0004.md)'s JSON-RPC 2.0 over WebSocket, [ADR 0007](../adr/ADR-0007.md)'s token→credential flow | **This is Holler's actual layer.** Everything else in this table is either upstream (ACP, the harness) or downstream (A2A, other agents) of it. |
| **ACP v2 harness** | Driving one coding-agent subprocess or attached endpoint | `session/prompt`, `session/cancel`, `session/update`, the new `requires_action` state ([`landscape-2026-09-07.md`](landscape-2026-09-07.md) §7) | Adopted directly — [ADR 0013](../adr/ADR-0013.md): the body speaks ACP v2 via the official Rust SDK. Holler's `interrupt` maps onto ACP `session/cancel` ([ADR 0009](../adr/ADR-0009.md)). |

Read top to bottom: an A2A-speaking caller elsewhere in an org's agent fleet could, in principle,
reach a Holler-supervised session through a future A2A bridge at the hub (reserved,
[holler-server#376](https://github.com/Performant-Labs/holler-server/issues/376)) — but the
session itself is driven by ACP v2 talking to a real harness, and the hop in between, hub↔body
over an outbound-only, token-identified circuit, is the part nothing else in this survey does.

## What is not claimed

Stated plainly, because the 2026-09-05 survey's own draft language ran the risk of overclaiming
before this port tightened it (see the acceptance check on this repo's tracking issue,
[#307](https://github.com/Performant-Labs/holler/issues/307)):

- **Not** inventing agent-to-agent communication. A2A, MCP, and a decade of RPC protocols exist.
- **Not** inventing a blocked/waiting-on-input state. A2A has `INPUT_REQUIRED`; ACP v2 now has
  `requires_action` ([`landscape-2026-09-07.md`](landscape-2026-09-07.md) §7).
- **Not** inventing interrupt/cancel. ACP already has `session/cancel`; Holler maps onto it
  ([ADR 0009](../adr/ADR-0009.md)).
- **Not** the first tool to do outbound-only, credentialed dial-home. NATS leaf nodes and every
  CI runner agent (GitHub Actions, GitLab, Buildkite) already do this, for build jobs instead of
  coding sessions ([`landscape-2026-09-07.md`](landscape-2026-09-07.md) §8).
- **Not** a claim that no comparable *feature* exists anywhere — Warp Remote Control, VS Code
  Agent Host, and a long tail of OSS projects (`agents-party`, `agent-room`, `harness-remote`,
  `remote-agent`, and others) each cover a piece of this space. The claim is narrower: none of
  them is the specific tuple — self-hosted, outbound-only, minted/revocable per-machine identity,
  heterogeneous harness, cooperative interrupt without killing the session
  ([`competitive-landscape-2026-09-05.md`](competitive-landscape-2026-09-05.md)).

## Risk register

Named risks to this positioning holding up over time, honestly stated rather than buried:

1. **Bring-your-own-compute from a cloud platform.** Several vendors surveyed on 2026-09-05
   (Cursor "My Machines," Devin Outposts) already let an operator register their own hardware with
   a vendor-hosted control plane. If any of them add self-hosting *of the control plane itself* —
   not just the worker — the "self-hosted" leg of Holler's claim narrows. Nothing found as of
   2026-09-07 does this; worth re-checking periodically.
2. **AGNTCY SLIM.** Not a competitor on Holler's own axis (SLIM explicitly trades away hub
   supervisability for end-to-end confidentiality — see
   [`landscape-2026-09-07.md`](landscape-2026-09-07.md) §5) but a risk to the *framing*: if SLIM's
   no-visible-hub model becomes the dominant expectation for agent-to-agent transport, Holler's
   supervisable-hub choice needs to be defended as a deliberate tradeoff (audit log, spend caps)
   rather than assumed as obviously correct. This page states that tradeoff explicitly so it isn't
   discovered as a surprise later.
3. **agentgateway's guardrails.** [agentgateway](https://agentgateway.dev/) and its cluster of
   open-source A2A gateways add auth, rate limiting, and policy enforcement in front of
   already-reachable agents. Whether any of them extend that into token-minted, outbound-only
   identity for *unreachable* agents — closing the specific gap Holler occupies — was **not fully
   verified during this port** (see [`landscape-2026-09-07.md`](landscape-2026-09-07.md) §4's own
   caveat). Flagged here explicitly: **verify before claiming the spend-cap gap** against this
   cluster specifically, rather than assuming the 2026-09-07 read still holds.
4. **A2A's own evolution.** A2A v1.0 already added multi-tenancy and signed Agent Cards in one
   release cycle. A future version that adds an outbound-only transport binding (SLIM is already
   positioned as exactly such a binding for A2A/MCP, per §5) would meaningfully close Holler's gap
   from the standards side rather than the vendor side — a different, harder-to-predict kind of
   risk than a single vendor shipping a competing feature.

## The "third party in the middle" rebuttal (Tailscale)

Verbatim from [ADR 0006](../adr/ADR-0006.md), because this is the question every reader of the
v1 deployment story asks first:

> TLS terminates on *your* hub machine with a private key that never leaves it; Tailscale sees no
> frame content; direct peers stay LAN-direct; DERP only ever carries encrypted WireGuard if a
> direct path fails. The dependency is on certificate issuance and DNS, not on your data.

Tailscale is underlay and a proxy, never identity: `circuit/join` with a minted token is still
required regardless of network path, and a tailnet IP is never trusted as auth
([ADR 0006](../adr/ADR-0006.md)). This is the same "supervisable hub, not end-to-end-blind" choice
named in risk #2 above, applied one layer down at the transport-underlay question specifically.
