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
| `join(state, ws_url, token_id, secret)` | `body join --server <ws_url> --token <id:secret> --hub-key <hex>` (the hub's real public key, read via `hub status --json` — the harness's stand-in for the operator's out-of-band copy, issue #322) and assert exit 0 |
| `hub_pubkey(state)` | `hub status --json` → the hub's X25519 public key (hex), for a test that wants to pass a *wrong* `--hub-key` |
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

## Load testing: `holler-load-test`

The harness above answers *does it work*. The load harness — the `holler-load-test` binary in [`crates/holler-load-test`](../crates/holler-load-test) (issue [#369](https://github.com/Performant-Labs/holler/issues/369)) — answers *what does it cost*, in numbers.

It exists as a purpose-built binary rather than a third-party tool for the same reason `stub-acp` does: Holler's wire is JSON-RPC 2.0 over a WebSocket ([ADR 0004](adr/ADR-0004.md)), not HTTP REST, and k6/wrk/vegeta/Locust cannot speak it at all. It drives two other binaries, `holler` and `stub-acp`, and by default finds them beside itself in the same `target/<profile>/` directory (override with `--holler-bin` / `--stub-acp-bin`).

### Building it: seed your own target dir from a warm one

What's expensive is compiling the third-party dependency graph, not Holler's own crates. That graph includes `aws-lc-sys`'s hundreds of C files, pulled in through `rustls`. A new `git worktree` starts with an empty `target/`, so a plain workspace build there compiles all of it from zero. That can take hours on a loaded machine.

So give the new worktree its own `target/`, seeded from one that already has a release workspace build, such as the main checkout's or another worktree's:

```bash
scripts/seed-target-dir.sh /path/to/warm/checkout/target
cargo build --workspace --release
./target/release/holler-load-test --scenario connection-scale --ramp 1,50,200 --json-out report.json
```

The script copy-on-write clones the warm dir, which is instant and uses no extra disk on APFS (macOS) and btrfs/xfs. It then deletes the cloned artifacts for Holler's own workspace crates, so those rebuild from your source while every dependency is reused. Measured: 7s to seed, then 17s for the release build (only Holler's five crates compile) and 10s for `cargo test`. `holler` and `stub-acp` are rebuilt from your source in the same pass and land next to the harness, so the defaults find them. A Homebrew-installed `holler` can lag `main`, so don't pass it via `--holler-bin` unless you've confirmed it matches your source.

**Don't share a target dir between worktrees**, whether by `CARGO_TARGET_DIR=<another worktree>/target` or by building in another checkout. Cargo fingerprints workspace crates by *workspace-relative* paths plus source mtimes, and it can't tell two checkouts apart. If your sources are older than the artifacts already in that dir, which is common for a branch whose edits predate someone else's build, cargo judges them fresh, compiles nothing, and leaves you a binary built from the other tree's source. It reports no error, and files new since that build, such as a new scenario module, are never checked at all. (Reproduced: a shared-dir build "finished" in 0.46s with the change missing from the binary.) Sharing also lets concurrent builds overwrite each other's final binaries.

**Keep `--workspace`. Don't narrow it to `-p holler-load-test`.** Cargo unifies dependency features across the packages you select, so a `-p holler-load-test` build compiles `rustls`, `tokio`, and `serde_json` with smaller feature sets than the seeded `--workspace` artifacts. Different features mean different fingerprints, and all of them recompile from scratch.

With no warm target dir anywhere, a plain `cargo build --workspace --release` is the from-scratch fallback. Budget for the full first build.

### What a run reports

Every run prints a human table to stdout and, with `--json-out`, writes the same measurements as JSON. It starts a real `holler hub serve` on a free loopback port (or drives an existing one via `--hub-url` + `--hub-state`), and samples that hub process's own RSS / thread count / CPU (`ps` on both platforms, `/proc` for the Linux-only fd count — see `src/proc.rs` for why CPU is a delta, never `ps %cpu`).

### The two client modes

| Mode | Scenario | What a "client" is |
|---|---|---|
| wire | `connection-scale` | An in-process client running the **real** circuit handshake — Noise XK `circuit/authenticate` → `circuit/prove`, then the bidirectional `circuit/hello` — held open with the same `session/presence` + WS-Ping heartbeat a live body emits. Indistinguishable from a body to the hub: it occupies a registry slot and counts in `hub status --json`'s `clients`. |
| body fleet | `body-fleet` | A real `holler body run` **process**, hosting M spawn-mode `stub-acp` sessions, with a rate-driven `say`/`interrupt`/`roster` call mix against it. |

Wire mode is not a shortcut around real processes — it is the only way to get the metric issue [#370](https://github.com/Performant-Labs/holler/issues/370) asks for. A subprocess cannot report the timing of its own internal handshake steps, and 200 bodies plus their 200 `stub-acp` children would swamp the machine being measured. Body-fleet mode is the process-level path, at the scale processes actually reach.

`--scenario session-scale` (issue [#371](https://github.com/Performant-Labs/holler/issues/371)) reuses body-fleet mode's real `holler body run` fleet, holding the body count N fixed and ramping the *sessions-per-body* count M instead — 10 → 100 → 500 — to isolate what scales with session count rather than connection count. See its own baseline table below.

`--scenario sustained-throughput` (issue [#372](https://github.com/Performant-Labs/holler/issues/372)) also reuses body-fleet mode's real fleet, but holds both N and M fixed and instead drives a **sustained** `say --queue` rate across every session concurrently for a real, non-trivial duration (`--sustained-secs`, default 60s) — a steady-state measurement, not a ramp. See its own baseline write-up below.

`--scenario churn` (issue [#373](https://github.com/Performant-Labs/holler/issues/373)) repeats real join → run → detach cycles, hard-checking that the hub returns to baseline after every one, and measures whether the roster ever notices an attach-mode backend killed from outside Holler. See its own baseline write-up below.

### Scenario 1 baseline: connection scale (issue #370)

Measured 2026-09-21 on macOS 15 / aarch64, 10 logical cores, release build, loopback, `--connect-concurrency 32`. Two independent runs; the spread between them is shown where it matters. **These are a baseline, not thresholds** — #370 is explicit that thresholds are TBD *from* this measurement, so nothing in the harness asserts one.

| N | live / target | `hub status` `clients` | handshake p50 | p90 | p99 | max | hub RSS | hub threads | hub CPU |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | 1 / 1 | 1 ✓ | 1.5–2.3 ms | — | — | 2.3 ms | 8.0–9.4 MiB | 11 | ~0% |
| 50 | 50 / 50 | 50 ✓ | 18.4–18.6 ms | 132–181 ms | 236 ms | 236 ms | 10.1–18.2 MiB | 42 | 1.8% |
| 200 | 200 / 200 | 200 ✓ | 9.1–12.3 ms | 234–281 ms | 500–518 ms | 559–611 ms | 28.0–41.9 MiB | 42 | ~11% |

The WebSocket dial itself is flat and sub-millisecond throughout (p50 0.29–1.86 ms at every rung); everything above is the circuit handshake on top of it. Three things the numbers say:

- **Threads do not scale with connections.** 10 idle → 42 at N=50 → **42 at N=200**. The hub's per-connection unit is a tokio task, not an OS thread; the step from 10 to 42 is the blocking pool the token store's `spawn_blocking` wrappers grow, and it plateaus.
- **RSS per connection is small and roughly flat** — ~80–200 KiB/client over the idle baseline at both N=50 and N=200. The run-to-run RSS spread at N=200 (28 vs 42 MiB) is allocator behaviour, not a per-client difference.
- **Handshake latency is dominated by lock serialization, not by connection count.** The p99 rises 236 ms → ~500 ms from N=50 to N=200 while the median stays in the 9–18 ms band: connections are queueing behind the token store's `flock`, not saturating a CPU.

### What the baseline found: concurrent authentication defeated itself

The first run of this scenario did not produce a baseline — it produced a defect. At N=50 only **3 of 50** connections completed the handshake, and at N=200 only **7 of 200**. The rest were refused `-32002 unauthenticated`, carrying the token store's own text: *"another holler process holds the token lock; retry"*.

`token::bound_record` (called twice per `circuit/authenticate`) and `token::touch_last_seen` (once per presence beat) both ran under the non-retrying `acquire_lock`, whose contention outcome is an error. Since `circuit/authenticate` maps any token-store failure to `-32002`, and the lockout counts a `-32002` as a *failed auth*, the hub's contention with itself cascaded into an IP lockout that force-closed further connections mid-handshake. Issue [#301](https://github.com/Performant-Labs/holler/issues/301) had already diagnosed and fixed this exact defect class one call site over, at `redeem`; both siblings now use the same `acquire_lock_retrying`, and `crates/holler-hub/tests/token_store_test.rs::concurrent_live_path_reads_never_lose_the_lock_race` is the regression guard (358 of 384 calls fail without the fix; 0 with it). The table above is the post-fix measurement.

### Scenario 2 baseline: session scale (issue #371)

`--scenario session-scale` fixes N (`--bodies`, default rung 5) real `holler body run` processes and ramps M (`--session-ramp`, default `10,100,500`) spawn-mode `stub-acp` sessions **per body** — the issue's own wording ("M (`stub-acp` sessions per body)") read literally — so the real total session count is `N * M`: 50 → 500 → 2500 at N=5. Each rung restarts every body fresh with that rung's own M, exactly like scenario 1 tears its ramp steps down between rungs.

Starting M sessions per body is cheap even at M=500: `SessionManager::start` (`crates/holler-body/src/session_manager.rs`) spawns one lightweight tokio task per session, not a real `stub-acp` process — a driver only spawns the real child on that session's first prompt ("restarts the driver on the next prompt … not eagerly", that module's own doc). So the M=500 rung is still 5 real OS processes hosting 2500 idle in-process tasks, plus a handful of real `stub-acp` children for the sessions this scenario actually drives a turn on (the fan-out probes below) — not 2500 real child processes.

Measured 2026-09-21 on macOS 26 / aarch64, 10 logical cores, release build, loopback: `--bodies 5 --session-ramp 10,100,500 --fanout-samples 5 --roster-reads 20`. Two independent runs; the spread between them is shown where it matters. **These are a baseline, not thresholds** — #371, like #370 before it, is explicit that thresholds are TBD *from* this measurement.

| M (sessions/body) | total sessions | `hub status` `sessions` | fan-out p50 | fan-out p90 | fan-out max | roster read p50 | roster read p90 | roster read max | hub RSS | hub threads |
|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 10 | 50 | 50 ✓ | 10.7–10.9 ms | 16.7–17.8 ms | 16.7–17.8 ms | 6.3 ms | 6.5–6.6 ms | 6.7–7.0 ms | 11.5 MiB | 12 |
| 100 | 500 | 500 ✓ | 14.9–15.2 ms | 15.8–21.8 ms | 15.8–21.8 ms | 11.4–11.7 ms | 11.9–12.0 ms | 12.4–13.9 ms | 17.6–17.7 MiB | 12 |
| 500 | 2500 | 2500 ✓ | 39.1–40.8 ms | 40.3–42.3 ms | 40.3–42.3 ms | 34.3–34.7 ms | 34.7–36.5 ms | 37.1–43.2 ms | 44.1–44.6 MiB | 12 |

"Fan-out" here means real `session/presence` propagation latency, not a per-connection push — see `crates/holler-load-test/src/session_scale.rs`'s own module doc for why a pull-based hub with a single shared roster table has nothing to fan a presence change *out* to, and why measuring the real propagation latency into that table (a real `say` driving a real `stub-acp` turn from `idle` to `working`, timed until the hub's roster observes it) is the honest reading of the issue's ask. This is the same propagation path issue #359 fixed a real race in (2026-09-21, a body publishing a transient `idle` presence between queued turns), so this measurement doubles as a standing regression guard for that fix.

Three things the numbers say:

- **The `sessions` invariant held at every rung, both runs.** `hub status --json`'s `sessions` matched the real configured total (50/500/2500) exactly. The scenario hard-fails (non-zero exit, no report emitted) on any mismatch rather than merely reporting one — see "What this found" below for that path firing for real.
- **Both latencies scale with session count, not body count.** N stayed fixed at 5 throughout; presence fan-out roughly quadrupled (≈11 ms → ≈40 ms) and roster read latency grew ≈5.5× (≈6 ms → ≈35 ms) from M=10 to M=500. Both operations walk the hub's roster table, consistent with an O(sessions) cost rather than an O(bodies) one.
- **Fan-out and roster read track each other closely at every rung** (10.7 vs 6.3 ms, 14.9 vs 11.4 ms, 39.1 vs 34.3 ms) — the dominant cost in both is the same roster-table read; the remaining ~4–5 ms gap is the real `say`-then-observe round trip on top of it.

#### What this found: a harness bug, not a hub defect

Unlike scenario 1, this scenario did not surface a Holler defect — but building it did surface a real bug in the harness itself. The first draft reused the same body labels (`ss0`, `ss1`, …) on every rung; the hub's token store refuses a second mint under a label already on file (`label "ss0" already in use`), so every body after the first rung failed to (re)join, `hub status --json` correctly reported `sessions=0`, and the scenario's own hard-fail invariant fired exactly as designed — a clear, non-zero-exit error naming both the mismatch and the underlying body-start failures. The fix is a run-scoped, monotonically increasing body-label counter in `session_scale.rs` instead of restarting the label index at 0 each rung. Recorded here, in the same spirit as #370's own "what the baseline found" entry, because the check that caught it is exactly the one #371 asked to be a hard failure rather than a soft one — and it was.

### Scenario 3 baseline: sustained throughput (issue #372)

`--scenario sustained-throughput` starts a real body fleet (`--bodies`, default 3, each hosting `--sessions-per-body` spawn-mode `stub-acp` sessions, default 1) and drives a **sustained** `say --queue` rate across every session concurrently — round-robin, at `--rate` calls/sec total — for a real, non-trivial wall-clock duration (`--sustained-secs`, default 60s). `--queue` is not incidental: a plain `say` against a busy session is refused outright with `-32009 session_busy`, which would just turn a rate above one session's own completion rate into a wall of harness failures instead of the real thing #372 asks about — whether `SessionManager`'s FIFO queue (`crates/holler-body/src/session_manager.rs`, hard-capped at `QUEUE_CAP = 64` by construction) grows unbounded under sustained load, or drains once load eases. A fixed `COOLDOWN` (8s) after the drive stops, with no new calls sent, is what lets "drains" actually happen before the run reads a "final" depth.

The queue-depth signal itself is not new instrumentation: `holler-body` already emits `component=session` `queue_enqueue`/`queue_dequeue`/`queue_full` debug events, each carrying the queue's own depth, at `HOLLER_DEBUG=quiet` — the level every scenario already runs bodies at (issue #197's instrumentation, `session_manager/task.rs::log_session`). This scenario's only new code is reading that existing line: `FleetMember::start_watched` (`crates/holler-load-test/src/fleet.rs`) pipes each watched body's stderr and relays those three event kinds as `QueueEvent`s, which a background aggregator thread sums across every session this run started (each session has its own independent FIFO). Zero new instrumentation landed in `holler-body` itself.

RSS is sampled repeatedly across the whole run (`--rss-sample-ms`, default 2s) rather than once at steady state, specifically so "flat after warmup" vs. "monotonic growth" is a real series (`metrics::RssPoint`), not a two-point guess.

Measured 2026-09-22 on macOS 26 / aarch64, 10 logical cores, release build, loopback: `--bodies 3 --sessions-per-body 1 --rate 20 --sustained-secs 60 --rss-sample-ms 2000` (3 sessions total; 20 calls/sec round-robin is ≈6.7 calls/sec/session against a measured no-queue turn latency of ~172–176ms/call, i.e. deliberately a little over one session's own completion rate, so a real backlog has something to build from). Two independent runs; the spread between them is shown where it matters. **This is a baseline, not a threshold** — #372, like #370/#371 before it, is explicit that thresholds are TBD *from* this measurement.

| | run 1 | run 2 |
|---|---:|---:|
| `say` calls driven | 1200 | 1200 |
| `say` latency p50 | 1686.9 ms | 1633.3 ms |
| `say` latency p90 | 1749.6 ms | 1779.9 ms |
| `say` latency p99 | 1769.3 ms | 1803.8 ms |
| `say` latency max | 1780.7 ms | 1830.2 ms |
| `say` latency min | 172.6 ms | 175.7 ms |
| `say` failures | 0 | 0 |
| queue depth, max observed | 29 | 29 |
| queue depth, final (post-cooldown) | 0 | 0 |
| queue-depth events observed | 2394 | 2394 |
| `queue_full` refusals (hit the 64 cap) | 0 | 0 |
| hub RSS, idle baseline | 8.0 MiB | 7.9 MiB |
| hub RSS, peak during drive | 10.5 MiB | 10.5 MiB |
| hub RSS, ~10s after cooldown | 9.3 MiB | 9.4 MiB |
| hub threads | 10 (steady) | 10 (steady) |
| hub RSS/client over baseline | 443 KiB | 501 KiB |

Four things the numbers say:

- **The FIFO queue is bounded by construction, and this run never got close to that bound.** `QUEUE_CAP = 64` is a compile-time constant on the session's own `VecDeque`; the largest total depth this run observed (summed across all 3 sessions) was 29 — well under a *single* session's own cap — and it never once triggered a `queue_full` refusal. "Grows unbounded" therefore has both a code-level answer (no — the type cannot hold more than 64 per session) and, now, an empirical one at a real sustained rate ~1.3× a session's own completion rate: the backlog grows, but slowly and boundedly, not runaway.
- **The queue drains completely once load eases.** Both runs' `queue_depth_final` (read after the 8s post-drive cooldown, before any process is torn down) is exactly 0 — every one of the ~1200 queued/dispatched turns this run pushed through eventually ran and cleared. This is the direct answer to #372's "or drains correctly once load eases".
- **`say` latency is queue-wait latency, not Holler overhead, once a backlog exists.** The no-queue floor (the `min_ms` — the first call into each session, before any backlog) is 172.6–175.7 ms, consistent with the stub's own ~100ms of chunk delay plus real process/wire overhead. Once a session has a few queued turns ahead of a new one, the p50 climbs to ~1.6–1.7s — that is the real time spent *waiting in the FIFO*, exactly what `--queue`'s own contract promises (append behind the current turn instead of refusing), not a hub regression.
- **Hub RSS is flat, not growing, once load eases — and both runs land within ~0.2 MiB of each other at every comparable point.** RSS climbs modestly during the drive (8.0 → 10.5 MiB, consistent with buffering ~1200 in-flight/queued `say` calls' state) and then **drops** to 9.3–9.4 MiB and stays flat there for the whole 8s+ cooldown window — below its own drive-time peak, not above it. Two independent 60-second runs showing the same shape (rise during load, fall and flatten once it eases, no post-cooldown climb) is the "no obvious leak under sustained throughput" #372 asks for; it is not proof no slower leak exists at a longer duration, which this scenario's own module doc and the caveat below are explicit about.

No Holler defect and no harness bug surfaced while building or running this scenario — the FIFO behaved exactly as its own hard cap and drain-on-completion design promise, at a real sustained rate. Unlike scenario 1's real concurrency defect or scenario 2's real harness label-reuse bug, there is nothing to report here beyond the measurement itself.

**Caveats, stated plainly:** 60 seconds is long enough to see the shape (rise, plateau, drain) but not long enough to rule out a slow leak that only shows up over many minutes or hours; RSS is sampled every 2s at the process level (`ps`), which cannot distinguish "genuinely flat" from "growing and shrinking within the sampling interval"; and the rate/session-count combination above was chosen specifically to produce *some* real queueing (see the module doc in `crates/holler-load-test/src/sustained_throughput.rs` for the reasoning) — a rate comfortably under a session's own completion rate would show `queue_depth_max: None` (nothing to observe) rather than a bounded backlog, which is also a legitimate, honestly-reported outcome the harness supports.

### Scenario 4 baseline: churn (issue #373)

`--scenario churn` measures two things.

**Churn cycles.** `--churn-cycles` times (default 20), it starts `--bodies` real `holler body run` processes (default 3, each hosting `--sessions-per-body` spawn-mode `stub-acp` sessions, default 1), waits until `hub status --json` reports every body and session, drives one real `say` per session, then detaches and kills every body. After each cycle, `clients` and `sessions` must return to their pre-run baseline within 30s. That's #373's correctness invariant ("no leaked `client_id`/session/token state"), so a cycle that misses it aborts the run with a non-zero exit rather than being reported as a slow number.

**Teardown modes** (`--teardown-modes`, rotated one per cycle; the default, `graceful`, is what Run #1 used):

| mode | how the wave dies | invariant |
|---|---|---|
| `graceful` | `body detach`, then SIGKILL of the process tree | hub back to baseline (hard) |
| `crash` | SIGKILL, no detach: the body never says goodbye | hub back to baseline (hard) |
| `hang` | SIGSTOP: sockets stay open, the body goes silent; then SIGKILL | self-cleanup within `--hang-budget-secs` is *recorded*, not required; after the SIGKILL, back to baseline is hard |
| `restart` | crash, wait for the hub to notice, then `body run` again on the same saved credential | rejoin, same `client_id`, exactly one connected roster row per session (all hard) |

Also: `--parallel-teardown` (tear a wave down all at once), `--resident` (one extra body stays up for the whole run and takes `say` traffic while each wave is torn down, so churn happens next to live traffic), `--label-reuse-probe` (after the run, try re-minting a gracefully detached body's label), `--churn-secs` (run for a wall-clock duration instead of a fixed cycle count), and a per-cycle series of hub RSS, threads, roster rows and minted credential records.

**Compressing the timers.** The roster's thresholds are overridable (`HOLLER_ROSTER_RECONNECT_MS`, `_GONE_MS`, `_PRUNE_MS`, `_SWEEP_MS`), which makes `hang` and pruning observable in minutes rather than hours. **Compress the body's heartbeat in the same proportion** (`HOLLER_HEARTBEAT_INTERVAL_MS`, default 15s): the hub ages rows by when it last heard from the body, so a reconnect threshold below the heartbeat interval sends *healthy* rows to `reconnecting` between beats. A first Run #2 attempt did exactly that (hub timers 5s/10s, heartbeat left at 15s) and produced two false positives — a "detected" dead backend and "disturbed" resident sessions — that vanished once the heartbeat was compressed to match. Production's ratio is 15s : 45s : 180s : 360s; divide all four by the same factor. The harness's hub and every body it spawns inherit its own environment, so exporting these before the run is enough, and the JSON report records them under `config.inherited_env`.

**Dead-backend detection.** This reproduces the 2026-09-21 incident #373 cites: an attach-mode backend (`opencode`) killed outside Holler's control while `holler roster` kept reporting its session `connected`/`idle`. The harness re-runs itself with the hidden `--fake-opencode` flag, which serves the same fake OpenCode HTTP server `holler-body`'s attach tests use, as a real separate process. It attaches a body session to that process, `SIGKILL`s it, and polls the roster (`--dead-backend-window-secs`, default 60) for the session to stop showing `conn_state=connected` with `state` idle or working. Not detecting it within the window is recorded as a result, not treated as a harness failure, since that's the bug being measured.

Measured 2026-09-22 on macOS 26 / aarch64, 10 logical cores, release build, loopback, defaults (`--churn-cycles 20 --bodies 3 --sessions-per-body 1 --dead-backend-window-secs 60`), 93s wall clock. **This is a baseline, not a threshold.**

| | result |
|---|---:|
| cycles that returned the hub to baseline | **20 / 20** |
| wave start → hub reports every body + session, p50 / p99 | 72.4 / 86.0 ms |
| `say` (60 calls), p50 / p99 / max | 183.2 / 313.3 / 313.4 ms |
| `say` failures | 0 |
| last body killed → hub back to baseline, p50 / p99 / max | **11.6 / 13.9 / 13.9 ms** |
| harness detaching all 3 bodies (sequential `body detach` + kill), p50 | 1332 ms |
| hub RSS before / after 20 cycles | 7.9 / 11.7 MiB |
| dead attach backend detected | **no, not within 60s** (roster still `state=idle conn_state=connected`) — [#397](https://github.com/Performant-Labs/holler/issues/397) |
| same, re-run with `--churn-cycles 1 --dead-backend-window-secs 240` | **no, not within 240s**, past the roster's 180s `gone` threshold |

What the numbers say:

- **Hub-side cleanup is correct and effectively immediate.** Every cycle returned `clients` and `sessions` to baseline, and the hub got there within about one poll interval (the 25ms poll plus a `hub status` round trip) of the last body dying. The 1.3s per cycle is the harness's own sequential `body detach` calls, measured separately so it doesn't masquerade as hub latency.
- **The dead-backend bug is still present, and it isn't slow detection; it's none.** SIGKILLing an attach-mode backend leaves its session `connected`/`idle` on the roster for the full window, including a 240s run that outlasts the roster's 180s `gone` threshold. The roster's own `reconnecting`/`gone` timers age rows by the *body's* last heartbeat, and the body is still alive and heartbeating (only its backend died), so those timers never fire. Detection has to come from the body noticing its backend is gone. That fix is tracked as [#397](https://github.com/Performant-Labs/holler/issues/397); this probe is its regression check.
- **RSS rose 3.8 MiB over the run.** That isn't proof of a leak: `gone` roster rows are retained by design until they're pruned 360s after going `gone` (`crates/holler-hub/src/roster.rs`), and this run is shorter than that. A run longer than the prune TTL would tell bounded retention from a real leak.

**Caveats, stated plainly:** timings come from polling, so they're only as precise as the poll interval plus one CLI round trip (tens of ms). Only spawn-mode sessions are churned. The dead-backend probe uses the fake OpenCode server, not a real `opencode`, so it proves the detection gap for any attach backend that dies, not anything specific to OpenCode itself.

#### Run #2: longer, larger, all four teardown modes

Measured 2026-09-22, 11:52 AM MDT, same host, release build. 20 minutes wall clock; 10 bodies × 10 sessions per cycle (100 sessions), the four modes rotating, parallel teardown, a resident body with 10 sessions, and compressed timers (heartbeat 1s, reconnect 3s, gone 12s, prune 24s, sweep 1s — production's ratios ÷15).

```
HOLLER_HEARTBEAT_INTERVAL_MS=1000 HOLLER_ROSTER_RECONNECT_MS=3000 HOLLER_ROSTER_GONE_MS=12000 \
HOLLER_ROSTER_PRUNE_MS=24000 HOLLER_ROSTER_SWEEP_MS=1000 HOLLER_STALL_MS=20000 \
holler-load-test --scenario churn --bodies 10 --sessions-per-body 10 \
  --teardown-modes graceful,crash,hang,restart --parallel-teardown --resident --label-reuse-probe \
  --churn-secs 1200 --hang-budget-secs 30 --dead-backend-window-secs 60
```

**578 cycles. 57,800 `say` calls, zero failures. 578/578 returned the hub to baseline.**

| | n | p50 | p99 | max |
|---|---:|---:|---:|---:|
| wave start → hub reports 10 bodies + 100 sessions | 578 | 521.7 ms | 982.7 ms | 1019.0 ms |
| `say` (one per session per cycle) | 57,800 | 244.1 ms | 342.6 ms | 469.0 ms |
| killed → hub back to baseline, all modes | 578 | 7.1 ms | 23.2 ms | 27.9 ms |
| … `crash` / `graceful` / `hang` / `restart` | 145/145/144/144 | 6.7 / 7.6 / 19.8 / 6.7 ms | 9.5 / 15.3 / 26.5 / 8.7 ms | |
| resident body's `say`, during teardowns | 5,780 | 173.6 ms | 194.8 ms | 252.0 ms |
| `restart`: respawn → rejoined on the saved credential | 144 | 447.2 ms | 528.7 ms | 533.8 ms |

What it establishes:

- **Cleanup is correct however a body dies.** Crash without detach and SIGKILL-after-hang clean up as fast as a graceful detach (all p50 under 20ms, all p99 under 27ms). #373's invariant held 578 times at 100 sessions per cycle.
- **Rejoining on a saved credential is clean.** Across 144 restart cycles: `identity changed = 0` (same `client_id`), `ghost/missing rows = 0` (exactly one roster row per session), `not connected = 0`.
- **A hung body is dropped at the reconnect threshold.** 144/144 hang cycles cleaned up on their own without being killed, p50 2846 ms against a compressed 3s threshold. A single confirming cycle at **production timers** took **45,024 ms**, i.e. exactly the 45s `reconnect` threshold, and 45.02 / 2.85 ≈ 15.8 matches the ÷15 compression. So: a body that goes silent stays counted in `hub status` for ~45s, then drops.
- **Churn doesn't disturb a live neighbour.** The resident body logged 0 disturbances across 5,780 post-cycle checks, and its `say` p50 (173.6 ms) stayed at the no-contention floor while the churning wave's own 100-way concurrent `say` p50 was 244.1 ms.

Two findings worth acting on, each now tracked on its own:

- **A detached body's label is never freed** ([#400](https://github.com/Performant-Labs/holler/issues/400)). Re-minting one is refused (`label "ch0-0" already in use`), and the store grew to **5,781 credential records** over the run, one per join, none reclaimed. Roster rows *are* bounded — they oscillated around 900–1,710 and never grew monotonically, so pruning works — but the credential store is unbounded under churn. That's the "leaked token state" clause of #373's own metric, and the reason `hub token delete`/`revoke` currently has to be an operator chore.
- **`hub token mint` doesn't retry under lock contention** ([#401](https://github.com/Performant-Labs/holler/issues/401)). 864 of 5,781 mints (~15%) had to be retried after `another holler process holds the token lock; retry`. The hub's internal redeem path already retries (`acquire_lock_retrying`, `crates/holler-hub/src/token.rs`); the operator-facing `mint` doesn't, so a scripted join racing a busy hub fails and the caller must implement the backoff. The harness now does, and counts the retries.

**Hub RSS is the open question** ([#402](https://github.com/Performant-Labs/holler/issues/402)). It went 7.9 → 102.1 MiB (peak 135.9 MiB) over 578 cycles, rising steeply for the first ~10 minutes and then oscillating roughly 90–113 MiB for the rest. Threads settled at 20 and stayed. With roster rows bounded, the unbounded credential store is the obvious suspect, but this run neither proves a leak nor rules one out: a longer run, or one holding the credential count fixed (the `restart` mode rejoins on saved credentials instead of minting), would separate "bounded but large" from "still growing". The JSON report's `churn.series` carries the per-cycle `rss_mib` / `roster_rows` / `minted_records` that question needs.

### It is not in CI, deliberately

`holler-load-test` is not wired into any workflow. Issue [#345](https://github.com/Performant-Labs/holler/issues/345) documents, with per-job evidence, that the shared self-hosted pool carries enough unrelated always-on load to make even the six-peer `load_roster_scale_test.rs` flap — a harness whose entire output is *timing numbers* would report that contention as a Holler regression. Run it on a dedicated or lightly-loaded machine and record what else was running, as #369 requires. Scheduling it (nightly, on a pinned host) is tracked on #369, not here.

## Beyond loopback: `interop.yml`

Everything in this harness runs on loopback (ADR 0002: "loopback-`ws`-only" is *kept, not retired*). Testing a *remote* body — hub on one host, body on another, across a tailnet or reverse proxy (the pattern ADR 0002 replaces the old SSH-tunnel stopgap with; the forward ADR that owns the remote pattern is reserved slot [#0006](adr/README.md) ↔ [issue #7](https://github.com/Performant-Labs/holler/issues/7)) — is a separate, opt-in path and is **not** part of the loopback matrix.

The opt-in is the **`test-tag-interop`** label. A catalog case carrying it is run with `cargo test … -- --ignored`, i.e. it is *excluded from the normal suite* and only runs when explicitly invoked (a real cross-process, possibly cross-host run). On the single binary, `interop` needs no sibling build and no `HOLLER_SERVER_BIN` env var threaded in — `cargo_bin` finds `holler` itself (the old two-repo layout required both). The selection/runner layer knows about this tag: `--tag-invert interop` is the default "skip the cross-host cases" for a normal run (see `running-tests.md`).

The catalog's interop cases are described in the master testing issue [#168](https://github.com/Performant-Labs/holler/issues/168) (the `protocol` group, range 2000). There is no catalog *data file* for this tag in-tree yet; the *mechanism* (a label + `-- --ignored` + a separate opt-in run) is what this story documents, and the cases will attach to it as they land.

### The manual cross-OS proof: `.github/workflows/interop.yml`

One rung further out than `test-tag-interop` (which still runs the catalog's `cargo test` harness, possibly cross-host but not necessarily cross-OS or over a real public network) is [`.github/workflows/interop.yml`](../.github/workflows/interop.yml) (issue [#193](https://github.com/Performant-Labs/holler/issues/193)): a manual-`workflow_dispatch`-only GitHub Actions workflow that proves a real hub on Linux talks to a real body on macOS over a real public `wss://` tunnel (a **reserved** ngrok domain, TLS-terminating at ngrok's edge — updated 2026-09-15, switched from an earlier named-Cloudflare-tunnel design). Both jobs build the same `holler` binary from the same checkout — one repo, one binary, one proof that it works cross-OS and cross-network, not just cross-process on loopback.

It never runs on `push`/`pull_request` — only a maintainer dispatching it by hand, from the repository's own Actions tab — because it spends real wall-clock time (a hub + tunnel + a 5-minute macOS body run) and reaches out to real ngrok infrastructure. The reserved domain is already provisioned; it requires one remaining one-time setup step (the `NGROK_AUTHTOKEN` secret — see the workflow file's own header comment); until that's configured, its jobs skip with a clear reason instead of failing. This is the cross-OS evidence catalog case **`hlr-1608`** (interop) points to: run the workflow, then paste the green run's URL as that case's evidence.

## Windows is deferred (off the CI matrix)

The CI matrix is **`ubuntu-latest` + `macos-latest` only** — Windows is deliberately *off*, not soft-failed ([`.github/workflows/ci.yml`](../.github/workflows/ci.yml), and [ADR 0002](adr/ADR-0002.md) records the retirement of "Windows on the CI matrix"). The reason is recorded in the ADR:

> Windows on the CI matrix … Real compile break (client#60) and runner timing (server#302); not v1.

Two concrete facts, both cited by `wire_selftest` itself:
- **A real compile break** (old `holler-client` #60) still breaks the Windows build.
- **Runner timing:** the closed-port canary's `connect-refused` latency on Windows blew the 2 s budget ("real connect-refused latency there blew the budget"), which is "exactly why Windows is off the CI matrix".

The harness's *code* is already cross-OS at the seams (`#[cfg(unix)]` vs `#[cfg(windows)]` spawn commands in `wire_selftest`; the process-group signal fallbacks). Windows is a *deferred story*, not a gap — when the compile break is fixed and a runner with deterministic refusal timing is in place, re-adding it to the matrix is a one-line matrix change.

### Platform parity: `platform_test.rs`

`crates/holler-cli/tests/platform_test.rs` (issue [#315](https://github.com/Performant-Labs/holler/issues/315)) is the small `platform` catalog group's automated half (`hlr-1400`–`1404`, `Type: auto`): it pins the Linux/macOS parity this harness *does* guarantee now that Windows is off the matrix. Five cases: a `--listen localhost:0` name is refused (only a numeric loopback literal is ever accepted — `localhost` is never resolved); IPv6 loopback `[::1]` joins end to end when the environment can bind it and **skips**, never fails, when it cannot; a `wss://` dial fails closed against a local `rcgen`-generated self-signed cert (the OS trust store is never bypassed); state-dir paths, lock files, and 0600 permissions are checked (`rstest`) over the file set that actually carries a permission contract; and an unavailable control socket reports the spec's clear message, exercised deterministically via the test-only `HOLLER_TEST_NO_CONTROL_SOCKET=1` env override.

This file does **not** cover the platform group's two manual, real-hardware cases — `hlr-1405`/`1406`, the cross-machine checkpoints (join/run/ping/roster/reconnect/revoke over a tailnet, and the say/interrupt/reprompt session checkpoints) tagged `test-tag-remote` — those are tracked separately in issue [#316](https://github.com/Performant-Labs/holler/issues/316) and are out of scope for `cargo test`.

### The capstone stubs never replace: `hlr-1103` (issue #317)

Every automated test in this harness — including `hlr-1405`/`1406` above, which are genuinely cross-machine — still drives `stub-acp`, a deterministic fake agent that always claims ACP protocol v2 (see `stub-acp`'s own module doc). No amount of loopback or cross-machine *wire* coverage against that stub proves a *real* harness (`opencode acp`, the Claude Code ACP bridge, …) actually works — and until [issue #363](https://github.com/Performant-Labs/holler/pull/363) added a v1-compatibility fallback, none of them did: every real ACP implementation checked (the newest `opencode`, the newest published `@agentclientprotocol/sdk`) negotiates protocol v1, never v2 (full writeup on [issue #362](https://github.com/Performant-Labs/holler/issues/362)).

**hlr-1103** (issue [#317](https://github.com/Performant-Labs/holler/issues/317)) is the gate that closes that hole: two real `opencode acp` sessions, on a real second machine, over a real tailnet — full run recorded on that issue, including the `roster`/`hub status --json` output and both log files. It is manual and real-hardware for the same reason `hlr-1405`/`1406` are: no stub, however faithful, is evidence that Holler's ACP driver actually works against something a real user would run.

## Known upstream-blocked tests: `agent-client-protocol` lost-wakeup (issue #272)

Four tests in this suite are permanently `#[ignore]`d — not because the behavior they check is wrong, but because they all depend on the same real, well-diagnosed bug in the upstream `agent-client-protocol` crate (a lost-wakeup in that crate's own task composition: the crashed child's stdout EOF never wakes the SDK's own crash-watcher task, so a driver never observes a mid-turn crash within any bounded time). [Issue #272](https://github.com/Performant-Labs/holler/issues/272) is the tracking record — it is the audit trail this table summarizes, not the other way around — and cross-references the upstream evidence: [`agentclientprotocol/rust-sdk` PR #261](https://github.com/agentclientprotocol/rust-sdk/pull/261) and issues [#250](https://github.com/agentclientprotocol/rust-sdk/issues/250)/[#254](https://github.com/agentclientprotocol/rust-sdk/issues/254). Per this repo's external-contribution policy, filing anything against that upstream repo needs a separate, explicit go-ahead that has not been given — do not do it without checking issue #272 first.

| Test | Location |
|---|---|
| `crash_mid_turn_is_error_not_hang` | `crates/holler-cli/tests/acp_driver_crash_test.rs:171` (full investigation in this file's own module doc) |
| `driver_crash_isolated_and_restarts_on_next_prompt` | `crates/holler-cli/tests/session_manager_test.rs:513` |
| `driver_crash_in_one_session_does_not_affect_concurrent_sibling_session` | `crates/holler-cli/tests/session_manager_test.rs:756` |
| `wait_fires_on_failed_when_stub_crashes` | `crates/holler-cli/tests/wait_test.rs:346` |

Close #272 (and un-ignore all four) once either the upstream crate ships a fix, or a local mitigation inside `holler-body`/`holler-cli` lets them pass reliably without waiting on upstream. Before adding a *new* test with a hard dependency on this same crash-detection path, check this table first — it would just be a fifth flake/hang on the same already-tracked defect, not new information.

## Secrets are never in the logs

The harness talks to real processes that (in CI, and in a real deployment) will have credentials — the hub's token store, ACP agent API keys, the GitHub token the runner uses. Two rules keep them out of the log:

1. **Diagnostics go to stderr; stdout is protocol only.** `stub-acp` (and, by the same rule, the body's stdout) carries *only* JSON frames. Anything human-readable — including anything that might echo a token — goes to stderr, which the harness reads but never asserts on and never surfaces in a result table.
2. **The runner never logs raw env.** `test-run.rb` / `test_selection.rb` read `GITHUB_TOKEN` only to build an `octokit` client; they never print it. (See `running-tests.md` for the "token-free" design that keeps even the read out of the preview path.)

## Runner on top of the harness

This doc is about the *wire* harness. There is a second, orthogonal layer: the **test runner** in `scripts/test-run.rb` (Ruby) selects cases from the GitHub issue catalog (the `#168` contract) and, for the `auto` cases, drives them — which ultimately *invokes* the `cargo test` harness above. The two are decoupled: the catalog contract is documented in the master testing issue [#168](https://github.com/Performant-Labs/holler/issues/168); the runner's commands and selection semantics are documented in [`running-tests.md`](running-tests.md).
