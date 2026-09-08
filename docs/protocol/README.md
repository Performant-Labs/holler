# All protocols

Holler's protocol surface, by hop. The **wire** pages are versioned and
cross-machine; the **internal** channels are not.

## Wire (versioned, reachable off-box)

| Hop | Protocol | Page |
| --- | --- | --- |
| hub ⇄ body | JSON-RPC 2.0 over WebSocket (A2A object model, ACP behind it) | [v2](v2.md) |

## Internal, non-wire

These channels carry the same JSON-RPC 2.0 **envelope** as the wire but are
**not** a versioned protocol: they are a local implementation detail, not a
cross-machine contract, and are not documented as a wire spec.

| Channel | Transport | Notes |
| --- | --- | --- |
| hub control socket (`hub/control.sock`) | Unix domain socket, mode `0600` | Carries the JSON-RPC 2.0 envelope **newline-delimited** (methods `control/status`, later `control/roster` / `say` / `interrupt` / `ping` / `query`). **Not versioned**, **not reachable off-box** (local socket only; Windows: unavailable — every one-shot command reports "no live hub reachable on this platform", exit 1). Story [holler #143](https://github.com/Performant-Labs/holler/issues/143). |
