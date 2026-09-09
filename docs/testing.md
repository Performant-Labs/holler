# Testing: the process-level wire harness

**Issue:** [#172](https://github.com/Performant-Labs/holler/issues/172) · **Status:** accepted · **Date:** 2026-09-08

This doc is the design of holler's test harness. It replaces the story-2 placeholder that only recorded the `#168` ID/label/grammar facts and the chain's two decisions. This is the first place the harness design is documented: what it is, how it is laid out, what it shares, and what it deliberately is not.

For **how to run** the tests (commands, runner selection, flags, Ruby gotchas) see [`running-tests.md`](running-tests.md). This doc is the *what and why*; that doc is the *how*.

## What the harness is

A **process-level wire harness**: the tests shell out to the *real* `holler` binary and drive it as if it were a user.

```
   cargo test -p holler-cli
            │  (Rust #[test], in-process)
            ▼
   tests/support/mod.rs
      ├─ spawn ──►  holler hub serve --listen 127.0.0.1:0
      │                  │  (real hub; real JSON-RPC v2 over loopback ws)
      │            Hub ◄──┘   (live process; tests reach it over ws://127.0.0.1:<port>)
      │
      └─ spawn ──►  holler body run --config sessions.toml
                        │  (real body; joins the hub over the wire)
                        └─ spawn ──►  stub-acp   (deterministic ACP v2 agent over stdio)
```

- **Hub side:** a real `holler hub serve` process on a free loopback port, talking protocol v2.
- **Body side:** the harness spawns the **real `holler body run`**, pointed at a `sessions.toml` whose sessions run the **`stub-acp`** agent (a deterministic ACP v2 stub) instead of a real model. The body then *itself* spawns and drives that agent — so the whole client path (body → ACP driver → stdio agent) is exercised, and only the model is a stub.
- **The wire is real.** Bodies join the live hub and the tests assert on real protocol v2 semantics (roster, status, say/interrupt round-trips). Nothing in the path is mocked.

The harness is written in **Rust** as an in-process `#[test]` suite under `tests/`, with a small shared module (`tests/support/mod.rs`). It is *not* a Playwright/WebDriver session-driver (there is no UI to drive) and not a shell-script harness. (Contrast: the **test-runner** in `scripts/test-run.rb` *is* a separate Ruby/shell layer *above* the harness — see "Runner on top".)

### Why real processes over the wire (and not Playwright)

The old `holler-server` / `holler-client` split could only test the wire from one repo at a time. The single-binary rebuild collapses that: one `cargo test` run now exercises hub *and* body against the real protocol. A browser-driver harness (Playwright) would model the *user interface*; there is no UI — the surface is a CLI speaking JSON-RPC over a WebSocket. So the harness speaks the wire directly and spawns real processes, which is both simpler and closer to production.

## Layout

| Path | What | Role |
|---|---|---|
| `crates/holler-cli/tests/support/` | shared harness module | real subprocess orchestration (see "Shared API") |
| `crates/holler-cli/tests/stub-acp/` | a separate binary target (`[[test]] name="stub-acp"`, no `harness`) | the deterministic ACP v2 agent the body connects to |
| `crates/holler-cli/tests/*_test.rs` | integration tests, one file per behavior (see `running-tests.md`) | the actual cases |
| `crates/holler-cli/tests/wire_selftest.rs` | the **canary** — a test-of-tests (see below) | proves the harness can *see* a failure |
| `crates/*/tests/` | per-crate integration tests (proto codec/log/banner, hub token_store) | same convention, no cross-crate deps |
| `crates/holler-cli/tests/fixture/` | in-repo data fixtures (e.g. the CLI surface files) | read-only inputs |

`crates/holler-cli/Cargo.toml` declares the two non-default test targets the harness needs:

```toml
[[test]]
name = "stub-acp"            # the agent stub: a plain binary, not a test binary
harness = false

[[test]]
name = "support_selftest"    # a unit-test of the shared module itself
```

### `stub-acp` — the deterministic ACP v2 agent

`stub-acp` is a real process speaking **JSON-RPC 2.0 over stdio** (newline-delimited JSON, one message per line). It is fully deterministic and cross-OS, and — unlike a model — it can simulate a permission request, so the `blocked` state is testable without a model. Its hard constraints: **no tokio** (a blocking `std::io` line loop plus one worker thread), **no shell**, and stdout carries *only* JSON lines (diagnostics go to stderr).

Flags (parsed by a tiny scanner, no CLI lib):

- `--sessions s1,s2` — advertised session names (parsed; `session/new` always returns the stub's own id).
- `--chunks N` — `agent_message_chunk` notifications per prompt (default 3).
- `--slow` — stretch the inter-chunk gap (200 ms vs ~50) so a `session/cancel` can land mid-turn and is *guaranteed* to be observed between chunks.
- `--ask-permission` — raise a real `session/request_permission` mid-turn, then block the turn on the client's answer (this is what exercises the `blocked` state).
- `--crash-after-prompt` — exit mid-turn (exercises the "agent died" path).

The concurrency model is one shared channel: the main thread forwards every inbound message to a single worker over an `mpsc`; the worker is the sole stdout writer and paces each inter-chunk gap with `rx.recv_timeout`, preferring a small `pending` stash. A `session/cancel` delivered during a gap is picked up by that `recv` immediately — no polling loop, no shared flag, no atomic visibility window — and the turn reports `cancelled`, never a stale `end_turn`.

## The canary: `wire_selftest` is the first gate

`tests/wire_selftest.rs` is a **test-of-tests**, and CI runs it *before* anything else (the `Cargo test` step that would otherwise also build the libs is split out precisely so this runs first — see `running-tests.md` for the command). Its job is to prove the harness can *detect* a failure before any real e2e test exists: if this file were green while the runner were broken, every later e2e would be theatre.

Each case is fast (a 2 s budget) and pins one failure class the runner must observe:

| Canary case | What it proves the harness can see |
|---|---|
| `designed_to_fail_case_is_detected` | a child process that `exit 1`s is observed as a *failure* (not swallowed), and a nested `assert!(false)` under `catch_unwind` is observed as a *panic* (not silently dropped) |
| `dial_closed_port_fails_fast` | a connect to a closed loopback port *fails fast*; it draws several distinct `127.0.0.1:0` ports and requires **at least one** to be refused — a broken runner that swallows refusals (or a port collision with a live process) cannot fake a green here |
| `tokio_runtime_boots` | the multi-thread runtime the whole binary depends on actually *boots* under the test harness (a 1 ms `sleep` must not hang) |

The closed-port case draws 5 ports rather than one because a busy CI runner can transiently hand out a port that another process immediately occupies; a single draw could "succeed" by collision and read as "closed ports accept connections". Requiring one genuine refusal is the check a blind runner cannot fake.

## Shared API: `crates/holler-cli/tests/support/mod.rs`

The shared harness is a public Rust module every integration test in `crates/holler-cli/tests/` uses. Its public surface (the API tests link against):

### The processes

| Name | Purpose |
|---|---|
| `StateDir` | an isolated per-test state dir, so parallel tests never share a hub's on-disk state. `.hub()`/`.body()` give the hub's and body's state roots. |
| `holler_bin()` / `stub_acp_bin()` | the compile-time paths (`env!("CARGO_BIN_EXE_holler")`, `…_stub-acp`) to the two binaries the harness spawns. Read at *compile* time deliberately: a missing binary then fails the *build* of a test target, never `exit(2)` the whole test process and hide every other test behind one blackout. |
| `Hub` | a live `holler hub serve` process bound to a free loopback port; exposes `port` (the OS-assigned port) and `ws_url()` (the `ws://127.0.0.1:<port>` bodies dial). |
| `Body` | a live `holler body run` process (spawned with a `sessions.toml` config) in its own process group. |
| `Stub` | the shared ACP client-side driver for `stub-acp`: spawn it with extra args, talk JSON-RPC over real stdin/stdout pipes, and reap it on drop. |

### Lifecycle (spawning + teardown)

| Method | Signature | Does |
|---|---|---|
| `Hub::start` | `(state: &StateDir) -> Hub` | spawn `holler hub serve --listen 127.0.0.1:0` in its own process group, then `wait_for` (≤10 s) the `{"event":"listening"}` JSON line on stderr to learn the bound port. |
| `Hub::stop` | `(self, timeout: Duration)` | consume the `Hub`; graceful SIGINT to the process group, wait up to `timeout`, then `kill_tree`. The `Drop` impl re-runs the same teardown (5 s) if a test lets the `Hub` fall out of scope or panics — so a hub is **never orphaned**. |
| `Body::start` | `(state: &StateDir, config: &Path) -> Body` | spawn `holler body run --config <sessions.toml>` in its own process group. |
| `Body::stop` | `(self, state: &StateDir, timeout: Duration)` | ask the body to `body detach`, wait up to `timeout`, then `kill_tree` the whole tree (the body + the agents it spawned). |

### The stub driver

| Method | Does |
|---|---|
| `Stub::start(extra)` | spawn `stub-acp` with `extra` args (e.g. `--slow`, `--ask-permission`), piped stdin/stdout (stderr nulled), and become the sole owner of the stdin pipe. |
| `Stub::send(line)` | write one pre-serialized JSON-RPC request line to the stub's stdin. |
| `Stub::read_line()` / `Stub::read_response(id)` | read the next newline-delimited JSON line off stdout; `read_response` skips notifications and returns the response with the given id. |
| `Stub::handshake()` | send `initialize` + `session/new`; assert the negotiated `protocolVersion` is 2 and the session id is `stub`. |
| `Stub::close()` | close stdin (EOF) then reap, returning the exit code. `Drop` does the same so a panicking test never leaks the stub. |

The shared fixed request lines live here too — `INITIALIZE`, `SESSION_NEW`, `PROMPT`, `CANCEL`, `UNKNOWN` (ids 1–5), plus `PERMISSION_REQUEST_ID` (10, the id of the `session/request_permission` request the stub raises under `--ask-permission`). Keeping them in the module means the driver and the tests that use it agree on the wire by construction.

### Observability + cross-process actions

These run the *real* `holler` CLI (in the given state dir) and parse its `--json` output, so tests assert on genuine protocol output rather than re-deriving it:

| Function | Runs / returns |
|---|---|
| `roster_json(state)` | `holler roster --json` → the hub's roster as a `Value` |
| `hub_status_json(state)` / `body_status_json(state)` | `hub status --json` / `body status --json` → a `Value` |
| `mint_token(state, label)` | `hub token mint --label <label> --json` → the `(token_id, secret)` join pair (no live hub needed — the token is persisted under the state dir) |
| `join(state, ws_url, token_id, secret)` | `body join --server <ws_url> --token <id:secret>` and assert exit 0 |
| `say(state, session, text)` / `interrupt(state, session)` | the CLI verbs; return the raw `Output` for the test to assert on |

### The cross-OS plumbing

| Function | Signature | Does |
|---|---|---|
| `wait_for` | `(timeout: Duration, check: impl FnMut() -> Option<T>) -> Option<T>` | poll `check` every ~50 ms until it returns `Some` or `timeout` elapses. **The module's only sanctioned wait** — it is bounded and returns `None` on timeout, so callers can distinguish "became ready" from "timed out". No `thread::sleep` gates anywhere in test code. |
| `kill_tree` | `(child: &mut Child)` | kill the child's *entire process tree* and reap it. Unix: `kill(-pgid, SIGKILL)` (one call reaps the child + every descendant, since the child leads its own group). Windows: `taskkill /F /T <pid>`. Idempotent — a no-op if the child already exited. |
| `make_own_process_group` | `(cmd: &mut Command)` | put the command's child in a new process group (pgid == child pid) at spawn. Unix-only (`process_group(0)`); a no-op elsewhere. Public so the harness's own selftests (and any test that spawns a helper it wants reaped as a tree) reuse the exact spawn-side setup. |

`kill_tree` + `make_own_process_group` are the cross-OS seams: on Unix a process leads its own group and is signalled by that group; on Windows there is no process-group signal, so `signal_tree` is a no-op and teardown falls to `taskkill /F /T`. Tests never touch signals directly — they call `Hub::stop` / `Body::stop` / `kill_tree`, which pick the right mechanism per OS.

### Rules the harness enforces (from [ADR 0002](adr/ADR-0002.md))

- **Bind `127.0.0.1:0`, never `localhost`.** `localhost` can resolve to `::1` as well as `127.0.0.1`; binding the literal IPv4 loopback keeps the port deterministic across machines (see `wire_selftest`'s `127.0.0.1` and `Hub::start`'s `--listen 127.0.0.1:0`).
- **Each spawned process gets its own process group.** This is what lets `Hub::stop` / `Body::stop` signal the *whole* subtree (the hub + the body it spawned + the stub) and reap it — otherwise a panic mid-test would orphan the hub (reparented to init) and leak it.
- **No blind sleeps.** Every "is it ready?" check goes through `wait_for` on an *observable* (the hub's `listening` log line on stderr, the body's stdin/stdout, a protocol frame) — never `thread::sleep` guessing. `wire_selftest` uses a deadline-bounded `sleep` only because it is *proving the runtime drives time*, not waiting on readiness.

## Beyond loopback: `interop.yml`

Everything in this harness runs on loopback (ADR 0002: "loopback-`ws`-only" is *kept, not retired*). Testing a *remote* body — hub on one host, body on another, across a tailnet or reverse proxy (the pattern ADR 0002 replaces the old SSH-tunnel stopgap with; the forward ADR that owns the remote pattern is reserved slot [#0006](adr/README.md) ↔ [issue #7](https://github.com/Performant-Labs/holler/issues/7)) — is a separate, opt-in path and is **not** part of the loopback matrix.

The opt-in is the **`test-tag-interop`** label. A catalog case carrying it is run with `cargo test … -- --ignored`, i.e. it is *excluded from the normal suite* and only runs when explicitly invoked (a real cross-process, possibly cross-host run). On the single binary, `interop` needs no sibling build and no `HOLLER_SERVER_BIN` env var threaded in — `cargo_bin` finds `holler` itself (the old two-repo layout required both). The selection/runner layer knows about this tag: `--tag-invert interop` is the default "skip the cross-host cases" for a normal run (see `running-tests.md`).

The catalog's interop cases are described in the master testing issue [#168](https://github.com/Performant-Labs/holler/issues/168) (the `protocol` group, range 2000). There is no `interop.yml` data file in-tree yet; the *mechanism* (a label + `-- --ignored` + a separate opt-in run) is what this story documents, and the cases will attach to it as they land.

## Windows is deferred (off the CI matrix)

The CI matrix is **`ubuntu-latest` + `macos-latest` only** — Windows is deliberately *off*, not soft-failed ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml), and [ADR 0002](adr/ADR-0002.md) records the retirement of "Windows on the CI matrix"). The reason is recorded in the ADR:

> Windows on the CI matrix … Real compile break (client#60) and runner timing (server#302); not v1.

Two concrete facts, both cited by `wire_selftest` itself:
- **A real compile break** (old `holler-client` #60) still breaks the Windows build.
- **Runner timing:** the closed-port canary's `connect-refused` latency on Windows blew the 2 s budget ("real connect-refused latency there blew the budget"), which is "exactly why Windows is off the CI matrix".

The harness's *code* is already cross-OS at the seams (`#[cfg(unix)]` vs `#[cfg(windows)]` spawn commands in `wire_selftest`; the process-group signal fallbacks). Windows is a *deferred story*, not a gap — when the compile break is fixed and a runner with deterministic refusal timing is in place, re-adding it to the matrix is a one-line matrix change.

## Secrets are never in the logs

The harness talks to real processes that (in CI, and in a real deployment) will have credentials — the hub's token store, ACP agent API keys, the GitHub token the runner uses. Two rules keep them out of the log:

1. **Diagnostics go to stderr; stdout is protocol only.** `stub-acp` (and, by the same rule, the body's stdout) carries *only* JSON frames. Anything human-readable — including anything that might echo a token — goes to stderr, which the harness reads but never asserts on and never surfaces in a result table.
2. **The runner never logs raw env.** `test-run.rb` / `test_selection.rb` read `GITHUB_TOKEN` only to build an `octokit` client; they never print it. (See `running-tests.md` for the "token-free" design that keeps even the read out of the preview path.)

## Runner on top of the harness

This doc is about the *wire* harness. There is a second, orthogonal layer: the **test runner** in `scripts/test-run.rb` (Ruby) selects cases from the GitHub issue catalog (the `#168` contract) and, for the `auto` cases, drives them — which ultimately *invokes* the `cargo test` harness above. The two are decoupled: the catalog contract is documented in the master testing issue [#168](https://github.com/Performant-Labs/holler/issues/168); the runner's commands and selection semantics are documented in [`running-tests.md`](running-tests.md).
