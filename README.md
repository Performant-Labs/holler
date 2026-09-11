# holler

Your agents are just a holler away — one binary, hub or body.

This repository is under construction — see the [Phase 0 epic](https://github.com/Performant-Labs/holler-server/issues/320) for what is being built here and why.

## Documentation

See [docs/README.md](docs/README.md) — the index. For testing: [the harness design](docs/testing.md) and [how to run the tests](docs/running-tests.md).

## Where Holler fits

Holler is a composition, not a green-field protocol: a self-hosted, outbound-only circuit for
interactive coding sessions on machines you own, with per-machine minted, revocable identity and
a hub that can supervise. See [docs/research/positioning.md](docs/research/positioning.md) for
the full honest statement — the three-layer table (A2A semantics / Holler transport+identity /
ACP v2 harness), what is not claimed, and the risk register — and
[docs/research/](docs/research/README.md) for the dated research it's built on.

## Attach convenience

`holler body attach` removes the "curl the endpoint and hand-edit a TOML" dance for attaching to an OpenCode session another process already owns (a Herdr pane, or a bare `opencode serve`):

```
holler body attach sessions --endpoint http://127.0.0.1:4096
```

Lists that endpoint's sessions as a `SESSION_ID TITLE UPDATED` table, newest first (`--json` prints a `{"sessions": [...]}` document instead). `--endpoint` defaults to `http://127.0.0.1:4096`; a non-2xx or unreachable endpoint exits 1, naming the URL(s) tried.

```
holler body attach init --endpoint http://127.0.0.1:4096 --session ses_1 --name alpha --out attach.toml --force
```

Writes a ready `mode = "attach"` `[[session]]` row to `--out` (default `./attach.toml`) and prints the next command to run. `--session` defaults to the endpoint's newest session (from the same listing `sessions` uses) and `--name` defaults to `alpha`. Refuses to overwrite an existing `--out` file unless `--force` is given (exit 3). Neither verb ever spawns a process, prompts a model, or touches Herdr — it is pure HTTP plus a file write.

## Harness recipes

A `[[session]]` row is config, not code (ADR 0012): pointing `harness` and `command` at a new
ACP-speaking adapter needs no Holler code change or release. For example, Claude Code itself
doesn't speak ACP — there is no `claude acp` subcommand — so a session that drives it goes through
[`@agentclientprotocol/claude-agent-acp`](https://www.npmjs.com/package/@agentclientprotocol/claude-agent-acp)
(npm), the actively-maintained ACP-project bridge for Claude Code:

```toml
[[session]]
name = "reviewer"
harness = "claude"
command = ["npx", "-y", "@agentclientprotocol/claude-agent-acp@0.1.5"]
```

- This package is maintained by the ACP project, **not shipped or endorsed by Anthropic**.
- Its predecessor, `@zed-industries/claude-code-acp`, is **deprecated** in favor of this package —
  don't reach for it in new config.
- Pin a specific adapter version (as above) rather than trusting `npx`'s latest-on-each-launch
  behavior: a silent adapter upgrade could change ACP behavior underneath an already-running body.
- `holler body support claude` (or `holler body query support claude`) answers the same way it
  does for every other harness — "does `command[0]` (here, `npx`) resolve on `$PATH` right now" —
  there is no `claude`-specific special-casing anywhere in that path.
- This recipe is mechanically identical to any other spawn-mode harness; what it does *not* cover
  is a real round trip against Claude Code itself. That's tracked as a separate, manual acceptance
  gate — [issue #294](https://github.com/Performant-Labs/holler/issues/294) — run by a human with
  real Claude Code credentials, not something CI or a background agent attempts.

## Debug output

Every `holler` role (`hub serve`, `body run`, and every one-shot CLI leaf) accepts `--debug none|quiet|noisy` (or `HOLLER_DEBUG`; the flag wins) and `--log-format text|json` (or `HOLLER_LOG_FORMAT`). Logging always goes to **stderr** — stdout stays reserved for command output (`--json`, `say`'s reply, …). `none` (the default) emits no debug lines; `quiet` emits one line per event with the frame's *shape* only (component, direction, method, id); `noisy` adds the full **redacted** JSON-RPC/HTTP frame body. `info`/`warn` events (connects, drops, refusals) are always emitted regardless of the debug level.

Every event carries a `component`, identifying which layer of a `say`/`interrupt` round trip produced it — so a slow reply can be diagnosed as "HTTP not landed" vs. "model still streaming" without reading code:

| component     | Role | What it logs |
| -------------- | ---- | ------------- |
| `wire`         | hub, body | Every JSON-RPC message in/out over the hub↔body WebSocket circuit (`session/prompt`, `session/update`, `session/presence`, `circuit/ping`, …). |
| `session`      | body | A session's mailbox enqueue/dequeue, its FIFO queue depth (`queue_enqueue`/`queue_dequeue`/`queue_full`), and its state transitions (`idle`/`working`/`input-required`). |
| `acp`          | body | ACP v2 requests/notifications to/from a spawned harness child (redacted), plus the child's spawn attempt/success/failure and connection-closed ("child exit") events. |
| `http_attach`  | body | Each HTTP call the attach driver makes to an externally-owned OpenCode session (method, path, status, elapsed ms), plus SSE connect/event/drop. |
| `talklog`      | hub | Each line appended to a session's on-disk talklog (`prompt`/`update`/`done`). |
| `roster`       | hub | Roster-level events (e.g. a name-claim held by a different token). |
| `registry`     | hub | A live circuit registering or being removed from the hub's in-memory body registry. |
| `token`        | hub | Token mint/redeem/ping activity. |
| `control`      | hub | Requests received on the hub's local control Unix socket (`control/status`, `control/say`, …). |
| `cli`          | either | The process's own startup/parsing (the `logging_started` banner, fail-closed refusals). |

Because the ACP SDK holler pins (`agent-client-protocol` 2.1.0) spawns and owns its child process internally, it exposes no pid or exit-status accessor to this codebase — `acp`'s spawn/child-exit events report `command`/`args`/`cwd` and "connection closed", not a pid, which is the most this driver can observe without forking the SDK.
