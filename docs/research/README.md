# Research

Index of everything under `docs/research/`.

**Research memos are not decisions — ADRs are.** Every file below is a discussion or survey
document: it informs later work, and in several cases directly names the reasoning behind
decisions already recorded in [`docs/adr/`](../adr/README.md) (the heartbeat/backoff/TTL numbers,
the hygiene/lockout posture, the `interrupt`-gets-an-ack call). None of it is itself a decision
record. If a recommendation below is adopted, it becomes its own numbered ADR — the memo stays as
the dated research trail behind that ADR, it does not get edited into one.

Most of this directory was **ported** from `holler-server`, this repo's retired predecessor —
each ported file says so at the top, with the source branch/commit and a terminology note
(`holler-server`'s "server"/"client" naming maps to this repo's hub/body). Two files are new to
this repo, written directly against the current rebuild.

## Prior art and competitive landscape

- [**prior-art-2026-09-05.md**](prior-art-2026-09-05.md) — the survey table from `holler-server`
  ADR-0001 (2026-09-05): c2c, ai-crew-sync, swarmcode, Harness Remote, opencode-orchestrator,
  Claude Code cross-session messaging, Buzz, Herdr, plus the same-machine cluster and sampled X
  posts. Ported, dated as of 2026-09-05. A 2026-09-25 addendum records tools named in
  [Bustamante's shared-board thread](https://x.com/nicbstme/status/2103355485973848128):
  October Bus, MCP Agent Mail, SW4RM, Paperclip, IAP v1.0, SAIHM, Collie, herdrup, and five named-but-unpublished claims. The
  same rows are on the comparison chart in
  [competitive-landscape-2026-09-05.md](competitive-landscape-2026-09-05.md).
- [**competitive-landscape-2026-09-05.md**](competitive-landscape-2026-09-05.md) — Grok's
  follow-up vendor survey (2026-09-05): Google/Antigravity, Cursor, Windsurf, Devin, Replit,
  GitHub Copilot/VS Code Agent Host, Amazon Q, Zed, Warp, Continue.dev, Amp, OpenCode, and an
  open-source-project cluster. Ported, dated as of 2026-09-05.
- [**landscape-2026-09-07.md**](landscape-2026-09-07.md) — **new to this repo.** The 2026-09-07
  exploration one layer up the stack: A2A v1.0 (AAIF/Linux Foundation), the OpenCode↔A2A wrapper
  projects, AWS AgentCore Gateway, the open-source A2A gateway cluster, AGNTCY SLIM's
  end-to-end-encryption-vs-supervisable-hub tradeoff, remote-daemon/bridge projects, ACP v2's
  `requires_action` state, and the boring outbound-only transport precedent (NATS leaf nodes, CI
  runner agents, JSON-RPC 2.0). Dated 2026-09-07, re-checked in places on 2026-09-10 while
  porting.

## Design-question memos

Four memos, each answering one specific design question the team raised, each independently
surveying comparable tools and primary sources (RFCs, vendor docs) before making a recommendation.
All four predate the hub/body, protocol v2 rebuild — ported verbatim with a terminology note and
links repointed at the exact `holler-server` commit each was verified against, since neither the
ADR numbers nor the code paths they cite exist on `holler-server`'s current `main`.

- [**dropped-connections.md**](dropped-connections.md) (2026-09-05) — heartbeat interval,
  missed-heartbeat threshold, reconnect backoff (Full Jitter, uncapped retries), session
  re-advertisement on reconnect, and a three-state roster (`connected` / `reconnecting` / `gone`).
  Surveys RFC 6455, AWS's backoff-and-jitter writeup, Phoenix Channels, Socket.IO, SignalR, gRPC
  keepalive, and mosh, plus the ADR-0001 competitive set.
- [**security-hijack-dos.md**](security-hijack-dos.md) (2026-09-05) — can the circuit be hijacked
  or knocked over. Verified directly against `holler-server`'s wire/token source at the time: no
  connection cap, no pre-auth timeout, no failed-auth throttle, no app-chosen frame-size limit, and
  one concrete hijack-adjacent gap (auth didn't close the superseded old socket). Cites SSH
  (`MaxStartups`/`MaxAuthTries`/`LoginGraceTime`), fail2ban defaults, and OWASP's WebSocket cheat
  sheet.
- [**message-integrity.md**](message-integrity.md) (2026-09-05) — what TCP/TLS actually guarantee
  for "arrived intact," survey of ack/QoS patterns in JSON-RPC, MQTT, and gRPC, and a
  recommendation to add a minimal ack scoped to `interrupt` only (MQTT QoS 1's shape, not QoS 2's).
- [**session-status-and-wait.md**](session-status-and-wait.md) (2026-09-08) — how to tell an
  orchestrator that a session went idle, blocked, or failed without a webhook and without relying on
  the operator agent's own discipline: wire push plus a blocking CLI `wait` versus A2A push
  notifications, edge-triggered waits with a `turn_id`, why `idle` is not `done`, the proposed state
  set, and "event in, LLM only for judgment." Ported from the never-merged
  `research/session-status-and-wait` branch; the `wait` verb it argued for shipped (see
  [`../orchestrating.md`](../orchestrating.md)), the rest is unverified against current code.

## Positioning

- [**positioning.md**](positioning.md) — the page the top-level [README](../../README.md) links:
  the honest "where Holler fits" statement — a composition, not a green-field protocol; the
  three-layer table (A2A semantics / Holler transport+identity / ACP v2 harness); what is
  explicitly not claimed; the risk register; and the Tailscale "third party in the middle"
  rebuttal from [ADR 0006](../adr/ADR-0006.md).
