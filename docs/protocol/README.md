# All protocols

Holler's protocol surface, by hop. The **wire** pages are versioned and
cross-machine; the **internal** channels are not.

## Wire (versioned, reachable off-box)

| Hop | Protocol | Confidentiality | Page |
| --- | --- | --- | --- |
| hub ⇄ body | JSON-RPC 2.0 over WebSocket (A2A object model, ACP behind it) | TLS 1.3 via a proxy in front of loopback `ws` (ADR 0006); native TLS is v2 | [v2](v2.md) |

## Internal, non-wire

These channels carry the same JSON-RPC 2.0 **envelope** as the wire but are
**not** a versioned protocol: they are a local implementation detail, not a
cross-machine contract, and are not documented as a wire spec.

| Channel | Transport | Notes |
| --- | --- | --- |
| hub control socket (`hub/control.sock`) | Unix domain socket, mode `0600` | Carries the JSON-RPC 2.0 envelope **newline-delimited** (methods `control/status`, later `control/roster` / `say` / `interrupt` / `ping` / `query`). **Not versioned**, **not reachable off-box** (local socket only; Windows: unavailable — every one-shot command reports "no live hub reachable on this platform", exit 1). Story [holler #143](https://github.com/Performant-Labs/holler/issues/143). |
| body ⇄ spawned harness | ACP **v2**, over the harness child's stdio | **Not** a wire hop (a local subprocess, not reachable off-box) and not this repo's own versioned protocol — it's the [Agent Client Protocol](https://agentclientprotocol.com), pinned to the `agent-client-protocol` crate **2.1.0** (the `unstable_protocol_v2` feature). `session/request_permission` and `elicitation/create` are held open and answered, not auto-denied — ACP v2's `requires_action` state_update is the `blocked`/`input-required` signal (there is no separate "ACP v1 has no blocked state" concern here: this repo never speaks ACP v1). Story [holler #188](https://github.com/Performant-Labs/holler/issues/188). |
