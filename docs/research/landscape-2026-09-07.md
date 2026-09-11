# Agent-interop landscape (2026-09-07)

> **This is a research memo, not a decision record — ADRs are.** New for this repo, written from
> the 2026-09-07 exploration referenced by
> [issue #307](https://github.com/Performant-Labs/holler/issues/307) (mirroring
> [holler-server#372](https://github.com/Performant-Labs/holler-server/issues/372)). It follows
> [`prior-art-2026-09-05.md`](prior-art-2026-09-05.md) and
> [`competitive-landscape-2026-09-05.md`](competitive-landscape-2026-09-05.md) two days later, but
> looks at a different layer: not "who else built a talk circuit," but "what has the wider
> agent-to-agent (A2A) interop world done by 2026-09-07, and where does that leave a claim like
> Holler's." Citations are dated because this space is moving fast; several items below were
> re-checked on 2026-09-10 while porting this memo and are noted where the check changed or
> sharpened a claim from the original exploration.

## 1. A2A v1.0 is real, and it is not a niche spec

The Agent2Agent (A2A) protocol — Google-originated, now under the **Agentic AI Foundation (AAIF)**
umbrella at the Linux Foundation — shipped its first stable **v1.0** specification in March 2026.
[AAIF's own project page](https://aaif.io/projects/agent2agent) and
[its "A2A joins AAIF" post](https://aaif.io/blog/a2a-joins-aaif) describe it as already running in
production across mobile platforms, cloud AI infrastructure, financial services, supply chain, and
enterprise IT — with **150+ contributing organizations** cited in AAIF/Linux Foundation materials.
Concretely, A2A appears wired into major agent-hosting surfaces: **Azure AI Foundry**, **Microsoft
Copilot Studio**, **Amazon Bedrock AgentCore Runtime**, and **Salesforce Agentforce 3** are named
integration points in the wider A2A/AAIF ecosystem writeups as of this exploration. (The AAIF blog
post confirms A2A's production breadth in general terms; the specific per-vendor integration list
was not re-verified line-by-line against each vendor's own docs during this port — treat the
vendor list as reported, not independently re-confirmed here.)

Mechanically, A2A v1.0 is JSON-RPC 2.0 over HTTP(S), built around:

- **Agent Cards** — a discovery document naming an agent's capabilities and connection info.
  Critically, **an Agent Card requires a reachable HTTPS endpoint** — A2A's whole discovery model
  assumes the agent being discovered is dialable, not behind an outbound-only NAT/firewall posture.
- **A `Part` model** — a message part is typed `text | file | data` in the public spec text (this
  memo's originating exploration named it `text|raw|url|data`; the difference is not material to
  Holler's positioning either way — the point stands that A2A parts are typed content, not a bare
  string).
- **Task states**, including an explicit **`INPUT_REQUIRED`** state for a task that is blocked
  waiting on the human/caller rather than idle or done.
- v1.0 additions: multi-protocol bindings and version negotiation, multi-tenancy, and **signed
  Agent Cards** for cryptographic identity verification.

Sources: [github.com/a2aproject/A2A](https://github.com/a2aproject/A2A),
[Linux Foundation A2A launch announcement](https://www.linuxfoundation.org/press/linux-foundation-launches-the-agent2agent-protocol-project-to-enable-secure-intelligent-communication-between-ai-agents),
[AAIF A2A v1.0 builder's guide](https://aaif.io/blog/a2a-v1-0-a-builder-s-guide-part-1-discovery-tasks-and-clients).

**Why this matters for Holler:** A2A already owns the *semantics* layer people expect —
task/message/part vocabulary, a blocked state, discovery. It does **not** own transport identity
or how a self-hosted operator reaches a machine behind NAT with no public HTTPS endpoint. That gap
is exactly where the rest of this memo, and [`positioning.md`](positioning.md), sit.

## 2. The OpenCode↔A2A wrapper projects: small, real, and not production-hardened

Two community projects bridge OpenCode sessions onto the A2A wire shape:
[`Intelligent-Internet/opencode-a2a`](https://github.com/Intelligent-Internet/opencode-a2a) and
[`shashikanth-gs/a2a-wrapper`](https://github.com/shashikanth-gs/a2a-wrapper). As of the 2026-09-07
exploration, both share the same three limitations, none of which is a deep architectural
constraint but all of which matter for anyone tempted to point one at the open internet:

- **Both are unlicensed** — no `LICENSE` file at the repo root (same shape of gap the
  2026-09-05 survey already flagged for `c2c`; see [ADR-0001 in this repo](../adr/ADR-0001.md)'s
  reasoning for why an unlicensed repo is "read for reference, don't vendor").
- **Both bind loopback only** — they are single-machine A2A shims for a locally-running OpenCode
  instance, not a cross-machine bridge.
- **Both authenticate with a static bearer token** — no mint/list/revoke lifecycle, no
  per-connection identity; whoever holds the token string is the agent, indefinitely.

**Why this matters for Holler:** these two projects are proof that "wrap a coding-agent harness
in A2A" is an obvious enough idea that it's already been done twice independently — and proof
that nobody has yet paired that wrapper with a real token lifecycle or a cross-machine transport
story. That's the same shape of gap the 2026-09-05 survey found one layer down (heterogeneous
harnesses, yes; minted/revocable cross-machine identity, no).

## 3. AWS AgentCore Gateway: real, but cloud-hosted-agents only

[Amazon Bedrock AgentCore Gateway](https://aws.amazon.com/blogs/machine-learning/introducing-amazon-bedrock-agentcore-gateway-transforming-enterprise-ai-agent-tool-development/)
and the newer [AWS Agent Registry](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/registry.html)
give a single place to register agents, control access, and route traffic — "governs agents across
environments: AWS services, third-party clouds, on-premises infrastructure, or a combination," per
AWS's own [serverless A2A gateway writeup](https://aws.amazon.com/blogs/machine-learning/building-a-serverless-a2a-gateway-for-agent-discovery-routing-and-access-control/).
A consumer reads an agent's descriptor, identifies the protocol (MCP vs. A2A), and connects
accordingly — a single orchestrator can discover and use mixed-protocol agents from one search
result.

The catch, and it's the same catch as A2A's Agent Card model in §1: **AgentCore Gateway routes to
agents that are themselves reachable endpoints** (AgentCore Runtime, or another registered
HTTPS-dialable target). It is a registry-and-router for **cloud-hosted or otherwise inbound-dialable
agents** — it does not solve "join a coding-agent session running on a homelab box behind no
public IP, no port-forward, no reachable HTTPS listener." That's structurally the same gap A2A's
own discovery model has, one layer up the stack (enterprise registry vs. protocol-level Agent
Card).

## 4. Open-source A2A gateways: proxies, still assuming reachability

A cluster of open-source projects put a gateway in front of A2A-speaking agents:
[**agentgateway**](https://agentgateway.dev/) (Rust, backed by solo.io), **TrustGate**,
**TrueFoundry**, **Tetrate**, and **NetFoundry** were named in the 2026-09-07 exploration as this
cluster. All of them are, at core, **proxies**: they add auth, rate limiting, observability, and
routing in front of agents that already expose an A2A- or MCP-shaped endpoint. None of them mint
an outbound-only, NAT-traversing join credential for an agent that has no listener at all — they
assume the thing being proxied is already reachable, the same assumption A2A's Agent Card model
and AgentCore Gateway both make. (Individual per-project claims — exact feature lists, license
terms — were not re-verified line-by-line for this port; treat this section as a category-level
finding: the open-source A2A-gateway cluster is a **reachability-assuming proxy layer**, not a
NAT-traversal or join-token layer.)

## 5. AGNTCY SLIM: the principled opposite of an auditing hub

[**SLIM**](https://datatracker.ietf.org/doc/draft-mpsb-agntcy-slim/) (Secure Low-Latency
Interactive Messaging), an AGNTCY project with an active IETF draft
(`draft-mpsb-agntcy-slim`, Rust implementation), is the most conceptually important finding in
this memo. SLIM is a transport layer for agent protocols (it can carry A2A or MCP) built on gRPC
over HTTP/2 and HTTP/3, and it integrates **Message Layer Security (MLS)** to provide
**quantum-safe end-to-end encryption between endpoints** — messages stay confidential "even when
passing through intermediate nodes or experiencing TLS termination along the communication path."
SLIM's data-plane routing nodes forward messages by metadata without being able to read them;
session-layer clients hold the secure group state.

**The argument worth recording verbatim, because it cuts directly against Holler's own design
posture:** SLIM's whole point is that no intermediary — including the operator's own hub — can
read agent-to-agent traffic. **You cannot have both end-to-end encryption and a policy-enforcing
hub.** A hub that audits every turn, enforces spend/turn caps, or logs prompts for review (the
kind of supervision [ADR 0016](https://github.com/Performant-Labs/holler/issues/17), a reserved
ADR slot as of this writing, will presumably cover — see
[holler-server#375](https://github.com/Performant-Labs/holler-server/issues/375) for the
turn/spend-caps story that will land it) is *structurally* a man-in-the-middle relative to an MLS-secured SLIM session — not an
implementation gap, an architectural choice on the opposite end of the same spectrum. Holler's
honest position (see [`positioning.md`](positioning.md)) is that it deliberately sits on the
hub-can-see-and-supervise side of that line; SLIM is the clearest evidence in this whole survey
that the alternative — outbound-only, name-addressed, provably-blind-hub — is a live, principled
design a serious standards effort is pursuing, not a strawman.

## 6. Remote daemons and bridges: same-vendor swarms, not a heterogeneous hub

Named in the 2026-09-07 exploration as adjacent prior art: **HumanLayer remote daemons**, **t54
agent-commons**, **agentroom**, **a2abridge**, and **claude-code-agent**. These were assessed
against a verdict table with claims 1–6 (matching the axes used in the 2026-09-05 competitive
survey — see [`competitive-landscape-2026-09-05.md`](competitive-landscape-2026-09-05.md) for the
axis definitions) with a per-claim **SHIPPED / PARTIAL / NOT FOUND** rating. The specific per-item
ratings from that verdict table were not re-derived or re-verified during this port — porting them
without the underlying per-claim evidence trail would risk misstating what each project actually
ships. What's safe to state, consistent with §1–5 above and with the 2026-09-05 survey: this
cluster follows the same overall pattern as the paid vendors surveyed on 2026-09-05 — same-vendor
or single-harness remote control, not a self-hosted, heterogeneous, join-token circuit.

## 7. ACP v2 `requires_action`: the "ACP has no blocked state" claim is stale

The Agent Client Protocol's own v2 RFDs (the prompt-lifecycle proposal, published on
[agentclientprotocol.com/rfds/v2/prompt](https://agentclientprotocol.com/rfds/v2/prompt))
introduce a **`requires_action`** session state: distinct from idle, used when the agent needs
user input to continue rather than simply having nothing to do. This directly answers a claim
that circulated earlier in this project's own research (and elsewhere) that "ACP has no blocked
state, unlike A2A's `INPUT_REQUIRED`" — as of ACP's v2 RFDs, that's no longer accurate. (The
2026-09-07 exploration dated this RFD to 2026-08-28; this port re-confirmed the `requires_action`
state's existence in the v2 prompt-lifecycle RFD directly on 2026-09-10, but did not independently
re-derive the exact landing date — cite the RFD page itself, not this memo, for the precise date.)

**Why this matters for Holler:** this repo's own [ADR 0009](../adr/ADR-0009.md) maps Holler's
`interrupt` to ACP `session/cancel`, and Holler's `body` layer sits on top of ACP v2 per
[ADR 0013](../adr/ADR-0013.md). A blocked/`requires_action` state on the ACP side is exactly the
kind of primitive [`positioning.md`](positioning.md)'s three-layer table depends on: A2A supplies
the cross-agent blocked-state *semantics* (`INPUT_REQUIRED`), ACP v2 now supplies the same shape
one hop down at the harness boundary, and Holler's job is to carry that state faithfully across
the hub↔body transport — not to invent it.

## 8. Prior art for the transport pattern itself

Three pieces of much older, thoroughly boring infrastructure precedent were named as grounding for
"outbound-only, minted-credential agents dialing home to a hub" as a *pattern*, independent of the
AI-agent framing entirely:

- **NATS leaf nodes** — a leaf node initiates an outbound connection to a NATS cluster/hub and
  extends the subject space; the hub never dials the leaf. Structurally the same shape as a
  Holler body dialing its hub.
- **CI runner agents** (GitHub Actions self-hosted runners, GitLab Runner, Buildkite agents,
  etc.) — every one of these polls or holds an outbound connection to a coordinator; none of them
  require the runner to expose an inbound listener. This is the most widely deployed instance of
  exactly Holler's transport shape, just for build jobs instead of coding-agent sessions.
- **JSON-RPC 2.0 everywhere** — A2A, ACP, and Holler's own [protocol v2](../protocol/v2.md) all
  independently converged on JSON-RPC 2.0 as the envelope. That convergence is itself a data
  point: it's not a Holler-specific choice, it's the default shape for this entire problem class
  in 2026.

**Why this matters for Holler:** none of these three are "agent" tools, which is exactly the
point — outbound-only credentialed dial-home is old, boring, well-understood infrastructure
practice, not a novel transport invented for this project. Holler's actual contribution is narrow
(see [`positioning.md`](positioning.md)): applying that boring pattern specifically to
interactive coding-agent sessions on machines the operator owns, with per-machine identity and an
optional supervising hub.

## Summary — what changed between 2026-09-05 and 2026-09-07

The 2026-09-05 survey ([`competitive-landscape-2026-09-05.md`](competitive-landscape-2026-09-05.md),
[`prior-art-2026-09-05.md`](prior-art-2026-09-05.md)) looked sideways, at other coding-agent
vendors and OSS projects, and found nobody shipping Holler's exact tuple. This 2026-09-07 pass
looked up a layer, at the agent-to-agent interop standards and infrastructure world, and the
picture is more nuanced than "still nobody's built this":

- A2A v1.0 already standardizes the semantics layer (discovery, tasks, blocked state) — but
  assumes reachable endpoints.
- AWS AgentCore Gateway and the open-source A2A gateway cluster both extend that same
  reachable-endpoint assumption into enterprise/self-hosted registries and proxies.
- SLIM is a serious, principled alternative that explicitly trades hub supervisability for
  end-to-end confidentiality — the opposite tradeoff from a Holler-shaped auditing hub.
- ACP v2 closed the "no blocked state" gap on the harness side.

None of this changes the 2026-09-05 conclusion that nobody ships Holler's specific tuple. It
sharpens *why*: the standards world solved discovery and semantics for reachable agents, and a
separate research thread (SLIM) is solving confidentiality for agents that explicitly don't want a
supervising hub. Holler's claim sits in the space between those two solved problems — see
[`positioning.md`](positioning.md) for the honest statement of exactly what that claim is and
isn't.
